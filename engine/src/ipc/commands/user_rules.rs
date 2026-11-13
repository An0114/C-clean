//! 用户自定义清理规则：模型 + 校验 + CRUD（task-078）。
//!
//! # 与内置规则库的关系
//!
//! `rules/cache_rules.json` 是**厂商下发的、只读的、受 SHA-256 保护的**那一层；
//! 用户规则是**本地可变的**那一层。物理上分开（DB 表 vs JSON 文件）而不是塞进
//! `rules_meta.custom_rules`，理由写在 `v4__user_rules.sql` 的文件头：
//! 规则库更新 / 回滚会整份覆盖文件，用户规则会被静默冲掉。
//!
//! # 三条硬约束（都在 [`validate`] 里）
//!
//! 1. **风险等级只允许 L1 / L2**。L3 在本项目里的语义是「红线，永不删除」
//!    （`cleanup_item` 表注释：L3 项永不写入本表）。一条 L3 的清理规则会扫出
//!    永远删不掉的条目，是自相矛盾的，所以直接拒；
//! 2. **模式必须有一个足够深的静态根目录**。`D:\**` 这种模式没有静态根，
//!    实现上等于「遍历整个 D 盘」——一次误操作就是几十分钟的磁盘扫描。
//!    要求静态根至少是 `<盘符>:\<一级目录>`；
//! 3. **静态根不能落在 L3 红线里**（`C:\Windows\System32\*`、桌面 / 文档 / 浏览器
//!    Profile 等）。创建时就拒，而不是扫出来一堆删不掉的条目再让用户困惑。
//!
//! 第 3 条在**扫描时还会再过一遍**（`scan/modules/user_rules.rs` 逐条过滤）——
//! 创建时的检查针对「模式」，扫描时的检查针对「真实路径」，两者不能互相替代
//! （模式静态根合法，但通配符可能展开到子目录里的敏感文件）。
//!
//! # 为什么 glob 匹配器是自己写的
//!
//! 只支持三种元字符（`*` / `**` / `?`），引 `globset` 会多一个依赖、且它的
//! `**` 语义与这里要的「段内 vs 跨段」需要额外配置。自己写 60 行 + DP 表，
//! 行为完全可控、可单测。**DP 而不是朴素递归**：`*a*a*a*a*a*a*b` 这类模式
//! 朴素递归是指数级，DP 是 O(P×T)，最多几十万次比较，无所谓。

use std::path::PathBuf;

use rusqlite::{params, Connection};

/// 一条用户自定义规则。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRule {
    /// 自增主键。
    pub id: i64,
    /// 规则名（用户可见）。
    pub name: String,
    /// 路径模式（glob，可含 `%ENV%` / `~`）。
    pub pattern: String,
    /// `"L1"` / `"L2"`。
    pub risk_level: String,
    /// 是否启用。DB 里存 0/1，对外是 bool（前端直接绑 checkbox）。
    pub enabled: bool,
    /// 创建时间（unix 秒）。
    pub created_at: i64,
    /// 更新时间（unix 秒）。
    pub updated_at: i64,
}

/// 规则校验 / 存储错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuleError {
    /// 规则名为空。
    #[error("规则名不能为空")]
    EmptyName,
    /// 规则名过长（> 60 字符）。
    #[error("规则名过长（最多 60 个字符）")]
    NameTooLong,
    /// 模式为空。
    #[error("路径模式不能为空")]
    EmptyPattern,
    /// 模式过长（> 260 字符，Windows 路径上限）。
    #[error("路径模式过长（最多 260 个字符）")]
    PatternTooLong,
    /// 风险等级非法（含 L3）。
    #[error("风险等级只能是 L1 或 L2（L3 是红线等级，不允许作为清理规则）")]
    BadRiskLevel,
    /// 模式没有可用的静态根目录（如 `*` / `D:\**`）。
    #[error("路径模式太宽泛：必须指定到具体目录，如 D:\\MyCache\\*")]
    NoStaticRoot,
    /// 静态根落在 L3 红线内。
    #[error("路径模式指向受保护的位置（{0}），不允许作为清理规则")]
    DeniedPath(String),
    /// task-086 5.4：UNC 网络盘规则只允许 L2（删除前必须确认）。
    #[error("网络盘（UNC）路径只能使用 L2（需确认），不允许 L1 自动勾选")]
    UncRequiresL2,
    /// 同一模式已存在。
    #[error("该路径模式已存在（规则不能重复）")]
    Duplicate,
    /// 要更新 / 删除的规则不存在。
    #[error("规则不存在（id={0}）")]
    NotFound(i64),
    /// DB 错误。
    #[error("数据库错误: {0}")]
    Db(String),
}

/// 规则名长度上限（字符数）。
pub const MAX_NAME_CHARS: usize = 60;
/// 模式长度上限（字符数）。
pub const MAX_PATTERN_CHARS: usize = 260;

/// 把模式里的分隔符统一成 `\`、并做大小写无关比较前的归一。
///
/// Windows 上 `/` 与 `\` 等价，而用户在输入框里很可能写成 `D:/MyCache/*`。
/// 只在匹配时归一，**存库时保留用户原样**——用户看得懂自己写的东西。
fn norm_sep(s: &str) -> String {
    s.replace('/', "\\")
}

