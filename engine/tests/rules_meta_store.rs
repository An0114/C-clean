//! task-022 集成测试：rules_meta 单行表 upsert / get_active。

use engine::db::conn::open;
use engine::rules::meta_store::{get_active_rules_meta, upsert_active_rules_meta};

#[test]
fn upsert_then_get_active() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    upsert_active_rules_meta(&conn, "1.0.0", "abc123", "local").unwrap();
    let row = get_active_rules_meta(&conn).unwrap().expect("row");
    assert_eq!(row.version, "1.0.0");
    assert_eq!(row.checksum_sha256, "abc123");
}

#[test]
fn upsert_new_version_replaces_old() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    upsert_active_rules_meta(&conn, "1.0.0", "sha_v1", "local").unwrap();
    upsert_active_rules_meta(&conn, "1.1.0", "sha_v2", "remote").unwrap();
    let row = get_active_rules_meta(&conn).unwrap().expect("row");
    assert_eq!(row.version, "1.1.0");
    assert_eq!(row.checksum_sha256, "sha_v2");
    // 单行表：id=1 唯一，不会残留旧行。
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM rules_meta", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 1);
}

#[test]
fn get_active_empty_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    assert!(get_active_rules_meta(&conn).unwrap().is_none());
}
