//! task-072 集成测试：清理方案模板（save / list / delete / apply / export / import）。
//!
//! 全部跑在临时 DB 上，不依赖真实扫描。
//!
//! 重点钉住两件事：
//! 1. **方案跨扫描复用**——`cleanup_plan.item_refs` 存的是三元组而不是 item id，
//!    重新扫描（旧 item 全删、新 item 换 id）后应用方案仍应匹配到新条目；
//! 2. **导出/导入的文件格式**——`format` / `version` 校验，防止把普通 JSON 当方案导进来。

use std::path::Path;

use engine::db::conn::open;
use engine::ipc::commands::plan::{
    apply_plan, delete_plan, export_plan, import_plan, list_plans, save_plan, PlanItemRef,
    PLAN_FORMAT, PLAN_FORMAT_VERSION,
};
use engine::ipc::error_codes::DRYRUN_EMPTY;

/// 建库 + 一个扫描会话，返回 `(tempdir, conn, session_id)`。
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

/// 插一条 cleanup_item，返回 id。
fn add_item(
    conn: &rusqlite::Connection,
    session_id: i64,
    module_code: &str,
    category: &str,
    path: &str,
    size: i64,
) -> i64 {
    conn.execute(
        "INSERT INTO cleanup_item
         (session_id, module_code, category, path, size_bytes, risk_level, description)
         VALUES (?1, ?2, ?3, ?4, ?5, 'L1', '')",
        rusqlite::params![session_id, module_code, category, path, size],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// 模拟一次重新扫描：删掉旧 item，开新会话、插新 item（id 全变）。
fn rescan_with(conn: &rusqlite::Connection, items: &[(&str, &str, &str, i64)]) -> Vec<i64> {
    conn.execute("DELETE FROM cleanup_item", []).unwrap();
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (2000, 'running', 'full')",
        [],
    )
    .unwrap();
    let sid = conn.last_insert_rowid();
    items
        .iter()
        .map(|(m, c, p, s)| add_item(conn, sid, m, c, p, *s))
        .collect()
}

// ===================== save / list / delete =====================

#[test]
fn save_then_list_reports_count_and_size() {
    let (_d, conn, sid) = setup();
    let a = add_item(&conn, sid, "temp", "user_temp", r"C:\T\a.tmp", 100);
    let b = add_item(&conn, sid, "temp", "user_temp", r"C:\T\b.tmp", 250);

    let id = save_plan(&conn, "我的方案", &[a, b], 5000).unwrap();
    assert!(id > 0);

    let plans = list_plans(&conn).unwrap();
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].name, "我的方案");
    assert_eq!(plans[0].item_count, 2);
    assert_eq!(plans[0].matched_count, 2);
    assert_eq!(plans[0].size_bytes, 350);
    assert_eq!(plans[0].created_at, 5000);
}

#[test]
fn save_dedupes_same_path() {
    let (_d, conn, sid) = setup();
    let a = add_item(&conn, sid, "temp", "user_temp", r"C:\T\a.tmp", 100);
    // 同一路径第二次被扫到（不同 session）→ 两行，但方案里只该存一条
    let a2 = add_item(&conn, sid, "temp", "user_temp", r"C:\T\a.tmp", 100);

    let id = save_plan(&conn, "dedupe", &[a, a2], 1).unwrap();
    let plans = list_plans(&conn).unwrap();
    assert_eq!(plans[0].item_count, 1, "同路径必须去重");
    assert_eq!(id, plans[0].id);
}

#[test]
fn save_rejects_empty_selection() {
    let (_d, conn, _sid) = setup();
    let err = save_plan(&conn, "x", &[], 1).unwrap_err();
    assert!(err.contains("没有选中"), "实际: {err}");
}

#[test]
fn save_rejects_blank_name() {
    let (_d, conn, sid) = setup();
    let a = add_item(&conn, sid, "temp", "user_temp", r"C:\T\a.tmp", 1);
    let err = save_plan(&conn, "   ", &[a], 1).unwrap_err();
    assert!(err.contains("方案名"), "实际: {err}");
}

