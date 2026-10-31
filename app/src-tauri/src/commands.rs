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

/// task-063：扫描取消标志（全局单例，setup 里注入）。
///
/// 用 `Arc<AtomicBool>` 而不是 `Mutex<bool>`：只需要「置位 / 读取」两个原子操作，
/// 且 `scan_cancel` 必须**立刻**生效（不能被正在跑的扫描持有锁挡住）。
#[derive(Default)]
pub struct ScanCancelState(pub std::sync::Arc<std::sync::atomic::AtomicBool>);

/// `scan_cancel`：请求取消当前扫描（task-063）。
///
/// **立即返回**，不等扫描真的停下——真正的停止发生在下一个脚本开始前
/// （`ScanOrchestrator` 在每轮循环开头检查标志）。这样 IPC 不会卡住 UI。
///
/// 取消后**已扫描的结果保留**（`cleanup_item` 的行不删），用户刷新列表就能看到。
#[tauri::command]
pub fn scan_cancel(cancel: tauri::State<'_, ScanCancelState>) -> Result<Envelope<bool>, String> {
    cancel.0.store(true, std::sync::atomic::Ordering::Relaxed);
    eprintln!("[scan_cancel] 已置取消标志");
    Ok(Envelope::ok(true))
}

/// `scan.start`：task-013 起真接通扫描。
#[tauri::command]
pub async fn scan_start(
    scope: String,
    target_drive: Option<String>,
    progress: tauri::State<'_, ProgressState>,
    cancel: tauri::State<'_, ScanCancelState>,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<engine::ipc::commands::scan_start::ScanStartResp>, String> {
    if validate_scan_input(&scope, &target_drive).is_err() {
        return Ok(Envelope::err(INTERNAL));
    }
    // task-063：每次开始前复位取消标志，否则上一次的「取消」会让这次一启动就停。
    cancel.0.store(false, std::sync::atomic::Ordering::Relaxed);

    let conn = engine::db::conn::open(&paths.data_root).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (unixepoch(), 'running', ?1)",
        [scope.as_str()],
    )
    .map_err(|e| e.to_string())?;
    let session_id = conn.last_insert_rowid();

    let emitter_guard = progress.0.lock().map_err(|e| e.to_string())?;
    let orch =
        ScanOrchestrator::new(session_id, &conn, &*emitter_guard).with_cancel(cancel.0.clone());
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

/// 定位提权 exe（task-062）。
///
/// 三个候选，按优先级：
/// 1. **安装态**：`bundle.resources` 把 `cclean-elevation.exe` 映射到安装目录**根**，
///    而 `scripts_dir` 就是 `<resource>/scripts`，所以 `parent()` 就是 resource 根；
/// 2. **本机打包后**：workspace 根的 `target/release/`；
/// 3. **dev**：`target/debug/`。
fn locate_elevation_exe(paths: &AppPaths) -> Option<std::path::PathBuf> {
    if let Some(root) = paths.scripts_dir.parent() {
        let p = root.join("cclean-elevation.exe");
        if p.is_file() {
            return Some(p);
        }
    }
    // dev / 本机：从当前工作目录往上找 workspace 根的 target。
    // tauri dev 的 cwd 是 `app/src-tauri`，`tauri build` 的 cwd 是 `app/`，
    // 所以两个层级都要试。
    let mut base = std::env::current_dir().ok()?;
    for _ in 0..3 {
        for profile in ["release", "debug"] {
            let p = base
                .join("target")
                .join(profile)
                .join("cclean-elevation.exe");
            if p.is_file() {
                return Some(p);
            }
        }
        if !base.pop() {
            break;
        }
    }
    None
}

