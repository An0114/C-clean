// task-040：dry-run 预览纯函数层。
//
// 只做「后端 DryRunResp → 用户可见文案」的转换，不碰 IPC、不产生任何副作用。
// 字段命名与后端 `#[serde(rename_all = "camelCase")]` 逐一对齐
// （见 engine/src/ipc/commands/dry_run.rs 的 DryRunResp / BreakdownItem / BlockedItem）。

import { formatBytes } from "./formatBytes";

/** 后端 `BreakdownItem`：按 模块+类别 聚合的一行。 */
export interface BreakdownItem {
  moduleCode: string;
  category: string;
  bytes: number;
}

/** 后端 `BlockedItem`：被 L3 红线拦下的一行；`denialReason` 是 `DenialReason` 的 Debug 串。 */
export interface BlockedItem {
  itemId: number;
  path: string;
  denialReason: string;
}

/** 后端 `DryRunResp`。注意 2002（L3 拒绝）时该结构在信封 `data` 里仍是非 null。 */
export interface DryRunResp {
  totalItems: number;
  totalBytes: number;
  breakdown: BreakdownItem[];
  blockedByL3: BlockedItem[];
}

/** 汇总标题：`将清理 3 项，共 1.2 GB`；无可预览条目时给固定文案。 */
export function formatDryRunHeadline(r: DryRunResp): string {
  if (r.totalItems === 0) return "没有可预览的条目";
  return `将清理 ${r.totalItems} 项，共 ${formatBytes(r.totalBytes)}`;
}

/**
 * 分类明细排序：`bytes` 倒序；同 `bytes` 按 `moduleCode`、再按 `category` 字典序。
 * 返回新数组，不改入参数组。
 */
export function sortBreakdown(rows: BreakdownItem[]): BreakdownItem[] {
  return [...rows].sort((a, b) => {
    if (b.bytes !== a.bytes) return b.bytes - a.bytes;
    if (a.moduleCode !== b.moduleCode) return a.moduleCode < b.moduleCode ? -1 : 1;
    if (a.category !== b.category) return a.category < b.category ? -1 : 1;
    return 0;
  });
}

/** 分类明细单行：`npm_cache / cache · 1.2 GB`。 */
export function formatBreakdownLine(row: BreakdownItem): string {
  return `${row.moduleCode} / ${row.category} · ${formatBytes(row.bytes)}`;
}

/** 已知 `DenialReason` 变体（Debug 串）→ 中文原因。 */
const DENIAL_TEXT: Record<string, string> = {
  L3PersonalDir: "个人目录（桌面/文档/下载等）",
  L3BrowserProfile: "浏览器用户数据目录",
  L3BrowserSensitive: "浏览器敏感文件（书签/密码/历史）",
  L3SystemPath: "系统关键路径",
  L3ReservedAction: "动作级禁令",
  PathTraversal: "路径穿越/非法路径",
};

/** `ResolveFailed(String)` 变体前缀——带参数，无法走查表，须单独判。 */
const RESOLVE_FAILED_PREFIX = "ResolveFailed";

/**
 * 红线原因 Debug 串 → 中文。
 *
 * 未知串原样返回：宁可显示原始标识，也不吞掉信息或抛错。
 */
export function describeDenial(reason: string): string {
  // 顺序要紧：ResolveFailed 带参数（如 `ResolveFailed("io error")`），须先于查表判断。
  if (reason.startsWith(RESOLVE_FAILED_PREFIX)) {
    const inner = reason
      .slice(RESOLVE_FAILED_PREFIX.length)
      .trim()
      .replace(/^\((.*)\)$/, "$1") // 去 Debug 的外层括号
      .trim()
      .replace(/^"(.*)"$/, "$1"); // 去 Debug 的字符串引号
    return inner ? `真实路径解析失败（${inner}）` : "真实路径解析失败";
  }
  return DENIAL_TEXT[reason] ?? reason;
}
