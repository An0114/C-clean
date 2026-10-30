//! 独立提权 exe（task-062）。
//!
//! # 生命周期
//!
//! **按需 UAC 提权、执行一条命令、写结果、立刻退出**。绝不常驻、不监听端口、
//! 不读 stdin——缩小被利用的面。普通用户直接双击运行没有意义，参数不全时
//! 以 `EXIT_BAD_USAGE` 退出。
//!
//! # 安全模型（为什么是白名单而不是「透传 argv」）
//!
//! 主进程传进来的参数**不可信**（主进程可能被注入，或参数里带用户可控字符串）。
//! 所以这里：
//! 1. `argv[1]` 必须命中 [`ALLOWED_COMMANDS`]，否则直接拒绝；
//! 2. 每个命令有自己的参数解析器，**只读它认识的 key**，多余的忽略；
//! 3. 绝不拼接命令行去调 `cmd.exe` / `powershell`——这是命令注入的经典入口。
//!
//! # 结果回传
//!
//! 退出码（粗粒度）+ 临时 JSON 文件（细粒度，含错误消息）。为什么不用 stdout：
//! 提权进程的 stdout 在主进程 `ShellExecuteW` 下拿不到（那是 `CreateProcess` 的
//! 能力）。JSON 文件路径由主进程通过 `--result` 传入，写失败时只靠退出码。
//!
//! # 为什么手写 FFI 不引 windows crate
//!
//! 只需要 `SRSetRestorePointW` 一个函数。引 windows crate 会让打包脚本里的
//! 「独立 target-dir 编译」多花几分钟（要编上千个模块），而手写声明只有几十行。

use std::io::Write;

// ===================== 退出码约定 =====================
//
// 主 exe（`engine::elevation`）靠这些值区分失败类型，**改动要同步两侧**。

/// 执行成功。
const EXIT_OK: i32 = 0;
/// 参数缺失 / 格式错误（例如没给 `--result`）。
const EXIT_BAD_USAGE: i32 = 2;
/// 命令不在白名单（**安全事件**，主进程应记日志）。
const EXIT_UNKNOWN_CMD: i32 = 3;
/// 命令本身执行失败（例如系统还原未开启）。
const EXIT_EXEC_FAILED: i32 = 4;

/// 命令白名单。**只有这里列出的才能执行。**
///
/// 后续 M1 的 WinSxS / 注册表任务会往这里加 `dism_cleanup` / `reg_export`，
/// 但每次新增都必须同时更新 `engine/src/elevation/mod.rs` 里的同名常量与单测。
const ALLOWED_COMMANDS: &[&str] = &["create_restore_point"];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = run(&args);
    std::process::exit(code);
}

fn run(args: &[String]) -> i32 {
    // 1. 至少要有一个命令名。
    let Some(cmd) = args.first() else {
        eprintln!("[elevation] 缺少命令。可用命令: {ALLOWED_COMMANDS:?}");
        return EXIT_BAD_USAGE;
    };

    // 2. 白名单校验（**在任何解析之前**，未知命令连参数都不看）。
    if !ALLOWED_COMMANDS.contains(&cmd.as_str()) {
        eprintln!("[elevation] 拒绝未知命令: {cmd}");
        return EXIT_UNKNOWN_CMD;
    }

    // 3. 取结果文件路径（所有命令都要）。
    let Some(result_path) = take_flag(&args[1..], "--result") else {
        eprintln!("[elevation] 缺少 --result <path>");
        return EXIT_BAD_USAGE;
    };

    // 4. 分发。
    match cmd.as_str() {
        "create_restore_point" => {
            let desc = take_flag(&args[1..], "--description")
                .unwrap_or_else(|| "C-clean cleanup".to_string());
            match restore::create_restore_point(&desc) {
                Ok(seq) => {
                    write_result(&result_path, true, cmd, "系统还原点已创建", Some(seq));
                    EXIT_OK
                }
                Err(e) => {
                    write_result(&result_path, false, cmd, &e, None);
                    eprintln!("[elevation] create_restore_point 失败: {e}");
                    EXIT_EXEC_FAILED
                }
            }
        }
        // 白名单已过滤，走到这里是 ALLOWED 与 match 不同步的 bug。
        other => {
            eprintln!("[elevation] 白名单与分发不一致: {other}");
            EXIT_UNKNOWN_CMD
        }
    }
}

