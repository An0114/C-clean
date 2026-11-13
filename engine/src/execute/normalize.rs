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

/// 把任意字符串规范化为 Windows 绝对路径（盘符或 UNC）。
///
/// 处理：正斜杠归一为反斜杠、剥离 `\\?\` / `\\?\UNC\` 扩展前缀、大写盘符、
/// 词法折叠 `.` / `..`；`..` 弹出根即拒绝。不解析符号链接、不触碰文件系统。
///
/// # task-086 5.4：UNC 不再一律拒绝
///
/// 此前 `\\server\share\...` 直接返回 [`NormalizeError::NotAbsolute`]，导致
/// 「网络盘用户规则」建得进去却永远扫不出条目。现在按「server + share + 至少
/// 一级目录」接受 UNC，并在 [`NormalizedPath::lower_components`] 里剥掉
/// `\\server\share` 前缀，让内置红线（个人目录 / 浏览器敏感文件）照常生效。
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
    // 剥离 Win32 扩展前缀：`\\?\C:\...` → `C:\...`；`\\?\UNC\server\share` → `\\server\share`。
    let s = if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else {
        match s.strip_prefix(r"\\?\") {
            Some(rest) => rest.to_string(),
            None => s,
        }
    };

    if s.starts_with(r"\\") {
        return normalize_unc(raw, &s);
    }

    // 必须形如 `X:\...`：字母盘符 + 冒号 + 反斜杠。
    let bytes = s.as_bytes();
    if bytes.len() < 3 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' || bytes[2] != b'\\' {
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

/// UNC 分支：`\\server\share[\dir...]`。
///
/// 要求 server 与 share 非空；路径本身允许只有 share（`\\server\share`），
/// 但**删除/规则校验**侧会要求至少多一级目录（见 `user_rules::static_root`）。
fn normalize_unc(raw: &str, s: &str) -> Result<NormalizedPath, NormalizeError> {
    let parts: Vec<&str> = s.trim_start_matches('\\').split('\\').collect();
    if parts.len() < 2 || parts[0].trim().is_empty() || parts[1].trim().is_empty() {
        return Err(NormalizeError::NotAbsolute);
    }
    let mut stack: Vec<&str> = Vec::new();
    for comp in &parts[2..] {
        match *comp {
            "" | "." => continue,
            ".." => return Err(NormalizeError::TraversalEscapes),
            c => stack.push(c),
        }
    }

    let mut normalized = format!(r"\\{}\{}", parts[0], parts[1]);
    if !stack.is_empty() {
        normalized.push('\\');
        normalized.push_str(&stack.join("\\"));
    }
    Ok(NormalizedPath {
        raw: raw.to_string(),
        normalized,
    })
}

impl NormalizedPath {
    /// 是否为 UNC 网络路径（`\\server\share\...`）。
    pub fn is_unc(&self) -> bool {
        self.normalized.starts_with(r"\\")
    }

    /// 大小写不敏感的组件列表（供白名单匹配器使用）：剥掉根前缀后按 `\` 切分。
    ///
    /// - 盘符路径：跳过 `C:`；
    /// - UNC：跳过空的头两段与 `server` / `share`，让组件序列与盘符路径同构
    ///   （`\\srv\share\Users\alice\Desktop` → `["users","alice","desktop"]`），
    ///   内置红线的「users/<用户名>/<个人目录>」判定无需为 UNC 写第二套。
    pub fn lower_components(&self) -> Vec<String> {
        let skip = if self.is_unc() { 4 } else { 1 };
        self.normalized
            .split('\\')
            .skip(skip)
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

    // ===== task-086 5.4：UNC =====

    #[test]
    fn accepts_unc_and_builds_components_without_server_share() {
        let p = normalize(r"\\nas\backup\Games\cache\a.bin").unwrap();
        assert!(p.is_unc());
        assert_eq!(p.normalized(), r"\\nas\backup\Games\cache\a.bin");
        assert_eq!(
            p.lower_components(),
            vec![
                "games".to_string(),
                "cache".to_string(),
                "a.bin".to_string()
            ],
            "组件序列应剥掉 server/share，与盘符路径同构"
        );
    }

    #[test]
    fn normalizes_unc_verbatim_prefix() {
        let p = normalize(r"\\?\UNC\nas\backup\dir\a.bin").unwrap();
        assert_eq!(p.normalized(), r"\\nas\backup\dir\a.bin");
    }

    #[test]
    fn unc_rejects_traversal_and_incomplete_share() {
        assert_eq!(
            normalize(r"\\nas\backup\..\secret"),
            Err(NormalizeError::TraversalEscapes)
        );
        assert_eq!(normalize(r"\\nas"), Err(NormalizeError::NotAbsolute));
        assert_eq!(normalize(r"\\nas\\dir"), Err(NormalizeError::NotAbsolute));
    }

    #[test]
    fn unc_personal_dir_components_match_drive_shape() {
        // \\server\share\Users\alice\Desktop → components ["users","alice","desktop"]
        let p = normalize(r"\\nas\backup\Users\alice\Desktop\a.docx").unwrap();
        assert_eq!(
            p.lower_components(),
            vec![
                "users".to_string(),
                "alice".to_string(),
                "desktop".to_string(),
                "a.docx".to_string()
            ]
        );
    }
}
