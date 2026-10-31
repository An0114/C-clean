//! WinSxS 组件存储（C 模块）分析（task-065）。
//!
//! # 为什么不是「解析 XML」
//!
//! 任务书写的是「解析 DISM 输出 XML」，但 `dism /online /cleanup-image
//! /analyzecomponentstore` **没有 XML 输出**（`/Format:XML` 不被接受，DISM 的
//! XML 只存在于 `/Get-Packages` 这类列举命令）。它只打印本地化文本。所以：
//!
//! 1. 提权 exe 侧强制加 `/English`，把标签锁成英文；
//! 2. 本模块按 `标签 : 值` 逐行解析英文文本。
//!
//! 不加 `/English` 的话，中文系统上标签会变成「实际大小」之类，解析必然全空——
//! 而「全空」在 UI 上看起来就是「可清理 0 字节」，比报错更危险。
//!
//! # 字段名的由来
//!
//! 任务书写的是 `{analyze_date, reusable_packets, backports_and_drivers,
//! recommended_cleanup}`，其中两个是误译：
//!
//! | 任务书 | DISM 真实标签 | 本模块字段 |
//! |---|---|---|
//! | `reusable_packets` | `Number of Reclaimable Packages` | `reclaimable_packages` |
//! | `backports_and_drivers` | `Backups and Disabled Features` | `backups_and_disabled_bytes` |
//!
//! 按 DISM 真实标签命名，避免后来人对着字段名猜错语义。

use std::path::Path;

use crate::elevation;

/// 提权 exe 命令名。**必须与 `elevation/src/main.rs` 的 `ALLOWED_COMMANDS` 一致**。
pub const DISM_ANALYZE_CMD: &str = "dism_analyze";
/// 清理命令名（同上）。
pub const DISM_CLEANUP_CMD: &str = "dism_cleanup";

/// 清理进度事件名。
///
/// 定义在 engine 而不是 Tauri 层：事件名是**两侧共用的契约**（Rust emit、
/// 前端 listen），各写一份迟早漂移成 `winsxs://progress` / `winsxs:progress`
/// 这种拼写差异，然后进度条永远不动还查不出原因。
pub const WINSXS_PROGRESS_EVENT: &str = "winsxs://progress";

/// WinSxS 分析结果。
///
/// `serde(rename_all = "camelCase")`：前端 interface 用 camelCase，
/// 两边字段名靠这条属性对齐（debt-cleanup-004 的教训：漏了就是一片 undefined）。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WinsxsAnalysis {
    /// 上次清理时间（DISM 的 `Date of Last Cleanup`，**原文透传**不重格式化）。
    ///
    /// 可能是 `Not available`（从未清理过）——所以是 `Option<String>` 而不是时间戳：
    /// 解析成时间戳就得处理时区与格式，而这里唯一的用途是「展示给用户看一眼」。
    pub last_cleanup: Option<String>,
    /// 可回收包数量（`Number of Reclaimable Packages`）。
    pub reclaimable_packages: u32,
    /// 组件存储实际大小（`Actual Size of Component Store`）。
    pub actual_size_bytes: u64,
    /// 「备份和已禁用功能」占用（`Backups and Disabled Features`）。
    pub backups_and_disabled_bytes: u64,
    /// 「缓存和临时数据」占用（`Cache and Temporary Data`）。
    pub cache_and_temp_bytes: u64,
    /// 估算可清理字节数。
    ///
    /// 优先用 DISM 明写的 `Actual Reclaimable Space`；部分版本不打印这一行，
    /// 此时用「备份 + 缓存」两项之和兜底（见 [`parse_analyze_output`]）。
    pub reclaimable_bytes: u64,
    /// DISM 是否建议清理（`Component Store Cleanup Recommended`）。
    pub recommended_cleanup: bool,
    /// 是否至少认出了一个关键标签。
    ///
    /// **不能省**：输出格式一变（微软改文案 / 系统语言没锁住），所有字段都会是
    /// 默认值 0，UI 上看起来就是「可清理 0 字节」——用户会以为「没什么可清的」，
    /// 而不是「分析失败了」。有这个标志就能明确报错。
    pub parsed: bool,
    /// DISM 原始输出（出错时给用户看原因，也是排障唯一线索）。
    pub raw_output: String,
}

impl WinsxsAnalysis {
    /// 有没有值得清理的东西（UI 用它决定「立即清理」按钮是否可点）。
    pub fn has_reclaimable(&self) -> bool {
        self.reclaimable_bytes > 0 || self.reclaimable_packages > 0
    }
}

