//! task-041 集成测试：rules reqwest 超时 + 404 可读错误文案。
//!
//! 用 `RULES_HTTP_TIMEOUT_SECS` / `RULES_HTTP_CONNECT_SECS` 把超时压到 1s 级，
//! 避免单测真等 10s。

use engine::rules::remote::{fetch_remote_meta_with_url, fetch_remote_rules_bytes_with_url};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

/// 起一个 tiny HTTP server，按给定状态码 + 响应体回复。返回 (port, shutdown_signal)。
fn spawn_server(status: u16, body: &'static str) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            // 读光请求头
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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
fn check_update_404_returns_readable_error() {
    let port = spawn_server(404, "not found");
    let url = format!("http://127.0.0.1:{port}/rules_meta.json");
    let err = fetch_remote_meta_with_url(&url).unwrap_err();
    assert!(err.contains("404"), "expected 404 message, got: {err}");
}

#[test]
fn check_update_timeout_returns_readable_error() {
    // 压短超时到 1s，避免真等 10s。
    std::env::set_var("RULES_HTTP_TIMEOUT_SECS", "1");
    std::env::set_var("RULES_HTTP_CONNECT_SECS", "1");

    // 起一个 server，只 listen 不 accept，TCP 握手由内核完成但永远不回响应，
    // 触发 client 读超时。
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        // 故意不 accept，保持 listener 挂着
        thread::sleep(std::time::Duration::from_secs(10));
        drop(listener);
    });

    let url = format!("http://127.0.0.1:{port}/rules_meta.json");
    let err = fetch_remote_meta_with_url(&url).unwrap_err();
    // 清理 env，避免影响其他用例。
    std::env::remove_var("RULES_HTTP_TIMEOUT_SECS");
    std::env::remove_var("RULES_HTTP_CONNECT_SECS");

    assert!(err.contains("超时"), "expected timeout message, got: {err}");
}

#[test]
fn fetch_rules_bytes_404_readable() {
    let port = spawn_server(404, "nope");
    let url = format!("http://127.0.0.1:{port}/cache_rules.json");
    let err = fetch_remote_rules_bytes_with_url(&url).unwrap_err();
    assert!(err.contains("404"), "got: {err}");
}
