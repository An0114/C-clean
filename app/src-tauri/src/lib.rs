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

            // task-083：启用软件感知扫描（进程级开关）。
            //
            // 为什么放在应用层而不是让 engine 默认开启：engine 的编排集成测试用
            // 隔离的临时 DB + 假脚本，无条件枚举真实注册表会把上百条本机缓存
            // 条目灌进去，把那批断言全部冲掉。由应用进程显式启用，后台自动
            // 扫描（`auto_scan::spawn` 在同一进程）也天然一起生效。
            // 置位在 `--auto-scan` 分支之前，两种启动模式行为一致。
            std::env::set_var("CCLEAN_SOFTWARE_AWARE", "1");

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
            // task-077：多盘扫描——固定磁盘枚举（Dashboard 多环形图数据源）
            commands::drive_list,
            // task-083：已安装软件清单（Items 页软件列的数据来源）
            commands::software_inventory,
            // task-078：用户自定义规则 CRUD
            commands::user_rules_list,
            commands::user_rules_add,
            commands::user_rules_update,
            commands::user_rules_set_enabled,
            commands::user_rules_delete,
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
            // task-084 A2/A3：大文件地图扩展（重复文件 / 文件夹分布，只读）
            commands::big_files_duplicates,
            commands::big_files_folder_stats,
            // task-067：残留增强（D 模块）
            commands::residual_scan,
            commands::residual_delete,
            // task-084 A4：残留一键定位（explorer /select）
            commands::residual_locate,
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

// ===================== task-062：系统托盘 / task-082：菜单完善 =====================
//
// 为什么用内置托盘而不是插件：Tauri 2 **没有** `tauri-plugin-tray` 这个插件
// （任务书写的那个不存在），托盘是内置能力，只是默认不开 `tray-icon` feature。
//
// 菜单五项：立即扫描 / 打开主窗口 / 暂停自动扫描 / 恢复自动扫描 / 退出。
// 「退出」不在 Rust 侧直接 `app.exit(0)`——PRD 要求二次确认，而托盘菜单回调里
// 拿不到 WebView 的 `confirm`。所以走「显示主窗口 + emit `tray://quit-request`」，
// 由前端弹 confirm，用户确认后调 `quit_app` 命令真正退出。
//
// # task-082 的三处改动
//
// ① **暂停/恢复自动扫描**：直接读写 `settings` 表的 `auto_scan_enabled`。
//    后台线程每轮 tick 都重新读它（见 `auto_scan.rs::tick_once`），所以改完
//    下一轮就生效——不需要通知线程、不需要重启、也不需要加新的 State。
// ② **左键单击改为 toggle**：原来只「显示」，现在「可见且聚焦时隐藏」。
// ③ **tooltip 显示状态**：启动时给一次，之后由后台线程每 60 秒刷新。

/// 托盘菜单项 id。
///
/// **抽成常量而不是散在字符串字面量里**：`setup_tray` 需要真实 `AppHandle`，
/// 单测里造不出来；但「菜单恰好五项、id 不重复、必需项都在」这类约束可以脱离
/// `AppHandle` 验证。id 只在这里定义一次，菜单创建与事件匹配都引用它，
/// 不会出现「加了菜单项却忘了加分支」或「两边 id 拼写不一致」。
const TRAY_ID_SCAN: &str = "scan";
const TRAY_ID_SHOW: &str = "show";
const TRAY_ID_PAUSE_AUTO: &str = "pause-auto";
const TRAY_ID_RESUME_AUTO: &str = "resume-auto";
const TRAY_ID_QUIT: &str = "quit";

/// 菜单顺序（也是单测里断言的集合）。
///
/// 运行时并不遍历它（每项文本、可用性都不同，只能逐个建），所以非测试构建下
/// 它是「未被引用」的——`cfg_attr` 只对非测试构建压掉这个警告，不改变语义。
#[cfg_attr(not(test), allow(dead_code))]
const TRAY_MENU_IDS: [&str; 5] = [
    TRAY_ID_SCAN,
    TRAY_ID_SHOW,
    TRAY_ID_PAUSE_AUTO,
    TRAY_ID_RESUME_AUTO,
    TRAY_ID_QUIT,
];

fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;
    use tauri::Emitter;

    let auto_on = read_auto_scan_enabled(app);

    let scan = MenuItem::with_id(app, TRAY_ID_SCAN, "立即扫描", true, None::<&str>)?;
    let show = MenuItem::with_id(app, TRAY_ID_SHOW, "打开主窗口", true, None::<&str>)?;
    // 两项按当前开关互斥可用：已经是「开」就没必要再点「恢复」。
    let pause = MenuItem::with_id(
        app,
        TRAY_ID_PAUSE_AUTO,
        "暂停自动扫描",
        auto_on,
        None::<&str>,
    )?;
    let resume = MenuItem::with_id(
        app,
        TRAY_ID_RESUME_AUTO,
        "恢复自动扫描",
        !auto_on,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, TRAY_ID_QUIT, "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&scan, &show, &pause, &resume, &quit])?;

    // 菜单回调是 `Fn`，要改两个菜单项的可用状态，得先把句柄 clone 进去。
    let pause_h = pause.clone();
    let resume_h = resume.clone();

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip(tray_tooltip_text(app))
        .menu(&menu)
        // 左键不弹菜单，留给 toggle 窗口（与 Windows 托盘习惯一致）。
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            TRAY_ID_SCAN => {
                show_main_window(app);
                // 让前端切到仪表盘并开始扫描（前端监听这个事件）。
                let _ = app.emit("tray://scan", ());
            }
            TRAY_ID_SHOW => show_main_window(app),
            TRAY_ID_PAUSE_AUTO => match set_auto_scan_enabled(app, false) {
                Ok(()) => {
                    let _ = pause_h.set_enabled(false);
                    let _ = resume_h.set_enabled(true);
                    refresh_tray_tooltip(app);
                }
                // 写失败只记日志：后台线程读的是 DB，这里改的只是菜单可用状态，
                // 失败时两个菜单项保持原样，用户再点一次即可。
                Err(e) => eprintln!("[tray] 暂停自动扫描失败: {e}"),
            },
            TRAY_ID_RESUME_AUTO => match set_auto_scan_enabled(app, true) {
                Ok(()) => {
                    let _ = pause_h.set_enabled(true);
                    let _ = resume_h.set_enabled(false);
                    refresh_tray_tooltip(app);
                }
                Err(e) => eprintln!("[tray] 恢复自动扫描失败: {e}"),
            },
            TRAY_ID_QUIT => {
                // 先显示窗口，让 confirm 有宿主；再请前端确认。
                show_main_window(app);
                let _ = app.emit("tray://quit-request", ());
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // 左键单击 → 显示/隐藏切换。
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
            }
        });

    // 图标：用窗口默认图标。取不到就不设（托盘仍会出现，只是用系统占位图标）。
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;

    // task-082：tooltip 状态刷新线程（见该函数注释）。
    spawn_tray_tooltip_refresh(app.clone());
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

/// task-082：左键单击托盘图标的「显示 / 隐藏」切换。
///
/// 判据是「**可见且聚焦**」才算「用户正在看它」。只判可见的话，窗口被别的
/// 程序挡在后面时点托盘会变成「隐藏」——用户会觉得点了没反应。
fn toggle_main_window(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("main") {
        let visible = w.is_visible().unwrap_or(false);
        let focused = w.is_focused().unwrap_or(false);
        if visible && focused {
            let _ = w.hide();
        } else {
            show_main_window(app);
        }
    }
}

/// 读 `auto_scan_enabled`。
///
/// 读失败按「关」处理：宁可让用户点一次「恢复自动扫描」，也不要显示成「已开启」
/// 却其实没在扫——后者会让人以为后台在干活，实际什么都没发生。
fn read_auto_scan_enabled(app: &tauri::AppHandle) -> bool {
    use tauri::Manager;
    let paths = app.state::<crate::paths::AppPaths>();
    match engine::db::conn::open(&paths.data_root) {
        Ok(conn) => engine::prefs::get_auto_scan_enabled(&conn),
        Err(e) => {
            eprintln!("[tray] 读自动扫描开关失败，按「关闭」处理: {e}");
            false
        }
    }
}

/// 写 `auto_scan_enabled`。后台线程下一轮 tick 就会读到新值，无需额外通知。
fn set_auto_scan_enabled(app: &tauri::AppHandle, on: bool) -> Result<(), String> {
    use tauri::Manager;
    let paths = app.state::<crate::paths::AppPaths>();
    let conn = engine::db::conn::open(&paths.data_root).map_err(|e| e.to_string())?;
    engine::prefs::set_auto_scan_enabled(&conn, on)
}

/// tooltip 里显示的产品名。
const TRAY_TOOLTIP_NAME: &str = "C-Disk Cleaner";

/// 拼 tooltip 文案。**纯函数**，不碰 DB / AppHandle——这样单测能直接验格式。
///
/// 字节格式化复用 `auto_scan::human_bytes`：同一个应用里两个「1024 字节怎么显示」
/// 的答案必须一致，各写一份迟早会分叉。
fn tooltip_line(pending: i64, freed_bytes: i64) -> String {
    format!(
        "{TRAY_TOOLTIP_NAME} · 待清理 {pending} 项 · 累计释放 {}",
        auto_scan::human_bytes(freed_bytes)
    )
}

