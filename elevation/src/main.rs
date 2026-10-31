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

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

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
/// task-065 加了 `dism_analyze` / `dism_cleanup`。这两个命令的 DISM 参数
/// **完全硬编码**（见 [`dism`] 模块）——不是「解析参数后拒绝危险值」，
/// 而是「根本没有传参入口」。`/ResetBase` 从结构上就不可能被送进 DISM。
const ALLOWED_COMMANDS: &[&str] = &["create_restore_point", "dism_analyze", "dism_cleanup"];

/// DISM 超时上限（15 分钟）。
///
/// `/StartComponentCleanup` 在机械盘 + 超大组件存储上能跑十几分钟；
/// `/AnalyzeComponentStore` 遇到另一个 DISM 实例持锁时会**挂住等**。
/// `Command::output()` 会无限等下去，而提权进程挂着不放会把 UAC 令牌一直占住，
/// 所以必须自己轮询 + 到点 kill。
const DISM_TIMEOUT: Duration = Duration::from_secs(15 * 60);

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
        // task-065：两条 DISM 命令。参数表见 [`dism::Action::args`]——**常量**，
        // 与 argv 完全无关，`/ResetBase` 没有任何入口能拼进去。
        "dism_analyze" => run_dism_cmd(cmd, &args[1..], &result_path, dism::Action::Analyze),
        "dism_cleanup" => run_dism_cmd(cmd, &args[1..], &result_path, dism::Action::Cleanup),
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
    write_json(path, &json);
}

/// 写「带原始输出」的结果 JSON（task-065，DISM 命令专用）。
///
/// 为什么单独一个函数而不是给 [`write_result`] 加参数：还原点命令的 JSON shape
/// 已有单测钉住（`result_json_has_expected_shape`），加个 `output` 字段会让它变。
/// 两种 shape 各自稳定，比一个「有时多一个字段」的结构更好断言。
///
/// `output` 是 DISM 的完整 stdout+stderr（OEM 解码后）。解析在 engine 侧做——
/// 提权 exe 只负责「跑 + 原样带回」，**不解释语义**，这样解析逻辑可单测、
/// 也能随 DISM 输出格式变化单独迭代。
fn write_output_result(path: &str, ok: bool, cmd: &str, message: &str, output: &str) {
    let json = format!(
        r#"{{"ok":{},"cmd":"{}","message":"{}","sequenceNumber":null,"output":"{}"}}"#,
        ok,
        json_escape(cmd),
        json_escape(message),
        json_escape(output)
    );
    write_json(path, &json);
}

fn write_json(path: &str, json: &str) {
    match std::fs::File::create(path).and_then(|mut f| f.write_all(json.as_bytes())) {
        Ok(()) => {}
        Err(e) => eprintln!("[elevation] 写结果文件失败 {path}: {e}"),
    }
}

