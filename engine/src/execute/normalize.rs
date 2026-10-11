//! Windows 绝对路径规范化器（词法层面，不解析符号链接）。
//!
//! 职责：把任意字符串输入统一成「反斜杠、大写盘符、无尾部、无 `..` / `.`」
//! 的绝对路径 newtype [`NormalizedPath`]，并在规范化阶段拒绝穿越、
//! 相对路径、空输入与 NUL。本模块**不**做文件系统 IO（符号链接、存在性
//! 留待后续 platform 模块）。

/// 规范化失败原因（与业务拒绝 [`super::whitelist::DenialReason`] 区分：
/// 这里是「输入根本不可信」，由上层统一折算成 `DenialReason::PathTraversal`）。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum NormalizeError {
    /// 空串或纯空白。
    #[error("empty or blank input")]
    Empty,
    /// 含 NUL 字节（防注入）。
    #[error("contains NUL byte")]
    Nul,
    /// 非盘符绝对路径（未带 `X:\` 前缀）。
    #[error("not an absolute path: expected drive letter (e.g. C:\\)")]
    NotAbsolute,
    /// `..` 弹出盘符根之外（穿越逃逸）。
    #[error("path traversal escapes drive root")]
    TraversalEscapes,
}

/// 规范化后的绝对路径 newtype：反斜杠、大写盘符、无尾部、无 `..` / `.`。
///
/// 内部同时保留「原始输入」与「规范化结果」，供审计日志记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedPath {
    raw: String,
    normalized: String,
}

impl NormalizedPath {
    /// 原始输入（审计用，逐字保留）。
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// 规范化结果（内部比较、白名单匹配用）。
    pub fn normalized(&self) -> &str {
        &self.normalized
    }
}

/// 把任意字符串规范化为 Windows 盘符绝对路径。
///
/// 处理：正斜杠归一为反斜杠、剥离 `\\?\` 扩展前缀、大写盘符、
/// 词法折叠 `.` / `..`；`..` 弹出盘符根即拒绝。
/// 不解析符号链接、不触碰文件系统。
pub fn normalize(raw: &str) -> Result<NormalizedPath, NormalizeError> {
    let input = raw.trim();
    if input.is_empty() {
        return Err(NormalizeError::Empty);
    }
    if input.contains('\0') {
        return Err(NormalizeError::Nul);
    }

    // 斜杠归一：Windows 同时接受 / 与 \，统一为 \。
    let s = input.replace('/', "\\");
    // 剥离 Win32 扩展前缀 `\\?\`（任务文档示例输入之一）。
    let s = match s.strip_prefix(r"\\?\") {
        Some(rest) => rest,
        None => &s,
    };

    // 必须形如 `X:\...`：字母盘符 + 冒号 + 反斜杠。
    let bytes = s.as_bytes();
    if bytes.len() < 3 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' || bytes[2] != b'\\' {
        // UNC（\\server\share）等无盘符绝对路径：本任务保守拒绝，
        // 留待 platform 任务按网络盘策略再开放。
        return Err(NormalizeError::NotAbsolute);
    }
    let drive = (bytes[0] as char).to_ascii_uppercase();
    let rest = &s[3..];

    // 词法折叠组件：`.`/空段跳过；删除入口对 `..` 一律直接拒绝
    // （见任务文档 E-14：Windows\Temp\..\..\Users\... 必须在规范化阶段拒）。
    // 删除网关不需要任何「未解析相对上跳」——上游传入的都是已解析的绝对路径，
    // 见到 `..` 即视为可疑输入，零折叠、零逃逸空间。
    let mut stack: Vec<&str> = Vec::new();
    for comp in rest.split('\\') {
        match comp {
            "" | "." => continue,
            ".." => return Err(NormalizeError::TraversalEscapes),
            c => stack.push(c),
        }
    }

    let normalized = if stack.is_empty() {
        format!("{drive}:\\")
    } else {
        format!("{drive}:\\{}", stack.join("\\"))
    };

    Ok(NormalizedPath {
        raw: raw.to_string(),
        normalized,
    })
}

impl NormalizedPath {
    /// 大小写不敏感的组件列表（供白名单匹配器使用）：剥掉盘符根后按 `\` 切分。
    pub fn lower_components(&self) -> Vec<String> {
        self.normalized
            .split('\\')
            .skip(1) // 跳过 "C:" 段
            .map(|c| c.to_lowercase())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_slashes_and_uppercases_drive() {
        let p = normalize(r"C:/Windows/Temp/a.log").unwrap();
        assert_eq!(p.normalized(), r"C:\Windows\Temp\a.log");
    }

    #[test]
    fn strips_verbatim_prefix() {
        let p = normalize(r"\\?\C:\Windows\Temp\a.log").unwrap();
        assert_eq!(p.normalized(), r"C:\Windows\Temp\a.log");
    }

    #[test]
    fn accepts_dot_segments_but_rejects_any_dotdot() {
        let p = normalize(r"C:\Windows\.\Temp\a.log").unwrap();
        assert_eq!(p.normalized(), r"C:\Windows\Temp\a.log");
        // 删除网关见到任何 `..` 一律拒（不折叠）
        assert_eq!(
            normalize(r"C:\Windows\Temp\..\System32\x.dll"),
            Err(NormalizeError::TraversalEscapes)
        );
    }

    #[test]
    fn rejects_traversal_escaping_root() {
        assert_eq!(
            normalize(r"C:\Windows\Temp\..\..\Users\alice\Documents\bank.pdf"),
            Err(NormalizeError::TraversalEscapes)
        );
    }

    #[test]
    fn rejects_relative_and_blank_and_nul() {
        assert_eq!(normalize(r"Windows\Temp"), Err(NormalizeError::NotAbsolute));
        assert_eq!(normalize("   "), Err(NormalizeError::Empty));
        assert_eq!(normalize("C:\\\0x"), Err(NormalizeError::Nul));
    }
}
