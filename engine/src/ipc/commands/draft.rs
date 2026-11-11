//! 草稿暂存（task-084 C2）：把用户勾选的待清理项持久化到 `draft_selections` 表。
//!
//! # 设计要点（对应 `v7__draft.sql` 文件头的说明）
//!
//! - **按路径存**：`cleanup_item.id` 每次扫描都是自增，重开 app 后同一文件的 id
//!   完全变了，按 id 存恢复不了；路径跨扫描稳定，是恢复回查的稳定键；
//! - **存完整快照**：每次勾选变更都重写整个集合，不做 add/remove 增量——草稿是
//!   「用户此刻选了什么」的瞬态，重写比增量简单、也永远不漂移；
//! - **即时写入 / 重开恢复 / 清理后清空** 三条语义对应 `draft_load` /
//!   `draft_clear`，前端在勾选变更、挂载、清理完成后分别调用。
//!
//! 纯逻辑层：只依赖 `rusqlite::Connection`，不依赖 Tauri / AppPaths，便于单测。

use rusqlite::{params, params_from_iter, Connection, OptionalExtension};

/// 一条草稿勾选。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftSelection {
    /// 自增主键。
    pub id: i64,
    /// 勾选发生时的扫描会话 id（可为空）。
    pub scan_id: Option<i64>,
    /// 被勾选清理项的绝对路径。
    pub item_path: String,
    /// 勾选/更新时间（unix 毫秒）。
    pub selected_at: i64,
}

/// 草稿读写的 DB 错误（薄封装，保持命令返回 `Result` 语义）。
#[derive(Debug, thiserror::Error)]
pub enum DraftError {
    /// 底层 SQLite 错误。
    #[error("数据库错误: {0}")]
    Db(#[from] rusqlite::Error),
}

/// 覆盖保存当前勾选快照：清空旧的全部，再写入本轮选中的路径。
///
/// # 为什么是「清空 + 重写」而不是逐条 upsert
///
/// 前端每次勾选变更都在本函数收到**完整集合**。逐条 upsert 会出现「上次选了又
/// 取消的那条还在表里」——因为它没在被 upsert 的集合里，就没有任何语句去删它。
/// 「清空 + 写入」天然保证表内容 == 本次勾选快照，不会残留。
///
/// `scan_id` 可为空（勾选本身不依赖某个 session），但前端通常会给，方便排查。
pub fn replace_selections(
    conn: &Connection,
    scan_id: Option<i64>,
    item_paths: &[String],
) -> Result<(), DraftError> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM draft_selections", [])?;
    for path in item_paths {
        // 路径为空串不属于「勾选了一个文件」，跳过（防御调用方 bug）。
        if path.trim().is_empty() {
            continue;
        }
        tx.execute(
            "INSERT INTO draft_selections (scan_id, item_path, selected_at)
             VALUES (?1, ?2, ?3)",
            params![scan_id, path, now_ms()],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// 读取全部草稿勾选（按 `selected_at` 升序，保持用户勾选顺序）。
pub fn load_selections(conn: &Connection) -> Result<Vec<DraftSelection>, DraftError> {
    let mut stmt = conn.prepare(
        "SELECT id, scan_id, item_path, selected_at
         FROM draft_selections ORDER BY selected_at ASC, id ASC",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(DraftSelection {
                id: r.get(0)?,
                scan_id: r.get(1)?,
                item_path: r.get(2)?,
                selected_at: r.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// 读取传入的路径集合里、确实在草稿中的（恢复时用于回查当前库是否存在）。
///
/// # 用途
///
/// 重开 app 恢复勾选时，草稿里的路径可能已在「当前扫描库里都不存在」（用户删过 /
/// 新扫描没再产出）。这个函数接收候选路径，返回在草稿中仍然存在的那些，供前端
/// 把 id 重新映射回 rows。若草稿整体被清空则返回空表。
pub fn load_existing_paths(
    conn: &Connection,
    candidates: &[String],
) -> Result<Vec<String>, DraftError> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }
    // 参数化 in-list：拼接 `?` 占位符（路径来自白名单控制的前端，数量小）。
    let placeholders = std::iter::repeat("?")
        .take(candidates.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!("SELECT item_path FROM draft_selections WHERE item_path IN ({placeholders})");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(candidates.iter()), |r| {
            r.get::<_, String>(0)
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// 清空整张草稿表（清理完成 / 用户手动取消勾选后调用）。
pub fn clear_selections(conn: &Connection) -> Result<(), DraftError> {
    conn.execute("DELETE FROM draft_selections", [])?;
    Ok(())
}

/// 当前 unix 毫秒时间戳。
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 单条路径是否在草稿中（供单元测试 / 小查询使用）。
#[allow(dead_code)]
fn contains_path(conn: &Connection, path: &str) -> Result<bool, DraftError> {
    let exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM draft_selections WHERE item_path = ?1",
            [path],
            |r| r.get(0),
        )
        .optional()
        .map_err(DraftError::from)?;
    Ok(exists.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrate::migrate;

    fn migrated() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn
    }

    #[test]
    fn draft_write_then_load_roundtrip() {
        let conn = migrated();
        replace_selections(&conn, Some(1), &["a.tmp".into(), "b.tmp".into()]).unwrap();
        let loaded = load_selections(&conn).unwrap();
        let paths: Vec<&str> = loaded.iter().map(|d| d.item_path.as_str()).collect();
        assert_eq!(paths, vec!["a.tmp", "b.tmp"]);
    }

    #[test]
    fn draft_replace_overwrites_previous() {
        // 先写 ABC，再只写 A → 表里必须只剩 A（取消的 B/C 不能残留）
        let conn = migrated();
        replace_selections(&conn, None, &["A".into(), "B".into(), "C".into()]).unwrap();
        replace_selections(&conn, None, &["A".into()]).unwrap();
        let loaded = load_selections(&conn).unwrap();
        let paths: Vec<&str> = loaded.iter().map(|d| d.item_path.as_str()).collect();
        assert_eq!(paths, vec!["A"], "重写后必须只剩本次勾选快照");
    }

    #[test]
    fn draft_clear_empties_table() {
        let conn = migrated();
        replace_selections(&conn, None, &["A".into()]).unwrap();
        clear_selections(&conn).unwrap();
        assert!(load_selections(&conn).unwrap().is_empty(), "清空后应为空");
    }

    #[test]
    fn draft_skips_empty_paths() {
        let conn = migrated();
        replace_selections(&conn, None, &["".into(), "  ".into(), "real.tmp".into()]).unwrap();
        let paths: Vec<String> = load_selections(&conn)
            .unwrap()
            .into_iter()
            .map(|d| d.item_path)
            .collect();
        assert_eq!(paths, vec!["real.tmp"], "空路径被称为空串的脏数据除外");
    }

    #[test]
    fn draft_load_existing_filters_by_candidates() {
        let conn = migrated();
        replace_selections(&conn, None, &["keep.tmp".into(), "gone.tmp".into()]).unwrap();
        // 候选里只有 keep.tmp 和别的路径；gone.tmp 不在当前候选里
        let got = load_existing_paths(&conn, &["keep.tmp".into(), "other.tmp".into()]).unwrap();
        assert_eq!(got, vec!["keep.tmp"]);
    }
}
