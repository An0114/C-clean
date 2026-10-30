//! task-061 集成测试：`cleanup_stats_trend` 每日聚合。
//!
//! 关键点：`cleanup_task.started_at` 是 **unix 秒**，聚合按 `date(...,'unixepoch','localtime')`
//! 分组——所以用例里的时间戳必须用「相对今天的天数偏移」构造，不能用固定常量，
//! 否则跨天/跨时区跑会飘。

use engine::audit_log::stats::cleanup_stats_trend;
use engine::db::conn::open;

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 插一行 cleanup_task，`days_ago` = 几天前（0 = 今天）。
///
/// 时间取「今天中午 12:00 减 N 天」——避开 00:00 / 23:59 的边界，
/// 否则用例在临近午夜运行时可能落到相邻日期。
fn insert_task_days_ago(conn: &rusqlite::Connection, days_ago: i64, freed_bytes: i64) {
    let now = now_unix();
    // 对齐到「今天 12:00 本地」：用本地日期的概念近似即可，这里用 now 减 12h 再加回，
    // 足以让「同一天内的多条」落在同一天。
    let base = now - (now % 86_400) + 12 * 3_600;
    let started_at = base - days_ago * 86_400;
    conn.execute(
        "INSERT INTO cleanup_task (session_id, started_at, finished_at,
             disk_before_bytes, disk_after_bytes, freed_bytes,
             success_count, fail_count, skip_count, dry_run_snapshot, safebox_enabled)
         VALUES (NULL, ?1, ?1, 0, 0, ?2, 1, 0, 0, '{}', 0)",
        rusqlite::params![started_at, freed_bytes],
    )
    .unwrap();
}

#[test]
fn trend_returns_one_point_per_day() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();

    // 3 天各一条 → 3 个点（验收 9）
    insert_task_days_ago(&conn, 0, 100);
    insert_task_days_ago(&conn, 1, 200);
    insert_task_days_ago(&conn, 2, 300);

    let pts = cleanup_stats_trend(&conn, 30).unwrap();
    assert_eq!(pts.len(), 3, "3 天数据应得 3 个点，实际 {pts:?}");

    // 升序：最早的在前
    assert!(
        pts[0].day < pts[1].day && pts[1].day < pts[2].day,
        "应按日期升序"
    );
    // 数值按天对应（2 天前 300、1 天前 200、今天 100）
    assert_eq!(pts[0].bytes_freed, 300);
    assert_eq!(pts[1].bytes_freed, 200);
    assert_eq!(pts[2].bytes_freed, 100);
}

#[test]
fn trend_sums_multiple_tasks_on_same_day() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();

    // 同一天两条 → 合并成 1 个点、值相加
    insert_task_days_ago(&conn, 0, 100);
    insert_task_days_ago(&conn, 0, 250);

    let pts = cleanup_stats_trend(&conn, 30).unwrap();
    assert_eq!(pts.len(), 1, "同一天应合并为一个点，实际 {pts:?}");
    assert_eq!(pts[0].bytes_freed, 350, "同一天应求和");
}

#[test]
fn trend_excludes_rows_outside_window() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();

    insert_task_days_ago(&conn, 0, 100);
    insert_task_days_ago(&conn, 40, 9_999); // 40 天前，超出 30 天窗口

    let pts = cleanup_stats_trend(&conn, 30).unwrap();
    assert_eq!(pts.len(), 1, "窗口外的记录不应计入，实际 {pts:?}");
    assert_eq!(pts[0].bytes_freed, 100);
}

#[test]
fn trend_empty_table_returns_empty_vec() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();

    let pts = cleanup_stats_trend(&conn, 30).unwrap();
    assert!(pts.is_empty(), "空表应返回空 Vec（前端据此显示占位）");
}

#[test]
fn trend_clamps_days_to_valid_range() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    insert_task_days_ago(&conn, 0, 42);

    // days=0 / 负数不能变成「查未来」而静默返回空——应被夹到 1
    assert_eq!(
        cleanup_stats_trend(&conn, 0).unwrap().len(),
        1,
        "days=0 应夹到 1"
    );
    assert_eq!(
        cleanup_stats_trend(&conn, -5).unwrap().len(),
        1,
        "days 负数应夹到 1"
    );
    // 超大值不 panic、不越界
    assert_eq!(cleanup_stats_trend(&conn, 99_999).unwrap().len(), 1);
}
