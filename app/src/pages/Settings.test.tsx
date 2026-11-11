// task-055 单元 5：设置页「手动检查应用更新」的回归用例。
//
// 覆盖：检查中按钮 disabled + 文案、结果文案带当前版本、失败文案复用
// classifyUpdateError、发现新版本时通过 onUpdateFound 通知外层强制显示 banner。

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, fireEvent, cleanup } from "@testing-library/react";
import Settings from "./Settings";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: vi.fn() }));
vi.mock("@tauri-apps/plugin-updater", () => ({ check: vi.fn() }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: vi.fn() }));

import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { check } from "@tauri-apps/plugin-updater";

const mockedInvoke = vi.mocked(invoke);
const mockedGetVersion = vi.mocked(getVersion);
const mockedCheck = vi.mocked(check);

const ok = (data: unknown) => ({ code: 0, message: "ok", data });

// task-084 B1/B5：get_prefs 未给这三项时，前端按 Settings.tsx 的默认值兜底；
// set_prefs 是整体覆盖，提交体会带上这些默认值。断言需与之一致。
const DEFAULT_EXTRA_PREFS = {
  autoCleanL1: false,
  autoCleanL1Threshold: 500 * 1024 * 1024,
  devCacheGradleMaven: false,
};

function makeUpdate(version = "0.1.7") {
  return {
    version,
    body: "",
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
  mockedGetVersion.mockResolvedValue("0.1.6");
  mockedInvoke.mockImplementation(((cmd: string) => {
    if (cmd === "rules_check_update") {
      return Promise.resolve(
        ok({
          status: "upToDate",
          localVersion: "1.0.0",
          localSha256: "a".repeat(64),
          remoteVersion: "1.0.0",
          remoteSha256: "a".repeat(64),
          message: "已是最新",
        })
      );
    }
    if (cmd === "rules_has_backup") return Promise.resolve(ok(false));
    return Promise.resolve(ok(null));
  }) as never);
});

afterEach(() => {
  cleanup();
});

describe("Settings 手动检查应用更新（task-055）", () => {
  it("检查中：按钮 disabled 且文案为「检查中…」", async () => {
    // 让 check 挂起，观察 loading 态
    let resolveCheck: (v: unknown) => void = () => {};
    mockedCheck.mockImplementation(
      () =>
        new Promise((r) => {
          resolveCheck = r;
        }) as never
    );

    render(<Settings />);
    const btn = await screen.findByText("检查应用更新");
    fireEvent.click(btn);

    // 注意：页面里「检查中…」可能同时出现在「远端版本」行（规则库在查）和按钮上，
    // 所以要按 tagName 精确取按钮，不能直接 findByText（会多匹配）。
    await waitFor(() => {
      expect(screen.queryByText("检查应用更新")).not.toBeInTheDocument();
    });
    const loadingBtn = screen
      .getAllByText("检查中…")
      .find((el) => el.tagName === "BUTTON");
    expect(loadingBtn).toBeDefined();
    expect(loadingBtn).toBeDisabled();

    resolveCheck(null);
    await waitFor(() => expect(screen.getByText("检查应用更新")).toBeInTheDocument());
  });

  it("发现新版本：文案「当前 v0.1.6 / 发现 v0.1.7」并通知外层", async () => {
    mockedCheck.mockResolvedValue(makeUpdate("0.1.7") as never);
    const onUpdateFound = vi.fn();

    render(<Settings onUpdateFound={onUpdateFound} />);
    fireEvent.click(await screen.findByText("检查应用更新"));

    expect(await screen.findByText("当前 v0.1.6 / 发现 v0.1.7")).toBeInTheDocument();
    expect(onUpdateFound).toHaveBeenCalledWith("0.1.7");
  });

  it("无更新：文案「当前 v0.1.6，已是最新版本」且不通知外层", async () => {
    mockedCheck.mockResolvedValue(null as never);
    const onUpdateFound = vi.fn();

    render(<Settings onUpdateFound={onUpdateFound} />);
    fireEvent.click(await screen.findByText("检查应用更新"));

    expect(await screen.findByText("当前 v0.1.6，已是最新版本")).toBeInTheDocument();
    expect(onUpdateFound).not.toHaveBeenCalled();
  });

  it("检查失败：显示中文映射文案（不是原始英文）", async () => {
    mockedCheck.mockRejectedValue(new Error("Network error: connection refused"));

    render(<Settings />);
    fireEvent.click(await screen.findByText("检查应用更新"));

    expect(
      await screen.findByText("网络连接失败，请检查网络后重试")
    ).toBeInTheDocument();
    expect(screen.queryByText(/Network error/)).not.toBeInTheDocument();
  });

  it("取不到当前版本时文案退化为不带「当前 vX」", async () => {
    mockedGetVersion.mockRejectedValue(new Error("acl denied"));
    mockedCheck.mockResolvedValue(makeUpdate("0.1.7") as never);

    render(<Settings />);
    fireEvent.click(await screen.findByText("检查应用更新"));

    expect(await screen.findByText("当前版本 / 发现 v0.1.7")).toBeInTheDocument();
  });
});

