//! 单行 NDJSON 解析：对齐 `scripts/PROTOCOL.md`（task-011）。
//!
//! 行协议（UTF-8，每行一个 JSON 对象）：
//!
//! ```json
//! {"type":"item","module_code":"temp","category":"user_temp",
//!  "path":"C:\\Users\\...\\a.tmp","size_bytes":1234,"risk_level":1}
//! {"type":"done","module_code":"temp","total_items":154,"total_bytes":16541385}
//! ```
//!
//! 用 `#[serde(tag = "type")]` 把 `type` 字段当判别式；
//! `deny_unknown_fields` 保证 ps1 端多塞字段时立刻报错而不是静默丢弃。

use serde::Deserialize;

/// 一行 NDJSON 事件：item（扫描到的文件）或 done（脚本结束）。
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum NdjsonEvent {
    /// 扫描到的一个可清理文件。
    #[serde(rename = "item")]
    Item {
        /// 模块 code，对应 ps1 脚本的 `$moduleCode`（temp / windows_update / ...）。
        module_code: String,
        /// 规则库类别键（user_temp / windows_temp / ...）。
        category: String,
        /// 文件绝对路径。
        path: String,
        /// 文件大小（字节）。
        size_bytes: i64,
        /// 风险等级：0/1/2 → L0/L1/L2（L3 由 ps1 端不输出，对齐红线约定）。
        risk_level: i64,
    },
    /// 脚本正常结束事件。
    #[serde(rename = "done")]
    Done {
        /// 结束的模块 code（应与 item 行一致）。
        module_code: String,
        /// 本次扫描累计 item 条数（用于对账）。
        total_items: i64,
        /// 本次扫描累计字节数。
        total_bytes: i64,
    },
}

/// 单行解析失败的错误。
#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    /// JSON 语法 / 字段缺失 / 多余字段 / 类型不对。
    #[error("JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),
}

/// 把一行 stdout 文本解析成 [`NdjsonEvent`]。
///
/// ps1 端用 `ConvertTo-Json -Compress` 输出，行尾是 `\r\n`；调用方负责
/// 按行切分并 trim `\r`，本函数只吃一行内的字符。
pub fn parse_line(line: &str) -> Result<NdjsonEvent, ParseError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        // ps1 偶尔会在末尾吐一个空行；用 serde_json 的 io 错误表达「非 JSON」。
        return Err(ParseError::Json(serde_json::Error::io(
            std::io::Error::new(std::io::ErrorKind::InvalidData, "empty line"),
        )));
    }
    Ok(serde_json::from_str(trimmed)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_item_line_ok() {
        let line = r#"{"type":"item","module_code":"temp","category":"user_temp","path":"C:\\a.tmp","size_bytes":100,"risk_level":1}"#;
        match parse_line(line).unwrap() {
            NdjsonEvent::Item {
                module_code,
                category,
                path,
                size_bytes,
                risk_level,
            } => {
                assert_eq!(module_code, "temp");
                assert_eq!(category, "user_temp");
                assert_eq!(path, "C:\\a.tmp");
                assert_eq!(size_bytes, 100);
                assert_eq!(risk_level, 1);
            }
            other => panic!("expected Item, got {other:?}"),
        }
    }

    #[test]
    fn parse_done_line_ok() {
        let line = r#"{"type":"done","module_code":"temp","total_items":2,"total_bytes":200}"#;
        match parse_line(line).unwrap() {
            NdjsonEvent::Done {
                module_code,
                total_items,
                total_bytes,
            } => {
                assert_eq!(module_code, "temp");
                assert_eq!(total_items, 2);
                assert_eq!(total_bytes, 200);
            }
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn parse_missing_field_err() {
        // 缺 path 字段。
        let line = r#"{"type":"item","module_code":"temp","category":"user_temp","size_bytes":100,"risk_level":1}"#;
        assert!(parse_line(line).is_err());
    }

    #[test]
    fn parse_unknown_field_err() {
        // 多了未知字段（deny_unknown_fields）。
        let line = r#"{"type":"item","module_code":"temp","category":"user_temp","path":"C:\\a.tmp","size_bytes":100,"risk_level":1,"extra":"x"}"#;
        assert!(parse_line(line).is_err());
    }

    #[test]
    fn parse_bad_json_err() {
        assert!(parse_line("not json").is_err());
        assert!(parse_line("").is_err());
    }
}
