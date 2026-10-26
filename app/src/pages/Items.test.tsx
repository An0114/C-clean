// task-049：Items 页「清理失败 → 展示占用进程 → 关闭并重试」的交互回归用例。
//
// 为什么放在组件级：本机测试会话里 WebView2 渲染进程起不来（无活动桌面，
// 详见 docs/test-reports/049-test-report.md 的 L3/L4 环境限制），CDP 与截图都
// 拿不到，GUI 点击路径只能用 jsdom 固化。覆盖任务书 C8–C12 的 DOM 行为：
//   C8/C9  接住 ExecuteSummary、失败明细（路径 + 原因 + 占用进程）；
//   C10    confirm 文案含进程清单与「未保存的数据可能丢失」，取消不调命令；
//   C11    重试后绿条 + 受保护进程提示；
//   C12    三态互斥（预览中重试按钮禁用）。
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, fireEvent, cleanup } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import Items from "./Items";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

const ROWS = [
  { id: 1, path: "C:\\Temp\\locked.txt", sizeBytes: 12, moduleCode: "A", sessionId: 1 },
];

/** 信封成功包装（与 engine::ipc::Envelope 对齐）。 */
const ok = (data: unknown) => ({ code: 0, message: "ok", data });

/** task-052 G2：cleanup_list_pending 的分页响应 shape（PendingPage）。 */
const pageOf = (items: unknown[], total: number, page = 1, pageSize = 100) =>
  ok({ items, total, page, pageSize });

/** cleanup_execute：1 项失败，占用进程 notepad.exe。 */
const SUMMARY_FAILED = {
  results: [
    {
      itemId: 1,
      path: "C:\\Temp\\locked.txt",
      deleted: false,
      bytesFreed: 0,
      error: "FILE_IN_USE: C:\\Temp\\locked.txt",
      lockingProcesses: [{ pid: 1234, appName: "notepad.exe", serviceName: null }],
    },
  ],
  deletedCount: 0,
  failedCount: 1,
  bytesFreedTotal: 0,
};

/** cleanup_retry_locked：关掉 notepad、删除成功，另有 explorer 被护栏拒绝。 */
const RETRY_RESP = {
  shutdown: {
    closed: [{ pid: 1234, appName: "notepad.exe", serviceName: null }],
    refused: [{ pid: 4321, appName: "explorer.exe", reason: "protected process" }],
  },
  summary: {
    results: [
      {
        itemId: 1,
        path: "C:\\Temp\\locked.txt",
        deleted: true,
        bytesFreed: 12,
        error: null,
        lockingProcesses: [],
      },
    ],
    deletedCount: 1,
    failedCount: 0,
    bytesFreedTotal: 12,
  },
};

/** close-experience-copy：mixed fail-closed——有受保护进程，后端整批不关（closed=[]、refused 非空、仍失败）。 */
const MIXED_FAIL_CLOSED_RESP = {
  shutdown: {
    closed: [],
    refused: [{ pid: 4321, appName: "explorer.exe", reason: "protected process" }],
  },
  summary: {
    results: [
      {
        itemId: 1,
        path: "C:\\Temp\\locked.txt",
        deleted: false,
        bytesFreed: 0,
        error: "文件被占用",
        lockingProcesses: [{ pid: 4321, appName: "explorer.exe", serviceName: null }],
      },
    ],
    deletedCount: 0,
    failedCount: 1,
    bytesFreedTotal: 0,
  },
};

function mockInvoke(overrides: Record<string, () => Promise<unknown>> = {}) {
  mockedInvoke.mockImplementation(((cmd: string) => {
    if (overrides[cmd]) return overrides[cmd]();
    if (cmd === "cleanup_list_pending") return Promise.resolve(pageOf(ROWS, ROWS.length));
    if (cmd === "cleanup_execute") return Promise.resolve(ok(SUMMARY_FAILED));
    if (cmd === "cleanup_retry_locked") return Promise.resolve(ok(RETRY_RESP));
    return Promise.resolve(ok(null));
  }) as never);
}

/** 渲染并等首屏列表加载完。 */
async function renderLoaded() {
  const view = render(<Items />);
  await waitFor(() => expect(screen.getByText(/locked\.txt/)).toBeInTheDocument());
  return view;
}

/** 勾选第一行。 */
function selectFirstRow() {
  const cb = document.querySelector("tbody input[type=checkbox]") as HTMLInputElement;
  fireEvent.click(cb);
}

beforeEach(() => {
  mockedInvoke.mockReset();
  mockInvoke();
  // jsdom 没实现 confirm，必须显式打桩。
  vi.spyOn(window, "confirm").mockReturnValue(false);
});

