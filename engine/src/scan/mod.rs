//! 扫描子系统（task-012）：消费 ps1 脚本（task-011）产出的 NDJSON 行协议。
//!
//! - `parser`：单行 NDJSON → [`NdjsonEvent`] 枚举（serde）。
//! - `runner`：spawn `powershell.exe -File <script>`，逐行读 stdout，
//!   item 行写 `cleanup_item` 表并 emit 进度，done 行结束。

pub mod parser;
// runner 在 commit 2 落地。

pub use parser::{NdjsonEvent, ParseError};
