//! IPC 命令纯逻辑（不依赖 Tauri / DB）。

pub mod auto_scan;
pub mod big_files;
pub mod dry_run;
pub mod plan;
pub mod registry_health;
pub mod residual;
pub mod scan_start;
/// task-083：已安装软件清单命令。
pub mod software_inventory;
/// 用户自定义清理规则（task-078）：模型 + 校验 + glob 匹配 + CRUD。
pub mod user_rules;
