//! 残留扫描（D 模块，task-067）：注册表卸载列表 ↔ 文件目录交叉比对。
//!
//! # 两类残留
//!
//! | 类型 | 定义 | 置信度 |
//! |---|---|---|
//! | **A 注册表残留** | 卸载项里有 `InstallLocation`，但该目录已不存在 | **高**（路径明确不存在，可判定） |
//! | **B 文件残留** | 安装目录下有文件夹，但注册表里找不到对应的卸载项 | **低**（见下） |
//!
//! # 为什么类型 B 的置信度只能是「低」
//!
//! 「注册表里没有」**不等于**「程序已卸载」。以下情况全都会被误判成孤儿：
//!
//! - **绿色软件 / 便携版**：本来就不写卸载项（`C:\Program Files\7-Zip` 手动解压版）；
//! - **`InstallLocation` 为空**：大量程序不写这个值，只能靠名字猜；
//! - **`InstallLocation` 指向父目录**：装在 `...\Vendor\Product`，注册表只记 `...\Vendor`；
//! - **微软商店应用**：装在 `WindowsApps`，不在 Uninstall 里；
//! - **用户自建目录**：`C:\Program Files\MyScripts` 根本不是安装的。
//!
//! 更危险的是：`engine::execute::whitelist` 的 L3 红线只覆盖个人目录 / 浏览器数据 /
//! System32 / Installer / pagefile，**`C:\Program Files\*` 不在红线内**。也就是说
//! 一旦误判，白名单拦不住。所以：
//!
//! 1. 类型 B 一律标 `confidence = "low"` + `risk_level = "L2"`，UI 必须逐项确认；
//! 2. 删除走**保险箱**（7 天可恢复），不是直接删；
//! 3. 删除前仍过 [`crate::execute::delete::execute`] 的唯一入口红线校验。
//!
//! 宁可漏报（把孤儿当成正常程序），不可误报（把正常程序当孤儿）。

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::execute::delete::{execute as redline_execute, Deleter, ExecuteOutcome, ExecuteRequest};
use crate::execute::normalize::normalize;
use crate::execute::normalize::NormalizedPath;
use crate::execute::safe_box::SafeBoxDeleter;
use crate::scan::big_files::dir_total_bytes;
use crate::scan::winreg::{self, RegRoot};

/// 残留类型：注册表残留（类型 A）。
pub const RESIDUAL_REGISTRY: &str = "registry";
/// 残留类型：文件残留（类型 B）。
pub const RESIDUAL_FILES: &str = "files";

/// 置信度：高（路径明确不存在）。
pub const CONFIDENCE_HIGH: &str = "high";
/// 置信度：低（只能靠「注册表里找不到」推断，会误伤）。
pub const CONFIDENCE_LOW: &str = "low";

/// 风险等级：L2（需逐项确认）。复用项目既有的 L0-L3 分级语义。
pub const RISK_L2: &str = "L2";

/// 单次扫描统计目录大小的总时间预算。
///
/// 孤儿目录可能有几十个，每个都要递归求和；不设上限时一次点击能转几分钟。
/// 超预算的目录 `size_bytes` 记 0 并在 note 里说明「未统计」——
/// **绝不用 0 冒充「空的」**，UI 会按 note 区分显示。
const SIZE_BUDGET: Duration = Duration::from_secs(15);

/// 一条注册表卸载项（只保留判定需要的字段）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UninstallEntry {
    /// `DisplayName`。「程序和功能」里显示的名字。
    pub display_name: String,
    /// `InstallLocation`（可能为空 / 指向父目录 / 带引号）。
    pub install_location: Option<String>,
    /// 完整注册表键路径，形如 `HKLM\Software\...\Uninstall\{GUID}`。
    pub registry_key: String,
}

/// 一个安装目录（只取顶层子目录，不递归）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallDir {
    /// 目录完整路径。
    pub path: PathBuf,
    /// 目录名（用于名称比对）。
    pub name: String,
}

