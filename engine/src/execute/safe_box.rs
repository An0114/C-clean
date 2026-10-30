//! 保险箱删除器（task-016）：删除前先把文件 `rename` 到 `<data_dir>/safe_box/`，
//! 用户后悔可恢复；超 5GB 按 `quarantined_at` 最旧永久剔除。
//!
//! 与任务书 §A 的偏差（按 v1__init.sql 真实 schema 适配）：
//! - 实际 `safe_box` 表字段为：id / original_path / safe_rel_path / size_bytes /
//!   quarantined_at / expires_at / status / source_task_id（不是任务书说的
//!   session_id / item_id / box_path / moved_at / purged_at）。
//! - `Deleter` trait 签名为 `delete_file(&NormalizedPath) -> Result<bool, String>`
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

use crate::audit_log::query::AuditLogEntry;
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
    /// 移入保险箱。
    ///
    /// task-052 G5：返回 `Ok(true)` = 真的移走了；`Ok(false)` = 源文件在移入前
    /// 就已不存在（NotFound 幂等），本次没有产生任何释放量。
    fn delete_file(&self, p: &NormalizedPath) -> Result<bool, String> {
        let src = p.normalized();
        let src_path = Path::new(src);
        let meta = match fs::metadata(src_path) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
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

        // 4. 先开事务并写行，**再**动文件（task-058）。
        //
        // task-016 遗留的孤儿根因就是顺序反了：旧实现先 rename 再 INSERT，
        // 一旦 INSERT 失败（磁盘满 / 约束 / 库被锁），文件已经在 box 目录里
        // 而 DB 无行——下次 `enforce_quota` 扫到它只会当孤儿标 purged，
        // 用户感知是「文件凭空消失」。
        //
        // 现在把 INSERT 提到 rename 之前，并且整段包在事务里：
        // - INSERT 失败 → 事务未提交、文件**还没动**，直接返回 Err，无孤儿；
        // - rename 失败 → `?` 提前返回，`Transaction` 的 Drop 自动 rollback，
        //   刚写的行消失，文件仍在原位（rename 失败本身不移动文件）；
        // - 跨盘 copy 成功但 remove 失败 → 删掉已 copy 的目标文件再报错。
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| format!("begin tx for {src}: {e}"))?;

        let now = ts as i64;
        let expires_at = now + 7 * 24 * 3600; // 7 天后过期（后续 TTL 任务用）
        tx.execute(
            "INSERT INTO safe_box
             (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status, source_task_id)
             VALUES (?1, ?2, ?3, ?4, ?5, 'in_box', ?6)",
            params![src, rel, size as i64, now, expires_at, self.source_task_id],
        )
        .map_err(|e| format!("insert safe_box row for {src}: {e}"))?;

        // 5. rename；跨盘 rename 失败时 fallback copy+remove。
        match fs::rename(src_path, &box_path) {
            Ok(()) => {}
            Err(e) if is_cross_device(&e) => {
                if let Err(e2) = fs::copy(src_path, &box_path) {
                    // copy 失败：box_path 可能已被创建为不完整文件，删掉再报错。
                    let _ = fs::remove_file(&box_path);
                    return Err(format!("copy fallback {src} -> {:?}: {e2}", box_path));
                }
                if let Err(e2) = fs::remove_file(src_path) {
                    // copy 成功但 remove 失败：源文件还在，box 里那份是重复的。
                    // 必须删掉目标，否则 enforce_quota 会把它当正常条目占配额。
                    let _ = fs::remove_file(&box_path);
                    return Err(match crate::execute::error_kind::classify_io_error(&e2) {
                        crate::execute::error_kind::DeleteFailureKind::SharingViolation => {
                            format!("FILE_IN_USE: {src}")
                        }
                        crate::execute::error_kind::DeleteFailureKind::PermissionDenied => {
                            format!("PERMISSION_DENIED: {src}")
                        }
                        _ => format!("copy+remove fallback {src} -> {:?}: {e2}", box_path),
                    });
                }
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
                    // NotFound：源文件本来就不在（竞态）。行已写进事务，这里要显式
                    // rollback 语义——`?` 返回后 Transaction Drop 会回滚，无需手动删。
                    crate::execute::error_kind::DeleteFailureKind::NotFound => Ok(false),
                    crate::execute::error_kind::DeleteFailureKind::Unknown => {
                        Err(format!("rename {src} -> {:?}: {e}", box_path))
                    }
                };
            }
        }

        // 6. 文件已就位 → 提交事务（行与文件此时一致）。
        tx.commit()
            .map_err(|e| format!("commit safe_box tx for {src}: {e}"))?;

        // 全部成功 = 文件真的从原位置移走了。
        Ok(true)
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

