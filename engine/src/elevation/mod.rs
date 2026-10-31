//! 提权 exe 的主进程侧封装（task-062）。
//!
//! 职责边界：
//! - **本模块只负责「拉起 + 等待 + 读结果」**，不关心提权 exe 内部做什么；
//! - 提权 exe 的路径由调用方传入（安装态是 `resource_dir()/cclean-elevation.exe`，
//!   dev 是 `target/{debug,release}/cclean-elevation.exe`）——engine 不认识 Tauri；
//! - 命令白名单在**两侧各校验一次**：本模块拒绝未知命令（早失败、不弹 UAC），
//!   提权 exe 也拒绝（防止有人绕过主进程直接调 exe）。
//!
//! # 为什么用 `ShellExecuteExW` 而不是 `ShellExecuteW`
//!
//! 任务书写的是 `ShellExecuteW("runas")`，但 `ShellExecuteW` **不返回进程句柄**，
//! 拿不到退出码、也没法等它结束。要「等 + 取退出码」必须用 `ShellExecuteExW` +
//! `SEE_MASK_NOCLOSEPROCESS`。
//!
//! # 结果回传的两条路
//!
//! | 通道 | 内容 | 用途 |
//! |---|---|---|
//! | 退出码 | 0/2/3/4 | 粗粒度分类，即使 JSON 写失败也能判断 |
//! | 临时 JSON | `{ok, cmd, message, sequenceNumber}` | 细粒度，带错误原文 |
//!
//! JSON 路径由本模块用 `--result` 传给提权 exe，读完即删。

use std::path::Path;

/// 命令白名单。**必须与 `elevation/src/main.rs` 的 `ALLOWED_COMMANDS` 完全一致**。
///
/// 两侧各有一份而不是共享常量：提权 exe 刻意不依赖 engine（零依赖、编译秒级），
/// 共享要引一个 common crate，收益不抵复杂度。代价是「改一处要改两处」，
/// 所以 `engine/tests/elevation_whitelist.rs` 直接 `include_str!` 读提权 exe 源码
/// 做一致性断言——漏同步会在 CI 里红，不会拖到运行期才发现「命令被静默拒绝」。
///
/// task-065：加 `dism_analyze` / `dism_cleanup`。注意**这两条命令没有参数通道**
/// （DISM 参数在提权 exe 里是硬编码常量），`/ResetBase` 无法传入。
pub const ALLOWED_COMMANDS: &[&str] = &["create_restore_point", "dism_analyze", "dism_cleanup"];

/// 提权 exe 的退出码约定（与 `elevation/src/main.rs` 同步）。
pub mod exit_codes {
    pub const OK: i32 = 0;
    pub const BAD_USAGE: i32 = 2;
    pub const UNKNOWN_CMD: i32 = 3;
    pub const EXEC_FAILED: i32 = 4;
}

/// 提权执行的结果。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElevatedResult {
    /// 命令是否成功。
    pub ok: bool,
    /// 提权 exe 的退出码。
    ///
    /// `serde(default)`：**提权 exe 写的 JSON 里没有这个字段**（它写的是
    /// `{ok, cmd, message, sequenceNumber}`），不加 default 会让整个解析失败。
    /// 真实值由 [`parse_result_json`] 从操作系统拿到的退出码覆盖。
    #[serde(default)]
    pub exit_code: i32,
    /// 人可读的说明（失败时是错误原因）。
    pub message: String,
    /// `create_restore_point` 专用：系统分配的还原点序列号。
    #[serde(default)]
    pub sequence_number: Option<i64>,
    /// `dism_*` 命令专用：DISM 的完整 stdout+stderr（OEM 解码后的原文）。
    ///
    /// 提权 exe 只负责「跑 + 原样带回」，**不解释语义**——解析在
    /// [`crate::scan::winsxs`] 里做，那样解析逻辑能单测，也能随 DISM
    /// 输出格式变化单独迭代，不用重新打包提权 exe。
    #[serde(default)]
    pub output: Option<String>,
}

impl ElevatedResult {
    fn from_exit_code(exit_code: i32, message: impl Into<String>) -> Self {
        Self {
            ok: exit_code == exit_codes::OK,
            exit_code,
            message: message.into(),
            sequence_number: None,
            output: None,
        }
    }
}

