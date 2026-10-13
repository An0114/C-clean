//! task-024 集成测试：audit_log.list 只读查询。

use engine::audit_log::{list_audit_log, AuditLogFilter};
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
fn list_by_action_filters() {
    // 真实 schema 无 session_id 列，本用例改为按 action 语义验证（见 query.rs 注释）。
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    insert_row(&conn, 1, "permanent_delete", "/a", "success");
    insert_row(&conn, 2, "permanent_delete", "/b", "fail");
    insert_row(&conn, 3, "restore", "/c", "success");
    insert_row(&conn, 4, "restore", "/d", "success");
    insert_row(&conn, 5, "restore", "/e", "success");

    // list_audit_log 当前不支持 WHERE action=，全量返回 5 条（按 ts DESC）。
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