/// 按 id 把文件从保险箱 rename 回 original_path，标记该行为 `status='restored'`
/// 并写 `restored_at`（task-056 起不再 DELETE），最后写 audit_log。
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
    // 3. 父目录缺失时自动 create_dir_all（task-043）。
    //    之前直接报 ParentMissing 让用户手动建目录；现在自动建多层父目录。
    if let Some(p) = orig.parent() {
        if !p.exists() {
            std::fs::create_dir_all(p)
                .map_err(|e| RestoreError::Io(format!("无法创建父目录 {}: {e}", p.display())))?;
        }
    }

    // 4. rename box → original
    std::fs::rename(&box_abs, orig)
        .map_err(|e| RestoreError::Io(format!("rename {box_abs:?} -> {orig:?}: {e}")))?;

    // 5. 标记为已恢复（task-056：不再 DELETE 行）
    //
    // 此前直接 DELETE，导致「这个文件什么时候恢复的」在 safe_box 里查不到，
    // 只能去 audit_log 里按路径反查（N+1，且路径可能重复）。改为保留行 +
    // 写 restored_at，与 v1 schema 里本就预留的 status='restored' 语义对齐。
    // 列表/配额/统计的查询都带 `status='in_box'` 过滤，所以外部行为不变。
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    conn.execute(
        "UPDATE safe_box SET status='restored', restored_at=?2 WHERE id=?1",
        params![safe_box_id, now],
    )
    .map_err(|e| RestoreError::Db(format!("mark safe_box row restored: {e}")))?;

    // 6. audit_log
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
    /// task-051：来源清理批次（`safe_box.source_task_id`），NULL = 历史遗留/未关联。
    pub source_task_id: Option<i64>,
}

/// task-051：保险箱列表过滤条件。
///
/// 语义：
/// - `session_ids` 空 = 不限 session；非空 = `source_task_id IN (...)`（内部去重，保序）；
/// - `from_ts` / `to_ts` 是**闭区间**的 Unix 秒；`None` 或 `<= 0` 视为未设置；
/// - `from_ts > to_ts` 由 [`list_paged`] 直接返回空页（不 Err）。
#[derive(Debug, Clone, Default)]
pub struct SafeBoxFilter {
    /// 空 = 不限 session；非空 = 组合过滤（内部先去重）。
    pub session_ids: Vec<i64>,
    /// 含下界；Unix 秒；`<= 0` 视为未设置。
    pub from_ts: Option<i64>,
    /// 含上界；Unix 秒；`<= 0` 视为未设置。
    pub to_ts: Option<i64>,
}

impl SafeBoxFilter {
    /// 归一化：session id 去重（保序）+ 把 `<= 0` 的时间视作未设置。
    pub fn normalized(&self) -> (Vec<i64>, Option<i64>, Option<i64>) {
        let mut ids: Vec<i64> = Vec::with_capacity(self.session_ids.len());
        for id in &self.session_ids {
            if !ids.contains(id) {
                ids.push(*id);
            }
        }
        (
            ids,
            self.from_ts.filter(|v| *v > 0),
            self.to_ts.filter(|v| *v > 0),
        )
    }

    /// 区间是否自相矛盾（`from > to`）。只有两端都设置时才可能为真。
    pub fn is_empty_range(&self) -> bool {
        let (_, from, to) = self.normalized();
        matches!((from, to), (Some(f), Some(t)) if f > t)
    }
}

