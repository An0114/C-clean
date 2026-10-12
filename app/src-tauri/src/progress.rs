//! 真实进度发送器（task-007）：把 engine 的 `ProgressEmitter` trait 落到 Tauri 上。
//!
//! 架构边界：本文件位于 `app/src-tauri`，允许依赖 tauri；`engine/` 侧不感知 tauri。

use engine::ipc::progress::{ProgressEmitter, ScanProgress, SCAN_PROGRESS_EVENT};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

/// 包装 `tauri::AppHandle` 的真实进度发送器。
///
/// emit 失败（如窗口已关）仅吞掉，不 panic、不返回 Result——
/// 进度通道绝不能影响主命令返回。
pub struct TauriProgressEmitter {
    app: AppHandle,
}

impl ProgressEmitter for TauriProgressEmitter {
    fn emit(&self, payload: &ScanProgress) {
        // 事件名直接引用 engine 常量，避免两处硬编码漂移。
        if let Err(e) = self.app.emit(SCAN_PROGRESS_EVENT, payload) {
            eprintln!("[progress] emit {} failed: {e}", SCAN_PROGRESS_EVENT);
        }
    }
}

/// 全局进度 State：100ms 节流包装 + 真实 Tauri emitter。
///
/// 由 Builder 的 setup 闭包构造后 `app.manage(...)` 注入，
/// 命令侧通过 `tauri::State<'_, ProgressState>` 取出。
pub struct ProgressState(pub Mutex<engine::ipc::progress::ThrottledEmitter<TauriProgressEmitter>>);

impl ProgressState {
    /// 用指定 AppHandle 构造（setup 闭包内调用）。
    pub fn new(app: AppHandle) -> Self {
        Self(Mutex::new(engine::ipc::progress::ThrottledEmitter::new(
            TauriProgressEmitter { app },
        )))
    }
}
