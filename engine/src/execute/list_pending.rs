//! task-029：列出待清理的 cleanup_item 行。
//!
//! **与任务书 schema 偏差**：v1__init.sql 的 `cleanup_item` 表**没有 status 列、也没有 scanned_at 列**
//! （真实列为 id/session_id/module_code/category/path/size_bytes/risk_level/...）。
//! 因此「pending」就是「表里所有行」——扫描写入即可见，cleanup.execute 不会删行（task-015 边界）。
//! 偏差点：
//! - 无 `WHERE status='pending'`；
//! - PendingItem 去掉 `scanned_at` 字段（无此列）；
//! - 清理后行仍在 DB（历史快照），前端若需「未清理」需 JOIN audit_log，留后续。
//!
//! task-052 G2：从「裸 Vec + limit/offset」升级为分页响应 `PendingPage`。
//! 触发原因是走查发现前端硬编码 `limit: 100`，第 101 条之后的条目（比如 %TEMP% 里被锁的
//! 小文件）在 UI 上根本找不到。分页 + `total` 之后，用户能翻页找到任意条目。

use rusqlite::params;

/// 一条待清理条目（前端 Items 表格行）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingItem {
    pub id: i64,
    pub path: String,
    pub size_bytes: i64,
    pub module_code: String,
    pub session_id: i64,
    /// task-064：风险等级 L0/L1/L2/L3，前端据此渲染徽标与 hover 说明。
    ///
    /// 之前没返回这个字段，前端只能「所有项一视同仁」——无法做「全选 L1」，
    /// 也无法告诉用户「这条删了会怎样」。
    pub risk_level: String,
}

/// 分页响应（task-052 G2，对齐 task-045 的 `SafeBoxPage` 模式）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingPage {
    /// 当前页条目。
    pub items: Vec<PendingItem>,
    /// 满足过滤条件的总条数（不受分页影响）。
    pub total: i64,
    /// 当前页码，从 1 起。
    pub page: u32,
    /// 每页条数（已 clamp 到 [1, 500]）。
    pub page_size: u32,
}

/// 每页条数上限：500。超过按 500 算，避免一次拉爆内存 / 卡死渲染。
pub const MAX_PAGE_SIZE: u32 = 500;
/// 默认每页条数：100（与走查前的前端硬编码一致，避免默认行为突变）。
pub const DEFAULT_PAGE_SIZE: u32 = 100;

/// 按 session_id（可选）分页列出 cleanup_item，`size_bytes DESC, id DESC`。
///
/// 排序里补 `id DESC` 是 task-052 G2 的关键：原 SQL 只按 `size_bytes DESC` 排，
/// 大小相同的行顺序未定义，SQLite 在不同 LIMIT/OFFSET 下可能给出不同顺序 →
/// 翻页时同一条目重复出现或整条漏掉（「跳行」）。
pub fn list_pending_page(
    conn: &rusqlite::Connection,
    session_id: Option<i64>,
    page: u32,
    page_size: u32,
) -> Result<PendingPage, String> {
    let page = page.max(1);
    let page_size = page_size.clamp(1, MAX_PAGE_SIZE);
    let offset = i64::from(page - 1) * i64::from(page_size);

    // total 与 items 共用同一过滤条件（?1），保证「共 N 条」和翻页范围一致。
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM cleanup_item WHERE (?1 IS NULL OR session_id = ?1)",
            params![session_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("list_pending count: {e}"))?;

    let sql = "SELECT id, path, size_bytes, module_code, session_id, risk_level \
               FROM cleanup_item \
               WHERE (?1 IS NULL OR session_id = ?1) \
               ORDER BY size_bytes DESC, id DESC \
               LIMIT ?2 OFFSET ?3";

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("list_pending prepare: {e}"))?;
    let rows = stmt
        .query_map(params![session_id, i64::from(page_size), offset], |r| {
            Ok(PendingItem {
                id: r.get(0)?,
                path: r.get(1)?,
                size_bytes: r.get(2)?,
                module_code: r.get(3)?,
                session_id: r.get(4)?,
                risk_level: r.get(5)?,
            })
        })
        .map_err(|e| format!("list_pending query: {e}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("list_pending row: {e}"))?);
    }

    Ok(PendingPage {
        items,
        total,
        page,
        page_size,
    })
}