/// 跑一条 DISM 命令并把原始输出带回（task-065）。
///
/// `--progress <path>` 可选：给了就把 DISM 的完整行实时追加进去，主进程侧轮询
/// 该文件做进度转发（提权进程的 stdout 在主进程 `ShellExecuteExW` 下拿不到）。
fn run_dism_cmd(cmd: &str, args: &[String], result_path: &str, action: dism::Action) -> i32 {
    let progress = take_flag(args, "--progress");
    match dism::run(action, progress.as_deref()) {
        Ok(out) => {
            let ok = out.exit_code == 0;
            let message = if ok {
                "DISM 执行成功".to_string()
            } else {
                format!("DISM 退出码 {}", out.exit_code)
            };
            write_output_result(result_path, ok, cmd, &message, &out.text);
            if ok {
                EXIT_OK
            } else {
                eprintln!("[elevation] {cmd} 失败: {message}");
                EXIT_EXEC_FAILED
            }
        }
        Err(e) => {
            write_output_result(result_path, false, cmd, &e, "");
            eprintln!("[elevation] {cmd} 失败: {e}");
            EXIT_EXEC_FAILED
        }
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

// ===================== DISM（task-065） =====================
//
// # 为什么 `/ResetBase` 是从结构上不可能，而不是「被检查后拒绝」
//
// 任务书写的是「拒绝 `/ResetBase` 参数（硬编码）」。如果按字面实现成「解析参数
// → 发现危险值就报错」，那这个检查本身就是新的攻击面（大小写变形、`/resetbase`
// 与 `/ResetBase` 同义、参数被拆成两段……）。本模块的做法更彻底：
//
// - [`dism::Action`] 只有 `Analyze` / `Cleanup` 两个变体；
// - [`dism::Action::args`] 返回 `&'static [&'static str]`，是**编译期常量**；
// - 整个调用链上**没有任何 `&str` 能流进 DISM 的命令行**。
//
// 也就是「没有传参入口」而非「有入口但拦住」。`/ResetBase` 拼不进去，因为它
// 没有地方可拼。
//
// # 为什么不用 XML
//
// 任务书说「解析 DISM 输出 XML」。**`/analyzecomponentstore` 没有 XML 输出**
// （实测：`/Format:XML` 不被接受，只有本地化文本）。所以只能解析文本，并且
// 必须加 `/English` 把标签锁成英文——否则中文系统上标签是「实际大小」，
// 按英文关键字解析必然全空。
//
// # 为什么要自己解码字节
//
// DISM 是控制台程序，写管道时用 **OEM 代码页**（简中系统是 CP936/GBK），
// 直接 `String::from_utf8_lossy` 会把中文路径变成一串 `�`。走
// `MultiByteToWideChar(CP_OEMCP)` 手动解码（与 [`restore`] 一样手写 FFI）。

mod dism {
    use super::{decode_oem, Command, Duration, Instant, Read, Stdio, Write, DISM_TIMEOUT};

    /// 允许执行的 DISM 动作。**只有这两个**——没有「自定义参数」变体。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Action {
        /// `dism /online /cleanup-image /analyzecomponentstore`
        Analyze,
        /// `dism /online /cleanup-image /startcomponentcleanup`
        Cleanup,
    }

    impl Action {
        /// 硬编码的 DISM 参数表。
        ///
        /// `/English` 是 DISM 全局选项，把输出固定为英文——engine 侧按英文标签
        /// 解析，不锁语言在中文系统上会全空。
        ///
        /// 注意**没有** `/ResetBase`：那是「清理后已安装更新无法卸载回退」的
        /// 不可逆开关，产品上禁用。
        pub fn args(self) -> &'static [&'static str] {
            match self {
                Action::Analyze => &[
                    "/online",
                    "/cleanup-image",
                    "/analyzecomponentstore",
                    "/English",
                ],
                Action::Cleanup => &[
                    "/online",
                    "/cleanup-image",
                    "/startcomponentcleanup",
                    "/English",
                ],
            }
        }
    }

    /// DISM 执行结果（退出码 + 原始输出）。
    pub struct Output {
        pub exit_code: i32,
        pub text: String,
    }

    /// DISM 可执行文件路径。
    ///
    /// **不走 PATH**：本进程是提权的，PATH 里若被塞了同名 `Dism.exe`，等于把
    /// 管理员权限直接交给它。固定从 `%SystemRoot%\System32` 取。
    fn dism_path() -> std::path::PathBuf {
        let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
        std::path::Path::new(&root)
            .join("System32")
            .join("Dism.exe")
    }

    /// 执行一个 DISM 动作。
    ///
    /// `progress` 给出时，DISM 的每个完整行会实时追加到该文件并 flush——
    /// 主进程侧靠轮询这个文件做进度转发。
    pub fn run(action: Action, progress: Option<&str>) -> Result<Output, String> {
        let progress_file = match progress {
            Some(p) => {
                Some(std::fs::File::create(p).map_err(|e| format!("创建进度文件失败 {p}: {e}"))?)
            }
            None => None,
        };

        let mut cmd = Command::new(dism_path());
        cmd.args(action.args())
            // 不读 stdin：DISM 偶发会等确认，给它 EOF 立刻退出，别挂死。
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            /// 不弹黑框（提权进程本身无窗口）。
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("启动 DISM 失败（{}）：{e}", dism_path().display()))?;

        let stdout = child.stdout.take().ok_or("DISM stdout 不可用")?;
        let stderr = child.stderr.take().ok_or("DISM stderr 不可用")?;

        // 两个读线程：管道缓冲区（约 4KB）写满后 DISM 会阻塞在 write 上，
        // 主线程若还在轮询 try_wait 就永远等不到退出 —— 必须并行抽干。
        let out_handle = std::thread::spawn(move || read_stream(stdout, progress_file));
        let err_handle = std::thread::spawn(move || read_stream(stderr, None));

        let deadline = Instant::now() + DISM_TIMEOUT;
        let status = loop {
            match child.try_wait() {
                Ok(Some(s)) => break s,
                Ok(None) => {
                    if Instant::now() >= deadline {
                        // 超时：kill + 回收，不能让提权进程挂着（UAC 令牌一直被占）。
                        let _ = child.kill();
                        let _ = child.wait();
                        let _ = out_handle.join();
                        let _ = err_handle.join();
                        return Err(format!(
                            "DISM 超时（{} 分钟未结束），已终止",
                            DISM_TIMEOUT.as_secs() / 60
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(200));
                }
                Err(e) => {
                    let _ = child.kill();
                    return Err(format!("等待 DISM 失败: {e}"));
                }
            }
        };

        let stdout_text = out_handle.join().unwrap_or_default();
        let stderr_text = err_handle.join().unwrap_or_default();
        let exit_code = status.code().unwrap_or(-1);

        // stderr 合并进 text：DISM 的失败原因常常只在 stderr（「错误: 740」）。
        let mut text = stdout_text;
        if !stderr_text.trim().is_empty() {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(&stderr_text);
        }
        Ok(Output { exit_code, text })
    }

    /// 抽干一个管道：返回解码后的全文；`sink` 存在时同时把**完整行**写进去。
    ///
    /// 按 `\r` 和 `\n` 双分隔：DISM 的进度条用 `\r` 原地刷新（
    /// `[=====100.0%=====]`），只有段落文字才用 `\n`。只认 `\n` 的话，进度条
    /// 那一行会攒到最后才吐出来，进度就变成「假实时」。
    fn read_stream<R: Read + Send + 'static>(mut r: R, mut sink: Option<std::fs::File>) -> String {
        let mut raw: Vec<u8> = Vec::new();
        let mut buf = [0u8; 8192];
        let mut line: Vec<u8> = Vec::new();

        loop {
            let n = match r.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };
            raw.extend_from_slice(&buf[..n]);

            if let Some(f) = sink.as_mut() {
                for &b in &buf[..n] {
                    if b == b'\n' || b == b'\r' {
                        if !line.is_empty() {
                            let _ = f.write_all(decode_oem(&line).as_bytes());
                            let _ = f.write_all(b"\n");
                            // 每行都 flush：进度文件的价值就在于「实时」。
                            let _ = f.flush();
                            line.clear();
                        }
                    } else {
                        line.push(b);
                    }
                }
            }
        }

        // 收尾：最后一行没有换行符也要吐出来。
        if let Some(f) = sink.as_mut() {
            if !line.is_empty() {
                let _ = f.write_all(decode_oem(&line).as_bytes());
                let _ = f.write_all(b"\n");
                let _ = f.flush();
            }
        }

        decode_oem(&raw)
    }
}

