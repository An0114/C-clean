# 任务编号：063 清理报告导出 + 前后空间对比 + 仪表盘环形图 + 取消扫描按钮

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：P0 未做清单第 3 项（P0-6 清理报告导出）+ 第 4 项（P0-8 仪表盘环形图 + P0-1 取消扫描按钮）。
> 上游依赖：task-062 已合并 main（merge `c36639f`，提权 exe 底座 + 系统还原点 + 托盘常驻完成）。
> **版本线**：v0.2.0 开发线持续推进——本任务是 P0 收尾体验项。
> **本任务范围（4 个功能单元）**：清理报告导出 + 前后空间对比 + 仪表盘 C 盘环形图 + 取消扫描按钮。
> **为什么合并**：都是 P0 体验项，前后空间对比（单元 2）依赖清理报告导出（单元 1）的字段；仪表盘环形图（单元 3）与取消扫描按钮（单元 4）都是 Dashboard 页体验增强。按「任务量尽量大」一次 PR 推进。
> **不做**：跨盘恢复 fallback / RmShutdown 三功能（维持现状）；WinSxS / 注册表（M1 模块，留后续）。

## 状态字段

pending

## 任务目标

### 1. P0-6 清理报告导出（HTML/CSV）

- **PRD 描述**：清理完成后生成报告，可导出为 HTML/CSV，含前后空间对比、成功/失败明细、释放总量。
- **现状**：`drive_used_bytes_before` 字段已预留（TRD §137），但无导出功能。
- **修法**：
  - 清理完成结果页加「导出报告」按钮；
  - HTML 导出：生成单文件 HTML（含内联 CSS，可直接打开）；
  - CSV 导出：导出明细行（路径 / 大小 / 状态 / 原因）；
  - 导出文件保存到 `%APPDATA%\com.and0114.cclean\reports\`。

### 2. 前后空间对比

- **TRD 描述**：用 `drive_used_bytes_before` 字段，清理前后对比 C 盘已用空间。
- **修法**：
  - 清理前记录 `drive_used_bytes_before`；
  - 清理后实时查 `drive_used_bytes_after`；
  - 结果页显示「清理前 X GB → 清理后 Y GB，释放 Z GB」。

### 3. P0-8 仪表盘 C 盘空间环形图

- **PRD §190 描述**：仪表盘显示 C 盘空间环形图（已用/可用比例）。
- **现状**：仪表盘已有统计卡片，但无环形图。
- **修法**：
  - 后端 `drive_usage()` 返回 `{total_bytes, used_bytes, free_bytes}`；
  - 前端 Dashboard 加 SVG 环形图（不引图表库）；
  - 环形图中心显示已用百分比。

### 4. P0-1 取消扫描按钮

- **PRD §96 描述**：扫描中用户可取消，立即停止并保留已扫描结果。
- **现状**：扫描中无取消按钮。
- **修法**：
  - 后端 `ScanOrchestrator` 加 cancellation token（`CancellationToken`）；
  - 前端扫描中显示「取消」按钮，点击后发 `scan_cancel` IPC；
  - 取消后保留已扫描到的结果（不清空）。

**本任务不做**：
- 不做 P0-4（保险箱 7 天 purge / 风险徽标 / 全选 L1 按钮）——留 task-064 收尾 P0；
- 不做 M1 C/D/E/F 模块（留后续任务）；
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 前置条件

1. `main` 已合并 task-062（merge `c36639f`）；
2. 关键锚点：
   - `engine/src/execute/executor.rs`（清理前记录 `drive_used_bytes_before`）
   - `engine/src/scan/orchestrator.rs`（加 cancellation token）
   - `engine/src/commands.rs`（加 `scan_cancel` IPC）
   - `app/src/pages/Items.tsx`（导出报告按钮 + 前后空间对比）
   - `app/src/pages/Dashboard.tsx`（环形图 + 取消扫描按钮）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 清理报告导出

1. 清理结果页加「导出报告」按钮；
2. HTML 导出：单文件 HTML，含内联 CSS，可直接打开；
3. CSV 导出：明细行（路径 / 大小 / 状态 / 原因）；
4. 导出文件保存到 `%APPDATA%\com.and0114.cclean\reports\`；
5. 单测：报告生成逻辑（HTML/CSV 格式正确）。

### 2. 前后空间对比

6. 清理前记录 `drive_used_bytes_before`；
7. 清理后实时查 `drive_used_bytes_after`；
8. 结果页显示「清理前 X → 清理后 Y，释放 Z」；
9. 单测：前后空间差值计算。

### 3. 仪表盘环形图

10. 后端 `drive_usage()` 返回 `{total, used, free}`；
11. 前端 Dashboard 加 SVG 环形图；
12. 环形图中心显示已用百分比；
13. 单测：`drive_usage()` 返回值格式。

### 4. 取消扫描按钮

14. 后端 `ScanOrchestrator` 加 cancellation token；
15. 前端扫描中显示「取消」按钮；
16. 取消后保留已扫描结果（不清空）；
17. 单测：cancellation token 触发后立即停止。

### 后端单测

18. `cargo test --workspace` 全绿（现有 291+8 + 本任务新增 ≥4 条全绿）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/executor.rs`（清理前记录 `drive_used_bytes_before`）
- `engine/src/scan/orchestrator.rs`（cancellation token）
- `engine/src/commands.rs`（`scan_cancel` IPC）
- `engine/src/report/mod.rs`（新建，HTML/CSV 导出）
- `engine/tests/`（报告/前后对比/取消扫描用例）
- `app/src/pages/Items.tsx`（导出报告 + 前后空间对比）
- `app/src/pages/Dashboard.tsx`（环形图 + 取消按钮）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-063-report-dashboard`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-063-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `feat(report): 清理报告导出（HTML 单文件 + CSV 明细，保存到 reports/） (#task-063)`（单元 1）
   —— 含 `engine/src/report/mod.rs` + `Items.tsx` + 单测。
2. `feat(execute): 前后空间对比（清理前/后 drive_used_bytes 差值显示） (#task-063)`（单元 2）
   —— 含 `executor.rs` + `Items.tsx` + 单测。
3. `feat(dashboard): C 盘空间环形图（SVG 环形图 + 已用百分比） (#task-063)`（单元 3）
   —— 含 `commands.rs` + `Dashboard.tsx` + 单测。
4. `feat(scan): 取消扫描按钮（cancellation token + 保留已扫描结果） (#task-063)`（单元 4）
   —— 含 `orchestrator.rs` + `commands.rs` + `Dashboard.tsx` + 单测。
5. 自验全绿后，最后一次提交：`docs(state): 任务063完成，报告导出+前后对比+环形图+取消扫描就绪 (#task-063)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 063: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「P0 第 5 项：保险箱 purge + 风险徽标 + 全选 L1（task-064，收尾 P0）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-30  task-063  报告导出+前后对比+环形图+取消扫描`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：报告生成格式、前后空间差值、cancellation token 立即停止。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **端到端写路径（必做，本任务硬要求）**：前后空间对比必须有一条真实「清理前记录 → 清理后查询 → 结果页显示差值」证据（不能只构造 mock）。
- **真机冒烟（必做、不可裁剪）**：
  - 清理结果页显示「清理前 X → 清理后 Y，释放 Z」；
  - 「导出报告」按钮生成 HTML 文件，可直接打开；
  - 「导出 CSV」按钮生成 CSV 文件，Excel 可打开；
  - 仪表盘 C 盘环形图渲染正确（已用/可用比例）；
  - 扫描中显示「取消」按钮，点击后立即停止并保留已扫描结果；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 6 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（导出按钮 + 环形图 + 取消按钮，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 前后空间对比 | 真机截图：结果页显示清理前/后/释放差值 |
| HTML 导出 | 真机证据：生成 HTML 文件，浏览器打开正常 |
| CSV 导出 | 真机证据：生成 CSV 文件，Excel 打开正常 |
| 环形图渲染 | 真机截图：仪表盘 SVG 环形图 |
| 取消扫描 | 真机证据：扫描中点击取消 → 立即停止 + 已扫描结果保留 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 6 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-063-report-dashboard`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - HTML 报告模板结构；
   - CSV 导出字段列表；
   - 前后空间差值计算公式；
   - 环形图 SVG 实现方式；
   - cancellation token 实现方式；
   - `cargo test --workspace` 通过输出（含新增 ≥4 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（6 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（P0 第 5 项：保险箱 purge / 风险徽标 / 全选 L1；M1 C/D/E/F 模块排队）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务063-报告导出+前后对比+环形图+取消扫描 (#task-063)`，
   该提交只含 `tasks/063-report-dashboard.md` 一个文件，落在 `feature/task-063-report-dashboard` 分支最前端。
