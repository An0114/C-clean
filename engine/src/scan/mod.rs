//! 扫描子系统（task-012）：消费 ps1 脚本（task-011）产出的 NDJSON 行协议。
//!
//! - `parser`：单行 NDJSON → [`NdjsonEvent`] 枚举（serde）。
//! - `runner`：spawn `powershell.exe -File <script>`，逐行读 stdout，
//!   item 行写 `cleanup_item` 表并 emit 进度，done 行结束。

pub mod big_files;
pub mod orchestrator;
pub mod parser;
pub mod registry_health;
pub mod residual;
pub mod runner;
/// Windows 注册表最小封装（task-068 抽出，供 residual / registry_health 共用）。
pub mod winreg;
/// WinSxS 组件存储分析/清理（task-065，C 模块）。
pub mod winsxs;

pub use big_files::{BigFileItem, BigFilesReport};
pub use orchestrator::{OrchestratorError, OrchestratorSummary, ScanOrchestrator};
pub use parser::{NdjsonEvent, ParseError};
pub use registry_health::{RegistryHealthItem, RegistryHealthReport};
pub use residual::{ResidualItem, ResidualReport};
pub use runner::{RunError, ScanRunner, ScanSummary};
pub use winsxs::WinsxsAnalysis;
