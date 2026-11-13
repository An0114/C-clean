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
//! - UNC（`\\server\share\...`）task-086 5.4 起允许解析（用户规则可指向网络盘，
//!   固定 L2 需确认）；`\\?\UNC\` 前缀统一还原为 `\\server\share\...`。

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

/// 用 Windows `GetLongPathNameW` 把路径中的 8.3 短名（如 `SYSTEM~1`）展开为长名
/// （如 `System32`）。task-086 2.3：短名转长名后再走 L3 匹配，否则
/// `C:\Windows\SYSTEM~1\kernel32.dll` 绕不过 `system32` 红线检查。
///
/// # 失败语义
///
/// 展开失败（路径不存在 / 无短名 / 非 Windows）时**原样返回**输入，不阻断流程：
/// 8.3 只是让 L3 判定更严格的增强项，不应因展开失败而拒绝合法清理。
#[cfg(windows)]
pub fn long_path_name(path: &str) -> String {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetLongPathNameW;

    // 转 UTF-16（Windows 文件名 API 用 wide 字符串），末尾补 NUL。
    let mut wide: Vec<u16> = path.encode_utf16().collect();
    wide.push(0);

    // 第一次调用拿所需缓冲区长度（含结尾 NUL）；0 表示失败（路径不存在等）。
    let needed = unsafe { GetLongPathNameW(PCWSTR(wide.as_ptr()), None) };
    if needed == 0 {
        return path.to_owned();
    }

    // 分配足够空间再展开（windows crate 的第二个参数是 Option<&mut [u16]> 切片，
    // 切片长度即底层 cchbuffer；尾部多留一个 NUL 槽避免截断）。
    let mut buf: Vec<u16> = vec![0u16; needed as usize + 1];
    let written = unsafe { GetLongPathNameW(PCWSTR(wide.as_ptr()), Some(buf.as_mut_slice())) };
    if written == 0 {
        return path.to_owned();
    }
    String::from_utf16_lossy(&buf[..written as usize])
}

/// 非 Windows 占位（本项目 Windows-only，仅为编译通过）：不做任何转换。
#[cfg(not(windows))]
pub fn long_path_name(path: &str) -> String {
    path.to_owned()
}

