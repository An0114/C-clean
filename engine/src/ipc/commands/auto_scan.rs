//! task-073：系统级定时扫描（Windows 任务计划程序）。
//!
//! # 为什么需要它
//!
//! task-061 的自动扫描是**应用内轮询**：应用在跑（含托盘常驻）才会触发。
//! 用户如果没开开机自启、或者手动退了托盘，就一次都不会扫。任务计划是
//! 「应用完全没运行时也能扫」的那一层。
//!
//! # 两者会不会重复扫
//!
//! 不会，而且是天然互斥的：计划任务拉起应用跑完一次后会把
//! `prefs.last_auto_scan_at` 更新成当前时间，应用内轮询的 `auto_scan_due()`
//! 看到「今天已经扫过」就跳过。反过来也一样。
//!
//! # 为什么用 `schtasks` 而不是 COM（`ITaskService`）
//!
//! COM 方案要引 `Win32_System_TaskScheduler` + 初始化 COM + 手写
//! `ITaskDefinition` / `ITrigger` / `IAction` 一整套接口，代码量是这里的
//! 3 倍以上，而能力只多出「更细的触发器配置」——本任务只需要「每天一次」。
//!
//! # 为什么全部靠**退出码**判断，不解析输出
//!
//! `schtasks` 的输出是**本地化文本**：中文系统上是「成功: 成功创建计划任务」，
//! 英文系统上是「SUCCESS: The scheduled task ...」。按字符串匹配在别人的机器上
//! 必挂（task-065 的 DISM、task-068 的 schtasks /query 都踩过同一个坑）。
//! 退出码是跨语言稳定的：0 = 成功 / 任务存在，非 0 = 失败 / 任务不存在。
//!
//! # 安全约束
//!
//! - 任务名固定为 [`TASK_NAME`]，不接受外部传入——否则等于开放「用任意名字
//!   覆盖系统里已有的计划任务」；
//! - 时间参数只接受 `HH:MM`，拒绝任何额外字符（防止 `/st 09:00 /ru SYSTEM`
//!   这种参数注入）；
//! - 可执行文件路径必须是**绝对路径且存在**——相对路径在计划任务里会被解析到
//!   `System32` 下，任务永远起不来。

use std::path::{Path, PathBuf};
use std::process::Command;

/// 计划任务名。**固定值**，不接受外部传入（见模块注释「安全约束」）。
pub const TASK_NAME: &str = "C-Disk Cleaner Auto Scan";

/// 默认触发时间（本地时区）。
pub const DEFAULT_TIME: &str = "09:00";

/// 传给应用的命令行开关。应用启动时看到它就不建窗口，扫完直接退出。
///
/// **必须与 `app/src-tauri/src/lib.rs` 的 `AUTO_SCAN_FLAG` 一致**——
/// 改一边忘另一边，计划任务就会变成「每天弹一个窗口」。
pub const AUTO_SCAN_FLAG: &str = "--auto-scan";

/// 校验触发时间格式：必须是 `HH:MM`，24 小时制。
///
/// 只做格式校验不够——`/st` 后面跟的字符串会原样进 `schtasks` 的参数表，
/// 允许任意字符就等于允许注入额外开关。所以这里**只接受数字和冒号**，
/// 且长度必须正好 5。
pub fn validate_time(t: &str) -> Result<(), String> {
    let bytes = t.as_bytes();
    if bytes.len() != 5 {
        return Err(format!("时间格式必须是 HH:MM，收到 {t:?}"));
    }
    if bytes[2] != b':' {
        return Err(format!("时间格式必须是 HH:MM，收到 {t:?}"));
    }
    if !bytes
        .iter()
        .enumerate()
        .all(|(i, b)| i == 2 || b.is_ascii_digit())
    {
        return Err(format!("时间只能含数字和冒号，收到 {t:?}"));
    }
    let hh: u32 = t[0..2].parse().map_err(|_| format!("小时非法: {t:?}"))?;
    let mm: u32 = t[3..5].parse().map_err(|_| format!("分钟非法: {t:?}"))?;
    if hh > 23 {
        return Err(format!("小时必须在 00-23，收到 {hh}"));
    }
    if mm > 59 {
        return Err(format!("分钟必须在 00-59，收到 {mm}"));
    }
    Ok(())
}

/// 拼 `/tr` 的值：`"<绝对路径>" --auto-scan`。
///
/// 路径**必须加引号**：安装态在 `C:\Program Files\C-Disk Cleaner\` 下，
/// 不加引号会被 schtasks 在空格处截断，任务指向 `C:\Program.exe`。
///
/// 这里只负责拼出「一个参数」的字符串；最终命令行转义由
/// `std::process::Command` 处理（它会把含空格的参数整体加引号，并把内部
/// `"` 转成 `\"`），所以不能自己再套一层转义。
pub fn build_run_value(exe: &Path) -> Result<String, String> {
    if !exe.is_absolute() {
        return Err(format!("计划任务必须用绝对路径，收到 {}", exe.display()));
    }
    if !exe.exists() {
        return Err(format!("可执行文件不存在: {}", exe.display()));
    }
    Ok(format!("\"{}\" {AUTO_SCAN_FLAG}", exe.display()))
}

