// bugfix/task-035-release-signing 的单元测试。
// 覆盖两条 bug 的判定逻辑：版本号解析/注入（bug-1）、签名产物校验与 latest.json（bug-2）。
// 跑法：node --test scripts/release-lib.test.mjs
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  normalizeVersion,
  patchTauriConfVersion,
  findUpdaterArtifact,
  verifyArtifacts,
  buildLatestJson,
} from "./release-lib.mjs";

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

test("normalizeVersion：手动输入的版本号优先，去掉 v 前缀", () => {
  assert.equal(normalizeVersion("0.2.0", "main"), "0.2.0");
  assert.equal(normalizeVersion("v0.2.0", "main"), "0.2.0");
  assert.equal(normalizeVersion("  0.2.0  ", "main"), "0.2.0");
});

test("normalizeVersion：没有输入时回退到 tag 名", () => {
  assert.equal(normalizeVersion("", "v0.1.7"), "0.1.7");
  assert.equal(normalizeVersion(undefined, "v0.1.7"), "0.1.7");
});

test("normalizeVersion：分支名不能被当成版本号（原 bug-1 成因之一）", () => {
  assert.throws(() => normalizeVersion("", "bugfix/task-035-release-signing"), /版本号不合法/);
  assert.throws(() => normalizeVersion("", "feature/task-035-tauri-updater"), /版本号不合法/);
  assert.throws(() => normalizeVersion("", "main"), /版本号不合法/);
});

test("normalizeVersion：空值与非法值直接报错，不静默兜底", () => {
  assert.throws(() => normalizeVersion("", ""), /版本号为空/);
  assert.throws(() => normalizeVersion("abc", ""), /版本号不合法/);
  assert.throws(() => normalizeVersion("1.2", ""), /版本号不合法/);
});

test("normalizeVersion：预发布版本号合法", () => {
  assert.equal(normalizeVersion("0.2.0-beta.1", ""), "0.2.0-beta.1");
});

test("patchTauriConfVersion：替换顶层 version，保留其余字段", () => {
  const before = [
    "{",
    '  "$schema": "https://schema.tauri.app/config/2",',
    '  "productName": "C-DiskCleaner",',
    '  "version": "0.1.0",',
    '  "identifier": "com.and0114.cclean",',
    '  "bundle": { "active": true, "createUpdaterArtifacts": true }',
    "}",
  ].join("\n");

  const after = patchTauriConfVersion(before, "0.2.3");
  const parsed = JSON.parse(after);

  assert.equal(parsed.version, "0.2.3");
  assert.equal(parsed.productName, "C-DiskCleaner");
  assert.equal(parsed.identifier, "com.and0114.cclean");
  assert.equal(parsed.bundle.createUpdaterArtifacts, true);
});

test("patchTauriConfVersion：不误伤嵌套对象里的 version 字段", () => {
  const before = [
    "{",
    '  "version": "0.1.0",',
    '  "plugins": {',
    '    "updater": { "version": "9.9.9" }',
    "  }",
    "}",
  ].join("\n");

  const parsed = JSON.parse(patchTauriConfVersion(before, "0.3.0"));
  assert.equal(parsed.version, "0.3.0");
  assert.equal(parsed.plugins.updater.version, "9.9.9");
});

test("patchTauriConfVersion：找不到 version 字段时报错而不是静默通过", () => {
  assert.throws(() => patchTauriConfVersion('{\n  "identifier": "x"\n}', "0.2.0"), /找不到顶层/);
});

test("findUpdaterArtifact：优先挑 NSIS 目录下成对的 .sig", () => {
  const files = [
    "msi/C-DiskCleaner_0.2.0_x64_en-US.msi",
    "msi/C-DiskCleaner_0.2.0_x64_en-US.msi.sig",
    "nsis/C-DiskCleaner_0.2.0_x64-setup.exe",
    "nsis/C-DiskCleaner_0.2.0_x64-setup.exe.sig",
  ];
  const artifact = findUpdaterArtifact(files);
  assert.equal(artifact.installer, "nsis/C-DiskCleaner_0.2.0_x64-setup.exe");
  assert.equal(artifact.sig, "nsis/C-DiskCleaner_0.2.0_x64-setup.exe.sig");
});

test("findUpdaterArtifact：只有 msi 签名时回退到 msi", () => {
  const files = ["msi/app_0.2.0_x64_en-US.msi", "msi/app_0.2.0_x64_en-US.msi.sig"];
  assert.equal(findUpdaterArtifact(files).installer, "msi/app_0.2.0_x64_en-US.msi");
});

test("findUpdaterArtifact：没有签名 / 签名没有对应安装包时返回 null", () => {
  assert.equal(findUpdaterArtifact(["nsis/app_0.2.0_x64-setup.exe"]), null);
  assert.equal(findUpdaterArtifact(["nsis/app_0.2.0_x64-setup.exe.sig"]), null);
  assert.equal(findUpdaterArtifact([]), null);
});

