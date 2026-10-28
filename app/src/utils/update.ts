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

/**
 * updater 原始错误 → 中文文案（task-055 单元 3）。
 *
 * 顺序有意义：先判网络再判签名。「签名校验失败」的原始错误里常常同时带
 * `https://...`（URL），若先判 URL 会被误归为网络问题，用户会一直重试网络
 * 而永远修不好。同理 `disk` 放在 `version` 之前——"no space left" 不含
 * version 词，但 `already` 这类词更罕见，放最后更安全。
 */
export function classifyUpdateError(raw: string): string {
  const s = String(raw ?? "");
  const lower = s.toLowerCase();

  if (/network|request|\burl\b|dns|timeout|connect|offline|fetch|http/i.test(lower)) {
    return "网络连接失败，请检查网络后重试";
  }
  if (/signature|\bkey\b|\bsign\b|signing|pubkey|minisign/i.test(lower)) {
    return "签名校验失败，更新包可能已损坏，请重新下载";
  }
  if (/disk|space|enospc/i.test(lower)) {
    return "磁盘空间不足，请清理 C 盘后重试";
  }
  if (/version|already|up to date/i.test(lower)) {
    return "当前已是最新版本";
  }
  return `更新失败：${s}（请截图反馈）`;
}
