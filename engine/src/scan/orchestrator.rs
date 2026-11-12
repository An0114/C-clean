//! `ScanOrchestrator`：按固定顺序跑 5 个扫描模块（task-013 起，task-076 起全部 Rust 优先）。
//!
//! 与 [`crate::scan::runner::ScanRunner`] 的关系：runner 一次只处理一个模块，
//! orchestrator 在 engine 层把 5 个模块按固定顺序串起来，汇总 item/byte 数，
//! 记录成功/失败模块名。失败模块不中断整体，5 个全失败才返回 `AllScriptsFailed`。
//!
//! # task-086：Rust 唯一扫描路径
//!
//! 5 个内置模块全部由 Rust 实现；ps1 文件与 `CCLEAN_RUST_*` 回退开关已删除，
//! 编排器不再 spawn PowerShell 解释器。

use std::path::Path;

use rusqlite::Connection;

use crate::ipc::progress::{ProgressEmitter, ScanProgress};
use crate::rules::models::CacheRules;
use crate::scan::drive_enum::drive_letter_of_path;
use crate::scan::modules::software_cache::SoftwareRules;
use crate::scan::runner::ScanRunner;

/// task-083：软件感知扫描开关（`CCLEAN_SOFTWARE_AWARE=1` 启用）。
///
/// 独立进程级开关，便于真机排查时对比结果。
pub fn software_awareness_enabled() -> bool {
    std::env::var("CCLEAN_SOFTWARE_AWARE").as_deref() == Ok("1")
}

/// 编译期内嵌的软件规则（打包安装时的兜底）。
const EMBEDDED_SOFTWARE_RULES: &str = include_str!("../../../rules/software_rules.json");

/// 加载软件规则：外部文件优先，缺失/损坏则回退到内嵌副本。
fn load_software_rules(scripts_dir: &Path) -> SoftwareRules {
    if let Some(external) = scripts_dir
        .parent()
        .map(|p| p.join("rules").join("software_rules.json"))
        .filter(|p| p.is_file())
    {
        match crate::scan::modules::software_cache::load(&external) {
            Ok(r) => return r,
            Err(e) => eprintln!("[orchestrator] 外部软件规则不可用，改用内嵌副本: {e}"),
        }
    }
    serde_json::from_str::<SoftwareRules>(EMBEDDED_SOFTWARE_RULES)
        .expect("内嵌 software_rules.json 必须能被解析")
}

/// 内置模块扫描顺序表。
///
/// 保留既有 `.ps1` 形态的模块 ID，避免改变扫描汇总、进度标签和历史统计协议；
/// 这些字符串不再用于定位或执行脚本文件。
const SCRIPTS: &[&str] = &[
    "scan_temp.ps1",
    "scan_windows_update.ps1",
    "scan_prefetch.ps1",
    "scan_browser_cache.ps1",
    "scan_dev_cache.ps1",
];

const TEMP_SCRIPT: &str = "scan_temp.ps1";
const WINDOWS_UPDATE_SCRIPT: &str = "scan_windows_update.ps1";
const PREFETCH_SCRIPT: &str = "scan_prefetch.ps1";
const BROWSER_CACHE_SCRIPT: &str = "scan_browser_cache.ps1";
const DEV_CACHE_SCRIPT: &str = "scan_dev_cache.ps1";

/// 一个盘的扫描统计（task-077）。
///
/// **数据来源是 DB 而不是内存累加**：`run_all` 结束时按 `session_id` 查一次
/// `cleanup_item`，与五个 Rust 模块的入库结果保持一致。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveStat {
    /// 盘符字母，大写。
    pub drive: char,
    /// 该盘条目数。
    pub items: i64,
    /// 该盘合计字节。
    pub bytes: i64,
}

/// 5 个脚本跑完的汇总。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OrchestratorSummary {
    /// 累计成功入库的 item 数。
    pub total_items: i64,
    /// 累计 done 事件里的 total_bytes。
    pub total_bytes: i64,
    /// 跑成功的脚本文件名。
    pub scripts_succeeded: Vec<String>,
    /// 失败的脚本文件名（含文件不存在 / run_script 报错）。
    pub scripts_failed: Vec<String>,
    /// task-077：因「根目录不在本次选中的盘上」而跳过的模块。
    ///
    /// **跳过不是失败**：用户只选了 D 盘，而 5 个模块的根目录都在 C 盘时，
    /// 一个模块都不该跑——这时的正确结果就是「0 项」，不是 `AllScriptsFailed`。
    pub scripts_skipped: Vec<String>,
    /// task-063：是否被用户取消（true 时 `scripts_*` 只反映取消前跑完的部分）。
    pub cancelled: bool,
    /// task-077：按盘汇总（含被选中但 0 项的盘，前端要显示「D: 0 项」）。
    pub per_drive: Vec<DriveStat>,
}

/// orchestrator 级错误：5 个脚本全部失败。
#[derive(thiserror::Error, Debug)]
pub enum OrchestratorError {
    /// 5 个脚本全部失败（含文件不存在）。
    #[error("全部 {0} 个脚本均失败")]
    AllScriptsFailed(usize),
}

/// 编排器：绑定 session_id + DB + emitter，跑一组脚本。
pub struct ScanOrchestrator<'conn, 'emitter> {
    session_id: i64,
    conn: &'conn Connection,
    emitter: &'emitter dyn ProgressEmitter,
    /// task-063：取消标志。`scan_cancel` 命令把它置 true，`run_all` 在每个脚本
    /// 开始前检查——**脚本粒度**取消（不打断正在跑的 ps1 子进程）。
    ///
    /// 为什么不做进程级中断：要 kill 正在跑的 PowerShell 子进程，得把 pid 从
    /// `ScanRunner` 一路传出来并调 `TerminateProcess`，而且 ps1 已经写入 DB 的
    /// 部分结果还得回滚。脚本粒度取消已经能让用户「立刻看到停止」，
    /// 代价是当前脚本会跑完（通常几秒）。
    cancel: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// task-077：本次要扫的盘符（大写）。`None` = 全部固定盘。
    ///
    /// **它是「用户意图」不是「实际扫到的盘」**：选中的盘如果没有任何扫描目标
    /// （本机上 5 个模块的根目录全在 C:，选 D: 就是这种情况），结果是 0 项，
    /// 前端会如实显示「D: 0 项」。不为了「看起来有东西」去扫不存在的路径。
    drives: Option<Vec<char>>,
    /// task-078：内置规则覆盖的目录前缀（归一化后），用于「内置优先于用户规则」。
    ///
    /// 由调用方（`scan_start` 命令）从 `rules/cache_rules.json` 加载后传入——
    /// 编排层不碰文件系统路径，也就不会在单测里被迫准备一份规则库。
    /// 空表 = 不做抑制（内置规则加载失败时的降级：宁可重复也不误吞用户条目）。
    builtin_prefixes: Vec<String>,
    /// task-083 A2：注入的完整规则库（含 `system` 组），5 个内置模块据此读路径。
    ///
    /// `None` = 规则库加载失败，拒绝注入——5 个模块退回各自的「环境变量硬编码」
    /// 降级兜底（`xxx::scan()`），宁可多一份路径常量也不让扫描整块失效。
    /// 主路径（App/CLI 加载成功）一定走这里，实现「同一规则只在 JSON 维护一份」。
    system_rules: Option<CacheRules>,
}

