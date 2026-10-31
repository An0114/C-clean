//! task-067：残留扫描（注册表↔文件交叉比对）与「移入保险箱」用例。
//!
//! 判定逻辑用纯函数 `cross_reference` 测（真注册表和真 Program Files 没法在
//! 单测里构造，而这一段恰恰最容易写错）；删除路径用 tempdir + 临时 DB 测真磁盘，
//! 因为它要验证的是「目录整体 rename 进保险箱、体积递归求和」这种实打实的行为。

use std::path::Path;

use engine::scan::residual::{
    clean_location, cross_reference, delete_residuals, expand_env, list_install_dirs,
    normalize_name, should_skip_dir_name, InstallDir, ResidualItem, UninstallEntry,
    CONFIDENCE_HIGH, CONFIDENCE_LOW, RESIDUAL_FILES, RESIDUAL_REGISTRY, RISK_L2,
};

fn entry(name: &str, loc: Option<&str>) -> UninstallEntry {
    UninstallEntry {
        display_name: name.to_string(),
        install_location: loc.map(|s| s.to_string()),
        registry_key: format!(r"HKLM\Software\...\Uninstall\{{{name}}}"),
    }
}

fn dir(path: &str, name: &str) -> InstallDir {
    InstallDir {
        path: std::path::PathBuf::from(path),
        name: name.to_string(),
    }
}

fn make_file(path: &Path, size: u64) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let f = std::fs::File::create(path).unwrap();
    f.set_len(size).unwrap();
}

fn setup_db() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = engine::db::conn::open(dir.path()).unwrap();
    (dir, conn)
}

// ===================== 类型 A：注册表残留 =====================

#[test]
fn type_a_when_install_location_missing() {
    let entries = vec![entry(
        "GhostApp",
        Some(r"C:\Program Files\Definitely-Not-Exists-CCLEAN"),
    )];
    let (reg, files) = cross_reference(&entries, &[]);

    assert_eq!(reg.len(), 1);
    assert_eq!(files.len(), 0);
    assert_eq!(reg[0].residual_type, RESIDUAL_REGISTRY);
    assert_eq!(reg[0].name, "GhostApp");
    assert_eq!(
        reg[0].confidence, CONFIDENCE_HIGH,
        "路径明确不存在 → 高置信"
    );
    assert_eq!(reg[0].risk_level, RISK_L2);
    assert_eq!(reg[0].size_bytes, 0, "目录都没了，体积必为 0");
    assert!(reg[0].registry_key.is_some(), "类型 A 必须带注册表键路径");
}

#[test]
fn type_a_skipped_when_location_exists() {
    // 用 tempdir 造一个真实存在的目录，避免依赖本机装了哪些软件
    let tmp = tempfile::tempdir().unwrap();
    let loc = tmp.path().to_string_lossy().to_string();
    let entries = vec![entry("InstalledApp", Some(&loc))];
    let (reg, _) = cross_reference(&entries, &[]);
    assert!(reg.is_empty(), "目录存在就不算残留");
}

#[test]
fn type_a_skipped_when_location_empty_or_truncated() {
    // 空 InstallLocation / 只有盘符的残缺值都判不了 —— 宁可漏报
    let entries = vec![
        entry("EmptyLoc", Some("")),
        entry("DriveOnly", Some("C:")),
        entry("NoLoc", None),
    ];
    let (reg, _) = cross_reference(&entries, &[]);
    assert!(reg.is_empty(), "判不了的不该硬报：{reg:?}");
}

// ===================== 类型 B：文件残留 =====================

#[test]
fn type_b_when_no_owner_found() {
    let dirs = vec![dir(r"C:\Program Files\OrphanApp", "OrphanApp")];
    // 不给 InstallLocation：给了它自己也会被判成类型 A（路径不存在），干扰断言
    let entries = vec![entry("SomethingElse", None)];

    let (reg, files) = cross_reference(&entries, &dirs);
    assert_eq!(reg.len(), 0);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].residual_type, RESIDUAL_FILES);
    assert_eq!(files[0].name, "OrphanApp");
    assert_eq!(
        files[0].confidence, CONFIDENCE_LOW,
        "「注册表里找不到」推不出「已卸载」，只能给低置信"
    );
    assert_eq!(
        files[0].risk_level, RISK_L2,
        "删 Program Files 必须逐项确认"
    );
    assert!(files[0].registry_key.is_none());
    assert!(files[0].note.contains("保险箱"), "note 要说明可恢复");
}

#[test]
fn type_b_not_flagged_when_install_location_is_parent() {
    // 常见真实数据：装在 ...\Vendor\Product，注册表只记 ...\Vendor
    let dirs = vec![dir(r"C:\Program Files\Vendor\Product", "Product")];
    let entries = vec![entry("Product 1.0", Some(r"C:\Program Files\Vendor"))];

    let (_, files) = cross_reference(&entries, &dirs);
    assert!(
        files.is_empty(),
        "InstallLocation 是父目录时必须算「有归属」，否则误报：{files:?}"
    );
}

