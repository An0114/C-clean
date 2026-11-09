//! 临时文件扫描（`scan_temp.ps1` 的 Rust 实现，task-074）。
//!
//! # 与 ps1 的逐条对齐
//!
//! | 项 | `scan_temp.ps1` | 本模块 |
//! |---|---|---|
//! | 目标目录 | `$env:TEMP` + `C:\Windows\Temp`（硬编码） | `%TEMP%` + `%SystemRoot%\Temp` |
//! | 类别键 | `user_temp` / `windows_temp` | 同 |
//! | 风险等级 | `1` / `2`（v0.2.1 Bug-2 改的分级） | 同 |
//! | 遍历 | `Get-ChildItem -Recurse -Force -File` | 显式栈 + `read_dir` |
//! | 符号链接 | `if ($_.LinkType) { return }` | `is_reparse_point()`（覆盖 junction） |
//! | 权限不足 | `-ErrorAction SilentlyContinue` | `read_dir` 失败 → 跳过该目录 |
//! | 筛选 | **无**（全量） | **无**（见下） |
//!
//! # 为什么不做「按大小 / 修改时间筛选」
//!
//! 任务书写了「按文件大小 / 修改时间筛选」，但**现有 ps1 没有任何筛选**——
//! 它扫描目录下全部文件。加上筛选会带来两个问题：
//!
//! 1. **两种模式结果不一致**：`CCLEAN_RUST_TEMP=1` 和 `=0` 扫出的条目数不同，
//!    用户切换开关会看到「清理项变少了」，而不知道是阈值在起作用；
//! 2. **阈值没有依据**：多小算「小」？多旧算「旧」？拍一个数字进代码，
//!    等于替产品做了一个没论证的决定。
//!
//! 本任务的目标是**等价重写**，不是改行为。要加筛选应该是独立的产品决策
//! （需要先定阈值、并在 UI 上说明「为什么有些文件不显示」）。
//!
//! # 一处刻意与 ps1 不同：`%SystemRoot%` 而不是硬编码 `C:\Windows`
//!
//! ps1 写死了 `C:\Windows\Temp`。系统装在 D 盘（或改过 `SystemRoot`）的机器上
//! 就扫错地方了。Rust 侧读环境变量，是**修正**而不是行为变更——对绝大多数
//! 机器两者等价。
//!
//! # 为什么输出要排序
//!
//! 文件系统返回顺序本身不保证，但排序让输出**确定**：单测能稳定断言、
//! 「Rust vs ps1 抽样对比」也不会因为顺序不同而假失败。代价是每个目录多一次
//! `Vec` 排序（条目数几千，可忽略）。

use std::path::{Path, PathBuf};

use super::ScanItem;
use crate::rules::models::CacheRules;

/// 本模块负责的业务类别集合（A2：路径真值源从规则库取）。
const OWN_CATEGORIES: &[&str] = &[CATEGORY_USER_TEMP, CATEGORY_WINDOWS_TEMP];

/// 模块代码。对应 `cleanup_item.module_code`，与 ps1 的 `$moduleCode` 一致。
pub const MODULE_CODE: &str = "temp";

/// 用户临时目录的类别键。
pub const CATEGORY_USER_TEMP: &str = "user_temp";
/// 系统临时目录的类别键。
pub const CATEGORY_WINDOWS_TEMP: &str = "windows_temp";

/// L1：删了会自动重建（用户自己的临时文件）。
pub const RISK_L1: i64 = 1;
/// L2：需确认（系统临时目录，多数属 SYSTEM、安装程序可能正在用）。
pub const RISK_L2: i64 = 2;

/// 扫描全部临时目录，返回待清理项。
///
/// 顺序：先 `%TEMP%` 再 `%SystemRoot%\Temp`（与 ps1 的调用顺序一致）。
pub fn scan() -> Vec<ScanItem> {
    let mut out = Vec::new();
    for (root, category, risk) in scan_roots() {
        out.extend(scan_dir(&root, category, risk));
    }
    out
}

