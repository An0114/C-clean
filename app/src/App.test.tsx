// task-055：M3 更新体验的回归用例。
//
// 覆盖任务书第 18 条要求的三组：
//   ① `classifyUpdateError` 5 类文案全覆盖（网络 / 签名 / 磁盘 / 已是最新 / 兜底）；
//   ② `shouldShowBanner` 的「稍后提醒」判断（同版本不弹 / 新版本弹）；
//   ③ `computePercent` 的百分比计算（含 total = 0 兜底）；
// 外加组件级：banner 的出现/关闭、下载进度渲染、失败态文案与重试按钮。
//
// 纯函数用例放这里而不是单独 utils 测试文件，是因为任务书白名单只放行
// `App.test.tsx` / `Settings.test.tsx` 两个测试文件。

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, fireEvent, cleanup, act } from "@testing-library/react";
import App from "./App";
import {
  classifyUpdateError,
  computePercent,
  formatMB,
  formatSpeed,
  DISMISSED_VERSION_KEY,
  shouldShowBanner,
} from "./utils/update";

// ---- mock 外部依赖 ----
// updater / process：真实实现要 Tauri 运行时，jsdom 里没有。
// api/core：App 默认渲染 Dashboard，它会 invoke("audit_log_stats")。
vi.mock("@tauri-apps/plugin-updater", () => ({ check: vi.fn() }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { check } from "@tauri-apps/plugin-updater";
import { invoke } from "@tauri-apps/api/core";

const mockedCheck = vi.mocked(check);
const mockedInvoke = vi.mocked(invoke);

const ok = (data: unknown) => ({ code: 0, message: "ok", data });

/** 造一个假 Update 对象（字段够 App 用即可）。 */
function makeUpdate(version = "0.1.7") {
  return {
    version,
    body: "修复若干问题",
    date: "2026-10-28",
    currentVersion: "0.1.6",
    rawJson: {},
    rid: 1,
    downloadAndInstall: vi.fn(async () => {}),
    download: vi.fn(async () => {}),
    install: vi.fn(async () => {}),
    close: vi.fn(async () => {}),
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  mockedInvoke.mockResolvedValue(
    ok({ totalBytesFreed: 0, successCount: 0, failedCount: 0, sessionCount: 0 }) as never
  );
});

afterEach(() => {
  cleanup();
});

// ============ ① 失败文案映射（5 类 + 兜底）============

describe("classifyUpdateError 五类文案", () => {
  it("网络类 → 网络连接失败", () => {
    expect(classifyUpdateError("Network error: connection refused")).toBe(
      "网络连接失败，请检查网络后重试"
    );
    expect(classifyUpdateError("Could not fetch a valid release JSON")).toBe(
      "网络连接失败，请检查网络后重试"
    );
  });

  it("签名类 → 签名校验失败", () => {
    expect(classifyUpdateError("Signature error: invalid signature")).toBe(
      "签名校验失败，更新包可能已损坏，请重新下载"
    );
    // 带 URL 的签名错误不能被误判成网络问题（判定顺序：网络在前，但 URL 需带
    // 真正的网络关键词才命中；这条原文只有 signature）。
    expect(classifyUpdateError("minisign verification failed")).toBe(
      "签名校验失败，更新包可能已损坏，请重新下载"
    );
  });

  it("磁盘类 → 磁盘空间不足", () => {
    expect(classifyUpdateError("No space left on device (os error 112)")).toBe(
      "磁盘空间不足，请清理 C 盘后重试"
    );
    expect(classifyUpdateError("ENOSPC")).toBe("磁盘空间不足，请清理 C 盘后重试");
  });

  it("版本类 → 已是最新", () => {
    expect(classifyUpdateError("The update is already up to date")).toBe(
      "当前已是最新版本"
    );
  });

  it("兜底 → 保留原文并提示截图", () => {
    const out = classifyUpdateError("Some weird failure 0x8007");
    expect(out).toContain("更新失败：Some weird failure 0x8007");
    expect(out).toContain("请截图反馈");
  });
});

// ============ ② 稍后提醒判断 ============

describe("shouldShowBanner（稍后提醒）", () => {
  it("没有忽略记录 → 弹", () => {
    expect(shouldShowBanner("0.1.7", null)).toBe(true);
  });

  it("忽略的是同一版本 → 不弹", () => {
    expect(shouldShowBanner("0.1.7", "0.1.7")).toBe(false);
  });

  it("忽略的是旧版本（现在有更新的）→ 弹", () => {
    expect(shouldShowBanner("0.1.8", "0.1.7")).toBe(true);
  });
});

// ============ ③ 百分比 / 格式化 ============

describe("computePercent", () => {
  it("正常比例取整", () => {
    expect(computePercent(50, 100)).toBe(50);
    expect(computePercent(1, 3)).toBe(33);
  });

  it("total = 0（服务器没给 Content-Length）→ 0，不除零", () => {
    expect(computePercent(1024, 0)).toBe(0);
    expect(computePercent(0, 0)).toBe(0);
  });

  it("downloaded 超过 total → 夹到 100", () => {
    expect(computePercent(150, 100)).toBe(100);
  });

  it("负数 / NaN 兜底", () => {
    expect(computePercent(-1, 100)).toBe(0);
    expect(computePercent(NaN, 100)).toBe(0);
  });
});

describe("formatMB / formatSpeed", () => {
  it("formatMB 保留 1 位小数", () => {
    expect(formatMB(0)).toBe("0.0 MB");
    expect(formatMB(1024 * 1024)).toBe("1.0 MB");
    expect(formatMB(4.25 * 1024 * 1024)).toBe("4.3 MB");
  });

  it("formatSpeed 按量级切换单位", () => {
    expect(formatSpeed(0)).toBe("—");
    expect(formatSpeed(512)).toBe("512 B/s");
    expect(formatSpeed(1536)).toBe("2 KB/s");
    expect(formatSpeed(2 * 1024 * 1024)).toBe("2.0 MB/s");
  });
});

// ============ 组件级 ============

describe("App 更新 banner", () => {
  it("发现新版本且未忽略 → 显示 banner", async () => {
    const u = makeUpdate("0.1.7");
    mockedCheck.mockResolvedValue(u as never);

    render(<App />);

    expect(await screen.findByText("发现新版本 v0.1.7")).toBeInTheDocument();
  });

  it("该版本已被「稍后」忽略 → 不显示 banner", async () => {
    localStorage.setItem(DISMISSED_VERSION_KEY, "0.1.7");
    mockedCheck.mockResolvedValue(makeUpdate("0.1.7") as never);

    render(<App />);

    // 等 Dashboard 首屏出来（说明挂载期 effect 都跑过），再断言 banner 不在。
    expect(await screen.findByText("总释放空间")).toBeInTheDocument();
    await waitFor(() => expect(mockedCheck).toHaveBeenCalled());
    expect(screen.queryByText(/发现新版本/)).not.toBeInTheDocument();
  });

  it("点「稍后」→ banner 消失且版本号写入 localStorage", async () => {
    mockedCheck.mockResolvedValue(makeUpdate("0.1.7") as never);
    render(<App />);

    fireEvent.click(await screen.findByText("稍后"));

    await waitFor(() =>
      expect(screen.queryByText("发现新版本 v0.1.7")).not.toBeInTheDocument()
    );
    expect(localStorage.getItem(DISMISSED_VERSION_KEY)).toBe("0.1.7");
  });

  it("下载中显示进度条百分比 + 已下载/总量", async () => {
    const total = 4 * 1024 * 1024;
    const u = makeUpdate("0.1.7");
    u.downloadAndInstall.mockImplementation(async (onEvent?: (e: unknown) => void) => {
      onEvent?.({ event: "Started", data: { contentLength: total } });
      onEvent?.({ event: "Progress", data: { chunkLength: total / 4 } });
    });
    mockedCheck.mockResolvedValue(u as never);

    render(<App />);
    fireEvent.click(await screen.findByText("立即更新"));

    expect(await screen.findByText("下载更新中 25%")).toBeInTheDocument();
    expect(screen.getByText(/已下载 1\.0 MB \/ 4\.0 MB/)).toBeInTheDocument();
  });

  it("total 未知时只显示已下载大小，不显示 0% 误导", async () => {
    const u = makeUpdate("0.1.7");
    u.downloadAndInstall.mockImplementation(async (onEvent?: (e: unknown) => void) => {
      // 不传 contentLength 模拟「服务器没给 Content-Length」
      onEvent?.({ event: "Started", data: {} });
      onEvent?.({ event: "Progress", data: { chunkLength: 512 * 1024 } });
    });
    mockedCheck.mockResolvedValue(u as never);

    render(<App />);
    fireEvent.click(await screen.findByText("立即更新"));

    expect(await screen.findByText(/总大小未知/)).toBeInTheDocument();
    expect(screen.getByText(/已下载 0\.5 MB/)).toBeInTheDocument();
  });

  it("下载失败 → 中文文案 + 「重试」「关闭」按钮", async () => {
    const u = makeUpdate("0.1.7");
    u.downloadAndInstall.mockRejectedValue(new Error("Network error"));
    mockedCheck.mockResolvedValue(u as never);

    render(<App />);
    fireEvent.click(await screen.findByText("立即更新"));

    expect(
      await screen.findByText("网络连接失败，请检查网络后重试")
    ).toBeInTheDocument();
    expect(screen.getByText("重试")).toBeInTheDocument();
    expect(screen.getByText("关闭")).toBeInTheDocument();
  });

  it("点「重试」再次调用 downloadAndInstall", async () => {
    const u = makeUpdate("0.1.7");
    u.downloadAndInstall.mockRejectedValue(new Error("Network error"));
    mockedCheck.mockResolvedValue(u as never);

    render(<App />);
    fireEvent.click(await screen.findByText("立即更新"));
    await screen.findByText("重试");

    u.downloadAndInstall.mockClear();
    await act(async () => {
      fireEvent.click(screen.getByText("重试"));
    });

    expect(u.downloadAndInstall).toHaveBeenCalledTimes(1);
  });

  it("Finished 事件后切「安装中，即将重启…」且按钮禁用", async () => {
    const u = makeUpdate("0.1.7");
    // 用「不 resolve」的 promise 把 phase 停在 installing，避免 relaunch 后被重置
    u.downloadAndInstall.mockImplementation((onEvent?: (e: unknown) => void) => {
      onEvent?.({ event: "Started", data: { contentLength: 1024 } });
      onEvent?.({ event: "Progress", data: { chunkLength: 1024 } });
      onEvent?.({ event: "Finished" });
      return new Promise<void>(() => {});
    });
    mockedCheck.mockResolvedValue(u as never);

    render(<App />);
    fireEvent.click(await screen.findByText("立即更新"));

    expect(await screen.findByText("安装中，即将重启…")).toBeInTheDocument();
    const btn = screen.getByText("安装中…");
    expect(btn).toBeDisabled();
  });
});

// task-061：仪表盘趋势图
describe("仪表盘趋势图（task-061）", () => {
  /** 区分命令的 mock：stats 给固定值，trend 由参数决定。 */
  function mockDashboard(trend: unknown) {
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "audit_log_stats") {
        return Promise.resolve(
          ok({ totalBytesFreed: 0, successCount: 0, failedCount: 0, sessionCount: 0 })
        );
      }
      if (cmd === "cleanup_stats_trend") return Promise.resolve(ok(trend));
      if (cmd === "get_prefs") return Promise.resolve(ok({ theme: "light" }));
      return Promise.resolve(ok(null));
    }) as never);
  }

  it("有数据时显示标题、首尾日期与峰值", async () => {
    mockDashboard([
      { day: "2026-10-27", bytesFreed: 1024 * 1024 },
      { day: "2026-10-28", bytesFreed: 2 * 1024 * 1024 },
      { day: "2026-10-29", bytesFreed: 512 * 1024 },
    ]);

    render(<App />);

    expect(await screen.findByText(/最近 30 天释放趋势/)).toBeInTheDocument();
    // 峰值取最大那天（2 MB）
    expect(screen.getByText(/峰值 2\.0 MB/)).toBeInTheDocument();
    // 首尾日期各出现一次
    expect(screen.getByText("2026-10-27")).toBeInTheDocument();
    expect(screen.getByText("2026-10-29")).toBeInTheDocument();
    // 三个柱子（rect 有 title）
    expect(screen.getByLabelText(/最近 30 天释放趋势柱状图/)).toBeInTheDocument();
  });

  it("空数据时显示「暂无清理记录」占位", async () => {
    mockDashboard([]);

    render(<App />);

    expect(await screen.findByText("暂无清理记录")).toBeInTheDocument();
  });

  it("趋势接口失败时显示失败占位，不影响卡片", async () => {
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "audit_log_stats") {
        return Promise.resolve(
          ok({ totalBytesFreed: 0, successCount: 0, failedCount: 0, sessionCount: 0 })
        );
      }
      if (cmd === "cleanup_stats_trend") return Promise.reject(new Error("boom"));
      if (cmd === "get_prefs") return Promise.resolve(ok({ theme: "light" }));
      return Promise.resolve(ok(null));
    }) as never);

    render(<App />);

    expect(await screen.findByText(/趋势数据加载失败/)).toBeInTheDocument();
    // 卡片仍在
    expect(screen.getByText("总释放空间")).toBeInTheDocument();
  });
});

