//! 注册表健康中心（F 模块，task-068）：Run 键 / 卸载残留 / 计划任务三分类体检。
//!
//! # 为什么三类里只有 Run 键可删
//!
//! | 分类 | 能不能代删 | 原因 |
//! |---|---|---|
//! | **Run 启动项** | ✅ 可删（删**值**，不删键） | 删错只影响「某个程序不再开机启动」，随时能从程序设置里恢复；删前导出 `.reg` 备份，双击即可还原 |
//! | **卸载残留** | ❌ 不代删 | 那是「应用和功能」里的注册表项，删了会影响卸载/修复流程；task-067 已经从**文件侧**处理了这类残留 |
//! | **计划任务** | ❌ 不代删 | 任务定义是 `System32\Tasks` 下的 XML，删任务比删 Run 值风险高得多（可能是系统组件），引导用户去「任务计划程序」 |
//!
//! # 为什么自己生成 `.reg` 而不调 `reg export`
//!
//! 1. **不 spawn 进程**：本项目原则是不调 `cmd.exe` / `powershell` / 外部 exe
//!    （命令注入 + 安全软件拦截两类问题，`reg.exe` 在受限环境会被直接拦掉）；
//! 2. **可单测**：生成的是纯字符串，输入输出都能钉死；
//! 3. 粒度相同：`.reg` 格式本来就不支持「只导一个值」，两边都只能导整键。
//!
//! # 风险分级
//!
//! - **L3（禁删）**：命令在 `C:\Windows\` 下（系统组件）、或名称命中安全软件关键词。
//!   后端会**独立复核**一次，不信前端传上来的等级——否则前端被改一行就能删系统启动项。
//! - **L2（需确认）**：其余。其中「命令指向的文件已不存在」是最值得清的一类
//!   （典型卸载残留），建议文案会明确这么说。

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::scan::residual;
use crate::scan::winreg::{self, RegRoot};

/// 分类标识：开机启动项。
pub const CATEGORY_RUN: &str = "run";
/// 分类标识：卸载残留。
pub const CATEGORY_UNINSTALL: &str = "uninstall";
/// 分类标识：计划任务。
pub const CATEGORY_TASK: &str = "task";

/// 风险等级：需逐项确认。
pub const RISK_L2: &str = "L2";
/// 风险等级：禁删（后端会独立复核，不只看前端）。
pub const RISK_L3: &str = "L3";

/// Run 键的三个位置（64 位 / 32 位 / 用户级）。
const RUN_SUBKEYS: &[(RegRoot, &str)] = &[
    (
        RegRoot::LocalMachine,
        r"Software\Microsoft\Windows\CurrentVersion\Run",
    ),
    (
        RegRoot::LocalMachine,
        r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run",
    ),
    (
        RegRoot::CurrentUser,
        r"Software\Microsoft\Windows\CurrentVersion\Run",
    ),
];

/// 一条注册表健康项。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryHealthItem {
    /// 显示名（Run 键的值名 / 卸载项 DisplayName / 任务名）。
    pub name: String,
    /// 路径或命令行（Run 是命令、卸载残留是失效目录、任务是可执行文件）。
    pub path: String,
    /// 分类，取 `CATEGORY_*`。
    pub category: String,
    /// 风险等级，取 `RISK_*`。
    pub risk_level: String,
    /// 处理建议（中文，UI 直接展示）。
    pub recommendation: String,
    /// 定位信息：Run 是完整键路径、任务是任务文件路径、卸载残留是注册表键。
    pub registry_key: String,
    /// Run 键的值名（删除时定位用）。非 Run 项为空。
    pub value_name: String,
    /// 目标文件是否存在（Run / 任务）；卸载残留恒 false（目录本来就不在了）。
    pub exists: bool,
    /// 是否允许本工具代删。L3 恒 false。
    pub deletable: bool,
}

