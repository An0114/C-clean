// bugfix/task-035-updater-acl：回归守护 Tauri ACL 权限配置。
//
// 背景（task-035 评审 P0）：capabilities/default.json 只授权了 core:*，
// 前端 Settings.tsx 调用 plugin-updater 的 check() 时被 Tauri ACL 拒绝，
// 报 `Command plugin:updater|check not allowed by ACL`，自动更新完全不可用。
// 本测试直接读取 src-tauri 的能力清单，断言 updater / process 插件命令已放行，
// 防止权限被后续改动误删导致同类回归。
import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";

interface CapabilitiesFile {
  identifier: string;
  permissions: string[];
}

function loadDefaultCapabilities(): CapabilitiesFile {
  // vitest/pnpm 从 app/ 目录运行，直接按 cwd 定位 src-tauri 能力清单
  return JSON.parse(
    readFileSync("src-tauri/capabilities/default.json", "utf-8"),
  ) as CapabilitiesFile;
}

describe("Tauri capabilities/default.json（bugfix task-035-updater-acl）", () => {
  it("放行 updater 插件（check / downloadAndInstall）", () => {
    const { permissions } = loadDefaultCapabilities();
    // updater:default 聚合了 allow-check + allow-download + allow-install + allow-download-and-install
    expect(permissions).toContain("updater:default");
  });

  it("放行 process 插件（relaunch() → restart 命令）", () => {
    const { permissions } = loadDefaultCapabilities();
    // tauri-plugin-process v2 无独立 allow-relaunch；relaunch() 映射到 restart，
    // process:default 聚合 allow-exit + allow-restart。
    expect(permissions).toContain("process:default");
  });

  it("保留既有扫描事件与 core 默认能力", () => {
    const { permissions } = loadDefaultCapabilities();
    expect(permissions).toContain("core:default");
    expect(permissions).toContain("core:event:default");
  });
});