/// 一条残留项。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResidualItem {
    /// 显示名（类型 A 用 `DisplayName`，类型 B 用文件夹名）。
    pub name: String,
    /// 路径（类型 A 是注册表记的 `InstallLocation`，类型 B 是文件夹路径）。
    pub path: String,
    /// 残留类型，取 [`RESIDUAL_REGISTRY`] / [`RESIDUAL_FILES`]。
    pub residual_type: String,
    /// 占用字节数。类型 A 恒为 0（目录都不存在了）；类型 B 为递归求和结果。
    pub size_bytes: u64,
    /// 注册表键路径（仅类型 A 有）。
    pub registry_key: Option<String>,
    /// 置信度，取 [`CONFIDENCE_HIGH`] / [`CONFIDENCE_LOW`]。
    pub confidence: String,
    /// 风险等级（`L2` = 需逐项确认）。
    pub risk_level: String,
    /// 人话说明（UI 直接展示）。
    pub note: String,
}

/// 一次残留扫描的完整结果。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResidualReport {
    /// 类型 A：注册表有记录、目录已不存在。
    pub registry_items: Vec<ResidualItem>,
    /// 类型 B：目录存在、注册表找不到对应卸载项。
    pub file_items: Vec<ResidualItem>,
    /// 两类合计字节数（类型 A 恒 0，所以实际等于类型 B 之和）。
    pub total_bytes: u64,
    /// 读到了多少个有效卸载项（过滤掉系统组件/补丁之后）。
    pub registry_entries: u64,
    /// 看了多少个安装目录。
    pub dirs_scanned: u64,
    /// 是否有目录因超预算没统计大小。
    pub size_truncated: bool,
    /// 扫描耗时（毫秒）。
    pub elapsed_ms: u64,
}

/// 删除（移入保险箱）结果。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResidualDeleteResult {
    /// 成功移入保险箱的路径。
    pub moved: Vec<String>,
    /// 失败的路径 + 原因。
    pub failed: Vec<ResidualDeleteFailure>,
    /// 移入的总字节数。
    pub bytes_moved: u64,
}

/// 单条删除失败。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResidualDeleteFailure {
    /// 失败的路径。
    pub path: String,
    /// 失败原因（人话）。
    pub reason: String,
}

// ===================== 注册表读取 =====================

/// 卸载项注册表路径（64 位视图）。
const UNINSTALL_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
/// 32 位程序的卸载项在 WOW6432Node 下（64 位进程读 `Software` 看不到它们）。
const UNINSTALL_SUBKEY_WOW: &str =
    r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

