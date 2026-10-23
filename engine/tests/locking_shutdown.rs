//! task-049 集成测试：`shutdown_locking_processes`（Restart Manager 请求关闭）。
//!
//! - `shutdown_no_lockers_is_idempotent`：CI 必跑，无占用 → 空结果且重复调用不报错。
//! - `shutdown_nonexistent_path_does_not_panic`：CI 必跑，坏路径不 panic。
//! - `shutdown_never_closes_self`：CI 必跑，本进程持句柄时绝不把自己放进 `closed`。
//! - `shutdown_returns_empty_on_non_windows`：非 Windows 编译期分支。
//! - `shutdown_locked_file_closes_holder`：`#[ignore]`，真机手动 `--ignored` 跑。
//!
//! 护栏纯函数 `is_protected_process` 是 `pub(crate)`，跨 crate 不可见，
//! 其单测放在 `engine/src/execute/locking.rs` 的 `#[cfg(test)] mod tests`。

use engine::execute::locking::shutdown_locking_processes;

#[test]
fn shutdown_no_lockers_is_idempotent() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let p = tmp.path().join("no_locker_049.txt");
    std::fs::write(&p, b"hello").expect("write");
    let path = p.to_string_lossy().to_string();

    // 第一次：没有任何进程打开它 → 不该调 RmShutdown，返回空结构。
    let first = shutdown_locking_processes(&path).expect("first call ok");
    assert!(first.closed.is_empty(), "closed={:?}", first.closed);
    assert!(first.refused.is_empty(), "refused={:?}", first.refused);

    // 第二次：幂等，重复点「重试」不应该报 RM 错误。
    let second = shutdown_locking_processes(&path).expect("second call ok");
    assert!(second.closed.is_empty());
    assert!(second.refused.is_empty());

    // 文件本身没被动过。
    assert!(p.exists(), "shutdown must not touch the file");
}

#[test]
fn shutdown_nonexistent_path_does_not_panic() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let missing = tmp.path().join("does_not_exist_049.txt");
    let path = missing.to_string_lossy().to_string();

    // 不存在路径：RM 可能注册失败（Err）也可能注册成功但零占用（Ok 空）。
    // 这里只要求「不 panic、不误报已关闭进程」。
    match shutdown_locking_processes(&path) {
        Ok(outcome) => {
            assert!(outcome.closed.is_empty(), "closed={:?}", outcome.closed);
            assert!(outcome.refused.is_empty(), "refused={:?}", outcome.refused);
        }
        Err(e) => {
            // 允许失败，但错误必须来自 RM 调用链，便于排查。
            assert!(
                e.contains("RmRegisterResources") || e.contains("RmGetList"),
                "unexpected error: {e}"
            );
        }
    }
}

#[test]
#[cfg(windows)]
fn shutdown_never_closes_self() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let p = tmp.path().join("held_by_self_049.txt");
    std::fs::write(&p, b"hi").expect("write");

    // 本进程自己持有句柄（默认共享模式不含 FILE_SHARE_DELETE）→ 制造真实占用。
    let held = std::fs::File::open(&p).expect("open");
    let path = p.to_string_lossy().to_string();
    let self_pid = std::process::id();

    let outcome = shutdown_locking_processes(&path).expect("shutdown ok");
    println!("self outcome: {outcome:?}");

    // 红线：无论 RM 是否枚举到本进程，都绝不能把自己放进 closed（不自杀）。
    assert!(
        outcome.closed.iter().all(|lp| lp.pid != self_pid),
        "must never close self: {outcome:?}"
    );
    // 若枚举到本进程，必须归入 refused 且原因稳定为 "self"。
    for r in &outcome.refused {
        assert_eq!(r.reason, "self", "unexpected refused reason: {r:?}");
    }

    drop(held);
}

#[test]
#[cfg(not(windows))]
fn shutdown_returns_empty_on_non_windows() {
    // 非 Windows：固定返回空结构，保证跨平台编译与调用方无需分支。
    let outcome = shutdown_locking_processes(r"C:\whatever\none.txt").expect("ok");
    assert!(outcome.closed.is_empty());
    assert!(outcome.refused.is_empty());
}