/// `cleanup.execute`：dryRun 通过后用户确认 → 真删文件。
#[tauri::command]
pub async fn cleanup_execute(
    item_ids: Vec<i64>,
    // task-062：用户在「系统还原点创建失败，是否继续清理？」确认框里点「继续」时传 true。
    // 不传 / 传 false = 需要还原点时必须先建成功。
    skip_restore_point: Option<bool>,
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

    // task-062：含 L2 风险项时，清理前必须建系统还原点。
    //
    // fail-closed：`needs_restore_point` 返回 Err（查库失败）时**按需要处理**，
    // 不能因为查不清就裸奔清理。
    if skip_restore_point != Some(true) {
        let need = engine::execute::executor::needs_restore_point(&conn, &item_ids);
        match need {
            Ok(false) => {}
            Ok(true) | Err(_) => {
                let exe = match locate_elevation_exe(&paths) {
                    Some(p) => p,
                    None => {
                        eprintln!("[cleanup_execute] 提权 exe 未找到");
                        return Ok(Envelope::err(engine::ipc::error_codes::RESTOREPOINT_FAILED));
                    }
                };
                if let Err(msg) = engine::elevation::ensure_restore_point(&conn, &exe) {
                    eprintln!("[cleanup_execute] 还原点创建失败: {msg}");
                    // 2007：前端据此弹「是否继续清理？」确认框，用户点继续后带
                    // skipRestorePoint=true 重调本命令。原因只在日志里——
                    // Envelope 的 data 类型被 ExecuteSummary 钉死，塞不进字符串。
                    return Ok(Envelope::err(engine::ipc::error_codes::RESTOREPOINT_FAILED));
                }
            }
        }
    }

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
    // task-060：需要 AppHandle 才能把「已等待 Xs」的进度事件推给前端。
    // 这是 Tauri 2 的命令注入参数，不由前端传。
    app: tauri::AppHandle,
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
    // task-058：白名单同一次读好一起捕获（空 = 不限制）。
    let auto_restart = engine::prefs::get_auto_restart(&conn);
    let restart_whitelist = engine::prefs::get_restart_whitelist(&conn);

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
    //
    // task-060：同时把「已等待秒数」每秒推给前端。engine 不认识 Tauri，
    // 只接受一个 `Arc<dyn Fn(u32)>` 回调，emit 在这里做——保持分层干净。
    // 事件名 `shutdown://progress`，payload `{ elapsedSecs, expectedMaxSecs }`。
    let app_for_progress = app.clone();
    let on_progress: engine::execute::locking::ProgressCallback =
        std::sync::Arc::new(move |secs: u32| {
            use tauri::Emitter;
            let _ = app_for_progress.emit(
                "shutdown://progress",
                serde_json::json!({
                    "elapsedSecs": secs,
                    // 30s 是 RM 内部对每个应用的处理上限（经验值），不是我们能设的超时。
                    // 前端用它画进度条，并在 secs > 30 时提示「仍在等待」。
                    "expectedMaxSecs": 30,
                }),
            );
        });

    let shutdown = tauri::async_runtime::spawn_blocking(move || {
        let mut acc = engine::execute::locking::ShutdownOutcome::default();
        // 同一进程可能占用多个文件，按 pid 去重，避免前端计数虚高。
        let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for p in targets {
            match engine::execute::locking::shutdown_locking_processes(
                &p,
                auto_restart,
                &restart_whitelist,
                // 每个文件独立关闭，计时各自从 1s 重新开始（前端会看到重置，
                // 属预期：这是「当前这个文件等了多久」而不是「整批等了多久」）。
                Some(on_progress.clone()),
            ) {
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

/// `audit_log.trend`：最近 N 天每日释放量（task-061 趋势图数据源）。
///
/// `days` 由前端传（默认 30），engine 侧会夹到 1..=365。
/// 只返回**有记录**的日期，缺失日期由前端补 0（理由见 `cleanup_stats_trend` 注释）。
#[tauri::command]
pub async fn cleanup_stats_trend(
    days: Option<i64>,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<Vec<engine::audit_log::stats::TrendPoint>>, String> {
    let conn = match engine::db::conn::open(&paths.data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[cleanup_stats_trend] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    match engine::audit_log::stats::cleanup_stats_trend(&conn, days.unwrap_or(30)) {
        Ok(points) => Ok(Envelope::ok(points)),
        Err(e) => {
            eprintln!("[cleanup_stats_trend] trend: {e}");
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
    // task-058：白名单与开关分开两个 key 存（开关是 bool、白名单是 JSON 数组），
    // 一起写成功才算保存成功；任一步失败都返回 INTERNAL 让前端回滚 UI。
    if let Err(e) = engine::prefs::set_restart_whitelist(&conn, &prefs.restart_whitelist) {
        eprintln!("[set_prefs] write whitelist: {e}");
        return Ok(Envelope::err(INTERNAL));
    }
    // task-061：主题。非法值由 set_theme 内部归一化成 system（不报错）。
    if let Err(e) = engine::prefs::set_theme(&conn, &prefs.theme) {
        eprintln!("[set_prefs] write theme: {e}");
        return Ok(Envelope::err(INTERNAL));
    }
    // task-061：自动扫描开关与间隔。间隔非法值由 set_auto_scan_interval 归一化成 daily。
    if let Err(e) = engine::prefs::set_auto_scan_enabled(&conn, prefs.auto_scan_enabled) {
        eprintln!("[set_prefs] write auto_scan_enabled: {e}");
        return Ok(Envelope::err(INTERNAL));
    }
    if let Err(e) = engine::prefs::set_auto_scan_interval(&conn, &prefs.auto_scan_interval) {
        eprintln!("[set_prefs] write auto_scan_interval: {e}");
        return Ok(Envelope::err(INTERNAL));
    }
    Ok(Envelope::ok(engine::prefs::Prefs::load(&conn)))
}

// ===================== task-063：报告导出 =====================

/// `report_export`：把一次清理结果渲染成 HTML / CSV 并落盘。
///
/// **入参带整个 `summary`**：清理结果只存在于前端内存（DB 里 `cleanup_item` 的
/// 行在删除后状态已变，重建不出「哪条失败、为什么」），所以由前端把结果原样传回。
/// 这也让「导出」是纯函数式的——不依赖任何后端会话状态。
///
/// 返回落盘后的**绝对路径**，前端可展示给用户「报告已保存到 …」。
#[tauri::command]
pub async fn report_export(
    format: String,
    summary: engine::execute::executor::ExecuteSummary,
    drive: Option<String>,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<String>, String> {
    let drive = drive.unwrap_or_else(|| "C".to_string());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let snapshot = engine::report::DriveSnapshot {
        before_used_bytes: summary.disk_before_bytes,
        after_used_bytes: summary.disk_after_bytes,
    };
    let meta = engine::report::ReportMeta {
        generated_at: now,
        drive: &drive,
        snapshot,
    };

    // 只认两种格式，其余按 html 处理（前端只会传这两个值）。
    let (content, ext) = if format == "csv" {
        (engine::report::build_csv(&summary), "csv")
    } else {
        (engine::report::build_html(&summary, &meta), "html")
    };

    let filename = format!("cleanup-report-{now}.{ext}");
    let dir = paths.data_root.join("reports");
    match engine::report::write_report(&dir, &filename, &content) {
        Ok(p) => Ok(Envelope::ok(p.to_string_lossy().to_string())),
        Err(e) => {
            eprintln!("[report_export] {e}");
            Ok(Envelope::err(INTERNAL))
        }
    }
}

// ===================== task-063：磁盘空间 =====================

/// `drive_usage`：查某个盘的总量/已用/可用（仪表盘环形图数据源）。
///
/// `drive` 不传时默认 `"C"`——本工具主要面向 C 盘，前端可以只在需要时显式指定。
#[tauri::command]
pub async fn drive_usage(
    drive: Option<String>,
) -> Result<Envelope<engine::report::DriveUsage>, String> {
    let d = drive.unwrap_or_else(|| "C".to_string());
    match engine::report::drive_usage(&d) {
        Ok(u) => Ok(Envelope::ok(u)),
        Err(e) => {
            eprintln!("[drive_usage] {e}");
            Ok(Envelope::err(INTERNAL))
        }
    }
}

// ===================== task-062：托盘 / 开机自启 =====================

/// `quit_app`：真正退出进程。
///
/// 托盘菜单的「退出」**不会**直接调它——先 emit `tray://quit-request` 让前端弹
/// 二次确认，用户点「确定」后才调到这里。PRD 要求「退出二次确认」，而托盘菜单
/// 回调里没有 WebView，弹不了 `confirm`。
#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    eprintln!("[quit_app] 用户确认退出，进程结束");
    app.exit(0);
}

/// `get_autostart_enabled`：开机自启是否开启。
///
/// **真实来源是系统注册表**（插件封装了 `HKCU\...\Run`），不另存一份 prefs——
/// 否则用户在「任务管理器 → 启动」里手动禁用了，我们的 prefs 还写着 true，
/// 两边打架。读插件状态永远是对的。
#[tauri::command]
pub fn get_autostart_enabled(app: tauri::AppHandle) -> Result<Envelope<bool>, String> {
    use tauri_plugin_autostart::ManagerExt;
    match app.autolaunch().is_enabled() {
        Ok(v) => Ok(Envelope::ok(v)),
        Err(e) => {
            // 读不到不等于「没开」——但要给前端一个确定答案，按未开启处理并留日志。
            eprintln!("[autostart] is_enabled 失败: {e}");
            Ok(Envelope::err(INTERNAL))
        }
    }
}

/// `set_autostart_enabled`：开 / 关开机自启。返回**写入后**的真实状态。
#[tauri::command]
pub fn set_autostart_enabled(
    app: tauri::AppHandle,
    enabled: bool,
) -> Result<Envelope<bool>, String> {
    use tauri_plugin_autostart::ManagerExt;
    let mgr = app.autolaunch();
    let r = if enabled { mgr.enable() } else { mgr.disable() };
    if let Err(e) = r {
        eprintln!("[autostart] set({enabled}) 失败: {e}");
        return Ok(Envelope::err(INTERNAL));
    }
    // 回读一次，把系统真实状态返回给前端（避免 UI 与实际不一致）。
    let actual = mgr.is_enabled().unwrap_or(enabled);
    Ok(Envelope::ok(actual))
}

// ===================== task-065：WinSxS 组件存储（C 模块） =====================

/// `winsxs_analyze`：跑 `dism /online /cleanup-image /analyzecomponentstore`。
///
/// **会弹 UAC**（DISM 必须管理员权限，实测非提权返回错误 740）。所以只能由用户
/// 主动点「分析」触发，绝不能在页面加载时自动跑。
///
/// # 为什么失败走 `Err(String)` 而不是信封
///
/// DISM 的失败原因（退出码 + 输出尾部）必须让用户看到，但 `Envelope` 的
/// `message` 是 `&'static str`、`data` 被出参类型钉死，动态文本塞不进去。
/// 这与本文件里 `engine::db::conn::open(...).map_err(|e| e.to_string())?` 的
/// 处理方式一致：**环境/权限类失败走 `Err`，业务判定失败才走信封**。
#[tauri::command]
pub async fn winsxs_analyze(
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<engine::scan::winsxs::WinsxsAnalysis>, String> {
    let exe = locate_elevation_exe(&paths).ok_or_else(|| {
        eprintln!("[winsxs_analyze] 提权 exe 未找到");
        "提权组件缺失，无法分析组件存储（请重新安装）".to_string()
    })?;

    let analysis = engine::scan::winsxs::analyze(&exe).map_err(|e| {
        eprintln!("[winsxs_analyze] {e}");
        e
    })?;

    eprintln!(
        "[winsxs_analyze] reclaimable={}B packages={} recommended={}",
        analysis.reclaimable_bytes, analysis.reclaimable_packages, analysis.recommended_cleanup
    );
    Ok(Envelope::ok(analysis))
}

/// `winsxs_cleanup`：跑 `dism /online /cleanup-image /startcomponentcleanup`。
///
/// # 会弹两次 UAC
///
/// 1. 建系统还原点（提权命令 `create_restore_point`）；
/// 2. 跑 DISM（提权命令 `dism_cleanup`）。
///
/// 两个独立的提权进程——还原点必须**先落地**才动手清，合并成一个进程反而做不到
/// 「建点失败就中止」的语义。
///
/// # `skipRestorePoint`
///
/// 还原点建失败时返回 2007（RESTOREPOINT_FAILED），前端据此弹「是否继续清理？」；
/// 用户点继续 → 带 `skipRestorePoint: true` 重调本命令。
///
/// # 进度
///
/// 每读到 DISM 的一行就 emit 一个 `winsxs://progress` 事件。emit 失败（窗口已关）
/// 只打日志——进度通道绝不能影响清理本身。
#[tauri::command]
pub async fn winsxs_cleanup(
    app: tauri::AppHandle,
    skip_restore_point: Option<bool>,
    paths: tauri::State<'_, AppPaths>,
) -> Result<Envelope<engine::scan::winsxs::WinsxsCleanupResult>, String> {
    use tauri::Emitter;

    let exe = locate_elevation_exe(&paths).ok_or_else(|| {
        eprintln!("[winsxs_cleanup] 提权 exe 未找到");
        "提权组件缺失，无法清理组件存储（请重新安装）".to_string()
    })?;
    let conn = engine::db::conn::open(&paths.data_root).map_err(|e| e.to_string())?;

    let on_line = |seq: u64, line: &str| {
        let payload = engine::scan::winsxs::WinsxsProgress {
            seq,
            line: line.to_string(),
        };
        if let Err(e) = app.emit(engine::scan::winsxs::WINSXS_PROGRESS_EVENT, payload) {
            eprintln!("[winsxs_cleanup] emit progress failed: {e}");
        }
    };

    match engine::scan::winsxs::cleanup(&conn, &exe, skip_restore_point == Some(true), &on_line) {
        Ok(r) => {
            eprintln!(
                "[winsxs_cleanup] ok={} exit={} restorePoint={:?} lines={}",
                r.ok, r.exit_code, r.restore_point_seq, r.line_count
            );
            Ok(Envelope::ok(r))
        }
        Err(engine::scan::winsxs::WinsxsCleanupError::RestorePoint(m)) => {
            // 2007：前端弹「是否继续清理？」，用户点继续后带 skipRestorePoint 重调。
            // 原因只写日志——Envelope 的 data 类型被出参钉死，塞不进字符串。
            eprintln!("[winsxs_cleanup] 还原点创建失败: {m}");
            Ok(Envelope::err(engine::ipc::error_codes::RESTOREPOINT_FAILED))
        }
        Err(e) => {
            // 提权链路失败（UAC 被取消等）：原因必须让用户看到。
            eprintln!("[winsxs_cleanup] {e}");
            Err(e.to_string())
        }
    }
}

// ===================== task-066：大文件地图（E 模块） =====================

/// 系统盘根路径，形如 `C:/`。
///
/// **不写死 `C:/`**：系统装在 D 盘的机器上，写死会让扫描永远返回空。
/// `SystemDrive` 是 Windows 安装时就定好的环境变量（值形如 `C:`），最可靠。
fn system_drive_root() -> std::path::PathBuf {
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
    std::path::PathBuf::from(format!("{drive}\\"))
}

/// `big_files_scan`：大文件地图（task-066）。
///
/// # 只读
///
/// 扫 hiberfil.sys / Windows.old / 回收站 / Top100，**一个字节都不删**。
/// 这四类的正规处理方式各不相同（`powercfg /h off`、磁盘清理、清空回收站、
/// 用户自己判断），所以本命令只负责「量出来 + 给建议」。
///
/// # 为什么是 async
///
/// Top100 要遍历系统盘的用户目录，几万到几十万条目，同步跑会卡住 IPC 线程池。
/// async 让它跑在 Tauri 的异步执行器上，UI 不受影响。
///
/// # 耗时保护
///
/// engine 侧有「20 万条目 / 20 秒」双上限，撞到就返回 `truncated: true`——
/// 宁可少列几条，也不能让一次点击变成「转圈几分钟」。
#[tauri::command]
pub async fn big_files_scan() -> Result<Envelope<engine::scan::big_files::BigFilesReport>, String> {
    let root = system_drive_root();
    let env = engine::ipc::commands::big_files::handle_big_files_scan(&root);

    // 只打一行摘要：扫描结果的明细可能有 100 条，全打出来日志没法看。
    if let Some(r) = &env.data {
        eprintln!(
            "[big_files_scan] root={} total={}B top={} scanned={} truncated={} {}ms",
            root.display(),
            r.total_bytes,
            r.top_files.len(),
            r.scanned_entries,
            r.truncated,
            r.elapsed_ms
        );
    }
    Ok(env)
}

// ===================== task-067：残留增强（D 模块） =====================

/// `residual_scan`：注册表卸载列表 ↔ 文件目录交叉比对。
///
/// 只读——扫注册表和目录，不改任何东西。会返回两类：
/// - **类型 A**（`confidence=high`）：注册表有 `InstallLocation` 但目录已不存在；
/// - **类型 B**（`confidence=low`）：安装目录存在但注册表找不到归属。
///
/// 类型 B 的误判率天然偏高（绿色软件/商店应用/无 `InstallLocation` 的程序都会
/// 落进来），所以后端一律标 `riskLevel=L2`，前端必须逐项确认。
#[tauri::command]
pub async fn residual_scan() -> Result<Envelope<engine::scan::residual::ResidualReport>, String> {
    let env = engine::ipc::commands::residual::handle_residual_scan();
    if let Some(r) = &env.data {
        eprintln!(
            "[residual_scan] entries={} dirs={} typeA={} typeB={} total={}B {}ms",
            r.registry_entries,
            r.dirs_scanned,
            r.registry_items.len(),
            r.file_items.len(),
            r.total_bytes,
            r.elapsed_ms
        );
    }
    Ok(env)
}

/// `residual_delete`：把勾选的残留目录**移入保险箱**（7 天可恢复）。
///
/// # 不是直接删
///
/// 先过 `engine::execute::delete::execute` 的唯一入口红线校验，再
/// `SafeBoxDeleter::delete_dir` 整体 rename 进 `%APPDATA%\...\safe_box`。
/// 目录结构完整保留，恢复后目录树原样回来；7 天后由 `purge_expired` 永久删除。
///
/// # 为什么需要 conn
///
/// `SafeBoxDeleter` 移入时要写 `safe_box` 行（7 天后 purge 靠它）。没有行
/// 就是「文件移走了但没人记得」，永远清不掉。
#[tauri::command]
pub async fn residual_delete(
    paths: Vec<String>,
    paths_ctx: tauri::State<'_, AppPaths>,
) -> Result<Envelope<engine::scan::residual::ResidualDeleteResult>, String> {
    let conn = engine::db::conn::open(&paths_ctx.data_root).map_err(|e| e.to_string())?;

    let env =
        engine::ipc::commands::residual::handle_residual_delete(&paths, &conn, &paths_ctx.box_dir);
    if let Some(r) = &env.data {
        eprintln!(
            "[residual_delete] moved={} failed={} bytes={}",
            r.moved.len(),
            r.failed.len(),
            r.bytes_moved
        );
        for f in &r.failed {
            eprintln!("[residual_delete] 失败 {}: {}", f.path, f.reason);
        }
    }
    Ok(env)
}

// ===================== task-068：注册表健康中心（F 模块） =====================

/// 计划任务目录（`%SystemRoot%\System32\Tasks`）。
///
/// 不写死 `C:/Windows`：系统装在别的盘 / 改过 SystemRoot 的机器上会读空。
fn scheduled_tasks_dir() -> std::path::PathBuf {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:/Windows".to_string());
    std::path::PathBuf::from(root)
        .join("System32")
        .join("Tasks")
}

/// `registry_health_scan`：Run 键 / 卸载残留 / 计划任务三分类体检（只读）。
///
/// `taskScanAvailable` 会如实反映计划任务目录能不能读——普通用户对该目录
/// 就是 Permission denied，前端要区分「没有任务」和「读不到」。
#[tauri::command]
pub async fn registry_health_scan(
) -> Result<Envelope<engine::scan::registry_health::RegistryHealthReport>, String> {
    let env =
        engine::ipc::commands::registry_health::handle_registry_health_scan(&scheduled_tasks_dir());
    if let Some(r) = &env.data {
        eprintln!(
            "[registry_health_scan] run={} uninstall={} task={} taskReadable={} {}ms",
            r.run_items.len(),
            r.uninstall_items.len(),
            r.task_items.len(),
            r.task_scan_available,
            r.elapsed_ms
        );
    }
    Ok(env)
}

/// `registry_health_delete`：删除 Run 启动项，**删前自动导出 `.reg` 备份**。
///
/// # 只删值不删键
///
/// `HKCU\...\Run` 下一个值对应一个程序，删键会连带干掉同键下其他所有启动项。
///
/// # fail-closed
///
/// 备份写不出来就不删。备份落在 `<data_root>\registry_backups\`，
/// 双击 `.reg` 即可还原。
///
/// # 后端独立复核
///
/// 删除前重新读值、重新判 L3，**不信前端传的 `riskLevel`**。
#[tauri::command]
pub async fn registry_health_delete(
    targets: Vec<engine::scan::registry_health::RunTarget>,
    paths_ctx: tauri::State<'_, AppPaths>,
) -> Result<Envelope<engine::scan::registry_health::RegistryDeleteResult>, String> {
    let backup_dir = paths_ctx.data_root.join("registry_backups");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let env = engine::ipc::commands::registry_health::handle_registry_health_delete(
        &backup_dir,
        &targets,
        now,
    );
    if let Some(r) = &env.data {
        eprintln!(
            "[registry_health_delete] deleted={} failed={} backups={}",
            r.deleted.len(),
            r.failed.len(),
            r.backups.len()
        );
        for b in &r.backups {
            eprintln!("[registry_health_delete] 备份 → {b}");
        }
        for f in &r.failed {
            eprintln!("[registry_health_delete] 失败 {}: {}", f.name, f.reason);
        }
    }
    Ok(env)
}
