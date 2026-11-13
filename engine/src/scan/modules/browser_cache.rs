//! 浏览器缓存扫描（`scan_browser_cache.ps1` 的 Rust 实现，task-075）。
//!
//! # 与 ps1 的逐条对齐
//!
//! | 项 | `scan_browser_cache.ps1` | 本模块 |
//! |---|---|---|
//! | Edge | `%LOCALAPPDATA%\Microsoft\Edge\User Data\Default\Cache` | 同 |
//! | Chrome | `%LOCALAPPDATA%\Google\Chrome\User Data\Default\Cache` | 同 |
//! | Firefox | **不扫** | `%LOCALAPPDATA%\Mozilla\Firefox\Profiles\*\cache2`（**新增**） |
//! | 风险等级 | `1`（L1） | 同 |
//! | 遍历 | `Get-ChildItem -Recurse -Force -File` | [`super::scan_dir`] |
//! | 符号链接 | `if ($_.LinkType) { return }` | `is_reparse_point()` |
//!
//! # Firefox 是本次**新增**的扫描目标（不是等价重写）
//!
//! 原 ps1 只扫 Edge / Chrome 的 `Default\Cache`，没有 Firefox。任务书要求补上，
//! 理由是成立的：`cache2` 是纯缓存（图片、脚本、字体），Firefox 会自动重建，
//! 属 L1。
//!
//! **但要清楚它的影响**：本机实测 Firefox `cache2` 有 **4569 个文件**，而 Edge
//! `Default\Cache` 只有 7 个。加上 Firefox 后清理列表的条目数会从个位数涨到
//! 几千——这是用户可见的巨大变化，不是「顺手加个目录」。
//!
//! 之所以还是加了：清理列表有分页（task-064 起），几千条不会卡；而且这些都是
//! 真能清出空间的纯缓存。**如果哪天要回退这个决定，删掉 [`firefox_cache_dirs`]
//! 的调用即可，其余代码不受影响。**
//!
//! # 只扫 `Default` profile，不扫 `Profile 1/2/...`
//!
//! 原 ps1 只扫 `Default`。Edge / Chrome 的多 profile 用户（工作号 + 个人号）
//! 会漏掉其他 profile 的缓存。**本次保持与 ps1 一致**，不顺手扩——多 profile
//! 要枚举 `User Data\Profile *` 目录，而且还要考虑 `Guest Profile`、
//! `System Profile` 这些不该碰的，属独立的产品决策。
//!
//! # 只扫 `Cache`，不扫 `Code Cache` / `GPUCache`
//!
//! 这三个都是纯缓存（ps1 的注释也这么说），但 ps1 只把 `Cache` 放进了扫描表。
//! 本次保持等价。要全扫需另开任务——它会再涨一批条目数。
//!
//! # 不碰 L3 的 Profile 文件
//!
//! `User Data\Default` 下面还有 `Login Data`（保存的密码）、`Bookmarks`、
//! `History`、`Cookies`。这些**绝对不在扫描范围内**——只进 `Cache` 子目录，
//! 不做任何通配匹配。这是本项目 L3 红线在模块层的体现。
//!
//! # A2b + C1：规则库锚目录 + 多 Profile 展开
//!
//! task-083 A2b 把浏览器的根目录真值源迁进 `cache_rules.json` 的 `system` 组：
//! Edge/Chrome 的锚是 `User Data\...` 目录、Firefox 的锚是 Firefox 安装根目录。
//! 规则库只维护**一份锚**，实际扫哪些 profile 由 [`crate::scan::profiles`] 在
//! 模块内部展开（Chrome/Edge 枚举 `Default`/`Profile N`、Firefox 读
//! `profiles.ini`）——这就是 C1 的多 profile 支持，且不破坏规则库「同一件事
//! 只有一份实现」的约定。
//!
//! 保留 [`scan`] / [`fixed_roots`] / [`firefox_cache_dirs`] 作为**降级兜底**：
//! 规则库加载失败时退回旧 ps1 等价逻辑，ps1 对比测试不受影响。

use std::path::{Path, PathBuf};

use super::{scan_dir, ScanItem};
use crate::rules::models::CacheRules;
use crate::scan::profiles::{chromium_profiles, firefox_profiles};

/// 模块代码。对应 `cleanup_item.module_code`，与 ps1 的 `$moduleCode` 一致。
pub const MODULE_CODE: &str = "browser_cache";

