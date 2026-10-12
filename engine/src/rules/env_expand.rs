//! 路径变量白名单展开（task-005 §D）。
//!
//! 仅放行 5 个环境变量 + `~` 前缀，其余一律拒绝：
//! - `%LOCALAPPDATA%` / `%APPDATA%` / `%TEMP%` / `%PROGRAMDATA%` / `%USERPROFILE%`（大小写不敏感）；
//! - `~` 仅作为路径开头，且后必须跟 `\` / `/` 或结尾，展开为 `%USERPROFILE%`；
//! - 正斜杠统一归一为反斜杠；
//! - **不做** `..` 规范化（那是 task-003 `normalize` 的职责）。

/// 白名单环境变量名（大写归一后比较）。
const WHITELIST_VARS: [&str; 5] = [
    "LOCALAPPDATA",
    "APPDATA",
    "TEMP",
    "PROGRAMDATA",
    "USERPROFILE",
];

/// 路径变量展开失败原因。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ExpandError {
    /// 出现了不在白名单内的环境变量（如 `%WINDIR%`），携带变量名（不含百分号）。
    #[error("不允许的环境变量: %{0}%")]
    UnknownVar(String),
    /// 白名单内的环境变量在当前系统未设置。
    #[error("环境变量未设置: %{0}%")]
    VarUnset(String),
    /// `~` 后未跟 `\` / `/` 或结尾（如 `~foo`），拒绝按主目录处理。
    #[error("波浪号只能作为主目录前缀: {0:?}")]
    TildeMasquerade(String),
}

/// 展开单条路径字符串中的白名单变量。
///
/// ```ignore
/// assert_eq!(expand("%LOCALAPPDATA%\\npm-cache")?, r"C:\Users\x\AppData\Local\npm-cache");
/// ```
pub fn expand(input: &str) -> Result<String, ExpandError> {
    // 1. `~` 前缀处理：必须紧跟分隔符或结束，否则按伪装拒绝。
    let body = if let Some(rest) = input.strip_prefix('~') {
        if !rest.is_empty() && !rest.starts_with('\\') && !rest.starts_with('/') {
            return Err(ExpandError::TildeMasquerade(input.to_string()));
        }
        let home = std::env::var("USERPROFILE").map_err(|_| ExpandError::VarUnset("USERPROFILE".into()))?;
        // 先把 `~` 整体替换为 home（后续 %var% 扫描再处理其余部分）。
        format!("{home}{rest}")
    } else {
        input.to_string()
    };

    // 2. 逐个替换 %VAR%。
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        // 读 %VAR% 直到下一个 %。
        let mut name = String::new();
        let mut closed = false;
        while let Some(nc) = chars.next() {
            if nc == '%' {
                closed = true;
                break;
            }
            name.push(nc);
        }
        if !closed {
            // 未闭合的 % 当作普通字符原样保留。
            out.push('%');
            out.push_str(&name);
            continue;
        }
        let upper = name.to_uppercase();
        if !WHITELIST_VARS.contains(&upper.as_str()) {
            return Err(ExpandError::UnknownVar(name));
        }
        let value = std::env::var(&upper).map_err(|_| ExpandError::VarUnset(upper))?;
        out.push_str(&value);
    }

    // 3. 正斜杠归一为反斜杠。
    Ok(out.replace('/', "\\"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_var_rejected() {
        assert_eq!(expand(r"%WINDIR%\Temp").unwrap_err(), ExpandError::UnknownVar("WINDIR".into()));
        assert_eq!(expand(r"%ProgramFiles%\App").unwrap_err(), ExpandError::UnknownVar("ProgramFiles".into()));
    }

    #[test]
    fn tilde_masquerade_rejected() {
        assert_eq!(expand("~foo").unwrap_err(), ExpandError::TildeMasquerade("~foo".into()));
    }

    #[test]
    fn forward_slash_normalized() {
        let r = expand("~/.npm/_cacache").unwrap();
        assert!(r.starts_with(&std::env::var("USERPROFILE").unwrap()));
        assert!(!r.contains('/'));
        assert!(r.ends_with("\\.npm\\_cacache"));
    }
}
