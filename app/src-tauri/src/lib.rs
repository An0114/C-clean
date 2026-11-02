//! Tauri 主进程壳（M0）。
//!
//! 注册命令：ping（task-001）、scan_start / cleanup_dry_run（task-004）。
//! sessionId 计数器用简单的 `Mutex<i64>` State 注入（task-004）。
//! debt-cleanup-003：AppPaths 在 setup 内构造并 manage，所有命令从 State 取路径。

mod auto_scan;
mod commands;
pub mod paths;
pub mod progress;

use tauri::Manager;

/// 空命令：返回 engine 占位字符串，供前端联调时验证 IPC 通路。
#[tauri::command]
fn ping() -> &'static str {
    engine::hello_engine()
}

/// task-073：静默扫描模式的启动开关。
///
/// **必须与 `engine::ipc::commands::auto_scan::AUTO_SCAN_FLAG` 一致**——
/// 计划任务注册的就是这个字符串。改一边忘另一边，任务会拉起一个完整窗口，
/// 变成「每天早上 9 点弹个窗」这种灾难（一致性由
/// `engine/tests/auto_scan_test.rs` 的静态检查兜底）。
const AUTO_SCAN_FLAG: &str = "--auto-scan";

/// Tauri 应用入口：构造默认 Builder 并挂上命令，随后阻塞运行窗口。
pub fn run() {
    // 由 Windows 任务计划拉起时带 `--auto-scan`：不建窗口、不起轮询、不建托盘，
    // 跑一次扫描（只扫不删）后自行退出。
    let auto_scan_mode = std::env::args().any(|a| a == AUTO_SCAN_FLAG);

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        // bugfix/task-035-updater-acl：为前端 relaunch() 提供 process 插件后端。
        .plugin(tauri_plugin_process::init())
        // task-061：自动扫描完成后发系统通知（Windows 通知中心）。
        .plugin(tauri_plugin_notification::init())
        // task-062：开机自启（写 HKCU\...\Run）。用插件而不是手写注册表，
        // 卸载时它会自己清理条目，不会留残渣。
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(std::sync::Mutex::new(0_i64))
        // task-007：在 setup 闭包里拿 AppHandle，构造全局进度 State 并注入。
        // debt-cleanup-003：构造 AppPaths（data_root/rules_dir/scripts_dir/box_dir）并注入。
        .setup(move |app| {
            let paths = paths::AppPaths::from_app(app.handle())?;
            app.manage(paths);
            app.manage(progress::ProgressState::new(app.handle().clone()));
            // task-063：扫描取消标志（scan_cancel 置位，scan_start 每轮复位）。
            app.manage(commands::ScanCancelState::default());

            if auto_scan_mode {
                // ===== 计划任务模式 =====
                // 只做「扫一遍 + 通知 + 退出」。**不建托盘**（否则托盘图标会闪
                // 一下又消失）、**不起 30 分钟轮询**（进程马上就退出了）。
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    let code = match auto_scan::run_once_for_task(&handle) {
                        Ok(msg) => {
                            eprintln!("[auto-scan-task] {msg}");
                            0
                        }
                        Err(e) => {
                            eprintln!("[auto-scan-task] 失败: {e}");
                            1
                        }
                    };
                    // 给系统通知一点投递时间再退出：通知是异步的，
                    // 进程立刻死掉的话 Windows 可能收不到（表现为「扫了但没提醒」）。
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    handle.exit(code);
                });
                // 窗口保持 `visible: false`（tauri.conf.json），全程不显示。
                return Ok(());
            }

            // ===== 正常启动 =====
            // task-061：起后台自动扫描线程（默认关闭，读 prefs 决定是否真的扫）。
            // 只在这里 spawn 一次；线程内部轮询，不做精确定时（理由见 auto_scan.rs）。
            auto_scan::spawn(app.handle().clone());
            // task-062：系统托盘（关闭窗口时最小化到托盘，而不是退出）。
            if let Err(e) = setup_tray(app.handle()) {
                // 托盘创建失败不该让整个应用起不来（例如无图标资源），
                // 降级为「关闭窗口即退出」——但要在日志里说清楚。
                eprintln!("[tray] 创建托盘失败，将退化为「关闭即退出」: {e}");
            }
            // task-064：保险箱 7 天自动 purge（启动跑一次 + 之后每 6 小时）。
            spawn_safe_box_purge(app.handle().clone());

            // task-073：窗口在 `tauri.conf.json` 里配成 `visible: false`，
            // 由这里显式显示——目的是让 `--auto-scan` 模式可以「从头到尾不闪窗」。
            //
            // **这一步失败不能让应用变成「看不见但还在跑」**：托盘已经建好了，
            // 用户仍能从托盘菜单「打开主窗口」把窗口叫回来，所以只记日志。
            if let Some(w) = app.get_webview_window("main") {
                if let Err(e) = w.show() {
                    eprintln!("[window] 显示主窗口失败（可从托盘打开）: {e}");
                }
            } else {
                eprintln!("[window] 找不到 main 窗口，可能配置文件被改动");
            }
            Ok(())
        })
        // task-062：拦截关闭 → 隐藏到托盘。
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // prevent_close：拦掉「销毁窗口」的默认行为。
                api.prevent_close();
                let _ = window.hide();
                eprintln!("[tray] 窗口已隐藏到托盘（进程仍在运行）");
            }
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            commands::scan_start,
            commands::scan_cancel,
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
            commands::cleanup_stats_trend,
            commands::cleanup_list_pending,
            commands::report_export,
            commands::drive_usage,
            commands::get_prefs,
            commands::set_prefs,
            // task-062：托盘 / 自启
            commands::quit_app,
            commands::get_autostart_enabled,
            commands::set_autostart_enabled,
            // task-065：WinSxS 组件存储（C 模块）
            commands::winsxs_analyze,
            commands::winsxs_cleanup,
            // task-066：大文件地图（E 模块，只读）
            commands::big_files_scan,
            // task-067：残留增强（D 模块）
            commands::residual_scan,
            commands::residual_delete,
            // task-068：注册表健康中心（F 模块）
            commands::registry_health_scan,
            commands::registry_health_delete,
            // task-073：系统级定时扫描（任务计划）
            commands::auto_scan_enable,
            commands::auto_scan_disable,
            commands::auto_scan_status,
            // task-072：清理方案模板（P1-7）
            commands::plan_save,
            commands::plan_list,
            commands::plan_delete,
            commands::plan_apply,
            commands::plan_export,
            commands::plan_import
        ])
        .run(tauri::generate_context!())
        .expect("error while running C-Disk Cleaner");
}

