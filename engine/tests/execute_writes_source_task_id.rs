//! task-052 G3 回归测试：`execute_items` 必须把 `cleanup_task.id` 传给 deleter，
//! 让 `safe_box.source_task_id` 真实落库。
//!
//! **为什么必须有一条"真实跑一遍"的用例**：task-051 的 26 条保险箱用例全是
//! 「先构造 DB 行 → 再读」的读路径测试，`source_task_id` 是测试自己写进去的，
//! 所以写路径把第三参写死 `None` 这个 bug 一条都没照到 —— 真机走查才发现
//! Session 下拉全空、来源列全是 `—`（走查报告 G3）。
//!
//! 本文件刻意**不构造 safe_box 行**，而是：
//! 1. 造真实文件 + cleanup_item 行；
//! 2. 调 `execute_items`（真实走白名单 → SafeBoxDeleter → 移入保险箱 → 写行）；
//! 3. 才去查 `safe_box.source_task_id` 与 `list_sessions`。
//!
//! 任何一步回退到「写死 None」，本文件立即红。

#![cfg(windows)]

use std::path::Path;

use engine::db::conn::open;
use engine::execute::delete::Deleter;
use engine::execute::executor::execute_items;
use engine::execute::safe_box::{list_sessions, SafeBoxDeleter};

/// 造库 + 一条 scan_session + 一个真实文件 + 对应 cleanup_item 行。
/// 返回 (临时库目录, 临时文件目录, conn, item_id)。
fn setup() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    rusqlite::Connection,
    i64,
) {
    let data_dir = tempfile::tempdir().unwrap();
    let file_dir = tempfile::tempdir().unwrap();
    let conn = open(data_dir.path()).unwrap();

    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (1000, 'finished', 'full')",
        [],
    )
    .unwrap();
    let sid = conn.last_insert_rowid();

    // 真实文件：SafeBoxDeleter 会把它 rename 进 <data_dir>/safe_box/。
    let file_path = file_dir.path().join("will_quarantine.tmp");
    std::fs::write(&file_path, b"0123456789").unwrap();

    conn.execute(
        "INSERT INTO cleanup_item (session_id, module_code, category, path, size_bytes,
             risk_level, description)
         VALUES (?1, 'temp', 'cat', ?2, 10, 'L1', '')",
        rusqlite::params![sid, file_path.to_string_lossy()],
    )
    .unwrap();
    let item_id = conn.last_insert_rowid();

    (data_dir, file_dir, conn, item_id)
}

