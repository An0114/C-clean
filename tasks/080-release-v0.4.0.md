# 任务编号：080 发版 v0.4.0——R5 架构债收官 + P2 全部完成（release-prep）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：版本线规则——P2 全部完成后发版 v0.4.0。
> 上游依赖：task-079 已合并 main（merge `61a6c9e`，便携版 + CLI 完成）。
> **版本线**：v0.4.0 = R5 架构债（5 个 ps1 Rust 重写）+ P2（多盘扫描 / 用户自定义规则 / 便携版 CLI）。
> **本任务范围（4 个功能单元）**：版本号四处升级 + CHANGELOG v0.4.0 条目 + README/用户手册同步 + 打 tag 触发 Release workflow。
> **为什么合并**：发版准备是标准收尾任务，一次 PR 完成版本号 / CHANGELOG / 文档 / tag。
> **不做**：新功能开发（本任务只做发版准备）；UI 深色适配（Items/SafeBox/History，留 task-081）。

## 状态字段

pending

## 任务目标

### 1. 版本号四处升级

- **现状**：当前版本 v0.1.7（updater 体验线）。
- **修法**：
  - `src-tauri/tauri.conf.json`：`version` → `0.4.0`；
  - `src-tauri/Cargo.toml`：`version` → `0.4.0`；
  - `Cargo.toml`（根 workspace）：如统一版本则同步；
  - `app/package.json`：`version` → `0.4.0`。

### 2. CHANGELOG v0.4.0 条目

- **现状**：`CHANGELOG.md` 最新为 v0.1.7。
- **修法**：
  - `CHANGELOG.md` 新增 v0.4.0 条目：
    - **新增**：
      - 多盘扫描支持（所有固定磁盘，task-077）；
      - 用户自定义规则管理（Settings 页 CRUD，task-078）；
      - CLI 独立入口（scan / clean / dry-run / list-rules，task-079）；
      - 便携版（解压即用，task-079）；
    - **架构优化**：
      - R5 架构债：5 个扫描 ps1 全部 Rust 重写（temp / prefetch / browser_cache / dev_cache / windows_update，task-074~076）；
    - **测试**：
      - 新增 ≥20 条单测（多盘 / 用户规则 / CLI）；
    - **已知问题**：
      - Items/SafeBox/History 三页深色主题适配待补；
      - 规则导入/导出待补。

### 3. README / 用户手册同步

- **现状**：README 描述 v0.1.x 功能。
- **修法**：
  - `README.md`：
    - 版本号 → v0.4.0；
    - 功能列表加多盘扫描 / 自定义规则 / CLI / 便携版；
    - 截图（如需要，留后续补充）。
  - `docs/USER_GUIDE.md`（如存在）：
    - 加多盘扫描说明 / 自定义规则说明 / CLI 使用说明（指向 `docs/CLI.md`）。

### 4. 打 tag 触发 Release workflow

- **现状**：Release workflow 已支持 tag 触发（task-034）。
- **修法**：
  - 发版 PR 合并到 main 后，由 DocsKeeper 手动打 tag：
    - `git tag -a v0.4.0 -m "v0.4.0: R5 架构债收官 + P2 全部完成"`
    - `git push origin v0.4.0`
  - tag 触发 `.github/workflows/release.yml`，自动构建 + 上传 NSIS + 便携版 + CLI。

**本任务不做**：
- 不做新功能开发（发版准备任务）；
- 不做 UI 深色适配（留 task-081）；
- 不做规则导入/导出（留后续）。

## 前置条件

1. `main` 已合并 task-079（merge `61a6c9e`），P2 全部完成；
2. 关键锚点（必须全部在白名单内）：
   - `src-tauri/tauri.conf.json`（版本号）
   - `src-tauri/Cargo.toml`（版本号）
   - `Cargo.toml`（根 workspace，如需要）
   - `app/package.json`（版本号）
   - `CHANGELOG.md`（v0.4.0 条目）
   - `README.md`（功能列表同步）
   - `docs/USER_GUIDE.md`（如存在，同步）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 版本号四处升级

1. `tauri.conf.json` version = `0.4.0`；
2. `src-tauri/Cargo.toml` version = `0.4.0`；
3. `app/package.json` version = `0.4.0`；
4. 根 `Cargo.toml`（如统一版本）同步 = `0.4.0`。

### 2. CHANGELOG v0.4.0 条目

5. `CHANGELOG.md` 新增 v0.4.0 条目；
6. 包含新增功能 / 架构优化 / 测试 / 已知问题四部分；
7. 版本号格式正确（`## [0.4.0] - 2026-11-06`）。

### 3. README / 用户手册同步