/// 校验命令是否在白名单内。
///
/// 抽成公开函数是为了让调用方（`executor` / 前端命令）能在**弹 UAC 之前**
/// 就拒绝掉错误输入——UAC 弹窗是打断性的，不该为了一个非法命令弹出来。
pub fn validate_command(cmd: &str) -> Result<(), String> {
    if ALLOWED_COMMANDS.contains(&cmd) {
        Ok(())
    } else {
        Err(format!(
            "命令 `{cmd}` 不在提权白名单内（允许：{ALLOWED_COMMANDS:?}）"
        ))
    }
}

/// 建系统还原点，成功后写 audit_log（task-062 验收 7/8/10）。
///
/// 为什么放在 engine 而不是 Tauri 层：要写 `audit_log` 就得拿到 `Connection`，
/// 而 `rusqlite::Connection` 不该跨 crate 出现在 app 侧的类型签名里。
///
/// 返回系统分配的 sequence number。**任何失败都返回 Err**，由调用方决定
/// 「是否让用户选择继续」——本函数不替用户做决定。
pub fn ensure_restore_point(conn: &rusqlite::Connection, exe_path: &Path) -> Result<i64, String> {
    let now = unix_secs();
    // task_id 传 0：真正的 cleanup_task 行要等 execute_items 才建，
    // 名称里的 `#0` 表示「本次清理（尚未分配 id）」。时间戳足以区分批次。
    let name = restore_point_name(0, now);

    let r = run_elevated(
        exe_path,
        "create_restore_point",
        &[("--description", name.as_str())],
    )?;

    if !r.ok {
        return Err(r.message);
    }
    let seq = r.sequence_number.unwrap_or(0);

    // 写失败只打日志——不能因为审计写不进去就否定「还原点已建成」这个事实。
    if let Err(e) = crate::execute::executor::log_restore_point(conn, &name, seq, now) {
        eprintln!("[ensure_restore_point] {e}");
    }

    Ok(seq)
}

fn unix_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 生成系统还原点的名称（task-062）。
/// 格式：`C-clean cleanup #<task_id> <YYYY-MM-DD HH:MM:SS>`
///
/// - 带 `#task_id`：用户事后在「系统还原」面板里能对上「这是哪一次清理建的」；
/// - 带时间：同一 task 理论上只会建一次，但重试时可能再建，时间用于区分；
/// - 时间用 **UTC**：不引 chrono（只为格式化引一个依赖不划算），手写转换在
///   [`format_utc`] 里。**已知代价**：比本地时间差几个时区，但作为「标记」够用
///   （用户在还原点面板看到的顺序仍是正确的）。
pub fn restore_point_name(task_id: i64, ts: i64) -> String {
    format!("C-clean cleanup #{task_id} {}", format_utc(ts))
}

/// unix 秒 → `YYYY-MM-DD HH:MM:SS`（UTC）。
///
/// 用 Howard Hinnant 的 `civil_from_days` 算法（公版、无依赖、无分支表）。
/// 抽成公开函数是为了单测能直接钉住格式。
pub fn format_utc(ts: i64) -> String {
    let days = ts.div_euclid(86_400);
    let secs_of_day = ts.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let hh = secs_of_day / 3_600;
    let mm = (secs_of_day % 3_600) / 60;
    let ss = secs_of_day % 60;
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}:{ss:02}")
}

/// 自 1970-01-01 起的天数 → (年, 月, 日)。
///
/// 算法来源：Howard Hinnant, "chrono-Compatible Low-Level Date Algorithms"。
/// 把 3 月当作一年的起点，避开闰日对后续月份的影响。
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]，3 月为 0
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 把 `(key, value)` 列表拼成提权 exe 的命令行参数串。
///
/// **每个 value 都用双引号包裹并转义内部引号**：值里可能带空格（例如还原点描述
/// `C-clean cleanup #12 2026-10-30 12:00`），不包引号会被拆成多个参数。
///
/// Windows 命令行里双引号的转义规则比较绕（`\"` 只在特定上下文有效）。这里用
/// 更稳的做法：**把 `"` 替换成空格**——还原点描述里出现双引号本就罕见，替换比
/// 拼错转义安全。值里的反斜杠不用处理（不在引号末尾时不需要转义）。
pub fn build_args_string(args: &[(&str, &str)]) -> String {
    let mut out = String::new();
    for (k, v) in args {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(k);
        out.push(' ');
        out.push('"');
        out.push_str(&v.replace('"', " "));
        out.push('"');
    }
    out
}

