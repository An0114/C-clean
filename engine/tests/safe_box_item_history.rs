//! task-051 集成测试：按保险箱条目查该原路径的审计历史（C15）。

use engine::db::conn::open;
use engine::execute::safe_box::item_history;

fn setup() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    (dir, conn)
}

/// 插一条 in_box 行，返回 safe_box.id。
fn insert_box_row(conn: &rusqlite::Connection, original_path: &str) -> i64 {
    conn.execute(
        "INSERT INTO safe_box (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status)
         VALUES (?1, 'x.bin', 10, 1000, 700000, 'in_box')",
        rusqlite::params![original_path],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// 插一条 audit_log。
fn insert_audit(conn: &rusqlite::Connection, ts: i64, action: &str, target: &str, result: &str) {
    conn.execute(
        "INSERT INTO audit_log (ts, actor, action, target, detail, result)
         VALUES (?1, 'user', ?2, ?3, '{}', ?4)",
        rusqlite::params![ts, action, target, result],
    )
    .unwrap();
}

#[test]
fn returns_history_of_same_path_in_ts_desc_order() {
    let (_dir, conn) = setup();
    let path = "C:\\Temp\\A.txt";
    let id = insert_box_row(&conn, path);

    insert_audit(&conn, 100, "quarantine", path, "success");
    insert_audit(&conn, 300, "restore", path, "success");
    insert_audit(&conn, 200, "permanent_delete", path, "fail");

    let h = item_history(&conn, id).unwrap();

    assert_eq!(h.len(), 3, "三条都该命中");
    let ts: Vec<i64> = h.iter().map(|e| e.created_at).collect();
    assert_eq!(ts, vec![300, 200, 100], "必须 ts DESC");
    assert_eq!(h[0].event_type, "restore");
    assert_eq!(h[1].result, "fail");
}

#[test]
fn path_match_is_case_insensitive() {
    let (_dir, conn) = setup();
    // 条目路径是小写，历史是大写（或反之）——Windows 路径大小写不敏感。
    let id = insert_box_row(&conn, "c:\\temp\\a.txt");
    insert_audit(&conn, 100, "quarantine", "C:\\Temp\\A.txt", "success");

    let h = item_history(&conn, id).unwrap();

    assert_eq!(h.len(), 1, "大小写不同也应命中（COLLATE NOCASE）");
    assert_eq!(h[0].path, "C:\\Temp\\A.txt", "返回的是库里原始写法");
}

#[test]
fn unrelated_paths_are_not_returned() {
    let (_dir, conn) = setup();
    let id = insert_box_row(&conn, "C:\\Temp\\A.txt");
    insert_audit(&conn, 100, "quarantine", "C:\\Temp\\B.txt", "success");

    let h = item_history(&conn, id).unwrap();

    assert!(h.is_empty(), "同目录不同文件不应混入");
}

#[test]
fn no_history_returns_empty_vec_not_error() {
    let (_dir, conn) = setup();
    let id = insert_box_row(&conn, "C:\\Temp\\nohistory.txt");

    let h = item_history(&conn, id).unwrap();

    assert!(h.is_empty());
}

#[test]
fn missing_item_returns_not_found_error() {
    let (_dir, conn) = setup();

    let err = item_history(&conn, 424242).unwrap_err();

    assert!(
        err.contains("not found"),
        "错误信息应含 not found，实际: {err}"
    );
    assert!(err.contains("424242"), "错误信息应带 id，实际: {err}");
}

#[test]
fn limit_caps_result_to_20() {
    let (_dir, conn) = setup();
    let path = "C:\\Temp\\many.txt";
    let id = insert_box_row(&conn, path);
    for i in 0..30 {
        insert_audit(&conn, 1000 + i, "scan", path, "success");
    }

    let h = item_history(&conn, id).unwrap();

    assert_eq!(h.len(), 20, "item_history 固定取最近 20 条");
    assert_eq!(h[0].created_at, 1029, "最新一条在前");
}
