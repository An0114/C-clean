// task-051：保险箱页「时间预设 / 多选 session / 来源列 / 审计历史展开」的交互回归用例。
//
// 为什么放组件级：本机测试会话里 WebView2 渲染进程起不来（无活动桌面，
// 详见 docs/test-reports/ 历史报告的环境限制），GUI 点击取证拿不到，
// 交互路径只能用 jsdom 固化（任务书 F28 的补偿证据之一）。
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, fireEvent, cleanup } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import SafeBox, { actionLabel, presetFromTs } from "./SafeBox";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

const ok = (data: unknown) => ({ code: 0, message: "ok", data });

const ENTRIES = [
  {
    id: 11,
    originalPath: "C:\\Temp\\a.bin",
    sizeBytes: 1024,
    quarantinedAt: 1_700_000_000,
    sourceTaskId: 7,
  },
  {
    id: 12,
    originalPath: "C:\\Temp\\b.bin",
    sizeBytes: 2048,
    quarantinedAt: 1_700_000_100,
    sourceTaskId: null,
  },
];

const SESSIONS = [
  { sessionId: 1, itemCount: 3, createdTs: 1_700_000_000 },
  { sessionId: 2, itemCount: 5, createdTs: 1_700_000_500 },
];

const HISTORY_ROWS = [
  {
    id: 101,
    eventType: "quarantine",
    path: "C:\\Temp\\a.bin",
    detail: "{\"safe_box_id\":11}",
    createdAt: 1_700_000_000,
    result: "success",
  },
  {
    id: 102,
    eventType: "permanent_delete",
    path: "C:\\Temp\\a.bin",
    detail: null,
    createdAt: 1_700_000_900,
    result: "success",
  },
];

/** 记录每次 safe_box_list 的实参，便于断言筛选条件。 */
function lastListArgs(): Record<string, unknown> | undefined {
  const calls = mockedInvoke.mock.calls.filter(([cmd]) => cmd === "safe_box_list");
  return calls.length === 0 ? undefined : (calls[calls.length - 1][1] as Record<string, unknown>);
}

function mockInvoke(overrides: Record<string, () => Promise<unknown>> = {}) {
  mockedInvoke.mockImplementation(((cmd: string, args?: Record<string, unknown>) => {
    if (overrides[cmd]) return overrides[cmd]();
    if (cmd === "safe_box_list") {
      const page = (args?.page as number) ?? 1;
      const pageSize = (args?.pageSize as number) ?? 50;
      return Promise.resolve(
        ok({ items: ENTRIES, total: 2, page, pageSize, totalBytes: 3072 })
      );
    }
    if (cmd === "safe_box_list_sessions") return Promise.resolve(ok(SESSIONS));
    if (cmd === "safe_box_item_history") return Promise.resolve(ok(HISTORY_ROWS));
    return Promise.resolve(ok(null));
  }) as never);
}

/** 渲染并等首屏列表加载完。 */
async function renderLoaded() {
  const view = render(<SafeBox />);
  await waitFor(() => expect(screen.getByText(/a\.bin/)).toBeInTheDocument());
  return view;
}

beforeEach(() => {
  mockedInvoke.mockReset();
  mockInvoke();
  vi.spyOn(window, "confirm").mockReturnValue(false);
});

afterEach(() => {
  cleanup();
  const c = window.confirm as unknown as { mockRestore?: () => void };
  if (typeof c.mockRestore === "function") c.mockRestore();
});

describe("纯函数（task-051）", () => {
  it("presetFromTs：全部不传，其余按秒回推", () => {
    const now = 1_700_000_000;
    expect(presetFromTs("all", now)).toBeUndefined();
    expect(presetFromTs("24h", now)).toBe(now - 86400);
    expect(presetFromTs("3d", now)).toBe(now - 3 * 86400);
    expect(presetFromTs("7d", now)).toBe(now - 7 * 86400);
  });

  it("actionLabel：已知动作中文化，未知动作原样显示", () => {
    expect(actionLabel("permanent_delete")).toBe("永久删除");
    expect(actionLabel("restore")).toBe("恢复");
    expect(actionLabel("purge")).toBe("清除");
    expect(actionLabel("quarantine")).toBe("移入保险箱");
    expect(actionLabel("scan")).toBe("扫描");
    expect(actionLabel("something_new")).toBe("something_new");
  });
});