/// 拼 `/create` 的完整参数表（**纯函数**，可单测）。
///
/// 单独抽出来是为了能断言「参数里不会混进 `/ru` / `/rl` 这类提权开关」——
/// 本任务**不需要管理员权限**（创建当前用户的任务，`schtasks` 默认以当前用户
/// 身份运行），加 `/ru SYSTEM` 反而会要求提权并让任务在 Session 0 里跑，
/// 拿不到用户的通知权限。
pub fn build_create_args(exe: &Path, time: &str) -> Result<Vec<String>, String> {
    validate_time(time)?;
    let run = build_run_value(exe)?;
    Ok(vec![
        "/create".to_string(),
        "/tn".to_string(),
        TASK_NAME.to_string(),
        "/tr".to_string(),
        run,
        "/sc".to_string(),
        "daily".to_string(),
        "/st".to_string(),
        time.to_string(),
        // /f：同名任务直接覆盖，不弹「是否替换」——非交互环境里弹窗等于卡死。
        "/f".to_string(),
    ])
}

/// `schtasks` 可执行文件的绝对路径。
///
/// 走 `%SystemRoot%\System32\`，不用 PATH 查找：PATH 可被篡改，
/// 而这是个会「注册开机后自动执行的东西」的调用，被劫持的后果比一般命令严重。
fn schtasks_exe() -> PathBuf {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    PathBuf::from(root).join("System32").join("schtasks.exe")
}

/// 跑一次 `schtasks`，返回 `(退出码, stdout, stderr)`。
///
/// **stdout/stderr 用 `from_utf8_lossy`**：schtasks 的输出是本地化文本，
/// 中文系统上是 GBK。这里不解析内容，只用于出错时把原文带给用户看，
/// 所以 lossy 转换够用（乱码总比丢信息强）。
fn run_schtasks(args: &[String]) -> Result<(i32, String, String), String> {
    let out = Command::new(schtasks_exe())
        .args(args)
        .output()
        .map_err(|e| format!("启动 schtasks 失败: {e}"))?;
    Ok((
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
        String::from_utf8_lossy(&out.stderr).trim().to_string(),
    ))
}

/// 注册（或覆盖）每日定时扫描任务。
pub fn enable(exe: &Path, time: &str) -> Result<(), String> {
    let args = build_create_args(exe, time)?;
    let (code, stdout, stderr) = run_schtasks(&args)?;
    if code != 0 {
        return Err(format!(
            "schtasks /create 失败（退出码 {code}）：{}",
            if stderr.is_empty() { &stdout } else { &stderr }
        ));
    }
    Ok(())
}

/// 删除定时扫描任务。
///
/// **任务不存在时视为成功**：用户可能在「任务计划程序」里手动删过，
/// 这时点关闭开关不该报错——目标状态（没有这个任务）已经达成了。
pub fn disable() -> Result<(), String> {
    let args = vec![
        "/delete".to_string(),
        "/tn".to_string(),
        TASK_NAME.to_string(),
        "/f".to_string(),
    ];
    let (code, stdout, stderr) = run_schtasks(&args)?;
    if code == 0 {
        return Ok(());
    }
    // 非 0 有两种：任务本来就不存在（可接受）/ 真失败（权限、任务被锁）。
    // 靠 `is_registered` 再确认一次，避免把「本来就没有」报成错误。
    if !is_registered()? {
        return Ok(());
    }
    Err(format!(
        "schtasks /delete 失败（退出码 {code}）：{}",
        if stderr.is_empty() { &stdout } else { &stderr }
    ))
}

/// 查询任务是否已注册。**只看退出码**，不解析本地化输出。
pub fn is_registered() -> Result<bool, String> {
    let args = vec![
        "/query".to_string(),
        "/tn".to_string(),
        TASK_NAME.to_string(),
    ];
    let (code, _stdout, _stderr) = run_schtasks(&args)?;
    Ok(code == 0)
}

/// 供前端展示的状态快照。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoScanTaskStatus {
    /// 任务是否已注册。
    pub registered: bool,
    /// 任务名（固定值，展示给用户看，方便他去「任务计划程序」里核对）。
    pub task_name: String,
    /// 计划触发时间（`HH:MM`）。未注册时仍是默认值——前端要能显示「开起来会是什么时候」。
    pub time: String,
}

