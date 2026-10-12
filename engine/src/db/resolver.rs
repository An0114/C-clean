//! DB 版 `ItemPathResolver`：按 item_ids 从 `cleanup_item` 表取路径/大小/模块。
//!
//! trait 签名（task-004）是无 Result 的：DB 查询出错（如表不存在）时返回空 Vec，
//! 并通过 eprintln! 留临时日志，不 panic、不 propagrate。

use rusqlite::{params, Connection};

use crate::ipc::commands::dry_run::{ItemPathResolver, ResolvedItem};

/// 持有 `&rusqlite::Connection` 的 DB 版条目解析器（零拷贝借用，生命周期由调用方保证）。
pub struct DbItemPathResolver<'conn> {
    conn: &'conn Connection,
}

impl<'conn> DbItemPathResolver<'conn> {
    /// 用一个已迁移过的连接构造 resolver。
    pub fn new(conn: &'conn Connection) -> Self {
        Self { conn }
    }
}

impl<'conn> ItemPathResolver for DbItemPathResolver<'conn> {
    fn resolve(&self, item_ids: &[i64]) -> Vec<ResolvedItem> {
        // 单条参数化查询，禁止字符串拼接 ids。
        let mut stmt = match self.conn.prepare(
            "SELECT id, module_code, category, path, size_bytes
             FROM cleanup_item
             WHERE id = ?1",
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[db_resolver] prepare failed: {e}");
                return Vec::new();
            }
        };

        let mut out = Vec::with_capacity(item_ids.len());
        for id in item_ids {
            // 查不到的 id 直接跳过；SQL 失败也跳过，不中断其余 id。
            let row = stmt.query_row(params![id], |r| {
                Ok(ResolvedItem {
                    item_id: r.get(0)?,
                    module_code: r.get(1)?,
                    category: r.get(2)?,
                    path: r.get(3)?,
                    // size_bytes 在表里是 INTEGER(i64)，本项目 size≥0，按 u64 收。
                    size_bytes: r.get::<_, i64>(4).map(|v| v.max(0) as u64)?,
                })
            });
            match row {
                Ok(item) => out.push(item),
                Err(rusqlite::Error::QueryReturnedNoRows) => {}
                Err(e) => eprintln!("[db_resolver] query_row id={id} failed: {e}"),
            }
        }
        out
    }
}
