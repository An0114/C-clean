//! task-022：远端 rules_meta.json 拉取（只查版本，不下载 cache_rules.json）。

/// 远端 rules_meta.json 的硬编码地址（公共仓库匿名拉取）。
pub const REMOTE_RULES_META_URL: &str =
    "https://raw.githubusercontent.com/An0114/C-clean/main/rules/rules_meta.json";

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
