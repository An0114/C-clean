//! Windows 注册表最小封装（task-067 起用，task-068 抽成独立模块）。
//!
//! # 为什么手写 FFI 而不是引 `winreg` crate
//!
//! 只需要「开键 / 枚举子键 / 枚举值 / 读字符串 / 读 DWORD / 删值」六件事，
//! 而 `windows` crate **已在依赖里**、`Win32_System_Registry` feature 也在
//! task-062 就为 `SHELLEXECUTEINFOW` 的 `hkeyClass` 字段开好了——
//! **Cargo.toml 一行都不用动**。引一个新 crate 只会拖慢编译（本 crate 一直把
//! 编译时间当指标）。
//!
//! # 为什么对调用方隐藏 `HKEY`
//!
//! 公开 API 只暴露 [`RegRoot`] 枚举，不暴露 `windows` 的 `HKEY` 类型。
//! 否则 engine 的每个调用方都得 `use windows::Win32::System::Registry::HKEY_*`，
//! 而且 engine 在非 Windows 平台就编不过（虽然本产品只跑 Windows，
//! 但测试代码不该被平台类型污染）。
//!
//! # 错误语义
//!
//! 读操作一律「拿不到就返回空 / None」——注册表里缺项是常态，不是错误。
//! 只有**删除**会返回 `Result`，因为删除失败必须让调用方知道。

use std::path::Path;

/// 注册表根键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegRoot {
    /// `HKEY_LOCAL_MACHINE`。
    LocalMachine,
    /// `HKEY_CURRENT_USER`。
    CurrentUser,
}

impl RegRoot {
    /// 写进 `.reg` 文件 / 日志里的根键名。
    pub fn label(self) -> &'static str {
        match self {
            Self::LocalMachine => "HKEY_LOCAL_MACHINE",
            Self::CurrentUser => "HKEY_CURRENT_USER",
        }
    }

    /// 短名（日志用）。
    pub fn short(self) -> &'static str {
        match self {
            Self::LocalMachine => "HKLM",
            Self::CurrentUser => "HKCU",
        }
    }
}

/// 注册表值类型（与 Win32 的 `REG_*` 常量一致）。
pub mod value_type {
    /// `REG_SZ`
    pub const SZ: u32 = 1;
    /// `REG_EXPAND_SZ`
    pub const EXPAND_SZ: u32 = 2;
    /// `REG_BINARY`
    pub const BINARY: u32 = 3;
    /// `REG_DWORD`
    pub const DWORD: u32 = 4;
    /// `REG_MULTI_SZ`
    pub const MULTI_SZ: u32 = 7;
}

/// 一个注册表值（原始字节 + 类型）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegValue {
    /// 值名。空字符串表示「默认值」。
    pub name: String,
    /// 值类型（`REG_SZ` = 1 等）。
    pub kind: u32,
    /// 原始数据字节。
    ///
    /// 字符串类型是 **UTF-16LE**（含结尾 NUL），不要当 UTF-8 解。
    pub data: Vec<u8>,
}

impl RegValue {
    /// 字符串类型的值 → `String`（去掉结尾 NUL）。非字符串类型返回 None。
    pub fn as_string(&self) -> Option<String> {
        if self.kind != value_type::SZ && self.kind != value_type::EXPAND_SZ {
            return None;
        }
        Some(utf16_bytes_to_string(&self.data))
    }

    /// DWORD 类型的值 → `u32`（小端序）。
    pub fn as_dword(&self) -> Option<u32> {
        if self.kind != value_type::DWORD || self.data.len() < 4 {
            return None;
        }
        Some(u32::from_le_bytes([
            self.data[0],
            self.data[1],
            self.data[2],
            self.data[3],
        ]))
    }
}

/// UTF-16LE 字节 → String（去掉结尾 NUL）。
pub fn utf16_bytes_to_string(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let s = String::from_utf16_lossy(&units);
    s.trim_end_matches('\0').to_string()
}

