//! task-020 集成测试：who_is_locking（Restart Manager FFI）。
//!
//! - `who_is_locking_no_lockers_returns_empty`：CI 必跑，空锁返回空 Vec。
//! - `who_is_locking_notepad_locks_file`：`#[ignore]`，DevAgent 手动跑 notepad 端到端。

use engine::execute::locking::who_is_locking;

#[test]
fn who_is_locking_no_lockers_returns_empty() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let p = tmp.path().join("no_locker.txt");
    std::fs::write(&p, b"hello").expect("write");
    let path = p.to_string_lossy().to_string();
    let result = who_is_locking(&path).expect("who_is_locking ok");
    // 没有进程打开它，应为空 Vec。
    assert!(result.is_empty(), "expected empty, got {result:?}");
}

#[test]
#[cfg(windows)]
#[ignore = "DevAgent 手动跑：notepad 端到端验证（GUI 不稳定，CI 跳过）"]
fn who_is_locking_notepad_locks_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let p = tmp.path().join("locked_by_notepad.txt");
    std::fs::write(&p, b"hi").expect("write");
    let path = p.to_string_lossy().to_string();

    let mut child = std::process::Command::new("notepad")
        .arg(&p)
        .spawn()
        .expect("spawn notepad");
    std::thread::sleep(std::time::Duration::from_secs(2));

    let result = who_is_locking(&path).expect("who_is_locking ok");
    println!("notepad result: {result:?}");
    assert!(!result.is_empty(), "expected notepad to lock the file");
    assert!(
        result.iter().any(|lp| lp.app_name.to_lowercase().contains("notepad")),
        "expected an entry whose app_name contains notepad, got {result:?}"
    );

    let _ = child.kill();
    let _ = child.wait();
}
