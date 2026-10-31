//! 大文件地图（E 模块，task-066）：**只读扫描，一个字节都不删**。
//!
//! # 为什么这一类只读
//!
//! 这四类东西的共同点是「大，但删法有讲究」：
//!
//! | 目标 | 为什么不能由本工具直接删 |
//! |---|---|
//! | `hiberfil.sys` | 系统休眠文件，被内核持有。正规做法是 `powercfg /h off`（会同时关掉快速启动），直接删会被系统立刻重建或报占用 |
//! | `Windows.old` | 回滚上一个大版本的唯一途径。正规入口是「磁盘清理 → 清理系统文件」，它有额外的安全校验 |
//! | 回收站 | 里面的东西可能是用户误删、正等着恢复的 |
//! | Top100 大文件 | 是用户自己的文件（虚拟机镜像、安装包、视频素材），**任何自动删除都是事故** |
//!
//! 所以本模块只做「量出来 + 给一条能照着做的建议」，把决定权留给用户。
//!
//! # 为什么 Top100 要跳过一堆目录
//!
//! 不跳的话 Top100 会被 `C:\Windows\WinSxS`、`C:\ProgramData\Package Cache`、
//! `C:\pagefile.sys` 这类**用户不该动的东西**占满，列出来等于误导。
//! 跳过策略见 [`should_skip_dir`] / [`should_skip_root_file`]。
//!
//! 另一个必须跳过的原因是**防循环**：目录 junction（如
//! `C:\Users\<u>\AppData\Local\Application Data` 指回自己）会让朴素递归无限转，
//! 所以遍历时对 reparse point 一律不进。

use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// 分类标识。前端按这些值选图标 / 分组，**改动要同步 `BigFiles.tsx`**。
pub const CATEGORY_HIBERFIL: &str = "hiberfil";
/// 分类标识：`C:\Windows.old`。
pub const CATEGORY_WINDOWS_OLD: &str = "windows_old";
/// 分类标识：回收站。
pub const CATEGORY_RECYCLE_BIN: &str = "recycle_bin";
/// 分类标识：Top100 大文件。
pub const CATEGORY_TOP_FILES: &str = "top_files";

/// Top 列表默认取多少条。
pub const TOP_N: usize = 100;
/// Top 列表的入围门槛（默认 100 MiB）。
///
/// 不设门槛的话，一个 20 万文件的用户目录会产出 20 万条候选，堆里全是几 KB 的
/// 噪音，既没意义又拖慢遍历。100 MiB 是「值得看一眼」的量级。
pub const DEFAULT_MIN_SIZE_BYTES: u64 = 100 * 1024 * 1024;
/// 遍历条目上限。撞到就停并标 `truncated`——宁可少列几条，也不能让一次点击
/// 变成「扫盘 10 分钟没反应」。
pub const DEFAULT_MAX_ENTRIES: u64 = 200_000;
/// 遍历时间预算（秒）。与条目上限双保险：条目少但都在慢盘上时，时间兜底。
pub const DEFAULT_BUDGET_SECS: u64 = 20;

/// 一个大文件条目（或一个分类的汇总条目）。
///
/// `serde(rename_all = "camelCase")`：前端 interface 用 camelCase，
/// 靠这条属性对齐（debt-cleanup-004 的教训：漏了就是一片 undefined）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BigFileItem {
    /// 文件或目录的完整路径。
    pub path: String,
    /// 占用字节数。
    pub size_bytes: u64,
    /// 分类标识，取 `CATEGORY_*` 常量之一。
    pub category: String,
    /// 处理建议（中文，**不含数字**——「可释放 X」由前端按实际大小拼，
    /// 后端写死数字会在不同机器上自相矛盾）。
    pub recommendation: String,
}

/// 一次扫描的完整结果。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BigFilesReport {
    /// 三个固定分类的汇总条目（hiberfil / Windows.old / 回收站），**恰好 3 条**。
    ///
    /// 为什么固定 3 条而不是「有就放、没有就不放」：前端是四张卡片固定布局，
    /// 缺一张会让界面看起来像加载失败。不存在的目标用 `size_bytes = 0` +
    /// 「不存在 / 未启用」的建议表示。
    pub categories: Vec<BigFileItem>,
    /// Top100 明细（按大小降序，已跳过系统目录）。
    pub top_files: Vec<BigFileItem>,
    /// 合计字节数 = 三个分类之和 + Top100 之和。
    ///
    /// 不会重复计算：Top100 遍历时跳过了 `hiberfil.sys` / `pagefile.sys` 等根目录
    /// 系统文件，也跳过了 `Windows.old` 与 `$Recycle.Bin`（见 [`should_skip_dir`]）。
    pub total_bytes: u64,
    /// 实际遍历了多少个文件条目（透明度：让用户知道「扫了多少」）。
    pub scanned_entries: u64,
    /// 是否因条目/时间上限提前停止。为 true 时 Top100 不是全盘真 Top100。
    pub truncated: bool,
    /// 本次扫描耗时（毫秒）。
    pub elapsed_ms: u64,
}

