//! task-043 集成测试：restore 自动 create_dir_all 父目录。

use engine::db::conn::open;
use engine::execute::safe_box::restore_from_safe_box;

fn setup() -> (tempfile::TempDir, rusqlite::Connection, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    let box_dir = dir.path().join("safe_box");
    std::fs::create_dir_all(&box_dir).unwrap();
    (dir, conn, box_dir)
}

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
fn restore_creates_missing_parent_dir() {
    let (dir, conn, box_dir) = setup();
    // 原路径位于不存在的多层子目录 a/b/c/file.txt
    let target = dir.path().join("a/b/c/file.txt");
    assert!(!target.parent().unwrap().exists());
    let id = insert_box_row(&conn, &box_dir, "x.tmp", &target);

    restore_from_safe_box(&conn, &box_dir, id).unwrap();

    assert!(target.exists(), "file should be restored");
    assert!(target.parent().unwrap().join("a").is_dir() || target.parent().unwrap().is_dir());
    // 多层目录应都被创建
    assert!(dir.path().join("a").is_dir());
    assert!(dir.path().join("a/b").is_dir());
    assert!(dir.path().join("a/b/c").is_dir());
}

#[test]
fn restore_preserves_existing_parent_dir() {
    let (dir, conn, box_dir) = setup();
    // 父目录已存在，里面放个无关文件
    let parent = dir.path().join("existing");
    std::fs::create_dir_all(&parent).unwrap();
    let sibling = parent.join("keepme.txt");
    std::fs::write(&sibling, b"keep").unwrap();

    let target = parent.join("file.txt");
    let id = insert_box_row(&conn, &box_dir, "y.tmp", &target);
    restore_from_safe_box(&conn, &box_dir, id).unwrap();

    assert!(target.exists());
    assert!(sibling.exists(), "sibling file must be preserved");
    assert_eq!(std::fs::read(&sibling).unwrap(), b"keep");
}

#[test]
fn restore_fails_when_mkdir_impossible() {
    let (dir, conn, box_dir) = setup();
    // 把父路径指到一个已存在的「文件」而非目录，create_dir_all 必失败。
    let fake_parent = dir.path().join("afile");
    std::fs::write(&fake_parent, b"not a dir").unwrap();
    let target = fake_parent.join("file.txt");
    let id = insert_box_row(&conn, &box_dir, "z.tmp", &target);

    let err = restore_from_safe_box(&conn, &box_dir, id).unwrap_err();
    let msg = format!("{err}");
    // 父路径不可用（父组件是文件）→ restore 必然失败（mkdir 或 rename 阶段）。
    assert!(
        msg.contains("无法创建父目录") || msg.contains("rename") || msg.contains("Io"),
        "expected mkdir/rename error, got: {msg}"
    );
}
