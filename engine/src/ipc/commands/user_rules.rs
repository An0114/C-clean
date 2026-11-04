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
}