// task-057：行为偏好 toggle
describe("Settings 行为偏好（task-057）", () => {
  it("启动时按 get_prefs 回填开关（true → 已开启）", async () => {
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "rules_check_update") {
        return Promise.resolve(
          ok({
            status: "upToDate",
            localVersion: "1.0.0",
            localSha256: "a".repeat(64),
            remoteVersion: "1.0.0",
            remoteSha256: "a".repeat(64),
            message: "ok",
          })
        );
      }
      if (cmd === "rules_has_backup") return Promise.resolve(ok(false));
      if (cmd === "get_prefs") return Promise.resolve(ok({ autoRestartOnClose: true }));
      return Promise.resolve(ok(null));
    }) as never);

    render(<Settings />);
    // 文案「已开启」应出现
    expect(await screen.findByText("已开启")).toBeInTheDocument();
    const cb = screen.getAllByRole("checkbox")[0] as HTMLInputElement;
    expect(cb.checked).toBe(true);
  });

  it("默认 false：get_prefs 返回 false 时勾选框不勾；切换时调 set_prefs", async () => {
    let setPrefsPayload: unknown = null;
    mockedInvoke.mockImplementation(((cmd: string, args: unknown) => {
      if (cmd === "rules_check_update") {
        return Promise.resolve(
          ok({
            status: "upToDate",
            localVersion: "1.0.0",
            localSha256: "a".repeat(64),
            remoteVersion: "1.0.0",
            remoteSha256: "a".repeat(64),
            message: "ok",
          })
        );
      }
      if (cmd === "rules_has_backup") return Promise.resolve(ok(false));
      if (cmd === "get_prefs") return Promise.resolve(ok({ autoRestartOnClose: false }));
      if (cmd === "set_prefs") {
        setPrefsPayload = args;
        return Promise.resolve(ok({ autoRestartOnClose: (args as { prefs: { autoRestartOnClose: boolean } }).prefs.autoRestartOnClose }));
      }
      return Promise.resolve(ok(null));
    }) as never);

    render(<Settings />);
    // task-061 起页面有多个「已关闭」（自动重启 + 自动扫描），取第一个（自动重启）。
    expect((await screen.findAllByText("已关闭")).length).toBeGreaterThan(0);
    const cb = screen.getAllByRole("checkbox")[0] as HTMLInputElement;
    expect(cb.checked).toBe(false);

    fireEvent.click(cb);
    await waitFor(() => {
      expect(setPrefsPayload).not.toBeNull();
    });
    // task-058：set_prefs 是「整体覆盖」，前端必须连当前白名单一起提交，
    // 否则会把用户已配好的白名单清空。这里 get_prefs 没给白名单 → 空数组。
    expect(setPrefsPayload).toEqual({
      prefs: { autoRestartOnClose: true, restartWhitelist: [], theme: "system", autoScanEnabled: false, autoScanInterval: "daily", ...DEFAULT_EXTRA_PREFS },
    });
  });
});