/// glob 匹配：`*` 段内任意、`**` 跨段任意、`?` 单个非分隔符字符。
///
/// 大小写不敏感（Windows 文件系统语义）。`pattern` 与 `text` 都应先经
/// [`norm_sep`]（本函数内部会再归一一次，调用方不必操心）。
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = norm_sep(pattern).to_lowercase().chars().collect();
    let t: Vec<char> = norm_sep(text).to_lowercase().chars().collect();

    // DP 表：memo[i][j] = pattern[i..] 能否匹配 text[j..]
    // 0 = 未算，1 = 能，2 = 不能
    let mut memo = vec![0u8; (p.len() + 1) * (t.len() + 1)];
    fn go(p: &[char], t: &[char], i: usize, j: usize, width: usize, memo: &mut Vec<u8>) -> bool {
        let key = i * width + j;
        if memo[key] != 0 {
            return memo[key] == 1;
        }
        let res = if i == p.len() {
            j == t.len()
        } else if p[i] == '*' {
            if i + 1 < p.len() && p[i + 1] == '*' {
                // `**`：可匹配零个或多个任意字符（含分隔符）。
                //
                // 特例：`**\` 要能匹配「零层目录」——`D:\a\**\*.tmp` 应该匹配
                // `D:\a\x.tmp`。做法是把紧随其后的分隔符一起跳过。
                //
                // **下标一律是 p 的绝对下标**（不能传子切片再配绝对下标，
                // 那样越界或错位）。
                let mut ok = false;
                if p.get(i + 2) == Some(&'\\') && go(p, t, i + 3, j, width, memo) {
                    ok = true;
                }
                if !ok {
                    for k in j..=t.len() {
                        if go(p, t, i + 2, k, width, memo) {
                            ok = true;
                            break;
                        }
                    }
                }
                ok
            } else {
                // `*`：匹配零个或多个**非分隔符**
                let mut ok = go(p, t, i + 1, j, width, memo);
                let mut k = j;
                while !ok && k < t.len() && t[k] != '\\' {
                    k += 1;
                    ok = go(p, t, i + 1, k, width, memo);
                }
                ok
            }
        } else if p[i] == '?' {
            j < t.len() && t[j] != '\\' && go(p, t, i + 1, j + 1, width, memo)
        } else {
            j < t.len() && t[j] == p[i] && go(p, t, i + 1, j + 1, width, memo)
        };
        memo[key] = if res { 1 } else { 2 };
        res
    }

    let width = t.len() + 1;
    go(&p, &t, 0, 0, width, &mut memo)
}

/// 模式的静态根目录：第一个通配符之前、最后一个分隔符处截断。
///
/// - `D:\MyCache\*` → `D:\MyCache`
/// - `D:\MyCache\**\*.tmp` → `D:\MyCache`
/// - `D:\My\Cache\a*.txt` → `D:\My\Cache`
/// - `D:\**` → `None`（没有静态根，等于扫整盘）
/// - `*.tmp` → `None`
///
/// 返回 `None` 时调用方必须**拒绝**这条规则（见 [`validate`]）。
pub fn static_root(pattern: &str) -> Option<PathBuf> {
    let s = norm_sep(pattern);
    let first_wild = s.find(['*', '?']).unwrap_or(s.len());
    let head = &s[..first_wild];
    // 截到最后一个分隔符（不含它），得到「最后一个完整目录」
    let cut = head.rfind('\\')?;
    let root = &head[..cut];
    if root.trim().is_empty() {
        return None;
    }

    // task-086 5.4：UNC 至少要 `<server>\<share>\<一级目录>`。
    // 只到 share 等于「遍历整个共享」，与 `D:\*` 同属太宽泛。
    if root.starts_with(r"\\") {
        let parts: Vec<&str> = root.trim_start_matches('\\').split('\\').collect();
        if parts.len() < 3
            || parts[0].trim().is_empty()
            || parts[1].trim().is_empty()
            || parts[2..].iter().any(|p| p.trim().is_empty())
        {
            return None;
        }
        return Some(PathBuf::from(root));
    }

    // 只认 `<字母>:\<非空>` 这种形态：`D:` 或 `D:\` 都算「太宽泛」
    let bytes = root.as_bytes();
    if bytes.len() < 4 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' {
        return None;
    }
    if root[2..].trim_matches('\\').is_empty() {
        return None;
    }
    Some(PathBuf::from(root))
}

