//! 公共数据层：SQLite 单文件连接入口、schema 迁移、表结构定义。
//!
//! 本模块只交付「能开库、表都在、约束生效、迁移可重放」这一层；
//! 业务 CRUD 仓储方法由后续任务（003 / 004 等）逐步落地，本任务不提前写。

pub mod conn;

/// 数据层统一错误类型。
///
/// 任务约束本任务不引入 `thiserror`，故手工实现 `Display` / `Error`。
#[derive(Debug)]
pub enum DbError {
    /// 数据目录创建等文件 I/O 失败
    Io(std::io::Error),
    /// rusqlite 打开连接 / 执行 SQL / PRAGMA 等操作失败
    Sqlite(rusqlite::Error),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::Io(e) => write!(f, "db io error: {e}"),
            DbError::Sqlite(e) => write!(f, "db sqlite error: {e}"),
        }
    }
}

impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DbError::Io(e) => Some(e),
            DbError::Sqlite(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for DbError {
    fn from(e: std::io::Error) -> Self {
        DbError::Io(e)
    }
}

impl From<rusqlite::Error> for DbError {
    fn from(e: rusqlite::Error) -> Self {
        DbError::Sqlite(e)
    }
}