/// Edge 缓存类别键（与 ps1 的 `'edge_cache'` 一致）。
pub const CATEGORY_EDGE: &str = "edge_cache";
/// Chrome 缓存类别键（与 ps1 的 `'chrome_cache'` 一致）。
pub const CATEGORY_CHROME: &str = "chrome_cache";
/// Firefox 缓存类别键（**task-075 新增**）。
pub const CATEGORY_FIREFOX: &str = "firefox_cache";

/// L1：浏览器缓存，删了浏览器会自己重建。
pub const RISK_L1: i64 = 1;

/// 本模块负责的业务类别集合（A2b：锚目录真值源从规则库取）。
const OWN_CATEGORIES: &[&str] = &[CATEGORY_EDGE, CATEGORY_CHROME, CATEGORY_FIREFOX];

/// 固定的两个缓存目录（Edge / Chrome 的 `Default\Cache`）。
///
/// `local_app_data` 显式传入而不是内部读环境变量——测试要能注入临时目录，
/// 而 `std::env::set_var` 会污染同进程其他用例。
///
/// 顺序与 ps1 的 `$roots` 一致（Edge 在前、Chrome 在后）。
pub fn fixed_roots(local_app_data: &Path) -> Vec<(PathBuf, &'static str)> {
    vec![
        (
            local_app_data.join(r"Microsoft\Edge\User Data\Default\Cache"),
            CATEGORY_EDGE,
        ),
        (
            local_app_data.join(r"Google\Chrome\User Data\Default\Cache"),
            CATEGORY_CHROME,
        ),
    ]
}

/// 枚举 Firefox 各 profile 下的 `cache2` 目录。
///
/// Firefox 的 profile 目录名带随机前缀（`nuxdfgk0.default-release`），没法写死，
/// 只能枚举 `Profiles` 下的子目录。这与 Edge/Chrome 的固定 `Default` 不同，
/// 所以单独一个函数。
///
/// 目录不存在 / 读不了都返回空（用户没装 Firefox 是常态）。
pub fn firefox_cache_dirs(local_app_data: &Path) -> Vec<PathBuf> {
    let profiles = local_app_data.join(r"Mozilla\Firefox\Profiles");
    let Ok(entries) = std::fs::read_dir(&profiles) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let cache2 = p.join("cache2");
        if cache2.is_dir() {
            out.push(cache2);
        }
    }
    // 排序让输出确定：同一台机器多次扫描的顺序必须一致
    out.sort();
    out
}

/// 扫描 `local_app_data` 下的全部浏览器缓存。
pub fn scan_at(local_app_data: &Path) -> Vec<ScanItem> {
    let mut out = Vec::new();

    for (dir, category) in fixed_roots(local_app_data) {
        out.extend(scan_dir(&dir, MODULE_CODE, category, RISK_L1));
    }
    for dir in firefox_cache_dirs(local_app_data) {
        out.extend(scan_dir(&dir, MODULE_CODE, CATEGORY_FIREFOX, RISK_L1));
    }

    out
}

/// 读 `%LOCALAPPDATA%` 后扫描。
///
/// 环境变量缺失时返回空：宁可少扫一类，也不要扫错目录（猜一个路径出来扫，
/// 可能扫到别的用户的数据）。
pub fn scan() -> Vec<ScanItem> {
    match std::env::var("LOCALAPPDATA") {
        Ok(v) if !v.trim().is_empty() => scan_at(Path::new(v.trim())),
        _ => Vec::new(),
    }
}