/// 读注册表卸载列表（HKLM 64 位 + HKLM 32 位 + HKCU）。
///
/// 过滤掉三类「不是可卸载程序」的项，否则会把补丁/运行库全列进来：
/// - `DisplayName` 为空（补丁、系统组件）；
/// - `SystemComponent == 1`（微软约定：不在「程序和功能」里显示）；
/// - `ParentKeyName` 存在（是某个程序下的子组件）。
pub fn scan_registry_uninstall() -> Vec<UninstallEntry> {
    #[cfg(windows)]
    {
        let mut out = Vec::new();
        out.extend(read_uninstall_root(
            RegRoot::LocalMachine,
            "HKLM",
            UNINSTALL_SUBKEY,
        ));
        out.extend(read_uninstall_root(
            RegRoot::LocalMachine,
            "HKLM",
            UNINSTALL_SUBKEY_WOW,
        ));
        out.extend(read_uninstall_root(
            RegRoot::CurrentUser,
            "HKCU",
            UNINSTALL_SUBKEY,
        ));
        out
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
fn read_uninstall_root(root: RegRoot, root_label: &str, subkey: &str) -> Vec<UninstallEntry> {
    let mut out = Vec::new();
    for sub in winreg::subkey_names(root, subkey) {
        let full = format!(r"{subkey}\{sub}");
        let display_name = match winreg::string_value(root, &full, "DisplayName") {
            Some(v) if !v.trim().is_empty() => v.trim().to_string(),
            // 没有 DisplayName 的是补丁/系统组件，不是「可卸载程序」
            _ => continue,
        };
        if winreg::dword_value(root, &full, "SystemComponent") == Some(1) {
            continue;
        }
        if winreg::string_value(root, &full, "ParentKeyName").is_some() {
            continue;
        }
        out.push(UninstallEntry {
            display_name,
            install_location: winreg::string_value(root, &full, "InstallLocation")
                .map(|s| clean_location(&s)),
            registry_key: format!(r"{root_label}\{full}"),
        });
    }
    out
}

#[cfg(not(windows))]
fn read_uninstall_root(_root: RegRoot, _root_label: &str, _subkey: &str) -> Vec<UninstallEntry> {
    Vec::new()
}

/// 清理 `InstallLocation` 的常见脏数据：去引号、去尾部反斜杠、去环境变量。
///
/// 真实数据里见过 `"C:\Program Files\Foo\"`（带引号）、`C:\Foo\`（尾斜杠）、
/// `%ProgramFiles%\Foo`（未展开）。前两种直接裁，第三种用环境变量展开。
pub fn clean_location(raw: &str) -> String {
    let mut s = raw.trim().trim_matches('"').trim().to_string();
    s = expand_env(&s);
    // 去掉尾部的分隔符，便于后面统一按 `\` 比较
    while s.len() > 3 && (s.ends_with('\\') || s.ends_with('/')) {
        s.pop();
    }
    s
}

/// 展开 `%VAR%` 形式的环境变量（只处理第一层，够用）。
///
/// 公开是为了单测能直接验证「展开不了要原样保留」这条——把路径吃掉比不展开更糟。
pub fn expand_env(s: &str) -> String {
    if !s.contains('%') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let name = &after[..end];
                match std::env::var(name) {
                    Ok(v) => out.push_str(&v),
                    // 展开不了就原样保留，别把路径吃掉
                    Err(_) => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

// ===================== 安装目录枚举 =====================

/// 安装目录根（只取顶层子目录）。
///
/// # 为什么只取 Program Files，**不取 AppData**（真机实测后的修正）
///
/// 最初把 `%LOCALAPPDATA%` / `%APPDATA%` 也当安装目录扫，本机实测直接产出
/// **182 条类型 B**，其中绝大多数是明显的误报：
///
/// ```text
/// C://Users//<u>//AppData//Local//Huawei        ← 华为 SDK，装了
/// C://Users//<u>//AppData//Roaming//LarkShell   ← 飞书，装了
/// C://Users//<u>//AppData//Roaming//Tencent     ← 腾讯系，装了
/// C://Users//<u>//AppData//Roaming//JetBrains   ← JetBrains，装了
/// ```
///
/// 原因很简单：**AppData 是「应用数据」目录，不是「程序安装」目录**，
/// 里面的东西本来就不写卸载项。把它当孤儿报出来，等于把「用户的正常数据」
/// 说成「残留」，比不报更糟——用户照着删一次就出事了。
///
/// 真正装在 AppData 里的程序（VS Code、Discord 这类 Electron 应用）
/// 卸载项通常在 HKCU，靠 [`has_owner`] 的名称匹配就能认出来；认不出的
/// 那几个不值得为它引入 180 条噪音。
///
/// # 为什么只取顶层
///
/// `Program Files\Vendor\Product` 这种两层结构里，「Vendor」目录本身没有
/// 卸载项，但删掉它会带走 Vendor 下所有产品——只列顶层能把风险限制在
/// 「一个顶层文件夹」。
pub fn install_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    // ProgramW6432 与 ProgramFiles 通常相同，用 contains 去重
    for var in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
        if let Ok(p) = std::env::var(var) {
            let pb = PathBuf::from(p);
            if pb.is_dir() && !roots.contains(&pb) {
                roots.push(pb);
            }
        }
    }
    roots
}

/// 顶层目录名黑名单：这些是系统/共享目录，永远不是「某个程序的残留」。
const SKIP_DIR_NAMES: &[&str] = &[
    "common files",
    "windowsapps",
    "modifiablewindowsapps",
    "windows defender",
    // 系统级组件：由 Windows 功能 / 其他产品带进来，本身没有独立卸载项。
    // 这一批是真机实测后逐个补的（每条都对应一个实际误报），不补的话
    // 用户会看到「Windows PowerShell 是残留」这种荒唐结论。
    "windows kits",
    "microsoft sdks",
    "wsl",
    "dotnet",
    "msixmgr",
    "windows security",
    "windows defender advanced threat protection",
    "hyper-v",
    "windows photo viewer",
    "windowspowershell",
    "application verifier",
    // 显卡驱动包：跟着驱动走，没有卸载项
    "amd",
    "nvidia corporation",
    "windows nt",
    "windows mail",
    "windows media player",
    "windows portable devices",
    "windows sidebar",
    "internet explorer",
    "microsoft", // 太宽泛，里面全是系统组件与运行库
    "microsoft office",
    "microsoft.net",
    "msbuild",
    "reference assemblies",
    "uninstall information",
    "package cache",
    "temp",
    "temporary internet files",
    "microsoft\\windows",
    "connecteddevicesplatform",
    "comms",
    "d3dscache",
    "packages",
    "placeholdertilelogofolder",
    "publishers",
    "virtualstore",
    "crashdumps",
    "elevateddiagnostics",
];

/// 目录名是否要跳过（大小写不敏感）。
pub fn should_skip_dir_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SKIP_DIR_NAMES.contains(&lower.as_str())
}

/// 枚举一个根目录下的顶层子目录（跳过黑名单与 reparse point）。
pub fn list_install_dirs(root: &Path) -> Vec<InstallDir> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return out;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        // 不跟随 junction：AppData\Local 下一堆指向别处的链接，跟进去会重复计数
        if ft.is_symlink() || !ft.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip_dir_name(&name) {
            continue;
        }
        out.push(InstallDir {
            path: entry.path(),
            name,
        });
    }
    out
}

