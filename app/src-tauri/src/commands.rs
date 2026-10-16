//! Tauri command 薄包装：把 engine 纯函数挂到 IPC 边界。
//!
//! 本层只做「前端参数 → engine 入参结构 → 信封」的透传；
//! 业务判定全部在 engine::ipc::commands 里，方便纯内存单测。
//! （Tauri 2 规则：带 State 引用的 async command 必须返回 Result。）
//!
//! debt-cleanup-003 Bug-2：所有路径从 `tauri::State<AppPaths>` 取，
//! 不再用 `std::env::current_dir()`（perMachine 安装后 current_dir 指向只读
//! Program Files，DB/safe_box/rules 更新全部 EACCES）。

use std::path::Path;

use engine::audit_log::{
    cleanup_stats, list_audit_log, AuditLogEntry, AuditLogFilter, CleanupStats,
};
use engine::execute::list_pending::{list_pending_items, PendingItem};
use engine::ipc::commands::dry_run::{handle_dry_run, DryRunReq, DryRunResp};
use engine::ipc::envelope::Envelope;
use engine::ipc::error_codes::{INTERNAL, RULE_CHECKSUM_MISMATCH, RULE_NETWORK_ERROR};
use engine::rules::external_deny_list;
use engine::rules::loader::load as load_rules;
use engine::rules::meta_store::{get_active_rules_meta, upsert_active_rules_meta};
use engine::rules::remote::fetch_remote_meta;
use engine::rules::{
    apply_update, fetch_remote_rules_bytes, has_backup, rollback, ApplyError, ApplyResult,
    CheckUpdateResult, RollbackResult, REMOTE_RULES_META_URL,
};
use engine::scan::ScanOrchestrator;

use crate::paths::AppPaths;
use crate::progress::ProgressState;

/// 加载 rules/ 并取出已展开的外部红线绝对路径列表。
///
/// rules_dir 由 AppPaths 在 setup 时定位（prod 从 bundled 复制到 data_root，
/// dev 回退仓库根 rules/）。加载失败一律保守返回 Err，由调用方转 5001。
fn load_external_deny(rules_dir: &Path) -> Result<Vec<String>, String> {
    let loaded = load_rules(rules_dir).map_err(|e| format!("rules load failed: {e}"))?;
    Ok(external_deny_list(&loaded))
}

/// 入参校验：scope ∈ {full, custom}；target_drive 形如 "C:"。
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
#[tauri::command]
pub async fn scan_start(
    scope: String,
    target_drive: Option<String>,
    progress: tauri::State<'_, ProgressState>,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<engine::ipc::commands::scan_start::ScanStartResp>, String> {
    if validate_scan_input(&scope, &target_drive).is_err() {
        return Ok(Envelope::err(INTERNAL));
    }

    let conn = engine::db::conn::open(&paths.data_root).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (unixepoch(), 'running', ?1)",
        [scope.as_str()],
    )
    .map_err(|e| e.to_string())?;
    let session_id = conn.last_insert_rowid();

    let emitter_guard = progress.0.lock().map_err(|e| e.to_string())?;
    let orch = ScanOrchestrator::new(session_id, &conn, &*emitter_guard);
    let outcome = orch.run_all(&paths.scripts_dir);
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

    Ok(Envelope::ok(
        engine::ipc::commands::scan_start::ScanStartResp { session_id },
    ))
}

