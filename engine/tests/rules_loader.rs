//! task-005 §G 集成测试：临时 rules/ 目录 + 程序算 SHA-256。
//!
//! 不发网络请求、不读真实规则目录；每个用例自建合法 cache_rules.json，
//! 再按用例故意破坏一处（hash / meta / 变量 / L3）。

use engine::execute::normalize::normalize;
use engine::execute::whitelist::{classify, classify_with_external};
use engine::rules::env_expand::{expand, ExpandError};
use engine::rules::loader::{external_deny_list, load, LoadError};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// 写一对合法文件（rules_dir = tempdir.path()），返回 tempdir 句柄。
fn write_valid_pair(rules_dir: &Path, cache: &str) {
    fs::write(rules_dir.join("cache_rules.json"), cache.as_bytes()).unwrap();
    let meta = format!(
        r#"{{"version":"1.0.0","checksum_sha256":"{}","source_url":"https://example.com/r.json","downloaded_at":0,"custom_rules":[]}}"#,
        sha256_hex(cache.as_bytes())
    );
    fs::write(rules_dir.join("rules_meta.json"), meta).unwrap();
}

const VALID_CACHE: &str = r#"{
  "version": "1.0.0",
  "modules": {
    "A": [
      {"id":"npm_cache","category":"npm_cache","display_name":"npm 缓存",
       "paths":["%LOCALAPPDATA%\\npm-cache","~/.npm/_cacache"],
       "risk_level":"L1","description":"d","impact_note":"i","rebuild_note":"r","elevated":false}
    ]
  },
  "whitelist_deny": ["%USERPROFILE%\\Desktop","%USERPROFILE%\\Documents","C:\\Windows\\System32","C:\\pagefile.sys"]
}"#;

#[test]
fn load_ok_with_valid_files() {
    let dir = tempfile::tempdir().unwrap();
    write_valid_pair(dir.path(), VALID_CACHE);
    let loaded = load(dir.path()).expect("合法文件应加载成功");
    assert_eq!(loaded.rules.version, "1.0.0");
    assert_eq!(loaded.rules.modules["A"].len(), 1);
}

#[test]
fn load_rejects_tampered_cache_rules() {
    let dir = tempfile::tempdir().unwrap();
    write_valid_pair(dir.path(), VALID_CACHE);
    // 末尾加一个空格 → 字节变了但 meta 未更新。
    fs::write(
        dir.path().join("cache_rules.json"),
        format!("{VALID_CACHE} "),
    )
    .unwrap();
    match load(dir.path()) {
        Err(LoadError::ChecksumMismatch { expected, actual }) => {
            assert!(!expected.is_empty());
            assert!(!actual.is_empty());
            assert_ne!(expected, actual);
        }
        other => panic!("应为 ChecksumMismatch，实际: {other:?}"),
    }
}

#[test]
fn load_rejects_bad_checksum_in_meta() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("cache_rules.json"), VALID_CACHE.as_bytes()).unwrap();
    fs::write(
        dir.path().join("rules_meta.json"),
        r#"{"version":"1.0.0","checksum_sha256":"0000000000000000000000000000000000000000000000000000000000000000","source_url":"u","downloaded_at":0,"custom_rules":[]}"#,
    )
    .unwrap();
    assert!(matches!(
        load(dir.path()),
        Err(LoadError::ChecksumMismatch { .. })
    ));
}

#[test]
fn load_rejects_unknown_env_var() {
    let bad = r#"{"version":"1.0.0","modules":{"A":[{"id":"x","category":"c","display_name":"n","paths":["%WINDIR%\\Temp"],"risk_level":"L1","description":"d","impact_note":"i","rebuild_note":"r","elevated":false}]},"whitelist_deny":[]}"#;
    let dir = tempfile::tempdir().unwrap();
    write_valid_pair(dir.path(), bad);
    match load(dir.path()) {
        Err(LoadError::Expand(ExpandError::UnknownVar(v))) => assert_eq!(v, "WINDIR"),
        other => panic!("应为 Expand(UnknownVar)，实际: {other:?}"),
    }
}

