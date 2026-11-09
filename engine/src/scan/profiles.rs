//! task-083 C1：浏览器多 Profile 枚举。
//!
//! # 为什么做成纯函数 / 可注入目录
//!
//! 单测不能依赖用户机器上到底有几个 Chrome profile。所以两个入口都接受
//! **显式的 `User Data` 目录 / `profiles.ini` 文本**，返回结果由调用方决定
//! 怎么用。真实扫描接线时把 `%LOCALAPPDATA%\Google\Chrome\User Data` 传进来即可。
//!
//! # Chrome / Edge 的识别规则
//!
//! `User Data` 下除了 profile 还有一堆别的东西（`Local State`、`Crashpad`、
//! `Default`、`Profile 1`…）。只有两类算 profile：
//! - 名字正好是 `Default`
//! - 名字形如 `Profile <数字>`
//!
//! 且目录里**至少要有** `Cache` / `Code Cache` / `GPUCache` 之一——
//! 空目录是残留，扫了也是 0 项，只会让 UI 多出一行没用的分组。
//!
//! # Firefox 的识别规则
//!
//! `profiles.ini` 是 INI 格式，每个 `[ProfileN]` 段有 `Name=` 与 `Path=`，
//! 还有 `[Install*]` 段（不是 profile，要跳过）。`Path` 可能是相对路径
//! （相对 `Profiles/` 目录）也可能是绝对路径，两者都要能处理。

use std::path::{Path, PathBuf};

/// 一个浏览器 profile 及其缓存根目录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserProfile {
    /// 展示名（Chrome: `Default` / `Profile 1`；Firefox: `Name=` 的值）。
    pub name: String,
    /// 该 profile 下真实存在的缓存目录。
    pub cache_roots: Vec<PathBuf>,
}

/// Chromium 系（Chrome / Edge）认的缓存子目录名。
const CHROMIUM_CACHE_DIRS: &[&str] = &["Cache", "Code Cache", "GPUCache"];
/// Firefox 认的缓存子目录名。
const FIREFOX_CACHE_DIRS: &[&str] = &["cache2", "startupCache", "thumbnails"];

fn existing_caches(profile_dir: &Path, names: &[&str]) -> Vec<PathBuf> {
    names
        .iter()
        .map(|n| profile_dir.join(n))
        .filter(|p| p.is_dir())
        .collect()
}

fn is_chromium_profile_dir(name: &str) -> bool {
    if name == "Default" {
        return true;
    }
    if let Some(rest) = name.strip_prefix("Profile ") {
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit());
    }
    false
}

/// 枚举 Chromium 系 `User Data` 目录下的所有 profile。
///
/// 目录不存在 / 读不了时返回空——用户没装这个浏览器是常态，不是错误。
pub fn chromium_profiles(user_data: &Path) -> Vec<BrowserProfile> {
    let Ok(entries) = std::fs::read_dir(user_data) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if !ft.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_chromium_profile_dir(&name) {
            continue;
        }
        let caches = existing_caches(&entry.path(), CHROMIUM_CACHE_DIRS);
        if caches.is_empty() {
            continue;
        }
        out.push(BrowserProfile {
            name,
            cache_roots: caches,
        });
    }
    // 排序保证输出确定（read_dir 顺序不保证），`Default` 排在 `Profile N` 前面
    out.sort_by(|a, b| {
        let rank = |n: &str| if n == "Default" { 0 } else { 1 };
        (rank(&a.name), &a.name).cmp(&(rank(&b.name), &b.name))
    });
    out
}

/// 解析 Firefox `profiles.ini` 文本，列出 profile。
///
/// `profiles_root` 是 `Profiles/` 目录；相对 `Path=` 相对它解析。
pub fn firefox_profiles(ini: &str, profiles_root: &Path) -> Vec<BrowserProfile> {
    let mut out = Vec::new();
    let mut cur_name: Option<String> = None;
    let mut cur_path: Option<String> = None;
    let mut in_profile_section = false;

    let flush = |name: &mut Option<String>,
                 path: &mut Option<String>,
                 root: &Path,
                 out: &mut Vec<BrowserProfile>| {
        if let (Some(n), Some(p)) = (name.take(), path.take()) {
            let dir = if std::path::Path::new(&p).is_absolute() {
                PathBuf::from(p)
            } else {
                // INI 里可能写成 `Profiles/xxxx` 或纯 `xxxx`，统一按正斜杠处理
                root.join(p.replace('/', "\\"))
            };
            let caches = existing_caches(&dir, FIREFOX_CACHE_DIRS);
            if !caches.is_empty() {
                out.push(BrowserProfile {
                    name: n,
                    cache_roots: caches,
                });
            }
        } else {
            name.take();
            path.take();
        }
    };

    for raw in ini.lines() {
        let line = raw.trim();
        if line.starts_with('[') && line.ends_with(']') {
            flush(&mut cur_name, &mut cur_path, profiles_root, &mut out);
            let section = &line[1..line.len() - 1];
            // `[Install...]` / `[General]` 不是 profile 段
            in_profile_section = section.starts_with("Profile");
            continue;
        }
        if !in_profile_section {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            match k.trim() {
                "Name" => cur_name = Some(v.trim().to_string()),
                "Path" => cur_path = Some(v.trim().to_string()),
                _ => {}
            }
        }
    }
    flush(&mut cur_name, &mut cur_path, profiles_root, &mut out);
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}
