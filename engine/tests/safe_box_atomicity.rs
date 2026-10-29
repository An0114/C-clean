//! task-058 单元 2：`SafeBoxDeleter` 的「行与文件一致」原子性。
//!
//! task-016 遗留的孤儿根因是**顺序反了**：旧实现先 `fs::rename` 再
//! `INSERT INTO safe_box`，一旦 INSERT 失败（磁盘满 / 约束 / 库被锁），
//! 文件已在 box 目录而 DB 无行——下次 `enforce_quota` 扫到它只会当孤儿标
//! `purged`，用户感知是「文件凭空消失」。
//!
//! 修法：把 INSERT 提到 rename **之前**，整段包在事务里。于是
//! - INSERT 失败 → 文件根本没动（不需要回滚文件）；
//! - rename 失败 → 事务 Drop 自动 rollback，行消失、文件在原位；
//! - 跨盘 copy 失败 / remove 失败 → 删掉半成品目标文件后再报错。
//!
//! 本文件用**外键约束**制造 INSERT 失败：`safe_box.source_task_id` 引用
//! `cleanup_task(id)`，传一个不存在的 id 即可让 INSERT 报
//! `FOREIGN KEY constraint failed`，无需改 schema、无需注入 mock。

use engine::execute::delete::Deleter;
use engine::execute::normalize::normalize;
use engine::execute::safe_box::{set_quota_for_test, SafeBoxDeleter};

fn setup() -> (tempfile::TempDir, rusqlite::Connection, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let conn = engine::db::conn::open(dir.path()).unwrap();
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (1000, 'running', 'full')",
        [],
    )
    .unwrap();
    let box_dir = dir.path().join("safe_box");
    // 配额调到足够大，避免 enforce_quota 干扰本用例（它只处理 status='in_box' 行）。
    set_quota_for_test(1024 * 1024 * 1024);
    (dir, conn, box_dir)
}

/// 数 box 目录里的文件（不含子目录）。
fn box_file_count(box_dir: &std::path::Path) -> usize {
    match std::fs::read_dir(box_dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .count(),
        Err(_) => 0, // 目录不存在 = 0 个文件
    }
}

fn box_row_count(conn: &rusqlite::Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM safe_box", [], |r| r.get(0))
        .unwrap()
}

/// **核心用例**：INSERT 失败 → 原文件还在原位、box 目录零孤儿、DB 零行。
#[test]
fn insert_failure_leaves_source_file_in_place_and_no_orphan() {
    let (dir, conn, box_dir) = setup();

    let src = dir.path().join("atomic.tmp");
    let content: Vec<u8> = (0..512u32).map(|i| (i % 251) as u8).collect();
    std::fs::write(&src, &content).unwrap();

    // source_task_id 指向不存在的 cleanup_task → INSERT 必然违反外键约束。
    let deleter = SafeBoxDeleter::new(&box_dir, &conn, Some(999_999));
    let np = normalize(src.to_str().unwrap()).unwrap();

    let err = deleter
        .delete_file(&np)
        .expect_err("外键约束失败必须让 delete_file 返回 Err");
    assert!(
        err.contains("insert safe_box row") || err.contains("FOREIGN KEY"),
        "错误应来自 INSERT 阶段，实际: {err}"
    );

    // ① 源文件还在原位、内容未变（说明 rename 根本没执行）
    assert!(src.exists(), "INSERT 失败时源文件必须留在原位");
    assert_eq!(std::fs::read(&src).unwrap(), content, "源文件内容不能变");

    // ② box 目录没有孤儿文件
    assert_eq!(box_file_count(&box_dir), 0, "box 目录不应留下任何孤儿文件");

    // ③ DB 里没有半条记录
    assert_eq!(box_row_count(&conn), 0, "失败的移入不应留下 DB 行");
}

/// 对照：正常路径下「文件进 box」与「DB 行」必须同时成立。
///
/// 防止上面的修法把正常路径也改坏（例如事务没 commit、或 commit 前就返回）。
#[test]
fn success_path_keeps_file_and_row_consistent() {
    let (dir, conn, box_dir) = setup();

    let src = dir.path().join("normal.tmp");
    let content: Vec<u8> = (0..256u32).map(|i| (i % 97) as u8).collect();
    std::fs::write(&src, &content).unwrap();

    // 先建一条真实 cleanup_task，让外键成立（列名按 v1__init.sql 真实 schema）。
    conn.execute(
        "INSERT INTO cleanup_task (session_id, started_at, finished_at, disk_before_bytes,
                                   disk_after_bytes, freed_bytes, success_count, fail_count,
                                   skip_count, dry_run_snapshot, restore_point_name, safebox_enabled)
         VALUES (NULL, 1000, 1001, 0, 0, 256, 1, 0, 0, '[]', NULL, 1)",
        [],
    )
    .unwrap();
    let task_id = conn.last_insert_rowid();

    let deleter = SafeBoxDeleter::new(&box_dir, &conn, Some(task_id));
    let np = normalize(src.to_str().unwrap()).unwrap();
    let moved = deleter.delete_file(&np).unwrap();
    assert!(moved, "正常移入应返回 true");

    // 文件离开原位
    assert!(!src.exists(), "源文件应已移走");
    // box 里恰好一个文件，内容一致
    assert_eq!(box_file_count(&box_dir), 1);
    let rel: String = conn
        .query_row("SELECT safe_rel_path FROM safe_box LIMIT 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(std::fs::read(box_dir.join(&rel)).unwrap(), content);

    // DB 一行、状态 in_box、source_task_id 正确
    let (status, size, sid): (String, i64, Option<i64>) = conn
        .query_row(
            "SELECT status, size_bytes, source_task_id FROM safe_box LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(status, "in_box");
    assert_eq!(size, content.len() as i64);
    assert_eq!(sid, Some(task_id), "source_task_id 必须真实写入");
}

/// 源文件不存在（竞态）：返回 `Ok(false)`（幂等），且不留 DB 行。
#[test]
fn missing_source_returns_false_without_row() {
    let (dir, conn, box_dir) = setup();

    let ghost = dir.path().join("never-existed.tmp");
    let deleter = SafeBoxDeleter::new(&box_dir, &conn, None);
    let np = normalize(ghost.to_str().unwrap()).unwrap();

    // 注意：`fs::rename` 对不存在的源返回 NotFound → 走 Ok(false) 分支，
    // 此时事务已被 Drop 回滚（行不落库）。
    let moved = deleter.delete_file(&np).unwrap_or(false);
    assert!(!moved, "源文件不存在时应返回 false");
    assert_eq!(box_row_count(&conn), 0, "不应留下 DB 行");
    assert_eq!(box_file_count(&box_dir), 0, "不应留下 box 文件");
}
