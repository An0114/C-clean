//! Windows 更新下载缓存扫描（`scan_windows_update.ps1` 的 Rust 实现，task-076）。
//!
//! # 与 ps1 的逐条对齐
//!
//! | 项 | `scan_windows_update.ps1` | 本模块 |
//! |---|---|---|
//! | 目标目录 | `C:\Windows\SoftwareDistribution\Download`（硬编码） | `%SystemRoot%\SoftwareDistribution\Download` |
//! | 类别键 | `update_download` | 同 |
//! | 风险等级 | `2`（L2） | 同 |
//! | 遍历 | `Get-ChildItem -Recurse -Force -File` | [`super::scan_dir`] |
//! | 符号链接 | `if ($_.LinkType) { return }` | `is_reparse_point()` |
//! | 筛选 | **无** | **无** |
//!
//! # 为什么是 L2 而不是 L1
//!
//! 这一条是整个扫描器里最容易判错的地方，理由必须写清楚：
//!
//! - `Download` 下的文件**大多属 SYSTEM**，普通权限根本删不掉，列出来用户点了
//!   也只会失败；
//! - **更新正在安装时删它会中断安装**——这不是「下次重新下载」那么轻的代价，
//!   中断的更新可能让系统停在一个半装好的状态（已解压的组件 + 未提交的事务）。
//!
//! 所以它跟 `user_temp` / 浏览器缓存这类「删了自动重建」的 L1 有本质区别，
//! 必须逐项确认。v0.2.1 的 Bug-2 就是这里被写死成 `1`，task-070 才改成 `2`。
//!
//! # 一处刻意与 ps1 不同：`%SystemRoot%` 而不是硬编码 `C:\Windows`
//!
//! 同 temp / prefetch：系统装在 D 盘（或改过 `SystemRoot`）的机器上，ps1 的硬编码
//! 会扫错地方。Rust 侧读环境变量，是**修正**而不是行为变更——对绝大多数机器两者
//! 等价。`SystemRoot` 缺失时才退回 `C:\Windows`（与 ps1 行为一致，属最后兜底）。
//!
//! # 普通用户扫不到东西是**正常现象**
//!
//! `SoftwareDistribution\Download` 的 ACL 只给 Administrators / SYSTEM 读权限，
//! 普通用户 `read_dir` 拿到 `Permission denied`（本机实测确认）。ps1 侧靠
//! `-ErrorAction SilentlyContinue` 吞掉，这里靠 [`super::scan_dir`] 的
//! 「读不了就跳过该目录」。
//!
//! **不为多列几条而要求管理员权限**：结果就是普通权限下这个模块通常为空，
//! 界面上显示 0 项。这与 ps1 行为一致，不是 bug。
//!
//! # 不做「按大小 / 修改时间筛选」
//!
//! 与其余四个模块同一判断：ps1 没有筛选，加阈值会让两种模式结果不同。
//! 本任务是等价重写，不是改行为。

use std::path::{Path, PathBuf};

use super::{scan_dir, ScanItem};
use crate::rules::models::CacheRules;

/// 本模块负责的业务类别集合（A2：路径真值源从规则库取）。
const OWN_CATEGORIES: &[&str] = &[CATEGORY_UPDATE_DOWNLOAD];

/// 模块代码。对应 `cleanup_item.module_code`，与 ps1 的 `$moduleCode` 一致。
pub const MODULE_CODE: &str = "windows_update";

/// 类别键。ps1 里 `Write-Item 'update_download' ...`。
pub const CATEGORY_UPDATE_DOWNLOAD: &str = "update_download";

/// L2：需逐项确认（文件多属 SYSTEM；更新安装中删会中断安装）。
pub const RISK_L2: i64 = 2;

/// 更新下载目录（`<system_root>\SoftwareDistribution\Download`）。
///
/// `system_root` 显式传入而不是内部读环境变量——测试要能注入临时目录，
/// 而 `std::env::set_var` 会污染同进程其他用例。
pub fn update_root(system_root: &Path) -> PathBuf {
    system_root.join(r"SoftwareDistribution\Download")
}

