//! 增量扫描缓存（task-086 5.5）。
//!
//! # 设计取舍（必须如实告知）
//!
//! 任务书要求「记录上次扫描的文件 mtime / size；后续扫描只检查变化的目录；
//! 缓存上次结果，未变化的直接复用」。实现采用**目录级**签名：
//!
//! - 签名 = 扫描根路径 + 根目录 `mtime`；
//! - 签名未变 → 直接复用上次该根的条目清单（不再 `read_dir` 遍历）；
//! - 签名变化 → 重新扫描该根并覆盖缓存。
//!
//! ## 已知近似
//!
//! Windows 上「原地改写文件内容」不会改变父目录 `mtime`，因此这类文件的
//! **大小可能沿用上次值**，直到目录结构（增删改名）发生变化。这是「按目录
//! 变化复用」的固有代价；删除动作仍按真实文件处理——文件已不存在时会记为
//! `already_gone`，不会误报成功释放。
//!
//! ## 只对「扫描根即叶子缓存目录」的模块启用
//!
//! `browser_cache` **不启用**增量：它的规则锚是浏览器安装根，需要读
//! `profiles.ini` / 枚举多个 Profile 才能定位真正的缓存目录；而缓存文件高频
//! 改写但安装根 `mtime` 几乎不动，按根签名会长期返回旧结果。宁可每次全量。
//!
//! ## 缓存落在 DB 而不是进程内存
//!
//! 应用重启后第一次扫描也能命中（用户关掉应用再打开是常态）。表结构见
//! `v9__longtail.sql`，主键是模块代码，每个模块一行。

use std::path::Path;

use rusqlite::{params, Connection};

use super::modules::{scan_dir, ScanItem};

/// 扫描一个根目录；签名未变则复用上次结果。
///
/// `module_code` 只用于缓存主键与日志，不参与业务判定。
pub fn scan_dir_cached(
    conn: &Connection,
    root: &Path,
    module_code: &str,
    category: &str,
    risk_level: i64,
) -> Vec<ScanItem> {
    // 根不存在：不缓存。目录后来被创建时能立刻扫到，不会被「missing 签名」骗过去。
    if !root.is_dir() {
        return scan_dir(root, module_code, category, risk_level);
    }

    let key = format!(
        "{module_code}|{category}|{risk_level}|{}",
        root.to_string_lossy().to_lowercase()
    );
    let sig = signature(root);

    if let Some(items) = load(conn, &key, &sig) {
        eprintln!(
            "[scan-cache] {module_code} 命中缓存（{} 项，根未变化）: {}",
            items.len(),
            root.display()
        );
        return items;
    }

    let items = scan_dir(root, module_code, category, risk_level);
    if let Err(e) = save(conn, &key, &sig, &items) {
        // 缓存写失败只影响下次是否复用，绝不能影响本次扫描结果。
        eprintln!("[scan-cache] {module_code} 写入失败（忽略）: {e}");
    }
    items
}

/// 清空全部缓存（测试 / 排障用）。
pub fn clear(conn: &Connection) -> Result<usize, String> {
    conn.execute("DELETE FROM scan_cache", [])
        .map_err(|e| e.to_string())
}

/// 目录签名：路径 + `mtime` 纳秒值。
///
/// 用纳秒而不是秒：两次扫描间隔可能只有几秒，秒级精度会把「刚变化」误判成
/// 「没变化」。
fn signature(root: &Path) -> String {
    let mtime = std::fs::metadata(root)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}|{mtime}", root.to_string_lossy().to_lowercase())
}

fn load(conn: &Connection, key: &str, sig: &str) -> Option<Vec<ScanItem>> {
    let raw: String = conn
        .query_row(
            "SELECT items_json FROM scan_cache WHERE module = ?1 AND signature = ?2",
            params![key, sig],
            |r| r.get(0),
        )
        .ok()?;
    serde_json::from_str(&raw).ok()
}

fn save(conn: &Connection, key: &str, sig: &str, items: &[ScanItem]) -> Result<(), String> {
    let json = serde_json::to_string(items).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO scan_cache (module, signature, items_json, updated_at)
         VALUES (?1, ?2, ?3, unixepoch())
         ON CONFLICT(module) DO UPDATE SET
             signature = excluded.signature,
             items_json = excluded.items_json,
             updated_at = excluded.updated_at",
        params![key, sig, json],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrate::migrate(&conn).unwrap();
        conn
    }

    fn make_tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.tmp"), vec![b'x'; 10]).unwrap();
        dir
    }

    #[test]
    fn second_scan_reuses_cache_until_root_changes() {
        let conn = db();
        let dir = make_tree();

        let first = scan_dir_cached(&conn, dir.path(), "temp", "user_temp", 1);
        assert_eq!(first.len(), 1);

        // 同一根、未改动 → 命中缓存，结果一致
        let second = scan_dir_cached(&conn, dir.path(), "temp", "user_temp", 1);
        assert_eq!(second, first);

        // 新增文件改变根目录 mtime → 缓存失效，必须看到 2 条。
        // 先 sleep 再写：部分文件系统 mtime 精度较粗，两次操作同刻会误判「没变化」。
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(dir.path().join("b.tmp"), vec![b'y'; 20]).unwrap();
        let third = scan_dir_cached(&conn, dir.path(), "temp", "user_temp", 1);
        assert_eq!(third.len(), 2, "根目录变化后必须重新扫描: {third:#?}");
    }

    #[test]
    fn different_module_or_category_do_not_share_cache() {
        let conn = db();
        let dir = make_tree();

        assert_eq!(
            scan_dir_cached(&conn, dir.path(), "temp", "user_temp", 1).len(),
            1
        );
        // 同根但不同 category → 独立缓存行，仍然各自扫描成功
        let other = scan_dir_cached(&conn, dir.path(), "temp", "windows_temp", 2);
        assert_eq!(other.len(), 1);
        assert_eq!(other[0].category, "windows_temp");
        assert_eq!(other[0].risk_level, 2);
    }

    #[test]
    fn missing_root_is_not_cached() {
        let conn = db();
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("no-such-dir");
        assert!(scan_dir_cached(&conn, &missing, "temp", "user_temp", 1).is_empty());
        let n: i64 = conn
            .query_row("SELECT count(*) FROM scan_cache", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0, "不存在的根不应写缓存");
    }

    #[test]
    fn clear_empties_cache() {
        let conn = db();
        let dir = make_tree();
        scan_dir_cached(&conn, dir.path(), "temp", "user_temp", 1);
        assert_eq!(clear(&conn).unwrap(), 1);
        assert_eq!(
            scan_dir_cached(&conn, dir.path(), "temp", "user_temp", 1).len(),
            1,
            "清空后应重新扫描而不是报错"
        );
    }
}
