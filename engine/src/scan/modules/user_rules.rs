//! 用户自定义规则扫描（task-078）。
//!
//! # 这个模块为什么必须存在
//!
//! 任务书把「规则引擎合并」放在 `rules/loader.rs` 里，但 **`rules/loader.rs` 在扫描
//! 链路上没有消费者**——它只服务两件事：给 `cleanup.dryRun` / `cleanup.execute`
//! 提供外部 L3 红线（`external_deny_list`），以及规则库更新 / 回滚时的比对。
//! 5 个内置模块各自硬编码自己的路径，从不读规则库。
//!
//! 也就是说：**只改 loader 不会让任何用户规则生效**。要让「新增规则 → 扫描 →
//! Items 出现」这条验收标准成立，必须有一个真正遍历用户规则路径的扫描器。
//! 这就是本模块。
//!
//! # 三道过滤，缺一不可
//!
//! 1. **模式匹配**：`glob_match(pattern, path)`——用户写 `D:\MyCache\*` 只匹配那一层；
//! 2. **L3 红线**：每个**真实路径**都过一遍 `execute::whitelist::classify`。
//!    创建规则时校验的是「模式的静态根」，而通配符可能展开到子目录里的敏感文件
//!    （`D:\Data\**\Login Data` 的静态根是合法的 `D:\Data`）。**两者不能互相替代**；
//! 3. **内置优先**：落在内置规则目录下的文件跳过——否则 `%TEMP%\*` 这样的用户规则
//!    会让同一批文件出现两遍（条目翻倍、大小重复计），而两边的风险等级可能不同。
//!
//! # 每个规则都有条数上限
//!
//! 用户可以把规则指向一个几十万文件的目录（`D:\Games\**`）。没有上限时一次扫描
//! 能跑几十分钟、写爆 `cleanup_item`。所以每条规则最多收 [`MAX_ITEMS_PER_RULE`]
//! 条，超出即停并打日志——**宁可少列也不卡死**。这与 Top100 截断是同一套取舍：
//! 用户能看到"被截断"的日志，而卡死的应用什么也说明不了。
//!
//! # 与内置模块的差异：`category` 用规则 id
//!
//! 内置模块的 `category` 是固定字符串（`user_temp` / `npm_cache`…）。用户规则没有
//! 稳定的类别键，所以用 `user_rule_<id>`：**id 稳定**（改名不影响），而规则名是
//! 用户随时可改的——用名字做 category 会让清理方案（存的是 `{module_code,
//! category, path}` 三元组）在改名后整份失效。

use std::path::Path;

use super::{is_reparse_point, ScanItem};

/// 模块代码。对应 `cleanup_item.module_code`。
pub const MODULE_CODE: &str = "user_rules";

/// 单条规则最多产出多少条 item。
///
/// 20_000 的依据：本机 Firefox 缓存（已知最大的内置目标）约 4.5k 条；用户规则的
/// 目录通常比这小一个量级。真要超过 2 万条，说明规则指得太宽——那时截断比让
/// 应用卡死更负责。
pub const MAX_ITEMS_PER_RULE: usize = 20_000;

/// 扫描用的一条规则（从 DB 行简化而来）。
///
/// 单独定义而不是直接吃 `ipc::commands::user_rules::UserRule`：`scan` 是内层模块，
/// 不该依赖 `ipc`（外层）。少一层依赖，单测也不用造 DB。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserRuleSpec {
    /// 规则 id（用于拼 `category`）。
    pub id: i64,
    /// 规则名（只用于日志；**不参与** category）。
    pub name: String,
    /// 已展开环境变量的路径模式。
    pub pattern: String,
    /// 风险等级整数：1 = L1，2 = L2（L3 在建规则时就被拒了）。
    pub risk_level: i64,
}

