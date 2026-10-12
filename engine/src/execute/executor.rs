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
use crate::execute::normalize::normalize;
use crate::execute::whitelist::classify_with_external;

/// 单条 item 的执行结果（返回给 Tauri / 测试断言）。
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub item_id: i64,
    pub path: String,
    pub deleted: bool,
    pub bytes_freed: u64,
    /// 失败原因；成功为 None。
    pub error: Option<String>,
}

/// 一次 execute_items 调用的聚合（写 cleanup_task 用）。
struct BatchSummary {
    freed_bytes: i64,
    success: u32,
    fail: u32,
    skip: u32,
}

/// 编排：对每个 id 先 SELECT path，再过一次白名单（防 dryRun 到 execute 之间路径被改），
/// 通过则调 `deleter.delete_file`，成功/失败都写 audit_log；最后写一行 cleanup_task。
///
/// - NotFound（DB 里没有这个 id）→ skip，不写 audit_log（任务书要求「不影响其他 item」）；
/// - L3 命中 → audit_log result='denied'，denial_reason 填具体原因；
/// - deleter 失败 → audit_log result='fail'，detail 带错误文案。
pub fn execute_items(
    item_ids: &[i64],
    conn: &Connection,
    external_deny: &[String],
    deleter: &dyn Deleter,
) -> Vec<ExecResult> {
    let mut results = Vec::with_capacity(item_ids.len());
    let mut summary = BatchSummary {
        freed_bytes: 0i64,
        success: 0,
        fail: 0,
        skip: 0,
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
                        bytes_freed: 0,
                        error: Some("not found".into()),
                    });
                    continue;
                }
            };

        // 2. 再过白名单（规范化 → 内置 L3 ∪ 外部红线）
        let np = match normalize(&path) {
            Ok(np) => np,
            Err(_) => {
                summary.fail += 1;
                write_audit(conn, now, &path, id, 0, "denied", "PathTraversal");
                results.push(ExecResult {
                    item_id: id,
                    path,
                    deleted: false,
                    bytes_freed: 0,
                    error: Some("blocked by L3: PathTraversal".into()),
                });
                continue;
            }
        };
        if let Err(reason) = classify_with_external(&np, external_deny) {
            let reason_str = format!("{reason:?}");
            summary.fail += 1;
            write_audit(conn, now, &path, id, 0, "denied", &reason_str);
            results.push(ExecResult {
                item_id: id,
                path,
                deleted: false,
                bytes_freed: 0,
                error: Some(format!("blocked by L3: {reason_str}")),
            });
            continue;
        }

        // 3. 真实删除
        let bytes_before = std::fs::metadata(np.normalized())
            .map(|m| m.len())
            .unwrap_or(0);
        match deleter.delete_file(&np) {
            Ok(()) => {
                summary.success += 1;
                summary.freed_bytes += bytes_before as i64;
                write_audit(conn, now, &path, id, bytes_before, "success", "");
                results.push(ExecResult {
                    item_id: id,
                    path,
                    deleted: true,
                    bytes_freed: bytes_before,
                    error: None,
                });
            }
            Err(e) => {
                summary.fail += 1;
                write_audit(conn, now, &path, id, bytes_before, "fail", &e);
                results.push(ExecResult {
                    item_id: id,
                    path,
                    deleted: false,
                    bytes_freed: 0,
                    error: Some(e),
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

    results
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn write_audit(
    conn: &Connection,
    ts: i64,
    target: &str,
    item_id: i64,
    bytes: u64,
    result: &str,
    denial_or_err: &str,
) {
    let detail = serde_json::json!({
        "item_id": item_id,
        "bytes_freed": bytes,
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
