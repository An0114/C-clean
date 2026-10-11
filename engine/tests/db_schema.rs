//! engine::db 集成测试（task-002 §E）。
//!
//! 统一用 `tempfile::tempdir()` 造临时数据目录，禁止触碰真实 `%LOCALAPPDATA%`；
//! 不依赖网络 / 真实 C 盘 / 真实注册表。

use engine::db::conn::open;
use rusqlite::Connection;

/// 读取 PRAGMA user_version
fn user_version(conn: &Connection) -> i64 {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read user_version")
}

/// 插入一条最小 scan_session，返回其 id
fn insert_session(conn: &Connection) -> i64 {
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (?1, ?2, ?3)",
        rusqlite::params![1000i64, "finished", "full"],
    )
    .expect("insert session");
    conn.last_insert_rowid()
}

/// 插入一条最小 cleanup_item
fn insert_item(conn: &Connection, session_id: i64) {
    conn.execute(
        "INSERT INTO cleanup_item
            (session_id, module_code, category, path, risk_level, description)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            session_id,
            "A",
            "npm_cache",
            r"C:\tmp\cache",
            "L1",
            "测试清理项"
        ],
    )
    .expect("insert item");
}

/// 插入一条最小 cleanup_task，返回其 id
fn insert_task(conn: &Connection, session_id: i64) -> i64 {
    conn.execute(
        "INSERT INTO cleanup_task
            (session_id, started_at, finished_at, disk_before_bytes, disk_after_bytes,
             dry_run_snapshot, safebox_enabled)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![session_id, 1000i64, 2000i64, 100_000i64, 90_000i64, "[]", 1i64],
    )
    .expect("insert task");
    conn.last_insert_rowid()
}

#[test]
fn opens_and_creates_db_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = open(dir.path()).expect("open db");
    drop(conn);

    let db_file = dir.path().join("data").join("cdiskcleaner.db");
    assert!(
        db_file.exists(),
        "应在 <data_root>/data/cdiskcleaner.db 创建库文件"
    );
    // tempdir 自动清理
    dir.close().expect("close tempdir");
}

#[test]
fn creates_all_9_business_tables() {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = open(dir.path()).expect("open db");

    let n: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
            [],
            |row| row.get(0),
        )
        .expect("count tables");
    assert_eq!(n, 9, "应恰好创建 9 张业务表（sqlite_sequence 不计入）");
}

#[test]
fn foreign_keys_pragma_on() {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = open(dir.path()).expect("open db");

    let fk: i64 = conn
        .pragma_query_value(None, "foreign_keys", |row| row.get(0))
        .expect("pragma foreign_keys");
    assert_eq!(fk, 1, "外键约束必须开启");
}

#[test]
fn user_version_bumped_to_1() {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = open(dir.path()).expect("open db");

    assert_eq!(user_version(&conn), 1, "首次迁移后 user_version 应为 1");
}

#[test]
fn migration_is_idempotent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let first = open(dir.path()).expect("first open");
    drop(first);

    // 第二次打开同一数据目录：不再执行任何迁移，仍 Ok 且版本仍为 1
    let second = open(dir.path()).expect("second open should succeed (idempotent)");
    assert_eq!(user_version(&second), 1, "重复打开后 user_version 不应变化");
}

#[test]
fn rules_meta_rejects_id_not_1() {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = open(dir.path()).expect("open db");

    // CHECK(id = 1) 单行约束：插入 id = 2 必须触发 IntegrityError
    let result = conn.execute(
        "INSERT INTO rules_meta
            (id, version, checksum_sha256, source_url, downloaded_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            2i64,
            "1.0.0",
            "x".repeat(64),
            "https://example/rules_meta.json",
            1000i64
        ],
    );
    match result {
        // rusqlite 0.40：约束冲突以 SqliteFailure 暴露，ErrorCode 映射为 ConstraintViolation
        // （覆盖 CHECK / NOT NULL / UNIQUE / FK 等所有 SQLITE_CONSTRAINT_* 子类）
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == rusqlite::ErrorCode::ConstraintViolation => {}
        other => panic!("插入 id=2 的 rules_meta 应触发约束冲突，实际: {other:?}"),
    }

    // 正向：id = 1 可以插入
    conn.execute(
        "INSERT INTO rules_meta
            (id, version, checksum_sha256, source_url, downloaded_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            1i64,
            "1.0.0",
            "x".repeat(64),
            "https://example/rules_meta.json",
            1000i64
        ],
    )
    .expect("insert rules_meta id=1");
}

#[test]
fn cleanup_item_cascades_with_session() {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = open(dir.path()).expect("open db");

    let sid = insert_session(&conn);
    insert_item(&conn, sid);
    insert_item(&conn, sid);

    let before: i64 = conn
        .query_row("SELECT count(*) FROM cleanup_item", [], |r| r.get(0))
        .unwrap();
    assert_eq!(before, 2);

    // 删除 session：cleanup_item 应 ON DELETE CASCADE 连带清空
    conn.execute(
        "DELETE FROM scan_session WHERE id = ?1",
        rusqlite::params![sid],
    )
    .expect("delete session");
    let after: i64 = conn
        .query_row("SELECT count(*) FROM cleanup_item", [], |r| r.get(0))
        .unwrap();
    assert_eq!(after, 0, "删除 session 后其下 cleanup_item 必须级联删除");
}

#[test]
fn registry_backup_sets_null_on_task_delete() {
    let dir = tempfile::tempdir().expect("tempdir");
    let conn = open(dir.path()).expect("open db");

    let sid = insert_session(&conn);
    let tid = insert_task(&conn, sid);
    conn.execute(
        "INSERT INTO registry_backup (task_id, reg_path, export_file_path, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![tid, r"HKCU\Software\Test", r"C:\tmp\backup.reg", 1000i64],
    )
    .expect("insert registry_backup");

    // 删除 task：registry_backup.task_id 应 ON DELETE SET NULL，记录本身保留
    conn.execute(
        "DELETE FROM cleanup_task WHERE id = ?1",
        rusqlite::params![tid],
    )
    .expect("delete task");
    let (count, task_id): (i64, Option<i64>) = conn
        .query_row(
            "SELECT count(*), max(task_id) FROM registry_backup",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(count, 1, "任务删除后备份记录应保留");
    assert_eq!(task_id, None, "task_id 应被置 NULL 而非连带删除");
}
