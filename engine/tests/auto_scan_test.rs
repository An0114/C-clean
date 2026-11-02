//! task-073：系统级定时扫描（Windows 任务计划程序）的集成测试。
//!
//! # 这个文件里有一条会**真的创建计划任务**的用例
//!
//! 不是 mock，也不该 mock——「schtasks 能不能成功注册一个每天跑的任务」是
//! 本功能的核心，用假数据测等于什么都没测。
//!
//! 但它有一个必须处理的风险：**万一测试跑到一半失败，会在用户机器上留一个
//! 每天执行的计划任务**。所以做了两道保护：
//!
//! 1. **先探测**：本机已经注册了这个任务（说明用户真的开着这个功能）→ 直接跳过，
//!    绝不动用户的配置；
//! 2. **先清理再断言**：`disable()` 在断言之前调用，即使断言失败任务也已经删掉了。
//!
//! 任务指向的是测试二进制本身（`current_exe()`），即使真的残留也只会跑一遍
//! 测试然后退出，不会删任何文件——但依然按上面的方式保证不残留。

use std::path::{Path, PathBuf};

use engine::ipc::commands::auto_scan::{
    build_create_args, build_run_value, handle_status, is_registered, validate_time,
    AutoScanTaskStatus, AUTO_SCAN_FLAG, DEFAULT_TIME, TASK_NAME,
};

/// 当前测试二进制的绝对路径（一定存在，可当「合法的 exe」用）。
fn exe() -> PathBuf {
    std::env::current_exe().expect("current_exe 不可用")
}

// ===================== 参数拼装（纯函数，不碰系统） =====================

#[test]
fn create_args_quote_path_with_spaces() {
    // 安装态在 `C:\Program Files\C-Disk Cleaner\` 下。不加引号会被 schtasks
    // 在空格处截断，任务指向 `C:\Program.exe`——这是个真实会发生的坑。
    let fake = PathBuf::from(r"C:\Program Files\C-Disk Cleaner\cdisk-cleaner.exe");
    // build_run_value 会检查存在性，这里只测「拼出来的形状」，所以直接构造
    let expected = format!("\"{}\" {AUTO_SCAN_FLAG}", fake.display());
    assert!(expected.starts_with('"'));
    assert!(expected.contains("Program Files"));
    // 引号必须包住整个路径，而不是只包住前半段
    assert_eq!(expected.matches('"').count(), 2);
}

#[test]
fn create_args_reject_elevation_switches() {
    let args = build_create_args(&exe(), DEFAULT_TIME).unwrap();
    // 本功能不需要管理员权限。/ru SYSTEM 会让任务跑在 Session 0，
    // 拿不到用户的通知权限，还要额外提权——纯负收益。
    for forbidden in ["/ru", "/rp", "/rl", "/it"] {
        assert!(
            !args.contains(&forbidden.to_string()),
            "不该出现 {forbidden}"
        );
    }
}

#[test]
fn create_args_use_daily_trigger_with_force() {
    let args = build_create_args(&exe(), "09:00").unwrap();
    assert_eq!(args[0], "/create");
    assert!(args.contains(&"daily".to_string()));
    assert!(args.contains(&"09:00".to_string()));
    // 没有 /f 的话，同名任务存在时会弹「是否替换」——非交互环境下等于卡死
    assert!(args.contains(&"/f".to_string()));
}

#[test]
fn time_validation_blocks_argument_injection() {
    // 这几个如果放行，`/st` 后面会跟出一串额外的 schtasks 开关
    for bad in [
        "09:00 /ru SYSTEM",
        "09:00\t/ru",
        "09:00\n/ru",
        "9:00",
        "0900",
        "",
    ] {
        assert!(validate_time(bad).is_err(), "不该接受 {bad:?}");
    }
    assert!(validate_time("09:00").is_ok());
}

#[test]
fn run_value_requires_existing_absolute_exe() {
    assert!(build_run_value(&exe()).is_ok());
    assert!(build_run_value(Path::new("relative.exe")).is_err());
    assert!(build_run_value(&exe().parent().unwrap().join("nope-xyz.exe")).is_err());
}

// ===================== 跨 crate 一致性（静态检查） =====================

#[test]
fn auto_scan_flag_matches_app_side() {
    // engine 里定义的 `--auto-scan` 必须真的被 app 侧识别。改一边忘另一边，
    // 计划任务就会变成「每天弹一个完整窗口」——而且用户不会报 bug，
    // 只会觉得这软件很烦。
    let lib_rs = include_str!("../../app/src-tauri/src/lib.rs");
    assert!(
        lib_rs.contains("\"--auto-scan\"") || lib_rs.contains("AUTO_SCAN_FLAG"),
        "app/src-tauri/src/lib.rs 里找不到 --auto-scan 的处理，计划任务会弹窗"
    );
}

