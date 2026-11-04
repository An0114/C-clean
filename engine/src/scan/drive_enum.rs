//! 固定磁盘枚举（task-077，P2 多盘扫描起步）。
//!
//! # 为什么要单独一个模块
//!
//! task-063 的 `report::drive_usage(drive)` 只查**指定**盘，前提是调用方已经知道
//! 有哪些盘。而「多盘扫描」的第一个问题恰恰是「有哪些盘」——这个信息只能从系统
//! 拿，不能猜。所以枚举放在这里，`report::drive_usage` 保持原样（它查单个盘的
//! 容量，语义不同，合并没有好处）。
//!
//! # 只收固定盘（`DRIVE_FIXED`）
//!
//! `GetLogicalDrives` 返回的位图里混着所有类型的卷：
//!
//! | 类型 | 例子 | 要不要扫 |
//! |---|---|---|
//! | `DRIVE_FIXED`(3) | 本机 C: / D: | **要** |
//! | `DRIVE_REMOVABLE`(2) | U 盘、SD 卡 | 不要——拔了就没，扫出来的路径立刻失效 |
//! | `DRIVE_REMOTE`(4) | 映射的网络盘 | 不要——延迟不可控，且属别人的数据 |
//! | `DRIVE_CDROM`(5) | 光驱 | 不要——没有「清理」可言 |
//! | `DRIVE_RAMDISK`(6) | 内存盘 | 不要——重启即消失 |
//! | `DRIVE_NO_ROOT_DIR`(1) | 空读卡器槽位 | 不要——`GetLogicalDrives` 会给位但没有卷 |
//!
//! **这一条是安全边界，不是优化**：把 U 盘 / 网络盘列进清理列表，用户一次误勾就是
//! 删别人机器上的数据。所以白名单式筛选，只认 3。
//!
//! # 为什么不用 `GetLogicalDriveStringsW`
//!
//! 它返回 `C:\0D:\0\0` 这样的双空结尾字符串，解析要处理 `\0` 分隔与结尾判定，
//! 比位图多一层出错面。位图 + `GetDriveTypeW` 两步更直白，且两者都在已启用的
//! `Win32_Storage_FileSystem` feature 下，不新增依赖。
//!
//! # 容量查询失败不丢掉这个盘
//!
//! `GetDiskFreeSpaceExW` 可能失败（卷正在被 chkdsk 修、权限异常）。这时**仍然返回
//! 这个盘**，容量给 0——「盘存在但查不到容量」和「盘不存在」是两回事：前者扫描
//! 仍然有意义，后者才会让用户以为少了一个盘。前端对 `totalBytes == 0` 显示
//! 「容量未知」而不是画一个空环。

use std::path::Path;

/// 一个固定磁盘。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    /// 盘符字母，大写（`'C'` / `'D'`）。
    pub letter: char,
    /// 总容量（字节）。查询失败时为 0（见模块文档）。
    pub total_bytes: u64,
    /// 可用（字节，全用户可用）。查询失败时为 0。
    pub free_bytes: u64,
}

impl DriveInfo {
    /// 已用字节 = total - free（饱和减法，防「free > total」这种异常卷导致下溢）。
    pub fn used_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.free_bytes)
    }

    /// 形如 `"C:"` 的盘标识（前端下拉框 / 进度文案用）。
    pub fn label(&self) -> String {
        format!("{}:", self.letter)
    }
}