/// 一次体检的完整结果。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryHealthReport {
    /// 开机启动项。
    pub run_items: Vec<RegistryHealthItem>,
    /// 卸载残留（注册表记录的安装目录已不存在）。
    pub uninstall_items: Vec<RegistryHealthItem>,
    /// 计划任务。
    pub task_items: Vec<RegistryHealthItem>,
    /// 计划任务目录是否可读。
    ///
    /// **必须区分「没有任务」和「读不到」**：`%SystemRoot%\System32\Tasks` 默认只有
    /// Administrators / SYSTEM 可读，普通用户直接 `Permission denied`。若只看
    /// `task_items.is_empty()` 就显示「没有计划任务」，等于骗用户。
    pub task_scan_available: bool,
    /// 扫描耗时（毫秒）。
    pub elapsed_ms: u64,
}

/// 删除目标（Run 键里的一个值）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunTarget {
    /// `HKLM` / `HKCU`。
    pub root: String,
    /// 键路径（不含根）。
    pub subkey: String,
    /// 值名。
    pub value_name: String,
}

/// 单条删除失败。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryDeleteFailure {
    /// 值名。
    pub name: String,
    /// 失败原因（人话）。
    pub reason: String,
}

/// 删除结果。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryDeleteResult {
    /// 成功删除的值名。
    pub deleted: Vec<String>,
    /// 失败项 + 原因。
    pub failed: Vec<RegistryDeleteFailure>,
    /// 本次产生的备份文件路径（UI 要提示用户「备份在哪」）。
    pub backups: Vec<String>,
}

// ===================== Run 键 =====================

/// 扫描开机启动项（HKLM 64 位 + HKLM 32 位 + HKCU）。
pub fn scan_run_keys() -> Vec<RegistryHealthItem> {
    let mut out = Vec::new();
    for (root, subkey) in RUN_SUBKEYS {
        for v in winreg::values(*root, subkey) {
            // 默认值（空名）不是启动项
            if v.name.is_empty() {
                continue;
            }
            let cmd = v.as_string().unwrap_or_default();
            // **必须展开环境变量再判存在性**：真实数据里有
            // `%windir%\system32\SecurityHealthSystray.exe` 这种写法，
            // 不展开会让 `Path::exists()` 恒为 false，把正常程序误判成残留。
            let exe = extract_exe_path(&cmd).map(|p| residual::expand_env(&p));
            let exists = exe
                .as_deref()
                .map(|p| Path::new(p).exists())
                .unwrap_or(false);
            let (risk, recommendation) = classify_run_risk(&v.name, exe.as_deref(), exists);
            out.push(RegistryHealthItem {
                name: v.name.clone(),
                path: cmd,
                category: CATEGORY_RUN.to_string(),
                deletable: risk != RISK_L3,
                risk_level: risk,
                recommendation,
                registry_key: format!(r"{}\{}", root.short(), subkey),
                value_name: v.name,
                exists,
            });
        }
    }
    // 先列「文件已不存在」的（最该清的），再按名字排
    out.sort_by(|a, b| {
        a.exists.cmp(&b.exists).then_with(|| {
            a.name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
        })
    });
    out
}

/// 从命令行里抠出可执行文件路径。
///
/// 真实数据三种形态都要处理：
/// - `"C:\Program Files\X\y.exe" --arg`（带引号，最规范）
/// - `C:\X\y.exe -arg`（无引号无空格路径）
/// - `C:\Program Files\X\y.exe --arg`（无引号**含空格**，只能靠 `.exe` 截断）
pub fn extract_exe_path(cmd: &str) -> Option<String> {
    let t = cmd.trim();
    if t.is_empty() {
        return None;
    }
    if let Some(rest) = t.strip_prefix('"') {
        return rest.find('"').map(|i| rest[..i].to_string());
    }
    // 无引号：优先找 `.exe` 截断（大小写不敏感），找不到再按第一个空格切
    let lower = t.to_ascii_lowercase();
    if let Some(i) = lower.find(".exe") {
        return Some(t[..i + 4].to_string());
    }
    t.split_whitespace().next().map(|s| s.to_string())
}

