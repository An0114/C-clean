//! 删除编排（task-015）：把 DB 里的 cleanup_item → 白名单 → FsDeleter → 落审计。
//!
//! 设计备注（与任务书 §B 列名偏差）：
//! - v1__init.sql 的 `cleanup_task` 是**批量任务行**（不是逐 item 行）：
//!   字段为 session_id / started_at / finished_at / disk_before_bytes / disk_after_bytes /
//!   freed_bytes / success_count / fail_count / skip_count / dry_run_snapshot / safebox_enabled。
//!   因此每次 `execute_items` 调用写**一行** cleanup_task，聚合计数。
//! - `audit_log` 字段为 ts / actor / action / target / detail / result / denial_reason，
//!   按任务书 §B 映射：event_type→action、path→target、created_at→ts、detail JSON 包 item_id/bytes。

use rusqlite::params;
use rusqlite::Connection;

use crate::execute::delete::Deleter;
use crate::execute::locking::{who_is_locking, LockingProcess};
use crate::execute::normalize::normalize;
use crate::execute::resolve::resolve_to_real;
use crate::execute::whitelist::classify_with_external;

/// 单条 item 的执行结果（返回给 Tauri / 测试断言）。
// task-063：加 Deserialize —— 前端要把清理结果原样传回 `report_export` 生成报告。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecResult {
    pub item_id: i64,
    pub path: String,
    pub deleted: bool,
    /// task-052 G5：删除前文件就已不存在（NotFound 幂等）。
    /// 既不算成功释放、也不算失败 —— 前端单独统计，文案为「M 项已不存在」。
    #[serde(default)]
    pub already_gone: bool,
    pub bytes_freed: u64,
    /// 失败原因；成功（含已不存在）为 None。
    pub error: Option<String>,
    /// task-021：FILE_IN_USE 时填占用进程列表；其他场景为空 Vec。
    #[serde(default)]
    pub locking_processes: Vec<LockingProcess>,
}

/// 一次 execute_items 调用的汇总（返回给 Tauri）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteSummary {
    pub results: Vec<ExecResult>,
    pub deleted_count: u64,
    pub failed_count: u64,
    /// task-052 G5：本次「已不存在」的条目数（不计入 bytes_freed，也不算失败）。
    #[serde(default)]
    pub already_gone_count: u64,
    pub bytes_freed_total: u64,
    /// task-063：清理**前**目标盘已用字节数（查不到为 None，前端显示「—」）。
    ///
    /// 与 `bytes_freed_total` 的区别：后者是「删掉的文件大小之和」，这里是
    /// 「磁盘实际已用量」。两者会有偏差（簇对齐、并发写入），报告里都展示。
    #[serde(default)]
    pub disk_before_bytes: Option<u64>,
    /// task-063：清理**后**目标盘已用字节数。
    #[serde(default)]
    pub disk_after_bytes: Option<u64>,
}

/// 一次 execute_items 调用的聚合（写 cleanup_task 用）。
struct BatchSummary {
    freed_bytes: i64,
    success: u32,
    fail: u32,
    skip: u32,
    /// task-052 G5：已不存在计数（只进返回值，不写 cleanup_task —— schema 无此列）。
    already_gone: u32,
}

/// 记一条「系统还原点已创建」到 audit_log（task-062 验收 10）。
///
/// 放在 executor 而不是 elevation 模块：elevation 刻意不碰 DB（它要能被提权 exe
/// 那种零依赖场景复用），而 executor 本来就在写 audit_log。
pub fn log_restore_point(
    conn: &Connection,
    name: &str,
    sequence: i64,
    ts: i64,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO audit_log (ts, actor, action, target, detail, result, denial_reason)
         VALUES (?1, 'elevated', 'restore_point', ?2, ?3, 'success', NULL)",
        params![ts, name, format!("sequence={sequence}")],
    )
    .map(|_| ())
    .map_err(|e| format!("log_restore_point: {e}"))
}

