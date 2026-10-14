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
                    .map_err(|e2| {
                        // copy+remove 的 remove 阶段也要分类（文件被占用时 remove 会失败）
                        match crate::execute::error_kind::classify_io_error(&e2) {
                            crate::execute::error_kind::DeleteFailureKind::SharingViolation => {
                                format!("FILE_IN_USE: {src}")
                            }
                            crate::execute::error_kind::DeleteFailureKind::PermissionDenied => {
                                format!("PERMISSION_DENIED: {src}")
                            }
                            _ => format!("copy+remove fallback {src} -> {:?}: {e2}", box_path),
                        }
                    })?;
            }
            // task-019：rename 失败按 errno 分类，给前端明确的 FILE_IN_USE / PERMISSION_DENIED 信号。
            Err(e) => {
                return match crate::execute::error_kind::classify_io_error(&e) {
                    crate::execute::error_kind::DeleteFailureKind::SharingViolation => {
                        Err(format!("FILE_IN_USE: {src}"))
                    }
                    crate::execute::error_kind::DeleteFailureKind::PermissionDenied => {
                        Err(format!("PERMISSION_DENIED: {src}"))
                    }
                    crate::execute::error_kind::DeleteFailureKind::NotFound => Ok(()),
                    crate::execute::error_kind::DeleteFailureKind::Unknown => {
                        Err(format!("rename {src} -> {:?}: {e}", box_path))
                    }
                };
            }
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

// ===================== task-017: restore =====================

/// 恢复结果。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub safe_box_id: i64,
    pub original_path: String,
    pub size_bytes: u64,
}

/// restore 错误（与任务书 §A 对齐；按真实 schema 适配）。
#[derive(Debug, thiserror::Error)]
pub enum RestoreError {
    #[error("保险箱条目不存在或已被永久剔除: id={0}")]
    NotFound(i64),
    #[error("原路径已被占用，拒绝覆盖: {0}")]
    TargetOccupied(String),
    #[error("原父目录不存在，请先手动创建: {0}")]
    ParentMissing(String),
    #[error("IO 错误: {0}")]
    Io(String),
    #[error("DB 错误: {0}")]
    Db(String),
}

/// 按 id 把文件从保险箱 rename 回 original_path，删 safe_box 行，写 audit_log。
///
/// 与任务书 §A 的偏差（按 v1__init.sql 真实 schema 适配）：
/// - 查询条件用 `status='in_box'`（不是 `purged_at IS NULL`）；
/// - 路径字段用 `safe_rel_path`（相对 box_dir），由调用方传 box_dir 拼绝对路径；
/// - audit_log 字段为 ts/actor/action/target/detail/result，映射 event_type→action='restore'。
pub fn restore_from_safe_box(
    conn: &Connection,
    box_dir: &Path,
    safe_box_id: i64,
) -> Result<RestoreResult, RestoreError> {
    // 1. 查行
    let row: (String, String, i64) = conn
        .query_row(
            "SELECT original_path, safe_rel_path, size_bytes
             FROM safe_box WHERE id=?1 AND status='in_box'",
            [safe_box_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| RestoreError::NotFound(safe_box_id))?;
    let (original_path, rel, size_bytes) = row;

    let orig = Path::new(&original_path);
    let box_abs = box_dir.join(&rel);

    // 2. 原路径已被占用 → 拒
    if orig.exists() {
        return Err(RestoreError::TargetOccupied(original_path));
    }
    // 3. 父目录必须存在
    match orig.parent() {
        Some(p) if !p.exists() => {
            return Err(RestoreError::ParentMissing(p.to_string_lossy().into()))
        }
        _ => {}
    }

    // 4. rename box → original
    std::fs::rename(&box_abs, orig)
        .map_err(|e| RestoreError::Io(format!("rename {box_abs:?} -> {orig:?}: {e}")))?;

    // 5. 删行
    conn.execute("DELETE FROM safe_box WHERE id=?1", [safe_box_id])
        .map_err(|e| RestoreError::Db(format!("delete safe_box row: {e}")))?;

    // 6. audit_log
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let detail = serde_json::json!({
        "safe_box_id": safe_box_id,
        "size_bytes": size_bytes,
        "from": rel,
    })
    .to_string();
    conn.execute(
        "INSERT INTO audit_log (ts, actor, action, target, detail, result)
         VALUES (?1, 'user', 'restore', ?2, ?3, 'success')",
        params![now, original_path, detail],
    )
    .map_err(|e| RestoreError::Db(format!("insert audit_log: {e}")))?;

    Ok(RestoreResult {
        safe_box_id,
        original_path,
        size_bytes: size_bytes as u64,
    })
}

// ===================== task-018: list =====================

/// 保险箱条目（前端 UI 用）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeBoxEntry {
    pub id: i64,
    pub original_path: String,
    pub size_bytes: u64,
    /// 移入时间（unix 秒），对应 schema 真实列 `quarantined_at`。
    pub quarantined_at: i64,
}