/// 从路径里取盘符字母（大写）。
///
/// # 认什么
///
/// - `C:\Users\...` → `'C'`
/// - `c:/users/...` → `'C'`（Windows 路径分隔符两种都认，且大小写不敏感）
/// - `\\?\C:\Users\...` → `'C'`（长路径前缀先剥掉，否则第 1 个字符是 `\`）
/// - `D:` → `'D'`
///
/// # 不认什么（返回 `None`）
///
/// - `\\server\share\x` → `None`：UNC 没有盘符，硬套会得到 `'\'` 这种垃圾；
/// - `/tmp/x` → `None`：非 Windows 风格绝对路径；
/// - `relative\x` → `None`：相对路径不该进 `cleanup_item`，真进来了也不该被算到某个盘。
///
/// # 为什么必须先剥 `\\?\` 前缀
///
/// `canonicalize()` 在 Windows 上返回的就是 `\\?\C:\...` 形式。task-066 的大文件
/// 扫描对路径做过 canonicalize，所以这个前缀在本项目里**真实存在**，不是理论情况。
/// 不剥的话第 1 个字符是 `\`、第 2 个是 `\`，判 `bytes[1] == b':'` 直接失败，
/// 结果是「明明是 C 盘的文件，却归不到任何盘」。
pub fn drive_letter_of(path: &str) -> Option<char> {
    // 逐级剥前缀：`\\?\` 与 `\\.\`（设备命名空间）都是 4 字符。
    let p = path
        .strip_prefix(r"\\?\")
        .or_else(|| path.strip_prefix(r"\\.\"));

    let s = p.unwrap_or(path);
    let bytes = s.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    if bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Some((bytes[0] as char).to_ascii_uppercase());
    }
    None
}

/// [`drive_letter_of`] 的 `Path` 版本（内部转 `to_string_lossy`）。
///
/// 单独给一个是为了让调用方不用每次写 `.to_string_lossy()`；
/// 路径不是合法 UTF-8 时 lossy 会替换成 `�`，但盘符是 ASCII，替换不会影响前两位。
pub fn drive_letter_of_path(path: &Path) -> Option<char> {
    drive_letter_of(&path.to_string_lossy())
}

/// 枚举所有**固定**磁盘（按盘符升序）。
///
/// 非 Windows 平台返回空：本项目只支持 Windows（`Cargo.toml` 的 windows 依赖
/// 本身就是 `cfg(windows)` 门控），但单测要在任意平台能编译，所以给一个空实现
/// 而不是 `compile_error!`。
#[cfg(windows)]
pub fn fixed_drives() -> Vec<DriveInfo> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives,
    };

    /// `GetDriveTypeW` 的返回值之一：固定磁盘。
    ///
    /// **不能用 `windows::Win32::Storage::FileSystem::DRIVE_FIXED`**——windows 0.58
    /// 没有在 FileSystem 模块里导出这组常量（实测 `unresolved import`），它们是
    /// WinBase.h 里的 `#define`，windows-rs 只在部分版本/模块下生成。
    /// 这里按头文件原值写字面量，并留注释，避免下次又去 import 一个不存在的名字。
    ///
    /// 完整取值：0=UNKNOWN / 1=NO_ROOT_DIR / 2=REMOVABLE / 3=FIXED /
    /// 4=REMOTE / 5=CDROM / 6=RAMDISK。
    const DRIVE_FIXED: u32 = 3;

    // 位图：bit0 = A:, bit1 = B:, ... 返回 0 表示调用失败（不是「没有盘」）。
    let mask = unsafe { GetLogicalDrives() };
    if mask == 0 {
        eprintln!("[drive_enum] GetLogicalDrives 返回 0（调用失败）");
        return Vec::new();
    }

    let mut out = Vec::new();
    for i in 0..26u32 {
        if mask & (1 << i) == 0 {
            continue;
        }
        let letter = (b'A' + i as u8) as char;
        // 根目录形式必须是 `C:\`，API 认这个形状。
        let root = format!("{letter}:\\");
        let root_w: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();

        // 只收固定盘。这一步必须在容量查询之前——查一个光驱的容量会卡几秒。
        let ty = unsafe { GetDriveTypeW(PCWSTR(root_w.as_ptr())) };
        if ty != DRIVE_FIXED {
            continue;
        }

        let mut free_to_caller: u64 = 0;
        let mut total: u64 = 0;
        let mut total_free: u64 = 0;
        // SAFETY: 三个 &mut 指向本栈帧上的 u64；root_w 在本调用期间存活且以 NUL 结尾。
        let ok = unsafe {
            GetDiskFreeSpaceExW(
                PCWSTR(root_w.as_ptr()),
                Some(&mut free_to_caller),
                Some(&mut total),
                Some(&mut total_free),
            )
        };
        if ok.is_err() {
            // 容量查不到不等于盘不存在（见模块文档），保留这个盘、容量记 0。
            eprintln!("[drive_enum] {letter}: 容量查询失败，仍纳入扫描（容量未知）");
            out.push(DriveInfo {
                letter,
                total_bytes: 0,
                free_bytes: 0,
            });
            continue;
        }

        out.push(DriveInfo {
            letter,
            total_bytes: total,
            // 与 report::drive_usage 一致：用 total_free（全用户可用）而不是
            // free_to_caller（当前用户配额）——用户问的是「盘还剩多少」。
            free_bytes: total_free,
        });
    }

    out.sort_by_key(|d| d.letter);
    out
}

