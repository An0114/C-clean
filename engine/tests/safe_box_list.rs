//! task-018 集成测试：list_safe_box。

use engine::db::conn::open;
use engine::execute::delete::Deleter;
use engine::execute::normalize::normalize;
use engine::execute::safe_box::list_safe_box;
use engine::execute::safe_box::restore_from_safe_box;
use engine::execute::safe_box::SafeBoxDeleter;

fn setup() -> (tempfile::TempDir, rusqlite::Connection, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (1000, 'running', 'full')",
        [],
    )
    .unwrap();
    let box_dir = dir.path().join("safe_box");
    (dir, conn, box_dir)
}

/// 把 src 文件移入保险箱，返回 safe_box.id。
fn move_into_box(
    conn: &rusqlite::Connection,
    box_dir: &std::path::Path,
    src: &std::path::Path,
    size: usize,
) -> i64 {
    std::fs::write(src, vec![0u8; size]).unwrap();
    let deleter = SafeBoxDeleter::new(box_dir, conn, None);
    let np = normalize(src.to_str().unwrap()).unwrap();
    deleter.delete_file(&np).unwrap();
    conn.query_row(
        "SELECT id FROM safe_box ORDER BY id DESC LIMIT 1",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn list_empty_returns_zero() {
    let (_dir, conn, _box) = setup();
    let r = list_safe_box(&conn).unwrap();
    assert_eq!(r.total_items, 0);
    assert_eq!(r.total_bytes, 0);
    assert!(r.entries.is_empty());
}

#[test]
fn list_returns_inserted_entries() {
    let (dir, conn, box_dir) = setup();
    let a = dir.path().join("a.tmp");
    let b = dir.path().join("b.tmp");
    let c = dir.path().join("c.tmp");
    move_into_box(&conn, &box_dir, &a, 100);
    move_into_box(&conn, &box_dir, &b, 200);
    move_into_box(&conn, &box_dir, &c, 300);

    let r = list_safe_box(&conn).unwrap();
    assert_eq!(r.total_items, 3);
    assert_eq!(r.total_bytes, 600);
    assert_eq!(r.entries.len(), 3);
    // 按 quarantined_at DESC；三条插入时间戳相同则按 id DESC 兜底（这里只查 count）
}

#[test]
fn list_excludes_purged() {
    let (dir, conn, box_dir) = setup();
    let a = dir.path().join("a.tmp");
    let b = dir.path().join("b.tmp");
    move_into_box(&conn, &box_dir, &a, 100);
    move_into_box(&conn, &box_dir, &b, 200);

    // 把第一行手动标 purged
    conn.execute("UPDATE safe_box SET status='purged' WHERE id=1", [])
        .unwrap();

    let r = list_safe_box(&conn).unwrap();
    assert_eq!(r.total_items, 1);
    assert_eq!(r.total_bytes, 200);
}

#[test]
fn list_excludes_restored() {
    let (dir, conn, box_dir) = setup();
    let a = dir.path().join("a.tmp");
    let b = dir.path().join("b.tmp");
    let id_a = move_into_box(&conn, &box_dir, &a, 100);
    move_into_box(&conn, &box_dir, &b, 200);

    restore_from_safe_box(&conn, &box_dir, id_a).unwrap();

    let r = list_safe_box(&conn).unwrap();
    assert_eq!(r.total_items, 1);
    assert_eq!(r.total_bytes, 200);
    assert!(r.entries[0].original_path.ends_with("b.tmp"));
}
