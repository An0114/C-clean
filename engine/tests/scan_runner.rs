//! task-012 集成测试：ScanRunner 端到端（用假 ps1 脚本喂 NDJSON）。
//!
//! - parser 单测已在 `engine/src/scan/parser.rs` 内联；
//! - 这里只覆盖 runner：spawn powershell、逐行解析、写 DB、emit；
//! - ps1 集成用例在 Linux/macOS 下跳过（无 powershell.exe）。

use engine::db::conn::open;
use engine::ipc::progress::{NoopEmitter, VecEmitter};
use engine::scan::ScanRunner;

/// 造临时库 + 一条 scan_session，返回 (conn, session_id)。
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

/// 把 ps1 内容写到临时文件，返回路径。
fn write_ps1(dir: &tempfile::TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let p = dir.path().join(name);
    std::fs::write(&p, content).unwrap();
    p
}

#[cfg(windows)]
#[test]
fn runner_inserts_items_and_emits() {
    let (_dir, conn, sid) = setup();
    let emitter = VecEmitter::default();
    let runner = ScanRunner::new(sid, &conn, &emitter);

    let ps1 = r#"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
[Console]::Out.WriteLine('{"type":"item","module_code":"test","category":"cat","path":"C:\\temp\\a.tmp","size_bytes":100,"risk_level":1}')
[Console]::Out.WriteLine('{"type":"item","module_code":"test","category":"cat","path":"C:\\temp\\b.tmp","size_bytes":100,"risk_level":2}')
[Console]::Out.WriteLine('{"type":"done","module_code":"test","total_items":2,"total_bytes":200}')
"#;
    let tmp = tempfile::tempdir().unwrap();
    let script = write_ps1(&tmp, "t.ps1", ps1);

    let summary = runner.run_script(&script).unwrap();
    assert_eq!(summary.items_inserted, 2);
    assert!(summary.done_seen);
    assert_eq!(summary.total_bytes, 200);

    let n: i64 = conn
        .query_row(
            "SELECT count(*) FROM cleanup_item WHERE session_id=?1",
            [sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 2);

    // emit 次数 = item 数（done 不 emit）。
    assert_eq!(emitter.snapshot().len(), 2);
}

#[cfg(windows)]
#[test]
fn runner_skips_bad_lines() {
    let (_dir, conn, sid) = setup();
    let runner = ScanRunner::new(sid, &conn, &NoopEmitter);

    let ps1 = r#"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
[Console]::Out.WriteLine('this is not json')
[Console]::Out.WriteLine('{"type":"item","module_code":"test","category":"cat","path":"C:\\temp\\a.tmp","size_bytes":100,"risk_level":1}')
[Console]::Out.WriteLine('{"type":"done","module_code":"test","total_items":1,"total_bytes":100}')
"#;
    let tmp = tempfile::tempdir().unwrap();
    let script = write_ps1(&tmp, "t.ps1", ps1);

    let summary = runner.run_script(&script).unwrap();
    assert_eq!(summary.items_inserted, 1);
    assert!(summary.done_seen);

    let n: i64 = conn
        .query_row(
            "SELECT count(*) FROM cleanup_item WHERE session_id=?1",
            [sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 1);
}

#[cfg(windows)]
#[test]
fn runner_script_failure_without_done_returns_err() {
    let (_dir, conn, sid) = setup();
    let runner = ScanRunner::new(sid, &conn, &NoopEmitter);

    let ps1 = r#"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
[Console]::Out.WriteLine('{"type":"item","module_code":"test","category":"cat","path":"C:\\temp\\a.tmp","size_bytes":100,"risk_level":1}')
exit 1
"#;
    let tmp = tempfile::tempdir().unwrap();
    let script = write_ps1(&tmp, "t.ps1", ps1);

    let err = runner.run_script(&script).unwrap_err();
    // item 已入库（即使最终报错）。
    let n: i64 = conn
        .query_row(
            "SELECT count(*) FROM cleanup_item WHERE session_id=?1",
            [sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 1);
    assert!(matches!(err, engine::scan::RunError::ScriptFailed(_)));
}
