// task-066：大文件地图页的渲染与只读约束用例。
//
// 为什么放组件级：本机 WebView2 渲染进程起不来（见 docs/test-reports/ 历史报告），
// 真机点击取证拿不到。而「只读」这条恰恰是**用断言能钉死、用肉眼反而容易漏**的
// ——人工走查只会看「有没有一个显眼的删除按钮」，不会去看每个 button 的文案。
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, fireEvent, cleanup } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import BigFiles from "./BigFiles";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

const GB = 1024 ** 3;
const ok = (data: unknown) => ({ code: 0, message: "ok", data });

const REPORT = {
  categories: [
    {
      path: "C:\\hiberfil.sys",
      sizeBytes: 6.4 * GB,
      category: "hiberfil",
      recommendation: "执行 powercfg /h off 可关闭休眠并释放",
    },
    {
      path: "C:\\Windows.old",
      sizeBytes: 12.5 * GB,
      category: "windows_old",
      recommendation: "用「磁盘清理 → 清理系统文件」删除",
    },
    {
      path: "C:\\$Recycle.Bin",
      sizeBytes: 2.1 * GB,
      category: "recycle_bin",
      recommendation: "打开回收站确认无需恢复后清空",
    },
  ],
  topFiles: [
    {
      path: "C:\\Users\\me\\vm.vhdx",
      sizeBytes: 8 * GB,
      category: "top_files",
      recommendation: "确认不再需要后手动删除",
    },
    {
      path: "C:\\Users\\me\\game.iso",
      sizeBytes: 4 * GB,
      category: "top_files",
      recommendation: "确认不再需要后手动删除",
    },
  ],
  totalBytes: 6.4 * GB + 12.5 * GB + 2.1 * GB + 8 * GB + 4 * GB,
  scannedEntries: 12345,
  truncated: false,
  elapsedMs: 3210,
};

function mockScan(data: unknown = REPORT) {
  mockedInvoke.mockImplementation(((cmd: string) => {
    if (cmd === "big_files_scan") return Promise.resolve(ok(data));
    return Promise.resolve(ok(null));
  }) as unknown as typeof invoke);
}

beforeEach(() => {
  mockedInvoke.mockReset();
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("大文件地图", () => {
  it("挂载即自动扫描，并渲染三个固定分类卡片 + Top100 汇总卡", async () => {
    mockScan();
    render(<BigFiles />);

    // 自动扫描（只读、不弹 UAC，所以可以进页面就扫）
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledWith("big_files_scan", {}));

    // 三张固定分类卡片
    expect(await screen.findByTestId("category-hiberfil")).toBeTruthy();
    expect(screen.getByTestId("category-windows_old")).toBeTruthy();
    expect(screen.getByTestId("category-recycle_bin")).toBeTruthy();
    expect(screen.getByTestId("category-top_files")).toBeTruthy();

    // 大小按 formatBytes 渲染
    expect(screen.getByText("6.4 GB")).toBeTruthy();
    expect(screen.getByText("12.5 GB")).toBeTruthy();
    expect(screen.getByText("2.1 GB")).toBeTruthy();
  });

  it("每个分类卡片都带可照着做的处理建议", async () => {
    mockScan();
    render(<BigFiles />);

    expect(await screen.findByText(/powercfg \/h off/)).toBeTruthy();
    expect(screen.getByText(/磁盘清理 → 清理系统文件/)).toBeTruthy();
    expect(screen.getByText(/打开回收站确认无需恢复后清空/)).toBeTruthy();
  });

  it("Top100 列表显示文件路径 + 大小", async () => {
    mockScan();
    render(<BigFiles />);

    expect(await screen.findByText("C:\\Users\\me\\vm.vhdx")).toBeTruthy();
    expect(screen.getByText("C:\\Users\\me\\game.iso")).toBeTruthy();
    expect(screen.getByText("最大的 2 个文件")).toBeTruthy();
    // Top 汇总 = 8 + 4 = 12 GB
    expect(screen.getByText("12.0 GB")).toBeTruthy();
  });

  it("只读：整页只有「重新扫描」一个按钮，没有任何删除入口", async () => {
    mockScan();
    render(<BigFiles />);
    await screen.findByTestId("category-hiberfil");

    const buttons = screen.getAllByRole("button");
    // 强断言：按钮文案白名单。多一个「删除 / 清理 / 一键释放」都会在这里红。
    expect(buttons.map((b) => b.textContent)).toEqual(["重新扫描"]);
    expect(screen.queryByText(/一键清理/)).toBeNull();
  });

  it("扫描失败时展示原因，且不留旧结果", async () => {
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "big_files_scan") return Promise.reject("根路径不是目录: C:\\");
      return Promise.resolve(ok(null));
    }) as unknown as typeof invoke);

    render(<BigFiles />);

    expect(await screen.findByText("扫描失败")).toBeTruthy();
    expect(screen.getByText(/根路径不是目录/)).toBeTruthy();
    // 失败后不该还挂着卡片（旧数字会让人以为这次也扫成功了）
    expect(screen.queryByTestId("category-hiberfil")).toBeNull();
  });

  it("撞到扫描上限时明确提示「结果可能不完整」", async () => {
    mockScan({ ...REPORT, truncated: true, scannedEntries: 200_000 });
    render(<BigFiles />);

    expect(await screen.findByText(/已达扫描上限提前停止/)).toBeTruthy();
  });

  it("没有大文件时不渲染空表格，给出说明", async () => {
    mockScan({ ...REPORT, topFiles: [], totalBytes: 21 * GB });
    render(<BigFiles />);

    expect(await screen.findByText("没有超过 100 MB 的用户文件")).toBeTruthy();
    // 空列表不该出现「最大的 0 个文件」这种空表格
    expect(screen.queryByText(/最大的 0 个文件/)).toBeNull();
  });

  it("点「重新扫描」会再次调用后端", async () => {
    mockScan();
    render(<BigFiles />);
    await screen.findByTestId("category-hiberfil");

    fireEvent.click(screen.getByText("重新扫描"));

    await waitFor(() => {
      const calls = mockedInvoke.mock.calls.filter(([c]) => c === "big_files_scan");
      expect(calls).toHaveLength(2);
    });
  });

  it("未知分类不会让页面崩（后端将来加分类的兜底）", async () => {
    mockScan({
      ...REPORT,
      categories: [
        ...REPORT.categories,
        { path: "C:\\x", sizeBytes: GB, category: "brand_new", recommendation: "新分类" },
      ],
    });
    render(<BigFiles />);

    // 兜底用 category 原值当标题，不白屏
    expect(await screen.findByText("brand_new")).toBeTruthy();
  });
});
