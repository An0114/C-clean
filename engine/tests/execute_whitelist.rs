//! task-003 集成测试：对 PRD P0-7 全部 L3 路径做 100% 拦截，
//! 对典型 L1 缓存路径做放行验证。全部纯内存，不碰真实文件系统。

use engine::execute::delete::{execute, ExecuteOutcome, ExecuteRequest, NoopDeleter, REJECT_CODE};
use engine::execute::whitelist::DenialReason;

/// 跑 execute 并返回最后一条被拒决策的拒绝原因（无拒绝则 None）。
fn last_denial(paths: Vec<String>, actions: Vec<String>) -> (bool, Option<DenialReason>) {
    let req = ExecuteRequest { paths, actions };
    let deleter = NoopDeleter::default();
    match execute(&req, &deleter).expect("execute should not error") {
        ExecuteOutcome::Allowed { .. } => (true, None),
        ExecuteOutcome::Rejected { decisions, code } => {
            assert_eq!(code, REJECT_CODE, "拒绝信封码必须为 2002");
            let reason = decisions.iter().filter_map(|d| d.denial_reason).next_back();
            (false, reason)
        }
    }
}

fn one(path: &str) -> Vec<String> {
    vec![path.to_string()]
}

// ---------- L3 拦截用例（全部必须被拒） ----------

#[test]
fn blocks_personal_desktop() {
    let (allowed, reason) = last_denial(one(r"C:\Users\alice\Desktop\secret.docx"), vec![]);
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3PersonalDir));
}

#[test]
fn blocks_personal_documents() {
    let (allowed, reason) = last_denial(one(r"C:\Users\alice\Documents\tax.pdf"), vec![]);
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3PersonalDir));
}

#[test]
fn blocks_personal_downloads() {
    let (allowed, reason) = last_denial(one(r"D:\Users\bob\Downloads\installer.exe"), vec![]);
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3PersonalDir));
}

#[test]
fn blocks_personal_pictures() {
    let (allowed, reason) = last_denial(one(r"C:\Users\alice\Pictures\birthday.jpg"), vec![]);
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3PersonalDir));
}

#[test]
fn blocks_personal_videos() {
    let (allowed, reason) = last_denial(one(r"C:\Users\alice\Videos\clip.mp4"), vec![]);
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3PersonalDir));
}

#[test]
fn blocks_personal_music() {
    let (allowed, reason) = last_denial(one(r"C:\Users\alice\Music\song.mp3"), vec![]);
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3PersonalDir));
}

#[test]
fn blocks_browser_user_data() {
    let (allowed, reason) = last_denial(
        one(r"C:\Users\alice\AppData\Local\Google\Chrome\User Data\Default\Cache"),
        vec![],
    );
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3BrowserProfile));
}

#[test]
fn blocks_browser_edge_user_data() {
    let (allowed, reason) = last_denial(
        one(r"C:\Users\alice\AppData\Local\Microsoft\Edge\User Data"),
        vec![],
    );
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3BrowserProfile));
}

#[test]
fn blocks_browser_bookmarks_file() {
    // 刻意放在 User Data 之外：精确文件名规则命中 L3BrowserSensitive。
    let (allowed, reason) = last_denial(one(r"C:\Temp\stolen\Bookmarks"), vec![]);
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3BrowserSensitive));
}

#[test]
fn blocks_browser_login_data() {
    let (allowed, reason) = last_denial(one(r"C:\Temp\stolen\Login Data"), vec![]);
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3BrowserSensitive));
}

#[test]
fn blocks_browser_history() {
    let (allowed, reason) = last_denial(one(r"C:\Temp\stolen\History"), vec![]);
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3BrowserSensitive));
}

#[test]
fn blocks_system32() {
    let (allowed, reason) = last_denial(one(r"C:\Windows\System32\kernel32.dll"), vec![]);
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3SystemPath));
}

#[test]
fn blocks_windows_installer() {
    let (allowed, reason) = last_denial(one(r"C:\Windows\Installer\{GUID}.msi"), vec![]);
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3SystemPath));
}

#[test]
fn blocks_pagefile() {
    let (allowed, reason) = last_denial(one(r"C:\pagefile.sys"), vec![]);
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3SystemPath));
}

#[test]
fn blocks_swapfile() {
    let (allowed, reason) = last_denial(one(r"C:\swapfile.sys"), vec![]);
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3SystemPath));
}

#[test]
fn blocks_resetbase_action() {
    let (allowed, _) = last_denial(vec![], vec!["dism /ResetBase".to_string()]);
    assert!(!allowed);
}

#[test]
fn blocks_path_traversal() {
    let (allowed, reason) = last_denial(
        one(r"C:\Windows\Temp\..\..\Users\alice\Documents\bank.pdf"),
        vec![],
    );
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::PathTraversal));
}

#[test]
fn blocks_case_insensitive() {
    let (allowed, reason) = last_denial(one(r"c:\wInDoWs\sYsTeM32\x.dll"), vec![]);
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3SystemPath));
}

#[test]
fn blocks_mixed_separators() {
    let (allowed, reason) = last_denial(one("C:/Windows/System32/x.dll"), vec![]);
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3SystemPath));
}

// ---------- 放行 / 行为用例 ----------

#[test]
fn allows_windows_temp_log() {
    let (allowed, _) = last_denial(one(r"C:\Windows\Temp\update.log"), vec![]);
    assert!(allowed);
}

#[test]
fn allows_user_appdata_local_temp() {
    let (allowed, _) = last_denial(one(r"C:\Users\alice\AppData\Local\Temp\foo.tmp"), vec![]);
    assert!(allowed);
}

#[test]
fn allows_simple_cache_is_rejected_as_profile_prefix() {
    // 反例：User Data 主目录前缀命中，Cache 子目录不代表放行。
    let (allowed, reason) = last_denial(
        one(r"C:\Users\alice\AppData\Local\Google\Chrome\User Data\Default\Cache\data_1"),
        vec![],
    );
    assert!(!allowed);
    assert_eq!(reason, Some(DenialReason::L3BrowserProfile));
}

#[test]
fn execute_rejected_does_not_call_deleter() {
    let req = ExecuteRequest {
        paths: vec![
            r"C:\Users\alice\Desktop\secret.docx".to_string(), // L3
            r"C:\Windows\Temp\update.log".to_string(),         // 合法
        ],
        actions: vec![],
    };
    let deleter = NoopDeleter::default();
    match execute(&req, &deleter).expect("no error") {
        ExecuteOutcome::Allowed { .. } => panic!("should be Rejected"),
        ExecuteOutcome::Rejected { decisions, code } => {
            assert_eq!(code, REJECT_CODE);
            assert_eq!(decisions.len(), 2);
        }
    }
    // 关键断言：被拒时 NoopDeleter 内部 Vec 长度为 0（完全没被调用）。
    assert_eq!(deleter.recorded().len(), 0);
}

#[test]
fn execute_allowed_returns_all_decisions() {
    let req = ExecuteRequest {
        paths: vec![
            r"C:\Windows\Temp\update.log".to_string(),
            r"C:\Users\alice\AppData\Local\Temp\foo.tmp".to_string(),
        ],
        actions: vec![],
    };
    let deleter = NoopDeleter::default();
    match execute(&req, &deleter).expect("no error") {
        ExecuteOutcome::Allowed { decisions } => assert_eq!(decisions.len(), 2),
        ExecuteOutcome::Rejected { .. } => panic!("should be Allowed"),
    }
}
