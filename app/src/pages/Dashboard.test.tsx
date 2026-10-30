// task-065：WinSxS 组件存储卡片的交互回归用例。
//
// 为什么放组件级：本机 WebView2 渲染进程起不来（详见 docs/test-reports/ 历史
// 报告的环境限制），真机点击取证拿不到；而 WinSxS 清理会真的删组件存储、
// 还会弹 UAC，**恰恰不能**为了取证反复真跑。所以：
//   - 「UI 渲染 + 确认框 + 2007 重试」这些纯前端路径用 jsdom 固化（本文件）；
//   - 「DISM 真的跑起来」那条由任务书的真机/打包冒烟覆盖。
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, fireEvent, cleanup, act } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import Dashboard from "./Dashboard";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);
const mockedListen = vi.mocked(listen);

const GB = 1024 ** 3;
const MB = 1024 ** 2;

const ok = (data: unknown) => ({ code: 0, message: "ok", data });

/** 与 engine WinsxsAnalysis（camelCase）对齐的样例。 */
const ANALYSIS = {
  lastCleanup: "2021-03-15 08:21:46",
  reclaimablePackages: 2,
  actualSizeBytes: 7.36 * GB,
  backupsAndDisabledBytes: 1.2 * GB,
  cacheAndTempBytes: 30.15 * MB,
  reclaimableBytes: 1.2 * GB,
  recommendedCleanup: true,
  parsed: true,
  rawOutput: "Actual Size of Component Store : 7.36 GB",
};

const NOTHING_TO_CLEAN = {
  ...ANALYSIS,
  reclaimablePackages: 0,
  reclaimableBytes: 0,
  recommendedCleanup: false,
};

const CLEAN_RESULT = {
  ok: true,
  exitCode: 0,
  restorePointSeq: 42,
  outputTail: "The operation completed successfully.",
  lineCount: 7,
};

interface WinsxsProgress {
  seq: number;
  line: string;
}

/** 捕获 winsxs://progress 的监听回调，让用例可以「假装 DISM 吐了一行」。 */
let progressHandlers: Array<(e: { payload: WinsxsProgress }) => void> = [];

function mockInvoke(overrides: Record<string, () => Promise<unknown>> = {}) {
  mockedInvoke.mockImplementation(((cmd: string) => {
    if (overrides[cmd]) return overrides[cmd]();
    if (cmd === "audit_log_stats") {
      return Promise.resolve(
        ok({ totalBytesFreed: 0, successCount: 0, failedCount: 0, sessionCount: 0 })
      );
    }
    if (cmd === "drive_usage") {
      return Promise.resolve(ok({ totalBytes: 0, usedBytes: 0, freeBytes: 0 }));
    }
    if (cmd === "cleanup_stats_trend") return Promise.resolve(ok([]));
    return Promise.resolve(ok(null));
  }) as unknown as typeof invoke);
}

/** 取出某个命令的全部调用实参。 */
function callsOf(cmd: string): Array<Record<string, unknown> | undefined> {
  return mockedInvoke.mock.calls
    .filter(([c]) => c === cmd)
    .map(([, args]) => args as Record<string, unknown> | undefined);
}

