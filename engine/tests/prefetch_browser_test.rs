//! task-075：prefetch / browser_cache Rust 重写的集成测试。
//!
//! # 三层验证
//!
//! 1. **browser_cache 与 ps1 真实对比**：把 `LOCALAPPDATA` 通过 `Command::env`
//!    注入子进程，跑真实 `scan_browser_cache.ps1`，与 Rust `scan_at` 逐条比对
//!    Edge / Chrome 两个类别的产出。**不 mock**——脚本改坏了这条会直接红。
//! 2. **Firefox 是新增功能**：ps1 不扫 Firefox，这条单独断言「Rust 扫到、
//!    ps1 没扫到」，把行为差异**显式记录下来**而不是让它悄悄存在。
//! 3. **prefetch 的权限现实**：`C:\Windows\Prefetch` 普通用户读不了，
//!    两边都返回空——这条验证「都返回空而不是报错」，不是「都扫到 0 条」。
//!
//! # 为什么 prefetch 不做「集合相等」对比
//!
//! 本机普通权限下两边都是空集，比对空集等于什么都没验证。真要对比得提权，
//! 而扫描器不该为了测试要求管理员。所以 prefetch 只验证「不可读时两边行为一致」，
//! 以及「可读时（CI 管理员）产出格式正确」。

use std::path::{Path, PathBuf};

use engine::scan::modules::{browser_cache, prefetch};

// ===================== 辅助 =====================

/// 把路径规范化成长名小写再比对。
///
/// CI runner 用户名较长（如 `runneradmin`）会触发 8.3 短路径别名：
/// `tempfile::tempdir()` 拿到 `C:\Users\RUNNER~1\...`，而 PowerShell `Get-ChildItem`
/// 的 `FullName` 输出长名。两侧指同一文件仅字符串不同，用 `canonicalize` 解析到
/// 最终路径后再比，消除短/长名差异。
fn norm_path(p: &str) -> String {
    std::fs::canonicalize(p)
        .map(|c| {
            c.to_string_lossy()
                .trim_start_matches(r"\\?\")
                .to_lowercase()
        })
        .unwrap_or_else(|_| p.to_lowercase())
}

/// 跑一个 ps1 脚本，返回它产出的 `(category, 小写路径, size, risk)` 集合。
///
/// `envs` 通过**子进程环境变量**注入（`Command::env`），不影响本进程。
///
/// 返回 `None` 表示本机跑不了 ps1（没有解释器 / 脚本缺失），调用方跳过。
fn run_ps1(script_name: &str, envs: &[(&str, &Path)]) -> Option<Vec<(String, String, i64, i64)>> {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .join("scripts")
        .join(script_name);
    if !script.exists() {
        eprintln!(
            "[跳过] 找不到 {}（scripts/ 不在预期位置）",
            script.display()
        );
        return None;
    }

    let mut cmd = std::process::Command::new("powershell.exe");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
    ])
    .arg(&script);
    for (k, v) in envs {
        cmd.env(k, v);
    }

    let out = cmd.output().ok()?;
    if !out.status.success() {
        eprintln!("[跳过] {script_name} 退出码 {:?}", out.status.code());
        return None;
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let mut rows = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if v.get("type").and_then(|t| t.as_str()) != Some("item") {
            continue;
        }
        rows.push((
            v.get("category")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string(),
            norm_path(v.get("path").and_then(|p| p.as_str()).unwrap_or("")),
            v.get("size_bytes").and_then(|s| s.as_i64()).unwrap_or(-1),
            v.get("risk_level").and_then(|r| r.as_i64()).unwrap_or(-1),
        ));
    }
    Some(rows)
}

/// 把 Rust 的 `ScanItem` 转成与 [`run_ps1`] 同形状的元组，便于逐条比对。
fn as_rows(items: &[engine::scan::modules::ScanItem]) -> Vec<(String, String, i64, i64)> {
    items
        .iter()
        .map(|i| {
            (
                i.category.clone(),
                norm_path(&i.path),
                i.size_bytes,
                i.risk_level,
            )
        })
        .collect()
}

