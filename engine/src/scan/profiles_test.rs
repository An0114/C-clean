//! task-083 C1：多 Profile 枚举单测（临时目录 + 假 INI，不读用户真实配置）。

use crate::scan::profiles::{chromium_profiles, firefox_profiles};
use std::fs;

#[test]
fn chromium_enumerates_default_and_numbered_profiles_only() {
    let tmp = tempfile::tempdir().unwrap();
    let user_data = tmp.path().join("User Data");
    fs::create_dir_all(user_data.join("Default").join("Cache")).unwrap();
    fs::create_dir_all(user_data.join("Default").join("Code Cache")).unwrap();
    fs::create_dir_all(user_data.join("Profile 1").join("GPUCache")).unwrap();
    fs::create_dir_all(user_data.join("Profile 2").join("GPUCache")).unwrap();
    // 这些都不该被当成 profile
    fs::create_dir_all(user_data.join("Crashpad").join("Cache")).unwrap();
    fs::create_dir_all(user_data.join("Profile X").join("Cache")).unwrap();
    fs::create_dir_all(user_data.join("Profile 3")).unwrap(); // 没有任何缓存目录
    fs::write(user_data.join("Local State"), b"{}").unwrap();

    let profiles = chromium_profiles(&user_data);
    let names: Vec<&str> = profiles.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["Default", "Profile 1", "Profile 2"]);
    assert_eq!(
        profiles[0].cache_roots.len(),
        2,
        "Default 有 Cache + Code Cache"
    );
    assert_eq!(profiles[1].cache_roots.len(), 1);
}

#[test]
fn chromium_returns_empty_for_missing_or_empty_dir() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(chromium_profiles(&tmp.path().join("does-not-exist")).is_empty());
    let empty = tmp.path().join("empty");
    fs::create_dir_all(&empty).unwrap();
    assert!(chromium_profiles(&empty).is_empty());
}

#[test]
fn firefox_parses_profiles_ini_and_skips_install_sections() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("Profiles");
    fs::create_dir_all(root.join("abc.default-release").join("cache2")).unwrap();
    fs::create_dir_all(root.join("xyz.dev-edition-default").join("startupCache")).unwrap();

    let ini = r#"
[General]
StartWithLastProfile=1

[Profile0]
Name=default-release
IsRelative=1
Path=Profiles/abc.default-release
Default=1

[Profile1]
Name=dev
IsRelative=1
Path=Profiles/xyz.dev-edition-default

[Install4F96D1932A9F858E]
Default=Profiles/abc.default-release
Locked=1
"#;
    let profiles = firefox_profiles(ini, tmp.path());
    let names: Vec<&str> = profiles.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["default-release", "dev"]);
    assert_eq!(profiles[0].cache_roots.len(), 1);
    assert!(profiles[0].cache_roots[0].ends_with("cache2"));
}

#[test]
fn firefox_handles_absolute_path_and_ignores_incomplete_sections() {
    let tmp = tempfile::tempdir().unwrap();
    let abs = tmp.path().join("absolute-profile");
    fs::create_dir_all(abs.join("cache2")).unwrap();

    let ini = format!(
        "[Profile0]\nName=abs\nIsRelative=0\nPath={}\n\n[Profile1]\nName=no-path\nIsRelative=1\n",
        abs.to_string_lossy().replace('\\', "/")
    );
    let profiles = firefox_profiles(&ini, tmp.path());
    assert_eq!(profiles.len(), 1, "缺 Path 的段不该产出 profile");
    assert_eq!(profiles[0].name, "abs");
}

#[test]
fn firefox_empty_ini_yields_no_profiles() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(firefox_profiles("", tmp.path()).is_empty());
    assert!(firefox_profiles("[General]\nStartWithLastProfile=1\n", tmp.path()).is_empty());
}
