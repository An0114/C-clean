//! task-068：注册表健康中心的判定逻辑、`.reg` 导出、任务 XML 解析与删除链路用例。
//!
//! 判定/生成类全用纯函数测；**删除链路**在 `HKCU\Software\C-Clean-Test-<pid>` 下
//! 造真实测试键跑（不改用户任何现有启动项），跑完删掉整个测试键。

use std::path::Path;

use engine::scan::registry_health::{
    classify_run_risk, delete_run_values, extract_exe_path, parse_task_xml, scan_scheduled_tasks,
    RunTarget, CATEGORY_TASK, RISK_L2, RISK_L3,
};
use engine::scan::winreg::{
    build_reg_export, sanitize_file_stem, utf16_bytes_to_string, value_type, RegRoot, RegValue,
};

// ===================== 命令行 → exe 路径 =====================

#[test]
fn extract_exe_handles_quoted_command() {
    // 最规范的形态：路径带引号，后面跟参数
    assert_eq!(
        extract_exe_path(r#""C:\Program Files\App\app.exe" --minimized"#).as_deref(),
        Some(r"C:\Program Files\App\app.exe")
    );
}

#[test]
fn extract_exe_handles_unquoted_with_spaces() {
    // 无引号但路径含空格：只能靠 `.exe` 截断（真实数据里很常见）
    assert_eq!(
        extract_exe_path(r"C:\Program Files\App\app.exe --minimized").as_deref(),
        Some(r"C:\Program Files\App\app.exe")
    );
    // 大小写不敏感
    assert_eq!(
        extract_exe_path(r"C:\X\APP.EXE /run").as_deref(),
        Some(r"C:\X\APP.EXE")
    );
}

#[test]
fn extract_exe_falls_back_to_first_token() {
    // 没有 .exe 的命令（如 rundll32 的参数形态异常时）按第一个空格切
    assert_eq!(
        extract_exe_path(r"C:\X\launcher.bat /silent").as_deref(),
        Some(r"C:\X\launcher.bat")
    );
    assert_eq!(extract_exe_path("   "), None);
    assert_eq!(extract_exe_path(""), None);
}

// ===================== 风险分级 =====================

#[test]
fn windows_dir_command_is_l3() {
    let (risk, rec) = classify_run_risk(
        "SomeSystemThing",
        Some(r"C:\Windows\System32\foo.exe"),
        true,
    );
    assert_eq!(risk, RISK_L3, "系统目录下的启动项必须 L3 禁删");
    assert!(rec.contains("不代删"), "实际: {rec}");
    // 大小写与斜杠都要归一化
    let (risk2, _) = classify_run_risk("X", Some("c:/windows/system32/bar.exe"), true);
    assert_eq!(risk2, RISK_L3);
}

#[test]
fn security_software_is_l3() {
    for name in ["SecurityHealth", "360Safe", "火绒安全", "KasperskyLab"] {
        let (risk, _) = classify_run_risk(name, Some(r"C:\Program Files\Sec\sec.exe"), true);
        assert_eq!(risk, RISK_L3, "{name} 应判 L3");
    }
    // 路径里带关键词也算（名称可能是随机串）
    let (risk, _) = classify_run_risk(
        "RandomName",
        Some(r"C:\Program Files\BitDefender\bd.exe"),
        true,
    );
    assert_eq!(risk, RISK_L3);
}

#[test]
fn missing_target_is_l2_and_says_so() {
    let (risk, rec) = classify_run_risk("GhostApp", Some(r"C:\Nope\ghost.exe"), false);
    assert_eq!(risk, RISK_L2);
    assert!(rec.contains("已不存在"), "要明确指出这是残留，实际: {rec}");
    assert!(rec.contains(".reg"), "要说明会备份，实际: {rec}");
}

#[test]
fn normal_third_party_is_l2() {
    let (risk, rec) = classify_run_risk("MyApp", Some(r"C:\Program Files\MyApp\app.exe"), true);
    assert_eq!(risk, RISK_L2);
    assert!(rec.contains("不再自动启动"), "实际: {rec}");
}

// ===================== 计划任务 XML =====================

#[test]
fn parse_task_xml_extracts_command_and_args() {
    let xml = r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Actions Context="Author">
    <Exec>
      <Command>C:\Program Files\App\app.exe</Command>
      <Arguments>--background --no-ui</Arguments>
    </Exec>
  </Actions>
</Task>"#;
    let (cmd, args) = parse_task_xml(xml).expect("应该解析出命令");
    assert_eq!(cmd, r"C:\Program Files\App\app.exe");
    assert_eq!(args, "--background --no-ui");
}

#[test]
fn parse_task_xml_unescapes_entities() {
    let xml =
        r#"<Exec><Command>C:\A&amp;B\x.exe</Command><Arguments>&quot;q&quot;</Arguments></Exec>"#;
    let (cmd, args) = parse_task_xml(xml).unwrap();
    assert_eq!(cmd, r"C:\A&B\x.exe");
    assert_eq!(args, r#""q""#);
}

#[test]
fn parse_task_xml_returns_none_without_command() {
    assert!(parse_task_xml("<Task></Task>").is_none());
    assert!(
        parse_task_xml("<Command>   </Command>").is_none(),
        "空白命令要当没有"
    );
}

#[test]
fn scan_scheduled_tasks_reads_real_xml_files() {
    let tmp = tempfile::tempdir().unwrap();
    let tasks = tmp.path();
    // 任务按目录分层：根下一个、子目录一个
    std::fs::create_dir_all(tasks.join("Microsoft")).unwrap();
    std::fs::write(
        tasks.join("MyApp.xml"),
        r#"<Task><Actions><Exec><Command>C:\Nope\ghost.exe</Command></Exec></Actions></Task>"#,
    )
    .unwrap();
    std::fs::write(
        tasks.join("Microsoft").join("SystemTask.xml"),
        r#"<Task><Actions><Exec><Command>C:\Windows\System32\svchost.exe</Command><Arguments>-k netsvcs</Arguments></Exec></Actions></Task>"#,
    )
    .unwrap();
    // 非 xml 文件应被忽略
    std::fs::write(tasks.join("notes.txt"), "not a task").unwrap();

    let items = scan_scheduled_tasks(tasks);

    assert_eq!(items.len(), 2, "实际: {items:?}");
    let app = items
        .iter()
        .find(|i| i.name == "MyApp")
        .expect("应有 MyApp");
    assert_eq!(app.category, CATEGORY_TASK);
    assert_eq!(app.risk_level, RISK_L2);
    assert!(!app.exists, "ghost.exe 不存在");
    assert!(!app.deletable, "计划任务一律不代删");

    // 子目录里的任务名要带路径前缀（与任务计划程序里显示一致）
    let sys = items
        .iter()
        .find(|i| i.name.contains("SystemTask"))
        .expect("应有 SystemTask");
    assert!(sys.name.starts_with("Microsoft"), "实际: {}", sys.name);
    assert_eq!(sys.risk_level, RISK_L3, "C:\\Windows 下的任务必须 L3");
    assert!(sys.path.contains("-k netsvcs"), "参数要拼进 path");
}

#[test]
fn scan_scheduled_tasks_on_missing_dir_is_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let items = scan_scheduled_tasks(&tmp.path().join("no-such-dir"));
    assert!(items.is_empty(), "目录不存在不是错误");
}

// ===================== .reg 导出 =====================

#[test]
fn reg_export_has_header_and_escaped_sz() {
    let values = vec![RegValue {
        name: "MyApp".to_string(),
        kind: value_type::SZ,
        data: utf16_le(r#""C:\A\B\app.exe" --x"#),
    }];
    let out = build_reg_export(RegRoot::CurrentUser, r"Software\Test\Run", &values);

    assert!(
        out.starts_with("Windows Registry Editor Version 5.00\r\n"),
        "实际: {out}"
    );
    assert!(
        out.contains(r"[HKEY_CURRENT_USER\Software\Test\Run]"),
        "实际: {out}"
    );
    // 反斜杠和引号都要转义，否则导入时报「格式无效」
    assert!(
        out.contains(r#""MyApp"="\"C:\\A\\B\\app.exe\" --x""#),
        "实际: {out}"
    );
}

#[test]
fn reg_export_handles_expand_sz_dword_and_default_value() {
    let values = vec![
        RegValue {
            name: String::new(), // 默认值 → @=
            kind: value_type::SZ,
            data: utf16_le("hello"),
        },
        RegValue {
            name: "Env".to_string(),
            kind: value_type::EXPAND_SZ,
            data: utf16_le("%SystemRoot%\\x.exe"),
        },
        RegValue {
            name: "Num".to_string(),
            kind: value_type::DWORD,
            data: 1u32.to_le_bytes().to_vec(),
        },
    ];
    let out = build_reg_export(RegRoot::LocalMachine, r"Software\T", &values);

    assert!(out.contains("@=\"hello\""), "默认值要写成 @=，实际: {out}");
    assert!(
        out.contains("hex(2):"),
        "EXPAND_SZ 要导成 hex(2)，实际: {out}"
    );
    assert!(out.contains("dword:00000001"), "实际: {out}");
}

#[test]
fn utf16_bytes_roundtrip() {
    let bytes = utf16_le("中文abc");
    assert_eq!(utf16_bytes_to_string(&bytes), "中文abc");
    // 没有结尾 NUL 也能解
    let raw: Vec<u8> = "abc".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    assert_eq!(utf16_bytes_to_string(&raw), "abc");
}

#[test]
fn sanitize_file_stem_replaces_illegal_chars() {
    assert_eq!(
        sanitize_file_stem(r"Software\Microsoft\Windows\CurrentVersion\Run"),
        "Software_Microsoft_Windows_CurrentVersion_Run"
    );
}

// ===================== 删除链路（真实注册表，用独立测试键） =====================

/// 造一个只属于本次测试的键路径，跑完删掉。
///
/// **每个用例必须用不同的 `suffix`**：Windows 删除键是延迟生效的，
/// 刚被 `RegDeleteKeyW` 删掉的键会处于「已标记删除」状态，此时再
/// `RegCreateKeyW` 同名键会返回 ERROR_KEY_DELETED(1018)。
#[cfg(windows)]
fn test_subkey(suffix: &str) -> String {
    format!(r"Software\C-Clean-Test-{}-{suffix}", std::process::id())
}

/// 在 `HKCU\<sub>` 下写一个 `REG_SZ` 值。
#[cfg(windows)]
fn write_test_value(sub: &str, name: &str, value: &str) {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyW, RegSetKeyValueW, HKEY, HKEY_CURRENT_USER, REG_SZ,
    };

    let sub_w: Vec<u16> = sub.encode_utf16().chain(std::iter::once(0)).collect();
    let name_w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let data: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();

    let mut hkey = HKEY::default();
    // SAFETY: 字符串均以 NUL 结尾且存活；hkey 是栈上变量，API 只写它。
    // 用 RegCreateKeyW 而不是 RegCreateKeyExW：后者带 SECURITY_ATTRIBUTES 参数，
    // 被 windows crate gate 在 Win32_Security feature 后面，为一个测试开那个 feature 不值得。
    let rc = unsafe { RegCreateKeyW(HKEY_CURRENT_USER, PCWSTR(sub_w.as_ptr()), &mut hkey) };
    assert!(rc.is_ok(), "建测试键失败: {rc:?}");

    // SAFETY: 同上；data 的字节长度按 u16 个数 * 2 告知。
    let rc = unsafe {
        RegSetKeyValueW(
            hkey,
            PCWSTR::null(),
            PCWSTR(name_w.as_ptr()),
            REG_SZ.0,
            Some(data.as_ptr() as *const _),
            (data.len() * 2) as u32,
        )
    };
    // SAFETY: 句柄由本函数独占。
    let _ = unsafe { RegCloseKey(hkey) };
    assert!(rc.is_ok(), "写测试值失败: {rc:?}");
}

/// 删掉整个测试键（清理，即使断言失败也尽量不留垃圾）。
#[cfg(windows)]
fn remove_test_key(sub: &str) {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{RegDeleteKeyW, HKEY_CURRENT_USER};

    let sub_w: Vec<u16> = sub.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: 字符串以 NUL 结尾且存活。
    let _ = unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, PCWSTR(sub_w.as_ptr())) };
}

