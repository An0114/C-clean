# 任务编号：061 M5 阶段起步——深色/浅色主题 + 清理历史趋势图 + 计划自动清理

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：PRD P1-1 / P1-5 / P2-3（M5 社区化长期阶段首批功能）。
> 上游依赖：v0.1.7 已发布（merge `3b2a961`），M2/M3/M4 全部收官；PROJECT_STATE 已知问题段已精简（commit `a1e72ac`）。
> **本任务范围（3 个功能单元，M5 起步厚任务）**：深色/浅色主题切换、清理历史趋势图、计划自动清理（定时任务）。
> **为什么把这些合并**：都是 M5 阶段首批 P1/P2 功能，互相独立但都属于「用户体验增强」类；按新提示词「同一阶段的相关功能合并派发」原则，一次 PR 启动 M5。
> **不做**：跨盘恢复 fallback / RmShutdown 连带兄弟进程 / RmShutdown 30s 无进度（用户决策维持现状）；代码签名 / 品牌图标 / G6（已决策不做或豁免）；差量更新 / release 瘦身（已决策不做）。

## 状态字段

pending

## 任务目标

### 1. P2-3 深色/浅色主题切换

- **PRD 描述**：用户可在深色/浅色主题间切换，跟随系统或手动指定。
- **修法**：
  - Tauri 2 内置主题 API（`tauri::theme::Theme`）或前端 CSS 变量切换；
  - 设置页加「外观」选项：跟随系统 / 浅色 / 深色（三态）；
  - 选择持久化到 prefs（复用 task-057 的 prefs 表）；
  - 首次启动跟随系统（`prefers-color-scheme`）。

### 2. P1-5 清理历史与趋势图

- **PRD 描述**：仪表盘显示最近 30 天清理量趋势图，让用户看到「清理收益」。
- **现状**：仪表盘已有统计卡片（清理轮次 / 释放总量），但无趋势图。
- **修法**：
  - 后端新增 `cleanup_stats_trend(days: i64) -> Vec<TrendPoint>`，按天聚合 `SUM(bytes_freed)`；
  - 前端仪表盘加趋势图（用轻量 SVG 或 recharts，不引重型库）；
  - 趋势图 X 轴 = 日期，Y 轴 = 释放空间（MB/GB 自适应）；
  - 空数据时显示「暂无清理记录」占位。

### 3. P1-1 计划自动清理

- **PRD 描述**：用户可设置定时任务（每天/每周/每月）自动扫描并清理。
- **修法**：
  - **MVP 范围**：只做「后台静默扫描 + 通知」，不做自动删除（安全第一）；
  - prefs 表加 `auto_scan_enabled: bool`（默认 false）+ `auto_scan_interval: String`（daily/weekly/monthly）；
  - 后端用 Windows 任务计划程序（`schtasks`）或 Tauri 后台定时器触发扫描；
  - 扫描完成后发系统通知（「上次扫描释放 X GB」），不自动删除；
  - 设置页加「自动扫描」开关 + 间隔选择（daily/weekly/monthly）。

**本任务不做**：
- 不做自动删除（安全第一，只扫描 + 通知）；
- 不做 P1-2 大文件地图 / P1-3 残留定位 / P1-4 注册表健康 / P1-7 方案模板 / P1-8 目标空间（留后续任务）；
- 不做 P2-1 多盘 / P2-2 自定义规则 / P2-4 便携版 CLI（留后续任务）。

## 前置条件

1. `main` 已合并 task-060（merge `3b2a961`），PROJECT_STATE 已更新（commit `a1e72ac`）；
2. 关键锚点：
   - `engine/src/prefs.rs`（加主题/自动扫描字段）
   - `engine/src/audit_log/stats.rs`（加趋势聚合 SQL）
   - `app/src/pages/Settings.tsx`（主题/自动扫描 UI）
   - `app/src/pages/Dashboard.tsx`（趋势图）
   - `app/src/index.css` 或 `App.tsx`（主题 CSS 变量）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 深色/浅色主题

1. 设置页「外观」三态：跟随系统 / 浅色 / 深色；
2. 选择持久化到 prefs（重开应用保留）；
3. 切换后立即生效（不需重启）；
4. 首次启动跟随系统；
5. 单测：prefs 默认值 = system。

