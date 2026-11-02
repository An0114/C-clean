//! v0.2.1 Bug-2 回归防线：扫描脚本的风险等级。
//!
//! # 背景
//!
//! 5 个 `.ps1` 原本把风险等级写死成 `1`（L1），导致清理列表「清一色 L1」，
//! 用户在真机验收时一眼看出不对（`PROJECT_STATE` Bug-2）。
//!
//! # 为什么是静态断言而不是跑脚本
//!
//! 跑脚本要 spawn 一个 Windows 命令行解释器、依赖本机环境（浏览器缓存目录
//! 是否存在、权限够不够），在 CI 与开发机上结论不稳定；而这次出问题的恰恰是
//! **源码里那个字面量**，静态检查就能钉死，且零依赖、秒级。
//!
//! 真正的端到端验证（扫出来确实是 L1+L2）交给真机冒烟——那是测试轮的活。

use std::path::{Path, PathBuf};

fn scripts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("engine 的父目录应是仓库根")
        .join("scripts")
}

fn read(name: &str) -> String {
    let p = scripts_dir().join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("读不到 {}: {e}", p.display()))
}

const SCRIPTS: &[&str] = &[
    "scan_browser_cache.ps1",
    "scan_dev_cache.ps1",
    "scan_prefetch.ps1",
    "scan_temp.ps1",
    "scan_windows_update.ps1",
];

/// 把 `Write-Item` 调用里第 4 个实参是**字面量数字**的情况抓出来。
///
/// 修复后的写法是 `Write-Item ... $risk` / `$r.risk`，不再是裸数字。
///
/// 注意**不能**取「行末 token」：旧代码那行长这样——
/// `try { Write-Item $cat $_.FullName $_.Length 1 } catch { ... }`
/// 行末是 `}`，取 last() 会漏判（第一版就是这么写的，测试假绿）。
/// 正确做法是取 `Write-Item` 之后的**前 4 个 token**：category / path / size / risk。
fn hardcoded_risk_literals(text: &str) -> Vec<String> {
    let mut hits = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("function ") || !trimmed.contains("Write-Item") {
            continue;
        }
        let Some(rest) = trimmed.split("Write-Item").nth(1) else {
            continue;
        };
        let toks: Vec<&str> = rest.split_whitespace().take(4).collect();
        if toks.len() < 4 {
            continue;
        }
        let risk = toks[3];
        if !risk.is_empty() && risk.chars().all(|c| c.is_ascii_digit()) {
            hits.push(trimmed.to_string());
        }
    }
    hits
}

/// 检测器自检：喂它一段「旧代码」，必须能抓到。
///
/// 没有这条，检测器写错了测试会静默通过——第一版就是这样假绿的。
#[test]
fn detector_catches_the_old_hardcoded_shape() {
    let old = r#"
            try { Write-Item $category $_.FullName $_.Length 1 } catch { [Console]::Error.WriteLine("skip") }
            try { Write-Item 'update_download' $_.FullName $_.Length 1 }
            try { Write-Item $r.category $_.FullName $_.Length 2 }
"#;
    let hits = hardcoded_risk_literals(old);
    assert_eq!(hits.len(), 3, "旧形状的三行都必须被抓到，实际: {hits:?}");

    let fixed = r#"
            try { Write-Item $category $_.FullName $_.Length $risk }
            try { Write-Item 'update_download' $_.FullName $_.Length $riskUpdateDownload }
            try { Write-Item $r.category $_.FullName $_.Length $r.risk }
"#;
    assert!(
        hardcoded_risk_literals(fixed).is_empty(),
        "修复后的写法不该被误报"
    );
}

#[test]
fn no_script_hardcodes_risk_level() {
    // Bug-2 的形态：`Write-Item $cat $path $len 1` —— 不管什么分类都 L1。
    for s in SCRIPTS {
        let text = read(s);
        let hits = hardcoded_risk_literals(&text);
        assert!(
            hits.is_empty(),
            "{s} 里还有把风险等级写死成数字的调用：\n  {}",
            hits.join("\n  ")
        );
    }
}

#[test]
fn system_directories_are_l2() {
    // `C:\Windows\Temp` 与更新下载缓存：多数文件属 SYSTEM，普通权限删不掉，
    // 且可能被安装程序/更新服务占用 —— 必须标 L2（需确认）。
    let temp = read("scan_temp.ps1");
    assert!(
        temp.contains("'windows_temp' 2"),
        "windows_temp 必须是 L2（需确认），实际脚本里找不到 'windows_temp' 2"
    );

    let update = read("scan_windows_update.ps1");
    assert!(
        update.contains("$riskUpdateDownload = 2"),
        "update_download 必须是 L2，实际脚本里找不到 $riskUpdateDownload = 2"
    );
}

#[test]
fn user_owned_caches_are_l1() {
    // 用户自己的临时目录、浏览器缓存、包管理器缓存、Prefetch：删了会自动重建。
    assert!(read("scan_temp.ps1").contains("'user_temp' 1"));
    assert!(read("scan_browser_cache.ps1").contains("$riskCache = 1"));
    assert!(read("scan_dev_cache.ps1").contains("$riskCache = 1"));
    assert!(read("scan_prefetch.ps1").contains("$riskPrefetch = 1"));
}
