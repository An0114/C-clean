//! 删除动作唯一入口（提交 2 仅骨架，串联逻辑见提交 5）。

use super::normalize::{normalize, NormalizedPath};
use super::resolve::resolve_to_real;
use super::whitelist::{self, DenialReason};

/// 真实删除器抽象：后续 safebox/platform 任务注入真实实现，
/// 本任务只提供 [`NoopDeleter`]（测试用，不碰磁盘）。
pub trait Deleter {
    /// 删除一个已通过白名单的文件。
    ///
    /// **返回值语义（task-052 G5）**：`Ok(true)` = 本次**真的**删掉/移走了文件；
    /// `Ok(false)` = 文件在删除前已不存在（NotFound 幂等），本次**什么都没删**。
    ///
    /// 为什么需要区分：扫描与清理之间存在时间窗，文件可能已被系统或其他程序删掉。
    /// 旧签名只返回 `Ok(())`，调用方无法分辨「真删了」和「本来就没有」，于是把
    /// 0 字节计入释放量，前端出现「已清理 1 项，释放 0 B」的荒谬文案（走查 G5）。
    fn delete_file(&self, p: &NormalizedPath) -> Result<bool, String>;
    /// 删除一个已通过白名单的目录。
    fn delete_dir(&self, p: &NormalizedPath) -> Result<(), String>;
}

/// 测试用删除器：仅把「本应被删」的路径名记录到内部 Vec，绝不触碰磁盘。
#[derive(Default)]
pub struct NoopDeleter {
    recorded: std::sync::Mutex<Vec<String>>,
}

