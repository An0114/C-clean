//! task-051 集成测试：safe_box.list_paged 的多 session（source_task_id）组合筛选（B10）。
//!
//! 真实 schema 里 safe_box 没有 session_id 列，只有 source_task_id 外键；
//! 一个 cleanup_task 对应一次清理 session，测试里的 session 写库时落到
//! source_task_id 列（需先插 scan_session + cleanup_task 满足 FK）。

use engine::db::conn::open;
use engine::execute::safe_box::{list_paged, SafeBoxFilter};

fn setup() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    (dir, conn)
}

fn seed_session(conn: &rusqlite::Connection, session_id: i64, base_ts: i64, count: i64) {
    conn.execute(
        "INSERT INTO scan_session (id, started_at, status, scope) VALUES (?1, ?2, 'finished', 'full')",
        rusqlite::params![session_id, base_ts],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO cleanup_task (id, session_id, started_at, finished_at, disk_before_bytes, disk_after_bytes, dry_run_snapshot, safebox_enabled)
         VALUES (?1, ?1, ?2, ?2, 100000, 90000, '[]', 1)",
        rusqlite::params![session_id, base_ts],
    )
    .unwrap();
    for i in 0..count {
        conn.execute(
            "INSERT INTO safe_box (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status, source_task_id)
             VALUES (?1, ?2, 10, ?3, ?3+604800, 'in_box', ?4)",
            rusqlite::params![
                format!("C:\\ghost\\s{session_id}_{i}.bin"),
                format!("s{session_id}_{i}.bin"),
                base_ts + i,
                session_id
            ],
        )
        .unwrap();
    }
}

/// 三个 session：1 → 3 条（ts 1000..1002）、2 → 2 条（ts 2000..2001）、3 → 4 条（ts 3000..3003）。
fn seed_three(conn: &rusqlite::Connection) {
    seed_session(conn, 1, 1000, 3);
    seed_session(conn, 2, 2000, 2);
    seed_session(conn, 3, 3000, 4);
}

#[test]
fn two_sessions_return_only_those_items() {
    let (_dir, conn) = setup();
    seed_three(&conn);

    let f = SafeBoxFilter {
        session_ids: vec![1, 3],
        ..Default::default()
    };
    let p = list_paged(&conn, 1, 100, &f).unwrap();

    assert_eq!(p.items.len(), 7, "session 1(3) + session 3(4)");
    assert_eq!(p.total, 7);
    assert_eq!(p.total_bytes, 70);
    for it in &p.items {
        let path = &it.original_path;
        assert!(
            path.contains("s1_") || path.contains("s3_"),
            "混入了非选中 session 的条目: {path}"
        );
    }
}

#[test]
fn single_existing_session_still_works() {
    let (_dir, conn) = setup();
    seed_three(&conn);

    let f = SafeBoxFilter {
        session_ids: vec![2],
        ..Default::default()
    };
    let p = list_paged(&conn, 1, 100, &f).unwrap();

    assert_eq!(p.items.len(), 2);
    assert_eq!(p.total, 2);
}

#[test]
fn nonexistent_session_returns_empty_page() {
    let (_dir, conn) = setup();
    seed_three(&conn);

    let f = SafeBoxFilter {
        session_ids: vec![999],
        ..Default::default()
    };
    let p = list_paged(&conn, 1, 100, &f).unwrap();

    assert!(p.items.is_empty());
    assert_eq!(p.total, 0);
    assert_eq!(p.total_bytes, 0);
}

#[test]
fn empty_session_ids_equals_no_filter() {
    let (_dir, conn) = setup();
    seed_three(&conn);

    let p = list_paged(&conn, 1, 100, &SafeBoxFilter::default()).unwrap();

    assert_eq!(p.items.len(), 9, "空 vec = 全部 3 个 session 共 9 条");
    assert_eq!(p.total, 9);
}

#[test]
fn duplicate_session_ids_are_deduped() {
    let (_dir, conn) = setup();
    seed_three(&conn);

    let f = SafeBoxFilter {
        session_ids: vec![1, 1, 1],
        ..Default::default()
    };
    let p = list_paged(&conn, 1, 100, &f).unwrap();

    assert_eq!(p.items.len(), 3, "重复 id 不应放大结果");
    assert_eq!(p.total, 3);
}

#[test]
fn multi_session_with_time_range_and_pagination() {
    let (_dir, conn) = setup();
    seed_three(&conn);

    // 选 session 1 + 3（共 7 条，ts 1000..1002 / 3000..3003），再限定 ts >= 2000 → 只剩 4 条。
    let f = SafeBoxFilter {
        session_ids: vec![1, 3],
        from_ts: Some(2000),
        to_ts: None,
    };
    let p = list_paged(&conn, 1, 3, &f).unwrap();

    assert_eq!(p.items.len(), 3, "page_size=3 时首页 3 条");
    assert_eq!(p.total, 4, "total 是 session+时间双条件后的总数");
    assert_eq!(p.total_bytes, 40);

    let p2 = list_paged(&conn, 2, 3, &f).unwrap();
    assert_eq!(p2.items.len(), 1, "第 2 页剩 1 条");
    assert_eq!(p2.total, 4, "翻页不改变 total");
    for it in p2.items.iter().chain(p.items.iter()) {
        assert!(it.quarantined_at >= 2000);
        assert!(
            it.original_path.contains("s3_"),
            "session 1 的条目 ts<2000 应被排除"
        );
    }
}