/// 判断这批 item 是否含 **L2 风险项**——有则清理前必须建系统还原点（task-062）。
///
/// # 为什么只对 L2 触发
///
/// - **L1**（缓存 / 临时文件）：删错了也能重建，为它弹一次 UAC + 等几十秒建还原点
///   是灾难级体验，用户会直接关掉这个功能；
/// - **L2**（需逐项确认的高风险项）：PRD 要求「清理前自动创建还原点」；
/// - **L3**：根本进不了 `cleanup_item` 表（红线在扫描侧就拦掉了）。
///
/// 所以判断依据就是「这批 id 里有没有 `risk_level = 'L2'` 的行」。
///
/// # 失败语义
///
/// 返回 `Err` 只代表**查库失败**，不代表「不需要还原点」。调用方遇到 `Err` 应当
/// 按「需要还原点」处理（fail-closed）——查不清风险就不该裸奔清理。
pub fn needs_restore_point(conn: &Connection, item_ids: &[i64]) -> Result<bool, String> {
    if item_ids.is_empty() {
        return Ok(false);
    }
    // 动态占位符：`IN (?,?,?)`。条数是运行期才知道的，不能预编译。
    let placeholders = std::iter::repeat("?")
        .take(item_ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT COUNT(*) FROM cleanup_item WHERE risk_level = 'L2' AND id IN ({placeholders})"
    );
    let n: i64 = conn
        .query_row(&sql, rusqlite::params_from_iter(item_ids.iter()), |r| {
            r.get(0)
        })
        .map_err(|e| format!("needs_restore_point: {e}"))?;
    Ok(n > 0)
}