/// 安全软件关键词（命中即 L3：删了会降低防护）。
const SECURITY_KEYWORDS: &[&str] = &[
    "securityhealth",
    "windowsdefender",
    "defender",
    "kaspersky",
    "avast",
    "avg",
    "bitdefender",
    "eset",
    "nod32",
    "mcafee",
    "symantec",
    "norton",
    "huorong",
    "火绒",
    "360",
    "qqpcmgr",
    "tencentpcmanager",
];

/// 判定一个启动项的风险等级 + 建议。
pub fn classify_run_risk(name: &str, exe: Option<&str>, exists: bool) -> (String, String) {
    // 1. 系统目录下的启动项：删了可能影响系统功能
    if let Some(p) = exe {
        let lower = p.replace('/', "\\").to_ascii_lowercase();
        if lower.starts_with("c:\\windows\\") {
            return (
                RISK_L3.to_string(),
                "系统组件（位于 C:\\Windows 下）的启动项，删除可能影响系统功能。本工具不代删，\
                 如确需处理请在「任务管理器 → 启动」里操作"
                    .to_string(),
            );
        }
    }
    // 2. 安全软件：删了降低防护
    let lower_name = name.to_ascii_lowercase();
    let lower_exe = exe.map(|p| p.to_ascii_lowercase()).unwrap_or_default();
    if SECURITY_KEYWORDS
        .iter()
        .any(|k| lower_name.contains(k) || lower_exe.contains(k))
    {
        return (
            RISK_L3.to_string(),
            "安全软件启动项，删除会降低防护。本工具不代删".to_string(),
        );
    }
    // 3. 目标文件不存在 —— 最值得清的一类（卸载残留）
    if !exists {
        return (
            RISK_L2.to_string(),
            "启动项指向的程序文件已不存在，大概率是卸载残留，可以删除（删前会自动导出 .reg 备份）"
                .to_string(),
        );
    }
    (
        RISK_L2.to_string(),
        "第三方开机启动项。删除后该程序不再自动启动，随时可从程序设置里恢复".to_string(),
    )
}

// ===================== 卸载残留 =====================

/// 卸载残留：注册表有 `InstallLocation` 但目录已不存在。
///
/// 直接复用 task-067 的 [`residual::scan_registry_uninstall`]，不重复读注册表。
/// 判定逻辑与 task-067 的类型 A 一致（`InstallLocation` 有值且目录不存在）。
pub fn scan_uninstall_residual() -> Vec<RegistryHealthItem> {
    let entries = residual::scan_registry_uninstall();
    let mut out = Vec::new();
    for e in entries {
        let Some(loc) = e.install_location else {
            continue;
        };
        // 空值 / 只有盘符的残缺值判不了（宁可漏报）
        if loc.trim().len() < 3 {
            continue;
        }
        if Path::new(&loc).exists() {
            continue;
        }
        out.push(RegistryHealthItem {
            name: e.display_name,
            path: loc,
            category: CATEGORY_UNINSTALL.to_string(),
            risk_level: RISK_L2.to_string(),
            recommendation: "注册表记录的安装目录已不存在。本工具不代删注册表键——\
                             请在「应用和功能」里修复或卸载，或用注册表编辑器手动删除该键"
                .to_string(),
            registry_key: e.registry_key,
            value_name: String::new(),
            exists: false,
            // 不代删：删「应用和功能」里的项会影响卸载/修复流程
            deletable: false,
        });
    }
    out.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
    });
    out
}

// ===================== 计划任务 =====================

