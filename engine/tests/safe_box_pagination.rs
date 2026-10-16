//! task-045 集成测试：safe_box.list_paged 分页。

use engine::db::conn::open;
use engine::execute::safe_box::list_paged;

fn setup() -> (tempfile::TempDir, rusqlite::Connection, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    let box_dir = dir.path().join("safe_box");
    std::fs::create_dir_all(&box_dir).unwrap();
    (dir, conn, box_dir)
}

/// 直接向 safe_box 表插 n 行（status='in_box'），original_path 用唯一路径。
fn insert_n(conn: &rusqlite::Connection, n: i64) {
    for i in 0..n {
        conn.execute(
            "INSERT INTO safe_box (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status)
             VALUES (?1, ?2, 10, 1000+?3, 1000+86400+?3, 'in_box')",
            rusqlite::params![format!("C:\\ghost\\{i}.bin"), format!("{i}.bin"), i],
        )
        .unwrap();
    }
}

#[test]
fn list_paged_returns_first_page() {
    let (_dir, conn, _box) = setup();
    insert_n(&conn, 60);
    let p = list_paged(&conn, 1, 50).unwrap();
    assert_eq!(p.items.len(), 50);
    assert_eq!(p.total, 60);
    assert_eq!(p.page, 1);
    assert_eq!(p.page_size, 50);
}

#[test]
fn list_paged_returns_second_page() {
    let (_dir, conn, _box) = setup();
    insert_n(&conn, 60);
    let p = list_paged(&conn, 2, 50).unwrap();
    assert_eq!(p.items.len(), 10);
    assert_eq!(p.total, 60);
    assert_eq!(p.page, 2);
}

#[test]
fn list_paged_clamps_page_size() {
    let (_dir, conn, _box) = setup();
    insert_n(&conn, 60);
    let p = list_paged(&conn, 1, 99999).unwrap();
    assert_eq!(p.page_size, 200);
    assert_eq!(p.items.len(), 60);
}

#[test]
fn list_paged_empty_when_beyond_last_page() {
    let (_dir, conn, _box) = setup();
    insert_n(&conn, 60);
    let p = list_paged(&conn, 99, 50).unwrap();
    assert_eq!(p.items.len(), 0);
    assert_eq!(p.total, 60);
}

#[test]
fn list_paged_id_desc_order() {
    let (_dir, conn, _box) = setup();
    insert_n(&conn, 30);
    let p1 = list_paged(&conn, 1, 10).unwrap();
    let p2 = list_paged(&conn, 2, 10).unwrap();
    // 第 1 页第一条 id 应大于第 2 页第一条 id（id DESC）。
    assert!(
        p1.items[0].id > p2.items[0].id,
        "p1[0].id={} p2[0].id={}",
        p1.items[0].id,
        p2.items[0].id
    );
    // 第 1 页最后一条 id 应大于第 2 页第一条 id。
    assert!(p1.items[9].id > p2.items[0].id);
}