/// 托盘 tooltip 文案。
///
/// 查不到统计就只显示名字——tooltip 是锦上添花，绝不能因为它失败而影响托盘可用。
fn tray_tooltip_text(app: &tauri::AppHandle) -> String {
    use tauri::Manager;
    let paths = app.state::<crate::paths::AppPaths>();
    let Ok(conn) = engine::db::conn::open(&paths.data_root) else {
        return TRAY_TOOLTIP_NAME.to_string();
    };
    let pending: i64 = conn
        .query_row("SELECT count(*) FROM cleanup_item", [], |r| r.get(0))
        .unwrap_or(0);
    let freed = engine::audit_log::cleanup_stats(&conn, None)
        .map(|s| s.total_bytes_freed)
        .unwrap_or(0);
    tooltip_line(pending, freed)
}

/// 立即刷一次 tooltip（菜单切换开关后调用，让状态马上反映出来）。
fn refresh_tray_tooltip(app: &tauri::AppHandle) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(&tray_tooltip_text(app)));
    }
}

/// task-082：每 60 秒刷新一次 tooltip。
///
/// **为什么是轮询而不是事件驱动**：扫描 / 清理的完成点在 `commands.rs` 与
/// `auto_scan.rs` 里，那两个文件不在本任务的白名单内。60 秒的代价是两次只读查询
/// （一次 count + 一次聚合），SQLite 上不到 1ms；换来的是「不改白名单外文件
/// 也能让托盘状态是新的」。
fn spawn_tray_tooltip_refresh(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
        refresh_tray_tooltip(&app);
    });
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

#[cfg(test)]
mod tests {
    use super::*;

    // ===== task-082：托盘菜单结构 =====
    //
    // `setup_tray` 本身要真实 `AppHandle`（窗口系统 + 托盘），单测里造不出来。
    // 但「菜单有几项、id 有没有重复、必需项在不在」这些是纯数据约束，
    // 用 `TRAY_MENU_IDS` 就能钉住——加菜单项时如果忘了同步这个常量，
    // 或者手滑把两个 id 写成一样，这里会立刻红。

    #[test]
    fn tray_menu_has_exactly_five_items() {
        // 五项：立即扫描 / 打开主窗口 / 暂停自动扫描 / 恢复自动扫描 / 退出
        assert_eq!(
            TRAY_MENU_IDS.len(),
            5,
            "托盘菜单应为 5 项，实际 {:?}",
            TRAY_MENU_IDS
        );
    }

    #[test]
    fn tray_menu_ids_are_unique() {
        // id 重复会让 `on_menu_event` 的 match 永远命中第一个分支，
        // 表现为「点了某项却执行了另一项」——最难查的那种。
        let mut seen = std::collections::HashSet::new();
        for id in TRAY_MENU_IDS {
            assert!(seen.insert(id), "托盘菜单 id 重复: {id}");
        }
    }

    #[test]
    fn tray_menu_contains_required_ids() {
        for want in ["scan", "show", "pause-auto", "resume-auto", "quit"] {
            assert!(
                TRAY_MENU_IDS.contains(&want),
                "托盘菜单缺少必需项 {want}，实际 {:?}",
                TRAY_MENU_IDS
            );
        }
    }

    // ===== task-082：tooltip 文案 =====

    #[test]
    fn tooltip_shows_pending_and_freed() {
        // 格式是三段：产品名 · 待清理 N 项 · 累计释放 X。
        // 段之间用「 · 」分隔——tooltip 是单行文本，没有换行可用。
        assert_eq!(
            tooltip_line(12, 3 * 1024 * 1024),
            "C-Disk Cleaner · 待清理 12 项 · 累计释放 3.0 MB"
        );
    }

    #[test]
    fn tooltip_handles_empty_state() {
        // 刚装完：0 项、0 字节。不能显示成「累计释放 -1 B」或空字符串。
        assert_eq!(
            tooltip_line(0, 0),
            "C-Disk Cleaner · 待清理 0 项 · 累计释放 0 B"
        );
    }

    #[test]
    fn tooltip_uses_gb_for_large_numbers() {
        // 累计释放很容易上 GB，tooltip 空间有限，不该显示成 3221225472 B。
        assert_eq!(
            tooltip_line(1, 3 * 1024 * 1024 * 1024),
            "C-Disk Cleaner · 待清理 1 项 · 累计释放 3.0 GB"
        );
    }
}
