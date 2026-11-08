//! task-083：规则匹配的扩展 schema、L3 反向断言、路径去重。
//!
//! # 为什么这三个东西放在一个文件里
//!
//! 它们都作用在「规则加载 / 命中」这条链路上，且共用同一套路径归一语义
//! （Windows 分隔符 + 大小写不敏感）。分开三个文件只会让 `mod.rs` 多三行
//! `pub mod`，没有实际收益。
//!
//! # 与已有代码的关系
//!
//! - `cache_rules.json` 的 `whitelist_deny` **本来就是** L3 红线清单
//!   （见 `models.rs::CacheRules.whitelist_deny`、`loader::external_deny_list`）。
//!   本模块直接复用它做反向断言，**不再新建第二份清单**——两份清单必然漂移，
//!   那正是本任务要消灭的「双份真值源」。
//! - `ipc/commands/user_rules.rs` 里有一份 `glob_match`，但 rules 层不能反向
//!   依赖 ipc 层，所以这里用 `regex` crate 把 glob 编译成正则，独立实现。

use serde::Deserialize;
use std::collections::HashMap;

use crate::rules::env_expand;
use crate::rules::models::CacheRules;

// ===================== 1. matches schema =====================

/// 一条文件级精筛条件。
///
/// JSON 形状（internally tagged）：
/// ```json
/// { "type": "glob",   "pattern": "*.tmp" }
/// { "type": "regex",  "pattern": "^cache_[0-9]+\\.bin$" }
/// { "type": "suffix", "pattern": ".log" }
/// ```
///
/// **作用于文件名（basename），不是完整路径**——`*.tmp` 匹配
/// `C:\Temp\a.tmp` 符合直觉；若作用于全路径，`*` 不跨分隔符就永远匹配不上。
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MatchSpec {
    Glob { pattern: String },
    Regex { pattern: String },
    Suffix { pattern: String },
}

/// 规则编译 / 匹配失败。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MatchError {
    #[error("规则 {rule_id} 的 matches[{index}] 正则非法: {reason}")]
    BadRegex {
        rule_id: String,
        index: usize,
        reason: String,
    },
}

/// 编译后的匹配器。
///
/// **为什么要预编译**：扫描时每个文件都要过一遍所有规则，若每次都
/// `Regex::new`，一条规则面对十万个文件就要编译十万次——纯浪费。
/// 加载规则时编译一次，扫描期只做 `is_match`。
#[derive(Debug, Clone)]
pub struct CompiledSpec {
    kind: CompiledKind,
    /// 原始 pattern，报错 / 日志时给人看。
    source: String,
}

#[derive(Debug, Clone)]
enum CompiledKind {
    Regex(regex::Regex),
    /// 已小写化的后缀，如 `.tmp`。
    Suffix(String),
}

impl CompiledSpec {
    pub fn source(&self) -> &str {
        &self.source
    }

    /// 对一个**文件名**（basename）做匹配，大小写不敏感。
    pub fn is_match(&self, file_name: &str) -> bool {
        match &self.kind {
            CompiledKind::Regex(re) => re.is_match(file_name),
            CompiledKind::Suffix(sfx) => file_name.to_lowercase().ends_with(sfx),
        }
    }
}

/// 取路径的最后一段（同时兼容 `\` 与 `/`）。
pub fn file_name(path: &str) -> &str {
    path.rsplit(|c| c == '\\' || c == '/')
        .next()
        .unwrap_or(path)
}

/// 把 glob 转成正则字符串。
///
/// - `*`：匹配**本层**任意字符（不含分隔符）
/// - `**`：匹配任意字符（含分隔符）——用于文件名匹配时等价于 `*`，保留语义一致
/// - `?`：匹配一个非分隔符字符
/// - 其余字符一律 `regex::escape`，避免 `.` `(` 这类元字符被当成正则
fn glob_to_regex(pattern: &str) -> String {
    let mut out = String::from("(?i)^");
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' => {
                if i + 1 < chars.len() && chars[i + 1] == '*' {
                    out.push_str(".*");
                    i += 2;
                } else {
                    // 文件名里不该有分隔符，`*` 不跨层更精确
                    out.push_str(r"[^\\/]*");
                    i += 1;
                }
            }
            '?' => {
                out.push_str(r"[^\\/]");
                i += 1;
            }
            c => {
                out.push_str(&regex::escape(&c.to_string()));
                i += 1;
            }
        }
    }
    out.push('$');
    out
}

/// 编译一条规则的 `matches`，供扫描期复用。
pub fn compile_specs(rule_id: &str, specs: &[MatchSpec]) -> Result<Vec<CompiledSpec>, MatchError> {
    let mut out = Vec::with_capacity(specs.len());
    for (index, spec) in specs.iter().enumerate() {
        let (source, pattern) = match spec {
            MatchSpec::Glob { pattern } => (pattern.clone(), glob_to_regex(pattern)),
            MatchSpec::Regex { pattern } => (pattern.clone(), format!("(?i){pattern}")),
            MatchSpec::Suffix { pattern } => {
                out.push(CompiledSpec {
                    kind: CompiledKind::Suffix(pattern.to_lowercase()),
                    source: pattern.clone(),
                });
                continue;
            }
        };
        let re = regex::Regex::new(&pattern).map_err(|e| MatchError::BadRegex {
            rule_id: rule_id.to_string(),
            index,
            reason: e.to_string(),
        })?;
        out.push(CompiledSpec {
            kind: CompiledKind::Regex(re),
            source,
        });
    }
    Ok(out)
}