/// 校验一条规则的三个字段。**所有写入路径都必须先过这里**。
pub fn validate(name: &str, pattern: &str, risk_level: &str) -> Result<(), RuleError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(RuleError::EmptyName);
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(RuleError::NameTooLong);
    }

    let pattern = pattern.trim();
    if pattern.is_empty() {
        return Err(RuleError::EmptyPattern);
    }
    if pattern.chars().count() > MAX_PATTERN_CHARS {
        return Err(RuleError::PatternTooLong);
    }

    // L3 = 红线，见模块文档。这里同时挡住拼写错误（"L3" / "l3" / "L4" 都拒）。
    let risk = risk_level.trim().to_ascii_uppercase();
    if risk != "L1" && risk != "L2" {
        return Err(RuleError::BadRiskLevel);
    }

    // 静态根：没有就拒（否则等于全盘遍历）
    let root = static_root(pattern).ok_or(RuleError::NoStaticRoot)?;

    // task-086 5.4：网络盘（UNC）只能 L2。
    //
    // 理由：UNC 路径无法通过盘符判断「是不是用户自己的数据」，删除又要走
    // copy+remove（跨设备、慢且可能中断）。强制 L2 让它**永远不会被「一键全选」**
    // 自动带上，用户必须在预览里逐项确认。
    if root.to_string_lossy().starts_with(r"\\") && risk != "L2" {
        return Err(RuleError::UncRequiresL2);
    }

    // 静态根不能落在 L3 红线里。用与执行链路**同一个**归一化 + 白名单，
    // 保证「创建时允许」与「执行时允许」是同一套判定，不会出现
    // 「建得进去、扫出来、删不掉」。
    let normalized = crate::execute::normalize::normalize(&root.to_string_lossy())
        .map_err(|e| RuleError::DeniedPath(format!("{root:?} 规范化失败: {e}")))?;
    if let Err(reason) = crate::execute::whitelist::classify(&normalized) {
        return Err(RuleError::DeniedPath(format!("{root:?} / {reason:?}")));
    }

    Ok(())
}

/// 列出全部规则（启用 + 停用），按 id 升序。
pub fn list(conn: &Connection) -> Result<Vec<UserRule>, RuleError> {
    query(
        conn,
        "SELECT id, name, pattern, risk_level, enabled, created_at, updated_at \
                 FROM user_rules ORDER BY id",
    )
}

/// 只列出启用的规则（扫描用）。
pub fn list_enabled(conn: &Connection) -> Result<Vec<UserRule>, RuleError> {
    query(
        conn,
        "SELECT id, name, pattern, risk_level, enabled, created_at, updated_at \
         FROM user_rules WHERE enabled = 1 ORDER BY id",
    )
}

fn query(conn: &Connection, sql: &str) -> Result<Vec<UserRule>, RuleError> {
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| RuleError::Db(e.to_string()))?;
    let rows = stmt
        .query_map([], |r| {
            Ok(UserRule {
                id: r.get(0)?,
                name: r.get(1)?,
                pattern: r.get(2)?,
                risk_level: r.get(3)?,
                // DB 里是 0/1，对外 bool
                enabled: r.get::<_, i64>(4)? != 0,
                created_at: r.get(5)?,
                updated_at: r.get(6)?,
            })
        })
        .map_err(|e| RuleError::Db(e.to_string()))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| RuleError::Db(e.to_string()))?);
    }
    Ok(out)
}

/// 按 id 取一条。
fn get(conn: &Connection, id: i64) -> Result<UserRule, RuleError> {
    conn.query_row(
        "SELECT id, name, pattern, risk_level, enabled, created_at, updated_at \
         FROM user_rules WHERE id = ?1",
        params![id],
        |r| {
            Ok(UserRule {
                id: r.get(0)?,
                name: r.get(1)?,
                pattern: r.get(2)?,
                risk_level: r.get(3)?,
                enabled: r.get::<_, i64>(4)? != 0,
                created_at: r.get(5)?,
                updated_at: r.get(6)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => RuleError::NotFound(id),
        other => RuleError::Db(other.to_string()),
    })
}

/// 新增一条规则，返回落库后的完整行（含 id 与时间戳）。
pub fn add(
    conn: &Connection,
    name: &str,
    pattern: &str,
    risk_level: &str,
) -> Result<UserRule, RuleError> {
    validate(name, pattern, risk_level)?;
    let pattern = pattern.trim();
    let risk = risk_level.trim().to_ascii_uppercase();

    conn.execute(
        "INSERT INTO user_rules (name, pattern, risk_level, enabled, created_at, updated_at)
         VALUES (?1, ?2, ?3, 1, unixepoch(), unixepoch())",
        params![name.trim(), pattern, risk],
    )
    .map_err(|e| {
        // 唯一索引冲突 → 明确的「模式重复」而不是笼统的 DB 错误
        let msg = e.to_string();
        if msg.contains("UNIQUE") {
            RuleError::Duplicate
        } else {
            RuleError::Db(msg)
        }
    })?;

    get(conn, conn.last_insert_rowid())
}

/// 更新一条规则（含启用开关），返回更新后的完整行。
pub fn update(
    conn: &Connection,
    id: i64,
    name: &str,
    pattern: &str,
    risk_level: &str,
    enabled: bool,
) -> Result<UserRule, RuleError> {
    validate(name, pattern, risk_level)?;
    let pattern = pattern.trim();
    let risk = risk_level.trim().to_ascii_uppercase();

    let n = conn
        .execute(
            "UPDATE user_rules
             SET name = ?1, pattern = ?2, risk_level = ?3, enabled = ?4, updated_at = unixepoch()
             WHERE id = ?5",
            params![name.trim(), pattern, risk, i64::from(enabled), id],
        )
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("UNIQUE") {
                RuleError::Duplicate
            } else {
                RuleError::Db(msg)
            }
        })?;

    if n == 0 {
        return Err(RuleError::NotFound(id));
    }
    get(conn, id)
}

