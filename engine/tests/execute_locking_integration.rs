//! task-021 集成测试：FILE_IN_USE 时 execute_items 自动调 who_is_locking。

use engine::db::conn::open;
use engine::execute::delete::Deleter;
use engine::execute::executor::execute_items;
use engine::execute::fs_deleter::FsDeleter;

/// 造临时库 + scan_session，返回 (tmpdir, conn, session_id)。
fn setup() -> (tempfile::TempDir, rusqlite::Connection, i64) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (1000, 'running', 'full')",
        [],
    )
    .unwrap();
    let sid = conn.last_insert_rowid();
    (dir, conn, sid)
}

fn insert_item(conn: &rusqlite::Connection, sid: i64, path: &str) -> i64 {
    conn.execute(
        "INSERT INTO cleanup_item
         (session_id, module_code, category, path, size_bytes, risk_level, description)
         VALUES (?1, 'A', 'test', ?2, 100, 'L1', 'desc')",
        rusqlite::params![sid, path],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// 模拟 deleter：直接返回给定错误字符串。
struct MockDeleter {
    err: Option<String>,
}
impl Deleter for MockDeleter {
    fn delete_file(&self, _p: &engine::execute::normalize::NormalizedPath) -> Result<(), String> {
        match &self.err {
            None => Ok(()),
            Some(e) => Err(e.clone()),
        }
    }
    fn delete_dir(&self, _p: &engine::execute::normalize::NormalizedPath) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn execute_items_unlocked_file_no_locking_processes() {
    let (dir, conn, sid) = setup();
    let p = dir.path().join("plain.tmp");
    std::fs::write(&p, b"hi").unwrap();
    let id = insert_item(&conn, sid, p.to_str().unwrap());

    let summary = execute_items(&[id], &conn, &[], |_id| Box::new(FsDeleter));
    assert_eq!(summary.deleted_count, 1);
    assert!(summary.results[0].locking_processes.is_empty());
}

#[test]
fn execute_items_permission_denied_skips_who_is_locking() {
    let (dir, conn, sid) = setup();
    let p = dir.path().join("perm.tmp");
    std::fs::write(&p, b"hi").unwrap();
    let id = insert_item(&conn, sid, p.to_str().unwrap());

    // Mock deleter 抛 PERMISSION_DENIED 前缀错误——不应触发 who_is_locking。
    let summary = execute_items(&[id], &conn, &[], |_id| {
        Box::new(MockDeleter {
            err: Some("PERMISSION_DENIED: mock".into()),
        })
    });
    assert_eq!(summary.failed_count, 1);
    assert!(summary.results[0].locking_processes.is_empty());
    // audit_log detail 走 task-019 文案
    let detail: String = conn
        .query_row(
            "SELECT detail FROM audit_log WHERE result='fail' ORDER BY ts DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        detail.contains("permission denied, needs elevation"),
        "detail={detail}"
    );
}

#[test]
#[cfg(windows)]
#[ignore = "DevAgent 手动跑：notepad 端到端验证（GUI 不稳定，CI 跳过）"]
fn execute_items_locked_file_reports_processes() {
    let (dir, conn, sid) = setup();
    let p = dir.path().join("locked.txt");
    std::fs::write(&p, b"hi").unwrap();
    let id = insert_item(&conn, sid, p.to_str().unwrap());

    let mut child = std::process::Command::new("notepad")
        .arg(&p)
        .spawn()
        .expect("spawn notepad");
    std::thread::sleep(std::time::Duration::from_secs(2));

    let summary = execute_items(&[id], &conn, &[], |_id| Box::new(FsDeleter));
    println!("summary: {summary:?}");
    assert_eq!(summary.failed_count, 1);
    assert!(!summary.results[0].locking_processes.is_empty());
    assert!(summary.results[0]
        .locking_processes
        .iter()
        .any(|lp| lp.app_name.to_lowercase().contains("notepad")));

    let detail: String = conn
        .query_row(
            "SELECT detail FROM audit_log WHERE result='fail' ORDER BY ts DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(detail.contains("file in use by:"), "detail={detail}");

    let _ = child.kill();
    let _ = child.wait();
}
