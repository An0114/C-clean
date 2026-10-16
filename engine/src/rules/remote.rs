//! task-022：远端 rules_meta.json 拉取（只查版本，不下载 cache_rules.json）。
//! task-023：apply_update——下载新 cache_rules.json + SHA-256 校验 + 备份替换 + upsert DB。
//! task-041：reqwest 统一加 10s 超时 + 5s 连接超时，错误映射为中文可读文案；
//!          超时常量可通过环境变量 `RULES_HTTP_TIMEOUT_SECS` / `RULES_HTTP_CONNECT_SECS`
//!          覆盖，便于单测用短超时替身（默认 10s / 5s）。

use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Duration;

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

/// task-042：checkUpdate 三态状态。
///
/// - `UpToDate`：远端可达且本地已是最新；
/// - `NewVersion`：远端可达且有新版本；
/// - `Offline`：远端不可达（超时/404/连接失败），本地规则库照常工作。
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckStatus {
    UpToDate,
    NewVersion,
    Offline,
}

/// task-042：checkUpdate 统一返回结构（网络失败不再返回错误信封）。
///
/// 网络失败时返回 `status=Offline`，前端据此显示黄条
/// 「离线模式：使用本地规则库 vX.Y.Z」，本地 bundled cache_rules 照常工作。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckUpdateResult {
    pub status: CheckStatus,
    pub local_version: String,
    pub local_sha256: String,
    pub remote_version: Option<String>,
    pub remote_sha256: Option<String>,
    /// 供前端直接展示的中文文案。
    pub message: String,
}

impl CheckUpdateResult {
    /// 远端可达 + 本地最新。
    pub fn up_to_date(
        local_version: String,
        local_sha: String,
        remote: &RemoteRulesMeta,
    ) -> Self {
        Self {
            status: CheckStatus::UpToDate,
            local_version,
            local_sha256: local_sha,
            remote_version: Some(remote.version.clone()),
            remote_sha256: Some(remote.checksum_sha256.clone()),
            message: "规则库已是最新".to_string(),
        }
    }

    /// 远端可达 + 有新版本。
    pub fn new_version(
        local_version: String,
        local_sha: String,
        remote: &RemoteRulesMeta,
    ) -> Self {
        Self {
            status: CheckStatus::NewVersion,
            local_version,
            local_sha256: local_sha,
            remote_version: Some(remote.version.clone()),
            remote_sha256: Some(remote.checksum_sha256.clone()),
            message: format!("发现新版本 v{}", remote.version),
        }
    }

    /// 远端不可达，离线模式。
    pub fn offline(local_version: String, local_sha: String, err: String) -> Self {
        Self {
            status: CheckStatus::Offline,
            local_version,
            local_sha256: local_sha,
            remote_version: None,
            remote_sha256: None,
            message: err,
        }
    }
}

/// 读取超时常量（默认 10s，单测用 `RULES_HTTP_TIMEOUT_SECS` 覆盖为 1s 级）。
fn timeout_secs() -> u64 {
    std::env::var("RULES_HTTP_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10)
}

/// 读取连接超时常量（默认 5s，单测用 `RULES_HTTP_CONNECT_SECS` 覆盖）。
fn connect_timeout_secs() -> u64 {
    std::env::var("RULES_HTTP_CONNECT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5)
}

/// 构造带超时的 reqwest blocking client。
fn build_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs()))
        .connect_timeout(Duration::from_secs(connect_timeout_secs()))
        .build()
        .map_err(|e| format!("init http client: {e}"))
}

/// 把 reqwest 错误映射为中文可读文案（前端红条直接展示）。
///
/// - 超时 → "规则库检查超时（10s），请检查网络后重试"
/// - 连接失败 → "无法连接规则库服务器，请检查网络"
/// - 404 → "规则库元文件不存在（404），请联系开发者"
/// - 其他 → reqwest error 原文前 200 字
fn map_http_error(e: reqwest::Error) -> String {
    let debug = format!("{e:?}");
    if e.is_timeout() || debug.contains("operation timed out") || debug.contains("timeout") {
        return format!("规则库检查超时（{}s），请检查网络后重试", timeout_secs());
    }
    if e.is_connect() {
        return "无法连接规则库服务器，请检查网络".to_string();
    }
    if let Some(status) = e.status() {
        if status == reqwest::StatusCode::NOT_FOUND {
            return "规则库元文件不存在（404），请联系开发者".to_string();
        }
        return format!("规则库服务器返回错误：HTTP {status}");
    }
    let raw = format!("{e}");
    format!("规则库检查失败：{}", &raw[..raw.len().min(200)])
}

/// 带超时的 GET，错误已映射为中文文案。
fn http_get(url: &str) -> Result<reqwest::blocking::Response, String> {
    let client = build_client()?;
    let resp = client.get(url).send().map_err(map_http_error)?;
    resp.error_for_status().map_err(map_http_error)
}

/// GET 远端 rules_meta.json 并反序列化为 [`RemoteRulesMeta`]。
///
/// 失败（网络不通 / 非 2xx / JSON 错）统一包装为 `Err(String)`，不做重试。
pub fn fetch_remote_meta() -> Result<RemoteRulesMeta, String> {
    fetch_remote_meta_with_url(REMOTE_RULES_META_URL)
}

/// 按指定 URL 拉 rules_meta（单测用本地 tiny server 注入）。
pub fn fetch_remote_meta_with_url(url: &str) -> Result<RemoteRulesMeta, String> {
    let resp = http_get(url)?;
    resp.json::<RemoteRulesMeta>()
        .map_err(|e| format!("解析远端 rules_meta 失败：{e}"))
}

/// GET 远端 cache_rules.json 字节流（不解析、不校验）。
///
/// task-023：拉到字节后由后续 apply_update 算 SHA-256 与 remote_meta 比对。
pub fn fetch_remote_rules_bytes() -> Result<Vec<u8>, String> {
    fetch_remote_rules_bytes_with_url(REMOTE_RULES_URL)
}

/// 按指定 URL 拉 cache_rules 字节（单测用）。
pub fn fetch_remote_rules_bytes_with_url(url: &str) -> Result<Vec<u8>, String> {
    let resp = http_get(url)?;
    resp.bytes()
        .map(|b| b.to_vec())
        .map_err(|e| format!("读取远端 rules 字节失败：{e}"))
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
