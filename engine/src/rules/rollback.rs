//! task-044：rules 手动回滚——从 `cache_rules.json.bak` 恢复上一版规则库。
//!
//! 与 apply_update 配套：apply_update 在写新文件前会把现有
//! `cache_rules.json` / `rules_meta.json` 备份为 `.bak`；
//! 本模块提供「一键回滚到上一版」的纯逻辑（无网络、无 UI）。
//!
//! 偏差：任务书写「读 .bak 的 version」，但 .bak 是 cache_rules.json，
//! version 字段在 CacheRules 模型里——所以直接从 .bak JSON 读 version。
//! rules_meta.json.bak 同步恢复，但 DB upsert 用 .bak 的 cache sha 与 version。

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::rules::meta_store::upsert_active_rules_meta;
use crate::rules::models::CacheRules;

/// 回滚结果。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackResult {
    /// 回滚到的上一版版本号（来自 .bak 的 cache_rules.json.version）。
    pub previous_version: String,
    /// 回滚后 cache_rules.json 字节的 SHA-256。
    pub restored_sha256: String,
    /// 人类可读结果文案。
    pub message: String,
}

/// cache_rules.json 路径（与 apply_update 复用同一相对路径）。
pub fn cache_path(rules_dir: &Path) -> PathBuf {
    rules_dir.join("cache_rules.json")
}

/// cache_rules.json.bak 路径（与 apply_update 复用同一相对路径）。
pub fn cache_bak_path(rules_dir: &Path) -> PathBuf {
    rules_dir.join("cache_rules.json.bak")
}

/// rules_meta.json 路径。
pub fn meta_path(rules_dir: &Path) -> PathBuf {
    rules_dir.join("rules_meta.json")
}

/// rules_meta.json.bak 路径。
pub fn meta_bak_path(rules_dir: &Path) -> PathBuf {
    rules_dir.join("rules_meta.json.bak")
}

/// 是否存在可回滚的备份（即 apply_update 是否成功跑过一次）。
pub fn has_backup(rules_dir: &Path) -> bool {
    cache_bak_path(rules_dir).exists()
}

/// 从 .bak 恢复 cache_rules.json 到 rules_dir，并 upsert DB。
///
/// 失败返回中文错误文案：
/// - `没有可回滚的备份（首次更新前）`
/// - `备份文件损坏：...`
/// - 其他 Io/DB 错误。
pub fn rollback(rules_dir: &Path, conn: &Connection) -> Result<RollbackResult, String> {
    let bak = cache_bak_path(rules_dir);
    if !bak.exists() {
        return Err("没有可回滚的备份（首次更新前）".to_string());
    }

    // 1. 读 .bak 字节并校验格式（必须是合法的 CacheRules JSON）。
    let bytes = std::fs::read(&bak).map_err(|e| format!("读取备份文件失败: {e}"))?;
    let cache: CacheRules = serde_json::from_slice(&bytes)
        .map_err(|e| format!("备份文件损坏：不是合法的 cache_rules.json: {e}"))?;
    let restored_sha256 = sha256_hex(&bytes);
    let previous_version = cache.version.clone();

    // 2. 把 .bak 内容覆盖回 cache_rules.json。
    let cache_target = cache_path(rules_dir);
    std::fs::write(&cache_target, &bytes)
        .map_err(|e| format!("写回 cache_rules.json 失败: {e}"))?;

    // 3. 同步恢复 rules_meta.json.bak（若存在）。失败只告警，不阻断主流程。
    let mbak = meta_bak_path(rules_dir);
    if mbak.exists() {
        if let Err(e) = std::fs::copy(&mbak, meta_path(rules_dir)) {
            eprintln!("[rules_rollback] restore meta bak failed: {e}");
        }
    }

    // 4. upsert DB rules_meta（用 .bak 的 version + sha）。
    upsert_active_rules_meta(conn, &previous_version, &restored_sha256, "local:rollback")
        .map_err(|e| format!("upsert rules_meta 失败: {e}"))?;

    Ok(RollbackResult {
        previous_version,
        restored_sha256,
        message: format!("已回滚到 v{}", cache.version),
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}
