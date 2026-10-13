//! 规则库子模块（task-005）。
//!
//! 三段式（按提交序列逐步落地）：
//! - [`models`]：cache_rules.json / rules_meta.json 的 serde 结构；
//! - [`env_expand`]：路径变量白名单展开（仅 5 个环境变量 + `~`）；
//! - [`loader`]：读盘 → SHA-256 比对 → 反序列化 → L3 检查 → 变量展开。

pub mod env_expand;
pub mod loader;
pub mod meta_store;
pub mod models;
pub mod remote;

pub use env_expand::{expand, ExpandError};
pub use loader::{external_deny_list, load, LoadError, LoadedRules};
pub use meta_store::{get_active_rules_meta, upsert_active_rules_meta, RulesMetaRow};
pub use models::{CacheRules, RuleEntry, RulesMeta};
pub use remote::{
    apply_update, fetch_remote_meta, fetch_remote_rules_bytes, ApplyError, ApplyResult,
    CheckUpdateResp, RemoteRulesMeta, REMOTE_RULES_META_URL, REMOTE_RULES_URL,
};
