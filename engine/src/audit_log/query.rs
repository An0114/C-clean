//! audit_log 只读查询（task-024）。
//!
//! 真实 schema：
//! ```sql
//! audit_log(id INTEGER PK, ts INTEGER, actor TEXT, action TEXT, target TEXT,
//!           detail TEXT NULL, result TEXT, denial_reason TEXT NULL)
//! ```
//! 对外把 `action` 暴露为 `event_type`、`target` 为 `path`、`ts` 为 `created_at`，
//! 让前端不需要关心库列名。

use rusqlite::params;

/// 单条审计日志（前端友好命名）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEntry {
    pub id: i64,
    /// 原 `action` 列：permanent_delete / restore / scan / quarantine 等。
    pub event_type: String,
    /// 原 `target` 列：路径 / 键路径 / URL。
    pub path: String,
    /// 原 `detail` 列：JSON 补充信息，可空。
    pub detail: Option<String>,
    /// 原 `ts` 列：Unix 秒。
    pub created_at: i64,
    /// success / fail / denied。
    pub result: String,
}

/// 查询过滤条件。
///
/// 任务书要求 `session_id: Option<i64>`，但真实表无 session_id 列；
/// 保留该字段以便前端协议兼容，当前实现忽略它（不进 WHERE）。
#[derive(Debug, Clone, Default)]
pub struct AuditLogFilter {
    /// 预留：按 session 过滤（当前 schema 无此列，忽略）。
    pub session_id: Option<i64>,
    /// 每页条数，默认 100。
    pub limit: i64,
    /// 偏移，默认 0。
    pub offset: i64,
}

/// 按 ts DESC, id DESC 分页返回 audit_log 条目。
pub fn list_audit_log(
    conn: &rusqlite::Connection,
    filter: &AuditLogFilter,
) -> Result<Vec<AuditLogEntry>, String> {
    let limit = if filter.limit <= 0 { 100 } else { filter.limit };
    let offset = if filter.offset < 0 { 0 } else { filter.offset };

    // session_id 暂未实现（真实表无此列），保留入参但不进 WHERE。
    let sql = "SELECT id, action, target, detail, ts, result \
               FROM audit_log \
               ORDER BY ts DESC, id DESC \
               LIMIT ?1 OFFSET ?2";
    let mut stmt = conn.prepare(sql).map_err(|e| format!("prepare: {e}"))?;
    let rows = stmt
        .query_map(params![limit, offset], |r| {
            Ok(AuditLogEntry {
                id: r.get(0)?,
                event_type: r.get(1)?,
                path: r.get(2)?,
                detail: r.get(3)?,
                created_at: r.get(4)?,
                result: r.get(5)?,
            })
        })
        .map_err(|e| format!("query_map: {e}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("row: {e}"))?);
    }
    Ok(out)
}

/// task-051：按 target（路径）查该路径的审计历史，`ts DESC, id DESC`，最多 `limit` 条。
///
/// - 路径比较用 `COLLATE NOCASE`：Windows 路径大小写不敏感，本应用写入走
///   normalize（盘符大写），但历史数据 / 手工写入可能大小写不一致；
/// - `limit` 内部 clamp 到 [1, 100]，防调用方传 99999；
/// - 无匹配返回空 vec（不是 Err）。
pub fn list_by_target(
    conn: &rusqlite::Connection,
    target: &str,
    limit: i64,
) -> Result<Vec<AuditLogEntry>, String> {
    let limit = limit.clamp(1, 100);

    let sql = "SELECT id, action, target, detail, ts, result \
               FROM audit_log \
               WHERE target = ?1 COLLATE NOCASE \
               ORDER BY ts DESC, id DESC \
               LIMIT ?2";
    let mut stmt = conn.prepare(sql).map_err(|e| format!("prepare: {e}"))?;
    let rows = stmt
        .query_map(params![target, limit], |r| {
            Ok(AuditLogEntry {
                id: r.get(0)?,
                event_type: r.get(1)?,
                path: r.get(2)?,
                detail: r.get(3)?,
                created_at: r.get(4)?,
                result: r.get(5)?,
            })
        })
        .map_err(|e| format!("query_map: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("row: {e}"))?);
    }
    Ok(out)
}
