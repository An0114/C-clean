//! 应用路径解析（debt-cleanup-003 Bug-2 / bugfix/task-050）。
//!
//! perMachine 安装到 `C:\Program Files\C-Disk Cleaner\` 后，`std::env::current_dir()`
//! 指向只读安装目录，SQLite DB / safe_box / rules 更新全部 EACCES。
//! 统一改为：
//! - **可写数据根**：`app.path().app_data_dir()`（`%APPDATA%\com.and0114.cclean\`）；
//! - **只读资源**：`app.path().resource_dir()`（打包后随安装包分发）；
//! - 首次启动把 bundled `rules/` 复制到 `data_root/rules/`，后续 `apply_update`
//!   直接写 data_root 下的副本（不再动安装目录）；
//! - `scripts/` 只读 ps1 脚本从 resource_dir 读。
//!
//! task-050 修复点：`bundle.resources` 曾用数组式，Tauri 按「保留原目录结构」把
//! `../../rules/x.json` 落到 `<安装目录>\_up_\_up_\rules\x.json`，而这里只认
//! `<resource_dir>/rules` → 全新安装用户首次启动找不到规则与脚本。现在
//! 打包侧改为映射式（见 `tauri.conf.json`），本文件同步把定位逻辑收紧：
//! release 下**只认** `resource_dir()`，不再用 cwd 向上搜索（否则从任意含
//! `rules/`+`scripts/` 的目录启动安装版就能劫持资源），也不再静默回退 `.`。

use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// 全应用共享的路径集合，setup 闭包内构造一次后 manage() 注入。
pub struct AppPaths {
    /// 可写数据根：SQLite DB、safe_box、用户更新后的 rules。
    pub data_root: PathBuf,
    /// 可写 rules 目录（首次从 bundled 复制，后续 apply_update 写这里）。
    pub rules_dir: PathBuf,
    /// 只读 ps1 扫描脚本目录。
    pub scripts_dir: PathBuf,
    /// 可写保险箱目录。
    pub box_dir: PathBuf,
}

/// 递归复制目录（仅用于首次启动 bundled → data_root）。
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// 返回第一个同时含 `rules/` 与 `scripts/` 的候选目录。
///
/// 抽成纯函数是为了可单测：判定只看磁盘结构，不依赖真实 cwd / 真实安装目录。
/// 用 `is_dir()` 而非 `exists()`——同名普通文件不算资源目录。
fn pick_resource_root(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|c| c.join("rules").is_dir() && c.join("scripts").is_dir())
        .cloned()
}

/// 组装资源候选目录列表（顺序即优先级）。
///
/// - **release**：只有 `resource_dir()`。`dev_cwd` 在 release 下恒为 `None`
///   （见 `from_app()` 里的 `cfg` 分叉），因此 release 分支根本不碰 cwd；
/// - **dev**：`resource_dir()` 之后追加 cwd 向上三级
///   （`app/src-tauri/` → `app/` → workspace 根），保证 `pnpm tauri dev` 照常工作。
fn build_candidates(resource_dir: Option<PathBuf>, dev_cwd: Option<PathBuf>) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = resource_dir.into_iter().collect();
    if cfg!(debug_assertions) {
        if let Some(cwd) = dev_cwd {
            candidates.push(cwd.clone());
            candidates.push(cwd.join(".."));
            candidates.push(cwd.join("../.."));
        }
    }
    candidates
}

/// 资源缺失时的错误信息：必须带**绝对路径**与每处实际缺失项。
///
/// 旧实现静默返回 `.`，打包不全时表现为后续扫描报 INTERNAL，现场极难定位；
/// 这里把「哪个目录缺什么」直接写进消息，让首次启动的失败可自解释。
fn describe_missing(candidates: &[PathBuf]) -> String {
    if candidates.is_empty() {
        return "无法获取 resource_dir()，且 release 模式不再回退当前工作目录；\
                安装包可能不完整，请重新安装"
            .to_string();
    }

    let details: Vec<String> = candidates
        .iter()
        .map(|c| {
            let mut missing: Vec<&str> = Vec::new();
            if !c.join("rules").is_dir() {
                missing.push("rules/");
            }
            if !c.join("scripts").is_dir() {
                missing.push("scripts/");
            }
            if missing.is_empty() {
                // 理论上不会走到：两条都在就不是缺失候选。
                format!("{} 结构完整", c.display())
            } else {
                format!("{} 下缺少 {}", c.display(), missing.join(" 与 "))
            }
        })
        .collect();

    format!(
        "找不到同时含 rules/ 与 scripts/ 的资源目录：{}；安装包可能不完整，请重新安装",
        details.join("；")
    )
}

impl AppPaths {
    /// 在 setup 闭包内调用一次，构造后 `app.manage(self)`。
    pub fn from_app(app: &AppHandle) -> Result<Self, String> {
        // 1. 可写数据根：%APPDATA%\com.and0114.cclean\
        let data_root = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("app_data_dir: {e}"))?;
        std::fs::create_dir_all(&data_root).map_err(|e| format!("mkdir data_root: {e}"))?;

        // 2. 只读资源目录：候选列表 + 纯函数挑选。
        //    release 分支不取 cwd —— 这行 cfg 分叉就是「安装版不会被工作目录劫持」的保证。
        #[cfg(debug_assertions)]
        let dev_cwd = std::env::current_dir().ok();
        #[cfg(not(debug_assertions))]
        let dev_cwd: Option<PathBuf> = None;

        let candidates = build_candidates(app.path().resource_dir().ok(), dev_cwd);
        let resource_dir =
            pick_resource_root(&candidates).ok_or_else(|| describe_missing(&candidates))?;

        // 3. rules_dir：可写副本。首次启动从 bundled 复制，失败即 Err（不吞）。
        let rules_dir = data_root.join("rules");
        let bundled_rules = resource_dir.join("rules");
        if !rules_dir.exists() {
            copy_dir_all(&bundled_rules, &rules_dir).map_err(|e| {
                format!(
                    "copy bundled rules {} -> {}: {e}",
                    bundled_rules.display(),
                    rules_dir.display()
                )
            })?;
        }

        // 4. scripts_dir：只读，随安装包版本走。
        let scripts_dir = resource_dir.join("scripts");

        // 5. safe_box：可写。
        let box_dir = data_root.join("safe_box");
        std::fs::create_dir_all(&box_dir).map_err(|e| format!("mkdir safe_box: {e}"))?;

        // 启动日志必须打印绝对路径：缺陷态这里是 `.\scripts`，一眼能看出来。
        eprintln!(
            "[paths] data_root={} rules_dir={} scripts_dir={} box_dir={}",
            data_root.display(),
            rules_dir.display(),
            scripts_dir.display(),
            box_dir.display()
        );

        Ok(Self {
            data_root,
            rules_dir,
            scripts_dir,
            box_dir,
        })
    }
}
