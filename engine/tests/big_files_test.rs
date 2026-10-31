//! task-066：大文件地图扫描函数用例。
//!
//! 全部在 `tempfile` 临时目录上跑**真实文件系统**，不 mock：
//! 这几个函数的核心价值就是「跟 Windows 文件系统打交道」，mock 掉 fs 之后
//! 测的就只剩自己的假数据了。临时目录能完整复现「目录嵌套 / 大小求和 /
//! 跳过策略 / reparse point」这些真实行为。
//!
//! 唯一没法在这里测的是回收站的**大小**——`SHQueryRecycleBinW` 查的是整盘，
//! 无法指向临时目录。所以回收站只断言结构正确（见最后一个用例的说明）。

use std::path::Path;

use engine::scan::big_files::{
    dir_total_bytes, scan_all, scan_hiberfil, scan_recycle_bin, scan_top_files, scan_windows_old,
    should_skip_dir, BigFilesReport, TopFilesOptions, CATEGORY_HIBERFIL, CATEGORY_RECYCLE_BIN,
    CATEGORY_TOP_FILES, CATEGORY_WINDOWS_OLD,
};

/// 造一个指定大小的文件（用 `set_len` 造稀疏文件，不真的写几百 MB）。
fn make_file(path: &Path, size: u64) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let f = std::fs::File::create(path).unwrap();
    f.set_len(size).unwrap();
}

/// 测试用的小上限参数：门槛 1 字节，避免造 100 MB 文件。
fn small_opts(limit: usize) -> TopFilesOptions {
    TopFilesOptions {
        limit,
        min_size_bytes: 1,
        max_entries: 100_000,
        budget: std::time::Duration::from_secs(30),
    }
}

// ===================== hiberfil.sys =====================

#[test]
fn hiberfil_reads_size_when_present() {
    let dir = tempfile::tempdir().unwrap();
    make_file(&dir.path().join("hiberfil.sys"), 4096);

    let item = scan_hiberfil(dir.path());
    assert_eq!(item.size_bytes, 4096);
    assert_eq!(item.category, CATEGORY_HIBERFIL);
    assert!(item.path.ends_with("hiberfil.sys"), "实际: {}", item.path);
    // 建议必须能照着做（含 powercfg），而不是一句「清理它」
    assert!(
        item.recommendation.contains("powercfg"),
        "实际: {}",
        item.recommendation
    );
}

#[test]
fn hiberfil_reports_zero_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    let item = scan_hiberfil(dir.path());

    // 不存在 = 休眠已关闭，这是正常状态，不能报错也不能编个数字
    assert_eq!(item.size_bytes, 0);
    assert_eq!(item.category, CATEGORY_HIBERFIL);
    assert!(
        item.recommendation.contains("休眠"),
        "应说明休眠状态，实际: {}",
        item.recommendation
    );
}

// ===================== Windows.old =====================

#[test]
fn windows_old_sums_nested_directory() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("Windows.old");
    make_file(&old.join("a.bin"), 1000);
    make_file(&old.join("sub/b.bin"), 2000);
    make_file(&old.join("sub/deep/c.bin"), 3000);

    let item = scan_windows_old(dir.path(), small_opts(10));
    assert_eq!(item.size_bytes, 6000, "必须递归求和");
    assert_eq!(item.category, CATEGORY_WINDOWS_OLD);
    assert!(
        item.recommendation.contains("磁盘清理"),
        "建议要指向正规入口"
    );
}

#[test]
fn windows_old_zero_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    let item = scan_windows_old(dir.path(), small_opts(10));
    assert_eq!(item.size_bytes, 0);
    assert!(
        item.recommendation.contains("不存在"),
        "实际: {}",
        item.recommendation
    );
}

// ===================== 目录求和 =====================

#[test]
fn dir_total_bytes_sums_all_files() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target");
    make_file(&target.join("x.bin"), 512);
    make_file(&target.join("nested/y.bin"), 512);
    make_file(&target.join("nested/deeper/z.bin"), 1024);

    assert_eq!(dir_total_bytes(&target), 2048);
    // 不存在的目录 → 0（不是 panic，也不是 Err）
    assert_eq!(dir_total_bytes(&dir.path().join("nope")), 0);
}

// ===================== Top N =====================

