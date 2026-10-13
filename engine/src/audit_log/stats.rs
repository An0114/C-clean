//! audit_log.stats：清理结果聚合查询（task-025）。
//!
//! **与任务书偏差**：v1__init.sql 的 `cleanup_task` 是「一次清理任务」级聚合行
//! （id/session_id/started_at/finished_at/disk_before/disk_after/freed_bytes/
//! success_count/fail_count/skip_count/...），**没有 status 列、也不是行级 success/failed**。
//! 聚合直接对单行字段求和：
//! - total_bytes_freed = SUM(freed_bytes)
//! - success_count     = SUM(success_count)
//! - failed_count      = SUM(fail_count)
//! - session_count     = COUNT(DISTINCT session_id)
//!
//! session_id 过滤：有则加 `AND session_id = ?`，否则全表。

use rusqlite::params;

/// 清理结果聚合（前端仪表盘卡片数据源）。
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupStats {
    pub total_bytes_freed: i64,
    pub success_count: i64,
    pub failed_count: i64,
    pub session_count: i64,
}

/// 按 session_id（可选）聚合 cleanup_task 表。
pub fn cleanup_stats(
    conn: &rusqlite::Connection,
    session_id: Option<i64>,
) -> Result<CleanupStats, String> {
    // 单条 SQL 一次拿全：COALESCE 防空表 NULL。
    let (sum_freed, sum_success, sum_failed, distinct_sessions): (i64, i64, i64, i64) =
        match session_id {
            Some(sid) => conn.query_row(
                "SELECT COALESCE(SUM(freed_bytes),0),
                        COALESCE(SUM(success_count),0),
                        COALESCE(SUM(fail_count),0),
                        COUNT(DISTINCT session_id)
                 FROM cleanup_task WHERE session_id = ?1",
                params![sid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            ),
            None => conn.query_row(
                "SELECT COALESCE(SUM(freed_bytes),0),
                        COALESCE(SUM(success_count),0),
                        COALESCE(SUM(fail_count),0),
                        COUNT(DISTINCT session_id)
                 FROM cleanup_task",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            ),
        }
        .map_err(|e| format!("cleanup_stats query: {e}"))?;

    Ok(CleanupStats {
        total_bytes_freed: sum_freed,
        success_count: sum_success,
        failed_count: sum_failed,
        session_count: distinct_sessions,
    })
}
