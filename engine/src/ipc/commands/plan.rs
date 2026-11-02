//! `plan.*` 命令纯逻辑（task-072，P1-7 清理方案模板）。
//!
//! 与 `residual` / `big_files` 同样的分工：真实 Tauri command 在
//! `app/src-tauri/src/commands.rs`（需要 State），这里只做「读库 + 装信封」，
//! 单测可以直接对临时 DB 调用。
//!
//! # 为什么方案存「三元组」而不是 item id
//!
//! `cleanup_plan.item_refs` 的 schema 注释写得很清楚：
//! 「不绑定 session_id（item_refs 存三元组 JSON），保证模板跨扫描复用」。
//!
//! 这是对的：`cleanup_item.id` 每次扫描都是新的一批，用它存方案，下次扫描后
//! 整份方案就全部失效。所以存 `{module_code, category, path}`，应用时按三元组
//! **回查当前库**拿 id。
//!
//! # 回查为什么取「最新一条」
//!
//! `cleanup_item` 会跨扫描累积（同一路径可能有多行：这次扫到、上次也扫到）。
//! 一条引用对应多行时取 `id` 最大的那条（最近一次扫描写入的），否则方案里 12 条
//! 会在界面上勾出 30 个勾选框——用户会以为方案存坏了。
//!
//! 取不到的引用不静默丢弃，放进 `unmatched` 让前端提示「方案里有 N 条本次没找到」
//! （可能是被清掉了、也可能是这个模块这次没扫到）。**悄悄少勾几条比报错更糟**。

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

use super::super::envelope::Envelope;
use crate::ipc::error_codes::{DRYRUN_EMPTY, INTERNAL};

/// 导出文件的格式标记。导入时校验，防止把别的 JSON 当方案导进来。
pub const PLAN_FORMAT: &str = "cclean-plan";
/// 导出文件格式版本。字段不兼容时递增，导入侧拒绝未知版本。
pub const PLAN_FORMAT_VERSION: u32 = 1;

/// 方案里的一条引用（三元组，跨扫描稳定）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanItemRef {
    /// 模块代码（A/B/C/D/E/F 或 temp / prefetch 这类）。
    pub module_code: String,
    /// 规则库类别键（如 `user_temp`）。
    pub category: String,
    /// 目标绝对路径。
    pub path: String,
}

/// 方案列表项。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummary {
    pub id: i64,
    pub name: String,
    pub note: Option<String>,
    /// 方案里存了多少条引用（稳定值）。
    pub item_count: i64,
    /// 本次在库里匹配到多少条（可能小于 `item_count`）。
    pub matched_count: i64,
    /// 匹配到的条目大小之和；0 表示本次一条都没匹配上。
    pub size_bytes: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 应用方案的结果。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanApplied {
    pub id: i64,
    pub name: String,
    /// 本次匹配到的 `cleanup_item.id`，前端直接拿去勾选。
    pub item_ids: Vec<i64>,
    /// 本次没匹配到的路径（已去重）。
    pub unmatched: Vec<String>,
    /// 匹配到的条目大小之和。
    pub size_bytes: i64,
}

/// 导出/导入用的文件内容。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanFile {
    pub format: String,
    pub version: u32,
    pub name: String,
    pub note: Option<String>,
    /// 导出时间（unix 秒），仅作说明，导入时忽略。
    pub exported_at: i64,
    pub item_refs: Vec<PlanItemRef>,
}

// ===================== 内部工具 =====================

