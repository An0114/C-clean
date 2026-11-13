//! task-086：dev_cache / windows_update 纯 Rust 扫描与编排集成测试。
//!
//! 覆盖：
//! 1. dev_cache 四个缓存根的扫描、大小、风险等级与负向边界；
//! 2. windows_update fabricated tree 的字段与 L2 风险等级；
//! 3. 空 scripts_dir 下 `run_all` 仍能跑完 5 个内置 Rust 模块；
//! 4. 真机扫描冒烟。

use engine::db::conn::open;
use engine::ipc::progress::NoopEmitter;
use engine::scan::modules::{dev_cache, windows_update};
use engine::scan::ScanOrchestrator;

// ===================== 辅助 =====================

/// 造假的 `%LOCALAPPDATA%` + `%APPDATA%`，四个 dev 缓存根各放一个文件。
fn make_dev_bases() -> (tempfile::TempDir, tempfile::TempDir) {
    let lad = tempfile::tempdir().unwrap();
    let ad = tempfile::tempdir().unwrap();

    let npm_new = lad.path().join("npm-cache").join("_cacache");
    std::fs::create_dir_all(&npm_new).unwrap();
    std::fs::write(npm_new.join("index-v5"), vec![b'x'; 100]).unwrap();

    let npm_old = ad.path().join("npm-cache");
    std::fs::create_dir_all(&npm_old).unwrap();
    std::fs::write(npm_old.join("content-v2"), vec![b'x'; 200]).unwrap();

    let pip = lad.path().join(r"pip\cache").join("wheels");
    std::fs::create_dir_all(&pip).unwrap();
    std::fs::write(pip.join("foo.whl"), vec![b'x'; 300]).unwrap();

    let pnpm = lad.path().join("pnpm-store").join("v3");
    std::fs::create_dir_all(&pnpm).unwrap();
    std::fs::write(pnpm.join("hash"), vec![b'x'; 400]).unwrap();

    (lad, ad)
}

/// 造临时库 + 一条 scan_session。返回的 TempDir 必须活到用完 `conn` 之后。
fn new_db() -> (tempfile::TempDir, rusqlite::Connection, i64) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (1000, 'running', 'full')",
        [],
    )
    .unwrap();
    let sid = conn.last_insert_rowid();
    (dir, conn, sid)
}

// ===================== ① dev_cache：扫描结果与边界 =====================

#[test]
fn dev_cache_scans_four_supported_roots() {
    let (lad, ad) = make_dev_bases();
    let items = dev_cache::scan_at(lad.path(), ad.path());

    assert_eq!(items.len(), 4, "四个缓存根各应扫到 1 个文件: {items:#?}");
    let total: i64 = items.iter().map(|i| i.size_bytes).sum();
    assert_eq!(total, 100 + 200 + 300 + 400);
    assert!(items.iter().all(|i| i.risk_level == dev_cache::RISK_L1));
    assert!(items
        .iter()
        .all(|i| i.module_code == dev_cache::MODULE_CODE));
}

#[test]
fn dev_cache_empty_bases_return_empty() {
    let lad = tempfile::tempdir().unwrap();
    let ad = tempfile::tempdir().unwrap();
    let items = dev_cache::scan_at(lad.path(), ad.path());
    assert!(items.is_empty(), "空缓存根应产出 0 条: {items:#?}");
}

#[test]
fn dev_cache_never_scans_gradle_or_maven() {
    // Gradle / Maven 不属于默认 dev_cache 扫描范围。
    // 这条把「不扫」钉成契约：将来谁想加，必须先改这条测试——
    // 也就必须先回答「删了要重下几百 MB 依赖，值不值」。
    //
    // 注意**不能**在 `pnpm-store` 下面造 `node_modules`：那属于已扫描根内部，
    // 扫到它是正确行为，断言会假红。
    let (lad, ad) = make_dev_bases();

    for (base, rel) in [
        (lad.path(), r"gradle\caches"),
        (lad.path(), "node_modules"),
        (ad.path(), r".m2\repository"),
    ] {
        let p = base.join(rel);
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("payload.bin"), vec![b'x'; 9_999]).unwrap();
    }

    let items = dev_cache::scan_at(lad.path(), ad.path());
    for it in &items {
        let s = it.path.to_lowercase();
        for forbidden in ["gradle", "node_modules", ".m2", "maven"] {
            assert!(
                !s.contains(forbidden),
                "扫到了不该碰的 {forbidden}: {}",
                it.path
            );
        }
    }
    // 该扫的还得在：4 个正常文件，大小不含 9999
    assert_eq!(items.len(), 4, "实际: {items:#?}");
    assert!(items.iter().all(|i| i.size_bytes != 9_999));
}

// ===================== ② windows_update：字段与风险等级 =====================