afterEach(() => {
  // vitest 未开 globals，RTL 不会自动 cleanup，必须手动卸载上一个用例的树。
  cleanup();
  // 只还原 window.confirm 的 spy：vi.restoreAllMocks() 会把 vi.fn() 的实现也清掉，
  // 导致下一个用例的 invoke 返回 undefined（列表变空）。
  const c = window.confirm as unknown as { mockRestore?: () => void };
  if (typeof c.mockRestore === "function") c.mockRestore();
});

describe("Items 清理失败明细（C8/C9）", () => {
  it("失败项展示路径 + 原因 + 占用进程，并出现重试按钮", async () => {
    await renderLoaded();
    selectFirstRow();
    fireEvent.click(screen.getByText("清理已选"));

    await waitFor(() =>
      expect(screen.getByText(/清理完成：成功 0 项 \/ 失败 1 项/)).toBeInTheDocument(),
    );
    const body = document.body.textContent ?? "";
    expect(body).toContain("C:\\Temp\\locked.txt");
    expect(body).toContain("文件被占用");
    expect(body).toContain("notepad.exe (PID 1234)");
    expect(screen.getByText("关闭占用进程并重试")).toBeEnabled();
  });

  it("没有占用进程时不显示重试按钮", async () => {
    mockInvoke({
      cleanup_execute: () =>
        Promise.resolve(
          ok({
            results: [
              {
                itemId: 1,
                path: "C:\\Temp\\locked.txt",
                deleted: false,
                bytesFreed: 0,
                error: "PERMISSION_DENIED: C:\\Temp\\locked.txt",
                lockingProcesses: [],
              },
            ],
            deletedCount: 0,
            failedCount: 1,
            bytesFreedTotal: 0,
          }),
        ),
    });
    await renderLoaded();
    selectFirstRow();
    fireEvent.click(screen.getByText("清理已选"));

    await waitFor(() => expect(screen.getByText(/失败 1 项/)).toBeInTheDocument());
    expect(screen.queryByText("关闭占用进程并重试")).toBeNull();
  });
});

describe("confirm 二次确认（C10）", () => {
  it("文案含进程清单与数据丢失风险提示", async () => {
    await renderLoaded();
    selectFirstRow();
    fireEvent.click(screen.getByText("清理已选"));
    await waitFor(() => expect(screen.getByText("关闭占用进程并重试")).toBeInTheDocument());

    fireEvent.click(screen.getByText("关闭占用进程并重试"));

    const confirmMock = vi.mocked(window.confirm);
    expect(confirmMock).toHaveBeenCalledTimes(1);
    const text = confirmMock.mock.calls[0][0] as string;
    expect(text).toContain("即将请求关闭以下进程");
    expect(text).toContain("notepad.exe (PID 1234)");
    expect(text).toContain("未保存的数据可能丢失");
    // close-experience-copy：提醒同一控制台内相关进程可能一并关闭。
    expect(text).toContain("同一控制台内的相关进程可能一并关闭");
  });

  it("取消后不调用 cleanup_retry_locked", async () => {
    await renderLoaded();
    selectFirstRow();
    fireEvent.click(screen.getByText("清理已选"));
    await waitFor(() => expect(screen.getByText("关闭占用进程并重试")).toBeInTheDocument());

    fireEvent.click(screen.getByText("关闭占用进程并重试"));

    const called = mockedInvoke.mock.calls.some(([cmd]) => cmd === "cleanup_retry_locked");
    expect(called).toBe(false);
    // 按钮保持可点。
    expect(screen.getByText("关闭占用进程并重试")).toBeEnabled();
  });
});

