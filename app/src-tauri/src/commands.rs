//! Tauri command 薄包装：把 engine 纯函数挂到 IPC 边界。
//!
//! 本层只做「前端参数 → engine 入参结构 → 信封」的透传；
//! 业务判定全部在 engine::ipc::commands 里，方便纯内存单测。
//! （Tauri 2 规则：带 State 引用的 async command 必须返回 Result。）

use engine::audit_log::{list_audit_log, AuditLogEntry, AuditLogFilter};
use engine::ipc::commands::dry_run::{handle_dry_run, DryRunReq, DryRunResp};
use engine::ipc::envelope::Envelope;
use engine::ipc::error_codes::INTERNAL;
use engine::rules::external_deny_list;
use engine::rules::loader::load as load_rules;
use engine::rules::meta_store::{get_active_rules_meta, upsert_active_rules_meta};
use engine::rules::remote::fetch_remote_meta;
use engine::rules::{
    apply_update, fetch_remote_rules_bytes, ApplyResult, CheckUpdateResp, REMOTE_RULES_META_URL,
};
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
/// 3. INSERT scan_session 拿真实 session_id（last_insert_rowid）；
/// 4. 串跑 5 个 ps1（ScanOrchestrator::run_all），item 写 cleanup_item、emit 进度；
///    注：M1 dev 期直接同步执行，后续 scan.cancel 引入时再改 spawn_blocking 线程模型；
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

/// `cleanup.dryRun`：加载 rules/ 外部红线 + 打开真实 DB，按 id 从 cleanup_item 取路径跑 dryRun。
///
/// task-014：把 MockResolver 换成 `DbItemPathResolver`——scan_start（task-013）已经
/// 把真实扫描结果写进 cleanup_item，这里按前端传的 itemIds 反查路径，再走白名单。
#[tauri::command]
pub async fn cleanup_dry_run(item_ids: Vec<i64>) -> Result<Envelope<DryRunResp>, String> {
    // 加载失败（checksum 不匹配等）→ 保守返回 5001，不静默放行。
    let external_deny = match load_external_deny() {
        Ok(v) => v,
        Err(_) => return Ok(Envelope::err(INTERNAL)),
    };

    // 与 scan_start 同一个 data_dir（开发期 current_dir；打包后改 app_data_dir）。
    let data_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let conn = engine::db::conn::open(&data_root).map_err(|e| e.to_string())?;
    let resolver = engine::db::resolver::DbItemPathResolver::new(&conn);

    Ok(handle_dry_run(
        DryRunReq { item_ids },
        &resolver,
        &external_deny,
    ))
}

/// `cleanup.execute`：dryRun 通过后用户确认 → 真删文件。
///
/// task-015：对每个 id 再查 DB、再过一次白名单（防 dryRun→execute 之间路径被改），
/// 用 `FsDeleter`（std::fs::remove_file）真删；成功/失败都落 audit_log，
/// 一次调用写一行 cleanup_task 聚合。
#[tauri::command]
pub async fn cleanup_execute(
    item_ids: Vec<i64>,
) -> Result<Envelope<engine::execute::executor::ExecuteSummary>, String> {
    if item_ids.is_empty() {
        return Ok(Envelope::err(engine::ipc::error_codes::DRYRUN_EMPTY));
    }
    let external_deny = match load_external_deny() {
        Ok(v) => v,
        Err(_) => return Ok(Envelope::err(INTERNAL)),
    };
    let data_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let conn = engine::db::conn::open(&data_root).map_err(|e| e.to_string())?;
    let box_dir = data_root.join("safe_box");
    std::fs::create_dir_all(&box_dir).map_err(|e| e.to_string())?;

    let summary = engine::execute::executor::execute_items(
        &item_ids,
        &conn,
        &external_deny,
        |_item_id| -> Box<dyn engine::execute::delete::Deleter + '_> {
            Box::new(engine::execute::safe_box::SafeBoxDeleter::new(
                &box_dir, &conn, None,
            ))
        },
    );
    Ok(Envelope::ok(summary))
}