/// 跑一次 DISM 组件存储分析。
///
/// 会弹 UAC（提权 exe 需要管理员才能跑 DISM）。调用方应保证是用户主动触发的。
pub fn analyze(exe_path: &Path) -> Result<WinsxsAnalysis, String> {
    let r = elevation::run_elevated(exe_path, DISM_ANALYZE_CMD, &[])?;
    let raw = r.output.clone().unwrap_or_default();

    if !r.ok {
        return Err(format!(
            "DISM 分析失败（退出码 {}）：{}",
            r.exit_code,
            tail_lines(&raw, 6)
        ));
    }

    let a = parse_analyze_output(&raw);
    if !a.parsed {
        // 解析不出关键字段 = 输出格式变了 / 语言没锁住。此时返回 0 字节会误导用户，
        // 必须报错并带上原文（与 DISM_ERROR 错误码「附 DISM 原始输出」一致）。
        return Err(format!(
            "无法解析 DISM 分析输出（输出格式可能已变化）：{}",
            tail_lines(&raw, 6)
        ));
    }
    Ok(a)
}

/// 解析 `dism /online /cleanup-image /analyzecomponentstore /English` 的文本输出。
///
/// 真实输出长这样（缩进子项、空行、进度条都会混进来）：
///
/// ```text
/// [==========================100.0%==========================]
///
/// Component Store (WinSxS) information:
///
/// Windows Explorer Reported Size of Component Store : 7.62 GB
///
/// Actual Size of Component Store : 7.36 GB
///
///     Shared with Windows : 6.13 GB
///     Backups and Disabled Features : 1.20 GB
///     Cache and Temporary Data : 30.15 MB
///
/// Date of Last Cleanup : 2021-03-15 08:21:46
///
/// Number of Reclaimable Packages : 0
/// Component Store Cleanup Recommended : No
/// ```
///
/// 解析策略：按 `" : "` 切「标签 / 值」。用 `split_once` 而不是 `split(':')`，
/// 因为值里可能带冒号（`2021-03-15 08:21:46`），而 DISM 的标签分隔符固定是
/// 「空格冒号空格」。
pub fn parse_analyze_output(text: &str) -> WinsxsAnalysis {
    let mut out = WinsxsAnalysis {
        raw_output: text.to_string(),
        ..Default::default()
    };
    let mut recognized = false;

    for line in text.lines() {
        let Some((key, value)) = line.split_once(" : ") else {
            continue;
        };
        let key = normalize_key(key);
        let value = value.trim();

        match key.as_str() {
            "actual size of component store" => {
                out.actual_size_bytes = parse_size(value).unwrap_or(0);
                recognized = true;
            }
            "backups and disabled features" => {
                out.backups_and_disabled_bytes = parse_size(value).unwrap_or(0);
                recognized = true;
            }
            "cache and temporary data" => {
                out.cache_and_temp_bytes = parse_size(value).unwrap_or(0);
                recognized = true;
            }
            "date of last cleanup" => {
                out.last_cleanup = Some(value.to_string());
                recognized = true;
            }
            "number of reclaimable packages" => {
                out.reclaimable_packages = value.parse().unwrap_or(0);
                recognized = true;
            }
            "component store cleanup recommended" => {
                out.recommended_cleanup = parse_yes_no(value);
                recognized = true;
            }
            // 不是所有版本都打印这一行（Win10 1809 之前没有）。有就用，没有走兜底。
            "actual reclaimable space" => {
                out.reclaimable_bytes = parse_size(value).unwrap_or(0);
                recognized = true;
            }
            _ => {}
        }
    }

    if out.reclaimable_bytes == 0 {
        // 兜底估算：DISM 自己认定的「可回收」就是「备份和已禁用功能」+
        // 「缓存和临时数据」。用 saturating_add 防极端值溢出。
        out.reclaimable_bytes = out
            .backups_and_disabled_bytes
            .saturating_add(out.cache_and_temp_bytes);
    }
    out.parsed = recognized;
    out
}

/// 标签归一化：小写 + 去掉首尾空白 + 内部连续空白压成一个空格。
///
/// 必须做：DISM 的子项带 4 空格缩进，且不同版本的标签里空格数不一致
/// （`Cache and Temporary Data` vs `Cache and  Temporary Data`）。
fn normalize_key(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for c in s.trim().chars() {
        if c.is_whitespace() {
            if !prev_space && !out.is_empty() {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(c.to_ascii_lowercase());
            prev_space = false;
        }
    }
    // 上面可能留了一个尾随空格（原串中间有空白但后面没内容）。
    out.trim_end().to_string()
}

/// `7.36 GB` → 字节数。
///
/// DISM 的数值最多两位小数，用 f64 足够；单位按 1024 进制（DISM 用的是这个，
/// 虽然它把 GB 显示成 10 进制营销值——两者差 7%，对「要不要清理」的判断无影响）。
pub fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let mut it = s.split_whitespace();
    let num: f64 = it.next()?.parse().ok()?;
    if !num.is_finite() || num < 0.0 {
        return None;
    }
    let unit = it.next().unwrap_or("B").to_ascii_uppercase();
    let mult: f64 = match unit.as_str() {
        "B" | "BYTE" | "BYTES" => 1.0,
        "KB" => 1024.0,
        "MB" => 1024.0 * 1024.0,
        "GB" => 1024.0 * 1024.0 * 1024.0,
        "TB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    // `as u64` 在 Rust 里是饱和转换（不会 UB），超大值只会截到 u64::MAX。
    Some((num * mult) as u64)
}

/// `Yes` / `No` → bool。顺带认中文「是 / 否」（万一 `/English` 没生效）。
fn parse_yes_no(s: &str) -> bool {
    let v = s.trim().to_ascii_lowercase();
    v.starts_with("yes") || v.starts_with('是')
}

/// 取最后 `n` 个非空行，拼成一行（错误消息里用）。
///
/// 为什么只要尾部：DISM 的失败原因永远在最后（「错误: 740」），前面全是进度条
/// 和版本信息，塞进信封会把 UI 撑爆。
pub fn tail_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join(" / ")
}

