//! task-083 B1：软件枚举的单测（用 hive 快照，不读真实注册表）。

use crate::scan::software_enum::{
    merge_snapshots, HiveSnapshot, RawEntry, SnapshotRoot, SoftwareEntry,
};

fn raw(subkey: &str, name: Option<&str>) -> RawEntry {
    RawEntry {
        subkey: subkey.to_string(),
        display_name: name.map(str::to_string),
        display_version: None,
        publisher: None,
        install_location: None,
        system_component: None,
    }
}

fn hklm(entries: Vec<RawEntry>) -> HiveSnapshot {
    HiveSnapshot {
        root: SnapshotRoot::LocalMachine,
        entries,
    }
}

fn hkcu(entries: Vec<RawEntry>) -> HiveSnapshot {
    HiveSnapshot {
        root: SnapshotRoot::CurrentUser,
        entries,
    }
}

#[test]
fn hklm_wins_over_hkcu_on_duplicate() {
    // 同一软件在 HKLM（机器级）和 HKCU（用户级）都有卸载键时，保留 HKLM 那条。
    let mut hkcu_vscode = raw("VSCode-User", Some("Visual Studio Code"));
    hkcu_vscode.display_version = Some("1.90.0-user".into());
    hkcu_vscode.install_location = Some(r"C:\Users\x\AppData\Local\Programs\VSCode".into());

    let mut hklm_vscode = raw("{GUID}", Some("Visual Studio Code"));
    hklm_vscode.display_version = Some("1.91.0".into());
    hklm_vscode.publisher = Some("Microsoft".into());

    let out = merge_snapshots(&[hkcu(vec![hkcu_vscode]), hklm(vec![hklm_vscode])]);
    assert_eq!(out.len(), 1, "同名必须合并成一条");
    assert_eq!(
        out[0].version, "1.91.0",
        "HKLM 优先，哪怕它在快照列表里排后面"
    );
    assert_eq!(out[0].publisher, "Microsoft");
    assert!(out[0].registry_key.starts_with("HKLM\\"));
}

#[test]
fn entries_without_name_or_system_component_are_skipped() {
    let mut patch = raw("KB123456", Some("Windows 补丁"));
    patch.system_component = Some(1);
    let out = merge_snapshots(&[hklm(vec![
        raw("{NO-NAME}", None),
        raw("{EMPTY}", Some("   ")),
        patch,
        raw("RealApp", Some("RealApp")),
    ])]);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].name, "RealApp");
}

#[test]
fn name_normalization_is_case_and_whitespace_insensitive() {
    let out = merge_snapshots(&[hklm(vec![
        raw("a", Some("Visual Studio Code")),
        raw("b", Some("visualstudiocode")),
        raw("c", Some("Visual Studio Code (User)")),
    ])]);
    // 前两条归一后同名，只留一条；带 "(User)" 的归一后不同名，应保留
    assert_eq!(out.len(), 2);
    assert!(out.iter().any(|e| e.name == "Visual Studio Code"));
    assert!(out.iter().any(|e| e.name == "Visual Studio Code (User)"));
}

#[test]
fn output_is_sorted_and_fields_are_mapped() {
    let mut e = raw("Slack", Some("Slack"));
    e.display_version = Some("4.39.95".into());
    e.publisher = Some("Slack Technologies".into());
    e.install_location = Some(r"C:\Program Files\Slack".into());

    let out = merge_snapshots(&[hklm(vec![
        e,
        raw("Chrome", Some("Google Chrome")),
        raw("Code", Some("Visual Studio Code")),
    ])]);
    let names: Vec<&str> = out.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["Google Chrome", "Slack", "Visual Studio Code"]);
    let slack: &SoftwareEntry = out.iter().find(|e| e.name == "Slack").unwrap();
    assert_eq!(slack.version, "4.39.95");
    assert_eq!(slack.publisher, "Slack Technologies");
    assert_eq!(slack.install_location, r"C:\Program Files\Slack");
    assert_eq!(
        slack.registry_key,
        r"HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall\Slack"
    );
}

#[test]
fn empty_snapshots_produce_empty_inventory() {
    assert!(merge_snapshots(&[]).is_empty());
    assert!(merge_snapshots(&[hklm(vec![]), hkcu(vec![])]).is_empty());
}
