//! Tauri 主进程壳（M0 最小空窗口）。
//!
//! 本任务只注册一个空命令 `ping`，回连 `engine::hello_engine()`，
//! 用于验证「前端 ↔ Tauri 壳 ↔ engine 核心库」三层链路连通。
//! 不注册任何业务 command（scan.start / cleanup.dryRun 等留给 task-004）。

/// 空命令：返回 engine 占位字符串，供前端联调时验证 IPC 通路。
#[tauri::command]
fn ping() -> &'static str {
    engine::hello_engine()
}

/// Tauri 应用入口：构造默认 Builder 并挂上空命令，随后阻塞运行窗口。
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping])
        .run(tauri::generate_context!())
        .expect("error while running C-Disk Cleaner");
}
