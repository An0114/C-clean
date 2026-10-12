//! task-020：who_is_locking——Windows Restart Manager FFI 查询占用进程 PID。
//!
//! 本模块只做「查询」（RmStartSession / RmRegisterResources / RmGetList / RmEndSession），
//! 不做请求关闭（RmShutdown / RmRestart 留 task-021+）。非 Windows 平台返回空 Vec。

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

/// 查询 `path` 被哪些进程占用。
///
/// Windows：走 Restart Manager 会话，返回占用进程列表。
/// 非 Windows：返回空 Vec（跨平台编译兜底）。
pub fn who_is_locking(path: &str) -> Result<Vec<LockingProcess>, String> {
    #[cfg(windows)]
    {
        who_is_locking_windows(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Ok(Vec::new())
    }
}

#[cfg(windows)]
fn who_is_locking_windows(path: &str) -> Result<Vec<LockingProcess>, String> {
    use windows::core::PCWSTR;
    use windows::Win32::System::RestartManager::{
        RmEndSession, RmGetList, RmRegisterResources, RmStartSession, RM_PROCESS_INFO,
    };

    // 1. 启动会话（handle 是 u32，由 RmStartSession 写入）
    let mut session_handle: u32 = 0;
    let mut session_key = [0u16; 33];
    unsafe {
        RmStartSession(
            &mut session_handle,
            0,
            windows::core::PWSTR(session_key.as_mut_ptr()),
        )
        .ok()
        .map_err(|e| format!("RmStartSession: {e}"))?;
    }

    // 2. 注册文件资源
    let path_w: Vec<u16> = path.encode_utf16().chain([0]).collect();
    let pwstr = PCWSTR(path_w.as_ptr());
    let files = [pwstr];
    if let Err(e) = unsafe { RmRegisterResources(session_handle, Some(&files), None, None) }.ok() {
        unsafe {
            let _ = RmEndSession(session_handle);
        }
        return Err(format!("RmRegisterResources: {e}"));
    }

    // 3. RmGetList：第一次 probe 拿 n_proc_info_needed，分配后第二次真拿。
    let mut n_proc_info_needed: u32 = 0;
    let mut n_proc_info: u32 = 0;
    let mut dw_reboot_reason: u32 = 0;
    let probe = unsafe {
        RmGetList(
            session_handle,
            &mut n_proc_info_needed,
            &mut n_proc_info,
            None,
            &mut dw_reboot_reason,
        )
    };
    // 第一次预期返回 ERROR_MORE_DATA(234)；其他错视为失败。
    if let Err(e) = probe.ok() {
        let code = e.code().0 as u32;
        if code != 234 {
            unsafe {
                let _ = RmEndSession(session_handle);
            }
            return Err(format!("RmGetList (probe, code={code}): {e}"));
        }
    }

    let mut out: Vec<LockingProcess> = Vec::new();
    if n_proc_info_needed > 0 {
        let size = core::mem::size_of::<RM_PROCESS_INFO>();
        let mut buf: Vec<u8> = vec![0u8; size * n_proc_info_needed as usize];
        n_proc_info = n_proc_info_needed;
        if let Err(e) = unsafe {
            RmGetList(
                session_handle,
                &mut n_proc_info_needed,
                &mut n_proc_info,
                Some(buf.as_mut_ptr() as *mut RM_PROCESS_INFO),
                &mut dw_reboot_reason,
            )
        }
        .ok()
        {
            unsafe {
                let _ = RmEndSession(session_handle);
            }
            return Err(format!("RmGetList (fetch): {e}"));
        }

        for i in 0..n_proc_info as usize {
            let p = unsafe { &*(buf.as_ptr().add(i * size) as *const RM_PROCESS_INFO) };
            let app_name = u16_to_string(&p.strAppName);
            let svc_raw = u16_to_string(&p.strServiceShortName);
            let service_name = if svc_raw.is_empty() {
                None
            } else {
                Some(svc_raw)
            };
            out.push(LockingProcess {
                pid: p.Process.dwProcessId,
                app_name,
                service_name,
            });
        }
    }

    // 4. 释放会话
    unsafe {
        let _ = RmEndSession(session_handle);
    }

    Ok(out)
}

/// 把 [u16] UTF-16 缓冲按第一个 NUL 截断转 String。
#[cfg(windows)]
fn u16_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}
