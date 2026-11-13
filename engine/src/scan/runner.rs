//! `ScanRunner`：spawn `powershell.exe -File <script>`，逐行消费 NDJSON。
//!
//! 设计要点（对齐任务 012 §C）：
//!
//! - 一次只跑一个脚本；编排（5 个 ps1 串跑）留后续 task。
//! - item 行：参数化 INSERT 进 `cleanup_item`，并 emit 进度。
//! - done 行：记账 total_bytes / done_seen，break。
//! - 行解析失败：`eprintln!` 跳过，不 panic、不中断。
//! - 子进程非 0 退出但已见到 done：视为成功（脚本部分失败但 NDJSON 完整）。
//! - stderr：整段读出来一次性 `eprintln!`，不解析。

use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicI64, Ordering};

use rusqlite::Connection;

use crate::ipc::progress::{ProgressEmitter, ScanProgress};
use crate::scan::parser::{parse_line, NdjsonEvent};

/// 单脚本运行结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanSummary {
    /// 成功写入 cleanup_item 的条数。
    pub items_inserted: i64,
    /// done 事件里的 total_bytes（未见到 done 时为 0）。
    pub total_bytes: i64,
    /// 是否见到 done 事件。
    pub done_seen: bool,
}

/// runner 级错误。
#[derive(thiserror::Error, Debug)]
pub enum RunError {
    /// spawn / wait 子进程失败。
    #[error("spawn powershell 失败: {0}")]
    Spawn(#[from] std::io::Error),
    /// 子进程非 0 退出且未见到 done 事件。
    #[error("脚本退出码 {0} 且未收到 done 事件")]
    ScriptFailed(i32),
    /// DB 写入失败（已被视为致命：无法保证数据完整性）。
    #[error("写 cleanup_item 失败: {0}")]
    Db(#[from] rusqlite::Error),
}

/// 亚线性插值的半程点（task-052 G1）。
///
/// 语义：本脚本内扫到第 `INTERP_HALF_POINT` 条时，正好走完该脚本分到的百分比区间的一半。
/// 取 50 是因为单个脚本的条目数多在 10~1500 之间（实测 temp 154 / dev_cache 33 /
/// windows_update 6），50 让进度条在前几十条就有可见推进，又不会在条目极多时早早顶到上界。
const INTERP_HALF_POINT: f64 = 50.0;

/// 把「本脚本内已扫 n 条」映射到 `[from, to]` 百分比区间（task-052 G1）。
///
/// 用 `n / (n + K)` 做亚线性插值而不是 `n / total`：
///
/// - ps1 协议**不预告**本脚本总条目数，拿不到 `total`，线性插值无从谈起；
/// - `n/(n+K)` 单调递增、有上界 1、且天然「前快后慢」，符合用户对扫描进度的直觉；
/// - 结果恒落在 `[from, to]`（`to` 只会在脚本结束时的边界事件里被精确打到）。
///
/// 边界约定：`n == 0` 返回 `from`；`from >= to` 时原样返回 `from`（退化区间不动）。
pub fn interpolate_percent(from: u8, to: u8, n: u64) -> u8 {
    if from >= to {
        return from;
    }
    let span = f64::from(to - from);
    let ratio = n as f64 / (n as f64 + INTERP_HALF_POINT);
    let value = f64::from(from) + span * ratio;
    // clamp 兜住浮点误差，保证永远不越过 to。
    value.round().clamp(f64::from(from), f64::from(to)) as u8
}

/// 扫描 runner：绑定一个 DB 连接 + 一个 emitter，跑多个脚本时复用。
pub struct ScanRunner<'conn, 'emitter> {
    session_id: i64,
    conn: &'conn Connection,
    emitter: &'emitter dyn ProgressEmitter,
    /// 已成功入库的 item 数（跨脚本累计）。
    scanned_count: AtomicI64,
    /// 本脚本分到的百分比区间 `[from, to]`（task-052 G1）。
    /// 默认 `(0, 0)`：保持「不经编排器直接跑单脚本」时 percent 恒 0 的旧行为。
    percent_span: (u8, u8),
}

impl<'conn, 'emitter> ScanRunner<'conn, 'emitter> {
    /// 构造：给定 session_id（须已在 scan_session 表中存在）、DB 连接、进度 emitter。
    pub fn new(
        session_id: i64,
        conn: &'conn Connection,
        emitter: &'emitter dyn ProgressEmitter,
    ) -> Self {
        Self {
            session_id,
            conn,
            emitter,
            scanned_count: AtomicI64::new(0),
            percent_span: (0, 0),
        }
    }

    /// 指定本脚本的百分比区间（task-052 G1，由 `ScanOrchestrator` 按脚本序号算出）。
    ///
    /// `to` 只作为插值上界；脚本跑完时由编排器补一条 `percent = to` 的边界事件。
    pub fn with_percent_span(mut self, from: u8, to: u8) -> Self {
        self.percent_span = (from, to);
        self
    }

    /// 跑一个 ps1 脚本，逐行消费 stdout。
    pub fn run_script(&self, script_path: &Path) -> Result<ScanSummary, RunError> {
        let mut child = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(script_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().expect("stdout piped");
        let mut stderr = child.stderr.take().expect("stderr piped");

        let mut items_inserted: i64 = 0;
        let mut total_bytes: i64 = 0;
        let mut done_seen = false;

        for line in BufReader::new(stdout).lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[scan] stdout read err: {e}");
                    break;
                }
            };
            // ps1 行尾是 \r\n，lines() 已剥；仍 trim 一次兜底。
            match parse_line(&line) {
                Ok(NdjsonEvent::Item {
                    module_code,
                    category,
                    path,
                    size_bytes,
                    risk_level,
                }) => {
                    if let Err(e) = self.insert_item(
                        &module_code,
                        &category,
                        &path,
                        size_bytes,
                        risk_level,
                        None,
                        None,
                        None,
                    ) {
                        // DB 错视为致命：不继续往表里灌半份数据。
                        return Err(RunError::Db(e));
                    }
                    items_inserted += 1;
                    let n = self.scanned_count.fetch_add(1, Ordering::Relaxed) + 1;
                    self.emitter.emit(&ScanProgress {
                        session_id: self.session_id,
                        // task-052 G1：按「本脚本已扫条数」在分到的区间内亚线性插值。
                        percent: interpolate_percent(
                            self.percent_span.0,
                            self.percent_span.1,
                            items_inserted as u64,
                        ),
                        current_path: path,
                        scanned_items: n as u64,
                    });
                }
                Ok(NdjsonEvent::Done {
                    total_items: _,
                    total_bytes: tb,
                    ..
                }) => {
                    done_seen = true;
                    total_bytes = tb;
                    break;
                }
                Err(e) => {
                    eprintln!("[scan] skip bad line: {e}: {line}");
                }
            }
        }

        let status = child.wait()?;
        // stderr 整段读出打印（不解析、不入库）。
        let mut buf = String::new();
        if stderr.read_to_string(&mut buf).is_ok() && !buf.trim().is_empty() {
            eprintln!("[scan][{}] stderr:\n{}", script_path.display(), buf);
        }

        if !done_seen && !status.success() {
            return Err(RunError::ScriptFailed(status.code().unwrap_or(-1)));
        }
        Ok(ScanSummary {
            items_inserted,
            total_bytes,
            done_seen,
        })
    }

