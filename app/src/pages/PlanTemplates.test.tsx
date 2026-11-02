// task-072：清理方案模板页的渲染与交互用例。
//
// 重点不在「页面能不能渲染」，而在四条容易被做错的语义：
//   ① 没勾任何条目时「保存」必须禁用（而不是点了没反应或存出空方案）；
//   ② 条目数要区分「方案里存了几条」和「本次匹配到几条」——只显示匹配数会让人
//      以为方案坏了；
//   ③ 应用方案时，一条都没匹配上就**不要切页**（否则用户到清理页发现什么都没勾）；
//   ④ 导出后必须显示完整路径（没有文件选择框，路径就是用户唯一的抓手）。

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, fireEvent, cleanup } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import PlanTemplates from "./PlanTemplates";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);
const ok = (data: unknown) => ({ code: 0, message: "ok", data });

const PLAN_A = {
  id: 1,
  name: "开发机日常",
  note: null,
  itemCount: 12,
  matchedCount: 12,
  sizeBytes: 1024 * 1024 * 3,
  createdAt: 1762000000,
  updatedAt: 1762000000,
};

/** 只匹配到 3 条的那种（模拟重扫后大部分条目已被清掉）。 */
const PLAN_PARTIAL = {
  id: 2,
  name: "老方案",
  note: null,
  itemCount: 12,
  matchedCount: 3,
  sizeBytes: 2048,
  createdAt: 1761900000,
  updatedAt: 1761900000,
};

/** 按命令分派的 mock：默认 plan_list 返回给定列表。 */
function mockWith(plans: unknown[]) {
  mockedInvoke.mockImplementation(((cmd: string) => {
    if (cmd === "plan_list") return Promise.resolve(ok(plans));
    return Promise.resolve(ok(null));
  }) as unknown as typeof invoke);
}

beforeEach(() => {
  mockedInvoke.mockReset();
});

afterEach(() => {
  cleanup();
});

