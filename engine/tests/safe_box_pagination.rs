//! task-045 闆嗘垚娴嬭瘯锛歴afe_box.list_paged 鍒嗛〉銆?

use engine::db::conn::open;
use engine::execute::safe_box::{list_paged, SafeBoxFilter};

fn setup() -> (tempfile::TempDir, rusqlite::Connection, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    let box_dir = dir.path().join("safe_box");
    std::fs::create_dir_all(&box_dir).unwrap();
    (dir, conn, box_dir)
}

/// 鐩存帴鍚?safe_box 琛ㄦ彃 n 琛岋紙status='in_box'锛夛紝original_path 鐢ㄥ敮涓€璺緞銆?
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
    let p = list_paged(&conn, 1, 50, &SafeBoxFilter::default()).unwrap();
    assert_eq!(p.items.len(), 50);
    assert_eq!(p.total, 60);
    assert_eq!(p.page, 1);
    assert_eq!(p.page_size, 50);
}

#[test]
fn list_paged_returns_second_page() {
    let (_dir, conn, _box) = setup();
    insert_n(&conn, 60);
    let p = list_paged(&conn, 2, 50, &SafeBoxFilter::default()).unwrap();
    assert_eq!(p.items.len(), 10);
    assert_eq!(p.total, 60);
    assert_eq!(p.page, 2);
}

#[test]
fn list_paged_clamps_page_size() {
    let (_dir, conn, _box) = setup();
    insert_n(&conn, 60);
    let p = list_paged(&conn, 1, 99999, &SafeBoxFilter::default()).unwrap();
    assert_eq!(p.page_size, 200);
    assert_eq!(p.items.len(), 60);
}

#[test]
fn list_paged_empty_when_beyond_last_page() {
    let (_dir, conn, _box) = setup();
    insert_n(&conn, 60);
    let p = list_paged(&conn, 99, 50, &SafeBoxFilter::default()).unwrap();
    assert_eq!(p.items.len(), 0);
    assert_eq!(p.total, 60);
}

#[test]
fn list_paged_id_desc_order() {
    let (_dir, conn, _box) = setup();
    insert_n(&conn, 30);
    let p1 = list_paged(&conn, 1, 10, &SafeBoxFilter::default()).unwrap();
    let p2 = list_paged(&conn, 2, 10, &SafeBoxFilter::default()).unwrap();
    // 绗?1 椤电涓€鏉?id 搴斿ぇ浜庣 2 椤电涓€鏉?id锛坕d DESC锛夈€?
    assert!(
        p1.items[0].id > p2.items[0].id,
        "p1[0].id={} p2[0].id={}",
        p1.items[0].id,
        p2.items[0].id
    );
    // 绗?1 椤垫渶鍚庝竴鏉?id 搴斿ぇ浜庣 2 椤电涓€鏉?id銆?
    assert!(p1.items[9].id > p2.items[0].id);
}