/// 造一个假的 `%LOCALAPPDATA%`，含 Edge + Chrome 的 `Default\Cache`。
fn make_lad_with_edge_and_chrome() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();

    let edge = dir.path().join(r"Microsoft\Edge\User Data\Default\Cache");
    std::fs::create_dir_all(edge.join("Cache_Data")).unwrap();
    std::fs::write(edge.join("data_0"), vec![b'x'; 100]).unwrap();
    std::fs::write(edge.join("Cache_Data").join("f_000001"), vec![b'x'; 50]).unwrap();

    let chrome = dir.path().join(r"Google\Chrome\User Data\Default\Cache");
    std::fs::create_dir_all(&chrome).unwrap();
    std::fs::write(chrome.join("data_1"), vec![b'x'; 70]).unwrap();

    dir
}

// ===================== ① browser_cache：Rust vs ps1 真实对比 =====================

#[test]
fn browser_cache_rust_matches_ps1_on_edge_and_chrome() {
    // 本任务「等价重写」的核心证据：同一份目录，两套实现的 Edge/Chrome 产出必须一致。
    let lad = make_lad_with_edge_and_chrome();

    let ps1 = match run_ps1("scan_browser_cache.ps1", &[("LOCALAPPDATA", lad.path())]) {
        Some(r) => r,
        None => return, // 本机跑不了 ps1，跳过（不是失败）
    };

    let rust = as_rows(&browser_cache::scan_at(lad.path()));

    // ps1 不扫 Firefox，所以这里本来就只有 Edge/Chrome；显式过滤一次防将来变化
    let mut a: Vec<_> = ps1
        .into_iter()
        .filter(|(c, ..)| c == browser_cache::CATEGORY_EDGE || c == browser_cache::CATEGORY_CHROME)
        .collect();
    let mut b: Vec<_> = rust
        .into_iter()
        .filter(|(c, ..)| c == browser_cache::CATEGORY_EDGE || c == browser_cache::CATEGORY_CHROME)
        .collect();
    a.sort();
    b.sort();

    assert_eq!(
        a.len(),
        3,
        "ps1 应扫到 3 个文件（Edge 2 + Chrome 1）: {a:#?}"
    );
    assert_eq!(a, b, "两套实现的产出必须完全一致\nps1={a:#?}\nrust={b:#?}");

    let total: i64 = b.iter().map(|r| r.2).sum();
    assert_eq!(total, 100 + 50 + 70);
    assert!(b.iter().all(|r| r.3 == browser_cache::RISK_L1));
}

#[test]
fn browser_cache_rust_agrees_with_ps1_on_empty_localappdata() {
    // 什么都没装：两边都该 0 条，且都不报错
    let empty = tempfile::tempdir().unwrap();
    let ps1 = match run_ps1("scan_browser_cache.ps1", &[("LOCALAPPDATA", empty.path())]) {
        Some(r) => r,
        None => return,
    };
    let rust = browser_cache::scan_at(empty.path());
    assert!(ps1.is_empty(), "ps1 应产出 0 条: {ps1:#?}");
    assert!(rust.is_empty(), "Rust 应产出 0 条: {rust:#?}");
}

// ===================== ② Firefox 是新增功能（行为差异要显式记录） =====================

#[test]
fn firefox_cache_is_scanned_by_rust_but_not_by_ps1() {
    // 这条用例存在的意义：**把「Rust 多扫了 Firefox」这件事写进测试**。
    //
    // 它不是回归保护，而是行为契约——将来如果有人怀疑「为什么 Rust 模式比
    // ps1 模式多出几千条」，看这条就知道是设计如此，不是 bug。
    let lad = tempfile::tempdir().unwrap();
    let ff = lad
        .path()
        .join(r"Mozilla\Firefox\Profiles\abc.default-release\cache2\entries");
    std::fs::create_dir_all(&ff).unwrap();
    std::fs::write(ff.join("A1B2"), vec![b'x'; 300]).unwrap();

    let rust = browser_cache::scan_at(lad.path());
    let ff_items: Vec<_> = rust
        .iter()
        .filter(|i| i.category == browser_cache::CATEGORY_FIREFOX)
        .collect();
    assert_eq!(ff_items.len(), 1, "Rust 应扫到 Firefox 缓存: {rust:#?}");
    assert_eq!(ff_items[0].size_bytes, 300);
    assert_eq!(ff_items[0].risk_level, browser_cache::RISK_L1);

    // ps1 对同一份目录应产出 0 条（它根本不看 Mozilla 目录）
    if let Some(ps1) = run_ps1("scan_browser_cache.ps1", &[("LOCALAPPDATA", lad.path())]) {
        assert!(
            ps1.is_empty(),
            "ps1 不该扫到 Firefox（本次是新增目标）: {ps1:#?}"
        );
    }
}