impl<'conn, 'emitter> ScanOrchestrator<'conn, 'emitter> {
    /// 构造：给定已存在的 session_id、DB 连接、进度 emitter。
    pub fn new(
        session_id: i64,
        conn: &'conn Connection,
        emitter: &'emitter dyn ProgressEmitter,
    ) -> Self {
        Self {
            session_id,
            conn,
            emitter,
            cancel: None,
            drives: None,
            builtin_prefixes: Vec::new(),
            system_rules: None,
        }
    }

    /// task-083 A2：注入规则库，让 5 个内置模块从 `system` 组读路径。
    ///
    /// 不调用 = `None` = 各模块走环境变量硬编码兜底。这是有意的降级方向：
    /// 规则库读不出来时扫描照常，而不是整块报告「规则缺失」。
    pub fn with_system_rules(mut self, rules: CacheRules) -> Self {
        self.system_rules = Some(rules);
        self
    }

    /// task-078：挂上内置规则的目录前缀（「内置优先」判定用）。
    ///
    /// 不调用 = 空表 = 不抑制。这是**有意的降级方向**：内置规则库读不出来时，
    /// 用户规则照常生效（可能产生重复条目），而不是整块失效。
    pub fn with_builtin_prefixes(mut self, prefixes: Vec<String>) -> Self {
        self.builtin_prefixes = prefixes;
        self
    }

    /// task-077：限定本次扫描的盘（不调用 = 全部固定盘）。
    ///
    /// 盘符统一转大写：`'c'` 和 `'C'` 是同一个盘，前端从不同地方传进来可能大小写
    /// 不一致，在入口处归一化比在每个比较点都写 `eq_ignore_ascii_case` 可靠。
    pub fn with_drives(mut self, drives: Vec<char>) -> Self {
        self.drives = Some(drives.into_iter().map(|c| c.to_ascii_uppercase()).collect());
        self
    }