### 2. 清理历史趋势图

6. 后端 `cleanup_stats_trend(days)` 返回按天聚合的 `SUM(bytes_freed)`；
7. 前端仪表盘显示最近 30 天趋势图（SVG 折线或柱状）；
8. 空数据时显示「暂无清理记录」占位；
9. 单测：趋势聚合 SQL（3 天数据 → 3 个点）。

### 3. 计划自动扫描

10. prefs 表加 `auto_scan_enabled`（默认 false）+ `auto_scan_interval`（默认 daily）；
11. 设置页加「自动扫描」开关 + 间隔选择；
12. 后台定时器触发扫描（不自动删除）；
13. 扫描完成发系统通知；
14. 单测：开关默认 false + 间隔默认 daily。

### 后端单测

15. `cargo test -p engine` 全绿（现有 255+8 + 本任务新增 ≥3 条全绿）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/prefs.rs`（主题/自动扫描字段）
- `engine/src/audit_log/stats.rs`（趋势聚合）
- `engine/tests/`（趋势/自动扫描用例）
- `app/src/pages/Settings.tsx`（外观/自动扫描 UI）
- `app/src/pages/Dashboard.tsx`（趋势图）
- `app/src/index.css` 或 `App.tsx`（主题 CSS 变量）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-061-m5-start`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-061-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 3 个功能单元拆 3 个原子提交：

1. `feat(ui): 深色/浅色主题切换（跟随系统/浅色/深色三态，prefs 持久化） (#task-061)`（单元 1）
2. `feat(stats): 清理历史趋势图（最近 30 天按天聚合，仪表盘 SVG 折线） (#task-061)`（单元 2）
3. `feat(scan): 计划自动扫描（开关+间隔，后台触发，只扫描不删除，完成发通知） (#task-061)`（单元 3）
4. 自验全绿后，最后一次提交：`docs(state): 任务061完成，M5 起步——主题切换/趋势图/自动扫描 (#task-061)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 061: ...`；
   - 「最近一次变更」追加一行，含 3 个单元摘要 + commit hash；
   - 「下一步计划」改为「M5 持续推进：P1-2/P1-3/P1-4/P1-7/P1-8 等后续功能」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-29  task-061  M5 起步（主题/趋势图/自动扫描）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：prefs 新字段默认值、趋势聚合 SQL、自动扫描开关默认值。
- **集成测试（必做）**：`cargo test -p engine` 全绿。
- **端到端写路径（必做，本任务硬要求）**：主题切换必须有一条真实「切换主题 → 查 prefs 存储 → 重开应用主题仍在」证据；自动扫描开关同理。
- **真机冒烟（必做、不可裁剪）**：
  - 主题切换：浅色/深色/跟随系统三态切换后立即生效；
  - 主题持久化：切换主题 → 重开应用 → 主题仍在；
  - 趋势图：仪表盘显示最近 30 天趋势（有历史数据时）；
  - 自动扫描：开启开关 → 后台扫描触发 → 发系统通知（不自动删除）；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 4 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（主题切换 UI + 趋势图 + 自动扫描开关，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 主题三态切换 | 真机截图：浅色/深色/跟随系统三态 |
| 主题持久化 | 真机证据：切换主题 → 重开应用 → 主题仍在（DevTools 查 prefs） |
| 趋势图渲染 | 真机截图：仪表盘趋势图 |
| 趋势图空数据 | 真机证据：无历史数据时显示占位 |
| 自动扫描开关 | 真机证据：开启开关 → 后台扫描触发 → 发系统通知 |
| 不自动删除 | 真机证据：扫描完成后文件仍在原位（不自动清理） |
| 不回归 | `cargo test -p engine` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 4 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-061-m5-start`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 主题实现方式（CSS 变量 vs Tauri theme API）；
   - 趋势图实现方式（SVG vs recharts）；
   - 自动扫描触发方式（schtasks vs 后台定时器）；
   - `cargo test -p engine` 通过输出（含新增 ≥3 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（4 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（M5 后续功能：P1-2/P1-3/P1-4/P1-7/P1-8 等）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务061-M5 起步（主题切换+趋势图+自动扫描） (#task-061)`，
   该提交只含 `tasks/061-m5-start.md` 一个文件，落在 `feature/task-061-m5-start` 分支最前端。
