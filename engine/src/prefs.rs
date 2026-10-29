//! 用户偏好存储（task-057 / task-058）。
//!
//! 复用 v1 已有的 `settings(key,value,updated_at)` 表，不加新列、不加迁移。
//! 键名统一带 `prefs.` 前缀，避免与未来其它设置混用。
//!
//! 目前存两项：
//! - `prefs.auto_restart_on_close`（`"1"` / `"0"`）：是否在 RmShutdown 之后
//!   对 bRestartable 进程调 RmRestart。**默认 false**（新用户首次读不到行时
//!   返回 false），符合产品约束「默认关闭」。
//! - `prefs.restart_whitelist`（JSON 数组字符串，如 `["notepad.exe","code*.exe"]`）：
//!   重启白名单。**默认空数组 = 不限制**（task-058）。
//!
//! 所有函数都是同步纯 SQLite 操作，由调用方（Tauri 命令）持有连接。

use rusqlite::Connection;

const KEY_AUTO_RESTART: &str = "prefs.auto_restart_on_close";
const KEY_RESTART_WHITELIST: &str = "prefs.restart_whitelist";
const KEY_THEME: &str = "prefs.theme";
const KEY_AUTO_SCAN_ENABLED: &str = "prefs.auto_scan_enabled";
const KEY_AUTO_SCAN_INTERVAL: &str = "prefs.auto_scan_interval";
/// 上次自动扫描的时间戳（unix 秒）。**不暴露给前端**，只用于「是否到期」判断。
const KEY_LAST_AUTO_SCAN_AT: &str = "prefs.last_auto_scan_at";

// ===================== task-061：自动扫描 =====================
//
// 间隔只有三档。不用「N 小时」自由输入：产品要求是「每天/每周/每月」，
// 自由输入会让「下次什么时候扫」变得难以解释，也不好做 UI。

pub const SCAN_DAILY: &str = "daily";
pub const SCAN_WEEKLY: &str = "weekly";
pub const SCAN_MONTHLY: &str = "monthly";

/// 归一化扫描间隔：只认三档，其余回落 `daily`。
pub fn normalize_scan_interval(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        SCAN_WEEKLY => SCAN_WEEKLY.to_string(),
        SCAN_MONTHLY => SCAN_MONTHLY.to_string(),
        _ => SCAN_DAILY.to_string(),
    }
}

/// 间隔 → 秒数。`monthly` 取 30 天（不按自然月，避免 28/29/31 天的边界差异）。
pub fn interval_secs(interval: &str) -> i64 {
    match normalize_scan_interval(interval).as_str() {
        SCAN_WEEKLY => 7 * 86_400,
        SCAN_MONTHLY => 30 * 86_400,
        _ => 86_400,
    }
}

/// 读「自动扫描」开关。缺行 / 非法值 → false（保守，绝不默认开启后台任务）。
pub fn get_auto_scan_enabled(conn: &Connection) -> bool {
    read_setting(conn, KEY_AUTO_SCAN_ENABLED)
        .map(|v| v == "1")
        .unwrap_or(false)
}

pub fn set_auto_scan_enabled(conn: &Connection, on: bool) -> Result<(), String> {
    write_setting(conn, KEY_AUTO_SCAN_ENABLED, if on { "1" } else { "0" })
}

/// 读扫描间隔。缺行 / 非法值 → `daily`。
pub fn get_auto_scan_interval(conn: &Connection) -> String {
    read_setting(conn, KEY_AUTO_SCAN_INTERVAL)
        .map(|v| normalize_scan_interval(&v))
        .unwrap_or_else(|| SCAN_DAILY.to_string())
}

pub fn set_auto_scan_interval(conn: &Connection, interval: &str) -> Result<(), String> {
    write_setting(
        conn,
        KEY_AUTO_SCAN_INTERVAL,
        &normalize_scan_interval(interval),
    )
}

/// 上次自动扫描时间（unix 秒）。缺行 / 非法值 → 0（视作「从没扫过」→ 立即到期）。
pub fn get_last_auto_scan_at(conn: &Connection) -> i64 {
    read_setting(conn, KEY_LAST_AUTO_SCAN_AT)
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(0)
}

pub fn set_last_auto_scan_at(conn: &Connection, ts: i64) -> Result<(), String> {
    write_setting(conn, KEY_LAST_AUTO_SCAN_AT, &ts.to_string())
}

