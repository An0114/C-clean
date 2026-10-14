//! task-037 集成测试：purge_item。

use engine::db::conn::open;
use engine::execute::safe_box::purge_item;

fn setup() -> (tempfile::TempDir, rusqlite::Connection, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    let box_dir = dir.path().join("safe_box");
    std::fs::create_dir_all(&box_dir).unwrap();
    (dir, conn, box_dir)
}

/// 直接插一行 safe_box，并在 box_dir 下写对应文件，返回 id。
fn insert_box_row(
    conn: &rusqlite::Connection,
    box_dir: &std::path::Path,
    rel: &str,
    size: usize,
) -> i64 {
    std::fs::write(box_dir.join(rel), vec![0u8; size]).unwrap();
    conn.execute(
        "INSERT INTO safe_box (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status)
         VALUES ('C:\\orig\\foo.bin', ?1, ?2, 1000, 1000+86400, 'in_box')",
        rusqlite::params![rel, size as i64],
    )
    .unwrap();
    conn.last_insert_rowid()
}

#[test]
fn purge_removes_file_and_db_row() {
    let (_dir, conn, box_dir) = setup();
    let id = insert_box_row(&conn, &box_dir, "1000_a.tmp", 100);
    assert!(box_dir.join("1000_a.tmp").exists());

    purge_item(&conn, &box_dir, id).unwrap();

    // 磁盘文件已删
    assert!(!box_dir.join("1000_a.tmp").exists());
    // DB 行已删
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM safe_box WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(n, 0);
    // audit_log 写了一条 purge
    let n_audit: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_log WHERE action='purge'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n_audit, 1);
}

#[test]
fn purge_missing_file_ok() {
    let (_dir, conn, box_dir) = setup();
    // DB 行在，但磁盘文件已被手动删
    let rel = "2000_gone.tmp";
    conn.execute(
        "INSERT INTO safe_box (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status)
         VALUES ('C:\\orig\\gone.bin', ?1, 50, 1000, 1000+86400, 'in_box')",
        [rel],
    )
    .unwrap();
    let id = conn.last_insert_rowid();
    assert!(!box_dir.join(rel).exists());

    // purge 不报错
    purge_item(&conn, &box_dir, id).unwrap();

    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM safe_box WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(n, 0);
}

#[test]
fn purge_not_found_returns_err() {
    let (_dir, conn, box_dir) = setup();
    let err = purge_item(&conn, &box_dir, 99999).unwrap_err();
    assert!(err.contains("not found"), "unexpected: {err}");
}