#[test]
fn save_skips_items_that_are_gone() {
    let (_d, conn, sid) = setup();
    let a = add_item(&conn, sid, "temp", "user_temp", r"C:\T\a.tmp", 10);
    // 99999 不存在（刚被清掉）
    save_plan(&conn, "部分有效", &[a, 99999], 1).unwrap();
    let plans = list_plans(&conn).unwrap();
    assert_eq!(plans[0].item_count, 1, "不存在的 id 应被跳过");
}

#[test]
fn save_errors_when_all_items_gone() {
    let (_d, conn, _sid) = setup();
    let err = save_plan(&conn, "全没了", &[123, 456], 1).unwrap_err();
    assert!(err.contains("已不存在"), "实际: {err}");
}

#[test]
fn delete_removes_plan_and_reports_missing() {
    let (_d, conn, sid) = setup();
    let a = add_item(&conn, sid, "temp", "user_temp", r"C:\T\a.tmp", 1);
    let id = save_plan(&conn, "待删", &[a], 1).unwrap();

    assert_eq!(delete_plan(&conn, id).unwrap(), id);
    assert!(list_plans(&conn).unwrap().is_empty());
    assert!(delete_plan(&conn, id).is_err(), "重复删除必须报错");
}

// ===================== apply（跨扫描复用） =====================

#[test]
fn apply_matches_current_items_by_triple() {
    let (_d, conn, sid) = setup();
    let a = add_item(&conn, sid, "temp", "user_temp", r"C:\T\a.tmp", 100);
    let id = save_plan(&conn, "p", &[a], 1).unwrap();

    let r = apply_plan(&conn, id).unwrap();
    assert_eq!(r.item_ids, vec![a]);
    assert!(r.unmatched.is_empty());
    assert_eq!(r.size_bytes, 100);
    assert_eq!(r.name, "p");
}

#[test]
fn apply_still_works_after_rescan_with_new_ids() {
    // **本任务最关键的一条**：方案存三元组，重新扫描后 item id 全变，仍要能应用。
    let (_d, conn, sid) = setup();
    let a = add_item(&conn, sid, "temp", "user_temp", r"C:\T\a.tmp", 100);
    let plan_id = save_plan(&conn, "跨扫描", &[a], 1).unwrap();

    let new_ids = rescan_with(
        &conn,
        &[
            ("temp", "user_temp", r"C:\T\a.tmp", 120),
            ("temp", "user_temp", r"C:\T\other.tmp", 50),
        ],
    );

    let r = apply_plan(&conn, plan_id).unwrap();
    assert_eq!(r.item_ids, vec![new_ids[0]], "必须匹配到新扫描里的同一路径");
    assert!(r.unmatched.is_empty());
    assert_eq!(r.size_bytes, 120, "大小应取本次扫描的真实值");
}

#[test]
fn apply_reports_unmatched_paths() {
    let (_d, conn, sid) = setup();
    let a = add_item(&conn, sid, "temp", "user_temp", r"C:\T\a.tmp", 100);
    let b = add_item(&conn, sid, "temp", "user_temp", r"C:\T\b.tmp", 100);
    let plan_id = save_plan(&conn, "两缺一", &[a, b], 1).unwrap();

    // 重扫后只剩 a
    rescan_with(&conn, &[("temp", "user_temp", r"C:\T\a.tmp", 100)]);

    let r = apply_plan(&conn, plan_id).unwrap();
    assert_eq!(r.item_ids.len(), 1);
    assert_eq!(r.unmatched, vec![r"C:\T\b.tmp".to_string()]);
}