#[test]
fn top_files_sorts_desc_and_respects_limit() {
    let dir = tempfile::tempdir().unwrap();
    make_file(&dir.path().join("small.bin"), 10);
    make_file(&dir.path().join("big.bin"), 3000);
    make_file(&dir.path().join("mid.bin"), 2000);
    make_file(&dir.path().join("tiny.bin"), 5);

    let (items, _, _) = scan_top_files(dir.path(), small_opts(2));

    assert_eq!(items.len(), 2, "limit=2 只能给 2 条");
    assert_eq!(items[0].size_bytes, 3000);
    assert_eq!(items[1].size_bytes, 2000);
    assert!(items[0].path.ends_with("big.bin"));
    assert!(items.iter().all(|i| i.category == CATEGORY_TOP_FILES));
}

#[test]
fn top_files_skips_system_directories() {
    let dir = tempfile::tempdir().unwrap();
    // 系统目录里的「巨大文件」不能出现在 Top100 —— 用户不该动它
    make_file(&dir.path().join("Windows/huge.bin"), 900_000);
    make_file(&dir.path().join("Program Files/pkg.bin"), 800_000);
    make_file(&dir.path().join("$Recycle.Bin/x.bin"), 700_000);
    make_file(&dir.path().join("Windows.old/old.bin"), 600_000);
    // 用户自己的文件应该出现
    make_file(&dir.path().join("Users/me/movie.mkv"), 500_000);

    let (items, _, _) = scan_top_files(dir.path(), small_opts(10));

    let paths: Vec<&str> = items.iter().map(|i| i.path.as_str()).collect();
    assert_eq!(paths.len(), 1, "只有用户文件该入榜，实际: {paths:?}");
    assert!(paths[0].ends_with("movie.mkv"), "实际: {paths:?}");
}

#[test]
fn top_files_skips_root_system_files() {
    let dir = tempfile::tempdir().unwrap();
    // 根目录这三个是「删不掉也不该删」的，绝不能出现在 Top100 里
    make_file(&dir.path().join("pagefile.sys"), 500_000);
    make_file(&dir.path().join("swapfile.sys"), 400_000);
    make_file(&dir.path().join("hiberfil.sys"), 300_000);
    make_file(&dir.path().join("user.iso"), 100_000);

    let (items, _, _) = scan_top_files(dir.path(), small_opts(10));

    let paths: Vec<&str> = items.iter().map(|i| i.path.as_str()).collect();
    assert_eq!(paths.len(), 1, "根目录系统文件必须跳过，实际: {paths:?}");
    assert!(paths[0].ends_with("user.iso"));
}

#[test]
fn top_files_respects_min_size() {
    let dir = tempfile::tempdir().unwrap();
    make_file(&dir.path().join("a.bin"), 100);
    make_file(&dir.path().join("b.bin"), 5000);

    let opts = TopFilesOptions {
        limit: 10,
        min_size_bytes: 1000,
        max_entries: 100_000,
        budget: std::time::Duration::from_secs(30),
    };
    let (items, _, _) = scan_top_files(dir.path(), opts);
    assert_eq!(items.len(), 1);
    assert!(items[0].path.ends_with("b.bin"));
}

#[test]
fn top_files_counts_scanned_entries() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..5 {
        make_file(&dir.path().join(format!("f{i}.bin")), 10);
    }
    let (_, scanned, truncated) = scan_top_files(dir.path(), small_opts(10));
    assert_eq!(scanned, 5, "应如实报告看了多少个条目");
    assert!(!truncated, "没撞上限就不该标 truncated");
}

#[test]
fn top_files_marks_truncated_when_entry_cap_hit() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..20 {
        make_file(&dir.path().join(format!("f{i}.bin")), 10);
    }
    let opts = TopFilesOptions {
        limit: 5,
        min_size_bytes: 1,
        max_entries: 5,
        budget: std::time::Duration::from_secs(30),
    };
    let (_, scanned, truncated) = scan_top_files(dir.path(), opts);
    // 撞上限要标出来——否则用户会把「只扫了一部分」当成「全盘就这么大」
    assert!(truncated, "撞到条目上限必须标 truncated");
    assert!(scanned <= 5, "实际: {scanned}");
}

#[test]
fn skip_dir_matching_is_case_insensitive() {
    for name in [
        "Windows",
        "WINDOWS",
        "program files",
        "Program Files (x86)",
        "$Recycle.Bin",
    ] {
        assert!(should_skip_dir(name), "{name} 应该被跳过");
    }
    for name in ["Users", "Documents", "AppData", "Downloads"] {
        assert!(!should_skip_dir(name), "{name} 不该被跳过");
    }
}

