//! M1-F 报表基础：audit_log 只读查询。
//!
//! task-024：提供 list 命令的 engine 层纯函数，按 ts 倒序 + 分页返回条目。
//! task-025：stats 聚合（total_bytes / success_count / failed_count / session_count）。
//!
//! 与任务书偏差：v1__init.sql 的 audit_log 真实列为
//! `id/ts/actor/action/target/detail/result/denial_reason`，**没有 session_id / event_type / path / created_at**；
//! 本模块按真实列名映射，对外结构使用业务语义命名（event_type=action、path=target、created_at=ts）。

pub mod query;
pub mod stats;

pub use query::{list_audit_log, AuditLogEntry, AuditLogFilter};
pub use stats::{cleanup_stats, CleanupStats};
