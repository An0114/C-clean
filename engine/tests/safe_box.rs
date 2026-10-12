//! task-016 集成测试：SafeBoxDeleter 移入保险箱 + quota FIFO 剔除。

use std::sync::Mutex;

use engine::db::conn::open;
use engine::execute::delete::Deleter;
use engine::execute::normalize::normalize;
use engine::execute::safe_box::{set_quota_for_test, SafeBoxDeleter};

/// 全局串行锁：QUOTA_BYTES 是进程级 AtomicU64，并行测试会互相覆盖 quota 值。
static TEST_LOCK: Mutex<()> = Mutex::new(());

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

#[test]
fn safe_box_moves_file_and_writes_row() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (dir, conn, box_dir) = setup();
    let src = dir.path().join("a.tmp");
    std::fs::write(&src, vec![0u8; 100]).unwrap();

    let deleter = SafeBoxDeleter::new(&box_dir, &conn, None);
    let np = normalize(src.to_str().unwrap()).unwrap();
    deleter.delete_file(&np).unwrap();

    assert!(!src.exists());
    assert!(box_dir.exists());
    let moved: Vec<_> = std::fs::read_dir(&box_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(moved.len(), 1);

    let (n, size): (i64, i64) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(size_bytes),0) FROM safe_box WHERE status='in_box'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(n, 1);
    assert_eq!(size, 100);
}

#[test]
fn safe_box_enforce_quota_noop_when_under_quota() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (_dir, conn, box_dir) = setup();
    set_quota_for_test(1024 * 1024); // 1 MB，测试下不会触发
    let deleter = SafeBoxDeleter::new(&box_dir, &conn, None);
    let usage = deleter.enforce_quota(100).unwrap();
    assert_eq!(usage, 0);
}

#[test]
fn safe_box_quota_evicts_oldest() {
    let _guard = TEST_LOCK.lock().unwrap();
    let (dir, conn, box_dir) = setup();
    set_quota_for_test(300); // 4 个 100 字节文件，第 4 个触发剔除

    // 手动塞 3 行 in_box，不同 quarantined_at
    std::fs::create_dir_all(&box_dir).unwrap();
    for (i, ts) in [1000i64, 2000, 3000].iter().enumerate() {
        let rel = format!("old_{}.bin", i);
        std::fs::write(box_dir.join(&rel), vec![0u8; 100]).unwrap();
        conn.execute(
            "INSERT INTO safe_box
             (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status)
             VALUES (?1, ?2, 100, ?3, ?3+86400, 'in_box')",
            rusqlite::params![format!("C:\\orig\\old_{i}.bin"), rel, ts],
        )
        .unwrap();
    }

    // 第 4 个 100 字节文件进来
    let src = dir.path().join("new.tmp");
    std::fs::write(&src, vec![0u8; 100]).unwrap();
    let deleter = SafeBoxDeleter::new(&box_dir, &conn, None);
    let np = normalize(src.to_str().unwrap()).unwrap();
    deleter.delete_file(&np).unwrap();

    // 最早的 old_0.bin 应被剔除（box_dir 里没了，DB 行 status='purged'）
    assert!(!box_dir.join("old_0.bin").exists());
    let status: String = conn
        .query_row(
            "SELECT status FROM safe_box WHERE safe_rel_path='old_0.bin'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "purged");

    // 现在 in_box 应有 3 行：old_1, old_2, new
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM safe_box WHERE status='in_box'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 3);
}
