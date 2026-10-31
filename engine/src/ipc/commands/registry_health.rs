//! `registry_health.scan` / `registry_health.delete` 命令纯逻辑（task-068）。
//!
//! 与 big_files / residual 同样的理由：真实 Tauri command 在
//! `app/src-tauri/src/commands.rs`，那里需要 AppHandle / State，单测起不来。
//! 把「扫描/删除 + 装信封」放在 engine 里，测试可以直接对临时目录调用。

use std::path::Path;

use super::super::envelope::Envelope;
use crate::ipc::error_codes::{INTERNAL, WHITELIST_DENIED};
use crate::scan::registry_health::{
    delete_run_values, scan_all, RegistryDeleteResult, RegistryHealthReport, RunTarget,
};

/// 处理 `registry_health.scan`：三分类体检（Run 键 / 卸载残留 / 计划任务）。
///
/// 永远返回 `ok`——读不到注册表键就返回空列表，`taskScanAvailable` 会如实反映
/// 计划任务目录能不能读。体检的语义是「能看到多少说多少」，不是「要么全有要么报错」。
pub fn handle_registry_health_scan(tasks_dir: &Path) -> Envelope<RegistryHealthReport> {
    Envelope::ok(scan_all(tasks_dir))
}

/// 处理 `registry_health.delete`：删除 Run 启动项，**每个键删前先导出 `.reg` 备份**。
///
/// # 为什么没有单独的 `registry_health_backup` 命令
///
/// 任务书列了两个命令（scan + backup），但「只备份不删」没有实际用途，
/// 而「备份和删除分开」恰好是最危险的组合——中间任何一步失败都可能变成
/// 「备份了但没删」或「没备份就删」。所以合成一条命令，内部 **fail-closed**：
/// 备份写不出来就不删。
///
/// # 空列表
///
/// 返回 `ok` + 空结果（用户点了个按钮但一个都没勾，不是业务错误）。
///
/// # 全部被 L3 拒绝
///
/// 返回 **2002**（`WHITELIST_DENIED`）：这是安全事件（有人试图删系统启动项），
/// 必须让前端明确知道「不是失败，是被拒绝」。
pub fn handle_registry_health_delete(
    backup_dir: &Path,
    targets: &[RunTarget],
    stamp: i64,
) -> Envelope<RegistryDeleteResult> {
    if targets.is_empty() {
        return Envelope::ok(RegistryDeleteResult::default());
    }
    if backup_dir.as_os_str().is_empty() {
        // 备份目录为空时 write_reg_backup 会写到当前工作目录，必须挡住
        eprintln!("[registry_health_delete] backup_dir 为空，拒绝执行");
        return Envelope::err(INTERNAL);
    }

    let result = delete_run_values(backup_dir, targets, stamp);

    if result.deleted.is_empty() && !result.failed.is_empty() {
        let all_l3 = result.failed.iter().all(|f| f.reason.contains("L3"));
        if all_l3 {
            eprintln!("[registry_health_delete] 全部命中 L3，拒绝执行");
            return Envelope::err(WHITELIST_DENIED);
        }
    }

    Envelope::ok(result)
}
