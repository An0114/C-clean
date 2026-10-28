// task-055：updater 相关的纯函数（便于单测，不进组件）。
//
// 为什么单独开文件而不是塞进 App.tsx：
// - App.tsx 是组件文件，多导出非组件会让 react-refresh 规则报警（`SafeBox.tsx` 已有 2 条同类 warning，不再增加）；
// - `Settings.tsx` 也要复用失败文案映射，而 `App.tsx` 会 import `Settings`，
//   把函数放 App 里会造成 App ↔ Settings 的循环依赖。

/** 速度估算窗口：每满 1s 用「本窗口增量 / 实际时长」算一次瞬时速度。 */
export const SPEED_WINDOW_MS = 1000;

/** EMA 平滑系数：新样本权重 0.4，历史值权重 0.6。 */
export const SPEED_EMA_ALPHA = 0.4;

/**
 * 下载百分比（0~100 整数）。
 *
 * `total <= 0` 是真实场景——服务器没给 `Content-Length` 时 `Started` 事件的
 * `contentLength` 是 `undefined`，此时无法算百分比，返回 0 让 UI 退化成
 * 「只显示已下载大小」。`downloaded` 超过 `total` 时夹到 100（分块重传可能造成）。
 */
export function computePercent(downloaded: number, total: number): number {
  if (!Number.isFinite(total) || total <= 0) return 0;
  if (!Number.isFinite(downloaded) || downloaded <= 0) return 0;
  return Math.min(100, Math.round((downloaded / total) * 100));
}

/** 字节 → 「X.X MB」（保留 1 位小数）。 */
export function formatMB(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0.0 MB";
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** 字节/秒 → 「X.X MB/s」/「N KB/s」/「N B/s」；非正数返回破折号。 */
export function formatSpeed(bytesPerSec: number): string {
  if (!Number.isFinite(bytesPerSec) || bytesPerSec <= 0) return "—";
  if (bytesPerSec >= 1024 * 1024) {
    return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
  }
  if (bytesPerSec >= 1024) return `${Math.round(bytesPerSec / 1024)} KB/s`;
  return `${Math.round(bytesPerSec)} B/s`;
}
