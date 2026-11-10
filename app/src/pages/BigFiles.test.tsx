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
      driveLetter: "C",
    },
    {
      path: "C:\\Users\\me\\game.iso",
      sizeBytes: 4 * GB,
      category: "top_files",
      recommendation: "确认不再需要后手动删除",
      driveLetter: "C",
    },
  ],
  // task-084 A1：目标空间（Downloads / Documents / Desktop / Temp）
  targetSpaces: [
    {
      path: "C:\\Users\\me\\Downloads\\big_installer.exe",
      sizeBytes: 700 * 1024 * 1024,
      category: "target_downloads",
      recommendation: "你的下载文件，确认不需要后手动删除",
    },
  ],
  totalBytes: 6.4 * GB + 12.5 * GB + 2.1 * GB + 8 * GB + 4 * GB,
  scannedEntries: 12345,
  truncated: false,
  elapsedMs: 3210,
};

const DUP_GROUPS = [
  {
    groupId: "abc1234567890000",
    sizeBytes: 15 * 1024 * 1024,
    files: [
      { path: "C:\\Users\\me\\Downloads\\a.bin", sizeBytes: 15 * 1024 * 1024, modifiedMs: 1000, isKeep: true, driveLetter: "C" },
      { path: "D:\\backup\\copy.bin", sizeBytes: 15 * 1024 * 1024, modifiedMs: 500, isKeep: false, driveLetter: "D" },
    ],
  },
];

const FOLDER_STATS = [
  {
    path: "C:\\Users\\me\\AppData\\Local",
    sizeBytes: 10 * 1024 * 1024 * 1024,
    fileCount: 500,
    category: "AppData",
    driveLetter: "C",
  },
  {
    path: "C:\\Users\\me\\Downloads",
    sizeBytes: 2 * 1024 * 1024 * 1024,
    fileCount: 200,
    category: "Downloads",
    driveLetter: "C",
  },
];

function mockScan(data: unknown = REPORT) {
  mockedInvoke.mockImplementation(((cmd: string) => {
    if (cmd === "big_files_scan") return Promise.resolve(ok(data));
    if (cmd === "big_files_duplicates") return Promise.resolve(ok(DUP_GROUPS));
    if (cmd === "big_files_folder_stats") return Promise.resolve(ok(FOLDER_STATS));
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
    expect(screen.getByText(/^最大的 2 个文件（按盘分组）$/)).toBeTruthy();
    // 按盘分组：C 盘标题出现在表内
    expect(screen.getByTestId("drive-header-C")).toBeTruthy();
    // Top 汇总 = 8 + 4 = 12 GB
    expect(screen.getByText("12.0 GB")).toBeTruthy();
  });

  it("只读：没有删除/清理/一键释放入口", async () => {
    mockScan();
    render(<BigFiles />);
    await screen.findByTestId("category-hiberfil");

    const buttons = screen.getAllByRole("button");
    // 按钮白名单：重新扫描 + 四个 tab。多一个「删除 / 清理 / 一键释放」都会在这里红。
    const labels = buttons.map((b) => b.textContent);
    for (const forbidden of [/一键清理/, /删除/, /释放/]) {
      expect(labels.some((t) => forbidden.test(t ?? ""))).toBe(false);
    }
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

  describe("task-084 新 tab", () => {
    it("目标空间 tab 按 category 分组展示 Downloads 大文件", async () => {
      mockScan();
      render(<BigFiles />);
      await screen.findByTestId("category-hiberfil");

      fireEvent.click(screen.getByRole("tab", { name: "目标空间" }));

      expect(await screen.findByTestId("target-target_downloads")).toBeTruthy();
      expect(screen.getByText(/big_installer\.exe/)).toBeTruthy();
      // 只读：目标空间也没有删除入口
      expect(screen.queryByText(/一键清理/)).toBeNull();
    });

    it("重复文件 tab 懒加载并展示重复组 + 保留标记", async () => {
      mockScan();
      render(<BigFiles />);
      await screen.findByTestId("category-hiberfil");

      fireEvent.click(screen.getByRole("tab", { name: "重复文件" }));

      expect(await screen.findByTestId("dup-group")).toBeTruthy();
      // 「保留 / 可清理」标记在重复组表格里
      expect(screen.getByText(/✅ 保留/)).toBeTruthy();
      expect(screen.getByText(/可清理/)).toBeTruthy();
      // 首次切到该 tab 才调后端
      await waitFor(() => expect(mockedInvoke).toHaveBeenCalledWith("big_files_duplicates", {}));
    });

    it("task-084 B3：Top100 按盘分组，多盘各自的路径都展示", async () => {
      mockScan({
        ...REPORT,
        topFiles: [
          ...REPORT.topFiles,
          {
            path: "D:\\Data\\movie.mkv",
            sizeBytes: 6 * GB,
            category: "top_files",
            recommendation: "确认不再需要后手动删除",
            driveLetter: "D",
          },
        ],
      });
      render(<BigFiles />);
      await screen.findByTestId("category-hiberfil");

      // 两个盘的标题都出现，跨盘文件也列出来了
      expect(screen.getByTestId("drive-header-C")).toBeTruthy();
      expect(screen.getByTestId("drive-header-D")).toBeTruthy();
      expect(screen.getByText("D:\\Data\\movie.mkv")).toBeTruthy();
    });

    it("task-084 B3：重复组里跨盘文件带盘符标注", async () => {
      mockScan();
      render(<BigFiles />);
      await screen.findByTestId("category-hiberfil");

      fireEvent.click(screen.getByRole("tab", { name: "重复文件" }));

      expect(await screen.findByTestId("dup-group")).toBeTruthy();
      // 跨盘重复：涉及盘 C 与 D
      expect(screen.getByText(/涉及盘：C: \/ D:/)).toBeTruthy();
      // 每行文件前标注盘符
      expect(screen.getByText(/\[C:\]/)).toBeTruthy();
      expect(screen.getByText(/\[D:\]/)).toBeTruthy();
    });

    it("文件夹分布 tab 懒加载并展示 Top 文件夹表", async () => {
      mockScan();
      render(<BigFiles />);
      await screen.findByTestId("category-hiberfil");

      fireEvent.click(screen.getByRole("tab", { name: "文件夹分布" }));

      expect(await screen.findByText(/占用最多的文件夹/)).toBeTruthy();
      expect(screen.getByText(/AppData.Loc/)).toBeTruthy();
      // task-084 B3：文件夹分布按盘分组，C 盘标题出现
      expect(screen.getByTestId("drive-header-C")).toBeTruthy();
      await waitFor(() => expect(mockedInvoke).toHaveBeenCalledWith("big_files_folder_stats", {}));
    });
  });
});