/// 判断「现在是否该跑自动扫描」。
///
/// 抽成纯函数是为了单测：定时器只要每秒问它一次即可，时间来源由调用方传，
/// 测试不必等真实时间。
///
/// `last_at <= 0`（从没扫过）→ 立刻到期。这是有意的：用户刚开启开关时
/// 应该马上看到效果，而不是等一整天。
pub fn auto_scan_due(enabled: bool, interval: &str, last_at: i64, now: i64) -> bool {
    if !enabled {
        return false;
    }
    if last_at <= 0 {
        return true;
    }
    now - last_at >= interval_secs(interval)
}

/// 读一个字符串设置项（缺行 → None）。
fn read_setting(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        rusqlite::params![key],
        |r| r.get(0),
    )
    .ok()
}

/// UPSERT 一个字符串设置项（updated_at 取当前 Unix 毫秒）。
fn write_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("system time before epoch: {e}"))?
        .as_millis() as i64;
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        rusqlite::params![key, value, now],
    )
    .map(|_| ())
    .map_err(|e| format!("upsert setting {key}: {e}"))
}

/// 主题的合法取值（task-061）。**必须是白名单**：读侧对任何非法值都回落
/// `system`，避免手改数据库或历史脏数据把界面搞成「既不是深也不是浅」。
pub const THEME_SYSTEM: &str = "system";
pub const THEME_LIGHT: &str = "light";
pub const THEME_DARK: &str = "dark";

/// 归一化主题值：只认三个合法值（大小写不敏感、忽略首尾空白），其余回落 `system`。
pub fn normalize_theme(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        THEME_LIGHT => THEME_LIGHT.to_string(),
        THEME_DARK => THEME_DARK.to_string(),
        _ => THEME_SYSTEM.to_string(),
    }
}

/// 读主题偏好。缺行 / 非法值 → `system`（跟随系统，首次启动的默认行为）。
pub fn get_theme(conn: &Connection) -> String {
    let v: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            rusqlite::params![KEY_THEME],
            |r| r.get(0),
        )
        .ok();
    v.map(|s| normalize_theme(&s))
        .unwrap_or_else(|| THEME_SYSTEM.to_string())
}

/// 写主题偏好。非法值会被归一化成 `system` 后落库（不报错——用户界面只可能
/// 传三个值，走到这里是调用方 bug，静默纠正比 500 更友好）。
pub fn set_theme(conn: &Connection, theme: &str) -> Result<(), String> {
    let normalized = normalize_theme(theme);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("system time before epoch: {e}"))?
        .as_millis() as i64;
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        rusqlite::params![KEY_THEME, normalized, now],
    )
    .map(|_| ())
    .map_err(|e| format!("upsert prefs theme: {e}"))
}

/// 读「关闭占用进程后自动重启」开关。
///
/// 缺行 / 值非法 → false（保守，绝不默认开启）。
pub fn get_auto_restart(conn: &Connection) -> bool {
    let v: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            rusqlite::params![KEY_AUTO_RESTART],
            |r| r.get(0),
        )
        .ok();
    matches!(v.as_deref(), Some("1"))
}

/// 写「关闭占用进程后自动重启」开关。
///
/// 用 UPSERT（INSERT OR REPLACE），updated_at 取当前 Unix 毫秒。
pub fn set_auto_restart(conn: &Connection, on: bool) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("system time before epoch: {e}"))?
        .as_millis() as i64;
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        rusqlite::params![KEY_AUTO_RESTART, if on { "1" } else { "0" }, now],
    )
    .map(|_| ())
    .map_err(|e| format!("upsert prefs: {e}"))
}

/// 给前端用的偏好快照（serde camelCase）。
///
/// task-058 起不再是 `Copy`（含 `Vec<String>`），调用方如需复制请显式 `clone()`。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    pub auto_restart_on_close: bool,
    /// 重启白名单（进程映像名，支持 `*` 通配）。空 = 不限制。
    ///
    /// `serde(default)`：旧前端不传该字段时按空数组处理，避免整个 `set_prefs`
    /// 因反序列化失败而 500。**注意**：前端每次 `set_prefs` 都必须带上当前值，
    /// 否则会把用户已配好的白名单覆盖成空。
    #[serde(default)]
    pub restart_whitelist: Vec<String>,
    /// 主题：`system` / `light` / `dark`（task-061）。默认 `system`。
    #[serde(default = "default_theme")]
    pub theme: String,
    /// 是否开启自动扫描（task-061）。默认 false——绝不默认在后台跑任务。
    #[serde(default)]
    pub auto_scan_enabled: bool,
    /// 自动扫描间隔：`daily` / `weekly` / `monthly`。默认 `daily`。
    #[serde(default = "default_scan_interval")]
    pub auto_scan_interval: String,
}

