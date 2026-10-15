//! task-039 集成测试：restore_many。

use engine::db::conn::open;
use engine::execute::safe_box::restore_many;

fn setup() -> (tempfile::TempDir, rusqlite::Connection, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    let box_dir = dir.path().join("safe_box");
    std::fs::create_dir_all(&box_dir).unwrap();
    (dir, conn, box_dir)
}

/// 插一行 safe_box（original_path 指向一个临时不存在的文件），返回 id。
/// 需要调用方保证 original_path 所在父目录存在、且原文件不存在。
fn insert_box_row(
    conn: &rusqlite::Connection,
    box_dir: &std::path::Path,
    rel: &str,
    original_path: &std::path::Path,
) -> i64 {
    std::fs::write(box_dir.join(rel), vec![0u8; 10]).unwrap();
    conn.execute(
        "INSERT INTO safe_box (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status)
         VALUES (?1, ?2, 10, 1000, 1000+86400, 'in_box')",
        rusqlite::params![original_path.to_string_lossy().to_string(), rel],
    )
    .unwrap();
    conn.last_insert_rowid()
}

#[test]
fn restore_many_all_success() {
    let (dir, conn, box_dir) = setup();
    // 原文件父目录 = tempdir 本身（存在）；原文件不存在。
    let p1 = dir.path().join("a_restored.bin");
    let p2 = dir.path().join("b_restored.bin");
    let id1 = insert_box_row(&conn, &box_dir, "1_a.tmp", &p1);
    let id2 = insert_box_row(&conn, &box_dir, "2_b.tmp", &p2);

    let r = restore_many(&conn, &box_dir, &[id1, id2]);
    assert_eq!(r.succeeded, 2);
    assert_eq!(r.failed, 0);
    assert!(r.errors.is_empty());
    assert!(p1.exists());
    assert!(p2.exists());
}

#[test]
fn restore_many_partial_fail() {
    let (dir, conn, box_dir) = setup();
    let p1 = dir.path().join("ok.bin");
    let id1 = insert_box_row(&conn, &box_dir, "ok.tmp", &p1);
    let ghost = 999_999_i64;

    let r = restore_many(&conn, &box_dir, &[id1, ghost]);
    assert_eq!(r.succeeded, 1);
    assert_eq!(r.failed, 1);
    assert_eq!(r.errors.len(), 1);
    assert!(
        r.errors[0].contains(&format!("id={ghost}")),
        "{}",
        r.errors[0]
    );
    assert!(p1.exists());
}

#[test]
fn restore_many_empty() {
    let (_dir, conn, box_dir) = setup();
    let r = restore_many(&conn, &box_dir, &[]);
    assert_eq!(r.succeeded, 0);
    assert_eq!(r.failed, 0);
    assert!(r.errors.is_empty());
}

/// 失败项夹在成功项中间：证明单个失败不阻断「后续 id」继续处理
/// （partial_fail 的失败项在末尾，无法证明这一点），且错误文案区分 ParentMissing。
#[test]
fn restore_many_failure_in_middle_does_not_stop_rest() {
    let (dir, conn, box_dir) = setup();
    let p1 = dir.path().join("first.bin");
    // 父目录不存在 → restore_from_safe_box 返 ParentMissing。
    let p2 = dir.path().join("no_such_dir").join("second.bin");
    let p3 = dir.path().join("third.bin");
    let id1 = insert_box_row(&conn, &box_dir, "m1.tmp", &p1);
    let id2 = insert_box_row(&conn, &box_dir, "m2.tmp", &p2);
    let id3 = insert_box_row(&conn, &box_dir, "m3.tmp", &p3);

    let r = restore_many(&conn, &box_dir, &[id1, id2, id3]);
    assert_eq!(r.succeeded, 2);
    assert_eq!(r.failed, 1);
    assert_eq!(r.errors.len(), 1);
    assert!(
        r.errors[0].contains(&format!("id={id2}")),
        "{}",
        r.errors[0]
    );
    assert!(r.errors[0].contains("父目录"), "{}", r.errors[0]);
    // 中间失败后，第三个 id 仍被处理并成功。
    assert!(p1.exists());
    assert!(p3.exists());
    assert!(!p2.exists());
}
