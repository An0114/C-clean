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
//!    RM 无法只关其中一个。因此 task-049b 起采取 **mixed fail-closed**：
//!    只要枚举出的占用进程里有任何一个被护栏拒绝，整批都不调 `RmShutdown`
//!    （`closed` 为空），保证 `refused` 里写的「受保护未关闭」与事实一致。
//!    代价是这类混合占用场景点「重试」不会关闭任何进程（宁可不关）。
//! 3. 护栏的判定依据是 **PID → 进程映像文件名**（`QueryFullProcessImageNameW`），
//!    不是 `RmGetList` 的 `strAppName`——后者是**本地化 FileDescription**
//!    （`zh-CN` 下 `explorer.exe` → `Windows 资源管理器`），按它匹配在非英文
//!    系统上必然失效（task-049 评审 Bug #1）。取不到映像名时 **fail-closed**
//!    （拒绝，reason = `unknown process`）。
//! 4. 同一个 console / 进程组里的兄弟进程可能被**连带**关闭：task-049b 实测，
//!    两个共用 console 的 `powershell.exe` holder，只对 A 的文件调 `RmShutdown`，
//!    B 也会退出（exit code 1，且没走到脚本结尾）。本模块无法控制这个粒度，
//!    调用方若发现相邻进程被波及，原因在这里。集成用例
//!    `shutdown_only_touches_registered_path` 给每个 holder 独立 console，
//!    以便把「只关注册资源」这一性质单独测出来。
//! 5. task-057 起，本模块在 `shutdown_locking_processes(path, auto_restart)` 里
//!    按需对 `bRestartable==TRUE` 的进程调 `RmRestart`（默认 false，由设置页开关控制）。

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
///
/// task-049b（评审 Bug #2）：`RmShutdown` 是**会话级**的，只要会话里有 1 个可关
/// 进程，被护栏拒绝的保护进程也会被一并请求关闭——那样 `refused` 里写「受保护
/// 未关闭」就与事实相反。所以现在的语义是：**只要存在被拒绝的进程，整批都不关**，
/// 此时 `closed` 必为空、`refused` 就是这次枚举到的全部被拒进程。
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
    /// `self` / `protected process` / `unknown process`。
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
/// task-049b：取不到进程映像名（`OpenProcess` 被拒 / 进程已退出）时的拒绝原因。
///
/// 与 `protected process` 分开是为了可排查：前者是「认不出是谁」，后者是
/// 「认出来了，但在名单里」。两者都拒绝（fail-closed），方向一致。
const REASON_UNKNOWN: &str = "unknown process";

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

/// 判断一个名字是否命中保护名单。
///
/// 两处调用：
/// 1. **主判定**（task-049b 起）：传入 `image_file_name(pid)` 的结果——真实的
///    exe 文件名，与系统语言无关，这是唯一可信的判据；
/// 2. **兜底判定**：传入 RM 的 `strAppName`。它是**应用友好名**（实测
///    `powershell.exe` → `Windows PowerShell`，`zh-CN` 下 `explorer.exe` →
///    `Windows 资源管理器`），只能覆盖「RM 直接返回 exe 名」的情形，不能作为
///    唯一防线（task-049 评审 Bug #1 的成因）。
///
/// 三条规则（任一命中即算命中）：
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

