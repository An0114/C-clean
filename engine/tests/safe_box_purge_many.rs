//! task-038 集成测试：purge_many。

use engine::db::conn::open;
use engine::execute::safe_box::purge_many;

fn setup() -> (tempfile::TempDir, rusqlite::Connection, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    let box_dir = dir.path().join("safe_box");
    std::fs::create_dir_all(&box_dir).unwrap();
    (dir, conn, box_dir)
}

/// 插一行 safe_box 并写对应磁盘文件，返回 id。
fn insert_box_row(
    conn: &rusqlite::Connection,
    box_dir: &std::path::Path,
    rel: &str,
) -> i64 {
    std::fs::write(box_dir.join(rel), vec![0u8; 10]).unwrap();
    conn.execute(
        "INSERT INTO safe_box (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status)
         VALUES ('C:\\orig\\foo.bin', ?1, 10, 1000, 1000+86400, 'in_box')",
        [rel],
    )
    .unwrap();
    conn.last_insert_rowid()
}

#[test]
fn purge_many_all_success() {
    let (_dir, conn, box_dir) = setup();
    let ids = vec![
        insert_box_row(&conn, &box_dir, "1_a.tmp"),
        insert_box_row(&conn, &box_dir, "2_b.tmp"),
        insert_box_row(&conn, &box_dir, "3_c.tmp"),
    ];

    let r = purge_many(&conn, &box_dir, &ids);
    assert_eq!(r.deleted, 3);
    assert_eq!(r.failed, 0);
    assert!(r.errors.is_empty());
}

#[test]
fn purge_many_partial_fail() {
    let (_dir, conn, box_dir) = setup();
    let id1 = insert_box_row(&conn, &box_dir, "ok1.tmp");
    let id2 = insert_box_row(&conn, &box_dir, "ok2.tmp");
    let ghost = 999_999_i64;

    let r = purge_many(&conn, &box_dir, &[id1, id2, ghost]);
    assert_eq!(r.deleted, 2);
    assert_eq!(r.failed, 1);
    assert_eq!(r.errors.len(), 1);
    assert!(r.errors[0].contains(&format!("id={ghost}")), "{}", r.errors[0]);
}

#[test]
fn purge_many_empty() {
    let (_dir, conn, box_dir) = setup();
    let r = purge_many(&conn, &box_dir, &[]);
    assert_eq!(r.deleted, 0);
    assert_eq!(r.failed, 0);
    assert!(r.errors.is_empty());
}