    /// task-063：挂上取消标志。不调用则行为与之前完全一致（不可取消）。
    pub fn with_cancel(mut self, cancel: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Self {
        self.cancel = Some(cancel);
        self
    }

    /// 是否已被请求取消。
    fn is_cancelled(&self) -> bool {
        cancel_requested(&self.cancel)
    }

    /// 按 [`SCRIPTS`] 的顺序跑 5 个 Rust 模块。
    ///
    /// - 单个模块失败不中断；
    /// - 计划内的模块全失败才返回 `Err(AllScriptsFailed)`；
    /// - 至少 1 个成功、或**全部因盘选择被跳过**，都返回 `Ok(summary)`。
    ///
    /// task-052 G1：按「模块序号 / 模块总数」把 0~100 切成 N 个槽位，每个模块只在
    /// 自己那一槽内推进（槽内插值见 [`crate::scan::runner::interpolate_percent`]），
    /// 模块跑完补一条「槽位上界」边界事件，全部结束后补一条 `percent = 100`。
    /// 这样任意时刻 `percent ∈ [0, 100]` 且单调不减。
    ///
    /// task-053 G7.2：收尾的 `percent = 100` 走 [`ProgressEmitter::emit_force`]
    /// 强制发送。原因：它紧跟最后一条 item 事件（同毫秒级），走普通 `emit` 会被
    /// `ThrottledEmitter` 的 100ms 丢弃式节流吞掉 → 进度条永远差一截（G7 现象）。
    /// 槽位边界事件间隔以秒计，不会被吞，仍走普通 `emit` 以保住节流上限。
    ///
    /// # task-077：多盘是「按盘过滤」而不是「按盘换路径」
    ///
    /// 5 个模块的根目录都是**用户 / 系统锚定**的：`%TEMP%`、`%SystemRoot%`、
    /// `%LOCALAPPDATA%`、`%APPDATA%`。它们没有「每盘一份」的对应物——`D:\Windows\Temp`
    /// 在绝大多数机器上根本不存在，`D:\Users\...\npm-cache` 同理。
    ///
    /// 所以「多盘扫描」在这里的正确语义是：
    ///
    /// 1. **模块级**：如果某个模块的根目录**全部**不在选中的盘上，整个模块跳过
    ///    （连扫描都不做，省掉几千个文件的遍历）。这一条自动满足「Prefetch /
    ///    Windows Update 只在系统盘跑」——它们的根目录是 `%SystemRoot%`，
    ///    用户不选系统盘就自然跳过，**不需要硬编码 `C:`**（Windows 装在 D 盘的
    ///    机器上照样正确）；
    /// 2. **条目级**：模块照跑，但只保留路径落在选中盘上的条目（`%TEMP%` 在 D:、
    ///    系统在 C: 的机器上，一个模块会跨两个盘）；
    /// 3. **进度**：标签带盘符（`"C: scan_browser_cache.ps1"`），因为每个模块在
    ///    实践中只锚定一个盘，「盘 × 模块」二维矩阵会退化成一条线——如实呈现，
    ///    不编造 5×N 个格子；
    /// 4. **汇总**：按盘给出条目数 / 字节数，被选中但 0 项的盘也列出来，
    ///    让用户看到「D: 0 项」而不是以为漏扫了。
    ///
    /// 槽位按**实际要跑的模块数**切分（不是固定 5），这样跳过模块时进度条不会
    /// 留出空洞。
    pub fn run_all(&self, scripts_dir: &Path) -> Result<OrchestratorSummary, OrchestratorError> {
        let mut summary = OrchestratorSummary::default();

        // 先决定跑哪些模块（task-077 盘选择），再按计划切进度槽位。
        let mut plan: Vec<&str> = Vec::new();
        for name in SCRIPTS {
            if self.module_runs(name) {
                plan.push(name);
            } else {
                let label = self.anchor_label(name).unwrap_or('-');
                eprintln!(
                    "[orchestrator] 跳过 {name}：根目录在 {label}: 上，不在本次选中的盘 {:?}",
                    self.drives
                );
                summary.scripts_skipped.push(name.to_string());
            }
        }
        let total = plan.len();

        for (idx, name) in plan.iter().enumerate() {
            // task-063：每个脚本开始前检查取消。已扫描到的结果保留在 DB 里
            // （cleanup_item 的行不删），前端刷新就能看到。
            if self.is_cancelled() {
                eprintln!("[orchestrator] 用户取消扫描，停在脚本 {name} 之前");
                summary.cancelled = true;
                break;
            }
            // 第 idx 个脚本分到的百分比区间 [from, to]（N 个脚本时依次是 0-100/N、...）。
            // total 已在上面保证 > 0（空计划直接走下面的 Ok 分支）。
            let from = (idx * 100 / total) as u8;
            let to = ((idx + 1) * 100 / total) as u8;

            // task-077：条目级过滤——只保留落在选中盘上的条目。
            let items = self.rust_module_items(name);
            let items = self.filter_by_selected_drives(name, items);
            self.run_rust_module(name, items, from, to, &mut summary);
        }

        // task-078：用户自定义规则（DB 驱动，不是 ps1 模块，也不是 Rust 内置模块）。
        // 必须在「全失败」判定**之前**跑：只有用户规则有产出时，整体也算成功。
        if !self.is_cancelled() {
            self.run_user_rules(&mut summary);
        }

        // task-083：软件感知扫描（数据驱动，同 user_rules，不走 SCRIPTS 循环）。
        // 放在用户规则之后：两者都是「追加的一步」，顺序不影响结果，但日志上
        // 先用户规则再软件缓存，排查时更直观。
        //
        // **进程级开关**：engine 的编排集成测试用的是隔离的临时 DB + 假脚本，
        // 只想测编排逻辑；若这里无条件跑真实注册表枚举，会往那些 DB 里
        // 塞进上百条本机缓存条目，把「2 条 / 3 条」这类断言全部冲掉。
        // 由 Tauri 应用层在 setup 里置 `CCLEAN_SOFTWARE_AWARE=1` 启用
        // （后台自动扫描是同一进程，天然一起生效）。
        if !self.is_cancelled() && software_awareness_enabled() {
            self.run_software_cache(scripts_dir, &mut summary);
        }

        // task-084 B5：Gradle / Maven 依赖缓存（独立开关的追加步骤，同 user_rules /
        // software_cache 不算脚本）。开关默认关闭（fail-closed）——删错要重下几百 MB
        // 依赖，绝不默认纳入扫描。
        if !self.is_cancelled() && crate::prefs::get_dev_cache_gradle_maven(self.conn) {
            self.run_gradle_maven_cache(&mut summary);
        }

        // task-084 C1：跨用户缓存扫描（枚举 C:\Users\*，逐个可读用户跑 Temp 缓存）。
        //
        // 与 user_rules / software_cache 同款「不算脚本」语义：附加能力，不在
        // [`SCRIPTS`] 循环里，不占进度槽位，失败/跳过不影响整次扫描的成败判定。
        // 完全对齐任务书 §C1 的安全护栏：只扫 `cache_readable == true` 的用户
        // （其他用户默认被 Windows DACL 挡在 AppData 之外），权限不足的那部分
        // 在枚举阶段就跳过并记了日志。
        if !self.is_cancelled() {
            self.run_multi_user_cache(&mut summary);
        }

        // task-077：全部被跳过 = 用户选了没有扫描目标的盘，这是**合法结果**（0 项），
        // 不是失败。只有「计划内有模块、且一个都没成功」才算全失败。
        if !plan.is_empty() && summary.scripts_succeeded.is_empty() {
            return Err(OrchestratorError::AllScriptsFailed(total));
        }

        // task-077：按盘汇总（从 DB 取，两条入库路径都能覆盖）。
        summary.per_drive = self.drive_stats();

        // 收尾：无论最后一个脚本是否成功，只要整体算成功就补一条 100%。
        // 必须 force——普通 emit 会被节流吞（task-053 G7.2）。
        self.emit_force_progress(100, "done", summary.total_items as u64);
        Ok(summary)
    }

    /// task-083：软件感知扫描（枚举已装软件 → 扫其缓存 → 入库）。
    ///
    /// # 规则文件从哪来
    ///
    /// 优先读 `scripts_dir` 的兄弟 `rules/software_rules.json`（dev 与打包安装
    /// 都指向 resource_dir，可被外部覆盖）；**找不到就 fallback 到编译期内嵌的
    /// 同一份 JSON**（`include_str!`）。这样即使安装包漏带规则文件、或用户
    /// 把它删了，软件感知也不会整块失效——只读资源不该只有一个来源。
    ///
    /// # 为什么不算「脚本」
    ///
    /// 与 `run_user_rules` 同理：它是数据驱动追加步骤，不在 [`SCRIPTS`] 里，
    /// 因此不占进度槽位，也不会让 `module_runs` 多一个特例。
    fn run_software_cache(&self, scripts_dir: &Path, summary: &mut OrchestratorSummary) {
        let rules = load_software_rules(scripts_dir);
        let inventory = crate::scan::software_enum::enumerate_installed();
        let items =
            crate::scan::modules::software_cache::scan(&inventory, &rules, &self.builtin_prefixes);

        // 条目级盘过滤：与内置模块同一口径，不依赖脚本名（本步骤没有脚本名）
        let items: Vec<_> = items
            .into_iter()
            .filter(
                |it| match (&self.drives, drive_letter_of_path(Path::new(&it.path))) {
                    (Some(ds), Some(c)) => ds.iter().any(|x| x.eq_ignore_ascii_case(&c)),
                    _ => true,
                },
            )
            .collect();

        eprintln!(
            "[orchestrator] software_cache: {} 个已装软件 / {} 条规则 → {} 项",
            inventory.len(),
            rules.software.len(),
            items.len()
        );
        if items.is_empty() {
            return;
        }

        let runner =
            ScanRunner::new(self.session_id, self.conn, self.emitter).with_percent_span(95, 100);
        match runner.insert_batch(&items) {
            Ok(s) => {
                summary.total_items += s.items_inserted;
                summary.total_bytes += s.total_bytes;
                // **故意不 push 到 scripts_succeeded / scripts_failed**：
                // 那两个字段是「脚本」的成败清单，`run_all` 末尾还用它判
                // 「是否全失败」。软件缓存是数据驱动的附加步骤，不是脚本——
                // 把它算进去会让「5 个模块」的既有契约变成 6 个，也会让
                // 附加能力失败时整次扫描被误判为失败。这里只累加数量 + 记日志。
                self.emit_progress(100, "software_cache", summary.total_items as u64);
            }
            Err(e) => {
                // 附加能力失败不该拖垮整次扫描（与 user_rules 读失败的处理同口径）
                eprintln!("[orchestrator] software_cache 入库失败，已跳过: {e}");
            }
        }
    }

    /// task-078：跑用户自定义规则（从 DB 读启用规则 → 扫描 → 入库）。
    ///
    /// # 为什么单独一个方法而不是塞进 [`SCRIPTS`] 循环
    ///
    /// 5 个内置模块是固定的 Rust 分发结构，而用户规则是**数据驱动**的：
    /// 规则来自 DB、条数随用户变化、盘归属由规则自己的静态根决定。硬塞进那个循环
    /// 会让 `module_runs` / `rust_module_items` 都要多一个特例分支。
    ///
    /// # 失败与成功语义
    ///
    /// - **读不到规则**：打日志后跳过，**不算失败**——用户规则是附加能力，
    ///   读不出来不该让整次扫描失败（DB 出错的话内置模块那条路也会失败）；
    /// - **没有启用规则**：直接返回，连 `scripts_succeeded` 都不加（否则汇总里
    ///   会凭空多一个从未运行的模块名）；
    /// - **有规则但 0 条命中**：算成功（与内置模块「扫到 0 项也算成功」一致）。
    fn run_user_rules(&self, summary: &mut OrchestratorSummary) {
        let rules = match crate::ipc::commands::user_rules::list_enabled(self.conn) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[orchestrator] 读取用户规则失败，跳过: {e}");
                return;
            }
        };
        if rules.is_empty() {
            return;
        }

        // 转成扫描模块认识的轻量结构（scan 是内层模块，不该依赖 ipc 的行类型）
        let specs: Vec<crate::scan::modules::user_rules::UserRuleSpec> = rules
            .iter()
            .map(|r| crate::scan::modules::user_rules::UserRuleSpec {
                id: r.id,
                name: r.name.clone(),
                pattern: r.pattern.clone(),
                // L3 在建规则时就被拒了，这里只可能是 L1/L2
                risk_level: if r.risk_level.eq_ignore_ascii_case("L2") {
                    2
                } else {
                    1
                },
            })
            .collect();