/// G3 核心：真实执行一遍，`safe_box.source_task_id` 必须等于本次 `cleanup_task.id`。
#[test]
fn execute_items_writes_source_task_id() {
    let (data_dir, _file_dir, conn, item_id) = setup();
    let box_dir = data_dir.path().join("safe_box");

    let summary = execute_items(&[item_id], &conn, &[], |_id, task_id| {
        let source = if task_id >= 0 { Some(task_id) } else { None };
        Box::new(SafeBoxDeleter::new(&box_dir, &conn, source))
    });
    assert_eq!(summary.deleted_count, 1, "文件应被移入保险箱");
    assert_eq!(summary.failed_count, 0);

    // 本次 cleanup_task 的 id。
    let task_id: i64 = conn
        .query_row(
            "SELECT id FROM cleanup_task ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(task_id > 0);

    // 关键断言：source_task_id 非 NULL 且等于本次 task id。
    let source: Option<i64> = conn
        .query_row(
            "SELECT source_task_id FROM safe_box ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        source,
        Some(task_id),
        "safe_box.source_task_id 必须是本次 cleanup_task.id（NULL 说明 deleter 又被写死 None）"
    );

    // 文件真的进箱了（不只是写了 DB 行）。
    let in_box: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM safe_box WHERE status='in_box'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(in_box, 1);
}

/// G3 用户可见后果：`list_sessions`（Session 下拉的数据源）必须能列出这一批。
#[test]
fn list_sessions_sees_executed_batch() {
    let (data_dir, _file_dir, conn, item_id) = setup();
    let box_dir = data_dir.path().join("safe_box");

    execute_items(&[item_id], &conn, &[], |_id, task_id| {
        let source = if task_id >= 0 { Some(task_id) } else { None };
        Box::new(SafeBoxDeleter::new(&box_dir, &conn, source))
    });

    let sessions = list_sessions(&conn).unwrap();
    assert_eq!(
        sessions.len(),
        1,
        "Session 下拉必须有 1 条（原 bug 下恒为空集）"
    );
    assert_eq!(sessions[0].item_count, 1);
    assert!(sessions[0].session_id > 0);
}

/// G3 负向：工厂拿到的 `task_id` 必须 > 0（即真的来自 last_insert_rowid），
/// 而不是调用方硬塞的常量。用闭包捕获记录实际入参。
#[test]
fn factory_receives_positive_task_id() {
    let (data_dir, _file_dir, conn, item_id) = setup();
    let box_dir = data_dir.path().join("safe_box");
    let seen = std::cell::Cell::new(-2i64);

    execute_items(&[item_id], &conn, &[], |_id, task_id| {
        seen.set(task_id);
        let source = if task_id >= 0 { Some(task_id) } else { None };
        Box::new(SafeBoxDeleter::new(&box_dir, &conn, source))
    });

    assert!(
        seen.get() > 0,
        "deleter 工厂拿到的 task_id 应为本批 cleanup_task.id，实际 {}",
        seen.get()
    );
}

/// G3 边界：多个 item 同批执行时，全部落到**同一个** source_task_id（一次清理 = 一个批次）。
#[test]
fn batch_shares_one_source_task_id() {
    let (data_dir, file_dir, conn, _first) = setup();
    let box_dir = data_dir.path().join("safe_box");

    // 再补两个真实文件 + 对应 cleanup_item 行。
    let mut ids = Vec::new();
    for i in 0..2 {
        let p: &Path = &file_dir.path().join(format!("extra_{i}.tmp"));
        std::fs::write(p, b"01234").unwrap();
        conn.execute(
            "INSERT INTO cleanup_item (session_id, module_code, category, path, size_bytes,
                 risk_level, description)
             VALUES (1, 'temp', 'cat', ?1, 5, 'L1', '')",
            rusqlite::params![p.to_string_lossy()],
        )
        .unwrap();
        ids.push(conn.last_insert_rowid());
    }

    execute_items(&ids, &conn, &[], |_id, task_id| {
        let source = if task_id >= 0 { Some(task_id) } else { None };
        Box::new(SafeBoxDeleter::new(&box_dir, &conn, source))
    });

    let distinct: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT source_task_id) FROM safe_box WHERE status='in_box'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(distinct, 1, "同一批清理必须共用同一个 source_task_id");

    let nulls: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM safe_box WHERE source_task_id IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(nulls, 0, "不应有任何一条 source_task_id 为 NULL");
}

/// G3 保证：`SafeBoxDeleter` 仍是唯一的写入方（防止有人绕过 execute_items 直写 NULL）。
/// 这里验证「显式传 None 时确实落 NULL」——说明列的写入路径受 deleter 参数控制，
/// 与上面的正向用例互为对照。
#[test]
fn deleter_with_none_writes_null() {
    let (data_dir, file_dir, conn, item_id) = setup();
    let box_dir = data_dir.path().join("safe_box");
    let p = file_dir.path().join("direct.tmp");
    std::fs::write(&p, b"abc").unwrap();
    let np = engine::execute::normalize::normalize(&p.to_string_lossy()).unwrap();

    let deleter = SafeBoxDeleter::new(&box_dir, &conn, None);
    deleter.delete_file(&np).unwrap();

    let source: Option<i64> = conn
        .query_row(
            "SELECT source_task_id FROM safe_box ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(source, None);
    // 同时确认这条不是 execute_items 写的那条（item_id 仍未被处理）。
    let untouched: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM cleanup_item WHERE id = ?1",
            [item_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(untouched, 1);
}