// task-058：重启白名单编辑框
describe("Settings 重启白名单（task-058）", () => {
  /** 通用 mock：rules 相关返回 upToDate，prefs 由参数控制。 */
  function mockWithPrefs(prefsPayload: unknown, onSet?: (args: unknown) => unknown) {
    mockedInvoke.mockImplementation(((cmd: string, args: unknown) => {
      if (cmd === "rules_check_update") {
        return Promise.resolve(
          ok({
            status: "upToDate",
            localVersion: "1.0.0",
            localSha256: "a".repeat(64),
            remoteVersion: "1.0.0",
            remoteSha256: "a".repeat(64),
            message: "ok",
          })
        );
      }
      if (cmd === "rules_has_backup") return Promise.resolve(ok(false));
      if (cmd === "get_prefs") return Promise.resolve(ok(prefsPayload));
      if (cmd === "set_prefs") {
        return Promise.resolve(ok(onSet ? onSet(args) : null));
      }
      return Promise.resolve(ok(null));
    }) as never);
  }

  it("启动时把已存白名单回填到 textarea（每行一条）", async () => {
    mockWithPrefs({ autoRestartOnClose: true, restartWhitelist: ["notepad.exe", "*.exe"] });

    render(<Settings />);
    const ta = (await screen.findByLabelText("重启白名单")) as HTMLTextAreaElement;
    await waitFor(() => {
      expect(ta.value).toBe("notepad.exe\n*.exe");
    });
    expect(screen.getByText("当前生效 2 条")).toBeInTheDocument();
  });

  it("白名单为空时显示「当前：不限制」", async () => {
    mockWithPrefs({ autoRestartOnClose: false, restartWhitelist: [] });

    render(<Settings />);
    expect(await screen.findByText("当前：不限制")).toBeInTheDocument();
  });

  it("编辑后点保存：按行解析、去空白、丢空行，并带上当前开关值", async () => {
    let setPayload: unknown = null;
    mockWithPrefs({ autoRestartOnClose: true, restartWhitelist: [] }, (args) => {
      setPayload = args;
      return {
        autoRestartOnClose: true,
        restartWhitelist: ["notepad.exe", "code.exe"],
      };
    });

    render(<Settings />);
    const ta = (await screen.findByLabelText("重启白名单")) as HTMLTextAreaElement;

    fireEvent.change(ta, { target: { value: "  notepad.exe \n\n  code.exe  \n   " } });
    fireEvent.click(screen.getByText("保存白名单"));

    await waitFor(() => expect(setPayload).not.toBeNull());
    expect(setPayload).toEqual({
      prefs: { autoRestartOnClose: true, restartWhitelist: ["notepad.exe", "code.exe"], theme: "system", autoScanEnabled: false, autoScanInterval: "daily", ...DEFAULT_EXTRA_PREFS },
    });
    // 保存后用后端返回值回填（去空白后的真实生效内容）
    await waitFor(() => expect(ta.value).toBe("notepad.exe\ncode.exe"));
    expect(await screen.findByText("白名单已保存（2 条）")).toBeInTheDocument();
  });

  it("清空白名单后保存 → 提交空数组并提示「不限制」", async () => {
    let setPayload: unknown = null;
    mockWithPrefs({ autoRestartOnClose: false, restartWhitelist: ["notepad.exe"] }, (args) => {
      setPayload = args;
      return { autoRestartOnClose: false, restartWhitelist: [] };
    });

    render(<Settings />);
    const ta = (await screen.findByLabelText("重启白名单")) as HTMLTextAreaElement;
    await waitFor(() => expect(ta.value).toBe("notepad.exe"));

    fireEvent.change(ta, { target: { value: "" } });
    fireEvent.click(screen.getByText("保存白名单"));

    await waitFor(() => expect(setPayload).not.toBeNull());
    expect(setPayload).toEqual({
      prefs: { autoRestartOnClose: false, restartWhitelist: [], theme: "system", autoScanEnabled: false, autoScanInterval: "daily", ...DEFAULT_EXTRA_PREFS },
    });
    expect(await screen.findByText("白名单已清空（不限制重启）")).toBeInTheDocument();
    expect(await screen.findByText("当前：不限制")).toBeInTheDocument();
  });
});

// task-061：外观（主题三态）
describe("Settings 外观（task-061）", () => {
  function mockPrefs(theme: string) {
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "rules_check_update") {
        return Promise.resolve(
          ok({
            status: "upToDate",
            localVersion: "1.0.0",
            localSha256: "a".repeat(64),
            remoteVersion: "1.0.0",
            remoteSha256: "a".repeat(64),
            message: "ok",
          })
        );
      }
      if (cmd === "rules_has_backup") return Promise.resolve(ok(false));
      if (cmd === "get_prefs") {
        return Promise.resolve(
          ok({ autoRestartOnClose: false, restartWhitelist: [], theme })
        );
      }
      return Promise.resolve(ok(null));
    }) as never);
  }

  it("三个选项齐全，get_prefs 的 theme 决定默认选中", async () => {
    mockPrefs("dark");

    render(<Settings />);
    // 三个标签都在
    expect(await screen.findByText("跟随系统")).toBeInTheDocument();
    expect(screen.getByText("浅色")).toBeInTheDocument();
    expect(screen.getByText("深色")).toBeInTheDocument();

    // theme=dark → 「深色」单选被选中
    const radios = screen.getAllByRole("radio") as HTMLInputElement[];
    await waitFor(() => {
      const dark = radios.find((r) => r.closest("label")?.textContent?.includes("深色"));
      expect(dark?.checked).toBe(true);
    });
  });

  it("缺 theme 时默认「跟随系统」选中", async () => {
    mockPrefs("system");

    render(<Settings />);
    const radios = screen.getAllByRole("radio") as HTMLInputElement[];
    await waitFor(() => {
      const sys = radios.find((r) => r.closest("label")?.textContent?.includes("跟随系统"));
      expect(sys?.checked).toBe(true);
    });
  });

  it("点「深色」→ set_prefs 带 theme=dark 且通知外层立即生效", async () => {
    let payload: unknown = null;
    const onThemeChange = vi.fn();
    mockedInvoke.mockImplementation(((cmd: string, args: unknown) => {
      if (cmd === "rules_check_update") {
        return Promise.resolve(
          ok({
            status: "upToDate",
            localVersion: "1.0.0",
            localSha256: "a".repeat(64),
            remoteVersion: "1.0.0",
            remoteSha256: "a".repeat(64),
            message: "ok",
          })
        );
      }
      if (cmd === "rules_has_backup") return Promise.resolve(ok(false));
      if (cmd === "get_prefs") {
        return Promise.resolve(ok({ autoRestartOnClose: false, restartWhitelist: [], theme: "system" }));
      }
      if (cmd === "set_prefs") {
        payload = args;
        return Promise.resolve(ok({ autoRestartOnClose: false, restartWhitelist: [], theme: "dark" }));
      }
      return Promise.resolve(ok(null));
    }) as never);

    render(<Settings onThemeChange={onThemeChange} />);
    await screen.findByText("深色");

    const radios = screen.getAllByRole("radio") as HTMLInputElement[];
    const dark = radios.find((r) => r.closest("label")?.textContent?.includes("深色"))!;
    fireEvent.click(dark);

    await waitFor(() => expect(payload).not.toBeNull());
    // 整体覆盖语义：其它字段必须一起带上，否则会被清空
    expect(payload).toEqual({
      prefs: { autoRestartOnClose: false, restartWhitelist: [], theme: "dark", autoScanEnabled: false, autoScanInterval: "daily", ...DEFAULT_EXTRA_PREFS },
    });
    expect(onThemeChange).toHaveBeenCalledWith("dark");
  });
});