/// 扫描 `<system_root>\SoftwareDistribution\Download`。
pub fn scan_at(system_root: &Path) -> Vec<ScanItem> {
    scan_dir(
        &update_root(system_root),
        MODULE_CODE,
        CATEGORY_UPDATE_DOWNLOAD,
        RISK_L2,
    )
}

/// 读 `%SystemRoot%` 后扫描。**普通权限下通常返回空**（目录 ACL 限制）。
pub fn scan() -> Vec<ScanItem> {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    scan_at(Path::new(&root))
}

/// A2：从规则库 `system` 组读路径后扫描（生产真值源）。
///
/// 与 [`scan`]（环境变量硬编码降级兜底）对应，编排层注入了 `CacheRules` 时走这里：
/// `%SystemRoot%\SoftwareDistribution\Download` 的根只维护在 `cache_rules.json`。
pub fn scan_from_rules(rules: &CacheRules) -> Vec<ScanItem> {
    crate::rules::schema::roots_for_categories(rules, OWN_CATEGORIES)
        .into_iter()
        .flat_map(|r| scan_dir(Path::new(&r.root), MODULE_CODE, &r.category, r.risk_level))
        .collect()
}

/// task-086 5.5：规则驱动 + 增量缓存（生产主入口）。
pub fn scan_from_rules_cached(rules: &CacheRules, conn: &rusqlite::Connection) -> Vec<ScanItem> {
    crate::rules::schema::roots_for_categories(rules, OWN_CATEGORIES)
        .into_iter()
        .flat_map(|r| {
            crate::scan::cache::scan_dir_cached(
                conn,
                Path::new(&r.root),
                MODULE_CODE,
                &r.category,
                r.risk_level,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 造一个假的 system_root，里面带 SoftwareDistribution\Download。
    fn make_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let dl = update_root(dir.path());
        fs::create_dir_all(dl.join("2026-10-31")).unwrap();
        fs::write(dl.join("windows10.0-kb5031-x64.cab"), vec![b'x'; 1000]).unwrap();
        fs::write(dl.join("2026-10-31").join("SSU-1.cab"), vec![b'x'; 500]).unwrap();
        fs::write(dl.join("2026-10-31").join("LCU-2.cab"), vec![b'x'; 250]).unwrap();
        dir
    }

    #[test]
    fn scan_at_collects_files_recursively() {
        let root = make_root();
        let items = scan_at(root.path());

        // 3 个普通文件，含一层子目录
        assert_eq!(items.len(), 3, "实际: {items:#?}");
        let total: i64 = items.iter().map(|i| i.size_bytes).sum();
        assert_eq!(total, 1000 + 500 + 250);
    }

    #[test]
    fn scan_at_tags_module_category_and_risk() {
        let root = make_root();
        let items = scan_at(root.path());
        assert!(!items.is_empty());
        for it in &items {
            assert_eq!(it.module_code, MODULE_CODE);
            assert_eq!(it.category, CATEGORY_UPDATE_DOWNLOAD);
            // **必须 L2**。写成 L1 会让「全选 L1」的清理计划把更新缓存一起删掉，
            // 而更新安装中删它会中断安装——这是 v0.2.1 Bug-2 的原样复现。
            assert_eq!(it.risk_level, RISK_L2, "更新缓存必须 L2，不能混进 L1");
        }
    }

    #[test]
    fn scan_at_returns_empty_when_missing() {
        // 目录不存在是正常情况（从没检查过更新 / 被组策略清过），不该 panic
        let dir = tempfile::tempdir().unwrap();
        assert!(scan_at(dir.path()).is_empty());
    }

    #[test]
    fn update_root_is_based_on_system_root() {
        let r = update_root(Path::new(r"D:\Windows"));
        // 系统装在 D 盘时也要跟着走，不能硬编码 C:
        assert_eq!(
            r,
            PathBuf::from(r"D:\Windows\SoftwareDistribution\Download")
        );
    }

    #[test]
    fn scan_does_not_panic_on_real_machine() {
        // 本机普通权限读不到真实目录，应返回空而不是 panic。
        // CI 上以管理员运行时这条会真的扫到东西——两种结果都算通过。
        let items = scan();
        assert!(items.iter().all(|i| i.module_code == MODULE_CODE));
        assert!(items.iter().all(|i| i.risk_level == RISK_L2));
    }
}
