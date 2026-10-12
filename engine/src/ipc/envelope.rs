//! 统一响应信封（TRD §5.1）：`{code, message, data}`。
//!
//! 所有 IPC 命令返回同一结构；失败时 TRD 规定 `data=null`，
//! 唯一例外是 cleanup.dryRun 的 L3 拒绝（需回传 blocked_by_l3 清单，
//! 见任务文档 §E 的显式偏离，用 [`Envelope::err_with_data`]）。

use super::error_codes::ErrorCode;

/// 统一响应信封。字段名 snake_case 直出 JSON（前端按 camelCase 解包由 Tauri 处理）。
#[derive(Debug, serde::Serialize)]
pub struct Envelope<T: serde::Serialize> {
    /// 0 = 成功；非 0 见 error_codes。
    pub code: i32,
    /// 英文 ok / 中文短句。
    pub message: &'static str,
    /// 业务数据；失败默认 None（序列化为 null）。
    pub data: Option<T>,
}

impl<T: serde::Serialize> Envelope<T> {
    /// 成功信封。
    pub fn ok(data: T) -> Self {
        Self {
            code: 0,
            message: "ok",
            data: Some(data),
        }
    }

    /// 标准失败信封：data=None（TRD §5.1 默认）。泛型可适配任意命令的出参类型。
    pub fn err(code: ErrorCode) -> Self {
        Self {
            code: code.code,
            message: code.message,
            data: None,
        }
    }

    /// 带数据的失败信封（仅 L3 拒绝这类「必须回传明细」的场景使用）。
    pub fn err_with_data(code: ErrorCode, data: T) -> Self {
        Self {
            code: code.code,
            message: code.message,
            data: Some(data),
        }
    }
}
