//! 通知中心历史（task-086 5.8）。
//!
//! # 为什么需要应用内历史
//!
//! 自动扫描 / 清理完成时已经通过 `tauri-plugin-notification` 发系统 toast
//! （task-061）。但 Windows 通知中心只保留系统侧的那份，用户在系统设置里
//! 清掉、或专注助手把 toast 吞掉之后，应用内就再也看不到「昨晚扫到了什么」。
//!
//! 这张表把「发过的通知」按时间保留最近 [`MAX_ROWS`] 条，Settings 页可直接回看。
//! 它**不是** toast 的替代品：发送失败也照样记录，用户至少能在应用内看到。
//!
//! # 上限策略
//!
//! 每次写入后裁剪到 [`MAX_ROWS`] 条。通知是低频事件（每天最多几次），200 条
//! 足以覆盖几个月；不设上限则表会无限增长，而这条链路没人会去手动清理。

use rusqlite::{params, Connection};

/// 保留的最大条数。
pub const MAX_ROWS: usize = 200;

/// 一条通知历史（前端直接渲染）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationEntry {
    pub id: i64,
    /// 写入时间（unix 秒）。
    pub ts: i64,
    pub title: String,
    pub body: String,
    pub read: bool,
}

/// 写入一条通知，返回新行 id。
///
/// `ts` 由 SQL 的 `unixepoch()` 生成，避免调用方时钟不一致导致排序错乱。
pub fn record(conn: &Connection, title: &str, body: &str) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO notification_history (ts, title, body, read)
         VALUES (unixepoch(), ?1, ?2, 0)",
        params![title, body],
    )
    .map_err(|e| format!("insert notification_history: {e}"))?;
    let id = conn.last_insert_rowid();

    // 裁剪旧行：保留最新 MAX_ROWS 条。
    conn.execute(
        "DELETE FROM notification_history
         WHERE id NOT IN (
             SELECT id FROM notification_history ORDER BY ts DESC, id DESC LIMIT ?1
         )",
        params![MAX_ROWS as i64],
    )
    .map_err(|e| format!("trim notification_history: {e}"))?;

    Ok(id)
}

/// 按时间倒序列出最近 `limit` 条（`limit` 会被夹到 `[1, MAX_ROWS]`）。
pub fn list(conn: &Connection, limit: u32) -> Result<Vec<NotificationEntry>, String> {
    let limit = limit.clamp(1, MAX_ROWS as u32) as i64;
    let mut stmt = conn
        .prepare(
            "SELECT id, ts, title, body, read
             FROM notification_history ORDER BY ts DESC, id DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([limit], |r| {
            Ok(NotificationEntry {
                id: r.get(0)?,
                ts: r.get(1)?,
                title: r.get(2)?,
                body: r.get(3)?,
                read: r.get::<_, i64>(4)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

/// 未读条数（Settings 入口徽标用）。
pub fn unread_count(conn: &Connection) -> Result<i64, String> {
    conn.query_row(
        "SELECT count(*) FROM notification_history WHERE read = 0",
        [],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

/// 全部标记为已读，返回受影响行数。
pub fn mark_all_read(conn: &Connection) -> Result<usize, String> {
    conn.execute(
        "UPDATE notification_history SET read = 1 WHERE read = 0",
        [],
    )
    .map_err(|e| e.to_string())
}

/// 清空历史，返回删除条数。
pub fn clear(conn: &Connection) -> Result<usize, String> {
    conn.execute("DELETE FROM notification_history", [])
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrate::migrate(&conn).unwrap();
        conn
    }

    #[test]
    fn record_then_list_is_newest_first() {
        let conn = db();
        record(&conn, "自动扫描", "扫到 3 项").unwrap();
        record(&conn, "清理完成", "释放 1.0 GB").unwrap();

        let rows = list(&conn, 10).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].title, "清理完成", "最新一条应在最前");
        assert!(!rows[0].read, "新通知默认未读");
    }

    #[test]
    fn unread_count_and_mark_all_read() {
        let conn = db();
        record(&conn, "a", "").unwrap();
        record(&conn, "b", "").unwrap();
        assert_eq!(unread_count(&conn).unwrap(), 2);
        assert_eq!(mark_all_read(&conn).unwrap(), 2);
        assert_eq!(unread_count(&conn).unwrap(), 0);
    }

    #[test]
    fn clear_removes_everything() {
        let conn = db();
        record(&conn, "a", "").unwrap();
        assert_eq!(clear(&conn).unwrap(), 1);
        assert!(list(&conn, 10).unwrap().is_empty());
    }

    #[test]
    fn history_is_capped_at_max_rows() {
        let conn = db();
        // 多写 5 条，验证裁剪不会留下超过上限的行。
        for i in 0..(MAX_ROWS + 5) {
            record(&conn, "n", &i.to_string()).unwrap();
        }
        let n: i64 = conn
            .query_row("SELECT count(*) FROM notification_history", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(n as usize, MAX_ROWS);
        // 最新一条要保留（裁剪删的是最旧的）
        let rows = list(&conn, MAX_ROWS as u32).unwrap();
        assert_eq!(rows[0].body, (MAX_ROWS + 4).to_string());
    }

    #[test]
    fn list_limit_is_clamped() {
        let conn = db();
        record(&conn, "only", "").unwrap();
        assert_eq!(list(&conn, 0).unwrap().len(), 1, "limit=0 应夹到 1");
    }
}
