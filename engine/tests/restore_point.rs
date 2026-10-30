//! task-062 集成测试：`needs_restore_point` 的 L2 判定。
//!
//! 这条判断决定「清理前要不要弹 UAC 建系统还原点」——判错的两个方向代价都很大：
//! - 该建没建 → L2 高风险项裸奔清理，无法回滚；
//! - 不该建却建 → 每次清 L1 缓存都弹 UAC + 等几十秒，用户会直接关掉功能。

use engine::db::conn::open;
use engine::execute::executor::needs_restore_point;

/// 插一行 cleanup_item。`risk` 传 "L1" / "L2"。
fn insert_item(conn: &rusqlite::Connection, session_id: i64, path: &str, risk: &str) -> i64 {
    conn.execute(
        "INSERT INTO cleanup_item
         (session_id, module_code, category, path, size_bytes, risk_level, description)
         VALUES (?1, 'A', 'test_cat', ?2, 1024, ?3, 'test item')",
        rusqlite::params![session_id, path, risk],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn setup() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    conn.execute(
        "INSERT INTO scan_session (id, started_at, status, scope) VALUES (1, 0, 'finished', 'full')",
        [],
    )
    .unwrap();
    (dir, conn)
}

#[test]
fn all_l1_does_not_need_restore_point() {
    let (_d, conn) = setup();
    let a = insert_item(&conn, 1, r"C:\tmp\a.tmp", "L1");
    let b = insert_item(&conn, 1, r"C:\tmp\b.tmp", "L1");

    assert!(
        !needs_restore_point(&conn, &[a, b]).unwrap(),
        "全是 L1 缓存时不该弹 UAC 建还原点"
    );
}

#[test]
fn any_l2_needs_restore_point() {
    let (_d, conn) = setup();
    let a = insert_item(&conn, 1, r"C:\tmp\a.tmp", "L1");
    let b = insert_item(&conn, 1, r"C:\Windows\b.log", "L2");

    assert!(
        needs_restore_point(&conn, &[a, b]).unwrap(),
        "只要有一个 L2，整批清理前就要建还原点"
    );
}

#[test]
fn empty_ids_never_needs_restore_point() {
    let (_d, conn) = setup();
    assert!(!needs_restore_point(&conn, &[]).unwrap());
}

#[test]
fn unknown_ids_do_not_trigger() {
    let (_d, conn) = setup();
    insert_item(&conn, 1, r"C:\tmp\a.tmp", "L2");
    // 传一个不存在的 id：查不到行 → 不触发（真正的「id 不存在」由 execute_items 记 skip）
    assert!(!needs_restore_point(&conn, &[999_999]).unwrap());
}

#[test]
fn l2_outside_the_batch_does_not_trigger() {
    let (_d, conn) = setup();
    let l1 = insert_item(&conn, 1, r"C:\tmp\a.tmp", "L1");
    // 库里另有一个 L2，但**不在本次 batch 里** → 不该触发
    insert_item(&conn, 1, r"C:\Windows\other.log", "L2");

    assert!(
        !needs_restore_point(&conn, &[l1]).unwrap(),
        "只清理 L1 时，库里其它 L2 不该触发还原点"
    );
}