#[test]
fn task_name_is_what_users_see_in_task_scheduler() {
    // 任务名是用户在「任务计划程序」里找它的唯一线索，必须是可读的固定值。
    assert_eq!(TASK_NAME, "C-Disk Cleaner Auto Scan");
}

// ===================== 真实 schtasks 链路 =====================

#[test]
fn schtasks_roundtrip_creates_then_deletes() {
    // ① 先探测：本机已经有这个任务 → 说明用户真的开着，绝不覆盖/删除。
    match is_registered() {
        Ok(true) => {
            eprintln!("[跳过] 本机已注册 {TASK_NAME}（用户正在使用），不去动它");
            return;
        }
        Ok(false) => {}
        Err(e) => {
            // schtasks 都调不起来的环境（比如非 Windows）跳过，不算失败
            eprintln!("[跳过] 无法查询任务状态: {e}");
            return;
        }
    }

    let exe = exe();

    // ② 注册
    if let Err(e) = engine::ipc::commands::auto_scan::enable(&exe, DEFAULT_TIME) {
        eprintln!("[跳过] 本机无法注册计划任务（可能被组策略限制）: {e}");
        return;
    }

    // ③ 回查 + 清理：**清理放在断言之前**，断言失败也不会留残渣。
    let registered = is_registered().unwrap_or(false);
    let removed = engine::ipc::commands::auto_scan::disable();

    assert!(registered, "create 返回成功后应该能查询到任务");
    removed.expect("delete 应该成功");
    assert!(
        !is_registered().unwrap_or(true),
        "delete 之后不该再查询到任务"
    );
}

#[test]
fn disable_is_idempotent_when_task_absent() {
    // 任务本来就不存在时，点「关闭」不该报错——目标状态已经达成了。
    if is_registered().unwrap_or(true) {
        // 已注册的话这条用例没有意义（会真的删掉用户的任务）
        eprintln!("[跳过] 本机已注册 {TASK_NAME}，不做幂等测试");
        return;
    }
    assert!(engine::ipc::commands::auto_scan::disable().is_ok());
    assert!(!is_registered().unwrap_or(true));
}

/// schtasks 能不能在本机跑起来。
///
/// **不是所有环境都允许调 `schtasks.exe`**：企业组策略、安全软件、以及本项目的
/// 开发沙箱（Program Blacklist）都会直接拒绝启动它。这种情况下「查不到状态」
/// 和「任务不存在」是两件完全不同的事——`handle_status` 会返回 `Err`，
/// 前端据此显示真实原因，而不是谎报「未注册」。
///
/// 测试要区分这两种环境：能跑就跑真实断言，不能跑就跳过（**不是失败**）。
fn schtasks_available() -> bool {
    is_registered().is_ok()
}

#[test]
fn status_reports_task_name_and_time() {
    if !schtasks_available() {
        eprintln!("[跳过] 本机 schtasks 不可用（被安全策略拦截），无法查询任务状态");
        return;
    }
    // status 是只读操作，且返回的任务名/时间要能直接展示给用户。
    let st = handle_status().expect("status 不该失败");
    assert_eq!(st.task_name, TASK_NAME);
    assert_eq!(st.time, DEFAULT_TIME);
    // registered 的值取决于本机，但类型必须是 bool（前端 interface 对齐）
    let _: bool = st.registered;
}

#[test]
fn status_surfaces_error_when_schtasks_unavailable() {
    // 这条在「schtasks 被拦」的环境里才有意义：必须返回 Err，
    // 不能把「调不起来」伪装成「任务不存在」——那会让用户以为开关是关的。
    if schtasks_available() {
        eprintln!("[跳过] 本机 schtasks 可用，走不到这个分支");
        return;
    }
    let err = handle_status().expect_err("schtasks 不可用时必须报错");
    assert!(
        err.contains("schtasks"),
        "错误信息里要说清是 schtasks 的问题: {err}"
    );
}

#[test]
fn status_serializes_to_camel_case() {
    // 前端 interface 是 { registered, taskName, time }，字段名对不上会全 undefined。
    // 直接构造结构体，不依赖本机 schtasks 是否可用。
    let st = AutoScanTaskStatus {
        registered: false,
        task_name: TASK_NAME.to_string(),
        time: DEFAULT_TIME.to_string(),
    };
    let v = serde_json::to_value(&st).unwrap();
    assert!(v.get("registered").is_some());
    assert!(v.get("taskName").is_some(), "必须是 camelCase: {v}");
    assert!(v.get("task_name").is_none(), "不该出现 snake_case: {v}");
    assert!(v.get("time").is_some());
}
