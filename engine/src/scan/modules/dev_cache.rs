//! 开发者缓存扫描（`scan_dev_cache.ps1` 的 Rust 实现，task-076）。
//!
//! # 与 ps1 的逐条对齐
//!
//! | 项 | `scan_dev_cache.ps1` | 本模块 |
//! |---|---|---|
//! | npm（新位置） | `%LOCALAPPDATA%\npm-cache` | 同 |
//! | npm（旧位置） | `%APPDATA%\npm-cache` | 同 |
//! | pip | `%LOCALAPPDATA%\pip\cache` | 同 |
//! | pnpm | `%LOCALAPPDATA%\pnpm-store` | 同 |
//! | 类别键 | `npm_cache` / `pip_cache` / `pnpm_store` | 同 |
//! | 风险等级 | `1`（L1，全部） | 同 |
//! | 遍历 | `Get-ChildItem -Recurse -Force -File` | [`super::scan_dir`] |
//! | 符号链接 | `if ($_.LinkType) { return }` | `is_reparse_point()` |
//! | 筛选 | **无** | **无**（见下） |
//!
//! # 任务书写的 Gradle / Maven **不在 ps1 里**，本次不加
//!
//! 任务书「现状」段说这个脚本扫「Node_modules / npm cache / pip cache / Gradle /
//! Maven 等」。实际读 `scripts/scan_dev_cache.ps1`（task-076 提交时的那版）：
//! `$roots` 只有 **4 条**——两个 npm-cache、一个 pip\cache、一个 pnpm-store，
//! **没有任何 Gradle / Maven 路径，也没有 node_modules**。
//!
//! 不加的理由不是「懒得加」，而是本任务自己的验收标准第 14 条：
//! **Rust 实现必须与 ps1 输出抽样对比一致**。多扫 Gradle / Maven 会让两边条目数
//! 对不上，那条对比就只能靠「过滤掉差异」来过关——把一个真实行为差异藏进测试的
//! 白名单里，比不加更糟。
//!
//! 另外 `~/.gradle/caches` 和 `~/.m2/repository` 里混着 `modules-2` 的依赖包元数据
//! 和 Gradle 的版本缓存，删错会让下次构建**重新下载全部依赖**（几百 MB 起）。
//! 要不要纳入、纳哪几个子目录，是独立的产品决策，得有独立任务和独立开关。
//!
//! # 不扫 `node_modules`
//!
//! 任务书提到了，但 ps1 没扫，且**绝不能随手加**：`node_modules` 是项目依赖，
//! 删掉等于让用户的项目当场跑不起来，重新装还要联网。它不属于「缓存」，
//! 属 L3 红线（用户数据）范畴，见 `execute::whitelist`。
//!
//! # npm 为什么要扫两个位置
//!
//! npm 6 及更早把缓存放在 `%APPDATA%\npm-cache`，npm 7 起改到
//! `%LOCALAPPDATA%\npm-cache`。老机器升级上来两个目录可能同时存在（旧的没删），
//! 所以两个都要看。**两者类别键相同**（都是 `npm_cache`）——它们在界面上是同一类
//! 东西，用户不关心 npm 的版本迁移史。
//!
//! # 为什么不按「文件大小 / 修改时间」筛选
//!
//! 与 temp / prefetch / browser_cache 同一判断：ps1 没有任何筛选，加阈值会让
//! `CCLEAN_RUST_DEV_CACHE=1` 和 `=0` 扫出的条目数不同，用户切换开关会看到
//! 「清理项变少了」却不知道是阈值在起作用。本任务是等价重写，不是改行为。
//!
//! # 全部 L1
//!
//! 包管理器缓存的语义就是「删了下次安装重新下载」——不丢数据、不坏环境，
//! 代价只是网速。这与 `update_download`（L2，可能正在被 Windows Update 使用）
//! 有本质区别。

use std::path::{Path, PathBuf};

use super::{scan_dir, ScanItem};
use crate::rules::models::CacheRules;

/// 本模块负责的业务类别集合（A2：路径真值源从规则库取）。
/// 注意 `npm_cache` 在 A 组与 `system` 组都出现——`roots_for_categories` 会按
/// 展开后的根目录去重，重复的那份不会重复扫。
const OWN_CATEGORIES: &[&str] = &[CATEGORY_NPM, CATEGORY_PIP, CATEGORY_PNPM];