/// 扫一批规则，返回全部匹配条目。
///
/// `builtin_prefixes` 来自 `rules::loader::builtin_dir_prefixes`（归一化后的目录
/// 前缀）；传空表表示「不做内置优先抑制」（内置规则加载失败时的降级，宁可重复
/// 也不误吞用户条目）。
///
/// `selected` 是本次选中的盘符；`None` = 全部。**按规则的静态根所在盘判断**，
/// 与内置模块的「模块级跳过」同一套语义。
pub fn scan_rules(
    rules: &[UserRuleSpec],
    builtin_prefixes: &[String],
    selected: Option<&[char]>,
) -> Vec<ScanItem> {
    let mut out = Vec::new();
    for rule in rules {
        let root = match crate::ipc::commands::user_rules::static_root(&rule.pattern) {
            Some(r) => r,
            None => {
                // 建规则时已校验过；走到这里说明 DB 里有一条绕过 IPC 写进来的脏数据
                eprintln!(
                    "[user_rules] 跳过规则 {}（id={}）：模式没有静态根 {}",
                    rule.name, rule.id, rule.pattern
                );
                continue;
            }
        };

        // 盘选择：规则的根不在选中盘上就整条跳过（与内置模块同一语义）
        if let Some(drives) = selected {
            let d = crate::scan::drive_enum::drive_letter_of_path(&root);
            let wanted = match d {
                Some(c) => drives.iter().any(|x| x.eq_ignore_ascii_case(&c)),
                // 盘符识别不出来（UNC 等）→ 不因盘选择而跳过，交给后面的红线判定
                None => true,
            };
            if !wanted {
                eprintln!(
                    "[user_rules] 跳过规则 {}：根目录 {root:?} 不在选中的盘 {:?}",
                    rule.name, drives
                );
                continue;
            }
        }

        let category = format!("user_rule_{}", rule.id);
        let mut taken = 0usize;
        let mut denied = 0usize;
        let mut shadowed = 0usize;

        // 回调式遍历：一旦凑够条数就立刻停，不再把整棵树走完。
        // （先收集再过滤的写法在 `D:\Games\**` 这种规则下会把几十万个 PathBuf
        // 塞进内存，而且明明已经够数了还在读盘。）
        for_each_file(&root, |path| {
            let path_str = path.to_string_lossy().to_string();

            // 1) 模式匹配
            if !crate::ipc::commands::user_rules::glob_match(&rule.pattern, &path_str) {
                return true;
            }
            // 2) L3 红线（真实路径，逐条判）
            match crate::execute::normalize::normalize(&path_str) {
                Ok(n) => {
                    if crate::execute::whitelist::classify(&n).is_err() {
                        denied += 1;
                        return true;
                    }
                }
                Err(_) => {
                    // 规范化失败（UNC / 异常路径）→ 不放行
                    denied += 1;
                    return true;
                }
            }
            // 3) 内置优先
            if crate::rules::loader::covered_by_builtin(&path_str, builtin_prefixes) {
                shadowed += 1;
                return true;
            }

            let size = std::fs::symlink_metadata(&path)
                .map(|m| m.len() as i64)
                .unwrap_or(0);
            out.push(ScanItem::new(
                MODULE_CODE,
                &category,
                path_str,
                size,
                rule.risk_level,
            ));
            taken += 1;
            taken < MAX_ITEMS_PER_RULE
        });

        if taken >= MAX_ITEMS_PER_RULE {
            eprintln!(
                "[user_rules] 规则 {} 达到上限 {MAX_ITEMS_PER_RULE} 条，已截断（请把模式收窄）",
                rule.name
            );
        }
        if denied > 0 || shadowed > 0 {
            eprintln!(
                "[user_rules] 规则 {}：命中 {taken} 条；{denied} 条因 L3 红线跳过、{shadowed} 条因内置规则优先跳过",
                rule.name
            );
        }
    }
    out
}

