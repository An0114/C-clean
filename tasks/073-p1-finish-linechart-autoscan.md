# 任务编号：073 P1 收尾——趋势图折线增强 + 开机自启/任务计划集成

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：P1 未做清单第二组收尾两项（P1-5 趋势图折线增强 + P1-1 开机自启系统级集成）。
> 上游依赖：task-072 已合并 main（merge `8ee6c11`，P1-7 清理方案模板完成）。
> **版本线**：v0.2.x P1 体验项收官。
> **本任务范围（4 个功能单元）**：趋势图折线增强（柱状→折线 + 时间范围切换）+ 开机自启开关（系统级）+ 后台任务计划集成 + 设置页接线。
> **为什么合并**：两个都是 P1 体验项收尾，趋势图增强（前端）与开机自启集成（后端+前端）虽不同模块，但都是 P1 最后一批，一次 PR 收官。
> **不做**：跨盘恢复 fallback / RmShutdown 三功能（维持现状）；架构债 ps1→Rust 重写（留 v0.4.x）。

## 状态字段

pending

## 任务目标

### 1. 趋势图折线增强

- **现状**：task-061 已做手写 SVG 柱状图（Dashboard 趋势图），显示近 7 天清理量。
- **修法**：
  - `app/src/pages/Dashboard.tsx`：
    - 柱状图改为折线图（SVG path，平滑曲线）；
    - 加时间范围切换：近 7 天 / 近 30 天 / 近 90 天；
    - Y 轴自适应（最大值动态缩放）；
    - hover 显示当日清理量 tooltip。

### 2. 开机自启开关（系统级）

- **现状**：task-062 托盘常驻已做「开机自启」开关，写注册表 `HKCU\...\Run`。
- **修法**：
  - 确认现有实现（注册表 Run 键写入）；
  - 设置页「开机自启」开关接线（如未接线）；
  - 显示自启状态（已开启/已关闭）；
  - 单测：开关切换后注册表正确写入/删除。

### 3. 后台任务计划集成

- **现状**：task-061 已做后台 30 分钟轮询（应用内轮询，只扫描不删除 + 系统通知）。
- **修法**：
  - `engine/src/ipc/commands/auto_scan.rs`（新建或修改）：
    - `auto_scan_enable()`：注册 Windows 任务计划程序（`schtasks /create`），系统级定时扫描；
    - `auto_scan_disable()`：删除任务计划；
    - `auto_scan_status()`：查询当前是否已注册；
  - 频率：每日一次（可配置，默认 09:00）。

### 4. 设置页接线

- **修法**：
  - `app/src/pages/Settings.tsx`：
    - 「开机自启」开关（注册表 Run 键）；
    - 「后台自动扫描」开关（任务计划程序）；
    - 显示当前状态；
    - 深色主题适配。

**本任务不做**：
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）；
- 不做架构债 ps1→Rust 重写（留 v0.4.x）。

## 前置条件

1. `main` 已合并 task-072（merge `8ee6c11`），P1-7 清理方案模板完成；
2. 关键锚点（必须全部在白名单内）：
   - 前端：`app/src/pages/Dashboard.tsx`（趋势图折线增强）
   - 后端：`engine/src/ipc/commands/auto_scan.rs`（新建或修改，任务计划集成）
   - 后端：`engine/src/ipc/commands/mod.rs`（注册 auto_scan 命令）
   - 前端：`app/src/pages/Settings.tsx`（开机自启 + 自动扫描开关）
   - 前端：`app/src/utils/invoke.ts`（API 封装，如需要）
   - 后端测试：`engine/tests/auto_scan_test.rs`（新建，如需要）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 趋势图折线增强

1. 柱状图改为折线图（SVG 平滑曲线）；
2. 时间范围切换：近 7 天 / 近 30 天 / 近 90 天；
3. Y 轴自适应（最大值动态缩放）；
4. hover 显示当日清理量 tooltip；
5. 单测：折线图渲染正确。

### 2. 开机自启开关

6. 设置页「开机自启」开关接线；
7. 开关切换后注册表 `HKCU\...\Run` 正确写入/删除；
8. 显示当前自启状态；
9. 单测：注册表写入/删除正确。

### 3. 后台任务计划集成

