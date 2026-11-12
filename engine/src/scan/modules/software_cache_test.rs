//! task-083 B2：软件缓存识别的单测（不读真实注册表、不真扫用户目录）。
//!
//! 用 `tempfile` 造临时缓存目录，`cache_paths` 直接写绝对路径
//! （`env_expand::expand` 对无变量的绝对路径原样返回），因此完全可注入。

use crate::scan::modules::software_cache::{
    category_slug, is_installed, scan, validate, SoftwareRule, SoftwareRules,
};
use crate::scan::software_enum::SoftwareEntry;

fn entry(name: &str) -> SoftwareEntry {
    SoftwareEntry {
        name: name.to_string(),
        version: "1.0".to_string(),
        publisher: String::new(),
        install_location: String::new(),
        registry_key: String::new(),
    }
}

fn rule(paths: &[&str], level: &str) -> SoftwareRule {
    SoftwareRule {
        software_name: "TestSoft".to_string(),
        match_names: vec!["TestSoft".to_string()],
        cache_paths: paths.iter().map(|s| s.to_string()).collect(),
        risk_level: level.to_string(),
        impact_note: String::new(),
        rebuild_note: String::new(),
    }
}

fn rules_of(rule: SoftwareRule) -> SoftwareRules {
    SoftwareRules {
        version: "1.0.0".to_string(),
        software: vec![rule],
    }
}

#[test]
fn validate_rejects_user_data_paths() {
    // 这个守卫是为了挡住「顺手把聊天记录 / 凭证库写进缓存路径」的事故
    for bad in [
        r"C:\Temp\FileStorage",
        r"C:\Temp\Msg\db",
        r"C:\Temp\Login Data",
        r"C:\Temp\settings.json",
        r"C:\Temp\data.db",
    ] {
        let r = rules_of(rule(&[bad], "L1"));
        assert!(validate(&r).is_err(), "{bad} 必须被拒绝");
    }
}

#[test]
fn validate_rejects_bad_level_and_empty_name() {
    assert!(validate(&rules_of(rule(&[r"C:\Temp"], "L3"))).is_err());
    let mut r = rule(&[r"C:\Temp"], "L1");
    r.software_name = "  ".to_string();
    assert!(validate(&rules_of(r)).is_err());
}

#[test]
fn validate_accepts_normal_cache_rules() {
    assert!(validate(&rules_of(rule(&[r"C:\Temp\Cache"], "L1"))).is_ok());
    assert!(validate(&rules_of(rule(&[r"%APPDATA%\Code\Cache"], "L2"))).is_ok());
}

#[test]
fn is_installed_matches_versioned_display_names() {
    let r = rule(&[], "L1");
    assert!(is_installed(&r, &[entry("TestSoft 1.91.0 (User)")]).is_some());
    assert!(is_installed(&r, &[entry("testsoft (64-bit)")]).is_some());
    assert!(is_installed(&r, &[entry("Unrelated App"), entry("TestSoft")]).is_some());
}

#[test]
fn is_installed_returns_none_when_absent() {
    let r = rule(&[], "L1");
    assert!(is_installed(&r, &[entry("Unrelated App")]).is_none());
    assert!(is_installed(&r, &[]).is_none());
}

#[test]
fn scan_skips_not_installed_software_even_if_dir_exists() {
    // 目录真的存在且有文件，但软件没装 → 一个条目都不该产出
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.tmp"), b"1234567890").unwrap();
    let r = rules_of(rule(&[&dir.path().to_string_lossy()], "L1"));
    let out = scan(&[entry("Unrelated App")], &r, &[]);
    assert!(out.is_empty(), "未安装的软件不该扫其缓存目录");
}

#[test]
fn scan_collects_installed_software_cache() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.tmp"), b"1234567890").unwrap();
    let r = rules_of(rule(&[&dir.path().to_string_lossy()], "L2"));
    let out = scan(&[entry("TestSoft 2.0")], &r, &[]);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].module_code, "software_cache");
    assert_eq!(out[0].category, "software_cache_testsoft");
    assert_eq!(out[0].size_bytes, 10);
    assert_eq!(out[0].risk_level, 2, "L2 → 整数 2");
}

/// task-086 3.3：影响说明从规则库透传到 ScanItem。
#[test]
fn scan_passes_impact_notes_from_rule_to_item() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.tmp"), b"data").unwrap();
    let mut r = rule(&[&dir.path().to_string_lossy()], "L2");
    r.impact_note = "删除后下次启动重新构建缓存".to_string();
    r.rebuild_note = "重启软件自动重建".to_string();
    let out = scan(&[entry("TestSoft")], &rules_of(r), &[]);
    assert_eq!(out.len(), 1);
    assert_eq!(
        out[0].impact_note.as_deref(),
        Some("删除后下次启动重新构建缓存"),
        "ScanItem 必须携带规则库的 impact_note"
    );
    assert_eq!(out[0].rebuild_note.as_deref(), Some("重启软件自动重建"));
}

#[test]
fn category_slug_is_stable_and_ascii_only() {
    assert_eq!(category_slug("Visual Studio Code"), "visual_studio_code");
    assert_eq!(category_slug("WeChat (64-bit)"), "wechat__64_bit");
    assert_eq!(
        category_slug("微信"),
        "unknown",
        "全是非 ASCII 时退回稳定占位，不能给空 category"
    );
}

#[test]
fn repository_software_rules_load_and_validate() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../rules/software_rules.json");
    let loaded = crate::scan::modules::software_cache::load(&path)
        .expect("真实软件规则库必须能加载并通过防呆校验");
    assert!(loaded.software.len() >= 5, "任务书要求至少 5 个常见软件");
    for expected in [
        "Visual Studio Code",
        "Slack",
        "Discord",
        "Microsoft Teams",
        "WeChat",
    ] {
        assert!(
            loaded.software.iter().any(|r| r.software_name == expected),
            "缺少 {expected}"
        );
    }
}
