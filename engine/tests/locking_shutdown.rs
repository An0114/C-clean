//! task-049 / task-049b 集成测试：`shutdown_locking_processes`（Restart Manager 请求关闭）。
//!
//! - `shutdown_no_lockers_is_idempotent`：CI 必跑，无占用 → 空结果且重复调用不报错。
//! - `shutdown_nonexistent_path_does_not_panic`：CI 必跑，坏路径不 panic。
//! - `shutdown_never_closes_self`：CI 必跑，本进程持句柄时绝不把自己放进 `closed`。
//! - `shutdown_returns_empty_on_non_windows`：非 Windows 编译期分支。
//! - `shutdown_locked_file_closes_holder`：`#[ignore]`，真机手动 `--ignored` 跑。
//! - `execute_retry_after_shutdown_deletes_locked_file`：`#[ignore]`，端到端。
//! - `shutdown_only_touches_registered_path`：`#[ignore]`，B7（旁路文件不误伤）。
//! - `shutdown_refuses_protected_image_holder`：`#[ignore]`，task-049b 护栏映像名判定。
//! - `holder_child_holds_file`：`#[ignore]`，被上一条复制成 `explorer.exe` 拉起当 holder。
//!
//! 护栏纯函数 `is_protected_process` / `decide` 是 `pub(crate)`，跨 crate 不可见，
//! 其单测放在 `engine/src/execute/locking.rs` 的 `#[cfg(test)] mod tests`。