        let items = crate::scan::modules::user_rules::scan_rules(
            &specs,
            &self.builtin_prefixes,
            self.drives.as_deref(),
        );
        eprintln!(
            "[orchestrator] user_rules: {} 条启用规则 → {} 项",
            specs.len(),
            items.len()
        );

        // 进度：5 个内置模块已经铺满 0~100，用户规则是追加的一步，
        // 用 95~100 这一段做条目内插值，收尾仍由 emit_force(100) 兜住。
        let runner =
            ScanRunner::new(self.session_id, self.conn, self.emitter).with_percent_span(95, 100);
        match runner.insert_batch(&items) {
            Ok(s) => {
                summary.total_items += s.items_inserted;
                summary.total_bytes += s.total_bytes;
                // 复用 scripts_succeeded 承载「跑成功的模块名」——它不是脚本，
                // 但语义一致（这个汇总字段的名字是 task-013 的历史遗留）。
                summary.scripts_succeeded.push("user_rules".to_string());
                self.emit_progress(100, "user_rules", summary.total_items as u64);
            }
            Err(e) => {
                eprintln!("[orchestrator] user_rules 入库失败: {e}");
                summary.scripts_failed.push("user_rules".to_string());
            }
        }
    }

    /// task-084 B5：跑 Gradle/Maven 依赖缓存扫描（独立开关，由 `run_all` 里读
    /// `prefs.dev_cache_gradle_maven` 决定是否调用）。
    ///
    /// # 为什么与 software_cache 同款「不算脚本」语义
    ///
    /// 它是数据驱动 / 独立开关的追加步骤，不在 [`SCRIPTS`] 循环里，因此不占进度
    /// 槽位、不 push 到 `scripts_succeeded` / `scripts_failed`——这样附加能力失败时
    /// 不会让「5 个模块」的全失败判定被多一个来源误伤。只累加数量并记日志。
    fn run_gradle_maven_cache(&self, summary: &mut OrchestratorSummary) {
        let items = crate::scan::modules::dev_cache::scan_gradle_maven();

        // 条目级盘过滤：与内置模块 / software_cache 同一口径。
        let items: Vec<_> = items
            .into_iter()
            .filter(
                |it| match (&self.drives, drive_letter_of_path(Path::new(&it.path))) {
                    (Some(ds), Some(c)) => ds.iter().any(|x| x.eq_ignore_ascii_case(&c)),
                    _ => true,
                },
            )
            .collect();

        eprintln!("[orchestrator] gradle_maven: {} 项", items.len());
        if items.is_empty() {
            return;
        }

        let runner =
            ScanRunner::new(self.session_id, self.conn, self.emitter).with_percent_span(95, 100);
        match runner.insert_batch(&items) {
            Ok(s) => {
                summary.total_items += s.items_inserted;
                summary.total_bytes += s.total_bytes;
                self.emit_progress(100, "gradle_maven", summary.total_items as u64);
            }
            Err(e) => {
                eprintln!("[orchestrator] gradle_maven 入库失败，已跳过: {e}");
            }
        }
    }

    /// task-084 C1：跨用户 Temp 缓存扫描。
    ///
    /// # 为什么「不算脚本」
    ///
    /// 与 [`Self::run_user_rules`] / [`Self::run_software_cache`] 同一口径：数据驱动
    /// 的附加步骤，不占进度槽位、不 push 到 `scripts_succeeded` / `scripts_failed`，
    /// 失败也不影响「5 个模块」的全失败判定。
    ///
    /// # 安全模型（对齐任务书 §C1）
    ///
    /// [`enumerate_user_profiles`] 已经帮我们把每个用户按 `cache_readable` 分好类：
    ///
    /// - 当前用户 + 能读的用户（同组/管理员）：`cache_readable == true`，扫；
    /// - 其他用户（默认被 DACL 挡）：`cache_readable == false`，**跳过**并记日志。
    ///
    /// 只扫 Windows 兜底存在的 `AppData\Local\Temp`：这是「用户级缓存」里权限最
    /// 宽松、风险最低的一类，跨用户时即使能读，也只碰它——不读其他用户的私有
    /// 浏览器 Profile / AppData 深层，那是任务书明确禁止的「私有 AppData」。
    /// 产出的条目统一带 `username`，落进 `cleanup_item.extra` 供前端分组展示。
    fn run_multi_user_cache(&self, summary: &mut OrchestratorSummary) {
        let profiles = enumerate_user_profiles();
        if profiles.is_empty() {
            return;
        }
        // 当前用户的 Temp 已经被 scan_temp 模块扫过，跨用户步骤跳过自己，避免重复。
        let current = std::env::var("USERNAME").unwrap_or_default();

        let mut items: Vec<crate::scan::modules::ScanItem> = Vec::new();
        let mut skipped: Vec<&str> = Vec::new();
        for p in &profiles {
            if p.username == current {
                continue;
            }
            let temp = std::path::Path::new(&p.root)
                .join("AppData")
                .join("Local")
                .join("Temp");
            // 枚举已判定 cache_readable，这里只是路径兜底。列目录失败的直接跳过。
            let found = match std::fs::read_dir(&temp) {
                Ok(_) => crate::scan::modules::scan_dir(
                    &temp,
                    "temp",
                    "user_temp",
                    1, // L1：Temp 目录下的缓存，可安全清理
                ),
                Err(_) => {
                    skipped.push(&p.username);
                    Vec::new()
                }
            };
            items.extend(
                found
                    .into_iter()
                    // 标注用户名：前端据此把条目归到对应 Profile 下
                    .map(|it| it.with_username(p.username.clone())),
            );
        }

        eprintln!(
            "[orchestrator] multi_user: {} 个用户 / 当前跳过自己 / 权限不足跳过 {:?} → {} 项",
            profiles.len(),
            skipped,
            items.len()
        );
        if items.is_empty() {
            return;
        }
        let runner =
            ScanRunner::new(self.session_id, self.conn, self.emitter).with_percent_span(95, 100);
        match runner.insert_batch(&items) {
            Ok(s) => {
                summary.total_items += s.items_inserted;
                summary.total_bytes += s.total_bytes;
                self.emit_progress(100, "multi_user", summary.total_items as u64);
            }
            Err(e) => {
                eprintln!("[orchestrator] multi_user 入库失败，已跳过: {e}");
            }
        }
    }

    /// task-077：这个模块的根目录是否落在选中的盘上。
    ///
    /// **读不到环境变量时返回 true（照跑）**：拿不到 `%TEMP%` 之类就没法判断，
    /// 这时「少扫一类」比「误跳过」更糟——跳过会让用户以为那个模块坏了。
    fn module_runs(&self, name: &str) -> bool {
        if self.drives.is_none() {
            return true;
        }
        let roots = root_env_names(name);
        if roots.is_empty() {
            return true;
        }
        let mut known = false;
        for env_name in roots {
            let Ok(val) = std::env::var(env_name) else {
                continue;
            };
            let Some(d) = crate::scan::drive_enum::drive_letter_of(&val) else {
                continue;
            };
            known = true;
            if self.drive_selected(d) {
                return true;
            }
        }
        // 一个根目录都没解析出来 → 判断不了 → 照跑
        !known
    }

    /// task-077：盘是否在本次选择范围内（未限定 = 全部）。
    fn drive_selected(&self, d: char) -> bool {
        drive_selected_from(self.drives.as_deref(), d)
    }

    /// task-077：模块锚点盘符（进度标签用）。读不到环境变量返回 `None`。
    fn anchor_label(&self, name: &str) -> Option<char> {
        root_env_names(name).iter().find_map(|env_name| {
            std::env::var(env_name)
                .ok()
                .and_then(|v| crate::scan::drive_enum::drive_letter_of(&v))
        })
    }

    /// task-077：进度标签，形如 `"C: scan_browser_cache.ps1"`。
    fn progress_label(&self, name: &str) -> String {
        match self.anchor_label(name) {
            Some(d) => format!("{d}: {name}"),
            None => name.to_string(),
        }
    }

    /// task-077：条目级盘过滤。盘符识别不出来的条目**保留**（见 `drive_letter_of`）。
    fn filter_by_selected_drives(
        &self,
        name: &str,
        items: Vec<crate::scan::modules::ScanItem>,
    ) -> Vec<crate::scan::modules::ScanItem> {
        if self.drives.is_none() {
            return items;
        }
        let before = items.len();
        let kept: Vec<_> = items
            .into_iter()
            .filter(
                |it| match crate::scan::drive_enum::drive_letter_of(&it.path) {
                    Some(d) => self.drive_selected(d),
                    None => true,
                },
            )
            .collect();
        if kept.len() != before {
            eprintln!(
                "[orchestrator] {name}：{} 项因不在选中盘上被过滤（{} → {}）",
                before - kept.len(),
                before,
                kept.len()
            );
        }
        kept
    }

    /// task-077：按盘汇总本 session 已入库的条目。
    ///
    /// 用 `path` 前缀算盘符而不是读 `drive_letter` 列——本方法在 v3 迁移之前就存在
    /// （提交顺序：多盘接线先于 DB 迁移），而且 `cleanup_item.path` 是唯一真值源，
    /// 生成列只是它的索引。`substr(path,2,1)=':'` 把 UNC / 相对路径排除掉。
    fn drive_stats(&self) -> Vec<DriveStat> {
        let mut stmt = match self.conn.prepare(
            "SELECT upper(substr(path,1,1)) AS d,
                    count(*) AS n,
                    coalesce(sum(size_bytes),0) AS b
             FROM cleanup_item
             WHERE session_id = ?1 AND substr(path,2,1) = ':'
             GROUP BY d
             ORDER BY d",
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[orchestrator] per_drive 查询失败: {e}");
                return Vec::new();
            }
        };

        let mut out: Vec<DriveStat> = stmt
            .query_map([self.session_id], |r| {
                let d: String = r.get(0)?;
                Ok(DriveStat {
                    drive: d.chars().next().unwrap_or('-'),
                    items: r.get(1)?,
                    bytes: r.get(2)?,
                })
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default();

        // 被选中但一条都没有的盘也要出现：前端要显示「D: 0 项」。
        if let Some(selected) = &self.drives {
            for d in selected {
                if !out.iter().any(|s| s.drive == *d) {
                    out.push(DriveStat {
                        drive: *d,
                        items: 0,
                        bytes: 0,
                    });
                }
            }
            out.sort_by_key(|s| s.drive);
        }
        out
    }

    /// 分发内置模块扫描；`SCRIPTS` 之外的模块 ID 属于编程错误。
    ///
    /// 抽成一个方法而不是在 `run_all` 里写五个 `if`：模块分发只有一处清单。
    ///
    /// A2：内置模块扫描的分发——注入规则走规则驱动，否则退回环境变量兜底。
    ///
    /// 为什么抽成函数指针而非每处 `if let`：五个模块的「规则入口 / 兜底入口」
    /// 成对出现，抽出来避免每个分支写一遍相同的 `if`。返回类型用完整路径，
    /// 避免再为这个内部辅助补 `use`。
    #[allow(clippy::type_complexity)]
    fn builtin_scan(
        rules: Option<&CacheRules>,
        from_rules: fn(&CacheRules) -> Vec<crate::scan::modules::ScanItem>,
        fallback: fn() -> Vec<crate::scan::modules::ScanItem>,
    ) -> Vec<crate::scan::modules::ScanItem> {
        match rules {
            Some(r) => from_rules(r),
            None => fallback(),
        }
    }

    /// **注意这里会真的执行扫描**（可能耗时：Firefox `cache2` 本机有 4569 个文件）。
    /// 调用方在循环开头已经做过模块粒度的取消检查。
    fn rust_module_items(&self, name: &str) -> Vec<crate::scan::modules::ScanItem> {
        let rules = self.system_rules.as_ref();
        match name {
            TEMP_SCRIPT => Self::builtin_scan(
                rules,
                crate::scan::modules::temp::scan_from_rules,
                crate::scan::modules::temp::scan,
            ),
            PREFETCH_SCRIPT => Self::builtin_scan(
                rules,
                crate::scan::modules::prefetch::scan_from_rules,
                crate::scan::modules::prefetch::scan,
            ),
            BROWSER_CACHE_SCRIPT => Self::builtin_scan(
                rules,
                crate::scan::modules::browser_cache::scan_from_rules,
                crate::scan::modules::browser_cache::scan,
            ),
            DEV_CACHE_SCRIPT => Self::builtin_scan(
                rules,
                crate::scan::modules::dev_cache::scan_from_rules,
                crate::scan::modules::dev_cache::scan,
            ),
            WINDOWS_UPDATE_SCRIPT => Self::builtin_scan(
                rules,
                crate::scan::modules::windows_update::scan_from_rules,
                crate::scan::modules::windows_update::scan,
            ),
            _ => unreachable!("未知内置扫描模块: {name}"),
        }
    }

    /// 跑一个 Rust 模块并把结果记进汇总。
    ///
    /// # 为什么抽成方法
    ///
    /// task-074 时只有 temp 一个模块走这条路，内联在 `run_all` 里没问题。
    /// task-075 起有三个（temp / prefetch / browser_cache），task-076 补齐五个，
    /// 五段复制粘贴的记账代码一旦有一处写漏（比如忘了 `push scripts_succeeded`），
    /// 表现是**「扫描成功了但汇总里说失败」**——这种不一致很难查。
    fn run_rust_module(
        &self,
        name: &str,
        items: Vec<crate::scan::modules::ScanItem>,
        from: u8,
        to: u8,
        summary: &mut OrchestratorSummary,
    ) {
        let runner =
            ScanRunner::new(self.session_id, self.conn, self.emitter).with_percent_span(from, to);
        match runner.insert_batch(&items) {
            Ok(s) => {
                summary.total_items += s.items_inserted;
                summary.total_bytes += s.total_bytes;
                summary.scripts_succeeded.push(name.to_string());
                eprintln!(
                    "[orchestrator] {name} 走 Rust 实现，入库 {} 项",
                    s.items_inserted
                );
                self.emit_progress(to, &self.progress_label(name), summary.total_items as u64);
            }
            Err(e) => {
                // 单模块入库失败只记该模块失败，不中断其他模块。
                eprintln!("[orchestrator] rust {name} failed: {e}");
                summary.scripts_failed.push(name.to_string());
            }
        }
    }

    /// 发一条编排层进度（脚本边界用，允许被节流丢弃）。
    fn emit_progress(&self, percent: u8, label: &str, scanned_items: u64) {
        self.emitter.emit(&ScanProgress {
            session_id: self.session_id,
            percent,
            current_path: label.to_string(),
            scanned_items,
        });
    }

    /// 发一条**不可被节流丢弃**的编排层进度（收尾用）。
    fn emit_force_progress(&self, percent: u8, label: &str, scanned_items: u64) {
        self.emitter.emit_force(&ScanProgress {
            session_id: self.session_id,
            percent,
            current_path: label.to_string(),
            scanned_items,
        });
    }
}