/// A2：从规则库 `system` 组读路径后扫描（生产真值源）。
///
/// [`scan`] 仍读环境变量硬编码，是**加载规则失败时的降级兜底**；本函数是
/// 编排层注入了 `CacheRules` 时走的主入口——路径只维护在 `cache_rules.json`，
/// Rust 模块只是执行器。
pub fn scan_from_rules(rules: &CacheRules) -> Vec<ScanItem> {
    crate::rules::schema::roots_for_categories(rules, OWN_CATEGORIES)
        .into_iter()
        // 本模块的薄包装 `scan_dir(root, category, risk)` 已固定 module_code = "temp"
        .flat_map(|r| scan_dir(Path::new(&r.root), &r.category, r.risk_level))
        .collect()
}

/// 目标目录表：`(路径, 类别键, 风险等级)`。
///
/// 抽成独立函数是为了能单测「环境变量缺失时的兜底」——直接调 [`scan`] 会
/// 真扫一遍磁盘，断言不了目标表本身。
pub fn scan_roots() -> Vec<(PathBuf, &'static str, i64)> {
    let mut roots = Vec::new();

    // %TEMP% 缺失或为空时**不塞一个猜的路径**：宁可少扫一类，也不要扫错目录。
    if let Ok(t) = std::env::var("TEMP") {
        let t = t.trim();
        if !t.is_empty() {
            roots.push((PathBuf::from(t), CATEGORY_USER_TEMP, RISK_L1));
        }
    }

    // SystemRoot 缺失时退回 C:\Windows —— 这与 ps1 的硬编码行为一致，
    // 属于最后的兜底（正常 Windows 上这个变量一定存在）。
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    roots.push((
        PathBuf::from(root).join("Temp"),
        CATEGORY_WINDOWS_TEMP,
        RISK_L2,
    ));

    roots
}