/// list 响应：条目 + 汇总。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeBoxListResp {
    pub entries: Vec<SafeBoxEntry>,
    pub total_items: i64,
    pub total_bytes: u64,
}

/// task-045：分页响应。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeBoxPage {
    /// 当前页条目。
    pub items: Vec<SafeBoxEntry>,
    /// 全表 total（与页大小无关）。
    pub total: i64,
    /// 当前页（1 起）。
    pub page: i64,
    /// 实际使用的 page_size（已 clamp 到 [1,200]）。
    pub page_size: i64,
    /// 全表字节汇总（保留原 UI 顶栏显示）。
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
            "SELECT id, original_path, size_bytes, quarantined_at, source_task_id
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
                source_task_id: r.get(4)?,
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

/// task-045 / task-051：分页列出 `status='in_box'` 条目。
///
/// - `page` 从 1 起，<1 按 1 处理；
/// - `page_size` clamp 到 [1, 200]（防前端传 99999）；
/// - ORDER BY id DESC（最新在前，稳定排序）；
/// - `items` / `COUNT(*)` / `SUM(size_bytes)` **共用同一套 WHERE 与参数**，
///   保证 `total` / `total_bytes` 与筛选条件一致（不是全表数）；
/// - `filter.from_ts > filter.to_ts` 直接返回空页（不 Err、不回落全量）。
///
/// task-051 重构：原来 `match session_id` 的两套硬编码 SQL（加时间区间会炸成 8 个
/// 分支）改为**动态 WHERE 拼接 + 占位符绑定**——拼进 SQL 的只有固定片段，
/// 任何用户输入（session id / 时间戳）都走 `?` 参数。
///
/// 真实 schema：safe_box 没有 session_id 列，只有 source_task_id；一个
/// cleanup_task 对应一次清理 session，因此前端的「session」= source_task_id。
pub fn list_paged(
    conn: &Connection,
    page: u32,
    page_size: u32,
    filter: &SafeBoxFilter,
) -> Result<SafeBoxPage, String> {
    let page = (page.max(1)) as i64;
    let page_size = (page_size.clamp(1, 200)) as i64;
    let offset = (page - 1) * page_size;

    let (session_ids, from_ts, to_ts) = filter.normalized();

    // 区间自相矛盾：空页（A5）。放在查询前，避免跑两条注定为 0 的聚合。
    if matches!((from_ts, to_ts), (Some(f), Some(t)) if f > t) {
        return Ok(SafeBoxPage {
            items: Vec::new(),
            total: 0,
            page,
            page_size,
            total_bytes: 0,
        });
    }

    // ---- 动态 WHERE：status='in_box' 打底，其余条件按需追加 ----
    let mut clauses: Vec<String> = vec!["status='in_box'".to_string()];
    if !session_ids.is_empty() {
        let holders = vec!["?"; session_ids.len()].join(", ");
        clauses.push(format!("source_task_id IN ({holders})"));
    }
    if from_ts.is_some() {
        clauses.push("quarantined_at >= ?".to_string());
    }
    if to_ts.is_some() {
        clauses.push("quarantined_at <= ?".to_string());
    }
    let where_clause = format!("WHERE {}", clauses.join(" AND "));

    // 绑定顺序必须与 WHERE 里占位符出现的顺序一致：session ids → from → to。
    let mut filter_params: Vec<i64> = session_ids;
    if let Some(f) = from_ts {
        filter_params.push(f);
    }
    if let Some(t) = to_ts {
        filter_params.push(t);
    }

    let count_sql = format!("SELECT COUNT(*) FROM safe_box {where_clause}");
    let total: i64 = conn
        .query_row(
            &count_sql,
            rusqlite::params_from_iter(filter_params.iter()),
            |r| r.get(0),
        )
        .map_err(|e| format!("count safe_box: {e}"))?;

    let sum_sql = format!("SELECT COALESCE(SUM(size_bytes),0) FROM safe_box {where_clause}");
    let total_bytes: i64 = conn
        .query_row(
            &sum_sql,
            rusqlite::params_from_iter(filter_params.iter()),
            |r| r.get(0),
        )
        .map_err(|e| format!("sum safe_box: {e}"))?;

    let items_sql = format!(
        "SELECT id, original_path, size_bytes, quarantined_at, source_task_id
         FROM safe_box
         {where_clause}
         ORDER BY id DESC
         LIMIT ? OFFSET ?"
    );
    let mut stmt = conn
        .prepare(&items_sql)
        .map_err(|e| format!("prepare list_paged: {e}"))?;

    // 分页参数追加在筛选参数之后（对应 SQL 末尾的 LIMIT ? OFFSET ?）。
    let mut item_params = filter_params;
    item_params.push(page_size);
    item_params.push(offset);

    let rows = stmt
        .query_map(rusqlite::params_from_iter(item_params.iter()), |r| {
            Ok(SafeBoxEntry {
                id: r.get(0)?,
                original_path: r.get(1)?,
                size_bytes: r.get::<_, i64>(2).map(|v| v as u64)?,
                quarantined_at: r.get(3)?,
                source_task_id: r.get(4)?,
            })
        })
        .map_err(|e| format!("query_map list_paged: {e}"))?;

    let mut items: Vec<SafeBoxEntry> = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("row: {e}"))?);
    }

    Ok(SafeBoxPage {
        items,
        total,
        page,
        page_size,
        total_bytes: total_bytes.max(0) as u64,
    })
}