/// A2b + C1：从规则库 `system` 组读锚目录 + 多 profile 展开（生产主入口）。
///
/// 每个类别的锚从 `cache_rules.json` 取（Edge/Chrome 的 `User Data`、Firefox
/// 的安装根），再用 [`chromium_profiles`] / [`firefox_profiles`] 枚举其下全部
/// profile，逐个扫 profile 的 Cache 目录。这样规则库里只维护一份锚，多 profile
/// 不漏扫；[`scan`] 仍是规则加载失败时的降级兜底。
///
/// # 为什么按 category 分发而不是统一扫锚
///
/// Chromium 系（Edge/Chrome）的 profile 是 `User Data` 下的 `Default`/`Profile N`
/// 目录；Firefox 用 `profiles.ini` 描述，profile 目录带随机前缀。两者结构不同，
/// 必须走各自的枚举器。绕开 `roots_for_categories` 后 category 也能正确下穿到
/// ScanItem，保证与 ps1 的类别键一致。
pub fn scan_from_rules(rules: &CacheRules) -> Vec<ScanItem> {
    let anchors = crate::rules::schema::roots_for_categories(rules, OWN_CATEGORIES);
    let mut out = Vec::new();
    for anchor in anchors {
        let path = Path::new(&anchor.root);
        match anchor.category.as_str() {
            CATEGORY_EDGE | CATEGORY_CHROME => {
                for profile in chromium_profiles(path) {
                    for cache in profile.cache_roots {
                        out.extend(scan_dir(
                            &cache,
                            MODULE_CODE,
                            &anchor.category,
                            anchor.risk_level,
                        ));
                    }
                }
            }
            CATEGORY_FIREFOX => {
                // profiles.ini 在 Firefox 安装根下，锚目录就是它的父目录。
                for profile in firefox_profiles_from_root(path) {
                    for cache in profile.cache_roots {
                        out.extend(scan_dir(
                            &cache,
                            MODULE_CODE,
                            CATEGORY_FIREFOX,
                            anchor.risk_level,
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// task-086 5.5：协议统一入口。
///
/// **不启用增量缓存**：本模块的规则锚是浏览器安装根，真正要扫的目录要先读
/// `profiles.ini` / 枚举 Profile 才能定位；而缓存文件高频改写、安装根 `mtime`
/// 几乎不动，按根签名复用会长期返回旧结果。理由详见 [`crate::scan::cache`]。
pub fn scan_from_rules_cached(rules: &CacheRules, _conn: &rusqlite::Connection) -> Vec<ScanItem> {
    scan_from_rules(rules)
}

/// 从 Firefox 安装根目录读取 `profiles.ini` 并展开所有 profile。
///
/// `root` 是 `...\Mozilla\Firefox`（不是 `Profiles` 子目录），
/// `profiles.ini` 就放在这里。锚目录展开成 `root/profiles.ini`，
/// profile 的相对路径以此为基准解析。
fn firefox_profiles_from_root(root: &Path) -> Vec<crate::scan::profiles::BrowserProfile> {
    let ini_path = root.join("profiles.ini");
    let ini = match std::fs::read_to_string(&ini_path) {
        Ok(s) => s,
        Err(_) => return Vec::new(), // 没装 Firefox，不是错误
    };
    firefox_profiles(&ini, root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::models::{CacheRules, RuleEntry};
    use std::collections::HashMap;
    use std::fs;

    /// 构造一条 `system` 组规则（多 profile 测试用，路径直接写临时目录、
    /// 不进 `%LOCALAPPDATA%`，便于断言展开结果）。
    fn system_entry(id: &str, category: &str, path: &str) -> RuleEntry {
        RuleEntry {
            id: id.to_string(),
            category: category.to_string(),
            display_name: id.to_string(),
            paths: vec![path.to_string()],
            risk_level: "L1".to_string(),
            description: String::new(),
            impact_note: String::new(),
            rebuild_note: String::new(),
            elevated: false,
            matches: Vec::new(),
        }
    }

    fn rules_with(system: Vec<RuleEntry>) -> CacheRules {
        let mut modules = HashMap::new();
        modules.insert("system".to_string(), system);
        CacheRules {
            version: "1.3.0".to_string(),
            modules,
            whitelist_deny: Vec::new(),
        }
    }

    /// 造一个假的 `%LOCALAPPDATA%` 结构。
    fn make_lad() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();

        // Edge：Default\Cache 两个文件
        let edge = dir.path().join(r"Microsoft\Edge\User Data\Default\Cache");
        fs::create_dir_all(edge.join("Cache_Data")).unwrap();
        fs::write(edge.join("data_0"), vec![b'x'; 100]).unwrap();
        fs::write(edge.join("Cache_Data").join("f_000001"), vec![b'x'; 50]).unwrap();

        // Chrome：不存在（模拟没装）

        // Firefox：两个 profile，一个带 cache2、一个不带
        let ff1 = dir
            .path()
            .join(r"Mozilla\Firefox\Profiles\abc123.default-release\cache2\entries");
        fs::create_dir_all(&ff1).unwrap();
        fs::write(ff1.join("A1B2"), vec![b'x'; 300]).unwrap();
        let ff2 = dir
            .path()
            .join(r"Mozilla\Firefox\Profiles\xyz789.dev-edition");
        fs::create_dir_all(&ff2).unwrap(); // 没有 cache2

        dir
    }

    #[test]
    fn scan_at_collects_edge_and_firefox() {
        let lad = make_lad();
        let items = scan_at(lad.path());

        let edge: Vec<_> = items
            .iter()
            .filter(|i| i.category == CATEGORY_EDGE)
            .collect();
        assert_eq!(edge.len(), 2, "Edge 应有 2 个文件: {items:#?}");
        assert_eq!(edge.iter().map(|i| i.size_bytes).sum::<i64>(), 150);

        let ff: Vec<_> = items
            .iter()
            .filter(|i| i.category == CATEGORY_FIREFOX)
            .collect();
        assert_eq!(ff.len(), 1, "只有带 cache2 的那个 profile 有文件");
        assert_eq!(ff[0].size_bytes, 300);

        // Chrome 不存在 → 0 条，且不该报错
        assert!(items.iter().all(|i| i.category != CATEGORY_CHROME));
    }

    #[test]
    fn scan_at_tags_module_and_risk() {
        let lad = make_lad();
        let items = scan_at(lad.path());
        assert!(!items.is_empty());
        for it in &items {
            assert_eq!(it.module_code, MODULE_CODE);
            // 浏览器缓存一律 L1：删了浏览器自动重建，不影响任何数据
            assert_eq!(it.risk_level, RISK_L1);
        }
    }

    #[test]
    fn scan_at_returns_empty_for_missing_localappdata() {
        let dir = tempfile::tempdir().unwrap();
        // 一个什么都没有的空目录
        assert!(scan_at(dir.path()).is_empty());
    }

    #[test]
    fn fixed_roots_match_ps1_paths() {
        // 路径写错就等于扫了个不存在的目录，且**不会报错**——只是永远 0 项。
        // 这条钉住与 ps1 完全一致的相对路径。
        let base = Path::new(r"C:\Users\test\AppData\Local");
        let roots = fixed_roots(base);
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].1, CATEGORY_EDGE);
        assert_eq!(
            roots[0].0,
            base.join(r"Microsoft\Edge\User Data\Default\Cache")
        );
        assert_eq!(roots[1].1, CATEGORY_CHROME);
        assert_eq!(
            roots[1].0,
            base.join(r"Google\Chrome\User Data\Default\Cache")
        );
    }

    #[test]
    fn firefox_cache_dirs_only_picks_profiles_with_cache2() {
        let lad = make_lad();
        let dirs = firefox_cache_dirs(lad.path());
        assert_eq!(dirs.len(), 1, "只有 abc123 有 cache2: {dirs:#?}");
        assert!(dirs[0].ends_with("cache2"));
        assert!(dirs[0].to_string_lossy().contains("abc123.default-release"));
    }

    #[test]
    fn firefox_cache_dirs_empty_when_firefox_absent() {
        let dir = tempfile::tempdir().unwrap();
        assert!(firefox_cache_dirs(dir.path()).is_empty());
    }

    #[test]
    fn scan_never_touches_profile_files() {
        // L3 红线在模块层的体现：`Login Data` / `Bookmarks` / `History` 这些
        // 绝不能出现在扫描结果里。造一份假的 Profile 目录来验证。
        let lad = make_lad();
        let default_dir = lad.path().join(r"Microsoft\Edge\User Data\Default");
        fs::write(default_dir.join("Login Data"), vec![b'x'; 999]).unwrap();
        fs::write(default_dir.join("Bookmarks"), vec![b'x'; 999]).unwrap();
        fs::write(default_dir.join("History"), vec![b'x'; 999]).unwrap();

        let items = scan_at(lad.path());
        for it in &items {
            let lower = it.path.to_lowercase();
            for forbidden in ["login data", "bookmarks", "history", "cookies"] {
                assert!(
                    !lower.contains(forbidden),
                    "扫到了不该碰的 Profile 文件: {}",
                    it.path
                );
            }
        }
        // 但 Cache 里的还是要在
        assert_eq!(
            items.iter().filter(|i| i.category == CATEGORY_EDGE).count(),
            2
        );
    }

    // ===================== A2b + C1：规则库锚 + 多 profile 展开 =====================

    /// Chromium 多 profile：`User Data` 下 Default + Profile 1 各带缓存目录，
    /// 应收齐两个 profile 的条目，且类别正确下穿到 ScanItem。
    #[test]
    fn scan_from_rules_enumerates_all_chromium_profiles() {
        let tmp = tempfile::tempdir().unwrap();
        let user_data = tmp.path().join("Chrome User Data");
        for (prof, files) in [
            ("Default", 2usize),
            ("Profile 1", 1usize),
            ("Profile 2", 3usize),
        ] {
            let cache = user_data.join(prof).join("Cache");
            fs::create_dir_all(&cache).unwrap();
            for i in 0..files {
                fs::write(cache.join(format!("f{i}")), vec![b'x'; 10]).unwrap();
            }
        }
        // 非 profile 目录不该被扫（Crashpad 有 Cache 也不能进）
        fs::create_dir_all(user_data.join("Crashpad").join("Cache")).unwrap();
        fs::write(
            user_data.join("Crashpad").join("Cache").join("dump"),
            vec![b'x'; 5],
        )
        .unwrap();

        let rules = rules_with(vec![system_entry(
            "browser_chrome",
            CATEGORY_CHROME,
            user_data.to_str().unwrap(),
        )]);
        let items = scan_from_rules(&rules);

        let chrome: Vec<_> = items
            .iter()
            .filter(|i| i.category == CATEGORY_CHROME)
            .collect();
        assert_eq!(
            chrome.len(),
            6,
            "Default 2 + Profile 1 的 1 + Profile 2 的 3: {items:#?}"
        );
        assert!(chrome.iter().all(|i| i.module_code == MODULE_CODE));
        assert!(chrome.iter().all(|i| i.risk_level == RISK_L1));
        // Crashpad 绝不能进（不是 profile 目录）
        assert!(chrome.iter().all(|i| !i.path.contains("Crashpad")));
    }

    /// Firefox：从安装根目录读 `profiles.ini` 展开多 profile。
    #[test]
    fn scan_from_rules_reads_firefox_profiles_ini() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("Mozilla").join("Firefox");
        fs::create_dir_all(root.join("Profiles")).unwrap();
        fs::create_dir_all(
            root.join("Profiles")
                .join("abc.default-release")
                .join("cache2"),
        )
        .unwrap();
        fs::write(
            root.join("Profiles")
                .join("abc.default-release")
                .join("cache2")
                .join("E1"),
            vec![b'x'; 25],
        )
        .unwrap();
        let ini = "[Profile0]\nName=default-release\nIsRelative=1\nPath=Profiles/abc.default-release\nDefault=1\n";
        fs::write(root.join("profiles.ini"), ini).unwrap();

        let rules = rules_with(vec![system_entry(
            "browser_firefox",
            CATEGORY_FIREFOX,
            root.to_str().unwrap(),
        )]);
        let items = scan_from_rules(&rules);

        let ff: Vec<_> = items
            .iter()
            .filter(|i| i.category == CATEGORY_FIREFOX)
            .collect();
        assert_eq!(ff.len(), 1, "cache2 里一个文件: {items:#?}");
        assert!(ff[0].path.to_lowercase().contains("cache2"));
        assert_eq!(ff[0].size_bytes, 25);
        assert_eq!(ff[0].module_code, MODULE_CODE);
        assert_eq!(ff[0].risk_level, RISK_L1);
    }

    /// Firefox 未安装（无 profiles.ini）时规则驱动路径返回空，不 panic。
    #[test]
    fn scan_from_rules_firefox_missing_ini_yields_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("no-firefox");
        fs::create_dir_all(&root).unwrap();
        let rules = rules_with(vec![system_entry(
            "browser_firefox",
            CATEGORY_FIREFOX,
            root.to_str().unwrap(),
        )]);
        assert!(scan_from_rules(&rules).is_empty());
    }

    /// Category 未在规则 system 组注册时，scan_from_rules 不产出任何条目。
    #[test]
    fn scan_from_rules_without_browser_anchors_yields_empty() {
        let rules = rules_with(vec![system_entry(
            "other",
            "some_cat",
            r"C:\does-not-exist",
        )]);
        assert!(scan_from_rules(&rules).is_empty());
    }
}
