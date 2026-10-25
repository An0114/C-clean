//! Tauri 主进程壳（M0）。
//!
//! 注册命令：ping（task-001）、scan_start / cleanup_dry_run（task-004）。
//! sessionId 计数器用简单的 `Mutex<i64>` State 注入（task-004）。
//! debt-cleanup-003：AppPaths 在 setup 内构造并 manage，所有命令从 State 取路径。

mod commands;
pub mod paths;
pub mod progress;

use tauri::Manager;

/// 空命令：返回 engine 占位字符串，供前端联调时验证 IPC 通路。
#[tauri::command]
fn ping() -> &'static str {
    engine::hello_engine()
}

/// Tauri 应用入口：构造默认 Builder 并挂上命令，随后阻塞运行窗口。
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        // bugfix/task-035-updater-acl：为前端 relaunch() 提供 process 插件后端。
        .plugin(tauri_plugin_process::init())
        .manage(std::sync::Mutex::new(0_i64))
        // task-007：在 setup 闭包里拿 AppHandle，构造全局进度 State 并注入。
        // debt-cleanup-003：构造 AppPaths（data_root/rules_dir/scripts_dir/box_dir）并注入。
        .setup(|app| {
            let paths = paths::AppPaths::from_app(app.handle())?;
            app.manage(paths);
            app.manage(progress::ProgressState::new(app.handle().clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            commands::scan_start,
            commands::cleanup_dry_run,
            commands::cleanup_execute,
            commands::cleanup_retry_locked,
            commands::safe_box_restore,
            commands::safe_box_list,
            commands::safe_box_list_sessions,
            commands::safe_box_item_history,
            commands::safe_box_purge,
            commands::safe_box_purge_many,
            commands::safe_box_restore_many,
            commands::rules_check_update,
            commands::rules_apply_update,
            commands::rules_has_backup,
            commands::rules_rollback,
            commands::audit_log_list,
            commands::audit_log_stats,
            commands::cleanup_list_pending
        ])
        .run(tauri::generate_context!())
        .expect("error while running C-Disk Cleaner");
}