/// `cleanup.dryRun`：加载 rules/ 外部红线 + 打开真实 DB，按 id 从 cleanup_item 取路径跑 dryRun。
#[tauri::command]
pub async fn cleanup_dry_run(
    item_ids: Vec<i64>,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<DryRunResp>, String> {
    let external_deny = match load_external_deny(&paths.rules_dir) {
        Ok(v) => v,
        Err(_) => return Ok(Envelope::err(INTERNAL)),
    };

    let conn = engine::db::conn::open(&paths.data_root).map_err(|e| e.to_string())?;
    let resolver = engine::db::resolver::DbItemPathResolver::new(&conn);

    Ok(handle_dry_run(
        DryRunReq { item_ids },
        &resolver,
        &external_deny,
    ))
}

/// `cleanup.execute`：dryRun 通过后用户确认 → 真删文件。
#[tauri::command]
pub async fn cleanup_execute(
    item_ids: Vec<i64>,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<engine::execute::executor::ExecuteSummary>, String> {
    if item_ids.is_empty() {
        return Ok(Envelope::err(engine::ipc::error_codes::DRYRUN_EMPTY));
    }
    let external_deny = match load_external_deny(&paths.rules_dir) {
        Ok(v) => v,
        Err(_) => return Ok(Envelope::err(INTERNAL)),
    };
    let conn = engine::db::conn::open(&paths.data_root).map_err(|e| e.to_string())?;

    let summary = engine::execute::executor::execute_items(
        &item_ids,
        &conn,
        &external_deny,
        |_item_id| -> Box<dyn engine::execute::delete::Deleter + '_> {
            Box::new(engine::execute::safe_box::SafeBoxDeleter::new(
                &paths.box_dir,
                &conn,
                None,
            ))
        },
    );
    Ok(Envelope::ok(summary))
}

/// `safe_box.restore`：按 safe_box id 把文件从保险箱 rename 回原路径。
#[tauri::command]
pub async fn safe_box_restore(
    safe_box_id: i64,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<engine::execute::safe_box::RestoreResult>, String> {
    let conn = engine::db::conn::open(&paths.data_root).map_err(|e| e.to_string())?;

    match engine::execute::safe_box::restore_from_safe_box(&conn, &paths.box_dir, safe_box_id) {
        Ok(r) => Ok(Envelope::ok(r)),
        Err(engine::execute::safe_box::RestoreError::TargetOccupied(_)) => {
            Ok(Envelope::err(engine::ipc::error_codes::FILE_OCCUPIED))
        }
        Err(e) => {
            eprintln!("[safe_box_restore] id={safe_box_id} failed: {e}");
            Ok(Envelope::err(INTERNAL))
        }
    }
}

/// `safe_box.list`：列出当前保险箱里所有未剔除条目（前端 UI 用）。
#[tauri::command]
pub async fn safe_box_list(
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<engine::execute::safe_box::SafeBoxListResp>, String> {
    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[safe_box_list] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    match engine::execute::safe_box::list_safe_box(&conn) {
        Ok(r) => Ok(Envelope::ok(r)),
        Err(e) => {
            eprintln!("[safe_box_list] list: {e}");
            Ok(Envelope::err(INTERNAL))
        }
    }
}

/// `safe_box.purge`：按 safe_box id 永久删除保险箱条目（删磁盘文件 + DB 行 + audit_log）。
#[tauri::command]
pub async fn safe_box_purge(
    item_id: i64,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<bool>, String> {
    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[safe_box_purge] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    match engine::execute::safe_box::purge_item(&conn, &paths.box_dir, item_id) {
        Ok(()) => Ok(Envelope::ok(true)),
        Err(e) => {
            eprintln!("[safe_box_purge] id={item_id} failed: {e}");
            Ok(Envelope::err(INTERNAL))
        }
    }
}

/// `safe_box.purgeMany`：批量永久删除——循环调 purge_item，单个失败不中断。
#[tauri::command]
pub async fn safe_box_purge_many(
    item_ids: Vec<i64>,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<engine::execute::safe_box::PurgeManyResult>, String> {
    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[safe_box_purge_many] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    // 批量 purge 不抛硬错（内部已收集 failed/errors），始终返回 Envelope::ok(...)。
    let r = engine::execute::safe_box::purge_many(&conn, &paths.box_dir, &item_ids);
    Ok(Envelope::ok(r))
}

/// `safe_box.restoreMany`：批量恢复——循环调 restore_from_safe_box，单个失败不中断。
#[tauri::command]
pub async fn safe_box_restore_many(
    item_ids: Vec<i64>,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<engine::execute::safe_box::BatchOpResult>, String> {
    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[safe_box_restore_many] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    // 批量恢复不抛硬错（内部已收集 failed/errors），始终返回 Envelope::ok(...)。
    let r = engine::execute::safe_box::restore_many(&conn, &paths.box_dir, &item_ids);
    Ok(Envelope::ok(r))
}

/// `rules.checkUpdate`：比对本地 rules_meta 与远端 GitHub Raw 版本。
///
/// task-042：远端不可达时不再返回错误信封，而是返回
/// `Envelope::ok(CheckUpdateResult{ status: Offline, ... })`，
/// 前端据此显示黄条「离线模式：使用本地规则库 vX.Y.Z」。
#[tauri::command]
pub async fn rules_check_update(
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<CheckUpdateResult>, String> {
    let local = match load_rules(&paths.rules_dir) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[rules_check_update] load local: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    let local_version = local.meta.version.clone();
    let local_sha = local.meta.checksum_sha256.clone();

    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[rules_check_update] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    if let Err(e) =
        upsert_active_rules_meta(&conn, &local_version, &local_sha, REMOTE_RULES_META_URL)
    {
        eprintln!("[rules_check_update] upsert: {e}");
        return Ok(Envelope::err(INTERNAL));
    }
    let _ = get_active_rules_meta(&conn);

    let remote = match fetch_remote_meta() {
        Ok(r) => r,
        Err(e) => {
            // task-042：离线模式——不返回错误信封，前端按 status=Offline 渲染黄条。
            eprintln!("[rules_check_update] fetch remote offline: {e}");
            return Ok(Envelope::ok(CheckUpdateResult::offline(
                local_version,
                local_sha,
                e,
            )));
        }
    };

    let result = CheckUpdateResult::classify(local_version, local_sha, &remote);
    Ok(Envelope::ok(result))
}

/// `rules.applyUpdate`：下载远端 cache_rules.json 并替换本地，SHA-256 校验通过后 upsert DB。
#[tauri::command]
pub async fn rules_apply_update(
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<ApplyResult>, String> {
    let meta = match fetch_remote_meta() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[rules_apply_update] fetch meta: {e}");
            return Ok(Envelope::err(RULE_NETWORK_ERROR));
        }
    };
    let bytes = match fetch_remote_rules_bytes() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[rules_apply_update] fetch rules: {e}");
            return Ok(Envelope::err(RULE_NETWORK_ERROR));
        }
    };
    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[rules_apply_update] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    match apply_update(&paths.rules_dir, &conn, &meta, &bytes) {
        Ok(r) => Ok(Envelope::ok(r)),
        Err(ApplyError::ShaMismatch { .. }) => {
            eprintln!("[rules_apply_update] sha mismatch, keeping old rules");
            Ok(Envelope::err(RULE_CHECKSUM_MISMATCH))
        }
        Err(e) => {
            eprintln!("[rules_apply_update] apply: {e}");
            Ok(Envelope::err(INTERNAL))
        }
    }
}

/// `rules_has_backup`：返回是否存在可回滚的 `.bak`（前端用来决定是否显示「回滚」按钮）。
#[tauri::command]
pub async fn rules_has_backup(
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<bool>, String> {
    Ok(Envelope::ok(has_backup(&paths.rules_dir)))
}

/// `rules_rollback`：从 `.bak` 恢复 cache_rules.json 到上一版。
///
/// 失败时返回 INTERNAL 错误信封；具体中文原因 eprintln 到 stderr。
#[tauri::command]
pub async fn rules_rollback(
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<RollbackResult>, String> {
    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[rules_rollback] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    match rollback(&paths.rules_dir, &conn) {
        Ok(r) => Ok(Envelope::ok(r)),
        Err(e) => {
            eprintln!("[rules_rollback] {e}");
            Ok(Envelope::err(INTERNAL))
        }
    }
}

/// `audit_log.list`：按 ts 倒序分页返回清理历史条目（只读）。
#[tauri::command]
pub async fn audit_log_list(
    session_id: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<Vec<AuditLogEntry>>, String> {
    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[audit_log_list] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    let filter = AuditLogFilter {
        session_id,
        limit: limit.unwrap_or(100),
        offset: offset.unwrap_or(0),
    };
    match list_audit_log(&conn, &filter) {
        Ok(v) => Ok(Envelope::ok(v)),
        Err(e) => {
            eprintln!("[audit_log_list] list: {e}");
            Ok(Envelope::err(INTERNAL))
        }
    }
}

/// `audit_log.stats`：返回清理结果聚合数字（total_bytes / success / fail / sessions）。
#[tauri::command]
pub async fn audit_log_stats(
    session_id: Option<i64>,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<CleanupStats>, String> {
    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[audit_log_stats] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    match cleanup_stats(&conn, session_id) {
        Ok(s) => Ok(Envelope::ok(s)),
        Err(e) => {
            eprintln!("[audit_log_stats] stats: {e}");
            Ok(Envelope::err(INTERNAL))
        }
    }
}

/// `cleanup.listPending`：列出 cleanup_item 表中所有待清理项（size 倒序）。
#[tauri::command]
pub async fn cleanup_list_pending(
    session_id: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<Vec<PendingItem>>, String> {
    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[cleanup_list_pending] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    match list_pending_items(&conn, session_id, limit.unwrap_or(100), offset.unwrap_or(0)) {
        Ok(v) => Ok(Envelope::ok(v)),
        Err(e) => {
            eprintln!("[cleanup_list_pending] list: {e}");
            Ok(Envelope::err(INTERNAL))
        }
    }
}
