# 任务编号：072 P1-7 清理方案模板（cleanup_plan 接线：save / apply / import / export）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：P1 未做清单第二组第 3 项（P1-7 清理方案模板，TRD §157）。原计划 task-070，因 v0.2.0 真机验收 Bug 延后。
> 上游依赖：task-071 已合并 main（merge `82223fa`，v0.2.1 patch 发版准备完成）。
> **版本线**：v0.2.x P1 体验项持续推进。
> **本任务范围（4 个功能单元）**：方案保存后端 + 方案应用后端 + 前端方案模板页面 + tab 路由注册。
> **为什么合并**：清理方案模板是一个完整业务闭环（后端 save/apply/import/export + 前端列表/应用/导入导出），必须一次 PR 打通。
> **不做**：P1-5 趋势图折线增强（task-061 已做本体）；P1-1 开机自启/任务计划集成（留后续）；跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 状态字段

pending

## 任务目标

### 1. 方案保存后端（cleanup_plan.save）

- **TRD §157 描述**：cleanup_plan 表已建，用户可将当前清理选择保存为命名方案。
- **现状**：表结构已存在，但无 save IPC。
- **修法**：
  - `engine/src/ipc/commands/plan.rs`（新建）：
    - `plan_save(name, selections)`：保存当前选择为命名方案；
    - `plan_list()`：列出所有已保存方案；
    - `plan_delete(id)`：删除方案。

### 2. 方案应用后端（cleanup_plan.apply）

- **修法**：
  - `plan_apply(id)`：返回方案数据，前端恢复勾选；
  - `plan_export(id)`：导出为 JSON 文件；
  - `plan_import(file_path)`：从 JSON 文件导入方案。

### 3. 前端方案模板页面

- **修法**：
  - `app/src/pages/PlanTemplates.tsx`（新建）：
    - 方案列表：名称 / 创建时间 / 条目数 / 大小；
    - 「保存当前选择为方案」按钮；
    - 「应用」按钮：恢复选择到 Items 页；
    - 「导出」按钮：下载 JSON 文件；
    - 「导入」按钮：选择 JSON 文件导入；
    - 「删除」按钮：confirm 后删除。

### 4. tab 路由注册

- **修法**：
  - `app/src/App.tsx`：Tab 类型加 `"plans"`，Tab 栏加「方案」按钮；
  - 切换渲染 `<PlanTemplates />`；
  - 深色主题适配（CSS 变量）。

**本任务不做**：
- 不做方案分享（仅本地导入导出 JSON）；
- 不做方案版本控制；
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 前置条件

1. `main` 已合并 task-071（merge `82223fa`），v0.2.1 patch 发版准备完成；
2. cleanup_plan 表已建（TRD §157），只需接线 IPC；
3. 关键锚点（必须全部在白名单内）：
   - 后端：`engine/src/ipc/commands/plan.rs`（新建）
   - 后端：`engine/src/ipc/commands/mod.rs`（注册）
   - 后端：`engine/src/lib.rs`（注册 IPC handler，如需要）
   - 后端测试：`engine/tests/plan_test.rs`（新建）
   - 前端：`app/src/pages/PlanTemplates.tsx`（新建）
   - 前端：`app/src/pages/PlanTemplates.test.tsx`（新建）
   - 前端：`app/src/App.tsx`（tab 注册）
   - 前端：`app/src/utils/invoke.ts`（API 封装，如需要）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 方案保存后端

1. `plan_save(name, selections)`：保存当前选择为命名方案；
2. `plan_list()`：列出所有已保存方案；
3. `plan_delete(id)`：删除方案；
4. 单测：save/list/delete 逻辑正确。

### 2. 方案应用后端

5. `plan_apply(id)`：返回方案数据，前端恢复勾选；
6. `plan_export(id)`：导出为 JSON 文件；
7. `plan_import(file_path)`：从 JSON 文件导入；
8. 单测：apply/export/import 逻辑正确。

### 3. 前端方案模板页面

9. 方案列表显示名称 / 创建时间 / 条目数 / 大小；
10. 「保存当前选择为方案」按钮；
11. 「应用」按钮：恢复选择到 Items 页；
12. 「导出 / 导入」按钮：JSON 文件操作；
13. 「删除」按钮：confirm 后删除；
14. 深色主题适配；
15. 单测：页面渲染正确。

