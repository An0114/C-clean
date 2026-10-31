//! task-065：DISM `/analyzecomponentstore` 文本输出解析用例。
//!
//! 为什么是**文本**而不是 XML：`/analyzecomponentstore` 没有 XML 输出
//! （任务书写「解析 XML」与事实不符，详见 `engine/src/scan/winsxs.rs` 模块注释）。
//!
//! 这里全部是纯函数用例，不启动 DISM、不弹 UAC——解析逻辑与「怎么跑 DISM」
//! 解耦，正是为了能在单测里钉住字段映射。

use engine::scan::winsxs::{parse_analyze_output, parse_size, tail_lines};

const GB: f64 = 1024.0 * 1024.0 * 1024.0;
const MB: f64 = 1024.0 * 1024.0;

/// 真实 `dism /online /cleanup-image /analyzecomponentstore /English` 的输出形态
/// （含进度条、空行、4 空格缩进的子项）。
const REAL_OUTPUT: &str = "\
Deployment Image Servicing and Management tool
Version: 10.0.19041.844

Image Version: 10.0.19041.844

[==========================100.0%==========================]

Component Store (WinSxS) information:

Windows Explorer Reported Size of Component Store : 7.62 GB

Actual Size of Component Store : 7.36 GB

    Shared with Windows : 6.13 GB
    Backups and Disabled Features : 1.20 GB
    Cache and Temporary Data : 30.15 MB

Date of Last Cleanup : 2021-03-15 08:21:46

Number of Reclaimable Packages : 0
Component Store Cleanup Recommended : No

The operation completed successfully.
";

#[test]
fn parses_all_key_fields_from_real_output() {
    let a = parse_analyze_output(REAL_OUTPUT);

    assert!(a.parsed, "真实输出必须能解析出关键字段");
    assert_eq!(a.actual_size_bytes, (7.36 * GB) as u64);
    assert_eq!(a.backups_and_disabled_bytes, (1.20 * GB) as u64);
    assert_eq!(a.cache_and_temp_bytes, (30.15 * MB) as u64);
    assert_eq!(a.reclaimable_packages, 0);
    assert!(!a.recommended_cleanup);
    assert_eq!(a.last_cleanup.as_deref(), Some("2021-03-15 08:21:46"));
    // 原始输出要原样保留（出错时前端展示、排障唯一线索）
    assert_eq!(a.raw_output, REAL_OUTPUT);
}

#[test]
fn reclaimable_falls_back_to_backups_plus_cache() {
    // 这份输出没有 `Actual Reclaimable Space` 行（多数 Win10 版本都不打印）
    let a = parse_analyze_output(REAL_OUTPUT);
    assert_eq!(
        a.reclaimable_bytes,
        (1.20 * GB) as u64 + (30.15 * MB) as u64,
        "缺 Reclaimable Space 行时应按「备份 + 缓存」兜底"
    );
    assert!(a.has_reclaimable(), "1.2GB 备份应该算「有东西可清」");
}

#[test]
fn explicit_reclaimable_space_wins_over_fallback() {
    // 有的版本会明写可清理空间，此时以它为准（不要再用两项之和覆盖）
    let text = "\
Actual Size of Component Store : 7.36 GB
    Backups and Disabled Features : 1.20 GB
    Cache and Temporary Data : 30.15 MB
Actual Reclaimable Space : 512.00 MB
Number of Reclaimable Packages : 3
Component Store Cleanup Recommended : Yes
";
    let a = parse_analyze_output(text);
    assert_eq!(a.reclaimable_bytes, (512.0 * MB) as u64);
    assert_eq!(a.reclaimable_packages, 3);
    assert!(a.recommended_cleanup, "Yes 必须解析成 true");
    assert!(a.has_reclaimable());
}

#[test]
fn recommended_no_is_false_and_not_truthy() {
    let a = parse_analyze_output("Component Store Cleanup Recommended : No");
    assert!(a.parsed);
    assert!(!a.recommended_cleanup, "No 不能被当成「非空字符串 = true」");
}

#[test]
fn label_whitespace_is_normalized() {
    // 不同版本标签里的空格数不一致，且子项带缩进——都要能认出来
    let text = "\
\tActual   Size of Component Store : 1.00 GB
        Cache  and Temporary Data : 2.00 MB
";
    let a = parse_analyze_output(text);
    assert_eq!(a.actual_size_bytes, (1.0 * GB) as u64);
    assert_eq!(a.cache_and_temp_bytes, (2.0 * MB) as u64);
}

#[test]
fn garbage_output_is_marked_unparsed() {
    // 解析不出来时必须能区分「没东西可清」和「分析失败」——
    // 全 0 但 parsed=false，调用方据此报错而不是显示「可清理 0 字节」。
    let text = "错误: 740\n需要提升权限才能运行 DISM。\n";
    let a = parse_analyze_output(text);
    assert!(!a.parsed);
    assert_eq!(a.reclaimable_bytes, 0);
    assert_eq!(a.reclaimable_packages, 0);
    assert!(!a.recommended_cleanup);
    // 原文照旧保留，错误消息里要用
    assert_eq!(a.raw_output, text);
}

#[test]
fn empty_output_is_unparsed_and_empty() {
    let a = parse_analyze_output("");
    assert!(!a.parsed);
    assert!(!a.has_reclaimable());
    assert!(a.last_cleanup.is_none());
}

#[test]
fn parse_size_handles_units() {
    assert_eq!(parse_size("0 B"), Some(0));
    assert_eq!(parse_size("512 B"), Some(512));
    assert_eq!(parse_size("1 KB"), Some(1024));
    assert_eq!(parse_size("1 MB"), Some(1024 * 1024));
    assert_eq!(parse_size("1 GB"), Some(1024 * 1024 * 1024));
    assert_eq!(parse_size("1 TB"), Some(1024_u64.pow(4)));
    // 无单位按字节（DISM 一定带单位，这里只是别 panic）
    assert_eq!(parse_size("42"), Some(42));
    // 大小写不敏感
    assert_eq!(parse_size("1 gb"), Some(1024 * 1024 * 1024));
    // 非法输入返回 None（而不是 0，避免把「解析失败」伪装成「0 字节」）
    assert_eq!(parse_size(""), None);
    assert_eq!(parse_size("abc GB"), None);
    assert_eq!(parse_size("-1 GB"), None);
    assert_eq!(parse_size("1 PB"), None);
}

#[test]
fn tail_lines_keeps_only_the_end() {
    let text = "a\n\n b \nc\n d \n";
    assert_eq!(tail_lines(text, 2), "c / d");
    // n 超过总行数不 panic
    assert_eq!(tail_lines(text, 99), "a / b / c / d");
    assert_eq!(tail_lines("", 3), "");
}