/// 编排：对每个 id 先 SELECT path，再过一次白名单（防 dryRun 到 execute 之间路径被改），
/// 通过则调 `deleter.delete_file`，成功/失败都写 audit_log；最后写一行 cleanup_task。
/// - NotFound（DB 里没有这个 id）→ skip，不写 audit_log（任务书要求「不影响其他 item」）；
/// - L3 命中 → audit_log result='denied'，denial_reason 填具体原因；
/// - deleter 失败 → audit_log result='fail'，detail 带错误文案。
///
/// task-052 G3：工厂闭包签名从 `FnMut(item_id)` 改为 `FnMut(item_id, task_id)`。
/// `task_id` 是本次调用刚写入的 `cleanup_task.id`（写失败时为 `-1`），
/// 供 `SafeBoxDeleter` 落 `safe_box.source_task_id` —— 否则该列恒 NULL，
/// 前端保险箱的 Session 下拉与「来源」列永远是空的。
pub fn execute_items<'a, F>(
    item_ids: &[i64],
    conn: &Connection,
    external_deny: &[String],
    mut deleter_factory: F,
) -> ExecuteSummary
where
    F: FnMut(i64, i64) -> Box<dyn Deleter + 'a>,
{
    // task-063：记录清理前目标盘已用量，供结果页做「前后对比」。
    // 盘符从第一条 item 的路径推断（本工具主要清 C 盘，但用户也可能清 D 盘，
    // 写死 "C" 会让非 C 盘的结果页数字完全对不上）。
    let drive = detect_drive(conn, item_ids);
    let disk_before_bytes = crate::report::drive_usage(&drive)
        .ok()
        .map(|u| u.used_bytes);

    let mut results = Vec::with_capacity(item_ids.len());
    let mut summary = BatchSummary {
        freed_bytes: 0i64,
        success: 0,
        fail: 0,
        skip: 0,
        already_gone: 0,
    };
    let now = now_unix();

    // 批量任务行：先写 started_at，结束后再 UPDATE finished_at/计数。
    let task_id = match conn.execute(
        "INSERT INTO cleanup_task
         (session_id, started_at, finished_at, disk_before_bytes, disk_after_bytes,
          freed_bytes, success_count, fail_count, skip_count, dry_run_snapshot, safebox_enabled)
         VALUES (NULL, ?1, ?1, 0, 0, 0, 0, 0, 0, ?2, 0)",
        params![
            now,
            &serde_json::to_string(item_ids).unwrap_or_else(|_| "[]".into())
        ],
    ) {
        Ok(_) => conn.last_insert_rowid(),
        Err(e) => {
            eprintln!("[executor] insert cleanup_task failed: {e}");
            -1
        }
    };

    for &id in item_ids {
        // 1. SELECT path from cleanup_item
        let path: String =
            match conn.query_row("SELECT path FROM cleanup_item WHERE id = ?1", [id], |r| {
                r.get(0)
            }) {
                Ok(p) => p,
                Err(_) => {
                    summary.skip += 1;
                    results.push(ExecResult {
                        item_id: id,
                        path: String::new(),
                        deleted: false,
                        already_gone: false,
                        bytes_freed: 0,
                        error: Some("not found".into()),
                        locking_processes: Vec::new(),
                    });
                    continue;
                }
            };

        // 2. 再过白名单（规范化 → 真实路径解析 → 内置 L3 ∪ 外部红线）
        //    task-008：必须对 canonicalize 后的真实路径做 L3 判定，
        //    否则 junction/symlink 可把 Temp 里的指针重定向到 Desktop 而绕过红线。
        let np = match normalize(&path) {
            Ok(np) => np,
            Err(_) => {
                summary.fail += 1;
                write_audit(conn, now, &path, id, 0, "denied", "PathTraversal", false);
                results.push(ExecResult {
                    item_id: id,
                    path,
                    deleted: false,
                    already_gone: false,
                    bytes_freed: 0,
                    error: Some("blocked by L3: PathTraversal".into()),
                    locking_processes: Vec::new(),
                });
                continue;
            }
        };
        // 解析真实路径；NotExists 时回退到输入路径分类（幂等重删已删文件）。
        let classify_target = match resolve_to_real(&np) {
            Ok(resolved) => match normalize(&resolved.real) {
                Ok(real_np) => real_np,
                Err(_) => np.clone(),
            },
            Err(crate::execute::resolve::ResolveError::NotExists(_)) => np.clone(),
            Err(e) => {
                summary.fail += 1;
                let reason = format!("ResolveFailed({e})");
                write_audit(conn, now, &path, id, 0, "denied", &reason, false);
                results.push(ExecResult {
                    item_id: id,
                    path,
                    deleted: false,
                    already_gone: false,
                    bytes_freed: 0,
                    error: Some(format!("blocked by L3: {reason}")),
                    locking_processes: Vec::new(),
                });
                continue;
            }
        };
        if let Err(reason) = classify_with_external(&classify_target, external_deny) {
            let reason_str = format!("{reason:?}");
            summary.fail += 1;
            write_audit(conn, now, &path, id, 0, "denied", &reason_str, false);
            results.push(ExecResult {
                item_id: id,
                path,
                deleted: false,
                already_gone: false,
                bytes_freed: 0,
                error: Some(format!("blocked by L3: {reason_str}")),
                locking_processes: Vec::new(),
            });
            continue;
        }

        // 3. 真实删除（per-item 构造 deleter，SafeBoxDeleter 需要绑定 item_id/source_task_id）
        let bytes_before = std::fs::metadata(np.normalized())
            .map(|m| m.len())
            .unwrap_or(0);
        // task-052 G3：把本次 cleanup_task.id 交给工厂，SafeBoxDeleter 据此写 source_task_id。
        // task_id < 0 表示 INSERT cleanup_task 失败，此时传 -1 让上层落 NULL（不写脏值）。
        let deleter = deleter_factory(id, task_id);
        match deleter.delete_file(&np) {
            // 真删 / 真移走：计入释放量。
            Ok(true) => {
                summary.success += 1;
                summary.freed_bytes += bytes_before as i64;
                write_audit(conn, now, &path, id, bytes_before, "success", "", false);
                results.push(ExecResult {
                    item_id: id,
                    path,
                    deleted: true,
                    already_gone: false,
                    bytes_freed: bytes_before,
                    error: None,
                    locking_processes: Vec::new(),
                });
            }
            // task-052 G5：文件在扫描后、清理前已被系统/其他程序删掉。
            // 幂等成功，但**没有**产生任何释放量 —— 单列计数，不算失败。
            Ok(false) => {
                summary.already_gone += 1;
                write_audit(conn, now, &path, id, 0, "success", "", true);
                results.push(ExecResult {
                    item_id: id,
                    path,
                    deleted: false,
                    already_gone: true,
                    bytes_freed: 0,
                    error: None,
                    locking_processes: Vec::new(),
                });
            }
            Err(e) => {
                summary.fail += 1;
                // task-019：把 deleter 抛出的 FILE_IN_USE / PERMISSION_DENIED 前缀翻译成
                // audit_log.detail 稳定文案，方便前端 toast 直接展示。
                // task-021：FILE_IN_USE 时自动调 who_is_locking 查占用进程列表，
                // 拼进 detail 并填到 ExecResult.locking_processes 返回给前端。
                let (detail, locking) = if e.starts_with("FILE_IN_USE:") {
                    match who_is_locking(&path) {
                        Ok(procs) if !procs.is_empty() => {
                            let names: Vec<String> = procs
                                .iter()
                                .map(|p| format!("{} (PID {})", p.app_name, p.pid))
                                .collect();
                            (format!("file in use by: {}", names.join(", ")), procs)
                        }
                        _ => (
                            "file in use by another process (unable to identify process)"
                                .to_string(),
                            Vec::new(),
                        ),
                    }
                } else if e.starts_with("PERMISSION_DENIED:") {
                    ("permission denied, needs elevation".to_string(), Vec::new())
                } else {
                    (e.clone(), Vec::new())
                };
                write_audit(conn, now, &path, id, bytes_before, "fail", &detail, false);
                results.push(ExecResult {
                    item_id: id,
                    path,
                    deleted: false,
                    already_gone: false,
                    bytes_freed: 0,
                    error: Some(e),
                    locking_processes: locking,
                });
            }
        }
    }

    // 4. 回写批量任务行
    if task_id >= 0 {
        let _ = conn.execute(
            "UPDATE cleanup_task
             SET finished_at = ?1, freed_bytes = ?2, success_count = ?3, fail_count = ?4, skip_count = ?5
             WHERE id = ?6",
            params![
                now,
                summary.freed_bytes,
                summary.success as i64,
                summary.fail as i64,
                summary.skip as i64,
                task_id
            ],
        );
    }

    let deleted_count = results.iter().filter(|r| r.deleted).count() as u64;
    // task-052 G5：failed_count 必须排除「已不存在」，否则前端会把幂等跳过算成失败。
    let failed_count = results
        .iter()
        .filter(|r| !r.deleted && !r.already_gone)
        .count() as u64;
    let already_gone_count = results.iter().filter(|r| r.already_gone).count() as u64;
    let bytes_freed_total: u64 = results.iter().map(|r| r.bytes_freed).sum();

    // task-063：清理后再查一次。查不到就留 None（前端显示「—」而不是 0）。
    let disk_after_bytes = crate::report::drive_usage(&drive)
        .ok()
        .map(|u| u.used_bytes);

    ExecuteSummary {
        results,
        deleted_count,
        failed_count,
        already_gone_count,
        bytes_freed_total,
        disk_before_bytes,
        disk_after_bytes,
    }
}

