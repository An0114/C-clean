//! `c-clean-cli`：C-Disk Cleaner 的命令行入口（task-079，P2 收官）。
//!
//! # 为什么 CLI 不依赖 Tauri
//!
//! 扫描、清理、白名单、保险箱全在 `engine` 里，Tauri 只是它的一个前端。CLI 直接
//! `use engine::...`，`cargo build -p c-clean-cli` 出来的二进制不带 WebView2、不带
//! 窗口、不带托盘 —— 这正是「便携版核心」要的东西：**解压即用，双击或命令行都能跑**。
//!
//! # 数据目录怎么定（三档，优先级从高到低）
//!
//! 1. `--data-dir <PATH>` 显式指定；
//! 2. 环境变量 `CCLEAN_DATA_DIR`；
//! 3. **便携布局**：`<exe 所在目录>/portable_data` 存在就用它 —— 便携版 zip 里预置
//!    这个空目录，数据就永远跟着 exe 走，换台机器拷走整个文件夹即可；
//! 4. 兜底：`%APPDATA%\com.and0114.cclean`（与 GUI 安装版**同一个库**）。
//!
//! 第 3 档为什么要判「目录存在」而不是「exe 目录可写」：可写性探测会真的落盘一个
//! 临时文件，在 Program Files 下还会弹一次 UAC 失败提示。存在性判断零副作用，
//! 而便携版是我们自己打的包，目录一定在。
//!
//! # 退出码
//!
//! | 码 | 含义 |
//! |---|---|
//! | 0 | 成功（`clean` 有失败条目也返回 0——失败明细在输出里，脚本按 `failed` 字段判断） |
//! | 1 | 运行期错误（DB 打不开 / 扫描失败 / 清理异常） |
//! | 2 | 用法错误（未知子命令、未知开关、缺参数值） |
//!
//! 用法错误单独用 2，是因为脚本里最常见的分支就是「我参数写错了」，与「跑起来但
//! 失败了」必须能区分——很多 CLI 把两者都返回 1，调用方只能靠 stderr 猜。
//!
//! # 安全边界（与 GUI 完全一致，一行都没绕过）
//!
//! - `clean` **默认是 dry-run**，必须显式 `--yes` 才动文件；
//! - 默认只清 **L1**（安全缓存），L2 需再加 `--include-l2`。这与 GUI「全选 L1」
//!   的取舍一致：L2 是「需逐项确认」的等级，命令行里没有逐项确认的界面，
//!   所以要求用户额外显式声明；
//! - 删除走 `execute_items` + `SafeBoxDeleter`：**先移进保险箱**（可恢复），
//!   不是直接抹掉。白名单 / L3 红线 / 审计日志全部复用 GUI 那条唯一入口；
//! - 不做系统还原点（那是 GUI 对 L2 批次的交互式流程，需要提权 exe 与 UAC 确认，
//!   命令行里没法弹窗）。**所以 `--include-l2` 时会明确警告**。

use std::path::{Path, PathBuf};

/// 成功。
const EXIT_OK: i32 = 0;
/// 运行期错误。
const EXIT_ERR: i32 = 1;
/// 用法错误。
const EXIT_USAGE: i32 = 2;

/// 输出格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// 人类可读的表格（默认）。
    Table,
    /// 机器可读的 JSON（供脚本消费）。
    Json,
}

/// 子命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    /// 扫描并列出可清理项（摘要 + 前 N 条）。
    Scan,
    /// 扫描并列出全部条目（比 scan 详细）。
    DryRun,
    /// 扫描 + 清理。`yes=false` 时等同于 dry-run。
    Clean { yes: bool, include_l2: bool },
    /// 列出内置规则 + 用户自定义规则。
    ListRules,
    /// 帮助。
    Help,
    /// 版本。
    Version,
}

/// 解析后的命令行选项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub cmd: Cmd,
    pub format: Format,
    /// 限定盘符（`None` = 全部固定盘）。
    pub drives: Option<Vec<char>>,
    /// 显式数据目录。
    pub data_dir: Option<PathBuf>,
    /// `scan` 最多打印多少条（`dry-run` 忽略它）。
    pub limit: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            cmd: Cmd::Help,
            format: Format::Table,
            drives: None,
            data_dir: None,
            limit: 50,
        }
    }
}

/// 用法文本（`--help` 与用法错误时打印同一份，避免两处描述漂移）。
pub const USAGE: &str = "\
c-clean-cli — C-Disk Cleaner 命令行（便携版核心）

用法:
  c-clean-cli <子命令> [选项]

子命令:
  scan          扫描并列出可清理项（摘要 + 前 N 条）
  dry-run       扫描并列出全部条目（不删除）
  clean         扫描 + 清理；默认只预览，加 --yes 才真的移入保险箱
  list-rules    列出内置规则与用户自定义规则
  help          显示本帮助