/// `safe_box.restore`：按 safe_box id 把文件从保险箱 rename 回原路径。
///
/// task-017：业务错误码映射（不新增错误码）：
/// - `TargetOccupied` → 2003 FILE_OCCUPIED（最接近：目标已被占用）；
/// - `NotFound` / `ParentMissing` / `Io` / `Db` → 5001 INTERNAL（前端 toast 兜底）。
#[tauri::command]
pub async fn safe_box_restore(
    safe_box_id: i64,
) -> Result<Envelope<engine::execute::safe_box::RestoreResult>, String> {
    let data_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let conn = engine::db::conn::open(&data_root).map_err(|e| e.to_string())?;
    let box_dir = data_root.join("safe_box");

    match engine::execute::safe_box::restore_from_safe_box(&conn, &box_dir, safe_box_id) {
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
pub async fn safe_box_list() -> Result<Envelope<engine::execute::safe_box::SafeBoxListResp>, String>
{
    let data_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let conn = match engine::db::conn::open(&data_root) {
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

/// `rules.checkUpdate`：比对本地 rules_meta 与远端 GitHub Raw 版本（不下载替换）。
///
/// task-022：
/// 1. 加载本地 rules/ → 拿到 version / checksum_sha256；
/// 2. upsert 到 rules_meta 单行表（幂等）；
/// 3. GET 远端 rules_meta.json；
/// 4. 比对 sha256 返 has_update。
#[tauri::command]
pub async fn rules_check_update() -> Result<Envelope<CheckUpdateResp>, String> {
    let data_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let rules_dir = data_root.join("rules");

    // 1. 加载本地规则（含 SHA-256 校验）。
    let local = match load_rules(&rules_dir) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[rules_check_update] load local: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    let local_version = local.meta.version.clone();
    let local_sha = local.meta.checksum_sha256.clone();

    // 2. DB upsert（幂等）。
    let conn = match engine::db::conn::open(&data_root) {
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
    let _ = get_active_rules_meta(&conn); // 留作后续扩展；当前直接用 local_*。

    // 3. 拉远端。
    let remote = match fetch_remote_meta() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[rules_check_update] fetch remote: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };

    Ok(Envelope::ok(CheckUpdateResp {
        has_update: local_sha != remote.checksum_sha256,
        current_version: local_version,
        remote_version: remote.version,
        current_sha256: local_sha,
        remote_sha256: remote.checksum_sha256,
    }))
}

/// `rules.applyUpdate`：下载远端 cache_rules.json 并替换本地，SHA-256 校验通过后 upsert DB。
///
/// task-023：
/// 1. GET 远端 rules_meta.json；
/// 2. GET 远端 cache_rules.json 字节流；
/// 3. `apply_update(rules_dir, conn, meta, bytes)`——校验/备份/写文件/upsert，失败自动回滚。
#[tauri::command]
pub async fn rules_apply_update() -> Result<Envelope<ApplyResult>, String> {
    let data_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let rules_dir = data_root.join("rules");

    let meta = match fetch_remote_meta() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[rules_apply_update] fetch meta: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    let bytes = match fetch_remote_rules_bytes() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[rules_apply_update] fetch rules: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    let conn = match engine::db::conn::open(&data_root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[rules_apply_update] open db: {e}");
            return Ok(Envelope::err(INTERNAL));
        }
    };
    match apply_update(&rules_dir, &conn, &meta, &bytes) {
        Ok(r) => Ok(Envelope::ok(r)),
        Err(e) => {
            eprintln!("[rules_apply_update] apply: {e}");
            Ok(Envelope::err(INTERNAL))
        }
    }
}

/// `audit_log.list`：按 ts 倒序分页返回清理历史条目（只读）。
///
/// task-024：M1-F 报表基础。入参 session_id 当前 schema 无对应列，仅透传忽略。
#[tauri::command]
pub async fn audit_log_list(
    session_id: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Envelope<Vec<AuditLogEntry>>, String> {
    let data_root = std::env::current_dir().map_err(|e| e.to_string())?;
    let conn = match engine::db::conn::open(&data_root) {
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
