//! Prefetch 预读取缓存扫描（`scan_prefetch.ps1` 的 Rust 实现，task-075）。
//!
//! # 与 ps1 的逐条对齐
//!
//! | 项 | `scan_prefetch.ps1` | 本模块 |
//! |---|---|---|
//! | 目标目录 | `C:\Windows\Prefetch`（硬编码） | `%SystemRoot%\Prefetch` |
//! | 类别键 | `prefetch`（与模块代码同名） | 同 |
//! | 风险等级 | `1`（L1） | 同 |
//! | 遍历 | `Get-ChildItem -Recurse -Force -File` | [`super::scan_dir`] |
//! | 符号链接 | `if ($_.LinkType) { return }` | `is_reparse_point()` |
//! | 筛选 | **无** | **无**（见下） |
//!
//! # 为什么 Prefetch 是 L1
//!
//! 它是 Windows 的「常用程序预读取缓存」，系统会自动重建。删掉只会让常用程序
//! **下次启动稍慢一点**，不影响任何数据。与浏览器缓存同属「安全缓存」。
//!
//! # 不做「按大小 / 修改时间筛选」
//!
//! 与 temp 模块同一判断：Prefetch 扫描定义为全量遍历，未经验证的阈值会
//! 静默改变清理范围。要加筛选应作为独立产品决策。
//!
//! # 普通用户读不到这个目录是**正常现象**
//!
//! `C:\Windows\Prefetch` 的默认 ACL 只给 Administrators / SYSTEM 读权限，
//! 普通用户 `read_dir` 会拿到 `Permission denied`（本机实测确认）。
//!
//! 这不是 bug，也不需要提权：**扫描器不为了多列几条而要求管理员权限**。
//! 结果就是「普通权限下这个模块恒为空」——界面上会显示 0 项，与 ps1 的
//! `-ErrorAction SilentlyContinue` 行为一致。真要清理 Prefetch 需要用户
//! 自己以管理员身份操作，那是另一条产品路径。

use std::path::PathBuf;

use super::{scan_dir, ScanItem};
use crate::rules::models::CacheRules;

/// 本模块负责的业务类别集合（A2：路径真值源从规则库取）。
const OWN_CATEGORIES: &[&str] = &[CATEGORY_PREFETCH];

/// 模块代码。对应 `cleanup_item.module_code`，与 ps1 的 `$moduleCode` 一致。
pub const MODULE_CODE: &str = "prefetch";

/// 类别键。ps1 里 `Write-Item 'prefetch' ...` 用的就是模块代码本身。
pub const CATEGORY_PREFETCH: &str = "prefetch";

/// L1：删了 Windows 会自动重建，只会让常用程序下次启动稍慢。
pub const RISK_L1: i64 = 1;

/// Prefetch 目录（`%SystemRoot%\Prefetch`）。
///
/// **不硬编码 `C:\Windows\Prefetch`**：ps1 是硬编码的，系统装在 D 盘或改过
/// `SystemRoot` 的机器上会扫错地方。`SystemRoot` 缺失时才退回 `C:\Windows`
/// （与 ps1 行为一致，属最后兜底）。
pub fn prefetch_root() -> PathBuf {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    PathBuf::from(root).join("Prefetch")
}

/// 扫描 `system_root\Prefetch`。
///
/// `system_root` 显式传入而不是内部读环境变量——测试要能注入临时目录，
/// 而 `std::env::set_var` 会污染同进程其他用例（见 task-074 的教训）。
pub fn scan_at(system_root: &std::path::Path) -> Vec<ScanItem> {
    scan_dir(
        &system_root.join("Prefetch"),
        MODULE_CODE,
        CATEGORY_PREFETCH,
        RISK_L1,
    )
}

/// 读环境变量后扫描。**普通权限下通常返回空**（目录 ACL 限制，见模块注释）。
pub fn scan() -> Vec<ScanItem> {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    scan_at(std::path::Path::new(&root))
}