describe("重试结果展示（C11）", () => {
  it("确认后展示绿条与受保护进程提示", async () => {
    await renderLoaded();
    selectFirstRow();
    fireEvent.click(screen.getByText("清理已选"));
    await waitFor(() => expect(screen.getByText("关闭占用进程并重试")).toBeInTheDocument());

    vi.mocked(window.confirm).mockReturnValue(true);
    fireEvent.click(screen.getByText("关闭占用进程并重试"));

    await waitFor(() =>
      expect(screen.getByText("已关闭 1 个进程，重试成功 1 项")).toBeInTheDocument(),
    );
    const body = document.body.textContent ?? "";
    expect(body).toContain("以下进程受保护未关闭");
    expect(body).toContain("explorer.exe（protected process）");

    // 调用参数只带失败项 id。
    const call = mockedInvoke.mock.calls.find(([cmd]) => cmd === "cleanup_retry_locked");
    expect(call?.[1]).toEqual({ itemIds: [1] });
  });

  it("mixed fail-closed（closed=0 + 受保护进程）时结果条解释未关闭原因", async () => {
    mockInvoke({ cleanup_retry_locked: () => Promise.resolve(ok(MIXED_FAIL_CLOSED_RESP)) });
    await renderLoaded();
    selectFirstRow();
    fireEvent.click(screen.getByText("清理已选"));
    await waitFor(() => expect(screen.getByText("关闭占用进程并重试")).toBeInTheDocument());

    vi.mocked(window.confirm).mockReturnValue(true);
    fireEvent.click(screen.getByText("关闭占用进程并重试"));

    // close-experience-copy：不能只显示「已关闭 0 个进程，仍失败 1 项」，必须解释原因。
    await waitFor(() =>
      expect(
        screen.getByText(/已关闭 0 个进程：因存在受保护进程，本次未关闭任何进程，仍失败 1 项/),
      ).toBeInTheDocument(),
    );
  });
});

describe("三态互斥（C12）", () => {
  it("预览进行中重试按钮禁用，预览结束后恢复", async () => {
    let releaseDryRun: (v: unknown) => void = () => {};
    mockInvoke({
      cleanup_dry_run: () =>
        new Promise((resolve) => {
          releaseDryRun = resolve;
        }),
    });

    await renderLoaded();
    selectFirstRow();
    fireEvent.click(screen.getByText("清理已选"));
    await waitFor(() => expect(screen.getByText("关闭占用进程并重试")).toBeInTheDocument());

    // 进入预览态（dry_run 挂起）。
    selectFirstRow();
    fireEvent.click(screen.getByText("预览可清理项"));
    await waitFor(() => expect(screen.getByText("预览中…")).toBeInTheDocument());

    expect(screen.getByText("关闭占用进程并重试")).toBeDisabled();

    // 预览返回后恢复可点。
    releaseDryRun(
      ok({ totalItems: 1, totalBytes: 12, breakdown: [], blockedByL3: [] }),
    );
    await waitFor(() =>
      expect(screen.getByText("关闭占用进程并重试")).toBeEnabled(),
    );
  });
});