/// task-046：按 source_task_id 分组，列出有哪些「session」进过保险箱。
///
/// - `session_id` 实际是 safe_box.source_task_id（cleanup_task.id）；
/// - `item_count` = 该 task 下 in_box 条数；
/// - `created_ts` = 该 task 下最早一条的 quarantined_at；
/// - 按 session_id DESC（最新 task 在前）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub session_id: i64,
    pub item_count: i64,
    pub created_ts: i64,
}

pub fn list_sessions(conn: &Connection) -> Result<Vec<SessionSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT source_task_id, COUNT(*), MIN(quarantined_at)
             FROM safe_box
             WHERE status='in_box' AND source_task_id IS NOT NULL
             GROUP BY source_task_id
             ORDER BY source_task_id DESC",
        )
        .map_err(|e| format!("prepare list_sessions: {e}"))?;

    let rows: Vec<SessionSummary> = stmt
        .query_map([], |r| {
            Ok(SessionSummary {
                session_id: r.get(0)?,
                item_count: r.get(1)?,
                created_ts: r.get(2)?,
            })
        })
        .map_err(|e| format!("query_map list_sessions: {e}"))?
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| format!("collect list_sessions: {e}"))?;

    Ok(rows)
}

/// task-051：按 safe_box id 查该条目**原路径**的审计历史（只读）。
///
/// - 先取 `original_path`，再按路径查 `audit_log`（大小写不敏感，见
///   [`crate::audit_log::query::list_by_target`]）；
/// - 条目不存在 → `Err("safe_box item not found: id=...")`；
/// - 只返回最近 20 条（`ts DESC, id DESC`）；无历史返回空 vec（不 Err）。
pub fn item_history(conn: &Connection, safe_box_id: i64) -> Result<Vec<AuditLogEntry>, String> {
    let original_path: String = conn
        .query_row(
            "SELECT original_path FROM safe_box WHERE id=?1",
            [safe_box_id],
            |r| r.get(0),
        )
        .map_err(|_| format!("safe_box item not found: id={safe_box_id}"))?;

    crate::audit_log::query::list_by_target(conn, &original_path, 20)
}

// ===================== task-037: purge =====================