beforeEach(() => {
  mockedInvoke.mockReset();
  mockedListen.mockReset();
  progressHandlers = [];
  mockedListen.mockImplementation((async (
    event: string,
    handler: (e: { payload: WinsxsProgress }) => void
  ) => {
    if (event === "winsxs://progress") progressHandlers.push(handler);
    return () => {};
  }) as unknown as typeof listen);
  vi.spyOn(window, "confirm").mockReturnValue(true);
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("WinSxS 组件存储卡片", () => {
  it("挂载时不自动分析（避免一进页面就弹 UAC）", async () => {
    mockInvoke();
    render(<Dashboard />);

    // 按钮在 = 卡片渲染好了；关键是没有自动发起分析
    expect(await screen.findByText("分析组件存储")).toBeTruthy();
    expect(callsOf("winsxs_analyze")).toHaveLength(0);
  });

  it("分析后显示可清理空间、可回收包数与 DISM 建议", async () => {
    mockInvoke({ winsxs_analyze: () => Promise.resolve(ok(ANALYSIS)) });
    render(<Dashboard />);

    fireEvent.click(await screen.findByText("分析组件存储"));

    // formatBytes(1.2 GB) = "1.2 GB"
    expect(await screen.findByText("1.2 GB")).toBeTruthy();
    expect(screen.getByText("可回收包")).toBeTruthy();
    expect(screen.getByText("建议清理")).toBeTruthy();
    expect(screen.getByText("2021-03-15 08:21:46")).toBeTruthy();
    expect(screen.getByText("立即清理")).toBeTruthy();
  });

  it("分析失败时展示 DISM 的真实原因，且不留旧结果", async () => {
    mockInvoke({
      winsxs_analyze: () => Promise.reject("DISM 分析失败（退出码 740）：需要提升权限"),
    });
    render(<Dashboard />);

    fireEvent.click(await screen.findByText("分析组件存储"));

    expect(await screen.findByText("分析失败")).toBeTruthy();
    expect(screen.getByText(/退出码 740/)).toBeTruthy();
    // 失败后不该还挂着「立即清理」按钮（旧结果会误导用户）
    expect(screen.queryByText("立即清理")).toBeNull();
  });

  it("没有可回收组件时「立即清理」禁用并给出说明", async () => {
    mockInvoke({ winsxs_analyze: () => Promise.resolve(ok(NOTHING_TO_CLEAN)) });
    render(<Dashboard />);

    fireEvent.click(await screen.findByText("分析组件存储"));
    const btn = (await screen.findByText("立即清理")) as HTMLButtonElement;

    expect(btn.disabled).toBe(true);
    expect(screen.getByText("当前没有可回收的组件，无需清理。")).toBeTruthy();
  });

  it("点「立即清理」先弹确认，确认后调 winsxs_cleanup 并展示结果", async () => {
    mockInvoke({
      winsxs_analyze: () => Promise.resolve(ok(ANALYSIS)),
      winsxs_cleanup: () => Promise.resolve(ok(CLEAN_RESULT)),
    });
    render(<Dashboard />);

    fireEvent.click(await screen.findByText("分析组件存储"));
    fireEvent.click(await screen.findByText("立即清理"));

    await waitFor(() => expect(window.confirm).toHaveBeenCalled());
    expect(await screen.findByText("组件存储清理完成")).toBeTruthy();
    expect(screen.getByText(/系统还原点 #42/)).toBeTruthy();
    expect(screen.getByText(/7 行 DISM 输出/)).toBeTruthy();
    expect(callsOf("winsxs_cleanup")).toEqual([{ skipRestorePoint: false }]);
  });

  it("用户取消确认时完全不调用清理命令", async () => {
    vi.mocked(window.confirm).mockReturnValue(false);
    mockInvoke({
      winsxs_analyze: () => Promise.resolve(ok(ANALYSIS)),
      winsxs_cleanup: () => Promise.resolve(ok(CLEAN_RESULT)),
    });
    render(<Dashboard />);

    fireEvent.click(await screen.findByText("分析组件存储"));
    fireEvent.click(await screen.findByText("立即清理"));

    await waitFor(() => expect(window.confirm).toHaveBeenCalled());
    expect(callsOf("winsxs_cleanup")).toHaveLength(0);
    expect(screen.queryByText("组件存储清理完成")).toBeNull();
  });

  it("还原点创建失败（2007）时二次确认，同意后带 skipRestorePoint 重调", async () => {
    let round = 0;
    mockInvoke({
      winsxs_analyze: () => Promise.resolve(ok(ANALYSIS)),
      winsxs_cleanup: () => {
        round += 1;
        return Promise.resolve(
          round === 1
            ? { code: 2007, message: "系统还原点创建失败", data: null }
            : ok({ ...CLEAN_RESULT, restorePointSeq: null })
        );
      },
    });
    render(<Dashboard />);

    fireEvent.click(await screen.findByText("分析组件存储"));
    fireEvent.click(await screen.findByText("立即清理"));

    expect(await screen.findByText("组件存储清理完成")).toBeTruthy();
    // 第一次确认（清理）+ 第二次确认（还原点失败是否继续）
    expect(window.confirm).toHaveBeenCalledTimes(2);
    expect(callsOf("winsxs_cleanup")).toEqual([
      { skipRestorePoint: false },
      { skipRestorePoint: true },
    ]);
    // 跳过还原点时如实显示「未创建」，不能伪装成建成了
    expect(screen.getByText(/本次未创建系统还原点/)).toBeTruthy();
  });

  it("还原点失败后用户选择取消，不重试并提示未执行", async () => {
    vi.mocked(window.confirm).mockReturnValueOnce(true).mockReturnValueOnce(false);
    mockInvoke({
      winsxs_analyze: () => Promise.resolve(ok(ANALYSIS)),
      winsxs_cleanup: () => Promise.resolve({ code: 2007, message: "系统还原点创建失败", data: null }),
    });
    render(<Dashboard />);

    fireEvent.click(await screen.findByText("分析组件存储"));
    fireEvent.click(await screen.findByText("立即清理"));

    expect(await screen.findByText("清理未完成")).toBeTruthy();
    expect(screen.getByText(/已取消：系统还原点创建失败/)).toBeTruthy();
    // 只调了一次（用户拒绝后不再重试）
    expect(callsOf("winsxs_cleanup")).toHaveLength(1);
  });

  it("清理过程中把 DISM 输出行实时铺出来", async () => {
    let resolveClean: (v: unknown) => void = () => {};
    const pending = new Promise((r) => {
      resolveClean = r;
    });
    mockInvoke({
      winsxs_analyze: () => Promise.resolve(ok(ANALYSIS)),
      winsxs_cleanup: () => pending,
    });
    render(<Dashboard />);

    fireEvent.click(await screen.findByText("分析组件存储"));
    fireEvent.click(await screen.findByText("立即清理"));

    // 订阅建好之前不会有进度面板；等 listen 被调用
    await waitFor(() => expect(progressHandlers).toHaveLength(1));
    expect(await screen.findByText(/正在创建系统还原点/)).toBeTruthy();

    act(() => {
      progressHandlers[0]({ payload: { seq: 0, line: "Cleaning up component store" } });
    });
    expect(await screen.findByText("Cleaning up component store")).toBeTruthy();

    await act(async () => {
      resolveClean(ok(CLEAN_RESULT));
      await pending;
    });
    expect(await screen.findByText("组件存储清理完成")).toBeTruthy();
  });
});
