//! task-086 1.3：目标空间推荐组合（PRD P1-8）。
//!
//! 核心交互「输入目标空闲空间 → 自动推荐勾选组合」：
//! - 候选只在 **L1 + L2**（PRD 口径：优先 L1，不足再纳入 L2）。
//!   L0 不入推荐（太琐碎、数量大，交给「一键清 L0」）；L3 是红线，绝不入候选。
//! - 贪心：按大小倒序，先吃安全的 L1，仍不够才吃 L2（L2 在 UI 上仍可微调取消）。
//! - 这是**推荐**，不是自动删除——前端只是预勾选，真正的删除仍走用户手动确认的
//!   `cleanup_execute` 那条链路（含逐级风险闸 + 还原点）。
//!
//! `recommend_plan(conn, target_bytes)` 是纯函数 + 一次只读查询，不产生任何副作用。

use rusqlite::Connection;

/// 一次推荐计划的完整结果（serde camelCase 对齐前端）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendPlan {
    /// 推荐勾选的 cleanup_item id（L1 优先，不足再补 L2）。
    pub item_ids: Vec<i64>,
    /// 推荐组合里来自 L1 的预估释放字节。
    pub l1_bytes: i64,
    /// 库中全部 L1 项可释放字节（用于「就算全勾也不够」的提示）。
    pub l1_total_bytes: i64,
    /// 推荐组合里来自 L2 的预估释放字节。
    pub l2_bytes: i64,
    /// 库中全部 L2 项可释放字节。
    pub l2_total_bytes: i64,
    /// 推荐组合预估释放字节（= l1_bytes + l2_bytes）。
    pub planned_bytes: i64,
    /// 用户设定的目标字节（target_mb 换算）。
    pub target_bytes: i64,
    /// 安全项是否足以达成目标（planned_bytes >= target_bytes）。
    pub achieved: bool,
    /// 未能达成时差的目标字节（target - 全部安全项可释放）；达成时为 0。
    pub shortfall_bytes: i64,
    /// 给前端的一句话提示（含「距目标还差 X」这类差额说明）。
    pub message: String,
}

/// 计算一次目标空间推荐。
///
/// `target_bytes`：用户目标（字节）。返回 [`RecommendPlan`]。
///
/// 贪心策略的取舍：不要背包最优解——用户目标是「达到某空闲空间」，不是「精确
/// 恢复到刚好」；从小到大吃反而可能吃掉一堆小文件凑数。按大小倒序先吃大的，
/// 用最少的条目逼近目标，列表也更好读。算法正确性不要求全局最优，够用即可。
pub fn recommend_plan(conn: &Connection, target_bytes: i64) -> Result<RecommendPlan, String> {
    let candidates = fetch_candidates(conn)?;

    let l1_total: i64 = candidates
        .iter()
        .filter(|c| c.risk_level == "L1")
        .map(|c| c.size_bytes)
        .sum();
    let l2_total: i64 = candidates
        .iter()
        .filter(|c| c.risk_level == "L2")
        .map(|c| c.size_bytes)
        .sum();

    let target = target_bytes.max(0);
    let (item_ids, l1_bytes, l2_bytes) = pick(&candidates, target);
    let planned = l1_bytes + l2_bytes;
    let achieved = planned >= target;
    let shortfall = if achieved {
        0
    } else {
        (target - planned).max(0)
    };

    let message = if achieved {
        let extra = planned - target;
        format!(
            "推荐 {n} 项可释放 {planned}，超过目标 {extra}",
            n = item_ids.len(),
            planned = bytes(planned),
            extra = bytes(extra),
        )
    } else {
        format!(
            "仅安全项最多可释放 {total}（其中 L2 {l2}），距目标还差 {gap}",
            total = bytes(l1_total + l2_total),
            l2 = bytes(l2_total),
            gap = bytes(shortfall),
        )
    };

    Ok(RecommendPlan {
        item_ids,
        l1_bytes,
        l2_bytes,
        l1_total_bytes: l1_total,
        l2_total_bytes: l2_total,
        planned_bytes: planned,
        target_bytes: target,
        achieved,
        shortfall_bytes: shortfall,
        message,
    })
}

/// 查询候选：所有 L1 / L2 的 cleanup_item（按大小倒序，大的在前）。
fn fetch_candidates(conn: &Connection) -> Result<Vec<Candidate>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, size_bytes, risk_level FROM cleanup_item
             WHERE risk_level IN ('L1', 'L2')
             ORDER BY size_bytes DESC, id DESC",
        )
        .map_err(|e| format!("prepare recommend candidates: {e}"))?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Candidate {
                id: r.get(0)?,
                size_bytes: r.get(1)?,
                risk_level: r.get::<_, String>(2)?.to_ascii_uppercase(),
            })
        })
        .map_err(|e| format!("query recommend candidates: {e}"))?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| format!("read recommend candidate row: {e}"))?);
    }
    // 保险：库里出现既不是 L1 也不是 L2 的异常值（理论上被 WHERE 挡住了）就不要。
    out.retain(|c| c.risk_level == "L1" || c.risk_level == "L2");
    Ok(out)
}

