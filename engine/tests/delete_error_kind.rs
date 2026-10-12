//! task-019 集成测试：classify_io_error + SafeBoxDeleter FILE_IN_USE 前缀。

use std::io;

use engine::execute::error_kind::{classify_io_error, DeleteFailureKind};

#[test]
fn classify_not_found() {
    let e = io::Error::from(io::ErrorKind::NotFound);
    assert_eq!(classify_io_error(&e), DeleteFailureKind::NotFound);
}

#[test]
fn classify_sharing_violation() {
    // Windows ERROR_SHARING_VIOLATION = 32
    let e = io::Error::from_raw_os_error(32);
    assert_eq!(classify_io_error(&e), DeleteFailureKind::SharingViolation);
}

#[test]
fn classify_permission_denied() {
    // Windows ERROR_ACCESS_DENIED = 5
    let e = io::Error::from_raw_os_error(5);
    assert_eq!(classify_io_error(&e), DeleteFailureKind::PermissionDenied);
}

#[test]
fn classify_unknown() {
    let e = io::Error::from(io::ErrorKind::Other);
    assert_eq!(classify_io_error(&e), DeleteFailureKind::Unknown);
}
