//! task-004 命令纯逻辑与节流器测试。

use engine::ipc::commands::dry_run::{handle_dry_run, DryRunReq, MockResolver, ResolvedItem};
use engine::ipc::commands::scan_start::{handle_scan_start, ScanStartReq};
use engine::ipc::progress::{
    NoopEmitter, ProgressEmitter, ScanProgress, ThrottledEmitter, VecEmitter,
};
use std::thread::sleep;
use std::time::Duration;

fn progress(session: i64) -> ScanProgress {
    ScanProgress {
        session_id: session,
        percent: 1,
        current_path: String::new(),
        scanned_items: 1,
    }
}

#[test]
fn scan_start_full_returns_incrementing_session_id() {
    let mut counter = 0;
    let r1 = handle_scan_start(
        ScanStartReq {
            scope: "full".into(),
            target_drive: None,
            drives: None,
        },
        &mut counter,
    );
    assert_eq!(r1.code, 0);
    assert_eq!(r1.data.as_ref().unwrap().session_id, 1);
    let r2 = handle_scan_start(
        ScanStartReq {
            scope: "custom".into(),
            target_drive: Some("C:".into()),
            drives: None,
        },
        &mut counter,
    );
    assert_eq!(r2.data.as_ref().unwrap().session_id, 2);
}

#[test]
fn scan_start_rejects_bad_scope() {
    let mut counter = 0;
    let r = handle_scan_start(
        ScanStartReq {
            scope: "fullx".into(),
            target_drive: None,
            drives: None,
        },
        &mut counter,
    );
    assert_eq!(r.code, 5001);
}

#[test]
fn scan_start_rejects_bad_drive() {
    let mut counter = 0;
    // "C:" 合法
    let ok = handle_scan_start(
        ScanStartReq {
            scope: "full".into(),
            target_drive: Some("C:".into()),
            drives: None,
        },
        &mut counter,
    );
    assert_eq!(ok.code, 0);
    // task-077：**"C" 现在合法**（此前 task-004 的校验要求必须带冒号）。
    // 放宽的原因：`drives` 入参按 API 文档就是 `["C"]` 形态（无冒号），而前端下拉框
    // 给的是 `"C:"`——两种都会真的传进来，只认一种就是等着踩。归一化在
    // `scan_start::parse_drive` 里做，四种写法（C / c / C: / c:）都认。
    let bare = handle_scan_start(
        ScanStartReq {
            scope: "full".into(),
            target_drive: Some("C".into()),
            drives: None,
        },
        &mut counter,
    );
    assert_eq!(bare.code, 0, "task-077 起 \"C\" 是合法盘符写法");
    // "C:\"" 非法（多了反斜杠）
    let bad2 = handle_scan_start(
        ScanStartReq {
            scope: "full".into(),
            target_drive: Some("C:\\".into()),
            drives: None,
        },
        &mut counter,
    );
    assert_eq!(bad2.code, 5001);
    // 多盘入参里任何一项非法 → 整体失败（不跳过坏项）
    let bad3 = handle_scan_start(
        ScanStartReq {
            scope: "full".into(),
            target_drive: None,
            drives: Some(vec!["C".into(), "1".into()]),
        },
        &mut counter,
    );
    assert_eq!(bad3.code, 5001);
    // 空数组非法（「一个盘都不扫」与「不指定」必须区分开）
    let bad4 = handle_scan_start(
        ScanStartReq {
            scope: "full".into(),
            target_drive: None,
            drives: Some(vec![]),
        },
        &mut counter,
    );
    assert_eq!(bad4.code, 5001);
}

#[test]
fn dry_run_empty_item_ids_returns_2001() {
    let resolver = MockResolver::new(vec![]);
    let r = handle_dry_run(DryRunReq { item_ids: vec![] }, &resolver, &[]);
    assert_eq!(r.code, 2001);
    assert!(r.data.is_none());
}

#[test]
fn dry_run_all_resolved_pass_returns_ok() {
    let resolver = MockResolver::new(vec![
        ResolvedItem {
            item_id: 1,
            path: r"C:\Windows\Temp\a.log".into(),
            module_code: "A".into(),
            category: "temp".into(),
            size_bytes: 100,
        },
        ResolvedItem {
            item_id: 2,
            path: r"C:\Windows\Temp\b.log".into(),
            module_code: "A".into(),
            category: "temp".into(),
            size_bytes: 300,
        },
    ]);
    let r = handle_dry_run(
        DryRunReq {
            item_ids: vec![1, 2],
        },
        &resolver,
        &[],
    );
    assert_eq!(r.code, 0);
    let data = r.data.unwrap();
    assert_eq!(data.total_items, 2);
    assert_eq!(data.total_bytes, 400);
    assert_eq!(data.breakdown.len(), 1);
    assert_eq!(data.breakdown[0].bytes, 400);
}

#[test]
fn dry_run_l3_path_returns_2002() {
    let resolver = MockResolver::new(vec![ResolvedItem {
        item_id: 7,
        path: r"C:\Users\alice\Desktop\secret.docx".into(),
        module_code: "A".into(),
        category: "user".into(),
        size_bytes: 42,
    }]);
    let r = handle_dry_run(DryRunReq { item_ids: vec![7] }, &resolver, &[]);
    assert_eq!(r.code, 2002);
    let data = r.data.unwrap(); // 显式偏离：L3 拒绝 data 不置 null
    assert_eq!(data.blocked_by_l3.len(), 1);
    assert_eq!(
        data.blocked_by_l3[0].path,
        r"C:\Users\alice\Desktop\secret.docx"
    );
    assert_eq!(data.blocked_by_l3[0].denial_reason, "L3PersonalDir");
}

#[test]
fn throttled_emitter_drops_fast_duplicates() {
    let emitter = ThrottledEmitter::new(VecEmitter::default());
    emitter.emit(&progress(1));
    emitter.emit(&progress(1));
    emitter.emit(&progress(1));
    assert_eq!(emitter.inner().snapshot().len(), 1);
    sleep(Duration::from_millis(110));
    emitter.emit(&progress(1));
    assert_eq!(emitter.inner().snapshot().len(), 2);
    // Noop 至少不 panic。
    NoopEmitter.emit(&progress(1));
}
