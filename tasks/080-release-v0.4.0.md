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
