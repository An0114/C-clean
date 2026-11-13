//! task-052 G1 集成测试：`ScanOrchestrator` 模块级百分比换算。
//!
//! 覆盖任务书第 18 条 G1 要求：
//! - 跑完 N 个模块后 percent **单调不减**；
//! - 任意时刻 percent ∈ [0, 100]；
//! - 全部模块跑完时 percent 精确到达 100。
//!
//! 编排用例直接运行真实 Rust 模块，并用 `VecEmitter` 收集所有 emit
//! （不经 100ms 节流，避免测试被节流吞事件）。通用 `ScanRunner` 的默认
//! percent 行为仍由独立假脚本用例覆盖。

#![cfg(windows)]

use engine::db::conn::open;
use engine::ipc::progress::VecEmitter;
use engine::scan::ScanOrchestrator;

/// 造临时库 + 一条 scan_session，返回 (tmpdir, conn, session_id)。
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

/// 为通用 ScanRunner 用例写一个假脚本：吐 `items` 条 item 事件 + 一条 done。
fn write_fake_script(dir: &std::path::Path, items: usize) -> std::path::PathBuf {
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
    let path = dir.join("fake_script.ps1");
    std::fs::write(&path, body).unwrap();
    path
}

/// G1 主用例：直接跑 5 个真实 Rust 模块，percent 单调不减、恒在 [0,100]、收尾为 100。
#[test]
fn percent_is_monotonic_and_reaches_100() {
    let (_dir, conn, sid) = setup();
    let empty_scripts_dir = tempfile::tempdir().unwrap();

    let emitter = VecEmitter::default();
    let orch = ScanOrchestrator::new(sid, &conn, &emitter);
    let summary = orch.run_all(empty_scripts_dir.path()).unwrap();
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

/// 通用 ScanRunner 不经编排器（默认 percent_span=(0,0)）时 percent 仍为 0。
/// 这里只验证 runner 协议解析与默认进度行为，因此使用通用假脚本。
#[test]
fn runner_without_span_keeps_percent_zero() {
    let (dir, conn, sid) = setup();
    let script = write_fake_script(dir.path(), 2);

    let emitter = VecEmitter::default();
    let runner = engine::scan::ScanRunner::new(sid, &conn, &emitter);
    runner.run_script(&script).unwrap();

    let events = emitter.snapshot();
    assert!(!events.is_empty());
    assert!(events.iter().all(|e| e.percent == 0));
}