    /// task-074：把一批 **Rust 侧扫描结果**入库并发进度。
    ///
    /// # 为什么复用 `insert_item` 而不是另写一份 INSERT
    ///
    /// 两条路径（ps1 / Rust）写进 `cleanup_item` 的行必须**完全一样**：
    /// `risk_level` 的整数→`'L1'`/`'L2'` 映射、`description` 的空串兜底、
    /// `session_id` 绑定。任何一处各写一份，就会出现「同一个文件在 Rust 模式是
    /// L2、ps1 模式是 L1」这种最难查的 bug——这正是架构债 R5 要消灭的东西，
    /// 不能重写的时候又造一个新的。
    ///
    /// # `done_seen: true`
    ///
    /// Rust 路径没有 done 事件，但语义上是「完整跑完」（函数返回即全部条目
    /// 都已入库）。调用方拿它跟 ps1 的返回值对比时不会被误判成「脚本没跑完」。
    pub fn insert_batch(
        &self,
        items: &[crate::scan::modules::ScanItem],
    ) -> Result<ScanSummary, RunError> {
        let mut items_inserted: i64 = 0;
        let mut total_bytes: i64 = 0;

        for it in items {
            self.insert_item(
                &it.module_code,
                &it.category,
                &it.path,
                it.size_bytes,
                it.risk_level,
                it.username.as_deref(),
                it.impact_note.as_deref(),
                it.rebuild_note.as_deref(),
            )?;
            items_inserted += 1;
            // 饱和加法：文件大小之和理论上不会溢出，但扫描结果来自外部文件系统，
            // 宁可截断也不要 panic（debug 下 u64/i64 溢出会直接 abort）。
            total_bytes = total_bytes.saturating_add(it.size_bytes);

            let n = self.scanned_count.fetch_add(1, Ordering::Relaxed) + 1;
            self.emitter.emit(&ScanProgress {
                session_id: self.session_id,
                // 与 ps1 路径同一套亚线性插值，进度条表现一致
                percent: interpolate_percent(
                    self.percent_span.0,
                    self.percent_span.1,
                    items_inserted as u64,
                ),
                current_path: it.path.clone(),
                scanned_items: n as u64,
            });
        }

        Ok(ScanSummary {
            items_inserted,
            total_bytes,
            done_seen: true,
        })
    }

