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

    // task-056：restore 不再 DELETE 行，改为 status='restored' + restored_at 写入。
    // 这样「这个文件什么时候恢复的」可从 safe_box 直接查到（task-017 遗留 B 低）。
    let (status, restored_at): (String, Option<i64>) = conn
        .query_row(
            "SELECT status, restored_at FROM safe_box WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "restored", "恢复后状态应为 restored");
    assert!(
        restored_at.is_some(),
        "restored_at 必须写入（不能是 NULL）"
    );
    assert!(restored_at.unwrap() > 0, "restored_at 应是有效 unix 秒");

    let n_audit: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_log WHERE action='restore'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n_audit, 1);
}

/// task-056：恢复后的行不再出现在保险箱列表 / 不占配额（外部行为与「删行」时代一致）。
#[test]
fn restored_row_disappears_from_list_and_quota() {
    let (dir, conn, box_dir) = setup();
    let src = dir.path().join("gone.tmp");
    let id = move_into_box(&conn, &box_dir, &src);

    let before = engine::execute::safe_box::list_safe_box(&conn).unwrap();
    assert_eq!(before.total_items, 1, "恢复前应在列表里");

    restore_from_safe_box(&conn, &box_dir, id).unwrap();

    let after = engine::execute::safe_box::list_safe_box(&conn).unwrap();
    assert_eq!(after.total_items, 0, "恢复后不应再出现在保险箱列表");
    assert_eq!(after.total_bytes, 0, "恢复后不应再计入保险箱总大小");
    // 行本身还在（审计价值），只是状态变了。
    let n_rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM safe_box WHERE id=?1", [id], |r| r.get(0))
        .unwrap();
    assert_eq!(n_rows, 1, "行应保留（status='restored'），不是被 DELETE");
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
fn restore_parent_missing_auto_creates_dirs() {
    // task-043：原父目录缺失时自动 create_dir_all，不再报 ParentMissing。
    let (dir, conn, box_dir) = setup();
    std::fs::create_dir_all(&box_dir).unwrap();
    let rel = "ghost.bin";
    std::fs::write(box_dir.join(rel), vec![0u8; 10]).unwrap();
    let target = dir.path().join("auto/a/b/ghost.bin");
    conn.execute(
        "INSERT INTO safe_box (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status)
         VALUES (?1, ?2, 10, 1000, 1000+86400, 'in_box')",
        rusqlite::params![target.to_string_lossy().to_string(), rel],
    )
    .unwrap();
    let id = conn.last_insert_rowid();

    let r = restore_from_safe_box(&conn, &box_dir, id).unwrap();
    assert!(target.exists());
    assert_eq!(r.original_path, target.to_string_lossy());
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

/// task-056 负向：同一 id 恢复第二次必须 NotFound。
///
/// 旧实现靠 DELETE 让行消失；新实现行还在（status='restored'），若不靠 status
/// 过滤就会允许「重复恢复」——把已经不存在的 box 文件再 rename 一次。这条用例
/// 守住「行还在 ≠ 可再恢复」。
#[test]
fn restore_twice_second_returns_not_found() {
    let (dir, conn, box_dir) = setup();
    let src = dir.path().join("twice.tmp");
    let id = move_into_box(&conn, &box_dir, &src);

    restore_from_safe_box(&conn, &box_dir, id).unwrap();

    let err = restore_from_safe_box(&conn, &box_dir, id).unwrap_err();
    match err {
        engine::execute::safe_box::RestoreError::NotFound(got) => assert_eq!(got, id),
        other => panic!("expected NotFound on second restore, got {other:?}"),
    }
}

#[test]
fn set_quota_still_works() {
    // 防止未使用警告（task-016 测试已覆盖 quota）
    set_quota_for_test(1024);
}
