//! 清理报告导出（task-063）。
//!
//! 提供三件事：
//! 1. [`drive_usage`]：查某个盘的容量/已用/可用（`GetDiskFreeSpaceExW`）；
//! 2. [`build_html`] / [`build_csv`]：把一次清理的结果渲染成报告字符串；
//! 3. [`write_report`]：落盘到 `<data_root>/reports/`。
//!
//! # 为什么磁盘查询放在 report 模块
//!
//! 「前后空间对比」是报告的核心字段（清理前 X → 清理后 Y，释放 Z），两者用的是
//! 同一份数据。单独开一个 `sysinfo` 模块只为了一个函数不划算，等以后有第二个
//! 系统信息查询（CPU / 内存）时再抽。
//!
//! # 为什么报告是「字符串拼接」而不是模板引擎
//!
//! 模板引擎（askama / tera）会带来编译期宏或运行时依赖，而我们的模板只有
//! 「标题 + 3 个汇总数字 + 一张表」。手写拼接 + 严格的 [`html_escape`] 足够，
//! 且**零依赖**（本 crate 的编译时间一直是重点关注项）。
//!
//! **安全要点**：所有来自文件系统 / 用户数据的文本（路径、错误消息）都必须过
//! [`html_escape`]，否则一个文件名里带 `<script>` 就能在报告里执行脚本。

use std::path::{Path, PathBuf};

/// 一个盘的空间使用情况。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveUsage {
    /// 总容量（字节）。
    pub total_bytes: u64,
    /// 已用（字节）= total - free。
    pub used_bytes: u64,
    /// 可用（字节，对所有用户可用，不只是当前用户）。
    pub free_bytes: u64,
}

impl DriveUsage {
    /// 已用百分比（0–100，四舍五入到整数）。总容量为 0 时返回 0（不除零）。
    pub fn used_percent(&self) -> u32 {
        if self.total_bytes == 0 {
            return 0;
        }
        // 用 u128 防 `used * 100` 溢出（u64::MAX * 100 会溢出）。
        let pct = (self.used_bytes as u128 * 100) / self.total_bytes as u128;
        pct.min(100) as u32
    }
}

/// 查 `drive`（如 `"C"` 或 `"C:"`）的空间使用情况。
pub fn drive_usage(drive: &str) -> Result<DriveUsage, String> {
    #[cfg(windows)]
    {
        use windows::core::HSTRING;
        use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

        // API 要的是「根目录」形式：`C:\`。
        let letter = drive.trim_end_matches(['\\', ':']);
        if letter.is_empty() {
            return Err("drive 不能为空".to_string());
        }
        let root = HSTRING::from(format!("{letter}:\\"));

        let mut free_to_caller: u64 = 0;
        let mut total: u64 = 0;
        let mut total_free: u64 = 0;
        // SAFETY: 三个 &mut 指向本栈帧上的 u64；root 在本调用期间存活。
        unsafe {
            GetDiskFreeSpaceExW(
                &root,
                Some(&mut free_to_caller),
                Some(&mut total),
                Some(&mut total_free),
            )
        }
        .map_err(|e| format!("查询 {letter}: 空间失败: {e}"))?;

        // 用 total_free（全用户可用）算 used，而不是 free_to_caller（当前用户配额）——
        // 用户关心的是「这个盘还剩多少」，不是「我这个账户还能写多少」。
        Ok(DriveUsage {
            total_bytes: total,
            used_bytes: total.saturating_sub(total_free),
            free_bytes: total_free,
        })
    }
    #[cfg(not(windows))]
    {
        let _ = drive;
        Err("磁盘空间查询仅支持 Windows".to_string())
    }
}

/// 报告里「清理前 / 清理后」的空间快照。
///
/// 两个字段都是 `Option`：查不到就显示「—」而不是 0（0 GB 会让人以为盘空了）。
#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveSnapshot {
    pub before_used_bytes: Option<u64>,
    pub after_used_bytes: Option<u64>,
}

impl DriveSnapshot {
    /// 实际释放的空间 = 前 - 后（**饱和减法**：清理期间用户可能又写入了新文件，
    /// 导致「后 > 前」，此时算 0 而不是负数）。
    ///
    /// 注意这与 `bytes_freed_total`（文件大小求和）不是一回事：
    ///
    /// - `bytes_freed_total` = 我们删掉的文件总大小（精确，但可能因簇对齐而偏大）；
    /// - 本函数 = 磁盘实际变化（含用户并发写入的干扰）。
    ///
    /// 报告里两个都展示，用户能看出差异。
    pub fn released_bytes(&self) -> Option<u64> {
        match (self.before_used_bytes, self.after_used_bytes) {
            (Some(b), Some(a)) => Some(b.saturating_sub(a)),
            _ => None,
        }
    }
}