### 4. tab 路由注册

16. Tab 栏加「方案」按钮；
17. 切换渲染 `<PlanTemplates />`；
18. 单测：tab 切换正确。

### 后端单测

19. `cargo test --workspace` 全绿（现有 + 本任务新增 ≥4 条全绿）。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**后端（engine）**：
- `engine/src/ipc/commands/plan.rs`（新建）
- `engine/src/ipc/commands/mod.rs`（注册 plan 命令）
- `engine/src/lib.rs`（注册 IPC handler，如需要）
- `engine/tests/plan_test.rs`（新建）

**前端（app）**：
- `app/src/pages/PlanTemplates.tsx`（新建）
- `app/src/pages/PlanTemplates.test.tsx`（新建）
- `app/src/App.tsx`（tab 类型 + 按钮 + 渲染）
- `app/src/utils/invoke.ts`（加 plan API 封装，如需要）

**收尾**：
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-072-plan-templates`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-072-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `feat(plan): 方案保存后端（save / list / delete，cleanup_plan 表接线） (#task-072)`（单元 1）
   —— 含 `engine/src/ipc/commands/plan.rs` + `commands/mod.rs` + `engine/tests/plan_test.rs`。
2. `feat(plan): 方案应用后端（apply / export / import JSON） (#task-072)`（单元 2）
   —— 含 `engine/src/ipc/commands/plan.rs` 续 + `lib.rs`。
3. `feat(ui): 方案模板页面（列表 + 保存/应用/导出/导入/删除） (#task-072)`（单元 3）
   —— 含 `app/src/pages/PlanTemplates.tsx` + `PlanTemplates.test.tsx` + `utils/invoke.ts`。
4. `feat(ui): App tab 注册「方案」按钮 + 深色适配 (#task-072)`（单元 4）
   —— 含 `app/src/App.tsx`。
5. 自验全绿后，最后一次提交：`docs(state): 任务072完成，P1-7 清理方案模板就绪 (#task-072)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 072: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「P1 体验项收尾（趋势图折线增强 / 开机自启任务计划集成）或架构债（ps1→Rust 重写）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-02  task-072  P1-7 清理方案模板`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：save/list/apply/export/import 逻辑、handler 接线、页面渲染、tab 切换。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **端到端写路径（必做，本任务硬要求）**：方案模板必须有一条真实「勾选清理项 → 保存方案 → 应用方案 → Items 页恢复勾选」证据（不能只构造 mock）。
- **真机冒烟（必做、不可裁剪）**：
  - Tab 栏显示「方案」按钮；
  - 点击切换到 PlanTemplates 页面；
  - 方案列表显示名称 / 创建时间 / 条目数；
  - 「保存当前选择为方案」按钮可用；
  - 「应用」按钮恢复勾选到 Items 页；
  - 「导出 / 导入」JSON 文件操作正常；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 6 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（方案模板页面 + tab 切换，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| save/list/delete 逻辑 | 单测输出（CRUD 正确） |
| apply/export/import 逻辑 | 单测输出（JSON 格式正确） |
| tab 切换 | 真机截图：Tab 栏显示「方案」按钮 |
| 方案列表 | 真机截图：名称 + 创建时间 + 条目数 |
| 保存方案 | 真机证据：勾选项 → 保存 → 列表出现 |
| 应用方案 | 真机证据：点应用 → Items 页恢复勾选 |
| 导出/导入 | 真机证据：JSON 文件下载 / 选择导入 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 6 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-072-plan-templates`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - cleanup_plan 表结构确认（字段 / 索引）；
   - 方案数据格式（JSON schema）；
   - 应用方案时 Items 页恢复勾选的实现方式；
   - `cargo test --workspace` 通过输出（含新增 ≥4 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（6 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（P1 体验项收尾：趋势图折线增强 / 开机自启任务计划集成；架构债：ps1→Rust 重写；Items/SafeBox/History 深色适配待补）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务072-P1-7 清理方案模板 (#task-072)`，
   该提交只含 `tasks/072-plan-templates.md` 一个文件，落在 `feature/task-072-plan-templates` 分支最前端。