#[test]
fn apply_picks_newest_row_when_path_duplicated() {
    // cleanup_item 跨扫描累积时，同路径可能有多行；方案里一条只能勾一个框，
    // 取 id 最大的（最近一次扫描写入的那行）。
    let (_d, conn, sid) = setup();
    let old = add_item(&conn, sid, "temp", "user_temp", r"C:\T\a.tmp", 100);
    let plan_id = save_plan(&conn, "dup", &[old], 1).unwrap();

    // 不删旧行，只追加一次新扫描（真实场景：旧 item 没被清掉）
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (3000, 'running', 'full')",
        [],
    )
    .unwrap();
    let sid2 = conn.last_insert_rowid();
    let newer = add_item(&conn, sid2, "temp", "user_temp", r"C:\T\a.tmp", 100);

    let r = apply_plan(&conn, plan_id).unwrap();
    assert_eq!(r.item_ids, vec![newer], "应取最新那行，且只取一条");
}

#[test]
fn apply_errors_on_missing_plan() {
    let (_d, conn, _sid) = setup();
    let err = apply_plan(&conn, 424242).unwrap_err();
    assert!(err.contains("方案不存在"), "实际: {err}");
}

// ===================== export / import =====================

#[test]
fn export_writes_valid_plan_file() {
    let (dir, conn, sid) = setup();
    let a = add_item(&conn, sid, "temp", "user_temp", r"C:\T\a.tmp", 100);
    let plan_id = save_plan(&conn, "导出方案", &[a], 1).unwrap();

    let export_dir = dir.path().join("plans").join("exports");
    let p = export_plan(&conn, plan_id, &export_dir, 7777).unwrap();
    assert!(p.is_file(), "导出文件必须真的落盘: {}", p.display());
    assert!(p.to_string_lossy().ends_with(".json"));

    let raw = std::fs::read_to_string(&p).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(v["format"], PLAN_FORMAT);
    assert_eq!(v["version"], PLAN_FORMAT_VERSION);
    assert_eq!(v["name"], "导出方案");
    assert_eq!(v["exportedAt"], 7777);
    assert_eq!(v["itemRefs"].as_array().unwrap().len(), 1);
    // camelCase 契约：不能出现 snake_case 键
    assert!(v.get("item_refs").is_none());
    assert_eq!(v["itemRefs"][0]["moduleCode"], "temp");
    assert_eq!(v["itemRefs"][0]["category"], "user_temp");
}

#[test]
fn export_sanitizes_plan_name_in_filename() {
    let (dir, conn, sid) = setup();
    let a = add_item(&conn, sid, "temp", "user_temp", r"C:\T\a.tmp", 1);
    // 名字里带路径分隔符：不能让它跑出导出目录
    let plan_id = save_plan(&conn, r"a/b\c:d", &[a], 1).unwrap();
    let export_dir = dir.path().join("exports");
    let p = export_plan(&conn, plan_id, &export_dir, 42).unwrap();
    assert_eq!(
        p.parent().unwrap(),
        export_dir.as_path(),
        "文件名清洗后必须仍落在导出目录里"
    );
}

#[test]
fn import_creates_plan_from_file() {
    let (dir, conn, sid) = setup();
    let a = add_item(&conn, sid, "temp", "user_temp", r"C:\T\a.tmp", 100);
    let plan_id = save_plan(&conn, "原始", &[a], 1).unwrap();
    let export_dir = dir.path().join("exports");
    let p = export_plan(&conn, plan_id, &export_dir, 1).unwrap();

    let new_id = import_plan(&conn, &p, 9000).unwrap();
    assert_ne!(new_id, plan_id, "导入应生成新方案");

    let plans = list_plans(&conn).unwrap();
    assert_eq!(plans.len(), 2);
    let imported = plans.iter().find(|x| x.id == new_id).unwrap();
    assert_eq!(imported.name, "原始");
    assert_eq!(imported.item_count, 1);
    assert_eq!(imported.matched_count, 1, "导入后应能匹配到当前条目");
}

