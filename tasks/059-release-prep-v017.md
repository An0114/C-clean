# 任务编号：059 v0.1.7 发版准备 + README/用户手册——版本号升级 + CHANGELOG 定稿 + README 重写 + 用户手册

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：PROJECT_STATE「下一步计划 5. v0.1.7 发版（待用户手动 workflow_dispatch）」。
> 上游依赖：task-058 已合并 main（merge `9826c15`，M4 收尾完成）。
> **本任务范围（4 个功能单元，厚任务）**：版本号 v0.1.6→v0.1.7 + CHANGELOG 定稿 + README 重写 + 用户手册。
> **为什么把这些合并**：都是发版前的最后一公里——版本号/CHANGELOG 是发版必备，README/用户手册是开源仓库的门面（项目已转 public），一次 PR 清掉。
> **不做**：workflow_dispatch 发版（用户手动）、代码签名（评估不做）、品牌图标（决策不做）、差量更新（决策不做）、G6 修复（豁免）。

## 状态字段

pending

## 任务目标

### 1. 版本号升级 v0.1.6 → v0.1.7

- **现状**：`package.json` / `src-tauri/tauri.conf.json` / `src-tauri/Cargo.toml` 三处版本号仍是 `0.1.6`；
- **修法**：三处统一升到 `0.1.7`；
- **校验**：`pnpm --filter app build` 产物文件名含 `0.1.7`。

### 2. CHANGELOG v0.1.7 定稿

- **现状**：task-058 已预写 v0.1.7 草稿条目（含占位日期）；
- **修法**：
  - 把草稿日期填为实际发版日（DevAgent 写任务时用 2026-10-29，发版若延后由用户手动改）；
  - 复核 v0.1.7 条目覆盖：RmRestart 自动重启（默认关闭 + 设置开关）、prefs 用户偏好、restored_at 审计列、safe_box 原子性事务、跨盘 rename 测试、RmRestart 白名单、G6 豁免口径；
  - 不写「Unreleased」段（直接定稿 v0.1.7）。

### 3. README.md 重写

- **现状**：仓库根 README.md 大概率是 Tauri 模板默认内容（「Tauri + React + TypeScript」脚手架说明）；
- **修法**：重写为开源项目 README，覆盖：
  - 项目名 + 一句话简介（「Windows C 盘清理工具，Tauri 2 + Rust + React」）；
  - 功能列表（扫描 / 清理 / 保险箱 / 审计日志 / updater / RmRestart）；
  - 截图位（留 `docs/screenshots/` 占位，不强制塞图）；
  - 安装指引（从 GitHub Releases 下 NSIS/MSI）；
  - 系统要求（Windows 10/11 64-bit）；
  - 开发指引（`pnpm install` + `pnpm tauri dev`）；
  - 技术栈（Tauri 2 / Rust / SQLite / React / Vite）；
  - License（MIT，需确认 LICENSE 文件存在）。

### 4. 用户手册 `docs/USER_GUIDE.md`

- **现状**：仓库无面向终端用户的手册（只有 PRD/TRD/ARCHITECTURE 等开发文档）；
- **修法**：写一份用户手册，覆盖：
  - 快速开始（安装 → 首次扫描 → 清理）；
  - 扫描页（进度条 / 脚本列表 / 计数）；
  - 清理页（分页 / 勾选 / 占用关闭 / RmRestart 开关行为）；
  - 保险箱页（时间筛选 / session 筛选 / 来源列 / 恢复 / purge）；
  - 历史页（审计日志 / 统计卡片）；
  - 设置页（检查更新 / 规则库 / 行为偏好：RmRestart 开关 + 白名单）；
  - 更新机制（updater banner / 下载进度 / 失败重试）；
  - 常见问题（SmartScreen 警告 / 被杀软误报 / 清理后文件去哪了）。

**本任务不做**：
- 不发版（用户手动 workflow_dispatch）；
- 不做代码签名（评估不做）；
- 不做品牌图标（决策不做）；
- 不做差量更新（决策不做）；
- 不做 G6 修复（豁免）。

## 前置条件

1. `main` 已合并 task-058（merge `9826c15`）；
2. 关键锚点：
   - `package.json` / `src-tauri/tauri.conf.json` / `src-tauri/Cargo.toml`（版本号三处）
   - `CHANGELOG.md`（v0.1.7 草稿已由 task-058 预写）
   - `README.md`（仓库根，待重写）
   - `docs/USER_GUIDE.md`（新建）
   - `LICENSE`（确认存在，不存在则补 MIT）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 版本号升级

