//! task-029：列出待清理的 cleanup_item 行。
//!
//! **与任务书 schema 偏差**：v1__init.sql 的 `cleanup_item` 表**没有 status 列、也没有 scanned_at 列**
//! （真实列为 id/session_id/module_code/category/path/size_bytes/risk_level/...）。
//! 因此「pending」就是「表里所有行」——扫描写入即可见，cleanup.execute 不会删行（task-015 边界）。
//! 偏差点：
//! - 无 `WHERE status='pending'`；
//! - PendingItem 去掉 `scanned_at` 字段（无此列）；
//! - 清理后行仍在 DB（历史快照），前端若需「未清理」需 JOIN audit_log，留后续。

use rusqlite::params;

/// 一条待清理条目（前端 Items 表格行）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingItem {
    pub id: i64,
    pub path: String,
    pub size_bytes: i64,
    pub module_code: String,
    pub session_id: i64,
}

/// 按 session_id（可选）列出 cleanup_item，size_bytes DESC。
pub fn list_pending_items(
    conn: &rusqlite::Connection,
    session_id: Option<i64>,
    limit: i64,
    offset: i64,
) -> Result<Vec<PendingItem>, String> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);

    let sql = "SELECT id, path, size_bytes, module_code, session_id \
               FROM cleanup_item \
               WHERE (?1 IS NULL OR session_id = ?1) \
               ORDER BY size_bytes DESC \
               LIMIT ?2 OFFSET ?3";

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("list_pending prepare: {e}"))?;
    let rows = stmt
        .query_map(params![session_id, limit, offset], |r| {
            Ok(PendingItem {
                id: r.get(0)?,
                path: r.get(1)?,
                size_bytes: r.get(2)?,
                module_code: r.get(3)?,
                session_id: r.get(4)?,
            })
        })
        .map_err(|e| format!("list_pending query: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("list_pending row: {e}"))?);
    }
    Ok(out)
}