#[test]
#[cfg(windows)]
fn delete_run_value_backs_up_then_deletes() {
    let sub = test_subkey("backup");
    remove_test_key(&sub); // 清掉上一轮可能残留的
    write_test_value(&sub, "TestEntry", r"C:\Nope\ghost.exe --x");

    let backup_dir = tempfile::tempdir().unwrap();
    let target = RunTarget {
        root: "HKCU".to_string(),
        subkey: sub.clone(),
        value_name: "TestEntry".to_string(),
    };

    let r = delete_run_values(backup_dir.path(), &[target], 1_700_000_000);

    // 1. 删成功
    assert_eq!(
        r.deleted,
        vec!["TestEntry".to_string()],
        "failed={:?}",
        r.failed
    );
    // 2. 产生了备份文件
    assert_eq!(r.backups.len(), 1, "必须备份后再删");
    let backup = Path::new(&r.backups[0]);
    assert!(backup.is_file(), "备份文件应存在: {}", backup.display());
    assert!(backup.to_string_lossy().ends_with("-1700000000.reg"));

    // 3. 备份内容能还原这个值（含值名与命令行）
    let content = std::fs::read_to_string(backup).unwrap();
    assert!(content.contains("Windows Registry Editor Version 5.00"));
    assert!(
        content.contains("TestEntry"),
        "备份里要有值名，实际: {content}"
    );
    assert!(
        content.contains("ghost.exe"),
        "备份里要有命令行，实际: {content}"
    );

    // 4. 值真的没了
    let remaining = engine::scan::winreg::values(RegRoot::CurrentUser, &sub);
    assert!(
        !remaining.iter().any(|v| v.name == "TestEntry"),
        "值应已删除，实际还剩: {remaining:?}"
    );

    remove_test_key(&sub);
}