/// OEM 代码页字节 → UTF-8 字符串。
///
/// 控制台程序（DISM）写管道时用 OEM 代码页而不是 UTF-8：简中系统是 CP936，
/// 直接当 UTF-8 解会把中文路径变成 `�`。用 `MultiByteToWideChar(CP_OEMCP)` 转。
#[cfg(windows)]
fn decode_oem(bytes: &[u8]) -> String {
    /// `CP_OEMCP`：当前系统 OEM 代码页。
    const CP_OEMCP: u32 = 1;

    #[link(name = "kernel32")]
    extern "system" {
        fn MultiByteToWideChar(
            code_page: u32,
            flags: u32,
            s: *const u8,
            cb_multi_byte: i32,
            out: *mut u16,
            cch_wide_char: i32,
        ) -> i32;
    }

    if bytes.is_empty() {
        return String::new();
    }
    let cb = bytes.len() as i32;
    // SAFETY: 指针与长度来自同一 slice；out 传 null 是「只问所需长度」的用法。
    let need =
        unsafe { MultiByteToWideChar(CP_OEMCP, 0, bytes.as_ptr(), cb, std::ptr::null_mut(), 0) };
    if need <= 0 {
        // 解码失败（非法字节序列）→ 退回 lossy，至少别丢内容。
        return String::from_utf8_lossy(bytes).to_string();
    }
    let mut buf = vec![0u16; need as usize];
    // SAFETY: buf 容量 = need 个 u16，正是上一次调用要求的长度。
    let got =
        unsafe { MultiByteToWideChar(CP_OEMCP, 0, bytes.as_ptr(), cb, buf.as_mut_ptr(), need) };
    if got <= 0 {
        return String::from_utf8_lossy(bytes).to_string();
    }
    buf.truncate(got as usize);
    String::from_utf16_lossy(&buf)
}

