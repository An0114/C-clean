//! task-019：删除失败错误分类。
//!
//! 把 `std::io::Error` 按 Windows 常见 errno 粗分成 4 类，让 SafeBoxDeleter /
//! executor 能给 audit_log 和前端一个稳定的「文件被占用 / 需提权 / 幂等 NotFound」
//! 信号。本模块只做数字比较，不引 windows crate FFI。

use std::io;

/// 删除失败的分类（与 Windows errno 对应）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteFailureKind {
    /// 文件不存在——幂等，不算失败（errno 2 / ErrorKind::NotFound）。
    NotFound,
    /// 权限不足，需要提权（errno 5 / ErrorKind::PermissionDenied）。
    PermissionDenied,
    /// 文件被其他进程占用（Windows ERROR_SHARING_VIOLATION = 32）。
    SharingViolation,
    /// 其他未知错误。
    Unknown,
}

/// 把 io::Error 分类。
///
/// Windows raw_os_error：
/// - 2 = ERROR_FILE_NOT_FOUND → NotFound
/// - 5 = ERROR_ACCESS_DENIED  → PermissionDenied
/// - 32 = ERROR_SHARING_VIOLATION → SharingViolation
///
/// 非 Windows 平台退到 `err.kind()` 粗分。
pub fn classify_io_error(err: &io::Error) -> DeleteFailureKind {
    // 1. ErrorKind::NotFound 跨平台稳定
    if err.kind() == io::ErrorKind::NotFound {
        return DeleteFailureKind::NotFound;
    }
    // 2. ErrorKind::PermissionDenied 跨平台稳定
    if err.kind() == io::ErrorKind::PermissionDenied {
        return DeleteFailureKind::PermissionDenied;
    }
    // 3. Windows 专用 errno 数字判定
    match err.raw_os_error() {
        Some(2) => DeleteFailureKind::NotFound,
        Some(5) => DeleteFailureKind::PermissionDenied,
        Some(32) => DeleteFailureKind::SharingViolation,
        _ => DeleteFailureKind::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_not_found_from_kind() {
        let e = io::Error::from(io::ErrorKind::NotFound);
        assert_eq!(classify_io_error(&e), DeleteFailureKind::NotFound);
    }

    #[test]
    fn classify_sharing_violation_from_raw_os_error() {
        // Windows 上 raw_os_error(32) = ERROR_SHARING_VIOLATION
        let e = io::Error::from_raw_os_error(32);
        assert_eq!(classify_io_error(&e), DeleteFailureKind::SharingViolation);
    }

    #[test]
    fn classify_permission_denied_from_raw_os_error() {
        // Windows 上 raw_os_error(5) = ERROR_ACCESS_DENIED
        let e = io::Error::from_raw_os_error(5);
        assert_eq!(classify_io_error(&e), DeleteFailureKind::PermissionDenied);
    }

    #[test]
    fn classify_unknown() {
        let e = io::Error::from(io::ErrorKind::Other);
        assert_eq!(classify_io_error(&e), DeleteFailureKind::Unknown);
    }
}