/// 从参数里取 `--key value` 的 value（找不到返回 None）。
///
/// 手写而不是引 clap：只有两个参数、且要保证「多余参数一律忽略」。
fn take_flag(args: &[String], key: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == key {
            return it.next().cloned();
        }
    }
    None
}

/// 写结果 JSON。**写失败只打日志**——不能因为写不了结果就改变退出码，
/// 否则主进程会看到「成功」变「失败」。
fn write_result(path: &str, ok: bool, cmd: &str, message: &str, seq: Option<i64>) {
    let seq_json = match seq {
        Some(s) => s.to_string(),
        None => "null".to_string(),
    };
    let json = format!(
        r#"{{"ok":{},"cmd":"{}","message":"{}","sequenceNumber":{}}}"#,
        ok,
        json_escape(cmd),
        json_escape(message),
        seq_json
    );
    match std::fs::File::create(path).and_then(|mut f| f.write_all(json.as_bytes())) {
        Ok(()) => {}
        Err(e) => eprintln!("[elevation] 写结果文件失败 {path}: {e}"),
    }
}

/// 最小 JSON 字符串转义（只处理必须转义的字符）。
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

// ===================== 系统还原点（Win32 FFI） =====================

#[cfg(windows)]
mod restore {
    /// `BEGIN_SYSTEM_CHANGE`：标记一次系统变更开始，**系统在这一步创建还原点**。
    const BEGIN_SYSTEM_CHANGE: u32 = 100;
    /// `END_SYSTEM_CHANGE`：标记变更结束（与 BEGIN 配对）。
    const END_SYSTEM_CHANGE: u32 = 101;
    /// `MODIFY_SETTINGS`：变更类型 = 修改系统设置（清理临时文件属于这一类，
    /// 比 `APPLICATION_INSTALL` 更贴切，还原点管理器里显示也更准确）。
    const MODIFY_SETTINGS: u32 = 12;

    /// `RESTOREPOINTINFOW.szDescription` 的最大长度（含结尾 NUL）。
    const MAX_DESC: usize = 256;

    #[repr(C)]
    struct RestorePointInfoW {
        event_type: u32,
        restore_pt_type: u32,
        sequence_number: i64,
        description: [u16; MAX_DESC],
    }

    #[repr(C)]
    struct StateMgrStatus {
        status: u32,
        sequence_number: i64,
    }

    // SRSetRestorePointW 在 sfc.dll（不是 advapi32，这点常被写错）。
    #[link(name = "sfc")]
    extern "system" {
        fn SRSetRestorePointW(
            restore_pt_spec: *const RestorePointInfoW,
            smgr_status: *mut StateMgrStatus,
        ) -> i32;
    }

