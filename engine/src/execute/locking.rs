//! 占用进程查询与请求关闭（Windows Restart Manager）。
//!
//! - task-020：`who_is_locking`——只读查询（RmStartSession / RmRegisterResources /
//!   RmGetList / RmEndSession）。
//! - task-049：`shutdown_locking_processes`——在同一个会话上追加
//!   `RmShutdown(RmForceShutdown)`，把「只报告」升级为「可请求关闭」。
//!
//! 非 Windows 平台一律返回空结果，保证跨平台编译。
//!
//! ## 已知限制（不静默，调用方必须知道）
//!
//! 1. `RmShutdown` 返回成功 **不等于** 目标进程已退出：RM 只是「请求关闭」，
//!    被关闭方可能弹「是否保存」对话框而阻塞，也可能在返回后才释放文件句柄。
//!    因此调用方**必须重新尝试删除**，不得假设「关了就一定删得掉」。
//! 2. RM 的关闭粒度是**会话**，不是进程：`RmShutdown` 会关闭该会话里所有占用
//!    已注册资源的应用。本模块的会话只注册了**一个**文件路径，所以作用域就是
//!    「占用这个文件的进程」；但若同一文件同时被保护进程与普通进程占用，
//!    RM 无法只关其中一个——护栏只能保证**不把它记进 `closed`**，
//!    对服务/系统进程 RM 本身也关不掉（Windows 不允许）。
//! 3. 本模块不提供 `RmRestart`（不自动重启被关掉的进程），留 task-050+。

/// 占用进程的最小可序列化视图（给前端 toast 用）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LockingProcess {
    pub pid: u32,
    /// 进程可执行文件名（UTF-16 转 Rust String）。
    pub app_name: String,
    /// 如果占用方是 Windows 服务，则为短名；否则 None。
    pub service_name: Option<String>,
}

/// task-049：一次 `shutdown_locking_processes` 的结果。
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShutdownOutcome {
    /// 已请求关闭（`RmShutdown` 返回成功）的进程。
    pub closed: Vec<LockingProcess>,
    /// 被护栏拒绝、**绝不关闭**的进程，带稳定拒绝原因。
    pub refused: Vec<RefusedProcess>,
}

/// task-049：被护栏拒绝的进程。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefusedProcess {
    pub pid: u32,
    pub app_name: String,
    /// 稳定文案，见 [`is_protected_process`]：`windows service` / `system process` /
    /// `self` / `protected process`。
    pub reason: String,
}

/// task-049：`cleanup_retry_locked` 命令的响应体。
///
/// 放在 engine 侧（而不是 `app/src-tauri/src/commands.rs`）的原因：app crate 的
/// `Cargo.toml` 没有 serde 依赖，且本任务不允许改依赖清单；信封要求 `T: Serialize`，
/// 所以结构体必须落在已有 serde 的 engine 里。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryLockedResp {
    /// 请求关闭的结果（closed 已请求关闭 / refused 被护栏拒绝）。
    pub shutdown: ShutdownOutcome,
    /// 关闭后重新执行删除的汇总（与 `cleanup_execute` 同一 shape）。
    pub summary: crate::execute::executor::ExecuteSummary,
}

// ---- 护栏拒绝原因（稳定英文文案，前端直接展示，不翻译不改写） ----

const REASON_SERVICE: &str = "windows service";
const REASON_SYSTEM: &str = "system process";
const REASON_SELF: &str = "self";
const REASON_PROTECTED: &str = "protected process";

/// 内置保护名单（小写比较；命中即拒绝关闭）。
///
/// 覆盖：桌面外壳、服务宿主、本地安全认证、会话/窗口管理、恶意软件防护，
/// 以及本应用自身（避免用户误关自己正在跑的程序）。
const PROTECTED_APP_NAMES: &[&str] = &[
    "explorer.exe",
    "svchost.exe",
    "lsass.exe",
    "csrss.exe",
    "wininit.exe",
    "services.exe",
    "winlogon.exe",
    "smss.exe",
    "dwm.exe",
    "registry",
    "msmpeng.exe",
    "cdisk-cleaner.exe",
];