/// 非 Windows 平台：没有盘符概念，返回空。
#[cfg(not(windows))]
pub fn fixed_drives() -> Vec<DriveInfo> {
    Vec::new()
}

/// 枚举固定盘并只返回盘符字母（`['C', 'D']`）。
pub fn fixed_drive_letters() -> Vec<char> {
    fixed_drives().iter().map(|d| d.letter).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_letter_of_plain_paths() {
        assert_eq!(
            drive_letter_of(r"C:\Users\An\AppData\Local\Temp"),
            Some('C')
        );
        assert_eq!(drive_letter_of(r"D:\GitHub\C-clean"), Some('D'));
        // 小写盘符统一成大写：后续比较 / 分组都用大写，别让大小写分叉
        assert_eq!(drive_letter_of(r"c:\windows"), Some('C'));
        // 正斜杠也认（Tauri / 前端可能传这种）
        assert_eq!(drive_letter_of("d:/games"), Some('D'));
        // 只有盘符没有路径
        assert_eq!(drive_letter_of("E:"), Some('E'));
    }

    #[test]
    fn drive_letter_of_strips_long_path_prefix() {
        // canonicalize() 的产物。不剥前缀就会得到 None（`\` 不是字母）。
        assert_eq!(drive_letter_of(r"\\?\C:\Windows\System32"), Some('C'));
        assert_eq!(drive_letter_of(r"\\.\D:\data"), Some('D'));
    }

    #[test]
    fn drive_letter_of_rejects_non_drive_paths() {
        // UNC 没有盘符，不能硬套成 '\'
        assert_eq!(drive_letter_of(r"\\server\share\file"), None);
        // 相对路径
        assert_eq!(drive_letter_of(r"relative\dir"), None);
        // 空 / 太短
        assert_eq!(drive_letter_of(""), None);
        assert_eq!(drive_letter_of("C"), None);
        // Unix 风格
        assert_eq!(drive_letter_of("/tmp/x"), None);
        // 冒号在第一位（如 "::"）不是盘符
        assert_eq!(drive_letter_of("::weird"), None);
    }

    #[test]
    fn drive_letter_of_path_matches_str_version() {
        assert_eq!(
            drive_letter_of_path(Path::new(r"C:\Users")),
            Some('C'),
            "Path 版本必须与 &str 版本一致"
        );
    }

    #[test]
    fn drive_info_used_bytes_is_saturating() {
        let d = DriveInfo {
            letter: 'C',
            total_bytes: 100,
            free_bytes: 40,
        };
        assert_eq!(d.used_bytes(), 60);
        assert_eq!(d.label(), "C:");

        // free > total（异常卷 / 配额报告异常）时不能下溢成天文数字
        let weird = DriveInfo {
            letter: 'D',
            total_bytes: 100,
            free_bytes: 200,
        };
        assert_eq!(weird.used_bytes(), 0, "free > total 时必须饱和到 0");
    }

    #[cfg(windows)]
    #[test]
    fn fixed_drives_contains_system_drive() {
        // Windows 上至少有一个固定盘（系统盘）。这条同时验证：
        // ① 位图解析没把字母算错（A+偏移）；② DRIVE_FIXED 判定没把系统盘漏掉。
        let drives = fixed_drives();
        assert!(!drives.is_empty(), "Windows 上固定盘列表不该为空");

        let sys = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
        let sys_letter = drive_letter_of(&sys).expect("SystemDrive 应形如 C:");
        assert!(
            drives.iter().any(|d| d.letter == sys_letter),
            "系统盘 {sys_letter}: 不在枚举结果里: {drives:?}"
        );

        // 按盘符升序
        let mut sorted = drives.clone();
        sorted.sort_by_key(|d| d.letter);
        assert_eq!(drives, sorted, "结果应按盘符升序");

        // 盘符必须两两不同（位图不会重复，但这条能挡住「A+偏移」写错导致的重复）
        for (i, a) in drives.iter().enumerate() {
            for b in &drives[i + 1..] {
                assert_ne!(a.letter, b.letter, "盘符重复");
            }
        }

        // 容量自洽：free ≤ total（容量未知时两者都是 0，也满足）
        for d in &drives {
            assert!(
                d.free_bytes <= d.total_bytes,
                "{}: free({}) > total({})",
                d.letter,
                d.free_bytes,
                d.total_bytes
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn fixed_drive_letters_matches_fixed_drives() {
        let a = fixed_drive_letters();
        let b: Vec<char> = fixed_drives().iter().map(|d| d.letter).collect();
        assert_eq!(a, b);
    }
}
