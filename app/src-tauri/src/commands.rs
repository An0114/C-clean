//! Tauri command 薄包装：把 engine 纯函数挂到 IPC 边界。
//!
//! 本层只做「前端参数 → engine 入参结构 → 信封」的透传；
//! 业务判定全部在 engine::ipc::commands 里，方便纯内存单测。
//! （Tauri 2 规则：带 State 引用的 async command 必须返回 Result。）

use engine::ipc::commands::dry_run::{handle_dry_run, DryRunReq, DryRunResp, MockResolver};
use engine::ipc::envelope::Envelope;
use engine::ipc::error_codes::INTERNAL;
use engine::rules::external_deny_list;
use engine::rules::loader::load as load_rules;
use engine::scan::ScanOrchestrator;

use crate::progress::ProgressState;

/// 加载 rules/ 并取出已展开的外部红线绝对路径列表。
///
/// 开发期硬编码仓库根 `rules/`（`current_dir`）；M1 打包后改为 Tauri 资源目录。
/// 加载失败（checksum 不匹配 / IO / JSON 错）一律保守返回 Err，由调用方转 5001。
fn load_external_deny() -> Result<Vec<String>, String> {
    let rules_dir = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .join("rules");
    let loaded = load_rules(&rules_dir).map_err(|e| format!("rules load failed: {e}"))?;
    Ok(external_deny_list(&loaded))
}

/// 入参校验：scope ∈ {full, custom}；target_drive 形如 "C:"。
/// 与 engine::ipc::commands::scan_start::handle_scan_start 的规则保持一致。
fn validate_scan_input(scope: &str, target_drive: &Option<String>) -> Result<(), ()> {
    if scope != "full" && scope != "custom" {
        return Err(());
    }
    if let Some(drive) = target_drive {
        let b = drive.as_bytes();
        if !(b.len() == 2 && b[0].is_ascii_alphabetic() && b[1] == b':') {
            return Err(());
        }
    }
    Ok(())
}

/// `scan.start`：task-013 起真接通扫描。
///
/// 1. 入参校验；
/// 2. 打开（必要时自动迁移）SQLite 库；
/// 3. `INSERT INTO scan_session ... RETURNING id` 拿真实 session_id；
/// 4. `spawn_blocking` 内串跑 5 个 ps1（`ScanOrchestrator::run_all`），
///    item 行写 cleanup_item、并通过 ProgressState emit 进度；
/// 5. 回写 scan_session.status='completed'/'failed'；
/// 6. 返回真实 session_id。
#[tauri::command]
pub async fn scan_start(
    scope: String,
    target_drive: Option<String>,
    progress: tauri::State<'_, ProgressState>,
    app: tauri::AppHandle,
) -> Result<Envelope<engine::ipc::commands::scan_start::ScanStartResp>, String> {
    if validate_scan_input(&scope, &target_drive).is_err() {
        return Ok(Envelope::err(INTERNAL));
    }

    // DB 数据根：开发期用 current_dir（与 rules/、scripts/ 一致）；
    // 打包后改为 `app.path().app_data_dir()`。
    let data_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let scripts_dir = data_root.join("scripts");

    // 注：task-013 暂未用 spawn_blocking——5 个 ps1 串跑约数秒，M1 dev 期
    // 在 async command 里直接同步执行可接受；后续引入 scan.cancel 时再改线程模型。
    let conn = engine::db::conn::open(&data_root).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (unixepoch(), 'running', ?1)",
        [scope.as_str()],
    )
    .map_err(|e| e.to_string())?;
    let session_id = conn.last_insert_rowid();

    let emitter_guard = progress.0.lock().map_err(|e| e.to_string())?;
    let orch = ScanOrchestrator::new(session_id, &conn, &*emitter_guard);
    let outcome = orch.run_all(&scripts_dir);
    drop(emitter_guard);

    let status = match &outcome {
        Ok(_) => "completed",
        Err(_) => "failed",
    };
    conn.execute(
        "UPDATE scan_session SET status = ?1, finished_at = unixepoch() WHERE id = ?2",
        (&status, &session_id),
    )
    .map_err(|e| e.to_string())?;

    let msg = match outcome {
        Ok(s) => format!(
            "items={} bytes={} ok=[{}] failed=[{}]",
            s.total_items,
            s.total_bytes,
            s.scripts_succeeded.join(","),
            s.scripts_failed.join(",")
        ),
        Err(e) => format!("orchestrator failed: {e}"),
    };
    eprintln!("[scan_start] session={session_id} {msg}");
    let _ = &app; // 预留：打包后用 app.path() 切数据目录。

    Ok(Envelope::ok(
        engine::ipc::commands::scan_start::ScanStartResp { session_id },
    ))
}

/// `cleanup.dryRun`：加载 rules/ 外部红线后跑 dryRun；rules 加载失败返 5001（保守拒绝）。
#[tauri::command]
pub async fn cleanup_dry_run(item_ids: Vec<i64>) -> Result<Envelope<DryRunResp>, String> {
    // 加载失败（checksum 不匹配等）→ 保守返回 5001，不静默放行。
    let external_deny = match load_external_deny() {
        Ok(v) => v,
        Err(_) => return Ok(Envelope::err(INTERNAL)),
    };
    let resolver = MockResolver::new(vec![]);
    Ok(handle_dry_run(
        DryRunReq { item_ids },
        &resolver,
        &external_deny,
    ))
}
