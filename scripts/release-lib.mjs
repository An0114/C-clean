// 发版流水线的纯逻辑（bugfix/task-035-release-signing）。
//
// 为什么单独抽成模块：版本号注入和 latest.json 生成以前是 release.yml 里内联的
// PowerShell，逻辑没法单测，两个 bug（版本号不生效 / 不生成 .sig）都是「静默失败」——
// CI 绿着，产物却是错的。抽出来后 release.yml 只做编排，逻辑由 node --test 覆盖
// （见 release-lib.test.mjs）。
//
// 只放纯函数 + 一个 CLI 入口，不引第三方依赖，Node 22 内置能力即可跑。
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

/** semver 校验：0.2.0 / 1.0.0-beta.1 / 1.0.0+build 都算合法 */
const SEMVER = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;

/**
 * 解析本次发版版本号。
 * 优先用手动触发的 version 输入，没有则回退到触发 ref（tag 名 v0.1.7）。
 * 两者都不合法就抛错——以前分支名会被当成版本号写进产物，是 bug-1 的一半成因。
 */
export function normalizeVersion(inputVersion, refName) {
  const fromInput = String(inputVersion ?? "").trim();
  const fromRef = String(refName ?? "").trim();
  const candidate = (fromInput || fromRef).replace(/^[vV]/, "");
  if (!candidate) {
    throw new Error("版本号为空：workflow_dispatch 必须填 version，或由 v* tag 触发");
  }
  if (!SEMVER.test(candidate)) {
    throw new Error(`版本号不合法: "${candidate}"（要求 semver，如 0.2.0；不能是分支名）`);
  }
  return candidate;
}

/**
 * 把版本号写进 tauri.conf.json 的顶层 version 字段，保留原文件格式。
 * 只动顶层：嵌套对象里也可能有 "version"（插件配置），不能被误替换。
 * 写回后立刻 JSON.parse 复验，避免「改了但没生效」这种静默失败。
 */
export function patchTauriConfVersion(text, version) {
  const re = /^([ \t]*)"version"[ \t]*:[ \t]*"[^"]*"/m;
  if (!re.test(text)) {
    throw new Error('tauri.conf.json 里找不到顶层 "version" 字段');
  }
  const patched = text.replace(re, `$1"version": "${version}"`);
  const parsed = JSON.parse(patched);
  if (parsed.version !== version) {
    throw new Error(`版本注入失败：写回后读到的版本是 ${parsed.version}，期望 ${version}`);
  }
  return patched;
}

/** 统一成正斜杠，方便跨平台比对 */
function normalizePath(p) {
  return String(p).replace(/\\/g, "/");
}

/**
 * 从文件清单里挑出 updater 需要的「安装包 + 签名」成对产物。
 * updater 走 NSIS，所以优先 nsis 目录；没有成对签名就返回 null（调用方据此报错）。
 */
export function findUpdaterArtifact(files) {
  const list = files.map(normalizePath);
  const set = new Set(list);
  const pairs = list
    .filter((f) => f.toLowerCase().endsWith(".sig"))
    .map((sig) => ({ sig, installer: sig.slice(0, -".sig".length) }))
    .filter((pair) => set.has(pair.installer));
  if (pairs.length === 0) return null;
  return pairs.find((pair) => /(^|\/)nsis\//i.test(pair.sig)) ?? pairs[0];
}

/**
 * 打包产物校验：安装包文件名里必须带本次版本号，且必须有成对 .sig。
 * 这是把「版本号不生效」「签名没执行」从静默失败变成 CI 红灯的关键一步。
 *
 * 注意：bundle 目录里可能残留上一次构建的旧版本产物（本地尤其常见），
 * 所以先按 `_<version>_` 过滤出本次产物再判定——否则旧文件既会误报版本不对，
 * 也会让「本次没签名但上次签过」蒙混过关。
 */
export function verifyArtifacts(version, files) {
  const list = files.map(normalizePath);
  const installers = list.filter((f) => /\.(exe|msi)$/i.test(f));
  if (installers.length === 0) {
    throw new Error("bundle 目录里没有任何 .exe/.msi 安装包，构建可能没跑完");
  }
  const token = `_${version}_`;
  const current = list.filter((f) => f.includes(token));
  const matchedInstallers = installers.filter((f) => f.includes(token));
  if (matchedInstallers.length === 0) {
    throw new Error(
      `安装包版本号不是 ${version}：实际产物 ${installers.map((f) => path.posix.basename(f)).join(", ")}` +
        "（版本号注入没生效）"
    );
  }
  const artifact = findUpdaterArtifact(current);
  if (!artifact) {
    throw new Error(
      `没有找到 ${version} 的成对 .sig 签名文件，签名未执行` +
        "（检查 bundle.createUpdaterArtifacts=true 与 TAURI_SIGNING_PRIVATE_KEY）"
    );
  }
  return { installers: matchedInstallers, artifact };
}

/**
 * 规范化签名私钥，并在写盘前就校验格式（回归 2026-10-23 run 39940302050）。
 *
 * Tauri 是把「整个私钥文件内容」当成一整段 base64 去解的，不会替你 strip 空白。
 * 所以文件里多一个字节都解不开：以前 release.yml 主动补了一个 "\n"，GitHub Secrets
 * 里粘贴原文也常带尾部换行，bundler 直接报
 *   failed to decode base64 secret key: Invalid symbol 10, offset 348
 * 而且它是在构建跑完 8 分钟、打完 msi 之后才报的。这里统一剔掉所有空白字符，
 * 并把「空 / 截断 / 非 base64 / 不是 minisign 私钥」全部提前到写盘时失败。
 */
export function normalizeSigningKey(raw) {
  if (typeof raw !== "string" || raw.trim() === "") {
    throw new Error("签名私钥为空：secret TAURI_SIGNING_PRIVATE_KEY 没配置或没传进来");
  }
  const compact = raw.replace(/\s+/g, "");
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(compact)) {
    throw new Error(
      "签名私钥含 base64 之外的字符，请确认 secret 内容就是 tauri signer generate 产出的 .key 文件原文"
    );
  }
  if (compact.length % 4 !== 0) {
    throw new Error(`签名私钥 base64 长度不是 4 的倍数（${compact.length} 字符），内容可能被截断`);
  }
  const decoded = Buffer.from(compact, "base64");
  if (!decoded.toString("utf8").startsWith("untrusted comment:")) {
    throw new Error("签名私钥解码后不是 minisign/rsign 私钥（缺少 untrusted comment 头）");
  }
  return { key: compact, decodedBytes: decoded.length };
}

