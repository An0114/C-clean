# 任务编号：079 P2 收官——便携版 + CLI 入口

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：P2 未做清单最后一项（便携版 + CLI）。
> 上游依赖：task-078 已合并 main（merge `af86e7e`，用户自定义规则管理完成）。
> **版本线**：v0.4.x P2 收官——便携版 + CLI。
> **本任务范围（4 个功能单元）**：CLI 独立 binary（调用 engine 库）+ CLI 子命令（scan/clean/dry-run/list-rules）+ 便携版打包配置 + CLI 文档。
> **为什么合并**：CLI 入口与便携版打包是强耦合（CLI binary 本身就是便携版核心），一次 PR 完成 P2 收官。
> **不做**：跨盘恢复 fallback（维持现状）；UI 深色适配（留后续小任务）。

## 状态字段

pending

## 任务目标

### 1. CLI 独立 binary

- **现状**：只有 Tauri GUI（`app/src-tauri/`），无命令行入口。
- **修法**：
  - `cli/Cargo.toml`（新建）：
    - 独立 binary `c-clean-cli`；
    - 依赖 `engine`（workspace 成员）；
    - 不依赖 `tauri` 或 `tauri-plugin-*`。
  - `cli/src/main.rs`（新建）：
    - 解析命令行参数（clap 4）；
    - 调用 engine 库执行扫描/清理；
    - 输出人类可读表格 + JSON 两种格式（`--format json`）。

### 2. CLI 子命令

- **修法**：
  - `cli/src/main.rs`：
    - `scan`：执行扫描，输出可清理项列表；
    - `clean`：执行清理（默认 dry-run，加 `--yes` 真实删除）；
    - `dry-run`：预览可清理项（同 scan，但更详细）；
    - `list-rules`：列出内置 + 用户规则；
    - `--format json`：机器可读输出（供脚本调用）。
  - 所有子命令复用 engine 的扫描/删除逻辑，不重复实现。

### 3. 便携版打包配置

- **现状**：NSIS 安装版（task-033 已做）。
- **修法**：
  - `src-tauri/tauri.conf.json`：
    - 加 `bundle.windows.portable` 配置（Tauri 2 原生支持）；
    - 便携版 = 单 exe，数据存 exe 同目录（`portable_data/`）；
  - `.github/workflows/release.yml`：
    - CI 同时产出 NSIS 安装版 + 便携版 zip；
    - 两个 artifact 都上传到 GitHub Release。

### 4. CLI 文档

- **修法**：
  - `docs/CLI.md`（新建）：
    - CLI 安装方式（从 Release 下载 `c-clean-cli.exe`）；
    - 子命令使用示例（scan / clean / dry-run / list-rules）；
    - JSON 输出格式说明；
    - 便携版使用方式（解压即用，数据存同目录）。

**本任务不做**：
- 不做跨盘恢复 fallback（维持现状）；
- 不做 UI 深色适配（Items/SafeBox/History，留后续小任务）；
- 不做规则导入/导出（留后续）。

## 前置条件

1. `main` 已合并 task-078（merge `af86e7e`），用户自定义规则完成；
2. 关键锚点（必须全部在白名单内）：
   - CLI 新建：`cli/Cargo.toml`
   - CLI 新建：`cli/src/main.rs`
   - 根 Cargo：`Cargo.toml`（workspace members 加 cli）
   - 打包：`src-tauri/tauri.conf.json`（便携版配置）
   - CI：`.github/workflows/release.yml`（便携版 artifact）
   - 文档：`docs/CLI.md`（新建）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. CLI 独立 binary

1. `c-clean-cli` 独立编译（不依赖 Tauri）；
2. 调用 engine 库复用扫描/删除逻辑；
3. 单测：CLI 编译通过。

### 2. CLI 子命令

4. `scan` 子命令输出可清理项列表；
5. `clean` 子命令默认 dry-run，`--yes` 真实删除；
6. `dry-run` 子命令预览可清理项；
7. `list-rules` 子命令列出内置 + 用户规则；
8. `--format json` 机器可读输出；
9. 单测：子命令执行正确。

### 3. 便携版打包

