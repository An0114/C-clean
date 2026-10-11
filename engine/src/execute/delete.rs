//! 删除动作唯一入口（提交 2 仅骨架，串联逻辑见提交 5）。

use super::normalize::NormalizedPath;
use super::whitelist::DenialReason;

/// 真实删除器抽象：后续 safebox/platform 任务注入真实实现，
/// 本任务只提供 [`NoopDeleter`]（测试用，不碰磁盘）。
pub trait Deleter {
    /// 删除一个已通过白名单的文件。
    fn delete_file(&self, p: &NormalizedPath) -> Result<(), String>;
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
    fn delete_file(&self, p: &NormalizedPath) -> Result<(), String> {
        self.recorded
            .lock()
            .map(|mut g| g.push(p.normalized().to_string()))
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

/// 删除唯一入口：规范化 → 内置红线 → 动作级禁令；
/// 任一项不通过即整体 [`ExecuteOutcome::Rejected`]，绝不调用 Deleter。
pub fn execute(
    _req: &ExecuteRequest,
    _deleter: &dyn Deleter,
) -> Result<ExecuteOutcome, ExecuteError> {
    // 提交 5 串联完整逻辑。
    Err(ExecuteError::Internal("execute not implemented yet".into()))
}