/// task-077：盘是否在本次选择范围内（`None` = 全部）。
///
/// 抽成自由函数是为了能直接单测：`ScanOrchestrator` 的构造需要真实 `Connection`
/// + `ProgressEmitter`，为测一个布尔判断去搭那套太重。
///
/// 大小写不敏感——`'c'` 与 `'C'` 是同一个盘，比较时不能因为调用方少写个大写就把盘漏掉。
fn drive_selected_from(selected: Option<&[char]>, d: char) -> bool {
    match selected {
        None => true,
        Some(list) => list.iter().any(|x| x.eq_ignore_ascii_case(&d)),
    }
}

/// task-077：每个模块的根目录来自哪些环境变量。
///
/// 这张表是「多盘」判断的唯一依据，所以必须与各模块 `scan()` 里真正读的变量一致：
///
/// | 模块 | 根目录 | 环境变量 |
/// |---|---|---|
/// | temp | `%TEMP%` + `%SystemRoot%\Temp` | `TEMP` + `SystemRoot` |
/// | prefetch | `%SystemRoot%\Prefetch` | `SystemRoot` |
/// | windows_update | `%SystemRoot%\SoftwareDistribution\Download` | `SystemRoot` |
/// | browser_cache | `%LOCALAPPDATA%\...` | `LOCALAPPDATA` |
/// | dev_cache | `%LOCALAPPDATA%\...` + `%APPDATA%\npm-cache` | `LOCALAPPDATA` + `APPDATA` |
///
/// temp 和 dev_cache 是**两个**变量：它们确实可能落在不同的盘上（用户把 TEMP
/// 搬到 D: 而系统在 C:），只取第一个会让另一个盘的条目被误跳过。
///
/// 返回空切片 = 该模块与盘无关（照跑）。这是给将来新增模块的默认行为：
/// 不做判断比做错判断安全。
fn root_env_names(script: &str) -> &'static [&'static str] {
    match script {
        TEMP_SCRIPT => &["TEMP", "SystemRoot"],
        WINDOWS_UPDATE_SCRIPT | PREFETCH_SCRIPT => &["SystemRoot"],
        BROWSER_CACHE_SCRIPT => &["LOCALAPPDATA"],
        DEV_CACHE_SCRIPT => &["LOCALAPPDATA", "APPDATA"],
        _ => &[],
    }
}

