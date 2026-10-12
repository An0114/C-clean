//! task-013 集成测试：`ScanOrchestrator` 串跑 5 个脚本。
//!
//! 用 tempdir 当 scripts_dir，写假 ps1（与 runner 测试同模式）。
//! 真 5 个脚本的文件名由 SCRIPTS 常量决定，测试只造其中几个来覆盖
//! 「成功 / 缺文件 / 全失败 / 写库行数」四条路径。

use engine::db::conn::open;
use engine::ipc::progress::NoopEmitter;
use engine::scan::{OrchestratorError, ScanOrchestrator};

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

#[cfg(windows)]
#[test]
fn orchestrator_runs_all_scripts() {
    // 注意：SCRIPTS 常量硬编码 5 个文件名；本测试只造其中 3 个合法脚本，
    // 另外 2 个故意缺失以同时覆盖「缺文件」分支（在另一个用例验证 Ok 语义）。
    let (dir, conn, sid) = setup();
    // 造 3 个合法脚本。
    for name in [
        "scan_temp.ps1",
        "scan_windows_update.ps1",
        "scan_dev_cache.ps1",
    ] {
        let p = dir.path().join(name);
        std::fs::write(
            &p,
            r#"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
[Console]::Out.WriteLine('{"type":"item","module_code":"x","category":"c","path":"C:\\t.tmp","size_bytes":10,"risk_level":1}')
[Console]::Out.WriteLine('{"type":"done","module_code":"x","total_items":1,"total_bytes":10}')
"#,
        )
        .unwrap();
    }
    // 另外 2 个缺失：scan_prefetch.ps1 / scan_browser_cache.ps1。
    let orch = ScanOrchestrator::new(sid, &conn, &NoopEmitter);
    let s = orch.run_all(dir.path()).unwrap();
    assert_eq!(s.scripts_succeeded.len(), 3);
    assert_eq!(s.scripts_failed.len(), 2);
    assert_eq!(s.total_items, 3);
}

#[cfg(windows)]
#[test]
fn orchestrator_skips_missing_scripts_returns_ok() {
    let (dir, conn, sid) = setup();
    // 只造 2 个合法脚本。
    for name in ["scan_temp.ps1", "scan_dev_cache.ps1"] {
        let p = dir.path().join(name);
        std::fs::write(
            &p,
            r#"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
[Console]::Out.WriteLine('{"type":"done","module_code":"x","total_items":0,"total_bytes":0}')
"#,
        )
        .unwrap();
    }
    let orch = ScanOrchestrator::new(sid, &conn, &NoopEmitter);
    let s = orch.run_all(dir.path()).unwrap();
    assert_eq!(s.scripts_succeeded.len(), 2);
    assert_eq!(s.scripts_failed.len(), 3);
}

#[cfg(windows)]
#[test]
fn orchestrator_all_failed_returns_err() {
    let (dir, conn, sid) = setup();
    // tempdir 里不放任何 ps1。
    let orch = ScanOrchestrator::new(sid, &conn, &NoopEmitter);
    let err = orch.run_all(dir.path()).unwrap_err();
    match err {
        OrchestratorError::AllScriptsFailed(n) => assert_eq!(n, 5),
    }
}

#[cfg(windows)]
#[test]
fn orchestrator_writes_cleanup_item_rows() {
    let (dir, conn, sid) = setup();
    let p = dir.path().join("scan_temp.ps1");
    std::fs::write(
        &p,
        r#"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
[Console]::Out.WriteLine('{"type":"item","module_code":"m","category":"c1","path":"C:\\a.tmp","size_bytes":100,"risk_level":1}')
[Console]::Out.WriteLine('{"type":"item","module_code":"m","category":"c2","path":"C:\\b.tmp","size_bytes":200,"risk_level":2}')
[Console]::Out.WriteLine('{"type":"done","module_code":"m","total_items":2,"total_bytes":300}')
"#,
    )
    .unwrap();
    let orch = ScanOrchestrator::new(sid, &conn, &NoopEmitter);
    let s = orch.run_all(dir.path()).unwrap();
    assert_eq!(s.total_items, 2);
    let n: i64 = conn
        .query_row(
            "SELECT count(*) FROM cleanup_item WHERE session_id=?1",
            [sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 2);
}