    /// 创建一个系统还原点，返回系统分配的序列号。
    ///
    /// 实现是「BEGIN + 立刻 END」的配对调用：BEGIN 触发建点，END 关闭变更期，
    /// 避免系统一直认为「有变更未结束」（会影响后续还原点合并）。
    ///
    /// # 失败原因（都会作为 Err 返回，由上层决定「是否继续清理」）
    ///
    /// - **系统还原被关闭**：最常见的失败。`SRSetRestorePointW` 返回 0，
    ///   `status.status` 给出错误码（如 `ERROR_SERVICE_DISABLED`）；
    /// - 不是管理员：调用本身会失败（正常路径下我们已提权，走到这里说明提权失效）；
    /// - 磁盘空间不足 / 组策略禁用。
    pub fn create_restore_point(description: &str) -> Result<i64, String> {
        // 描述截断到 255 个 UTF-16 码元（留 1 个给 NUL）。超长会被系统拒绝。
        let mut buf = [0u16; MAX_DESC];
        for (i, u) in description.encode_utf16().take(MAX_DESC - 1).enumerate() {
            buf[i] = u;
        }

        let mut info = RestorePointInfoW {
            event_type: BEGIN_SYSTEM_CHANGE,
            restore_pt_type: MODIFY_SETTINGS,
            sequence_number: 0,
            description: buf,
        };
        let mut status = StateMgrStatus {
            status: 0,
            sequence_number: 0,
        };

        // SAFETY: 两个指针都指向本函数栈上、已完整初始化的 repr(C) 结构体；
        // SRSetRestorePointW 只读第一个、只写第二个，生命周期覆盖整个调用。
        let ok = unsafe { SRSetRestorePointW(&info, &mut status) };
        if ok == 0 {
            return Err(format!(
                "SRSetRestorePointW(BEGIN) 失败，系统状态码 {:#x}（常见原因：系统还原未开启）",
                status.status
            ));
        }

        // 配对 END：把刚建的点的 sequence number 回填，标记变更结束。
        let seq = status.sequence_number;
        info.event_type = END_SYSTEM_CHANGE;
        info.sequence_number = seq;
        // SAFETY: 同上，结构体仍在本栈帧内有效。
        let ok_end = unsafe { SRSetRestorePointW(&info, &mut status) };
        if ok_end == 0 {
            // 还原点已经建出来了（BEGIN 成功），END 失败不应让上层认为「没建成」。
            eprintln!(
                "[elevation] SRSetRestorePointW(END) 失败，状态码 {:#x}（还原点已创建）",
                status.status
            );
        }

        Ok(seq)
    }
}

#[cfg(not(windows))]
mod restore {
    pub fn create_restore_point(_description: &str) -> Result<i64, String> {
        Err("系统还原点仅支持 Windows".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitelist_rejects_unknown_command() {
        // 未知命令：即使给了 --result 也必须被拒，且退出码是 EXIT_UNKNOWN_CMD
        let args = vec![
            "format_disk".to_string(),
            "--result".to_string(),
            "x.json".to_string(),
        ];
        assert_eq!(run(&args), EXIT_UNKNOWN_CMD, "未知命令必须拒绝");
    }

    #[test]
    fn missing_command_is_bad_usage() {
        assert_eq!(run(&[]), EXIT_BAD_USAGE);
    }

    #[test]
    fn known_command_without_result_is_bad_usage() {
        let args = vec!["create_restore_point".to_string()];
        assert_eq!(run(&args), EXIT_BAD_USAGE, "缺 --result 应报用法错误");
    }

    #[test]
    fn take_flag_reads_value_and_ignores_extras() {
        let args: Vec<String> = [
            "--description",
            "abc",
            "--extra",
            "zzz",
            "--result",
            "r.json",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(take_flag(&args, "--description").as_deref(), Some("abc"));
        assert_eq!(take_flag(&args, "--result").as_deref(), Some("r.json"));
        assert_eq!(take_flag(&args, "--missing"), None);
        // key 在末尾但没有 value → None（不能 panic）
        let dangling: Vec<String> = vec!["--result".to_string()];
        assert_eq!(take_flag(&dangling, "--result"), None);
    }

    #[test]
    fn json_escape_handles_quotes_and_control_chars() {
        assert_eq!(json_escape(r#"a"b"#), r#"a\"b"#);
        assert_eq!(json_escape("a\\b"), r#"a\\b"#);
        assert_eq!(json_escape("a\nb"), r#"a\nb"#);
        assert_eq!(json_escape("a\u{1}b"), r#"a\u0001b"#);
        // 中文原样保留（不转义成 \uXXXX）
        assert_eq!(json_escape("清理"), "清理");
    }

    #[test]
    fn result_json_has_expected_shape() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("r.json");
        write_result(
            p.to_str().unwrap(),
            true,
            "create_restore_point",
            "ok",
            Some(42),
        );
        let s = std::fs::read_to_string(&p).unwrap();
        assert_eq!(
            s,
            r#"{"ok":true,"cmd":"create_restore_point","message":"ok","sequenceNumber":42}"#
        );
    }

    #[test]
    fn result_json_null_sequence_when_none() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("r.json");
        write_result(p.to_str().unwrap(), false, "x", "bad", None);
        let s = std::fs::read_to_string(&p).unwrap();
        assert!(s.contains(r#""sequenceNumber":null"#), "实际: {s}");
        assert!(s.contains(r#""ok":false"#));
    }
}
