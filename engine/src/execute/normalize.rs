//! Windows 绝对路径规范化器（词法层面，不解析符号链接）。
//!
//! 提交 2 仅占位类型与访问器，具体实现见提交 3。

/// 规范化后的绝对路径 newtype：反斜杠、大写盘符、无尾部、无 `..` / `.`。
///
/// 内部同时保留「原始输入」与「规范化结果」，供审计日志记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedPath {
    raw: String,
    normalized: String,
}

impl NormalizedPath {
    /// 原始输入（审计用）。
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// 规范化结果（内部比较、白名单匹配用）。
    pub fn normalized(&self) -> &str {
        &self.normalized
    }
}
