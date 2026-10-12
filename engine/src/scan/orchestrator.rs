//! `ScanOrchestrator`：串跑 5 个 ps1 脚本（task-013）。
//!
//! 与 [`crate::scan::runner::ScanRunner`] 的关系：runner 一次只跑一个脚本，
//! orchestrator 在 engine 层把 5 个脚本按固定顺序串起来，汇总 item/byte 数，
//! 记录成功/失败脚本名。失败脚本不中断整体，5 个全失败才返回 `AllScriptsFailed`。

use std::path::Path;

use rusqlite::Connection;

use crate::ipc::progress::ProgressEmitter;
use crate::scan::runner::ScanRunner;

/// 硬编码 5 个 ps1 文件名（task-011 产出）。
const SCRIPTS: &[&str] = &[
    "scan_temp.ps1",
    "scan_windows_update.ps1",
    "scan_prefetch.ps1",
    "scan_browser_cache.ps1",
    "scan_dev_cache.ps1",
];

/// 5 个脚本跑完的汇总。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OrchestratorSummary {
    /// 累计成功入库的 item 数。
    pub total_items: i64,
    /// 累计 done 事件里的 total_bytes。
    pub total_bytes: i64,
    /// 跑成功的脚本文件名。
    pub scripts_succeeded: Vec<String>,
    /// 失败的脚本文件名（含文件不存在 / run_script 报错）。
    pub scripts_failed: Vec<String>,
}

/// orchestrator 级错误：5 个脚本全部失败。
#[derive(thiserror::Error, Debug)]
pub enum OrchestratorError {
    /// 5 个脚本全部失败（含文件不存在）。
    #[error("全部 {0} 个脚本均失败")]
    AllScriptsFailed(usize),
}

/// 编排器：绑定 session_id + DB + emitter，跑一组脚本。
pub struct ScanOrchestrator<'conn, 'emitter> {
    session_id: i64,
    conn: &'conn Connection,
    emitter: &'emitter dyn ProgressEmitter,
}

impl<'conn, 'emitter> ScanOrchestrator<'conn, 'emitter> {
    /// 构造：给定已存在的 session_id、DB 连接、进度 emitter。
    pub fn new(
        session_id: i64,
        conn: &'conn Connection,
        emitter: &'emitter dyn ProgressEmitter,
    ) -> Self {
        Self {
            session_id,
            conn,
            emitter,
        }
    }

    /// 在 `scripts_dir` 下串跑硬编码的 5 个 ps1。
    ///
    /// - 单个脚本失败不中断；
    /// - 5 个全失败才返回 `Err(AllScriptsFailed)`；
    /// - 至少 1 个成功即返回 `Ok(summary)`。
    pub fn run_all(&self, scripts_dir: &Path) -> Result<OrchestratorSummary, OrchestratorError> {
        let mut summary = OrchestratorSummary::default();

        for name in SCRIPTS {
            let path = scripts_dir.join(name);
            if !path.exists() {
                eprintln!("[orchestrator] missing script: {name}");
                summary.scripts_failed.push(name.to_string());
                continue;
            }
            let runner = ScanRunner::new(self.session_id, self.conn, self.emitter);
            match runner.run_script(&path) {
                Ok(s) => {
                    summary.total_items += s.items_inserted;
                    summary.total_bytes += s.total_bytes;
                    summary.scripts_succeeded.push(name.to_string());
                }
                Err(e) => {
                    eprintln!("[orchestrator] script {name} failed: {e}");
                    summary.scripts_failed.push(name.to_string());
                }
            }
        }

        if summary.scripts_succeeded.is_empty() {
            return Err(OrchestratorError::AllScriptsFailed(SCRIPTS.len()));
        }
        Ok(summary)
    }
}