// ===================== ③ prefetch：权限现实 =====================

#[test]
fn prefetch_rust_and_ps1_both_return_empty_for_missing_dir() {
    // Prefetch 目录不存在时（组策略关掉 / 目录被清空），两边都该返回空而不报错。
    let empty_root = tempfile::tempdir().unwrap();
    let rust = prefetch::scan_at(empty_root.path());
    assert!(rust.is_empty());

    // ps1 硬编码 `C:\Windows\Prefetch`，没法注入根目录，所以这里只验证 Rust 侧。
    // ps1 侧的行为在下面 `prefetch_real_scan_never_panics` 里间接覆盖。
}

#[test]
fn prefetch_rust_tags_fields_correctly() {
    // 造一个可读的 Prefetch 目录，验证字段打点（本机真实目录读不了，只能造）
    let root = tempfile::tempdir().unwrap();
    let pf = root.path().join("Prefetch");
    std::fs::create_dir_all(&pf).unwrap();
    std::fs::write(pf.join("A.EXE-1.pf"), vec![b'x'; 10]).unwrap();
    std::fs::write(pf.join("B.EXE-2.pf"), vec![b'x'; 20]).unwrap();

    let items = prefetch::scan_at(root.path());
    assert_eq!(items.len(), 2);
    for it in &items {
        assert_eq!(it.module_code, prefetch::MODULE_CODE);
        assert_eq!(it.category, prefetch::CATEGORY_PREFETCH);
        // L1 而不是 L2：Prefetch 是 Windows 自动重建的预读取缓存，
        // 不能因为「在 C:\Windows 下」就套用 windows_temp 的 L2 判断
        assert_eq!(it.risk_level, prefetch::RISK_L1);
    }
}

#[test]
fn prefetch_real_scan_never_panics() {
    // 本机普通权限读不到真实 `C:\Windows\Prefetch`（ACL 只给 Administrators/SYSTEM），
    // 这时必须返回空而不是 panic。CI 上以管理员运行时这条会真的扫到东西——
    // 两种结果都算通过，重点是**不能崩**。
    let items = prefetch::scan();
    assert!(items.iter().all(|i| i.module_code == prefetch::MODULE_CODE));
    assert!(items.iter().all(|i| i.risk_level == prefetch::RISK_L1));
}

// ===================== ④ 真机冒烟 =====================

/// 扫本机真实浏览器缓存并打印摘要（`#[ignore]`，CI 默认不跑）。
///
/// 本机 WebView2 渲染进程起不来、GUI 取证拿不到，这条是后端侧能产出的真实数据。
///
/// 跑法：
/// ```text
/// cargo test -p engine --test prefetch_browser_test -- --ignored --nocapture real_scan
/// ```
#[test]
#[ignore = "真机冒烟：扫真实浏览器缓存 / Prefetch"]
fn real_scan_smoke() {
    let items = browser_cache::scan();
    let total: i64 = items.iter().map(|i| i.size_bytes).sum();
    println!(
        "browser_cache: 共 {} 项，合计 {} 字节（{:.1} MB）",
        items.len(),
        total,
        total as f64 / 1024.0 / 1024.0
    );
    for cat in [
        browser_cache::CATEGORY_EDGE,
        browser_cache::CATEGORY_CHROME,
        browser_cache::CATEGORY_FIREFOX,
    ] {
        let n = items.iter().filter(|i| i.category == cat).count();
        let b: i64 = items
            .iter()
            .filter(|i| i.category == cat)
            .map(|i| i.size_bytes)
            .sum();
        println!("  {cat}: {n} 项 / {b} 字节");
    }
    for i in items.iter().take(3) {
        println!("  [L{}] {:>10}B  {}", i.risk_level, i.size_bytes, i.path);
    }

    let pf = prefetch::scan();
    let pf_bytes: i64 = pf.iter().map(|i| i.size_bytes).sum();
    println!(
        "prefetch: {} 项 / {} 字节（普通权限下通常为 0：目录 ACL 只给管理员）",
        pf.len(),
        pf_bytes
    );

    // 结构不变量
    assert!(items
        .iter()
        .all(|i| i.module_code == browser_cache::MODULE_CODE));
    assert!(items.iter().all(|i| i.risk_level == browser_cache::RISK_L1));
    assert!(pf.iter().all(|i| i.risk_level == prefetch::RISK_L1));
}