// task-061：自动扫描
describe("Settings 自动扫描（task-061）", () => {
  function mockScanPrefs(enabled: boolean, interval: string) {
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "rules_check_update") {
        return Promise.resolve(
          ok({
            status: "upToDate",
            localVersion: "1.0.0",
            localSha256: "a".repeat(64),
            remoteVersion: "1.0.0",
            remoteSha256: "a".repeat(64),
            message: "ok",
          })
        );
      }
      if (cmd === "rules_has_backup") return Promise.resolve(ok(false));
      if (cmd === "get_prefs") {
        return Promise.resolve(
          ok({
            autoRestartOnClose: false,
            restartWhitelist: [],
            theme: "system",
            autoScanEnabled: enabled,
            autoScanInterval: interval,
          })
        );
      }
      return Promise.resolve(ok(null));
    }) as never);
  }

  it("默认关闭：开关不勾，间隔选项禁用并提示先开开关", async () => {
    mockScanPrefs(false, "daily");

    render(<Settings />);
    expect(await screen.findByText("定期在后台扫描")).toBeInTheDocument();

    const boxes = screen.getAllByRole("checkbox") as HTMLInputElement[];
    // 第一个是自动重启，第二个才是自动扫描
    const scanBox = boxes[boxes.length - 1];
    await waitFor(() => expect(scanBox.checked).toBe(false));

    // 间隔单选全部禁用
    const intervals = screen.getAllByRole("radio").filter((r) =>
      ["每天", "每周", "每月"].includes(r.closest("label")?.textContent ?? "")
    ) as HTMLInputElement[];
    expect(intervals.length).toBe(3);
    expect(intervals.every((r) => r.disabled)).toBe(true);

    expect(screen.getByText(/先开启上面的开关，间隔选择才会生效/)).toBeInTheDocument();
  });

  it("开启后间隔可选；切换间隔提交 set_prefs 且带全字段", async () => {
    let payload: unknown = null;
    mockedInvoke.mockImplementation(((cmd: string, args: unknown) => {
      if (cmd === "rules_check_update") {
        return Promise.resolve(
          ok({
            status: "upToDate",
            localVersion: "1.0.0",
            localSha256: "a".repeat(64),
            remoteVersion: "1.0.0",
            remoteSha256: "a".repeat(64),
            message: "ok",
          })
        );
      }
      if (cmd === "rules_has_backup") return Promise.resolve(ok(false));
      if (cmd === "get_prefs") {
        return Promise.resolve(
          ok({
            autoRestartOnClose: false,
            restartWhitelist: [],
            theme: "system",
            autoScanEnabled: true,
            autoScanInterval: "daily",
          })
        );
      }
      if (cmd === "set_prefs") {
        payload = args;
        return Promise.resolve(
          ok({
            autoRestartOnClose: false,
            restartWhitelist: [],
            theme: "system",
            autoScanEnabled: true,
            autoScanInterval: "weekly",
          })
        );
      }
      return Promise.resolve(ok(null));
    }) as never);

    render(<Settings />);
    await screen.findByText("定期在后台扫描");

    // 开启态下间隔可选
    const weekly = screen
      .getAllByRole("radio")
      .find((r) => r.closest("label")?.textContent === "每周") as HTMLInputElement;
    await waitFor(() => expect(weekly.disabled).toBe(false));

    fireEvent.click(weekly);
    await waitFor(() => expect(payload).not.toBeNull());

    // 整体覆盖：五个字段都要在，少一个就会被清空
    expect(payload).toEqual({
      prefs: {
        autoRestartOnClose: false,
        restartWhitelist: [],
        theme: "system",
        autoScanEnabled: true,
        autoScanInterval: "weekly",
        ...DEFAULT_EXTRA_PREFS,
      },
    });
  });
});

