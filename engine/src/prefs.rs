//! 用户偏好存储（task-057）。
//!
//! 复用 v1 已有的 `settings(key,value,updated_at)` 表，不加新列、不加迁移。
//! 键名统一带 `prefs.` 前缀，避免与未来其它设置混用。
//!
//! 目前只存一个开关：
//! - `prefs.auto_restart_on_close`（`"1"` / `"0"`）：是否在 RmShutdown 之后
//!   对 bRestartable 进程调 RmRestart。**默认 false**（新用户首次读不到行时
//!   返回 false），符合产品约束「默认关闭」。
//!
//! 所有函数都是同步纯 SQLite 操作，由调用方（Tauri 命令）持有连接。

use rusqlite::Connection;

const KEY_AUTO_RESTART: &str = "prefs.auto_restart_on_close";

/// 读「关闭占用进程后自动重启」开关。
///
/// 缺行 / 值非法 → false（保守，绝不默认开启）。
pub fn get_auto_restart(conn: &Connection) -> bool {
    let v: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            rusqlite::params![KEY_AUTO_RESTART],
            |r| r.get(0),
        )
        .ok();
    matches!(v.as_deref(), Some("1"))
}

/// 写「关闭占用进程后自动重启」开关。
///
/// 用 UPSERT（INSERT OR REPLACE），updated_at 取当前 Unix 毫秒。
pub fn set_auto_restart(conn: &Connection, on: bool) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("system time before epoch: {e}"))?
        .as_millis() as i64;
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        rusqlite::params![KEY_AUTO_RESTART, if on { "1" } else { "0" }, now],
    )
    .map(|_| ())
    .map_err(|e| format!("upsert prefs: {e}"))
}

/// 给前端用的偏好快照（serde camelCase）。
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    pub auto_restart_on_close: bool,
}

impl Prefs {
    pub fn load(conn: &Connection) -> Self {
        Self {
            auto_restart_on_close: get_auto_restart(conn),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_temp() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = crate::db::conn::open(dir.path()).unwrap();
        (dir, conn)
    }

    #[test]
    fn default_is_false_when_row_missing() {
        let (_dir, conn) = open_temp();
        assert!(!get_auto_restart(&conn), "缺行必须返回 false");
    }

    #[test]
    fn set_then_get_roundtrip() {
        let (_dir, conn) = open_temp();
        assert!(!get_auto_restart(&conn));
        set_auto_restart(&conn, true).unwrap();
        assert!(get_auto_restart(&conn), "set(true) 后读应得 true");
        set_auto_restart(&conn, false).unwrap();
        assert!(!get_auto_restart(&conn), "set(false) 后读应得 false");
    }

    #[test]
    fn unknown_value_falls_back_to_false() {
        let (_dir, conn) = open_temp();
        // 直接写一个非法值，模拟历史脏数据；必须 fail-closed 到 false。
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, 'yes', 1)",
            rusqlite::params![KEY_AUTO_RESTART],
        )
        .unwrap();
        assert!(!get_auto_restart(&conn), "非法值必须回退 false");
    }

    #[test]
    fn upsert_does_not_touch_other_keys() {
        let (_dir, conn) = open_temp();
        // 别的 key 预置一个值，set_auto_restart 不能把它冲掉。
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES ('other.key', 'keep', 1)",
            [],
        )
        .unwrap();
        set_auto_restart(&conn, true).unwrap();
        let other: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key='other.key'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(other, "keep", "未知/其它 key 不能被破坏");
    }

    #[test]
    fn prefs_snapshot_roundtrip() {
        let (_dir, conn) = open_temp();
        let p = Prefs::load(&conn);
        assert!(!p.auto_restart_on_close);
        set_auto_restart(&conn, true).unwrap();
        let p2 = Prefs::load(&conn);
        assert!(p2.auto_restart_on_close);
    }
}