选项:
  --format <table|json>   输出格式（默认 table）
  --drive <C,D>           只扫指定盘（逗号分隔；默认全部固定盘）
  --data-dir <PATH>       数据目录（默认 %APPDATA%\\com.and0114.cclean，
                          便携版为 exe 同目录的 portable_data）
  --limit <N>             scan 打印条数上限（默认 50）
  --yes                   clean 时真的执行（否则只预览）
  --include-l2            clean 时连 L2 一起清（默认只清 L1 安全项）
  -h, --help              显示帮助
  -V, --version           显示版本

示例:
  c-clean-cli scan
  c-clean-cli scan --format json --limit 20
  c-clean-cli clean --yes
  c-clean-cli clean --yes --include-l2 --drive C
  c-clean-cli list-rules --format json

退出码: 0 成功 / 1 运行期错误 / 2 用法错误
";

/// 解析 `--drive` 的值（`C` / `c` / `C:` 都认，逗号分隔）。
///
/// 与 `scan_start::parse_drive` 同一套宽容度：前端下拉框给 `C:`、文档写 `C`，
/// 命令行用户两种都会打。**不认 `CC` / `1` / 空项**——静默忽略会让用户以为
/// 「只扫了 C 盘」而实际扫了全部。
fn parse_drives(raw: &str) -> Result<Vec<char>, String> {
    let mut out: Vec<char> = Vec::new();
    for part in raw.split(',') {
        let s = part.trim();
        if s.is_empty() {
            return Err(format!("--drive 里有空项: {raw:?}"));
        }
        let letter = s.strip_suffix(':').unwrap_or(s);
        let mut chars = letter.chars();
        let c = chars
            .next()
            .ok_or_else(|| format!("--drive 盘符为空: {raw:?}"))?;
        if chars.next().is_some() || !c.is_ascii_alphabetic() {
            return Err(format!("--drive 盘符非法: {s:?}（应形如 C 或 C:）"));
        }
        let up = c.to_ascii_uppercase();
        if !out.contains(&up) {
            out.push(up);
        }
    }
    Ok(out)
}

/// 解析参数。**纯函数**（不含 `std::env::args`），可直接单测。
pub fn parse_args(argv: &[String]) -> Result<Options, String> {
    let mut opt = Options::default();
    let mut i = 1usize; // 跳过 argv[0]（程序名）
    let mut cmd: Option<Cmd> = None;
    let mut clean_yes = false;
    let mut clean_l2 = false;

    while i < argv.len() {
        let a = argv[i].as_str();
        match a {
            "-h" | "--help" => {
                cmd = Some(Cmd::Help);
                i += 1;
            }
            "-V" | "--version" => {
                cmd = Some(Cmd::Version);
                i += 1;
            }
            "--yes" => {
                clean_yes = true;
                i += 1;
            }
            "--include-l2" => {
                clean_l2 = true;
                i += 1;
            }
            "--format" => {
                let v = argv
                    .get(i + 1)
                    .ok_or("--format 需要一个值（table / json）")?;
                opt.format = match v.as_str() {
                    "table" => Format::Table,
                    "json" => Format::Json,
                    other => return Err(format!("--format 只支持 table / json，收到 {other:?}")),
                };
                i += 2;
            }
            "--drive" => {
                let v = argv.get(i + 1).ok_or("--drive 需要一个值（如 C,D）")?;
                opt.drives = Some(parse_drives(v)?);
                i += 2;
            }
            "--data-dir" => {
                let v = argv.get(i + 1).ok_or("--data-dir 需要一个路径")?;
                opt.data_dir = Some(PathBuf::from(v));
                i += 2;
            }
            "--limit" => {
                let v = argv.get(i + 1).ok_or("--limit 需要一个数字")?;
                opt.limit = v
                    .parse::<usize>()
                    .map_err(|_| format!("--limit 需要一个非负整数，收到 {v:?}"))?;
                i += 2;
            }
            other if other.starts_with('-') => {
                return Err(format!("未知选项: {other}"));
            }
            other => {
                // 子命令只认一个；重复给就报错（`scan clean` 这种笔误不能静默取第一个）
                if cmd.is_some() {
                    return Err(format!("只能指定一个子命令，多余的: {other}"));
                }
                cmd = Some(match other {
                    "scan" => Cmd::Scan,
                    "dry-run" => Cmd::DryRun,
                    "clean" => Cmd::Clean {
                        yes: false, // 先占位，循环结束后按开关回填
                        include_l2: false,
                    },
                    "list-rules" => Cmd::ListRules,
                    "help" => Cmd::Help,
                    unknown => return Err(format!("未知子命令: {unknown}")),
                });
                i += 1;
            }
        }
    }

    opt.cmd = match cmd {
        // `--yes` / `--include-l2` 只对 clean 有意义。给别的子命令加这两个开关是
        // 笔误（例如把 `clean --yes` 写成 `scan --yes`），静默忽略会让用户以为
        // 文件被清了。直接报错。
        Some(Cmd::Clean { .. }) => Cmd::Clean {
            yes: clean_yes,
            include_l2: clean_l2,
        },
        Some(other) => {
            if clean_yes || clean_l2 {
                return Err("--yes / --include-l2 只能与 clean 一起用".to_string());
            }
            other
        }
        // 不给子命令 = 打帮助（不做任何扫描）。这是最不容易出事的默认行为。
        None => {
            if clean_yes || clean_l2 {
                return Err("--yes / --include-l2 只能与 clean 一起用".to_string());
            }
            Cmd::Help
        }
    };
    Ok(opt)
}