1. `package.json` version = `0.1.7`；
2. `src-tauri/tauri.conf.json` version = `0.1.7`；
3. `src-tauri/Cargo.toml` version = `0.1.7`；
4. `pnpm --filter app build` 产物文件名含 `0.1.7`。

### 2. CHANGELOG 定稿

5. `CHANGELOG.md` v0.1.7 条目日期填为 `2026-10-29`（或实际发版日）；
6. 覆盖 RmRestart / prefs / restored_at / safe_box 原子性 / 跨盘测试 / RmRestart 白名单 / G6 豁免；
7. 不写 Unreleased 段。

### 3. README 重写

8. `README.md` 含项目名 + 一句话简介；
9. 含功能列表（≥6 项）；
10. 含安装指引（GitHub Releases 链接）；
11. 含开发指引（pnpm install + pnpm tauri dev）；
12. 含技术栈列表；
13. LICENSE 文件存在且为 MIT（若不存在则新建）。

### 4. 用户手册

14. `docs/USER_GUIDE.md` 存在；
15. 覆盖快速开始 / 扫描页 / 清理页 / 保险箱页 / 历史页 / 设置页 / 更新机制 / FAQ 共 8 节；
16. 语言为中文（目标用户是中文用户）。

### 自验

17. `cargo check --workspace --all-targets` 退出码 0（本任务不改 Rust 逻辑，预期不回归）；
18. `pnpm --filter app lint / test / build` 全绿。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `package.json`（version）
- `src-tauri/tauri.conf.json`（version）
- `src-tauri/Cargo.toml`（version）
- `CHANGELOG.md`（v0.1.7 定稿）
- `README.md`（重写）
- `docs/USER_GUIDE.md`（新建）
- `LICENSE`（如不存在则新建 MIT）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-059-release-prep-v017`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-059-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `chore(release): 版本号升级 v0.1.6 → v0.1.7（package.json / tauri.conf.json / Cargo.toml） (#task-059)`（单元 1）
2. `docs(changelog): v0.1.7 条目定稿（补日期 + 复核覆盖） (#task-059)`（单元 2）
3. `docs(readme): 重写 README——项目简介/功能列表/安装指引/开发指引/技术栈 (#task-059)`（单元 3）
4. `docs(guide): 新增用户手册 USER_GUIDE.md（8 节覆盖全功能） (#task-059)`（单元 4）
5. 自验全绿后，最后一次提交：`docs(state): 任务059完成，v0.1.7 发版准备就绪，待用户手动 workflow_dispatch (#task-059)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 059: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「v0.1.7 发版（用户手动 workflow_dispatch，version=0.1.7）；后续按需迭代（代码签名购买决策 / 品牌图标）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-29  task-059  v0.1.7 发版准备（版本号/CHANGELOG/README/用户手册）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0；
- 版本号三处一致 = `0.1.7`（grep 验证）。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：本任务不改业务逻辑，预期现有 255+8 条全绿不回归。
- **集成测试（必做）**：`cargo test --workspace` + `pnpm test` 全绿。
- **端到端（本任务不做）**：无 DB 字段派生 UI 改动。
- **真机冒烟（本任务不强制）**：DevAgent 只做代码/文档/版本号改动，不做真机验证；但合并后由用户手动发版时验证。
- **打包安装冒烟（必做、不可裁剪）**：本任务合并后由用户在发版后跑（NSIS 干净目录安装验证 v0.1.7 版本号显示）。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务主体是文档/版本号改动，不直接命中「前端可观察行为」；但合并后立即触发 NSIS/MSI 打包，打包安装冒烟**必做、不可裁剪**（由用户在发版时执行）。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 版本号三处一致 | `grep -r "0.1.7" package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml` 输出 |
| 产物文件名含 0.1.7 | `pnpm build` 输出 + `src-tauri/target/release/bundle/` 目录列表 |
| CHANGELOG 定稿 | 文件内容预览（v0.1.7 日期 + 条目齐全） |
| README 重写 | 文件内容预览（简介/功能/安装/开发/技术栈） |
| 用户手册 | 文件存在 + 8 节目录齐全 |
| LICENSE | 文件存在 + MIT 开头 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码/文档提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-059-release-prep-v017`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 版本号 diff；
   - CHANGELOG v0.1.7 条目摘要；
   - README 最终结构；
   - 用户手册 8 节目录；
   - LICENSE 文件状态（已存在 / 新建）；
   - `cargo test --workspace` / `pnpm build` 通过输出；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（v0.1.7 发版待用户手动；代码签名购买决策待用户拍板；品牌图标仍不做）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务059-v0.1.7 发版准备 (#task-059)`，
   该提交只含 `tasks/059-release-prep-v017.md` 一个文件，落在 `feature/task-059-release-prep-v017` 分支最前端。
