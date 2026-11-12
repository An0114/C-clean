//! 基于 `PRAGMA user_version` 的手写迁移器（不引入 refinery / sqlx::migrate!）。
//!
//! 迁移文件按 `v{N}__*.sql` 命名，并以 `include_str!` 在编译期内嵌进二进制，
//! 运行时不做任何文件系统查找，也不对路径 / 表名做字符串插值（ARCHITECTURE §A03）。
//!
//! 语义：
//! - 读当前 `user_version = N`；
//! - 依次执行所有「版本号 > N」的迁移，每个迁移单独一个事务；
//! - 某迁移执行成功 → 同事务内 `PRAGMA user_version = 新版本` 并 COMMIT；
//! - 任一迁移失败 → 事务回滚、`user_version` 不变，库停留在旧 schema；
//! - 重复打开同一库时 `user_version` 已是最新，不再执行任何 SQL（幂等）。

use rusqlite::Connection;

use crate::db::DbError;

/// 已登记迁移表：(版本号, SQL 内容)。
///
/// 后续任务新增 `v2__xxx.sql` 时，在本表末尾追加一行即可；
/// 表内顺序即版本升序，等价于「按文件名字典序」的语义。
///
/// 版本历史：
/// - v1（task-002）：九张表初始 schema；
/// - v2（task-056）：`safe_box` 加 `restored_at` 审计列（restore 改为 UPDATE 不再 DELETE）；
/// - v3（task-077）：`cleanup_item` 加 `drive_letter` 派生列（盘符）+ 组合索引；
/// - v4（task-078）：新增 `user_rules` 表（用户自定义清理规则）；
/// - v5（task-083）：新增 `software_inventory` 表（软件清单缓存，B1 落地）。
/// - v6（task-084）：新增 `bigfiles_target_space` + `duplicate_files` 表（大文件地图 v2 缓存）。
/// - v7（task-084）：新增 `draft_selections` 表（草稿暂存，用户勾选跨会话恢复）。
/// - v8（task-086）：`audit_log` 加 `session_id`，支持按扫描会话筛选。
///
/// # 为什么 v3 不是任务书说的 v2
///
/// 任务书写 `v2__drive_letter.sql`，但 v2 已被 task-056 占用。`migrate()` 只执行
/// 「版本号 > 当前 user_version」的迁移，两个 v2 里后一个永远不会跑——库停在
/// 「有 restored_at、没有 drive_letter」的状态，且没有任何报错。
static MIGRATIONS: &[(u32, &str)] = &[
    (1, include_str!("migrations/v1__init.sql")),
    (2, include_str!("migrations/v2__safe_box_restored_at.sql")),
    (3, include_str!("migrations/v3__drive_letter.sql")),
    (4, include_str!("migrations/v4__user_rules.sql")),
    (5, include_str!("migrations/v5__software_awareness.sql")),
    (6, include_str!("migrations/v6__bigfiles_v2.sql")),
    (7, include_str!("migrations/v7__draft.sql")),
    (8, include_str!("migrations/v8__audit_log_session.sql")),
];

