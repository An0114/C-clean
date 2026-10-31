//! task-065：WinSxS 清理链路的**顺序**与**前端契约**用例。
//!
//! 这里不真的跑 DISM（单测里跑 DISM 会弹 UAC、还会真删组件存储，绝不能做）。
//! 但有两件事必须钉死，而且**不依赖真的跑起来**：
//!
//! 1. **还原点必须在 DISM 之前**——用「提权 exe 不存在」这个可控失败点，
//!    观察失败被归到哪一类，就能反推它先走了哪一步；
//! 2. **事件名 / JSON 字段名**——前端 `listen("winsxs://progress")` 和
//!    interface 字段是硬编码的，改了就静默失效（进度条永远不动）。

use engine::scan::winsxs::{
    cleanup, WinsxsCleanupError, WinsxsCleanupResult, WinsxsProgress, WINSXS_PROGRESS_EVENT,
};

/// 不存在的提权 exe：`run_elevated*` 会在「文件不存在」这一步就失败，
/// 不会真的拉起任何进程。
const MISSING_EXE: &str = "Z:/nope/cclean-elevation.exe";

fn setup() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = engine::db::conn::open(dir.path()).unwrap();
    (dir, conn)
}

#[test]
fn restore_point_runs_before_dism() {
    let (_d, conn) = setup();
    let err = cleanup(&conn, std::path::Path::new(MISSING_EXE), false, &|_, _| {})
        .expect_err("提权 exe 不存在时必须失败");

    // 失败被归到「还原点」这一类 = 它确实**先**去建还原点了。
    // 如果顺序写反（先 DISM 后建点），这里会是 Elevation。
    match err {
        WinsxsCleanupError::RestorePoint(m) => {
            assert!(m.contains("不存在"), "实际: {m}");
        }
        other => panic!("应先在还原点阶段失败，实际: {other:?}"),
    }
}

#[test]
fn skip_restore_point_goes_straight_to_dism() {
    let (_d, conn) = setup();
    let err = cleanup(&conn, std::path::Path::new(MISSING_EXE), true, &|_, _| {})
        .expect_err("提权 exe 不存在时必须失败");

    // 跳过还原点 → 直接进 DISM 阶段 → 失败归类是 Elevation
    match err {
        WinsxsCleanupError::Elevation(m) => {
            assert!(m.contains("不存在"), "实际: {m}");
        }
        other => panic!("跳过还原点时应直接进 DISM 阶段，实际: {other:?}"),
    }
}

#[test]
fn progress_event_name_is_stable() {
    // 前端 listen 的事件名是硬编码字符串，改名不会编译报错，只会让进度条永远不动。
    assert_eq!(WINSXS_PROGRESS_EVENT, "winsxs://progress");
}

#[test]
fn progress_payload_serializes_camel_case() {
    let p = WinsxsProgress {
        seq: 7,
        line: "Cleaning up component store".to_string(),
    };
    let json = serde_json::to_string(&p).unwrap();
    assert_eq!(json, r#"{"seq":7,"line":"Cleaning up component store"}"#);
}

#[test]
fn cleanup_result_serializes_camel_case() {
    // 前端 interface 用 camelCase（Tauri 只做参数名转换，**不做**返回结构转换，
    // 字段名靠 #[serde(rename_all = "camelCase")]）。
    let r = WinsxsCleanupResult {
        ok: true,
        exit_code: 0,
        restore_point_seq: Some(42),
        output_tail: "The operation completed successfully.".to_string(),
        line_count: 3,
    };
    let v: serde_json::Value = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert_eq!(v["ok"], serde_json::json!(true));
    assert_eq!(v["exitCode"], serde_json::json!(0));
    assert_eq!(v["restorePointSeq"], serde_json::json!(42));
    assert_eq!(v["lineCount"], serde_json::json!(3));
    assert!(v["outputTail"]
        .as_str()
        .unwrap()
        .contains("completed successfully"));
    // 反面：绝不能出现 snake_case 键（出现过一次就是一片 undefined）
    assert!(v.get("restore_point_seq").is_none());
}

#[test]
fn skipped_restore_point_serializes_null_seq() {
    let r = WinsxsCleanupResult {
        restore_point_seq: None,
        ..Default::default()
    };
    let v: serde_json::Value = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    assert!(
        v["restorePointSeq"].is_null(),
        "跳过的还原点必须是 null 而不是 0"
    );
}

#[test]
fn cleanup_error_messages_are_actionable() {
    // 这两条文案会直接显示在 UI 上，必须能区分「还原点问题」和「提权问题」——
    // 前者前端要弹「是否继续清理」，后者只能让用户重试/重装。
    let a = WinsxsCleanupError::RestorePoint("系统还原未开启".to_string());
    assert!(a.to_string().contains("还原点"));
    assert!(a.to_string().contains("系统还原未开启"));

    let b = WinsxsCleanupError::Elevation("用户取消了 UAC 提权请求".to_string());
    assert!(b.to_string().contains("提权"));
    assert!(b.to_string().contains("UAC"));
}