#[cfg(not(windows))]
fn decode_oem(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
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

    // ============ task-065：DISM 命令白名单与 /ResetBase ============

    #[test]
    fn dism_commands_pass_whitelist() {
        // 关键区分：返回 BAD_USAGE（缺 --result）说明**已经通过白名单校验**，
        // 而不是 UNKNOWN_CMD。白名单漏加会让这里变成 EXIT_UNKNOWN_CMD。
        for cmd in ["dism_analyze", "dism_cleanup"] {
            assert_eq!(
                run(&[cmd.to_string()]),
                EXIT_BAD_USAGE,
                "{cmd} 必须在白名单内（应报「缺 --result」而非「未知命令」）"
            );
        }
    }

    #[test]
    fn dism_action_args_never_contain_resetbase() {
        // `/ResetBase` 会让「已安装更新无法卸载回退」，产品上禁用。
        // 参数表是 &'static 常量，这里钉住它不含该开关。
        for action in [dism::Action::Analyze, dism::Action::Cleanup] {
            for a in action.args() {
                assert!(
                    !a.to_ascii_lowercase().contains("resetbase"),
                    "{action:?} 的参数里出现了 /ResetBase：{a}"
                );
            }
        }
        // 顺带钉住「清理用的不是 /ResetBase 而是 /StartComponentCleanup」
        assert!(dism::Action::Cleanup
            .args()
            .contains(&"/startcomponentcleanup"));
        assert!(dism::Action::Analyze
            .args()
            .contains(&"/analyzecomponentstore"));
        // 两条命令都必须带 /English（engine 按英文标签解析，不锁语言会全空）
        for action in [dism::Action::Analyze, dism::Action::Cleanup] {
            assert!(
                action.args().contains(&"/English"),
                "{action:?} 缺 /English"
            );
        }
    }

    #[test]
    fn extra_argv_cannot_inject_dism_flags() {
        // 用户在命令行里塞 `/ResetBase`：`take_flag` 只认已知 key，其余一律丢弃；
        // DISM 参数表来自 `Action::args()`（常量），与 argv 无任何关联。
        let args: Vec<String> = ["--result", "x.json", "/ResetBase"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(take_flag(&args, "--result").as_deref(), Some("x.json"));
        assert_eq!(
            take_flag(&args, "--progress"),
            None,
            "没有 --progress 就不该有进度文件"
        );
        // 参数表与 argv 完全无关：这是「无传参入口」的可执行证明
        assert!(!dism::Action::Cleanup
            .args()
            .iter()
            .any(|a| a.to_ascii_lowercase().contains("resetbase")));
    }

    #[test]
    fn output_result_json_carries_raw_dism_output() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("d.json");
        write_output_result(
            p.to_str().unwrap(),
            true,
            "dism_analyze",
            "DISM 执行成功",
            "Actual Size of Component Store : 7.36 GB",
        );
        let s = std::fs::read_to_string(&p).unwrap();
        // 原有四个字段保持兼容（engine 的 ElevatedResult 照旧能解析）
        assert!(s.contains(r#""ok":true"#), "实际: {s}");
        assert!(s.contains(r#""cmd":"dism_analyze""#));
        assert!(s.contains(r#""sequenceNumber":null"#));
        // 新增 output 字段，原文一字不改地带着
        assert!(
            s.contains(r#""output":"Actual Size of Component Store : 7.36 GB""#),
            "实际: {s}"
        );
    }

    #[test]
    fn output_result_escapes_quotes_and_newlines() {
        // DISM 输出里一定有换行；不转义会写出非法 JSON，engine 侧解析直接失败。
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("d2.json");
        write_output_result(
            p.to_str().unwrap(),
            false,
            "dism_cleanup",
            "错误: 740",
            "line1\nline2",
        );
        let s = std::fs::read_to_string(&p).unwrap();
        assert!(s.contains(r"line1\nline2"), "换行必须转义成 \\n，实际: {s}");
        assert!(!s.contains("line1\nline2"), "不能出现裸换行");
    }

    #[test]
    fn decode_oem_roundtrips_ascii() {
        // 只断言 ASCII：CI 是 en-US（CP_OEMCP=437），GBK 字节在那边解不出来，
        // 钉死中文会让 windows-latest 挂掉。中文场景由真机冒烟覆盖。
        assert_eq!(decode_oem(b""), "");
        assert_eq!(
            decode_oem(b"Component Store Cleanup Recommended : No"),
            "Component Store Cleanup Recommended : No"
        );
    }
}