/// 枚举 `root\subkey` 下的直接子键名。打不开就返回空。
pub fn subkey_names(root: RegRoot, subkey: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        imp::subkey_names(root, subkey)
    }
    #[cfg(not(windows))]
    {
        let _ = (root, subkey);
        Vec::new()
    }
}

/// 读一个 `REG_SZ` / `REG_EXPAND_SZ` 值。
pub fn string_value(root: RegRoot, subkey: &str, value: &str) -> Option<String> {
    #[cfg(windows)]
    {
        imp::string_value(root, subkey, value)
    }
    #[cfg(not(windows))]
    {
        let _ = (root, subkey, value);
        None
    }
}

/// 读一个 `REG_DWORD` 值。
pub fn dword_value(root: RegRoot, subkey: &str, value: &str) -> Option<u32> {
    #[cfg(windows)]
    {
        imp::dword_value(root, subkey, value)
    }
    #[cfg(not(windows))]
    {
        let _ = (root, subkey, value);
        None
    }
}

/// 枚举 `root\subkey` 下的**所有值**（含默认值）。打不开就返回空。
pub fn values(root: RegRoot, subkey: &str) -> Vec<RegValue> {
    #[cfg(windows)]
    {
        imp::values(root, subkey)
    }
    #[cfg(not(windows))]
    {
        let _ = (root, subkey);
        Vec::new()
    }
}

/// 删除 `root\subkey` 下的一个**值**（不是删整个键）。
///
/// 只删值不删键：删整键会把同键下其他程序的启动项一起干掉。
pub fn delete_value(root: RegRoot, subkey: &str, value: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        imp::delete_value(root, subkey, value)
    }
    #[cfg(not(windows))]
    {
        let _ = (root, subkey, value);
        Err("注册表操作仅支持 Windows".to_string())
    }
}

// ===================== .reg 导出（纯文本生成，可单测） =====================

/// 生成 `.reg` 文件内容（Windows Registry Editor 5.00 格式，双击可导入还原）。
///
/// # 为什么自己拼而不是 `reg export`
///
/// 1. **不 spawn 进程**：本项目的原则是不调 `cmd.exe` / `powershell` / 外部 exe，
///    避免命令注入与「安全软件拦截」两类问题（`reg.exe` 在受限环境里会被直接拦掉）；
/// 2. **可单测**：生成的是纯字符串，输入输出都能在单测里钉死；
/// 3. **粒度可控**：`reg export` 只能导整个键，这里也只导整个键（`.reg` 格式本来
///    就不支持单值导入），但内容完全由我们决定。
///
/// 字符串值转义规则：`\` → `\\`，`"` → `\"`。不转义会让导入时报「格式无效」。
pub fn build_reg_export(root: RegRoot, subkey: &str, values: &[RegValue]) -> String {
    let mut out = String::from("Windows Registry Editor Version 5.00\r\n\r\n");
    out.push_str(&format!("[{}\\{}]\r\n", root.label(), subkey));
    for v in values {
        out.push_str(&reg_value_line(v));
        out.push('\r');
        out.push('\n');
    }
    out
}

/// 单个值 → `.reg` 里的一行。
fn reg_value_line(v: &RegValue) -> String {
    // 默认值（空名）在 .reg 里写作 `@=`
    let name_part = if v.name.is_empty() {
        "@".to_string()
    } else {
        format!("\"{}\"", escape_reg_string(&v.name))
    };

    match v.kind {
        value_type::SZ => format!(
            "{}={}",
            name_part,
            quote_reg_string(&utf16_bytes_to_string(&v.data))
        ),
        value_type::EXPAND_SZ => format!("{}=hex(2):{}", name_part, hex_list(&v.data)),
        value_type::DWORD => {
            let n = v.as_dword().unwrap_or(0);
            format!("{}=dword:{:08x}", name_part, n)
        }
        // 其他类型（BINARY / MULTI_SZ / 未知）统一按 hex 原样导出，
        // 导入时 Windows 会按 `hex(N)` 还原成对应类型。
        other => {
            if other == 0 {
                format!("{}=hex:{}", name_part, hex_list(&v.data))
            } else {
                format!("{}=hex({}):{}", name_part, other, hex_list(&v.data))
            }
        }
    }
}

