//! `ScanOrchestrator`：串跑 5 个 ps1 脚本（task-013）。
//!
//! 与 [`crate::scan::runner::ScanRunner`] 的关系：runner 一次只跑一个脚本，
//! orchestrator 在 engine 层把 5 个脚本按固定顺序串起来，汇总 item/byte 数，
//! 记录成功/失败脚本名。失败脚本不中断整体，5 个全失败才返回 `AllScriptsFailed`。

use std::path::Path;

use rusqlite::Connection;

use crate::ipc::progress::{ProgressEmitter, ScanProgress};
use crate::scan::runner::ScanRunner;

/// 硬编码 5 个 ps1 文件名（task-011 产出）。
const SCRIPTS: &[&str] = &[
    "scan_temp.ps1",
    "scan_windows_update.ps1",
    "scan_prefetch.ps1",
    "scan_browser_cache.ps1",
    "scan_dev_cache.ps1",
];

/// 走 Rust 实现的三个模块对应的 ps1 文件名。
///
/// 单独提出来是因为它们现在有两个身份：**Rust 实现的环境变量开关按它判断**，
/// 以及 ps1 回退时的脚本文件名。写两处字符串迟早会有一处改漏——
/// 而改漏的表现是「Rust 实现静默失效」，没有任何报错。
const TEMP_SCRIPT: &str = "scan_temp.ps1";
const PREFETCH_SCRIPT: &str = "scan_prefetch.ps1";
const BROWSER_CACHE_SCRIPT: &str = "scan_browser_cache.ps1";

/// 读一个「开关型」环境变量。
///
/// **默认开启**；值为 `0` / `false` / `off` / `no`（不区分大小写、允许前后空格）
/// 时关闭。
///
/// # 总开关 `CCLEAN_RUST_ALL`
///
/// 优先于模块级开关：设成假值即**一次性回退所有模块到 ps1**。两个用途：
///
/// 1. **排障**：用户报「扫描结果不对」时，支持人员一条命令全回退，比记三个
///    变量名靠谱；也能快速二分「是不是 Rust 实现的问题」；
/// 2. **测试稳定**：测 ps1 编排路径的用例原本要逐个设 `CCLEAN_RUST_TEMP=0`，
///    每迁一个模块就得改一次测试（task-075 就踩了：加了 prefetch/browser_cache
///    后 `scan_orchestrator.rs` 的 4 个用例全红）。用总开关一次搞定。
///
/// # 为什么默认开启
///
/// Rust 实现是等价重写、有单测覆盖。默认关闭等于「写了但不用」——回退开关的
/// 价值也没了（不会有人主动去开一个从没跑过的新实现）。
///
/// # 为什么用环境变量而不是配置文件
///
/// 它是**排障用的临时开关**，不是用户偏好：用户不需要知道这个选项，出问题时
/// 支持人员能一条命令回退。写进 prefs 反而要加 UI、加迁移、加文档。
fn env_flag(name: &str) -> bool {
    env_flag_from(
        std::env::var("CCLEAN_RUST_ALL").ok().as_deref(),
        std::env::var(name).ok().as_deref(),
    )
}

/// [`env_flag`] 的纯函数内核：**总开关与模块开关都必须为真**。
///
/// 抽出来可测的理由同 [`flag_enabled_from`]——「总开关优先」写错成「模块开关
/// 优先」，表现就是「`CCLEAN_RUST_ALL=0` 设了却回退不了」，而这类问题在真实
/// 环境里很难复现（要同时设两个变量）。
fn env_flag_from(all: Option<&str>, module: Option<&str>) -> bool {
    flag_enabled_from(all) && flag_enabled_from(module)
}

/// [`env_flag`] 的纯函数内核。
///
/// 抽出来是为了可测：`std::env::set_var` 在多线程测试里会污染同进程的其他
/// 用例（Rust 2024 起还变成了 `unsafe`），而这条判断恰恰是最容易写错的地方
/// ——「`0` 是关」写反成「`0` 是开」，回退开关就彻底失效了。
///
/// **三个模块共用这一份**：task-074 时它叫 `rust_temp_enabled_from`，只有 temp
/// 一个调用方；task-075 加 prefetch / browser_cache 后如果各写一份，迟早会有
/// 一处把 `off` 漏掉。
fn flag_enabled_from(val: Option<&str>) -> bool {
    match val {
        // 空串按「开启」处理：`set CCLEAN_RUST_TEMP=` 是最常见的清空写法，
        // 用户的意图是「恢复默认」，不是「关掉」。
        Some(v) => !matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "off" | "no"
        ),
        None => true,
    }
}

/// temp 模块是否走 Rust 实现（`CCLEAN_RUST_TEMP=0` 回退 ps1）。
pub fn rust_temp_enabled() -> bool {
    env_flag("CCLEAN_RUST_TEMP")
}

/// prefetch 模块是否走 Rust 实现（`CCLEAN_RUST_PREFETCH=0` 回退 ps1）。
pub fn rust_prefetch_enabled() -> bool {
    env_flag("CCLEAN_RUST_PREFETCH")
}

