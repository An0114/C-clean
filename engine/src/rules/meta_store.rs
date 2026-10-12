//! task-022：rules_meta 单行表（id=1）的 upsert / get_active。
//!
//! 与任务书 §A 的偏差：实际 v1__init.sql 的 `rules_meta` 是 CHECK(id=1) 单行表，
//! 列名为 `version / checksum_sha256 / source_url / downloaded_at / custom_rules`，
//! 没有 `is_active` 列。upsert 直接 `INSERT OR REPLACE INTO ... WHERE id=1`。

use rusqlite::Connection;

/// 当前生效的规则元数据行（单行表）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RulesMetaRow {
    pub version: String,
    pub checksum_sha256: String,
    pub source_url: String,
    pub downloaded_at: i64,
}

/// 把当前本地 rules_meta 写入单行表（id=1），幂等。
///
/// - `source_url` 暂用本地文件占位（远端 URL 在 fetch_remote_meta 里硬编码）；
/// - `downloaded_at` 取当前 unix 时间戳。
pub fn upsert_active_rules_meta(
    conn: &Connection,
    version: &str,
    checksum_sha256: &str,
    source_url: &str,
) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    conn.execute(
        "INSERT OR REPLACE INTO rules_meta (id, version, checksum_sha256, source_url, downloaded_at)
         VALUES (1, ?1, ?2, ?3, ?4)",
        rusqlite::params![version, checksum_sha256, source_url, now],
    )
    .map_err(|e| format!("upsert_active_rules_meta: {e}"))?;
    Ok(())
}

/// 读取当前生效的规则元数据（单行表；空表返 None）。
pub fn get_active_rules_meta(conn: &Connection) -> Result<Option<RulesMetaRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT version, checksum_sha256, source_url, downloaded_at FROM rules_meta WHERE id = 1",
        )
        .map_err(|e| format!("get_active_rules_meta prepare: {e}"))?;
    let mut rows = stmt
        .query([])
        .map_err(|e| format!("get_active_rules_meta query: {e}"))?;
    match rows.next().map_err(|e| format!("get_active_rules_meta next: {e}"))? {
        Some(r) => Ok(Some(RulesMetaRow {
            version: r.get(0).map_err(|e| format!("version: {e}"))?,
            checksum_sha256: r.get(1).map_err(|e| format!("checksum_sha256: {e}"))?,
            source_url: r.get(2).map_err(|e| format!("source_url: {e}"))?,
            downloaded_at: r.get(3).map_err(|e| format!("downloaded_at: {e}"))?,
        })),
        None => Ok(None),
    }
}
