//! task-086：prefetch / browser_cache 纯 Rust 扫描集成测试。
//!
//! 覆盖 Edge / Chrome / Firefox 缓存、空目录、Prefetch 缺失目录与字段打点，
//! 以及真实目录扫描不 panic 的结构不变量。

use engine::scan::modules::{browser_cache, prefetch};

// ===================== 辅助 =====================

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

// ===================== ① browser_cache：受控目录扫描 =====================

#[test]
fn browser_cache_scans_edge_and_chrome() {
    let lad = make_lad_with_edge_and_chrome();
    let items = browser_cache::scan_at(lad.path());
    let caches: Vec<_> = items
        .iter()
        .filter(|i| {
            i.category == browser_cache::CATEGORY_EDGE
                || i.category == browser_cache::CATEGORY_CHROME
        })
        .collect();

    assert_eq!(
        caches.len(),
        3,
        "应扫到 Edge 2 个 + Chrome 1 个文件: {items:#?}"
    );
    let total: i64 = caches.iter().map(|i| i.size_bytes).sum();
    assert_eq!(total, 100 + 50 + 70);
    assert!(caches
        .iter()
        .all(|i| i.risk_level == browser_cache::RISK_L1));
    assert!(items
        .iter()
        .all(|i| i.module_code == browser_cache::MODULE_CODE));
}

#[test]
fn browser_cache_empty_localappdata_returns_empty() {
    let empty = tempfile::tempdir().unwrap();
    let items = browser_cache::scan_at(empty.path());
    assert!(items.is_empty(), "空 LOCALAPPDATA 应产出 0 条: {items:#?}");
}

// ===================== ② Firefox 缓存 =====================

#[test]
fn firefox_cache_is_scanned() {
    let lad = tempfile::tempdir().unwrap();
    let ff = lad
        .path()
        .join(r"Mozilla\Firefox\Profiles\abc.default-release\cache2\entries");
    std::fs::create_dir_all(&ff).unwrap();
    std::fs::write(ff.join("A1B2"), vec![b'x'; 300]).unwrap();

    let items = browser_cache::scan_at(lad.path());
    let ff_items: Vec<_> = items
        .iter()
        .filter(|i| i.category == browser_cache::CATEGORY_FIREFOX)
        .collect();
    assert_eq!(ff_items.len(), 1, "应扫到 Firefox 缓存: {items:#?}");
    assert_eq!(ff_items[0].size_bytes, 300);
    assert_eq!(ff_items[0].risk_level, browser_cache::RISK_L1);
    assert_eq!(ff_items[0].module_code, browser_cache::MODULE_CODE);
}

// ===================== ③ prefetch：目录与权限边界 =====================

#[test]
fn prefetch_returns_empty_for_missing_dir() {
    // Prefetch 目录不存在时（组策略关掉 / 目录被清空）应返回空而不报错。
    let empty_root = tempfile::tempdir().unwrap();
    let items = prefetch::scan_at(empty_root.path());
    assert!(items.is_empty());
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