/// 判断 `app_name` 是否命中保护名单。
///
/// RM 的 `strAppName` 是**应用友好名**（实测 `powershell.exe` → `Windows PowerShell`），
/// 不一定是 exe 文件名，所以不能只做全名相等比较：
/// 1. 全名相等（`explorer.exe`）；
/// 2. 去掉 `.exe` 后相等（`EXPLORER.EXE` → `explorer`）；
/// 3. 去掉 `.exe` 后以名单基名结尾（`Windows Explorer` → `explorer`）。
///
/// 第 3 条会带来极少数误伤（名字恰好以保护名结尾的第三方程序），但方向是
/// 「宁可不关」——保护类判断的误伤代价远低于误杀系统进程。
fn matches_protected_name(app_name: &str) -> bool {
    let name = app_name.trim().to_lowercase();
    if name.is_empty() {
        return false;
    }
    let base = name.strip_suffix(".exe").unwrap_or(&name);
    PROTECTED_APP_NAMES.iter().any(|p| {
        let p_base = p.strip_suffix(".exe").unwrap_or(p);
        name == *p || base == p_base || base.ends_with(p_base)
    })
}

/// 判断一个占用进程是否**禁止关闭**。
///
/// 返回 `Some(reason)` 表示拒绝（reason 为四个稳定文案之一），`None` 表示可关。
///
/// 判定顺序（先命中先返回，避免同一进程给出模棱两可的原因）：
/// 1. `pid == self_pid` → `self`（不自杀，优先级最高）；
/// 2. `pid <= 4` → `system process`（Idle / System）；
/// 3. `service_name` 非空 → `windows service`（服务一律不碰）；
/// 4. `app_name` 为空 → `system process`（保守，认不出就不关）；
/// 5. `app_name` 忽略大小写命中内置名单 → `protected process`。
pub(crate) fn is_protected_process(
    app_name: &str,
    service_name: Option<&str>,
    pid: u32,
    self_pid: u32,
) -> Option<&'static str> {
    if pid == self_pid {
        return Some(REASON_SELF);
    }
    if pid <= 4 {
        return Some(REASON_SYSTEM);
    }
    if let Some(svc) = service_name {
        if !svc.trim().is_empty() {
            return Some(REASON_SERVICE);
        }
    }
    let name = app_name.trim();
    if name.is_empty() {
        return Some(REASON_SYSTEM);
    }
    if matches_protected_name(name) {
        return Some(REASON_PROTECTED);
    }
    None
}

/// 查询 `path` 被哪些进程占用。
///
/// Windows：走 Restart Manager 会话，返回占用进程列表。
/// 非 Windows：返回空 Vec（跨平台编译兜底）。
pub fn who_is_locking(path: &str) -> Result<Vec<LockingProcess>, String> {
    #[cfg(windows)]
    {
        let session = RmSession::start()?;
        session.register(path)?;
        Ok(session.list()?.iter().map(to_locking_process).collect())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Ok(Vec::new())
    }
}

/// 请求关闭占用 `path` 的进程（task-049）。
///
/// 流程：`RmStartSession → RmRegisterResources(只注册这一个文件) → RmGetList →
/// 护栏过滤 → RmShutdown(RmForceShutdown) → RmEndSession`。
///
/// - 无占用进程：不调 `RmShutdown`，返回空结果（幂等，重复点「重试」不报错）；
/// - 有占用但全部被护栏拒绝：不调 `RmShutdown`，`closed` 为空、`refused` 完整；
/// - 至少一个可关进程：调 `RmShutdown`，成功后把这些进程放进 `closed`。
///
/// 会话由 [`RmSession`] 的 `Drop` 收尾，任何提前返回都不会泄漏 handle。
pub fn shutdown_locking_processes(path: &str) -> Result<ShutdownOutcome, String> {
    #[cfg(windows)]
    {
        let session = RmSession::start()?;
        session.register(path)?;
        let infos = session.list()?;
        if infos.is_empty() {
            // 幂等：没有占用方就直接收工，不碰 RmShutdown。
            return Ok(ShutdownOutcome::default());
        }

        let self_pid = std::process::id();
        let mut closable: Vec<LockingProcess> = Vec::new();
        let mut refused: Vec<RefusedProcess> = Vec::new();
        for info in &infos {
            let lp = to_locking_process(info);
            match is_protected_process(&lp.app_name, lp.service_name.as_deref(), lp.pid, self_pid) {
                Some(reason) => refused.push(RefusedProcess {
                    pid: lp.pid,
                    app_name: lp.app_name.clone(),
                    reason: reason.to_string(),
                }),
                None => closable.push(lp),
            }
        }

        if closable.is_empty() {
            // 全部被护栏拒绝：连 RmShutdown 都不调（绝不关闭）。
            return Ok(ShutdownOutcome {
                closed: Vec::new(),
                refused,
            });
        }

        session.shutdown()?;
        Ok(ShutdownOutcome {
            closed: closable,
            refused,
        })
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Ok(ShutdownOutcome::default())
    }
}

