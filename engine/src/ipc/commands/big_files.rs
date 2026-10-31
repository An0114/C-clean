//! `big_files.scan` 命令纯逻辑（task-066）。
//!
//! 为什么这一层还要包一下：真实 Tauri command 在 `app/src-tauri/src/commands.rs`，
//! 那里拿不到纯逻辑的单测（需要 AppHandle / State）。把「扫描 + 装信封」放在
//! engine 里，测试可以直接对临时目录调用，不用起 Tauri 运行时。

use std::path::Path;

use super::super::envelope::Envelope;
use crate::scan::big_files::{scan_all, BigFilesReport, TopFilesOptions};

/// 处理 `big_files.scan`：在 `root`（形如 `C:\`）下跑一次只读扫描。
///
/// # 为什么返回 `Envelope` 而不是 `Result`
///
/// 扫描**没有「失败」这个状态**——单个分类读不到（权限/文件不存在）就按 0 处理
/// 并在 `recommendation` 里说明，其余分类照常返回（见 [`scan_all`] 的注释）。
/// 用户点「扫描」是想看全景，为某一个文件读不到就整页报错，等于什么都看不到。
///
/// 真要报错只有「传进来的根路径不合法」这一种，那是调用方的 bug，用 5001 兜底。
pub fn handle_big_files_scan(root: &Path) -> Envelope<BigFilesReport> {
    if !root.is_dir() {
        // 根路径不存在：说明调用方给错了盘符（比如系统装在 D: 但传了 C:\）。
        // 这不是业务失败，但要明确告诉前端，别让用户以为「盘是空的」。
        eprintln!("[big_files_scan] 根路径不是目录: {}", root.display());
        return Envelope::err(crate::ipc::error_codes::INTERNAL);
    }
    let report = scan_all(root, TopFilesOptions::default());
    Envelope::ok(report)
}
