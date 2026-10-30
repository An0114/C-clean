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

// ===================== task-061：每日趋势 =====================

/// 趋势图的一个数据点。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendPoint {
    /// 日期，`YYYY-MM-DD`（**本地时区**，用户看的是自己日历上的「那天」）。
    pub day: String,
    /// 当天释放的总字节数。
    pub bytes_freed: i64,
}

/// 最近 `days` 天（含今天）的每日释放量，按日期**升序**。
///
/// # 设计取舍
///
/// - **只返回有记录的日期**，缺失的日期由前端补 0。理由：后端补零要生成完整
///   日期序列，SQLite 没有 `generate_series`，得写递归 CTE 或手工循环，可读性
///   明显变差；而「哪几天没有数据」本来就是前端画图时最清楚的事。
/// - **按本地时区分组**（`'localtime'`）：用户看的是「10 月 29 日」这个日历概念，
///   不是 UTC 日期。跨时区用户可能看到「同一次清理落在不同天」，这是预期行为。
/// - `days` 会被夹到 `1..=365`：传 0 或负数会让 `WHERE started_at >= now` 变成
///   「未来」，静默返回空；夹一下比让调用方自己保证更稳。
pub fn cleanup_stats_trend(
    conn: &rusqlite::Connection,
    days: i64,
) -> Result<Vec<TrendPoint>, String> {
    let span = days.clamp(1, 365);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // 从「span 天前的 00:00（本地）」开始算，而不是「now - span*86400」——
    // 后者会把今天截掉半天，导致今天的柱状图偏低。
    let since = now - span * 86_400;

    let mut stmt = conn
        .prepare(
            "SELECT date(started_at, 'unixepoch', 'localtime') AS day,
                    COALESCE(SUM(freed_bytes), 0) AS bytes
             FROM cleanup_task
             WHERE started_at >= ?1
             GROUP BY day
             ORDER BY day ASC",
        )
        .map_err(|e| format!("cleanup_stats_trend prepare: {e}"))?;

    let rows = stmt
        .query_map(params![since], |r| {
            Ok(TrendPoint {
                day: r.get(0)?,
                bytes_freed: r.get(1)?,
            })
        })
        .map_err(|e| format!("cleanup_stats_trend query: {e}"))?;

    let mut out = Vec::new();
    for it in rows {
        out.push(it.map_err(|e| format!("cleanup_stats_trend row: {e}"))?);
    }
    Ok(out)
}