/// `.reg` 字符串转义：`\` 与 `"`。
fn escape_reg_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c => out.push(c),
        }
    }
    out
}

/// `.reg` 字符串值：加引号 + 转义。
fn quote_reg_string(s: &str) -> String {
    format!("\"{}\"", escape_reg_string(s))
}

/// 字节序列 → `41,00,42,00` 形式（大写两位 hex，逗号分隔）。
///
/// 超过 25 字节要换行 + 续行 `\`，否则某些 Windows 版本的导入器会截断。
fn hex_list(bytes: &[u8]) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(bytes.len());
    for b in bytes {
        parts.push(format!("{:02X}", b));
    }
    // 每 25 个字节折一行，行尾加 `\` 续行（.reg 的约定）
    let mut out = String::new();
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            out.push(',');
            if i % 25 == 0 {
                out.push_str("\\\r\n  ");
            }
        }
        out.push_str(p);
    }
    out
}

/// 备份文件名里不能出现的字符替换成 `_`。
///
/// 键路径里有 `\` 和空格，直接当文件名会建出子目录或非法名。
pub fn sanitize_file_stem(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | ' ' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect()
}

/// 把注册表内容写成 `.reg` 备份文件，返回完整路径。
///
/// `stamp` 由调用方给（unix 秒），这样单测能钉死文件名、不用等时钟。
pub fn write_reg_backup(
    dir: &Path,
    file_stem: &str,
    content: &str,
    stamp: i64,
) -> Result<std::path::PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("创建备份目录失败 {}: {e}", dir.display()))?;
    let path = dir.join(format!("{file_stem}-{stamp}.reg"));
    std::fs::write(&path, content)
        .map_err(|e| format!("写备份文件失败 {}: {e}", path.display()))?;
    Ok(path)
}

// ===================== Windows 实现 =====================