/// `auto_scan_enable` 的纯逻辑层：注册任务并返回**注册后**的真实状态。
///
/// 回读一次而不是直接返回 `registered: true`：`schtasks /create` 可能返回 0
/// 但任务被组策略立刻禁用/删除（企业环境常见）。回读能保证界面显示的是真实状态。
pub fn handle_enable(exe: &Path, time: &str) -> Result<AutoScanTaskStatus, String> {
    enable(exe, time)?;
    let registered = is_registered()?;
    if !registered {
        return Err("任务注册后回查不到，可能被组策略或安全软件拦截".to_string());
    }
    Ok(AutoScanTaskStatus {
        registered: true,
        task_name: TASK_NAME.to_string(),
        time: time.to_string(),
    })
}

/// `auto_scan_disable` 的纯逻辑层：删除任务并返回删除后的真实状态。
pub fn handle_disable() -> Result<AutoScanTaskStatus, String> {
    disable()?;
    Ok(AutoScanTaskStatus {
        registered: is_registered()?,
        task_name: TASK_NAME.to_string(),
        time: DEFAULT_TIME.to_string(),
    })
}

/// `auto_scan_status` 的纯逻辑层：只查询，不改动任何东西。
pub fn handle_status() -> Result<AutoScanTaskStatus, String> {
    Ok(AutoScanTaskStatus {
        registered: is_registered()?,
        task_name: TASK_NAME.to_string(),
        time: DEFAULT_TIME.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exe() -> PathBuf {
        // 用当前测试二进制当「存在的绝对路径」——它一定存在且是绝对路径。
        std::env::current_exe().unwrap()
    }

    #[test]
    fn validate_time_accepts_hh_mm() {
        assert!(validate_time("00:00").is_ok());
        assert!(validate_time("09:00").is_ok());
        assert!(validate_time("23:59").is_ok());
    }

    #[test]
    fn validate_time_rejects_out_of_range() {
        assert!(validate_time("24:00").is_err());
        assert!(validate_time("09:60").is_err());
    }

    #[test]
    fn validate_time_rejects_injection_attempts() {
        // 这几个如果放行，会变成 schtasks 的额外开关
        for bad in [
            "09:00 /ru SYSTEM",
            "09:00 /rl HIGHEST",
            "9:00",
            "09-00",
            "09:0a",
            "",
            "09:00:00",
        ] {
            assert!(validate_time(bad).is_err(), "不该接受 {bad:?}");
        }
    }

    #[test]
    fn run_value_quotes_path_and_appends_flag() {
        let e = exe();
        let v = build_run_value(&e).unwrap();
        assert!(v.starts_with('"'), "路径必须加引号，实际: {v}");
        assert!(v.contains(AUTO_SCAN_FLAG));
        assert!(v.ends_with(AUTO_SCAN_FLAG));
    }

    #[test]
    fn run_value_rejects_relative_path() {
        // 相对路径在计划任务里会被解析到 System32 下，任务永远起不来
        let err = build_run_value(Path::new("cdisk-cleaner.exe")).unwrap_err();
        assert!(err.contains("绝对路径"), "实际: {err}");
    }

    #[test]
    fn run_value_rejects_missing_file() {
        let err = build_run_value(&exe().parent().unwrap().join("no-such-xyz.exe")).unwrap_err();
        assert!(err.contains("不存在"), "实际: {err}");
    }

    #[test]
    fn create_args_has_daily_trigger_and_force() {
        let args = build_create_args(&exe(), "09:00").unwrap();
        assert_eq!(args[0], "/create");
        // 任务名必须是我们固定的那个，不能被外部换掉
        let i = args.iter().position(|a| a == "/tn").unwrap();
        assert_eq!(args[i + 1], TASK_NAME);
        let i = args.iter().position(|a| a == "/sc").unwrap();
        assert_eq!(args[i + 1], "daily");
        let i = args.iter().position(|a| a == "/st").unwrap();
        assert_eq!(args[i + 1], "09:00");
        // /f 必须有：非交互环境里弹「是否替换」等于卡死
        assert!(args.contains(&"/f".to_string()));
    }

    #[test]
    fn create_args_never_asks_for_elevation() {
        // 本功能不需要管理员权限；加 /ru SYSTEM 会要求提权并把任务丢进
        // Session 0，拿不到用户的通知权限。
        let args = build_create_args(&exe(), "09:00").unwrap();
        for forbidden in ["/ru", "/rp", "/rl", "/it"] {
            assert!(
                !args.iter().any(|a| a == forbidden),
                "不该出现 {forbidden}: {args:?}"
            );
        }
    }

    #[test]
    fn create_args_rejects_bad_time() {
        assert!(build_create_args(&exe(), "9:00").is_err());
    }

    #[test]
    fn task_name_is_stable() {
        // 任务名是用户在「任务计划程序」里找它的唯一线索，改名等于让人找不到。
        assert_eq!(TASK_NAME, "C-Disk Cleaner Auto Scan");
    }

    #[test]
    fn auto_scan_flag_matches_app_side() {
        // 与 app 侧的一致性由 engine/tests/auto_scan_test.rs 的静态检查兜底，
        // 这里只钉住字面值，防止随手改成 --silent 之类。
        assert_eq!(AUTO_SCAN_FLAG, "--auto-scan");
    }
}