// task-062：开机自启
describe("Settings 开机自启（task-062）", () => {
  function mockAutostart(enabled: boolean, onSet?: (v: boolean) => boolean) {
    mockedInvoke.mockImplementation(((cmd: string, args: unknown) => {
      if (cmd === "rules_check_update") {
        return Promise.resolve(
          ok({
            status: "upToDate",
            localVersion: "1.0.0",
            localSha256: "a".repeat(64),
            remoteVersion: "1.0.0",
            remoteSha256: "a".repeat(64),
            message: "ok",
          })
        );
      }
      if (cmd === "rules_has_backup") return Promise.resolve(ok(false));
      if (cmd === "get_prefs") {
        return Promise.resolve(
          ok({
            autoRestartOnClose: false,
            restartWhitelist: [],
            theme: "system",
            autoScanEnabled: false,
            autoScanInterval: "daily",
          })
        );
      }
      if (cmd === "get_autostart_enabled") return Promise.resolve(ok(enabled));
      if (cmd === "set_autostart_enabled") {
        const want = (args as { enabled: boolean }).enabled;
        return Promise.resolve(ok(onSet ? onSet(want) : want));
      }
      return Promise.resolve(ok(null));
    }) as never);
  }

  it("按 get_autostart_enabled 回填开关（true → 已开启）", async () => {
    mockAutostart(true);

    render(<Settings />);
    expect(await screen.findByText("开机自启")).toBeInTheDocument();

    // 自启开关是页面上最后一个 checkbox
    const boxes = screen.getAllByRole("checkbox") as HTMLInputElement[];
    await waitFor(() => expect(boxes[boxes.length - 1].checked).toBe(true));
  });

  it("默认关闭：get_autostart_enabled=false 时不勾；切换时调 set_autostart_enabled", async () => {
    let called: boolean | null = null;
    mockAutostart(false, (v) => {
      called = v;
      return v;
    });

    render(<Settings />);
    await screen.findByText("开机自启");

    const boxes = screen.getAllByRole("checkbox") as HTMLInputElement[];
    const box = boxes[boxes.length - 1];
    await waitFor(() => expect(box.checked).toBe(false));

    fireEvent.click(box);
    await waitFor(() => expect(called).not.toBeNull());
    expect(called).toBe(true);
  });

  it("写入失败时回滚开关并提示", async () => {
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "rules_check_update") {
        return Promise.resolve(ok({ status: "upToDate", localVersion: "1.0.0", localSha256: "a".repeat(64), remoteVersion: "1.0.0", remoteSha256: "a".repeat(64), message: "ok" }));
      }
      if (cmd === "rules_has_backup") return Promise.resolve(ok(false));
      if (cmd === "get_prefs") {
        return Promise.resolve(ok({ autoRestartOnClose: false, restartWhitelist: [], theme: "system", autoScanEnabled: false, autoScanInterval: "daily" }));
      }
      if (cmd === "get_autostart_enabled") return Promise.resolve(ok(false));
      if (cmd === "set_autostart_enabled") return Promise.reject(new Error("注册表写入被拒"));
      return Promise.resolve(ok(null));
    }) as never);

    render(<Settings />);
    await screen.findByText("开机自启");

    const boxes = screen.getAllByRole("checkbox") as HTMLInputElement[];
    const box = boxes[boxes.length - 1];
    fireEvent.click(box);

    // 回滚：仍是不勾
    await waitFor(() => expect(box.checked).toBe(false));
    expect(await screen.findByText(/设置开机自启失败/)).toBeInTheDocument();
  });

  // task-073：以下两条补的是「状态可信度」这一层。
  it("后端回读的状态与请求不一致时，以回读为准（不谎报已开启）", async () => {
    // 真实场景：组策略禁用了 Run 键写入，或安全软件静默拦掉。
    // 插件的 enable() 可能返回 Ok，但回读 is_enabled() 仍是 false。
    // 这种时候 UI 必须显示「已关闭」——否则用户以为设上了，实际没有。
    mockAutostart(false, (want) => !want);

    render(<Settings />);
    await screen.findByText("开机自启");

    const boxes = screen.getAllByRole("checkbox") as HTMLInputElement[];
    const box = boxes[boxes.length - 1];
    fireEvent.click(box);

    // 请求 true，后端回读 false → 最终必须回到不勾
    await waitFor(() => expect(box.checked).toBe(false));
    // 「已关闭」在页面上有多处（自动重启 / 自动扫描 / 开机自启），不能按唯一文本查。
    // 真正的判据是：没有任何开关处于「已开启」（本用例里三个开关都该是关的）。
    expect(screen.queryByText("已开启")).toBeNull();
  });

  it("说明开关状态存在注册表哪里（用户能自己去任务管理器核对）", async () => {
    mockAutostart(false);

    render(<Settings />);
    await screen.findByText("开机自启");

    // 路径写在界面上，用户不必翻文档就能核实「到底生效没有」
    expect(
      screen.getByText(/HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run/)
    ).toBeInTheDocument();
    expect(screen.getByText(/任务管理器 → 启动/)).toBeInTheDocument();
  });
});

