//! task-029 集成测试：`list_pending_page`（原 `list_pending_items`）。
//!
//! 与任务书偏差：cleanup_item 表无 status 列，「pending」= 表内所有行。
//! 故用例1 改为「列出所有行」（不区分状态），用例2/3 保持 session 过滤 / size 倒序。
//!
//! task-052 G2：签名升级为 `(conn, session_id, page, page_size) -> PendingPage`，
//! 新增 `total` 与「同 size 翻页不跳行」两条回归用例。

use engine::db::conn::open;
use engine::execute::list_pending::list_pending_page;

fn ensure_session(conn: &rusqlite::Connection, id: i64) {
    conn.execute(
        "INSERT OR IGNORE INTO scan_session (id, started_at, status, scope)
         VALUES (?1, 0, 'finished', 'full')",
        rusqlite::params![id],
    )
    .unwrap();
}

fn insert_item(
    conn: &rusqlite::Connection,
    session_id: i64,
    size: i64,
    path: &str,
    mod_code: &str,
) {
    conn.execute(
        "INSERT INTO cleanup_item (session_id, module_code, category, path, size_bytes,
             risk_level, description)
         VALUES (?1, ?2, 'cat', ?3, ?4, 'L1', 'desc')",
        rusqlite::params![session_id, mod_code, path, size],
    )
    .unwrap();
}

#[test]
fn list_returns_all_rows_no_status_column() {
    // 任务书原假设 status='pending' 过滤；实际表无 status 列，返回所有行。
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    ensure_session(&conn, 1);
    insert_item(&conn, 1, 100, "/a", "A");
    insert_item(&conn, 1, 200, "/b", "B");
    let page = list_pending_page(&conn, None, 1, 100).unwrap();
    assert_eq!(page.items.len(), 2);
    assert_eq!(page.total, 2);
}

#[test]
fn list_filters_by_session() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    ensure_session(&conn, 1);
    ensure_session(&conn, 2);
    insert_item(&conn, 1, 100, "/a", "A");
    insert_item(&conn, 2, 200, "/b", "B");
    let page = list_pending_page(&conn, Some(1), 1, 100).unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].path, "/a");
    // total 必须跟随 session 过滤，否则前端分页栏会算出多余的页。
    assert_eq!(page.total, 1);
}

#[test]
fn list_orders_by_size_desc() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    ensure_session(&conn, 1);
    insert_item(&conn, 1, 100, "/small", "A");
    insert_item(&conn, 1, 300, "/big", "A");
    insert_item(&conn, 1, 200, "/mid", "A");
    let page = list_pending_page(&conn, None, 1, 100).unwrap();
    assert_eq!(page.items.len(), 3);
    assert_eq!(page.items[0].size_bytes, 300);
    assert_eq!(page.items[1].size_bytes, 200);
    assert_eq!(page.items[2].size_bytes, 100);
}

#[test]
fn list_applies_page_and_page_size() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    ensure_session(&conn, 1);
    for i in 0..5 {
        insert_item(&conn, 1, i * 100, &format!("/p{}", i), "A");
    }
    let page1 = list_pending_page(&conn, None, 1, 2).unwrap();
    assert_eq!(page1.items.len(), 2);
    assert_eq!(page1.items[0].size_bytes, 400);
    assert_eq!(page1.page, 1);
    assert_eq!(page1.page_size, 2);
    assert_eq!(page1.total, 5);

    let page2 = list_pending_page(&conn, None, 2, 2).unwrap();
    assert_eq!(page2.items.len(), 2);
    assert_eq!(page2.items[0].size_bytes, 200);
    assert_eq!(page2.page, 2);
}

/// task-052 G2 回归：`page_size` 越界必须被 clamp，不能让前端传 0/超大值把查询打挂。
#[test]
fn list_clamps_page_size_and_page() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    ensure_session(&conn, 1);
    insert_item(&conn, 1, 10, "/a", "A");

    // page=0 视为第 1 页；page_size=0 视为 1 条。
    let p = list_pending_page(&conn, None, 0, 0).unwrap();
    assert_eq!(p.page, 1);
    assert_eq!(p.page_size, 1);
    assert_eq!(p.items.len(), 1);

    // page_size 超上限按 500 算。
    let p = list_pending_page(&conn, None, 1, 99_999).unwrap();
    assert_eq!(p.page_size, 500);
}

/// task-052 G2 回归（核心）：`size_bytes` 全部相同的条目翻页时必须不重不漏。
///
/// 原 SQL 只 `ORDER BY size_bytes DESC`，同 size 行的相对顺序未定义，
/// SQLite 在换 LIMIT/OFFSET 时可能给出不同顺序 → 翻页跳行 / 重复。
/// 排序补 `id DESC` 后顺序确定，本用例遍历全部分页并断言 id 集合完整。
#[test]
fn list_paging_does_not_skip_equal_sizes() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    ensure_session(&conn, 1);
    // 6 条同 size 条目，page_size=2 → 3 页。
    for i in 0..6 {
        insert_item(&conn, 1, 500, &format!("/same{}", i), "A");
    }

    let mut seen: Vec<i64> = Vec::new();
    for page_no in 1..=3u32 {
        let p = list_pending_page(&conn, None, page_no, 2).unwrap();
        assert_eq!(p.total, 6, "每页返回的 total 必须一致");
        assert_eq!(p.items.len(), 2, "第 {page_no} 页应有 2 条");
        seen.extend(p.items.iter().map(|i| i.id));
    }

    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), 6, "翻页出现重复或漏行：{seen:?}");
}
