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
    list_pending_filtered(conn, session_id, None, page, page_size)
}

/// task-077：按盘筛选的分页列表（`drive` = `"C"` / `"c"` / `"C:"`，`None` = 全部）。
///
/// # 为什么新增函数而不是给 `list_pending_page` 加参数
///
/// 加参数会改公开签名，`engine/tests/list_pending_items.rs` 等一批用例的字面量
/// 全要跟着改——为一个可选的过滤条件付这个代价不值得。旧函数保持原样、转调
/// 本函数的 `drive=None` 分支，行为逐字不变。
///
/// # 为什么用 `drive_letter` 列而不是 `substr(path,1,1)`
///
/// v3 迁移（task-077）建了 `(session_id, drive_letter)` 组合索引。写成
/// `WHERE substr(path,1,1) = ?` 的话索引完全用不上，几千行的表要全扫。
///
/// 盘符归一化在这里做（`upper` + 去冒号）：前端下拉框的值是 `"C:"`，API 文档写
/// 的是 `"C"`，两种都会真的传进来。归一化放在 SQL 参数上而不是调用方，是为了让
/// 「传 `"c"` 也能查到」这件事只在一处成立。
pub fn list_pending_page_by_drive(
    conn: &rusqlite::Connection,
    session_id: Option<i64>,
    drive: Option<&str>,
    page: u32,
    page_size: u32,
) -> Result<PendingPage, String> {
    let normalized = drive.map(normalize_drive);
    // 归一化后是空串（如传了 ":"）→ 当作「不筛选」而不是「筛一个不存在的盘」，
    // 否则界面会显示 0 条且没有任何解释。
    let normalized = normalized.filter(|d| !d.is_empty());
    list_pending_filtered(conn, session_id, normalized.as_deref(), page, page_size)
}

/// `"C:"` / `"c"` → `"C"`；其余原样（大写）。
fn normalize_drive(raw: &str) -> String {
    raw.trim().trim_end_matches(':').to_ascii_uppercase()
}

