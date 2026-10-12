//! 规则库子模块（task-005）。
//!
//! 三段式（按提交序列逐步落地）：
//! - [`models`]：cache_rules.json / rules_meta.json 的 serde 结构；
//! - [`env_expand`]：路径变量白名单展开（仅 5 个环境变量 + `~`）；
//! - `loader`：读盘 → SHA-256 比对 → 反序列化 → L3 检查 → 变量展开。

pub mod env_expand;
pub mod models;

pub use env_expand::{expand, ExpandError};
pub use models::{CacheRules, RuleEntry, RulesMeta};
