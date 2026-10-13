// task-032：formatTs 纯函数单测。
import { describe, it, expect } from "vitest";
import { formatTs } from "./formatTimestamp";

describe("formatTs", () => {
  it("0 → -", () => {
    expect(formatTs(0)).toBe("-");
  });
  it("NaN → -", () => {
    expect(formatTs(NaN)).toBe("-");
  });
  it("负数 → -", () => {
    expect(formatTs(-1)).toBe("-");
  });
  it("合法 unix 秒 → 非 - 的字符串", () => {
    const s = formatTs(1700000000);
    expect(s).not.toBe("-");
    expect(typeof s).toBe("string");
    expect(s.length).toBeGreaterThan(0);
  });
});