test("verifyArtifacts：版本号与签名都正确时通过", () => {
  const files = [
    "nsis/C-DiskCleaner_0.2.0_x64-setup.exe",
    "nsis/C-DiskCleaner_0.2.0_x64-setup.exe.sig",
    "msi/C-DiskCleaner_0.2.0_x64_en-US.msi",
    "msi/C-DiskCleaner_0.2.0_x64_en-US.msi.sig",
  ];
  const { installers, artifact } = verifyArtifacts("0.2.0", files);
  assert.equal(installers.length, 2);
  assert.match(artifact.sig, /nsis/);
});

test("verifyArtifacts：安装包版本号没跟着变时报错（原 bug-1 现象）", () => {
  const files = ["nsis/C-DiskCleaner_0.1.0_x64-setup.exe", "nsis/C-DiskCleaner_0.1.0_x64-setup.exe.sig"];
  assert.throws(() => verifyArtifacts("0.2.0", files), /安装包版本号不是 0\.2\.0/);
});

test("verifyArtifacts：没有 .sig 时报错（原 bug-2 现象）", () => {
  const files = ["nsis/C-DiskCleaner_0.2.0_x64-setup.exe", "msi/C-DiskCleaner_0.2.0_x64_en-US.msi"];
  assert.throws(() => verifyArtifacts("0.2.0", files), /\.sig/);
});

test("verifyArtifacts：没有任何安装包时报错", () => {
  assert.throws(() => verifyArtifacts("0.2.0", []), /没有任何 \.exe\/\.msi/);
});

test("verifyArtifacts：bundle 里残留旧版本产物不影响本次判定", () => {
  const files = [
    "nsis/C-Disk Cleaner_0.1.0_x64-setup.exe",
    "msi/C-Disk Cleaner_0.1.0_x64_en-US.msi",
    "nsis/C-DiskCleaner_0.2.0_x64-setup.exe",
    "nsis/C-DiskCleaner_0.2.0_x64-setup.exe.sig",
  ];
  const { installers, artifact } = verifyArtifacts("0.2.0", files);
  assert.equal(installers.length, 1);
  assert.match(artifact.sig, /0\.2\.0/);
});

test("verifyArtifacts：本次没签名、只有旧版本签名时不能蒙混过关", () => {
  const files = [
    "nsis/C-DiskCleaner_0.1.0_x64-setup.exe",
    "nsis/C-DiskCleaner_0.1.0_x64-setup.exe.sig",
    "nsis/C-DiskCleaner_0.2.0_x64-setup.exe",
  ];
  assert.throws(() => verifyArtifacts("0.2.0", files), /没有找到 0\.2\.0 的成对 \.sig/);
});

test("buildLatestJson：URL 指向本次版本对应的安装包，签名为原文", () => {
  const raw = buildLatestJson({
    version: "0.2.0",
    installerName: "C-DiskCleaner_0.2.0_x64-setup.exe",
    signature: "  dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZQ==\n",
    repo: "An0114/C-clean",
    pubDate: "2026-10-23T05:00:00Z",
  });

  assert.ok(raw.endsWith("\n"));
  const parsed = JSON.parse(raw);
  assert.equal(parsed.version, "0.2.0");
  assert.equal(parsed.pub_date, "2026-10-23T05:00:00Z");
  assert.equal(
    parsed.platforms["windows-x86_64"].url,
    "https://github.com/An0114/C-clean/releases/download/v0.2.0/C-DiskCleaner_0.2.0_x64-setup.exe"
  );
  assert.equal(parsed.platforms["windows-x86_64"].signature, "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZQ==");
});

// ===== 契约测试：守住这次两个 bug 的修复不被回退 =====
test("契约：tauri.conf.json 必须开启 createUpdaterArtifacts（否则 .sig 不会生成）", () => {
  const conf = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, "app/src-tauri/tauri.conf.json"), "utf8"));
  assert.equal(conf.bundle.createUpdaterArtifacts, true);
  assert.ok(Array.isArray(conf.bundle.targets) && conf.bundle.targets.includes("nsis"));
});

test("契约：updater 公钥与 endpoint 仍在位", () => {
  const conf = JSON.parse(fs.readFileSync(path.join(REPO_ROOT, "app/src-tauri/tauri.conf.json"), "utf8"));
  assert.ok(conf.plugins.updater.pubkey.length > 40);
  assert.ok(conf.plugins.updater.endpoints.some((u) => u.endsWith("latest.json")));
});

test("契约：release.yml 必须注入私钥、校验产物版本号并上传 .sig", () => {
  const yml = fs.readFileSync(path.join(REPO_ROOT, ".github/workflows/release.yml"), "utf8");
  assert.match(yml, /TAURI_SIGNING_PRIVATE_KEY/);
  assert.match(yml, /release-lib\.mjs verify-artifacts/);
  assert.match(yml, /nsis\/\*\.sig/);
});
