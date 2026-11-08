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
    /// task-083 反向断言：L1/L2 规则的 paths 落在 L3 红线目录下。
    ///
    /// 与 `L3InRules` 的区别：那条是「规则自己标了 L3」，这条是「规则标了 L1
    /// 但路径指向受保护目录」——后者更危险，因为它能绕过分级检查被真的删掉。
    #[error("L3 红线反向断言失败（{0} 条冲突）: {1:?}")]
    L3PathConflict(usize, Vec<String>),
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

/// 把 CRLF（`\r\n`）归一为 LF（`\n`）。
///
/// `rules_meta.checksum_sha256` 存的是 LF 字节流的哈希；而 Windows 上
/// git `autocrlf` 或编辑器可能把磁盘文件写成 CRLF。这里与 CI 守卫步骤
/// 做同样的归一，保证运行时校验口径与存储口径一致，不因行尾差异误判篡改。
fn to_lf(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            out.push(b'\n');
            i += 2;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

/// 从 `<rules_dir>` 加载并校验规则库。
///
/// 目录约定：`cache_rules.json` 与 `rules_meta.json` 同目录。
pub fn load(rules_dir: &Path) -> Result<LoadedRules, LoadError> {
    // 1. 读两文件字节。
    let cache_raw = std::fs::read(rules_dir.join("cache_rules.json"))?;
    let meta_bytes = std::fs::read(rules_dir.join("rules_meta.json"))?;

    // 2. 反序列化 meta（meta 结构本身简单，不参与 checksum 校验）。
    let meta: RulesMeta = serde_json::from_slice(&meta_bytes)?;

    // 3. 归一 CRLF→LF 后再算 SHA-256，与 meta.checksum_sha256 严格比对
    //    （大小写敏感，必须小写）。行尾归一与 CI 守卫步骤一致。
    let cache_bytes = to_lf(&cache_raw);
    let actual = sha256_hex(&cache_bytes);
    if actual != meta.checksum_sha256 {
        return Err(LoadError::ChecksumMismatch {
            expected: meta.checksum_sha256,
            actual,
        });
    }

    // 4. 反序列化 cache（用归一后的字节，JSON 解析不关心行尾）。
    let rules: CacheRules = serde_json::from_slice(&cache_bytes)?;

    // 5. L3 检查：任一可清理条目 risk_level == "L3" 即拒绝。
    for entries in rules.modules.values() {
        for entry in entries {
            if entry.risk_level == "L3" {
                return Err(LoadError::L3InRules(entry.id.clone()));
            }
        }
    }

    // 5b. task-083 反向断言：L1/L2 的 paths 不得落在 whitelist_deny 里。
    //     复用同一份红线清单（不新建第二份），冲突明细全量返回，
    //     让启动日志能一次列出所有问题而不是修一条报一条。
    let conflicts = crate::rules::schema::find_l3_conflicts(&rules);
    if !conflicts.is_empty() {
        let lines: Vec<String> = conflicts
            .iter()
            .map(|c| format!("{} ({} ⊂ {})", c.rule_id, c.path, c.deny))
            .collect();
        return Err(LoadError::L3PathConflict(lines.len(), lines));
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

/// 内置规则覆盖的**目录前缀**（task-078，供「内置优先于用户规则」判定）。
///
/// 输出统一成：展开后的小写、`\` 分隔、**无尾分隔符**（`c:\users\an\appdata\local\temp`）。
/// 归一化放在这里而不是每个调用点，是因为这个字符串要参与前缀比较——
/// 大小写或分隔符不一致会让比较静默失效（用户规则与内置规则重复扫同一批文件，
/// 条目翻倍、大小重复计）。
///
/// 展开失败的条目**跳过**（加载流程已经校验过能展开，这里失败只可能是环境变量
/// 在加载后被改掉；跳过比 panic 好）。
pub fn builtin_dir_prefixes(loaded: &LoadedRules) -> Vec<String> {
    let mut out = Vec::new();
    for entries in loaded.rules.modules.values() {
        for entry in entries {
            for raw in &entry.paths {
                if let Ok(p) = env_expand::expand(raw) {
                    out.push(normalize_prefix(&p));
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// 归一化一个目录前缀：`/` → `\`、小写、去掉尾部 `\`（保留盘符后的那个）。
fn normalize_prefix(p: &str) -> String {
    let s = p.replace('/', "\\").to_lowercase();
    // `c:\` 保留（根目录），`c:\foo\` → `c:\foo`
    let trimmed = s.trim_end_matches('\\');
    if trimmed.len() == 2 && trimmed.ends_with(':') {
        // 只剥到 `c:` 时把分隔符还回去，否则 `c:` 会前缀匹配上 `c:\anything`
        format!("{trimmed}\\")
    } else if trimmed.is_empty() {
        s
    } else {
        trimmed.to_string()
    }
}

/// `path` 是否已被某个内置规则覆盖（前缀比较，大小写 / 分隔符不敏感）。
///
/// # 为什么用「目录前缀」而不是「精确相等」
///
/// 内置规则登记的是目录（`%TEMP%`、`%LOCALAPPDATA%\npm-cache`），而用户规则
/// 匹配到的是**文件**（`...\Temp\a.tmp`）。要判「这个文件归内置管」只能看前缀。
///
/// # 边界必须落在分隔符上
///
/// 纯字符串前缀会把 `c:\temp2\x` 判成在 `c:\temp` 之下——那是错的。所以比较时
/// 要求 `path` 在 `prefix` 之后紧跟 `\`（或完全相等）。
pub fn covered_by_builtin(path: &str, prefixes: &[String]) -> bool {
    let p = path.replace('/', "\\").to_lowercase();
    prefixes.iter().any(|prefix| {
        if p == *prefix {
            return true;
        }
        match p.strip_prefix(prefix.as_str()) {
            // `c:\temp` + `\a.tmp` → 命中；`c:\temp2` + `2\x` → 不命中
            Some(rest) => rest.starts_with('\\'),
            None => false,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::models::{CacheRules, RuleEntry};
    use std::collections::HashMap;

    fn entry(id: &str, paths: &[&str]) -> RuleEntry {
        RuleEntry {
            id: id.to_string(),
            category: id.to_string(),
            display_name: id.to_string(),
            paths: paths.iter().map(|s| s.to_string()).collect(),
            risk_level: "L1".to_string(),
            description: String::new(),
            impact_note: String::new(),
            rebuild_note: String::new(),
            elevated: false,
            // task-083：新增的 matches 字段，存量用例按「旧规则」处理（空 = 不过滤）
            matches: Vec::new(),
        }
    }

    fn loaded(modules: &[(&str, Vec<RuleEntry>)]) -> LoadedRules {
        let mut map = HashMap::new();
        for (k, v) in modules {
            map.insert(k.to_string(), v.clone());
        }
        LoadedRules {
            rules: CacheRules {
                version: "test".to_string(),
                modules: map,
                whitelist_deny: vec![],
            },
            meta: crate::rules::models::RulesMeta {
                version: "test".to_string(),
                checksum_sha256: String::new(),
                source_url: String::new(),
                downloaded_at: 0,
                custom_rules: serde_json::Value::Null,
            },
        }
    }

    // ===== task-078：内置优先判定 =====

    #[test]
    fn builtin_prefixes_are_normalized_and_deduped() {
        // 同一目录写两遍（大小写 / 分隔符不同）只留一份，且统一成小写反斜杠、无尾分隔符
        let l = loaded(&[
            ("A", vec![entry("a", &[r"C:\Temp\"])]),
            ("B", vec![entry("b", &["c:/temp"])]),
        ]);
        let p = builtin_dir_prefixes(&l);
        assert_eq!(p, vec![r"c:\temp".to_string()], "应去重并归一: {p:?}");
    }

    #[test]
    fn builtin_prefix_keeps_drive_root_separator() {
        // `c:` 不能当前缀——它会把 `c:\anything` 全判成内置覆盖
        let l = loaded(&[("A", vec![entry("a", &[r"C:\"])])]);
        let p = builtin_dir_prefixes(&l);
        assert_eq!(p, vec![r"c:\".to_string()]);
    }

    #[test]
    fn covered_by_builtin_matches_children_only() {
        let prefixes = vec![r"c:\temp".to_string()];

        assert!(covered_by_builtin(r"C:\Temp\a.tmp", &prefixes));
        assert!(
            covered_by_builtin(r"c:/temp/sub/b.tmp", &prefixes),
            "分隔符不敏感"
        );
        assert!(
            covered_by_builtin(r"C:\Temp", &prefixes),
            "目录本身也算覆盖"
        );

        // 关键边界：`c:\temp2` 不是 `c:\temp` 的子路径
        assert!(
            !covered_by_builtin(r"C:\Temp2\a.tmp", &prefixes),
            "纯字符串前缀会把 temp2 误判成 temp 之下"
        );
        assert!(
            !covered_by_builtin(r"D:\Temp\a.tmp", &prefixes),
            "不同盘不覆盖"
        );
    }

    #[test]
    fn empty_prefixes_cover_nothing() {
        // 内置规则加载失败时传空表 → 不做任何抑制（宁可重复也不误吞用户条目）
        assert!(!covered_by_builtin(r"C:\Temp\a.tmp", &[]));
    }

    #[test]
    fn external_deny_list_still_works() {
        // 既有行为不能被 task-078 的改动带坏
        let mut l = loaded(&[]);
        l.rules.whitelist_deny = vec![r"C:\Keep".to_string()];
        assert_eq!(external_deny_list(&l), vec![r"C:\Keep".to_string()]);
    }
}