/// 模块代码。对应 `cleanup_item.module_code`，与 ps1 的 `$moduleCode` 一致。
pub const MODULE_CODE: &str = "dev_cache";

/// npm 缓存类别键（ps1 里两个 npm 路径共用这一个）。
pub const CATEGORY_NPM: &str = "npm_cache";
/// pip 缓存类别键。
pub const CATEGORY_PIP: &str = "pip_cache";
/// pnpm store 类别键。
pub const CATEGORY_PNPM: &str = "pnpm_store";

/// L1：包管理器缓存，删了下次安装重新下载。
pub const RISK_L1: i64 = 1;
/// L2：Gradle/Maven 依赖缓存，删错要重下几百 MB 依赖——需逐项确认（不被一键全选命中）。
pub const RISK_L2: i64 = 2;

/// Gradle 缓存类别键（`~/.gradle/caches`）。
pub const CATEGORY_GRADLE: &str = "gradle_cache";
/// Maven 本地仓库类别键（`~/.m2/repository`）。
pub const CATEGORY_MAVEN: &str = "maven_cache";

/// 目标目录表：`(路径, 类别键)`。
///
/// `local_app_data` / `app_data` 显式传入而不是内部读环境变量——测试要能注入
/// 临时目录，而 `std::env::set_var` 会污染同进程其他用例（task-074 的教训）。
///
/// **顺序与 ps1 的 `$roots` 完全一致**：两条 npm、pip、pnpm。顺序本身不影响
/// 正确性，但保持一致能让「Rust vs ps1 抽样对比」在调试时逐行对得上。
pub fn scan_roots(local_app_data: &Path, app_data: &Path) -> Vec<(PathBuf, &'static str)> {
    vec![
        (local_app_data.join("npm-cache"), CATEGORY_NPM),
        (app_data.join("npm-cache"), CATEGORY_NPM),
        (local_app_data.join(r"pip\cache"), CATEGORY_PIP),
        (local_app_data.join("pnpm-store"), CATEGORY_PNPM),
    ]
}

/// 扫描给定两个根下的全部开发者缓存。
///
/// 目录不存在是**常态**（用户没装 pip / 没装 pnpm），[`scan_dir`] 会直接返回空。
pub fn scan_at(local_app_data: &Path, app_data: &Path) -> Vec<ScanItem> {
    let mut out = Vec::new();
    for (dir, category) in scan_roots(local_app_data, app_data) {
        out.extend(scan_dir(&dir, MODULE_CODE, category, RISK_L1));
    }
    out
}

/// 读环境变量后扫描。
///
/// # 环境变量缺失时**跳过那一类**，不猜路径
///
/// 与 browser_cache 同一原则：宁可少扫一类，也不要扫错目录（猜一个路径出来扫，
/// 可能扫到别的用户 / 别的盘的数据）。
///
/// 注意这与 ps1 有一处细微差别：ps1 里 `Join-Path $env:LOCALAPPDATA 'npm-cache'`
/// 在变量为空时会**抛异常**，整个脚本直接挂掉（`-ErrorAction` 管不到
/// `Join-Path`）。Rust 侧是跳过该根、继续扫其余的——对正常 Windows 机器
/// （这两个变量必然存在）两者结果相同，只在异常环境下 Rust 更稳。
pub fn scan() -> Vec<ScanItem> {
    let local_app_data = env_dir("LOCALAPPDATA");
    let app_data = env_dir("APPDATA");

    let mut out = Vec::new();
    // 四个根里只有 `%APPDATA%\npm-cache` 依赖 APPDATA，其余三个都依赖
    // LOCALAPPDATA。所以不能「任一缺失就整体返回空」——那样 APPDATA 缺失时
    // 会连带丢掉 pip / pnpm 的结果。
    if let Some(lad) = &local_app_data {
        for (dir, category) in [
            (lad.join("npm-cache"), CATEGORY_NPM),
            (lad.join(r"pip\cache"), CATEGORY_PIP),
            (lad.join("pnpm-store"), CATEGORY_PNPM),
        ] {
            out.extend(scan_dir(&dir, MODULE_CODE, category, RISK_L1));
        }
    }
    if let Some(ad) = &app_data {
        out.extend(scan_dir(
            &ad.join("npm-cache"),
            MODULE_CODE,
            CATEGORY_NPM,
            RISK_L1,
        ));
    }
    out
}