impl NoopDeleter {
    /// 取出已记录路径（测试断言用）。
    pub fn recorded(&self) -> Vec<String> {
        self.recorded.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

impl Deleter for NoopDeleter {
    fn delete_file(&self, p: &NormalizedPath) -> Result<bool, String> {
        self.recorded
            .lock()
            .map(|mut g| {
                g.push(p.normalized().to_string());
                // Noop 不碰磁盘，一律当作「真删了」。
                true
            })
            .map_err(|e| e.to_string())
    }

    fn delete_dir(&self, p: &NormalizedPath) -> Result<(), String> {
        self.recorded
            .lock()
            .map(|mut g| g.push(p.normalized().to_string()))
            .map_err(|e| e.to_string())
    }
}

/// 删除请求：待删路径列表 + 动作列表（动作级检查用）。
#[derive(Debug, Clone)]
pub struct ExecuteRequest {
    /// 待删路径（任意字符串，会先经规范化）。
    pub paths: Vec<String>,
    /// 动作字符串列表，例如 `["dism", "registry_delete"]`。
    pub actions: Vec<String>,
}

/// 单条路径的判定结果（审计/UI 展示用）。
#[derive(Debug, Clone)]
pub struct PathDecision {
    /// 原始输入。
    pub input: String,
    /// 规范化结果；规范化失败时为 None。
    pub normalized: Option<String>,
    /// 是否允许删除。
    pub allowed: bool,
    /// 拒绝原因（allowed=false 时必有值）。
    pub denial_reason: Option<DenialReason>,
}

/// 整体决策结果。`code = 2002` 对应 ARCHITECTURE §2.3 白名单拒绝信封码。
#[derive(Debug)]
pub enum ExecuteOutcome {
    /// 全部路径与动作通过红线校验。
    Allowed { decisions: Vec<PathDecision> },
    /// 任一路径或动作命中红线，整体拒绝（此时不调用 Deleter）。
    Rejected {
        decisions: Vec<PathDecision>,
        code: u16,
    },
}

/// execute() 自身的错误（与「业务拒绝」不同：拒绝是正常返回值，错误是程序异常）。
#[derive(Debug, thiserror::Error)]
pub enum ExecuteError {
    #[error("internal error: {0}")]
    Internal(String),
}

/// 单条路径被拒的公共收尾：翻标记 + 落一条 PathDecision。
#[allow(clippy::too_many_arguments)]
fn push_rejected(
    decisions: &mut Vec<PathDecision>,
    any_rejected: &mut bool,
    raw: &str,
    normalized: String,
    reason: DenialReason,
) {
    *any_rejected = true;
    decisions.push(PathDecision {
        input: raw.to_string(),
        normalized: Some(normalized),
        allowed: false,
        denial_reason: Some(reason),
    });
}

/// 白名单拒绝信封码（ARCHITECTURE §2.3：code=2002 / audit_log: whitelist_denied）。
pub const REJECT_CODE: u16 = 2002;

/// 删除唯一入口：逐条规范化 → 跑内置红线 → 动作级禁令；
/// 任一项不通过即整体 [`ExecuteOutcome::Rejected`]，**绝不调用 Deleter**；
/// 全部通过返回 [`ExecuteOutcome::Allowed`]（本任务也不自动调 Deleter，
/// 由上层 safebox/IPC 任务拿到 Allowed 后再决定是否真删）。
pub fn execute(
    req: &ExecuteRequest,
    _deleter: &dyn Deleter,
) -> Result<ExecuteOutcome, ExecuteError> {
    let mut decisions: Vec<PathDecision> = Vec::with_capacity(req.paths.len());
    let mut any_path_rejected = false;

    // 1. 逐条路径：规范化 → 真实路径解析（task-008）→ 用真实路径跑内置红线。
    for raw in &req.paths {
        let np = match normalize(raw) {
            Ok(np) => np,
            // 规范化失败统一折算为 PathTraversal（任务契约）。
            Err(_) => {
                any_path_rejected = true;
                decisions.push(PathDecision {
                    input: raw.clone(),
                    normalized: None,
                    allowed: false,
                    denial_reason: Some(DenialReason::PathTraversal),
                });
                continue;
            }
        };

        // 解析到磁盘真实路径：挡住 junction/symlink 重定向（task-008）。
        // 注意：解析失败（如路径尚不存在）时不放宽红线——仍对输入路径做 L3 判定，
        // 命中 L3 则返回对应变体，否则才折算为 ResolveFailed。
        match resolve_to_real(&np) {
            Ok(resolved) => match normalize(&resolved.real) {
                Ok(real_np) => match whitelist::classify(&real_np) {
                    Ok(()) => decisions.push(PathDecision {
                        input: raw.clone(),
                        normalized: Some(real_np.normalized().to_string()),
                        allowed: true,
                        denial_reason: None,
                    }),
                    Err(reason) => push_rejected(
                        &mut decisions,
                        &mut any_path_rejected,
                        raw,
                        real_np.normalized().to_string(),
                        reason,
                    ),
                },
                Err(_) => push_rejected(
                    &mut decisions,
                    &mut any_path_rejected,
                    raw,
                    resolved.real,
                    DenialReason::PathTraversal,
                ),
            },
            Err(e) => {
                // 解析失败的分级处理：
                // - 输入路径命中 L3 → 直接返回 L3 变体（不存在也不能放行红线）；
                // - 路径不存在（NotExists）→ 信任输入字符串放行（真实扫描器只会传
                //   磁盘上已存在的条目，单测里 L1 临时文件是不存在的占位）；
                // - 其他 IO 错误（UNC / 环路等）→ 记 ResolveFailed 拒绝。
                match whitelist::classify(&np) {
                    Ok(()) if matches!(e, crate::execute::resolve::ResolveError::NotExists(_)) => {
                        decisions.push(PathDecision {
                            input: raw.clone(),
                            normalized: Some(np.normalized().to_string()),
                            allowed: true,
                            denial_reason: None,
                        });
                    }
                    Ok(()) => push_rejected(
                        &mut decisions,
                        &mut any_path_rejected,
                        raw,
                        np.normalized().to_string(),
                        DenialReason::ResolveFailed(e.to_string()),
                    ),
                    Err(reason) => push_rejected(
                        &mut decisions,
                        &mut any_path_rejected,
                        raw,
                        np.normalized().to_string(),
                        reason,
                    ),
                }
            }
        }
    }

    // 2. 动作级禁令：任一动作被禁 → 整体拒绝。
    let action_rejected = req
        .actions
        .iter()
        .any(|a| whitelist::check_action(a).is_err());

    if any_path_rejected || action_rejected {
        return Ok(ExecuteOutcome::Rejected {
            decisions,
            code: REJECT_CODE,
        });
    }

    Ok(ExecuteOutcome::Allowed { decisions })
}
