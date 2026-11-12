//! task-024 集成测试：audit_log.list 只读查询。

use engine::audit_log::{list_audit_log, list_scan_sessions, AuditLogFilter};
use engine::db::conn::open;

fn insert_row(conn: &rusqlite::Connection, ts: i64, action: &str, target: &str, result: &str) {
    conn.execute(
        "INSERT INTO audit_log (ts, actor, action, target, detail, result)
         VALUES (?1, 'user', ?2, ?3, NULL, ?4)",
        rusqlite::params![ts, action, target, result],
    )
    .unwrap();
}

#[test]
fn list_empty_returns_empty() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    let v = list_audit_log(&conn, &AuditLogFilter::default()).unwrap();
    assert!(v.is_empty());
}

#[test]
fn list_all_returns_desc_order() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    for i in 0..5 {
        insert_row(
            &conn,
            1000 + i,
            "permanent_delete",
            &format!("/f{}", i),
            "success",
        );
    }
    let v = list_audit_log(&conn, &AuditLogFilter::default()).unwrap();
    assert_eq!(v.len(), 5);
    // ts DESC：最新在前
    assert_eq!(v[0].created_at, 1004);
    assert_eq!(v[4].created_at, 1000);
    assert_eq!(v[0].event_type, "permanent_delete");
    assert_eq!(v[0].path, "/f4");
}

#[test]
fn list_by_action_returns_all_rows() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    insert_row(&conn, 1, "permanent_delete", "/a", "success");
    insert_row(&conn, 2, "permanent_delete", "/b", "fail");
    insert_row(&conn, 3, "restore", "/c", "success");
    insert_row(&conn, 4, "restore", "/d", "success");
    insert_row(&conn, 5, "restore", "/e", "success");

    let v = list_audit_log(
        &conn,
        &AuditLogFilter {
            session_id: None,
            limit: 100,
            offset: 0,
        },
    )
    .unwrap();
    assert_eq!(v.len(), 5);
    assert_eq!(v[0].created_at, 5);
}

#[test]
fn list_filters_by_scan_session() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    for id in [1, 2] {
        conn.execute(
            "INSERT INTO scan_session (id, started_at, status, scope) VALUES (?1, ?1, 'finished', 'full')",
            [id],
        )
        .unwrap();
    }
    for (ts, session) in [(1, 1), (2, 2), (3, 1)] {
        conn.execute(
            "INSERT INTO audit_log (ts, actor, action, target, result, session_id)
             VALUES (?1, 'user', 'permanent_delete', '/f', 'success', ?2)",
            rusqlite::params![ts, session],
        )
        .unwrap();
    }
    insert_row(&conn, 4, "restore", "/legacy", "success");

    let rows = list_audit_log(
        &conn,
        &AuditLogFilter {
            session_id: Some(1),
            limit: 100,
            offset: 0,
        },
    )
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.session_id == Some(1)));
    assert_eq!(rows[0].created_at, 3);

    let sessions = list_scan_sessions(&conn).unwrap();
    assert_eq!(
        sessions.iter().map(|s| s.id).collect::<Vec<_>>(),
        vec![2, 1]
    );
}

#[test]
fn list_pagination_limit_offset() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    for i in 0..10 {
        insert_row(
            &conn,
            1000 + i,
            "permanent_delete",
            &format!("/f{}", i),
            "success",
        );
    }
    let p1 = list_audit_log(
        &conn,
        &AuditLogFilter {
            session_id: None,
            limit: 3,
            offset: 0,
        },
    )
    .unwrap();
    assert_eq!(p1.len(), 3);
    assert_eq!(p1[0].created_at, 1009);
    assert_eq!(p1[2].created_at, 1007);

    let p2 = list_audit_log(
        &conn,
        &AuditLogFilter {
            session_id: None,
            limit: 3,
            offset: 3,
        },
    )
    .unwrap();
    assert_eq!(p2.len(), 3);
    assert_eq!(p2[0].created_at, 1006);
    assert_eq!(p2[2].created_at, 1004);
}