#[test]
fn type_b_not_flagged_when_dir_name_matches_location_tail() {
    let dirs = vec![dir(r"C:\Program Files\MyTool", "MyTool")];
    let entries = vec![entry("Whatever Name", Some(r"C:\Program Files\MyTool"))];

    let (_, files) = cross_reference(&entries, &dirs);
    assert!(files.is_empty(), "路径完全相等必须算有归属");
}

#[test]
fn type_b_not_flagged_when_display_name_fuzzy_matches() {
    // 只有 DisplayName 能对上：带版本号/架构后缀的情况
    let dirs = vec![dir(r"C:\Program Files\VisualStudio", "VisualStudio")];
    let entries = vec![entry("Visual Studio Community 2022 (x64)", None)];

    let (_, files) = cross_reference(&entries, &dirs);
    assert!(
        files.is_empty(),
        "DisplayName 模糊匹配要能命中（宁可漏报不可误报）：{files:?}"
    );
}

#[test]
fn type_b_flags_dir_with_completely_unrelated_entries() {
    // 反向验证：名字完全不沾边时必须报出来，否则这个功能就没用了
    let dirs = vec![dir(r"C:\Program Files\LeftoverFolder", "LeftoverFolder")];
    let entries = vec![entry("Completely Different Thing", None)];

    let (_, files) = cross_reference(&entries, &dirs);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name, "LeftoverFolder");
}

// ===================== 名称/路径归一化 =====================

#[test]
fn normalize_name_strips_punctuation_and_case() {
    assert_eq!(
        normalize_name("Microsoft Visual C++ 2015 (x64)"),
        "microsoftvisualc2015x64"
    );
    assert_eq!(normalize_name("7-Zip"), "7zip");
    // 中文要保留（is_alphanumeric 对 CJK 为 true）
    assert_eq!(normalize_name("腾讯 QQ"), "腾讯qq");
    assert_eq!(normalize_name(""), "");
}

#[test]
fn clean_location_removes_quotes_and_trailing_separator() {
    assert_eq!(
        clean_location(r#""C:\Program Files\Foo\"#),
        r"C:\Program Files\Foo"
    );
    assert_eq!(clean_location(r"C:\Foo\\"), r"C:\Foo");
    // 盘符根不能把分隔符吃掉（否则 C:\ 变成 C:，语义变了）
    assert_eq!(clean_location(r"C:\"), r"C:\");
}

#[test]
fn clean_location_expands_environment_variables() {
    // %SystemRoot% 在任何 Windows 上都存在，比 %ProgramFiles% 更可靠
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    assert_eq!(clean_location(r"%SystemRoot%\Foo"), format!(r"{root}\Foo"));
    // 展开不了的原样保留，别把路径吃掉
    assert_eq!(
        expand_env(r"%CCLEAN_NO_SUCH_VAR%\x"),
        r"%CCLEAN_NO_SUCH_VAR%\x"
    );
}

#[test]
fn skip_dir_name_matches_case_insensitively() {
    for name in ["Common Files", "WindowsApps", "Microsoft", "Package Cache"] {
        assert!(should_skip_dir_name(name), "{name} 应跳过");
    }
    for name in ["7-Zip", "Notepad++", "MyApp"] {
        assert!(!should_skip_dir_name(name), "{name} 不该跳过");
    }
}

// ===================== 目录枚举 =====================

#[test]
fn list_install_dirs_returns_top_level_only_and_skips_blacklist() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("7-Zip")).unwrap();
    std::fs::create_dir_all(root.join("Common Files")).unwrap();
    std::fs::create_dir_all(root.join("Microsoft")).unwrap();
    // 嵌套目录不该被单独列出（只取顶层）
    std::fs::create_dir_all(root.join("7-Zip").join("Lang")).unwrap();

    let dirs = list_install_dirs(root);
    let names: Vec<&str> = dirs.iter().map(|d| d.name.as_str()).collect();

    assert_eq!(
        names,
        vec!["7-Zip"],
        "只列顶层且跳过黑名单，实际: {names:?}"
    );
}

#[test]
fn list_install_dirs_on_missing_root_returns_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let dirs = list_install_dirs(&tmp.path().join("no-such-dir"));
    assert!(dirs.is_empty(), "根不存在不是错误，返回空即可");
}

// ===================== 删除（移入保险箱） =====================

#[test]
fn delete_moves_whole_directory_into_safe_box() {
    let (tmp, conn) = setup_db();
    let box_dir = tmp.path().join("box");
    // 造一个「孤儿目录」：两个文件共 3000 字节，含子目录
    let junk = tmp.path().join("ProgramFiles").join("JunkApp");
    make_file(&junk.join("a.bin"), 1000);
    make_file(&junk.join("sub").join("b.bin"), 2000);

    let r = delete_residuals(&[junk.to_string_lossy().to_string()], &conn, &box_dir);

    assert_eq!(r.moved.len(), 1, "failed={:?}", r.failed);
    assert_eq!(
        r.bytes_moved, 3000,
        "目录体积必须递归求和——`metadata().len()` 对目录返回 0，会让保险箱配额失效"
    );
    assert!(!junk.exists(), "原目录应已整体移走");

    // 保险箱里是**一个完整目录**，内部结构保留（恢复后目录树原样回来）
    let entries: Vec<_> = std::fs::read_dir(&box_dir).unwrap().flatten().collect();
    assert_eq!(entries.len(), 1, "保险箱里应只有一项");
    let boxed = entries[0].path();
    assert!(boxed.is_dir(), "移进来的应该是目录：{}", boxed.display());
    assert!(
        boxed.join("sub").join("b.bin").is_file(),
        "目录内部结构必须完整保留"
    );

    // safe_box 表里要有一行（7 天后 purge_expired 靠它自动清）
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM safe_box", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1, "移入保险箱必须落库，否则 7 天后不会被自动清理");
}

