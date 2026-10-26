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
    // task-052 G4：session_count 口径改为 cleanup_task 行数 → 3 行 = 3 轮。
    assert_eq!(s.session_count, 3);
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
fn stats_session_count_equals_task_rows() {
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

/// 插入一行 `session_id = NULL` 的 cleanup_task（复刻 `execute_items` 的真实写入）。
fn insert_task_null_session(conn: &rusqlite::Connection, freed_bytes: i64) {
    conn.execute(
        "INSERT INTO cleanup_task (session_id, started_at, finished_at,
             disk_before_bytes, disk_after_bytes, freed_bytes,
             success_count, fail_count, skip_count, dry_run_snapshot, safebox_enabled)
         VALUES (NULL, 0, 0, 0, 0, ?1, 1, 0, 0, '{}', 0)",
        rusqlite::params![freed_bytes],
    )
    .unwrap();
}

/// task-052 G4 回归（核心）：`execute_items` 写 cleanup_task 时 `session_id` 恒为 NULL，
/// 旧的 `COUNT(DISTINCT session_id)` 会忽略 NULL → 前端「清理轮次」恒 0（走查 G4）。
/// 改成 `COUNT(*)` 后，两行 NULL session 的批次必须算 2 轮。
#[test]
fn stats_counts_null_session_tasks() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    insert_task_null_session(&conn, 100);
    insert_task_null_session(&conn, 250);

    let s = cleanup_stats(&conn, None).unwrap();
    assert_eq!(
        s.session_count, 2,
        "两行 NULL session 的 cleanup_task 必须算 2 轮"
    );
    assert_eq!(s.total_bytes_freed, 350);
    assert_eq!(s.success_count, 2);
}

/// task-052 G4：混合 NULL 与非 NULL session 时，轮次 = 行数（不是非 NULL 去重数）。
#[test]
fn stats_counts_mixed_null_and_non_null() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    ensure_session(&conn, 1);
    insert_task(&conn, 1, 10, 1, 0);
    insert_task(&conn, 1, 20, 1, 0);
    insert_task_null_session(&conn, 30);

    let s = cleanup_stats(&conn, None).unwrap();
    assert_eq!(s.session_count, 3);
}