/// 扫描计划任务（读 `%SystemRoot%\System32\Tasks` 下的 XML）。
///
/// # 为什么不调 `schtasks /query`
///
/// 1. 违反「不 spawn 外部 exe」原则；
/// 2. 输出是**本地化文本**，中文系统上列宽/表头都会变，解析必挂（task-065 的 DISM 踩过同样的坑）；
/// 3. `System32\Tasks` 下的 XML 是任务定义本身，字段稳定、跨语言一致。
///
/// # 局限
///
/// 这个目录普通用户只能读一部分（系统级任务需要管理员），所以列表会不全。
/// 读不到的文件直接跳过，不报错——体检要的是「能看到多少说多少」。
pub fn scan_scheduled_tasks(tasks_dir: &Path) -> Vec<RegistryHealthItem> {
    let mut out = Vec::new();
    collect_task_files(tasks_dir, 0, &mut out);

    let mut items: Vec<RegistryHealthItem> = Vec::new();
    for file in out {
        let Ok(content) = std::fs::read_to_string(&file) else {
            continue;
        };
        let Some((command, args)) = parse_task_xml(&content) else {
            continue;
        };
        let exists = Path::new(&command).exists();
        // 任务名 = 相对 Tasks 目录的路径（去掉 .xml），与「任务计划程序」里显示的一致
        let name = file
            .strip_prefix(tasks_dir)
            .unwrap_or(&file)
            .with_extension("")
            .to_string_lossy()
            .replace('/', "\\");

        let (risk, recommendation) = if command
            .replace('/', "\\")
            .to_ascii_lowercase()
            .starts_with("c:\\windows\\")
        {
            (
                RISK_L3.to_string(),
                "系统组件的计划任务，删除可能影响系统功能。本工具不代删，\
                 如确需处理请用「任务计划程序」"
                    .to_string(),
            )
        } else if !exists {
            (
                RISK_L2.to_string(),
                "任务指向的程序已不存在，可能是卸载残留。本工具不代删计划任务，\
                 可在「任务计划程序」里删除该任务"
                    .to_string(),
            )
        } else {
            (
                RISK_L2.to_string(),
                "第三方计划任务。本工具不代删，如不需要请在「任务计划程序」里禁用或删除"
                    .to_string(),
            )
        };

        let full_path = if args.is_empty() {
            command
        } else {
            format!("{command} {args}")
        };
        items.push(RegistryHealthItem {
            name,
            path: full_path,
            category: CATEGORY_TASK.to_string(),
            risk_level: risk,
            recommendation,
            registry_key: file.to_string_lossy().to_string(),
            value_name: String::new(),
            exists,
            // 计划任务一律不代删
            deletable: false,
        });
    }

    items.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
    });
    items
}

/// 递归收集任务 XML 文件（任务按目录分层，深度通常 ≤ 3）。
fn collect_task_files(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    // 深度上限防异常目录结构
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_symlink() {
            continue;
        }
        if ft.is_dir() {
            collect_task_files(&entry.path(), depth + 1, out);
        } else if ft.is_file() {
            // 只认 .xml（目录里还有别的杂物）
            let is_xml = entry
                .path()
                .extension()
                .map(|e| e.eq_ignore_ascii_case("xml"))
                .unwrap_or(false);
            if is_xml {
                out.push(entry.path());
            }
        }
    }
}

/// 从任务 XML 里抠出 `(Command, Arguments)`。
///
/// 不引 XML 库：只需要两个固定标签的文本内容，引 quick-xml 会让编译时间再涨一截。
/// 任务 XML 结构固定（`<Actions><Exec><Command>…</Command><Arguments>…</Arguments>`），
/// 字符串查找足够稳。
pub fn parse_task_xml(content: &str) -> Option<(String, String)> {
    let command = tag_text(content, "Command")?;
    if command.trim().is_empty() {
        return None;
    }
    let args = tag_text(content, "Arguments").unwrap_or_default();
    Some((xml_unescape(&command), xml_unescape(&args)))
}

/// 取 `<tag>…</tag>` 之间的文本。
fn tag_text(content: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = content.find(&open)? + open.len();
    let end = content[start..].find(&close)? + start;
    Some(content[start..end].trim().to_string())
}

/// XML 实体反转义（只处理任务 XML 里会出现的五个）。
fn xml_unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

// ===================== 汇总 =====================

/// 跑一次完整体检。`tasks_dir` 由调用方给（安装态是 `%SystemRoot%\System32\Tasks`）。
pub fn scan_all(tasks_dir: &Path) -> RegistryHealthReport {
    let started = Instant::now();
    let run_items = scan_run_keys();
    let uninstall_items = scan_uninstall_residual();

    // 先探一次可读性：读不到就别假装「扫了但没结果」
    let task_scan_available = std::fs::read_dir(tasks_dir).is_ok();
    let task_items = if task_scan_available {
        scan_scheduled_tasks(tasks_dir)
    } else {
        Vec::new()
    };

    RegistryHealthReport {
        run_items,
        uninstall_items,
        task_items,
        task_scan_available,
        elapsed_ms: started.elapsed().as_millis() as u64,
    }
}