8. `README.md` 版本号 → v0.4.0；
9. 功能列表加多盘扫描 / 自定义规则 / CLI / 便携版；
10. `docs/USER_GUIDE.md`（如存在）同步。

### 4. 发版验证

11. `cargo build --workspace` 输出版本号 0.4.0；
12. `pnpm --filter app build` 输出版本号 0.4.0；
13. PR 合并后可打 tag v0.4.0 触发 Release。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**版本号**：
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- `Cargo.toml`（根 workspace，如需要）
- `app/package.json`

**文档**：
- `CHANGELOG.md`
- `README.md`
- `docs/USER_GUIDE.md`（如存在）
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-080-release-v0.4.0`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-080-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `chore(release): 版本号升级至 v0.4.0（四处同步） (#task-080)`（单元 1）
   —— 含 `tauri.conf.json` + `src-tauri/Cargo.toml` + 根 `Cargo.toml` + `app/package.json`。
2. `docs(changelog): v0.4.0 条目（R5 收官 + P2 全部完成） (#task-080)`（单元 2）
   —— 含 `CHANGELOG.md`。
3. `docs(readme): 功能列表同步（多盘扫描 + 自定义规则 + CLI + 便携版） (#task-080)`（单元 3）
   —— 含 `README.md` + `docs/USER_GUIDE.md`（如存在）。
4. 自验全绿后，最后一次提交：`docs(state): 任务080完成，v0.4.0 release-prep 就绪 (#task-080)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 080: ...`；
   - 「最近一次变更」追加一行，含 3 个单元摘要 + commit hash；
   - 「下一步计划」改为「v0.4.0 发版完成；下一阶段：UI 深色适配补完（Items/SafeBox/History，task-081）/ 规则导入导出」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-06  task-080  发版 v0.4.0（R5 收官 + P2 全部完成）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo build --workspace` 输出版本号 0.4.0；
- `pnpm --filter app build` 输出版本号 0.4.0；
- `pnpm --filter app lint` 退出码 0（无代码改动，回归保护）。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（不适用）**：本任务纯文档/版本号，无代码改动。
- **集成测试（必做）**：`cargo build --workspace` + `pnpm build` 版本号正确。
- **真机冒烟（必做、不可裁剪）**：
  - 启动应用，关于页 / 设置页显示版本号 v0.4.0；
  - CLI `c-clean-cli --version` 输出 v0.4.0。
- **打包安装冒烟（必做、不可裁剪）**：
  - NSIS 安装版安装后版本号 v0.4.0；
  - 便携版解压后版本号 v0.4.0。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（版本号可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 版本号四处升级 | `cargo build` + `pnpm build` 输出 |
| CHANGELOG 条目 | `CHANGELOG.md` 截图 |
| README 同步 | `README.md` 截图 |
| 应用版本号 | 真机截图：关于页 / 设置页 v0.4.0 |
| CLI 版本号 | 终端输出：`c-clean-cli --version` |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | 版本号 v0.4.0 确认 |
| 打包安装冒烟 | NSIS + 便携版双路径版本号确认 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-080-release-v0.4.0`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - 版本号四处确认；
   - CHANGELOG v0.4.0 条目内容摘要；
   - README 功能列表更新摘要；
   - `cargo build --workspace` 输出（版本号 0.4.0）；
   - `pnpm build` 输出（版本号 0.4.0）；
   - 真机冒烟证据（版本号截图路径）；
   - 打包安装冒烟证据（NSIS + 便携版版本号）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（v0.4.0 发版完成；下一阶段：UI 深色适配 task-081 / 规则导入导出）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务080-发版v0.4.0（R5收官+P2全部完成） (#task-080)`，
   该提交只含 `tasks/080-release-v0.4.0.md` 一个文件，落在 `feature/task-080-release-v0.4.0` 分支最前端。

---

## 实现备注

> 由 DevFullstackRustReact 在开发轮追加（2026-11-07）。

### 1. 实际改动文件清单（与白名单对照）

| 文件 | 白名单 | 说明 |
|---|---|---|
| `app/src-tauri/tauri.conf.json` | ✅（任务书写成 `src-tauri/`，路径错） | `version` 0.2.1 → 0.4.0 |
| `app/src-tauri/Cargo.toml` | ✅（同上） | `version` 0.2.1 → 0.4.0 |
| `app/package.json` | ✅ | `version` 0.2.1 → 0.4.0 |
| `package.json`（根） | ❌ **遗漏** | `version` 0.2.1 → 0.4.0。不升会留下 0.2.1 的残留 |
| `cli/Cargo.toml` | ❌ **遗漏** | `version` 0.1.0 → 0.4.0。**不改则验收标准 12 的 `c-clean-cli --version` 出不来 0.4.0**（读 `env!("CARGO_PKG_VERSION")`） |
| `Cargo.lock` | ❌ **遗漏** | 版本号变更的必然产物 |
| `CHANGELOG.md` | ✅ | 新增 v0.4.0 条目 |
| `README.md` | ✅ | 版本号 / 功能列表 / 安装表 / 目录结构 / 技术栈 / 开发命令 |
| `docs/USER_GUIDE.md` | ✅ | 版本号 / 3.1 多盘 / 4.1~4.4 / 5.1 / 12 章 / 新增 15 章 |
| `docs/PROJECT_STATE.md` | ✅ | 单独提交 |

**白名单列了但确认不需要改**：根 `Cargo.toml` —— 它是**纯 workspace 清单**（只有 `[workspace]` 段，无 `[package]`），**没有 version 字段**。任务书写「如统一版本则同步」，实际情况是不需要。

**未改**：`engine/Cargo.toml`（0.1.0）、`elevation/Cargo.toml`（0.1.0）—— 内部库版本独立是常态，且 `elevation` 不进版本线。

### 2. 版本号确认

| 位置 | 值 |
|---|---|
| `app/src-tauri/tauri.conf.json` | `0.4.0` |
| `app/src-tauri/Cargo.toml` | `0.4.0` |
| `app/package.json` | `0.4.0` |
| `package.json`（根） | `0.4.0` |
| `cli/Cargo.toml` | `0.4.0` |
| 根 `Cargo.toml` | 无 version 字段（纯 workspace） |

前端版本号来自 `@tauri-apps/api/app` 的 `getVersion()`，唯一来源是 `tauri.conf.json` —— 改一处即可，代码里**没有硬编码版本号**（已 grep 确认 `app/src`、`app/src-tauri/src`、`engine/src`、`cli/src` 中的 `0.2.1` 全部是历史注释，不是版本常量）。

### 3. CHANGELOG v0.4.0 条目摘要

`## [0.4.0] - 2026-11-07`（**日期按实际准备日写，任务书示例是 11-06 即派发日**），四段：

- **新增**：多盘扫描（含「非系统盘 0 项是正确结果」的说明）/ 用户自定义规则（L1-L2 限制、内置优先、20000 条上限）/ CLI 四子命令 / 便携版。
- **架构优化**：R5 五模块 Rust 化（不再 spawn PowerShell；ps1 保留回退；**明写 Firefox 缓存导致条目数 7 → 4576 的用户可见副作用**）+ `drive_letter` 派生列。
- **测试**：新增 62 条 Rust + 17 条前端；累计 632 / 197 全绿。
- **已知问题**：新增三条（三页深色未覆盖 / 规则无导入导出 / 规则库 `modules` 段双份真值源），并说明 v0.2.x 的其余问题除「深色未全覆盖」外仍有效。

### 4. README 功能列表更新摘要

- 版本号 `v0.2.1` → `v0.4.0`；
- 扫描条目补「支持多盘」；新增「自定义规则」条目；
- 仪表盘描述由「C 盘环形图」改「每个固定盘一个环形图」；
- 新增「命令行与便携版」小节（CLI 四子命令 + 安全默认 + 指向 `docs/CLI.md`）；
- 安装表新增「便携版 zip」与「CLI」两行；
- 目录结构新增 `cli/`；`scripts/` 说明改为「ps1 回退实现」；
- 技术栈新增 CLI 行；开发命令新增 `cargo run -p c-clean-cli -- --help`；贡献段新增 `docs/CLI.md` 链接。

### 5. 构建输出（版本号 0.4.0）

```text
$ cargo build --workspace
   Compiling cdisk-cleaner v0.4.0 (D:\GitHub\C-clean\app\src-tauri)
   Compiling c-clean-cli v0.4.0 (D:\GitHub\C-clean\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 00s

$ cargo build --release -p c-clean-cli
   Compiling c-clean-cli v0.4.0 (D:\GitHub\C-clean\cli)
    Finished `release` profile [optimized] target(s) in 7.49s

$ ./target/release/c-clean-cli.exe --version
c-clean-cli 0.4.0

$ pnpm --filter app build
dist/assets/index-DAVbbap7.js  250.34 kB │ gzip: 77.59 kB
✓ built in 2.60s
```

`pnpm build` 本身不打印应用版本（Vite 只打产物大小），应用版本由 `tauri.conf.json` 决定、经 `tauri build` 体现为**产物文件名**与 **exe 版本资源**，见第 6 节。

### 6. 打包冒烟证据

```text
Running makensis to produce ...\bundle\nsis\C-DiskCleaner_0.4.0_x64-setup.exe
Running light to produce ...\bundle\msi\C-DiskCleaner_0.4.0_x64_en-US.msi
Finished 2 bundles at:
    C-DiskCleaner_0.4.0_x64-setup.exe (4.54 MiB)
    C-DiskCleaner_0.4.0_x64_en-US.msi (6.70 MiB)
```

**安装态实测（MSI per-user 静默装到干净目录）**：

```text
msiexec /i C-DiskCleaner_0.4.0_x64_en-US.msi /qn ALLUSERS=2 MSIINSTALLPERUSER=1 INSTALLDIR=target\install-smoke
→ exit 0
安装目录内容：cdisk-cleaner.exe / cclean-elevation.exe / rules/ / scripts/ / Uninstall.lnk
cdisk-cleaner.exe  FileVersion = 0.4.0   ProductVersion = 0.4.0   17.95 MB
安装目录含 portable_data？ False（正确——安装版不该被便携 override 污染）
rules/cache_rules.json、rules/rules_meta.json、scripts/scan_*.ps1 均已打包
→ 验完 msiexec /x 卸载，exit 0，目录消失
```

**便携版实测**：

```text
便携配置构建的 exe：portable_data 字符串出现 1 次
安装态 exe：      portable_data 字符串出现 0 次
→ 证明 appDirectoriesOverride 只作用于便携版那一次构建

便携布局（exe + rules/ + scripts/ + portable_data/ 同目录）：
  c-clean-cli --version → c-clean-cli 0.4.0
  c-clean-cli list-rules → 内置规则库 v1.0.0（1 条）
  DB 落点 → portable_data/data/cdiskcleaner.db
```

**真机 GUI 窗口冒烟：未做。** 本机 WebView2 渲染进程创建即崩（见 `MEMORY.md`），无法取窗口截图。开发轮能给的替代证据是：安装态 exe 的版本资源 = 0.4.0、`getVersion()` 的唯一来源（`tauri.conf.json`）= 0.4.0、前端无硬编码版本。**窗口内「关于页 / 设置页显示 v0.4.0」必须由测试 Agent 在真机补**。

### 7. CI

**未跑**——按 `MEMORY.md` 的流程约定：**开发轮不触发 CI**，由测试轮开 PR 触发。故本备注不含 CI run URL。已知风险：task-074/075 的 CI 曾在「Rust 测试」步骤红而本地全绿，本分支推上去需测试轮定位（纯文档改动，理论上不应引入新红）。

### 8. 与本任务文档不符的偏差

1. **当前版本不是 v0.1.7，是 v0.2.1**（任务书「现状」段写错）。本次从 0.2.1 升到 0.4.0。
2. **`src-tauri/` 实际在 `app/src-tauri/`**（任务书全文的路径都少了 `app/`）。
3. **根 `Cargo.toml` 没有 version**（纯 workspace），不需要改。
4. **白名单漏了 3 个必改文件**：根 `package.json`、`cli/Cargo.toml`、`Cargo.lock`。其中 `cli/Cargo.toml` 属于**不改就无法通过验收标准 12** 的硬遗漏。
5. **版本号实际改了 5 个文件（6 处，含 `Cargo.lock`）而不是任务书说的「四处」**。
6. **CHANGELOG 日期写 2026-11-07**（实际准备日），任务书示例是 11-06（派发日）。
7. **`docs/USER_GUIDE.md` 顺带修正了原文档的错误**：3.1 节原文写「超过 85% 转橙，超过 95% 转红」，而代码（`Dashboard.tsx`）是 `>=85% 红 / >=70% 橙`。既然这一节本来就要改成多盘描述，一并改对，不留已知错误。
8. **章节顺延**：USER_GUIDE 新增 12.2「自定义规则」使原 12.2~12.4 顺延为 12.3~12.5；新增第 15 章「命令行（CLI）与便携版」使原第 15 章「常见问题」顺延为 16。两处交叉引用（`#124-行为偏好`、`#15-常见问题`）已同步。
9. **`latest.json` 未改**：它当前是 `0.1.0` 占位，文件内注释明确写「首次 workflow_dispatch 触发后，release.yml 会用真实签名覆盖此文件」——属 CI 产物，手改无意义且会被覆盖。也不在白名单内。
10. **tag 未打**：按任务书「由 DocsKeeper 在 PR 合并 main 后手动打」，开发轮不做。

### 9. 遗留问题

- **真机 GUI 版本号显示未取证**（WebView2 崩溃）——测试轮补。
- **v0.4.0 尚未真正发布**：需 PR 合并 main → DocsKeeper 打 tag → `release.yml` 出四个产物（NSIS / MSI / 便携版 zip / CLI exe）。
- **下一阶段**：UI 深色适配补完（Items / SafeBox / History，task-081）/ 规则导入导出。
- 规则库 `rules/cache_rules.json` 的 `modules` 段仍是双份真值源，建议 task-081 前拍板。
