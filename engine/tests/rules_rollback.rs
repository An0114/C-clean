//! task-044 集成测试：rules 手动回滚。

use engine::db::conn::open;
use engine::rules::rollback::{cache_bak_path, cache_path, has_backup, rollback};

/// 造一个最小合法的 cache_rules.json 字节。
fn make_cache(version: &str) -> Vec<u8> {
    serde_json::json!({
        "version": version,
        "modules": {},
        "whitelist_deny": []
    })
    .to_string()
    .into_bytes()
}

#[test]
fn has_backup_false_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path()).unwrap();
    assert!(!has_backup(dir.path()));
}

#[test]
fn has_backup_true_when_bak_exists() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path()).unwrap();
    std::fs::write(cache_bak_path(dir.path()), make_cache("0.0.0")).unwrap();
    assert!(has_backup(dir.path()));
}

#[test]
fn rollback_no_bak_returns_err() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path()).unwrap();
    let conn = open(dir.path()).unwrap();
    let err = rollback(dir.path(), &conn).unwrap_err();
    assert!(err.contains("没有可回滚的备份"), "got: {err}");
}

#[test]
fn rollback_restores_from_bak() {
    let dir = tempfile::tempdir().unwrap();
    let rules_dir = dir.path().join("rules");
    std::fs::create_dir_all(&rules_dir).unwrap();
    let conn = open(dir.path()).unwrap();

    // 当前 cache = v1，.bak = v0
    std::fs::write(cache_path(&rules_dir), make_cache("1.0.0")).unwrap();
    std::fs::write(cache_bak_path(&rules_dir), make_cache("0.9.0")).unwrap();

    let r = rollback(&rules_dir, &conn).unwrap();
    assert_eq!(r.previous_version, "0.9.0");
    assert!(!r.restored_sha256.is_empty());
    assert!(r.message.contains("0.9.0"));

    // cache_rules.json 内容已变成 v0
    let now = std::fs::read(cache_path(&rules_dir)).unwrap();
    let v: serde_json::Value = serde_json::from_slice(&now).unwrap();
    assert_eq!(v["version"], "0.9.0");
}

#[test]
fn rollback_corrupted_bak_returns_err() {
    let dir = tempfile::tempdir().unwrap();
    let rules_dir = dir.path().join("rules");
    std::fs::create_dir_all(&rules_dir).unwrap();
    let conn = open(dir.path()).unwrap();
    std::fs::write(cache_path(&rules_dir), make_cache("1.0.0")).unwrap();
    std::fs::write(cache_bak_path(&rules_dir), b"{{{not json").unwrap();

    let err = rollback(&rules_dir, &conn).unwrap_err();
    assert!(err.contains("备份文件损坏"), "got: {err}");
}
