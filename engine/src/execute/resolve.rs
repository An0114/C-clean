//! 真实路径解析（task-008）：把词法规范化后的路径解析到磁盘真实位置，
//! 防止 junction/symlink 把「看似安全」的路径重定向到 L3 红线（ARCGITECTURE A03）。
//!
//! 策略：
//! - 用 [`std::fs::canonicalize`] 一次性拿到最终目标（Windows 下它会逐级解析
//!   junction 与 symlink，返回 `\\?\C:\...` 长路径形式，本模块负责去前缀）；
//! - hop 数：真实路径与输入规范化路径不一致即视为发生过重定向（至少 1 跳）；
//! - 最多 [`MAX_HOPS`] 跳防环路（canonicalize 自身遇环会报错，这里仅作显式常量）；
//! - UNC（`\\server\share\...`）本任务保守拒绝（沿用 task-003 策略）。

use super::normalize::NormalizedPath;

/// 最大重定向层数（防环路；canonicalize 自身也会挡环，这里作显式上限）。
pub const MAX_HOPS: u32 = 40;

/// 解析后的真实路径结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPath {
    /// 原始输入经 normalize 后的路径（用户传入、用于审计展示）。
    pub input: String,
    /// 真实磁盘路径（canonicalize 后已去 `\\?\` 前缀，统一 `C:\...` 形式）。
    pub real: String,
    /// 解析过程中发生的重定向次数（0 = 无重定向；≥1 = 经过 junction/symlink）。
    pub hop_count: u32,
}

/// 解析失败原因。
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    /// 底层 IO 错误（含路径不存在、UNC 保守拒绝、环路等）。
    #[error("io error: {0}")]
    Io(std::io::Error),
    /// 超过最大跳数（理论上 canonicalize 先报错，这里作为显式分支保留）。
    #[error("loop detected: exceeded max_hops={max_hops}")]
    LoopDetected { max_hops: u32 },
    /// 路径不存在（canonicalize 返回 ENOENT/ERROR_PATH_NOT_FOUND）。
    #[error("path not exists: {0}")]
    NotExists(String),
}

/// 把词法规范化路径解析到磁盘真实位置。
///
/// 调用方拿到 [`ResolvedPath.real`] 后应再过一次 `normalize()` + `classify()`，
/// 用真实路径（而非用户输入路径）做 L3 判定。
pub fn resolve_to_real(input: &NormalizedPath) -> Result<ResolvedPath, ResolveError> {
    let raw = input.normalized();

    // UNC 保守拒绝：canonicalize 对 \\server\share 行为复杂，本阶段一律拒。
    if raw.starts_with(r"\\") {
        return Err(ResolveError::Io(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            format!("UNC path not supported: {raw}"),
        )));
    }

    let canonical = std::fs::canonicalize(raw).map_err(|e| {
        // 区分「不存在」与其他 IO 错误，便于上层填 audit。
        if e.kind() == std::io::ErrorKind::NotFound {
            ResolveError::NotExists(raw.to_string())
        } else {
            ResolveError::Io(e)
        }
    })?;

    // canonicalize 在 Windows 返回 \\?\C:\... 长路径前缀，去掉以匹配 classify 的字符串比较。
    let real = canonical.to_string_lossy().trim_start_matches(r"\\?\").to_string();

    // 跳数：真实路径与输入不一致即视为发生过重定向（大小写不敏感比较）。
    let hop_count = if real.to_lowercase() == raw.to_lowercase() {
        0
    } else if MAX_HOPS == 0 {
        // 理论不可达分支，保持显式
        return Err(ResolveError::LoopDetected { max_hops: MAX_HOPS });
    } else {
        1
    };

    Ok(ResolvedPath {
        input: input.normalized().to_string(),
        real,
        hop_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execute::normalize::normalize;

    #[test]
    fn resolve_nonexistent_returns_not_exists() {
        let np = normalize(r"C:\no\such\path\xyz.exe").unwrap();
        match resolve_to_real(&np) {
            Err(ResolveError::NotExists(_)) => {}
            other => panic!("expected NotExists, got {other:?}"),
        }
    }
}