// ===================== 删除（带 .reg 备份） =====================

/// 删除若干 Run 键值，**每个键删前先导出 `.reg` 备份**。
///
/// # fail-closed
///
/// 备份写不出来就**不删**——没有退路的删除不做。
///
/// # 后端独立复核风险等级
///
/// 不信任前端传上来的 `riskLevel`：删除前重新从注册表读值、重新算风险，
/// L3 直接拒。否则前端被改一行就能删掉系统启动项。
pub fn delete_run_values(
    backup_dir: &Path,
    targets: &[RunTarget],
    stamp: i64,
) -> RegistryDeleteResult {
    let mut result = RegistryDeleteResult::default();

    // 同一个键只备份一次（同一 Run 键下删多个值时，重复导出是浪费）
    let mut backed_up: Vec<(String, String)> = Vec::new();

    for t in targets {
        let Some(root) = parse_root(&t.root) else {
            result.failed.push(RegistryDeleteFailure {
                name: t.value_name.clone(),
                reason: format!("未知注册表根: {}", t.root),
            });
            continue;
        };

        // 1. 重新读值（不信前端）
        let vals = winreg::values(root, &t.subkey);
        let Some(v) = vals.iter().find(|v| v.name == t.value_name) else {
            // 值已经不在了：幂等跳过，不算失败也不算成功
            continue;
        };

        // 2. 重新判风险
        let cmd = v.as_string().unwrap_or_default();
        // 与 scan_run_keys 保持一致：先展开环境变量再判存在性
        let exe = extract_exe_path(&cmd).map(|p| residual::expand_env(&p));
        let exists = exe
            .as_deref()
            .map(|p| Path::new(p).exists())
            .unwrap_or(false);
        let (risk, _) = classify_run_risk(&t.value_name, exe.as_deref(), exists);
        if risk == RISK_L3 {
            result.failed.push(RegistryDeleteFailure {
                name: t.value_name.clone(),
                reason: "命中 L3 保护（系统组件或安全软件启动项），后端拒绝删除".to_string(),
            });
            continue;
        }

        // 3. 备份（fail-closed：备份不成功就不删）
        let key_id = format!("{}|{}", t.root, t.subkey);
        if !backed_up.iter().any(|(k, _)| k == &key_id) {
            let content = winreg::build_reg_export(root, &t.subkey, &vals);
            let stem = format!("{}-{}", root.short(), winreg::sanitize_file_stem(&t.subkey));
            match winreg::write_reg_backup(backup_dir, &stem, &content, stamp) {
                Ok(p) => {
                    backed_up.push((key_id.clone(), p.to_string_lossy().to_string()));
                    result.backups.push(p.to_string_lossy().to_string());
                }
                Err(e) => {
                    // 备份失败 → 这条不删（没有退路的删除不做）
                    result.failed.push(RegistryDeleteFailure {
                        name: t.value_name.clone(),
                        reason: format!("备份失败，已中止删除：{e}"),
                    });
                    continue;
                }
            }
        }

        // 4. 删值（只删值，不删键——删键会连带干掉同键下其他程序的启动项）
        match winreg::delete_value(root, &t.subkey, &t.value_name) {
            Ok(()) => result.deleted.push(t.value_name.clone()),
            Err(e) => result.failed.push(RegistryDeleteFailure {
                name: t.value_name.clone(),
                reason: e,
            }),
        }
    }

    result
}

/// `"HKLM"` / `"HKCU"` → [`RegRoot`]。
fn parse_root(s: &str) -> Option<RegRoot> {
    match s.to_ascii_uppercase().as_str() {
        "HKLM" | "HKEY_LOCAL_MACHINE" => Some(RegRoot::LocalMachine),
        "HKCU" | "HKEY_CURRENT_USER" => Some(RegRoot::CurrentUser),
        _ => None,
    }
}
