//! 应用路径解析（debt-cleanup-003 Bug-2）。
//!
//! perMachine 安装到 `C:\Program Files\C-Disk Cleaner\` 后，`std::env::current_dir()`
//! 指向只读安装目录，SQLite DB / safe_box / rules 更新全部 EACCES。
//! 统一改为：
//! - **可写数据根**：`app.path().app_data_dir()`（`%APPDATA%\com.and0114.cclean\`）；
//! - **只读资源**：`app.path().resource_dir()`（打包后随安装包分发）；
//! - 首次启动把 bundled `rules/` 复制到 `data_root/rules/`，后续 `apply_update`
//!   直接写 data_root 下的副本（不再动安装目录）；
//! - `scripts/` 只读 ps1 脚本从 resource_dir 读。

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

/// 在 dev / prod 两种模式下定位只读资源目录。
///
/// prod：`resource_dir()` 即安装目录，`rules/` `scripts/` 由 bundle.resources 带过去。
/// dev：cwd 是 `app/src-tauri/`，resource_dir() 指向 target/debug/，里面没有
///      rules/scripts；逐级向上找（src-tauri/ → app/ → workspace 根）。
fn locate_resource_dir() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        // 逐级向上：src-tauri/、app/、workspace 根。
        for candidate in [cwd.clone(), cwd.join(".."), cwd.join("../..")] {
            if candidate.join("rules").exists() && candidate.join("scripts").exists() {
                return candidate;
            }
        }
    }
    // 最终回退（prod 走这里）。
    PathBuf::from(".")
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

        // 2. 只读资源目录。
        let resource_dir = if let Ok(res) = app.path().resource_dir() {
            if res.join("rules").exists() || res.join("scripts").exists() {
                res
            } else {
                locate_resource_dir()
            }
        } else {
            locate_resource_dir()
        };

        // 3. rules_dir：可写副本。首次启动从 bundled 复制。
        let rules_dir = data_root.join("rules");
        let bundled_rules = resource_dir.join("rules");
        if !rules_dir.exists() && bundled_rules.exists() {
            copy_dir_all(&bundled_rules, &rules_dir)
                .map_err(|e| format!("copy bundled rules: {e}"))?;
        }

        // 4. scripts_dir：只读。
        let scripts_dir = resource_dir.join("scripts");

        // 5. safe_box：可写。
        let box_dir = data_root.join("safe_box");
        std::fs::create_dir_all(&box_dir).map_err(|e| format!("mkdir safe_box: {e}"))?;

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