/// 一条推荐的候选清理项。
struct Candidate {
    id: i64,
    size_bytes: i64,
    /// 归一化成大写后的等级（"L1" / "L2"）。
    risk_level: String,
}

/// 贪心挑选：第一轮只吃 L1 直到达标，第二轮才补 L2。
///
/// 返回 `(item_ids, l1_bytes, l2_bytes)`。input 已按大小倒序，两轮都从头扫。
fn pick(candidates: &[Candidate], target: i64) -> (Vec<i64>, i64, i64) {
    let mut ids = Vec::new();
    let mut l1 = 0i64;
    let mut l2 = 0i64;

    for c in candidates.iter().filter(|c| c.risk_level == "L1") {
        if l1 >= target {
            break;
        }
        ids.push(c.id);
        l1 += c.size_bytes.max(0);
    }
    if l1 < target {
        for c in candidates.iter().filter(|c| c.risk_level == "L2") {
            if l1 + l2 >= target {
                break;
            }
            ids.push(c.id);
            l2 += c.size_bytes.max(0);
        }
    }
    (ids, l1, l2)
}

/// 字节 → 人类可读（1024 进制，与前端口径一致）。
fn bytes(b: i64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let n = b.max(0) as f64;
    if n >= GB {
        format!("{:.1} GB", n / GB)
    } else if n >= MB {
        format!("{:.1} MB", n / MB)
    } else if n >= KB {
        format!("{:.1} KB", n / KB)
    } else {
        format!("{n:.0} B")
    }
}

#[cfg(test)]
mod tests {
    use super::pick;
    use super::Candidate;

    fn cand(id: i64, size: i64, level: &str) -> Candidate {
        Candidate {
            id,
            size_bytes: size,
            risk_level: level.to_string(),
        }
    }

    #[test]
    fn l1_only_reaches_target() {
        // L1 = [60, 40]，L2 有一个更大的 100。目标 100 恰好被两个 L1 拼满：
        // 验证「L1 优先，够达标的组合绝不掺 L2」——即便 L2 单条更大也不选它。
        let rows = vec![cand(1, 60, "L1"), cand(2, 40, "L1"), cand(3, 100, "L2")];
        let (ids, l1, l2) = pick(&rows, 100);
        assert_eq!(ids, vec![1, 2], "两个 L1 已拼满 100，不该退而求其次碰 L2");
        assert_eq!(l1, 100);
        assert_eq!(l2, 0);
    }

    #[test]
    fn l1_insufficient_includes_l2() {
        let rows = vec![cand(1, 50, "L1"), cand(2, 90, "L2"), cand(3, 30, "L2")];
        let (ids, l1, l2) = pick(&rows, 80);
        // 先吃最大 L1(50)，不够再吃最大 L2(90) → 达标即停。
        assert_eq!(ids, vec![1, 2]);
        assert_eq!(l1, 50);
        assert_eq!(l2, 90);
    }

    #[test]
    fn l2_picked_in_size_desc_order() {
        let rows = vec![
            cand(1, 10, "L1"),
            cand(2, 100, "L2"),
            cand(3, 60, "L2"),
            cand(4, 20, "L2"),
        ];
        // L1 全吃(10)不够，再按大小吃 L2：100 → 达标(110>=100)即停，不吃 60。
        let (ids, l1, l2) = pick(&rows, 100);
        assert_eq!(ids, vec![1, 2]);
        assert_eq!(l1, 10);
        assert_eq!(l2, 100);
    }

    #[test]
    fn zero_target_picks_nothing() {
        let rows = vec![cand(1, 100, "L1"), cand(2, 50, "L2")];
        let (ids, l1, l2) = pick(&rows, 0);
        assert!(ids.is_empty());
        assert_eq!(l1, 0);
        assert_eq!(l2, 0);
    }

    #[test]
    fn negative_sizes_are_clamped() {
        // 库里不该有负 size，但即使有也不能把推荐算坏 / 触发过量删除。
        let rows = vec![cand(1, -50, "L1"), cand(2, 200, "L1")];
        let (ids, l1, l2) = pick(&rows, 100);
        assert_eq!(ids, vec![1, 2]);
        assert_eq!(l1, 200, "负大小按 0 计，保证不缩水");
        assert_eq!(l2, 0);
    }
}