#[test]
fn windows_update_rust_tags_l2_on_fabricated_tree() {
    // 真实目录读不了时，用造出来的目录验证字段打点。
    let root = tempfile::tempdir().unwrap();
    let dl = windows_update::update_root(root.path());
    std::fs::create_dir_all(dl.join("2026-10-31")).unwrap();
    std::fs::write(dl.join("kb.cab"), vec![b'x'; 1000]).unwrap();
    std::fs::write(dl.join("2026-10-31").join("ssu.cab"), vec![b'x'; 500]).unwrap();

    let items = windows_update::scan_at(root.path());
    assert_eq!(items.len(), 2, "实际: {items:#?}");
    for it in &items {
        assert_eq!(it.module_code, windows_update::MODULE_CODE);
        assert_eq!(it.category, windows_update::CATEGORY_UPDATE_DOWNLOAD);
        // **必须 L2**：写成 L1 会让「全选 L1」的清理计划把更新缓存一起删掉，
        // 而更新安装中删它会中断安装（v0.2.1 Bug-2 的原样复现）
        assert_eq!(it.risk_level, windows_update::RISK_L2);
    }
}

// ===================== ③ 编排层收官证据 =====================

#[test]
fn orchestrator_runs_five_rust_modules_with_empty_scripts_dir() {
    // scripts_dir 是空目录，5 个内置模块仍须全部成功，证明运行路径不依赖外部脚本。
    // 与「结果里出现了某模块的条目」相比，这条证明的是代码路径，不受本机缓存影响。
    let empty_scripts_dir = tempfile::tempdir().unwrap();

    let (_db_dir, conn, sid) = new_db();
    let s = ScanOrchestrator::new(sid, &conn, &NoopEmitter)
        .run_all(empty_scripts_dir.path())
        .expect("5 个 Rust 模块都应成功");

    assert_eq!(
        s.scripts_succeeded.len(),
        5,
        "空 scripts_dir 下仍应有 5 个模块成功，实际: {:?}",
        s.scripts_succeeded
    );
    assert!(
        s.scripts_failed.is_empty(),
        "不该有失败模块: {:?}",
        s.scripts_failed
    );

    // 内置与已启用附加扫描只能写入已知 module_code。
    let mut codes: Vec<String> = conn
        .prepare("SELECT DISTINCT module_code FROM cleanup_item WHERE session_id=?1")
        .unwrap()
        .query_map([sid], |r| r.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    codes.sort();
    for c in &codes {
        assert!(
            [
                "temp",
                "windows_update",
                "prefetch",
                "browser_cache",
                "dev_cache",
                // 软件感知是数据驱动的追加步骤，不占 5 个内置模块槽位。
                "software_cache"
            ]
            .contains(&c.as_str()),
            "出现了非法 module_code: {c}"
        );
    }
    // 汇总条数必须等于实际入库条数，各模块共用 insert_batch 的记账不能漂。
    let n: i64 = conn
        .query_row(
            "SELECT count(*) FROM cleanup_item WHERE session_id=?1",
            [sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(s.total_items, n, "汇总条数与实际入库条数必须一致");
}

// ===================== ④ 真机冒烟 =====================

/// 扫本机真实 dev 缓存 / 更新缓存并打印摘要（`#[ignore]`，CI 默认不跑）。
///
/// 跑法：
/// ```text
/// cargo test -p engine --test devcache_windowsupdate_test -- --ignored --nocapture real_scan
/// ```
#[test]
#[ignore = "真机冒烟：扫真实 dev 缓存 / SoftwareDistribution"]
fn real_scan_smoke() {
    let items = dev_cache::scan();
    let total: i64 = items.iter().map(|i| i.size_bytes).sum();
    println!(
        "dev_cache: 共 {} 项，合计 {:.1} MB",
        items.len(),
        total as f64 / 1024.0 / 1024.0
    );
    for cat in [
        dev_cache::CATEGORY_NPM,
        dev_cache::CATEGORY_PIP,
        dev_cache::CATEGORY_PNPM,
    ] {
        let n = items.iter().filter(|i| i.category == cat).count();
        let b: i64 = items
            .iter()
            .filter(|i| i.category == cat)
            .map(|i| i.size_bytes)
            .sum();
        println!("  {cat}: {n} 项 / {:.1} MB", b as f64 / 1024.0 / 1024.0);
    }

    let wu = windows_update::scan();
    let wu_bytes: i64 = wu.iter().map(|i| i.size_bytes).sum();
    println!(
        "windows_update: {} 项 / {:.1} MB（普通权限下通常为 0：目录 ACL 只给管理员）",
        wu.len(),
        wu_bytes as f64 / 1024.0 / 1024.0
    );

    assert!(items
        .iter()
        .all(|i| i.module_code == dev_cache::MODULE_CODE));
    assert!(items.iter().all(|i| i.risk_level == dev_cache::RISK_L1));
    assert!(wu.iter().all(|i| i.risk_level == windows_update::RISK_L2));
}
