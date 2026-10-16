//! task-046 集成测试：safe_box session 过滤。
//!
//! 注意：真实 schema 里 safe_box 没有 session_id 列，只有 source_task_id
//! 外键（一个 cleanup_task 对应一次清理 session）。以下测试里的 session_id
//! 写库时落到 source_task_id 列。

use engine::db::conn::open;
use engine::execute::safe_box::{list_paged, list_sessions};

fn setup() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    (dir, conn)
}

/// 插一行 safe_box，指定 source_task_id（即测试语义上的 session_id）。
fn insert_one(conn: &rusqlite::Connection, session_id: i64, idx: i64, ts: i64) {
    conn.execute(
        "INSERT INTO safe_box (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status, source_task_id)
         VALUES (?1, ?2, 10, ?3, ?3+86400, 'in_box', ?4)",
        rusqlite::params![format!("C:\\ghost\\s{session_id}_{idx}.bin"), format!("s{session_id}_{idx}.bin"), ts, session_id],
    )
    .unwrap();
}

/// 造 5 行：session 1 有 3 条、session 2 有 2 条。
///
/// source_task_id 是 cleanup_task 外键，先插 scan_session + cleanup_task
/// 以满足 FK 约束。
fn seed_two_sessions(conn: &rusqlite::Connection) {
    for s in [1i64, 2] {
        conn.execute(
            "INSERT INTO scan_session (id, started_at, status, scope) VALUES (?1, ?2, 'finished', 'full')",
            rusqlite::params![s, 1000 + s * 1000],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO cleanup_task (id, session_id, started_at, finished_at, disk_before_bytes, disk_after_bytes, dry_run_snapshot, safebox_enabled)
             VALUES (?1, ?1, ?2, ?2, 100000, 90000, '[]', 1)",
            rusqlite::params![s, 1000 + s * 1000],
        )
        .unwrap();
    }
    for i in 0..3 {
        insert_one(conn, 1, i, 1000 + i);
    }
    for i in 0..2 {
        insert_one(conn, 2, i, 5000 + i);
    }
}

#[test]
fn list_paged_no_session_returns_all() {
    let (_dir, conn) = setup();
    seed_two_sessions(&conn);
    let p = list_paged(&conn, 1, 100, None).unwrap();
    assert_eq!(p.items.len(), 5);
    assert_eq!(p.total, 5);
}

#[test]
fn list_paged_filter_by_session() {
    let (_dir, conn) = setup();
    seed_two_sessions(&conn);
    let p = list_paged(&conn, 1, 100, Some(1)).unwrap();
    assert_eq!(p.items.len(), 3);
    assert_eq!(p.total, 3);
    // 所有行都应是 session 1 来源。
    for it in &p.items {
        assert!(it.original_path.contains("s1_"));
    }
}

#[test]
fn list_paged_filter_no_match() {
    let (_dir, conn) = setup();
    seed_two_sessions(&conn);
    let p = list_paged(&conn, 1, 100, Some(999)).unwrap();
    assert_eq!(p.items.len(), 0);
    assert_eq!(p.total, 0);
}

#[test]
fn list_sessions_groups_by_session() {
    let (_dir, conn) = setup();
    seed_two_sessions(&conn);
    let sessions = list_sessions(&conn).unwrap();
    assert_eq!(sessions.len(), 2);
    // ORDER BY source_task_id DESC：session 2 在前。
    assert_eq!(sessions[0].session_id, 2);
    assert_eq!(sessions[0].item_count, 2);
    assert_eq!(sessions[0].created_ts, 5000);
    assert_eq!(sessions[1].session_id, 1);
    assert_eq!(sessions[1].item_count, 3);
    assert_eq!(sessions[1].created_ts, 1000);
}
