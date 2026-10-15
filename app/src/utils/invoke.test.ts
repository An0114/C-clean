// task-040：invoke 信封解包行为单测。
//
// 守护两点：
//   1. api() 抛错语义不变（5 个既有页面依赖）；
//   2. apiRaw() 对 2002 不抛错且能读到 data.blockedByL3——dryRun 预览的命门。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { api, apiRaw } from "./invoke";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

beforeEach(() => {
  mockedInvoke.mockReset();
});

describe("api（解包语义不变）", () => {
  it("code=0 → 返回 data", async () => {
    mockedInvoke.mockResolvedValue({ code: 0, message: "ok", data: [1, 2, 3] });
    await expect(api<number[]>("cleanup_list_pending", { limit: 100 })).resolves.toEqual([
      1, 2, 3,
    ]);
  });

  it("code=2002 → 抛 Error，message 取自信封", async () => {
    mockedInvoke.mockResolvedValue({
      code: 2002,
      message: "检测到受保护路径，操作已拒绝（L3 红线）",
      data: { totalItems: 0, totalBytes: 0, breakdown: [], blockedByL3: [] },
    });
    await expect(api("cleanup_dry_run", { itemIds: [1] })).rejects.toThrow(
      "检测到受保护路径，操作已拒绝（L3 红线）",
    );
  });
});

describe("apiRaw（原样返回信封）", () => {
  it("code=0 → 返回整个信封（而非 data）", async () => {
    mockedInvoke.mockResolvedValue({ code: 0, message: "ok", data: { totalItems: 2 } });
    await expect(apiRaw("cleanup_dry_run", { itemIds: [1, 2] })).resolves.toEqual({
      code: 0,
      message: "ok",
      data: { totalItems: 2 },
    });
  });

  it("code=2002 → 不抛错，且 data.blockedByL3 可读", async () => {
    mockedInvoke.mockResolvedValue({
      code: 2002,
      message: "检测到受保护路径，操作已拒绝（L3 红线）",
      data: {
        totalItems: 0,
        totalBytes: 0,
        breakdown: [],
        blockedByL3: [{ itemId: 7, path: "C:\\Windows\\System32\\a.dll", denialReason: "L3SystemPath" }],
      },
    });
    const env = await apiRaw<{
      blockedByL3: { itemId: number; path: string; denialReason: string }[];
    }>("cleanup_dry_run", { itemIds: [7] });

    expect(env.code).toBe(2002);
    expect(env.data?.blockedByL3).toHaveLength(1);
    expect(env.data?.blockedByL3[0].denialReason).toBe("L3SystemPath");
  });

  it("code=2001 → data 为 null（标准失败信封）", async () => {
    mockedInvoke.mockResolvedValue({ code: 2001, message: "待清理清单为空", data: null });
    const env = await apiRaw("cleanup_dry_run", { itemIds: [] });

    expect(env.code).toBe(2001);
    expect(env.data).toBeNull();
  });
});
