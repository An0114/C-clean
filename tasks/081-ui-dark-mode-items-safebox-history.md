# 任务编号：081 UI 深色适配补完——Items / SafeBox / History 三页

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：v0.4.0 发版后遗留 UI 适配项（task-061 已做 App/Dashboard/Settings 三页深色，Items/SafeBox/History 三页未适配）。
> 上游依赖：task-080 已合并 main（merge `1fb9bb8`），v0.4.0 已发版（tag `v0.4.0`）。
> **版本线**：v0.4.x 补丁——UI 深色适配补完。
> **本任务范围（3 个功能单元）**：Items 页深色适配 + SafeBox 页深色适配 + History 页深色适配。
> **为什么合并**：三页都是纯样式调整，模式一致（跟随全局 theme context），合并一次 PR 完成。
> **不做**：规则导入/导出（留 task-082）；跨盘恢复 fallback（维持现状）。

## 状态字段

pending

## 任务目标

### 1. Items 页深色适配

- **现状**：`app/src/pages/Items.tsx` 使用硬编码浅色样式（背景白 / 文字黑），未跟随全局 theme。
- **修法**：
  - 读取全局 theme context（task-061 已实现）；
  - 背景色：浅色 `#ffffff` / 深色 `#1e1e1e`；
  - 文字色：浅色 `#333333` / 深色 `#e0e0e0`；
  - 表格边框：浅色 `#e0e0e0` / 深色 `#3c3c3c`；
  - 按钮 / checkbox / 进度条样式跟随主题。

### 2. SafeBox 页深色适配

- **现状**：`app/src/pages/SafeBox.tsx` 同上，硬编码浅色。
- **修法**：
  - 同 Items 页模式；
  - 红色「永久删除」按钮在深色模式下调整亮度（避免刺眼）。

### 3. History 页深色适配

- **现状**：`app/src/pages/History.tsx` 同上，硬编码浅色。
- **修法**：
  - 同 Items 页模式；
  - 时间戳 / 大小数字样式跟随主题。

**本任务不做**：
- 不做规则导入/导出（留 task-082）；
- 不做跨盘恢复 fallback（维持现状）；
- 不做新功能（纯样式适配）。

## 前置条件

1. `main` 已合并 task-080（merge `1fb9bb8`），v0.4.0 已发版；
2. 全局 theme context 已实现（task-061，App/Dashboard/Settings 三页已适配）；
3. 关键锚点（必须全部在白名单内）：
   - `app/src/pages/Items.tsx`（深色适配）
   - `app/src/pages/SafeBox.tsx`（深色适配）
   - `app/src/pages/History.tsx`（深色适配）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. Items 页深色适配

1. Items 页浅色模式下样式不变（回归保护）；
2. Items 页深色模式下背景 / 文字 / 边框 / 按钮 / checkbox / 进度条正确；
3. 表格行 hover 状态在深色模式下可辨认。

### 2. SafeBox 页深色适配

4. SafeBox 页浅色模式下样式不变；
5. SafeBox 页深色模式下样式正确；
6. 红色「永久删除」按钮深色模式下亮度调整。

### 3. History 页深色适配

7. History 页浅色模式下样式不变；
8. History 页深色模式下样式正确；
9. 时间戳 / 大小数字深色模式下可辨认。

### 后端单测

10. `pnpm --filter app test` 全绿（现有组件测试不回归）。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**前端修改**：
- `app/src/pages/Items.tsx`（深色适配）
- `app/src/pages/SafeBox.tsx`（深色适配）
- `app/src/pages/History.tsx`（深色适配）

**文档**：
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

**不修改**：
- 全局 theme context（task-061 已实现，不改动）
- App.tsx / Dashboard.tsx / Settings.tsx（已适配，不改动）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-081-ui-dark-mode-items-safebox-history`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-081-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 3 个功能单元拆 3 个原子提交：

1. `feat(ui): Items 页深色适配（跟随全局 theme context） (#task-081)`（单元 1）
   —— 含 `app/src/pages/Items.tsx`。
2. `feat(ui): SafeBox 页深色适配（含红色按钮亮度调整） (#task-081)`（单元 2）
   —— 含 `app/src/pages/SafeBox.tsx`。
3. `feat(ui): History 页深色适配（时间戳/数字样式跟随主题） (#task-081)`（单元 3）
   —— 含 `app/src/pages/History.tsx`。
4. 自验全绿后，最后一次提交：`docs(state): 任务081完成，UI 深色适配补完（Items/SafeBox/History 三页） (#task-081)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 081: ...`；
   - 「最近一次变更」追加一行，含 3 个单元摘要 + commit hash；
   - 「下一步计划」改为「UI 深色适配全部完成；下一阶段：规则导入/导出（task-082）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-07  task-081  UI 深色适配补完（Items/SafeBox/History）`。

**提交前自验（必须全绿）：**
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：现有组件测试不回归。
- **真机冒烟（必做、不可裁剪）**：
  - Items 页浅色模式样式不变；
  - Items 页深色模式样式正确（背景/文字/表格/按钮）；
  - SafeBox 页深色模式样式正确（红色按钮亮度合适）；
  - History 页深色模式样式正确（时间戳/数字可辨认）；
  - 三页切换浅色/深色即时生效（无需重启）；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 5 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（UI 样式可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| Items 浅色回归 | 真机截图：Items 浅色模式 |
| Items 深色适配 | 真机截图：Items 深色模式 |
| SafeBox 深色适配 | 真机截图：SafeBox 深色模式 |
| History 深色适配 | 真机截图：History 深色模式 |
| 即时切换 | 真机证据：Settings 切换 theme，三页即时变化 |
| 不回归 | `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 5 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-081-ui-dark-mode-items-safebox-history`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - 深色配色方案确认（背景 / 文字 / 边框 / 按钮色值）；
   - 红色按钮亮度调整方式；
   - `pnpm test` 输出（无回归）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（5 条截图路径：Items 浅色 / Items 深色 / SafeBox 深色 / History 深色 / 即时切换）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（UI 深色适配全部完成；下一阶段：规则导入/导出 task-082）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务081-UI深色适配补完（Items/SafeBox/History） (#task-081)`，
   该提交只含 `tasks/081-ui-dark-mode-items-safebox-history.md` 一个文件，落在 `feature/task-081-ui-dark-mode-items-safebox-history` 分支最前端。
