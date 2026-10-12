//! 保险箱删除器（task-016）：删除前先把文件 `rename` 到 `<data_dir>/safe_box/`，
//! 用户后悔可恢复；超 5GB 按 `quarantined_at` 最旧永久剔除。
//!
//! 与任务书 §A 的偏差（按 v1__init.sql 真实 schema 适配）：
//! - 实际 `safe_box` 表字段为：id / original_path / safe_rel_path / size_bytes /
//!   quarantined_at / expires_at / status / source_task_id（不是任务书说的
//!   session_id / item_id / box_path / moved_at / purged_at）。
//! - `Deleter` trait 签名为 `delete_file(&NormalizedPath) -> Result<(), String>`
//!   （task-003 定型），不是 `&str -> Result<u64, String>`；bytes_freed 由 executor
//!   在调 delete_file 之前用 `fs::metadata` 取。
//! - 容量常量用 `AtomicU64`，测试时通过 `set_quota_for_test()` 覆盖，
//!   避免在生产代码里到处 `#[cfg(test)]`。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use rusqlite::params;
use rusqlite::Connection;

use crate::execute::delete::Deleter;
use crate::execute::normalize::NormalizedPath;

/// 保险箱默认容量上限：5 GB（生产值）。测试可通过 [`set_quota_for_test`] 覆盖。
static QUOTA_BYTES: AtomicU64 = AtomicU64::new(5 * 1024 * 1024 * 1024);

/// 测试覆盖 quota（比如 300 字节）。生产代码不要调。
pub fn set_quota_for_test(v: u64) {
    QUOTA_BYTES.store(v, Ordering::SeqCst);
}

fn current_quota() -> u64 {
    QUOTA_BYTES.load(Ordering::SeqCst)
}

/// 保险箱删除器：per-item 构造，持有该 item 的 DB 句柄与 box 根目录。
pub struct SafeBoxDeleter<'conn> {
    box_dir: PathBuf,
    conn: &'conn Connection,
    /// 关联的 cleanup_task.id（批量任务行，task-015 已写）；可空。
    source_task_id: Option<i64>,
}

impl<'conn> SafeBoxDeleter<'conn> {
    pub fn new(box_dir: &Path, conn: &'conn Connection, source_task_id: Option<i64>) -> Self {
        Self {
            box_dir: box_dir.to_path_buf(),
            conn,
            source_task_id,
        }
    }

    /// 当前保险箱「在箱」总大小（status='in_box' 行的 size_bytes 之和）。
    pub fn current_usage(&self) -> Result<u64, String> {
        let v: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(size_bytes), 0) FROM safe_box WHERE status='in_box'",
                [],
                |r| r.get(0),
            )
            .map_err(|e| format!("query safe_box sum: {e}"))?;
        Ok(v as u64)
    }

    /// 超容时按 `quarantined_at ASC` 永久剔除最旧文件，直到剩余空间足够放 `incoming`。
    ///
    /// 返回剔除后剩余总大小。
    pub fn enforce_quota(&self, incoming: u64) -> Result<u64, String> {
        let quota = current_quota();
        let mut usage = self.current_usage()?;
        if usage + incoming <= quota {
            return Ok(usage);
        }

        // 按时间最旧逐行剔除，直到够放新文件。
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, safe_rel_path FROM safe_box
                 WHERE status='in_box' ORDER BY quarantined_at ASC",
            )
            .map_err(|e| format!("prepare evict stmt: {e}"))?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| format!("query evict rows: {e}"))?;

        let mut evicted_sizes: Vec<(i64, i64)> = Vec::new();
        for row in rows {
            if usage + incoming <= quota {
                break;
            }
            let (id, rel) = row.map_err(|e| format!("iter evict row: {e}"))?;
            let path = self.box_dir.join(&rel);
            match fs::remove_file(&path) {
                Ok(()) => {
                    let size: i64 = self
                        .conn
                        .query_row("SELECT size_bytes FROM safe_box WHERE id=?1", [id], |r| {
                            r.get(0)
                        })
                        .unwrap_or(0);
                    let _ = self
                        .conn
                        .execute("UPDATE safe_box SET status='purged' WHERE id=?1", [id]);
                    usage = usage.saturating_sub(size as u64);
                    evicted_sizes.push((id, size));
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    // 磁盘上已经没了，DB 行标 purged 即可，不阻塞 quota。
                    let _ = self
                        .conn
                        .execute("UPDATE safe_box SET status='purged' WHERE id=?1", [id]);
                }
                Err(e) => return Err(format!("evict {path:?}: {e}")),
            }
        }

        let _ = evicted_sizes;
        Ok(usage)
    }
}

impl<'conn> Deleter for SafeBoxDeleter<'conn> {
    fn delete_file(&self, p: &NormalizedPath) -> Result<(), String> {
        let src = p.normalized();
        let src_path = Path::new(src);
        let meta = match fs::metadata(src_path) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(format!("stat {src}: {e}")),
        };
        let size = meta.len();

        // 1. 先确保 box_dir 存在
        fs::create_dir_all(&self.box_dir)
            .map_err(|e| format!("create box_dir {:?}: {e}", self.box_dir))?;

        // 2. quota
        self.enforce_quota(size)?;

        // 3. 构造 box 文件名：<unix_ts>_<原始文件名>
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let file_name = src_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.bin");
        let rel = format!("{ts}_{file_name}");
        let box_path = self.box_dir.join(&rel);

        // 4. rename；跨盘 rename 失败时 fallback copy+remove。
        match fs::rename(src_path, &box_path) {
            Ok(()) => {}
            Err(e) if is_cross_device(&e) => {
                fs::copy(src_path, &box_path)
                    .and_then(|_| fs::remove_file(src_path))
                    .map_err(|e2| format!("copy+remove fallback {src} -> {:?}: {e2}", box_path))?;
            }
            Err(e) => return Err(format!("rename {src} -> {:?}: {e}", box_path)),
        }

        // 5. 写 safe_box 行
        let now = ts as i64;
        let expires_at = now + 7 * 24 * 3600; // 7 天后过期（后续 TTL 任务用）
        self.conn
            .execute(
                "INSERT INTO safe_box
                 (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status, source_task_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'in_box', ?6)",
                params![src, rel, size as i64, now, expires_at, self.source_task_id],
            )
            .map_err(|e| format!("insert safe_box row: {e}"))?;

        Ok(())
    }

    fn delete_dir(&self, p: &NormalizedPath) -> Result<(), String> {
        Err(format!(
            "SafeBoxDeleter::delete_dir not implemented for {} (task-016 only moves files)",
            p.normalized()
        ))
    }
}

/// 判断是否为跨盘 rename（Windows ERROR_NOT_SAME_DEVICE=17；Linux EXDEV=18）。
fn is_cross_device(e: &io::Error) -> bool {
    matches!(e.raw_os_error(), Some(17) | Some(18))
}
