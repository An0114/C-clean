import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import History from "./History";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const mockedInvoke = vi.mocked(invoke);
const ok = (data: unknown) => ({ code: 0, message: "ok", data });

const sessions = [
  { id: 2, startedAt: 2000, status: "finished" },
  { id: 1, startedAt: 1000, status: "finished" },
];

beforeEach(() => {
  mockedInvoke.mockReset();
  mockedInvoke.mockImplementation(((cmd: string) => {
    if (cmd === "scan_session_list") return Promise.resolve(ok(sessions));
    if (cmd === "audit_log_list") {
      return Promise.resolve(ok([
        { id: 1, sessionId: 1, eventType: "permanent_delete", path: "C:\\Temp\\a.tmp", detail: null, createdAt: 1000 },
      ]));
    }
    return Promise.resolve(ok(null));
  }) as unknown as typeof invoke);
});

afterEach(cleanup);

describe("History 会话筛选", () => {
  it("加载扫描会话下拉并默认查询全部会话", async () => {
    render(<History />);
    expect(await screen.findByTestId("session-filter")).toBeTruthy();
    expect(screen.getByText(/#2 ·/)).toBeTruthy();
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledWith("audit_log_list", {
      sessionId: null,
      limit: 20,
      offset: 0,
    }));
  });

  it("选择会话后从第一页按 sessionId 重新查询", async () => {
    render(<History />);
    const select = await screen.findByTestId("session-filter");
    fireEvent.change(select, { target: { value: "2" } });
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledWith("audit_log_list", {
      sessionId: 2,
      limit: 20,
      offset: 0,
    }));
  });
});
