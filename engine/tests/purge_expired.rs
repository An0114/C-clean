//! task-064 集成测试：保险箱 7 天自动 purge。
//!
//! 硬要求（任务书「端到端写路径」）：必须验证「造过期行 → purge → DB 行消失 +
//! 磁盘文件消失」，不能只测返回值。
//!
//! `now` 由调用方传入，所以这里能构造「昨天过期 / 明天过期」而不用等真实时间。

use engine::db::conn::open;
use engine::execute::safe_box::purge_expired;
use std::path::Path;

/// 在 box_dir 下造一个真实文件，并插一行 safe_box。
fn insert_boxed(
    conn: &rusqlite::Connection,
    box_dir: &Path,
    rel: &str,
    size: i64,
    expires_at: i64,
) -> i64 {
    let abs = box_dir.join(rel);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&abs, vec![b'x'; size.max(0) as usize]).unwrap();

    conn.execute(
        "INSERT INTO safe_box
         (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status, source_task_id)
         VALUES (?1, ?2, ?3, 0, ?4, 'in_box', NULL)",
        rusqlite::params![format!(r"C:\tmp\{rel}"), rel, size, expires_at],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn setup() -> (tempfile::TempDir, rusqlite::Connection, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    let box_dir = dir.path().join("safe_box");
    std::fs::create_dir_all(&box_dir).unwrap();
    (dir, conn, box_dir)
}

fn count_rows(conn: &rusqlite::Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM safe_box", [], |r| r.get(0))
        .unwrap()
}

const NOW: i64 = 1_000_000_000;

#[test]
fn purges_expired_row_and_deletes_file() {
    let (_d, conn, box_dir) = setup();
    // 昨天过期
    let id = insert_boxed(&conn, &box_dir, "a/x.tmp", 100, NOW - 86_400);
    let abs = box_dir.join("a/x.tmp");
    assert!(abs.exists(), "前置：文件应存在");

    let r = purge_expired(&conn, &box_dir, NOW);

    assert_eq!(r.purged, 1, "过期行应被 purge");
    assert_eq!(r.failed, 0);
    assert_eq!(r.bytes_freed, 100);
    assert_eq!(count_rows(&conn), 0, "DB 行必须消失");
    assert!(!abs.exists(), "磁盘文件必须消失");

    // audit_log 必须记一条 auto_purge（PRD 要求可追溯）
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM audit_log WHERE action='auto_purge' AND target=?1",
            [r"C:\tmp\a/x.tmp".to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 1, "每条 purge 都要有 auto_purge 审计（id={id}）");
}

#[test]
fn keeps_unexpired_rows() {
    let (_d, conn, box_dir) = setup();
    // 明天才过期
    insert_boxed(&conn, &box_dir, "b/y.tmp", 50, NOW + 86_400);

    let r = purge_expired(&conn, &box_dir, NOW);

    assert!(r.is_noop(), "没有过期项时不该干活");
    assert_eq!(count_rows(&conn), 1, "未过期行必须保留");
    assert!(box_dir.join("b/y.tmp").exists(), "未过期的文件必须保留");
}

#[test]
fn mixed_batch_only_removes_expired() {
    let (_d, conn, box_dir) = setup();
    insert_boxed(&conn, &box_dir, "old1.tmp", 10, NOW - 1);
    insert_boxed(&conn, &box_dir, "old2.tmp", 20, NOW - 100_000);
    insert_boxed(&conn, &box_dir, "new1.tmp", 30, NOW + 1);
    insert_boxed(&conn, &box_dir, "new2.tmp", 40, NOW + 999_999);

    let r = purge_expired(&conn, &box_dir, NOW);

    assert_eq!(r.purged, 2, "只删两条过期的");
    assert_eq!(r.bytes_freed, 30, "释放量 = 10 + 20");
    assert_eq!(count_rows(&conn), 2, "两条未过期的必须留着");
    assert!(!box_dir.join("old1.tmp").exists());
    assert!(!box_dir.join("old2.tmp").exists());
    assert!(box_dir.join("new1.tmp").exists());
    assert!(box_dir.join("new2.tmp").exists());
}

#[test]
fn expired_row_with_missing_file_still_deletes_row() {
    let (_d, conn, box_dir) = setup();
    insert_boxed(&conn, &box_dir, "gone.tmp", 10, NOW - 1);
    // 模拟「用户手动删了磁盘文件，但 DB 行还在」
    std::fs::remove_file(box_dir.join("gone.tmp")).unwrap();

    let r = purge_expired(&conn, &box_dir, NOW);

    assert_eq!(r.purged, 1, "文件已不存在时仍应删行（否则成永久孤儿行）");
    assert_eq!(count_rows(&conn), 0);
}

#[test]
fn boundary_expires_at_equal_now_is_kept() {
    let (_d, conn, box_dir) = setup();
    // 恰好等于 now：条件是 `expires_at < now`，等于不算过期
    insert_boxed(&conn, &box_dir, "edge.tmp", 10, NOW);

    let r = purge_expired(&conn, &box_dir, NOW);

    assert!(r.is_noop(), "expires_at == now 不算过期");
    assert_eq!(count_rows(&conn), 1);
}

#[test]
fn ignores_rows_not_in_box() {
    let (_d, conn, box_dir) = setup();
    // status 不是 in_box（例如已恢复）即使过期也不该被 purge
    conn.execute(
        "INSERT INTO safe_box
         (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status, source_task_id)
         VALUES ('C:\\x', 'restored.tmp', 5, 0, ?1, 'restored', NULL)",
        [NOW - 86_400],
    )
    .unwrap();

    let r = purge_expired(&conn, &box_dir, NOW);

    assert!(r.is_noop(), "非 in_box 的行不该被自动 purge");
    assert_eq!(count_rows(&conn), 1);
}

// ===================== v0.2.1 Bug-3：目录条目 =====================

/// 在 box_dir 下造一个**真实目录**（含嵌套文件），并插一行 safe_box。
///
/// 残留扫描（task-067）移入保险箱的是**整个目录**，不是单文件。旧实现一律
/// `fs::remove_file`，对这种行必然失败 → 前端只看到「永久删除失败：内部错误」。
fn insert_boxed_dir(
    conn: &rusqlite::Connection,
    box_dir: &Path,
    rel: &str,
    expires_at: i64,
) -> i64 {
    let abs = box_dir.join(rel);
    std::fs::create_dir_all(abs.join("nested")).unwrap();
    std::fs::write(abs.join("a.bin"), vec![b'x'; 128]).unwrap();
    std::fs::write(abs.join("nested").join("b.bin"), vec![b'y'; 64]).unwrap();

    conn.execute(
        "INSERT INTO safe_box
         (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status, source_task_id)
         VALUES (?1, ?2, 192, 0, ?3, 'in_box', NULL)",
        rusqlite::params![format!(r"C:/Program Files/{rel}"), rel, expires_at],
    )
    .unwrap();
    conn.last_insert_rowid()
}

#[test]
fn purge_expired_removes_directory_entries() {
    // Bug-3 回归：目录条目过期后必须能递归删掉，而不是每轮都失败重试。
    let (_dir, conn, box_dir) = setup();
    let id = insert_boxed_dir(&conn, &box_dir, "HONOR", 100);

    let r = purge_expired(&conn, &box_dir, 1_000);

    assert_eq!(r.purged, 1, "目录条目应被清掉，errors={:?}", r.errors);
    assert_eq!(r.failed, 0, "不该有失败：{:?}", r.errors);
    assert!(!box_dir.join("HONOR").exists(), "目录必须真的被删掉");
    assert_eq!(count_rows(&conn), 0);
    let _ = id;
}

#[test]
fn purge_item_removes_directory_entries() {
    // 与上面同一个 bug，但走「手动永久删除」那条路（purge_item）。
    use engine::execute::safe_box::purge_item;

    let (_dir, conn, box_dir) = setup();
    let id = insert_boxed_dir(&conn, &box_dir, "ZeroG", 100);

    purge_item(&conn, &box_dir, id).expect("目录条目永久删除必须成功");

    assert!(!box_dir.join("ZeroG").exists());
    assert_eq!(count_rows(&conn), 0);
}

#[test]
fn purge_item_is_idempotent_when_directory_already_gone() {
    // 磁盘上已经没了（用户手动清过保险箱目录）→ 仍然删 DB 行，不报错。
    use engine::execute::safe_box::purge_item;

    let (_dir, conn, box_dir) = setup();
    let id = insert_boxed_dir(&conn, &box_dir, "Gone", 100);
    std::fs::remove_dir_all(box_dir.join("Gone")).unwrap();

    purge_item(&conn, &box_dir, id).expect("目录已不存在也必须能删行");
    assert_eq!(count_rows(&conn), 0);
}