// ===================== 交叉比对 =====================

/// 名称归一化：只保留字母数字（含 CJK）并转小写。
///
/// `Microsoft Visual C++ 2015 (x64)` → `microsoftvisualc2015x64`
/// 这样「带版本号/架构后缀」的名字能和文件夹名对上。
pub fn normalize_name(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_alphanumeric() {
            for lc in c.to_lowercase() {
                out.push(lc);
            }
        }
    }
    out
}

/// 路径归一化（比较用）：小写 + 反斜杠 + 去尾部分隔符。
fn norm_path(s: &str) -> String {
    let mut p = s.replace('/', "\\").to_ascii_lowercase();
    while p.len() > 3 && p.ends_with('\\') {
        p.pop();
    }
    p
}

/// 一个安装目录能否在卸载列表里找到「可能的归属」。
///
/// 三级判定，**任一级命中就算「不是孤儿」**（宁可漏报）：
/// 1. 路径级：`InstallLocation` 与目录互为前缀（含相等）；
/// 2. 目录名级：`InstallLocation` 的末段等于目录名；
/// 3. 名称级：`DisplayName` 与目录名归一化后双向包含，且较短串 ≥ 4 字符。
fn has_owner(dir: &InstallDir, entries: &[UninstallEntry]) -> bool {
    let dir_path = norm_path(&dir.path.to_string_lossy());
    let dir_name_norm = normalize_name(&dir.name);

    for e in entries {
        if let Some(loc) = &e.install_location {
            let loc = loc.trim();
            if loc.is_empty() {
                continue;
            }
            let loc_path = norm_path(loc);

            // 1. 路径级
            if loc_path == dir_path
                || dir_path.starts_with(&format!("{loc_path}\\"))
                || loc_path.starts_with(&format!("{dir_path}\\"))
            {
                return true;
            }

            // 2. 目录名级
            if let Some(last) = loc_path.rsplit('\\').next() {
                if !last.is_empty() && last == dir.name.to_ascii_lowercase() {
                    return true;
                }
            }
        }

        // 3. 名称级
        let entry_norm = normalize_name(&e.display_name);
        if dir_name_norm.len() >= 4
            && entry_norm.len() >= 4
            && (entry_norm.contains(&dir_name_norm) || dir_name_norm.contains(&entry_norm))
        {
            return true;
        }
    }
    false
}

