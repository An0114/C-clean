// task-067：残留扫描页的渲染与「只读/可移入保险箱」边界用例。
//
// 重点不在「页面能不能渲染」，而在三条**安全边界**：
//   ① 注册表残留 tab 里绝对不能出现勾选框（本工具不代删注册表键）；
//   ② 文件残留默认不勾选，也不提供「全选」；
//   ③ 移入保险箱必须先过 confirm，且取消时一个 IPC 都不发。
// 这三条用断言钉死比人工走查可靠——人看页面只会注意到「有没有显眼的删除按钮」。

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, fireEvent, cleanup } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import Residual from "./Residual";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);
const ok = (data: unknown) => ({ code: 0, message: "ok", data });

const REGISTRY_ITEM = {
  name: "Google Chrome",
  path: "C:\\Program Files (x86)\\Chromebrowser",
  residualType: "registry",
  sizeBytes: 0,
  registryKey: "HKLM\\Software\\...\\Uninstall\\{chrome-guid}",
  confidence: "high",
  riskLevel: "L2",
  note: "注册表记录的安装目录已不存在",
};

const FILE_ITEM_A = {
  name: "WinRAR",
  path: "C:\\Program Files\\WinRAR",
  residualType: "files",
  sizeBytes: 474,
  registryKey: null,
  confidence: "low",
  riskLevel: "L2",
  note: "注册表里找不到对应的卸载项",
};

const FILE_ITEM_B = {
  name: "Zero G Registry",
  path: "C:\\Program Files\\Zero G Registry",
  residualType: "files",
  sizeBytes: 17599,
  registryKey: null,
  confidence: "low",
  riskLevel: "L2",
  note: "注册表里找不到对应的卸载项",
};

const REPORT = {
  registryItems: [REGISTRY_ITEM],
  fileItems: [FILE_ITEM_A, FILE_ITEM_B],
  totalBytes: 474 + 17599,
  registryEntries: 94,
  dirsScanned: 28,
  sizeTruncated: false,
  elapsedMs: 75,
};

const DELETE_OK = { moved: [FILE_ITEM_A.path], failed: [], bytesMoved: 474 };

function mockInvoke(overrides: Record<string, () => Promise<unknown>> = {}) {
  mockedInvoke.mockImplementation(((cmd: string) => {
    if (overrides[cmd]) return overrides[cmd]();
    if (cmd === "residual_scan") return Promise.resolve(ok(REPORT));
    if (cmd === "residual_delete") return Promise.resolve(ok(DELETE_OK));
    return Promise.resolve(ok(null));
  }) as unknown as typeof invoke);
}

function callsOf(cmd: string): Array<Record<string, unknown> | undefined> {
  return mockedInvoke.mock.calls
    .filter(([c]) => c === cmd)
    .map(([, args]) => args as Record<string, unknown> | undefined);
}