/// 判断**最终组件本身**是否为符号链接 / junction（reparse point）。
///
/// task-086 2.2：评审场景 `C:\Temp\link -> C:\Users\alice\Documents` 中
/// `link` 本身是 symlink，词法上落在 Temp（不在红线）。保守策略是**直接拒绝**
/// 删除这种重定向指针本身（code 2009「拒绝删除符号链接/Junction」）。
///
/// # 与 [`count_reparse_hops`] 的区别
///
/// - `count_reparse_hops` 统计整条路径里**所有**中间层 junction（如 `AppData\Local`
///   常是 junction），用于 hop 数告警；
/// - 本函数只关心**待删目标本身**是不是 symlink/junction——删除整个 link 指针
///   而非其指向目标。中间层 junction 是被正常穿越的，不应连坐。
///
/// 用 [`std::fs::symlink_metadata`]（不跟随最终符号链接）取原文件属性，
/// 检查 `FILE_ATTRIBUTE_REPARSE_POINT` 位。
#[cfg(windows)]
pub fn is_symlink_or_junction(path: &str) -> bool {
    use std::os::windows::fs::MetadataExt;
    std::fs::symlink_metadata(path)
        .map(|md| md.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
        .unwrap_or(false)
}

/// 非 Windows 占位。
#[cfg(not(windows))]
pub fn is_symlink_or_junction(_path: &str) -> bool {
    false
}

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
///
/// # task-086 5.4：UNC 不再一律拒绝
///
/// 用户自定义规则可以指向网络共享（L2，需确认）。`canonicalize` 对 UNC 实际可用，
/// 返回 `\\?\UNC\server\share\...`；这里统一去前缀，让上层拿到的 `real` 与
/// 用户规则里的 `\\server\share\...` 形态一致（否则字符串前缀匹配永远不中）。
pub fn resolve_to_real(input: &NormalizedPath) -> Result<ResolvedPath, ResolveError> {
    let raw = input.normalized();

    let canonical = std::fs::canonicalize(raw).map_err(|e| {
        // 区分「不存在」与其他 IO 错误，便于上层填 audit。
        if e.kind() == std::io::ErrorKind::NotFound {
            ResolveError::NotExists(raw.to_string())
        } else {
            ResolveError::Io(e)
        }
    })?;

    // canonicalize 在 Windows 返回 `\\?\C:\...` / `\\?\UNC\server\share\...`。
    // 两种前缀都要去掉，才能与 classify / 规则库的字符串比较对齐。
    let canonical_str = canonical.to_string_lossy();
    let real = match canonical_str.strip_prefix(r"\\?\UNC\") {
        Some(rest) => format!(r"\\{rest}"),
        None => canonical_str.trim_start_matches(r"\\?\").to_string(),
    };

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

    #[cfg(windows)]
    #[test]
    fn long_path_name_roundtrips_short_to_long() {
        use std::os::windows::ffi::OsStrExt;
        use windows::core::PCWSTR;
        use windows::Win32::Storage::FileSystem::GetShortPathNameW;

        // 造一个真实存在的唯一目录，用它自己的 8.3 短名做还原测试。
        let dir = std::env::temp_dir().join(format!("trae_longpath_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let short = {
            let mut wide: Vec<u16> = dir.as_os_str().encode_wide().collect();
            wide.push(0);
            let needed = unsafe { GetShortPathNameW(PCWSTR(wide.as_ptr()), None) };
            if needed == 0 {
                return; // 系统未启用 8.3 别名时短名=长名，往返仍成立
            }
            let mut buf = vec![0u16; needed as usize + 1];
            let written =
                unsafe { GetShortPathNameW(PCWSTR(wide.as_ptr()), Some(buf.as_mut_slice())) };
            String::from_utf16_lossy(&buf[..written as usize])
        };

        // 短名 → 长名展开：应还原为原来的长目录路径（大小写不敏感）。
        // 不能拿 `dir` 当期望值：CI runner 的 `temp_dir()` 本身可能就是短名
        // （`C:\Users\RUNNER~1\...`），展开后反而与它不等。用 canonicalize
        // 取真实长路径再比较。
        let long = long_path_name(&short);
        let expected = std::fs::canonicalize(&dir)
            .unwrap()
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .trim_end_matches('\\')
            .to_lowercase();
        assert_eq!(long.trim_end_matches('\\').to_lowercase(), expected);
    }

    #[cfg(windows)]
    #[test]
    fn long_path_name_identity_when_no_short_component() {
        // 对不含 8.3 段的长路径，展开幂等（结果与原路径一致，大小写不敏感）。
        let out = long_path_name(r"C:\Windows\System32");
        assert_eq!(out.to_lowercase(), r"c:\windows\system32".to_lowercase());
    }

    #[cfg(windows)]
    #[test]
    fn is_symlink_or_junction_flags_link_but_not_plain_dir() {
        use std::os::windows::fs::symlink_dir;

        let base = std::env::temp_dir().join(format!("trae_link_{}", std::process::id()));
        let target = base.join("real");
        let link = base.join("alias");
        std::fs::create_dir_all(&target).unwrap();
        // 普通目录不是 symlink。
        assert!(!is_symlink_or_junction(&target.to_string_lossy()));

        // 创建链接（Windows 上对目录 symlink_dir 即 soft link，同为 reparse point）。
        if let Err(e) = symlink_dir(&target, &link) {
            // 无开发者模式/权限时创建失败 → 跳过，避免 CI 误报。
            eprintln!("skipping symlink detection fixture: {e}");
            return;
        }
        assert!(is_symlink_or_junction(&link.to_string_lossy()));
        // 删除时链接本身被拒，但真实目录不受影响。
        let _ = std::fs::remove_dir_all(&base);
    }
}