/// A2：从规则库 `system` 组读路径后扫描（生产真值源）。
///
/// 与 [`scan`]（环境变量硬编码降级兜底）对应，编排层注入了 `CacheRules` 时走这里：
/// npm / pip / pnpm 的缓存根只维护在 `cache_rules.json`。
pub fn scan_from_rules(rules: &CacheRules) -> Vec<ScanItem> {
    crate::rules::schema::roots_for_categories(rules, OWN_CATEGORIES)
        .into_iter()
        .flat_map(|r| scan_dir(Path::new(&r.root), MODULE_CODE, &r.category, r.risk_level))
        .collect()
}

/// task-084 B5：Gradle / Maven 依赖缓存扫描（独立开关，独立于 [`scan`]）。
///
/// # 为什么不并入 [`scan`] / `scan_from_rules`
///
/// 这四个（两个 npm、pip、pnpm）是「删错重下几分钟」的轻缓存；而 `~/.gradle/caches`
/// 与 `~/.m2/repository` 动辄几百 MB，**扫进清单本身不危险（只扫不删），危险的是
/// 被一键全选附带清掉**。所以它是**独立的追加步骤**：
///
/// - 不进入 `OWN_CATEGORIES` / 规则库 `system` 组（在那里会用 L1 被「一键全选」
///   命中）——这里固定 L2，用户必须逐项确认才删；
/// - 由编排器按 `prefs.dev_cache_gradle_maven` 独立开关决定是否执行（fail-closed）；
/// - 不进 Rust/ps1 抽样对比（那是 `scan_dev_cache.ps1` 4 个根的等价性契约，
///   Gradle/Maven 本来就不在 ps1 里）。
///
/// 根目录基于 `%USERPROFILE%`（不是硬编码 C 盘），避免扫到别的用户的资料。
pub fn gradle_maven_roots(user_profile: &Path) -> Vec<(PathBuf, &'static str)> {
    vec![
        (user_profile.join(r".gradle\caches"), CATEGORY_GRADLE),
        (user_profile.join(r".m2\repository"), CATEGORY_MAVEN),
    ]
}

/// 读 `%USERPROFILE%` 后扫描 Gradle/Maven 缓存。
///
/// `%USERPROFILE%` 缺失/空白时跳过整类（与 [`scan`] 的「宁可少扫一类也不猜路径」
/// 同一原则）。目录不存在是常态（没装 Gradle/Maven 的机器），[`scan_dir`] 返回空。
pub fn scan_gradle_maven() -> Vec<ScanItem> {
    let mut out = Vec::new();
    if let Some(home) = env_dir("USERPROFILE") {
        for (dir, category) in gradle_maven_roots(&home) {
            out.extend(scan_dir(&dir, MODULE_CODE, category, RISK_L2));
        }
    }
    out
}

/// 读一个环境变量并转成路径；缺失或空白返回 `None`。
fn env_dir(name: &str) -> Option<PathBuf> {
    dir_from(std::env::var(name).ok().as_deref())
}