/// 判断文件是否通过规则的精筛。
///
/// **兼容旧规则**：`specs` 为空 = 该文件级过滤不存在，一律放行，
/// 仍走原来的 `paths` 目录前缀匹配。多条 `matches` 之间是 **OR**：
/// 它们是并列的候选模式（「`*.tmp` 或 `*.log` 都算」），AND 会让用户
/// 写出永远不命中的规则。
pub fn matches_file(specs: &[CompiledSpec], path: &str) -> bool {
    if specs.is_empty() {
        return true;
    }
    let name = file_name(path);
    specs.iter().any(|s| s.is_match(name))
}

// ===================== 2. L3 反向断言 =====================

/// 一条「L1/L2 规则路径落在 L3 红线内」的冲突。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct L3Conflict {
    pub rule_id: String,
    pub path: String,
    pub deny: String,
}

/// 路径归一：分隔符统一成 `/`、转小写、去掉尾部斜杠。
///
/// Windows 文件系统大小写不敏感，`C:\Windows\System32` 与
/// `c:/windows/system32` 必须视为同一条路径，否则红线能被大小写绕开。
fn norm_path(s: &str) -> String {
    let s = s.replace('\\', "/").to_lowercase();
    s.trim_end_matches('/').to_string()
}

/// `path` 是否等于 `deny` 或位于 `deny` 之下（含子目录）。
fn is_under(path: &str, deny: &str) -> bool {
    let p = norm_path(path);
    let d = norm_path(deny);
    if d.is_empty() {
        return false;
    }
    p == d || p.starts_with(&format!("{d}/"))
}

/// 找出所有 L1/L2 规则路径与 L3 红线的冲突。
///
/// **纯读**：不改规则、不 panic，方便单测逐个断言冲突明细。
/// 无法展开变量的路径交给 `loader` 的既有 `Expand` 错误去报，这里跳过——
/// 同一个问题不该报两次。
pub fn find_l3_conflicts(rules: &CacheRules) -> Vec<L3Conflict> {
    let denies: Vec<String> = rules
        .whitelist_deny
        .iter()
        .filter_map(|raw| env_expand::expand(raw).ok())
        .collect();

    let mut out = Vec::new();
    for entries in rules.modules.values() {
        for entry in entries {
            for raw in &entry.paths {
                let Ok(expanded) = env_expand::expand(raw) else {
                    continue;
                };
                for deny in &denies {
                    if is_under(&expanded, deny) {
                        out.push(L3Conflict {
                            rule_id: entry.id.clone(),
                            path: raw.clone(),
                            deny: deny.clone(),
                        });
                    }
                }
            }
        }
    }
    out
}

/// 启动期防呆：有冲突就 **panic**，把冲突规则 id 全打出来。
///
/// 为什么用 panic 而不是返回错误：这是**配置错误**，不是运行时可恢复的异常。
/// 带着一份「把 System32 标成 L1」的规则继续跑，后果远比进程起不来严重。
/// `loader::load` 另有返回 `Err` 的版本，供需要优雅降级的调用方使用。
pub fn assert_no_l3_conflict_or_panic(rules: &CacheRules) {
    let conflicts = find_l3_conflicts(rules);
    if conflicts.is_empty() {
        return;
    }
    let lines: Vec<String> = conflicts
        .iter()
        .map(|c| {
            format!(
                "规则 {} 的 path {} 落在 L3 红线 {} 内",
                c.rule_id, c.path, c.deny
            )
        })
        .collect();
    panic!(
        "L3 红线反向断言失败（{} 条冲突）：\n  {}",
        conflicts.len(),
        lines.join("\n  ")
    );
}

// ===================== 3. 路径去重 =====================

/// 一次「规则 → 文件」的命中记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleHit {
    /// 被命中的文件绝对路径。
    pub path: String,
    /// 命中它的规则 id。
    pub rule_id: String,
    /// 规则声明的根路径（**越长 = 越具体**）。
    pub rule_path: String,
    /// `L1` / `L2`。
    pub risk_level: String,
}

fn level_rank(level: &str) -> u8 {
    match level.to_uppercase().as_str() {
        "L3" => 3,
        "L2" => 2,
        "L1" => 1,
        "L0" => 0,
        _ => 0,
    }
}

/// `a` 是否比 `b` 更应该保留。
///
/// 优先级：**更具体的路径 > 更高的风险等级 > 规则 id 字典序**（最后一条保证
/// 输出确定，否则 HashMap 的迭代顺序会让单测随机假失败）。
fn better(a: &RuleHit, b: &RuleHit) -> bool {
    let la = norm_path(&a.rule_path).len();
    let lb = norm_path(&b.rule_path).len();
    if la != lb {
        return la > lb;
    }
    let ra = level_rank(&a.risk_level);
    let rb = level_rank(&b.risk_level);
    if ra != rb {
        return ra > rb;
    }
    a.rule_id < b.rule_id
}

/// 去重：同一文件被多条规则命中时只保留一条，避免大小被重复统计。
///
/// 路径比较用归一后的值（大小写 / 分隔符差异视为同一文件），但**返回的是
/// 原始 `RuleHit`**，下游拿到的路径仍是能直接用于 IO 的原值。
pub fn dedup_hits(hits: Vec<RuleHit>) -> Vec<RuleHit> {
    let mut best: HashMap<String, RuleHit> = HashMap::new();
    for hit in hits {
        let key = norm_path(&hit.path);
        match best.get(&key) {
            None => {
                best.insert(key, hit);
            }
            Some(prev) if better(&hit, prev) => {
                best.insert(key, hit);
            }
            _ => {}
        }
    }
    let mut out: Vec<RuleHit> = best.into_values().collect();
    out.sort_by(|a, b| norm_path(&a.path).cmp(&norm_path(&b.path)));
    out
}