/// 报告渲染所需的元信息。
pub struct ReportMeta<'a> {
    /// 生成时间（unix 秒）。
    pub generated_at: i64,
    /// 盘符（用于文案「C 盘」）。
    pub drive: &'a str,
    pub snapshot: DriveSnapshot,
}

/// 渲染单文件 HTML 报告。
///
/// 全部 CSS 内联（`<style>` 块），不引外部资源——用户双击就能打开，
/// 拷到别的机器也不会丢样式。
pub fn build_html(
    summary: &crate::execute::executor::ExecuteSummary,
    meta: &ReportMeta<'_>,
) -> String {
    let mut rows = String::new();
    for r in &summary.results {
        let (status, status_class) = if r.deleted {
            ("已删除", "ok")
        } else if r.already_gone {
            ("已不存在", "skip")
        } else {
            ("失败", "fail")
        };
        let reason = r.error.as_deref().unwrap_or("—");
        rows.push_str(&format!(
            "<tr><td class=\"path\">{}</td><td class=\"num\">{}</td><td class=\"{}\">{}</td><td>{}</td></tr>\n",
            html_escape(&r.path),
            human_bytes(r.bytes_freed),
            status_class,
            status,
            html_escape(reason),
        ));
    }

    let before = meta
        .snapshot
        .before_used_bytes
        .map(human_bytes)
        .unwrap_or_else(|| "—".to_string());
    let after = meta
        .snapshot
        .after_used_bytes
        .map(human_bytes)
        .unwrap_or_else(|| "—".to_string());
    let released = meta
        .snapshot
        .released_bytes()
        .map(human_bytes)
        .unwrap_or_else(|| "—".to_string());

    format!(
        r#"<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<title>C-Disk Cleaner 清理报告</title>
<style>
  body {{ font-family: system-ui, -apple-system, "Segoe UI", sans-serif; margin: 32px; color: #111827; background: #f9fafb; }}
  h1 {{ font-size: 22px; margin: 0 0 4px; }}
  .meta {{ color: #6b7280; font-size: 13px; margin-bottom: 20px; }}
  .cards {{ display: flex; gap: 12px; flex-wrap: wrap; margin-bottom: 20px; }}
  .card {{ background: #fff; border: 1px solid #e5e7eb; border-radius: 8px; padding: 14px 18px; min-width: 150px; }}
  .card .label {{ font-size: 12px; color: #6b7280; }}
  .card .value {{ font-size: 20px; font-weight: 600; margin-top: 4px; }}
  table {{ border-collapse: collapse; width: 100%; background: #fff; border: 1px solid #e5e7eb; border-radius: 8px; overflow: hidden; }}
  th, td {{ padding: 8px 12px; text-align: left; font-size: 13px; border-bottom: 1px solid #f3f4f6; }}
  th {{ background: #f9fafb; font-weight: 600; }}
  td.num {{ text-align: right; white-space: nowrap; }}
  td.path {{ font-family: ui-monospace, Consolas, monospace; font-size: 12px; word-break: break-all; }}
  .ok {{ color: #16a34a; }}
  .fail {{ color: #dc2626; }}
  .skip {{ color: #6b7280; }}
  .foot {{ margin-top: 16px; font-size: 12px; color: #9ca3af; }}
</style>
</head>
<body>
<h1>C-Disk Cleaner 清理报告</h1>
<div class="meta">生成时间：{generated}　目标盘：{drive} 盘</div>
<div class="cards">
  <div class="card"><div class="label">删除成功</div><div class="value">{deleted}</div></div>
  <div class="card"><div class="label">失败</div><div class="value">{failed}</div></div>
  <div class="card"><div class="label">文件大小合计</div><div class="value">{freed}</div></div>
  <div class="card"><div class="label">清理前已用</div><div class="value">{before}</div></div>
  <div class="card"><div class="label">清理后已用</div><div class="value">{after}</div></div>
  <div class="card"><div class="label">磁盘实际释放</div><div class="value">{released}</div></div>
</div>
<table>
<thead><tr><th>路径</th><th>大小</th><th>状态</th><th>原因</th></tr></thead>
<tbody>
{rows}</tbody>
</table>
<div class="foot">由 C-Disk Cleaner 生成。删除的文件可在保险箱保留期内恢复。</div>
</body>
</html>
"#,
        generated = format_utc(meta.generated_at),
        drive = html_escape(meta.drive),
        deleted = summary.deleted_count,
        failed = summary.failed_count,
        freed = human_bytes(summary.bytes_freed_total),
        before = before,
        after = after,
        released = released,
        rows = rows,
    )
}

/// 渲染 CSV 明细。
///
/// **带 UTF-8 BOM**：不带 BOM 的话，Excel（中文 Windows）会把 UTF-8 当 ANSI 读，
/// 中文路径全是乱码——这是国内用户最常报的「CSV 打不开」的真实原因。
///
/// 列：路径 / 大小(字节) / 状态 / 原因。
pub fn build_csv(summary: &crate::execute::executor::ExecuteSummary) -> String {
    let mut out = String::from("\u{feff}");
    out.push_str("路径,大小(字节),状态,原因\n");
    for r in &summary.results {
        let status = if r.deleted {
            "已删除"
        } else if r.already_gone {
            "已不存在"
        } else {
            "失败"
        };
        out.push_str(&format!(
            "{},{},{},{}\n",
            csv_escape(&r.path),
            r.bytes_freed,
            status,
            csv_escape(r.error.as_deref().unwrap_or("")),
        ));
    }
    out
}

/// 把报告写到 `dir/<filename>`，返回绝对路径。目录不存在会自动创建。
pub fn write_report(dir: &Path, filename: &str, content: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("创建报告目录失败 {}: {e}", dir.display()))?;
    let path = dir.join(filename);
    std::fs::write(&path, content).map_err(|e| format!("写报告失败 {}: {e}", path.display()))?;
    Ok(path)
}

// ===================== 工具函数 =====================

/// 字节 → 人类可读（1024 进制、1 位小数）。
fn human_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/// HTML 转义。**所有来自外部的文本都必须过这里**（路径里可能有 `<`、`&`）。
fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// CSV 字段转义：含逗号 / 引号 / 换行时用双引号包裹，内部引号翻倍（RFC 4180）。
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// unix 秒 → `YYYY-MM-DD HH:MM:SS`（UTC）。
///
/// 与 `elevation::format_utc` 同一套算法（Howard Hinnant `civil_from_days`）。
/// 刻意不共用：那个是 `pub` 但语义属于提权模块，报告不该依赖它。
fn format_utc(ts: i64) -> String {
    let days = ts.div_euclid(86_400);
    let secs = ts.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}",
        secs / 3_600,
        (secs % 3_600) / 60,
        secs % 60
    )
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execute::executor::{ExecResult, ExecuteSummary};

    fn mk(
        path: &str,
        deleted: bool,
        already_gone: bool,
        bytes: u64,
        err: Option<&str>,
    ) -> ExecResult {
        ExecResult {
            item_id: 1,
            path: path.to_string(),
            deleted,
            already_gone,
            bytes_freed: bytes,
            error: err.map(|s| s.to_string()),
            locking_processes: Vec::new(),
        }
    }

    fn summary_of(results: Vec<ExecResult>) -> ExecuteSummary {
        let deleted_count = results.iter().filter(|r| r.deleted).count() as u64;
        let failed_count = results
            .iter()
            .filter(|r| !r.deleted && !r.already_gone)
            .count() as u64;
        let bytes_freed_total = results.iter().map(|r| r.bytes_freed).sum();
        ExecuteSummary {
            results,
            deleted_count,
            failed_count,
            already_gone_count: 0,
            bytes_freed_total,
            disk_before_bytes: None,
            disk_after_bytes: None,
        }
    }

    #[test]
    fn used_percent_basic() {
        let u = DriveUsage {
            total_bytes: 1000,
            used_bytes: 250,
            free_bytes: 750,
        };
        assert_eq!(u.used_percent(), 25);
        // 总容量 0 不除零
        let z = DriveUsage {
            total_bytes: 0,
            used_bytes: 0,
            free_bytes: 0,
        };
        assert_eq!(z.used_percent(), 0);
    }

    #[test]
    fn used_percent_does_not_overflow_on_huge_disks() {
        // 8 TB 盘：used*100 超过 u32，必须用 u128 算
        let u = DriveUsage {
            total_bytes: 8 * 1024u64.pow(4),
            used_bytes: 4 * 1024u64.pow(4),
            free_bytes: 4 * 1024u64.pow(4),
        };
        assert_eq!(u.used_percent(), 50);
    }

    #[test]
    fn snapshot_released_uses_saturating_sub() {
        let s = DriveSnapshot {
            before_used_bytes: Some(1000),
            after_used_bytes: Some(400),
        };
        assert_eq!(s.released_bytes(), Some(600));

        // 清理期间用户又写了文件 → after > before，应算 0 而不是负数
        let s2 = DriveSnapshot {
            before_used_bytes: Some(400),
            after_used_bytes: Some(1000),
        };
        assert_eq!(s2.released_bytes(), Some(0));

        // 缺任一侧 → None（显示「—」）
        assert_eq!(DriveSnapshot::default().released_bytes(), None);
        assert_eq!(
            DriveSnapshot {
                before_used_bytes: Some(1),
                after_used_bytes: None
            }
            .released_bytes(),
            None
        );
    }

    #[test]
    fn csv_has_bom_header_and_rows() {
        let s = summary_of(vec![
            mk(r"C:\tmp\a.tmp", true, false, 2048, None),
            mk(r"C:\tmp\b.tmp", false, false, 0, Some("文件被占用")),
        ]);
        let csv = build_csv(&s);
        // BOM 必须是第一个字符（否则 Excel 中文乱码）
        assert!(csv.starts_with('\u{feff}'), "CSV 必须带 UTF-8 BOM");
        assert!(csv.contains("路径,大小(字节),状态,原因"));
        assert!(csv.contains("C:\\tmp\\a.tmp,2048,已删除,"));
        assert!(csv.contains("C:\\tmp\\b.tmp,0,失败,文件被占用"));
    }

    #[test]
    fn csv_escapes_comma_and_quote() {
        let s = summary_of(vec![mk(
            r"C:\a,b\c.tmp",
            false,
            false,
            0,
            Some(r#"say "hi""#),
        )]);
        let csv = build_csv(&s);
        // 含逗号的路径要被双引号包裹
        assert!(csv.contains(r#""C:\a,b\c.tmp""#), "实际: {csv}");
        // 内部引号翻倍
        assert!(csv.contains(r#""say ""hi""""#), "实际: {csv}");
    }

    #[test]
    fn html_escapes_dangerous_chars() {
        let s = summary_of(vec![mk(
            r"C:\x<script>alert(1)</script>.tmp",
            false,
            false,
            0,
            Some("a & b"),
        )]);
        let meta = ReportMeta {
            generated_at: 0,
            drive: "C",
            snapshot: DriveSnapshot::default(),
        };
        let html = build_html(&s, &meta);
        // 绝不能出现可执行的 <script>
        assert!(!html.contains("<script>"), "路径未转义，存在 XSS");
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("a &amp; b"));
    }

    #[test]
    fn html_contains_summary_and_table() {
        let s = summary_of(vec![
            mk(r"C:\tmp\a.tmp", true, false, 1024, None),
            mk(r"C:\tmp\b.tmp", false, true, 0, None),
        ]);
        let meta = ReportMeta {
            generated_at: 0,
            drive: "C",
            snapshot: DriveSnapshot {
                before_used_bytes: Some(10 * 1024 * 1024 * 1024),
                after_used_bytes: Some(9 * 1024 * 1024 * 1024),
            },
        };
        let html = build_html(&s, &meta);
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("C-Disk Cleaner 清理报告"));
        // 三种状态都要能渲染
        assert!(html.contains("已删除"));
        assert!(html.contains("已不存在"));
        // 前后对比：10 GB → 9 GB，释放 1 GB
        assert!(html.contains("10.0 GB"), "实际缺清理前数值");
        assert!(html.contains("9.0 GB"), "实际缺清理后数值");
        assert!(html.contains("1.0 GB"), "实际缺释放量");
    }

    #[test]
    fn html_shows_dash_when_snapshot_missing() {
        let s = summary_of(vec![]);
        let meta = ReportMeta {
            generated_at: 0,
            drive: "C",
            snapshot: DriveSnapshot::default(),
        };
        let html = build_html(&s, &meta);
        // 查不到空间时显示「—」，不能显示 0 GB（会让人以为盘空了）。
        // 注意「文件大小合计 0 B」是合理的（确实没删东西），只校验空间三项。
        assert!(html.contains("—"), "缺数据时空间卡片应显示破折号");
        assert!(!html.contains("0.0 GB"), "缺数据时不该显示 0.0 GB");
        assert!(!html.contains("0.0 MB"), "缺数据时不该显示 0.0 MB");
    }

    #[test]
    fn write_report_creates_dir_and_file() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("reports");
        let p = write_report(&target, "r.html", "<html>ok</html>").unwrap();
        assert!(p.exists());
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "<html>ok</html>");
    }
}
