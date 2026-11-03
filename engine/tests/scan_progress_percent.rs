//! task-052 G1 集成测试：`ScanOrchestrator` 脚本级百分比换算。
//!
//! 覆盖任务书第 18 条 G1 要求：
//! - 跑完 N 个脚本后 percent **单调递增**；
//! - 任意时刻 percent ∈ [0, 100]；
//! - 全部脚本跑完时 percent 精确到达 100。
//!
//! 用 `VecEmitter` 直接收集所有 emit（不经 100ms 节流，避免测试被节流吞事件）。

#![cfg(windows)]

use engine::db::conn::open;
use engine::ipc::progress::VecEmitter;
use engine::scan::ScanOrchestrator;

/// 造临时库 + 一条 scan_session，返回 (tmpdir, conn, session_id)。
fn setup() -> (tempfile::TempDir, rusqlite::Connection, i64) {
    // task-074：本文件测的是 **ps1 脚本的进度百分比换算**（造假脚本 → 解析
    // NDJSON → 按脚本序号切槽位）。temp 模块从 task-074 起默认走 Rust 实现
    // （不读脚本、不产生 ps1 的 done 事件），不关掉的话：
    //   - `all_failed_does_not_emit_100` 会因为 temp 走 Rust 成功而失效
    //     （它期望 5 个全失败，实际 4 失败 1 成功 → 会 emit 100）；
    //   - 其他用例的槽位边界断言也会因为多出一个成功的「脚本」而偏移。
    //
    // 每个集成测试文件是独立进程，`set_var` 不会污染其他文件。
    // 用**总开关**而不是逐个模块设：task-075 加了 prefetch / browser_cache 后，
    // 只设 CCLEAN_RUST_TEMP 已经不够（那两个模块仍走 Rust，造的假脚本不会
    // 被用到，4 个用例全红）。总开关一次搞定，以后迁新模块也不用再改这里。
    std::env::set_var("CCLEAN_RUST_ALL", "0");
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

/// 写一个假 ps1：吐 `items` 条 item 事件 + 一条 done。
fn write_script(dir: &std::path::Path, name: &str, items: usize) {
    let mut body =
        String::from("[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)\n");
    for i in 0..items {
        body.push_str(&format!(
            "[Console]::Out.WriteLine('{{\"type\":\"item\",\"module_code\":\"m\",\"category\":\"c\",\"path\":\"C:\\\\p{i}.tmp\",\"size_bytes\":10,\"risk_level\":1}}')\n"
        ));
    }
    let total_bytes = items * 10;
    body.push_str(&format!(
        "[Console]::Out.WriteLine('{{\"type\":\"done\",\"module_code\":\"m\",\"total_items\":{items},\"total_bytes\":{total_bytes}}}')\n"
    ));
    std::fs::write(dir.join(name), body).unwrap();
}

/// G1 主用例：5 个脚本各 4 条 item，percent 单调不减、恒在 [0,100]、收尾为 100。
#[test]
fn percent_is_monotonic_and_reaches_100() {
    let (dir, conn, sid) = setup();
    for name in [
        "scan_temp.ps1",
        "scan_windows_update.ps1",
        "scan_prefetch.ps1",
        "scan_browser_cache.ps1",
        "scan_dev_cache.ps1",
    ] {
        write_script(dir.path(), name, 4);
    }

    let emitter = VecEmitter::default();
    let orch = ScanOrchestrator::new(sid, &conn, &emitter);
    let summary = orch.run_all(dir.path()).unwrap();
    assert_eq!(summary.scripts_succeeded.len(), 5);

    let events = emitter.snapshot();
    assert!(!events.is_empty(), "编排器必须至少 emit 一次进度");

    // 1) 全部落在 [0, 100]
    for e in &events {
        assert!(e.percent <= 100, "percent 越界: {}", e.percent);
    }

    // 2) 单调不减
    let mut prev = 0u8;
    for (i, e) in events.iter().enumerate() {
        assert!(
            e.percent >= prev,
            "第 {i} 条进度出现回退：{prev} -> {}（{events:?}）",
            e.percent
        );
        prev = e.percent;
    }

    // 3) 收尾精确到 100
    assert_eq!(
        events.last().unwrap().percent,
        100,
        "全部脚本跑完必须到 100"
    );

    // 4) 每个脚本槽位都打到了上界（5 个脚本 → 20/40/60/80/100 都应出现）
    let percents: Vec<u8> = events.iter().map(|e| e.percent).collect();
    for expected in [20u8, 40, 60, 80, 100] {
        assert!(
            percents.contains(&expected),
            "缺少槽位边界 {expected}%，实际序列 {percents:?}"
        );
    }
}

/// G1 边界：只有 1 个脚本可跑（其余缺失）时，仍要走到 100。
#[test]
fn percent_reaches_100_with_single_script() {
    let (dir, conn, sid) = setup();
    write_script(dir.path(), "scan_temp.ps1", 3);

    let emitter = VecEmitter::default();
    let orch = ScanOrchestrator::new(sid, &conn, &emitter);
    orch.run_all(dir.path()).unwrap();

    let events = emitter.snapshot();
    assert_eq!(events.last().unwrap().percent, 100);
    let mut prev = 0u8;
    for e in &events {
        assert!(e.percent >= prev);
        prev = e.percent;
    }
}

/// G1 负向：5 个脚本全缺失 → 返回 Err，且**不得**把进度推到 100
/// （否则前端会先显示"扫描完成"再弹错误）。
#[test]
fn all_failed_does_not_emit_100() {
    let (dir, conn, sid) = setup();
    let emitter = VecEmitter::default();
    let orch = ScanOrchestrator::new(sid, &conn, &emitter);
    assert!(orch.run_all(dir.path()).is_err());

    let events = emitter.snapshot();
    assert!(
        events.iter().all(|e| e.percent < 100),
        "全失败时不应 emit 100%：{events:?}"
    );
}

/// G1 兼容：不经编排器（默认 percent_span=(0,0)）时单脚本 percent 仍为 0，
/// 保证 runner 的旧行为不被本任务改变。
#[test]
fn runner_without_span_keeps_percent_zero() {
    let (dir, conn, sid) = setup();
    write_script(dir.path(), "scan_temp.ps1", 2);

    let emitter = VecEmitter::default();
    let runner = engine::scan::ScanRunner::new(sid, &conn, &emitter);
    runner
        .run_script(&dir.path().join("scan_temp.ps1"))
        .unwrap();

    let events = emitter.snapshot();
    assert!(!events.is_empty());
    assert!(events.iter().all(|e| e.percent == 0));
}
