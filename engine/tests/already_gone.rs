//! task-052 G5 集成测试：区分「真删了」与「本来就不存在」。
//!
//! 走查缺陷：部分批次显示「已清理 1 项，释放 0 B」，历史行 `bytes_freed: 0` / `err: null`。
//! 根因是 `SafeBoxDeleter::delete_file` 对 NotFound 幂等返回 `Ok(())`，调用方无法分辨
//! 「真删了」和「本来就没有」，于是把 0 字节当成功释放量计入。
//!
//! 本文件覆盖任务书第 18 条 G5 要求：
//! - deleter 对 NotFound 返回 `Ok(false)`；
//! - executor 聚合时 `already_gone` 不计入 `bytes_freed`、也不算失败；
//! - 混合场景（有真删 + 有已不存在）三类计数各归各位。

#![cfg(windows)]

use engine::db::conn::open;
use engine::execute::delete::Deleter;
use engine::execute::executor::execute_items;
use engine::execute::fs_deleter::FsDeleter;
use engine::execute::normalize::normalize;
use engine::execute::safe_box::SafeBoxDeleter;

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

fn insert_item(conn: &rusqlite::Connection, sid: i64, path: &str, size: i64) -> i64 {
    conn.execute(
        "INSERT INTO cleanup_item
         (session_id, module_code, category, path, size_bytes, risk_level, description)
         VALUES (?1, 'A', 'test', ?2, ?3, 'L1', 'desc')",
        rusqlite::params![sid, path, size],
    )
    .unwrap();
    conn.last_insert_rowid()
}

// ---------------------------------------------------------------------------
// 第一层：deleter 自身的返回值语义
// ---------------------------------------------------------------------------

/// G5-1：`FsDeleter` 遇到不存在的文件必须返回 `Ok(false)`（不是 `Ok(true)`）。
#[test]
fn fs_deleter_returns_false_for_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope.tmp");
    let np = normalize(missing.to_str().unwrap()).unwrap();

    let got = FsDeleter.delete_file(&np).unwrap();
    assert!(!got, "NotFound 必须返回 false（表示没删到任何东西）");
}

/// G5-2：`SafeBoxDeleter` 遇到不存在的文件同样返回 `Ok(false)`，且不写 safe_box 行。
#[test]
fn safe_box_deleter_returns_false_for_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    let missing = dir.path().join("ghost.tmp");
    let np = normalize(missing.to_str().unwrap()).unwrap();

    let box_dir = dir.path().join("safe_box");
    let got = SafeBoxDeleter::new(&box_dir, &conn, Some(1))
        .delete_file(&np)
        .unwrap();
    assert!(!got, "NotFound 必须返回 false");

    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM safe_box", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 0, "没移走东西就不该写 safe_box 行");
}

/// G5-3：真实文件必须返回 `Ok(true)`（防止修 G5 时把正向语义也弄丢）。
#[test]
fn fs_deleter_returns_true_for_real_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("real.tmp");
    std::fs::write(&f, b"hello").unwrap();
    let np = normalize(f.to_str().unwrap()).unwrap();

    let got = FsDeleter.delete_file(&np).unwrap();
    assert!(got, "真的删掉了必须返回 true");
    assert!(!f.exists());
}

// ---------------------------------------------------------------------------
// 第二层：executor 聚合口径（走查缺陷的直接复现）
// ---------------------------------------------------------------------------

/// G5-4（核心回归）：扫描后文件被外部删掉 → 只算「已不存在」，不算释放、不算失败。
///
/// 这正是走查看到的「已清理 1 项，释放 0 B」场景。
#[test]
fn executor_counts_already_gone_separately() {
    let (dir, conn, sid) = setup();
    let f = dir.path().join("vanished.tmp");
    std::fs::write(&f, vec![0u8; 100]).unwrap();
    let id = insert_item(&conn, sid, f.to_str().unwrap(), 100);

    // 模拟「扫描后、清理前被系统/其他程序删掉」。
    std::fs::remove_file(&f).unwrap();
    assert!(!f.exists());

    let summary = execute_items(&[id], &conn, &[], |_id, _task_id| Box::new(FsDeleter));

    assert_eq!(summary.already_gone_count, 1, "应记 1 项已不存在");
    assert_eq!(summary.deleted_count, 0, "没有真删任何东西");
    assert_eq!(summary.failed_count, 0, "已不存在不算失败");
    assert_eq!(
        summary.bytes_freed_total, 0,
        "释放量必须是 0，而不是旧行为的 100"
    );

    let r = &summary.results[0];
    assert!(r.already_gone);
    assert!(!r.deleted);
    assert_eq!(r.bytes_freed, 0);
    assert!(r.error.is_none(), "已不存在不应带错误文案");
}

/// G5-5：混合场景——一个真删 + 一个已不存在，三类计数互不串味。
#[test]
fn executor_mixed_real_and_gone() {
    let (dir, conn, sid) = setup();
    let real = dir.path().join("real.tmp");
    let gone = dir.path().join("gone.tmp");
    std::fs::write(&real, vec![0u8; 250]).unwrap();
    std::fs::write(&gone, vec![0u8; 999]).unwrap();
    let id_real = insert_item(&conn, sid, real.to_str().unwrap(), 250);
    let id_gone = insert_item(&conn, sid, gone.to_str().unwrap(), 999);
    std::fs::remove_file(&gone).unwrap();

    let summary = execute_items(&[id_real, id_gone], &conn, &[], |_id, _task_id| {
        Box::new(FsDeleter)
    });

    assert_eq!(summary.deleted_count, 1);
    assert_eq!(summary.already_gone_count, 1);
    assert_eq!(summary.failed_count, 0);
    // 只算真实文件那 250 字节，绝不把已不存在的 999 算进去。
    assert_eq!(summary.bytes_freed_total, 250);
}

/// G5-6：audit_log 的 detail 必须带上 `already_gone` 标记，便于事后取证。
#[test]
fn audit_log_records_already_gone_flag() {
    let (dir, conn, sid) = setup();
    let f = dir.path().join("ghost.tmp");
    std::fs::write(&f, vec![0u8; 10]).unwrap();
    let id = insert_item(&conn, sid, f.to_str().unwrap(), 10);
    std::fs::remove_file(&f).unwrap();

    execute_items(&[id], &conn, &[], |_id, _task_id| Box::new(FsDeleter));

    let detail: String = conn
        .query_row(
            "SELECT detail FROM audit_log ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&detail).unwrap();
    assert_eq!(v["already_gone"], serde_json::json!(true));
    assert_eq!(v["bytes_freed"], serde_json::json!(0));
}

/// G5-7：cleanup_task 的 success_count 不应把「已不存在」算成成功释放
/// （否则 Dashboard 的成功数与释放量对不上）。
#[test]
fn cleanup_task_counts_exclude_already_gone() {
    let (dir, conn, sid) = setup();
    let f = dir.path().join("gone.tmp");
    std::fs::write(&f, vec![0u8; 42]).unwrap();
    let id = insert_item(&conn, sid, f.to_str().unwrap(), 42);
    std::fs::remove_file(&f).unwrap();

    execute_items(&[id], &conn, &[], |_id, _task_id| Box::new(FsDeleter));

    let (freed, success, fail): (i64, i64, i64) = conn
        .query_row(
            "SELECT freed_bytes, success_count, fail_count FROM cleanup_task ORDER BY id DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(freed, 0);
    assert_eq!(success, 0, "已不存在不算成功");
    assert_eq!(fail, 0, "已不存在也不算失败");
}
