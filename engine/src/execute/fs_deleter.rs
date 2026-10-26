//! 真实文件删除器（task-015）：实现 [`crate::execute::delete::Deleter`]。
//!
//! 与 NoopDeleter 区别：直接调 `std::fs::remove_file` 碰磁盘。
//! 错误语义：
//! - 文件不存在（NotFound）→ `Ok(false)`，幂等（已删过就算成功，但**没删掉东西**）；
//! - 权限不足 / 文件占用 → Err(结构化文案)；
//! - 其他 IO 错误 → Err。
//!
//! task-052 G5：返回值从 `Result<(), String>` 改为 `Result<bool, String>`，
//! `false` 表示「删除前文件就已不存在」，供 `executor` 单独统计 `already_gone`。

use std::fs;
use std::io;

use super::delete::Deleter;
use super::normalize::NormalizedPath;

/// 真实文件删除器：调 `std::fs::remove_file`。
pub struct FsDeleter;

impl Deleter for FsDeleter {
    fn delete_file(&self, p: &NormalizedPath) -> Result<bool, String> {
        let path = p.normalized();
        // 先取大小（bytes_freed 写 DB 用）；不存在则按幂等处理并告知调用方「没删到东西」。
        let meta = match fs::metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(format!("stat {path}: {e}")),
        };
        if meta.is_dir() {
            return Err(format!("{path} is a directory, use delete_dir"));
        }
        match fs::remove_file(path) {
            Ok(()) => Ok(true),
            // 极端竞态：stat 之后被别人删了。对用户而言仍是「已不存在」。
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(format!("remove_file {path}: {e}")),
        }
    }

    fn delete_dir(&self, p: &NormalizedPath) -> Result<(), String> {
        // task-015 明确只删文件；目录删除留后续。
        Err(format!(
            "delete_dir not implemented for {} (task-015 only deletes files)",
            p.normalized()
        ))
    }
}
