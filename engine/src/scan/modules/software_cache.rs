//! task-083 B2：基于「已安装软件清单」的软件缓存识别。
//!
//! # 与 5 个内置模块的区别
//!
//! 内置模块**不管软件装没装**，直接扫固定目录；本模块先拿
//! [`crate::scan::software_enum::SoftwareEntry`] 清单，只对**真的装了**的软件
//! 扫其缓存模板路径。没装 VSCode 就不该去 `%APPDATA%\Code\Cache` 白跑一趟。
//!
//! # 三道安全护栏（缺一不可）
//!
//! 1. **加载期防呆**：`software_rules.json` 里出现敏感关键词（聊天记录、
//!    浏览器凭证、配置库）或 L3 红线路径，直接拒绝加载。规则是人写的，
//!    「顺手把 `Msg` 写进路径」这种事故必须被挡在启动前；
//! 2. **扫描期逐文件判 L3**：模板路径合法不代表展开后的每个文件都合法，
//!    逐条过 `execute::whitelist::classify`（与 `user_rules` 同一口径）；
//! 3. **条数上限**：软件缓存目录可能有几十万个文件，超过 [`MAX_ITEMS_PER_RULE`]
//!    立刻停，宁可少列也不卡死。
//!
//! # 为什么 category 用 slug
//!
//! 清理方案存的是 `{module_code, category, path}` 三元组，category 必须稳定。
//! 软件名是给人看的（含空格、中文、版本号），直接当 category 会在改名后
//! 让整份方案失效。slug 只保留 ASCII 字母数字，其余转 `_`。

use serde::Deserialize;
use std::path::Path;

use crate::rules::env_expand;
use crate::scan::modules::{scan_dir, ScanItem};
use crate::scan::software_enum::SoftwareEntry;

pub const MODULE_CODE: &str = "software_cache";
/// 单条软件规则最多收多少条（与 `user_rules` 同量级）。
pub const MAX_ITEMS_PER_RULE: usize = 20_000;

/// 规则文件顶层。
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SoftwareRules {
    pub version: String,
    pub software: Vec<SoftwareRule>,
}

/// 一个软件的缓存模板。
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SoftwareRule {
    /// UI 展示用的规范名。
    pub software_name: String,
    /// 注册表 `DisplayName` 的候选（大小写/空格不敏感，子串匹配）。
    pub match_names: Vec<String>,
    /// 缓存目录模板（支持 `%APPDATA%` 等 5 个白名单变量）。
    pub cache_paths: Vec<String>,
    /// `L1` / `L2`。
    pub risk_level: String,
    pub impact_note: String,
    pub rebuild_note: String,
}

/// 加载期就要拒绝的敏感路径片段。
///
/// 这些是**用户数据**，不是缓存：浏览器凭证库、聊天记录、设置文件、DB。
/// 大小写不敏感；命中即视为「规则写错了」，不是静默跳过。
const FORBIDDEN_PATH_PARTS: &[&str] = &[
    "login data",
    "cookies",
    "settings.json",
    "filestorage",
    "\\msg\\",
    ".db",
    "credentials",
];

/// 加载并校验软件规则文件。
pub fn load(path: &Path) -> Result<SoftwareRules, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("读 {} 失败: {e}", path.display()))?;
    let rules: SoftwareRules =
        serde_json::from_slice(&bytes).map_err(|e| format!("解析 {} 失败: {e}", path.display()))?;
    validate(&rules)?;
    Ok(rules)
}