use engine::execute::locking::{shutdown_locking_processes, who_is_locking};

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
    let first = execute_items(&[item_id], &conn, &[], |_id, _task_id| {
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
    let second = execute_items(&[item_id], &conn, &[], |_id, _task_id| {
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

/// 一个「外部进程独占持有文件」的 holder，带就绪/自然退出两个标记文件。
///
/// 为什么不再用「盲等 5 秒」：评审侧观察到 B7 用例偶发失败（`b after=[]`），
/// 但无法区分「holder 自己退出」和「被 RmShutdown 波及」。用标记文件把两种
/// 情况分开：
/// - `.ready`：脚本已拿到独占句柄（等它出现，而不是 sleep 固定时长）；
/// - `.done`：脚本正常走完（`Start-Sleep` 到点后写），被强杀时**不会**存在。
#[cfg(windows)]
struct Holder {
    child: std::process::Child,
    ready: std::path::PathBuf,
    done: std::path::PathBuf,
}

#[cfg(windows)]
impl Holder {
    /// PowerShell 版 holder：独占打开 → 写 ready → 睡 180s → 写 done → 关闭。
    ///
    /// 180s 远大于单条用例的耗时（RmShutdown 最坏 ~30s），因此「用例还没跑完
    /// holder 就自然退出」不可能发生，`.done` 也就成了「自然退出」的判据。
    ///
    /// `CREATE_NO_WINDOW`（0x0800_0000）给每个 holder 一个**独立**的隐藏控制台：
    /// 否则两个 holder 与测试进程共用同一个 console，`RmShutdown` 强杀 A 时
    /// 会连带把共用 console 的 B 一起带走（实测：单跑通过、全量连跑必失败，
    /// `holder_b_exit=1` 且没有 done 标记）。真实场景里两个占用方本来就是彼此
    /// 独立的进程，独立控制台更贴近事实。
    fn spawn_powershell(file: &str, tag: &str) -> Self {
        use std::os::windows::process::CommandExt;

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        let dir = std::path::Path::new(file)
            .parent()
            .expect("holder file has parent")
            .to_path_buf();
        let ready = dir.join(format!("holder_{tag}.ready"));
        let done = dir.join(format!("holder_{tag}.done"));
        let script = format!(
            "$f=[System.IO.File]::Open('{file}','Open','ReadWrite','None'); \
             Set-Content -Path '{ready}' -Value $PID; \
             Start-Sleep -Seconds 180; \
             Set-Content -Path '{done}' -Value 'done'; \
             $f.Close()",
            file = file.replace('\\', "\\\\"),
            ready = ready.to_string_lossy().replace('\\', "\\\\"),
            done = done.to_string_lossy().replace('\\', "\\\\"),
        );
        let child = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .expect("spawn powershell holder");
        Self { child, ready, done }
    }

    /// 等 holder 真的拿到句柄（最多 20s；PowerShell 冷启动在 CI 上可能 3-5s）。
    fn wait_ready(&self) -> bool {
        for _ in 0..200 {
            if self.ready.exists() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        false
    }

    /// (是否存活, 退出码)。
    fn poll(&mut self) -> (bool, Option<i32>) {
        match self.child.try_wait() {
            Ok(Some(status)) => (false, status.code()),
            _ => (true, None),
        }
    }

    /// 自然退出标记：只有脚本跑到 `$f.Close()` 之前才写。
    fn exited_naturally(&self) -> bool {
        self.done.exists()
    }

    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// task-049 B7：只关闭「本次注册的资源」的占用方，不误伤其他文件。
///
/// 两个文件各由一个外部进程独占持有：只对 A 调 shutdown → A 的占用必须消失，
/// B 的占用必须原样保留（B 依然锁着、文件依然删不掉）。
///
/// task-049b 起失败必须可归因（评审 Bug #3）：失败信息里带 `b_before` / `b_after` /
/// `holder_b_alive` / `holder_b_exit` / `holder_b_done_marker`，能区分
/// 「holder 自然退出（环境抖动）」与「RmShutdown 真的波及了旁路进程」。
#[test]
#[cfg(windows)]
#[ignore = "真机冒烟：需要两个真实外部占用进程，RmShutdown 最坏等 30s，CI 跳过"]
fn shutdown_only_touches_registered_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let a = tmp.path().join("target_a_049.txt");
    let b = tmp.path().join("bystander_b_049.txt");
    std::fs::write(&a, b"aaa").expect("write a");
    std::fs::write(&b, b"bbb").expect("write b");
    let path_a = a.to_string_lossy().to_string();
    let path_b = b.to_string_lossy().to_string();

    let mut holder_a = Holder::spawn_powershell(&path_a, "a");
    let mut holder_b = Holder::spawn_powershell(&path_b, "b");
    assert!(
        holder_a.wait_ready(),
        "holder A 未在 20s 内就绪（ready 标记没出现）"
    );
    assert!(
        holder_b.wait_ready(),
        "holder B 未在 20s 内就绪（ready 标记没出现）"
    );

    // 前置：两个文件都确实被 RM 枚举为「有占用」（ready 只证明句柄已拿到）。
    let a_before = who_is_locking(&path_a).expect("who a");
    let b_before = who_is_locking(&path_b).expect("who b");
    assert!(
        !a_before.is_empty(),
        "file A should be locked before shutdown: {a_before:?}"
    );
    assert!(
        !b_before.is_empty(),
        "file B should be locked before shutdown: {b_before:?}"
    );

    // 只对 A 请求关闭。
    let outcome = shutdown_locking_processes(&path_a).expect("shutdown a ok");
    println!("shutdown a: {outcome:?}");
    assert!(!outcome.closed.is_empty(), "closed={outcome:?}");

    // A 的占用应消失（≤10s 轮询）。
    let mut a_gone = false;
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if who_is_locking(&path_a)
            .map(|v| v.is_empty())
            .unwrap_or(false)
        {
            a_gone = true;
            break;
        }
    }
    assert!(a_gone, "A still locked after shutdown");

    // B 必须毫发无损：占用还在，且仍处于「删不掉」状态。
    let b_after = who_is_locking(&path_b).expect("who b after");
    let (holder_b_alive, holder_b_exit) = holder_b.poll();
    let holder_b_done = holder_b.exited_naturally();
    println!(
        "b_before={b_before:?} b_after={b_after:?} holder_b_alive={holder_b_alive} \
         holder_b_exit={holder_b_exit:?} holder_b_done_marker={holder_b_done}"
    );

    // 归因（评审 Bug #3 要求）：holder 是 180s 长睡眠，用例总耗时远小于 180s，
    // 所以「没写 done 就退出」= 被外部终止（RM 波及旁路进程）——这是真缺陷，
    // 不能再用 inconclusive 糊过去。
    if !holder_b_alive && !holder_b_done {
        holder_a.kill();
        panic!(
            "RmShutdown 波及了未注册路径的旁路进程：holder_b_exit={holder_b_exit:?} \
             b_before={b_before:?} b_after={b_after:?}（holder 未写 done 标记即退出）"
        );
    }
    assert!(
        holder_b_alive,
        "inconclusive: holder B 在用例结束前自行退出（done 标记={holder_b_done}，\
         exit={holder_b_exit:?}），环境抖动，请重跑"
    );
    assert_eq!(
        b_after.len(),
        b_before.len(),
        "shutdown on A must not touch B's lockers: b_before={b_before:?} b_after={b_after:?}"
    );
    assert!(
        std::fs::remove_file(&b).is_err(),
        "B must still be undeletable (its holder is alive)"
    );

    holder_a.kill();
    holder_b.kill();
}

/// task-049b（评审 Bug #1）：护栏按 **PID → 映像文件名** 判定，不受系统语言影响。
///
/// 构造方式：把本测试二进制复制成 `explorer.exe`，再用 libtest 的 `--exact`
/// 拉起其中的 `holder_child_holds_file` 子用例去独占持有文件。于是 RM 看到的是
/// 一个**映像名 = explorer.exe** 的真实外部进程——这正好复刻了评审 Bug #1 的
/// 场景（`zh-CN` 下 RM 报的是 `Windows 资源管理器`，旧实现三条名字规则全不命中），
/// 且不需要 GUI、不依赖真 explorer 恰好持有文件，任意语言环境都能跑。
#[test]
#[cfg(windows)]
#[ignore = "真机冒烟：需要外部进程真实占用，CI 跳过"]
fn shutdown_refuses_protected_image_holder() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let target = tmp.path().join("held_by_fake_explorer_049b.txt");
    std::fs::write(&target, b"x").expect("write");
    let path = target.to_string_lossy().to_string();

    let fake_explorer = tmp.path().join("explorer.exe");
    std::fs::copy(
        std::env::current_exe().expect("current_exe"),
        &fake_explorer,
    )
    .expect("copy test exe as explorer.exe");

    let ready = tmp.path().join("fake_explorer.ready");
    let mut child = std::process::Command::new(&fake_explorer)
        .args([
            "--exact",
            "holder_child_holds_file",
            "--ignored",
            "--nocapture",
        ])
        .env("HOLDER_FILE", &path)
        .env("HOLDER_READY", &ready)
        .spawn()
        .expect("spawn fake explorer holder");

    // 等它真的持有文件（ready 标记 + RM 枚举双重确认，不盲等）。
    let mut locked = false;
    for _ in 0..200 {
        if ready.exists()
            && who_is_locking(&path)
                .map(|v| !v.is_empty())
                .unwrap_or(false)
        {
            locked = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    if !locked {
        let _ = child.kill();
        let _ = child.wait();
        panic!(
            "fake explorer 未能在 20s 内持有文件（ready={}）",
            ready.exists()
        );
    }

    let outcome = shutdown_locking_processes(&path).expect("shutdown ok");
    println!("fake explorer outcome: {outcome:?}");

    // 红线：explorer.exe 命中护栏 → 一个都不关，且必须出现在 refused 里。
    assert!(
        outcome.closed.iter().all(|lp| lp.pid != child.id()),
        "explorer.exe 绝不能进 closed: {outcome:?}"
    );
    assert!(
        outcome.closed.is_empty(),
        "存在受保护进程时不应关闭任何进程（mixed fail-closed）: {outcome:?}"
    );
    let refused = outcome.refused.iter().find(|r| r.pid == child.id());
    assert_eq!(
        refused.map(|r| r.reason.as_str()),
        Some("protected process"),
        "refused 应含 protected process: {outcome:?}"
    );
    // 护栏拒绝 → 不调 RmShutdown → holder 必须还活着。
    assert!(
        child.try_wait().expect("try_wait").is_none(),
        "受保护进程不应被关闭"
    );

    let _ = child.kill();
    let _ = child.wait();
}

/// holder 子用例：只被 [`shutdown_refuses_protected_image_holder`] 以「复制成
/// `explorer.exe` 的测试二进制」形式拉起，用来扮演持有文件的外部进程。
/// 常规跑测试时永不执行（`#[ignore]` + 需要环境变量）。
#[test]
#[cfg(windows)]
#[ignore = "holder 子进程专用，不单独跑"]
fn holder_child_holds_file() {
    // 被 `--include-ignored` 全量跑时会进来，但没有环境变量就静默跳过，
    // 避免污染连跑证据（真正的 holder 一定由上面那条用例带 env 拉起）。
    let (Ok(file), Ok(ready)) = (std::env::var("HOLDER_FILE"), std::env::var("HOLDER_READY"))
    else {
        println!("holder_child_holds_file: 无 HOLDER_FILE/HOLDER_READY，跳过（非 holder 子进程）");
        return;
    };
    // Rust 的 File::open 不带 FILE_SHARE_DELETE → 天然制造「删不掉」的占用。
    let handle = std::fs::File::open(&file).expect("open holder file");
    std::fs::write(&ready, b"ready").expect("write ready marker");
    std::thread::sleep(std::time::Duration::from_secs(120));
    drop(handle);
}
