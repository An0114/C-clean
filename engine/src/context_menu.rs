//! Windows 右键菜单集成（task-086 5.7）+ 单目录一次性扫描。
//!
//! # 注册位置
//!
//! `HKCU\Software\Classes\Directory\shell\CClean`：
//! - 默认值 = 菜单显示名；
//! - `Icon` = 应用 exe（资源管理器会用它画菜单图标）；
//! - `command` 子键默认值 = `"<exe>" --scan-path "%1"`。
//!
//! 用 `HKCU` 而不是 `HKLM`：写当前用户不需要管理员权限，卸载时也只影响本用户；
//! `Directory\shell` 会同时出现在**文件夹右键**和文件夹背景右键里，符合
//! 「用 C-clean 清理此文件夹」的语义。
//!
//! # 为什么命令里带 `--scan-path`
//!
//! 复用主程序而不是另做一个 CLI：菜单点击后由 [`APP`] 主进程启动、读取参数、
//! 扫描目标目录。参数解析在 `app/src-tauri/src/lib.rs`，本模块只负责
//! 「拼命令行」与「读写注册表」——纯字符串部分可单测，不碰真实注册表。
//!
//! # 一次性扫描的安全边界
//!
//! [`collect_items`] 走与常规扫描相同的 [`crate::scan::modules::scan_dir`]，
//! 并对**每个真实文件**再过一遍内置红线；目标目录本身也先过一次红线。
//! 风险等级固定 L2：右键菜单是用户主动指定的任意目录，必须逐项确认，
//! 不允许被「一键全选」当成安全缓存。

use std::path::Path;

use crate::execute::normalize::normalize;
use crate::execute::resolve::resolve_to_real;
use crate::execute::whitelist::classify;
use crate::scan::modules::{scan_dir, ScanItem};

/// 注册表子键（相对 `HKEY_CURRENT_USER`）。
pub const REG_SUBKEY: &str = r"Software\Classes\Directory\shell\CClean";

/// 启动参数：告诉主进程「启动后扫描这个目录」。
pub const ARG_SCAN_PATH: &str = "--scan-path";

/// 右键菜单显示名。
pub const MENU_LABEL: &str = "用 C-Disk Cleaner 清理此文件夹";

/// 一次性扫描的模块代码（与内置模块区分，便于历史里识别来源）。
pub const MODULE_CODE: &str = "context_menu";
/// 一次性扫描的类别键。
pub const CATEGORY: &str = "context_menu";
/// 固定 L2：用户指定的任意目录必须逐项确认。
pub const RISK_L2: i64 = 2;

/// 拼注册表 `command` 默认值。
///
/// `%1` 是资源管理器传入的目录（带引号由这里负责——路径含空格时不能裸拼）。
pub fn build_command(exe: &Path) -> String {
    format!("\"{}\" {} \"%1\"", exe.display(), ARG_SCAN_PATH)
}

/// 菜单是否已注册（读默认值是否非空）。
pub fn is_installed() -> bool {
    crate::scan::winreg::string_value(crate::scan::winreg::RegRoot::CurrentUser, REG_SUBKEY, "")
        .is_some()
}

/// 注册右键菜单。重复调用是幂等的（覆盖同名值）。
pub fn install(exe: &Path) -> Result<(), String> {
    use crate::scan::winreg::{set_string_value, RegRoot};
    set_string_value(RegRoot::CurrentUser, REG_SUBKEY, "", MENU_LABEL)?;
    set_string_value(
        RegRoot::CurrentUser,
        REG_SUBKEY,
        "Icon",
        &exe.to_string_lossy(),
    )?;
    set_string_value(
        RegRoot::CurrentUser,
        &format!(r"{REG_SUBKEY}\command"),
        "",
        &build_command(exe),
    )
}

/// 注销右键菜单（删整棵子树；不存在视为成功）。
pub fn uninstall() -> Result<(), String> {
    crate::scan::winreg::delete_tree(crate::scan::winreg::RegRoot::CurrentUser, REG_SUBKEY)
}

/// 扫描一个用户指定目录，返回待清理项（已过 L3 红线）。
///
/// 路径先 `normalize` → `resolve_to_real` → 再用**真实路径**判红线：
/// 与删除链路完全同一套判定，避免「右键扫出来的项删不掉」。
/// 目录不存在 / 不是目录 / 命中红线都返回明确错误。
pub fn collect_items(root: &str) -> Result<Vec<ScanItem>, String> {
    let np = normalize(root).map_err(|e| format!("路径不合法: {e}"))?;
    let resolved = resolve_to_real(&np).map_err(|e| format!("路径解析失败: {e}"))?;
    let real = normalize(&resolved.real).map_err(|e| format!("真实路径不合法: {e}"))?;

    if let Err(reason) = classify(&real) {
        return Err(format!("目标目录位于受保护位置，拒绝扫描: {reason:?}"));
    }
    if !Path::new(&resolved.real).is_dir() {
        return Err(format!("目标不是文件夹: {}", resolved.real));
    }

    // 逐文件红线：目录本身合法不代表里面没有个人文件（用户可能选中 C:\Users）。
    let items: Vec<ScanItem> = scan_dir(Path::new(&resolved.real), MODULE_CODE, CATEGORY, RISK_L2)
        .into_iter()
        .filter(|it| match normalize(&it.path) {
            Ok(n) => classify(&n).is_ok(),
            Err(_) => false,
        })
        .collect();

    Ok(items)
}

/// 一次性扫描结果（IPC 响应体）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextScanResp {
    /// 新建的扫描会话 id（Items 页可按它筛选）。
    pub session_id: i64,
    /// 入库条目数。
    pub items: i64,
    /// 合计字节。
    pub bytes: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_command_quotes_exe_and_placeholder() {
        let cmd = build_command(Path::new(
            r"C:\Program Files\C-Disk Cleaner\cdisk-cleaner.exe",
        ));
        assert_eq!(
            cmd,
            r#""C:\Program Files\C-Disk Cleaner\cdisk-cleaner.exe" --scan-path "%1""#
        );
    }

    #[test]
    fn registry_key_is_per_user_directory_shell() {
        assert!(REG_SUBKEY.starts_with(r"Software\Classes\Directory\shell\"));
        assert!(!REG_SUBKEY.contains("HKLM"));
    }

    #[test]
    fn collect_items_rejects_protected_directory() {
        // 不依赖目录真实存在：红线判定在 is_dir 之前。
        let err = collect_items(r"C:\Windows\System32").unwrap_err();
        assert!(err.contains("受保护"), "实际: {err}");
    }

    #[test]
    fn collect_items_scans_temp_dir_as_l2() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.tmp"), vec![b'x'; 16]).unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub").join("b.log"), b"abc").unwrap();

        let items = collect_items(&dir.path().to_string_lossy()).expect("临时目录应可扫描");
        assert_eq!(items.len(), 2, "实际: {items:#?}");
        assert!(items.iter().all(|i| i.module_code == MODULE_CODE));
        assert!(items.iter().all(|i| i.risk_level == RISK_L2));
        assert!(items.iter().all(|i| i.category == CATEGORY));
    }
}
