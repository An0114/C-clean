//! task-065：提权命令白名单的**两侧一致性**测试。
//!
//! 白名单在 `elevation/src/main.rs` 与 `engine/src/elevation/mod.rs` 各存一份
//! （提权 exe 刻意零依赖，不引 common crate 共享常量）。漏同步的后果很隐蔽：
//! engine 侧放行 → 拉起提权 exe → 提权侧不认识 → 退出码 3 → 用户看到
//! 「未知命令」，而两侧源码各自看起来都没问题。
//!
//! 这里直接 `include_str!` 把提权 exe 的源码读进测试，抠出它的常量来比对。
//! 这是编译期就把两份钉在一起的唯一办法（运行期比对需要真的跑一次提权，
//! 单测里会弹 UAC，绝对不能做）。

/// 提权 exe 源码。路径相对本文件（`engine/tests/`）。
const ELEVATION_SRC: &str = include_str!("../../elevation/src/main.rs");

/// 从源码里抠出 `const ALLOWED_COMMANDS: &[&str] = &[...];` 的字符串字面量。
///
/// 不用正则（不想为一个测试引 regex 依赖）；按「定位常量 → 截到第一个 `;` →
/// 扫引号对」三步走。截到 `;` 而不是只看一行，是为了容忍 rustfmt 把长行折行。
fn parse_allowed_from_source(src: &str) -> Vec<String> {
    let start = src
        .find("const ALLOWED_COMMANDS")
        .expect("elevation/src/main.rs 里找不到 const ALLOWED_COMMANDS");
    let rest = &src[start..];
    let end = rest
        .find(';')
        .expect("ALLOWED_COMMANDS 声明没有以 `;` 结束");
    let decl = &rest[..end];

    // 只扫 `=` 之后的部分：常量名和类型标注里不可能有字符串字面量，
    // 但显式切开更不容易误伤。
    let tail = match decl.split_once('=') {
        Some((_, t)) => t,
        None => panic!("ALLOWED_COMMANDS 声明里没有 `=`"),
    };

    let mut out = Vec::new();
    let mut cur = String::new();
    let mut inside = false;
    for c in tail.chars() {
        match c {
            '"' => {
                if inside {
                    out.push(std::mem::take(&mut cur));
                }
                inside = !inside;
            }
            _ if inside => cur.push(c),
            _ => {}
        }
    }
    out
}

#[test]
fn engine_whitelist_matches_elevation_binary() {
    let mut from_src = parse_allowed_from_source(ELEVATION_SRC);
    let mut from_engine: Vec<String> = engine::elevation::ALLOWED_COMMANDS
        .iter()
        .map(|s| s.to_string())
        .collect();

    // 解析器自检：抠出来是空的说明解析逻辑坏了，此时两个空 vec 相等毫无意义。
    assert!(
        !from_src.is_empty(),
        "没能从提权 exe 源码里解析出任何命令，解析器已失效"
    );

    from_src.sort();
    from_engine.sort();
    assert_eq!(
        from_src, from_engine,
        "两侧命令白名单不一致（提权 exe 侧 / engine 侧）——必须同时修改"
    );
}

#[test]
fn dism_commands_are_whitelisted_on_engine_side() {
    // task-065：WinSxS 分析/清理走提权 exe，engine 侧必须放行，
    // 否则 `run_elevated` 会在弹 UAC **之前**就报「不在白名单」。
    for cmd in ["dism_analyze", "dism_cleanup"] {
        assert!(
            engine::elevation::validate_command(cmd).is_ok(),
            "{cmd} 必须在 engine 侧白名单内"
        );
    }
}

#[test]
fn resetbase_is_never_a_command() {
    // `/ResetBase` 是 DISM 的**参数**而不是本 exe 的命令；这里再钉一次
    // 「没有任何命令名与它沾边」，防止有人为了「支持」它而开一条新命令。
    for cmd in engine::elevation::ALLOWED_COMMANDS {
        assert!(
            !cmd.to_ascii_lowercase().contains("resetbase"),
            "白名单里出现了与 /ResetBase 相关的命令：{cmd}"
        );
    }
}
