//! task-025 集成测试：cleanup_stats 聚合查询。

use engine::audit_log::cleanup_stats;
use engine::db::conn::open;

/// 插入一行 cleanup_task（必填字段给占位值）。
fn insert_task(
    conn: &rusqlite::Connection,
    session_id: i64,
    freed_bytes: i64,
    success_count: i64,
    fail_count: i64,
) {
    conn.execute(
        "INSERT INTO cleanup_task (session_id, started_at, finished_at,
             disk_before_bytes, disk_after_bytes, freed_bytes,
             success_count, fail_count, skip_count, dry_run_snapshot, safebox_enabled)
         VALUES (?1, 0, 0, 0, 0, ?2, ?3, ?4, 0, '{}', 0)",
        rusqlite::params![session_id, freed_bytes, success_count, fail_count],
    )
    .unwrap();
}

/// 前置：cleanup_task.session_id 是外键，先插 scan_session。
fn ensure_session(conn: &rusqlite::Connection, id: i64) {
    conn.execute(
        "INSERT OR IGNORE INTO scan_session (id, started_at, status, scope)
         VALUES (?1, 0, 'finished', 'full')",
        rusqlite::params![id],
    )
    .unwrap();
}

#[test]
fn stats_empty_returns_zero() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    let s = cleanup_stats(&conn, None).unwrap();
    assert_eq!(s.total_bytes_freed, 0);
    assert_eq!(s.success_count, 0);
    assert_eq!(s.failed_count, 0);
    assert_eq!(s.session_count, 0);
}

#[test]
fn stats_counts_success_failed() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    ensure_session(&conn, 1);
    // 3 次清理会话：每次 1 success + 不同 freed_bytes；1 次只 fail。
    insert_task(&conn, 1, 100, 1, 0);
    insert_task(&conn, 1, 200, 1, 0);
    insert_task(&conn, 1, 300, 1, 2);
    let s = cleanup_stats(&conn, None).unwrap();
    assert_eq!(s.total_bytes_freed, 600);
    assert_eq!(s.success_count, 3);
    assert_eq!(s.failed_count, 2);
    assert_eq!(s.session_count, 1);
}

#[test]
fn stats_filters_by_session() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    ensure_session(&conn, 1);
    ensure_session(&conn, 2);
    insert_task(&conn, 1, 100, 2, 0);
    insert_task(&conn, 2, 50, 1, 0);
    let s = cleanup_stats(&conn, Some(1)).unwrap();
    assert_eq!(s.total_bytes_freed, 100);
    assert_eq!(s.success_count, 2);
    assert_eq!(s.failed_count, 0);
    assert_eq!(s.session_count, 1);
}

#[test]
fn stats_distinct_session_count() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    for sid in [1, 2, 3] {
        ensure_session(&conn, sid);
    }
    insert_task(&conn, 1, 10, 1, 0);
    insert_task(&conn, 2, 20, 1, 0);
    insert_task(&conn, 3, 30, 1, 0);
    let s = cleanup_stats(&conn, None).unwrap();
    assert_eq!(s.session_count, 3);
    assert_eq!(s.total_bytes_freed, 60);
}
