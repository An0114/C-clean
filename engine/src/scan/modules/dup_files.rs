//! task-084 A2：重复文件检测（**只读，一个字节都不删**）。
//!
//! # 思路
//!
//! 全盘对每个文件算哈希是灾难（几百万文件 × 读盘）。这里用两步缩小范围：
//!
//! 1. **按大小分组**：先只读 `metadata.len()`，把 >[`DUP_MIN_SIZE_BYTES`] 的文件按大小
//!    归桶；只有「同大小 + 桶里 ≥2 个」才可能重复（不同大小内容必不同）；
//! 2. **只对可能重复的算 SHA-256**：同大小桶里若有 ≥2 个，才逐个读内容算哈希，
//!    同哈希（内容真相同）判为重复组。
//!
//! 这一步把「对每个文件读盘」降到「对同大小多个文件的少数桶读盘」，小文件洪水
//! （几 KB 的配置、图标）完全不参与，性能可控。
//!
//! # 保守策略
//!
//! 重复组内**保留最后修改时间最新的一份**（`modified_ms` 最大 → `is_keep = true`），
//! 其余标记可清理。**不自动删**——只报告，决定权留给用户。
//!
//! # 单测怎么测
//!
//! 公开一个接受 `roots` 的内部函数；对临时目录构造两个内容相同的文件，断言
//! 识别出一组重复且恰有一份 `is_keep`。默认 `scan_duplicates()` 用真实用户目录。

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

/// 只对大于等于该字节数的文件做重复检测（默认 10 MiB）。
///
/// 为什么不是默认所有文件：潜在重复只可能在「同大小」之间找；但把所有文件都算一遍
/// 大小并归桶本身就要扫全目录。把门槛抬高到 10MB，小文件直接不参与，扫描量级可控。
pub const DUP_MIN_SIZE_BYTES: u64 = 10 * 1024 * 1024;

/// 重复检测扫的用户目录（相对 `%USERPROFILE%`）。
pub const DUP_SCAN_DIRS: [&str; 3] = ["Downloads", "Documents", "Desktop"];

/// 一个重复组里的单个文件。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DupFile {
    /// 文件绝对路径。
    pub path: String,
    /// 字节数。同组文件内容相同，大小必然相等。
    pub size_bytes: u64,
    /// 最后修改时间（epoch 毫秒）。（保留最新修改时间的那份）
    pub modified_ms: i64,
    /// 建议保留（同组内恰一份 `true`）。
    pub is_keep: bool,
}

/// 一个重复分组：内容（SHA-256）相同的一组文件。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DupGroup {
    /// 内容哈希（SHA-256 十六进制前 16 位，足够区分）。
    pub group_id: String,
    /// 单个文件字节数（每组至少 2 个，**可释放 = 组大小 × (n-1)** 由前端算）。
    pub size_bytes: u64,
    /// 组内文件（按路径字典序，保证确定性输出）。
    pub files: Vec<DupFile>,
}

/// 默认扫描入口：扫 `%USERPROFILE%` 下的 Downloads/Documents/Desktop。
pub fn scan_duplicates() -> Vec<DupGroup> {
    scan_duplicates_in(&default_profile_dirs())
}

/// 相对 `%USERPROFILE%` 拼出三个用户目录，只保留确实存在的。
fn default_profile_dirs() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let Ok(profile) = std::env::var("USERPROFILE") else {
        return out;
    };
    for dir in DUP_SCAN_DIRS {
        let d = PathBuf::from(&profile).join(dir);
        if d.is_dir() {
            out.push(d);
        }
    }
    out
}

/// 对给定根目录集合做重复检测（公开给单测在临时目录上直接验证）。
pub fn scan_duplicates_in(roots: &[PathBuf]) -> Vec<DupGroup> {
    // 第一步：同大小归桶。
    let mut by_size: HashMap<u64, Vec<(PathBuf, i64)>> = HashMap::new();
    for root in roots {
        let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let Ok(md) = fs::symlink_metadata(&path) else {
                    continue;
                };
                // reparse point 不进，避免 junction 绕圈与重复计数。
                if is_reparse(&md) {
                    continue;
                }
                if md.is_dir() {
                    stack.push(path);
                } else if md.is_file() && md.len() >= DUP_MIN_SIZE_BYTES {
                    let modified = modified_ms(&md);
                    by_size.entry(md.len()).or_default().push((path, modified));
                }
            }
        }
    }

    // 第二步：只对「同大小且 ≥2 个」的桶算哈希。
    let mut groups: Vec<DupGroup> = Vec::new();
    for (size, candidates) in by_size {
        if candidates.len() < 2 {
            continue;
        }
        // 同大小 → 同哈希归组
        let mut by_hash: HashMap<String, Vec<(PathBuf, i64)>> = HashMap::new();
        for (path, modified) in candidates {
            let Some(digest) = file_sha256(&path) else {
                continue;
            };
            by_hash.entry(digest).or_default().push((path, modified));
        }
        for (digest, files) in by_hash {
            if files.len() < 2 {
                continue;
            }
            // 保留最新修改时间那份
            let keep_idx = files
                .iter()
                .enumerate()
                .max_by_key(|(_, (_p, m))| *m)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let mut dup_files: Vec<DupFile> = files
                .into_iter()
                .enumerate()
                .map(|(i, (path, modified))| DupFile {
                    path: path.to_string_lossy().to_string(),
                    size_bytes: size,
                    modified_ms: modified,
                    is_keep: i == keep_idx,
                })
                .collect();
            dup_files.sort_by(|a, b| a.path.cmp(&b.path));
            groups.push(DupGroup {
                group_id: digest.chars().take(16).collect(),
                size_bytes: size,
                files: dup_files,
            });
        }
    }

    // 组按大小降序（大的重复组先展示），同大小按 group_id 保证确定。
    groups.sort_by(|a, b| {
        b.size_bytes
            .cmp(&a.size_bytes)
            .then_with(|| a.group_id.cmp(&b.group_id))
    });
    groups
}

