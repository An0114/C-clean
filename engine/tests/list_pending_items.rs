//! task-029 集成测试：list_pending_items。
//!
//! 与任务书偏差：cleanup_item 表无 status 列，「pending」= 表内所有行。
//! 故用例1 改为「列出所有行」（不区分状态），用例2/3 保持 session 过滤 / size 倒序。

use engine::db::conn::open;
use engine::execute::list_pending::list_pending_items;

fn ensure_session(conn: &rusqlite::Connection, id: i64) {
    conn.execute(
        "INSERT OR IGNORE INTO scan_session (id, started_at, status, scope)
         VALUES (?1, 0, 'finished', 'full')",
        rusqlite::params![id],
    )
    .unwrap();
}

fn insert_item(conn: &rusqlite::Connection, session_id: i64, size: i64, path: &str, mod_code: &str) {
    conn.execute(
        "INSERT INTO cleanup_item (session_id, module_code, category, path, size_bytes,
             risk_level, description)
         VALUES (?1, ?2, 'cat', ?3, ?4, 'L1', 'desc')",
        rusqlite::params![session_id, mod_code, path, size],
    )
    .unwrap();
}

#[test]
fn list_returns_all_rows_no_status_column() {
    // 任务书原假设 status='pending' 过滤；实际表无 status 列，返回所有行。
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    ensure_session(&conn, 1);
    insert_item(&conn, 1, 100, "/a", "A");
    insert_item(&conn, 1, 200, "/b", "B");
    let rows = list_pending_items(&conn, None, 100, 0).unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn list_filters_by_session() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    ensure_session(&conn, 1);
    ensure_session(&conn, 2);
    insert_item(&conn, 1, 100, "/a", "A");
    insert_item(&conn, 2, 200, "/b", "B");
    let rows = list_pending_items(&conn, Some(1), 100, 0).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].path, "/a");
}

#[test]
fn list_orders_by_size_desc() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    ensure_session(&conn, 1);
    insert_item(&conn, 1, 100, "/small", "A");
    insert_item(&conn, 1, 300, "/big", "A");
    insert_item(&conn, 1, 200, "/mid", "A");
    let rows = list_pending_items(&conn, None, 100, 0).unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].size_bytes, 300);
    assert_eq!(rows[1].size_bytes, 200);
    assert_eq!(rows[2].size_bytes, 100);
}

#[test]
fn list_applies_limit_offset() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    ensure_session(&conn, 1);
    for i in 0..5 {
        insert_item(&conn, 1, i * 100, &format!("/p{}", i), "A");
    }
    let page = list_pending_items(&conn, None, 2, 0).unwrap();
    assert_eq!(page.len(), 2);
    assert_eq!(page[0].size_bytes, 400);
    let page2 = list_pending_items(&conn, None, 2, 2).unwrap();
    assert_eq!(page2.len(), 2);
    assert_eq!(page2[0].size_bytes, 200);
}