#[cfg(windows)]
mod imp {
    use super::{RegRoot, RegValue};
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteKeyValueW, RegEnumKeyExW, RegEnumValueW, RegGetValueW, RegOpenKeyExW,
        HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
    };

    /// 一次枚举最多读多少个子键 / 值（防御异常大的键导致死循环）。
    const MAX_ENTRIES: usize = 4096;

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn hkey_of(root: RegRoot) -> HKEY {
        match root {
            RegRoot::LocalMachine => HKEY_LOCAL_MACHINE,
            RegRoot::CurrentUser => HKEY_CURRENT_USER,
        }
    }

    /// 开键的 RAII 包装：`Drop` 时关掉，避免每条错误分支都要手写 `RegCloseKey`。
    struct Key(HKEY);

    impl Drop for Key {
        fn drop(&mut self) {
            // SAFETY: 句柄由本结构独占，只关一次。
            let _ = unsafe { RegCloseKey(self.0) };
        }
    }

    fn open(root: RegRoot, subkey: &str) -> Option<Key> {
        let path = to_wide(subkey);
        let mut hkey = HKEY::default();
        // SAFETY: path 以 NUL 结尾且在本函数内存活；hkey 是栈上变量，API 只写它。
        let rc =
            unsafe { RegOpenKeyExW(hkey_of(root), PCWSTR(path.as_ptr()), 0, KEY_READ, &mut hkey) };
        if rc.is_err() {
            return None;
        }
        Some(Key(hkey))
    }

    pub fn subkey_names(root: RegRoot, subkey: &str) -> Vec<String> {
        let Some(key) = open(root, subkey) else {
            return Vec::new();
        };
        let mut names = Vec::new();
        let mut buf = [0u16; 512];
        for idx in 0..MAX_ENTRIES {
            let mut len: u32 = buf.len() as u32;
            // SAFETY: buf 容量已通过 len 告知；可选参数传 None（不需要类名/时间）。
            let rc = unsafe {
                RegEnumKeyExW(
                    key.0,
                    idx as u32,
                    PWSTR(buf.as_mut_ptr()),
                    &mut len,
                    None,
                    PWSTR::null(),
                    None,
                    None,
                )
            };
            if rc.is_err() {
                break; // ERROR_NO_MORE_ITEMS 就是枚举完了
            }
            names.push(String::from_utf16_lossy(&buf[..len as usize]));
        }
        names
    }

    pub fn string_value(root: RegRoot, subkey: &str, value: &str) -> Option<String> {
        let mut buf = vec![0u8; 2048];
        let mut len: u32 = buf.len() as u32;
        let path = to_wide(subkey);
        let name = to_wide(value);
        // SAFETY: 两个字符串都以 NUL 结尾且存活；buf 容量通过 len 告知。
        let rc = unsafe {
            RegGetValueW(
                hkey_of(root),
                PCWSTR(path.as_ptr()),
                PCWSTR(name.as_ptr()),
                RRF_RT_REG_SZ,
                None,
                Some(buf.as_mut_ptr() as *mut _),
                Some(&mut len),
            )
        };
        if rc.is_err() || len < 2 {
            return None;
        }
        let text = super::utf16_bytes_to_string(&buf[..len as usize]);
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    pub fn dword_value(root: RegRoot, subkey: &str, value: &str) -> Option<u32> {
        let mut data: u32 = 0;
        let mut len: u32 = 4;
        let path = to_wide(subkey);
        let name = to_wide(value);
        // SAFETY: data 是栈上 u32，len 已告知其大小；字符串均以 NUL 结尾且存活。
        let rc = unsafe {
            RegGetValueW(
                hkey_of(root),
                PCWSTR(path.as_ptr()),
                PCWSTR(name.as_ptr()),
                RRF_RT_REG_DWORD,
                None,
                Some(&mut data as *mut u32 as *mut _),
                Some(&mut len),
            )
        };
        if rc.is_err() {
            None
        } else {
            Some(data)
        }
    }

    pub fn values(root: RegRoot, subkey: &str) -> Vec<RegValue> {
        let Some(key) = open(root, subkey) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut name_buf = [0u16; 512];
        // 单个值的数据上限：8 KiB 足够覆盖启动项命令行（注册表值上限是 1 MiB，
        // 但启动项不会那么大；超了就跳过，不为了极端情况分配 1 MiB）
        let mut data_buf = vec![0u8; 8192];

        for idx in 0..MAX_ENTRIES {
            let mut name_len: u32 = name_buf.len() as u32;
            let mut kind: u32 = 0;
            let mut data_len: u32 = data_buf.len() as u32;

            // SAFETY: 两个缓冲的容量都通过对应 len 告知；kind 是栈上 u32。
            let rc = unsafe {
                RegEnumValueW(
                    key.0,
                    idx as u32,
                    PWSTR(name_buf.as_mut_ptr()),
                    &mut name_len,
                    None,
                    Some(&mut kind),
                    Some(data_buf.as_mut_ptr()),
                    Some(&mut data_len),
                )
            };
            if rc.is_err() {
                break;
            }
            out.push(RegValue {
                name: String::from_utf16_lossy(&name_buf[..name_len as usize]),
                kind,
                data: data_buf[..(data_len as usize).min(data_buf.len())].to_vec(),
            });
        }
        out
    }

    pub fn delete_value(root: RegRoot, subkey: &str, value: &str) -> Result<(), String> {
        let path = to_wide(subkey);
        let name = to_wide(value);
        // SAFETY: 两个字符串都以 NUL 结尾且在本调用期间存活。
        let rc = unsafe {
            RegDeleteKeyValueW(hkey_of(root), PCWSTR(path.as_ptr()), PCWSTR(name.as_ptr()))
        };
        if rc.is_err() {
            return Err(format!(
                "删除注册表值失败（{}\\{} :: {}）：{rc:?}",
                root.short(),
                subkey,
                value
            ));
        }
        Ok(())
    }
}
