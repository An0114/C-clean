//! IPC 网关（task-004）：把「前端要什么、后端回什么」的契约用 Rust 类型钉死。
//!
//! 纯内存、不依赖 Tauri：命令处理函数接受 trait 注入，返回统一信封；
//! 真实 Tauri command 包装在 `app/src-tauri/src/commands.rs`。

pub mod commands;
pub mod envelope;
pub mod error_codes;
pub mod progress;
