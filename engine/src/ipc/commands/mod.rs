//! IPC 命令纯逻辑（不依赖 Tauri / DB）。

pub mod big_files;
pub mod dry_run;
pub mod registry_health;
pub mod residual;
pub mod scan_start;
