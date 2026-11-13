//! task-004 信封序列化与错误码测试。

use engine::ipc::envelope::Envelope;
use engine::ipc::error_codes::{self, WHITELIST_DENIED};

#[test]
fn ok_envelope_serializes_to_expected_json() {
    let env = Envelope::ok(serde_json::json!({"a": 1}));
    assert_eq!(
        serde_json::to_value(env).unwrap(),
        serde_json::json!({"code": 0, "message": "ok", "data": {"a": 1}})
    );
}

#[test]
fn err_envelope_sets_data_null() {
    let env: Envelope<()> = Envelope::err(WHITELIST_DENIED);
    assert_eq!(
        serde_json::to_value(env).unwrap(),
        serde_json::json!({
            "code": 2002,
            "message": "检测到受保护路径，操作已拒绝（L3 红线）",
            "data": null
        })
    );
}

#[test]
fn all_error_codes_have_message() {
    for c in error_codes::ALL.iter() {
        assert!(!c.message.is_empty(), "错误码 {} 文案为空", c.code);
    }
    assert_eq!(
        error_codes::ALL.len(),
        15,
        "TRD §5.3 共 14 个错误码（task-084 加 2008；task-086 组2 加 2009 符号链接拒绝）"
    );
}