/// Top100 遍历的可调参数（抽出来是为了单测能把上限压到很小）。
#[derive(Debug, Clone, Copy)]
pub struct TopFilesOptions {
    /// 取前多少条。
    pub limit: usize,
    /// 入围门槛（字节）。
    pub min_size_bytes: u64,
    /// 最多遍历多少个条目。
    pub max_entries: u64,
    /// 时间预算。
    pub budget: Duration,
}

impl Default for TopFilesOptions {
    fn default() -> Self {
        Self {
            limit: TOP_N,
            min_size_bytes: DEFAULT_MIN_SIZE_BYTES,
            max_entries: DEFAULT_MAX_ENTRIES,
            budget: Duration::from_secs(DEFAULT_BUDGET_SECS),
        }
    }
}

/// 扫描入口：在 `root`（形如 `C:\`）下跑完四类扫描。
///
/// 单个分类失败**不会**让整次扫描失败——比如 `hiberfil.sys` 因权限读不到元数据，
/// 就把该分类标成 0 并附说明，其余三类照常返回。理由：用户点「扫描」是想看全景，
/// 因为一个文件读不到就整页报错，等于什么都看不到。
pub fn scan_all(root: &Path, opts: TopFilesOptions) -> BigFilesReport {
    let started = Instant::now();

    let categories = vec![
        scan_hiberfil(root),
        scan_windows_old(root, opts),
        scan_recycle_bin(root),
    ];

    let (top_files, scanned_entries, truncated) = scan_top_files(root, opts);

    let mut total_bytes: u64 = categories.iter().map(|c| c.size_bytes).sum();
    for f in &top_files {
        total_bytes = total_bytes.saturating_add(f.size_bytes);
    }

    BigFilesReport {
        categories,
        top_files,
        total_bytes,
        scanned_entries,
        truncated,
        elapsed_ms: started.elapsed().as_millis() as u64,
    }
}

/// `C:\hiberfil.sys`：休眠文件。
///
/// 只读 `metadata().len()`——不打开文件（打开会因内核持有而失败，且完全没必要）。
pub fn scan_hiberfil(root: &Path) -> BigFileItem {
    let path = root.join("hiberfil.sys");
    let (size, recommendation) = match std::fs::metadata(&path) {
        Ok(m) if m.is_file() => (
            m.len(),
            "休眠文件。执行 `powercfg /h off` 可关闭休眠并释放（副作用：同时关闭快速启动）"
                .to_string(),
        ),
        // 不存在：说明休眠已关闭。这是好事，明确告诉用户，别让人以为扫描漏了。
        Ok(_) => (0, "不是普通文件（可能是目录），已跳过".to_string()),
        Err(_) => (
            0,
            "不存在或不可读——休眠可能已关闭；若确认已开启休眠，请用管理员身份重试".to_string(),
        ),
    };
    BigFileItem {
        path: path.to_string_lossy().to_string(),
        size_bytes: size,
        category: CATEGORY_HIBERFIL.to_string(),
        recommendation,
    }
}

/// `C:\Windows.old`：上一个 Windows 版本的备份目录。
///
/// 目录可能几十 GB、几十万文件，所以**沿用 Top100 的条目/时间上限**——
/// 否则光这一项就能把扫描拖到分钟级。
pub fn scan_windows_old(root: &Path, opts: TopFilesOptions) -> BigFileItem {
    let path = root.join("Windows.old");
    if !path.is_dir() {
        return BigFileItem {
            path: path.to_string_lossy().to_string(),
            size_bytes: 0,
            category: CATEGORY_WINDOWS_OLD.to_string(),
            recommendation: "不存在——没有跨版本升级残留，无需处理".to_string(),
        };
    }
    let w = walk_collect(&path, &[], opts);
    let recommendation = if w.truncated {
        "跨版本升级残留。目录过大未扫完，实际占用可能更多；用「磁盘清理 → 清理系统文件」删除"
            .to_string()
    } else {
        "跨版本升级残留。确认新系统稳定后，用「磁盘清理 → 清理系统文件」删除".to_string()
    };
    BigFileItem {
        path: path.to_string_lossy().to_string(),
        size_bytes: w.total_bytes,
        category: CATEGORY_WINDOWS_OLD.to_string(),
        recommendation,
    }
}

