# 任务编号：082 UI 体验批量优化（仪表盘布局 + 列滚动 + 托盘菜单 + 逐项影响说明）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：v0.4.0 后 backlog 批次一（UI 组），用户拍板「5 个 UI 项不拆分，打包成一个任务派发」。
> 上游依赖：task-081 已合并 main（merge `6db58cd`，深色三页适配完成）。
> **版本线**：v0.4.x UI 体验批量优化。
> **本任务范围（5 个功能单元）**：仪表盘布局优化 + 注册表页命令列横向滚动 + 清理页路径列横向滚动 + 托盘菜单完善 + 逐项影响说明（hover 提示）。
> **为什么合并**：用户明确拍板「5 个 UI 项不拆分，打包成一个任务」，且都是纯前端样式/交互改进，模式一致。
> **不做**：批次二 P0 地基（088/089）；批次三软件感知（090+）；跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 状态字段

pending

## 任务目标

### 1. 仪表盘布局优化（082）

- **现状**：Dashboard 页面布局紧凑，卡片间距/字体大小/响应式适配有改进空间。
- **修法**：
  - `app/src/pages/Dashboard.tsx`：
    - 卡片间距从 8px 调整为 12px，提升呼吸感；
    - 数字卡片字体放大（主数字 28px → 32px）；
    - WinSxS 卡片与趋势图并排布局优化；
    - 多盘环形图间距优化（task-077 刚加多盘，需对齐视觉）；
    - 深色主题下卡片阴影微调。

### 2. 注册表页命令列横向滚动（083）

- **现状**：`app/src/pages/RegistryHealth.tsx`（注册表健康中心，task-068）的「命令」列内容过长时被截断，无横向滚动。
- **修法**：
  - 注册表表格的命令列（Run 键的命令行）：
    - 加 `overflow-x: auto` + `max-width: 300px`；
    - 鼠标 hover 显示完整路径（title 属性）；
    - 深色主题下滚动条样式适配。

### 3. 清理页路径列横向滚动（084）

- **现状**：`app/src/pages/Items.tsx`（清理页）的「路径」列内容过长时被截断。
- **修法**：
  - Items 表格的路径列：
    - 加 `overflow-x: auto` + `max-width: 400px`；
    - 鼠标 hover 显示完整路径（title 属性）；
    - 深色主题下滚动条样式适配；
    - 与 task-081 深色适配对齐（CSS 变量）。

### 4. 托盘菜单完善（086）

- **现状**：task-062 已做托盘常驻（关闭=最小化托盘、右键菜单「立即扫描/打开主窗口/退出」）。
- **修法**：
  - 托盘右键菜单完善：
    - 「立即扫描」→ 触发扫描（同 Dashboard 按钮）；
    - 「打开主窗口」→ 显示窗口；
    - 「暂停自动扫描」/「恢复自动扫描」→ 切换后台轮询（task-061 已做）；
    - 「退出」→ 二次确认（task-062 已做）；
  - 左键单击托盘图标 → 显示/隐藏窗口（toggle）；
  - 托盘 tooltip 显示当前状态（如「C-clean | 已扫描 N 项 | 已释放 X MB」）。

### 5. 逐项影响说明（087）

- **现状**：task-064 已做风险徽标（L0/L1/L2/L3 四色 + hover title）。
- **修法**：
  - Items 页每项加「影响说明」hover 提示：
    - L1 项：「安全清理，删除后下次使用自动重建（如缓存）」；
    - L2 项：「需确认，可能影响正在运行的程序（如临时安装文件）」；
    - L3 项：「受保护，不允许删除（如系统文件）」；
  - 保险箱页每项加「来源/大小」hover 补充；
  - 残留页（Residual.tsx）每项加「置信度说明」（high/low）。

**本任务不做**：
- 不做批次二 P0 地基（088/089，留后续）；
- 不做批次三软件感知（090+，留后续）；
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 前置条件

1. `main` 已合并 task-081（merge `6db58cd`），深色三页适配完成；
2. 全局 CSS 变量体系已建立（task-061/081，`--bg/--surface/--text/--text-muted/--border`）；
3. 关键锚点（必须全部在白名单内）：
   - 前端：`app/src/pages/Dashboard.tsx`（布局优化）
   - 前端：`app/src/pages/RegistryHealth.tsx`（命令列滚动）
   - 前端：`app/src/pages/Items.tsx`（路径列滚动 + 影响说明）
   - 前端：`app/src/pages/Residual.tsx`（置信度说明）
   - 后端：`app/src-tauri/src/lib.rs`（托盘菜单完善）
   - 后端：`app/src-tauri/src/tray.rs`（如存在，或新建）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 仪表盘布局优化

1. 卡片间距 12px，呼吸感提升；
2. 数字卡片主字体 32px；
3. 多盘环形图间距对齐；
4. 深色主题下卡片阴影微调；
5. 单测：组件渲染正确。

### 2. 注册表页命令列横向滚动

6. 命令列 `max-width: 300px` + `overflow-x: auto`；
7. hover 显示完整命令（title 属性）；
8. 深色主题滚动条样式适配；
9. 单测：表格渲染正确。

### 3. 清理页路径列横向滚动

10. 路径列 `max-width: 400px` + `overflow-x: auto`；
11. hover 显示完整路径（title 属性）；
12. 深色主题滚动条样式适配；
13. 单测：表格渲染正确。

