//! 进度事件通道（ARCHITECTURE §3：scan://progress ≤1 次/100ms 节流）。
//!
//! 本任务只定义 trait、payload 与节流逻辑；真实 Tauri AppHandle emit
//! 实现留到后续任务注入。
//!
//! task-053 G7：`ThrottledEmitter` 是**丢弃式**节流（窗口内事件直接丢、无
//! trailing 补偿）。丢弃对高频中间进度无害，但对「丢了就语义错误」的收尾
//! 事件是致命的——扫描最后一条 `percent = 100` 常与上一条 item 事件同毫秒
//! 级，必被吞。故 trait 增加 [`ProgressEmitter::emit_force`] 作为逃生口。

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
    ///
    /// 允许被节流/去抖实现丢弃——常规高频进度走这里。
    fn emit(&self, payload: &ScanProgress);

    /// 强制发送一条进度：**必须真正到达订阅方**，不允许被节流/去抖丢弃。
    ///
    /// 用于收尾这类「丢了就语义错误」的事件（task-053 G7：扫描结束的
    /// `percent = 100` 一旦被 100ms 节流吞掉，进度条就永远差一截）。
    ///
    /// 默认实现直接转发给 [`emit`](Self::emit)——本身不节流的实现无需覆写。
    fn emit_force(&self, payload: &ScanProgress) {
        self.emit(payload);
    }
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

/// trait 对象透传实现（让 `Box<dyn ProgressEmitter>` 也能当 emitter 用）。
///
/// `emit_force` 必须一并透传：漏掉就会退化成默认实现 → 调 `emit` → 被内层
/// `ThrottledEmitter` 节流吞掉，等于白加了这个方法（task-053 G7.1 回归点）。
impl ProgressEmitter for Box<dyn ProgressEmitter> {
    fn emit(&self, payload: &ScanProgress) {
        self.as_ref().emit(payload);
    }

    fn emit_force(&self, payload: &ScanProgress) {
        self.as_ref().emit_force(payload);
    }
}

/// 100ms 节流包装：距上次 emit <100ms 的直接丢弃，否则转发给内部 emitter。
pub struct ThrottledEmitter<E: ProgressEmitter = Box<dyn ProgressEmitter>> {
    inner: E,
    last: Mutex<Option<Instant>>,
}

impl<E: ProgressEmitter> ThrottledEmitter<E> {
    /// 每 100ms 最多转发一次的最小间隔。
    pub const MIN_INTERVAL_MS: u128 = 100;

    /// 用具体类型 emitter 包装（测试可直接拿到内部引用做断言）。
    pub fn new(inner: E) -> Self {
        Self {
            inner,
            last: Mutex::new(None),
        }
    }

    /// 取出内部 emitter 引用（测试断言用）。
    pub fn inner(&self) -> &E {
        &self.inner
    }
}

impl ThrottledEmitter<Box<dyn ProgressEmitter>> {
    /// 用 trait 对象 emitter 包装（生产代码注入用）。
    pub fn boxed(inner: Box<dyn ProgressEmitter>) -> Self {
        Self {
            inner,
            last: Mutex::new(None),
        }
    }
}

impl<E: ProgressEmitter> ProgressEmitter for ThrottledEmitter<E> {
    fn emit(&self, payload: &ScanProgress) {
        let now = Instant::now();
        let mut guard = self.last.lock().ok();
        let should_send = match guard.as_ref() {
            None => true,
            Some(g) => match &**g {
                None => true,
                Some(prev) => now.duration_since(*prev).as_millis() >= Self::MIN_INTERVAL_MS,
            },
        };
        if should_send {
            if let Some(g) = guard.as_mut() {
                **g = Some(now);
            }
            self.inner.emit(payload);
        }
    }

    /// task-053 G7.1：绕过节流直接转发，并把窗口起点重置为「此刻」。
    ///
    /// 重置窗口是为了**保住节流上限**：若不重置，紧随 force 之后的普通 emit
    /// 会因为「距上次普通 emit 已超 100ms」而立刻再发一条，导致 100ms 窗口内
    /// 出现两次推送。重置后 force 事件本身充当本窗口的唯一次推送。
    fn emit_force(&self, payload: &ScanProgress) {
        // 先落时间戳再发送：`if let` 块结束即释放锁，inner.emit 不持锁。
        if let Ok(mut guard) = self.last.lock() {
            *guard = Some(Instant::now());
        }
        self.inner.emit(payload);
    }
}