#[test]
fn load_rejects_l3_in_rules() {
    let bad = r#"{"version":"1.0.0","modules":{"A":[{"id":"danger","category":"c","display_name":"n","paths":["%TEMP%\\x"],"risk_level":"L3","description":"d","impact_note":"i","rebuild_note":"r","elevated":false}]},"whitelist_deny":[]}"#;
    let dir = tempfile::tempdir().unwrap();
    write_valid_pair(dir.path(), bad);
    match load(dir.path()) {
        Err(LoadError::L3InRules(id)) => assert_eq!(id, "danger"),
        other => panic!("应为 L3InRules，实际: {other:?}"),
    }
}

#[test]
fn expand_resolves_six_whitelist_vars() {
    for raw in [
        "%LOCALAPPDATA%\\foo",
        "%APPDATA%\\foo",
        "%TEMP%\\foo",
        "%PROGRAMDATA%\\foo",
        "%USERPROFILE%\\foo",
        "~/foo",
    ] {
        let out = expand(raw).unwrap();
        assert!(!out.is_empty(), "{raw} 展开为空");
        assert!(!out.contains('%'), "{raw} 展开后仍含 %: {out}");
        assert!(out.ends_with("\\foo"), "{raw} → {out}");
    }
}

#[test]
fn expand_rejects_tilde_masquerade() {
    assert_eq!(
        expand("~foo").unwrap_err(),
        ExpandError::TildeMasquerade("~foo".into())
    );
}

#[test]
fn external_deny_list_union_with_builtin() {
    let dir = tempfile::tempdir().unwrap();
    write_valid_pair(dir.path(), VALID_CACHE);
    let loaded = load(dir.path()).unwrap();

    let ext = external_deny_list(&loaded);
    // %USERPROFILE%\Desktop 展开为真实绝对路径。
    let desktop = ext
        .iter()
        .find(|p| p.to_lowercase().ends_with("\\desktop"))
        .expect("外部红线应含 Desktop")
        .clone();

    // 外部命中 → 拒。
    let np = normalize(&desktop).unwrap();
    assert!(classify_with_external(&np, &ext).is_err());

    // System32 不在 external 列表里，仅靠内置红线 → 仍拒（证明是并集而非替换）。
    let sys = normalize(r"C:\Windows\System32\kernel32.dll").unwrap();
    assert!(classify_with_external(&sys, &ext).is_err());
    // 空 external 时内置红线照样生效。
    assert!(classify_with_external(&sys, &[]).is_err());
}

#[test]
fn classify_builtin_unchanged() {
    // 回归保护：task-003 已有行为不被 task-005 改动。
    let p = normalize(r"C:\Users\alice\Desktop\a.docx").unwrap();
    assert!(classify(&p).is_err());
    let s = normalize(r"C:\Windows\System32\kernel32.dll").unwrap();
    assert!(classify(&s).is_err());
    let ok = normalize(r"C:\Windows\Temp\update.log").unwrap();
    assert!(classify(&ok).is_ok());
}

#[test]
fn load_tolerates_crlf_line_endings_on_disk() {
    // Windows 上 git autocrlf / 编辑器可能把 cache_rules.json 写成 CRLF；
    // rules_meta.checksum 存的是 LF 字节哈希，运行时必须归一后再算，
    // 否则合法文件会被误判为篡改（与 CI 守卫步骤同口径）。
    let dir = tempfile::tempdir().unwrap();
    let crlf_cache = VALID_CACHE.replace('\n', "\r\n");
    fs::write(dir.path().join("cache_rules.json"), crlf_cache.as_bytes()).unwrap();
    // meta 里的 checksum 用 LF 版本算（即 rules_meta.json 应当存的值）。
    let meta = format!(
        r#"{{"version":"1.0.0","checksum_sha256":"{}","source_url":"u","downloaded_at":0,"custom_rules":[]}}"#,
        sha256_hex(VALID_CACHE.as_bytes())
    );
    fs::write(dir.path().join("rules_meta.json"), meta).unwrap();

    let loaded = load(dir.path()).expect("CRLF 行尾的规则文件应被正常加载");
    assert_eq!(loaded.rules.version, "1.0.0");
}