/// 规则校验：**纯函数**，单测可以直接构造规则来验防呆。
pub fn validate(rules: &SoftwareRules) -> Result<(), String> {
    for rule in &rules.software {
        if rule.software_name.trim().is_empty() {
            return Err("存在 software_name 为空的规则".to_string());
        }
        if rule.match_names.is_empty() {
            return Err(format!("规则 {} 没有 match_names", rule.software_name));
        }
        if !matches!(rule.risk_level.as_str(), "L1" | "L2") {
            return Err(format!(
                "规则 {} 的 risk_level={} 非法（只允许 L1/L2）",
                rule.software_name, rule.risk_level
            ));
        }
        for raw in &rule.cache_paths {
            let lower = raw.to_lowercase().replace('/', "\\");
            for bad in FORBIDDEN_PATH_PARTS {
                if lower.contains(bad) {
                    return Err(format!(
                        "规则 {} 的路径 {} 含敏感片段 {:?}，拒绝加载（会把用户数据当缓存删掉）",
                        rule.software_name, raw, bad
                    ));
                }
            }
            // 变量展开失败（白名单外的变量 / 未设置的变量）也要在加载期暴露
            let expanded = env_expand::expand(raw)
                .map_err(|e| format!("规则 {} 的路径 {} 无法展开: {e}", rule.software_name, raw))?;
            // L3 红线：模板根路径本身就不能落在受保护目录下
            if let Ok(n) = crate::execute::normalize::normalize(&expanded) {
                if crate::execute::whitelist::classify(&n).is_err() {
                    return Err(format!(
                        "规则 {} 的路径 {} 落在 L3 红线内，拒绝加载",
                        rule.software_name, raw
                    ));
                }
            }
        }
    }
    Ok(())
}

fn norm(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// 软件名归一的 slug（只留 ASCII 字母数字）。
pub fn category_slug(name: &str) -> String {
    let out: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let collapsed = out.trim_matches('_').to_string();
    if collapsed.is_empty() {
        "unknown".to_string()
    } else {
        collapsed
    }
}

fn risk_int(level: &str) -> i64 {
    match level {
        "L2" => 2,
        "L0" => 0,
        _ => 1,
    }
}

/// 判断这台机器上是否装了某个软件。
///
/// 用「归一后的子串」而不是精确相等：`DisplayName` 常带版本、
/// `(User)`、`(64-bit)` 这类后缀，精确相等会漏掉一大半。
pub fn is_installed(rule: &SoftwareRule, inventory: &[SoftwareEntry]) -> Option<SoftwareEntry> {
    let names: Vec<String> = rule.match_names.iter().map(|n| norm(n)).collect();
    inventory
        .iter()
        .find(|e| {
            let n = norm(&e.name);
            names.iter().any(|m| !m.is_empty() && n.contains(m))
        })
        .cloned()
}

/// 扫描已安装软件的缓存目录。
///
/// `builtin_prefixes` 是内置模块的根路径前缀：落在内置目录下的文件跳过，
/// 避免同一文件被算两遍（task-078 立的规矩，C3 去重也依赖它）。
pub fn scan(
    inventory: &[SoftwareEntry],
    rules: &SoftwareRules,
    builtin_prefixes: &[String],
) -> Vec<ScanItem> {
    let mut out = Vec::new();
    for rule in &rules.software {
        let Some(installed) = is_installed(rule, inventory) else {
            // 未安装 → 连目录都不去看，这是本模块存在的理由
            continue;
        };
        let category = format!("{MODULE_CODE}_{}", category_slug(&rule.software_name));
        let risk = risk_int(&rule.risk_level);
        let mut taken = 0usize;

        for raw in &rule.cache_paths {
            if taken >= MAX_ITEMS_PER_RULE {
                break;
            }
            let Ok(root) = env_expand::expand(raw) else {
                continue;
            };
            if !Path::new(&root).is_dir() {
                continue;
            }
            for item in scan_dir(Path::new(&root), MODULE_CODE, &category, risk) {
                // 逐文件判 L3：模板合法 ≠ 展开出来的每个文件都合法
                let Ok(n) = crate::execute::normalize::normalize(&item.path) else {
                    continue;
                };
                if crate::execute::whitelist::classify(&n).is_err() {
                    continue;
                }
                if crate::rules::loader::covered_by_builtin(&item.path, builtin_prefixes) {
                    continue;
                }
                out.push(item.with_impact(&rule.impact_note, &rule.rebuild_note));
                taken += 1;
                if taken >= MAX_ITEMS_PER_RULE {
                    eprintln!(
                        "[software_cache] {} ({}) 达到上限 {MAX_ITEMS_PER_RULE} 条，已截断",
                        rule.software_name, installed.name
                    );
                    break;
                }
            }
        }
    }
    out
}
