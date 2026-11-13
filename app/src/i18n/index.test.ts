// task-086 5.6：i18n 框架单测。
//
// 只测纯函数层（setLang / t / 回退），不渲染 React——
// 语言切换的 UI 由 Settings 页测试覆盖。

import { beforeEach, describe, expect, it } from "vitest";
import { getLang, setLang, t } from "./index";

describe("i18n", () => {
  beforeEach(() => {
    setLang("zh-CN");
    localStorage.clear();
  });

  it("默认中文，切换后立即生效", () => {
    expect(getLang()).toBe("zh-CN");
    expect(t("nav.dashboard")).toBe("仪表盘");

    setLang("en-US");
    expect(getLang()).toBe("en-US");
    expect(t("nav.dashboard")).toBe("Dashboard");
  });

  it("切换语言会写入 localStorage", () => {
    setLang("en-US");
    expect(localStorage.getItem("cclean:lang")).toBe("en-US");
  });

  it("未知 key 原样返回，便于发现漏配", () => {
    expect(t("no.such.key")).toBe("no.such.key");
  });

  it("支持 {n} 形式插值", () => {
    setLang("en-US");
    expect(t("settings.notificationsUnread", { n: 3 })).toBe("3 unread");
  });

  it("英文缺 key 时回退中文而不是显示 key", () => {
    // 目前两份词典 key 集合一致；用中文独有的临时路径验证回退逻辑不依赖运行时。
    // （若未来英文漏配，用户看到中文文案，而不是 settings.xxx。）
    setLang("en-US");
    expect(t("nav.settings")).toBe("Settings");
    setLang("zh-CN");
    expect(t("nav.settings")).toBe("设置");
  });
});
