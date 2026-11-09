//! task-084 A3：文件夹分布统计（**只读**）。
//!
//! 大文件地图（task-066）只列单个大文件，用户想知道「哪个文件夹占最多」没法一眼看出。
//! 本模块对用户目录做**文件夹大小的递归统计**，输出 Top20（按子树总大小倒序）。
//!
//! # 实现要点
//!
//! - **顶层一次遍历 + 后序累加**，不是对每个文件夹各扫一遍（那会退化成 O(n²)）：
//!   先 BFS 收集所有目录并记录各自「直属文件大小」，再按「子目录先于父目录」的顺序
//!   做一次累加，得到每个目录的**子树总大小**；
//! - **reparse point 不进**：junction / symlink 会让递归无限转并重复计数；
//! - 任何 `read_dir` / `metadata` 错误**跳过继续**（用户目录常有 ACL 拒绝的子树）；
//! - 输出**确定**：按大小降序，同大小按路径字典序——单测能稳定断言。
//!
//! # 与任务书「walkdir + rayon」的偏离
//!
//! 任务书写「用 walkdir 并行递归（rayon）」。本仓库不依赖 walkdir / rayon，且并行
//! 缓存盘遍历对结果顺序不友好（不利于确定性测试）。这里用现有代码风格的显式栈单线程
//! 遍历，语义一致、零新增依赖、输出确定。性能上用户目录一次顺序遍历完全可接受。

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

/// 输出 Top N 个文件夹（默认 20）。
pub const FOLDER_TOP_N: usize = 20;

/// 统计的用户目录（相对 `%USERPROFILE%`）。
pub const FOLDER_SCAN_DIRS: [&str; 4] = ["Downloads", "Documents", "Desktop", "AppData"];

/// 一个文件夹的分布条目。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderStat {
    /// 文件夹绝对路径。
    pub path: String,
    /// 该文件夹**子树总大小**（含全部后代文件）。
    pub size_bytes: u64,
    /// 该文件夹直属文件个数（不含后代的子目录文件，仅用于呈现粗粒度）。
    pub file_count: u64,
    /// 所属类目（Downloads / Documents / Desktop / AppData）。
    pub category: String,
    /// 盘符（task-084 B3 多盘扩展；C 盘留空保持旧语义）。大文件地图跨盘后按此分组。
    pub drive_letter: String,
}

/// 默认入口：扫 `%USERPROFILE%` 下四个用户目录。
pub fn folder_stats() -> Vec<FolderStat> {
    folder_stats_in(&default_profile_dirs())
}

fn default_profile_dirs() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let Ok(profile) = std::env::var("USERPROFILE") else {
        return out;
    };
    for dir in FOLDER_SCAN_DIRS {
        let d = PathBuf::from(&profile).join(dir);
        if d.is_dir() {
            out.push(d);
        }
    }
    out
}

/// 对给定根目录集合统计文件夹分布（公开给单测在临时目录上直接验证）。
///
/// `category` 由根目录名派生（Downloads/Documents/Desktop/AppData 之外的根取目录名）。
pub fn folder_stats_in(roots: &[PathBuf]) -> Vec<FolderStat> {
    // BFS：收集所有目录 + 每目录「直属文件大小/个数」。
    // `order` 保持 BFS 顺序（父目录先于子目录）。
    let mut order: Vec<(PathBuf, String)> = Vec::new(); // (dir, category)
    let mut own_size: HashMap<PathBuf, u64> = HashMap::new();
    let mut own_files: HashMap<PathBuf, u64> = HashMap::new();

    let mut queue: VecDeque<(PathBuf, String)> = VecDeque::new();
    for root in roots {
        let cat = root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "其他".to_string());
        queue.push_back((root.clone(), cat));
    }

    while let Some((dir, cat)) = queue.pop_front() {
        order.push((dir.clone(), cat.clone()));
        let mut direct = 0u64;
        let mut files = 0u64;
        let Ok(entries) = std::fs::read_dir(&dir) else {
            // 目录不可读：直属计 0，仍登记进 order（它自己也是一个文件夹条目）。
            own_size.insert(dir.clone(), 0);
            own_files.insert(dir.clone(), 0);
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(md) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            // reparse point 不进
            if is_reparse(&md) {
                continue;
            }
            if md.is_dir() {
                queue.push_back((path, cat.clone()));
            } else if md.is_file() {
                direct += md.len();
                files += 1;
            }
        }
        own_size.insert(dir.clone(), direct);
        own_files.insert(dir.clone(), files);
    }

    // 后序累加：BFS 反序 = 子目录先于父目录。子树大小 = 直属 + 所有直接子目录的子树。
    let mut subtree: HashMap<PathBuf, u64> = HashMap::new();
    for (dir, _cat) in order.iter().rev() {
        let mut sum = own_size.get(dir).copied().unwrap_or(0);
        if let Some(child) = subtree.remove(dir) {
            sum += child;
        }
        subtree.insert(dir.clone(), sum);
        if let Some(parent) = dir.parent() {
            if own_size.contains_key(parent) {
                *subtree.entry(parent.to_path_buf()).or_insert(0) += sum;
            }
        }
    }

    // 汇总输出：path / 子树大小 / 直属文件数 / category。
    let mut items: Vec<FolderStat> = order
        .into_iter()
        .map(|(dir, cat)| {
            // dir 的子目录贡献已累加进 subtree[dir]；直属文件数仅该目录自身。
            let size = subtree.get(&dir).copied().unwrap_or(0);
            FolderStat {
                path: dir.to_string_lossy().to_string(),
                size_bytes: size,
                file_count: own_files.get(&dir).copied().unwrap_or(0),
                category: cat,
                drive_letter: drive_letter_of(&dir),
            }
        })
        .collect();

    // 按大小降序；同大小按路径升序（确定性）。
    items.sort_by(|a, b| {
        b.size_bytes
            .cmp(&a.size_bytes)
            .then_with(|| a.path.cmp(&b.path))
    });
    items.truncate(FOLDER_TOP_N);
    items
}

