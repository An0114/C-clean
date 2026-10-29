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
    const cb = screen.getByRole("checkbox") as HTMLInputElement;
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
    expect(await screen.findByText("已关闭")).toBeInTheDocument();
    const cb = screen.getByRole("checkbox") as HTMLInputElement;
    expect(cb.checked).toBe(false);

    fireEvent.click(cb);
    await waitFor(() => {
      expect(setPrefsPayload).not.toBeNull();
    });
    // task-058：set_prefs 是「整体覆盖」，前端必须连当前白名单一起提交，
    // 否则会把用户已配好的白名单清空。这里 get_prefs 没给白名单 → 空数组。
    expect(setPrefsPayload).toEqual({
      prefs: { autoRestartOnClose: true, restartWhitelist: [] },
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
      prefs: { autoRestartOnClose: true, restartWhitelist: ["notepad.exe", "code.exe"] },
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
      prefs: { autoRestartOnClose: false, restartWhitelist: [] },
    });
    expect(await screen.findByText("白名单已清空（不限制重启）")).toBeInTheDocument();
    expect(await screen.findByText("当前：不限制")).toBeInTheDocument();
  });
});