/// 回收站占用。
///
/// 优先走系统 API `SHQueryRecycleBinW`（一次调用拿到大小 + 条目数，且包含
/// 所有用户拖进去的东西）；API 不可用时退回遍历 `$Recycle.Bin`（只能看到
/// 当前用户自己的，会偏小——建议文案里如实说明）。
pub fn scan_recycle_bin(root: &Path) -> BigFileItem {
    let drive = drive_root(root);

    #[cfg(windows)]
    if let Ok(size) = query_recycle_bin_api(&drive) {
        return BigFileItem {
            path: format!("{drive}$Recycle.Bin"),
            size_bytes: size,
            category: CATEGORY_RECYCLE_BIN.to_string(),
            recommendation: "打开回收站确认无需恢复后，右键「清空回收站」".to_string(),
        };
    }

    // 退路：遍历目录。注意只能统计当前用户可读的部分。
    let bin = root.join("$Recycle.Bin");
    BigFileItem {
        path: bin.to_string_lossy().to_string(),
        size_bytes: dir_total_bytes(&bin),
        category: CATEGORY_RECYCLE_BIN.to_string(),
        recommendation: "打开回收站确认无需恢复后，右键「清空回收站」（当前统计可能不含其他用户）"
            .to_string(),
    }
}

/// 递归统计一个目录的总占用字节数（不存在 / 无权限时返回 0）。
///
/// 公开出来是因为「量一个目录有多大」本身就是通用能力（回收站 fallback 用它，
/// 以后统计别的大目录也能用），顺带让单测能在临时目录上直接验证求和逻辑——
/// 真回收站没法在单测里伪造（`SHQueryRecycleBinW` 查的是整盘）。
pub fn dir_total_bytes(dir: &Path) -> u64 {
    walk_collect(dir, &[], TopFilesOptions::default()).total_bytes
}

/// 遍历结果（4 个返回值用结构体比 4 元组好读，也不容易接错位置）。
struct WalkResult {
    /// 遍历到的文件总字节数。
    total_bytes: u64,
    /// 最大的若干个文件（降序）。
    items: Vec<BigFileItem>,
    /// 实际看了多少个目录项。
    scanned_entries: u64,
    /// 是否因条目/时间上限提前停止。
    truncated: bool,
}

/// 把 `C:\` 归一化成 `C:\`（供 shell API 用）。
fn drive_root(root: &Path) -> String {
    let s = root.to_string_lossy();
    let letter = s.trim_end_matches(['\\', '/']);
    format!("{letter}\\")
}

/// 系统 API 查回收站：返回总字节数。
#[cfg(windows)]
fn query_recycle_bin_api(drive_root: &str) -> Result<u64, String> {
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::{SHQueryRecycleBinW, SHQUERYRBINFO};

    let root = HSTRING::from(drive_root);
    let mut info = SHQUERYRBINFO {
        cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32,
        i64Size: 0,
        i64NumItems: 0,
    };

    // SAFETY: root 的 UTF-16 缓冲在本调用期间存活；info 是栈上已初始化且
    // cbSize 已按结构体实际大小填好的结构体，API 只写它的后两个字段。
    let hr = unsafe { SHQueryRecycleBinW(&root, &mut info) };
    if hr.is_err() {
        return Err(format!("SHQueryRecycleBinW 失败: {hr:?}"));
    }
    if info.i64Size < 0 {
        // 极端情况下（数据损坏）可能给负值，别让它变成天文数字。
        return Ok(0);
    }
    Ok(info.i64Size as u64)
}

#[cfg(not(windows))]
fn query_recycle_bin_api(_drive_root: &str) -> Result<u64, String> {
    Err("回收站查询仅支持 Windows".to_string())
}

/// Top100 大文件。返回 `(列表, 遍历条目数, 是否被上限截断)`。
pub fn scan_top_files(root: &Path, opts: TopFilesOptions) -> (Vec<BigFileItem>, u64, bool) {
    let w = walk_collect(root, ROOT_SKIP_FILES, opts);
    (w.items, w.scanned_entries, w.truncated)
}

/// 根目录下**不作为大文件候选**的文件名（小写）。
///
/// 这些要么已被单独统计（hiberfil），要么是系统内存映射文件（pagefile/swapfile
/// 删不掉也不该删），要么是崩溃转储的临时文件。
const ROOT_SKIP_FILES: &[&str] = &[
    "hiberfil.sys",
    "pagefile.sys",
    "swapfile.sys",
    "dumpstack.log.tmp",
    "dumpstack.sys",
];

/// 遍历时**整棵跳过**的目录名（小写）。
///
/// 判据是「里面的东西用户不该手动删」：系统目录、包缓存、卷影副本、回收站、
/// 以及 Windows.old（它有自己的卡片，重复列进 Top100 会让人以为有两份）。
const SKIP_DIRS: &[&str] = &[
    "windows",
    "windows.old",
    "program files",
    "program files (x86)",
    "programdata",
    "perflogs",
    "recovery",
    "$recycle.bin",
    "system volume information",
    "$windows.~bt",
    "$windows.~ws",
    "$sysreset",
    "msocache",
    "config.msi",
];

