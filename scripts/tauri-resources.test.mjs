// bugfix/task-050 的契约测试：钉住 bundle.resources 的映射形态与落位目标。
//
// 背景：数组式 resources 会让 Tauri 按「保留原目录结构」把 `../../rules/x.json`
// 落到 `<安装目录>\_up_\_up_\rules\x.json`，而 AppPaths 只认 `<resource_dir>/rules`
// → 全新安装用户首次启动找不到规则与脚本（发版阻塞级缺陷）。
//
// 跑法：node --test scripts/tauri-resources.test.mjs（零额外依赖）
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const TAURI_DIR = path.join(REPO_ROOT, "app", "src-tauri");
const CONF_PATH = path.join(TAURI_DIR, "tauri.conf.json");

// 期望的 7 条映射：源（相对 app/src-tauri/）→ 安装包内目标路径。
// 目标前缀只允许 rules/ 与 scripts/，与 AppPaths 的 resource_dir 定位严格对齐。
const EXPECTED = {
  "../../rules/cache_rules.json": "rules/cache_rules.json",
  "../../rules/rules_meta.json": "rules/rules_meta.json",
  "../../scripts/scan_temp.ps1": "scripts/scan_temp.ps1",
  "../../scripts/scan_windows_update.ps1": "scripts/scan_windows_update.ps1",
  "../../scripts/scan_prefetch.ps1": "scripts/scan_prefetch.ps1",
  "../../scripts/scan_browser_cache.ps1": "scripts/scan_browser_cache.ps1",
  "../../scripts/scan_dev_cache.ps1": "scripts/scan_dev_cache.ps1",
  // task-062：提权 exe 随安装包部署到安装目录根（与主 exe 同级）。
  // 源文件由 scripts/build-elevation.mjs 在 beforeBuildCommand 阶段产出。
  "binaries/cclean-elevation.exe": "cclean-elevation.exe",
};

// task-062：提权 exe 落在安装目录**根**（与主 exe 同级），所以多一个空前缀。
// 它不能进 rules/ 或 scripts/——那两个目录是 AppPaths 给规则/脚本用的。
const ALLOWED_PREFIXES = ["rules/", "scripts/", ""];

function readConf() {
  return JSON.parse(fs.readFileSync(CONF_PATH, "utf8"));
}

test("bundle.resources 必须是映射对象（数组式会让资源落到 _up_ 目录）", () => {
  const { resources } = readConf().bundle;
  assert.ok(
    resources && typeof resources === "object" && !Array.isArray(resources),
    "bundle.resources 必须是 {源: 目标} 对象；数组式会保留原目录结构并生成 _up_ 前缀"
  );
});

test("映射条数恰好 8 条，且源 → 目标与契约完全一致", () => {
  const { resources } = readConf().bundle;
  const actual = Object.entries(resources).sort(([a], [b]) => a.localeCompare(b));
  const expected = Object.entries(EXPECTED).sort(([a], [b]) => a.localeCompare(b));
  assert.equal(actual.length, 8, `期望 8 条资源映射，实际 ${actual.length} 条`);
  assert.deepEqual(
    actual,
    expected,
    "资源映射与契约不一致：规则库 2 个 json + 5 个 scan_*.ps1 + 1 个提权 exe，目标路径必须逐字对齐"
  );
});

test("每条目标路径必须以 rules/ 或 scripts/ 开头，或是根目录的提权 exe", () => {
  const { resources } = readConf().bundle;
  for (const [src, dest] of Object.entries(resources)) {
    assert.equal(typeof dest, "string", `${src} 的目标路径必须是字符串`);
    assert.ok(
      ALLOWED_PREFIXES.some((p) => dest.startsWith(p)),
      `${src} → ${dest}：目标必须落在 rules/ 或 scripts/ 下（或为根目录的 cclean-elevation.exe）`
    );
  }
});

test("目标路径不得包含 .. 或 _up_（否则又回到缺陷态布局）", () => {
  const { resources } = readConf().bundle;
  for (const [src, dest] of Object.entries(resources)) {
    assert.ok(!dest.includes(".."), `${src} → ${dest}：目标不得含 ..`);
    assert.ok(!dest.includes("_up_"), `${src} → ${dest}：目标不得含 _up_`);
  }
});

test("不得用通配符整目录搬运（会把 PROTOCOL.md / release-lib.mjs 等构建期资产塞进安装包）", () => {
  const { resources } = readConf().bundle;
  for (const src of Object.keys(resources)) {
    assert.ok(!src.includes("*"), `${src}：资源源路径不得使用通配符`);
  }
});

test("每条源路径在仓库磁盘上真实存在（相对 app/src-tauri/ 解析）", () => {
  const { resources } = readConf().bundle;
  for (const src of Object.keys(resources)) {
    const abs = path.resolve(TAURI_DIR, src);
    assert.ok(fs.existsSync(abs), `资源源文件不存在：${abs}（打包会直接失败）`);
  }
});
