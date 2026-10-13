// task-027：unix 秒级时间戳 → 本地可读字符串。
// 非法输入（NaN / 负数）回退 "-"。
export function formatTs(unixSec: number): string {
  if (!Number.isFinite(unixSec) || unixSec <= 0) return "-";
  try {
    return new Date(unixSec * 1000).toLocaleString("zh-CN", { hour12: false });
  } catch {
    return "-";
  }
}
