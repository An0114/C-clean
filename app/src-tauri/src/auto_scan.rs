//! task-061：后台自动扫描（**只扫描 + 通知，绝不自动删除**）。
//!
//! # 为什么只扫描不删除
//!
//! 自动删除是「用户不在场时执行不可逆操作」，与产品定位（每个删除决定都有依据、
//! 每次清理都能追溯）直接冲突。所以自动化的边界划在「帮你看一眼」，真正的清理
//! 仍然要用户手动确认。
//!
//! # 为什么用轮询而不是精确定时
//!
//! 每 [`TICK`] 看一次「是否到期」。看起来笨，但：
//! - 用户随时可能改间隔 / 开关，精确 `sleep(到下次扫描)` 会在改动后错位；
//! - 用户改系统时间、机器休眠唤醒后，精确定时会漂移；
//! - 一次轮询成本就是一条 SQLite 查询，可以忽略。
//!
//! # 与手动扫描的互斥
//!
//! 复用同一个 `ProgressState`（内部是 Mutex）。后台扫描用 **try_lock**：
//! 拿不到说明用户正在手动扫描，本轮直接跳过——不能阻塞用户操作，也不能并发写
//! `scan_session`。

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

/// 轮询间隔：30 分钟。最小间隔是 daily，30 分钟精度足够。
const TICK: Duration = Duration::from_secs(30 * 60);

/// 启动后先等 60s：避开应用启动时的「检查更新 + 首屏加载」高峰，
/// 也避免刚开机就抢 CPU。
const INITIAL_DELAY: Duration = Duration::from_secs(60);

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 启动后台线程。**只调一次**（在 setup 里）。
pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(INITIAL_DELAY);
        loop {
            match tick_once(&app) {
                Ok(Some(msg)) => eprintln!("[auto-scan] {msg}"),
                Ok(None) => {}
                Err(e) => eprintln!("[auto-scan] tick failed: {e}"),
            }
            std::thread::sleep(TICK);
        }
    });
}

/// 一次检查。返回 `Some(说明)` 表示本轮做了什么（用于日志），`None` = 没到期/跳过。
fn tick_once(app: &AppHandle) -> Result<Option<String>, String> {
    let paths = app.state::<crate::paths::AppPaths>();

    // 1. 读偏好 + 判断是否到期（用独立连接，读完即释放，不占着锁）。
    let (enabled, interval, last, now) = {
        let conn = engine::db::conn::open(&paths.data_root).map_err(|e| e.to_string())?;
        (
            engine::prefs::get_auto_scan_enabled(&conn),
            engine::prefs::get_auto_scan_interval(&conn),
            engine::prefs::get_last_auto_scan_at(&conn),
            unix_now(),
        )
    };

    if !engine::prefs::auto_scan_due(enabled, &interval, last, now) {
        return Ok(None);
    }

    // 2. 与手动扫描互斥：拿不到锁说明用户正在扫，本轮跳过（不排队、不阻塞）。
    let progress = app.state::<crate::progress::ProgressState>();
    let Ok(emitter_guard) = progress.0.try_lock() else {
        return Ok(Some("用户正在手动扫描，跳过本轮自动扫描".into()));
    };

    let conn = engine::db::conn::open(&paths.data_root).map_err(|e| e.to_string())?;

    // 3. 建 session（scope 记成 `auto`，历史页能看出这是后台扫的）。
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (unixepoch(), 'running', 'auto')",
        [],
    )
    .map_err(|e| format!("insert auto scan_session: {e}"))?;
    let session_id = conn.last_insert_rowid();

    // 4. 跑扫描（与 scan_start 完全同一条编排链路，只是不 emit 给前端也行——
    //    emit 到 `scan://progress` 时如果用户不在仪表盘页，没人听，无害）。
    let orch = engine::scan::ScanOrchestrator::new(session_id, &conn, &*emitter_guard);
    let outcome = orch.run_all(&paths.scripts_dir);
    drop(emitter_guard);

    let status = if outcome.is_ok() {
        "completed"
    } else {
        "failed"
    };
    let _ = conn.execute(
        "UPDATE scan_session SET status = ?1, finished_at = unixepoch() WHERE id = ?2",
        (status, &session_id),
    );

    // 5. 统计本次结果（只读，不删任何东西）。
    let (items, bytes): (i64, i64) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(size_bytes), 0)
             FROM cleanup_item WHERE session_id = ?1",
            [session_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((0, 0));

    // 6. 无论成败都记「上次自动扫描时间」，避免失败时每 30 分钟重试轰炸。
    {
        let conn2 = engine::db::conn::open(&paths.data_root).map_err(|e| e.to_string())?;
        let _ = engine::prefs::set_last_auto_scan_at(&conn2, unix_now());
    }

    // 7. 通知（失败不影响扫描结果）。
    let body = if outcome.is_err() {
        "自动扫描失败，请打开应用查看详情".to_string()
    } else if items == 0 {
        "自动扫描完成：没发现可清理项".to_string()
    } else {
        format!(
            "自动扫描完成：发现 {} 项可清理，约 {}（仅扫描，未删除）",
            items,
            human_bytes(bytes)
        )
    };
    let _ = notify(app, &body);

    Ok(Some(format!(
        "完成 session={session_id} status={status} items={items} bytes={bytes}"
    )))
}

/// 发系统通知。用官方 notification 插件（dev / 安装态都可用）。
fn notify(app: &AppHandle, body: &str) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;
    app.notification()
        .builder()
        .title("C-Disk Cleaner 自动扫描")
        .body(body)
        .show()
        .map_err(|e| format!("show notification: {e}"))
}

/// 字节 → 人类可读（与前端 `formatBytes` 的口径保持一致：1024 进制、1 位小数）。
fn human_bytes(bytes: i64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes.max(0) as f64;
    if b >= GB {
        format!("{:.1} GB", b / GB)
    } else if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        // 注意用 clamp 后的值：负数（不应出现）不能打印成 "-1 B"。
        format!("{} B", bytes.max(0))
    }
}

/// 让前端知道「后台扫描完成」（仪表盘可据此刷新统计与列表）。
#[allow(dead_code)]
fn emit_done(app: &AppHandle, items: i64) {
    let _ = app.emit("auto-scan://done", serde_json::json!({ "items": items }));
}

#[cfg(test)]
mod tests {
    use super::human_bytes;

    #[test]
    fn human_bytes_units() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1024), "1.0 KB");
        assert_eq!(human_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(human_bytes(3 * 1024 * 1024 * 1024), "3.0 GB");
        // 负数兜底（不应出现，但不能 panic）
        assert_eq!(human_bytes(-1), "0 B");
    }
}
