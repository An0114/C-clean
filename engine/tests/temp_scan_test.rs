//! task-086：`temp` 纯 Rust 扫描与 `ScanRunner` 入库集成测试。
//!
//! 覆盖受控目录的递归扫描、路径 / 大小 / 风险等级、空目录、模块字段，
//! 以及 `ScanItem` 落库、影响说明和逐条进度事件。

use std::path::Path;
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
/// 固定文件名让递归扫描结果可以逐条核对。
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

// ===================== ① ScanItem 入库 =====================

#[test]
fn insert_batch_writes_expected_row_shape() {
    // ScanItem 入库后必须保留模块、分类、路径、大小与风险等级等字段。
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
    // risk 整数按 0/1/2 → L0/L1/L2 映射为数据库文本。
    assert_eq!(rows[0].4, "L1");
    assert_eq!(rows[1].4, "L2");
    // description 是 NOT NULL，扫描条目未提供描述时写入空串。
    assert_eq!(rows[0].5, "");
    assert_eq!(rows[0].0, "temp");
    assert_eq!(rows[0].1, "user_temp");
    assert_eq!(rows[0].3, 100);
}

/// task-086 3.3：影响说明从 ScanItem 透传到 cleanup_item 落库。
#[test]
fn insert_batch_persists_impact_notes() {
    let (_dir, conn, sid) = setup();
    let emitter = VecEmitter::default();
    let runner = ScanRunner::new(sid, &conn, &emitter);

    // 规则库语义：software_cache 扫到条目时用 with_impact 携带影响说明。
    let items = vec![ScanItem::new(
        "software_cache",
        "software_cache_vscode",
        r"C:\A\c.tmp".to_string(),
        50,
        1,
    )
    .with_impact("删除后下次启动重新下载", "重启软件自动重建")];
    runner.insert_batch(&items).expect("入库应成功");

    let (impact, rebuild): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT impact_note, rebuild_note FROM cleanup_item WHERE session_id=?1",
            [sid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(impact.as_deref(), Some("删除后下次启动重新下载"));
    assert_eq!(rebuild.as_deref(), Some("重启软件自动重建"));
}

/// task-086 3.3：未携带影响说明的条目落库为 NULL（前端退回通用文案）。
#[test]
fn insert_batch_leaves_impact_notes_null_when_absent() {
    let (_dir, conn, sid) = setup();
    let emitter = VecEmitter::default();
    let runner = ScanRunner::new(sid, &conn, &emitter);

    // 内置模块（temp）不携带影响说明，落库应为 NULL 而非空串——
    // 空串会覆盖前端「无说明时显示通用 L1/L2 文案」的兜底逻辑。
    let items = vec![ScanItem::new(
        "temp",
        "user_temp",
        r"C:\T\a.tmp".to_string(),
        100,
        1,
    )];
    runner.insert_batch(&items).expect("入库应成功");

    let (impact, rebuild): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT impact_note, rebuild_note FROM cleanup_item WHERE session_id=?1",
            [sid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(impact.is_none(), "无影响说明时 impact_note 应为 NULL");
    assert!(rebuild.is_none(), "无重建说明时 rebuild_note 应为 NULL");
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

// ===================== ② Rust 扫描结果 =====================

#[test]
fn rust_scan_finds_expected_files_with_sizes_and_risk() {
    let (tree, mut expected) = make_temp_tree();
    let items = temp::scan_dir(tree.path(), temp::CATEGORY_USER_TEMP, temp::RISK_L1);

    let mut paths: Vec<String> = items
        .iter()
        .map(|i| {
            Path::new(&i.path)
                .strip_prefix(tree.path())
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    paths.sort();
    expected.sort_unstable();
    assert_eq!(paths, expected, "应递归扫到受控目录中的全部文件");

    let total: i64 = items.iter().map(|i| i.size_bytes).sum();
    assert_eq!(total, 11 + 22 + 33 + 44);
    assert!(items.iter().all(|i| i.risk_level == temp::RISK_L1));
    assert!(items.iter().all(|i| i.category == temp::CATEGORY_USER_TEMP));
}

#[test]
fn rust_scan_empty_directory_returns_empty() {
    let empty = tempfile::tempdir().unwrap();
    let items = temp::scan_dir(empty.path(), temp::CATEGORY_USER_TEMP, temp::RISK_L1);
    assert!(items.is_empty(), "空目录应产出 0 条: {items:#?}");
}

// ===================== ③ Rust 扫描字段与风险边界 =====================

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
