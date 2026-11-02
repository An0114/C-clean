//! Rust 原生扫描模块（task-074 起，架构债 R5）。
//!
//! # 为什么要有这一层
//!
//! 5 个扫描模块原本都是 `.ps1`：spawn `powershell.exe`、靠 stdout 的 NDJSON 回传。
//! 这条链路脆在四处，而且四处都真实踩过：
//!
//! 1. **编码**：ps1 往管道写的是控制台代码页（中文系统 GBK），不是 UTF-8；
//! 2. **执行策略**：`-ExecutionPolicy Bypass` 本身就是安全软件眼里的可疑行为；
//! 3. **脚本文件丢失**：脚本从 `resource_dir` 读，安装包漏带就整块功能失效；
//! 4. **行为漂移**：不同 PowerShell 版本对 `Get-ChildItem -Force` 的处理不完全一样。
//!
//! 改成 Rust 后这些全没了，而且扫描逻辑进了类型系统，能被单测覆盖。
//!
//! # 为什么按模块逐个迁移，而不是一次全改
//!
//! 5 个一起改，出问题无法判断是哪一处引入的；而且 temp 之外的 4 个模块各有
//! 自己的边界（浏览器缓存路径随版本变、prefetch 需要管理员、Windows 更新目录
//! 属 SYSTEM），迁移时要逐个确认。temp 最简单、影响面最小，先拿它当模板。
//!
//! 每个模块迁移完都保留 ps1 作为回退（环境变量控制），确认稳定后再删脚本。

pub mod temp;

/// 一个待清理项（Rust 侧扫描模块的统一产出）。
///
/// # 为什么 `risk_level` 是 `i64` 而不是枚举
///
/// ps1 协议里就是 `1` / `2` 这样的整数，`runner::ScanRunner::insert_item`
/// 负责映射成 `'L1'` / `'L2'` 文本入库。这里保持整数，两条路径（Rust / ps1）
/// 的入库逻辑才能**完全共用一份**——换成枚举就要多一层转换，
/// 而「多一层转换」正是两套实现开始漂移的起点。
///
/// 取值约定（与 ps1 一致）：`0` = L0 / `1` = L1 / `2` = L2。L3 不允许出现在
/// 扫描结果里（白名单层会拦，见 `execute::whitelist`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanItem {
    /// 模块代码（`temp` / `browser_cache` / ...）。对应 `cleanup_item.module_code`。
    pub module_code: String,
    /// 类别键（`user_temp` / `windows_temp` / ...）。对应 `cleanup_item.category`。
    pub category: String,
    /// 绝对路径。
    pub path: String,
    /// 文件大小（字节）。
    pub size_bytes: i64,
    /// 风险等级整数（0/1/2）。
    pub risk_level: i64,
}

impl ScanItem {
    /// 构造一个条目。字段顺序与 ps1 的 `Write-Item` 一致，便于对照。
    pub fn new(
        module_code: &str,
        category: &str,
        path: String,
        size_bytes: i64,
        risk_level: i64,
    ) -> Self {
        Self {
            module_code: module_code.to_string(),
            category: category.to_string(),
            path,
            size_bytes,
            risk_level,
        }
    }
}

/// 判断「这个文件/目录是不是 reparse point（符号链接 / junction）」。
///
/// **必须检查，不能只靠 `is_symlink()`**：Windows 的 junction（目录联接）
/// 在 Rust 里 `file_type().is_symlink()` 返回 **false**，但它是 reparse point，
/// 递归进去会绕圈（`C:\Users\All Users` → `C:\ProgramData` 这种）。
///
/// ps1 侧用 `$_.LinkType` 判断，那个属性对 symlink 和 junction 都非空，
/// 所以两边语义一致。
pub fn is_reparse_point(md: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // FILE_ATTRIBUTE_REPARSE_POINT = 0x400
        if md.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    md.file_type().is_symlink()
}
