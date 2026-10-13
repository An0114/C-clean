//! 真实路径解析（task-008）：把词法规范化后的路径解析到磁盘真实位置，
//! 防止 junction/symlink 把「看似安全」的路径重定向到 L3 红线（ARCHITECTURE A03）。
//!
//! 策略：
//! - 用 [`std::fs::canonicalize`] 一次性拿到最终目标（Windows 下它会逐级解析
//!   junction 与 symlink，返回 `\\?\C:\...` 长路径形式，本模块负责去前缀）；
//! - hop 数：逐级 [`std::fs::symlink_metadata`] 查 `FILE_ATTRIBUTE_REPARSE_POINT`，
//!   遇到真正的 junction/symlink 才计 1 跳。**不再用 canonicalize 后的字符串
//!   与输入做相等比较**——CI runner 的 TEMP 用 8.3 短名（`RUNNER~1`），
//!   canonicalize 会解析成长名（`runneradmin`），字符串不等会误判 hop=1。
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

/// Windows: FILE_ATTRIBUTE_REPARSE_POINT（junction / symlink / 其他重解析点）。
#[cfg(windows)]
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

/// 逐级查 reparse point，统计路径里真正的 junction/symlink 数。
///
/// `normalized` 形如 `C:\Users\runneradmin\AppData\Local\Temp\hello.txt`。
/// 对每个前缀目录调用 [`std::fs::symlink_metadata`]（不跟随最终符号链接），
/// 检查 Windows 文件属性里的 REPARSE_POINT 位。
///
/// 8.3 短名（`RUNNER~1`）不是 reparse point——NTFS 直接把它解析成长名，
/// 返回的 metadata 不带 REPARSE_POINT 位，因此不会被误计。
#[cfg(windows)]
fn count_reparse_hops(normalized: &str) -> u32 {
    use std::os::windows::fs::MetadataExt;

    let mut parts = normalized.split('\\');
    let drive = parts.next().unwrap_or(""); // "C:"
    let mut current = format!("{drive}\\");
    let mut hops = 0u32;
    for comp in parts {
        if comp.is_empty() {
            continue;
        }
        current.push_str(comp);
        // symlink_metadata 不跟随最终组件的符号链接；中间层 junction 会被
        // Windows 自动穿透，但我们只看「当前这个组件本身」的属性位——
        // 它是 junction 时属性里会带 REPARSE_POINT。
        if let Ok(md) = std::fs::symlink_metadata(&current) {
            if md.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                hops += 1;
                if hops >= MAX_HOPS {
                    return hops;
                }
            }
        }
        current.push('\\');
    }
    hops
}

/// 非 Windows 平台占位（本项目 Windows-only，仅为编译通过）。
#[cfg(not(windows))]
fn count_reparse_hops(_normalized: &str) -> u32 {
    0
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
    let real = canonical
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_string();

    // hop 数：逐级查 reparse point（不再靠 canonicalize 字符串比对）。
    let hop_count = count_reparse_hops(raw);

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