/// task-063：读取消标志。没有标志（未挂载）时视为「不取消」。
///
/// 抽成自由函数是为了能直接单测——`ScanOrchestrator` 的构造需要真实
/// `Connection` + `ProgressEmitter`，为测一个布尔判断去搭那套太重。
fn cancel_requested(flag: &Option<std::sync::Arc<std::sync::atomic::AtomicBool>>) -> bool {
    flag.as_ref()
        .map(|c| c.load(std::sync::atomic::Ordering::Relaxed))
        .unwrap_or(false)
}

// ===================== task-084 C1：多用户 Profile =====================
//
// 跨用户扫描的正确语义（对齐任务书 §C1）：
//
// 1. **枚举**：列出 `C:\Users\*` 下所有「有 Desktop 子目录」的用户目录；
// 2. **跳过系统保留目录**（Public / Default / Default User / All Users 等）：它们
//    不是真实用户，扫了只会灌噪音；
// 3. **安全护栏**：只扫「当前用户 + 其他用户的公开部分」。Windows 的 `AppData`
//    （尤其是 `AppData\Local`）默认带 DACL 拒绝其他用户读取，跨用户扫缓存时要
//    逐个判断可读性——读不了的**跳过并在汇总里记录**，而不是报错或假装扫了；
// 4. **结果带 username**：跨用户扫出的条目在 `ScanItem.username` 里标注，最终
//    落进 `cleanup_item.extra` 的 `{"username":"..."}`。

/// 一个可扫描的用户 Profile（task-084 C1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserProfile {
    /// 用户名（`C:\Users\<username>` 里的 `<username>`）。
    pub username: String,
    /// 用户目录绝对路径（如 `C:\Users\Bob`）。
    pub root: String,
    /// 该用户目录下是否有 `Desktop` 子目录（枚举的硬性过滤条件）。
    ///
    /// 为什么用它当「是不是真实用户」的依据：Windows 每个真实用户默认都有桌面；
    /// 系统保留目录（Public / Default）要么没有、要么是模板。有桌面 → 大概率是
    /// 还在用（或曾登录）的真实用户，值得尝试扫。
    pub has_desktop: bool,
    /// 该用户缓存目录**是否可读**（`AppData\Local` 存在且当前进程能列目录）。
    ///
    /// - `true`：可以扫（通常是当前用户自己，或管理员/同一组的其他用户）；
    /// - `false`：权限不足，跨用户扫**跳过**并记录（安全护栏 §C1）。
    ///   单独提成字段而不是靠外层 `read_dir` 返回错误，是为了让「枚举」和
    ///   「可扫性」双双可单测断言，而不是只靠日志。
    pub cache_readable: bool,
}

/// 「系统保留目录」黑名单：这些 `C:\Users\*` 下的目录不是真实用户，扫了只是
/// 噪音（而且 `Default User` / `All Users` 还常是 junction，`read_dir` 会绕圈）。
const SYSTEM_RESERVED_PROFILES: &[&str] = &[
    "Public",
    "Default",
    "Default User",
    "All Users",
    "LocalService",
    "NetworkService",
];

/// 枚举 `C:\Users\*` 下所有用户 Profile（task-084 C1）。
///
/// 返回**全部**有 Desktop 的目录（含当前用户），并按用户名排序保持确定性。
/// 是否真的去扫由调用方按 `cache_readable` 决定。真正 `read_dir` C:\Users 失败
/// （几乎不可能）时返回空表——多用户扫描是附加能力，失败不该让主扫描报错。
pub fn enumerate_user_profiles() -> Vec<UserProfile> {
    let users_root = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
    let root_str = format!(r"{users_root}\Users");
    let root = std::path::Path::new(&root_str);
    if !root.is_dir() {
        // 系统不在预期位置（例如装了多个系统盘扫描别的盘），跳过而非报错。
        eprintln!("[profiles] {root:?} 不是目录，跳过跨用户扫描");
        return Vec::new();
    }
    collect_profiles_from(root)
}

