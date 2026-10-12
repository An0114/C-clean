//! `cache_rules.json` / `rules_meta.json` 的 serde 模型（task-005 §C）。
//!
//! 字段名 snake_case 与 JSON 严格对齐；`risk_level` 本任务保持字符串，
//! loader 侧负责识别 `"L3"` 并拒绝。

use serde::Deserialize;
use std::collections::HashMap;

/// 初始规则库顶层结构：版本号 + 六大模块（A~F）条目表 + 外部红线表。
#[derive(Debug, Clone, Deserialize)]
pub struct CacheRules {
    /// 语义化版本，如 `"1.0.0"`。
    pub version: String,
    /// 模块键（"A".."F"）→ 该模块下的清理条目列表。
    pub modules: HashMap<String, Vec<RuleEntry>>,
    /// 外部 L3 红线（路径列表），加载后与引擎内置红线取并集。
    pub whitelist_deny: Vec<String>,
}

/// 单条清理规则（对应 TRD §4.10 的一条模块条目）。
#[derive(Debug, Clone, Deserialize)]
pub struct RuleEntry {
    /// 稳定 id，如 `"npm_cache"`。
    pub id: String,
    /// 业务分类，如 `"npm_cache"`。
    pub category: String,
    /// 展示名（中文）。
    pub display_name: String,
    /// 待清理路径列表（可能含白名单环境变量 / `~`）。
    pub paths: Vec<String>,
    /// 风险等级字符串 `"L0"/"L1"/"L2"`；`"L3"` 不允许出现在可清理规则里。
    pub risk_level: String,
    /// 对用户展示的描述。
    pub description: String,
    /// 删除后影响说明。
    pub impact_note: String,
    /// 重建方式说明（自动重建 / 需重装等）。
    pub rebuild_note: String,
    /// 删除是否需要管理员提权。
    pub elevated: bool,
}

/// `rules_meta.json` 元数据（与 `rules_meta` 表字段一一对应）。
#[derive(Debug, Clone, Deserialize)]
pub struct RulesMeta {
    pub version: String,
    /// `cache_rules.json` 字节的 SHA-256 小写 hex（无 `0x` 前缀）。
    pub checksum_sha256: String,
    /// 实际拉取地址（本任务不联网，仅记录来源）。
    pub source_url: String,
    /// 拉取时间戳；初始静态文件为 0。
    pub downloaded_at: i64,
    /// 用户自定义规则数组，本任务不展开。
    pub custom_rules: serde_json::Value,
}
