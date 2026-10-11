//! 内置 L3 红线白名单匹配器（硬编码，永不被外部规则移除）。
//!
//! 提交 2 仅暴露拒绝原因枚举骨架，匹配逻辑见提交 4。

/// 路径/动作被内置红线拒绝时的结构化原因（供上层审计与 UI 展示）。
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