/// `serde(default)` 的默认值：跟随系统。
fn default_theme() -> String {
    THEME_SYSTEM.to_string()
}

fn default_scan_interval() -> String {
    SCAN_DAILY.to_string()
}

impl Prefs {
    pub fn load(conn: &Connection) -> Self {
        Self {
            auto_restart_on_close: get_auto_restart(conn),
            restart_whitelist: get_restart_whitelist(conn),
            theme: get_theme(conn),
            auto_scan_enabled: get_auto_scan_enabled(conn),
            auto_scan_interval: get_auto_scan_interval(conn),
        }
    }
}

// ===================== task-058：重启白名单 =====================

/// 读重启白名单。缺行 / JSON 损坏 / 非字符串数组 → 空 Vec（= 不限制，保守）。
pub fn get_restart_whitelist(conn: &Connection) -> Vec<String> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            rusqlite::params![KEY_RESTART_WHITELIST],
            |r| r.get(0),
        )
        .ok();
    let Some(raw) = raw else { return Vec::new() };

    match serde_json::from_str::<Vec<String>>(&raw) {
        Ok(v) => normalize_whitelist(&v),
        Err(e) => {
            // 脏数据不能让「重启行为」变成不可预期的：直接当成空（不限制）并留日志。
            eprintln!("[prefs] restart_whitelist 解析失败，按空处理: {e}");
            Vec::new()
        }
    }
}

/// 写重启白名单。
///
/// 空列表 → **删除该行**（回到「不限制」的默认态），而不是存 `[]`——
/// 这样「用户从没配过」和「用户清空了」在存储层是同一个状态，读侧逻辑更简单。
pub fn set_restart_whitelist(conn: &Connection, list: &[String]) -> Result<(), String> {
    let normalized = normalize_whitelist(list);

    if normalized.is_empty() {
        conn.execute(
            "DELETE FROM settings WHERE key = ?1",
            rusqlite::params![KEY_RESTART_WHITELIST],
        )
        .map_err(|e| format!("delete prefs restart_whitelist: {e}"))?;
        return Ok(());
    }

    let json = serde_json::to_string(&normalized)
        .map_err(|e| format!("serialize restart_whitelist: {e}"))?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("system time before epoch: {e}"))?
        .as_millis() as i64;
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        rusqlite::params![KEY_RESTART_WHITELIST, json, now],
    )
    .map(|_| ())
    .map_err(|e| format!("upsert prefs restart_whitelist: {e}"))
}