    /// 参数化 INSERT 一条 cleanup_item。
    ///
    /// 表 schema（v1__init.sql）：
    /// - `risk_level` 是 TEXT（'L0'/'L1'/'L2'），ps1 给的是 int，这里映射；
    /// - `description` NOT NULL，ps1 协议里没有该字段，给空串兜底。
    ///
    /// `impact_note` / `rebuild_note`：task-086 3.3 从规则库热取。Rust 路径（如
    /// `software_cache`）传规则里的值；ps1 路径协议里没有，传 `None`。
    #[allow(clippy::too_many_arguments)]
    fn insert_item(
        &self,
        module_code: &str,
        category: &str,
        path: &str,
        size_bytes: i64,
        risk_level: i64,
        username: Option<&str>,
        impact_note: Option<&str>,
        rebuild_note: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let risk_text = match risk_level {
            0 => "L0",
            1 => "L1",
            2 => "L2",
            other => {
                eprintln!("[scan] unknown risk_level={other}, clamp to L2");
                "L2"
            }
        };
        // task-084 C1：用户名写进 extra JSON 列（`cleanup_item` 的既有扩展槽位）。
        // 有用户名 → `{"username":"Bob"}`；没有 → 写 `'null'` 与「没扫跨用户」区分，
        // 但避免 `{"username":null}` 这种半吊子 JSON，直接 NULL。
        let extra =
            username.map(|u| format!(r#"{{"username":{u_json}}}"#, u_json = Self::json_escape(u)));
        self.conn.execute(
            "INSERT INTO cleanup_item (session_id, module_code, category, path, size_bytes, risk_level, description, extra, impact_note, rebuild_note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, '', ?7, ?8, ?9)",
            rusqlite::params![
                self.session_id,
                module_code,
                category,
                path,
                size_bytes,
                risk_text,
                extra,
                impact_note,
                rebuild_note
            ],
        )?;
        Ok(())
    }

    /// 把用户名转义后嵌进 JSON 字符串字面量（`extra` 列存的是一段 JSON）。
    ///
    /// Windows 用户名可以含 `"` / `\` / 控制字符，直接 `format!` 会拼坏 JSON。
    fn json_escape(s: &str) -> String {
        s.chars()
            .fold(String::with_capacity(s.len() + 8), |mut acc, c| {
                match c {
                    '"' => acc.push_str("\\\""),
                    '\\' => acc.push_str("\\\\"),
                    '\n' => acc.push_str("\\n"),
                    '\r' => acc.push_str("\\r"),
                    '\t' => acc.push_str("\\t"),
                    c if (c as u32) < 0x20 => {
                        acc.push_str(&format!("\\u{:04x}", c as u32));
                    }
                    c => acc.push(c),
                }
                acc
            })
    }
}

#[cfg(test)]
mod tests {
    use super::interpolate_percent;

    /// task-052 G1：`n == 0` 必须正好落在区间起点（进度条从基线开始，不跳）。
    #[test]
    fn interpolate_starts_at_from() {
        assert_eq!(interpolate_percent(20, 40, 0), 20);
        assert_eq!(interpolate_percent(0, 20, 0), 0);
        assert_eq!(interpolate_percent(80, 100, 0), 80);
    }

    /// task-052 G1：任意 n 都不得越过区间上界，也不得低于下界。
    #[test]
    fn interpolate_stays_within_span() {
        for n in [0u64, 1, 7, 50, 200, 10_000, u64::MAX / 2] {
            let p = interpolate_percent(20, 40, n);
            assert!((20..=40).contains(&p), "n={n} -> percent={p} 越界");
        }
    }

    /// task-052 G1：随 n 单调不减（进度条只能往前走）。
    #[test]
    fn interpolate_is_monotonic() {
        let mut prev = interpolate_percent(0, 100, 0);
        for n in 1..=2000u64 {
            let cur = interpolate_percent(0, 100, n);
            assert!(cur >= prev, "n={n} 出现回退：{prev} -> {cur}");
            prev = cur;
        }
    }

    /// task-052 G1：半程点语义——扫到 K 条时正好走完区间一半。
    #[test]
    fn interpolate_half_at_half_point() {
        // INTERP_HALF_POINT = 50，区间 0..=100 → 50 条时约 50%。
        assert_eq!(interpolate_percent(0, 100, 50), 50);
        // 区间 20..=40（跨度 20）→ 50 条时约 30%。
        assert_eq!(interpolate_percent(20, 40, 50), 30);
    }

    /// task-052 G1：退化区间（from >= to）原样返回 from，不 panic、不回退。
    #[test]
    fn interpolate_degenerate_span_returns_from() {
        assert_eq!(interpolate_percent(0, 0, 123), 0);
        assert_eq!(interpolate_percent(50, 50, 999), 50);
        assert_eq!(interpolate_percent(60, 40, 999), 60);
    }
}