/// 交叉比对（**纯逻辑**：不读注册表、不扫盘，入参由调用方准备好）。
///
/// 抽成纯函数是为了能单测「分类判定」——真注册表和真 Program Files 没法在
/// 单测里构造，而这一段的判定规则恰恰是最容易写错的地方。
///
/// 返回 `(类型 A 列表, 类型 B 列表)`。类型 B 的 `size_bytes` 恒为 0，
/// 由 [`scan_all`] 后续按预算填充（统计大小要碰磁盘，不适合放纯函数里）。
pub fn cross_reference(
    entries: &[UninstallEntry],
    dirs: &[InstallDir],
) -> (Vec<ResidualItem>, Vec<ResidualItem>) {
    // ---- 类型 A：注册表有 InstallLocation，但目录不存在 ----
    let mut registry_items = Vec::new();
    for e in entries {
        let Some(loc) = &e.install_location else {
            continue;
        };
        // 空值或形如 "C:" 的残缺值判不了，直接跳过（宁可漏报）
        if loc.trim().len() < 3 {
            continue;
        }
        if Path::new(loc).exists() {
            continue;
        }
        registry_items.push(ResidualItem {
            name: e.display_name.clone(),
            path: loc.clone(),
            residual_type: RESIDUAL_REGISTRY.to_string(),
            size_bytes: 0,
            registry_key: Some(e.registry_key.clone()),
            confidence: CONFIDENCE_HIGH.to_string(),
            risk_level: RISK_L2.to_string(),
            note: "注册表记录的安装目录已不存在。程序可能已手动删除或半卸载；\
                   本工具不代删注册表键——如需清理，请在「应用和功能」里修复或卸载，\
                   或用注册表编辑器手动删除该键"
                .to_string(),
        });
    }

    // ---- 类型 B：目录存在，但注册表找不到归属 ----
    let mut file_items = Vec::new();
    for dir in dirs {
        if has_owner(dir, entries) {
            continue;
        }
        file_items.push(ResidualItem {
            name: dir.name.clone(),
            path: dir.path.to_string_lossy().to_string(),
            residual_type: RESIDUAL_FILES.to_string(),
            size_bytes: 0,
            registry_key: None,
            confidence: CONFIDENCE_LOW.to_string(),
            risk_level: RISK_L2.to_string(),
            note: "注册表里找不到对应的卸载项。绿色软件、便携版、商店应用都会落到这里，\
                   删除前请务必自己确认。本工具会移入保险箱保留 7 天，可恢复"
                .to_string(),
        });
    }

    registry_items.sort_by(|a, b| a.name.cmp(&b.name));
    (registry_items, file_items)
}

/// 跑一次完整扫描：读注册表 → 枚举安装目录 → 交叉比对 → 统计大小。
pub fn scan_all() -> ResidualReport {
    let started = Instant::now();
    let entries = scan_registry_uninstall();

    let mut dirs = Vec::new();
    for root in install_roots() {
        dirs.extend(list_install_dirs(&root));
    }
    let dirs_scanned = dirs.len() as u64;

    let (registry_items, mut file_items) = cross_reference(&entries, &dirs);

    // 只有确定是孤儿才统计大小（递归求和很贵），并且整体设时间预算。
    let mut size_truncated = false;
    let deadline = started + SIZE_BUDGET;
    for item in &mut file_items {
        if Instant::now() >= deadline {
            size_truncated = true;
            item.note = "注册表里找不到对应的卸载项。**未统计大小**（超出统计预算）；\
                         绿色软件、便携版、商店应用都会落到这里，删除前请务必自己确认"
                .to_string();
            continue;
        }
        item.size_bytes = dir_total_bytes(Path::new(&item.path));
    }

    // 大的排前面：用户最关心占空间的
    file_items.sort_by(|a, b| {
        b.size_bytes
            .cmp(&a.size_bytes)
            .then_with(|| a.name.cmp(&b.name))
    });

    let total_bytes = file_items.iter().map(|i| i.size_bytes).sum();

    ResidualReport {
        registry_items,
        file_items,
        total_bytes,
        registry_entries: entries.len() as u64,
        dirs_scanned,
        size_truncated,
        elapsed_ms: started.elapsed().as_millis() as u64,
    }
}

// ===================== 删除（移入保险箱） =====================

/// 只用于红线校验的占位 Deleter。
///
/// [`redline_execute`] 本身**不会调用** deleter（它只做判定，真删由调用方决定），
/// 所以这里给一个「被调到就说明用错了」的实现，比复用 `NoopDeleter` 更安全
/// —— 后者会把路径悄悄记进 `recorded`，容易让人误以为「已经删过了」。
struct RedlineOnlyDeleter;

