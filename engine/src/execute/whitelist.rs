//! 内置 L3 红线白名单匹配器（硬编码，永不被外部规则移除）。
//!
//! 对应 ARCHITECTURE R3「规则更新不得移除内置红线」与 PRD P0-7：
//! 用户个人目录 / 浏览器个人数据 / 系统关键路径 / 动作级禁令。
//! 匹配全部用字符串比较 + 前缀判断，不引入 regex。

use super::normalize::NormalizedPath;

/// 用户个人目录（文件系统真实名 + 中文本地化别名，补充匹配）。
/// 取「users/<任意用户名>/<本列表>」结构，不写死真实用户名。
const PERSONAL_DIRS: [&str; 12] = [
    "desktop",
    "documents",
    "downloads",
    "pictures",
    "videos",
    "music",
    // 中文本地化文件夹名（Windows 显示名；NTFS 真实名仍为英文，
    // 这里作补充匹配，防御某些工具直接用显示名当路径传上来）。
    "桌面",
    "文档",
    "下载",
    "图片",
    "视频",
    "音乐",
];

/// 浏览器敏感文件（精确文件名，不分父目录）。
const BROWSER_SENSITIVE_FILES: [&str; 3] = ["bookmarks", "login data", "history"];

/// 路径被内置红线拒绝时的结构化原因（供上层审计与 UI 展示）。
///
/// `PathTraversal` 表示规范化阶段已拒绝（不会出现在 execute 返回的路径决策里，
/// 仅作为枚举保留，便于上层把「规范化失败」与「命中 L3」区分统计）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenialReason {
    /// 桌面 / 文档 / 下载 / 图片 / 视频 / 音乐（含中文本地化文件夹名）。
    L3PersonalDir,
    /// 浏览器 User Data / Firefox Profiles 主目录（前缀命中即拒）。
    L3BrowserProfile,
    /// 任意路径下名为 Bookmarks / Login Data / History 的敏感文件。
    L3BrowserSensitive,
    /// System32 / Windows\Installer / pagefile.sys / swapfile.sys。
    L3SystemPath,
    /// 动作级禁令（如 DISM `/ResetBase`）。
    L3ReservedAction,
    /// 规范化阶段已拒（枚举保留，正常不出现在返回结果中）。
    PathTraversal,
}

/// 对一条已规范化路径跑全部内置红线；通过返回 Ok(())，命中返回对应拒绝原因。
pub fn classify(p: &NormalizedPath) -> Result<(), DenialReason> {
    let lower = p.normalized().to_lowercase();
    let comps = p.lower_components();

    // 1. 系统关键路径：System32 及后代 / Installer 及后代 / pagefile / swapfile 本体。
    if comps.starts_with(&["windows".into(), "system32".into()])
        || comps.starts_with(&["windows".into(), "installer".into()])
        || comps == vec!["pagefile.sys".to_string()]
        || comps == vec!["swapfile.sys".to_string()]
    {
        return Err(DenialReason::L3SystemPath);
    }

    // 2. 浏览器主目录前缀：Chrome/Edge 的 User Data（可作路径本身）、Firefox 的 Profiles。
    if lower.contains(r"\user data\")
        || lower.ends_with(r"\user data")
        || lower.contains(r"\mozilla\firefox\")
    {
        return Err(DenialReason::L3BrowserProfile);
    }

    // 3. 用户个人目录：users/<用户名/<个人目录>（任意盘符）。
    for (i, seg) in comps.iter().enumerate() {
        if PERSONAL_DIRS.contains(&seg.as_str()) && i >= 2 && comps[i - 2] == "users" {
            return Err(DenialReason::L3PersonalDir);
        }
    }

    // 4. 浏览器敏感文件：任意父目录下的 Bookmarks / Login Data / History。
    if let Some(file) = comps.last() {
        if BROWSER_SENSITIVE_FILES.contains(&file.as_str()) {
            return Err(DenialReason::L3BrowserSensitive);
        }
    }

    Ok(())
}

/// 动作级禁令检查：拒绝 DISM `/ResetBase` 等高危动作字符串。
/// （`/ResetBase` 字面量只允许出现在本拒绝分支与测试代码中。）
pub fn check_action(action: &str) -> Result<(), DenialReason> {
    if action.to_lowercase().contains("/resetbase") {
        return Err(DenialReason::L3ReservedAction);
    }
    Ok(())
}

/// 内置红线 + 外部红线并集匹配（task-005 §F 最小扩展）。
///
/// 先跑内置 [`classify`]（行为不变），再跑 `external` 前缀匹配：
/// 外部红线只能**新增**拒绝，不能放宽内置红线。`external` 元素为已展开的
/// 绝对路径（反斜杠），比较大小写不敏感，要求路径等于该前缀或以 `\` 延伸。
/// 外部命中统一归为 [`DenialReason::L3SystemPath`]（外部硬编码红线桶）。
pub fn classify_with_external(p: &NormalizedPath, external: &[String]) -> Result<(), DenialReason> {
    classify(p)?;
    let lower = p.normalized().to_lowercase();
    for ext in external {
        let ext_lower = ext.to_lowercase();
        if lower == ext_lower || lower.starts_with(&format!("{ext_lower}\\")) {
            return Err(DenialReason::L3SystemPath);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execute::normalize::normalize;

    fn classify_of(raw: &str) -> Result<(), DenialReason> {
        classify(&normalize(raw).unwrap())
    }

    #[test]
    fn personal_dirs_are_blocked() {
        assert_eq!(
            classify_of(r"C:\Users\alice\Desktop\a.docx"),
            Err(DenialReason::L3PersonalDir)
        );
        assert_eq!(
            classify_of(r"D:\Users\bob\Downloads\a.exe"),
            Err(DenialReason::L3PersonalDir)
        );
    }

    #[test]
    fn system_paths_are_blocked() {
        assert_eq!(
            classify_of(r"C:\Windows\System32\kernel32.dll"),
            Err(DenialReason::L3SystemPath)
        );
        assert_eq!(
            classify_of(r"C:\pagefile.sys"),
            Err(DenialReason::L3SystemPath)
        );
    }

    #[test]
    fn cache_dir_is_allowed() {
        assert!(classify_of(r"C:\Windows\Temp\update.log").is_ok());
    }

    #[test]
    fn resetbase_action_is_blocked() {
        assert_eq!(
            check_action("dism /ResetBase"),
            Err(DenialReason::L3ReservedAction)
        );
        assert!(check_action("dism /cleanup-image").is_ok());
    }
}