#[test]
fn import_rejects_foreign_json() {
    let (dir, conn, _sid) = setup();
    let f = dir.path().join("other.json");
    std::fs::write(&f, r#"{"hello":"world"}"#).unwrap();
    let err = import_plan(&conn, &f, 1).unwrap_err();
    assert!(err.contains("不是合法的方案文件"), "实际: {err}");
}

#[test]
fn import_rejects_wrong_format_marker() {
    let (dir, conn, _sid) = setup();
    let f = dir.path().join("bad.json");
    let body = serde_json::json!({
        "format": "something-else",
        "version": 1,
        "name": "x",
        "note": null,
        "exportedAt": 0,
        "itemRefs": [{"moduleCode":"temp","category":"user_temp","path":"C:\\a"}],
    });
    std::fs::write(&f, body.to_string()).unwrap();
    let err = import_plan(&conn, &f, 1).unwrap_err();
    assert!(err.contains("不是方案文件"), "实际: {err}");
}

#[test]
fn import_rejects_empty_item_refs() {
    let (dir, conn, _sid) = setup();
    let f = dir.path().join("empty.json");
    let body = serde_json::json!({
        "format": PLAN_FORMAT,
        "version": PLAN_FORMAT_VERSION,
        "name": "空方案",
        "note": null,
        "exportedAt": 0,
        "itemRefs": [],
    });
    std::fs::write(&f, body.to_string()).unwrap();
    let err = import_plan(&conn, &f, 1).unwrap_err();
    assert!(err.contains("没有任何条目"), "实际: {err}");
}

#[test]
fn import_roundtrip_preserves_refs() {
    let (dir, conn, sid) = setup();
    let a = add_item(&conn, sid, "temp", "user_temp", r"C:\T\a.tmp", 1);
    let b = add_item(
        &conn,
        sid,
        "prefetch",
        "prefetch",
        r"C:\Windows\Prefetch\x.pf",
        2,
    );
    let plan_id = save_plan(&conn, "往返", &[a, b], 1).unwrap();
    let export_dir = dir.path().join("exports");
    let p = export_plan(&conn, plan_id, &export_dir, 1).unwrap();

    let new_id = import_plan(&conn, &p, 2).unwrap();
    let applied = apply_plan(&conn, new_id).unwrap();
    assert_eq!(applied.item_ids.len(), 2, "导入的方案必须仍能匹配两条");
}

// ===================== handler（信封层） =====================

#[test]
fn handler_save_empty_returns_2001() {
    let (_d, conn, _sid) = setup();
    let env = engine::ipc::commands::plan::handle_plan_save(&conn, "x", &[], 1).unwrap();
    assert_eq!(env.code, DRYRUN_EMPTY.code);
    assert!(env.data.is_none());
}

#[test]
fn handler_list_returns_ok_envelope() {
    let (_d, conn, sid) = setup();
    let a = add_item(&conn, sid, "temp", "user_temp", r"C:\T\a.tmp", 5);
    save_plan(&conn, "env", &[a], 1).unwrap();

    let env = engine::ipc::commands::plan::handle_plan_list(&conn).unwrap();
    assert_eq!(env.code, 0);
    assert_eq!(env.data.unwrap().len(), 1);
}

#[test]
fn handler_export_empty_dir_returns_internal() {
    let (_d, conn, sid) = setup();
    let a = add_item(&conn, sid, "temp", "user_temp", r"C:\T\a.tmp", 1);
    let id = save_plan(&conn, "x", &[a], 1).unwrap();

    let env = engine::ipc::commands::plan::handle_plan_export(&conn, id, Path::new(""), 1).unwrap();
    assert_eq!(env.code, engine::ipc::error_codes::INTERNAL.code);
}

#[test]
fn plan_item_ref_serializes_camel_case() {
    let r = PlanItemRef {
        module_code: "temp".into(),
        category: "user_temp".into(),
        path: r"C:\a".into(),
    };
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["moduleCode"], "temp");
    assert!(v.get("module_code").is_none(), "不能出现 snake_case 键");
}