impl Deleter for RedlineOnlyDeleter {
    fn delete_file(&self, _p: &NormalizedPath) -> Result<bool, String> {
        Err("RedlineOnlyDeleter 不该被调用（红线校验不删除任何东西）".to_string())
    }
    fn delete_dir(&self, _p: &NormalizedPath) -> Result<(), String> {
        Err("RedlineOnlyDeleter 不该被调用（红线校验不删除任何东西）".to_string())
    }
}

/// 把残留目录**移入保险箱**（7 天可恢复），不是直接删。
///
/// # 安全链路（两道）
///
/// 1. [`redline_execute`]：项目唯一的删除入口，跑 L3 红线 + 动作级禁令。
///    任一条被拒 → 整体不进保险箱，记入 `failed` 并带上拒绝原因。
/// 2. [`SafeBoxDeleter::delete_file`]：`rename` 进 box_dir 并写 `safe_box` 行，
///    7 天后由 `purge_expired` 自动永久删除。目录是**整体 rename**，
///    内部结构完整保留，恢复后目录树原样回来。
///
/// # 为什么逐条而不是整批
///
/// 整批过红线时只要有一条被拒就整体 `Rejected`（`execute` 的语义），
/// 那会让用户「勾了 10 个，因为 1 个是桌面目录，10 个全没动」。
/// 逐条能把影响限制在单条，也便于把每条的原因分开告诉用户。
pub fn delete_residuals(
    paths: &[String],
    conn: &rusqlite::Connection,
    box_dir: &Path,
) -> ResidualDeleteResult {
    let mut result = ResidualDeleteResult::default();

    for raw in paths {
        // 1. 唯一入口红线校验
        let req = ExecuteRequest {
            paths: vec![raw.clone()],
            // 删除残留目录不涉及 DISM / 注册表动作，动作列表留空
            actions: Vec::new(),
        };
        match redline_execute(&req, &RedlineOnlyDeleter) {
            Ok(ExecuteOutcome::Rejected { decisions, .. }) => {
                let reason = decisions
                    .iter()
                    .find_map(|d| d.denial_reason.clone())
                    .map(|r| format!("命中安全红线（{r:?}），已拒绝"))
                    .unwrap_or_else(|| "命中安全红线，已拒绝".to_string());
                result.failed.push(ResidualDeleteFailure {
                    path: raw.clone(),
                    reason,
                });
                continue;
            }
            Ok(ExecuteOutcome::Allowed { .. }) => {}
            Err(e) => {
                result.failed.push(ResidualDeleteFailure {
                    path: raw.clone(),
                    reason: format!("红线校验内部错误：{e}"),
                });
                continue;
            }
        }

        // 2. 规范化（execute 内部已做过，但 deleter 要 NormalizedPath 实例）
        let np = match normalize(raw) {
            Ok(np) => np,
            Err(e) => {
                result.failed.push(ResidualDeleteFailure {
                    path: raw.clone(),
                    reason: format!("路径规范化失败：{e}"),
                });
                continue;
            }
        };

        // 3. 路径已经不在了（用户刚手动删过）→ 什么都不做。
        //
        // 必须在这里挡一道：`SafeBoxDeleter::delete_dir` 对不存在的路径返回
        // `Ok(())`（幂等），若直接信它就会把「什么都没做」记成「已移入保险箱」，
        // UI 上出现「已移入 1 项」但实际一个字节没动的假成功。
        if !Path::new(np.normalized()).exists() {
            continue;
        }

        // 4. 删除前先量体积（移走后就算不出来了）
        let size = dir_total_bytes(Path::new(np.normalized()));

        // 5. 移入保险箱
        let deleter = SafeBoxDeleter::new(box_dir, conn, None);
        // 用 `delete_dir` 而不是 `delete_file`：语义正确（这是目录），
        // 且 SafeBoxDeleter 的 delete_dir 已实现为「整体 rename 进保险箱」。
        match deleter.delete_dir(&np) {
            Ok(()) => {
                result.moved.push(np.normalized().to_string());
                result.bytes_moved = result.bytes_moved.saturating_add(size);
            }
            Err(e) => result.failed.push(ResidualDeleteFailure {
                path: raw.clone(),
                reason: e,
            }),
        }
    }

    result
}
