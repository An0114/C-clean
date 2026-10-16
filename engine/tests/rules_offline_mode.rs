//! task-042 集成测试：rules.checkUpdate 三态（Offline / UpToDate / NewVersion）。

use engine::rules::remote::{fetch_remote_meta_with_url, CheckStatus, CheckUpdateResult};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

/// 起一个 tiny HTTP server 返回给定 JSON body。
fn spawn_json(status: u16, body: &'static str) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            );
            let _ = stream.flush();
        }
    });
    port
}

#[test]
fn check_update_offline_when_request_fails() {
    // 压短超时，起永不响应的端口。
    std::env::set_var("RULES_HTTP_TIMEOUT_SECS", "1");
    std::env::set_var("RULES_HTTP_CONNECT_SECS", "1");
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        thread::sleep(std::time::Duration::from_secs(10));
        drop(listener);
    });

    let url = format!("http://127.0.0.1:{port}/rules_meta.json");
    let err = fetch_remote_meta_with_url(&url).unwrap_err();
    std::env::remove_var("RULES_HTTP_TIMEOUT_SECS");
    std::env::remove_var("RULES_HTTP_CONNECT_SECS");

    let r = CheckUpdateResult::offline("1.0.0".into(), "deadbeef".into(), err);
    assert!(matches!(r.status, CheckStatus::Offline));
    assert_eq!(r.local_version, "1.0.0");
    assert!(r.remote_version.is_none());
    assert!(r.message.contains("超时"));
}

#[test]
fn check_update_uptodate_when_same_version() {
    let sha = format!("{:064x}", 42u64);
    let body = format!(r#"{{"version":"1.0.0","checksum_sha256":"{sha}"}}"#);
    let port = spawn_json(200, Box::leak(body.into_boxed_str()));
    let url = format!("http://127.0.0.1:{port}/rules_meta.json");
    let remote = fetch_remote_meta_with_url(&url).unwrap();
    let r = CheckUpdateResult::classify("1.0.0".into(), sha.clone(), &remote);
    assert!(matches!(r.status, CheckStatus::UpToDate));
    assert_eq!(r.remote_version.as_deref(), Some("1.0.0"));
}

#[test]
fn check_update_new_version_when_higher() {
    let old_sha = format!("{:064x}", 1u64);
    let new_sha = format!("{:064x}", 2u64);
    let body = format!(r#"{{"version":"1.1.0","checksum_sha256":"{new_sha}"}}"#);
    let port = spawn_json(200, Box::leak(body.into_boxed_str()));
    let url = format!("http://127.0.0.1:{port}/rules_meta.json");
    let remote = fetch_remote_meta_with_url(&url).unwrap();
    let r = CheckUpdateResult::classify("1.0.0".into(), old_sha, &remote);
    assert!(matches!(r.status, CheckStatus::NewVersion));
    assert_eq!(r.remote_version.as_deref(), Some("1.1.0"));
    assert!(r.message.contains("1.1.0"));
}