// ============ task-073：系统级定时扫描（任务计划）============
//
// 这块的关键不是「开关能不能点」，而是**状态可信度**：
// - 查询失败时必须显示真实原因，绝不能伪装成「未注册」（用户会以为开关是关的）；
// - 注册后用后端回读的状态覆盖（schtasks 可能返回 0 但任务被组策略立刻禁掉）；
// - 主开关关闭时子开关要禁用（否则用户开了系统级定时也永远不会扫）。

describe("Settings 系统级定时扫描（task-073）", () => {
  function mockTask(
    status: "ok" | "err",
    registered = false,
    onCall?: (cmd: string, args: unknown) => void
  ) {
    mockedInvoke.mockImplementation(((cmd: string, args: unknown) => {
      if (onCall) onCall(cmd, args);
      if (cmd === "rules_check_update") {
        return Promise.resolve(
          ok({
            status: "upToDate",
            localVersion: "1.0.0",
            localSha256: "a".repeat(64),
            remoteVersion: "1.0.0",
            remoteSha256: "a".repeat(64),
            message: "ok",
          })
        );
      }
      if (cmd === "rules_has_backup") return Promise.resolve(ok(false));
      if (cmd === "get_prefs") {
        return Promise.resolve(
          ok({
            autoRestartOnClose: false,
            restartWhitelist: [],
            theme: "system",
            autoScanEnabled: true,
            autoScanInterval: "daily",
          })
        );
      }
      if (cmd === "get_autostart_enabled") return Promise.resolve(ok(false));
      if (cmd === "auto_scan_status") {
        if (status === "err") return Promise.reject(new Error("schtasks.exe 被安全策略拦截"));
        return Promise.resolve(
          ok({ registered, taskName: "C-Disk Cleaner Auto Scan", time: "09:00" })
        );
      }
      if (cmd === "auto_scan_enable") {
        return Promise.resolve(
          ok({ registered: true, taskName: "C-Disk Cleaner Auto Scan", time: "09:00" })
        );
      }
      if (cmd === "auto_scan_disable") {
        return Promise.resolve(
          ok({ registered: false, taskName: "C-Disk Cleaner Auto Scan", time: "09:00" })
        );
      }
      return Promise.resolve(ok(null));
    }) as never);
  }

  it("已注册时显示「已注册」，并显示任务名（用户可去任务计划程序核对）", async () => {
    mockTask("ok", true);
    render(<Settings />);

    expect(await screen.findByText("系统级定时（应用未运行时也扫描）")).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByLabelText("系统级定时扫描")).toBeChecked()
    );
    expect(screen.getByText("已注册")).toBeInTheDocument();
    expect(screen.getByText("C-Disk Cleaner Auto Scan")).toBeInTheDocument();
  });

  it("未注册时显示「未注册」且开关不勾", async () => {
    mockTask("ok", false);
    render(<Settings />);

    await screen.findByText("系统级定时（应用未运行时也扫描）");
    const box = (await screen.findByLabelText("系统级定时扫描")) as HTMLInputElement;
    await waitFor(() => expect(box.checked).toBe(false));
    expect(screen.getByText("未注册")).toBeInTheDocument();
  });

  it("查询失败时显示真实原因，不伪装成「未注册」", async () => {
    mockTask("err");
    render(<Settings />);

    await screen.findByText("系统级定时（应用未运行时也扫描）");
    // 原因必须透出来（可能是组策略/安全软件拦截）
    expect(await screen.findByText(/schtasks\.exe 被安全策略拦截/)).toBeInTheDocument();
    // 而且不能显示「未注册」——那是在说谎
    expect(screen.queryByText("未注册")).toBeNull();
  });

  it("开启时调 auto_scan_enable 并带 time=09:00", async () => {
    const calls: Array<{ cmd: string; args: unknown }> = [];
    mockTask("ok", false, (cmd, args) => calls.push({ cmd, args }));
    render(<Settings />);

    const box = (await screen.findByLabelText("系统级定时扫描")) as HTMLInputElement;
    await waitFor(() => expect(box.checked).toBe(false));
    fireEvent.click(box);

    await waitFor(() => {
      const c = calls.find((x) => x.cmd === "auto_scan_enable");
      expect(c).toBeTruthy();
      expect(c!.args).toMatchObject({ time: "09:00" });
    });
    // 用后端回读的状态覆盖
    await waitFor(() => expect(box.checked).toBe(true));
    expect(screen.getByText("已注册")).toBeInTheDocument();
  });

  it("关闭时调 auto_scan_disable", async () => {
    const calls: Array<{ cmd: string; args: unknown }> = [];
    mockTask("ok", true, (cmd, args) => calls.push({ cmd, args }));
    render(<Settings />);

    const box = (await screen.findByLabelText("系统级定时扫描")) as HTMLInputElement;
    await waitFor(() => expect(box.checked).toBe(true));
    fireEvent.click(box);

    await waitFor(() => expect(calls.some((x) => x.cmd === "auto_scan_disable")).toBe(true));
    await waitFor(() => expect(box.checked).toBe(false));
  });

  it("主开关关闭时系统级开关禁用（否则开了也永远不会扫）", async () => {
    // 单独铺一份 autoScanEnabled=false 的 mock
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "rules_check_update") {
        return Promise.resolve(
          ok({
            status: "upToDate",
            localVersion: "1.0.0",
            localSha256: "a".repeat(64),
            remoteVersion: "1.0.0",
            remoteSha256: "a".repeat(64),
            message: "ok",
          })
        );
      }
      if (cmd === "rules_has_backup") return Promise.resolve(ok(false));
      if (cmd === "get_prefs") {
        return Promise.resolve(
          ok({
            autoRestartOnClose: false,
            restartWhitelist: [],
            theme: "system",
            autoScanEnabled: false,
            autoScanInterval: "daily",
          })
        );
      }
      if (cmd === "get_autostart_enabled") return Promise.resolve(ok(false));
      if (cmd === "auto_scan_status") {
        return Promise.resolve(
          ok({ registered: false, taskName: "C-Disk Cleaner Auto Scan", time: "09:00" })
        );
      }
      return Promise.resolve(ok(null));
    }) as never);

    render(<Settings />);

    const box = (await screen.findByLabelText("系统级定时扫描")) as HTMLInputElement;
    await waitFor(() => expect(box.disabled).toBe(true));
    expect(screen.getByText(/先开启上面的「定期在后台扫描」/)).toBeInTheDocument();
  });
});