/// 单个占用进程的判定核心（纯函数，映像名由调用方注入，便于单测穷举）。
///
/// `image_name` 为 `None` 表示「取不到映像名」，此时 **fail-closed**（拒绝）。
///
/// 判定顺序（先命中先返回，避免同一进程给出模棱两可的原因）：
/// 1. `pid == self_pid` → `self`（不自杀，优先级最高）；
/// 2. `pid <= 4` → `system process`（Idle / System）；
/// 3. `service_name` 非空 → `windows service`（服务一律不碰）；
/// 4. `image_name` 命中内置名单 → `protected process`（**主判据，与语言无关**）；
/// 5. 取不到 `image_name` → `unknown process`（**fail-closed**，认不出就不关）；
/// 6. `app_name` 为空 → `system process`（保守）；
/// 7. `app_name` 命中内置名单 → `protected process`（兜底，覆盖 RM 直接返回
///    exe 名的情形）。
fn decide(
    app_name: &str,
    service_name: Option<&str>,
    pid: u32,
    self_pid: u32,
    image_name: Option<&str>,
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
    match image_name {
        Some(img) => {
            if matches_protected_name(img) {
                return Some(REASON_PROTECTED);
            }
        }
        // 取不到映像名：认不出是谁，宁可不关。
        None => return Some(REASON_UNKNOWN),
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

/// 判断一个占用进程是否**禁止关闭**（对外入口，映像名由 PID 实时查询）。
///
/// 返回 `Some(reason)` 表示拒绝（reason 为稳定文案之一），`None` 表示可关。
pub(crate) fn is_protected_process(
    app_name: &str,
    service_name: Option<&str>,
    pid: u32,
    self_pid: u32,
) -> Option<&'static str> {
    let image = image_file_name(pid);
    decide(app_name, service_name, pid, self_pid, image.as_deref())
}

/// 批量判定的收口：落实 mixed fail-closed。
///
/// mixed fail-closed（task-049b，评审 Bug #2）：`RmShutdown` 是会话级的，
/// 会话里只要有一个可关进程，被拒绝的保护进程也会被一并请求关闭。为了不让
/// `refused` 的「受保护未关闭」变成假话，**只要有任何一个进程被拒绝，整批都不关**
/// （`closable` 清空 → 调用方不调 `RmShutdown`）。
///
/// 被清掉的可关进程不会静默消失：前端的失败明细里仍然列着它们
/// （`execute_items` 返回的 `lockingProcesses`）。
fn apply_mixed_fail_closed(
    closable: Vec<LockingProcess>,
    refused: Vec<RefusedProcess>,
) -> (Vec<LockingProcess>, Vec<RefusedProcess>) {
    if !closable.is_empty() && !refused.is_empty() {
        return (Vec::new(), refused);
    }
    (closable, refused)
}

/// 取进程映像文件名（小写，含扩展名）；取不到返回 `None`。
///
/// 用 `QueryFullProcessImageNameW` 而不是 RM 的 `strAppName`：前者返回真实的
/// 可执行文件路径，与系统语言无关；后者是本地化的 FileDescription。
///
/// 失败分支（一律返回 `None`，由 [`decide`] fail-closed 兜住）：
/// - `OpenProcess` 被拒（权限不足 / 进程已退出）→ `None`；
/// - `QueryFullProcessImageNameW` 失败（进程刚退出 / 缓冲区不足）→ `None`；
/// - 路径末段为空 → `None`。
#[cfg(windows)]
fn image_file_name(pid: u32) -> Option<String> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };

    /// 句柄 RAII：任何提前 return 都会 `CloseHandle`，不新增泄漏点。
    struct OwnedHandle(HANDLE);

    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            // SAFETY: 句柄由 OpenProcess 返回，且只被这一个 OwnedHandle 持有。
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    if pid == 0 {
        return None;
    }
    // SAFETY: 只请求查询权限（PROCESS_QUERY_LIMITED_INFORMATION），不改变目标
    // 进程状态；失败返回 Err，由 `?` 转成 None（fail-closed）。
    let handle =
        OwnedHandle(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?);

    // 32768 u16 覆盖 \\?\ 长路径上限，避免 ERROR_INSUFFICIENT_BUFFER。
    let mut buf = vec![0u16; 32768];
    let mut len = buf.len() as u32;
    // SAFETY: buf 整块可写且容量由 len 声明；handle 由 OwnedHandle 保证有效。
    unsafe {
        QueryFullProcessImageNameW(
            handle.0,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
    }
    .ok()?;

    let path = String::from_utf16_lossy(&buf[..(len as usize).min(buf.len())]);
    let name = path
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// 非 Windows：没有「进程映像」概念，恒返回 `None`（调用方 fail-closed）。
#[cfg(not(windows))]
fn image_file_name(_pid: u32) -> Option<String> {
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

/// 请求关闭占用 `path` 的进程（task-049），可选在关闭后自动重启被关进程（task-057）。
///
/// 流程：`RmStartSession → RmRegisterResources(只注册这一个文件) → RmGetList →
/// 护栏过滤（PID → 映像名）→ RmShutdown(RmForceShutdown) → RmRestart(bRestartable) → RmEndSession`。
///
/// - 无占用进程：不调 `RmShutdown`，返回空结果（幂等，重复点「重试」不报错）；
/// - 有占用但**任何一个**被护栏拒绝：不调 `RmShutdown`，`closed` 为空、`refused` 完整
///   （mixed fail-closed，见 [`apply_mixed_fail_closed`]）；
/// - 全部可关：调 `RmShutdown`，成功后把这些进程放进 `closed`；
/// - `auto_restart=true` 时，在 `RmShutdown` 之后对 `bRestartable==TRUE` 的 pid
///   调 `RmRestart`（总超时 10s，失败仅 warning 不阻塞删除）。**默认 false**。
///
/// 会话由 [`RmSession`] 的 `Drop` 收尾，任何提前返回都不会泄漏 handle。
pub fn shutdown_locking_processes(path: &str, auto_restart: bool) -> Result<ShutdownOutcome, String> {
    #[cfg(windows)]
    {
        let session = RmSession::start()?;
        session.register(path)?;
        let infos = session.list()?;
        if infos.is_empty() {
            // 幂等：没有占用方就直接收工，不碰 RmShutdown。
            return Ok(ShutdownOutcome::default());
        }

        let lockers: Vec<LockingProcess> = infos.iter().map(to_locking_process).collect();
        let self_pid = std::process::id();
        let mut closable: Vec<LockingProcess> = Vec::new();
        let mut refused: Vec<RefusedProcess> = Vec::new();
        for lp in &lockers {
            match is_protected_process(&lp.app_name, lp.service_name.as_deref(), lp.pid, self_pid) {
                Some(reason) => refused.push(RefusedProcess {
                    pid: lp.pid,
                    app_name: lp.app_name.clone(),
                    reason: reason.to_string(),
                }),
                None => closable.push(lp.clone()),
            }
        }
        let (closable, refused) = apply_mixed_fail_closed(closable, refused);

        if closable.is_empty() {
            // 全被护栏拒绝（或 mixed fail-closed）：连 RmShutdown 都不调（绝不关闭）。
            return Ok(ShutdownOutcome {
                closed: Vec::new(),
                refused,
            });
        }

        session.shutdown()?;

        // task-057：可选自动重启。只对 bRestartable=TRUE 且在 closable 集合里的 pid 调用。
        if auto_restart {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            let closable_pids: std::collections::HashSet<u32> =
                closable.iter().map(|lp| lp.pid).collect();
            for info in &infos {
                if std::time::Instant::now() >= deadline {
                    eprintln!("[rm-restart] 10s 总超时，剩余 pid 不再尝试");
                    break;
                }
                let pid = info.Process.dwProcessId;
                if !closable_pids.contains(&pid) {
                    continue;
                }
                if !rm_restartable(info) {
                    continue;
                }
                if let Err(e) = session.restart(pid) {
                    // 单个重启失败不阻塞主流程（删除已经在 RmShutdown 后立即进行）。
                    eprintln!("[rm-restart] pid={pid} restart failed: {e}");
                }
            }
        }

        Ok(ShutdownOutcome {
            closed: closable,
            refused,
        })
    }
    #[cfg(not(windows))]
    {
        let _ = (path, auto_restart);
        Ok(ShutdownOutcome::default())
    }
}

/// 纯函数：根据 RM_PROCESS_INFO 的 `bRestartable` 字段判断是否可被 RmRestart 拉起。
///
/// 抽成独立函数是为了单测：windows crate 的 `BOOL` 包了 i32，TRUE=1。
#[cfg(windows)]
fn rm_restartable(info: &RM_PROCESS_INFO) -> bool {
    info.bRestartable.as_bool()
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

    /// task-057：对单个 pid 调 `RmRestart`，尝试拉起刚才被 RmShutdown 关掉的进程。
    ///
    /// 只对 `bRestartable==TRUE` 的进程有意义（调用方已过滤）；失败由调用方记录，
    /// 不影响主删除流程。
    fn restart(&self, pid: u32) -> Result<(), String> {
        use windows::Win32::System::RestartManager::RmRestart;

        // SAFETY: handle 同 shutdown；pid 由调用方从 RmGetList 返回值中取出，合法；
        // 状态回调传 None（不关心重启进度）。
        unsafe { RmRestart(self.handle, pid, None) }
            .ok()
            .map_err(|e| format!("RmRestart(pid={pid}): {e}"))
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
    //! task-049 / task-049b 护栏纯函数单测。
    //!
    //! 放在模块内而不是 `engine/tests/`：`is_protected_process` / `decide` 是
    //! `pub(crate)`，跨 crate 的集成测试看不见它（见任务实现备注里的偏差说明）。
    //!
    //! task-049b 起，判定主依据是**映像文件名**，所以绝大多数用例直接调
    //! [`decide`] 并显式注入映像名——结果与跑测试的机器无关（不会因为测试机上
    //! 恰好存在某个 pid 而漂移）。只有「取不到映像名」相关的用例才走
    //! [`is_protected_process`]（真实 `OpenProcess` 路径）。

    use super::{
        apply_mixed_fail_closed, decide, is_protected_process, rm_restartable, LockingProcess,
        RefusedProcess, REASON_PROTECTED, REASON_SYSTEM, REASON_UNKNOWN,
    };

    const SELF: u32 = 4242;

    /// 构造一个普通占用进程（无服务名），用于批量判定用例。
    fn lp(pid: u32, app_name: &str) -> LockingProcess {
        LockingProcess {
            pid,
            app_name: app_name.to_string(),
            service_name: None,
        }
    }

    /// 构造一条「被护栏拒绝」的记录。
    fn refused(pid: u32, app_name: &str, reason: &'static str) -> RefusedProcess {
        RefusedProcess {
            pid,
            app_name: app_name.to_string(),
            reason: reason.to_string(),
        }
    }

    #[test]
    fn allows_normal_process() {
        assert_eq!(
            decide("notepad.exe", None, 1234, SELF, Some("notepad.exe")),
            None
        );
    }

    #[test]
    fn refuses_explorer() {
        assert_eq!(
            decide("explorer.exe", None, 1234, SELF, Some("explorer.exe")),
            Some(REASON_PROTECTED)
        );
    }

    #[test]
    fn refuses_svchost() {
        assert_eq!(
            decide("svchost.exe", None, 1234, SELF, Some("svchost.exe")),
            Some(REASON_PROTECTED)
        );
    }

    #[test]
    fn refuses_protected_name_case_insensitive() {
        assert_eq!(
            decide("EXPLORER.EXE", None, 1234, SELF, Some("EXPLORER.EXE")),
            Some(REASON_PROTECTED)
        );
        assert_eq!(
            decide("MsMpEng.exe", None, 1234, SELF, Some("MsMpEng.exe")),
            Some(REASON_PROTECTED)
        );
    }

    #[test]
    fn refuses_windows_service() {
        assert_eq!(
            decide(
                "anything.exe",
                Some("wuauserv"),
                1234,
                SELF,
                Some("anything.exe")
            ),
            Some("windows service")
        );
    }

    #[test]
    fn refuses_system_pid() {
        // pid 判定先于映像名判定：即使取不到映像名，也给出 system process。
        assert_eq!(decide("System", None, 4, SELF, None), Some(REASON_SYSTEM));
        assert_eq!(decide("Idle", None, 0, SELF, None), Some(REASON_SYSTEM));
    }

    #[test]
    fn refuses_self() {
        assert_eq!(
            decide("cdisk-cleaner.exe", None, SELF, SELF, None),
            Some("self")
        );
    }

    #[test]
    fn refuses_empty_app_name() {
        assert_eq!(
            decide("   ", None, 1234, SELF, Some("notepad.exe")),
            Some(REASON_SYSTEM)
        );
    }

    #[test]
    fn refuses_rm_friendly_name_of_protected_app() {
        // 兜底规则：RM 的 strAppName 是应用友好名（实测 "Windows Explorer"），
        // 即使映像名查到了别的东西，友好名命中名单也必须拦住。
        assert_eq!(
            decide("Windows Explorer", None, 1234, SELF, Some("notepad.exe")),
            Some(REASON_PROTECTED)
        );
    }

    #[test]
    fn allows_rm_friendly_name_of_normal_app() {
        // 反例：真实 RM 输出 "Windows PowerShell"，不能被误判成受保护进程。
        assert_eq!(
            decide(
                "Windows PowerShell",
                None,
                1234,
                SELF,
                Some("powershell.exe")
            ),
            None
        );
    }

    // ---- task-049b 新增（评审 Bug #1：本地化应用名可绕过护栏） ----

    #[test]
    fn refuses_localized_explorer_description() {
        // 任务书 A5 的语义：本地化应用名 + 取不到映像名 → fail-closed。
        // pid 用 `u32::MAX - 1`（必然不存在）而不是 `1234`：`1234` 是合法 PID，
        // 一旦被某个普通进程占用，`OpenProcess` 就会成功、映像名不命中名单、
        // app_name 兜底也命中不了英文名单，断言随即漂移（测试轮实测本机 PID 1234
        // 当时未被占用，属偶然，不能作为用例前提）。
        assert_eq!(
            is_protected_process("Windows 资源管理器", None, u32::MAX - 1, SELF),
            Some(REASON_UNKNOWN),
            "本地化应用名 + 取不到映像名 必须 fail-closed"
        );

        // 真正的修复路径：pid 能取到映像名时，按映像名判定，与语言无关。
        // zh-CN 真机实测：explorer.exe → "Windows 资源管理器"，
        // dwm.exe → "桌面 窗口管理器"，旧的三条名字规则全部不命中。
        assert_eq!(
            decide("Windows 资源管理器", None, 1234, SELF, Some("explorer.exe")),
            Some(REASON_PROTECTED)
        );
        assert_eq!(
            decide("桌面 窗口管理器", None, 1234, SELF, Some("dwm.exe")),
            Some(REASON_PROTECTED)
        );
        // 反向确认：本地化名不是靠「往名单里加中文名」修好的。
        assert_eq!(
            decide("Windows 资源管理器", None, 1234, SELF, Some("notepad.exe")),
            None
        );
    }

    #[test]
    fn refuses_when_image_name_unavailable() {
        // 纯函数层：取不到映像名 → 拒绝（fail-closed），而不是放行。
        assert_eq!(
            decide("notepad.exe", None, 1234, SELF, None),
            Some(REASON_UNKNOWN)
        );
        // 真实 API 层：u32::MAX - 1 必然不是合法进程 → OpenProcess 失败 → 拒绝。
        assert_eq!(
            is_protected_process("notepad.exe", None, u32::MAX - 1, SELF),
            Some(REASON_UNKNOWN)
        );
    }

    // ---- task-049b 新增（评审 Bug #2：mixed 场景 fail-closed） ----

    #[test]
    fn mixed_batch_is_fail_closed() {
        // 混合占用：notepad 可关，但 explorer 被拒绝 → 一个都不关。
        let (closable, refused_list) = apply_mixed_fail_closed(
            vec![lp(1000, "notepad.exe")],
            vec![refused(1001, "Windows 资源管理器", REASON_PROTECTED)],
        );
        assert!(
            closable.is_empty(),
            "mixed 场景必须一个都不关: {closable:?}"
        );
        assert_eq!(refused_list.len(), 1, "refused={refused_list:?}");
        assert_eq!(refused_list[0].pid, 1001);
        assert_eq!(refused_list[0].reason, REASON_PROTECTED);
    }

    #[test]
    fn batch_without_protected_process_closes_all() {
        // 全是普通进程 → 全部可关（fail-closed 不能把正常场景也挡掉）。
        let lockers = vec![lp(1000, "notepad.exe"), lp(1001, "chrome.exe")];
        let (closable, refused_list) = apply_mixed_fail_closed(lockers, Vec::new());
        assert_eq!(closable.len(), 2, "closable={closable:?}");
        assert!(refused_list.is_empty(), "refused={refused_list:?}");
    }

    #[test]
    fn batch_all_refused_stays_refused() {
        // 全部被拒绝 → closable 本来就空，refused 原样保留（不重复包装）。
        let (closable, refused_list) = apply_mixed_fail_closed(
            Vec::new(),
            vec![
                refused(1000, "notepad.exe", REASON_UNKNOWN),
                refused(1001, "explorer.exe", REASON_PROTECTED),
            ],
        );
        assert!(closable.is_empty(), "closable={closable:?}");
        assert_eq!(refused_list.len(), 2, "refused={refused_list:?}");
    }

    // ---- task-057：RmRestart 纯逻辑判定 ----

    #[cfg(windows)]
    #[test]
    fn rm_restartable_只对_b_true_返回_true() {
        use windows::Win32::Foundation::BOOL;
        use windows::Win32::System::RestartManager::RM_PROCESS_INFO;

        let mut yes = RM_PROCESS_INFO::default();
        yes.bRestartable = BOOL(1);
        assert!(rm_restartable(&yes), "bRestartable=TRUE 才允许重启");

        let mut no = RM_PROCESS_INFO::default();
        no.bRestartable = BOOL(0);
        assert!(!rm_restartable(&no), "bRestartable=FALSE 不重启");
    }
}