10. `auto_scan_enable()`：注册 Windows 任务计划程序；
11. `auto_scan_disable()`：删除任务计划；
12. `auto_scan_status()`：查询注册状态；
13. 单测：任务计划注册/删除正确。

### 4. 设置页接线

14. 设置页加「后台自动扫描」开关；
15. 显示当前状态；
16. 深色主题适配；
17. 单测：开关渲染正确。

### 后端单测

18. `cargo test --workspace` 全绿（现有 + 本任务新增 ≥4 条全绿）。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**后端（engine）**：
- `engine/src/ipc/commands/auto_scan.rs`（新建或修改，任务计划集成）
- `engine/src/ipc/commands/mod.rs`（注册 auto_scan 命令）
- `engine/src/lib.rs`（注册 IPC handler，如需要）
- `engine/tests/auto_scan_test.rs`（新建，如需要）

**前端（app）**：
- `app/src/pages/Dashboard.tsx`（趋势图折线增强）
- `app/src/pages/Settings.tsx`（开机自启 + 自动扫描开关）
- `app/src/utils/invoke.ts`（API 封装，如需要）

**收尾**：
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-073-p1-finish-linechart-autoscan`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-073-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `feat(dashboard): 趋势图折线增强（柱状→折线 + 时间范围切换 + hover tooltip） (#task-073)`（单元 1）
   —— 含 `app/src/pages/Dashboard.tsx`。
2. `feat(settings): 开机自启开关接线（注册表 Run 键 + 状态显示） (#task-073)`（单元 2）
   —— 含 `app/src/pages/Settings.tsx` + 后端（如需要）。
3. `feat(auto_scan): 后台任务计划集成（schtasks 注册/删除/状态查询） (#task-073)`（单元 3）
   —— 含 `engine/src/ipc/commands/auto_scan.rs` + `commands/mod.rs` + 测试。
4. `feat(settings): 后台自动扫描开关接线（任务计划状态 + 深色适配） (#task-073)`（单元 4）
   —— 含 `app/src/pages/Settings.tsx` + `utils/invoke.ts`。
5. 自验全绿后，最后一次提交：`docs(state): 任务073完成，P1 体验项收官（趋势图折线+开机自启+任务计划集成） (#task-073)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 073: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「P1 体验项全部收官；后续进入架构债（ps1→Rust 重写，v0.4.x）或 P2 项（多盘扫描 / 自定义规则 / 便携版 CLI）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-02  task-073  P1 收尾（趋势图折线+开机自启+任务计划）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：折线图渲染、注册表写入/删除、任务计划注册/删除、设置页开关。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **端到端写路径（必做，本任务硬要求）**：
  - 趋势图：切时间范围 → 折线图数据更新；
  - 开机自启：开关 → 注册表写入 → 重启生效；
  - 自动扫描：开关 → 任务计划注册 → schtasks /query 确认。
- **真机冒烟（必做、不可裁剪）**：
  - Dashboard 趋势图显示折线图（不再是柱状图）；
  - 时间范围切换（7/30/90 天）数据正确；
  - 设置页「开机自启」开关切换后注册表正确写入；
  - 设置页「后台自动扫描」开关切换后任务计划正确注册；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 4 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（趋势图折线 + 设置页开关，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 折线图渲染 | 真机截图：Dashboard 折线图 |
| 时间范围切换 | 真机证据：7/30/90 天切换数据更新 |
| 开机自启开关 | 真机证据：开关 → 注册表写入/删除 |
| 任务计划注册 | 真机证据：schtasks /query 确认 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 4 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-073-p1-finish-linechart-autoscan`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - 折线图 SVG 实现方式（path 平滑曲线）；
   - 时间范围切换数据来源（trend[] 字段）；
   - 开机自启注册表路径确认；
   - 任务计划 schtasks 命令参数；
   - `cargo test --workspace` 通过输出（含新增 ≥4 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（4 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（P1 全部收官；下一阶段：架构债 ps1→Rust 重写 / P2 多盘扫描 / 自定义规则 / 便携版 CLI；Items/SafeBox/History 深色适配待补）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务073-P1收尾（趋势图折线+开机自启+任务计划集成） (#task-073)`，
   该提交只含 `tasks/073-p1-finish-linechart-autoscan.md` 一个文件，落在 `feature/task-073-p1-finish-linechart-autoscan` 分支最前端。