// ===================== task-062：系统托盘 =====================
//
// 为什么用内置托盘而不是插件：Tauri 2 **没有** `tauri-plugin-tray` 这个插件
// （任务书写的那个不存在），托盘是内置能力，只是默认不开 `tray-icon` feature。
//
// 菜单三项：立即扫描 / 打开主窗口 / 退出。
// 「退出」不在 Rust 侧直接 `app.exit(0)`——PRD 要求二次确认，而托盘菜单回调里
// 拿不到 WebView 的 `confirm`。所以走「显示主窗口 + emit `tray://quit-request`」，
// 由前端弹 confirm，用户确认后调 `quit_app` 命令真正退出。

fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;
    use tauri::Emitter;

    let scan = MenuItem::with_id(app, "scan", "立即扫描", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "打开主窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&scan, &show, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("C-Disk Cleaner")
        .menu(&menu)
        // 左键点图标直接显示窗口（与右键出菜单区分，符合 Windows 习惯）。
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "scan" => {
                show_main_window(app);
                // 让前端切到仪表盘并开始扫描（前端监听这个事件）。
                let _ = app.emit("tray://scan", ());
            }
            "show" => show_main_window(app),
            "quit" => {
                // 先显示窗口，让 confirm 有宿主；再请前端确认。
                show_main_window(app);
                let _ = app.emit("tray://quit-request", ());
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // 左键单击托盘图标 → 显示窗口。
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    // 图标：用窗口默认图标。取不到就不设（托盘仍会出现，只是用系统占位图标）。
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

/// 显示并聚焦主窗口（托盘菜单 / 图标点击都走这里）。
fn show_main_window(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

// ===================== task-064：保险箱 7 天自动 purge =====================

/// 每 6 小时检查一次过期条目。
///
/// **为什么不是每小时**：过期判定粒度是「7 天」，6 小时精度绰绰有余；
/// 每小时跑一次只是白耗 IO（要扫 safe_box 表 + 可能删文件）。
///
/// **为什么不在前端触发**：purge 必须**用户不打开应用时也能发生**——否则
/// 「7 天后自动删除」变成「7 天后第一次打开应用时删除」，与 PRD 不符。
/// 但 Tauri 进程不常驻（用户关了窗口只是隐藏到托盘，退出后进程就没了），
/// 所以严格意义上「到点必删」需要计划任务——**本期按「应用运行期间定期检查 +
/// 启动时补一次」实现**，已记入遗留问题。
fn spawn_safe_box_purge(app: tauri::AppHandle) {
    use tauri::Manager;

    std::thread::spawn(move || loop {
        let paths = app.state::<paths::AppPaths>();
        match engine::db::conn::open(&paths.data_root) {
            Ok(conn) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                let r = engine::execute::safe_box::purge_expired(&conn, &paths.box_dir, now);
                if !r.is_noop() {
                    eprintln!(
                        "[auto-purge] 已永久删除 {} 条（失败 {}，释放 {} 字节）",
                        r.purged, r.failed, r.bytes_freed
                    );
                    for e in &r.errors {
                        eprintln!("[auto-purge] {e}");
                    }
                }
            }
            Err(e) => eprintln!("[auto-purge] open db: {e}"),
        }
        std::thread::sleep(std::time::Duration::from_secs(6 * 3600));
    });
}
