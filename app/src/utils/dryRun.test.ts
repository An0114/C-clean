// task-040：dryRun 预览纯函数单测。
import { describe, it, expect } from "vitest";
import {
  describeDenial,
  formatBreakdownLine,
  formatDryRunHeadline,
  sortBreakdown,
  type BreakdownItem,
  type DryRunResp,
} from "./dryRun";

function resp(partial: Partial<DryRunResp>): DryRunResp {
  return {
    totalItems: 0,
    totalBytes: 0,
    breakdown: [],
    blockedByL3: [],
    ...partial,
  };
}

describe("formatDryRunHeadline", () => {
  it("0 项 → 固定文案", () => {
    expect(formatDryRunHeadline(resp({ totalItems: 0, totalBytes: 0 }))).toBe(
      "没有可预览的条目",
    );
  });

  it("非 0 项 → 项数 + formatBytes 口径", () => {
    expect(
      formatDryRunHeadline(resp({ totalItems: 3, totalBytes: 2 * 1024 * 1024 * 1024 })),
    ).toBe("将清理 3 项，共 2.0 GB");
  });

  it("小于 1KB 走 B 档（与 formatBytes 一致）", () => {
    expect(formatDryRunHeadline(resp({ totalItems: 1, totalBytes: 500 }))).toBe(
      "将清理 1 项，共 500 B",
    );
  });
});

describe("sortBreakdown", () => {
  const rows: BreakdownItem[] = [
    { moduleCode: "npm_cache", category: "cache", bytes: 100 },
    { moduleCode: "temp", category: "files", bytes: 500 },
    { moduleCode: "dev_cache", category: "cache", bytes: 100 },
  ];

  it("按 bytes 倒序", () => {
    expect(sortBreakdown(rows).map((r) => r.bytes)).toEqual([500, 100, 100]);
  });

  it("同 bytes 时按 moduleCode 字典序 → dev_cache 在 npm_cache 前", () => {
    expect(
      sortBreakdown(rows)
        .filter((r) => r.bytes === 100)
        .map((r) => r.moduleCode),
    ).toEqual(["dev_cache", "npm_cache"]);
  });

  it("不改入参数组", () => {
    const before = rows.map((r) => r.moduleCode);
    sortBreakdown(rows);
    expect(rows.map((r) => r.moduleCode)).toEqual(before);
  });
});

describe("formatBreakdownLine", () => {
  it("模块 / 类别 · 大小", () => {
    expect(
      formatBreakdownLine({ moduleCode: "npm_cache", category: "cache", bytes: 2048 }),
    ).toBe("npm_cache / cache · 2.0 KB");
  });
});

describe("describeDenial", () => {
  it("六个已知变体映射为中文", () => {
    expect(describeDenial("L3PersonalDir")).toBe("个人目录（桌面/文档/下载等）");
    expect(describeDenial("L3BrowserProfile")).toBe("浏览器用户数据目录");
    expect(describeDenial("L3BrowserSensitive")).toBe("浏览器敏感文件（书签/密码/历史）");
    expect(describeDenial("L3SystemPath")).toBe("系统关键路径");
    expect(describeDenial("L3ReservedAction")).toBe("动作级禁令");
    expect(describeDenial("PathTraversal")).toBe("路径穿越/非法路径");
  });

  it("ResolveFailed 带参 → 提取原文并加中文前缀", () => {
    expect(describeDenial('ResolveFailed("io error")')).toBe(
      "真实路径解析失败（io error）",
    );
  });

  it("ResolveFailed 无参 → 只给前缀文案", () => {
    expect(describeDenial("ResolveFailed")).toBe("真实路径解析失败");
  });

  it("未知串原样返回（不吞信息、不抛错）", () => {
    expect(describeDenial("SomeFutureVariant")).toBe("SomeFutureVariant");
  });
});
