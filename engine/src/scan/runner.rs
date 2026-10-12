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

/// 扫描 runner：绑定一个 DB 连接 + 一个 emitter，跑多个脚本时复用。
pub struct ScanRunner<'conn, 'emitter> {
    session_id: i64,
    conn: &'conn Connection,
    emitter: &'emitter dyn ProgressEmitter,
    /// 已成功入库的 item 数（跨脚本累计）。
    scanned_count: AtomicI64,
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
        }
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
                    if let Err(e) =
                        self.insert_item(&module_code, &category, &path, size_bytes, risk_level)
                    {
                        // DB 错视为致命：不继续往表里灌半份数据。
                        return Err(RunError::Db(e));
                    }
                    items_inserted += 1;
                    let n = self.scanned_count.fetch_add(1, Ordering::Relaxed) + 1;
                    self.emitter.emit(&ScanProgress {
                        session_id: self.session_id,
                        // 单脚本内 percent 未知，统一给 0；后续编排脚本跑完再换算百分比。
                        percent: 0,
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

    /// 参数化 INSERT 一条 cleanup_item。
    ///
    /// 表 schema（v1__init.sql）：
    /// - `risk_level` 是 TEXT（'L0'/'L1'/'L2'），ps1 给的是 int，这里映射；
    /// - `description` NOT NULL，ps1 协议里没有该字段，给空串兜底。
    fn insert_item(
        &self,
        module_code: &str,
        category: &str,
        path: &str,
        size_bytes: i64,
        risk_level: i64,
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
        self.conn.execute(
            "INSERT INTO cleanup_item (session_id, module_code, category, path, size_bytes, risk_level, description)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, '')",
            rusqlite::params![self.session_id, module_code, category, path, size_bytes, risk_text],
        )?;
        Ok(())
    }
}
