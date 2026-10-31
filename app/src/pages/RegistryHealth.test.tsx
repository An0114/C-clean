// task-068：注册表健康中心的渲染与三条安全边界用例。
//
// 重点断言：
//   ① 三个分类 tab 都在，且带数量；
//   ② **L3 项没有勾选框**（系统组件/安全软件不给删的机会）；
//   ③ 卸载残留 / 计划任务两个 tab 完全没有勾选框；
//   ④ 删除前必须 confirm，取消时一个 IPC 都不发；
//   ⑤ 删除结果必须显示**备份文件路径**（不告诉用户备份在哪等于没备份）；
//   ⑥ 计划任务目录读不到时说明「需要管理员权限」，而不是「没有计划任务」。

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, fireEvent, cleanup } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import RegistryHealth from "./RegistryHealth";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);
const ok = (data: unknown) => ({ code: 0, message: "ok", data });

const RUN_L2_MISSING = {
  name: "GhostLauncher",
  path: "D:\\OldGame\\launcher.exe",
  category: "run",
  riskLevel: "L2",
  recommendation: "启动项指向的程序文件已不存在，大概率是卸载残留，可以删除",
  registryKey: "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
  valueName: "GhostLauncher",
  exists: false,
  deletable: true,
};

const RUN_L2_OK = {
  name: "Everything",
  path: "D:\\everything\\Everything.exe -startup",
  category: "run",
  riskLevel: "L2",
  recommendation: "第三方开机启动项。删除后该程序不再自动启动",
  registryKey: "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
  valueName: "Everything",
  exists: true,
  deletable: true,
};

const RUN_L3 = {
  name: "SecurityHealth",
  path: "%windir%\\system32\\SecurityHealthSystray.exe",
  category: "run",
  riskLevel: "L3",
  recommendation: "安全软件启动项，删除会降低防护。本工具不代删",
  registryKey: "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
  valueName: "SecurityHealth",
  exists: true,
  deletable: false,
};

const UNINSTALL_ITEM = {
  name: "Google Chrome",
  path: "C:\\Program Files (x86)\\Chromebrowser",
  category: "uninstall",
  riskLevel: "L2",
  recommendation: "注册表记录的安装目录已不存在。本工具不代删注册表键",
  registryKey: "HKLM\\Software\\...\\Uninstall\\{chrome}",
  valueName: "",
  exists: false,
  deletable: false,
};

const TASK_ITEM = {
  name: "MyAppUpdate",
  path: "C:\\Nope\\updater.exe --silent",
  category: "task",
  riskLevel: "L2",
  recommendation: "任务指向的程序已不存在，可能是卸载残留。本工具不代删计划任务",
  registryKey: "C:\\Windows\\System32\\Tasks\\MyAppUpdate.xml",
  valueName: "",
  exists: false,
  deletable: false,
};

const REPORT = {
  runItems: [RUN_L2_MISSING, RUN_L2_OK, RUN_L3],
  uninstallItems: [UNINSTALL_ITEM],
  taskItems: [TASK_ITEM],
  taskScanAvailable: true,
  elapsedMs: 62,
};

const DELETE_OK = {
  deleted: ["GhostLauncher"],
  failed: [],
  backups: ["C:\\Users\\An\\AppData\\Roaming\\com.and0114.cclean\\registry_backups\\HKCU-Software-...-Run-1700000000.reg"],
};