### 4. 托盘菜单完善

14. 右键菜单含「立即扫描/打开主窗口/暂停自动扫描/恢复自动扫描/退出」；
15. 左键单击 toggle 显示/隐藏窗口；
16. tooltip 显示当前状态；
17. 单测：托盘菜单注册正确。

### 5. 逐项影响说明

18. Items 页 L1/L2/L3  hover 提示文案正确；
19. 保险箱页 hover 补充来源/大小；
20. 残留页 hover 置信度说明；
21. 单测：hover 提示渲染正确。

### 后端单测

22. `cargo test --workspace` 全绿（现有 + 本任务新增 ≥3 条全绿）；
23. `pnpm --filter app test` 全绿（现有组件测试不回归）。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**前端修改**：
- `app/src/pages/Dashboard.tsx`（布局优化）
- `app/src/pages/RegistryHealth.tsx`（命令列滚动）
- `app/src/pages/Items.tsx`（路径列滚动 + 影响说明）
- `app/src/pages/Residual.tsx`（置信度说明）
- `app/src/pages/SafeBox.tsx`（hover 补充，如需要）

**后端修改**：
- `app/src-tauri/src/lib.rs`（托盘菜单完善）
- `app/src-tauri/src/tray.rs`（如存在，或新建托盘逻辑文件）

**文档**：
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

**不修改**：
- 全局 CSS 变量体系（task-061/081 已建立，不改动）
- Settings.tsx（已适配，不改动）
- App.tsx（tab 结构不变，不改动）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-082-ui-batch-dashboard-scroll-tray-tooltip`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-082-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 5 个功能单元拆 5 个原子提交：

1. `feat(ui): 仪表盘布局优化（卡片间距 + 字体放大 + 多盘对齐） (#task-082)`（单元 1）
   —— 含 `app/src/pages/Dashboard.tsx`。
2. `feat(ui): 注册表页命令列横向滚动（max-width + overflow-x + title） (#task-082)`（单元 2）
   —— 含 `app/src/pages/RegistryHealth.tsx`。
3. `feat(ui): 清理页路径列横向滚动（max-width + overflow-x + title） (#task-082)`（单元 3）
   —— 含 `app/src/pages/Items.tsx`。
4. `feat(tray): 托盘菜单完善（右键菜单 + 左键 toggle + tooltip 状态） (#task-082)`（单元 4）
   —— 含 `app/src-tauri/src/lib.rs` + `tray.rs`（如新建）。
5. `feat(ui): 逐项影响说明（L1/L2/L3 hover 文案 + 残留置信度说明） (#task-082)`（单元 5）
   —— 含 `app/src/pages/Items.tsx` + `Residual.tsx` + `SafeBox.tsx`（如需要）。
6. 自验全绿后，最后一次提交：`docs(state): 任务082完成，UI 体验批量优化（仪表盘布局 + 列滚动 + 托盘菜单 + 影响说明） (#task-082)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 082: ...`；
   - 「最近一次变更」追加一行，含 5 个单元摘要 + commit hash；
   - 「下一步计划」改为「批次一 UI 全部完成；下一阶段：批次二 P0 地基（088 双份真值源收敛 + 089 规则 schema matches）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-08  task-082  UI 批量优化（仪表盘+滚动+托盘+影响说明）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：组件渲染、托盘菜单注册、hover 提示。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **真机冒烟（必做、不可裁剪）**：
  - Dashboard 卡片间距/字体大小视觉改善；
  - 注册表页命令列横向滚动（hover 显示完整命令）；
  - 清理页路径列横向滚动（hover 显示完整路径）；
  - 托盘右键菜单 5 项完整；
  - 左键单击 toggle 显示/隐藏窗口；
  - 托盘 tooltip 显示当前状态；
  - Items 页 L1/L2/L3 hover 提示文案正确；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 7 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（UI 布局/滚动/托盘/hover，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| Dashboard 布局优化 | 真机截图：卡片间距/字体对比 |
| 注册表命令列滚动 | 真机证据：hover 显示完整命令 |
| 清理页路径列滚动 | 真机证据：hover 显示完整路径 |
| 托盘菜单完善 | 真机证据：右键菜单 5 项 + tooltip |
| 逐项影响说明 | 真机证据：L1/L2/L3 hover 文案 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 7 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 5 个代码提交 + 1 个 `docs(state)` 提交，共 6 个 commit，全部 push 到 `feature/task-082-ui-batch-dashboard-scroll-tray-tooltip`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - Dashboard 布局调整具体数值（间距/字体/阴影）；
   - 列滚动实现方式（max-width + overflow-x + title）；
   - 托盘菜单完善具体项（5 项 + tooltip 文案）；
   - hover 提示文案确认（L1/L2/L3）；
   - `cargo test --workspace` 通过输出（含新增 ≥3 条用例）；
   - `pnpm test` 输出（无回归）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（7 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（批次一 UI 全部完成；下一阶段：批次二 P0 地基 088/089）；
3. `docs/PROJECT_STATE.md` 已按第 6 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务082-UI批量优化（仪表盘布局+列滚动+托盘菜单+影响说明） (#task-082)`，
   该提交只含 `tasks/082-ui-batch-dashboard-scroll-tray-tooltip.md` 一个文件，落在 `feature/task-082-ui-batch-dashboard-scroll-tray-tooltip` 分支最前端。
