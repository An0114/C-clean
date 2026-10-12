//! task-017 集成测试：restore_from_safe_box。

use engine::db::conn::open;
use engine::execute::delete::Deleter;
use engine::execute::normalize::normalize;
use engine::execute::safe_box::{restore_from_safe_box, set_quota_for_test, SafeBoxDeleter};

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
) -> i64 {
    std::fs::write(src, vec![0u8; 100]).unwrap();
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
fn restore_returns_file_to_original_path() {
    let (dir, conn, box_dir) = setup();
    let src = dir.path().join("a.tmp");
    let id = move_into_box(&conn, &box_dir, &src);
    assert!(!src.exists());

    let r = restore_from_safe_box(&conn, &box_dir, id).unwrap();
    assert_eq!(r.safe_box_id, id);
    assert!(src.exists());

    let n_box: i64 = conn
        .query_row("SELECT COUNT(*) FROM safe_box WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(n_box, 0);
    let n_audit: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_log WHERE action='restore'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n_audit, 1);
}

#[test]
fn restore_target_occupied_returns_error() {
    let (dir, conn, box_dir) = setup();
    let src = dir.path().join("b.tmp");
    let id = move_into_box(&conn, &box_dir, &src);

    // 在原路径写一个新文件
    std::fs::write(&src, vec![0u8; 50]).unwrap();

    let err = restore_from_safe_box(&conn, &box_dir, id).unwrap_err();
    match err {
        engine::execute::safe_box::RestoreError::TargetOccupied(_) => {}
        other => panic!("expected TargetOccupied, got {other:?}"),
    }
    // box 文件没动，DB 行还在
    let n_box: i64 = conn
        .query_row("SELECT COUNT(*) FROM safe_box WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(n_box, 1);
}

#[test]
fn restore_parent_missing_returns_error() {
    let (_dir, conn, box_dir) = setup();
    // 直接塞一行 original_path 指向不存在的父目录
    std::fs::create_dir_all(&box_dir).unwrap();
    let rel = "ghost.bin";
    std::fs::write(box_dir.join(rel), vec![0u8; 10]).unwrap();
    conn.execute(
        "INSERT INTO safe_box (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status)
         VALUES ('C:\\nonexistent\\foo\\ghost.bin', ?1, 10, 1000, 1000+86400, 'in_box')",
        [rel],
    )
    .unwrap();
    let id = conn.last_insert_rowid();

    let err = restore_from_safe_box(&conn, &box_dir, id).unwrap_err();
    match err {
        engine::execute::safe_box::RestoreError::ParentMissing(_) => {}
        other => panic!("expected ParentMissing, got {other:?}"),
    }
}

#[test]
fn restore_purged_row_returns_not_found() {
    let (_dir, conn, box_dir) = setup();
    std::fs::create_dir_all(&box_dir).unwrap();
    let rel = "purged.bin";
    std::fs::write(box_dir.join(rel), vec![0u8; 10]).unwrap();
    conn.execute(
        "INSERT INTO safe_box (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status)
         VALUES ('C:\\orig\\purged.bin', ?1, 10, 1000, 1000+86400, 'purged')",
        [rel],
    )
    .unwrap();
    let id = conn.last_insert_rowid();

    let err = restore_from_safe_box(&conn, &box_dir, id).unwrap_err();
    match err {
        engine::execute::safe_box::RestoreError::NotFound(_) => {}
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn set_quota_still_works() {
    // 防止未使用警告（task-016 测试已覆盖 quota）
    set_quota_for_test(1024);
}