// ---------------------------------------------------------------------------
// task-052 G5：区分「真删了」与「本来就不存在」（走查缺陷——「已清理 1 项，释放 0 B」）
// ---------------------------------------------------------------------------
describe("已不存在 vs 已清理（G5）", () => {
  it("alreadyGoneCount > 0 时文案补「M 项已不存在（未计入释放量）」", async () => {
    mockInvoke({
      cleanup_execute: () =>
        Promise.resolve(
          ok({
            results: [
              {
                itemId: 1,
                path: "C:\\Temp\\locked.txt",
                deleted: true,
                alreadyGone: false,
                bytesFreed: 12,
                error: null,
                lockingProcesses: [],
              },
              {
                itemId: 2,
                path: "C:\\Temp\\vanished.txt",
                deleted: false,
                alreadyGone: true,
                bytesFreed: 0,
                error: null,
                lockingProcesses: [],
              },
            ],
            deletedCount: 1,
            failedCount: 0,
            alreadyGoneCount: 1,
            bytesFreedTotal: 12,
          }),
        ),
    });

    await renderLoaded();
    selectFirstRow();
    fireEvent.click(screen.getByText("清理已选"));

    await waitFor(() =>
      expect(screen.getByText(/已清理 1 项，释放 12 B；1 项已不存在（未计入释放量）/)).toBeInTheDocument(),
    );
  });

  it("alreadyGoneCount == 0 时保持原文案（不出现「已不存在」）", async () => {
    mockInvoke({
      cleanup_execute: () =>
        Promise.resolve(
          ok({
            results: [
              {
                itemId: 1,
                path: "C:\\Temp\\locked.txt",
                deleted: true,
                alreadyGone: false,
                bytesFreed: 12,
                error: null,
                lockingProcesses: [],
              },
            ],
            deletedCount: 1,
            failedCount: 0,
            alreadyGoneCount: 0,
            bytesFreedTotal: 12,
          }),
        ),
    });

    await renderLoaded();
    selectFirstRow();
    fireEvent.click(screen.getByText("清理已选"));

    await waitFor(() => expect(screen.getByText(/已清理 1 项，释放 12 B/)).toBeInTheDocument());
    expect(document.body.textContent ?? "").not.toContain("已不存在");
  });

  it("已不存在的条目不得渲染成失败明细", async () => {
    mockInvoke({
      cleanup_execute: () =>
        Promise.resolve(
          ok({
            results: [
              {
                itemId: 1,
                path: "C:\\Temp\\vanished.txt",
                deleted: false,
                alreadyGone: true,
                bytesFreed: 0,
                error: null,
                lockingProcesses: [],
              },
            ],
            deletedCount: 0,
            failedCount: 0,
            alreadyGoneCount: 1,
            bytesFreedTotal: 0,
          }),
        ),
    });

    await renderLoaded();
    selectFirstRow();
    fireEvent.click(screen.getByText("清理已选"));

    await waitFor(() => expect(screen.getByText(/1 项已不存在/)).toBeInTheDocument());
    // 没有失败项 → 不应出现「未知原因」这种假失败行，也不该出现重试按钮。
    expect(document.body.textContent ?? "").not.toContain("未知原因");
    expect(screen.queryByText("关闭占用进程并重试")).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// task-052 G2：清理列表分页（走查缺陷——原硬编码 limit:100，第 101 条之后不可见）
// ---------------------------------------------------------------------------
describe("清理列表分页（G2）", () => {
  /** 造第 n 页的假数据：每页 size 递减，便于区分页。 */
  const makeRows = (page: number, count: number) =>
    Array.from({ length: count }, (_, i) => ({
      id: (page - 1) * 100 + i + 1,
      path: `C:\\Temp\\p${page}_${i}.tmp`,
      sizeBytes: 1000 - i,
      moduleCode: "A",
      sessionId: 1,
    }));

  it("首屏按 {page:1, pageSize:100} 请求，并展示总数与页码", async () => {
    await renderLoaded();

    const call = mockedInvoke.mock.calls.find(([cmd]) => cmd === "cleanup_list_pending");
    expect(call?.[1]).toEqual({ page: 1, pageSize: 100 });

    expect(screen.getByText(/第 1 \/ 1 页（共 1 条）/)).toBeInTheDocument();
  });

  it("总数 > pageSize 时出现两页，下一页请求 page=2 且清空已选", async () => {
    // 150 条：第 1 页 100 条、第 2 页 50 条。
    mockInvoke({
      cleanup_list_pending: () => {
        // 第几次调用就是第几页（首屏 1 次 + 翻页 1 次）。
        const n = mockedInvoke.mock.calls.filter(
          ([cmd]) => cmd === "cleanup_list_pending",
        ).length;
        return Promise.resolve(
          n === 1
            ? pageOf(makeRows(1, 100), 150, 1, 100)
            : pageOf(makeRows(2, 50), 150, 2, 100),
        );
      },
    });

    render(<Items />);
    await waitFor(() => expect(screen.getByText(/第 1 \/ 2 页（共 150 条）/)).toBeInTheDocument());

    // 第 1 页勾选 1 项。
    selectFirstRow();
    expect(screen.getByText(/已选 1 项/)).toBeInTheDocument();

    // 上一页在第 1 页必须禁用。
    expect(screen.getByText("上一页")).toBeDisabled();

    fireEvent.click(screen.getByText("下一页"));

    await waitFor(() => expect(screen.getByText(/第 2 \/ 2 页（共 150 条）/)).toBeInTheDocument());

    // 翻页清空勾选：否则会跨页误删用户看不见的条目。
    expect(screen.getByText(/已选 0 项/)).toBeInTheDocument();

    // 第 2 页确实换了数据。
    await waitFor(() => expect(screen.getByText(/p2_0\.tmp/)).toBeInTheDocument());

    // 第 2 页时下一页必须禁用（page*pageSize >= total）。
    expect(screen.getByText("下一页")).toBeDisabled();
  });

  it("翻页请求带上真实页码（不读闭包旧 state）", async () => {
    mockInvoke({
      cleanup_list_pending: () => {
        const n = mockedInvoke.mock.calls.filter(
          ([cmd]) => cmd === "cleanup_list_pending",
        ).length;
        return Promise.resolve(
          n === 1 ? pageOf(makeRows(1, 100), 150, 1, 100) : pageOf(makeRows(2, 50), 150, 2, 100),
        );
      },
    });

    render(<Items />);
    await waitFor(() => expect(screen.getByText(/第 1 \/ 2 页/)).toBeInTheDocument());

    fireEvent.click(screen.getByText("下一页"));
    await waitFor(() => expect(screen.getByText(/第 2 \/ 2 页/)).toBeInTheDocument());

    const calls = mockedInvoke.mock.calls.filter(([cmd]) => cmd === "cleanup_list_pending");
    expect(calls[0][1]).toEqual({ page: 1, pageSize: 100 });
    expect(calls[1][1]).toEqual({ page: 2, pageSize: 100 });
  });
});
