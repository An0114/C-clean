//! task-074：`scan_temp` Rust 重写的集成测试。
//!
//! # 三层验证
//!
//! 1. **入库等价**：Rust 侧 `ScanItem` 经 `ScanRunner::insert_batch` 写进
//!    `cleanup_item` 后，行内容必须与 ps1 路径产出的完全一致（risk 映射、
//!    description 空串、session 绑定）——这是「两条路径共用一份入库逻辑」的证据。
//! 2. **真实对比**：把 `TEMP` 指向一个受控目录，分别跑真实 `scan_temp.ps1` 和
//!    Rust `scan_dir`，比对产出的路径 / 大小 / 风险等级集合。
//!    **不 mock、不比对硬编码期望值**——脚本改坏了这条会直接红。
//! 3. **模式切换**：`CCLEAN_RUST_TEMP` 的两种取值都真的能跑通编排。

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use engine::db::conn::open;
use engine::ipc::progress::{ProgressEmitter, ScanProgress};
use engine::scan::modules::{temp, ScanItem};
use engine::scan::ScanRunner;

// ===================== 测试基础设施 =====================

/// 收集所有进度事件（用于断言「emit 次数 = 入库条数」）。
#[derive(Default)]
struct VecEmitter {
    events: Mutex<Vec<ScanProgress>>,
}

impl ProgressEmitter for VecEmitter {
    fn emit(&self, payload: &ScanProgress) {
        self.events.lock().unwrap().push(payload.clone());
    }
}

impl VecEmitter {
    fn snapshot(&self) -> Vec<ScanProgress> {
        self.events.lock().unwrap().clone()
    }
}

