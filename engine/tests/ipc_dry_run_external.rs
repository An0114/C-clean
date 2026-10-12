//! task-010 集成测试：外部 whitelist_deny 接入 cleanup.dryRun。
//!
//! 用 tempfile 造临时 rules/，cache_rules.json 含一条外部红线
//! `%LOCALAPPDATA%\SomeSensitiveCache`（不在内置 PERSONAL_DIRS 中，
//! 以证明是外部红线而非内置 L3 命中），配对 rules_meta.json 写真实 SHA-256，
//! 再走完整 load → external_deny_list → handle_dry_run。

use engine::ipc::commands::dry_run::{handle_dry_run, DryRunReq, MockResolver, ResolvedItem};
use engine::rules::external_deny_list;
use engine::rules::loader::load;
use sha2::{Digest, Sha256};
use std::fs;

/// 算 cache_rules.json 字节的 SHA-256 hex。
fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// 造临时 rules/ 目录，whitelist_deny = ["%LOCALAPPDATA%\\SomeSensitiveCache"]。
/// 选 LOCALAPPDATA 而非 USERPROFILE\Videos，是因为 videos 在内置 PERSONAL_DIRS 中，
/// 无法区分「内置命中」与「外部红线命中」。
fn setup_rules() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let cache = r#"{
      "version": "1.0.0",
      "modules": {},
      "whitelist_deny": ["%LOCALAPPDATA%\\SomeSensitiveCache"]
    }"#;
    fs::write(dir.path().join("cache_rules.json"), cache).unwrap();
    let meta = format!(
        r#"{{"version":"1.0.0","checksum_sha256":"{}","source_url":"https://x/r.json","downloaded_at":0,"custom_rules":[]}}"#,
        sha256_hex(cache.as_bytes())
    );
    fs::write(dir.path().join("rules_meta.json"), meta).unwrap();
    dir
}

/// 外部红线命中（非内置 L3 路径）→ 2002，denial_reason 为 L3SystemPath（外部桶）。
#[test]
fn dry_run_external_deny_blocks_path() {
    let dir = setup_rules();
    let loaded = load(dir.path()).expect("rules load");
    let deny = external_deny_list(&loaded);
    assert_eq!(deny.len(), 1, "应有一条展开后的外部红线: {deny:?}");

    // 用展开后路径的子文件作为待删路径；该路径不在内置 PERSONAL_DIRS 中，
    // 能被拒只能是外部红线命中。
    let target = format!(r"{}\data.bin", deny[0]);
    let resolver = MockResolver::new(vec![ResolvedItem {
        item_id: 1,
        path: target,
        module_code: "A".into(),
        category: "x".into(),
        size_bytes: 10,
    }]);
    let env = handle_dry_run(DryRunReq { item_ids: vec![1] }, &resolver, &deny);
    assert_eq!(env.code, 2002, "外部红线命中必须 2002: {env:?}");
    let data = env.data.expect("2002 需回传 blocked 明细");
    assert_eq!(data.blocked_by_l3.len(), 1);
    assert!(data.blocked_by_l3[0].path.contains("SomeSensitiveCache"));
    // 外部红线命中归 L3SystemPath（非内置 L3PersonalDir）。
    assert!(
        data.blocked_by_l3[0].denial_reason.contains("L3SystemPath"),
        "外部红线应记 L3SystemPath，实际: {}",
        data.blocked_by_l3[0].denial_reason
    );
}

/// 不传外部红线（&[]）时，内置 L3 桌面红线仍独立生效。
#[test]
fn dry_run_no_external_deny_falls_back_to_builtin() {
    let resolver = MockResolver::new(vec![ResolvedItem {
        item_id: 1,
        path: r"C:\Users\alice\Desktop\secret.docx".into(),
        module_code: "A".into(),
        category: "x".into(),
        size_bytes: 10,
    }]);
    let env = handle_dry_run(DryRunReq { item_ids: vec![1] }, &resolver, &[]);
    assert_eq!(env.code, 2002);
    let data = env.data.unwrap();
    assert_eq!(data.blocked_by_l3.len(), 1);
    assert!(data.blocked_by_l3[0]
        .denial_reason
        .contains("L3PersonalDir"));
}

/// 纯内置红线独立生效（不依赖 rules 文件）。
#[test]
fn dry_run_builtin_still_blocks_when_rules_empty() {
    let resolver = MockResolver::new(vec![ResolvedItem {
        item_id: 1,
        path: r"C:\Windows\System32\kernel32.dll".into(),
        module_code: "A".into(),
        category: "x".into(),
        size_bytes: 10,
    }]);
    let env = handle_dry_run(DryRunReq { item_ids: vec![1] }, &resolver, &[]);
    assert_eq!(env.code, 2002);
    assert_eq!(env.data.unwrap().blocked_by_l3.len(), 1);
}
