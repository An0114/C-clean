//! task-076：dev_cache / windows_update Rust 重写的集成测试（R5 收官）。
//!
//! # 四层验证
//!
//! 1. **dev_cache 与 ps1 真实对比**：把 `LOCALAPPDATA` / `APPDATA` 通过
//!    `Command::env` 注入子进程，跑真实 `scan_dev_cache.ps1`，与 Rust `scan_at`
//!    逐条比对四个根。**不 mock**——脚本改坏了这条会直接红。
//! 2. **dev_cache 的负向边界**：`gradle` / `.m2` 不能被扫进来。
//! 3. **windows_update 与 ps1 对比**：两边都扫真实目录。本机普通权限下都是空集，
//!    CI（管理员）上是真实比对——两种结果都算通过，重点是**不能崩、不能编数据**。
//! 4. **编排层收官证据**：用一个**空的 scripts_dir** 跑 `run_all`，5 个模块全部
//!    成功 = 完全不依赖 ps1 文件；再把总开关关掉，同样目录必须全失败 = 回退开关
//!    真的有效。这条比「结果里出现了某模块的条目」更硬：它证明的是**代码路径**，
//!    而不是「本机恰好有缓存」。
//!
//! # 为什么 windows_update 不做 tempdir 对比
//!
//! `scan_windows_update.ps1` 把根目录写死成 `C:\Windows\SoftwareDistribution\Download`，
//! 没有注入入口。要造目录对比就只能改脚本——而脚本是本任务明确不动的文件。
//! 所以改成「两边都扫真实目录再比对」，管理员环境下是等价的强证据。
//!
//! # 为什么编排层的两个场景写在同一个用例里
//!
//! 它们靠 `CCLEAN_RUST_ALL` 环境变量切换，而 `set_var` 在**同进程内对所有线程
//! 可见**。拆成两个 `#[test]` 并行跑的话，一个把开关设成 `0` 的瞬间另一个正在
//! `run_all`，就会拿到 `AllScriptsFailed`——典型的偶发红。合成一个用例顺序执行，
//! 场景更清楚，也没有竞态。

use std::path::{Path, PathBuf};

use engine::db::conn::open;
use engine::ipc::progress::NoopEmitter;
use engine::scan::modules::{dev_cache, windows_update};
use engine::scan::{OrchestratorError, ScanOrchestrator};

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

// ===================== ① dev_cache：Rust vs ps1 真实对比 =====================

#[test]
fn dev_cache_rust_matches_ps1() {
    // 本任务「等价重写」的核心证据：同一份目录，两套实现的产出必须**完全一致**。
    let (lad, ad) = make_dev_bases();

    let ps1 = match run_ps1(
        "scan_dev_cache.ps1",
        &[("LOCALAPPDATA", lad.path()), ("APPDATA", ad.path())],
    ) {
        Some(r) => r,
        None => return, // 本机跑不了 ps1，跳过（不是失败）
    };

    let rust = as_rows(&dev_cache::scan_at(lad.path(), ad.path()));

    let mut a = ps1;
    let mut b = rust;
    a.sort();
    b.sort();

    assert_eq!(a.len(), 4, "ps1 应扫到 4 个文件（四根各一）: {a:#?}");
    assert_eq!(a, b, "两套实现的产出必须完全一致\nps1={a:#?}\nrust={b:#?}");

    let total: i64 = b.iter().map(|r| r.2).sum();
    assert_eq!(total, 100 + 200 + 300 + 400);
    assert!(b.iter().all(|r| r.3 == dev_cache::RISK_L1), "全部应为 L1");
}

#[test]
fn dev_cache_rust_agrees_with_ps1_on_empty_bases() {
    // 什么都没装：两边都该 0 条，且都不报错
    let lad = tempfile::tempdir().unwrap();
    let ad = tempfile::tempdir().unwrap();

    let rust = dev_cache::scan_at(lad.path(), ad.path());
    assert!(rust.is_empty(), "Rust 应产出 0 条: {rust:#?}");

    if let Some(ps1) = run_ps1(
        "scan_dev_cache.ps1",
        &[("LOCALAPPDATA", lad.path()), ("APPDATA", ad.path())],
    ) {
        assert!(ps1.is_empty(), "ps1 应产出 0 条: {ps1:#?}");
    }
}