#[test]
#[cfg(windows)]
#[ignore = "DevAgent 手动跑：真实占用端到端验证（需真机 GUI 环境，CI 跳过）"]
fn shutdown_locked_file_closes_holder() {
    // 注意（与任务书偏差）：Windows 11 新版记事本是打包应用，打开文件后**不长期
    // 持有句柄**（实测 RmGetList 返回 0），无法用来造占用。这里改用 PowerShell
    // 以独占方式（FileShare.None）持有句柄，行为更贴近真实占用场景。
    let tmp = tempfile::tempdir().expect("tempdir");
    let p = tmp.path().join("locked_049.txt");
    std::fs::write(&p, b"hi").expect("write");
    let path = p.to_string_lossy().to_string();

    let script = format!(
        "$f=[System.IO.File]::Open('{}','Open','ReadWrite','None'); Start-Sleep -Seconds 60; $f.Close()",
        path.replace('\\', "\\\\")
    );
    let mut child = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .spawn()
        .expect("spawn powershell holder");
    std::thread::sleep(std::time::Duration::from_secs(4));

    let t0 = std::time::Instant::now();
    let outcome = shutdown_locking_processes(&path).expect("shutdown ok");
    println!("shutdown took {:?}, outcome: {outcome:?}", t0.elapsed());
    assert!(
        !outcome.closed.is_empty(),
        "expected the holder in closed, got {outcome:?}"
    );
    assert!(outcome.refused.is_empty(), "refused={:?}", outcome.refused);
    // RmShutdown 返回成功 ≠ 进程已退出；轮询确认占用真的消失（≤5s）。
    let mut gone = false;
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if engine::execute::locking::who_is_locking(&path)
            .map(|v| v.is_empty())
            .unwrap_or(false)
        {
            gone = true;
            break;
        }
    }
    assert!(gone, "holder still locking after shutdown");

    let _ = child.kill();
    let _ = child.wait();
}

/// task-049 端到端：占用 → 首次删除失败并报出占用进程 → 请求关闭 → 重试删除成功。
///
/// 这条用例走的是 `cleanup_retry_locked` 命令内部的**同一段编排**
/// （`execute_items` + `SafeBoxDeleter`），因此它的通过可以替代 GUI 点击，
/// 作为「关闭后重试真的能删掉文件」的后端证据。
#[test]
#[cfg(windows)]
#[ignore = "真机冒烟：需要真实外部进程占用，RmShutdown 最坏等 30s，CI 跳过"]
fn execute_retry_after_shutdown_deletes_locked_file() {
    use engine::db::conn::open;
    use engine::execute::executor::execute_items;
    use engine::execute::safe_box::SafeBoxDeleter;

    let tmp = tempfile::tempdir().expect("tempdir");
    let conn = open(tmp.path()).expect("open db");
    let box_dir = tmp.path().join("safe_box");
    std::fs::create_dir_all(&box_dir).expect("mkdir safe_box");

    let target = tmp.path().join("locked_e2e_049.txt");
    std::fs::write(&target, b"payload").expect("write");
    let path = target.to_string_lossy().to_string();

    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (1000, 'completed', 'full')",
        [],
    )
    .expect("insert session");
    let sid = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO cleanup_item
         (session_id, module_code, category, path, size_bytes, risk_level, description)
         VALUES (?1, 'A', 'smoke049', ?2, 7, 'L1', 'task-049 e2e')",
        rusqlite::params![sid, path],
    )
    .expect("insert item");
    let item_id = conn.last_insert_rowid();

    // 外部进程独占持有句柄。
    let script = format!(
        "$f=[System.IO.File]::Open('{}','Open','ReadWrite','None'); Start-Sleep -Seconds 90; $f.Close()",
        path.replace('\\', "\\\\")
    );
    let mut holder = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .spawn()
        .expect("spawn holder");
    std::thread::sleep(std::time::Duration::from_secs(4));

    // 第一次删除：必然失败，且要报出占用进程。
    let first = execute_items(&[item_id], &conn, &[], |_id| {
        Box::new(SafeBoxDeleter::new(&box_dir, &conn, None))
    });
    println!("first: {first:?}");
    assert_eq!(first.deleted_count, 0);
    assert!(
        !first.results[0].locking_processes.is_empty(),
        "expected locking processes in first attempt: {first:?}"
    );

    // 请求关闭占用进程。
    let outcome = shutdown_locking_processes(&path).expect("shutdown ok");
    println!("shutdown: {outcome:?}");
    assert!(!outcome.closed.is_empty(), "closed={outcome:?}");

    // 重试删除（RmShutdown 返回后句柄可能还没释放，给一点收尾时间）。
    std::thread::sleep(std::time::Duration::from_secs(2));
    let second = execute_items(&[item_id], &conn, &[], |_id| {
        Box::new(SafeBoxDeleter::new(&box_dir, &conn, None))
    });
    println!("second: {second:?}");
    assert_eq!(second.deleted_count, 1, "retry should delete: {second:?}");
    assert!(!target.exists(), "file should be gone");

    // 真实 JSON shape（实现备注要贴实际输出，不贴设计稿）。
    let resp = engine::execute::locking::RetryLockedResp {
        shutdown: outcome,
        summary: second,
    };
    println!(
        "retry_locked_resp_json: {}",
        serde_json::to_string(&resp).expect("serialize")
    );

    let _ = holder.kill();
    let _ = holder.wait();
}