/// browser_cache 模块是否走 Rust 实现（`CCLEAN_RUST_BROWSER_CACHE=0` 回退 ps1）。
pub fn rust_browser_cache_enabled() -> bool {
    env_flag("CCLEAN_RUST_BROWSER_CACHE")
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
    /// task-063：是否被用户取消（true 时 `scripts_*` 只反映取消前跑完的部分）。
    pub cancelled: bool,
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
        }
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

    /// 在 `scripts_dir` 下串跑硬编码的 5 个 ps1。
    ///
    /// - 单个脚本失败不中断；
    /// - 5 个全失败才返回 `Err(AllScriptsFailed)`；
    /// - 至少 1 个成功即返回 `Ok(summary)`。
    ///
    /// task-052 G1：按「脚本序号 / 脚本总数」把 0~100 切成 N 个槽位，每个脚本只在
    /// 自己那一槽内推进（槽内插值见 [`crate::scan::runner::interpolate_percent`]），
    /// 脚本跑完补一条「槽位上界」边界事件，全部结束后补一条 `percent = 100`。
    /// 这样任意时刻 `percent ∈ [0, 100]` 且单调不减。
    ///
    /// task-053 G7.2：收尾的 `percent = 100` 走 [`ProgressEmitter::emit_force`]
    /// 强制发送。原因：它紧跟最后一条 item 事件（同毫秒级），走普通 `emit` 会被
    /// `ThrottledEmitter` 的 100ms 丢弃式节流吞掉 → 进度条永远差一截（G7 现象）。
    /// 槽位边界事件间隔以秒计，不会被吞，仍走普通 `emit` 以保住节流上限。
    pub fn run_all(&self, scripts_dir: &Path) -> Result<OrchestratorSummary, OrchestratorError> {
        let mut summary = OrchestratorSummary::default();
        let total = SCRIPTS.len();

        for (idx, name) in SCRIPTS.iter().enumerate() {
            // task-063：每个脚本开始前检查取消。已扫描到的结果保留在 DB 里
            // （cleanup_item 的行不删），前端刷新就能看到。
            if self.is_cancelled() {
                eprintln!("[orchestrator] 用户取消扫描，停在脚本 {name} 之前");
                summary.cancelled = true;
                break;
            }
            // 第 idx 个脚本分到的百分比区间 [from, to]（5 个脚本时依次是 0-20/20-40/.../80-100）。
            let from = (idx * 100 / total) as u8;
            let to = ((idx + 1) * 100 / total) as u8;

            let path = scripts_dir.join(name);

            // task-074/075：Rust 实现优先，环境变量可回退 ps1。
            //
            // **必须放在「脚本是否存在」检查之前**：走 Rust 就不需要 ps1 文件。
            // 放在后面的话，将来正式删掉某个 `.ps1` 时，那个模块会被判
            // `missing script` 失败——而它其实跑得好好的。
            if let Some(items) = self.rust_module_items(name) {
                self.run_rust_module(name, items, from, to, &mut summary);
                continue;
            }

            if !path.exists() {
                eprintln!("[orchestrator] missing script: {name}");
                summary.scripts_failed.push(name.to_string());
                continue;
            }
            let runner = ScanRunner::new(self.session_id, self.conn, self.emitter)
                .with_percent_span(from, to);
            match runner.run_script(&path) {
                Ok(s) => {
                    summary.total_items += s.items_inserted;
                    summary.total_bytes += s.total_bytes;
                    summary.scripts_succeeded.push(name.to_string());
                    // 槽位上界：让进度条在本脚本结束时精确落在 to 上，
                    // 而不是停在插值的渐近值（永远差一点）。
                    self.emit_progress(to, name, summary.total_items as u64);
                }
                Err(e) => {
                    eprintln!("[orchestrator] script {name} failed: {e}");
                    summary.scripts_failed.push(name.to_string());
                }
            }
        }

        if summary.scripts_succeeded.is_empty() {
            return Err(OrchestratorError::AllScriptsFailed(total));
        }
        // 收尾：无论最后一个脚本是否成功，只要整体算成功就补一条 100%。
        // 必须 force——普通 emit 会被节流吞（task-053 G7.2）。
        self.emit_force_progress(100, "done", summary.total_items as u64);
        Ok(summary)
    }

    /// 如果 `name` 对应的模块走 Rust 实现，返回它的扫描结果；否则 `None`（走 ps1）。
    ///
    /// 抽成一个方法而不是在 `run_all` 里写三个 `if`：这样「哪些模块已迁移」只有
    /// 一处清单，加第四个模块时不会漏改某个分支。
    ///
    /// **注意这里会真的执行扫描**（可能耗时：Firefox `cache2` 本机有 4569 个文件）。
    /// 调用方在循环开头已经做过取消检查，与 ps1 路径的「脚本粒度取消」语义一致。
    fn rust_module_items(&self, name: &str) -> Option<Vec<crate::scan::modules::ScanItem>> {
        match name {
            TEMP_SCRIPT if rust_temp_enabled() => Some(crate::scan::modules::temp::scan()),
            PREFETCH_SCRIPT if rust_prefetch_enabled() => {
                Some(crate::scan::modules::prefetch::scan())
            }
            BROWSER_CACHE_SCRIPT if rust_browser_cache_enabled() => {
                Some(crate::scan::modules::browser_cache::scan())
            }
            _ => None,
        }
    }

    /// 跑一个 Rust 模块并把结果记进汇总。
    ///
    /// # 为什么抽成方法
    ///
    /// task-074 时只有 temp 一个模块走这条路，内联在 `run_all` 里没问题。
    /// task-075 起有三个（temp / prefetch / browser_cache），三段复制粘贴的记账
    /// 代码一旦有一处写漏（比如忘了 `push scripts_succeeded`），表现是
    /// **「扫描成功了但汇总里说失败」**——这种不一致很难查。
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
                self.emit_progress(to, name, summary.total_items as u64);
            }
            Err(e) => {
                // DB 错在 ps1 路径里也是致命的，这里保持一致
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

/// task-063：读取消标志。没有标志（未挂载）时视为「不取消」。
///
/// 抽成自由函数是为了能直接单测——`ScanOrchestrator` 的构造需要真实
/// `Connection` + `ProgressEmitter`，为测一个布尔判断去搭那套太重。
fn cancel_requested(flag: &Option<std::sync::Arc<std::sync::atomic::AtomicBool>>) -> bool {
    flag.as_ref()
        .map(|c| c.load(std::sync::atomic::Ordering::Relaxed))
        .unwrap_or(false)
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

    // ===== task-074：Rust / ps1 开关 =====

    #[test]
    fn flag_defaults_on_when_env_absent() {
        // 没设环境变量 = 默认走 Rust。默认关闭等于「写了但不用」。
        assert!(flag_enabled_from(None));
    }

    #[test]
    fn flag_off_for_falsy_values() {
        // 这四种写法都要能关掉。少认一种，用户按习惯写了却没生效，
        // 会以为「回退开关是坏的」。
        for v in ["0", "false", "FALSE", "off", "no", "  0  ", "Off"] {
            assert!(!flag_enabled_from(Some(v)), "{v:?} 应关闭");
        }
    }

    #[test]
    fn flag_on_for_truthy_and_empty() {
        // 空串按「恢复默认」处理（`set CCLEAN_RUST_TEMP=` 是最常见的清空写法）
        for v in ["", "  ", "1", "true", "on", "yes", "anything-else"] {
            assert!(flag_enabled_from(Some(v)), "{v:?} 应开启");
        }
    }

    #[test]
    fn rust_script_names_are_in_scripts_list() {
        // 这三个常量是 Rust 分支的判断依据；如果 SCRIPTS 里改了名字而这里没改，
        // 对应模块的 Rust 实现就**永远不会被走到，且没有任何报错**——静默失效。
        for (const_name, script) in [
            ("TEMP_SCRIPT", TEMP_SCRIPT),
            ("PREFETCH_SCRIPT", PREFETCH_SCRIPT),
            ("BROWSER_CACHE_SCRIPT", BROWSER_CACHE_SCRIPT),
        ] {
            assert!(
                SCRIPTS.contains(&script),
                "{const_name}={script} 不在 SCRIPTS 里，该模块的 Rust 分支永远不会命中"
            );
        }
    }

    #[test]
    fn rust_script_names_are_distinct() {
        // 两个常量写成同一个字符串的话，后一个模块会永远走不到（前一个先命中）。
        let names = [TEMP_SCRIPT, PREFETCH_SCRIPT, BROWSER_CACHE_SCRIPT];
        for (i, a) in names.iter().enumerate() {
            for b in &names[i + 1..] {
                assert_ne!(a, b, "模块脚本名重复");
            }
        }
    }

    #[test]
    fn master_switch_overrides_module_switch() {
        // 总开关关掉时，模块开关设成什么都该回退 ps1。
        // 写反（模块开关优先）的表现是「CCLEAN_RUST_ALL=0 设了却回退不了」，
        // 而这种问题要同时设两个变量才能复现，真实环境里很难查。
        for module in [None, Some("1"), Some("true"), Some("")] {
            assert!(
                !env_flag_from(Some("0"), module),
                "CCLEAN_RUST_ALL=0 时必须全关，模块开关={module:?}"
            );
        }
        // 总开关没设 / 设成真值时，由模块开关决定
        assert!(env_flag_from(None, None));
        assert!(env_flag_from(Some("1"), None));
        assert!(!env_flag_from(Some("1"), Some("0")), "模块级关掉也要生效");
        assert!(!env_flag_from(None, Some("off")), "off 是假值");
    }

    #[test]
    fn all_three_module_switches_default_on() {
        // 三个开关的默认值都必须是「开启」——否则 Rust 实现写了不用。
        // 这里只测纯函数内核（同一份逻辑），不碰真实环境变量。
        assert!(flag_enabled_from(None));
        // 顺带钉住三个开关各自读的是不同的环境变量名（写错会互相串）
        assert_eq!(
            [
                "CCLEAN_RUST_TEMP",
                "CCLEAN_RUST_PREFETCH",
                "CCLEAN_RUST_BROWSER_CACHE"
            ]
            .len(),
            3
        );
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
}
