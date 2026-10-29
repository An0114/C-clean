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
use engine::execute::list_pending::{list_pending_page, PendingPage, DEFAULT_PAGE_SIZE};
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
        // task-052 G3：把本次 cleanup_task.id 传下去，SafeBoxDeleter 才能写
        // safe_box.source_task_id（原写死 None → 该列恒 NULL，前端 Session 下拉/来源列全空）。
        // task_id < 0 表示 cleanup_task 插入失败，此时仍落 NULL，不写脏值。
        |_item_id, task_id| -> Box<dyn engine::execute::delete::Deleter + '_> {
            let source = if task_id >= 0 { Some(task_id) } else { None };
            Box::new(engine::execute::safe_box::SafeBoxDeleter::new(
                &paths.box_dir,
                &conn,
                source,
            ))
        },
    );
    Ok(Envelope::ok(summary))
}

/// `cleanup.retryLocked`：请求关闭占用进程 → 自动重试删除（task-049）。
///
/// 安全边界：
/// - 只处理 `item_ids` 里的条目，逐个按 id 反查 `cleanup_item.path`，查不到就跳过；
/// - 关闭动作走 `shutdown_locking_processes`，内置护栏（服务/系统/自身/保护名单）
///   命中即拒绝，绝不关闭；
/// - 删除仍复用 `execute_items`，**白名单二次校验照常生效**（L3 红线拦得住）；
/// - 单条 shutdown 失败只打日志，不中断整批。
#[tauri::command]
pub async fn cleanup_retry_locked(
    item_ids: Vec<i64>,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<engine::execute::locking::RetryLockedResp>, String> {
    if item_ids.is_empty() {
        return Ok(Envelope::err(engine::ipc::error_codes::DRYRUN_EMPTY));
    }
    let external_deny = match load_external_deny(&paths.rules_dir) {
        Ok(v) => v,
        Err(_) => return Ok(Envelope::err(INTERNAL)),
    };
    let conn = engine::db::conn::open(&paths.data_root).map_err(|e| e.to_string())?;

    // task-057：在 spawn_blocking 之前读一次偏好（默认 false），闭包按值捕获。
    let auto_restart = engine::prefs::get_auto_restart(&conn);

    // 1. 只收集本次 item_ids 对应的路径（查不到跳过，不中断）。
    let mut targets: Vec<String> = Vec::with_capacity(item_ids.len());
    for id in &item_ids {
        match conn.query_row("SELECT path FROM cleanup_item WHERE id = ?1", [id], |r| {
            r.get::<_, String>(0)
        }) {
            Ok(p) => targets.push(p),
            Err(_) => eprintln!("[cleanup_retry_locked] id={id} not found, skipped"),
        }
    }

    // 2. RmShutdown 是阻塞 FFI（最坏等目标进程响应，实测可达 ~30s），
    //    丢进 blocking 线程池，避免占住 async runtime 拖慢其他 IPC。
    let shutdown = tauri::async_runtime::spawn_blocking(move || {
        let mut acc = engine::execute::locking::ShutdownOutcome::default();
        // 同一进程可能占用多个文件，按 pid 去重，避免前端计数虚高。
        let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for p in targets {
            match engine::execute::locking::shutdown_locking_processes(&p, auto_restart) {
                Ok(outcome) => {
                    for lp in outcome.closed {
                        if seen.insert(lp.pid) {
                            acc.closed.push(lp);
                        }
                    }
                    for r in outcome.refused {
                        if seen.insert(r.pid) {
                            acc.refused.push(r);
                        }
                    }
                }
                // 单条失败不中断整批（与 task-038 purge_many 风格一致）。
                Err(e) => eprintln!("[cleanup_retry_locked] shutdown failed for {p}: {e}"),
            }
        }
        acc
    })
    .await
    .map_err(|e| format!("shutdown task failed: {e}"))?;

    // 3. 复用 cleanup_execute 的同一段删除编排（禁止第二套删除逻辑）。
    let summary = engine::execute::executor::execute_items(
        &item_ids,
        &conn,
        &external_deny,
        // task-052 G3：与 cleanup_execute 保持一致，把本次 cleanup_task.id 传给 deleter。
        |_item_id, task_id| -> Box<dyn engine::execute::delete::Deleter + '_> {
            let source = if task_id >= 0 { Some(task_id) } else { None };
            Box::new(engine::execute::safe_box::SafeBoxDeleter::new(
                &paths.box_dir,
                &conn,
                source,
            ))
        },
    );

    Ok(Envelope::ok(engine::execute::locking::RetryLockedResp {
        shutdown,
        summary,
    }))
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

/// `safe_box.list`：分页列出当前保险箱里未剔除条目（task-045 / task-051）。
///
/// - `page` 从 1 起；`page_size` 后端 clamp 到 [1,200]；
/// - `session_ids` 可选：按 safe_box.source_task_id 组合过滤（空/缺省 = 全部）；
/// - `from_ts` / `to_ts` 可选：按 quarantined_at 的闭区间过滤（Unix 秒，<=0 视为未设置）；
/// - 返回 `SafeBoxPage { items, total, page, pageSize, totalBytes }`，
///   其中 `total` / `totalBytes` 与筛选条件一致。
#[tauri::command]
pub async fn safe_box_list(
    page: u32,
    page_size: u32,
    session_ids: Option<Vec<i64>>,
    from_ts: Option<i64>,
    to_ts: Option<i64>,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<engine::execute::safe_box::SafeBoxPage>, String> {
    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[safe_box_list] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    let filter = engine::execute::safe_box::SafeBoxFilter {
        session_ids: session_ids.unwrap_or_default(),
        from_ts,
        to_ts,
    };
    match engine::execute::safe_box::list_paged(&conn, page, page_size, &filter) {
        Ok(r) => Ok(Envelope::ok(r)),
        Err(e) => {
            eprintln!("[safe_box_list] list_paged: {e}");
            Ok(Envelope::err(INTERNAL))
        }
    }
}

/// `safe_box.item_history`：按 safe_box id 查该条目原路径的审计历史（task-051，只读）。
///
/// - 入参 `safe_box_id`（前端 camelCase `safeBoxId`）；
/// - 返回 `AuditLogEntry[]`（camelCase：eventType/path/detail/createdAt/result）；
/// - 条目不存在 → `INTERNAL`（engine 侧 `safe_box item not found`）。
#[tauri::command]
pub async fn safe_box_item_history(
    safe_box_id: i64,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<Vec<engine::audit_log::AuditLogEntry>>, String> {
    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[safe_box_item_history] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    match engine::execute::safe_box::item_history(&conn, safe_box_id) {
        Ok(r) => Ok(Envelope::ok(r)),
        Err(e) => {
            eprintln!("[safe_box_item_history] item_history: {e}");
            Ok(Envelope::err(INTERNAL))
        }
    }
}

/// `safe_box.list_sessions`：按 source_task_id 分组列出有哪些 session 进过保险箱（task-046）。
#[tauri::command]
pub async fn safe_box_list_sessions(
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<Vec<engine::execute::safe_box::SessionSummary>>, String> {
    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[safe_box_list_sessions] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    match engine::execute::safe_box::list_sessions(&conn) {
        Ok(r) => Ok(Envelope::ok(r)),
        Err(e) => {
            eprintln!("[safe_box_list_sessions] list_sessions: {e}");
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
pub async fn rules_has_backup(paths: tauri::State<'_, AppPaths>) -> Result<Envelope<bool>, String> {
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

/// `cleanup.listPending`：分页列出 cleanup_item 表中所有待清理项（size 倒序）。
///
/// task-052 G2：入参由 `limit/offset` 改为 `page/page_size`，返回
/// `PendingPage { items, total, page, pageSize }`。原前端硬编码 `limit: 100`
/// 导致第 101 条之后的条目在 UI 上不可见（走查 G2 缺陷）。
#[tauri::command]
pub async fn cleanup_list_pending(
    session_id: Option<i64>,
    page: Option<u32>,
    page_size: Option<u32>,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<PendingPage>, String> {
    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[cleanup_list_pending] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    match list_pending_page(
        &conn,
        session_id,
        page.unwrap_or(1),
        page_size.unwrap_or(DEFAULT_PAGE_SIZE),
    ) {
        Ok(v) => Ok(Envelope::ok(v)),
        Err(e) => {
            eprintln!("[cleanup_list_pending] list: {e}");
            Ok(Envelope::err(INTERNAL))
        }
    }
}

/// `get_prefs`：读用户偏好（task-057）。新用户缺行时默认 auto_restart_on_close=false。
#[tauri::command]
pub async fn get_prefs(
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<engine::prefs::Prefs>, String> {
    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[get_prefs] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    Ok(Envelope::ok(engine::prefs::Prefs::load(&conn)))
}

/// `set_prefs`：写用户偏好（task-057）。目前只支持 autoRestartOnClose 一个开关。
#[tauri::command]
pub async fn set_prefs(
    prefs: engine::prefs::Prefs,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<engine::prefs::Prefs>, String> {
    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[set_prefs] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    if let Err(e) = engine::prefs::set_auto_restart(&conn, prefs.auto_restart_on_close) {
        eprintln!("[set_prefs] write: {e}");
        return Ok(Envelope::err(INTERNAL));
    }
    Ok(Envelope::ok(engine::prefs::Prefs::load(&conn)))
}