/// 归一化白名单：去首尾空白、丢空行、**统一小写**（Windows 进程名不区分大小写）。
///
/// 不去重——去重对匹配结果无影响，但会让用户看到的顺序和输入不一致；
/// 保留原顺序、保留重复项（`["a","a"]` 与 `["a"]` 语义相同，无副作用）。
fn normalize_whitelist(list: &[String]) -> Vec<String> {
    list.iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

/// `*` 通配匹配（仅支持 `*`，不支持 `?` / 字符类——够用且好推理）。
///
/// 语义：`*` 匹配任意长度（含空）的字符序列，其余字符按字面比较。
/// 例：`notepad.exe` 精确匹配；`*.exe` 匹配任意 exe；`code*` 匹配 `code.exe` / `codeserver.exe`。
pub fn wildcard_match(pattern: &str, name: &str) -> bool {
    // 两侧都按小写比（调用方可能传未归一化的串）。
    let p = pattern.trim().to_lowercase();
    let n = name.trim().to_lowercase();

    if p == "*" {
        return true;
    }
    if !p.contains('*') {
        return p == n;
    }

    let parts: Vec<&str> = p.split('*').collect();
    let mut rest: &str = &n;

    for (idx, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if idx == 0 {
            // 首段必须以它开头（说明 pattern 不以 * 开头）
            if !rest.starts_with(part) {
                return false;
            }
            rest = &rest[part.len()..];
        } else {
            match rest.find(part) {
                Some(pos) => rest = &rest[pos + part.len()..],
                None => return false,
            }
        }
    }

    // pattern 不以 * 结尾时，必须已经吃到字符串末尾
    if !p.ends_with('*') && !rest.is_empty() {
        return false;
    }
    true
}

/// 判断「被关闭的可重启进程」是否**全部**落在白名单内。
///
/// # 为什么是「全部」而不是「任一」
///
/// `RmRestart` **没有按 pid 重启的参数**——一次调用会把本会话内所有
/// `bRestartable==TRUE` 的进程全部拉起（见 `locking.rs` 的注释与 task-057
/// 测试轮修的 flags 误用 bug）。所以「只重启白名单内的那几个」在 RM 机制下
/// 无法实现。唯一安全的做法是 **fail-closed**：
///
/// - 白名单为空 → 不限制（保持 task-057 现状）；
/// - 白名单非空 → 只有被关闭的可重启进程**全部**命中白名单，才调 `RmRestart`；
///   只要有一个不在名单内，就整体跳过——否则一调就会把用户明确排除的进程也拉起来。
///
/// `names` 为空（没有可重启进程）时返回 `false`：调用方本来就会跳过，语义上
/// 「没有东西可重启」不该被当成「允许重启」。
pub fn whitelist_allows_restart(whitelist: &[String], names: &[String]) -> bool {
    let wl = normalize_whitelist(whitelist);
    if wl.is_empty() {
        return true; // 空 = 不限制
    }
    if names.is_empty() {
        return false;
    }
    names.iter().all(|name| {
        let n = name.trim().to_lowercase();
        wl.iter().any(|pat| wildcard_match(pat, &n))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_temp() -> (tempfile::TempDir, Connection) {
        let dir = tempfile::tempdir().unwrap();
        let conn = crate::db::conn::open(dir.path()).unwrap();
        (dir, conn)
    }

    #[test]
    fn default_is_false_when_row_missing() {
        let (_dir, conn) = open_temp();
        assert!(!get_auto_restart(&conn), "缺行必须返回 false");
    }

    #[test]
    fn set_then_get_roundtrip() {
        let (_dir, conn) = open_temp();
        assert!(!get_auto_restart(&conn));
        set_auto_restart(&conn, true).unwrap();
        assert!(get_auto_restart(&conn), "set(true) 后读应得 true");
        set_auto_restart(&conn, false).unwrap();
        assert!(!get_auto_restart(&conn), "set(false) 后读应得 false");
    }

    #[test]
    fn unknown_value_falls_back_to_false() {
        let (_dir, conn) = open_temp();
        // 直接写一个非法值，模拟历史脏数据；必须 fail-closed 到 false。
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES (?1, 'yes', 1)",
            rusqlite::params![KEY_AUTO_RESTART],
        )
        .unwrap();
        assert!(!get_auto_restart(&conn), "非法值必须回退 false");
    }

    #[test]
    fn upsert_does_not_touch_other_keys() {
        let (_dir, conn) = open_temp();
        // 别的 key 预置一个值，set_auto_restart 不能把它冲掉。
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES ('other.key', 'keep', 1)",
            [],
        )
        .unwrap();
        set_auto_restart(&conn, true).unwrap();
        let other: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key='other.key'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(other, "keep", "未知/其它 key 不能被破坏");
    }

    #[test]
    fn prefs_snapshot_roundtrip() {
        let (_dir, conn) = open_temp();
        let p = Prefs::load(&conn);
        assert!(!p.auto_restart_on_close);
        set_auto_restart(&conn, true).unwrap();
        let p2 = Prefs::load(&conn);
        assert!(p2.auto_restart_on_close);
    }

    // ============ task-058：重启白名单 ============

    #[test]
    fn whitelist_defaults_to_empty_when_row_missing() {
        let (_dir, conn) = open_temp();
        assert!(
            get_restart_whitelist(&conn).is_empty(),
            "缺行必须返回空 Vec（= 不限制）"
        );
        assert!(Prefs::load(&conn).restart_whitelist.is_empty());
    }

    #[test]
    fn whitelist_roundtrip_persists_and_normalizes() {
        let (_dir, conn) = open_temp();
        set_restart_whitelist(
            &conn,
            &[" Notepad.EXE ".to_string(), String::new(), "code*".into()],
        )
        .unwrap();

        let got = get_restart_whitelist(&conn);
        assert_eq!(
            got,
            vec!["notepad.exe".to_string(), "code*".to_string()],
            "应去空白、丢空行、统一小写"
        );
        // 再读一次（模拟重开应用）：必须仍在
        assert_eq!(get_restart_whitelist(&conn), got);
    }

    #[test]
    fn whitelist_empty_write_removes_row() {
        let (_dir, conn) = open_temp();
        set_restart_whitelist(&conn, &["a.exe".into()]).unwrap();
        assert_eq!(get_restart_whitelist(&conn).len(), 1);

        // 写空列表 → 删行（回到默认态），不是存 []
        set_restart_whitelist(&conn, &[]).unwrap();
        assert!(get_restart_whitelist(&conn).is_empty());
        let rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key='prefs.restart_whitelist'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rows, 0, "空列表应删除该行");
    }

    #[test]
    fn whitelist_corrupt_json_falls_back_to_empty() {
        let (_dir, conn) = open_temp();
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES ('prefs.restart_whitelist', 'not-json', 1)",
            [],
        )
        .unwrap();
        assert!(
            get_restart_whitelist(&conn).is_empty(),
            "脏数据必须降级为空（不限制），不能崩"
        );
    }

    #[test]
    fn wildcard_match_basics() {
        assert!(wildcard_match("notepad.exe", "notepad.exe"));
        assert!(wildcard_match("NOTEPAD.EXE", "notepad.exe"), "不区分大小写");
        assert!(!wildcard_match("notepad.exe", "notepad2.exe"));

        assert!(wildcard_match("*.exe", "chrome.exe"));
        assert!(wildcard_match("*.exe", "code.exe"));
        assert!(!wildcard_match("*.exe", "chrome.dll"));

        assert!(wildcard_match("code*", "code.exe"));
        assert!(wildcard_match("code*", "codeserver.exe"));
        assert!(!wildcard_match("code*", "vscode.exe"));

        assert!(wildcard_match("*", "anything.exe"), "* 匹配一切");
        assert!(wildcard_match("*note*", "mynotepad.exe"));
        assert!(
            !wildcard_match("note*exe", "notepad.dll"),
            "尾部字面量不匹配"
        );
    }

    #[test]
    fn whitelist_allows_restart_three_states() {
        let names = vec!["notepad.exe".to_string()];

        // ① 空白名单 = 不限制 → 允许
        assert!(whitelist_allows_restart(&[], &names));

        // ② 白名单命中 → 允许
        assert!(whitelist_allows_restart(
            &["notepad.exe".to_string()],
            &names
        ));
        assert!(whitelist_allows_restart(&["*.exe".to_string()], &names));

        // ③ 白名单不命中 → 不允许（fail-closed，绝不连带重启名单外的）
        assert!(!whitelist_allows_restart(
            &["chrome.exe".to_string()],
            &names
        ));
    }

    #[test]
    fn whitelist_requires_all_names_to_match() {
        // RmRestart 是全有或全无：只要有一个不在名单内就必须整体跳过。
        let names = vec!["notepad.exe".to_string(), "chrome.exe".to_string()];
        assert!(!whitelist_allows_restart(
            &["notepad.exe".to_string()],
            &names
        ));
        assert!(whitelist_allows_restart(
            &["notepad.exe".to_string(), "chrome.exe".to_string()],
            &names
        ));
        assert!(whitelist_allows_restart(&["*".to_string()], &names));
    }

    #[test]
    fn whitelist_rejects_empty_name_list() {
        // 没有可重启进程时不该被当成「允许」——调用方本就会跳过。
        assert!(!whitelist_allows_restart(&["a.exe".to_string()], &[]));
    }

    // ============ task-061：主题 ============

    #[test]
    fn theme_defaults_to_system_when_row_missing() {
        let (_dir, conn) = open_temp();
        assert_eq!(get_theme(&conn), "system", "缺行必须默认跟随系统");
        assert_eq!(Prefs::load(&conn).theme, "system");
    }

    #[test]
    fn theme_roundtrip_persists() {
        let (_dir, conn) = open_temp();
        set_theme(&conn, "dark").unwrap();
        assert_eq!(get_theme(&conn), "dark");
        // 再读一次（模拟重开应用）
        assert_eq!(get_theme(&conn), "dark", "重开应用后主题必须保留");
        set_theme(&conn, "light").unwrap();
        assert_eq!(get_theme(&conn), "light");
        set_theme(&conn, "system").unwrap();
        assert_eq!(get_theme(&conn), "system");
    }

    #[test]
    fn theme_normalizes_illegal_values_to_system() {
        let (_dir, conn) = open_temp();
        // 大小写 / 空白都要能归一化
        assert_eq!(normalize_theme("  DARK "), "dark");
        assert_eq!(normalize_theme("Light"), "light");
        // 非法值一律回落 system（不能把界面搞成「既不是深也不是浅」）
        assert_eq!(normalize_theme("sepia"), "system");
        assert_eq!(normalize_theme(""), "system");

        // 直接写脏数据，读侧也要兜住
        conn.execute(
            "INSERT INTO settings (key, value, updated_at) VALUES ('prefs.theme', 'sepia', 1)",
            [],
        )
        .unwrap();
        assert_eq!(get_theme(&conn), "system", "脏数据必须回落 system");
    }

    #[test]
    fn set_theme_writes_normalized_value() {
        let (_dir, conn) = open_temp();
        set_theme(&conn, "  DARK  ").unwrap();
        let raw: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key='prefs.theme'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(raw, "dark", "落库的应是归一化后的值");
    }

    // ============ task-061：自动扫描 ============

    #[test]
    fn auto_scan_defaults_off_and_daily() {
        let (_dir, conn) = open_temp();
        assert!(!get_auto_scan_enabled(&conn), "缺行必须默认关闭");
        assert_eq!(get_auto_scan_interval(&conn), "daily", "缺行默认 daily");
        let p = Prefs::load(&conn);
        assert!(!p.auto_scan_enabled);
        assert_eq!(p.auto_scan_interval, "daily");
    }

    #[test]
    fn auto_scan_roundtrip_persists() {
        let (_dir, conn) = open_temp();
        set_auto_scan_enabled(&conn, true).unwrap();
        set_auto_scan_interval(&conn, "weekly").unwrap();
        assert!(get_auto_scan_enabled(&conn));
        assert_eq!(get_auto_scan_interval(&conn), "weekly");
        // 重开应用（再读一次）
        assert!(get_auto_scan_enabled(&conn));
        assert_eq!(get_auto_scan_interval(&conn), "weekly");
    }

    #[test]
    fn auto_scan_interval_normalizes_illegal_to_daily() {
        let (_dir, conn) = open_temp();
        assert_eq!(normalize_scan_interval("  WEEKLY "), "weekly");
        assert_eq!(normalize_scan_interval("monthly"), "monthly");
        assert_eq!(
            normalize_scan_interval("hourly"),
            "daily",
            "非法值回落 daily"
        );
        assert_eq!(normalize_scan_interval(""), "daily");

        conn.execute(
            "INSERT INTO settings (key, value, updated_at)
             VALUES ('prefs.auto_scan_interval', 'every-minute', 1)",
            [],
        )
        .unwrap();
        assert_eq!(
            get_auto_scan_interval(&conn),
            "daily",
            "脏数据必须回落 daily"
        );
    }

    #[test]
    fn interval_secs_maps_three_tiers() {
        assert_eq!(interval_secs("daily"), 86_400);
        assert_eq!(interval_secs("weekly"), 7 * 86_400);
        assert_eq!(interval_secs("monthly"), 30 * 86_400);
        // 非法值按 daily
        assert_eq!(interval_secs("bogus"), 86_400);
    }

    #[test]
    fn auto_scan_due_three_states() {
        let now = 1_000_000_000_i64;

        // 关闭 → 永不到期
        assert!(!auto_scan_due(false, "daily", 0, now));

        // 开启 + 从没扫过 → 立刻到期（用户刚开开关就该看到效果）
        assert!(auto_scan_due(true, "daily", 0, now));

        // 开启 + 上次扫描距今不足间隔 → 不到期
        assert!(!auto_scan_due(true, "daily", now - 3_600, now));

        // 开启 + 距上次已超间隔 → 到期
        assert!(auto_scan_due(true, "daily", now - 86_400, now));
        assert!(auto_scan_due(true, "weekly", now - 7 * 86_400, now));
        assert!(!auto_scan_due(true, "weekly", now - 6 * 86_400, now));
    }

    #[test]
    fn last_auto_scan_at_defaults_to_zero_and_roundtrips() {
        let (_dir, conn) = open_temp();
        assert_eq!(get_last_auto_scan_at(&conn), 0, "缺行 = 从没扫过");
        set_last_auto_scan_at(&conn, 1_234_567).unwrap();
        assert_eq!(get_last_auto_scan_at(&conn), 1_234_567);
    }
}