/// [`env_dir`] 的纯函数内核。
///
/// 空白也要当缺失：`set LOCALAPPDATA=` 之后变量存在但为空串，直接
/// `Path::new("")` 会得到相对路径，`scan_dir` 里的 `is_dir()` 可能命中**当前
/// 工作目录**——那就成了扫自己的仓库，比少扫一类危险得多。
///
/// 抽出来是为了能测「空串」这个分支：`std::env::set_var` 在多线程测试里会污染
/// 同进程其他用例（Rust 2024 起还是 `unsafe`），造不出「变量存在但为空」的现场。
fn dir_from(val: Option<&str>) -> Option<PathBuf> {
    match val {
        Some(v) if !v.trim().is_empty() => Some(PathBuf::from(v.trim())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 造一个假的 `%LOCALAPPDATA%` + `%APPDATA%`，四个根各放几个文件。
    fn make_bases() -> (tempfile::TempDir, tempfile::TempDir) {
        let lad = tempfile::tempdir().unwrap();
        let ad = tempfile::tempdir().unwrap();

        // %LOCALAPPDATA%\npm-cache（新位置）
        let npm_new = lad.path().join("npm-cache").join("_cacache");
        fs::create_dir_all(&npm_new).unwrap();
        fs::write(npm_new.join("index-v5"), vec![b'x'; 100]).unwrap();
        fs::write(lad.path().join("npm-cache").join("_logs"), vec![b'x'; 20]).unwrap();

        // %APPDATA%\npm-cache（旧位置）
        let npm_old = ad.path().join("npm-cache");
        fs::create_dir_all(&npm_old).unwrap();
        fs::write(npm_old.join("content-v2"), vec![b'x'; 200]).unwrap();

        // %LOCALAPPDATA%\pip\cache
        let pip = lad.path().join(r"pip\cache").join("wheels");
        fs::create_dir_all(&pip).unwrap();
        fs::write(pip.join("foo.whl"), vec![b'x'; 300]).unwrap();

        // %LOCALAPPDATA%\pnpm-store
        let pnpm = lad.path().join("pnpm-store").join("v3");
        fs::create_dir_all(&pnpm).unwrap();
        fs::write(pnpm.join("hash"), vec![b'x'; 400]).unwrap();

        (lad, ad)
    }

    #[test]
    fn scan_at_collects_all_four_roots() {
        let (lad, ad) = make_bases();
        let items = scan_at(lad.path(), ad.path());

        // 5 个普通文件：npm 新位置 2 + npm 旧位置 1 + pip 1 + pnpm 1
        assert_eq!(items.len(), 5, "实际: {items:#?}");
        let total: i64 = items.iter().map(|i| i.size_bytes).sum();
        assert_eq!(total, 100 + 20 + 200 + 300 + 400);
    }

    #[test]
    fn scan_at_tags_category_by_root() {
        let (lad, ad) = make_bases();
        let items = scan_at(lad.path(), ad.path());

        let count = |cat: &str| items.iter().filter(|i| i.category == cat).count();
        // 两个 npm 根共用 npm_cache 类别：新位置 2 + 旧位置 1
        assert_eq!(
            count(CATEGORY_NPM),
            3,
            "两个 npm 根都归 npm_cache: {items:#?}"
        );
        assert_eq!(count(CATEGORY_PIP), 1);
        assert_eq!(count(CATEGORY_PNPM), 1);
    }

    #[test]
    fn scan_at_tags_module_and_risk() {
        let (lad, ad) = make_bases();
        let items = scan_at(lad.path(), ad.path());
        assert!(!items.is_empty());
        for it in &items {
            assert_eq!(it.module_code, MODULE_CODE);
            // 包管理器缓存一律 L1：删了下次安装重新下载，不丢数据
            assert_eq!(it.risk_level, RISK_L1, "不该有非 L1 的开发者缓存项");
        }
    }

    #[test]
    fn scan_at_skips_missing_roots() {
        // 只装 pip 的机器：不该因为 npm/pnpm 目录不存在而报错或产出多余条目
        let lad = tempfile::tempdir().unwrap();
        let ad = tempfile::tempdir().unwrap();
        let pip = lad.path().join(r"pip\cache");
        fs::create_dir_all(&pip).unwrap();
        fs::write(pip.join("only.whl"), vec![b'x'; 7]).unwrap();

        let items = scan_at(lad.path(), ad.path());
        assert_eq!(items.len(), 1, "实际: {items:#?}");
        assert_eq!(items[0].category, CATEGORY_PIP);
        assert_eq!(items[0].size_bytes, 7);
    }

    #[test]
    fn scan_roots_match_ps1_paths() {
        // 路径写错等于扫一个不存在的目录，而且**不会报错**——只是永远 0 项。
        // 这条钉住与 ps1 `$roots` 完全一致的相对路径与顺序。
        let base_lad = Path::new(r"C:\Users\test\AppData\Local");
        let base_ad = Path::new(r"C:\Users\test\AppData\Roaming");
        let roots = scan_roots(base_lad, base_ad);

        assert_eq!(roots.len(), 4);
        assert_eq!(
            roots[0],
            (base_lad.join("npm-cache"), CATEGORY_NPM),
            "第 1 条：%LOCALAPPDATA%\\npm-cache"
        );
        assert_eq!(
            roots[1],
            (base_ad.join("npm-cache"), CATEGORY_NPM),
            "第 2 条：%APPDATA%\\npm-cache"
        );
        assert_eq!(
            roots[2],
            (base_lad.join(r"pip\cache"), CATEGORY_PIP),
            "第 3 条：%LOCALAPPDATA%\\pip\\cache"
        );
        assert_eq!(
            roots[3],
            (base_lad.join("pnpm-store"), CATEGORY_PNPM),
            "第 4 条：%LOCALAPPDATA%\\pnpm-store"
        );
    }

    #[test]
    fn scan_roots_never_include_node_modules_or_gradle() {
        // 反向断言：这四个根里**不能**出现 node_modules（删了项目就跑不起来，
        // 属 L3 用户数据）和 gradle / maven（ps1 没扫，纳进来会让 Rust/ps1
        // 对比失败，且删错要重下几百 MB 依赖）。
        let base = Path::new(r"C:\Users\test\AppData\Local");
        for (p, _) in scan_roots(base, base) {
            let s = p.to_string_lossy().to_lowercase();
            for forbidden in ["node_modules", "gradle", "maven", ".m2"] {
                assert!(!s.contains(forbidden), "不该扫 {forbidden}: {s}");
            }
        }
    }

    #[test]
    fn scan_at_returns_empty_for_empty_bases() {
        let lad = tempfile::tempdir().unwrap();
        let ad = tempfile::tempdir().unwrap();
        assert!(scan_at(lad.path(), ad.path()).is_empty());
    }

    #[test]
    fn scan_does_not_panic_on_real_machine() {
        // 真机：读环境变量扫一遍。结果取决于本机装了什么（可能 0 条），
        // 重点是**不能崩**，且字段打点必须正确。
        let items = scan();
        assert!(items.iter().all(|i| i.module_code == MODULE_CODE));
        assert!(items.iter().all(|i| i.risk_level == RISK_L1));
        assert!(items.iter().all(|i| {
            matches!(
                i.category.as_str(),
                CATEGORY_NPM | CATEGORY_PIP | CATEGORY_PNPM
            )
        }));
    }

    #[test]
    fn dir_from_rejects_missing_and_blank() {
        // 空串必须当缺失：否则 Path::new("") 会指向当前目录，
        // `is_dir()` 命中后就把**仓库自己**当成缓存扫了。
        assert_eq!(dir_from(None), None);
        assert_eq!(dir_from(Some("")), None);
        assert_eq!(dir_from(Some("   ")), None);
        assert_eq!(dir_from(Some("\t\n")), None);

        // 正常值要原样（去掉首尾空格）转成路径
        assert_eq!(
            dir_from(Some(r"  C:\Users\test\AppData\Local  ")),
            Some(PathBuf::from(r"C:\Users\test\AppData\Local"))
        );
    }

    #[test]
    fn env_dir_missing_variable_is_none() {
        // 真环境里取一个肯定不存在的变量名
        assert_eq!(env_dir("CCLEAN_DEFINITELY_NOT_SET_076"), None);
    }

    // ============ task-084 B5：Gradle / Maven ============

    #[test]
    fn gradle_maven_roots_match_expected_paths() {
        // 钉住基于 %USERPROFILE% 的相对根，绝不硬编码 C 盘（否则会扫到别的用户）
        let home = Path::new(r"C:\Users\test");
        let roots = gradle_maven_roots(home);
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0], (home.join(r".gradle\caches"), CATEGORY_GRADLE));
        assert_eq!(roots[1], (home.join(r".m2\repository"), CATEGORY_MAVEN));
    }

    // scan_gradle_maven 依赖 %USERPROFILE% 环境变量，无法在测试里安全注入
    // （std::env::set_var 会污染同进程其他用例），这里只测 layout 与「目录不存在返回空」。
    #[test]
    fn scan_gradle_maven_layout_empty_for_missing_dirs() {
        let home = tempfile::tempdir().unwrap();
        // 没有 .gradle/.m2 的「全新用户」：不该产出任意条目，也不该报错
        for (dir, _) in gradle_maven_roots(home.path()) {
            assert!(!dir.exists());
        }
        // gradle_maven_roots 本身只是路径表，能否产出条目由 scan_dir 决定；
        // 这里验证「路径存在时」穿过 scan_dir 也能打点正确字段。
        let gradle_dir = home.path().join(r".gradle\caches\modules-2");
        fs::create_dir_all(&gradle_dir).unwrap();
        fs::write(gradle_dir.join("dep.jar"), vec![b'x'; 50]).unwrap();
        for (dir, category) in gradle_maven_roots(home.path()) {
            let items = super::scan_dir(&dir, MODULE_CODE, category, RISK_L2);
            if category == CATEGORY_GRADLE {
                assert!(!items.is_empty(), "gradle 目录已存在，应当扫出条目");
                assert!(items.iter().all(|i| i.risk_level == RISK_L2));
            } else {
                assert!(items.is_empty(), "maven 目录不存在，应当 0 项");
            }
        }
    }
}