// ===================== task-078：用户自定义规则 =====================

/** 两条样例规则：一条启用 L1、一条停用 L2。 */
const RULES = [
  {
    id: 1,
    name: "我的缓存",
    pattern: "D:\\MyCache\\*",
    riskLevel: "L1",
    enabled: true,
    createdAt: 1700000000,
    updatedAt: 1700000000,
  },
  {
    id: 2,
    name: "旧日志",
    pattern: "D:\\Logs\\**\\*.log",
    riskLevel: "L2",
    enabled: false,
    createdAt: 1700000001,
    updatedAt: 1700000001,
  },
];

/** 取出某命令的全部调用实参。 */
function ruleCalls(cmd: string): Array<Record<string, unknown> | undefined> {
  return mockedInvoke.mock.calls
    .filter(([c]) => c === cmd)
    .map(([, args]) => args as Record<string, unknown> | undefined);
}

/** 在默认 mock 之上叠加 user_rules_* 的响应。 */
function withRules(rules: unknown[], extra: Record<string, () => Promise<unknown>> = {}) {
  mockedInvoke.mockImplementation(((cmd: string) => {
    if (extra[cmd]) return extra[cmd]();
    if (cmd === "user_rules_list") return Promise.resolve(ok(rules));
    if (cmd === "rules_has_backup") return Promise.resolve(ok(false));
    if (cmd === "rules_check_update") {
      return Promise.resolve(
        ok({
          status: "upToDate",
          localVersion: "1.0.0",
          localSha256: "a".repeat(64),
          remoteVersion: "1.0.0",
          remoteSha256: "a".repeat(64),
          message: "已是最新",
        })
      );
    }
    return Promise.resolve(ok(null));
  }) as never);
}