/// 文件是否是 reparse point（symlink / junction）。
///
/// Windows 下 `FileType::is_symlink()` 对 junction 返回 **false**，必须额外查
/// 文件属性位 `FILE_ATTRIBUTE_REPARSE_POINT (0x400)`——否则 `C:\Users\All Users` 这类
/// 目录联接会被当成正常目录递归进去，既绕圈又重复计数。
fn is_reparse(md: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if md.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    md.file_type().is_symlink()
}

/// 取最后修改时间（epoch 毫秒）；拿不到就取当前时间戳兜底。
fn modified_ms(md: &fs::Metadata) -> i64 {
    md.modified()
        .map(|m| {
            m.duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0)
        })
        .unwrap_or_else(|_| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64
        })
}

/// 流式 SHA-256（分 1 MiB 块读，避免 10MB+ 文件一次性吃进内存）。
fn file_sha256(path: &Path) -> Option<String> {
    let mut f = File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    /// 规则：重复检测只认 >10MB 文件，测试文件也要越过门槛（否则被大小分组滤掉）。
    fn mk_big_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let p = dir.join(name);
        let mut f = File::create(&p).expect("create file");
        // 填到 DUP_MIN_SIZE_BYTES 以上（用内容块反复写，避免一次性占用大内存）。
        let mut written: u64 = 0;
        while written < DUP_MIN_SIZE_BYTES + 1024 {
            f.write_all(content).expect("write");
            written += content.len() as u64;
        }
        p
    }

    /// A2 验收：构造两个同内容文件，断言重复组识别 ≥2 条。
    #[test]
    fn detects_duplicate_group_with_two_identical_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let content = b"identical-content-block-for-sha256-dup-test-";
        let a = mk_big_file(dir.path(), "dup_a.bin", content);
        let b = mk_big_file(dir.path(), "dup_b.bin", content);

        let groups = scan_duplicates_in(&[dir.path().to_path_buf()]);
        let matches: Vec<&DupGroup> = groups
            .iter()
            .filter(|g| g.files.iter().any(|f| f.path == a.to_string_lossy()))
            .filter(|g| g.files.iter().any(|f| f.path == b.to_string_lossy()))
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "两个同内容文件应被归为同一个重复组，实际组数 {}",
            matches.len()
        );
        let group = matches[0];
        assert_eq!(group.size_bytes, a.metadata().unwrap().len());
        assert_eq!(group.files.len(), 2, "重复组应恰好含两个文件");
    }

    /// A2 验收补充：<10MB 的小文件不参与重复检测。
    #[test]
    fn small_files_are_not_checked_for_duplicates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let small = dir.path().join("small.txt");
        std::fs::write(&small, b"tiny").expect("write small");
        let groups = scan_duplicates_in(&[dir.path().to_path_buf()]);
        assert!(groups.is_empty(), "低于门槛的文件不应触发任何重复组");
    }

    /// A2 验收：同内容三份，保留最新修改时间的那份（is_keep 恰一份）。
    #[test]
    fn keeps_the_latest_modified_copy_in_group() {
        let dir = tempfile::tempdir().expect("tempdir");
        let content = b"same-content-keep-latest-";
        let dir_p = dir.path().to_path_buf();
        let old = mk_big_file(&dir_p, "old.bin", content);
        let mid = mk_big_file(&dir_p, "mid.bin", content);
        let new = mk_big_file(&dir_p, "new.bin", content);
        set_mtime(&old, now_ms() - 5000);
        set_mtime(&mid, now_ms() - 2000);
        set_mtime(&new, now_ms() - 10);

        let groups = scan_duplicates_in(&[dir.path().to_path_buf()]);
        let group = groups
            .iter()
            .find(|g| g.files.len() == 3)
            .expect("三份同内容应在一个重复组");
        let keepers: Vec<&DupFile> = group.files.iter().filter(|f| f.is_keep).collect();
        assert_eq!(keepers.len(), 1, "每组应恰有一份 is_keep");
        assert!(
            keepers[0].path.ends_with("new.bin"),
            "保留最新修改时间的 new.bin，实际 {}",
            keepers[0].path
        );
    }

    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }

    /// 用 File::set_times 直接改写文件「最后修改时间」（标准库能力，不引第三方）。
    fn set_mtime(path: &Path, ms: i64) {
        let sys = UNIX_EPOCH + Duration::from_millis(ms.max(0) as u64);
        if let Ok(f) = File::open(path) {
            let _ = f.set_times(fs::FileTimes::new().set_modified(sys));
        }
    }
}
