//! `cleanup.dryRun` 命令纯逻辑：itemIds → 路径 → 跑 task-003 白名单。

use std::collections::BTreeMap;

use super::super::envelope::Envelope;
use super::super::error_codes::{DRYRUN_EMPTY, WHITELIST_DENIED};
use crate::execute::normalize::normalize;
use crate::execute::resolve::{is_symlink_or_junction, long_path_name};
use crate::execute::whitelist::classify_with_external;

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
    /// DenialReason 的 Debug 字符串，如 "BuiltinRedline(L3PersonalDir)"、"ExternalRedline"。
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

/// 处理 cleanup.dryRun：空清单 → 2001；任一命中（内置 L3 ∪ 外部红线）→ 2002（附 blocked 明细）；
/// 全过 → 聚合返回。`external_deny` 为已展开的绝对路径黑名单（来自 rules JSON）。
pub fn handle_dry_run(
    req: DryRunReq,
    resolver: &dyn ItemPathResolver,
    external_deny: &[String],
) -> Envelope<DryRunResp> {
    if req.item_ids.is_empty() {
        return Envelope::err(DRYRUN_EMPTY);
    }
    let resolved = resolver.resolve(&req.item_ids);
    if resolved.is_empty() {
        return Envelope::err(DRYRUN_EMPTY);
    }

    // 逐条跑 task-003 白名单（内置 L3 ∪ 外部红线）；规范化失败按 PathTraversal 记为被拦。
    let mut blocked: Vec<BlockedItem> = Vec::new();
    let mut pass: Vec<&ResolvedItem> = Vec::new();
    for item in &resolved {
        // task-086 2.3：先展开 8.3 短名成 8.3 长名，再做红线匹配。
        let long_path = long_path_name(&item.path);
        // task-086 2.2：待删目标本身是 symlink/junction → 预检即拦（code 2009），
        // 与 execute 阶段的真实删除决策保持一致。
        if is_symlink_or_junction(&long_path) {
            blocked.push(BlockedItem {
                item_id: item.item_id,
                path: item.path.clone(),
                denial_reason: "SymlinkRefused".to_string(),
            });
            continue;
        }
        match normalize(&long_path) {
            Ok(np) => match classify_with_external(&np, external_deny) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn item(item_id: i64, path: &str) -> ResolvedItem {
        ResolvedItem {
            item_id,
            path: path.to_string(),
            module_code: "cache".to_string(),
            category: "temp".to_string(),
            size_bytes: 10,
        }
    }

    #[test]
    fn symlink_target_is_blocked_in_dry_run() {
        // Windows + 有权限创建 symlink 时才真正断言；其他平台/环境跳过（逻辑仍覆盖）。
        #[cfg(windows)]
        {
            use std::os::windows::fs::symlink_dir;

            let base = std::env::temp_dir().join(format!("trae_drylink_{}", std::process::id()));
            let target = base.join("real");
            let link = base.join("alias");
            std::fs::create_dir_all(&target).unwrap();
            if symlink_dir(&target, &link).is_ok() {
                let resolver = MockResolver::new(vec![item(1, &link.to_string_lossy())]);
                let resp = handle_dry_run(DryRunReq { item_ids: vec![1] }, &resolver, &[]);
                assert_ne!(resp.code, 0);
                assert_eq!(resp.code, 2002);
                let data = resp.data.expect("blocked list attached");
                assert_eq!(data.blocked_by_l3.len(), 1);
                assert_eq!(data.blocked_by_l3[0].denial_reason, "SymlinkRefused");
            }
            let _ = std::fs::remove_dir_all(&base);
        }
    }

    #[test]
    fn plain_temp_path_passes_dry_run() {
        let resolver = MockResolver::new(vec![item(1, r"C:\Windows\Temp\update.log")]);
        let resp = handle_dry_run(DryRunReq { item_ids: vec![1] }, &resolver, &[]);
        assert_eq!(resp.code, 0);
        assert_eq!(resp.data.map(|d| d.total_items), Some(1));
    }
}