/// task-084 C1：从指定根目录枚举「有 Desktop 的真实用户目录」（纯函数，可单测）。
///
/// 从 [`enumerate_user_profiles`] 抽出来并把根目录做成参数，就是为了能拿临时目录
/// 直接测枚举/过滤/排序逻辑——真正的 `C:\Users` 在 CI 上不存在，没法断言。
fn collect_profiles_from(root: &std::path::Path) -> Vec<UserProfile> {
    let mut out: Vec<UserProfile> = std::fs::read_dir(root)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| e.file_type().ok().filter(|t| t.is_dir()).map(|_| e))
                .map(|e| {
                    let username = e.file_name().to_string_lossy().to_string();
                    let dir = e.path();
                    let has_desktop = dir.join("Desktop").is_dir();
                    // 安全护栏：能列 AppData\Local 才算「可扫」；列不出来（权限不足）就跳过。
                    let cache_readable =
                        std::fs::read_dir(dir.join("AppData").join("Local")).is_ok();
                    UserProfile {
                        username,
                        root: dir.to_string_lossy().to_string(),
                        has_desktop,
                        cache_readable,
                    }
                })
                .filter(|p| {
                    p.has_desktop && !SYSTEM_RESERVED_PROFILES.contains(&p.username.as_str())
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| a.username.cmp(&b.username));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn cancel_flag_none_means_not_cancelled() {
        // 未挂载取消标志 = 老行为（不可取消）
        assert!(!cancel_requested(&None));
    }

    #[test]
    fn cancel_flag_defaults_false_then_flips() {
        let flag = Arc::new(AtomicBool::new(false));
        let opt = Some(flag.clone());
        assert!(!cancel_requested(&opt), "初始应为未取消");

        // scan_cancel 做的就是这一步
        flag.store(true, Ordering::Relaxed);
        assert!(cancel_requested(&opt), "置位后应立即读到 true");
    }

    // ===== 内置模块 ID 契约 =====

    #[test]
    fn rust_script_names_are_in_scripts_list() {
        // 这五个常量是 Rust 分支的判断依据；如果 SCRIPTS 里改了名字而这里没改，
        // 对应模块的 Rust 实现就**永远不会被走到，且没有任何报错**——静默失效。
        for (const_name, script) in [
            ("TEMP_SCRIPT", TEMP_SCRIPT),
            ("WINDOWS_UPDATE_SCRIPT", WINDOWS_UPDATE_SCRIPT),
            ("PREFETCH_SCRIPT", PREFETCH_SCRIPT),
            ("BROWSER_CACHE_SCRIPT", BROWSER_CACHE_SCRIPT),
            ("DEV_CACHE_SCRIPT", DEV_CACHE_SCRIPT),
        ] {
            assert!(
                SCRIPTS.contains(&script),
                "{const_name}={script} 不在 SCRIPTS 里，该模块的 Rust 分支永远不会命中"
            );
        }
        // 五个分发常量都必须登记在扫描顺序表中。
    }

    #[test]
    fn rust_script_names_are_distinct() {
        // 两个常量写成同一个字符串的话，后一个模块会永远走不到（前一个先命中）。
        let names = [
            TEMP_SCRIPT,
            WINDOWS_UPDATE_SCRIPT,
            PREFETCH_SCRIPT,
            BROWSER_CACHE_SCRIPT,
            DEV_CACHE_SCRIPT,
        ];
        for (i, a) in names.iter().enumerate() {
            for b in &names[i + 1..] {
                assert_ne!(a, b, "模块脚本名重复");
            }
        }
    }

    #[test]
    fn cancel_flag_reset_between_scans() {
        // scan_start 每轮复位，避免上一轮的取消让新扫描一启动就停
        let flag = Arc::new(AtomicBool::new(true));
        let opt = Some(flag.clone());
        assert!(cancel_requested(&opt));
        flag.store(false, Ordering::Relaxed);
        assert!(!cancel_requested(&opt), "复位后新扫描应正常跑");
    }

    // ===== task-077：多盘选择 =====

    #[test]
    fn drive_selected_all_when_unset() {
        // 不传 drives = 全部固定盘。写反的话「不选」会变成「一个都不扫」，
        // 用户点开始扫描什么都不会发生，且没有任何报错。
        for d in ['C', 'D', 'Z'] {
            assert!(drive_selected_from(None, d), "{d}: 未限定时应全选");
        }
    }

    #[test]
    fn drive_selected_matches_case_insensitively() {
        // 前端可能传 "c" 也可能传 "C"；比较不忽略大小写就会「选了 C 却没扫 C」。
        let sel = ['C', 'D'];
        assert!(drive_selected_from(Some(&sel), 'C'));
        assert!(drive_selected_from(Some(&sel), 'c'));
        assert!(drive_selected_from(Some(&sel), 'D'));
        assert!(!drive_selected_from(Some(&sel), 'E'), "没选的盘不能扫");
        // 空选择（不该出现，但要有确定行为）：一个都不选
        assert!(!drive_selected_from(Some(&[]), 'C'));
    }

    #[test]
    fn root_env_names_cover_every_script() {
        // 每个模块都必须能在表里查到根目录变量——漏一个的后果是那个模块
        // 「永远不被盘选择影响」，用户只选 D 盘时它照样去扫 C 盘。
        for script in SCRIPTS {
            let names = root_env_names(script);
            assert!(!names.is_empty(), "{script} 没有登记根目录环境变量");
            for n in names {
                // 必须是真实存在的环境变量名（大小写按 Windows 惯例）
                assert!(
                    ["TEMP", "SystemRoot", "LOCALAPPDATA", "APPDATA"].contains(n),
                    "{script} 登记了未知环境变量 {n}"
                );
            }
        }
        // 未知脚本返回空表 = 与盘无关、照跑（将来加模块时的安全默认值）
        assert!(root_env_names("scan_future.ps1").is_empty());
    }

    #[test]
    fn root_env_names_marks_prefetch_and_update_as_system_anchored() {
        // 「Prefetch / Windows Update 只在系统盘跑」这条需求**不靠硬编码 C:**
        // 实现——它们的根目录是 %SystemRoot%，用户不选系统盘就自然跳过。
        // 这条钉住「它们只登记 SystemRoot」这个前提。
        assert_eq!(root_env_names(PREFETCH_SCRIPT), &["SystemRoot"]);
        assert_eq!(root_env_names(WINDOWS_UPDATE_SCRIPT), &["SystemRoot"]);
        // temp / dev_cache 可能跨两个盘，只取一个会让另一个盘的条目被误跳过
        assert_eq!(root_env_names(TEMP_SCRIPT), &["TEMP", "SystemRoot"]);
        assert_eq!(
            root_env_names(DEV_CACHE_SCRIPT),
            &["LOCALAPPDATA", "APPDATA"]
        );
    }

    #[test]
    fn with_drives_normalizes_case() {
        // 入口归一化后，后续所有比较都可以直接用大写
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let emitter = crate::ipc::progress::NoopEmitter;
        let orch = ScanOrchestrator::new(1, &conn, &emitter).with_drives(vec!['c', 'd']);
        assert!(orch.drive_selected('C'), "小写传入也应命中大写查询");
        assert!(orch.drive_selected('D'));
        assert!(!orch.drive_selected('E'));
    }

    /// task-077 真机冒烟：扫真实固定盘，打印按盘汇总（`#[ignore]`，CI 默认不跑）。
    ///
    /// 跑法：
    /// ```text
    /// cargo test -p engine --lib scan::orchestrator -- --ignored --nocapture real_scan_multi_drive
    /// ```
    ///
    /// # 它验证的核心事实
    ///
    /// 5 个模块的根目录全部锚定在**用户 / 系统目录**上（`%TEMP%` / `%APPDATA%` /
    /// `%SystemRoot%`），所以：
    ///
    /// - 默认（不限盘）时，条目全部落在**用户目录所在的那个盘**，其他固定盘
    ///   如实显示 0 项；
    /// - 只选「用户目录不在的那个盘」时，5 个模块**全部跳过**，结果是 0 项而
    ///   **不是** `AllScriptsFailed`——这是「用户选了一个没有清理目标的盘」的
    ///   正确语义，不是错误。
    ///
    /// 这两条如果不写下来，下一个人很容易把「D 盘 0 项」当成 bug。
    #[test]
    #[ignore = "真机冒烟：枚举真实固定盘并扫描"]
    fn real_scan_multi_drive_smoke() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::migrate::migrate(&conn).unwrap();
        conn.execute(
            "INSERT INTO scan_session (id, started_at, status, scope) VALUES (1, 1000, 'running', 'full')",
            [],
        )
        .unwrap();

        let drives = crate::scan::drive_enum::fixed_drives();
        println!("固定盘: {drives:?}");
        let empty = tempfile::tempdir().unwrap();
        let emitter = crate::ipc::progress::NoopEmitter;

        // ① 默认：不限盘
        let all = ScanOrchestrator::new(1, &conn, &emitter)
            .run_all(empty.path())
            .expect("默认扫描应成功");
        println!(
            "① 不限盘：items={} bytes={} ok={:?} skipped={:?}",
            all.total_items, all.total_bytes, all.scripts_succeeded, all.scripts_skipped
        );
        for d in &all.per_drive {
            println!("   {}: {} 项 / {} 字节", d.drive, d.items, d.bytes);
        }
        assert_eq!(all.scripts_succeeded.len(), 5, "5 个模块都应走 Rust 成功");
        assert!(!all.per_drive.is_empty(), "per_drive 不该为空");

        // ② 只选「用户目录不在的那个盘」→ 全跳过，但仍返回 Ok
        let user_drive = std::env::var("LOCALAPPDATA")
            .ok()
            .and_then(|v| crate::scan::drive_enum::drive_letter_of(&v))
            .expect("本机应有 LOCALAPPDATA");
        if let Some(other) = drives.iter().map(|d| d.letter).find(|d| *d != user_drive) {
            // 用**独立的 session**：① 已经往 session 1 里写了七千多行，
            // 共用的话 `per_drive` 会同时含 C: 的旧数据，断言就说明不了问题。
            conn.execute(
                "INSERT INTO scan_session (id, started_at, status, scope) VALUES (2, 2000, 'running', 'full')",
                [],
            )
            .unwrap();
            let only_other = ScanOrchestrator::new(2, &conn, &emitter)
                .with_drives(vec![other])
                .run_all(empty.path())
                .expect("只选无目标的盘不该是错误");
            println!(
                "② 只选 {other}: → items={} skipped={:?}",
                only_other.total_items, only_other.scripts_skipped
            );
            assert!(
                only_other.total_items == 0,
                "该盘上没有缓存扫描目标，不该有条目"
            );
            assert_eq!(
                only_other.scripts_skipped.len(),
                5,
                "5 个模块的根目录都不在这个盘上，应全部跳过"
            );
            assert!(
                only_other
                    .per_drive
                    .iter()
                    .any(|d| d.drive == other && d.items == 0),
                "被选中但 0 项的盘也要出现在 per_drive 里（前端要显示「0 项」）"
            );
            // 独立 session 下不该出现别的盘的条目
            assert!(
                only_other.per_drive.iter().all(|d| d.drive == other),
                "本次只选了 {other}:，per_drive 不该出现其他盘: {:?}",
                only_other.per_drive
            );
        } else {
            println!("② 跳过：本机只有一个固定盘");
        }
    }

    // ===== task-084 C1：多用户 Profile 枚举 =====

    /// 造一个临时 `Users` 根：给定用户名列表，各建 `Desktop`；`readable` 的用户
    /// 再补一个 `AppData\Local`（可读判据）。
    fn make_users_root(names: &[&str], readable: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for name in names {
            let u = dir.path().join(name);
            std::fs::create_dir_all(u.join("Desktop")).unwrap();
        }
        for name in readable {
            let u = dir.path().join(name);
            std::fs::create_dir_all(u.join("AppData").join("Local")).unwrap();
        }
        dir
    }

    #[test]
    fn profiles_include_only_desktop_users() {
        // 只有有 Desktop 的目录才算真实用户；没有 Desktop 的（例如纯模板目录）不算
        let dir = make_users_root(&["Alice", "Ghost", "Bob"], &[]);
        std::fs::create_dir(dir.path().join("GhostNotAUser")).unwrap();
        // Ghost 只有 Desktop 没有 AppData（可读性 false），Alice/Bob 同样没建 Local
        let profiles = collect_profiles_from(dir.path());
        let names: Vec<&str> = profiles.iter().map(|p| p.username.as_str()).collect();
        assert_eq!(
            names,
            vec!["Alice", "Bob", "Ghost"],
            "都应枚举到（有 Desktop），且按名字排序"
        );
        // GhostNotAUser 没有 Desktop，必须被过滤掉
        assert!(!names.contains(&"GhostNotAUser"));
    }

    #[test]
    fn profiles_skip_system_reserved() {
        // Public / Default 等系统保留目录不是真实用户，扫了只会灌噪音
        let dir = make_users_root(
            &["Public", "Default", "Default User", "All Users", "Real"],
            &["Public", "Default", "Default User", "All Users", "Real"],
        );
        let profiles = collect_profiles_from(dir.path());
        let names: Vec<&str> = profiles.iter().map(|p| p.username.as_str()).collect();
        assert_eq!(names, vec!["Real"], "系统保留目录必须全被排除: {names:?}");
    }

    #[test]
    fn profiles_flag_cache_readability() {
        // cache_readable 只反映 AppData\Local 能否列目录——能读的才算可扫
        let dir = make_users_root(&["Alice", "Bob"], &["Alice"]);
        let profiles = collect_profiles_from(dir.path());
        let alice = profiles.iter().find(|p| p.username == "Alice").unwrap();
        let bob = profiles.iter().find(|p| p.username == "Bob").unwrap();
        assert!(alice.cache_readable, "Alice 有 AppData\\Local，应可扫");
        assert!(!bob.cache_readable, "Bob 没有 AppData\\Local，应标记不可扫");
        // root 是绝对路径、且带用户名
        assert!(alice.root.ends_with("Alice"));
    }

    #[test]
    fn profiles_sort_is_deterministic() {
        // 枚举顺序不保证，但产出必须按用户名排序——单测和 UI 分组都依赖确定性
        let dir = make_users_root(&["Zoe", "Amy", "Ken"], &[]);
        let profiles = collect_profiles_from(dir.path());
        let names: Vec<&str> = profiles.iter().map(|p| p.username.as_str()).collect();
        assert_eq!(
            names,
            vec!["Amy", "Ken", "Zoe"],
            "必须按用户名升序，避免顺序抖动"
        );
    }

    #[test]
    fn with_username_tags_scan_item() {
        let it = crate::scan::modules::ScanItem::new("temp", "user_temp", "C:/x".into(), 1, 1);
        assert_eq!(it.username, None, "默认不标注（当前用户内置模块）");
        let tagged = it.clone().with_username("Bob");
        assert_eq!(tagged.username.as_deref(), Some("Bob"));
        assert_eq!(it.username, None, "with_username 不得原地改原条目");
    }
}
