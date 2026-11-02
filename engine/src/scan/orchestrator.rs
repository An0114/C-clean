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

/// task-074：temp 模块对应的 ps1 文件名。
///
/// 单独提出来是因为它现在有两个身份：**Rust 实现的环境变量开关按它判断**，
/// 以及 ps1 回退时的脚本文件名。写两处字符串迟早会有一处改漏。
const TEMP_SCRIPT: &str = "scan_temp.ps1";

/// task-074：temp 模块是否走 Rust 实现。
///
/// **默认开启**；`CCLEAN_RUST_TEMP=0`（或 `false` / `off` / `no`）回退 ps1。
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
///
/// 抽成 `pub` 是为了能单测——`run_all` 需要真实 DB + emitter，为测一个布尔
/// 判断去搭那套太重。
pub fn rust_temp_enabled() -> bool {
    rust_temp_enabled_from(std::env::var("CCLEAN_RUST_TEMP").ok().as_deref())
}

/// [`rust_temp_enabled`] 的纯函数内核。
///
/// 抽出来是为了可测：`std::env::set_var` 在多线程测试里会污染同进程的其他
/// 用例（Rust 2024 起还变成了 `unsafe`），而这条判断恰恰是最容易写错的地方
/// ——「`0` 是关」写反成「`0` 是开」，回退开关就彻底失效了。
fn rust_temp_enabled_from(val: Option<&str>) -> bool {
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

            // task-074：temp 走 Rust 实现（默认），`CCLEAN_RUST_TEMP=0` 回退 ps1。
            //
            // **必须放在「脚本是否存在」检查之前**：走 Rust 就不需要 ps1 文件。
            // 放在后面的话，将来正式删掉 `scan_temp.ps1` 时，这个模块会被判
            // `missing script` 失败——而它其实跑得好好的。
            if *name == TEMP_SCRIPT && rust_temp_enabled() {
                let runner = ScanRunner::new(self.session_id, self.conn, self.emitter)
                    .with_percent_span(from, to);
                let items = crate::scan::modules::temp::scan();
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
    fn rust_temp_defaults_on_when_env_absent() {
        // 没设环境变量 = 默认走 Rust。默认关闭等于「写了但不用」。
        assert!(rust_temp_enabled_from(None));
    }

    #[test]
    fn rust_temp_off_for_falsy_values() {
        // 这四种写法都要能关掉。少认一种，用户按习惯写了却没生效，
        // 会以为「回退开关是坏的」。
        for v in ["0", "false", "FALSE", "off", "no", "  0  ", "Off"] {
            assert!(!rust_temp_enabled_from(Some(v)), "{v:?} 应关闭");
        }
    }

    #[test]
    fn rust_temp_on_for_truthy_and_empty() {
        // 空串按「恢复默认」处理（`set CCLEAN_RUST_TEMP=` 是最常见的清空写法）
        for v in ["", "  ", "1", "true", "on", "yes", "anything-else"] {
            assert!(rust_temp_enabled_from(Some(v)), "{v:?} 应开启");
        }
    }

    #[test]
    fn temp_script_name_is_in_scripts_list() {
        // TEMP_SCRIPT 是 Rust 分支的判断依据；如果 SCRIPTS 里改了名字而这里
        // 没改，Rust 实现就永远不会被走到，且**没有任何报错**——静默失效。
        assert!(
            SCRIPTS.contains(&TEMP_SCRIPT),
            "TEMP_SCRIPT={TEMP_SCRIPT} 不在 SCRIPTS 里，Rust 分支永远不会命中"
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