describe("方案模板页", () => {
  it("挂载时拉取方案列表，并渲染名称/时间/条目数/大小", async () => {
    mockWith([PLAN_A]);
    render(<PlanTemplates currentSelection={[]} onApply={() => {}} />);

    expect(await screen.findByText("开发机日常")).toBeTruthy();
    // 条目数
    expect(screen.getByText(/12 条/)).toBeTruthy();
    // 大小（3 MB）
    expect(screen.getByText("3.0 MB")).toBeTruthy();
    // 创建时间不是空
    expect(screen.queryByText("—")).toBeNull();
    // 调用了 plan_list 且无参
    expect(mockedInvoke).toHaveBeenCalledWith("plan_list", {});
  });

  it("空列表给出可执行的下一步，而不是一片空白", async () => {
    mockWith([]);
    render(<PlanTemplates currentSelection={[]} onApply={() => {}} />);
    expect(await screen.findByText(/还没有保存任何方案/)).toBeTruthy();
    expect(screen.getByText(/在「清理」页勾选条目/)).toBeTruthy();
  });

  it("没勾选条目时「保存」禁用，并说明原因", async () => {
    mockWith([]);
    render(<PlanTemplates currentSelection={[]} onApply={() => {}} />);
    await screen.findByText(/还没有保存任何方案/);

    const btn = screen.getByRole("button", { name: /保存（0 项）/ }) as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
    expect(screen.getByText(/当前没有勾选任何条目/)).toBeTruthy();
  });

  it("勾选了条目但没填名字时「保存」仍禁用", async () => {
    mockWith([]);
    render(<PlanTemplates currentSelection={[1, 2, 3]} onApply={() => {}} />);
    await screen.findByText(/还没有保存任何方案/);

    const btn = screen.getByRole("button", { name: /保存（3 项）/ }) as HTMLButtonElement;
    expect(btn.disabled).toBe(true);

    fireEvent.change(screen.getByLabelText("方案名"), { target: { value: "我的方案" } });
    await waitFor(() => {
      expect(
        (screen.getByRole("button", { name: /保存（3 项）/ }) as HTMLButtonElement).disabled,
      ).toBe(false);
    });
  });

  it("保存时把 name 与当前勾选的 itemIds 一起发给后端", async () => {
    mockWith([]);
    render(<PlanTemplates currentSelection={[7, 8]} onApply={() => {}} />);
    await screen.findByText(/还没有保存任何方案/);

    fireEvent.change(screen.getByLabelText("方案名"), { target: { value: "  我的方案  " } });
    fireEvent.click(screen.getByRole("button", { name: /保存（2 项）/ }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("plan_save", {
        name: "我的方案", // 必须 trim
        itemIds: [7, 8],
      });
    });
    // 保存成功后提示 + 重新拉列表
    expect(await screen.findByText(/已保存方案「我的方案」，共 2 项/)).toBeTruthy();
    expect(mockedInvoke).toHaveBeenCalledWith("plan_list", {});
  });

  it("保存返回 2001（空清单）时把后端文案显示出来", async () => {
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "plan_list") return Promise.resolve(ok([]));
      if (cmd === "plan_save") {
        return Promise.resolve({ code: 2001, message: "待清理清单为空", data: null });
      }
      return Promise.resolve(ok(null));
    }) as unknown as typeof invoke);

    render(<PlanTemplates currentSelection={[1]} onApply={() => {}} />);
    await screen.findByText(/还没有保存任何方案/);
    fireEvent.change(screen.getByLabelText("方案名"), { target: { value: "x" } });
    fireEvent.click(screen.getByRole("button", { name: /保存（1 项）/ }));

    expect(await screen.findByText("待清理清单为空")).toBeTruthy();
  });

  it("「本次匹配」与方案条目数不同时两个数都要显示", async () => {
    mockWith([PLAN_PARTIAL]);
    render(<PlanTemplates currentSelection={[]} onApply={() => {}} />);

    expect(await screen.findByText("老方案")).toBeTruthy();
    expect(screen.getByText(/12 条/)).toBeTruthy();
    // 必须显式写「本次匹配 3」，否则用户会以为方案被改小了
    expect(screen.getByText(/（本次匹配 3）/)).toBeTruthy();
  });

  it("一条都没匹配上时显示「—」而不是 0 B", async () => {
    mockWith([{ ...PLAN_PARTIAL, matchedCount: 0, sizeBytes: 0 }]);
    render(<PlanTemplates currentSelection={[]} onApply={() => {}} />);

    expect(await screen.findByText("老方案")).toBeTruthy();
    expect(screen.getByText("—")).toBeTruthy();
    expect(screen.queryByText("0 B")).toBeNull();
  });

  it("应用方案：把 itemIds 与提示交给外层", async () => {
    const onApply = vi.fn();
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "plan_list") return Promise.resolve(ok([PLAN_A]));
      if (cmd === "plan_apply") {
        return Promise.resolve(
          ok({
            id: 1,
            name: "开发机日常",
            itemIds: [11, 12],
            unmatched: [],
            sizeBytes: 100,
          }),
        );
      }
      return Promise.resolve(ok(null));
    }) as unknown as typeof invoke);

    render(<PlanTemplates currentSelection={[]} onApply={onApply} />);
    await screen.findByText("开发机日常");
    fireEvent.click(screen.getByRole("button", { name: "应用" }));

    await waitFor(() => {
      expect(onApply).toHaveBeenCalledWith({
        itemIds: [11, 12],
        notice: "已应用方案「开发机日常」：勾选 2 项",
      });
    });
  });

  it("应用方案时有未匹配项，提示里必须说明有几条没找到", async () => {
    const onApply = vi.fn();
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "plan_list") return Promise.resolve(ok([PLAN_PARTIAL]));
      if (cmd === "plan_apply") {
        return Promise.resolve(
          ok({
            id: 2,
            name: "老方案",
            itemIds: [21, 22, 23],
            unmatched: ["C:\\a", "C:\\b", "C:\\c"],
            sizeBytes: 2048,
          }),
        );
      }
      return Promise.resolve(ok(null));
    }) as unknown as typeof invoke);

    render(<PlanTemplates currentSelection={[]} onApply={onApply} />);
    await screen.findByText("老方案");
    fireEvent.click(screen.getByRole("button", { name: "应用" }));

    await waitFor(() => {
      const arg = onApply.mock.calls[0][0];
      expect(arg.itemIds).toEqual([21, 22, 23]);
      expect(arg.notice).toContain("另有 3 条本次没找到");
    });
  });

  it("应用方案一条都没匹配上时：不切页，直接报错说明", async () => {
    const onApply = vi.fn();
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "plan_list") return Promise.resolve(ok([PLAN_PARTIAL]));
      if (cmd === "plan_apply") {
        return Promise.resolve(
          ok({ id: 2, name: "老方案", itemIds: [], unmatched: [], sizeBytes: 0 }),
        );
      }
      return Promise.resolve(ok(null));
    }) as unknown as typeof invoke);

    render(<PlanTemplates currentSelection={[]} onApply={onApply} />);
    await screen.findByText("老方案");
    fireEvent.click(screen.getByRole("button", { name: "应用" }));

    expect(await screen.findByText(/本次都没找到/)).toBeTruthy();
    expect(onApply).not.toHaveBeenCalled();
  });

  it("导出后显示完整路径（没有文件选择框，路径是唯一抓手）", async () => {
    const exported = "C:\\Users\\An\\AppData\\Roaming\\com.and0114.cclean\\plans\\exports\\x-1.json";
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "plan_list") return Promise.resolve(ok([PLAN_A]));
      if (cmd === "plan_export") return Promise.resolve(ok(exported));
      return Promise.resolve(ok(null));
    }) as unknown as typeof invoke);

    render(<PlanTemplates currentSelection={[]} onApply={() => {}} />);
    await screen.findByText("开发机日常");
    fireEvent.click(screen.getByRole("button", { name: "导出" }));

    const pathBox = await screen.findByTestId("plan-export-path");
    expect(pathBox.textContent).toBe(exported);
    expect(mockedInvoke).toHaveBeenCalledWith("plan_export", { id: 1 });
  });

  it("导入：路径为空时一个 IPC 都不发", async () => {
    mockWith([]);
    render(<PlanTemplates currentSelection={[]} onApply={() => {}} />);
    await screen.findByText(/还没有保存任何方案/);

    fireEvent.click(screen.getByRole("button", { name: "导入" }));

    expect(await screen.findByText(/请填写要导入的 JSON 文件完整路径/)).toBeTruthy();
    expect(mockedInvoke).not.toHaveBeenCalledWith("plan_import", expect.anything());
  });

  it("导入：把路径原样发给后端并刷新列表", async () => {
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "plan_list") return Promise.resolve(ok([]));
      if (cmd === "plan_import") return Promise.resolve(ok(9));
      return Promise.resolve(ok(null));
    }) as unknown as typeof invoke);

    render(<PlanTemplates currentSelection={[]} onApply={() => {}} />);
    await screen.findByText(/还没有保存任何方案/);

    fireEvent.change(screen.getByLabelText("导入文件路径"), {
      target: { value: "  D:\\tmp\\p.json  " },
    });
    fireEvent.click(screen.getByRole("button", { name: "导入" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("plan_import", {
        filePath: "D:\\tmp\\p.json", // 必须 trim
      });
    });
    expect(await screen.findByText("已导入为新方案")).toBeTruthy();
  });

  it("删除必须过 confirm；取消时一个 IPC 都不发", async () => {
    mockWith([PLAN_A]);
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(false);

    render(<PlanTemplates currentSelection={[]} onApply={() => {}} />);
    await screen.findByText("开发机日常");
    fireEvent.click(screen.getByRole("button", { name: "删除" }));

    expect(confirmSpy).toHaveBeenCalled();
    expect(mockedInvoke).not.toHaveBeenCalledWith("plan_delete", expect.anything());
    confirmSpy.mockRestore();
  });

  it("确认后删除，并把 id 发给后端", async () => {
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
    mockedInvoke.mockImplementation(((cmd: string) => {
      if (cmd === "plan_list") return Promise.resolve(ok([PLAN_A]));
      if (cmd === "plan_delete") return Promise.resolve(ok(1));
      return Promise.resolve(ok(null));
    }) as unknown as typeof invoke);

    render(<PlanTemplates currentSelection={[]} onApply={() => {}} />);
    await screen.findByText("开发机日常");
    fireEvent.click(screen.getByRole("button", { name: "删除" }));

    await waitFor(() => {
      expect(mockedInvoke).toHaveBeenCalledWith("plan_delete", { id: 1 });
    });
    expect(await screen.findByText(/已删除方案「开发机日常」/)).toBeTruthy();
    confirmSpy.mockRestore();
  });

  it("列表拉取失败时报错，不假装「没有方案」", async () => {
    mockedInvoke.mockImplementation((() =>
      Promise.reject(new Error("db locked"))) as unknown as typeof invoke);

    render(<PlanTemplates currentSelection={[]} onApply={() => {}} />);
    expect(await screen.findByText(/db locked/)).toBeTruthy();
  });
});