/// 按 id 永久删除保险箱条目：删磁盘文件 + 删 DB 行 + 写 audit_log。
///
/// - 磁盘文件不存在不报错（手动清理过的场景仍能删行）；
/// - DB 行不存在返回 `Err("safe_box item not found")`；
/// - audit_log action='purge'，target=original_path，detail 含 safe_rel_path。
pub fn purge_item(conn: &Connection, box_dir: &Path, item_id: i64) -> Result<(), String> {
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

// ===================== task-038: purge_many =====================

/// 批量 purge 结果（前端 UI 用）。
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurgeManyResult {
    /// 成功删除条数。
    pub deleted: usize,
    /// 失败条数。
    pub failed: usize,
    /// 失败原因列表（`id=<id>: <err>`）。
    pub errors: Vec<String>,
}

// ===================== task-064：7 天自动 purge =====================

/// 自动 purge 的结果（返回给调用方打日志 / 前端提示）。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurgeExpiredResult {
    /// 成功永久删除的条数。
    pub purged: usize,
    /// 失败条数（磁盘删不掉 / DB 写失败）。
    pub failed: usize,
    /// 这些条目占用的字节数（成功删掉的那部分）。
    pub bytes_freed: u64,
    /// 失败原因（`id=<id>: <err>`），便于排查。
    pub errors: Vec<String>,
}

impl PurgeExpiredResult {
    /// 本轮有没有真的干活（没有过期项时调用方不必打日志）。
    pub fn is_noop(&self) -> bool {
        self.purged == 0 && self.failed == 0
    }
}

/// 永久删除所有 `expires_at < now` 的保险箱条目（PRD §396「7 天后自动 purge」）。
///
/// # 为什么逐条而不是一条批量 DELETE
///
/// 每条都要写 `audit_log`（`action='auto_purge'`）——用户必须能查到「哪些文件被
/// 系统自动删掉了」。一条批量 DELETE 只能记一行汇总，等于把审计粒度丢了。
///
/// # 为什么 `now` 是参数而不是函数内取
///
/// 单测要能构造「昨天过期 / 明天过期」而不用真的等一天。
///
/// # 失败语义
///
/// **单条失败不影响其它条**：磁盘删不掉（被占用）就记 failed 并**保留 DB 行**，
/// 下轮再试。绝不能「文件没删掉就把行删了」——那样文件变成永久孤儿，用户
/// 再也找不到它。
pub fn purge_expired(conn: &Connection, box_dir: &Path, now: i64) -> PurgeExpiredResult {
    let mut result = PurgeExpiredResult::default();

    // 1. 先把过期候选捞出来（一次查询，避免边删边查）。
    let candidates: Vec<(i64, String, String, i64)> = {
        let mut stmt = match conn.prepare(
            "SELECT id, original_path, safe_rel_path, size_bytes FROM safe_box
             WHERE status='in_box' AND expires_at < ?1",
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[purge_expired] prepare: {e}");
                result.failed += 1;
                result.errors.push(format!("prepare: {e}"));
                return result;
            }
        };
        let rows = stmt.query_map([now], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)));
        match rows {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(e) => {
                eprintln!("[purge_expired] query: {e}");
                result.failed += 1;
                result.errors.push(format!("query: {e}"));
                return result;
            }
        }
    };

    // 2. 逐条：删文件 → 删行 → 写审计。
    for (id, original_path, rel, size) in candidates {
        let box_abs = box_dir.join(&rel);
        match fs::remove_file(&box_abs) {
            Ok(()) => {}
            // 文件已经不在了（用户手动删过 / 上次 purge 删了文件没删行）→ 继续删行。
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => {
                // 文件删不掉（被占用）→ **保留 DB 行**，下轮重试。
                result.failed += 1;
                result
                    .errors
                    .push(format!("id={id}: remove {box_abs:?}: {e}"));
                continue;
            }
        }

        if let Err(e) = conn.execute("DELETE FROM safe_box WHERE id=?1", [id]) {
            result.failed += 1;
            result.errors.push(format!("id={id}: delete row: {e}"));
            continue;
        }

        let detail = serde_json::json!({
            "safe_box_id": id,
            "safe_rel_path": rel,
            "size_bytes": size,
            "reason": "expired_7d",
        })
        .to_string();
        // 审计写失败不影响「已经删掉了」这个事实，只记日志。
        if let Err(e) = conn.execute(
            "INSERT INTO audit_log (ts, actor, action, target, detail, result)
             VALUES (?1, 'system', 'auto_purge', ?2, ?3, 'success')",
            params![now, original_path, detail],
        ) {
            eprintln!("[purge_expired] audit_log id={id}: {e}");
        }

        result.purged += 1;
        result.bytes_freed += size.max(0) as u64;
    }

    result
}