// ===================== 清理（task-065 单元 2 续） =====================

/// 清理进度事件载荷。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WinsxsProgress {
    /// 行序号（从 0 开始，单调递增）。前端用它做 key，也用来判断「有没有新行」。
    pub seq: u64,
    /// DISM 输出的一行（已去掉空行）。
    pub line: String,
}

/// WinSxS 清理结果。
///
/// 为什么 DISM 失败也返回 `Ok`（`ok: false`）而不是 `Err`：失败时**输出尾部是
/// 唯一线索**（「错误: 740」/「组件存储损坏」），返回 Err 会把它丢掉，用户只看到
/// 一句「清理失败」无从下手。真正的「没能执行」（UAC 被拒 / 提权 exe 缺失）
/// 才走 [`WinsxsCleanupError`]。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WinsxsCleanupResult {
    /// DISM 退出码是否为 0。
    pub ok: bool,
    /// 提权 exe 报告的退出码。
    pub exit_code: i32,
    /// 清理前建成的系统还原点序号；跳过了还原点时为 `None`。
    pub restore_point_seq: Option<i64>,
    /// DISM 输出尾部（最多 12 行，失败原因一定在尾部）。
    pub output_tail: String,
    /// 转发出去多少行进度。
    pub line_count: u64,
}

/// 清理失败的两类「没能执行」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WinsxsCleanupError {
    /// 还原点没建成（系统还原被关闭 / 磁盘空间不足 / 用户拒了 UAC）。
    ///
    /// 单独一类是因为前端要据此弹「是否继续清理？」——用户在确认框点继续后
    /// 带 `skip_restore_point=true` 重调。
    RestorePoint(String),
    /// 提权链路本身失败（UAC 被取消 / 提权 exe 不存在 / 白名单拒绝）。
    Elevation(String),
}

impl std::fmt::Display for WinsxsCleanupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RestorePoint(m) => write!(f, "系统还原点创建失败：{m}"),
            Self::Elevation(m) => write!(f, "提权执行失败：{m}"),
        }
    }
}

/// 跑一次 DISM 组件存储清理。
///
/// # 顺序（不可颠倒）
///
/// 1. **先建系统还原点**——`/StartComponentCleanup` 会删掉旧版本组件，
///    不可逆，必须在动手之前留退路。`skip_restore_point=false` 时建不成
///    就直接返回 [`WinsxsCleanupError::RestorePoint`]（fail-closed）。
/// 2. 再跑 DISM，边跑边把输出行抛给 `on_line`。
///
/// # 为什么这里没有 `/ResetBase` 参数
///
/// 参数在提权 exe 里是硬编码常量（`elevation::dism::Action::Cleanup::args`），
/// 本函数连传参的口子都没有——**不是「传了会被拒」，而是「没法传」**。
pub fn cleanup(
    conn: &rusqlite::Connection,
    exe_path: &Path,
    skip_restore_point: bool,
    on_line: &(dyn Fn(u64, &str) + Sync),
) -> Result<WinsxsCleanupResult, WinsxsCleanupError> {
    use std::sync::atomic::{AtomicU64, Ordering};

    let restore_point_seq = if skip_restore_point {
        // 用户已经在「还原点创建失败，是否继续清理？」里点了继续。
        // 不建还原点，但**不掩盖事实**：结果里 restore_point_seq = None。
        None
    } else {
        Some(
            elevation::ensure_restore_point(conn, exe_path)
                .map_err(WinsxsCleanupError::RestorePoint)?,
        )
    };

    // 行数统计放在闭包里做，而不是事后数 output：DISM 的输出可能被 tail_lines
    // 截断，事后数出来的不是「实际转发给用户的行数」。
    let counter = AtomicU64::new(0);
    let r = elevation::run_elevated_streaming(exe_path, DISM_CLEANUP_CMD, &[], &|seq, line| {
        counter.fetch_add(1, Ordering::Relaxed);
        on_line(seq, line);
    })
    .map_err(WinsxsCleanupError::Elevation)?;

    let raw = r.output.unwrap_or_default();
    Ok(WinsxsCleanupResult {
        ok: r.ok,
        exit_code: r.exit_code,
        restore_point_seq,
        output_tail: tail_lines(&raw, 12),
        line_count: counter.load(Ordering::Relaxed),
    })
}