/// 深度优先遍历 `root` 下的普通文件；`visit` 返回 `false` 时立即停止整次遍历。
///
/// 显式栈而不是递归：用户规则的根可能很深（`D:\Games\**`），递归会爆栈。
/// 每个目录内排序，保证输出确定（单测能稳定断言，用户看到的顺序也不随机）。
fn for_each_file<F>(root: &Path, mut visit: F)
where
    F: FnMut(std::path::PathBuf) -> bool,
{
    if !root.is_dir() {
        return;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            // 权限不足等：跳过该目录，不中断整条规则
            continue;
        };
        let mut children: Vec<std::path::PathBuf> = entries.flatten().map(|e| e.path()).collect();
        children.sort();
        for child in children {
            let Ok(md) = std::fs::symlink_metadata(&child) else {
                continue;
            };
            if is_reparse_point(&md) {
                continue;
            }
            if md.is_dir() {
                stack.push(child);
            } else if md.is_file() && !visit(child) {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn spec(id: i64, pattern: &str, risk: i64) -> UserRuleSpec {
        UserRuleSpec {
            id,
            name: format!("rule{id}"),
            pattern: pattern.to_string(),
            risk_level: risk,
        }
    }

    /// 造一个临时「缓存目录」并返回 (tempdir, 该目录的 glob 模式)。
    fn make_cache() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("MyCache");
        fs::create_dir_all(cache.join("sub")).unwrap();
        fs::write(cache.join("a.tmp"), vec![b'x'; 100]).unwrap();
        fs::write(cache.join("b.log"), vec![b'x'; 20]).unwrap();
        fs::write(cache.join("sub").join("c.tmp"), vec![b'x'; 300]).unwrap();
        // 只匹配本层的 .tmp：b.log 扩展名不符、sub\c.tmp 在下一层（`*` 不跨目录）
        let pattern = format!(r"{}\*.tmp", cache.to_string_lossy());
        (dir, pattern)
    }

    #[test]
    fn scans_only_matching_files() {
        let (_g, pattern) = make_cache();
        let items = scan_rules(&[spec(1, &pattern, 1)], &[], None);

        // `*` 不跨目录：只有 a.tmp 命中（b.log 扩展名不符、sub\c.tmp 在下一层）
        assert_eq!(items.len(), 1, "实际: {items:#?}");
        assert!(items[0].path.ends_with("a.tmp"));
        assert_eq!(items[0].size_bytes, 100);
        assert_eq!(items[0].module_code, MODULE_CODE);
        assert_eq!(items[0].category, "user_rule_1");
        assert_eq!(items[0].risk_level, 1);
    }

    #[test]
    fn double_star_reaches_nested_files() {
        let (_g, pattern) = make_cache();
        let root = std::path::Path::new(&pattern).parent().unwrap();
        let pattern2 = format!(r"{}\**\*.tmp", root.to_string_lossy());
        let items = scan_rules(&[spec(2, &pattern2, 2)], &[], None);

        // a.tmp（零层）+ sub\c.tmp（一层）
        assert_eq!(items.len(), 2, "实际: {items:#?}");
        assert!(
            items.iter().all(|i| i.risk_level == 2),
            "风险等级应取规则值"
        );
    }

    #[test]
    fn category_is_rule_id_not_name() {
        // 改名不能让 category 变——清理方案存的是 category，改名后方案会整份失效
        let (_g, pattern) = make_cache();
        let mut s = spec(7, &pattern, 1);
        s.name = "用户改过的名字".to_string();
        let items = scan_rules(&[s], &[], None);
        assert_eq!(items[0].category, "user_rule_7");
    }

    #[test]
    fn skips_rules_whose_root_is_not_selected() {
        let (_g, pattern) = make_cache();
        // 该临时目录在 C 盘（或当前盘）；选一个不同的盘 → 整条规则跳过
        let root = std::path::Path::new(&pattern).parent().unwrap();
        let here = crate::scan::drive_enum::drive_letter_of_path(root).unwrap();
        let other = if here == 'Z' { 'Y' } else { 'Z' };
        let items = scan_rules(&[spec(1, &pattern, 1)], &[], Some(&[other]));
        assert!(items.is_empty(), "根不在选中盘上应跳过");

        // 选中本盘则正常扫到
        let items = scan_rules(&[spec(1, &pattern, 1)], &[], Some(&[here]));
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn builtin_prefix_shadows_user_rule() {
        // 用户规则指到内置目录内部 → 条目被抑制（避免同一文件出现两遍）
        let (_g, pattern) = make_cache();
        let root = std::path::Path::new(&pattern).parent().unwrap();
        let prefix = root.to_string_lossy().to_lowercase();
        let items = scan_rules(&[spec(1, &pattern, 1)], &[prefix], None);
        assert!(items.is_empty(), "内置已覆盖的路径不该重复产出");

        // 前缀换成别处 → 不抑制
        let items = scan_rules(&[spec(1, &pattern, 1)], &[r"q:\nowhere".to_string()], None);
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn missing_root_yields_nothing() {
        // 规则指向一个不存在的目录：不报错、不产出
        let items = scan_rules(&[spec(1, r"Q:\NoSuchDir\*", 1)], &[], None);
        assert!(items.is_empty());
    }

    #[test]
    fn empty_rule_list_yields_nothing() {
        assert!(scan_rules(&[], &[], None).is_empty());
    }
}