// ===================== 汇总 =====================

#[test]
fn scan_all_returns_three_categories_plus_top_files() {
    let dir = tempfile::tempdir().unwrap();
    make_file(&dir.path().join("hiberfil.sys"), 1000);
    make_file(&dir.path().join("Windows.old/a.bin"), 2000);
    make_file(&dir.path().join("Users/me/big.iso"), 3000);

    let report = scan_all(dir.path(), small_opts(10));

    // 三张卡片必须固定 3 条（前端是固定布局，缺一张像加载失败）
    assert_eq!(report.categories.len(), 3);
    let cats: Vec<&str> = report
        .categories
        .iter()
        .map(|c| c.category.as_str())
        .collect();
    assert!(cats.contains(&CATEGORY_HIBERFIL));
    assert!(cats.contains(&CATEGORY_WINDOWS_OLD));
    assert!(cats.contains(&CATEGORY_RECYCLE_BIN));

    // Top100 里只有用户文件
    assert_eq!(report.top_files.len(), 1);
    assert!(report.top_files[0].path.ends_with("big.iso"));

    // 合计 = 三分类之和 + Top 之和（跳过策略保证不重复计算）。
    //
    // 注意**不能用固定数字**：回收站那一项走的是 `SHQueryRecycleBinW`，
    // 查的是本机整盘回收站（临时目录也解析到同一个盘），每台机器都不同。
    // 所以断言的是**不变量**——total 恰好等于各部分之和，而不是某个具体值。
    let cat_sum: u64 = report.categories.iter().map(|c| c.size_bytes).sum();
    let top_sum: u64 = report.top_files.iter().map(|f| f.size_bytes).sum();
    assert_eq!(report.total_bytes, cat_sum + top_sum);
    // 三分类里 hiberfil + Windows.old 是我们造出来的，必须精确对上
    let hiberfil = report
        .categories
        .iter()
        .find(|c| c.category == CATEGORY_HIBERFIL)
        .unwrap();
    let old = report
        .categories
        .iter()
        .find(|c| c.category == CATEGORY_WINDOWS_OLD)
        .unwrap();
    assert_eq!(hiberfil.size_bytes, 1000);
    assert_eq!(old.size_bytes, 2000);
    assert!(report.scanned_entries >= 1);
    assert!(!report.truncated);
}

#[test]
fn report_serializes_camel_case() {
    let dir = tempfile::tempdir().unwrap();
    make_file(&dir.path().join("hiberfil.sys"), 1024);

    let report = scan_all(dir.path(), small_opts(10));
    let v: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();

    // 前端 interface 用 camelCase，靠 #[serde(rename_all)] 对齐
    assert!(v["topFiles"].is_array(), "实际: {v}");
    assert!(v["totalBytes"].is_number());
    assert!(v["scannedEntries"].is_number());
    assert!(v["elapsedMs"].is_number());
    assert!(v["truncated"].is_boolean());
    let first = &v["categories"][0];
    assert!(first["sizeBytes"].is_number(), "实际: {first}");
    assert!(first["recommendation"].is_string());
    // 反面：绝不能出现 snake_case 键
    assert!(v.get("top_files").is_none());
    assert!(first.get("size_bytes").is_none());
}

#[test]
fn report_default_is_empty() {
    // Default 派生用于「扫描还没跑完」的占位态，字段不能是 NaN / 负数
    let r = BigFilesReport::default();
    assert_eq!(r.total_bytes, 0);
    assert!(r.categories.is_empty());
    assert!(r.top_files.is_empty());
}

#[test]
fn recycle_bin_returns_structure() {
    let dir = tempfile::tempdir().unwrap();
    let item = scan_recycle_bin(dir.path());

    // **只断言结构，不断言大小**：`SHQueryRecycleBinW` 查的是整盘回收站，
    // 无法指向临时目录，所以这里的大小是「本机真实回收站」的值，
    // 每台机器都不同。断言它是个合理的数（< 100 TB）即可。
    assert_eq!(item.category, CATEGORY_RECYCLE_BIN);
    assert!(
        item.size_bytes < 100 * 1024_u64.pow(4),
        "实际: {}",
        item.size_bytes
    );
    assert!(!item.recommendation.is_empty());
    assert!(
        item.path.to_lowercase().contains("recycle"),
        "实际: {}",
        item.path
    );
}
