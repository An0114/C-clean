//! 进度事件通道（ARCHITECTURE §3：scan://progress ≤1 次/100ms 节流）。
//!
//! 本任务只定义 trait、payload 与节流逻辑；真实 Tauri AppHandle emit
//! 实现留到后续任务注入。

use std::sync::Mutex;
use std::time::Instant;

/// 事件名常量：Rust → 前端的扫描进度推送。
pub const SCAN_PROGRESS_EVENT: &str = "scan://progress";

/// 进度事件负载（事件 payload）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    /// 会话 id（scan.start 返回值）。
    pub session_id: i64,
    /// 0~100。
    pub percent: u8,
    /// 当前正在扫描的路径。
    pub current_path: String,
    /// 已扫描条目数。
    pub scanned_items: u64,
}

/// 进度发送器抽象：真实实现用 Tauri AppHandle emit，测试用内存实现。
pub trait ProgressEmitter: Send + Sync + 'static {
    /// 发送一条进度（实现方必须非阻塞、不 panic）。
    fn emit(&self, payload: &ScanProgress);
}

/// 吞掉所有 emit 的空实现（默认）。
#[derive(Default)]
pub struct NoopEmitter;

impl ProgressEmitter for NoopEmitter {
    fn emit(&self, _payload: &ScanProgress) {}
}

/// 内存记录实现：把 emit 的 payload 全量 push 进 Vec（测试断言用）。
#[derive(Default)]
pub struct VecEmitter {
    items: Mutex<Vec<ScanProgress>>,
}

impl VecEmitter {
    /// 取出已记录的 payload 快照。
    pub fn snapshot(&self) -> Vec<ScanProgress> {
        self.items.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

impl ProgressEmitter for VecEmitter {
    fn emit(&self, payload: &ScanProgress) {
        if let Ok(mut g) = self.items.lock() {
            g.push(payload.clone());
        }
    }
}

/// 100ms 节流包装：距上次 emit <100ms 的直接丢弃，否则转发给内部 emitter。
pub struct ThrottledEmitter {
    inner: Box<dyn ProgressEmitter>,
    last: Mutex<Option<Instant>>,
}

impl ThrottledEmitter {
    /// 每 100ms 最多转发一次的最小间隔。
    pub const MIN_INTERVAL_MS: u128 = 100;

    /// 用任意 emitter 包装一层节流。
    pub fn new(inner: Box<dyn ProgressEmitter>) -> Self {
        Self {
            inner,
            last: Mutex::new(None),
        }
    }
}

impl ProgressEmitter for ThrottledEmitter {
    fn emit(&self, payload: &ScanProgress) {
        let now = Instant::now();
        let mut guard = self.last.lock().ok();
        let should_send = match guard.as_ref() {
            // 锁中毒：保守放行，不吞进度。
            None => true,
            Some(g) => match &**g {
                None => true,
                Some(prev) => {
                    now.duration_since(*prev).as_millis() >= Self::MIN_INTERVAL_MS
                }
            },
        };
        if should_send {
            if let Some(g) = guard.as_mut() {
                **g = Some(now);
            }
            self.inner.emit(payload);
        }
    }
}