fn list_pending_filtered(
    conn: &rusqlite::Connection,
    session_id: Option<i64>,
    drive: Option<&str>,
    page: u32,
    page_size: u32,
) -> Result<PendingPage, String> {
    let page = page.max(1);
    let page_size = page_size.clamp(1, MAX_PAGE_SIZE);
    let offset = i64::from(page - 1) * i64::from(page_size);

    // total 与 items 共用同一过滤条件（?1 / ?2），保证「共 N 条」和翻页范围一致。
    // drive_letter 是 v3 的派生列；NULL 的行（UNC / 相对路径）在按盘筛选时被排除，
    // 这与「它不属于任何盘」的语义一致。
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM cleanup_item
             WHERE (?1 IS NULL OR session_id = ?1)
               AND (?2 IS NULL OR drive_letter = ?2)",
            params![session_id, drive],
            |r| r.get(0),
        )
        .map_err(|e| format!("list_pending count: {e}"))?;

    let sql = "SELECT id, path, size_bytes, module_code, session_id, risk_level \
               FROM cleanup_item \
               WHERE (?1 IS NULL OR session_id = ?1) \
                 AND (?2 IS NULL OR drive_letter = ?2) \
               ORDER BY size_bytes DESC, id DESC \
               LIMIT ?3 OFFSET ?4";

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("list_pending prepare: {e}"))?;
    let rows = stmt
        .query_map(
            params![session_id, drive, i64::from(page_size), offset],
            |r| {
                Ok(PendingItem {
                    id: r.get(0)?,
                    path: r.get(1)?,
                    size_bytes: r.get(2)?,
                    module_code: r.get(3)?,
                    session_id: r.get(4)?,
                    risk_level: r.get(5)?,
                })
            },
        )
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// 内存库 + 迁移到最新 + 一条 session + 三条不同盘的条目。
    fn seeded() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrate::migrate(&conn).unwrap();
        conn.execute(
            "INSERT INTO scan_session (id, started_at, status, scope) VALUES (1, 1000, 'running', 'full')",
            [],
        )
        .unwrap();
        for (path, size) in [
            (r"C:/temp/a.tmp", 300),
            (r"c:/temp/b.tmp", 200),
            (r"D:/cache/c.tmp", 100),
            // UNC：drive_letter 为 NULL，按盘筛选时应被排除
            (r"\\server\share\d.tmp", 50),
        ] {
            conn.execute(
                "INSERT INTO cleanup_item (session_id, module_code, category, path, size_bytes, risk_level, description)
                 VALUES (1, 'temp', 'user_temp', ?1, ?2, 'L1', '')",
                rusqlite::params![path, size],
            )
            .unwrap();
        }
        conn
    }

    #[test]
    fn no_drive_filter_returns_everything() {
        let conn = seeded();
        let p = list_pending_page(&conn, None, 1, 100).unwrap();
        assert_eq!(p.total, 4, "不筛盘时 4 条全在（含 UNC）");
        assert_eq!(p.items.len(), 4);
    }

    #[test]
    fn drive_filter_matches_case_and_colon_variants() {
        let conn = seeded();
        // "C" / "c" / "C:" 三种写法都必须命中同一个盘的两条
        for raw in ["C", "c", "C:", "c:"] {
            let p = list_pending_page_by_drive(&conn, None, Some(raw), 1, 100).unwrap();
            assert_eq!(p.total, 2, "{raw:?} 应命中 C 盘 2 条");
            assert!(p
                .items
                .iter()
                .all(|i| i.path.to_lowercase().starts_with("c:")));
        }
        let d = list_pending_page_by_drive(&conn, None, Some("D"), 1, 100).unwrap();
        assert_eq!(d.total, 1);
        assert_eq!(d.items[0].size_bytes, 100);
    }

    #[test]
    fn drive_filter_excludes_unc_rows() {
        // drive_letter 为 NULL 的行（UNC）不属于任何盘，按盘筛时必须消失——
        // 否则「只看 C 盘」的列表里混进一条 \\server\share，用户会以为筛错了。
        let conn = seeded();
        let p = list_pending_page_by_drive(&conn, None, Some("C"), 1, 100).unwrap();
        assert!(
            p.items.iter().all(|i| !i.path.starts_with(r"\\")),
            "UNC 条目不该出现在按盘筛选结果里"
        );
    }

    #[test]
    fn blank_drive_means_no_filter() {
        // 传 ":" / "" / "  " 这类归一化后为空的盘符 → 当作不筛选，
        // 而不是「筛一个不存在的盘」导致列表空白且没有解释
        let conn = seeded();
        for raw in [":", "", "   "] {
            let p = list_pending_page_by_drive(&conn, None, Some(raw), 1, 100).unwrap();
            assert_eq!(p.total, 4, "{raw:?} 应退化为不筛选");
        }
    }

    #[test]
    fn drive_filter_keeps_pagination_consistent() {
        // total 必须与 items 用同一过滤条件：否则「共 N 条」与翻页范围对不上，
        // 用户翻到第 2 页看到空列表
        let conn = seeded();
        let p = list_pending_page_by_drive(&conn, None, Some("C"), 1, 1).unwrap();
        assert_eq!(p.total, 2, "total 应是过滤后的总数");
        assert_eq!(p.items.len(), 1, "每页 1 条");
        let p2 = list_pending_page_by_drive(&conn, None, Some("C"), 2, 1).unwrap();
        assert_eq!(p2.items.len(), 1, "第 2 页应还有一条");
        assert_ne!(p.items[0].id, p2.items[0].id, "两页不能是同一条");
    }

    #[test]
    fn drive_filter_combines_with_session_filter() {
        let conn = seeded();
        // 不存在的 session → 0 条（两个条件取交集，不是并集）
        let p = list_pending_page_by_drive(&conn, Some(99), Some("C"), 1, 100).unwrap();
        assert_eq!(p.total, 0);
        let ok = list_pending_page_by_drive(&conn, Some(1), Some("C"), 1, 100).unwrap();
        assert_eq!(ok.total, 2);
    }
}
