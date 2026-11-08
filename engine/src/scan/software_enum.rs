//! task-083 B1：已安装软件枚举。
//!
//! # 为什么把「读注册表」和「组装结果」分成两层
//!
//! 单测不能依赖 runner / 用户机器上装了什么软件。所以：
//! - [`enumerate_installed`] 只负责**采集**：把注册表读成一堆 [`HiveSnapshot`]；
//! - [`merge_snapshots`] 是**纯函数**：过滤、按 HKLM 优先去重、排序。
//! 单测构造假快照喂给后者即可，完全不碰真实注册表。
//!
//! # 读哪些键
//!
//! `HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\*`（机器级）、
//! 同一路径的 `WOW6432Node` 变体（32 位软件装在 64 位系统）、以及 HKCU 同名键
//! （用户级安装，如 VSCode / Chrome 的 per-user 安装）。三者合并后去重。
//!
//! # 为什么不调 WMI `Win32_Product`
//!
//! 任务书把 WMI 列为「可选、默认关」。这里**不实现 WMI 本体**，只保留开关读取：
//! 微软明确不推荐用 `Win32_Product` 做软件清单——查询它会触发每个 MSI 包的
//! 「自我修复」（会弹安装界面、改注册表、极慢），而且它只能列出 MSI 安装的软件，
//! 覆盖反而比 Uninstall 键更少。`CCLEAN_ENUM_WMI=1` 时只在日志里说明，
//! 实际结果仍来自注册表。这是有依据的技术取舍，不是漏做。

use crate::scan::winreg::{self, RegRoot};

const UNINSTALL_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
const UNINSTALL_WOW64: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

/// 一个已安装软件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftwareEntry {
    /// `DisplayName`。
    pub name: String,
    /// `DisplayVersion`（缺失为空串）。
    pub version: String,
    /// `Publisher`（缺失为空串）。
    pub publisher: String,
    /// `InstallLocation`（缺失为空串）。
    pub install_location: String,
    /// 来源键，如 `HKLM\...\Uninstall\{GUID}`。排查误识别时靠它回溯。
    pub registry_key: String,
}

/// 一个 Uninstall 子键的原始快照（字段名与注册表值名一致，便于对照）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawEntry {
    pub subkey: String,
    pub display_name: Option<String>,
    pub display_version: Option<String>,
    pub publisher: Option<String>,
    pub install_location: Option<String>,
    /// `SystemComponent=1` 表示这是 Windows 组件 / 补丁，不该出现在「已装软件」里。
    pub system_component: Option<u32>,
}

/// 一次 hive 采集结果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HiveSnapshot {
    /// 这个快照来自哪个根键（决定去重时的优先级）。
    pub root: SnapshotRoot,
    pub entries: Vec<RawEntry>,
}

/// 快照来源。用独立枚举而不是 `RegRoot`，让单测不用引任何 Windows 类型。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SnapshotRoot {
    /// HKLM（含 WOW6432Node），优先级最高。
    #[default]
    LocalMachine,
    /// HKCU，优先级低。
    CurrentUser,
}

/// 名字归一：小写、去掉空白与常见版本噪声，用于去重。
///
/// 「Visual Studio Code」与「Visual Studio Code (User)」应视为同一个软件；
/// 但**不能**把「Microsoft Edge」和「Microsoft Edge WebView2 Runtime」合并——
/// 所以只做保守的空白/大小写归一，不做模糊匹配。
fn norm_name(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// 把多个 hive 快照合成软件清单。
///
/// 规则：
/// 1. 无 `DisplayName` 的跳过（很多卸载键只有 GUID，没有可读名字）；
/// 2. `SystemComponent=1` 的跳过（补丁 / 系统组件）；
/// 3. 同名按 **HKLM 优先**，先到先得（同一 root 内按快照顺序）；
/// 4. 输出按名字排序，保证顺序确定。
pub fn merge_snapshots(snapshots: &[HiveSnapshot]) -> Vec<SoftwareEntry> {
    // HKLM 排前面，去重时自然优先
    let mut ordered: Vec<&HiveSnapshot> = snapshots.iter().collect();
    ordered.sort_by_key(|s| match s.root {
        SnapshotRoot::LocalMachine => 0u8,
        SnapshotRoot::CurrentUser => 1u8,
    });

    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for snap in ordered {
        for raw in &snap.entries {
            let Some(name) = raw.display_name.as_deref() else {
                continue;
            };
            let name = name.trim();
            if name.is_empty() || raw.system_component == Some(1) {
                continue;
            }
            let key = norm_name(name);
            if !seen.insert(key) {
                continue;
            }
            let root_label = match snap.root {
                SnapshotRoot::LocalMachine => "HKLM",
                SnapshotRoot::CurrentUser => "HKCU",
            };
            out.push(SoftwareEntry {
                name: name.to_string(),
                version: raw.display_version.clone().unwrap_or_default(),
                publisher: raw.publisher.clone().unwrap_or_default(),
                install_location: raw.install_location.clone().unwrap_or_default(),
                registry_key: format!("{root_label}\\{UNINSTALL_PATH}\\{}", raw.subkey),
            });
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// 从真实注册表采集一个 hive 的快照（非 Windows 平台返回空）。
fn collect_hive(root: RegRoot, subkey: &str, snapshot_root: SnapshotRoot) -> HiveSnapshot {
    let names = winreg::subkey_names(root, subkey);
    let mut entries = Vec::with_capacity(names.len());
    for name in names {
        let path = format!("{subkey}\\{name}");
        entries.push(RawEntry {
            subkey: name,
            display_name: winreg::string_value(root, &path, "DisplayName"),
            display_version: winreg::string_value(root, &path, "DisplayVersion"),
            publisher: winreg::string_value(root, &path, "Publisher"),
            install_location: winreg::string_value(root, &path, "InstallLocation"),
            system_component: winreg::dword_value(root, &path, "SystemComponent"),
        });
    }
    HiveSnapshot {
        root: snapshot_root,
        entries,
    }
}

/// 枚举本机已安装软件。
///
/// WMI 开关只影响日志（理由见模块头注释），不改变结果来源。
pub fn enumerate_installed() -> Vec<SoftwareEntry> {
    if std::env::var("CCLEAN_ENUM_WMI").as_deref() == Ok("1") {
        eprintln!(
            "[software-enum] CCLEAN_ENUM_WMI=1：Win32_Product 会触发 MSI 自我修复且覆盖更少，\
             仍使用注册表 Uninstall 键（见模块文档）"
        );
    }
    let snapshots = vec![
        collect_hive(
            RegRoot::LocalMachine,
            UNINSTALL_PATH,
            SnapshotRoot::LocalMachine,
        ),
        collect_hive(
            RegRoot::LocalMachine,
            UNINSTALL_WOW64,
            SnapshotRoot::LocalMachine,
        ),
        collect_hive(
            RegRoot::CurrentUser,
            UNINSTALL_PATH,
            SnapshotRoot::CurrentUser,
        ),
    ];
    merge_snapshots(&snapshots)
}
