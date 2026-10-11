//! `scan.start` 命令纯逻辑（本任务不真启动扫描，只发递增 sessionId）。

use super::super::envelope::Envelope;
use super::super::error_codes::INTERNAL;

/// `scan.start` 入参。前端传 camelCase（targetDrive），Rust 侧见 snake_case。
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStartReq {
    /// "full" | "custom"。
    pub scope: String,
    /// 单盘符选择，形如 "C:"（可选）。
    pub target_drive: Option<String>,
}

/// `scan.start` 出参：立即返回会话 id，进度走事件通道。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStartResp {
    /// 递增整数会话 id。
    pub session_id: i64,
}

/// 处理 scan.start：校验入参 → 递增 sessionId → 返回信封。
/// 不启动后台线程、不写 DB、不 emit 进度（后续任务接入）。
pub fn handle_scan_start(
    req: ScanStartReq,
    next_session_id: &mut i64,
) -> Envelope<ScanStartResp> {
    if req.scope != "full" && req.scope != "custom" {
        return Envelope::err(INTERNAL);
    }
    if let Some(drive) = &req.target_drive {
        // 形如 "C:"：1 个字母 + 冒号，仅此形态合法。
        let bytes = drive.as_bytes();
        let ok = bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
        if !ok {
            return Envelope::err(INTERNAL);
        }
    }
    *next_session_id += 1;
    Envelope::ok(ScanStartResp {
        session_id: *next_session_id,
    })
}