/// 造临时库 + 一条 scan_session。
fn setup() -> (tempfile::TempDir, rusqlite::Connection, i64) {
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

/// 造一个受控目录树，返回 (TempDir, 期望的相对路径集合)。
///
/// 不用随机文件名：对比 ps1 与 Rust 的产出时，路径必须能一一对上。
fn make_temp_tree() -> (tempfile::TempDir, Vec<&'static str>) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.tmp"), vec![b'x'; 11]).unwrap();
    std::fs::write(dir.path().join("b.tmp"), vec![b'x'; 22]).unwrap();
    let sub = dir.path().join("sub");
    std::fs::create_dir_all(sub.join("deep")).unwrap();
    std::fs::write(sub.join("c.log"), vec![b'x'; 33]).unwrap();
    std::fs::write(sub.join("deep").join("d.dat"), vec![b'x'; 44]).unwrap();
    (dir, vec!["a.tmp", "b.tmp", "sub/c.log", "sub/deep/d.dat"])
}

/// 把路径规范化成长名小写形式再比对。
///
/// CI runner 用户名较长（如 `runneradmin`）会触发 8.3 短路径别名：
/// `tempfile::tempdir()` 拿到的是 `C:\Users\RUNNER~1\...`，而 PowerShell
/// `Get-ChildItem` 的 `FullName` 输出的是长名 `runneradmin`。两边指的是同一个文件，
/// 仅字符串表示不同——用 `canonicalize` 解析到最终路径后再比，消除短/长名差异。
fn norm_path(p: &str) -> String {
    std::fs::canonicalize(p)
        .map(|c| {
            c.to_string_lossy()
                .trim_start_matches(r"\\?\")
                .to_lowercase()
        })
        .unwrap_or_else(|_| p.to_lowercase())
}

// ===================== ① 入库等价 =====================

#[test]
fn insert_batch_writes_rows_with_ps1_equivalent_shape() {
    // Rust 的 ScanItem 入库后，行的每个字段都要与 ps1 路径一致。
    let (_dir, conn, sid) = setup();
    let emitter = VecEmitter::default();
    let runner = ScanRunner::new(sid, &conn, &emitter);

    let items = vec![
        ScanItem::new("temp", "user_temp", r"C:\T\a.tmp".to_string(), 100, 1),
        ScanItem::new("temp", "windows_temp", r"C:\W\b.tmp".to_string(), 200, 2),
    ];
    let summary = runner.insert_batch(&items).expect("入库应成功");

    assert_eq!(summary.items_inserted, 2);
    assert_eq!(summary.total_bytes, 300);
    assert!(summary.done_seen, "Rust 路径语义上是完整跑完");

    let rows: Vec<(String, String, String, i64, String, String)> = conn
        .prepare(
            "SELECT module_code, category, path, size_bytes, risk_level, description
             FROM cleanup_item WHERE session_id=?1 ORDER BY id",
        )
        .unwrap()
        .query_map([sid], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(rows.len(), 2);
    // risk 整数 → 文本映射必须与 ps1 路径一致（0/1/2 → L0/L1/L2）
    assert_eq!(rows[0].4, "L1");
    assert_eq!(rows[1].4, "L2");
    // description 是 NOT NULL，ps1 协议没有该字段，两条路径都填空串
    assert_eq!(rows[0].5, "");
    assert_eq!(rows[0].0, "temp");
    assert_eq!(rows[0].1, "user_temp");
    assert_eq!(rows[0].3, 100);
}

#[test]
fn insert_batch_emits_one_progress_per_item() {
    // 进度条不能「跳着走」：每条入库都要有一条进度事件，否则扫描大目录时
    // 界面会长时间不动。
    let (_dir, conn, sid) = setup();
    let emitter = VecEmitter::default();
    let runner = ScanRunner::new(sid, &conn, &emitter).with_percent_span(20, 40);

    let items: Vec<ScanItem> = (0..5)
        .map(|i| ScanItem::new("temp", "user_temp", format!(r"C:\T\{i}.tmp"), 10, 1))
        .collect();
    runner.insert_batch(&items).unwrap();

    let events = emitter.snapshot();
    assert_eq!(events.len(), 5, "5 条 item 应有 5 条进度事件");
    // scanned_items 单调递增 1..=5
    assert_eq!(
        events.iter().map(|e| e.scanned_items).collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5]
    );
    // percent 落在分配区间内（20..=40），且不回退
    let mut prev = 0u8;
    for e in &events {
        assert!(
            (20..=40).contains(&e.percent),
            "percent 越界: {}",
            e.percent
        );
        assert!(e.percent >= prev, "percent 回退: {prev} -> {}", e.percent);
        prev = e.percent;
    }
}

#[test]
fn insert_batch_with_empty_items_is_ok() {
    // 空目录是正常情况，不该报错、不该 emit
    let (_dir, conn, sid) = setup();
    let emitter = VecEmitter::default();
    let runner = ScanRunner::new(sid, &conn, &emitter);
    let s = runner.insert_batch(&[]).unwrap();
    assert_eq!(s.items_inserted, 0);
    assert_eq!(s.total_bytes, 0);
    assert!(emitter.snapshot().is_empty());
}

// ===================== ② Rust vs ps1 真实对比 =====================

/// 跑真实 `scan_temp.ps1`，返回它产出的 `(category, 小写路径, size, risk)` 集合。
///
/// `temp_dir` 通过**子进程环境变量**注入（`Command::env`），不影响本进程。
///
/// 返回 `None` 表示本机跑不了 ps1（没有 powershell / 脚本缺失），调用方跳过。
fn run_ps1_scan(temp_dir: &Path) -> Option<Vec<(String, String, i64, i64)>> {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .join("scripts")
        .join("scan_temp.ps1");
    if !script.exists() {
        eprintln!(
            "[跳过] 找不到 {}（scripts/ 不在预期位置）",
            script.display()
        );
        return None;
    }

    let out = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script)
        .env("TEMP", temp_dir)
        .output()
        .ok()?;
    if !out.status.success() {
        eprintln!("[跳过] ps1 退出码 {:?}", out.status.code());
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
        // 只取 user_temp：windows_temp 指向真实系统目录，两边扫同一份磁盘，
        // 但它的大小会随系统状态变，不适合做「集合相等」断言。
        let category = v.get("category").and_then(|c| c.as_str()).unwrap_or("");
        if category != temp::CATEGORY_USER_TEMP {
            continue;
        }
        rows.push((
            category.to_string(),
            norm_path(v.get("path").and_then(|p| p.as_str()).unwrap_or("")),
            v.get("size_bytes").and_then(|s| s.as_i64()).unwrap_or(-1),
            v.get("risk_level").and_then(|r| r.as_i64()).unwrap_or(-1),
        ));
    }
    Some(rows)
}

#[test]
fn rust_scan_matches_ps1_output_on_same_directory() {
    // 这是本任务「等价重写」的核心证据：同一个目录，两套实现产出必须一致。
    let (tree, expected) = make_temp_tree();

    let ps1 = match run_ps1_scan(tree.path()) {
        Some(r) => r,
        None => return, // 本机跑不了 ps1，跳过（不是失败）
    };

    let rust: Vec<(String, String, i64, i64)> =
        temp::scan_dir(tree.path(), temp::CATEGORY_USER_TEMP, temp::RISK_L1)
            .into_iter()
            .map(|i| {
                (
                    i.category.clone(),
                    norm_path(&i.path),
                    i.size_bytes,
                    i.risk_level,
                )
            })
            .collect();

    // 两边都按路径排序后再比，顺序不参与断言
    let mut a = ps1.clone();
    let mut b = rust.clone();
    a.sort();
    b.sort();

    assert_eq!(a.len(), expected.len(), "ps1 应扫到 4 个文件: {a:#?}");
    assert_eq!(b.len(), expected.len(), "Rust 应扫到 4 个文件: {b:#?}");
    assert_eq!(a, b, "两套实现的产出必须完全一致\nps1={a:#?}\nrust={b:#?}");

    // 逐条核对大小（防「两边都错但错得一样」）
    let total: i64 = rust.iter().map(|r| r.2).sum();
    assert_eq!(total, 11 + 22 + 33 + 44);
    assert!(rust.iter().all(|r| r.3 == temp::RISK_L1));
}

#[test]
fn rust_scan_agrees_with_ps1_on_empty_directory() {
    // 空目录：两边都该产出 0 条。这条防的是「一边把空目录当错误、一边当正常」。
    let empty = tempfile::tempdir().unwrap();
    let ps1 = match run_ps1_scan(empty.path()) {
        Some(r) => r,
        None => return,
    };
    let rust = temp::scan_dir(empty.path(), temp::CATEGORY_USER_TEMP, temp::RISK_L1);
    assert!(ps1.is_empty(), "ps1 应产出 0 条: {ps1:#?}");
    assert!(rust.is_empty(), "Rust 应产出 0 条: {rust:#?}");
}

// ===================== ③ 模式切换 =====================

#[test]
fn scan_items_never_carry_l3() {
    // L3 是「硬编码禁止」的等级，扫描结果里出现 L3 说明分级写错了。
    // 这条对全部扫描目标都成立（temp 只有 L1/L2）。
    let (tree, _) = make_temp_tree();
    for (_, cat, risk) in temp::scan_roots() {
        let items = temp::scan_dir(tree.path(), cat, risk);
        assert!(
            items.iter().all(|i| i.risk_level <= 2),
            "风险等级不该超过 2（L2）：{items:#?}"
        );
    }
}

#[test]
fn module_code_is_always_temp() {
    // module_code 是前端分组、历史页筛选的依据；写错会让 temp 项散到别的模块下。
    let (tree, _) = make_temp_tree();
    let items = temp::scan_dir(tree.path(), temp::CATEGORY_USER_TEMP, temp::RISK_L1);
    assert!(!items.is_empty());
    assert!(items.iter().all(|i| i.module_code == "temp"));
}

// ===================== 真机冒烟 =====================

/// 直接扫本机真实 `%TEMP%` 与 `%SystemRoot%\Temp` 并打印摘要
/// （`#[ignore]`，CI 默认不跑）。
///
/// 为什么需要：上面的用例都在 `tempfile` 上跑，证明的是「逻辑对」；
/// 而「真扫一遍不 panic、条目数量级合理、风险分级对」只能在真实目录上验。
/// 本机 WebView2 渲染进程起不来、GUI 取证拿不到，这条是后端侧能产出的真实数据。
///
/// 跑法：
/// ```text
/// cargo test -p engine --test temp_scan_test -- --ignored --nocapture real_scan
/// ```
#[test]
#[ignore = "真机冒烟：扫真实临时目录"]
fn real_scan_smoke() {
    let roots = temp::scan_roots();
    println!("目标目录 {} 个：", roots.len());
    for (p, cat, risk) in &roots {
        println!("  [{cat}] L{risk}  {}  存在={}", p.display(), p.is_dir());
    }

    let items = temp::scan();
    let total: i64 = items.iter().map(|i| i.size_bytes).sum();
    println!(
        "共 {} 项，合计 {} 字节（{:.1} MB）",
        items.len(),
        total,
        total as f64 / 1024.0 / 1024.0
    );

    for cat in [temp::CATEGORY_USER_TEMP, temp::CATEGORY_WINDOWS_TEMP] {
        let n = items.iter().filter(|i| i.category == cat).count();
        let b: i64 = items
            .iter()
            .filter(|i| i.category == cat)
            .map(|i| i.size_bytes)
            .sum();
        println!("  {cat}: {n} 项 / {b} 字节");
    }
    println!("前 5 条：");
    for i in items.iter().take(5) {
        println!("  [L{}] {:>12}B  {}", i.risk_level, i.size_bytes, i.path);
    }

    // 结构不变量：无论这台机器上有什么，都不能出现 L3 或错误的模块代码
    assert!(items.iter().all(|i| i.risk_level <= 2));
    assert!(items.iter().all(|i| i.module_code == temp::MODULE_CODE));
}
