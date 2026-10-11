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
static MIGRATIONS: &[(u32, &str)] = &[(1, include_str!("migrations/v1__init.sql"))];

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
        tx.pragma_update(None, "user_version", &version)?;
        tx.commit()?;
        latest = *version;
    }
    Ok(latest)
}