describe("自定义规则（task-078）", () => {
  it("列表渲染：名称 / 模式 / 风险徽标 / 启用开关", async () => {
    withRules(RULES);
    render(<Settings />);

    expect(await screen.findByText("我的缓存")).toBeInTheDocument();
    // 模式串在表格里出现一次，说明文案里也举了同一个例子 → 用 getAllByText，
    // 并确认其中至少有一个是表格单元格（不是只匹配到提示文字）
    const patternNodes = screen.getAllByText("D:\\MyCache\\*");
    expect(patternNodes.some((el) => el.tagName === "TD")).toBe(true);
    expect(screen.getByText("L1")).toBeInTheDocument();
    expect(screen.getByText("L2")).toBeInTheDocument();

    const on = screen.getByLabelText("启用规则 我的缓存") as HTMLInputElement;
    const off = screen.getByLabelText("启用规则 旧日志") as HTMLInputElement;
    expect(on.checked).toBe(true);
    expect(off.checked).toBe(false);
  });

  it("没有规则时给引导文案，而不是让人以为规则丢了", async () => {
    withRules([]);
    render(<Settings />);
    expect(await screen.findByText(/还没有自定义规则/)).toBeInTheDocument();
  });

  it("新增：弹窗填写后调用 user_rules_add，实参是 camelCase", async () => {
    let listCalls = 0;
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "user_rules_list") {
        listCalls += 1;
        // 第一次空，保存后返回一条（模拟刷新）
        return Promise.resolve(ok(listCalls === 1 ? [] : RULES.slice(0, 1)));
      }
      if (cmd === "user_rules_add") return Promise.resolve(ok(RULES[0]));
      if (cmd === "rules_has_backup") return Promise.resolve(ok(false));
      if (cmd === "rules_check_update") {
        return Promise.resolve(
          ok({
            status: "upToDate",
            localVersion: "1.0.0",
            localSha256: "a".repeat(64),
            remoteVersion: null,
            remoteSha256: null,
            message: "ok",
          })
        );
      }
      return Promise.resolve(ok(null));
    }) as never);

    render(<Settings />);
    fireEvent.click(await screen.findByText("新增规则"));

    // 弹窗出现
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("规则名称"), { target: { value: "新规则" } });
    fireEvent.change(screen.getByLabelText("路径模式"), { target: { value: "D:\\New\\*" } });
    fireEvent.change(screen.getByLabelText("风险等级"), { target: { value: "L2" } });
    fireEvent.click(screen.getByText("保存"));

    await waitFor(() => {
      expect(ruleCalls("user_rules_add")[0]).toEqual({
        name: "新规则",
        pattern: "D:\\New\\*",
        riskLevel: "L2",
      });
    });
    // 保存后弹窗关闭 + 列表刷新
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    expect(await screen.findByText("我的缓存")).toBeInTheDocument();
  });

  it("编辑：表单预填，保存时带上 id 与原启用状态", async () => {
    withRules(RULES, {
      user_rules_update: () => Promise.resolve(ok(RULES[1])),
    });
    render(<Settings />);

    // 第二行（停用那条）的「编辑」
    const editBtns = await screen.findAllByText("编辑");
    fireEvent.click(editBtns[1]);

    const nameInput = screen.getByLabelText("规则名称") as HTMLInputElement;
    expect(nameInput.value).toBe("旧日志");
    expect((screen.getByLabelText("路径模式") as HTMLInputElement).value).toBe(
      "D:\\Logs\\**\\*.log"
    );

    fireEvent.change(nameInput, { target: { value: "旧日志2" } });
    fireEvent.click(screen.getByText("保存"));

    await waitFor(() => {
      expect(ruleCalls("user_rules_update")[0]).toEqual({
        id: 2,
        name: "旧日志2",
        pattern: "D:\\Logs\\**\\*.log",
        riskLevel: "L2",
        // 关键：改名字不能把开关重置成「开」
        enabled: false,
      });
    });
  });

  it("开关：点一下就调 set_enabled，传相反值", async () => {
    withRules(RULES, {
      user_rules_set_enabled: () => Promise.resolve(ok(RULES[0])),
    });
    render(<Settings />);

    const on = (await screen.findByLabelText("启用规则 我的缓存")) as HTMLInputElement;
    fireEvent.click(on);

    await waitFor(() => {
      expect(ruleCalls("user_rules_set_enabled")[0]).toEqual({ id: 1, enabled: false });
    });
  });

  it("删除：confirm 取消则不调后端；确认才删", async () => {
    withRules(RULES, {
      user_rules_delete: () => Promise.resolve(ok(true)),
    });
    const spy = vi.spyOn(window, "confirm").mockReturnValue(false);
    render(<Settings />);

    const delBtns = await screen.findAllByText("删除");
    fireEvent.click(delBtns[0]);
    expect(spy).toHaveBeenCalled();
    expect(ruleCalls("user_rules_delete")).toHaveLength(0);

    spy.mockReturnValue(true);
    fireEvent.click(delBtns[0]);
    await waitFor(() => {
      expect(ruleCalls("user_rules_delete")[0]).toEqual({ id: 1 });
    });
    spy.mockRestore();
  });

  it("后端校验失败时把真实原因显示出来（不是「内部错误」）", async () => {
    withRules([], {
      user_rules_add: () =>
        Promise.reject("路径模式太宽泛：必须指定到具体目录，如 D:\\MyCache\\*"),
    });
    render(<Settings />);

    fireEvent.click(await screen.findByText("新增规则"));
    fireEvent.change(screen.getByLabelText("规则名称"), { target: { value: "x" } });
    fireEvent.change(screen.getByLabelText("路径模式"), { target: { value: "D:\\**" } });
    fireEvent.click(screen.getByText("保存"));

    expect(await screen.findByText(/路径模式太宽泛/)).toBeInTheDocument();
    // 失败时弹窗不关，用户可以直接改
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("规则名或模式为空时前端先拦一道，不发请求", async () => {
    withRules([]);
    render(<Settings />);

    fireEvent.click(await screen.findByText("新增规则"));
    fireEvent.click(screen.getByText("保存"));

    expect(await screen.findByText(/规则名与路径模式都不能为空/)).toBeInTheDocument();
    expect(ruleCalls("user_rules_add")).toHaveLength(0);
  });
});
