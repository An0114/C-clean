//! 连接入口：按「数据根」打开 `<data_root>\data\cdiskcleaner.db`。

use std::path::Path;

use rusqlite::Connection;

use crate::db::DbError;

/// 打开（必要时自动创建）SQLite 单文件库并完成初始化：
///
/// 1. 在 `data_root` 下自动创建 `data/` 目录（含不存在的父目录）；
/// 2. 打开 `<data_root>\data\cdiskcleaner.db`；
/// 3. 开启外键约束（`PRAGMA foreign_keys = ON`）与 WAL 日志模式；
/// 4. （见 `migrate.rs`）连接打开后自动执行未应用的 schema 迁移，重复打开幂等。
///
/// 路径全部由参数注入：**不写死任何用户目录**（如 `C:\Users\...`），
/// 也**不读环境变量**——安装版 / 便携版路径切换由上层 Tauri 壳决定，
/// 本模块只吃调用方算好的 `data_root`。
pub fn open(data_root: &Path) -> Result<Connection, DbError> {
    // TRD §8：库文件固定位于「数据根\data\cdiskcleaner.db」，
    // 让调用方只传「数据根」，本函数自己拼 `data\` 后缀。
    let data_dir = data_root.join("data");
    std::fs::create_dir_all(&data_dir)?;

    let db_path = data_dir.join("cdiskcleaner.db");
    let conn = Connection::open(&db_path)?;

    // SQLite 外键约束默认关闭，必须显式开启（TRD §4：外键开启）。
    conn.pragma_update(None, "foreign_keys", &true)?;
    // WAL：桌面单用户场景下读写并发与崩溃恢复更友好。
    conn.pragma_update(None, "journal_mode", &"WAL")?;

    // schema 迁移在任务 002 的迁移器落地后串入（见 migrate.rs）。

    Ok(conn)
}
