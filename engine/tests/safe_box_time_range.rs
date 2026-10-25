//! task-051 集成测试：safe_box.list_paged 的时间区间筛选（A7）。
//!
//! 时间单位是 Unix **秒**（`quarantined_at` 来自 `SystemTime::as_secs()`）。
//! 时间窗是**闭区间**：`from <= quarantined_at <= to`。

use engine::db::conn::open;
use engine::execute::safe_box::{list_paged, SafeBoxFilter};

fn setup() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    (dir, conn)
}

/// 插一行 safe_box：指定 quarantined_at 与 size_bytes。
fn insert_at(conn: &rusqlite::Connection, name: &str, ts: i64, size: i64) {
    conn.execute(
        "INSERT INTO safe_box (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status)
         VALUES (?1, ?2, ?3, ?4, ?4+604800, 'in_box')",
        rusqlite::params![format!("C:\\ghost\\{name}.bin"), format!("{name}.bin"), size, ts],
    )
    .unwrap();
}

/// 造 5 行：ts = 100/200/300/400/500，每行 10 字节。
fn seed_5(conn: &rusqlite::Connection) {
    for (i, ts) in [100i64, 200, 300, 400, 500].iter().enumerate() {
        insert_at(conn, &format!("r{i}"), *ts, 10);
    }
}

fn filter(from: Option<i64>, to: Option<i64>) -> SafeBoxFilter {
    SafeBoxFilter {
        session_ids: Vec::new(),
        from_ts: from,
        to_ts: to,
    }
}

#[test]
fn only_from_returns_items_after_bound_and_total_matches() {
    let (_dir, conn) = setup();
    seed_5(&conn);

    let p = list_paged(&conn, 1, 100, &filter(Some(300), None)).unwrap();

    // ts = 300 / 400 / 500
    assert_eq!(p.items.len(), 3);
    assert_eq!(p.total, 3, "total 必须与筛选后的条数一致（不是全表 5）");
    assert_eq!(p.total_bytes, 30, "totalBytes 也必须按筛选条件汇总");
    for it in &p.items {
        assert!(it.quarantined_at >= 300, "越界条目: {}", it.quarantined_at);
    }
}

#[test]
fn only_to_returns_items_before_bound_and_total_matches() {
    let (_dir, conn) = setup();
    seed_5(&conn);

    let p = list_paged(&conn, 1, 100, &filter(None, Some(300))).unwrap();

    // ts = 100 / 200 / 300
    assert_eq!(p.items.len(), 3);
    assert_eq!(p.total, 3);
    assert_eq!(p.total_bytes, 30);
    for it in &p.items {
        assert!(it.quarantined_at <= 300, "越界条目: {}", it.quarantined_at);
    }
}

#[test]
fn closed_interval_includes_both_bounds() {
    let (_dir, conn) = setup();
    seed_5(&conn);

    // from == 200 与 to == 400 的条目都必须被包含（闭区间）。
    let p = list_paged(&conn, 1, 100, &filter(Some(200), Some(400))).unwrap();

    assert_eq!(p.items.len(), 3);
    assert_eq!(p.total, 3);
    let ts: Vec<i64> = p.items.iter().map(|i| i.quarantined_at).collect();
    assert_eq!(ts, vec![400, 300, 200], "id DESC 顺序，且 200/400 都在内");
}

#[test]
fn from_greater_than_to_returns_empty_page_not_error() {
    let (_dir, conn) = setup();
    seed_5(&conn);

    let p = list_paged(&conn, 2, 20, &filter(Some(400), Some(200))).unwrap();

    assert!(p.items.is_empty());
    assert_eq!(p.total, 0);
    assert_eq!(p.total_bytes, 0);
    // page / pageSize 原样回显（clamp 后）。
    assert_eq!(p.page, 2);
    assert_eq!(p.page_size, 20);
}

#[test]
fn non_positive_bounds_are_ignored() {
    let (_dir, conn) = setup();
    seed_5(&conn);

    // 0 / 负数视为未设置 → 等价于不过滤。
    let p = list_paged(&conn, 1, 100, &filter(Some(0), Some(-1))).unwrap();

    assert_eq!(p.items.len(), 5);
    assert_eq!(p.total, 5);
}

#[test]
fn time_range_with_pagination_keeps_total_as_range_count() {
    let (_dir, conn) = setup();
    // 窗口内 25 行（ts 1000..1024），窗口外各 10 行。
    for i in 0..10 {
        insert_at(&conn, &format!("before{i}"), 500 + i, 10);
    }
    for i in 0..25 {
        insert_at(&conn, &format!("in{i}"), 1000 + i, 10);
    }
    for i in 0..10 {
        insert_at(&conn, &format!("after{i}"), 2000 + i, 10);
    }

    let p2 = list_paged(&conn, 2, 10, &filter(Some(1000), Some(1024))).unwrap();

    assert_eq!(p2.page, 2);
    assert_eq!(p2.page_size, 10);
    assert_eq!(p2.items.len(), 10, "第 2 页应是 10 条");
    assert_eq!(p2.total, 25, "total 是全区间总数，不是全表 45");
    assert_eq!(p2.total_bytes, 250, "totalBytes 同样是区间内汇总");
    for it in &p2.items {
        assert!(
            (1000..=1024).contains(&it.quarantined_at),
            "第 2 页混入区间外条目: {}",
            it.quarantined_at
        );
    }
}
