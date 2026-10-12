//! SHA-256 校验加载器（task-005 §E）。
//!
//! 流程严格按序：读盘 → 算 hash → 与 meta.checksum 严格比对 → 反序列化
//! → L3 检查 → 变量展开。任一失败立即返回，**不写盘、不保留半成品**——
//! 调用方负责在失败时沿用旧版规则（ARCHITECTURE §3.3：校验失败保留旧版）。

use crate::rules::env_expand::{self, ExpandError};
use crate::rules::models::{CacheRules, RulesMeta};
use sha2::{Digest, Sha256};
use std::path::Path;

/// 加载成功的规则集（rules + meta 一并交给上层）。
#[derive(Debug, Clone)]
pub struct LoadedRules {
    pub rules: CacheRules,
    pub meta: RulesMeta,
}

/// 加载失败原因。
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// 读文件失败。
    #[error("读取规则文件失败: {0}")]
    Io(#[from] std::io::Error),
    /// JSON 反序列化失败。
    #[error("规则 JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),
    /// 本地 cache_rules.json 与 meta 记录的 checksum 不一致（疑似被篡改）。
    #[error("SHA-256 校验不匹配: expected={expected}, actual={actual}")]
    ChecksumMismatch { expected: String, actual: String },
    /// 规则条目出现 `risk_level="L3"`（L3 永不写入可清理规则）。
    #[error("规则条目禁止为 L3: {0}")]
    L3InRules(String),
    /// 路径变量展开失败。
    #[error("路径变量展开失败: {0}")]
    Expand(#[from] ExpandError),
}

/// 计算字节流的 SHA-256，返回小写 hex（无 `0x` 前缀）。
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// 从 `<rules_dir>` 加载并校验规则库。
///
/// 目录约定：`cache_rules.json` 与 `rules_meta.json` 同目录。
pub fn load(rules_dir: &Path) -> Result<LoadedRules, LoadError> {
    // 1. 读两文件字节。
    let cache_bytes = std::fs::read(rules_dir.join("cache_rules.json"))?;
    let meta_bytes = std::fs::read(rules_dir.join("rules_meta.json"))?;

    // 2. 反序列化 meta（meta 结构本身简单，不参与 checksum 校验）。
    let meta: RulesMeta = serde_json::from_slice(&meta_bytes)?;

    // 3. 算 cache 的 SHA-256，与 meta 严格比对（大小写敏感，必须小写）。
    let actual = sha256_hex(&cache_bytes);
    if actual != meta.checksum_sha256 {
        return Err(LoadError::ChecksumMismatch {
            expected: meta.checksum_sha256,
            actual,
        });
    }

    // 4. 反序列化 cache。
    let rules: CacheRules = serde_json::from_slice(&cache_bytes)?;

    // 5. L3 检查：任一可清理条目 risk_level == "L3" 即拒绝。
    for entries in rules.modules.values() {
        for entry in entries {
            if entry.risk_level == "L3" {
                return Err(LoadError::L3InRules(entry.id.clone()));
            }
        }
    }

    // 6. 变量展开校验：whitelist_deny 与所有 paths 都必须能展开。
    for raw in &rules.whitelist_deny {
        env_expand::expand(raw)?;
    }
    for entries in rules.modules.values() {
        for entry in entries {
            for raw in &entry.paths {
                env_expand::expand(raw)?;
            }
        }
    }

    Ok(LoadedRules { rules, meta })
}

/// 返回展开后的外部 L3 红线绝对路径列表（task-005 §F）。
///
/// 由加载流程保证所有 `whitelist_deny` 条目均可展开，此处 unwrap 不会失败；
/// 交给 `crate::execute::whitelist::classify_with_external` 与内置红线取并集。
pub fn external_deny_list(loaded: &LoadedRules) -> Vec<String> {
    loaded
        .rules
        .whitelist_deny
        .iter()
        .filter_map(|raw| env_expand::expand(raw).ok())
        .collect()
}