#[cfg(windows)]
fn to_locking_process(p: &RM_PROCESS_INFO) -> LockingProcess {
    let app_name = u16_to_string(&p.strAppName);
    let svc_raw = u16_to_string(&p.strServiceShortName);
    let service_name = if svc_raw.is_empty() {
        None
    } else {
        Some(svc_raw)
    };
    LockingProcess {
        pid: p.Process.dwProcessId,
        app_name,
        service_name,
    }
}

#[cfg(windows)]
use windows::Win32::System::RestartManager::RM_PROCESS_INFO;

/// Restart Manager 会话的 RAII 包装。
///
/// `RmEndSession` 放在 `Drop` 里，保证「成功 / 失败 / 提前返回」三条路径都会释放
/// 会话句柄——这是 task-049 验收项 A2「会话不泄漏」的实现方式。
#[cfg(windows)]
struct RmSession {
    handle: u32,
}

#[cfg(windows)]
impl RmSession {
    fn start() -> Result<Self, String> {
        use windows::core::PWSTR;
        use windows::Win32::System::RestartManager::RmStartSession;

        let mut handle: u32 = 0;
        let mut key = [0u16; 33];
        // SAFETY: handle / key 都是本函数栈上的有效可写内存；key 长度 33 是
        // CCH_RM_SESSION_KEY + 1，符合 RmStartSession 对 szSessionKey 的要求。
        unsafe { RmStartSession(&mut handle, 0, PWSTR(key.as_mut_ptr())) }
            .ok()
            .map_err(|e| format!("RmStartSession: {e}"))?;
        Ok(Self { handle })
    }

    fn register(&self, path: &str) -> Result<(), String> {
        use windows::core::PCWSTR;
        use windows::Win32::System::RestartManager::RmRegisterResources;

        let path_w: Vec<u16> = path.encode_utf16().chain([0]).collect();
        let files = [PCWSTR(path_w.as_ptr())];
        // SAFETY: path_w 在调用期间存活；files 指向它的首元素；
        // 只注册一个文件资源，保证 RmShutdown 的作用域就是「占用这个文件的进程」。
        unsafe { RmRegisterResources(self.handle, Some(&files), None, None) }
            .ok()
            .map_err(|e| format!("RmRegisterResources: {e}"))
    }

    /// `RmGetList` 两段式：先 probe 拿所需条数，再按条数取真实数据。
    fn list(&self) -> Result<Vec<RM_PROCESS_INFO>, String> {
        use windows::Win32::Foundation::ERROR_MORE_DATA;
        use windows::Win32::System::RestartManager::RmGetList;

        let mut needed: u32 = 0;
        let mut count: u32 = 0;
        let mut reboot_reason: u32 = 0;
        // SAFETY: 三个 &mut 均指向本函数栈上有效变量；rgaffectedapps 传 None 是
        // RmGetList 文档规定的 probe 用法（返回 ERROR_MORE_DATA + 所需条数）。
        let probe = unsafe {
            RmGetList(
                self.handle,
                &mut needed,
                &mut count,
                None,
                &mut reboot_reason,
            )
        };
        // 注意：这里必须比 WIN32_ERROR 的原始码（probe.0），不能用
        // `Error::code()`——那是 HRESULT，ERROR_MORE_DATA 会变成 0x800700EA
        // （task-049 修：旧实现按 234 比较，导致「真有进程占用」时直接报错，
        // who_is_locking 在 task-020/021 里实际上从未返回过占用进程）。
        if probe.0 != 0 && probe.0 != ERROR_MORE_DATA.0 {
            return Err(format!(
                "RmGetList (probe, code={}): {}",
                probe.0,
                std::io::Error::from_raw_os_error(probe.0 as i32)
            ));
        }

        if needed == 0 {
            return Ok(Vec::new());
        }

        // 直接用 RM_PROCESS_INFO 数组接收，保证内存对齐（不用 Vec<u8> 手工算偏移）。
        let mut buf: Vec<RM_PROCESS_INFO> = vec![RM_PROCESS_INFO::default(); needed as usize];
        count = needed;
        // SAFETY: buf 至少有 needed 个元素，容量足够；needed 是入参上限，
        // count 由系统写回实际条数（≤ needed）。
        unsafe {
            RmGetList(
                self.handle,
                &mut needed,
                &mut count,
                Some(buf.as_mut_ptr()),
                &mut reboot_reason,
            )
        }
        .ok()
        .map_err(|e| format!("RmGetList (fetch): {e}"))?;

        buf.truncate(count.min(buf.len() as u32) as usize);
        Ok(buf)
    }

