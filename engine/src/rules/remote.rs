//! task-022：远端 rules_meta.json 拉取（只查版本，不下载 cache_rules.json）。
//! task-023：apply_update——下载新 cache_rules.json + SHA-256 校验 + 备份替换 + upsert DB。

use sha2::{Digest, Sha256};
use std::path::Path;

/// 远端 rules_meta.json 的硬编码地址（公共仓库匿名拉取）。
pub const REMOTE_RULES_META_URL: &str =
    "https://raw.githubusercontent.com/An0114/C-clean/main/rules/rules_meta.json";

/// 远端 cache_rules.json 的硬编码地址（与 meta 同目录）。
pub const REMOTE_RULES_URL: &str =
    "https://raw.githubusercontent.com/An0114/C-clean/main/rules/cache_rules.json";

/// 远端 rules_meta.json 的最小结构（只取比对版本需要的两个字段）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RemoteRulesMeta {
    pub version: String,
    #[serde(rename = "checksum_sha256")]
    pub checksum_sha256: String,
}

/// `rules.checkUpdate` 命令的响应体（透传给前端）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckUpdateResp {
    pub has_update: bool,
    pub current_version: String,
    pub remote_version: String,
    pub current_sha256: String,
    pub remote_sha256: String,
}

/// GET 远端 rules_meta.json 并反序列化为 [`RemoteRulesMeta`]。
///
/// 失败（网络不通 / 非 2xx / JSON 错）统一包装为 `Err(String)`，不做重试。
pub fn fetch_remote_meta() -> Result<RemoteRulesMeta, String> {
    let resp = reqwest::blocking::get(REMOTE_RULES_META_URL)
        .map_err(|e| format!("GET {REMOTE_RULES_META_URL}: {e}"))?;
    let resp = resp
        .error_for_status()
        .map_err(|e| format!("remote non-2xx: {e}"))?;
    resp.json::<RemoteRulesMeta>()
        .map_err(|e| format!("decode remote rules_meta: {e}"))
}

/// GET 远端 cache_rules.json 字节流（不解析、不校验）。
///
/// task-023：拉到字节后由后续 apply_update 算 SHA-256 与 remote_meta 比对。
pub fn fetch_remote_rules_bytes() -> Result<Vec<u8>, String> {
    let resp = reqwest::blocking::get(REMOTE_RULES_URL)
        .map_err(|e| format!("GET {REMOTE_RULES_URL}: {e}"))?;
    let resp = resp
        .error_for_status()
        .map_err(|e| format!("remote non-2xx: {e}"))?;
    resp.bytes()
        .map(|b| b.to_vec())
        .map_err(|e| format!("read body: {e}"))
}

/// 算字节流 SHA-256（小写 hex，无 `0x` 前缀）。
fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// applyUpdate 结果。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    pub new_version: String,
    pub new_sha256: String,
}

/// applyUpdate 失败原因。
#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    #[error("SHA-256 不匹配：预期 {expected}，实际 {actual}")]
    ShaMismatch { expected: String, actual: String },
    #[error("IO 错误: {0}")]
    Io(String),
    #[error("DB 错误: {0}")]
    Db(String),
    #[error("回滚失败: {0}")]
    RollbackFailed(String),
}

/// 把新 rules 字节安全替换到本地 rules_dir：
///
/// 1. 校验 `remote_rules_bytes` 的 SHA-256 必须等于 `remote_meta.checksum_sha256`（防篡改）；
/// 2. 备份现有 `cache_rules.json` / `rules_meta.json` 为 `.bak`；
/// 3. 写新文件；
/// 4. upsert `rules_meta` 单行表；
/// 5. 任一步失败 → 从 `.bak` 恢复原文件。
pub fn apply_update(
    rules_dir: &Path,
    conn: &rusqlite::Connection,
    remote_meta: &RemoteRulesMeta,
    remote_rules_bytes: &[u8],
) -> Result<ApplyResult, ApplyError> {
    let expected = remote_meta.checksum_sha256.clone();
    let actual = sha256_hex(remote_rules_bytes);
    if expected != actual {
        return Err(ApplyError::ShaMismatch { expected, actual });
    }

    let cache_path = rules_dir.join("cache_rules.json");
    let meta_path = rules_dir.join("rules_meta.json");
    let cache_bak = rules_dir.join("cache_rules.json.bak");
    let meta_bak = rules_dir.join("rules_meta.json.bak");

    // 2. 备份（不存在则跳过，写入时自然创建新文件）。
    if cache_path.exists() {
        std::fs::copy(&cache_path, &cache_bak)
            .map_err(|e| ApplyError::Io(format!("backup cache: {e}")))?;
    }
    if meta_path.exists() {
        std::fs::copy(&meta_path, &meta_bak)
            .map_err(|e| ApplyError::Io(format!("backup meta: {e}")))?;
    }

    // 3. 写新文件；失败即回滚。
    let write_result = (|| -> Result<(), ApplyError> {
        std::fs::write(&cache_path, remote_rules_bytes)
            .map_err(|e| ApplyError::Io(format!("write cache: {e}")))?;
        let meta_json = serde_json::json!({
            "version": remote_meta.version,
            "checksum_sha256": remote_meta.checksum_sha256,
            "source_url": REMOTE_RULES_URL,
            "downloaded_at": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            "custom_rules": [],
        });
        let meta_str = serde_json::to_string_pretty(&meta_json)
            .map_err(|e| ApplyError::Io(format!("encode meta: {e}")))?;
        std::fs::write(&meta_path, meta_str)
            .map_err(|e| ApplyError::Io(format!("write meta: {e}")))?;
        Ok(())
    })();

    if let Err(e) = write_result {
        rollback(&cache_path, &meta_path, &cache_bak, &meta_bak);
        return Err(e);
    }

    // 4. upsert DB；失败同样回滚文件。
    if let Err(e) = crate::rules::meta_store::upsert_active_rules_meta(
        conn,
        &remote_meta.version,
        &remote_meta.checksum_sha256,
        REMOTE_RULES_URL,
    ) {
        rollback(&cache_path, &meta_path, &cache_bak, &meta_bak);
        return Err(ApplyError::Db(e));
    }

    Ok(ApplyResult {
        new_version: remote_meta.version.clone(),
        new_sha256: actual,
    })
}

/// 从 .bak 恢复原文件（失败只在错误里体现，不 panic）。
fn rollback(cache_path: &Path, meta_path: &Path, cache_bak: &Path, meta_bak: &Path) {
    if cache_bak.exists() {
        let _ = std::fs::copy(cache_bak, cache_path);
    }
    if meta_bak.exists() {
        let _ = std::fs::copy(meta_bak, meta_path);
    }
}