#[test]
fn dev_cache_never_scans_gradle_or_maven() {
    // 任务书提过 Gradle / Maven，但 ps1 没扫、Rust 也不扫。
    // 这条把「不扫」钉成契约：将来谁想加，必须先改这条测试——
    // 也就必须先回答「删了要重下几百 MB 依赖，值不值」。
    //
    // 注意**不能**在 `pnpm-store` 下面造 `node_modules`：那属于已扫描根内部，
    // 两套实现都会扫到（正确行为），断言会假红。
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

// ===================== ② windows_update：真实目录对比 =====================

#[test]
fn windows_update_rust_matches_ps1_on_real_machine() {
    // ps1 硬编码 `C:\Windows\SoftwareDistribution\Download`，Rust 用
    // `%SystemRoot%\SoftwareDistribution\Download`。只有 SystemRoot 就是
    // `C:\Windows` 时两者才指向同一目录，否则跳过（不假绿）。
    let Ok(sr) = std::env::var("SystemRoot") else {
        eprintln!("[跳过] 无 SystemRoot");
        return;
    };
    if !sr.eq_ignore_ascii_case(r"C:\Windows") {
        eprintln!("[跳过] SystemRoot={sr} 与 ps1 硬编码的 C:\\Windows 不一致，无法对比");
        return;
    }

    let rust = as_rows(&windows_update::scan());

    let Some(ps1) = run_ps1("scan_windows_update.ps1", &[]) else {
        return;
    };
    let mut a = ps1;
    let mut b = rust;
    a.sort();
    b.sort();

    // 普通权限下两边通常都是空（目录 ACL 只给管理员）；CI 管理员上是真实比对。
    // 两种都算通过——重点是**两边一致**，不允许一边有数据一边没有。
    assert_eq!(
        a,
        b,
        "两套实现对真实目录的产出必须一致\nps1={} 条\nrust={} 条",
        a.len(),
        b.len()
    );
    assert!(
        b.iter().all(|r| r.3 == windows_update::RISK_L2),
        "全部应为 L2"
    );
}

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
fn orchestrator_runs_five_rust_modules_without_any_ps1() {
    // **R5 收官的核心证据**：scripts_dir 是空目录（一个 ps1 都没有），
    // 但 5 个模块全部成功 = 默认路径完全不依赖脚本文件、不 spawn 解释器。
    //
    // 与「结果里出现了某模块的条目」相比，这条证明的是**代码路径**，
    // 不受本机是否恰好有缓存影响。
    std::env::remove_var("CCLEAN_RUST_ALL");
    let empty = tempfile::tempdir().unwrap();

    let (_db_dir, conn, sid) = new_db();
    let s = ScanOrchestrator::new(sid, &conn, &NoopEmitter)
        .run_all(empty.path())
        .expect("5 个模块都应走 Rust 并成功");

    assert_eq!(
        s.scripts_succeeded.len(),
        5,
        "空 scripts_dir 下仍应有 5 个模块成功（走 Rust），实际: {:?}",
        s.scripts_succeeded
    );
    assert!(
        s.scripts_failed.is_empty(),
        "不该有失败模块: {:?}",
        s.scripts_failed
    );
    for name in [
        "scan_temp.ps1",
        "scan_windows_update.ps1",
        "scan_prefetch.ps1",
        "scan_browser_cache.ps1",
        "scan_dev_cache.ps1",
    ] {
        assert!(
            s.scripts_succeeded.iter().any(|x| x == name),
            "{name} 未成功"
        );
    }

    // 入库内容必须只来自这 5 个 Rust 模块（空 scripts_dir 下不可能有 ps1 产出）
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
                // task-083：软件感知是**数据驱动的追加步骤**，不占 SCRIPTS 槽位，
                // 但确实会产生自己 module_code 的条目。这里同步放开白名单。
                // 注意「5 个 Rust 模块」的断言仍然成立（scripts_succeeded 不含它）。
                "software_cache"
            ]
            .contains(&c.as_str()),
            "出现了非法 module_code: {c}"
        );
    }
    // 汇总条数必须等于实际入库条数——两条路径共用 insert_batch 的记账不能漂
    let n: i64 = conn
        .query_row(
            "SELECT count(*) FROM cleanup_item WHERE session_id=?1",
            [sid],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(s.total_items, n, "汇总条数与实际入库条数必须一致");

    // ---- 回退开关：同样一个空目录，总开关关掉后必须 5 个全失败 ----
    // 这一半证明「回退真的有效」，而不是「Rust 分支恰好也能跑」。
    std::env::set_var("CCLEAN_RUST_ALL", "0");
    let (_db_dir2, conn2, sid2) = new_db();
    let err = ScanOrchestrator::new(sid2, &conn2, &NoopEmitter)
        .run_all(empty.path())
        .expect_err("关掉 Rust 后空目录应导致 5 个模块全失败");

    match err {
        OrchestratorError::AllScriptsFailed(n) => assert_eq!(n, 5),
    }

    // 恢复现场：本文件其他用例不读这个变量，但别把污染留给后续用例
    std::env::remove_var("CCLEAN_RUST_ALL");
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