/// 解析提权 exe 写的结果 JSON。
///
/// 解析失败不算致命：说明提权 exe 可能没跑起来（UAC 被拒 / 崩溃），此时靠退出码
/// 兜底，message 用原始文本（便于排查）。
pub fn parse_result_json(raw: &str, exit_code: i32) -> ElevatedResult {
    match serde_json::from_str::<ElevatedResult>(raw) {
        Ok(mut r) => {
            // 以退出码为准：JSON 里的 ok 可能与退出码不一致（例如被篡改），
            // 退出码来自操作系统，更可信。
            r.exit_code = exit_code;
            r.ok = exit_code == exit_codes::OK;
            r
        }
        Err(_) => ElevatedResult::from_exit_code(
            exit_code,
            if raw.trim().is_empty() {
                format!("提权 exe 未返回结果（退出码 {exit_code}）")
            } else {
                format!("提权 exe 返回了无法解析的结果（退出码 {exit_code}）：{raw}")
            },
        ),
    }
}

/// 拉起提权 exe 执行一条命令，等它结束并读回结果。
///
/// # 参数
///
/// - `exe_path`：提权 exe 的绝对路径（调用方负责定位）；
/// - `cmd`：必须在 [`ALLOWED_COMMANDS`] 内；
/// - `args`：命令参数（不含 `--result`，本函数自动加）。
///
/// # 错误
///
/// - 命令不在白名单 → 立刻 Err，**不弹 UAC**；
/// - 用户在 UAC 弹窗点「否」→ Err，message 含「用户取消」；
/// - 提权 exe 不存在 → Err。
///
/// 注意：**命令本身执行失败不是 Err**，而是 `Ok(ElevatedResult { ok: false, .. })`——
/// 调用方要靠这个区分「没能执行」和「执行了但失败」（后者要弹「是否继续清理」）。
pub fn run_elevated(
    exe_path: &Path,
    cmd: &str,
    args: &[(&str, &str)],
) -> Result<ElevatedResult, String> {
    validate_command(cmd)?;

    if !exe_path.exists() {
        return Err(format!("提权 exe 不存在: {}", exe_path.display()));
    }

    // 结果文件放在系统临时目录：提权进程在另一个用户上下文里也能写（Temp 对
    // 当前用户可写；提权后仍是同一用户、只是令牌提权）。
    let result_path = std::env::temp_dir().join(format!(
        "cclean-elev-{}-{}.json",
        std::process::id(),
        unix_millis()
    ));
    // 先删掉可能存在的残留，避免读到上一次的结果。
    let _ = std::fs::remove_file(&result_path);

    let mut all_args: Vec<(&str, &str)> = args.to_vec();
    let result_str = result_path.to_string_lossy().to_string();
    all_args.push(("--result", result_str.as_str()));
    let args_string = build_args_string(&all_args);

    let exit_code = shell_execute_and_wait(exe_path, &args_string)?;

    // 读结果（可能不存在：UAC 被拒 / 提权 exe 崩溃）。
    let raw = std::fs::read_to_string(&result_path).unwrap_or_default();
    let _ = std::fs::remove_file(&result_path);

    Ok(parse_result_json(&raw, exit_code))
}

fn unix_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// 从累计字节里切出「新增的完整行」，返回 `(行列表, 新游标)`。
///
/// 为什么按**字节**而不是 `&str`：进度文件是提权进程边跑边写的，读到的那一刻
/// 末尾可能是**半个多字节字符**。先 `from_utf8_lossy` 会插入 U+FFFD 改变字节
/// 长度，游标算术随之失效（下次切片位置全错）。所以只在「最后一个 `\n` 之前」
/// 这段确定完整的字节上做解码。
///
/// 空行会被丢掉：DISM 输出里空行比正文还多，转发给 UI 只是噪音。
pub fn drain_complete_lines(content: &[u8], cursor: usize) -> (Vec<String>, usize) {
    let start = cursor.min(content.len());
    let slice = &content[start..];
    let Some(pos) = slice.iter().rposition(|&b| b == b'\n') else {
        // 还没有完整的行（或者一个字节都没新增）
        return (Vec::new(), start);
    };
    let text = String::from_utf8_lossy(&slice[..pos]);
    let lines: Vec<String> = text
        .split('\n')
        // Windows 换行：`\r` 已经不算分隔符（我们只按 `\n` 切），这里剥掉
        .map(|l| l.trim_end_matches('\r').to_string())
        .filter(|l| !l.trim().is_empty())
        .collect();
    (lines, start + pos + 1)
}

