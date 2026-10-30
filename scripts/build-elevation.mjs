// task-062：把提权 exe 编译好并复制到 Tauri 的 resources 目录。
//
// 为什么需要这个脚本：
// `elevation` 是 workspace 里的独立 crate，但 `tauri build` **不会**自动把它编进
// `bundle.resources`——resources 只认「已存在的文件」。所以要在打包前显式：
//   1) `cargo build --release -p elevation`（产物落在 workspace 根的 target/release/）
//   2) 复制到 `app/src-tauri/binaries/cclean-elevation.exe`
//   3) tauri.conf.json 的 resources 再把 binaries/ 下的文件映射到安装目录根
//
// 为什么放在 `beforeBuildCommand` 里而不是独立步骤：
// `tauri build` 的执行顺序是「beforeBuildCommand → cargo build → 打包」，
// 是**串行**的，所以这里跑 cargo 不会和 tauri 的 cargo 抢 target 目录锁。
//
// 跑法：node scripts/build-elevation.mjs [--debug]
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const REPO_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const DEBUG = process.argv.includes("--debug");
const PROFILE = DEBUG ? "debug" : "release";

const SRC = path.join(
  REPO_ROOT,
  "target",
  PROFILE,
  process.platform === "win32" ? "cclean-elevation.exe" : "cclean-elevation"
);
const DEST_DIR = path.join(REPO_ROOT, "app", "src-tauri", "binaries");
const DEST = path.join(DEST_DIR, path.basename(SRC));

function log(msg) {
  process.stdout.write(`[build-elevation] ${msg}\n`);
}

// 1. 编译（继承 stdio，编译错误能直接看到）
log(`编译 elevation（${PROFILE}）…`);
execFileSync(
  "cargo",
  ["build", "-p", "elevation", ...(DEBUG ? [] : ["--release"])],
  { cwd: REPO_ROOT, stdio: "inherit" }
);

// 2. 校验产物真的存在（cargo 返回 0 但没有产物的情况不该静默通过）
if (!fs.existsSync(SRC)) {
  console.error(`[build-elevation] 编译后找不到产物: ${SRC}`);
  process.exit(1);
}

// 3. 复制
fs.mkdirSync(DEST_DIR, { recursive: true });
fs.copyFileSync(SRC, DEST);
const size = fs.statSync(DEST).size;
log(`已复制 → ${path.relative(REPO_ROOT, DEST)}（${size} 字节）`);

// 4. 兜底自检：产物必须能被执行（这里是「跑一次 --help 等价物」——
//    不带参数会走 EXIT_BAD_USAGE，正好验证白名单/用法检查链路是通的）。
if (process.platform === "win32") {
  try {
    execFileSync(DEST, [], { stdio: "pipe" });
    console.error("[build-elevation] 无参数调用应返回非 0（用法错误），但它成功了");
    process.exit(1);
  } catch (e) {
    // 预期：退出码 2（EXIT_BAD_USAGE）
    if (e.status !== 2) {
      console.error(
        `[build-elevation] 自检失败：无参数调用应退出 2，实际 ${e.status}`
      );
      process.exit(1);
    }
    log("自检通过：无参数调用按约定返回退出码 2");
  }
}
