//! Tauri command 薄包装：把 engine 纯函数挂到 IPC 边界。
//!
//! 本层只做「前端参数 → engine 入参结构 → 信封」的透传；
//! 业务判定全部在 engine::ipc::commands 里，方便纯内存单测。
//! （Tauri 2 规则：带 State 引用的 async command 必须返回 Result。）

use engine::ipc::commands::dry_run::{handle_dry_run, DryRunReq, DryRunResp, MockResolver};
use engine::ipc::commands::scan_start::{handle_scan_start, ScanStartReq, ScanStartResp};
use engine::ipc::envelope::Envelope;

/// `scan.start`：立即返回递增 sessionId（本任务不真启动扫描）。
/// sessionId 计数器存于 Tauri State（Mutex<i64>）。
#[tauri::command]
pub async fn scan_start(
    scope: String,
    target_drive: Option<String>,
    state: tauri::State<'_, std::sync::Mutex<i64>>,
) -> Result<Envelope<ScanStartResp>, String> {
    let mut counter = state.lock().map_err(|e| e.to_string())?;
    Ok(handle_scan_start(
        ScanStartReq {
            scope,
            target_drive,
        },
        &mut counter,
    ))
}

/// `cleanup.dryRun`：本任务用空 MockResolver（不接 DB），
/// 非空 itemIds 一律回 2001；真实 resolver 注入留后续任务。
#[tauri::command]
pub async fn cleanup_dry_run(item_ids: Vec<i64>) -> Result<Envelope<DryRunResp>, String> {
    let resolver = MockResolver::new(vec![]);
    Ok(handle_dry_run(DryRunReq { item_ids }, &resolver))
}