/// 读一个方案的名字/备注/引用列表。方案不存在时报错。
fn load_plan(
    conn: &Connection,
    plan_id: i64,
) -> Result<(String, Option<String>, Vec<PlanItemRef>), String> {
    let row: Option<(String, Option<String>, String)> = conn
        .query_row(
            "SELECT name, note, item_refs FROM cleanup_plan WHERE id=?1",
            [plan_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|e| format!("query cleanup_plan: {e}"))?;
    let (name, note, refs_json) = row.ok_or_else(|| format!("方案不存在：id={plan_id}"))?;
    let refs: Vec<PlanItemRef> = serde_json::from_str(&refs_json)
        .map_err(|e| format!("方案 id={plan_id} 的 item_refs 不是合法 JSON：{e}"))?;
    Ok((name, note, refs))
}

/// 按三元组回查当前库，返回 `(命中的 item_id 列表, 未命中的路径列表, 命中总大小)`。
///
/// 每条引用只取 `id` 最大的那一行——`cleanup_item` 跨扫描累积，同路径可能有多行，
/// 全取会让方案里的 12 条变成界面上 30 个勾选。
fn match_current_items(
    conn: &Connection,
    refs: &[PlanItemRef],
) -> Result<(Vec<i64>, Vec<String>, i64), String> {
    let mut ids = Vec::with_capacity(refs.len());
    let mut unmatched: Vec<String> = Vec::new();
    let mut total: i64 = 0;

    for r in refs {
        let hit: Option<(i64, i64)> = conn
            .query_row(
                "SELECT id, size_bytes FROM cleanup_item
                  WHERE module_code=?1 AND category=?2 AND path=?3
                  ORDER BY id DESC LIMIT 1",
                params![r.module_code, r.category, r.path],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| format!("match cleanup_item: {e}"))?;

        match hit {
            Some((id, size)) => {
                ids.push(id);
                total = total.saturating_add(size.max(0));
            }
            None => {
                if !unmatched.iter().any(|p| p == &r.path) {
                    unmatched.push(r.path.clone());
                }
            }
        }
    }
    Ok((ids, unmatched, total))
}

/// 文件名里不能出现的字符换成下划线。
///
/// 方案名是用户随手起的（可能带 `/`、`:`、`?`），直接拼进路径会跑到别的目录去
/// 或者创建失败。保留字母/数字/CJK，其余一律替换。
fn sanitize_filename(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
            out.push(c);
        } else {
            out.push('_');
        }
    }
    let trimmed = out.trim().to_string();
    if trimmed.is_empty() {
        "plan".to_string()
    } else {
        trimmed
    }
}

/// 当前 unix 秒。
fn unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ===================== 单元 1：save / list / delete =====================

/// 把「当前勾选的条目」保存成命名方案。
///
/// `item_ids` 是 `cleanup_item.id`：前端只持有 id（`PendingItem` 里没有 category，
/// 前端拼不出完整三元组），由后端反查三元组更可靠——也避免前端传来一份对不上的
/// 数据。已被清掉/不存在的 id 会被跳过；一条都取不到时报错。
pub fn save_plan(conn: &Connection, name: &str, item_ids: &[i64], now: i64) -> Result<i64, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("方案名不能为空".to_string());
    }
    if item_ids.is_empty() {
        return Err("没有选中任何条目".to_string());
    }

    // 1. 反查三元组 + 去重（同一路径重复勾选时只存一条）
    let mut refs: Vec<PlanItemRef> = Vec::with_capacity(item_ids.len());
    for id in item_ids {
        let row: Option<(String, String, String)> = conn
            .query_row(
                "SELECT module_code, category, path FROM cleanup_item WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| format!("query cleanup_item id={id}: {e}"))?;
        let Some((module_code, category, path)) = row else {
            continue; // 条目已被清掉，跳过
        };
        if refs
            .iter()
            .any(|r| r.module_code == module_code && r.category == category && r.path == path)
        {
            continue;
        }
        refs.push(PlanItemRef {
            module_code,
            category,
            path,
        });
    }
    if refs.is_empty() {
        return Err("选中的条目已不存在（可能刚被清理过），无法保存".to_string());
    }

    let json = serde_json::to_string(&refs).map_err(|e| format!("序列化 item_refs: {e}"))?;
    conn.execute(
        "INSERT INTO cleanup_plan (name, note, is_template, item_refs, created_at, updated_at)
         VALUES (?1, NULL, 0, ?2, ?3, ?3)",
        params![name, json, now],
    )
    .map_err(|e| format!("insert cleanup_plan: {e}"))?;
    Ok(conn.last_insert_rowid())
}

