//! task-053 G7 关键回归：**经过真实 `ThrottledEmitter`** 的收尾事件用例。
//!
//! 为什么必须单独有这个文件：task-052 的 `scan_progress_percent.rs` 用裸
//! `VecEmitter`（不经节流器）断言收尾为 100，必然通过——而生产链路上
//! `ScanOrchestrator` 拿到的 emitter 是 `&ThrottledEmitter<TauriProgressEmitter>`，
//! 收尾事件紧跟最后一条 item 事件（同毫秒级），会被 100ms 丢弃式节流吞掉。
//! **测试替身绕过了生产链路**，于是 G7 真机复验才暴露。
//!
//! 本文件的核心价值：让测试走与生产完全相同的包装层。
//!
//! - G7.1：`emit_force` 穿透节流窗口；
//! - G7.2：编排器收尾走 force，最后到达订阅方的事件 `percent === 100`；
//! - G7.4：同时断言节流仍生效（事件数 ≤ 节流前），不是「全放行」；
//! - 防退化：`Box<dyn ProgressEmitter>` 包装后 `emit_force` 仍能穿透。

#![cfg(windows)]

use engine::db::conn::open;
use engine::ipc::progress::{ProgressEmitter, ScanProgress, ThrottledEmitter, VecEmitter};
use engine::scan::ScanOrchestrator;

const SCRIPTS: [&str; 5] = [
    "scan_temp.ps1",
    "scan_windows_update.ps1",
    "scan_prefetch.ps1",
    "scan_browser_cache.ps1",
    "scan_dev_cache.ps1",
];

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

/// 5 个脚本各 3 条 item。
fn write_all_scripts(dir: &std::path::Path) {
    for name in SCRIPTS {
        write_script(dir, name, 3);
    }
}

fn progress(percent: u8) -> ScanProgress {
    ScanProgress {
        session_id: 1,
        percent,
        current_path: format!("p{percent}"),
        scanned_items: 0,
    }
}

/// **G7 主用例（关键回归）**：真实 `ThrottledEmitter` 包 `VecEmitter` 跑完整扫描，
/// 断言最后到达订阅方的事件是 `percent === 100`，且事件数确实被节流压下来。
#[test]
fn throttled_orchestrator_tail_event_reaches_100() {
    let (dir, conn, sid) = setup();
    write_all_scripts(dir.path());

    // 基线：裸 VecEmitter（不节流），拿到「节流前应有的事件数」。
    let raw = VecEmitter::default();
    ScanOrchestrator::new(sid, &conn, &raw)
        .run_all(dir.path())
        .unwrap();
    let raw_count = raw.snapshot().len();
    assert!(raw_count > 0, "基线必须至少 emit 一次");

    // 被测：与生产同构的 ThrottledEmitter 包装层。
    let throttled_inner = VecEmitter::default();
    let throttled = ThrottledEmitter::new(throttled_inner);
    ScanOrchestrator::new(sid, &conn, &throttled)
        .run_all(dir.path())
        .unwrap();

    let events = throttled.inner().snapshot();
    assert!(!events.is_empty(), "节流后也必须至少有事件到达");

    // ① 关键断言：最后到达的是 100，不是 96/88。
    assert_eq!(
        events.last().unwrap().percent,
        100,
        "收尾事件被节流吞了——最后到达的是 {}，完整序列 {:?}",
        events.last().unwrap().percent,
        events.iter().map(|e| e.percent).collect::<Vec<_>>()
    );

    // ② 节流仍生效：事件数必须少于节流前的全量（不是全放行）。
    assert!(
        events.len() < raw_count,
        "节流未生效：节流后 {} 条 vs 节流前 {} 条",
        events.len(),
        raw_count
    );

    // ③ 单调不减、不越界（节流不应破坏顺序语义）。
    let mut prev = 0u8;
    for e in &events {
        assert!(e.percent >= prev, "节流后出现回退：{prev} -> {}", e.percent);
        assert!(e.percent <= 100);
        prev = e.percent;
    }
}

/// **G7.1 单元级**：窗口内被丢的事件不影响 force；force 一定穿透。
#[test]
fn emit_force_bypasses_throttle_window() {
    let inner = VecEmitter::default();
    let throttled = ThrottledEmitter::new(inner);

    // 第一次 emit 必发（窗口为空）。
    throttled.emit(&progress(10));
    // 紧接着两条落在同一 100ms 窗口内，必被丢。
    throttled.emit(&progress(20));
    throttled.emit(&progress(30));
    // force 不受窗口约束。
    throttled.emit_force(&progress(100));

    let got: Vec<u8> = throttled.inner().snapshot().iter().map(|e| e.percent).collect();
    assert_eq!(
        got,
        vec![10, 100],
        "期望 [10, 100]（中间两条被丢、force 穿透），实际 {got:?}"
    );
}

/// **G7.1 语义**：force 之后窗口被重置，紧随其后的普通 emit 不会再多发一条
/// （否则 100ms 内会出现两次推送，破坏 ≤1 次/100ms 的架构约束）。
#[test]
fn emit_force_resets_throttle_window() {
    let inner = VecEmitter::default();
    let throttled = ThrottledEmitter::new(inner);

    throttled.emit_force(&progress(100));
    // 若窗口没被重置，这条会因为「距上次普通 emit 很久」而立刻发出。
    throttled.emit(&progress(99));

    let got: Vec<u8> = throttled.inner().snapshot().iter().map(|e| e.percent).collect();
    assert_eq!(got, vec![100], "force 后窗口未重置，实际 {got:?}");
}

/// **防退化**：`Box<dyn ProgressEmitter>` 包装后 `emit_force` 必须继续穿透。
///
/// 漏掉 `impl ProgressEmitter for Box<dyn ProgressEmitter>` 里的 `emit_force`
/// 透传，会退化成默认实现 → 调 `emit` → 被内层节流吞掉，方法等于白加。
/// 用 `Arc<VecEmitter>` 共享观测点，才能从 Box 外面看到真实到达序列。
#[test]
fn boxed_emitter_emit_force_still_passes_through() {
    /// 把 `Arc<VecEmitter>` 适配成 `ProgressEmitter`，供 Box 内部持有、外部观测。
    struct ArcEmitter(std::sync::Arc<VecEmitter>);

    impl ProgressEmitter for ArcEmitter {
        fn emit(&self, payload: &ScanProgress) {
            self.0.emit(payload);
        }
    }

    let shared = std::sync::Arc::new(VecEmitter::default());
    let boxed: Box<dyn ProgressEmitter> = Box::new(ThrottledEmitter::new(ArcEmitter(
        std::sync::Arc::clone(&shared),
    )));

    boxed.emit(&progress(10));
    boxed.emit(&progress(20)); // 同一窗口内，应被节流丢掉
    boxed.emit_force(&progress(100));

    let got: Vec<u8> = shared.snapshot().iter().map(|e| e.percent).collect();
    assert_eq!(
        got,
        vec![10, 100],
        "Box 透传丢失：emit_force 退化成 emit 被节流吞了，实际 {got:?}"
    );
}

/// **G7.4 节流仍生效**：高频连续 emit 必须被压到远低于原始条数。
#[test]
fn throttled_emit_still_drops_within_window() {
    let inner = VecEmitter::default();
    let throttled = ThrottledEmitter::new(inner);

    for p in 0..20u8 {
        throttled.emit(&progress(p));
    }

    let got = throttled.inner().snapshot();
    assert_eq!(got.len(), 1, "20 条同窗口事件只应放过第 1 条，实际 {}", got.len());
    assert_eq!(got[0].percent, 0);
}