#[test]
fn delete_refuses_personal_desktop_path() {
    // 桌面在 L3 红线里（PERSONAL_DIRS），必须被唯一入口拒绝
    let (tmp, conn) = setup_db();
    let box_dir = tmp.path().join("box");
    let userprofile = std::env::var("USERPROFILE").unwrap_or_else(|_| r"C:\Users\test".to_string());
    let desktop_junk = format!(r"{userprofile}\Desktop\cclean-should-never-be-moved");

    let r = delete_residuals(&[desktop_junk], &conn, &box_dir);

    assert!(r.moved.is_empty(), "桌面路径绝不能进保险箱：{:?}", r.moved);
    assert_eq!(r.failed.len(), 1, "必须记一条失败并带上原因");
    assert!(!r.failed[0].reason.is_empty(), "失败原因不能是空字符串");
}

#[test]
fn delete_skips_missing_path_without_failing() {
    // 用户刚手动删过：不该报错（幂等），也不该计入释放量
    let (tmp, conn) = setup_db();
    let box_dir = tmp.path().join("box");
    let ghost = tmp.path().join("GhostApp");

    let r = delete_residuals(&[ghost.to_string_lossy().to_string()], &conn, &box_dir);
    // 完全 no-op：既不能「假装移走了」，也不必报错打扰用户
    assert!(r.moved.is_empty(), "不存在的路径不能被记成已移入");
    assert!(r.failed.is_empty(), "不存在的路径不该报错：{:?}", r.failed);
    assert_eq!(r.bytes_moved, 0);
}

#[test]
fn delete_empty_input_is_noop() {
    let (tmp, conn) = setup_db();
    let r = delete_residuals(&[], &conn, &tmp.path().join("box"));
    assert_eq!(r.moved.len(), 0);
    assert_eq!(r.failed.len(), 0);
    assert_eq!(r.bytes_moved, 0);
}

// ===================== 序列化契约 =====================

#[test]
fn report_and_items_serialize_camel_case() {
    let item = ResidualItem {
        name: "X".into(),
        path: r"C:\X".into(),
        residual_type: RESIDUAL_FILES.into(),
        size_bytes: 1,
        registry_key: None,
        confidence: CONFIDENCE_LOW.into(),
        risk_level: RISK_L2.into(),
        note: "n".into(),
    };
    let v: serde_json::Value = serde_json::to_value(&item).unwrap();

    // 前端 interface 用 camelCase，靠 #[serde(rename_all)] 对齐
    assert!(v["sizeBytes"].is_number(), "实际: {v}");
    assert!(v["residualType"].is_string());
    assert!(v["registryKey"].is_null());
    assert!(v["riskLevel"].is_string());
    // 反面：不能出现 snake_case 键
    assert!(v.get("size_bytes").is_none());
    assert!(v.get("residual_type").is_none());
}

// ===================== 真机冒烟辅助 =====================

/// 直接扫本机注册表 + 安装目录并打印摘要（`#[ignore]`，CI 默认不跑）。
///
/// 为什么需要：判定逻辑的单测全在构造数据上跑，证明的是「规则按我写的执行」；
/// 而「在真机上误报率有多高」只能在真机上量。本机 WebView2 渲染进程起不来、
/// GUI 取证拿不到，这条是后端侧能产出的真实数据证据。
///
/// 跑法：
/// ```text
/// cargo test -p engine --test residual_test -- --ignored --nocapture real_scan
/// ```
#[test]
#[ignore = "真机冒烟：读真实注册表 + 扫真实安装目录"]
fn real_scan_smoke() {
    let r = engine::scan::residual::scan_all();

    println!(
        "registry_entries={} dirs_scanned={} typeA={} typeB={} total={}B size_truncated={} elapsed={}ms",
        r.registry_entries,
        r.dirs_scanned,
        r.registry_items.len(),
        r.file_items.len(),
        r.total_bytes,
        r.size_truncated,
        r.elapsed_ms
    );
    println!("--- 类型 A（注册表残留，路径已不存在）---");
    for i in r.registry_items.iter().take(10) {
        println!("  {}  →  {}", i.name, i.path);
    }
    println!("--- 类型 B（文件残留，注册表无对应项）---");
    for i in r.file_items.iter().take(15) {
        println!("  {:>12}B  {}", i.size_bytes, i.path);
    }

    // 结构不变量
    assert!(
        r.registry_entries > 0,
        "本机至少应该读到一些卸载项，读到 0 说明注册表读取有问题"
    );
}