/// 列出所有方案，附带「本次匹配到多少条 / 多大」。
///
/// `matched_count` 与 `item_count` 分开给：方案存了 12 条、这次只匹配到 3 条，
/// 界面必须能说清「12 条（本次匹配 3 条）」，而不是显示一个让人误以为丢数据的 3。
pub fn list_plans(conn: &Connection) -> Result<Vec<PlanSummary>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, note, item_refs, created_at, updated_at
               FROM cleanup_plan ORDER BY updated_at DESC, id DESC",
        )
        .map_err(|e| format!("prepare list cleanup_plan: {e}"))?;

    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
            ))
        })
        .map_err(|e| format!("query list cleanup_plan: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        let (id, name, note, refs_json, created_at, updated_at) =
            row.map_err(|e| format!("iter cleanup_plan: {e}"))?;
        // 单条方案 JSON 坏了不该让整页列表挂掉：跳过它并打日志。
        let refs: Vec<PlanItemRef> = match serde_json::from_str(&refs_json) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[plan_list] 方案 id={id} 的 item_refs 解析失败，已跳过：{e}");
                continue;
            }
        };
        let (ids, _unmatched, size) = match_current_items(conn, &refs)?;
        out.push(PlanSummary {
            id,
            name,
            note,
            item_count: refs.len() as i64,
            matched_count: ids.len() as i64,
            size_bytes: size,
            created_at,
            updated_at,
        });
    }
    Ok(out)
}

/// 删除方案。返回被删的 id（便于前端确认）。
pub fn delete_plan(conn: &Connection, plan_id: i64) -> Result<i64, String> {
    let n = conn
        .execute("DELETE FROM cleanup_plan WHERE id=?1", [plan_id])
        .map_err(|e| format!("delete cleanup_plan: {e}"))?;
    if n == 0 {
        return Err(format!("方案不存在：id={plan_id}"));
    }
    Ok(plan_id)
}

// ===================== 单元 2：apply / export / import =====================

/// 应用方案：把三元组映射成**当前**的 `cleanup_item.id`。
///
/// 返回的 `item_ids` 前端直接拿去 `setSelected(new Set(...))`；
/// `unmatched` 让前端提示「有 N 条本次没找到」。
pub fn apply_plan(conn: &Connection, plan_id: i64) -> Result<PlanApplied, String> {
    let (name, _note, refs) = load_plan(conn, plan_id)?;
    let (item_ids, unmatched, size_bytes) = match_current_items(conn, &refs)?;
    Ok(PlanApplied {
        id: plan_id,
        name,
        item_ids,
        unmatched,
        size_bytes,
    })
}

/// 导出一个方案为 JSON 文件，返回落盘后的绝对路径。
///
/// # 为什么写到固定目录而不是弹「另存为」
///
/// 本项目**没有引 `tauri-plugin-dialog`**（安装包体积 + 权限面都要涨），
/// 所以没有系统文件选择框。沿用 `report_export` 的做法：写到
/// `<data_root>/plans/exports/`，把完整路径返回给界面展示——用户可以复制，
/// 也可以直接在那个目录里看到文件。导入侧同样让用户把路径粘回来。
pub fn export_plan(
    conn: &Connection,
    plan_id: i64,
    export_dir: &Path,
    now: i64,
) -> Result<PathBuf, String> {
    let (name, note, refs) = load_plan(conn, plan_id)?;
    let file = PlanFile {
        format: PLAN_FORMAT.to_string(),
        version: PLAN_FORMAT_VERSION,
        name: name.clone(),
        note,
        exported_at: now,
        item_refs: refs,
    };
    let json = serde_json::to_string_pretty(&file).map_err(|e| format!("序列化方案: {e}"))?;

    std::fs::create_dir_all(export_dir)
        .map_err(|e| format!("创建导出目录 {} 失败: {e}", export_dir.display()))?;
    let filename = format!("{}-{}.json", sanitize_filename(&name), now);
    let path = export_dir.join(filename);
    std::fs::write(&path, json).map_err(|e| format!("写导出文件 {} 失败: {e}", path.display()))?;
    Ok(path)
}