/// 只切换启用状态（前端开关用，不动其它字段）。
///
/// 单独一个函数而不是复用 [`update`]：开关是高频、单字段的操作，
/// 复用 `update` 要求前端把 name / pattern / risk 一起回传——多传一个字段就多一个
/// 「用户改了名字但没保存就切开关，名字被旧值覆盖」的机会。
pub fn set_enabled(conn: &Connection, id: i64, enabled: bool) -> Result<UserRule, RuleError> {
    let n = conn
        .execute(
            "UPDATE user_rules SET enabled = ?1, updated_at = unixepoch() WHERE id = ?2",
            params![i64::from(enabled), id],
        )
        .map_err(|e| RuleError::Db(e.to_string()))?;
    if n == 0 {
        return Err(RuleError::NotFound(id));
    }
    get(conn, id)
}

/// 删除一条规则。返回是否真的删掉（false = 本来就不存在）。
pub fn delete(conn: &Connection, id: i64) -> Result<bool, RuleError> {
    let n = conn
        .execute("DELETE FROM user_rules WHERE id = ?1", params![id])
        .map_err(|e| RuleError::Db(e.to_string()))?;
    Ok(n > 0)
}

// ===================== task-084 B4：导入 / 导出 JSON =====================
//
// # 为什么还要一层「格式标记」
//
// 方案模板（`plan.rs`）已经有一套 export/import，但那是给「清理方案」用的、
// 带 `cclean-plan` 标记。用户规则是**不同的东西**，导入侧必须能区分——
// 把一份规则 JSON 误当方案导入，或反过来，都只会得到一条「看起来对了但毫无
// 作用」的空记录。所以这里用自己的 `cclean-user-rules` 标记，与方案互不相认。
//
// # 冲突合并策略（`ConflictPolicy`）
//
// `user_rules` 的 pattern 有唯一索引（`idx_user_rules_pattern`），导入时撞车是
// 常态（用户导出后改了名字再导回，或两台机器合并）。两种策略：
// - `Skip`：撞车的整条跳过，保留库里的原样（保守，永远不会覆盖用户已有数据）；
// - `Overwrite`：用文件里的新值覆盖同名（同 pattern）的旧记录。
//
// 无论哪种，新增的条目仍走 [`add`] → 完整校验（L1/L2、静态根、红线），
// 不合格的条目不会被「跳过冲突」绕过。

/// 导入导出文件的格式标记。导入时校验，防止把方案或其他 JSON 当规则导进来。
pub const USER_RULE_FORMAT: &str = "cclean-user-rules";
/// 导出文件格式版本。字段不兼容时递增，导入侧拒绝未知版本。
pub const USER_RULE_FORMAT_VERSION: u32 = 1;

/// 一条规则的导出形态（去掉 `id` / 时间戳等库内字段）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleFileEntry {
    pub name: String,
    pub pattern: String,
    /// `"L1"` / `"L2"`。
    pub risk_level: String,
    pub enabled: bool,
}

/// 导出/导入用的文件内容。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleFile {
    pub format: String,
    pub version: u32,
    /// 导出时间（unix 秒），仅作说明，导入时忽略。
    /// `serde(default)`：手工构造 / 旧版文件缺该字段按 0 处理，不因这个说明性
    /// 字段拒绝整份导入——否则无法走到下面真正的 format 校验。
    #[serde(default)]
    pub exported_at: i64,
    /// `serde(default)`：缺该字段按空列表处理，先用 format/version 决定接受与否，
    /// 不因一个空的规则数组拒绝整个文件。
    #[serde(default)]
    pub rules: Vec<RuleFileEntry>,
}

/// 导入冲突策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy {
    /// 撞车（pattern 已存在）整条跳过。
    Skip,
    /// 用文件里的新值覆盖已存在的同名记录。
    Overwrite,
}

/// 导入结果汇总。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    /// 新写入的条数。
    pub added: usize,
    /// 因冲突被跳过的条数。
    pub skipped: usize,
    /// 因冲突被覆盖的条数。
    pub overwritten: usize,
    /// 校验失败被拒的条数（错误明细在 `errors`）。
    pub rejected: usize,
    /// 每条被拒原因（`第 N 条：原因`）。
    pub errors: Vec<String>,
}

/// 从 lib 层「读全部规则」后导出为一个 JSON 字符串。
///
/// 与 `plan::export_plan` 的分工一致：数据库读写 + 序列化在 engine 完成，
/// 落盘路径由调用方（Tauri 命令层）传入 `export_dir`。返回可写盘的文件内容。
pub fn export_to_json(conn: &Connection, exported_at: i64) -> Result<String, RuleError> {
    let rules = list(conn)?;
    let entries: Vec<RuleFileEntry> = rules
        .into_iter()
        .map(|r| RuleFileEntry {
            name: r.name,
            pattern: r.pattern,
            risk_level: r.risk_level,
            enabled: r.enabled,
        })
        .collect();
    let file = RuleFile {
        format: USER_RULE_FORMAT.to_string(),
        version: USER_RULE_FORMAT_VERSION,
        exported_at,
        rules: entries,
    };
    serde_json::to_string_pretty(&file)
        .map_err(|e| RuleError::Db(format!("规则序列化为 JSON 失败: {e}")))
}