/// 选数据目录。**纯函数**（入参全是显式值），便于单测三档优先级。
///
/// `portable_exists` 由调用方用 `Path::is_dir()` 探测后传入——把「探测」与
/// 「决策」分开，测试就不用真的造 exe 目录。
fn pick_data_root(
    cli: Option<&Path>,
    env: Option<&str>,
    exe_dir: &Path,
    appdata: Option<&Path>,
    portable_exists: bool,
) -> Result<PathBuf, String> {
    if let Some(p) = cli {
        return Ok(p.to_path_buf());
    }
    if let Some(e) = env {
        let e = e.trim();
        if !e.is_empty() {
            return Ok(PathBuf::from(e));
        }
    }
    if portable_exists {
        return Ok(exe_dir.join("portable_data"));
    }
    match appdata {
        Some(a) => Ok(a.join("com.and0114.cclean")),
        // %APPDATA% 缺失（极端环境）时不要瞎猜路径——猜错会把数据写到别人目录里。
        None => Err("无法确定数据目录：%APPDATA% 缺失，请用 --data-dir 指定".to_string()),
    }
}

/// 人类可读的字节数（与前端 `formatBytes` 同一套单位，保留 1 位小数）。
fn fmt_bytes(n: u64) -> String {
    const KB: f64 = 1024.0;
    let f = n as f64;
    if f < KB {
        return format!("{n} B");
    }
    if f < KB * KB {
        return format!("{:.1} KB", f / KB);
    }
    if f < KB * KB * KB {
        return format!("{:.1} MB", f / (KB * KB));
    }
    format!("{:.2} GB", f / (KB * KB * KB))
}

/// 显示宽度：CJK / 全角字符按 2 列算。
///
/// 不这么做的话，含中文的路径会把表格右半边全部推歪——而用户规则名、部分缓存目录
/// 名确实可能是中文（本机就有 `%LOCALAPPDATA%\Tencent` 这类）。
fn display_width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            let u = c as u32;
            let wide = (0x1100..=0x115F).contains(&u)
                || (0x2E80..=0xA4CF).contains(&u)
                || (0xAC00..=0xD7A3).contains(&u)
                || (0xF900..=0xFAFF).contains(&u)
                || (0xFE30..=0xFE6F).contains(&u)
                || (0xFF00..=0xFF60).contains(&u)
                || (0xFFE0..=0xFFE6).contains(&u);
            if wide {
                2
            } else {
                1
            }
        })
        .sum()
}

/// 按显示宽度右侧补空格。
fn pad(s: &str, width: usize) -> String {
    let w = display_width(s);
    if w >= width {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(width - w))
    }
}

/// 表格里的一行（从 `PendingItem` 转来，避免渲染逻辑依赖 DB 类型）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub path: String,
    pub size_bytes: i64,
    pub module_code: String,
    pub risk_level: String,
}