#[test]
#[cfg(windows)]
fn delete_skips_missing_value_without_failing() {
    let sub = test_subkey("skip-missing");
    remove_test_key(&sub);
    write_test_value(&sub, "OnlyOne", r"C:\Nope\x.exe");

    let backup_dir = tempfile::tempdir().unwrap();
    let r = delete_run_values(
        backup_dir.path(),
        &[RunTarget {
            root: "HKCU".to_string(),
            subkey: sub.clone(),
            value_name: "NotThere".to_string(),
        }],
        1,
    );

    // 值不存在 → 幂等跳过：既不算删成功，也不该报错打扰用户
    assert!(r.deleted.is_empty());
    assert!(r.failed.is_empty(), "failed={:?}", r.failed);
    assert!(r.backups.is_empty(), "没删任何东西就不该产生备份");

    remove_test_key(&sub);
}

#[test]
#[cfg(windows)]
fn delete_refuses_l3_entry() {
    let sub = test_subkey("l3");
    remove_test_key(&sub);
    // 命令指向 C:\Windows 下 → L3，后端必须独立拒绝（不依赖前端传的等级）
    write_test_value(&sub, "EvilEntry", r"C:\Windows\System32\svchost.exe");

    let backup_dir = tempfile::tempdir().unwrap();
    let r = delete_run_values(
        backup_dir.path(),
        &[RunTarget {
            root: "HKCU".to_string(),
            subkey: sub.clone(),
            value_name: "EvilEntry".to_string(),
        }],
        1,
    );

    assert!(r.deleted.is_empty(), "L3 项不能被删");
    assert_eq!(r.failed.len(), 1);
    assert!(
        r.failed[0].reason.contains("L3"),
        "实际: {}",
        r.failed[0].reason
    );

    // 值必须还在
    let remaining = engine::scan::winreg::values(RegRoot::CurrentUser, &sub);
    assert!(remaining.iter().any(|v| v.name == "EvilEntry"));

    remove_test_key(&sub);
}