/// 从 JSON 字符串导入规则，按 [`ConflictPolicy`] 合并。
///
/// - 文件头 `format` / `version` 不符 → 整个拒绝（不是部分导入）；
/// - 每条规则仍走 [`add`] 的完整校验，`errors` 记录被拒明细；
/// - 冲突判定用 pattern（唯一索引所在列）：库里已有同 pattern → 跳过或覆盖。
pub fn import_from_json(
    conn: &Connection,
    json: &str,
    policy: ConflictPolicy,
) -> Result<ImportResult, RuleError> {
    let file: RuleFile = serde_json::from_str(json)
        .map_err(|e| RuleError::Db(format!("规则 JSON 解析失败: {e}")))?;
    if file.format != USER_RULE_FORMAT {
        return Err(RuleError::Db(format!(
            "不是用户规则文件（format={:?}），请导出后再导入或选对文件",
            file.format
        )));
    }
    if file.version > USER_RULE_FORMAT_VERSION {
        return Err(RuleError::Db(format!(
            "规则文件版本过新（v{}，当前支持 v{}），请升级后再导入",
            file.version, USER_RULE_FORMAT_VERSION
        )));
    }

    // 一次取出现有 pattern 集合，冲突判定 O(1)。用小写归一化而非原样比较：
    // Windows 路径大小写不敏感，`D:\MyCache` 与 `d:\mycache` 撞的是同一条规则。
    let existing = existing_patterns_lower(conn)?;

    let mut result = ImportResult {
        added: 0,
        skipped: 0,
        overwritten: 0,
        rejected: 0,
        errors: Vec::new(),
    };

    for (idx, entry) in file.rules.iter().enumerate() {
        let no = idx + 1;
        // 名称/风险等级允许 trim；pattern 不得被改为别的意思trim不能动斜杠，只 trim 首尾空白。
        let pattern = entry.pattern.trim();
        let key = pattern.to_ascii_lowercase();
        if existing.contains(&key) {
            match policy {
                ConflictPolicy::Skip => result.skipped += 1,
                ConflictPolicy::Overwrite => {
                    // 库里已有同 pattern 的规则 → 用文件里的新值更新它。
                    // 需取到那条记录的 id；用 pattern 精确查询（唯一索引保底）。
                    match update_by_pattern(
                        conn,
                        pattern,
                        &entry.name,
                        &entry.risk_level,
                        entry.enabled,
                    ) {
                        Ok(()) => {
                            result.overwritten += 1;
                        }
                        Err(e) => {
                            result.rejected += 1;
                            result.errors.push(format!("第 {no} 条覆盖失败: {e}"));
                        }
                    }
                }
            }
            // 无论覆盖成功与否，该 pattern 仍占用，无需重复 add
            continue;
        }
        match add(conn, &entry.name, pattern, &entry.risk_level) {
            Ok(added) => {
                // add 成功后启用状态恒为 true；若文件里要求停用，切换一下。
                if !entry.enabled {
                    let _ = set_enabled(conn, added.id, false);
                }
                // 已用 match 绑定 added，避免在 Ok(_) 里再做一次查询
                let _ = added;
                result.added += 1;
            }
            Err(e) => {
                result.rejected += 1;
                result
                    .errors
                    .push(format!("第 {no} 条（{}）被拒: {e}", entry.name));
            }
        }
    }
    Ok(result)
}

/// 取现有全部 pattern 的小写集合（导入冲突判定用）。
fn existing_patterns_lower(
    conn: &Connection,
) -> Result<std::collections::HashSet<String>, RuleError> {
    let mut stmt = conn
        .prepare("SELECT pattern FROM user_rules")
        .map_err(|e| RuleError::Db(e.to_string()))?;
    let mut set = std::collections::HashSet::new();
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| RuleError::Db(e.to_string()))?;
    for row in rows {
        set.insert(
            row.map_err(|e| RuleError::Db(e.to_string()))?
                .to_ascii_lowercase(),
        );
    }
    Ok(set)
}

