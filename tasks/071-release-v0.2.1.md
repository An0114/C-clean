# 任务编号：071 v0.2.1 patch 发版准备（版本号升级 + CHANGELOG Bug 修复段）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：task-070 已合并 main（merge `93efffe`，v0.2.0 真机验收 3 Bug 修复完成），按版本线规则走 v0.2.1 patch 发版。
> 上游依赖：task-070 已合并 main，3 个 Bug 全部修复。
> **版本线**：v0.2.1 = patch 发版（只修 Bug，不新增功能）。
> **本任务范围（2 个功能单元）**：版本号升级（0.2.0 → 0.2.1）+ CHANGELOG v0.2.1 段落（3 个 Bug 修复）。
> **为什么合并**：patch 发版是一次完整 PR，版本号 + CHANGELOG 必须同步更新。
> **不做**：功能开发（原 P1-7 清理方案模板延后到 task-072）；跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 状态字段

pending

## 任务目标

### 1. 版本号升级（0.2.0 → 0.2.1）

- **现状**：`app/package.json` 版本号 `0.2.0`（task-069 发版准备已升级）。
- **修法**：
  - `app/package.json`：`"version": "0.2.0"` → `"0.2.1"`；
  - 其他版本号字段同步升级（根 package.json / tauri.conf.json / src-tauri Cargo.toml）；
  - engine/elevation Cargo.toml 保持 0.1.0 不变（内部构件不单独发版）；
  - 确认无遗漏。

### 2. CHANGELOG v0.2.1 段落

- **修法**：
  - `CHANGELOG.md` 顶部加 `## [0.2.1] - 2026-11-02` 段落；
  - 按 Keep a Changelog 格式分「修复」段（patch 发版无新增/变更）；
  - 修复内容含：
    - **Bug-1 提权命令 argv 缺失**（task-070）：WinSxS 分析按钮 DISM 退出码 3，根因是提权命令名从未进入 argv 数组；
    - **Bug-2 扫描风险等级硬编码 L1**（task-070）：5 个 ps1 扫描脚本把所有项写死成 L1，改为按分类判定 L0/L1/L2；
    - **Bug-3 目录永久删除失败 + 残留不记来源**（task-070）：保险箱目录条目永久删除报内部错误，残留移入保险箱未写 source_task_id。

**本任务不做**：
- 不做 README / 用户手册同步（patch 发版，无新功能，无使用说明变化）；
- 不做功能开发（原 P1-7 清理方案模板延后到 task-072）；
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 前置条件

1. `main` 已合并 task-070（merge `93efffe`），3 个 Bug 全部修复；
2. 关键锚点（必须全部在白名单内）：
   - `app/package.json`（版本号）
   - 根 `package.json`（版本号）
   - `tauri.conf.json`（版本号）
   - `src-tauri/Cargo.toml`（版本号）
   - `CHANGELOG.md`（v0.2.1 段落）
   - `docs/PROJECT_STATE.md`（收尾更新）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 版本号升级

1. `app/package.json` 版本号 `0.2.0` → `0.2.1`；
2. 其他版本号字段同步升级（根 package.json / tauri.conf.json / src-tauri Cargo.toml）；
3. engine/elevation Cargo.toml 保持 0.1.0 不变；
4. `pnpm --filter app build` 成功，无版本不一致报错。

### 2. CHANGELOG v0.2.1 段落

5. `CHANGELOG.md` 顶部加 `## [0.2.1] - 2026-11-02`；
6. 「修复」段含 3 个 Bug 修复说明；
7. Keep a Changelog 格式正确。

### 构建验证

8. `pnpm --filter app build` 退出码 0；
9. `cargo check --workspace` 退出码 0。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**版本号（4 处）**：
- `app/package.json`（version 字段）
- 根 `package.json`（version 字段）
- `tauri.conf.json`（version 字段）
- `src-tauri/Cargo.toml`（version 字段）

**文档**：
- `CHANGELOG.md`（加 v0.2.1 段落）
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

**不修改**：
- `README.md`（patch 发版，无新功能）
- `docs/USER_GUIDE.md`（patch 发版，无使用说明变化）
- `engine/Cargo.toml` / `elevation/Cargo.toml`（保持 0.1.0）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-071-release-v0.2.1`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-071-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 2 个功能单元拆 2 个原子提交：

1. `chore(release): 版本号 0.2.0 → 0.2.1 (patch) (#task-071)`（单元 1）
   —— 含 4 处版本号字段。
2. `docs(changelog): v0.2.1 段落（3 个真机验收 Bug 修复） (#task-071)`（单元 2）
   —— 含 `CHANGELOG.md`。
3. 自验全绿后，最后一次提交：`docs(state): 任务071完成，v0.2.1 patch 发版准备就绪 (#task-071)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 071: ...`；
   - 「最近一次变更」追加一行，含 2 个单元摘要 + commit hash；
   - 「下一步计划」改为「v0.2.1 patch 发版准备就绪，等用户手动 workflow_dispatch 发版；后续推进 P1-7 清理方案模板（task-072）或 P1 体验项」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-02  task-071  v0.2.1 patch 发版准备`。

**提交前自验（必须全绿）：**
- `cargo check --workspace` 退出码 0；
- `pnpm --filter app build` 退出码 0；
- 版本号全局搜索确认无遗漏 `0.2.0`（除历史 CHANGELOG 段）。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **构建验证（必做）**：`cargo check --workspace` + `pnpm build` 全绿。
- **文档检查（必做）**：CHANGELOG 格式正确、版本号四处一致。
- **真机/打包冒烟**：**本任务为纯文档/版本号变更，无前端可观察行为，真机/打包冒烟可裁剪**——但需在 NSIS 干净目录安装后确认版本号显示为 0.2.1（简单验证）。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 版本号升级 | 4 处版本号字段 diff 截图 |
| CHANGELOG 段落 | `CHANGELOG.md` v0.2.1 段落截图 |
| 构建验证 | `pnpm build` 输出（无 error） |
| 版本号显示 | 安装后「关于」页显示 0.2.1 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 2 个代码提交 + 1 个 `docs(state)` 提交，共 3 个 commit，全部 push 到 `feature/task-071-release-v0.2.1`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - 版本号升级的所有文件清单（4 处版本号字段）；
   - CHANGELOG v0.2.1 段落字数 / 条目数；
   - `pnpm build` 输出（无 error）；
   - 版本号全局搜索结果（确认无遗漏 0.2.0）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（等用户手动 workflow_dispatch 发版；后续 P1-7 清理方案模板 task-072）；
3. `docs/PROJECT_STATE.md` 已按第 3 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务071-v0.2.1 patch发版准备 (#task-071)`，
   该提交只含 `tasks/071-release-v0.2.1.md` 一个文件，落在 `feature/task-071-release-v0.2.1` 分支最前端。