#[test]
fn delete_with_unknown_root_fails_softly() {
    let backup_dir = tempfile::tempdir().unwrap();
    let r = delete_run_values(
        backup_dir.path(),
        &[RunTarget {
            root: "HKXX".to_string(),
            subkey: "Software\\X".to_string(),
            value_name: "Y".to_string(),
        }],
        1,
    );
    assert!(r.deleted.is_empty());
    assert_eq!(r.failed.len(), 1);
    assert!(r.failed[0].reason.contains("未知注册表根"));
}

// ===================== 辅助 =====================

/// 字符串 → UTF-16LE 字节（含结尾 NUL），模拟注册表里的存储形态。
fn utf16_le(s: &str) -> Vec<u8> {
    s.encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(|u| u.to_le_bytes())
        .collect()
}

// ===================== 真机冒烟辅助 =====================

/// 直接扫本机 Run 键 / 卸载残留 / 计划任务并打印摘要（`#[ignore]`，CI 不跑）。
///
/// 本机 WebView2 渲染进程起不来、GUI 取证拿不到，这条是后端侧能产出的真实数据证据。
///
/// 跑法：
/// ```text
/// cargo test -p engine --test registry_health_test -- --ignored --nocapture real_scan
/// ```
#[test]
#[ignore = "真机冒烟：读真实 Run 键 / 卸载列表 / 计划任务"]
fn real_scan_smoke() {
    let tasks_dir = std::env::var("SystemRoot")
        .map(|r| std::path::PathBuf::from(r).join("System32").join("Tasks"))
        .unwrap_or_else(|_| std::path::PathBuf::from(r"C:/Windows/System32/Tasks"));

    let r = engine::scan::registry_health::scan_all(&tasks_dir);

    println!(
        "run={} uninstall={} task={} elapsed={}ms",
        r.run_items.len(),
        r.uninstall_items.len(),
        r.task_items.len(),
        r.elapsed_ms
    );

    println!("--- 启动项（Run）---");
    for i in &r.run_items {
        println!(
            "  [{}] {}  exists={} del={}  {}",
            i.risk_level, i.name, i.exists, i.deletable, i.path
        );
    }
    println!("--- 卸载残留 ---");
    for i in r.uninstall_items.iter().take(10) {
        println!("  [{}] {}  →  {}", i.risk_level, i.name, i.path);
    }
    println!("--- 计划任务（前 10）---");
    for i in r.task_items.iter().take(10) {
        println!(
            "  [{}] {}  exists={}  {}",
            i.risk_level, i.name, i.exists, i.path
        );
    }

    // 结构不变量：三分类必须都在（哪怕某一类为空）
    assert!(r.run_items.iter().all(|i| i.category == "run"));
    assert!(r.task_items.iter().all(|i| i.category == "task"));
    assert!(r.uninstall_items.iter().all(|i| i.category == "uninstall"));
    // L3 一律不可删
    assert!(r
        .run_items
        .iter()
        .all(|i| !(i.risk_level == "L3" && i.deletable)));
}

#[test]
fn scan_all_reports_task_dir_unreadable() {
    // 计划任务目录读不到时必须如实标记，不能显示成「没有任务」——
    // 普通用户对 C:/Windows/System32/Tasks 就是 Permission denied。
    let tmp = tempfile::tempdir().unwrap();
    let r = engine::scan::registry_health::scan_all(&tmp.path().join("no-such-dir"));
    assert!(!r.task_scan_available, "读不到目录必须标记 available=false");
    assert!(r.task_items.is_empty());
    // Run 键和卸载列表不受影响，照常返回
    assert!(!r.run_items.is_empty(), "本机至少有几个启动项");
}

#[test]
fn scan_all_reports_task_dir_available() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("T.xml"),
        r#"<Task><Actions><Exec><Command>C:/Nope/x.exe</Command></Exec></Actions></Task>"#,
    )
    .unwrap();
    let r = engine::scan::registry_health::scan_all(tmp.path());
    assert!(r.task_scan_available, "能读到就必须标 true");
    assert_eq!(r.task_items.len(), 1);
}