/// 批量永久删除：循环调 [`purge_item`]，单个失败不中断，收集 errors。
///
/// - 空 slice 直接返默认空结果；
/// - 单个 purge_item 失败 → `failed += 1` 并 push 一条错误，不阻断后续 id。
pub fn purge_many(conn: &Connection, box_dir: &Path, item_ids: &[i64]) -> PurgeManyResult {
    let mut out = PurgeManyResult::default();
    for &id in item_ids {
        match purge_item(conn, box_dir, id) {
            Ok(()) => out.deleted += 1,
            Err(e) => {
                out.failed += 1;
                out.errors.push(format!("id={id}: {e}"));
            }
        }
    }
    out
}

// ===================== task-039: restore_many =====================

/// 批量恢复结果（前端 UI 用）。purge 走独立的 [`PurgeManyResult`]（字段名叫 `deleted`），两者不共用结构。
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchOpResult {
    /// 成功条数（restore 语义）。
    pub succeeded: usize,
    /// 失败条数。
    pub failed: usize,
    /// 失败原因列表（`id=<id>: <err>`）。
    pub errors: Vec<String>,
}

/// 批量恢复：循环调 [`restore_from_safe_box`]，单个失败不中断，收集 errors。
///
/// - 空 slice 直接返默认空结果；
/// - 单个恢复失败（如 ParentMissing / TargetOccupied / NotFound）→ `failed += 1` 并 push 一条错误。
pub fn restore_many(conn: &Connection, box_dir: &Path, item_ids: &[i64]) -> BatchOpResult {
    let mut out = BatchOpResult::default();
    for &id in item_ids {
        match restore_from_safe_box(conn, box_dir, id) {
            Ok(_) => out.succeeded += 1,
            Err(e) => {
                out.failed += 1;
                out.errors.push(format!("id={id}: {e}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_dedups_session_ids_keeping_order() {
        let f = SafeBoxFilter {
            session_ids: vec![3, 1, 3, 2, 1],
            ..Default::default()
        };
        let (ids, from, to) = f.normalized();
        assert_eq!(ids, vec![3, 1, 2], "去重且保持首次出现顺序");
        assert!(from.is_none() && to.is_none());
    }

    #[test]
    fn filter_treats_non_positive_ts_as_unset() {
        let f = SafeBoxFilter {
            from_ts: Some(0),
            to_ts: Some(-5),
            ..Default::default()
        };
        let (_, from, to) = f.normalized();
        assert_eq!(from, None, "from_ts <= 0 视为未设置");
        assert_eq!(to, None, "to_ts <= 0 视为未设置");
        assert!(!f.is_empty_range(), "两端都未设置不算空区间");
    }

    #[test]
    fn filter_keeps_positive_bounds() {
        let f = SafeBoxFilter {
            from_ts: Some(100),
            to_ts: Some(200),
            ..Default::default()
        };
        let (_, from, to) = f.normalized();
        assert_eq!(from, Some(100));
        assert_eq!(to, Some(200));
        assert!(!f.is_empty_range(), "from <= to 是合法闭区间");
    }

    #[test]
    fn filter_empty_range_only_when_from_greater_than_to() {
        let bad = SafeBoxFilter {
            from_ts: Some(201),
            to_ts: Some(200),
            ..Default::default()
        };
        assert!(bad.is_empty_range());

        // 相等是合法闭区间（A4：from == ts 与 to == ts 都要包含）。
        let equal = SafeBoxFilter {
            from_ts: Some(200),
            to_ts: Some(200),
            ..Default::default()
        };
        assert!(!equal.is_empty_range());

        // 只有单边时不判空。
        let only_from = SafeBoxFilter {
            from_ts: Some(999),
            ..Default::default()
        };
        assert!(!only_from.is_empty_range());
    }

    #[test]
    fn filter_default_is_no_filter() {
        let f = SafeBoxFilter::default();
        let (ids, from, to) = f.normalized();
        assert!(ids.is_empty());
        assert!(from.is_none() && to.is_none());
        assert!(!f.is_empty_range());
    }
}
