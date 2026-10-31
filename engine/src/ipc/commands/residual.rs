//! `residual.scan` / `residual.delete` 命令纯逻辑（task-067）。
//!
//! 与 `big_files` 同样的理由：真实 Tauri command 在 `app/src-tauri/src/commands.rs`，
//! 那里需要 AppHandle / State，单测起不来。把「扫描/删除 + 装信封」放在 engine 里，
//! 测试可以直接对临时 DB + 临时目录调用。

use std::path::Path;

use super::super::envelope::Envelope;
use crate::ipc::error_codes::{INTERNAL, WHITELIST_DENIED};
use crate::scan::residual::{delete_residuals, scan_all, ResidualDeleteResult, ResidualReport};

/// 处理 `residual.scan`：读注册表 + 扫安装目录 + 交叉比对。
///
/// # 为什么永远返回 `Envelope::ok`
///
/// 扫描没有「失败」这个状态——注册表打不开就返回空列表、目录读不了就跳过，
/// 结果里 `registryEntries` / `dirsScanned` 会如实反映「实际读到了多少」。
/// 用户点扫描是想看全景，因为某个键没权限就整页报错，等于什么都看不到。
pub fn handle_residual_scan() -> Envelope<ResidualReport> {
    Envelope::ok(scan_all())
}

/// 处理 `residual.delete`：把勾选的残留目录**移入保险箱**（7 天可恢复）。
///
/// # 空列表
///
/// 返回 `Ok(空结果)` 而不是错误——用户点了个「移入保险箱」但一个都没勾，
/// 不是业务错误，UI 自己会禁用按钮。
///
/// # 全部被红线拒绝
///
/// 返回 **2002**（`WHITELIST_DENIED`）：这是安全事件，必须让前端明确知道
/// 「不是失败，是被拒绝」。部分失败时仍返回 `ok`，逐条原因在 `failed` 里
/// ——整批报错会让用户以为「全都失败了」，实际可能只挂了一条。
pub fn handle_residual_delete(
    paths: &[String],
    conn: &rusqlite::Connection,
    box_dir: &Path,
) -> Envelope<ResidualDeleteResult> {
    if paths.is_empty() {
        return Envelope::ok(ResidualDeleteResult::default());
    }
    if box_dir.as_os_str().is_empty() {
        // 理论上不会发生（AppPaths 一定给值），但保险箱路径为空时
        // SafeBoxDeleter 会把文件 rename 到当前目录，必须挡在门外。
        eprintln!("[residual_delete] box_dir 为空，拒绝执行");
        return Envelope::err(INTERNAL);
    }

    let result = delete_residuals(paths, conn, box_dir);

    // 一条都没成功、且全是红线拒绝 → 这是安全事件，用 2002 而不是 ok
    if result.moved.is_empty() && !result.failed.is_empty() {
        let all_denied = result.failed.iter().all(|f| f.reason.contains("安全红线"));
        if all_denied {
            eprintln!("[residual_delete] 全部命中红线，拒绝执行");
            return Envelope::err(WHITELIST_DENIED);
        }
    }

    Envelope::ok(result)
}
