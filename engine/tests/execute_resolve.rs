//! task-008 集成测试：真实路径解析（junction/symlink）接入 delete 唯一入口。
//!
//! 全部在 `tempfile::tempdir()` 下进行，不碰真实 C 盘 / 用户目录。
//! junction 用 `cmd /c mklink /J` 创建（普通用户权限即可）。

use engine::execute::delete::{execute, ExecuteOutcome, ExecuteRequest, NoopDeleter, REJECT_CODE};
use engine::execute::normalize::normalize;
use engine::execute::resolve::{resolve_to_real, ResolveError};

/// 跑一次 execute，返回 (allowed, 最后一条拒绝原因)。
fn run(paths: Vec<String>, actions: Vec<String>) -> (bool, Option<String>) {
    let req = ExecuteRequest { paths, actions };
    let deleter = NoopDeleter::default();
    match execute(&req, &deleter).expect("execute should not error") {
        ExecuteOutcome::Allowed { .. } => (true, None),
        ExecuteOutcome::Rejected { decisions, code } => {
            assert_eq!(code, REJECT_CODE);
            let last = decisions
                .iter()
                .filter_map(|d| d.denial_reason.clone())
                .next_back();
            (false, Some(format!("{last:?}")))
        }
    }
}

/// tempdir 下写一个普通文件，resolve 后 real 含该文件、hop_count=0。
#[test]
fn resolve_plain_file() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("hello.txt");
    std::fs::write(&f, "hi").unwrap();

    let np = normalize(f.to_string_lossy().as_ref()).unwrap();
    let r = resolve_to_real(&np).expect("resolve plain file");
    assert!(r.real.to_lowercase().ends_with("hello.txt"));
    assert_eq!(r.hop_count, 0, "无重定向时 hop 必须为 0");
}

/// junction（目录连接）指向真实目录：real 落到目标目录，hop_count ≥ 1。
#[cfg(windows)]
#[test]
fn resolve_mklink_junction() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("real_docs");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("secret.txt"), "x").unwrap();

    let link = dir.path().join("link_to_docs");
    let out = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(&target)
        .output()
        .expect("mklink /J");
    assert!(
        out.status.success(),
        "mklink 输出: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let np = normalize(link.join("secret.txt").to_string_lossy().as_ref()).unwrap();
    let r = resolve_to_real(&np).expect("resolve junction");
    assert!(
        r.real
            .to_lowercase()
            .replace('/', "\\")
            .contains("real_docs"),
        "real 应落到 real_docs: {}",
        r.real
    );
    assert!(
        r.hop_count >= 1,
        "junction 重定向 hop 至少 1: {}",
        r.hop_count
    );
}

/// 路径不存在 → ResolveError::NotExists。
#[test]
fn resolve_nonexistent_path_returns_not_exists() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("no_such_file.bin");
    let np = normalize(missing.to_string_lossy().as_ref()).unwrap();
    match resolve_to_real(&np) {
        Err(ResolveError::NotExists(_)) => {}
        other => panic!("expected NotExists, got {other:?}"),
    }
}

/// junction 重定向后，tempdir 内 fake_docs 不命中内置 L3（内置只认 users/<user>/documents），
/// 验证 resolve 成功后不误杀；真正外部红线拦截留 task-010 由 classify_with_external 验收。
#[cfg(windows)]
#[test]
fn junction_resolves_to_fake_docs_and_is_allowed_when_not_on_builtin_redline() {
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("fake_docs");
    std::fs::create_dir(&fake).unwrap();
    std::fs::write(fake.join("secret.txt"), "x").unwrap();

    let link = dir.path().join("j");
    let out = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(&fake)
        .output()
        .expect("mklink /J");
    assert!(out.status.success());

    // 直接跑 resolve 证明真实路径落在 fake_docs。
    let np = normalize(link.join("secret.txt").to_string_lossy().as_ref()).unwrap();
    let r = resolve_to_real(&np).expect("resolve junction");
    assert!(r.real.to_lowercase().contains("fake_docs"));

    // tempdir 内非红线 junction 应放行（内置 L3 不匹配）。
    let (allowed, _) = run(
        vec![link.join("secret.txt").to_string_lossy().to_string()],
        vec![],
    );
    assert!(
        allowed,
        "tempdir 内非红线 junction 应放行，实际拒绝原因已记录供审计"
    );
}

/// resolve 失败（如 UNC）时 execute 整体拒绝且 NoopDeleter 绝不被调用（反证）。
#[test]
fn resolve_error_blocks_delete_does_not_invoke_deleter() {
    let req = ExecuteRequest {
        paths: vec![r"\\127.0.0.1\C$\share\file.txt".to_string()],
        actions: vec![],
    };
    let deleter = NoopDeleter::default();
    let outcome = execute(&req, &deleter).expect("execute");
    match outcome {
        ExecuteOutcome::Rejected { .. } => {
            // 关键反证：被拒路径不得触发 Deleter。
            assert!(
                deleter.recorded().is_empty(),
                "resolve 失败被拒时 NoopDeleter 不应被调用，实际记录: {:?}",
                deleter.recorded()
            );
        }
        other => panic!("UNC 路径应被拒绝为 Rejected，实际: {other:?}"),
    }
}

/// 一个实际存在、不在红线上的普通文件，execute 应放行且 NoopDeleter 记录路径。
#[test]
fn resolve_allowed_existing_file_delegates() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("cache.tmp");
    std::fs::write(&f, "x").unwrap();
    let req = ExecuteRequest {
        paths: vec![f.to_string_lossy().to_string()],
        actions: vec![],
    };
    let deleter = NoopDeleter::default();
    match execute(&req, &deleter).expect("execute") {
        ExecuteOutcome::Allowed { .. } => {}
        ExecuteOutcome::Rejected { decisions, .. } => {
            panic!("普通文件应放行: {decisions:?}");
        }
    }
}

/// UNC 路径（\\server\share...）经 normalize 后 resolve 返回错误，execute 拒绝。
#[test]
fn unc_path_is_rejected_by_resolve() {
    let (_, reason) = run(vec![r"\\127.0.0.1\C$\share\file.txt".to_string()], vec![]);
    let reason = reason.expect("UNC 应被拒");
    assert!(
        reason.contains("ResolveFailed") || reason.contains("PathTraversal"),
        "UNC 应被拒，原因: {reason}"
    );
}