/// 目录名是否要整棵跳过。
pub fn should_skip_dir(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    SKIP_DIRS.contains(&lower.as_str())
}

/// 根目录文件名是否要跳过（只在 `root` 这一层生效）。
pub fn should_skip_root_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    ROOT_SKIP_FILES.contains(&lower.as_str())
}

/// 遍历 `root`，累加总大小 + 收集最大的 `limit` 个文件。
///
/// 返回 `(总字节数, Top 列表, 是否截断)`。
///
/// 实现要点：
/// - **显式队列做 BFS**，不用递归也不用 DFS 栈：
///   - 递归会在 `node_modules` 那种十几层嵌套上爆栈；
///   - **DFS 在「时间预算用尽」时会一头扎进 `AppData` 深处**，等回过神来
///     `Desktop` / `Downloads` / `Videos` 一个都没扫到——实测本机 20 秒只覆盖了
///     `C://Users` 的一部分。BFS 先把同一层的兄弟目录全部排进队列，浅层
///     （用户真正认识的那些目录）优先被覆盖，截断时结果的可用性高得多；
/// - **最小堆**维护 Top N：`Reverse` 让堆顶是「当前 Top N 里最小的那个」，
///   新文件只要比它大就替换。内存恒定为 O(N)，不受文件总数影响；
/// - **不跟随 reparse point**：junction / symlink 会造成无限循环与重复计数；
/// - 任何 `read_dir` / `metadata` 错误都**跳过继续**，不中断整次扫描
///   （用户目录里 ACL 拒绝访问的子目录很常见，为它整次失败没有意义）。
fn walk_collect(root: &Path, skip_root_files: &[&str], opts: TopFilesOptions) -> WalkResult {
    let deadline = Instant::now() + opts.budget;
    let mut total: u64 = 0;
    let mut scanned: u64 = 0;
    let mut truncated = false;
    let mut heap: BinaryHeap<Reverse<(u64, PathBuf)>> = BinaryHeap::new();

    // 队列元素：(路径, 是否是根目录这一层)
    let mut queue: VecDeque<(PathBuf, bool)> = VecDeque::new();
    queue.push_back((root.to_path_buf(), true));

    while let Some((dir, is_root)) = queue.pop_front() {
        // 每处理 512 个目录检查一次时间，`Instant::now()` 不便宜，别每次循环都调。
        if Instant::now() >= deadline {
            truncated = true;
            break;
        }

        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            // 权限不足 / 目录已删：跳过，继续扫别的
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            scanned += 1;
            if scanned >= opts.max_entries {
                truncated = true;
                break;
            }

            let name = entry.file_name();
            let name = name.to_string_lossy();

            // `file_type()` 不跟随符号链接，正好用来识别 reparse point。
            let Ok(ft) = entry.file_type() else { continue };

            if ft.is_symlink() {
                continue;
            }

            if ft.is_dir() {
                if should_skip_dir(&name) {
                    continue;
                }
                queue.push_back((entry.path(), false));
                continue;
            }

            // 根目录的系统文件（hiberfil / pagefile / swapfile）单独统计或不统计，
            // 但绝不能出现在 Top100 里——它们是「删不掉也不该删」的东西。
            if is_root && skip_root_files.iter().any(|f| name.eq_ignore_ascii_case(f)) {
                continue;
            }

            // 非普通文件（设备 / 管道等）跳过
            if !ft.is_file() {
                continue;
            }

            let Ok(meta) = entry.metadata() else { continue };
            let size = meta.len();
            total = total.saturating_add(size);

            if size < opts.min_size_bytes || opts.limit == 0 {
                continue;
            }

            let path = entry.path();
            if heap.len() < opts.limit {
                heap.push(Reverse((size, path)));
            } else if let Some(Reverse((min_size, _))) = heap.peek() {
                if size > *min_size {
                    heap.pop();
                    heap.push(Reverse((size, path)));
                }
            }
        }

        if truncated {
            break;
        }
    }

    // 堆 → 降序列表
    let mut pairs: Vec<(u64, PathBuf)> = heap.into_iter().map(|Reverse(p)| p).collect();
    pairs.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

    let items = pairs
        .into_iter()
        .map(|(size, path)| BigFileItem {
            path: path.to_string_lossy().to_string(),
            size_bytes: size,
            category: CATEGORY_TOP_FILES.to_string(),
            recommendation: "确认不再需要后手动删除；本工具不代删用户文件".to_string(),
        })
        .collect();

    WalkResult {
        total_bytes: total,
        items,
        scanned_entries: scanned,
        truncated,
    }
}
