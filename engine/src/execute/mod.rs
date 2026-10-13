//! 执行引擎（task-003）：删除动作的代码级唯一入口 + L3 红线白名单匹配器。
//!
//! 任何来源（UI / CLI / ps1 返回 / 提权回包）要删文件时，都必须经过
//! [`delete::execute`]；本模块词法层面完成路径规范化与 L3 判定，
//! 真实删除动作由后续任务通过 [`delete::Deleter`] trait 注入，本任务不碰磁盘。

pub mod delete;
pub mod error_kind;
pub mod executor;
pub mod fs_deleter;
pub mod list_pending;
pub mod locking;
pub mod normalize;
pub mod resolve;
pub mod safe_box;
pub mod whitelist;