/// list 响应：条目 + 汇总。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeBoxListResp {
    pub entries: Vec<SafeBoxEntry>,
    pub total_items: i64,
    pub total_bytes: u64,
}

/// 列出当前 `status='in_box'` 的所有条目，按 quarantined_at DESC（最新在前）。
///
/// 与任务书 §A 的偏差（按 v1__init.sql 真实 schema 适配）：
/// - 过滤条件 `status='in_box'`（任务书写 `purged_at IS NULL`）；
/// - 时间列 `quarantined_at`（任务书写 `moved_at`）；
/// - size_bytes 用 u64（真实列 INTEGER，i64 绑定后转 u64）。
pub fn list_safe_box(conn: &Connection) -> Result<SafeBoxListResp, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, original_path, size_bytes, quarantined_at
             FROM safe_box WHERE status='in_box'
             ORDER BY quarantined_at DESC",
        )
        .map_err(|e| format!("prepare list_safe_box: {e}"))?;

    let entries: Vec<SafeBoxEntry> = stmt
        .query_map([], |r| {
            Ok(SafeBoxEntry {
                id: r.get(0)?,
                original_path: r.get(1)?,
                size_bytes: r.get::<_, i64>(2).map(|v| v as u64)?,
                quarantined_at: r.get(3)?,
            })
        })
        .map_err(|e| format!("query_map list_safe_box: {e}"))?
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| format!("collect list_safe_box: {e}"))?;

    let total_items = entries.len() as i64;
    let total_bytes = entries.iter().map(|e| e.size_bytes).sum();

    Ok(SafeBoxListResp {
        entries,
        total_items,
        total_bytes,
    })
}

// ===================== task-037: purge =====================

/// 按 id 永久删除保险箱条目：删磁盘文件 + 删 DB 行 + 写 audit_log。
///
/// - 磁盘文件不存在不报错（手动清理过的场景仍能删行）；
/// - DB 行不存在返回 `Err("safe_box item not found")`；
/// - audit_log action='purge'，target=original_path，detail 含 safe_rel_path。
pub fn purge_item(
    conn: &Connection,
    box_dir: &Path,
    item_id: i64,
) -> Result<(), String> {
    // 1. 查行
    let row: (String, String) = conn
        .query_row(
            "SELECT original_path, safe_rel_path FROM safe_box
             WHERE id=?1 AND status='in_box'",
            [item_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| format!("safe_box item not found: id={item_id}"))?;
    let (original_path, rel) = row;

    // 2. 删磁盘文件（NotFound 吞掉）
    let box_abs = box_dir.join(&rel);
    match fs::remove_file(&box_abs) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("remove {box_abs:?}: {e}")),
    }

    // 3. 删 DB 行（purge 即永久删除，不保留 status='purged' 行）
    conn.execute("DELETE FROM safe_box WHERE id=?1", [item_id])
        .map_err(|e| format!("delete safe_box row: {e}"))?;

    // 4. audit_log
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let detail = serde_json::json!({
        "safe_box_id": item_id,
        "safe_rel_path": rel,
    })
    .to_string();
    conn.execute(
        "INSERT INTO audit_log (ts, actor, action, target, detail, result)
         VALUES (?1, 'user', 'purge', ?2, ?3, 'success')",
        params![now, original_path, detail],
    )
    .map_err(|e| format!("insert audit_log: {e}"))?;

    Ok(())
}