// ============ ④ tab 切换（task-066：大文件地图）============
//
// 任务书验收 16「单测：tab 切换正确」。App.test.tsx 是唯一能测 tab 的地方
// （tab 状态在 App 里），所以本任务为它新增这一条——已在任务书白名单偏差里登记。

describe("tab 切换（task-066 大文件）", () => {
  it("Tab 栏有「大文件」按钮，点击后渲染大文件地图页", async () => {
    // App 默认渲染 Dashboard；切到「大文件」后 BigFiles 会调 big_files_scan。
    // 默认 mock 对所有命令都回 stats 对象，BigFiles 拿到会崩，所以这里按命令分派。
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "big_files_scan") {
        return Promise.resolve(
          ok({
            categories: [
              {
                path: "C://hiberfil.sys",
                sizeBytes: 1024,
                category: "hiberfil",
                recommendation: "powercfg /h off",
              },
              {
                path: "C://Windows.old",
                sizeBytes: 0,
                category: "windows_old",
                recommendation: "不存在",
              },
              {
                path: "C://$Recycle.Bin",
                sizeBytes: 0,
                category: "recycle_bin",
                recommendation: "清空回收站",
              },
            ],
            topFiles: [],
            totalBytes: 1024,
            scannedEntries: 3,
            truncated: false,
            elapsedMs: 5,
          })
        );
      }
      return Promise.resolve(
        ok({ totalBytesFreed: 0, successCount: 0, failedCount: 0, sessionCount: 0 })
      );
    }) as unknown as typeof invoke);

    render(<App />);

    // 按钮存在（验收 14）
    const btn = await screen.findByText("大文件");
    fireEvent.click(btn);

    // 切过去之后渲染的是大文件页，不是仪表盘（验收 15）
    expect(await screen.findByText("大文件地图")).toBeTruthy();
    expect(screen.getByTestId("category-hiberfil")).toBeTruthy();
    expect(screen.queryByText("清理统计")).toBeNull();
  });
});
