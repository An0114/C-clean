//! TRD §5.3 错误码常量表：i32 值 + 中文用户可见文案一一对应。
//!
//! 错误码语义固定在本文件，前端与提权管道共用同一套编号。

/// 错误码值与文案对（const 结构体，零运行时开销）。
#[derive(Debug, Clone, Copy)]
pub struct ErrorCode {
    /// 机器可读编号（信封里的 code 字段）。
    pub code: i32,
    /// 用户可见中文短句。
    pub message: &'static str,
}

/// 成功。
pub const OK: ErrorCode = ErrorCode {
    code: 0,
    message: "ok",
};
/// 扫描已取消。
pub const SCAN_CANCELLED: ErrorCode = ErrorCode {
    code: 1002,
    message: "扫描已取消",
};
/// 待清理清单为空。
pub const DRYRUN_EMPTY: ErrorCode = ErrorCode {
    code: 2001,
    message: "待清理清单为空",
};
/// L3 红线拒绝。
pub const WHITELIST_DENIED: ErrorCode = ErrorCode {
    code: 2002,
    message: "检测到受保护路径，操作已拒绝（L3 红线）",
};
/// 文件被进程占用。
pub const FILE_OCCUPIED: ErrorCode = ErrorCode {
    code: 2003,
    message: "文件被进程占用",
};
/// 提权被取消。
pub const ELEVATION_DENIED: ErrorCode = ErrorCode {
    code: 2004,
    message: "需要管理员权限，提权被取消",
};
/// 保险箱空间不足。
pub const SAFEBOX_FULL: ErrorCode = ErrorCode {
    code: 2005,
    message: "保险箱空间不足且自动淘汰失败",
};
/// 注册表备份失败。
pub const REG_BACKUP_FAILED: ErrorCode = ErrorCode {
    code: 2006,
    message: "注册表备份失败，删除已取消",
};
/// 还原点创建失败。
pub const RESTOREPOINT_FAILED: ErrorCode = ErrorCode {
    code: 2007,
    message: "系统还原点创建失败",
};
/// DISM 组件存储错误。
pub const DISM_ERROR: ErrorCode = ErrorCode {
    code: 3001,
    message: "组件存储分析失败（附 DISM 原始输出）",
};
/// 规则库校验和不匹配。
pub const RULE_CHECKSUM_MISMATCH: ErrorCode = ErrorCode {
    code: 3002,
    message: "规则库校验失败，已保留旧版本",
};
/// 规则库网络错误。
pub const RULE_NETWORK_ERROR: ErrorCode = ErrorCode {
    code: 3003,
    message: "规则库更新失败，继续使用本地版本",
};
/// 内部错误。
pub const INTERNAL: ErrorCode = ErrorCode {
    code: 5001,
    message: "内部错误（附日志 ID）",
};

/// 全部 13 个错误码（供遍历测试）。
pub const ALL: [ErrorCode; 13] = [
    OK,
    SCAN_CANCELLED,
    DRYRUN_EMPTY,
    WHITELIST_DENIED,
    FILE_OCCUPIED,
    ELEVATION_DENIED,
    SAFEBOX_FULL,
    REG_BACKUP_FAILED,
    RESTOREPOINT_FAILED,
    DISM_ERROR,
    RULE_CHECKSUM_MISMATCH,
    RULE_NETWORK_ERROR,
    INTERNAL,
];