10. `tauri.conf.json` 加便携版配置；
11. CI 同时产出 NSIS 安装版 + 便携版 zip；
12. 便携版数据存 exe 同目录；
13. 单测：便携版 exe 可启动。

### 4. CLI 文档

14. `docs/CLI.md` 包含安装方式 + 子命令示例 + JSON 格式说明；
15. 便携版使用方式说明。

### 后端单测

16. `cargo test --workspace` 全绿（现有 + 本任务新增 ≥3 条全绿）。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**CLI 新建**：
- `cli/Cargo.toml`（新建）
- `cli/src/main.rs`（新建）

**根 Cargo**：
- `Cargo.toml`（workspace members 加 cli）

**打包**：
- `src-tauri/tauri.conf.json`（便携版配置）

**CI**：
- `.github/workflows/release.yml`（便携版 artifact）

**文档**：
- `docs/CLI.md`（新建）
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-079-p2-portable-cli`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-079-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `feat(cli): CLI 独立 binary（clap 4 + 复用 engine 库） (#task-079)`（单元 1）
   —— 含 `cli/Cargo.toml` + `cli/src/main.rs` + 根 `Cargo.toml`。
2. `feat(cli): CLI 子命令（scan / clean / dry-run / list-rules + --format json） (#task-079)`（单元 2）
   —— 含 `cli/src/main.rs`。
3. `feat(bundle): 便携版打包配置（tauri.conf portable + CI 双 artifact） (#task-079)`（单元 3）
   —— 含 `src-tauri/tauri.conf.json` + `.github/workflows/release.yml`。
4. `docs(cli): CLI 使用文档（安装 + 子命令示例 + JSON 格式） (#task-079)`（单元 4）
   —— 含 `docs/CLI.md`。
5. 自验全绿后，最后一次提交：`docs(state): 任务079完成，P2 收官——便携版 + CLI 入口就绪 (#task-079)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 079: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「P2 全部完成；下一阶段：发版 v0.4.0（release-prep）/ UI 深色适配补完 / 规则导入导出」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-06  task-079  P2 收官（便携版 + CLI）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0（无前端改动，回归保护）；
- `cargo run -p c-clean-cli -- --help` 输出帮助信息。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：CLI 编译、子命令执行、JSON 输出格式。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **真机冒烟（必做、不可裁剪）**：
  - `c-clean-cli scan` 输出可清理项列表；
  - `c-clean-cli scan --format json` 输出 JSON；
  - `c-clean-cli list-rules` 列出内置 + 用户规则；
  - 便携版 exe 解压后可启动，数据存同目录；
  - CDP 抓 console 0 红错（GUI 回归）。
- **打包安装冒烟（必做、不可裁剪）**：
  - NSIS 安装版安装正常；
  - 便携版 zip 解压即用，无需安装；
  - 两个 artifact 都在 GitHub Release 可见。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（便携版可启动 + GUI 回归），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| CLI 编译 | `cargo build --workspace` 输出 |
| CLI 子命令 | 终端输出：scan / clean / list-rules 示例 |
| JSON 输出 | 终端输出：`--format json` 正确 JSON |
| 便携版 exe | 真机截图：解压后启动 |
| 双 artifact | GitHub Release 页面截图（NSIS + portable.zip） |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS + 便携版双路径复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-079-p2-portable-cli`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - CLI binary 名称确认（`c-clean-cli`）；
   - 子命令参数设计确认；
   - 便携版数据路径确认（exe 同目录 `portable_data/`）；
   - CI 双 artifact 配置确认；
   - `cargo test --workspace` 通过输出（含新增 ≥3 条用例）；
   - `c-clean-cli --help` 输出；
   - 真机冒烟证据（4 条终端输出路径）；
   - 打包安装冒烟证据（NSIS + 便携版双路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（P2 全部完成；下一阶段：发版 v0.4.0 / UI 深色适配补完 / 规则导入导出）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务079-P2收官 便携版+CLI入口 (#task-079)`，
   该提交只含 `tasks/079-p2-portable-cli.md` 一个文件，落在 `feature/task-079-p2-portable-cli` 分支最前端。
