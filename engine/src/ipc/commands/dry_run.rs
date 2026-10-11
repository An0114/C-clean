//! `cleanup.dryRun` 命令纯逻辑：itemIds → 路径 → 跑 task-003 白名单。

use std::collections::BTreeMap;

use super::super::envelope::Envelope;
use super::super::error_codes::{DRYRUN_EMPTY, WHITELIST_DENIED};
use crate::execute::normalize::normalize;
use crate::execute::whitelist::classify;

/// 解析后的条目（resolver 的输出）。
#[derive(Debug, Clone)]
pub struct ResolvedItem {
    pub item_id: i64,
    pub path: String,
    pub module_code: String,
    pub category: String,
    pub size_bytes: u64,
}

/// itemIds → 路径元信息解析器 trait（真实实现是后续 DB 仓储；本任务给内存 Mock）。
pub trait ItemPathResolver {
    /// 按 item_ids 取条目；找不到的 id 直接跳过。
    fn resolve(&self, item_ids: &[i64]) -> Vec<ResolvedItem>;
}

/// 内存 Mock resolver：构造时塞入一批条目，按 ids 过滤返回。
pub struct MockResolver {
    items: Vec<ResolvedItem>,
}

impl MockResolver {
    /// 用一批预置条目构造。
    pub fn new(items: Vec<ResolvedItem>) -> Self {
        Self { items }
    }
}

impl ItemPathResolver for MockResolver {
    fn resolve(&self, item_ids: &[i64]) -> Vec<ResolvedItem> {
        self.items
            .iter()
            .filter(|it| item_ids.contains(&it.item_id))
            .cloned()
            .collect()
    }
}

/// `cleanup.dryRun` 入参（前端传 itemIds）。
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRunReq {
    pub item_ids: Vec<i64>,
}

/// 按模块+类别聚合的一行。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownItem {
    pub module_code: String,
    pub category: String,
    pub bytes: u64,
}

/// 被 L3 拦下的一行。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockedItem {
    pub item_id: i64,
    pub path: String,
    /// DenialReason 的 Debug 字符串，如 "L3PersonalDir"。
    pub denial_reason: String,
}

/// `cleanup.dryRun` 出参。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRunResp {
    pub total_items: u64,
    pub total_bytes: u64,
    pub breakdown: Vec<BreakdownItem>,
    pub blocked_by_l3: Vec<BlockedItem>,
}

/// 处理 cleanup.dryRun：空清单 → 2001；任一命中 L3 → 2002（附 blocked 明细）；
/// 全过 → 聚合返回。
pub fn handle_dry_run(
    req: DryRunReq,
    resolver: &dyn ItemPathResolver,
) -> Envelope<DryRunResp> {
    if req.item_ids.is_empty() {
        return Envelope::err(DRYRUN_EMPTY);
    }
    let resolved = resolver.resolve(&req.item_ids);
    if resolved.is_empty() {
        return Envelope::err(DRYRUN_EMPTY);
    }

    // 逐条跑 task-003 白名单；规范化失败按 PathTraversal 记为被拦。
    let mut blocked: Vec<BlockedItem> = Vec::new();
    let mut pass: Vec<&ResolvedItem> = Vec::new();
    for item in &resolved {
        match normalize(&item.path) {
            Ok(np) => match classify(&np) {
                Ok(()) => pass.push(item),
                Err(reason) => blocked.push(BlockedItem {
                    item_id: item.item_id,
                    path: item.path.clone(),
                    denial_reason: format!("{reason:?}"),
                }),
            },
            Err(_) => blocked.push(BlockedItem {
                item_id: item.item_id,
                path: item.path.clone(),
                denial_reason: "PathTraversal".to_string(),
            }),
        }
    }

    if !blocked.is_empty() {
        // 对 TRD §5.1「失败 data=null」的显式偏离：L3 拒绝必须回传 blocked 清单。
        return Envelope::err_with_data(
            WHITELIST_DENIED,
            DryRunResp {
                total_items: 0,
                total_bytes: 0,
                breakdown: vec![],
                blocked_by_l3: blocked,
            },
        );
    }

    let total_items = pass.len() as u64;
    let total_bytes: u64 = pass.iter().map(|it| it.size_bytes).sum();
    let mut agg: BTreeMap<(String, String), u64> = BTreeMap::new();
    for item in &pass {
        *agg.entry((item.module_code.clone(), item.category.clone()))
            .or_insert(0) += item.size_bytes;
    }
    let breakdown = agg
        .into_iter()
        .map(|((module_code, category), bytes)| BreakdownItem {
            module_code,
            category,
            bytes,
        })
        .collect();

    Envelope::ok(DryRunResp {
        total_items,
        total_bytes,
        breakdown,
        blocked_by_l3: vec![],
    })
}