describe("首屏加载（D16/D21）", () => {
  it("调用 safe_box_list 且不带 sessionIds / fromTs", async () => {
    await renderLoaded();

    const args = lastListArgs();
    expect(args).toBeDefined();
    expect(args?.page).toBe(1);
    expect(args?.sessionIds).toBeUndefined();
    expect(args?.fromTs).toBeUndefined();
    // 同时拉了 session 列表填多选框。
    expect(mockedInvoke.mock.calls.some(([c]) => c === "safe_box_list_sessions")).toBe(true);
  });

  it("表格渲染「来源」列：#N 与 — 两种形态", async () => {
    await renderLoaded();

    expect(screen.getByText("来源")).toBeInTheDocument();
    expect(screen.getByText("#7")).toBeInTheDocument();
    // sourceTaskId=null 的行显示破折号。
    expect(screen.getAllByText("—").length).toBeGreaterThanOrEqual(1);
  });
});

describe("时间预设（D16）", () => {
  it("点「24 小时」带 fromTs（>0）且重置到第 1 页", async () => {
    await renderLoaded();

    fireEvent.click(screen.getByText("24 小时"));

    await waitFor(() => {
      const args = lastListArgs();
      expect(args?.fromTs).toBeGreaterThan(0);
    });
    const args = lastListArgs();
    expect(args?.page).toBe(1);
    // 24 小时的窗口应落在「现在 - 86400」附近（允许几秒误差）。
    const expected = Math.floor(Date.now() / 1000) - 86400;
    expect(Math.abs((args?.fromTs as number) - expected)).toBeLessThan(5);
  });

  it("筛选变化后清空已选（批量按钮回到 0）", async () => {
    await renderLoaded();

    // 勾选第一行。
    const cb = document.querySelector("tbody input[type=checkbox]") as HTMLInputElement;
    fireEvent.click(cb);
    await waitFor(() =>
      expect(screen.getByText("批量删除已选(1)")).toBeInTheDocument()
    );

    fireEvent.click(screen.getByText("24 小时"));
    await waitFor(() =>
      expect(screen.getByText("批量删除已选(0)")).toBeInTheDocument()
    );
  });
});

describe("多选 session（D17）", () => {
  it("选中 2 个 session → 调用带 sessionIds: [1, 2]", async () => {
    await renderLoaded();

    const select = screen.getByLabelText("session 多选") as HTMLSelectElement;
    await waitFor(() => expect(select.options.length).toBe(2));
    Array.from(select.options).forEach((o) => {
      o.selected = o.value === "1" || o.value === "2";
    });
    fireEvent.change(select);

    await waitFor(() => {
      const args = lastListArgs();
      expect(args?.sessionIds).toEqual([1, 2]);
    });
    expect(lastListArgs()?.page).toBe(1);
  });

  it("「清空」后不再传 sessionIds", async () => {
    await renderLoaded();

    const select = screen.getByLabelText("session 多选") as HTMLSelectElement;
    await waitFor(() => expect(select.options.length).toBe(2));
    Array.from(select.options).forEach((o) => {
      o.selected = o.value === "2";
    });
    fireEvent.change(select);
    await waitFor(() => expect(lastListArgs()?.sessionIds).toEqual([2]));

    fireEvent.click(screen.getByText("清空"));
    await waitFor(() => expect(lastListArgs()?.sessionIds).toBeUndefined());
  });
});

describe("审计历史展开（D19）", () => {
  it("点「历史」调 safe_box_item_history 并中文化渲染动作", async () => {
    await renderLoaded();

    fireEvent.click(screen.getAllByText("历史")[0]);

    await waitFor(() => expect(screen.getByText("移入保险箱")).toBeInTheDocument());
    // 「永久删除」同时出现在每行的操作按钮与历史动作里 → 用 getAllByText 断言至少 2 处。
    expect(screen.getAllByText("永久删除").length).toBeGreaterThanOrEqual(2);

    const call = mockedInvoke.mock.calls.find(([c]) => c === "safe_box_item_history");
    expect(call?.[1]).toEqual({ safeBoxId: 11 });
    // 展开后按钮文案变「收起」。
    expect(screen.getByText("收起")).toBeInTheDocument();
  });

  it("无历史时显示「该路径暂无审计记录」", async () => {
    mockInvoke({ safe_box_item_history: () => Promise.resolve(ok([])) });
    await renderLoaded();

    fireEvent.click(screen.getAllByText("历史")[0]);

    await waitFor(() =>
      expect(screen.getByText("该路径暂无审计记录")).toBeInTheDocument()
    );
  });

  it("再次点击同一行收起", async () => {
    await renderLoaded();

    fireEvent.click(screen.getAllByText("历史")[0]);
    await waitFor(() => expect(screen.getByText("收起")).toBeInTheDocument());

    fireEvent.click(screen.getByText("收起"));
    await waitFor(() =>
      expect(screen.queryByText("移入保险箱")).toBeNull()
    );
  });
});