beforeEach(() => {
  mockedInvoke.mockReset();
  vi.spyOn(window, "confirm").mockReturnValue(true);
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("残留扫描", () => {
  it("挂载即扫描，两个分类 tab 带数量", async () => {
    mockInvoke();
    render(<Residual />);

    await waitFor(() => expect(callsOf("residual_scan")).toHaveLength(1));
    expect(await screen.findByText("文件残留（2）")).toBeTruthy();
    expect(screen.getByText("注册表残留（1）")).toBeTruthy();
    // 统计条如实反映读到了多少
    expect(screen.getByText(/读取 94 个卸载项/)).toBeTruthy();
  });

  it("文件残留列表显示名称 / 路径 / 大小 / 类型", async () => {
    mockInvoke();
    render(<Residual />);

    expect(await screen.findByText("WinRAR")).toBeTruthy();
    expect(screen.getByText("C:\\Program Files\\WinRAR")).toBeTruthy();
    expect(screen.getByText("474 B")).toBeTruthy();
    // 类型文字（验收 9 要求「类型」列）
    expect(screen.getAllByText("文件残留").length).toBeGreaterThan(0);
    // 低置信 + L2 徽标
    expect(screen.getAllByText("低置信")).toHaveLength(2);
    expect(screen.getAllByText("L2 需确认")).toHaveLength(2);
  });

  it("注册表残留 tab 里没有勾选框，只给键路径让人手动处理", async () => {
    mockInvoke();
    render(<Residual />);
    await screen.findByText("文件残留（2）");

    fireEvent.click(screen.getByText("注册表残留（1）"));

    expect(await screen.findByText("Google Chrome")).toBeTruthy();
    expect(screen.getByText(/Uninstall/)).toBeTruthy();
    // 核心断言：注册表残留不能有任何勾选框
    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
    expect(screen.getByText(/本工具不代删注册表键/)).toBeTruthy();
    // 也不该出现「移入保险箱」按钮（注意：顶部说明段落里也有这四个字，
    // 所以必须按 button 角色查，不能按文本查）
    expect(screen.queryByRole("button", { name: /移入保险箱/ })).toBeNull();
  });

  it("默认一个都不勾，操作条不出现（没有「全选」按钮）", async () => {
    mockInvoke();
    render(<Residual />);
    await screen.findByText("WinRAR");

    const boxes = screen.getAllByRole("checkbox") as HTMLInputElement[];
    expect(boxes).toHaveLength(2);
    expect(boxes.every((b) => !b.checked)).toBe(true);
    expect(screen.queryByRole("button", { name: /移入保险箱/ })).toBeNull();
    // 刻意不提供批量勾选：低置信结果不该鼓励批量操作
    expect(screen.queryByText(/全选/)).toBeNull();
  });

  it("勾选后出现操作条，显示项数与合计大小", async () => {
    mockInvoke();
    render(<Residual />);
    await screen.findByText("WinRAR");

    fireEvent.click(screen.getByLabelText("选择 WinRAR"));

    expect(await screen.findByText(/已选/)).toBeTruthy();
    expect(screen.getByText("移入保险箱（1 项）")).toBeTruthy();
  });

  it("用户取消确认时一个 IPC 都不发", async () => {
    vi.mocked(window.confirm).mockReturnValue(false);
    mockInvoke();
    render(<Residual />);
    await screen.findByText("WinRAR");

    fireEvent.click(screen.getByLabelText("选择 WinRAR"));
    fireEvent.click(screen.getByText("移入保险箱（1 项）"));

    await waitFor(() => expect(window.confirm).toHaveBeenCalled());
    expect(callsOf("residual_delete")).toHaveLength(0);
  });

  it("确认后调用 residual_delete，实参是勾选的路径数组", async () => {
    mockInvoke();
    render(<Residual />);
    await screen.findByText("WinRAR");

    fireEvent.click(screen.getByLabelText("选择 WinRAR"));
    fireEvent.click(screen.getByText("移入保险箱（1 项）"));

    await waitFor(() => expect(callsOf("residual_delete")).toHaveLength(1));
    expect(callsOf("residual_delete")[0]).toEqual({ paths: [FILE_ITEM_A.path] });
    expect(await screen.findByText(/已移入保险箱 1 项/)).toBeTruthy();
    expect(screen.getByText(/7 天内可在「保险箱」页恢复/)).toBeTruthy();
  });

  it("2002（安全红线）时给出明确提示，而不是笼统的失败", async () => {
    mockInvoke({
      residual_delete: () =>
        Promise.resolve({ code: 2002, message: "检测到受保护路径，操作已拒绝（L3 红线）", data: null }),
    });
    render(<Residual />);
    await screen.findByText("WinRAR");

    fireEvent.click(screen.getByLabelText("选择 WinRAR"));
    fireEvent.click(screen.getByText("移入保险箱（1 项）"));

    expect(await screen.findByText("未能移入保险箱")).toBeTruthy();
    expect(screen.getByText(/安全红线/)).toBeTruthy();
  });

  it("部分失败时同时展示成功数与逐条原因", async () => {
    mockInvoke({
      residual_delete: () =>
        Promise.resolve(
          ok({
            moved: [FILE_ITEM_A.path],
            failed: [{ path: FILE_ITEM_B.path, reason: "文件被占用" }],
            bytesMoved: 474,
          })
        ),
    });
    render(<Residual />);
    await screen.findByText("WinRAR");

    fireEvent.click(screen.getByLabelText("选择 WinRAR"));
    fireEvent.click(screen.getByLabelText("选择 Zero G Registry"));
    fireEvent.click(screen.getByText("移入保险箱（2 项）"));

    expect(await screen.findByText(/已移入保险箱 1 项/)).toBeTruthy();
    expect(screen.getByText(/失败 1 项/)).toBeTruthy();
    expect(screen.getByText(/文件被占用/)).toBeTruthy();
  });

  it("扫描失败时展示原因，不留旧列表", async () => {
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "residual_scan") return Promise.reject("注册表读取失败：拒绝访问");
      return Promise.resolve(ok(null));
    }) as unknown as typeof invoke);

    render(<Residual />);

    expect(await screen.findByText("扫描失败")).toBeTruthy();
    expect(screen.getByText(/拒绝访问/)).toBeTruthy();
    expect(screen.queryByText("WinRAR")).toBeNull();
  });

  it("没有文件残留时给出说明而不是空表格", async () => {
    mockInvoke({
      residual_scan: () =>
        Promise.resolve(ok({ ...REPORT, fileItems: [], totalBytes: 0 })),
    });
    render(<Residual />);

    expect(await screen.findByText(/没有发现文件残留/)).toBeTruthy();
  });
});