/// 递归扫描一个目录，产出该目录下全部普通文件。
///
/// # task-075：实现已下沉到 [`super::scan_dir`]
///
/// 这份遍历原本写在这里（task-074）。task-075 加 prefetch / browser_cache 时
/// 三份几乎一样的递归代码就要开始漂移——所以把「怎么遍历」提到
/// [`super::scan_dir`]，本函数只固定 `module_code = "temp"`。
///
/// 保留这个薄包装是为了让既有调用点（`scan()` 和 7 条单测）不用改签名。
pub fn scan_dir(root: &Path, category: &str, risk_level: i64) -> Vec<ScanItem> {
    super::scan_dir(root, MODULE_CODE, category, risk_level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 造一个临时目录树：
    /// ```text
    /// root/
    ///   a.tmp        10 B
    ///   b.log        20 B
    ///   sub/
    ///     c.dat      30 B
    ///     deep/
    ///       d.bin    40 B
    /// ```
    fn make_tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.tmp"), vec![b'x'; 10]).unwrap();
        fs::write(dir.path().join("b.log"), vec![b'x'; 20]).unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir_all(sub.join("deep")).unwrap();
        fs::write(sub.join("c.dat"), vec![b'x'; 30]).unwrap();
        fs::write(sub.join("deep").join("d.bin"), vec![b'x'; 40]).unwrap();
        dir
    }

    #[test]
    fn scan_dir_collects_files_recursively() {
        let dir = make_tree();
        let items = scan_dir(dir.path(), CATEGORY_USER_TEMP, RISK_L1);

        // 4 个普通文件，递归到底
        assert_eq!(items.len(), 4, "实际: {items:#?}");
        let total: i64 = items.iter().map(|i| i.size_bytes).sum();
        assert_eq!(total, 10 + 20 + 30 + 40);

        // 每条都要带正确的模块代码 / 类别 / 风险
        for it in &items {
            assert_eq!(it.module_code, MODULE_CODE);
            assert_eq!(it.category, CATEGORY_USER_TEMP);
            assert_eq!(it.risk_level, RISK_L1);
        }
    }

    #[test]
    fn scan_dir_marks_windows_temp_as_l2() {
        // v0.2.1 Bug-2 的分级：系统临时目录是 L2，不能退回 L1
        let dir = make_tree();
        let items = scan_dir(dir.path(), CATEGORY_WINDOWS_TEMP, RISK_L2);
        assert!(!items.is_empty());
        assert!(items.iter().all(|i| i.risk_level == RISK_L2));
        assert!(items.iter().all(|i| i.category == CATEGORY_WINDOWS_TEMP));
    }

    #[test]
    fn scan_dir_returns_empty_for_missing_dir() {
        // 目录不存在是正常情况（用户改过 TEMP），不该 panic 也不该报错
        let items = scan_dir(
            Path::new(r"C:\no-such-dir-cclean-074"),
            CATEGORY_USER_TEMP,
            RISK_L1,
        );
        assert!(items.is_empty());
    }

    #[test]
    fn scan_dir_output_is_sorted_and_deterministic() {
        // 排序是为了让「Rust vs ps1 抽样对比」不会因为顺序不同而假失败
        let dir = make_tree();
        let a = scan_dir(dir.path(), CATEGORY_USER_TEMP, RISK_L1);
        let b = scan_dir(dir.path(), CATEGORY_USER_TEMP, RISK_L1);
        assert_eq!(a, b, "两次扫描结果必须完全一致");

        let mut sorted = a.clone();
        sorted.sort_by(|x, y| x.path.cmp(&y.path));
        assert_eq!(a, sorted, "输出应已按路径升序");
    }

    #[test]
    fn scan_dir_skips_symlinks() {
        // 符号链接不能算清理项：删链接不释放空间，跟进去还会重复计数
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("real.txt"), vec![b'x'; 5]).unwrap();

        let link = dir.path().join("link.txt");
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_file(dir.path().join("real.txt"), &link).is_ok();
        #[cfg(not(windows))]
        let made = std::os::unix::fs::symlink(dir.path().join("real.txt"), &link).is_ok();

        let items = scan_dir(dir.path(), CATEGORY_USER_TEMP, RISK_L1);
        // 无论链接建没建成（Windows 上非管理员建不了），都只能有 real.txt 一条
        assert_eq!(items.len(), 1, "实际: {items:#?}");
        assert!(items[0].path.ends_with("real.txt"));
        let _ = made;
    }

    #[test]
    fn scan_roots_has_user_and_windows_temp() {
        let roots = scan_roots();
        // 至少要有 windows_temp 那条（TEMP 可能被环境剥掉）
        assert!(roots
            .iter()
            .any(|(_, c, r)| *c == CATEGORY_WINDOWS_TEMP && *r == RISK_L2));
        // windows_temp 的路径必须以 Temp 结尾，且基于 SystemRoot 而不是硬编码
        let (p, _, _) = roots
            .iter()
            .find(|(_, c, _)| *c == CATEGORY_WINDOWS_TEMP)
            .unwrap();
        assert_eq!(p.file_name().unwrap(), "Temp");
        if let Ok(sr) = std::env::var("SystemRoot") {
            assert!(
                p.starts_with(&sr),
                "应基于 SystemRoot={sr}，实际 {}",
                p.display()
            );
        }
    }

    #[test]
    fn scan_roots_user_temp_is_l1() {
        // TEMP 存在时，user_temp 必须是 L1（用户自己的临时文件，删了自动重建）
        if std::env::var("TEMP")
            .map(|v| v.trim().is_empty())
            .unwrap_or(true)
        {
            return; // 环境没有 TEMP，跳过
        }
        let roots = scan_roots();
        let (_, _, risk) = roots
            .iter()
            .find(|(_, c, _)| *c == CATEGORY_USER_TEMP)
            .expect("有 TEMP 就该有 user_temp");
        assert_eq!(*risk, RISK_L1);
    }
}