/// A2：从规则库 `system` 组读路径后扫描（生产真值源）。
///
/// 与 [`scan`]（环境变量硬编码降级兜底）对应，编排层注入了 `CacheRules` 时走这里：
/// `%SystemRoot%\Prefetch` 的根只维护在 `cache_rules.json`，本模块只是执行器。
pub fn scan_from_rules(rules: &CacheRules) -> Vec<ScanItem> {
    crate::rules::schema::roots_for_categories(rules, OWN_CATEGORIES)
        .into_iter()
        .flat_map(|r| {
            scan_dir(
                std::path::Path::new(&r.root),
                MODULE_CODE,
                &r.category,
                r.risk_level,
            )
        })
        .collect()
}

/// task-086 5.5：规则驱动 + 增量缓存（生产主入口）。
pub fn scan_from_rules_cached(rules: &CacheRules, conn: &rusqlite::Connection) -> Vec<ScanItem> {
    crate::rules::schema::roots_for_categories(rules, OWN_CATEGORIES)
        .into_iter()
        .flat_map(|r| {
            crate::scan::cache::scan_dir_cached(
                conn,
                std::path::Path::new(&r.root),
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

    fn make_prefetch(root: &std::path::Path) -> std::path::PathBuf {
        let p = root.join("Prefetch");
        fs::create_dir_all(p.join("sub")).unwrap();
        fs::write(p.join("NOTEPAD.EXE-ABC.pf"), vec![b'x'; 100]).unwrap();
        fs::write(p.join("CHROME.EXE-DEF.pf"), vec![b'x'; 200]).unwrap();
        fs::write(p.join("sub").join("X.pf"), vec![b'x'; 50]).unwrap();
        // 非 .pf 文件也会被扫到 —— ps1 没按扩展名过滤，只按「文件」过滤
        fs::write(p.join("README.txt"), vec![b'x'; 10]).unwrap();
        p
    }

    #[test]
    fn scan_at_collects_all_files_recursively() {
        let dir = tempfile::tempdir().unwrap();
        make_prefetch(dir.path());
        let items = scan_at(dir.path());

        // 4 个普通文件（含非 .pf 的），递归到底
        assert_eq!(items.len(), 4, "实际: {items:#?}");
        let total: i64 = items.iter().map(|i| i.size_bytes).sum();
        assert_eq!(total, 100 + 200 + 50 + 10);
    }

    #[test]
    fn scan_at_tags_module_category_and_risk() {
        let dir = tempfile::tempdir().unwrap();
        make_prefetch(dir.path());
        let items = scan_at(dir.path());
        assert!(!items.is_empty());
        for it in &items {
            assert_eq!(it.module_code, MODULE_CODE);
            assert_eq!(it.category, CATEGORY_PREFETCH);
            // Prefetch 是安全缓存，必须是 L1（不能因为「在 C:\Windows 下」就升 L2，
            // 那是 temp 模块 windows_temp 的判断，不适用于自动重建的预读取缓存）
            assert_eq!(it.risk_level, RISK_L1);
        }
    }

    #[test]
    fn scan_at_returns_empty_when_prefetch_missing() {
        // 目录不存在是正常情况（比如被组策略关掉了 Prefetch），不该 panic
        let dir = tempfile::tempdir().unwrap();
        assert!(scan_at(dir.path()).is_empty());
    }

    #[test]
    fn prefetch_root_is_based_on_system_root() {
        let r = prefetch_root();
        assert_eq!(r.file_name().unwrap(), "Prefetch");
        if let Ok(sr) = std::env::var("SystemRoot") {
            assert!(
                r.starts_with(&sr),
                "应基于 SystemRoot={sr}，实际 {}",
                r.display()
            );
        }
    }

    #[test]
    fn scan_does_not_panic_when_unreadable() {
        // 本机普通权限读不到真实 Prefetch 目录，这时应返回空而不是 panic。
        // 这条在 CI（管理员）上会真的扫到东西，两种结果都算通过。
        let items = scan();
        assert!(items.iter().all(|i| i.risk_level == RISK_L1));
        assert!(items.iter().all(|i| i.module_code == MODULE_CODE));
    }
}
