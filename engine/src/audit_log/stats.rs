//! audit_log.stats：清理结果聚合查询（task-025）。
//!
//! **与任务书偏差**：v1__init.sql 的 `cleanup_task` 是「一次清理任务」级聚合行
//! （id/session_id/started_at/finished_at/disk_before/disk_after/freed_bytes/
//! success_count/fail_count/skip_count/...），**没有 status 列、也不是行级 success/failed**。
//! 聚合直接对单行字段求和：
//! - total_bytes_freed = SUM(freed_bytes)
//! - success_count     = SUM(success_count)
//! - failed_count      = SUM(fail_count)
//! - session_count     = COUNT(*)
//!
//! session_id 过滤：有则加 `AND session_id = ?`，否则全表。
//!
//! **task-052 G4 口径迁移**：`session_count` 原本是 `COUNT(DISTINCT session_id)`，
//! 但 `execute_items` 插入 cleanup_task 时 `session_id` 写死 NULL，而
//! `COUNT(DISTINCT x)` 会忽略 NULL → 该值恒为 0，前端「清理轮次」卡片永远显示 0
//! （走查 G4 缺陷）。改为 `COUNT(*)`（cleanup_task 行数）：语义上「轮次」本就是
//! 「清理批次」，一次执行一行 cleanup_task，COUNT(*) 更贴语义，也不必为了
//! 跨 session 聚合去补 session_id 的写入链路。
//!
//! 迁移前后对比（同一份数据，2 行 cleanup_task 且 session_id 均为 NULL）：
//! - 迁移前 `COUNT(DISTINCT session_id)` → **0**（NULL 被忽略）
//! - 迁移后 `COUNT(*)` → **2**

use rusqlite::params;

/// 清理结果聚合（前端仪表盘卡片数据源）。
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupStats {
    pub total_bytes_freed: i64,
    pub success_count: i64,
    pub failed_count: i64,
    /// task-052 G4：清理轮次 = cleanup_task 行数（不再用 COUNT(DISTINCT session_id)）。
    pub session_count: i64,
}

/// 按 session_id（可选）聚合 cleanup_task 表。
pub fn cleanup_stats(
    conn: &rusqlite::Connection,
    session_id: Option<i64>,
) -> Result<CleanupStats, String> {
    // 单条 SQL 一次拿全：COALESCE 防空表 NULL。
    let (sum_freed, sum_success, sum_failed, task_rows): (i64, i64, i64, i64) = match session_id {
        Some(sid) => conn.query_row(
            "SELECT COALESCE(SUM(freed_bytes),0),
                    COALESCE(SUM(success_count),0),
                    COALESCE(SUM(fail_count),0),
                    COUNT(*)
             FROM cleanup_task WHERE session_id = ?1",
            params![sid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        ),
        None => conn.query_row(
            "SELECT COALESCE(SUM(freed_bytes),0),
                    COALESCE(SUM(success_count),0),
                    COALESCE(SUM(fail_count),0),
                    COUNT(*)
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
        session_count: task_rows,
    })
}