/// 按 pattern（唯一索引列）精确更新一条规则。返回更新行数（0 = 不存在）。
///
/// 复用 [`update`] 的校验逻辑——先把库里那条读出来拿 id，再走 `update`。
fn update_by_pattern(
    conn: &Connection,
    pattern: &str,
    name: &str,
    risk_level: &str,
    enabled: bool,
) -> Result<(), RuleError> {
    let id: i64 = conn
        .query_row(
            "SELECT id FROM user_rules WHERE pattern = ?1",
            params![pattern],
            |r| r.get(0),
        )
        .map_err(|_| RuleError::NotFound(0))?;
    update(conn, id, name, pattern, risk_level, enabled).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrate::migrate(&conn).unwrap();
        conn
    }

    // ===== glob 匹配 =====

    #[test]
    fn glob_single_star_stays_within_one_segment() {
        assert!(glob_match(r"D:\MyCache\*", r"D:\MyCache\a.tmp"));
        // `*` 不跨目录：这是「只清这一层」的语义，跨层必须写 `**`
        assert!(!glob_match(r"D:\MyCache\*", r"D:\MyCache\sub\a.tmp"));
        // 目录本身（不带尾分隔符）不该被 `\*` 匹配上——少一个 `\` 就是另一种意思
        assert!(!glob_match(r"D:\MyCache\*", r"D:\MyCache"));
        // `*` 匹配「零个字符」是标准语义：带尾分隔符时能对上
        assert!(glob_match(r"D:\MyCache\*", r"D:\MyCache\"));
    }

    #[test]
    fn glob_double_star_crosses_segments() {
        assert!(glob_match(r"D:\MyCache\**\*.tmp", r"D:\MyCache\a.tmp"));
        assert!(glob_match(r"D:\MyCache\**\*.tmp", r"D:\MyCache\a\b\c.tmp"));
        // 扩展名不符不匹配
        assert!(!glob_match(r"D:\MyCache\**\*.tmp", r"D:\MyCache\a\log"));
        // 根目录前缀必须一致
        assert!(!glob_match(r"D:\MyCache\**", r"D:\Other\a.tmp"));
    }

    #[test]
    fn glob_is_case_insensitive_and_separator_agnostic() {
        // Windows 语义：大小写不敏感 + `/` 与 `\` 等价
        assert!(glob_match(r"d:\mycache\*", r"D:\MyCache\A.TMP"));
        assert!(glob_match(r"D:/MyCache/*", r"D:\MyCache\a.tmp"));
        assert!(glob_match(r"D:\MyCache\*", r"D:/MyCache/a.tmp"));
    }

    #[test]
    fn glob_question_mark_is_one_non_separator_char() {
        assert!(glob_match(r"D:\c\a?c", r"D:\c\abc"));
        assert!(!glob_match(r"D:\c\a?c", r"D:\c\ac"));
        // `?` 不匹配分隔符
        assert!(!glob_match(r"D:\c\a?c", r"D:\c\a\c"));
    }

    #[test]
    fn glob_does_not_blow_up_on_pathological_pattern() {
        // 朴素递归在这里是指数级；DP 是 O(P×T)。跑不通就是实现退化了。
        let pat = r"D:\x\*a*a*a*a*a*a*a*a*a*a*b";
        let text = r"D:\x\aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaac";
        assert!(!glob_match(pat, text));
    }

    // ===== 静态根 =====

    #[test]
    fn static_root_truncates_at_last_separator_before_wildcard() {
        assert_eq!(
            static_root(r"D:\MyCache\*"),
            Some(PathBuf::from(r"D:\MyCache"))
        );
        assert_eq!(
            static_root(r"D:\MyCache\**\*.tmp"),
            Some(PathBuf::from(r"D:\MyCache"))
        );
        // 通配符在文件名中间：根仍是所在目录
        assert_eq!(
            static_root(r"D:\My\Cache\a*.txt"),
            Some(PathBuf::from(r"D:\My\Cache"))
        );
        // 正斜杠也要认
        assert_eq!(
            static_root("D:/MyCache/*"),
            Some(PathBuf::from(r"D:\MyCache"))
        );
    }

    #[test]
    fn static_root_rejects_too_broad_patterns() {
        // 这四条都等于「扫整盘」或「没有根」，必须返回 None 被拒
        for p in [
            r"D:\**",
            r"D:\*",
            r"*.tmp",
            r"**\*.tmp",
            r"\Windows\*",
            r"D:",
        ] {
            assert_eq!(static_root(p), None, "{p} 不该有静态根");
        }
    }

    // ===== task-086 5.4：UNC 规则 =====

    #[test]
    fn static_root_accepts_unc_with_one_subdir() {
        assert_eq!(
            static_root(r"\\nas\backup\Games\*"),
            Some(PathBuf::from(r"\\nas\backup\Games"))
        );
        // 只到共享根太宽泛
        assert_eq!(static_root(r"\\nas\backup\*"), None);
        assert_eq!(static_root(r"\\nas\*"), None);
    }

    #[test]
    fn validate_forces_l2_for_unc_rules() {
        // L1 会让网络盘条目进入「一键全选」，必须拒
        assert_eq!(
            validate("网络缓存", r"\\nas\backup\Games\*", "L1"),
            Err(RuleError::UncRequiresL2)
        );
        assert!(validate("网络缓存", r"\\nas\backup\Games\*", "L2").is_ok());
        // 红线仍然生效：指向共享里的个人目录要拒
        assert!(matches!(
            validate("坏规则", r"\\nas\backup\Users\alice\Desktop\*", "L2"),
            Err(RuleError::DeniedPath(_))
        ));
    }

    // ===== 校验 =====

    #[test]
    fn validate_rejects_bad_input() {
        assert_eq!(validate("", r"D:\a\*", "L1"), Err(RuleError::EmptyName));
        assert_eq!(
            validate(&"名".repeat(61), r"D:\a\*", "L1"),
            Err(RuleError::NameTooLong)
        );
        assert_eq!(validate("n", "", "L1"), Err(RuleError::EmptyPattern));
        // L3 是红线等级，必须拒；拼写错误也一并拒
        for bad in ["L3", "l3", "L0", "L4", "", "high"] {
            assert_eq!(
                validate("n", r"D:\a\*", bad),
                Err(RuleError::BadRiskLevel),
                "{bad:?} 应被拒"
            );
        }
        assert_eq!(validate("n", r"D:\**", "L1"), Err(RuleError::NoStaticRoot));
    }

    #[test]
    fn validate_accepts_l1_and_l2() {
        assert!(validate("我的缓存", r"D:\MyCache\*", "L1").is_ok());
        assert!(validate("我的缓存", r"D:\MyCache\*", "l2").is_ok());
    }

    #[test]
    fn validate_rejects_l3_protected_locations() {
        // 静态根落在红线里 → 建的时候就拒，而不是扫出一堆删不掉的条目
        for p in [
            r"C:\Windows\System32\*",
            r"C:\Windows\Installer\**",
            r"C:\Users\An\Desktop\*",
            r"C:\Users\An\Documents\**",
        ] {
            match validate("bad", p, "L1") {
                Err(RuleError::DeniedPath(_)) => {}
                other => panic!("{p} 应被拒为 DeniedPath，实际 {other:?}"),
            }
        }
    }

    // ===== CRUD =====

    #[test]
    fn crud_round_trip() {
        let conn = db();

        let r = add(&conn, "缓存A", r"D:\MyCache\*", "L1").unwrap();
        assert!(r.id > 0);
        assert!(r.enabled, "新建默认启用");
        assert_eq!(r.risk_level, "L1");
        assert!(r.created_at > 0);

        // 列表
        assert_eq!(list(&conn).unwrap().len(), 1);

        // 更新（含关闭）
        let u = update(&conn, r.id, "缓存A2", r"D:\MyCache\**", "L2", false).unwrap();
        assert_eq!(u.name, "缓存A2");
        assert_eq!(u.risk_level, "L2");
        assert!(!u.enabled);

        // 只有启用的才算「扫描用」
        assert!(list_enabled(&conn).unwrap().is_empty());
        set_enabled(&conn, r.id, true).unwrap();
        assert_eq!(list_enabled(&conn).unwrap().len(), 1);

        // 删除
        assert!(delete(&conn, r.id).unwrap());
        assert!(!delete(&conn, r.id).unwrap(), "重复删除应返回 false");
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn duplicate_pattern_is_rejected() {
        let conn = db();
        add(&conn, "a", r"D:\Same\*", "L1").unwrap();
        // 名字不同、模式相同 → 拒（否则同一批文件被扫两遍）
        assert_eq!(
            add(&conn, "b", r"D:\Same\*", "L1"),
            Err(RuleError::Duplicate)
        );
        // 名字相同、模式不同 → 允许
        assert!(add(&conn, "a", r"D:\Other\*", "L1").is_ok());
    }

    #[test]
    fn update_and_set_enabled_report_missing_id() {
        let conn = db();
        assert_eq!(
            update(&conn, 999, "n", r"D:\a\*", "L1", true),
            Err(RuleError::NotFound(999))
        );
        assert_eq!(set_enabled(&conn, 999, true), Err(RuleError::NotFound(999)));
    }

    #[test]
    fn add_rejects_invalid_without_writing() {
        let conn = db();
        assert!(add(&conn, "n", r"D:\**", "L1").is_err());
        assert!(list(&conn).unwrap().is_empty(), "校验失败不该落库");
    }

    // ===== task-084 B4：导入 / 导出 JSON =====

    #[test]
    fn export_to_json_round_trips_all_rules() {
        let conn = db();
        add(&conn, "缓存A", r"D:\MyCache\*", "L1").unwrap();
        let b = add(&conn, "缓存B", r"D:\Other\**", "L2").unwrap();
        set_enabled(&conn, b.id, false).unwrap();

        let json = export_to_json(&conn, 123).unwrap();
        let file: RuleFile = serde_json::from_str(&json).unwrap();
        assert_eq!(file.format, USER_RULE_FORMAT);
        assert_eq!(file.version, USER_RULE_FORMAT_VERSION);
        assert_eq!(file.exported_at, 123);
        assert_eq!(file.rules.len(), 2);
        // 名称/模式/等级启用都要带上（id 与时间戳不该出现）
        let a = file
            .rules
            .iter()
            .find(|r| r.pattern == r"D:\MyCache\*")
            .unwrap();
        assert_eq!(a.name, "缓存A");
        assert_eq!(a.risk_level, "L1");
        assert!(a.enabled);
        let bb = file
            .rules
            .iter()
            .find(|r| r.pattern == r"D:\Other\**")
            .unwrap();
        assert!(!bb.enabled, "停用状态要随导出带走");
    }

    #[test]
    fn export_empty_rules_yields_empty_file() {
        let conn = db();
        let json = export_to_json(&conn, 0).unwrap();
        let file: RuleFile = serde_json::from_str(&json).unwrap();
        assert!(file.rules.is_empty(), "空规则库导出也该是合法文件");
    }

    #[test]
    fn import_adds_new_rules_preserving_enabled() {
        let conn = db();
        let json = r#"{
            "format": "cclean-user-rules",
            "version": 1,
            "exportedAt": 1,
            "rules": [
                {"name": "N1", "pattern": "D:\\A\\*", "riskLevel": "L1", "enabled": true},
                {"name": "N2", "pattern": "D:\\B\\*", "riskLevel": "L2", "enabled": false}
            ]
        }"#;
        let r = import_from_json(&conn, json, ConflictPolicy::Skip).unwrap();
        assert_eq!(r.added, 2);
        assert_eq!(r.skipped, 0);
        assert_eq!(r.rejected, 0);

        let all = list(&conn).unwrap();
        assert_eq!(all.len(), 2);
        let n2 = all.iter().find(|x| x.name == "N2").unwrap();
        assert!(!n2.enabled, "文件里 enabled=false 要落到库");
        let n1 = all.iter().find(|x| x.name == "N1").unwrap();
        assert!(n1.enabled);
    }

    #[test]
    fn import_skip_keeps_existing() {
        let conn = db();
        add(&conn, "旧名", r"D:\Same\*", "L1").unwrap();

        let json = r#"{
            "format": "cclean-user-rules",
            "version": 1,
            "exportedAt": 1,
            "rules": [
                {"name": "新名", "pattern": "D:\\Same\\*", "riskLevel": "L2", "enabled": false}
            ]
        }"#;
        let r = import_from_json(&conn, json, ConflictPolicy::Skip).unwrap();
        assert_eq!(r.added, 0);
        assert_eq!(r.skipped, 1, "撞车 → 跳过");

        let all = list(&conn).unwrap();
        assert_eq!(all.len(), 1);
        let old = &all[0];
        assert_eq!(old.name, "旧名", "Skip 策略必须保留库里原样");
        assert_eq!(old.risk_level, "L1");
        assert!(old.enabled);
    }

    #[test]
    fn import_overwrite_updates_existing() {
        let conn = db();
        add(&conn, "旧名", r"D:\Same\*", "L1").unwrap();

        let json = r#"{
            "format": "cclean-user-rules",
            "version": 1,
            "exportedAt": 1,
            "rules": [
                {"name": "新名", "pattern": "D:\\Same\\*", "riskLevel": "L2", "enabled": true}
            ]
        }"#;
        let r = import_from_json(&conn, json, ConflictPolicy::Overwrite).unwrap();
        assert_eq!(r.added, 0);
        assert_eq!(r.overwritten, 1, "撞车 → 覆盖");

        let all = list(&conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "新名", "Overwrite 策略要写入文件里的新值");
        assert_eq!(all[0].risk_level, "L2");
    }

    #[test]
    fn import_rejects_wrong_format() {
        let conn = db();
        // 与方案文件区分的关键：format 不是 cclean-user-rules 必须整个拒绝
        let json = r#"{"format": "cclean-plan", "version": 1, "name": "x", "itemRefs": []}"#;
        let e = import_from_json(&conn, json, ConflictPolicy::Skip).unwrap_err();
        assert!(
            matches!(e, RuleError::Db(_)) && e.to_string().contains("不是用户规则文件"),
            "实际: {e:?}"
        );
    }

    #[test]
    fn import_rejects_unknown_version() {
        let conn = db();
        let json = r#"{
            "format": "cclean-user-rules",
            "version": 999,
            "exportedAt": 1,
            "rules": []
        }"#;
        let e = import_from_json(&conn, json, ConflictPolicy::Skip).unwrap_err();
        assert!(e.to_string().contains("版本过新"), "实际: {e:?}");
    }

    #[test]
    fn import_rejects_invalid_entries_without_partial_write() {
        let conn = db();
        // 一条合法 + 一条「模式太宽泛」：合法的要写入，非法的记进 errors
        let json = r#"{
            "format": "cclean-user-rules",
            "version": 1,
            "exportedAt": 1,
            "rules": [
                {"name": "好", "pattern": "D:\\Good\\*", "riskLevel": "L1", "enabled": true},
                {"name": "坏", "pattern": "D:\\**", "riskLevel": "L1", "enabled": true}
            ]
        }"#;
        let r = import_from_json(&conn, json, ConflictPolicy::Skip).unwrap();
        assert_eq!(r.added, 1);
        assert_eq!(r.rejected, 1);
        assert_eq!(r.errors.len(), 1);
        assert!(
            r.errors[0].contains("坏"),
            "错误的明细要指向被拒条目: {:?}",
            r.errors
        );
        assert_eq!(list(&conn).unwrap().len(), 1, "只有合法条目落库");
    }

    #[test]
    fn import_conflict_is_case_insensitive() {
        let conn = db();
        add(&conn, "原样", r"D:\Same\*", "L1").unwrap();
        // 文件里写成小写盘符，应仍判定为撞车（Windows 路径大小写不敏感）
        let json = r#"{
            "format": "cclean-user-rules",
            "version": 1,
            "exportedAt": 1,
            "rules": [
                {"name": "小写", "pattern": "d:\\same\\*", "riskLevel": "L1", "enabled": true}
            ]
        }"#;
        let r = import_from_json(&conn, json, ConflictPolicy::Skip).unwrap();
        assert_eq!(r.skipped, 1, "大小写不同也算同一条规则");
        assert_eq!(list(&conn).unwrap().len(), 1);
    }

    #[test]
    fn import_and_export_round_trip() {
        // 导出 → 清库 → 导入 → 内容一致，端到端验证格式自洽
        let conn = db();
        add(&conn, "A", r"D:\MyCache\*", "L1").unwrap();
        let b = add(&conn, "B", r"D:\Other\**", "L2").unwrap();
        set_enabled(&conn, b.id, false).unwrap();

        let json = export_to_json(&conn, 1).unwrap();
        let clean = db();
        let r = import_from_json(&clean, &json, ConflictPolicy::Skip).unwrap();
        assert_eq!(r.added, 2);
        assert_eq!(r.rejected, 0);

        let src = list(&conn).unwrap();
        let dst = list(&clean).unwrap();
        assert_eq!(src.len(), dst.len());
        assert_eq!(
            src.iter()
                .map(|x| (&x.pattern, &x.risk_level, x.enabled))
                .collect::<Vec<_>>(),
            dst.iter()
                .map(|x| (&x.pattern, &x.risk_level, x.enabled))
                .collect::<Vec<_>>()
        );
    }
}
