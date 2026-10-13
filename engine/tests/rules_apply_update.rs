//! task-023 集成测试：rules.apply_update 安全替换。

use engine::db::conn::open;
use engine::rules::remote::{apply_update, ApplyError, RemoteRulesMeta};
use sha2::{Digest, Sha256};

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

#[test]
fn apply_update_replaces_files_and_upserts_db() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let rules_dir = root.join("rules");
    std::fs::create_dir_all(&rules_dir).unwrap();

    // 旧文件
    std::fs::write(rules_dir.join("cache_rules.json"), r#"{"old":1}"#).unwrap();
    std::fs::write(rules_dir.join("rules_meta.json"), r#"{"version":"v0"}"#).unwrap();

    let conn = open(root).unwrap();

    let new_bytes = br#"{"new":1}"#;
    let new_sha = sha256_hex(new_bytes);
    let meta = RemoteRulesMeta {
        version: "v1".into(),
        checksum_sha256: new_sha,
    };

    let result = apply_update(&rules_dir, &conn, &meta, new_bytes).unwrap();
    assert_eq!(result.new_version, "v1");
    assert_eq!(result.new_sha256, meta.checksum_sha256);

    // 文件被替换
    assert_eq!(
        std::fs::read(rules_dir.join("cache_rules.json")).unwrap(),
        new_bytes
    );
    let new_meta = std::fs::read_to_string(rules_dir.join("rules_meta.json")).unwrap();
    assert!(new_meta.contains("v1"));

    // .bak 存在
    assert!(rules_dir.join("cache_rules.json.bak").exists());
    assert!(rules_dir.join("rules_meta.json.bak").exists());

    // DB active 是新版本
    let row = engine::rules::meta_store::get_active_rules_meta(&conn)
        .unwrap()
        .unwrap();
    assert_eq!(row.version, "v1");
    assert_eq!(row.checksum_sha256, meta.checksum_sha256);
}

#[test]
fn apply_update_sha_mismatch_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let rules_dir = root.join("rules");
    std::fs::create_dir_all(&rules_dir).unwrap();

    let old_cache = br#"{"old":1}"#;
    let old_meta = br#"{"version":"v0"}"#;
    std::fs::write(rules_dir.join("cache_rules.json"), old_cache).unwrap();
    std::fs::write(rules_dir.join("rules_meta.json"), old_meta).unwrap();

    let conn = open(root).unwrap();
    let new_bytes = br#"{"new":1}"#;
    let meta = RemoteRulesMeta {
        version: "v1".into(),
        checksum_sha256: "deadbeef".into(),
    };

    let err = apply_update(&rules_dir, &conn, &meta, new_bytes).unwrap_err();
    match err {
        ApplyError::ShaMismatch { expected, actual } => {
            assert_eq!(expected, "deadbeef");
            assert_eq!(actual, sha256_hex(new_bytes));
        }
        _ => panic!("expected ShaMismatch, got {err:?}"),
    }

    // 本地文件没动，.bak 没生成
    assert_eq!(
        std::fs::read(rules_dir.join("cache_rules.json")).unwrap(),
        old_cache
    );
    assert_eq!(
        std::fs::read(rules_dir.join("rules_meta.json")).unwrap(),
        old_meta
    );
    assert!(!rules_dir.join("cache_rules.json.bak").exists());
    assert!(!rules_dir.join("rules_meta.json.bak").exists());
}

#[test]
fn apply_update_db_failure_rolls_back() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let rules_dir = root.join("rules");
    std::fs::create_dir_all(&rules_dir).unwrap();

    let old_cache = br#"{"old":1}"#;
    let old_meta = br#"{"version":"v0"}"#;
    std::fs::write(rules_dir.join("cache_rules.json"), old_cache).unwrap();
    std::fs::write(rules_dir.join("rules_meta.json"), old_meta).unwrap();

    let conn = open(root).unwrap();
    // 把 rules_meta 表 DROP 掉，upsert 必然失败。
    conn.execute("DROP TABLE rules_meta", []).unwrap();

    let new_bytes = br#"{"new":1}"#;
    let new_sha = sha256_hex(new_bytes);
    let meta = RemoteRulesMeta {
        version: "v1".into(),
        checksum_sha256: new_sha,
    };

    let err = apply_update(&rules_dir, &conn, &meta, new_bytes).unwrap_err();
    match err {
        ApplyError::Db(_) => {}
        other => panic!("expected Db, got {other:?}"),
    }

    // 文件被回滚到旧内容
    assert_eq!(
        std::fs::read(rules_dir.join("cache_rules.json")).unwrap(),
        old_cache
    );
    assert_eq!(
        std::fs::read(rules_dir.join("rules_meta.json")).unwrap(),
        old_meta
    );
}