/// 从 JSON 文件导入方案，返回新建的方案 id。
///
/// # 校验
///
/// - `format` 必须是 `cclean-plan`、`version` 必须是已知版本——否则用户误选一个
///   普通 JSON 也会「导入成功」出一条空方案；
/// - `itemRefs` 为空视为无效文件（存下来也没用）；
/// - 名字为空时用文件名兜底，不因为一个字段就整份拒绝。
pub fn import_plan(conn: &Connection, file_path: &Path, now: i64) -> Result<i64, String> {
    let raw = std::fs::read_to_string(file_path)
        .map_err(|e| format!("读文件 {} 失败: {e}", file_path.display()))?;
    let file: PlanFile = serde_json::from_str(&raw)
        .map_err(|e| format!("不是合法的方案文件（JSON 解析失败）：{e}"))?;

    if file.format != PLAN_FORMAT {
        return Err(format!(
            "不是方案文件（format={:?}，期望 {PLAN_FORMAT:?}）",
            file.format
        ));
    }
    if file.version != PLAN_FORMAT_VERSION {
        return Err(format!(
            "方案文件版本不支持（version={}，本版本只认 {PLAN_FORMAT_VERSION}）",
            file.version
        ));
    }
    if file.item_refs.is_empty() {
        return Err("方案文件里没有任何条目，已拒绝导入".to_string());
    }

    let name = if file.name.trim().is_empty() {
        file_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "导入的方案".to_string())
    } else {
        file.name.trim().to_string()
    };

    let json =
        serde_json::to_string(&file.item_refs).map_err(|e| format!("序列化 item_refs: {e}"))?;
    conn.execute(
        "INSERT INTO cleanup_plan (name, note, is_template, item_refs, created_at, updated_at)
         VALUES (?1, ?2, 0, ?3, ?4, ?4)",
        params![name, file.note, json, now],
    )
    .map_err(|e| format!("insert cleanup_plan: {e}"))?;
    Ok(conn.last_insert_rowid())
}

// ===================== IPC handler（装信封） =====================

/// 处理 `plan.save`。
///
/// 空选择返回 **2001**（`DRYRUN_EMPTY`，「待清理清单为空」）：语义最接近，
/// 前端据此提示「先勾选要保存的条目」。其余失败（名字为空 / 条目已消失 /
/// 写库失败）走 `Err(String)`，原因要能显示给用户。
pub fn handle_plan_save(
    conn: &Connection,
    name: &str,
    item_ids: &[i64],
    now: i64,
) -> Result<Envelope<i64>, String> {
    if item_ids.is_empty() {
        return Ok(Envelope::err(DRYRUN_EMPTY));
    }
    let id = save_plan(conn, name, item_ids, now)?;
    Ok(Envelope::ok(id))
}

/// 处理 `plan.list`。读失败走 `Err`，不返回「空列表」假装成功。
pub fn handle_plan_list(conn: &Connection) -> Result<Envelope<Vec<PlanSummary>>, String> {
    Ok(Envelope::ok(list_plans(conn)?))
}

/// 处理 `plan.delete`。
pub fn handle_plan_delete(conn: &Connection, plan_id: i64) -> Result<Envelope<i64>, String> {
    Ok(Envelope::ok(delete_plan(conn, plan_id)?))
}

/// 处理 `plan.apply`。
pub fn handle_plan_apply(conn: &Connection, plan_id: i64) -> Result<Envelope<PlanApplied>, String> {
    Ok(Envelope::ok(apply_plan(conn, plan_id)?))
}

/// 处理 `plan.export`。返回落盘路径。
pub fn handle_plan_export(
    conn: &Connection,
    plan_id: i64,
    export_dir: &Path,
    now: i64,
) -> Result<Envelope<String>, String> {
    if export_dir.as_os_str().is_empty() {
        // 空目录会把 JSON 写到进程当前工作目录，必须挡在门外。
        eprintln!("[plan_export] export_dir 为空，拒绝执行");
        return Ok(Envelope::err(INTERNAL));
    }
    let p = export_plan(conn, plan_id, export_dir, now)?;
    Ok(Envelope::ok(p.to_string_lossy().to_string()))
}

/// 处理 `plan.import`。返回新方案 id。
pub fn handle_plan_import(
    conn: &Connection,
    file_path: &str,
    now: i64,
) -> Result<Envelope<i64>, String> {
    if file_path.trim().is_empty() {
        return Err("请填写要导入的 JSON 文件路径".to_string());
    }
    let id = import_plan(conn, Path::new(file_path.trim()), now)?;
    Ok(Envelope::ok(id))
}

/// 供调用方取「现在」（避免各处重复写 SystemTime 样板）。
pub fn now_secs() -> i64 {
    unix_secs()
}
