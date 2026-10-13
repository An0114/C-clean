// task-032：formatBytes 纯函数单测。
import { describe, it, expect } from "vitest";
import { formatBytes } from "./formatBytes";

describe("formatBytes", () => {
  it("0 → 0 B", () => {
    expect(formatBytes(0)).toBe("0 B");
  });
  it("500 B", () => {
    expect(formatBytes(500)).toBe("500 B");
  });
  it("2048 → 2.0 KB", () => {
    expect(formatBytes(2048)).toBe("2.0 KB");
  });
  it("5 MB", () => {
    expect(formatBytes(5 * 1024 * 1024)).toBe("5.0 MB");
  });
  it("2 GB", () => {
    expect(formatBytes(2 * 1024 * 1024 * 1024)).toBe("2.0 GB");
  });
});