/** 生成 updater 索引文件 latest.json（内容就是 Tauri 约定的 platforms 结构） */
export function buildLatestJson({ version, installerName, signature, repo, pubDate, notes }) {
  const json = {
    version,
    notes: notes ?? `v${version} 发布`,
    pub_date: pubDate,
    platforms: {
      "windows-x86_64": {
        signature: String(signature ?? "").trim(),
        url: `https://github.com/${repo}/releases/download/v${version}/${installerName}`,
      },
    },
  };
  return `${JSON.stringify(json, null, 2)}\n`;
}

/** 递归列出目录下所有文件（返回相对 root 的路径，正斜杠） */
export function listFilesRecursive(root) {
  const out = [];
  const walk = (dir) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) walk(full);
      else if (entry.isFile()) out.push(normalizePath(path.relative(root, full)));
    }
  };
  if (fs.existsSync(root)) walk(root);
  return out;
}

/** 读 tauri.conf.json 当前的版本号（CI 不注入版本号时用它做校验基准） */
export function readConfVersion(confPath = "app/src-tauri/tauri.conf.json") {
  return JSON.parse(fs.readFileSync(confPath, "utf8")).version;
}

// ===== CLI 入口（release.yml / ci.yml 调这里）=====
function readFlag(argv, name) {
  const i = argv.indexOf(`--${name}`);
  return i >= 0 ? argv[i + 1] : undefined;
}

function main(argv) {
  const [command] = argv;
  const repo = readFlag(argv, "repo") ?? "An0114/C-clean";

  switch (command) {
    case "resolve-version": {
      const version = normalizeVersion(readFlag(argv, "input"), readFlag(argv, "ref"));
      process.stdout.write(version);
      return;
    }
    case "patch-conf": {
      const version = readFlag(argv, "version");
      const confPath = readFlag(argv, "conf") ?? "app/src-tauri/tauri.conf.json";
      const before = fs.readFileSync(confPath, "utf8");
      const after = patchTauriConfVersion(before, version);
      fs.writeFileSync(confPath, after, "utf8");
      console.log(`${confPath}: ${JSON.parse(before).version} -> ${version}`);
      return;
    }
    case "verify-artifacts": {
      const version = readFlag(argv, "version") ?? readConfVersion(readFlag(argv, "conf"));
      const bundleDir = readFlag(argv, "bundle") ?? "target/release/bundle";
      const { installers, artifact } = verifyArtifacts(version, listFilesRecursive(bundleDir));
      console.log(`安装包 ${installers.length} 个，版本号均为 ${version}`);
      console.log(`签名配对：${artifact.installer} + ${artifact.sig}`);
      return;
    }
    case "write-key": {
      // 把 secret 里的私钥规范化后写盘：不加换行、不加 BOM、不加任何多余字节。
      const envName = readFlag(argv, "from-env") ?? "TAURI_SIGNING_PRIVATE_KEY";
      const outPath = readFlag(argv, "out");
      if (!outPath) throw new Error("write-key 需要 --out <私钥文件路径>");
      const { key, decodedBytes } = normalizeSigningKey(process.env[envName]);
      fs.writeFileSync(outPath, key, { encoding: "utf8" });
      const written = fs.statSync(outPath).size;
      if (written !== key.length) {
        throw new Error(`私钥写盘后字节数不对：${written} != ${key.length}（可能被追加了换行或 BOM）`);
      }
      console.log(
        `私钥已写入 ${outPath}：${written} 字符 / 解码后 ${decodedBytes} 字节，无尾部换行、无 BOM`
      );
      return;
    }
    case "generate-latest": {
      const version = readFlag(argv, "version") ?? readConfVersion(readFlag(argv, "conf"));
      const bundleDir = readFlag(argv, "bundle") ?? "target/release/bundle";
      const outPath = readFlag(argv, "out") ?? "latest.json";
      const files = listFilesRecursive(bundleDir);
      const artifact = findUpdaterArtifact(files);
      if (!artifact) {
        throw new Error("没有找到成对的 .sig 签名文件，无法生成 latest.json");
      }
      const pubDate = new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
      const content = buildLatestJson({
        version,
        installerName: path.posix.basename(artifact.installer),
        signature: fs.readFileSync(path.join(bundleDir, artifact.sig), "utf8"),
        repo,
        pubDate,
      });
      fs.writeFileSync(outPath, content, "utf8");
      console.log(`latest.json 已生成：${outPath}`);
      console.log(content);
      return;
    }
    default:
      throw new Error(`未知子命令: ${command ?? "(空)"}`);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(`::error::${error.message}`);
    process.exit(1);
  }
}
