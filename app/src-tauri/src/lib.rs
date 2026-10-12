//! Tauri 主进程壳（M0）。
//!
//! 注册命令：ping（task-001）、scan_start / cleanup_dry_run（task-004）。
//! sessionId 计数器用简单的 `Mutex<i64>` State 注入（task-004）。

mod commands;
pub mod progress;

/// 空命令：返回 engine 占位字符串，供前端联调时验证 IPC 通路。
#[tauri::command]
fn ping() -> &'static str {
    engine::hello_engine()
}

/// Tauri 应用入口：构造默认 Builder 并挂上命令，随后阻塞运行窗口。
pub fn run() {
    tauri::Builder::default()
        .manage(std::sync::Mutex::new(0_i64))
        .invoke_handler(tauri::generate_handler![
            ping,
            commands::scan_start,
            commands::cleanup_dry_run
        ])
        .run(tauri::generate_context!())
        .expect("error while running C-Disk Cleaner");
}