/// 渲染条目表。`detailed=false` 时只出「大小 / 风险 / 模块 / 路径」四列。
pub fn render_rows(rows: &[Row]) -> String {
    if rows.is_empty() {
        return "（没有可清理项）\n".to_string();
    }
    let headers = ["大小", "风险", "模块", "路径"];
    let cells: Vec<[String; 4]> = rows
        .iter()
        .map(|r| {
            [
                fmt_bytes(r.size_bytes.max(0) as u64),
                r.risk_level.clone(),
                r.module_code.clone(),
                r.path.clone(),
            ]
        })
        .collect();

    // 每列宽度 = 表头与该列所有单元格的最大显示宽度
    let mut widths = [0usize; 4];
    for (i, h) in headers.iter().enumerate() {
        widths[i] = display_width(h);
    }
    for row in &cells {
        for (i, c) in row.iter().enumerate() {
            widths[i] = widths[i].max(display_width(c));
        }
    }

    let mut out = String::new();
    out.push_str(
        &headers
            .iter()
            .enumerate()
            .map(|(i, h)| pad(h, widths[i]))
            .collect::<Vec<_>>()
            .join("  "),
    );
    out.push('\n');
    out.push_str(
        &widths
            .iter()
            .map(|w| "-".repeat(*w))
            .collect::<Vec<_>>()
            .join("  "),
    );
    out.push('\n');
    for row in &cells {
        let line = row
            .iter()
            .enumerate()
            .map(|(i, c)| pad(c, widths[i]))
            .collect::<Vec<_>>()
            .join("  ");
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

// ===================== 运行期 =====================

/// 运行期错误（与用法错误分开，决定退出码）。
enum RunError {
    Usage(String),
    Fail(String),
}

impl From<String> for RunError {
    fn from(s: String) -> Self {
        RunError::Fail(s)
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let code = match run(&argv) {
        Ok(c) => c,
        Err(RunError::Usage(m)) => {
            eprintln!("参数错误: {m}\n");
            eprint!("{USAGE}");
            EXIT_USAGE
        }
        Err(RunError::Fail(m)) => {
            eprintln!("错误: {m}");
            EXIT_ERR
        }
    };
    std::process::exit(code);
}

fn run(argv: &[String]) -> Result<i32, RunError> {
    let opt = parse_args(argv).map_err(RunError::Usage)?;

    match opt.cmd {
        Cmd::Help => {
            print!("{USAGE}");
            return Ok(EXIT_OK);
        }
        Cmd::Version => {
            println!("c-clean-cli {}", env!("CARGO_PKG_VERSION"));
            return Ok(EXIT_OK);
        }
        _ => {}
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let appdata = std::env::var("APPDATA").ok();
    let data_root = pick_data_root(
        opt.data_dir.as_deref(),
        std::env::var("CCLEAN_DATA_DIR").ok().as_deref(),
        &exe_dir,
        appdata.as_deref().map(Path::new),
        exe_dir.join("portable_data").is_dir(),
    )?;

    match opt.cmd {
        Cmd::ListRules => cmd_list_rules(&opt, &data_root, &exe_dir),
        Cmd::Scan => cmd_scan(&opt, &data_root, &exe_dir, false),
        Cmd::DryRun => cmd_scan(&opt, &data_root, &exe_dir, true),
        Cmd::Clean { yes, include_l2 } => {
            if !yes {
                eprintln!("[提示] 未加 --yes，只预览不删除。确认要清理请加 --yes。\n");
            }
            cmd_clean(&opt, &data_root, &exe_dir, yes, include_l2)
        }
        Cmd::Help | Cmd::Version => unreachable!("已在上方返回"),
    }
}

/// 规则目录：优先用数据目录里的可写副本（GUI 更新过的那份），否则用 exe 同目录的
/// 出厂副本。两处都没有就返回 `None`（不阻止扫描，只是没有外部 L3 红线）。
fn resolve_rules_dir(data_root: &Path, exe_dir: &Path) -> Option<PathBuf> {
    let writable = data_root.join("rules");
    if writable.is_dir() {
        return Some(writable);
    }
    let bundled = exe_dir.join("rules");
    if bundled.is_dir() {
        return Some(bundled);
    }
    None
}

/// 载入规则（失败返回空，调用方按「没有外部红线」处理）。
fn load_rules_or_empty(rules_dir: &Option<PathBuf>) -> Option<engine::rules::LoadedRules> {
    let dir = rules_dir.as_ref()?;
    match engine::rules::loader::load(dir) {
        Ok(l) => Some(l),
        Err(e) => {
            eprintln!("[警告] 规则库加载失败，本次不做外部红线校验: {e}");
            None
        }
    }
}

/// `list-rules`：内置规则（规则库 JSON）+ 用户规则（DB）。
fn cmd_list_rules(opt: &Options, data_root: &Path, exe_dir: &Path) -> Result<i32, RunError> {
    let rules_dir = resolve_rules_dir(data_root, exe_dir);
    let loaded = load_rules_or_empty(&rules_dir);

    // 内置规则按模块展开成扁平列表
    let mut builtin: Vec<(String, String, String)> = Vec::new(); // (module, id, risk)
    if let Some(l) = &loaded {
        let mut mods: Vec<(&String, &Vec<engine::rules::RuleEntry>)> =
            l.rules.modules.iter().collect();
        mods.sort_by(|a, b| a.0.cmp(b.0));
        for (m, entries) in mods {
            for e in entries {
                builtin.push((m.clone(), e.id.clone(), e.risk_level.clone()));
            }
        }
    }

    // 用户规则要读 DB；打不开不算错（可能是第一次运行、库还没建），给空列表 + 提示
    let conn = engine::db::conn::open(data_root).map_err(|e| format!("打开数据库失败: {e}"))?;
    let user = engine::ipc::commands::user_rules::list(&conn).unwrap_or_default();

    match opt.format {
        Format::Json => {
            let v = serde_json::json!({
                "rulesVersion": loaded.as_ref().map(|l| l.rules.version.clone()),
                "builtin": builtin.iter().map(|(m, id, risk)| serde_json::json!({
                    "module": m, "id": id, "riskLevel": risk
                })).collect::<Vec<_>>(),
                "user": user.iter().map(|u| serde_json::json!({
                    "id": u.id, "name": u.name, "pattern": u.pattern,
                    "riskLevel": u.risk_level, "enabled": u.enabled
                })).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
        }
        Format::Table => {
            match &loaded {
                Some(l) => println!("内置规则库 v{}（{} 条）", l.rules.version, builtin.len()),
                None => println!("内置规则库：未找到（扫描不受影响，只是没有外部 L3 红线）"),
            }
            for (m, id, risk) in &builtin {
                println!("  [{m}] {id}  ({risk})");
            }
            println!("\n用户自定义规则（{} 条）", user.len());
            if user.is_empty() {
                println!("  （无。可在 GUI 设置页添加）");
            }
            for u in &user {
                println!(
                    "  #{:<3} {:<20} {:<32} {} {}",
                    u.id,
                    u.name,
                    u.pattern,
                    u.risk_level,
                    if u.enabled { "启用" } else { "停用" }
                );
            }
        }
    }
    Ok(EXIT_OK)
}

/// 建一条 scan_session，返回 id。
///
/// CLI 需要直接 `execute` 一条 INSERT（engine 没有暴露「建会话」的封装，那是
/// GUI 的 `scan_start` 命令里内联的）。所以 cli 依赖 rusqlite——但只用它的方法，
/// 不构造连接（连接一律来自 `engine::db::conn::open`，迁移由 engine 保证）。
fn new_session(conn: &rusqlite::Connection) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (unixepoch(), 'running', 'full')",
        [],
    )
    .map_err(|e| format!("插入 scan_session 失败: {e}"))?;
    Ok(conn.last_insert_rowid())
}

/// 扫描一轮，返回 (session_id, 条目)。
fn do_scan(
    data_root: &Path,
    exe_dir: &Path,
    drives: Option<&[char]>,
) -> Result<(i64, Vec<Row>), String> {
    let conn = engine::db::conn::open(data_root).map_err(|e| format!("打开数据库失败: {e}"))?;
    let session_id = new_session(&conn)?;

    let emitter = engine::ipc::progress::NoopEmitter;
    let mut orch = engine::scan::ScanOrchestrator::new(session_id, &conn, &emitter);
    if let Some(d) = drives {
        orch = orch.with_drives(d.to_vec());
    }
    // 内置优先抑制：规则库读不出来就传空表（不抑制，用户规则照常生效）
    if let Some(l) = load_rules_or_empty(&resolve_rules_dir(data_root, exe_dir)) {
        orch = orch.with_builtin_prefixes(engine::rules::loader::builtin_dir_prefixes(&l));
    }

    // 脚本目录：CLI 便携版里 5 个模块全走 Rust，ps1 只是回退。找不到脚本目录也
    // 不影响（`run_all` 只在「回退到 ps1」时才需要它）。
    let scripts_dir = {
        let d = exe_dir.join("scripts");
        if d.is_dir() {
            d
        } else {
            data_root.join("scripts")
        }
    };

    let summary = orch
        .run_all(&scripts_dir)
        .map_err(|e| format!("扫描失败: {e}"))?;

    conn.execute(
        "UPDATE scan_session SET status='completed', finished_at=unixepoch() WHERE id=?1",
        [session_id],
    )
    .map_err(|e| format!("更新 session 状态失败: {e}"))?;

    // 拉本轮全部条目（上限 500 = list_pending 的 MAX_PAGE_SIZE）
    let page = engine::execute::list_pending::list_pending_page(
        &conn,
        Some(session_id),
        1,
        engine::execute::list_pending::MAX_PAGE_SIZE,
    )?;
    let rows: Vec<Row> = page
        .items
        .iter()
        .map(|i| Row {
            path: i.path.clone(),
            size_bytes: i.size_bytes,
            module_code: i.module_code.clone(),
            risk_level: i.risk_level.clone(),
        })
        .collect();

    eprintln!(
        "[scan] session={session_id} 条目={} 合计={} 成功模块={:?} 跳过={:?}",
        summary.total_items,
        fmt_bytes(summary.total_bytes.max(0) as u64),
        summary.scripts_succeeded,
        summary.scripts_skipped
    );
    Ok((session_id, rows))
}

/// `scan` / `dry-run`。
fn cmd_scan(
    opt: &Options,
    data_root: &Path,
    exe_dir: &Path,
    detailed: bool,
) -> Result<i32, RunError> {
    let (_sid, rows) = do_scan(data_root, exe_dir, opt.drives.as_deref())?;
    let total_bytes: i64 = rows.iter().map(|r| r.size_bytes.max(0)).sum();

    match opt.format {
        Format::Json => {
            let shown = if detailed {
                rows.len()
            } else {
                rows.len().min(opt.limit)
            };
            let v = serde_json::json!({
                "totalItems": rows.len(),
                "totalBytes": total_bytes,
                "shown": shown,
                "items": rows.iter().take(shown).map(|r| serde_json::json!({
                    "path": r.path,
                    "sizeBytes": r.size_bytes,
                    "module": r.module_code,
                    "riskLevel": r.risk_level,
                })).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
        }
        Format::Table => {
            println!(
                "共 {} 项，合计 {}\n",
                rows.len(),
                fmt_bytes(total_bytes.max(0) as u64)
            );
            let take = if detailed {
                rows.len()
            } else {
                rows.len().min(opt.limit)
            };
            print!("{}", render_rows(&rows[..take]));
            if take < rows.len() {
                println!(
                    "\n（只显示前 {take} 条，加 --limit {} 或改用 dry-run 看全部）",
                    rows.len()
                );
            }
            if detailed {
                println!("\n提示：以上只是预览，没有删除任何文件。要清理请用 `clean --yes`。");
            }
        }
    }
    Ok(EXIT_OK)
}

/// `clean`：扫描 → （`--yes` 时）移入保险箱。
fn cmd_clean(
    opt: &Options,
    data_root: &Path,
    exe_dir: &Path,
    yes: bool,
    include_l2: bool,
) -> Result<i32, RunError> {
    let (sid, rows) = do_scan(data_root, exe_dir, opt.drives.as_deref())?;

    // 选哪些条目：默认只 L1；--include-l2 才带上 L2。L3 永远不会出现在扫描结果里
    // （白名单层就拦了），这里不需要再判。
    let picked: Vec<&Row> = rows
        .iter()
        .filter(|r| r.risk_level == "L1" || (include_l2 && r.risk_level == "L2"))
        .collect();

    if !yes {
        match opt.format {
            Format::Json => {
                let v = serde_json::json!({
                    "dryRun": true,
                    "sessionId": sid,
                    "wouldCleanItems": picked.len(),
                    "wouldCleanBytes": picked.iter().map(|r| r.size_bytes.max(0)).sum::<i64>(),
                    "skippedL2": rows.iter().filter(|r| r.risk_level == "L2").count(),
                    "items": picked.iter().map(|r| serde_json::json!({
                        "path": r.path, "sizeBytes": r.size_bytes, "riskLevel": r.risk_level,
                    })).collect::<Vec<_>>(),
                });
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            }
            Format::Table => {
                println!(
                    "【预览】将清理 {} 项，合计 {}（未删除任何文件）\n",
                    picked.len(),
                    fmt_bytes(picked.iter().map(|r| r.size_bytes.max(0)).sum::<i64>() as u64)
                );
                let owned: Vec<Row> = picked.iter().map(|r| (*r).clone()).collect();
                print!("{}", render_rows(&owned));
                let l2 = rows.iter().filter(|r| r.risk_level == "L2").count();
                if l2 > 0 && !include_l2 {
                    println!(
                        "\n另有 {l2} 条 L2（需逐项确认）未包含；确认要一起清请加 --include-l2。"
                    );
                }
                println!(
                    "\n确认无误后执行：c-clean-cli clean --yes{}",
                    if include_l2 { " --include-l2" } else { "" }
                );
            }
        }
        return Ok(EXIT_OK);
    }

    if picked.is_empty() {
        println!(
            "没有需要清理的条目（共扫描到 {} 项，无 L1{}）。",
            rows.len(),
            if include_l2 { "/L2" } else { "" }
        );
        return Ok(EXIT_OK);
    }

    if include_l2 {
        eprintln!(
            "[警告] 本次包含 L2 条目。命令行不创建系统还原点（GUI 才有），\
             清理后无法整体回滚；文件会进保险箱，可逐条恢复。"
        );
    }

    // 只清本轮扫出来的条目：按 path 反查 id（list_pending 已经给了 id，这里为了
    // 与 Row 解耦重新查一次；条目数上限 500，成本可忽略）
    let conn = engine::db::conn::open(data_root).map_err(|e| format!("打开数据库失败: {e}"))?;
    let page = engine::execute::list_pending::list_pending_page(
        &conn,
        Some(sid),
        1,
        engine::execute::list_pending::MAX_PAGE_SIZE,
    )?;
    let wanted: std::collections::HashSet<&str> = picked.iter().map(|r| r.path.as_str()).collect();
    let ids: Vec<i64> = page
        .items
        .iter()
        .filter(|i| wanted.contains(i.path.as_str()))
        .map(|i| i.id)
        .collect();

    if ids.is_empty() {
        return Err(RunError::Fail("选中的条目在数据库中已不存在".to_string()));
    }

    let external_deny = load_rules_or_empty(&resolve_rules_dir(data_root, exe_dir))
        .map(|l| engine::rules::loader::external_deny_list(&l))
        .unwrap_or_default();
    let box_dir = data_root.join("safe_box");

    let summary = engine::execute::executor::execute_items(
        &ids,
        &conn,
        &external_deny,
        |_item_id, task_id| -> Box<dyn engine::execute::delete::Deleter + '_> {
            let source = if task_id >= 0 { Some(task_id) } else { None };
            Box::new(engine::execute::safe_box::SafeBoxDeleter::new(
                &box_dir, &conn, source,
            ))
        },
    );

    match opt.format {
        Format::Json => {
            let v = serde_json::json!({
                "dryRun": false,
                "sessionId": sid,
                "deletedCount": summary.deleted_count,
                "failedCount": summary.failed_count,
                "alreadyGoneCount": summary.already_gone_count,
                "bytesFreedTotal": summary.bytes_freed_total,
                "safeBoxDir": box_dir.to_string_lossy(),
                "failures": summary.results.iter()
                    .filter(|r| !r.deleted)
                    .map(|r| serde_json::json!({
                        "path": r.path, "error": r.error,
                    })).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
        }
        Format::Table => {
            println!(
                "清理完成：成功 {} 项 / 失败 {} 项 / 已不存在 {} 项，释放 {}",
                summary.deleted_count,
                summary.failed_count,
                summary.already_gone_count,
                fmt_bytes(summary.bytes_freed_total)
            );
            println!("文件已移入保险箱：{}（可逐条恢复）", box_dir.display());
            let fails: Vec<&engine::execute::executor::ExecResult> =
                summary.results.iter().filter(|r| !r.deleted).collect();
            if !fails.is_empty() {
                println!("\n失败明细：");
                for f in fails {
                    println!(
                        "  {}  —  {}",
                        f.path,
                        f.error.as_deref().unwrap_or("（未知原因）")
                    );
                }
            }
        }
    }
    Ok(EXIT_OK)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(args: &[&str]) -> Vec<String> {
        std::iter::once("c-clean-cli".to_string())
            .chain(args.iter().map(|s| s.to_string()))
            .collect()
    }

    // ===== 参数解析 =====

    #[test]
    fn no_args_shows_help() {
        // 不给子命令 = 打帮助，绝不默认执行扫描（最不容易出事的默认值）
        let o = parse_args(&argv(&[])).unwrap();
        assert_eq!(o.cmd, Cmd::Help);
        assert_eq!(o.format, Format::Table);
        assert_eq!(o.limit, 50);
    }

    #[test]
    fn parses_subcommands() {
        assert_eq!(parse_args(&argv(&["scan"])).unwrap().cmd, Cmd::Scan);
        assert_eq!(parse_args(&argv(&["dry-run"])).unwrap().cmd, Cmd::DryRun);
        assert_eq!(
            parse_args(&argv(&["list-rules"])).unwrap().cmd,
            Cmd::ListRules
        );
        // clean 默认 yes=false（只预览）
        assert_eq!(
            parse_args(&argv(&["clean"])).unwrap().cmd,
            Cmd::Clean {
                yes: false,
                include_l2: false
            }
        );
        assert_eq!(
            parse_args(&argv(&["clean", "--yes", "--include-l2"]))
                .unwrap()
                .cmd,
            Cmd::Clean {
                yes: true,
                include_l2: true
            }
        );
    }

    #[test]
    fn parses_value_flags() {
        let o = parse_args(&argv(&[
            "scan",
            "--format",
            "json",
            "--limit",
            "10",
            "--drive",
            "c,D:",
            "--data-dir",
            "X:/tmp",
        ]))
        .unwrap();
        assert_eq!(o.format, Format::Json);
        assert_eq!(o.limit, 10);
        // 大小写与冒号都归一
        assert_eq!(o.drives, Some(vec!['C', 'D']));
        assert_eq!(o.data_dir, Some(PathBuf::from("X:/tmp")));
    }

    #[test]
    fn rejects_usage_errors() {
        for bad in [
            vec!["nope"],
            vec!["scan", "--format", "yaml"],
            vec!["scan", "--format"],
            vec!["scan", "--drive"],
            vec!["scan", "--limit", "abc"],
            vec!["scan", "--wat"],
            // 两个子命令
            vec!["scan", "clean"],
            // --yes 只能配 clean：写错子命令时静默忽略会让用户以为文件被清了
            vec!["scan", "--yes"],
            vec!["list-rules", "--include-l2"],
        ] {
            assert!(parse_args(&argv(&bad)).is_err(), "{bad:?} 应报用法错误");
        }
    }

    #[test]
    fn parse_drives_rejects_garbage() {
        assert_eq!(parse_drives("C"), Ok(vec!['C']));
        assert_eq!(parse_drives("c:,D"), Ok(vec!['C', 'D']));
        // 重复项去掉
        assert_eq!(parse_drives("C,C:"), Ok(vec!['C']));
        for bad in ["", "1", "CC", "C,,D", " C , "] {
            assert!(parse_drives(bad).is_err(), "{bad:?} 应被拒");
        }
    }

    // ===== 数据目录三档优先级 =====

    #[test]
    fn data_root_priority() {
        let exe = Path::new("X:/app");
        let appdata = Some(Path::new("X:/roaming"));

        // 1) 命令行最高
        assert_eq!(
            pick_data_root(
                Some(Path::new("X:/cli")),
                Some("X:/env"),
                exe,
                appdata,
                true
            )
            .unwrap(),
            PathBuf::from("X:/cli")
        );
        // 2) 环境变量次之
        assert_eq!(
            pick_data_root(None, Some("X:/env"), exe, appdata, true).unwrap(),
            PathBuf::from("X:/env")
        );
        // 3) 便携布局：exe 同目录（此时环境变量为空）
        assert_eq!(
            pick_data_root(None, Some("   "), exe, appdata, true).unwrap(),
            PathBuf::from("X:/app/portable_data")
        );
        // 4) 兜底 %APPDATA%
        assert_eq!(
            pick_data_root(None, None, exe, appdata, false).unwrap(),
            PathBuf::from("X:/roaming/com.and0114.cclean")
        );
        // 连 %APPDATA% 都没有 → 报错，不瞎猜路径
        assert!(pick_data_root(None, None, exe, None, false).is_err());
    }

    // ===== 渲染 =====

    #[test]
    fn renders_table_with_aligned_columns() {
        let rows = vec![
            Row {
                path: "C:\\a.tmp".into(),
                size_bytes: 1024,
                module_code: "temp".into(),
                risk_level: "L1".into(),
            },
            Row {
                path: "C:\\某个很长的中文路径\\b.log".into(),
                size_bytes: 3 * 1024 * 1024,
                module_code: "user_rules".into(),
                risk_level: "L2".into(),
            },
        ];
        let out = render_rows(&rows);
        let lines: Vec<&str> = out.lines().collect();
        // 表头 + 分隔线 + 2 行
        assert_eq!(lines.len(), 4, "实际:\n{out}");
        assert!(lines[0].contains("大小") && lines[0].contains("路径"));
        assert!(out.contains("C:\\a.tmp"));
        assert!(out.contains("1.0 KB"));
        assert!(out.contains("3.0 MB"));
        // 对齐的判据不是「整行等宽」（每行末尾会 trim 掉空格），而是
        // **「路径」列的起始显示宽度一致**——含中文的那行如果按字符数补空格，
        // 这里就会差 1~2 列，表格右半边整个歪掉。
        let path_col = |line: &str, needle: &str| -> usize {
            let idx = line
                .find(needle)
                .unwrap_or_else(|| panic!("找不到 {needle}: {line}"));
            display_width(&line[..idx])
        };
        let h = path_col(lines[0], "路径");
        let r1 = path_col(lines[2], "C:\\a.tmp");
        let r2 = path_col(lines[3], "C:\\某个");
        assert_eq!(h, r1, "表头与第一行路径列起点不一致:\n{out}");
        assert_eq!(
            h, r2,
            "含中文的行路径列起点不一致（CJK 宽度没按 2 算）:\n{out}"
        );
    }

    #[test]
    fn renders_empty_list() {
        assert!(render_rows(&[]).contains("没有可清理项"));
    }

    #[test]
    fn formats_bytes() {
        assert_eq!(fmt_bytes(0), "0 B");
        assert_eq!(fmt_bytes(1023), "1023 B");
        assert_eq!(fmt_bytes(1024), "1.0 KB");
        assert_eq!(fmt_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(fmt_bytes(3 * 1024 * 1024 * 1024), "3.00 GB");
    }

    #[test]
    fn display_width_counts_cjk_as_two() {
        assert_eq!(display_width("abc"), 3);
        assert_eq!(display_width("中文"), 4);
        assert_eq!(display_width("a中b"), 4);
    }
}
