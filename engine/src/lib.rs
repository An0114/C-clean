//! C-Disk Cleaner 引擎核心库（M0 空 crate 占位）。
//!
//! 本任务（task-001）只放一个占位函数冒烟通过即可；
//! task-002 起在此依次落地 `db/`、`execute/`、`grade/` 等子模块，
//! 本文件保持最小，不预建任何子模块目录，避免与后续任务抢写锁。

/// 占位入口：供 Tauri 壳的 `ping` 命令回连验证 workspace 链接是否正常。
pub fn hello_engine() -> &'static str {
    "engine"
}

/// 公共数据层：SQLite 连接入口与 schema 迁移（task-002）。
pub mod db;

/// 审计日志只读查询（task-024，M1-F 报表基础）。
pub mod audit_log;

/// 执行引擎：删除唯一入口与 L3 红线白名单匹配器（task-003）。
pub mod execute;

/// IPC 网关：统一信封、命令纯逻辑、进度事件通道（task-004）。
pub mod ipc;

/// 规则库：静态 JSON 模型、环境变量白名单展开、SHA-256 校验加载（task-005）。
pub mod rules;

/// 扫描子系统：消费 ps1 NDJSON 行协议，spawn 子进程、写 cleanup_item、emit 进度（task-012）。
pub mod scan;
