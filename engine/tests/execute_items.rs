//! task-015 集成测试：`execute_items` 编排（真删文件 + 落 audit_log/cleanup_task）。

use engine::db::conn::open;
use engine::execute::executor::execute_items;
use engine::execute::fs_deleter::FsDeleter;

/// 造临时库 + scan_session，返回 (tmpdir, conn, session_id)。
fn setup() -> (tempfile::TempDir, rusqlite::Connection, i64) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (1000, 'running', 'full')",
        [],
    )
    .unwrap();
    let sid = conn.last_insert_rowid();
    (dir, conn, sid)
}

/// 在 cleanup_item 插入一行，返回 id。
fn insert_item(conn: &rusqlite::Connection, sid: i64, path: &str) -> i64 {
    conn.execute(
        "INSERT INTO cleanup_item
         (session_id, module_code, category, path, size_bytes, risk_level, description)
         VALUES (?1, 'A', 'test', ?2, 100, 'L1', 'desc')",
        rusqlite::params![sid, path],
    )
    .unwrap();
    conn.last_insert_rowid()
}

#[test]
fn execute_items_deletes_real_files() {
    let (dir, conn, sid) = setup();
    let f1 = dir.path().join("a.tmp");
    let f2 = dir.path().join("b.tmp");
    let f3 = dir.path().join("c.tmp");
    std::fs::write(&f1, vec![0u8; 100]).unwrap();
    std::fs::write(&f2, vec![0u8; 100]).unwrap();
    std::fs::write(&f3, vec![0u8; 100]).unwrap();
    let ids: Vec<i64> = [&f1, &f2, &f3]
        .iter()
        .map(|p| insert_item(&conn, sid, p.to_str().unwrap()))
        .collect();

    let summary = execute_items(&ids, &conn, &[], |_id| Box::new(FsDeleter));
    assert_eq!(summary.deleted_count, 3);
    assert!(!f1.exists() && !f2.exists() && !f3.exists());

    let n_task: i64 = conn
        .query_row("SELECT COUNT(*) FROM cleanup_task", [], |r| r.get(0))
        .unwrap();
    let n_success: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_log WHERE result='success'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n_task, 1);
    assert_eq!(n_success, 3);
}

#[test]
fn execute_items_blocked_by_l3() {
    let (_dir, conn, sid) = setup();
    // 命中内置 L3：Desktop 下的文件。
    let id = insert_item(&conn, sid, r"C:\Users\alice\Desktop\secret.docx");

    let summary = execute_items(&[id], &conn, &[], |_id| Box::new(FsDeleter));
    assert_eq!(summary.deleted_count, 0);
    assert_eq!(summary.failed_count, 1);
    assert!(summary.results[0].error.as_deref().unwrap().contains("L3"));

    let n_success: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_log WHERE result='success'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let n_denied: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_log WHERE result='denied'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n_success, 0);
    assert_eq!(n_denied, 1);
}

#[test]
fn execute_items_skips_missing_id() {
    let (dir, conn, sid) = setup();
    let f1 = dir.path().join("x.tmp");
    std::fs::write(&f1, vec![0u8; 10]).unwrap();
    let id_real = insert_item(&conn, sid, f1.to_str().unwrap());

    let summary = execute_items(&[id_real, 999999], &conn, &[], |_id| Box::new(FsDeleter));
    assert_eq!(summary.deleted_count, 1);
    // 第二条是 not found，不算 fail（skip）
    let skipped: Vec<_> = summary
        .results
        .iter()
        .filter(|r| r.error.as_deref() == Some("not found"))
        .collect();
    assert_eq!(skipped.len(), 1);
    assert!(!f1.exists());
}

#[test]
fn execute_items_idempotent_on_already_deleted() {
    let (dir, conn, sid) = setup();
    let f1 = dir.path().join("y.tmp");
    std::fs::write(&f1, vec![0u8; 10]).unwrap();
    let id = insert_item(&conn, sid, f1.to_str().unwrap());

    let s1 = execute_items(&[id], &conn, &[], |_id| Box::new(FsDeleter));
    assert_eq!(s1.deleted_count, 1);
    // 第二次删：文件已不存在 → FsDeleter 返回 Ok(()) 幂等
    let s2 = execute_items(&[id], &conn, &[], |_id| Box::new(FsDeleter));
    assert_eq!(s2.deleted_count, 1);
    // 两次都写了 success audit
    let n_success: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_log WHERE result='success'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n_success, 2);
}