fn drive_letter_of(dir: &Path) -> String {
    let s = dir.to_string_lossy();
    // 形如 `C:\Users\...` → 取首字符；非盘符路径返回空。
    let bytes = s.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' {
        (bytes[0] as char).to_string()
    } else {
        String::new()
    }
}

fn is_reparse(md: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if md.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    md.file_type().is_symlink()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    /// 造一棵确定性的目录树：
    ///   root/
    ///     big/      内含 3 个文件共 300B
    ///     medium/   内含 1 个文件 100B  + 子目录 nested/（1 个文件 50B → 子树 150B）
    ///     tiny/     空
    fn build_tree(root: &Path) {
        let big = root.join("big");
        let medium = root.join("medium");
        let nested = medium.join("nested");
        let tiny = root.join("tiny");
        for d in [&big, &medium, &nested, &tiny] {
            fs::create_dir_all(d).expect("create dir");
        }
        for (i, name) in ["a", "b", "c"].iter().enumerate() {
            let size = 100u64 * (i as u64 + 1);
            fs::write(big.join(name), vec![0u8; size as usize]).expect("write big file");
        }
        fs::write(medium.join("f"), vec![0u8; 100]).expect("write medium file");
        fs::write(nested.join("n"), vec![0u8; 50]).expect("write nested file");
    }

    /// A3 验收：构造目录树，断言 Top 排序 ≥2 条、大文件夹排在前面。
    #[test]
    fn folder_stats_orders_by_subtree_size_desc() {
        let dir = tempfile::tempdir().expect("tempdir");
        build_tree(dir.path());

        let stats = folder_stats_in(&[dir.path().to_path_buf()]);
        // 目录树：root / big / medium / nested / tiny → 5 个目录条目
        assert!(stats.len() >= 2, "应至少输出 2 个文件夹条目");

        // 文件夹「子树」大小：内含文件全部记入。
        let big = stats
            .iter()
            .find(|s| s.path.ends_with("\\big"))
            .expect("big exists");
        let medium = stats
            .iter()
            .find(|s| s.path.ends_with("\\medium"))
            .expect("medium exists");
        // big 子树 600B（100+200+300）> medium 子树 150B
        assert_eq!(big.size_bytes, 600);
        assert_eq!(medium.size_bytes, 150);
        // 大文件夹应排在前面（降序）
        let big_idx = stats
            .iter()
            .position(|s| s.path.ends_with("\\big"))
            .unwrap();
        let medium_idx = stats
            .iter()
            .position(|s| s.path.ends_with("\\medium"))
            .unwrap();
        assert!(big_idx < medium_idx, "big 应排在 medium 之前：{stats:?}");
    }

    /// A3 补充：空目录应计入条目（size 0），且 file_count 为直属文件数。
    #[test]
    fn empty_dir_appears_with_zero_size() {
        let dir = tempfile::tempdir().expect("tempdir");
        let empty = dir.path().join("empty_dir");
        fs::create_dir_all(&empty).expect("create empty");

        // 再放一个有文件的兄弟目录，保证 empty 也被遍历到。
        fs::write(dir.path().join("f.txt"), b"xyz").expect("write file");

        let stats = folder_stats_in(&[dir.path().to_path_buf()]);
        let empty_stat = stats
            .iter()
            .find(|s| s.path.ends_with("empty_dir"))
            .expect("empty_dir 应出现在结果里");
        assert_eq!(empty_stat.size_bytes, 0, "空目录大小为 0");
        assert_eq!(empty_stat.file_count, 0, "空目录直属文件数为 0");
    }
}