function mockInvoke(overrides: Record<string, () => Promise<unknown>> = {}) {
  mockedInvoke.mockImplementation(((cmd: string) => {
    if (overrides[cmd]) return overrides[cmd]();
    if (cmd === "registry_health_scan") return Promise.resolve(ok(REPORT));
    if (cmd === "registry_health_delete") return Promise.resolve(ok(DELETE_OK));
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

describe("注册表健康中心", () => {
  it("挂载即扫描，三个分类 tab 带数量", async () => {
    mockInvoke();
    render(<RegistryHealth />);

    await waitFor(() => expect(callsOf("registry_health_scan")).toHaveLength(1));
    expect(await screen.findByText("启动项（3）")).toBeTruthy();
    expect(screen.getByText("卸载残留（1）")).toBeTruthy();
    expect(screen.getByText("计划任务（1）")).toBeTruthy();
    // 顶部要点出「几个启动项的文件已不存在」
    expect(screen.getByText(/1 个启动项指向的文件已不存在/)).toBeTruthy();
  });

  it("启动项列表显示名称 / 命令 / 风险 / 建议", async () => {
    mockInvoke();
    render(<RegistryHealth />);

    expect(await screen.findByText("GhostLauncher")).toBeTruthy();
    expect(screen.getByText("D:\\OldGame\\launcher.exe")).toBeTruthy();
    // 「文件已不存在」徽标只在 missing 那行出现
    expect(screen.getByText("文件已不存在")).toBeTruthy();
    expect(screen.getAllByText("L2 需确认")).toHaveLength(2);
    expect(screen.getByText("L3 禁删")).toBeTruthy();
    expect(screen.getByText(/大概率是卸载残留/)).toBeTruthy();
  });

  it("L3 项没有勾选框（系统组件/安全软件不给删的机会）", async () => {
    mockInvoke();
    render(<RegistryHealth />);
    await screen.findByText("SecurityHealth");

    // 三条启动项里只有两条 deletable → 只有两个勾选框
    const boxes = screen.getAllByRole("checkbox");
    expect(boxes).toHaveLength(2);
    // SecurityHealth 那一行不该有 checkbox
    expect(screen.queryByLabelText("选择 SecurityHealth")).toBeNull();
    expect(screen.getByLabelText("选择 GhostLauncher")).toBeTruthy();
  });

  it("卸载残留 tab 只读：没有勾选框也没有删除按钮", async () => {
    mockInvoke();
    render(<RegistryHealth />);
    await screen.findByText("启动项（3）");

    fireEvent.click(screen.getByText("卸载残留（1）"));

    expect(await screen.findByText("Google Chrome")).toBeTruthy();
    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
    expect(screen.queryByRole("button", { name: /删除/ })).toBeNull();
    expect(screen.getAllByText(/本工具不代删注册表键/).length).toBeGreaterThan(0);
  });

  it("计划任务 tab 只读，且说明不代删", async () => {
    mockInvoke();
    render(<RegistryHealth />);
    await screen.findByText("启动项（3）");

    fireEvent.click(screen.getByText("计划任务（1）"));

    expect(await screen.findByText("MyAppUpdate")).toBeTruthy();
    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
    expect(screen.getAllByText(/本工具不代删计划任务/).length).toBeGreaterThan(0);
  });

  it("计划任务目录读不到时提示「需要管理员权限」，而不是「没有计划任务」", async () => {
    mockInvoke({
      registry_health_scan: () =>
        Promise.resolve(
          ok({ ...REPORT, taskItems: [], taskScanAvailable: false })
        ),
    });
    render(<RegistryHealth />);
    await screen.findByText("启动项（3）");

    fireEvent.click(screen.getByText("计划任务（0）"));

    expect(await screen.findByText(/需要管理员权限/)).toBeTruthy();
    // 关键：不能说「没有计划任务」——那是骗用户
    expect(screen.queryByText("没有发现计划任务。")).toBeNull();
  });

  it("默认不勾选，勾选后出现「删除并备份」按钮", async () => {
    mockInvoke();
    render(<RegistryHealth />);
    await screen.findByText("GhostLauncher");

    expect(screen.queryByRole("button", { name: /删除并备份/ })).toBeNull();

    fireEvent.click(screen.getByLabelText("选择 GhostLauncher"));

    expect(await screen.findByText("删除并备份（1 项）")).toBeTruthy();
  });

  it("取消确认时一个 IPC 都不发", async () => {
    vi.mocked(window.confirm).mockReturnValue(false);
    mockInvoke();
    render(<RegistryHealth />);
    await screen.findByText("GhostLauncher");

    fireEvent.click(screen.getByLabelText("选择 GhostLauncher"));
    fireEvent.click(screen.getByText("删除并备份（1 项）"));

    await waitFor(() => expect(window.confirm).toHaveBeenCalled());
    expect(callsOf("registry_health_delete")).toHaveLength(0);
  });

  it("确认后调用 delete，实参是 RunTarget 数组（拆出 root/subkey）", async () => {
    mockInvoke();
    render(<RegistryHealth />);
    await screen.findByText("GhostLauncher");

    fireEvent.click(screen.getByLabelText("选择 GhostLauncher"));
    fireEvent.click(screen.getByText("删除并备份（1 项）"));

    await waitFor(() => expect(callsOf("registry_health_delete")).toHaveLength(1));
    expect(callsOf("registry_health_delete")[0]).toEqual({
      targets: [
        {
          root: "HKCU",
          subkey: "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
          valueName: "GhostLauncher",
        },
      ],
    });
  });

  it("结果里必须显示备份文件路径", async () => {
    mockInvoke();
    render(<RegistryHealth />);
    await screen.findByText("GhostLauncher");

    fireEvent.click(screen.getByLabelText("选择 GhostLauncher"));
    fireEvent.click(screen.getByText("删除并备份（1 项）"));

    expect(await screen.findByText(/已删除 1 个启动项/)).toBeTruthy();
    expect(screen.getByText(/备份文件（双击即可还原）/)).toBeTruthy();
    expect(screen.getByText(/registry_backups/)).toBeTruthy();
  });

  it("2002（L3 红线）时给出明确提示", async () => {
    mockInvoke({
      registry_health_delete: () =>
        Promise.resolve({ code: 2002, message: "检测到受保护路径，操作已拒绝（L3 红线）", data: null }),
    });
    render(<RegistryHealth />);
    await screen.findByText("GhostLauncher");

    fireEvent.click(screen.getByLabelText("选择 GhostLauncher"));
    fireEvent.click(screen.getByText("删除并备份（1 项）"));

    expect(await screen.findByText("未能删除")).toBeTruthy();
    expect(screen.getByText(/安全红线/)).toBeTruthy();
  });

  it("扫描失败时展示原因，不留旧列表", async () => {
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "registry_health_scan") return Promise.reject("注册表读取失败：拒绝访问");
      return Promise.resolve(ok(null));
    }) as unknown as typeof invoke);

    render(<RegistryHealth />);

    expect(await screen.findByText("扫描失败")).toBeTruthy();
    expect(screen.getByText(/拒绝访问/)).toBeTruthy();
    expect(screen.queryByText("GhostLauncher")).toBeNull();
  });
});
