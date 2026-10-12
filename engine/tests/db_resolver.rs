//! task-009 集成测试：DB 版 `DbItemPathResolver`。
//!
//! 每个测试用 `tempfile::tempdir()` 造临时数据根，通过 task-002 迁移器自动建表，
//! 不碰真实用户目录、不发网络。

use engine::db::conn::open;
use engine::db::resolver::DbItemPathResolver;
use engine::ipc::commands::dry_run::{handle_dry_run, DryRunReq, ItemPathResolver};

/// 造临时库 + 一条 scan_session，返回 session_id。
fn setup_session(conn: &rusqlite::Connection, started_at: i64) -> i64 {
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (?1, 'running', 'full')",
        [started_at],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// 插一条 cleanup_item。
fn insert_item(conn: &rusqlite::Connection, session_id: i64, id: i64, path: &str, size: i64) {
    conn.execute(
        "INSERT INTO cleanup_item (id, session_id, module_code, category, path, size_bytes, risk_level, description)
         VALUES (?1, ?2, 'A', 'npm_cache', ?3, ?4, 'L1', 'test')",
        rusqlite::params![id, session_id, path, size],
    )
    .unwrap();
}

#[test]
fn resolver_returns_empty_when_no_items() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    let r = DbItemPathResolver::new(&conn);
    assert!(r.resolve(&[1, 2, 3]).is_empty());
}

#[test]
fn resolver_returns_matching_items() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    let sid = setup_session(&conn, 1000);
    insert_item(&conn, sid, 10, r"C:\Temp\a.log", 100);
    insert_item(&conn, sid, 11, r"C:\Temp\b.log", 200);
    insert_item(&conn, sid, 12, r"C:\Temp\c.log", 300);

    let r = DbItemPathResolver::new(&conn);
    let got = r.resolve(&[10, 12]);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].item_id, 10);
    assert_eq!(got[0].path, r"C:\Temp\a.log");
    assert_eq!(got[0].size_bytes, 100);
    assert_eq!(got[0].module_code, "A");
    assert_eq!(got[0].category, "npm_cache");
    assert_eq!(got[1].item_id, 12);
    assert_eq!(got[1].size_bytes, 300);
}

#[test]
fn resolver_skips_unknown_ids() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    let sid = setup_session(&conn, 1000);
    insert_item(&conn, sid, 10, r"C:\Temp\a.log", 100);

    let r = DbItemPathResolver::new(&conn);
    let got = r.resolve(&[10, 999]);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].item_id, 10);
}

#[test]
fn resolver_dry_run_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    let sid = setup_session(&conn, 1000);
    // 一条 temp 放行项 + 一条 L3 桌面红线项。
    insert_item(&conn, sid, 1, r"C:\Windows\Temp\update.log", 50);
    insert_item(&conn, sid, 2, r"C:\Users\alice\Desktop\secret.docx", 999);

    let resolver = DbItemPathResolver::new(&conn);
    let env = handle_dry_run(
        DryRunReq {
            item_ids: vec![1, 2],
        },
        &resolver as &dyn ItemPathResolver,
        &[],
    );
    // WHITELIST_DENIED = 2002。
    assert_eq!(env.code, 2002, "L3 命中必须 2002: {env:?}");
    let data = env.data.expect("2002 偏离 TRD 需回传 blocked 明细");
    assert_eq!(data.blocked_by_l3.len(), 1);
    assert_eq!(data.blocked_by_l3[0].item_id, 2);
    assert!(data.blocked_by_l3[0]
        .denial_reason
        .contains("L3PersonalDir"));
}