/// 从 `item_ids` 的第一条记录推断目标盘符（`"C"`）。查不到时回落 `"C"`。
///
/// 只取第一条而不是逐条检查：一次清理跨盘是极罕见的，为它做「多盘分别统计」
/// 会让报告结构复杂一个量级，收益不抵成本。
fn detect_drive(conn: &Connection, item_ids: &[i64]) -> String {
    item_ids
        .first()
        .and_then(|id| {
            conn.query_row("SELECT path FROM cleanup_item WHERE id = ?1", [id], |r| {
                r.get::<_, String>(0)
            })
            .ok()
        })
        .and_then(|p| p.chars().next())
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase().to_string())
        .unwrap_or_else(|| "C".to_string())
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 写一条 audit_log。
///
/// task-052 G5：新增 `already_gone` 参数，写进 detail JSON 便于事后取证
/// （同一 result='success' 下区分「真删了 N 字节」和「本来就不存在」）。
///
/// 参数虽多但都是审计行的扁平字段，包成结构体反而增加 6 个调用点的噪音；
/// 与 `delete.rs::push_rejected` 的处理保持一致，显式 allow。
#[allow(clippy::too_many_arguments)]
fn write_audit(
    conn: &Connection,
    ts: i64,
    target: &str,
    item_id: i64,
    bytes: u64,
    result: &str,
    denial_or_err: &str,
    already_gone: bool,
) {
    let detail = serde_json::json!({
        "item_id": item_id,
        "bytes_freed": bytes,
        "already_gone": already_gone,
        "err": if result == "fail" { Some(denial_or_err) } else { None },
    })
    .to_string();
    let denial_param: Option<&str> = if result == "denied" {
        Some(denial_or_err)
    } else {
        None
    };
    let _ = conn.execute(
        "INSERT INTO audit_log (ts, actor, action, target, detail, result, denial_reason)
         VALUES (?1, 'user', 'permanent_delete', ?2, ?3, ?4, ?5)",
        params![ts, target, detail, result, denial_param],
    );
}