/// 对连接执行所有「版本号 > 当前 user_version」的迁移，返回迁移后的最新版本号。
pub fn migrate(conn: &Connection) -> Result<u32, DbError> {
    let current: u32 = conn.pragma_query_value(None, "user_version", |row| {
        row.get::<_, i64>(0).map(|v| v as u32)
    })?;

    let mut latest = current;
    for (version, sql) in MIGRATIONS.iter().filter(|(v, _)| *v > current) {
        // 单迁移独立事务：失败即整张回滚，user_version 保持不变。
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", version)?;
        tx.commit()?;
        latest = *version;
    }
    Ok(latest)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 开一个内存库并迁移到最新。
    fn migrated() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn
    }

    /// 插一条 cleanup_item（只填 NOT NULL 必需字段）。
    ///
    /// 先补一条 `scan_session`：`cleanup_item.session_id` 有外键约束，
    /// 直接插 1 会 `FOREIGN KEY constraint failed`（787）。
    fn insert(conn: &Connection, path: &str) {
        conn.execute(
            "INSERT OR IGNORE INTO scan_session (id, started_at, status, scope)
             VALUES (1, 1000, 'running', 'full')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO cleanup_item (session_id, module_code, category, path, size_bytes, risk_level, description)
             VALUES (1, 'temp', 'user_temp', ?1, 100, 'L1', '')",
            [path],
        )
        .unwrap();
    }

    #[test]
    fn migrate_reaches_latest() {
        // 版本号是「迁移真的跑到最后一个」的唯一硬证据。
        // 加新迁移时**必须**同步这里——它是最容易忘、也最容易暴露「迁移没生效」的一条。
        let conn = migrated();
        let v: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, 8, "最新版本应为 v8（audit_log.session_id）");
    }

    #[test]
    fn v8_adds_audit_log_session_column_and_index() {
        let conn = migrated();
        let cols: Vec<String> = conn
            .prepare("PRAGMA table_info(audit_log)")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(cols.iter().any(|c| c == "session_id"));

        let indexes: Vec<String> = conn
            .prepare("PRAGMA index_list(audit_log)")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(indexes.iter().any(|i| i == "idx_audit_log_session"));
    }

    #[test]
    fn v3_adds_drive_letter_column() {
        let conn = migrated();
        // **必须是 table_xinfo**：`PRAGMA table_info` 不列出生成列（无论
        // VIRTUAL 还是 STORED），用它断言「列存在」会永远失败。
        let mut stmt = conn.prepare("PRAGMA table_xinfo(cleanup_item)").unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(
            cols.iter().any(|c| c == "drive_letter"),
            "cleanup_item 应有 drive_letter 列，实际: {cols:?}"
        );
    }

    #[test]
    fn v6_adds_bigfiles_tables() {
        let conn = migrated();
        let names: Vec<String> = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type='table' \
                 AND name IN ('bigfiles_target_space','duplicate_files') ORDER BY name",
            )
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert_eq!(
            names,
            vec![
                "bigfiles_target_space".to_string(),
                "duplicate_files".to_string()
            ],
            "v6 应建 bigfiles_target_space + duplicate_files 两张表"
        );

        // duplicate_files 的 is_keep 默认 0（保守策略，不自动删）。
        // pragma_table_info.dflt_value 以字符串形式返回默认值（如 "0" / "1"）。
        let dflt: Option<String> = conn
            .query_row(
                "SELECT dflt_value FROM pragma_table_info('duplicate_files') WHERE name='is_keep'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            dflt.as_deref(),
            Some("0"),
            "duplicate_files.is_keep 默认值应为 0"
        );
    }

    #[test]
    fn v3_creates_drive_letter_index() {
        let conn = migrated();
        let mut stmt = conn.prepare("PRAGMA index_list(cleanup_item)").unwrap();
        let idx: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(
            idx.iter().any(|n| n == "idx_cleanup_item_drive_letter"),
            "缺少 drive_letter 索引，实际: {idx:?}"
        );
    }

    #[test]
    fn drive_letter_is_derived_from_path() {
        // 生成列必须自动算出来，不需要（也不允许）写入方提供
        let conn = migrated();
        for (path, want) in [
            (r"C:/Users/An/AppData/Local/Temp/a.tmp", Some("C")),
            (r"d:/games/b.bin", Some("D")),
            (r"c:/slash/style", Some("C")),
            // canonicalize 的长路径前缀也要认（与 Rust 侧 drive_letter_of 对齐）
            (r"\\?\E:/Windows/Temp/c.tmp", Some("E")),
            // UNC / 相对路径没有盘符 → NULL，不能变成 '\' 或 ''
            (r"\\server\share\file", None),
            (r"relative\dir\f", None),
        ] {
            insert(&conn, path);
            let got: Option<String> = conn
                .query_row(
                    "SELECT drive_letter FROM cleanup_item WHERE path = ?1",
                    [path],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(
                got.as_deref(),
                want,
                "path={path:?} 的 drive_letter 应为 {want:?}"
            );
        }
    }

    #[test]
    fn drive_letter_matches_rust_helper() {
        // 两处判定必须逐条一致：DB 生成列 ↔ scan::drive_enum::drive_letter_of。
        // 不一致的表现是「Rust 过滤时认为在 D 盘、DB 里记成 C 盘」，最难查的那种。
        let conn = migrated();
        for path in [
            r"C:/a",
            r"z:/b",
            r"\\?\F:/c",
            r"\\server\share\d",
            r"relative\e",
        ] {
            insert(&conn, path);
            let db: Option<String> = conn
                .query_row(
                    "SELECT drive_letter FROM cleanup_item WHERE path = ?1",
                    [path],
                    |r| r.get(0),
                )
                .unwrap();
            let rust = crate::scan::drive_enum::drive_letter_of(path).map(|c| c.to_string());
            assert_eq!(db, rust, "path={path:?}：DB 与 Rust 判定不一致");
        }
    }

    #[test]
    fn migrate_is_idempotent() {
        // 重复打开同一库不该重复执行迁移（ALTER TABLE 第二次会报「列已存在」）。
        // task-086：v8 已加入（audit_log.session_id），最新版本升到 8。
        let conn = migrated();
        assert_eq!(migrate(&conn).unwrap(), 8);
        assert_eq!(migrate(&conn).unwrap(), 8);
    }

    #[test]
    fn v3_does_not_break_existing_inserts() {
        // 生成列不能被写入方显式赋值；既有 INSERT（不列 drive_letter）必须照常成功
        let conn = migrated();
        insert(&conn, r"C:/x");
        let n: i64 = conn
            .query_row("SELECT count(*) FROM cleanup_item", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    // ===== task-078：v4 user_rules =====

    #[test]
    fn v4_creates_user_rules_table() {
        let conn = migrated();
        let mut stmt = conn.prepare("PRAGMA table_xinfo(user_rules)").unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        for want in [
            "id",
            "name",
            "pattern",
            "risk_level",
            "enabled",
            "created_at",
            "updated_at",
        ] {
            assert!(
                cols.iter().any(|c| c == want),
                "user_rules 缺列 {want}: {cols:?}"
            );
        }
    }

    #[test]
    fn v4_rejects_l3_and_bad_enabled_at_db_level() {
        // L3 = 红线，库层面直接挡死：即使绕过 IPC 直连 DB 也写不进去
        let conn = migrated();
        let bad_l3 = conn.execute(
            "INSERT INTO user_rules (name, pattern, risk_level, created_at, updated_at)
             VALUES ('x', 'D:\\a\\*', 'L3', 1, 1)",
            [],
        );
        assert!(bad_l3.is_err(), "L3 规则必须被 CHECK 拒绝");

        let bad_enabled = conn.execute(
            "INSERT INTO user_rules (name, pattern, risk_level, enabled, created_at, updated_at)
             VALUES ('x', 'D:\\b\\*', 'L1', 2, 1, 1)",
            [],
        );
        assert!(bad_enabled.is_err(), "enabled 只能是 0/1");
    }

    #[test]
    fn v4_pattern_is_unique() {
        // 同一模式两条规则会让同一批文件被扫两遍（条目翻倍、大小重复计）
        let conn = migrated();
        let ins = |n: &str| {
            conn.execute(
                "INSERT INTO user_rules (name, pattern, risk_level, created_at, updated_at)
                 VALUES (?1, 'D:\\Same\\*', 'L1', 1, 1)",
                [n],
            )
        };
        assert!(ins("a").is_ok());
        assert!(ins("b").is_err(), "重复 pattern 必须被唯一索引拒绝");
    }

    // ===== task-083：v5 software_inventory =====

    #[test]
    fn v5_creates_software_inventory_table() {
        let conn = migrated();
        let mut stmt = conn
            .prepare("PRAGMA table_xinfo(software_inventory)")
            .unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        for want in [
            "id",
            "name",
            "version",
            "publisher",
            "install_location",
            "registry_key",
            "detected_at",
        ] {
            assert!(
                cols.iter().any(|c| c == want),
                "software_inventory 缺列 {want}: {cols:?}"
            );
        }
    }

    #[test]
    fn v5_name_index_and_unique_registry_key() {
        let conn = migrated();

        let mut idx: Vec<String> = conn
            .prepare("PRAGMA index_list(software_inventory)")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        idx.sort();
        assert_eq!(
            idx,
            vec![
                "idx_software_inventory_name".to_string(),
                "idx_software_inventory_registry_key".to_string()
            ]
        );

        // registry_key 唯一：同一软件（同出处）只允许一条
        let ins = |k: &str| {
            conn.execute(
                "INSERT INTO software_inventory (name, registry_key, detected_at)
                 VALUES (?1, 'HKLM\\...\\x', 1000)",
                [k],
            )
        };
        assert!(ins("A").is_ok());
        assert!(ins("B").is_err(), "重复 registry_key 必须被唯一索引拒绝");
    }

    // ===== task-084 C2：v7 draft_selections =====

    #[test]
    fn v7_creates_draft_selections_table() {
        let conn = migrated();
        let mut stmt = conn
            .prepare("PRAGMA table_xinfo(draft_selections)")
            .unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        for want in ["id", "scan_id", "item_path", "selected_at"] {
            assert!(
                cols.iter().any(|c| c == want),
                "draft_selections 缺列 {want}: {cols:?}"
            );
        }
    }

    #[test]
    fn v7_item_path_is_unique() {
        // 同一路径只允许一行（upsert 语义），重复插入必须被唯一索引拒绝
        let conn = migrated();
        let ins = |p: &str| {
            conn.execute(
                "INSERT INTO draft_selections (item_path, selected_at) VALUES (?1, 1000)",
                [p],
            )
        };
        assert!(ins(r"C:\Users\A\Temp\x.tmp").is_ok());
        assert!(
            ins(r"C:\Users\A\Temp\x.tmp").is_err(),
            "重复 item_path 必须被唯一索引拒绝"
        );
    }
}