/// 轮询进度文件，把新增的完整行通过 `on_line(序号, 行内容)` 抛出去。
///
/// 为什么轮询文件而不是管道：提权进程的 stdout 在主进程 `ShellExecuteExW` 下
/// 拿不到（那是 `CreateProcess` 的能力）。提权 exe 把 DISM 的输出按行落盘，
/// 主进程侧 tail 这个文件，就是「跨提权边界转发进度」的最简可靠做法。
fn tail_progress(
    path: &Path,
    stop: &std::sync::atomic::AtomicBool,
    on_line: &(dyn Fn(u64, &str) + Sync),
) {
    use std::sync::atomic::Ordering;

    let mut cursor = 0usize;
    let mut seq = 0u64;

    loop {
        // 先看停止标志、再读一次：退出前保证「最后写进去的行」也被读出来，
        // 否则最后一屏 DISM 输出（含成功/失败结论）会丢。
        let finished = stop.load(Ordering::Relaxed);

        if let Ok(bytes) = std::fs::read(path) {
            // 文件被重建（提权 exe 用 File::create 截断）：游标必须归零，
            // 否则会一直停在「越界」状态再也读不到东西。
            if bytes.len() < cursor {
                cursor = 0;
            }
            let (lines, next) = drain_complete_lines(&bytes, cursor);
            cursor = next;
            for line in lines {
                on_line(seq, &line);
                seq += 1;
            }
        }

        if finished {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

/// 拉起提权 exe 执行命令，**同时**把它的进度实时转发出去。
///
/// 与 [`run_elevated`] 的区别只有一个：多传一个 `--progress <path>`，
/// 并在等待期间开一个 tail 线程把进度文件的新增行抛给 `on_line`。
///
/// `on_line` 在**后台线程**里被调用，所以要求 `Sync`；它必须自己保证线程安全
/// （通常就是 emit 一个 Tauri 事件 / 往 `Arc<Mutex<Vec>>` 里 push）。
///
/// 用 `std::thread::scope` 而不是 `'static` 线程：`on_line` 常常借用调用方的
/// 局部变量（比如 `&AppHandle`），要求 `'static` 会逼调用方去 `clone` 一堆东西。
pub fn run_elevated_streaming(
    exe_path: &Path,
    cmd: &str,
    args: &[(&str, &str)],
    on_line: &(dyn Fn(u64, &str) + Sync),
) -> Result<ElevatedResult, String> {
    validate_command(cmd)?;

    if !exe_path.exists() {
        return Err(format!("提权 exe 不存在: {}", exe_path.display()));
    }

    let stamp = format!("{}-{}", std::process::id(), unix_millis());
    let result_path = std::env::temp_dir().join(format!("cclean-elev-{stamp}.json"));
    let progress_path = std::env::temp_dir().join(format!("cclean-progress-{stamp}.log"));
    let _ = std::fs::remove_file(&result_path);
    let _ = std::fs::remove_file(&progress_path);

    let mut all_args: Vec<(&str, &str)> = args.to_vec();
    let progress_str = progress_path.to_string_lossy().to_string();
    let result_str = result_path.to_string_lossy().to_string();
    all_args.push(("--progress", progress_str.as_str()));
    all_args.push(("--result", result_str.as_str()));
    let args_string = build_args_string(&all_args);

    let stop = std::sync::atomic::AtomicBool::new(false);
    let exit = std::thread::scope(|s| {
        let tail = s.spawn(|| tail_progress(&progress_path, &stop, on_line));
        let exit = shell_execute_and_wait(exe_path, &args_string);
        // 无论成功失败都要停 tail，否则 scope 结束时 join 会一直等下去。
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = tail.join();
        exit
    });

    let exit_code = exit?;

    let raw = std::fs::read_to_string(&result_path).unwrap_or_default();
    let _ = std::fs::remove_file(&result_path);
    let _ = std::fs::remove_file(&progress_path);

    Ok(parse_result_json(&raw, exit_code))
}

#[cfg(windows)]
fn shell_execute_and_wait(exe_path: &Path, args_string: &str) -> Result<i32, String> {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    // HSTRING 要活到调用结束——放在函数作用域而不是临时变量里。
    let verb = HSTRING::from("runas");
    let file = HSTRING::from(exe_path.to_string_lossy().to_string());
    let params = HSTRING::from(args_string);

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        // 要进程句柄才能等它结束、取退出码。
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };

    // SAFETY: info 是栈上完整初始化的结构体，cbSize 已设；三个 HSTRING 在本函数内
    // 一直存活（比 ShellExecuteExW 调用活得久）。返回后 hProcess 由我们负责 Close。
    unsafe { ShellExecuteExW(&mut info) }.map_err(|e| {
        // ERROR_CANCELLED(1223) 是用户在 UAC 弹窗点了「否」，要给友好文案。
        let code = e.code().0 as u32;
        if code & 0xFFFF == 1223 {
            "用户取消了 UAC 提权请求".to_string()
        } else {
            format!("启动提权 exe 失败: {e}")
        }
    })?;

    let h: HANDLE = info.hProcess;
    if h.is_invalid() {
        return Err("提权进程未返回句柄（可能未真正启动）".to_string());
    }

    // SAFETY: h 是 ShellExecuteExW 返回的有效进程句柄；等待 INFINITE 直到退出。
    let wait = unsafe { WaitForSingleObject(h, INFINITE) };
    let mut code: u32 = 0;
    // SAFETY: h 有效；code 指向本栈帧上的 u32。
    let got = unsafe { GetExitCodeProcess(h, &mut code) };
    // SAFETY: 句柄由本函数独占，关闭一次。
    let _ = unsafe { CloseHandle(h) };

    if got.is_err() {
        return Err("读取提权进程退出码失败".to_string());
    }
    if wait != WAIT_OBJECT_0 {
        return Err(format!("等待提权进程失败（wait={})", wait.0));
    }

    Ok(code as i32)
}

#[cfg(not(windows))]
fn shell_execute_and_wait(_exe_path: &Path, _args_string: &str) -> Result<i32, String> {
    Err("提权仅支持 Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_unknown_command() {
        assert!(validate_command("format_disk").is_err());
        assert!(validate_command("").is_err());
        // 大小写敏感：`CREATE_RESTORE_POINT` 不算命中
        assert!(validate_command("CREATE_RESTORE_POINT").is_err());
    }

    #[test]
    fn validate_accepts_whitelisted_command() {
        assert!(validate_command("create_restore_point").is_ok());
    }

    #[test]
    fn args_string_quotes_values_with_spaces() {
        let s = build_args_string(&[("--description", "C-clean cleanup #12 2026-10-30")]);
        assert_eq!(s, "--description \"C-clean cleanup #12 2026-10-30\"");
    }

    #[test]
    fn args_string_neutralizes_embedded_quotes() {
        // 值里的双引号替换成空格，避免破坏参数边界（防注入）
        let s = build_args_string(&[("--description", "a\"b")]);
        assert_eq!(s, "--description \"a b\"");
        // 不能出现裸露的引号把参数截断
        assert_eq!(s.matches('"').count(), 2);
    }

    #[test]
    fn args_string_joins_multiple() {
        let s = build_args_string(&[("--a", "1"), ("--b", "2 3")]);
        assert_eq!(s, "--a \"1\" --b \"2 3\"");
    }

    #[test]
    fn parse_result_reads_full_json() {
        let raw = r#"{"ok":true,"cmd":"create_restore_point","message":"ok","sequenceNumber":42}"#;
        let r = parse_result_json(raw, 0);
        assert!(r.ok);
        assert_eq!(r.exit_code, 0);
        assert_eq!(r.message, "ok");
        assert_eq!(r.sequence_number, Some(42));
    }

    #[test]
    fn parse_result_falls_back_when_json_missing() {
        // 空内容（UAC 被拒 / 崩溃）：靠退出码兜底，message 要说明情况
        let r = parse_result_json("", 4);
        assert!(!r.ok);
        assert_eq!(r.exit_code, 4);
        assert!(r.message.contains("未返回结果"), "实际: {}", r.message);

        // 非 JSON：保留原文便于排查
        let r2 = parse_result_json("garbage", 4);
        assert!(!r2.ok);
        assert!(r2.message.contains("garbage"));
    }

    #[test]
    fn parse_result_trusts_exit_code_over_json_ok() {
        // JSON 说 ok=true 但退出码非 0 → 以退出码为准（退出码来自 OS，更可信）
        let raw = r#"{"ok":true,"cmd":"x","message":"","sequenceNumber":null}"#;
        let r = parse_result_json(raw, 4);
        assert!(!r.ok, "退出码非 0 时 ok 必须为 false");
    }

    #[test]
    fn run_elevated_rejects_unknown_cmd_before_launching() {
        // 路径不存在也没关系：白名单校验在路径检查之前，不该报「文件不存在」
        let err = run_elevated(Path::new("Z:/nope/cclean-elevation.exe"), "rm_rf", &[])
            .expect_err("未知命令必须拒绝");
        assert!(err.contains("不在提权白名单"), "实际: {err}");
    }

    #[test]
    fn run_elevated_errors_when_exe_missing() {
        let err = run_elevated(
            Path::new("Z:/nope/cclean-elevation.exe"),
            "create_restore_point",
            &[],
        )
        .expect_err("exe 不存在必须报错");
        assert!(err.contains("不存在"), "实际: {err}");
    }

    // ============ task-062：还原点名称 ============

    #[test]
    fn format_utc_known_timestamps() {
        // 1970-01-01 00:00:00
        assert_eq!(format_utc(0), "1970-01-01 00:00:00");
        // 2000-01-01 00:00:00（闰年边界：1999→2000）
        assert_eq!(format_utc(946_684_800), "2000-01-01 00:00:00");
        // 2024-02-29 12:34:56（闰日，验证 2 月 29 天不被算错）
        assert_eq!(format_utc(1_709_210_096), "2024-02-29 12:34:56");
        // 2026-10-30 00:00:00
        assert_eq!(format_utc(1_793_318_400), "2026-10-30 00:00:00");
    }

    #[test]
    fn restore_point_name_has_task_id_and_timestamp() {
        let name = restore_point_name(12, 1_793_318_400);
        assert_eq!(name, "C-clean cleanup #12 2026-10-30 00:00:00");
        // 必须带 #task_id（用户要在还原点面板里对上「哪次清理」）
        assert!(name.contains("#12"));
        assert!(name.starts_with("C-clean cleanup "));
    }

    #[test]
    fn restore_point_name_fits_in_255_utf16_units() {
        // Windows 的 szDescription 上限是 256 个 UTF-16 码元（含 NUL）。
        // 名称长度不能逼近上限，否则会被截断成难看的半截字符串。
        let name = restore_point_name(i64::MAX, 1_793_318_400);
        assert!(
            name.encode_utf16().count() < 200,
            "名称过长会被系统截断: {name}"
        );
    }

    // ============ task-065：进度文件 tail ============

    #[test]
    fn drain_returns_only_complete_lines() {
        // 只有到 `\n` 才算一行：末尾半行要留到下次（DISM 会边跑边写）
        let (lines, next) = drain_complete_lines(b"a\nb\nc", 0);
        assert_eq!(lines, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(next, 4, "游标必须停在最后一个 \\n 之后");

        // 续读：拿到剩下那半行 + 后续
        let (lines2, next2) = drain_complete_lines(b"a\nb\nc\nd\n", next);
        assert_eq!(lines2, vec!["c".to_string(), "d".to_string()]);
        assert_eq!(next2, 8);
    }

    #[test]
    fn drain_skips_blank_lines_and_strips_cr() {
        // DISM 输出空行极多，且换行是 \r\n；空行转发给 UI 只是噪音
        let (lines, _) = drain_complete_lines(b"a\r\n\r\n  \r\nb\r\n", 0);
        assert_eq!(lines, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn drain_handles_no_newline_yet() {
        let (lines, next) = drain_complete_lines(b"partial", 0);
        assert!(lines.is_empty());
        assert_eq!(next, 0, "没有完整行时游标不动");
    }

    #[test]
    fn drain_is_byte_safe_on_partial_utf8() {
        // 「清」的 UTF-8 是 E6 B8 85；只写了一半（E6 B8）时不能被当完整字符解，
        // 也不能因为非法 UTF-8 而 panic。整行不完整 → 这一行先不吐。
        let bytes = b"line1\n\xE6\xB8";
        let (lines, next) = drain_complete_lines(bytes, 0);
        assert_eq!(lines, vec!["line1".to_string()]);
        assert_eq!(next, 6);
    }

    #[test]
    fn drain_resets_when_file_shrinks() {
        // 提权 exe 用 File::create 截断重建进度文件：游标越界时必须能自愈
        let (lines, next) = drain_complete_lines(b"x\n", 999);
        assert!(lines.is_empty(), "越界游标不该读出错乱内容");
        assert_eq!(next, 2);
    }
}