    /// 请求关闭会话内所有占用进程（`RmForceShutdown`）。
    fn shutdown(&self) -> Result<(), String> {
        use windows::Win32::System::RestartManager::{RmForceShutdown, RmShutdown};

        // SAFETY: handle 来自 RmStartSession 且由 Drop 保证只 End 一次；
        // 回调传 None（不关心关闭进度）；RmForceShutdown 让被关方走强制关闭流程。
        unsafe { RmShutdown(self.handle, RmForceShutdown.0 as u32, None) }
            .ok()
            .map_err(|e| format!("RmShutdown: {e}"))
    }
}

#[cfg(windows)]
impl Drop for RmSession {
    fn drop(&mut self) {
        use windows::Win32::System::RestartManager::RmEndSession;

        // SAFETY: handle 只在 RmSession 里保存，Drop 只会执行一次；
        // RmEndSession 的返回值这里无法处理（析构中），失败仅意味着会话已被系统回收。
        unsafe {
            let _ = RmEndSession(self.handle);
        }
    }
}

/// 把 [u16] UTF-16 缓冲按第一个 NUL 截断转 String。
#[cfg(windows)]
fn u16_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

#[cfg(test)]
mod tests {
    //! task-049 护栏纯函数单测（8 条，验收项 A3）。
    //!
    //! 放在模块内而不是 `engine/tests/`：`is_protected_process` 是 `pub(crate)`，
    //! 跨 crate 的集成测试看不见它（见任务实现备注里的偏差说明）。

    use super::is_protected_process;

    const SELF: u32 = 4242;

    #[test]
    fn allows_normal_process() {
        assert_eq!(is_protected_process("notepad.exe", None, 1234, SELF), None);
    }

    #[test]
    fn refuses_explorer() {
        assert_eq!(
            is_protected_process("explorer.exe", None, 1234, SELF),
            Some("protected process")
        );
    }

    #[test]
    fn refuses_svchost() {
        assert_eq!(
            is_protected_process("svchost.exe", None, 1234, SELF),
            Some("protected process")
        );
    }

    #[test]
    fn refuses_protected_name_case_insensitive() {
        assert_eq!(
            is_protected_process("EXPLORER.EXE", None, 1234, SELF),
            Some("protected process")
        );
        assert_eq!(
            is_protected_process("MsMpEng.exe", None, 1234, SELF),
            Some("protected process")
        );
    }

    #[test]
    fn refuses_windows_service() {
        assert_eq!(
            is_protected_process("anything.exe", Some("wuauserv"), 1234, SELF),
            Some("windows service")
        );
    }

    #[test]
    fn refuses_system_pid() {
        assert_eq!(
            is_protected_process("System", None, 4, SELF),
            Some("system process")
        );
        assert_eq!(
            is_protected_process("Idle", None, 0, SELF),
            Some("system process")
        );
    }

    #[test]
    fn refuses_self() {
        assert_eq!(
            is_protected_process("cdisk-cleaner.exe", None, SELF, SELF),
            Some("self")
        );
    }

    #[test]
    fn refuses_empty_app_name() {
        assert_eq!(
            is_protected_process("   ", None, 1234, SELF),
            Some("system process")
        );
    }

    #[test]
    fn refuses_rm_friendly_name_of_protected_app() {
        // RM 的 strAppName 是应用友好名，不是 exe 文件名（实测 PowerShell 报
        // "Windows PowerShell"），所以 "Windows Explorer" 也必须拦住。
        assert_eq!(
            is_protected_process("Windows Explorer", None, 1234, SELF),
            Some("protected process")
        );
    }

    #[test]
    fn allows_rm_friendly_name_of_normal_app() {
        // 反例：真实 RM 输出 "Windows PowerShell"，不能被误判成受保护进程。
        assert_eq!(
            is_protected_process("Windows PowerShell", None, 1234, SELF),
            None
        );
    }
}
