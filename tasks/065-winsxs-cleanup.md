# 任务编号：065 M1 C 模块——WinSxS 清理（DISM 分析+清理，走提权 exe）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：M1 业务模块缺口第一组第 1 项（C 模块 WinSxS 清理，TRD §82/§370）。
> 上游依赖：task-064 已合并 main（merge `d7d155a`，P0 全部清完）；task-062 提权 exe 底座已就绪（merge `c36639f`）。
> **版本线**：v0.3.x 起步——M1 业务模块落地。
> **本任务范围（4 个功能单元）**：WinSxS 分析命令 + 清理命令 + 前端 UI + 提权 exe 白名单扩展。
> **为什么合并**：C 模块 WinSxS 是一个完整业务闭环（后端分析 + 后端清理 + 前端展示 + 提权命令），必须一次 PR 打通。
> **不做**：D 模块残留 / E 模块大文件 / F 模块注册表（留后续任务）；跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 状态字段

pending

## 任务目标

### 1. WinSxS 分析命令（DISM /AnalyzeComponentStore）

- **TRD §82 描述**：调用 `dism /online /cleanup-image /analyzecomponentstore`，返回可清理空间。
- **修法**：
  - 提权 exe 加命令 `dism_analyze`；
  - 后端 `engine/src/scan/winsxs.rs` 封装调用，解析 DISM 输出 XML；
  - 返回 `{analyze_date, reusable_packets, backports_and_drivers, recommended_cleanup: bool}`；
  - 扫描时自动跑（或用户手动触发）。

### 2. WinSxS 清理命令（DISM /StartComponentCleanup）

- **TRD §370 描述**：调用 `dism /online /cleanup-image /startcomponentcleanup`，**硬编码禁 /ResetBase**（回退不了更新）。
- **修法**：
  - 提权 exe 加命令 `dism_cleanup`；
  - 后端封装调用，实时转发输出进度；
  - 硬编码禁 `/ResetBase`（防误删回退能力）；
  - 清理前自动创建系统还原点（task-062 已就绪）。

### 3. 前端 WinSxS UI

- **修法**：
  - Dashboard 或 Items 页加「WinSxS」模块；
  - 显示分析结果（可清理空间 / 是否建议清理）；
  - 「立即清理」按钮 → 弹 confirm「将删除 X GB，是否继续？」；
  - 清理中显示进度（DISM 输出转发）。

### 4. 提权 exe 白名单扩展

- **修法**：
  - `elevation/` 命令白名单加 `dism_analyze` / `dism_cleanup`；
  - 拒绝 `/ResetBase` 参数（硬编码）；
  - 单测：白名单拒绝 `/ResetBase`。

**本任务不做**：
- 不做 D 模块残留 / E 模块大文件 / F 模块注册表（留后续任务）；
- 不做 `/ResetBase`（硬编码禁）；
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 前置条件

1. `main` 已合并 task-064（merge `d7d155a`），P0 全部清完；
2. 提权 exe 底座已就绪（task-062，`engine::elevation::run_elevated` API 可用）；
3. 关键锚点：
   - `elevation/src/main.rs`（命令白名单加 `dism_analyze` / `dism_cleanup`）
   - `engine/src/scan/winsxs.rs`（新建，DISM 分析+清理封装）
   - `engine/src/commands.rs`（加 `winsxs_analyze` / `winsxs_cleanup` IPC）
   - `app/src/pages/Dashboard.tsx` 或 `Items.tsx`（WinSxS 模块 UI）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. WinSxS 分析

1. 提权 exe 加命令 `dism_analyze`；
2. 后端解析 DISM `/analyzecomponentstore` 输出 XML；
3. 返回 `{reusable_packets, backports, recommended_cleanup}`；
4. 单测：DISM 输出 XML 解析正确。

### 2. WinSxS 清理

5. 提权 exe 加命令 `dism_cleanup`；
6. 硬编码禁 `/ResetBase`；
7. 清理前自动创建系统还原点；
8. 实时转发 DISM 输出进度；
9. 单测：白名单拒绝 `/ResetBase`。

### 3. 前端 WinSxS UI

10. 显示分析结果（可清理空间 / 建议清理）；
11. 「立即清理」按钮弹 confirm；
12. 清理中显示进度；
13. 单测：UI 渲染正确。

### 4. 提权 exe 白名单

14. 白名单加 `dism_analyze` / `dism_cleanup`；
15. 拒绝 `/ResetBase` 参数；
16. 单测：白名单拒绝未知命令 + `/ResetBase`。

### 后端单测

17. `cargo test --workspace` 全绿（现有 + 本任务新增 ≥4 条全绿）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `elevation/src/main.rs`（命令白名单加 dism 命令）
- `engine/src/scan/winsxs.rs`（新建）
- `engine/src/commands.rs`（WinSxS IPC）
- `engine/tests/`（DISM 解析 + 白名单用例）
- `app/src/pages/Dashboard.tsx` 或 `Items.tsx`（WinSxS UI）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-065-winsxs-cleanup`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-065-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `feat(elevation): 提权 exe 白名单加 dism_analyze / dism_cleanup，硬编码禁 /ResetBase (#task-065)`（单元 1+4）
   —— 含 `elevation/src/main.rs` + 单测。
2. `feat(scan): WinSxS 分析命令（DISM /AnalyzeComponentStore + XML 解析） (#task-065)`（单元 2）
   —— 含 `engine/src/scan/winsxs.rs` + `commands.rs` + 单测。
3. `feat(scan): WinSxS 清理命令（DISM /StartComponentCleanup + 还原点自动创建） (#task-065)`（单元 2 续）
   —— 含 `winsxs.rs` + 单测。
4. `feat(ui): WinSxS 模块 UI（分析结果 + 清理按钮 + 进度显示） (#task-065)`（单元 3）
   —— 含 `Dashboard.tsx` 或 `Items.tsx` + 单测。
5. 自验全绿后，最后一次提交：`docs(state): 任务065完成，M1 C 模块 WinSxS 清理就绪 (#task-065)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 065: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「M1 第 2 项：E 模块大文件地图（task-066）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-30  task-065  M1 C 模块 WinSxS 清理`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：DISM XML 解析、白名单拒绝 `/ResetBase`、还原点自动创建。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **端到端写路径（必做，本任务硬要求）**：WinSxS 清理必须有一条真实「点清理按钮 → 提权 exe 跑 DISM → 系统还原点创建 → DISM 输出转发 → 完成」证据（不能只构造 mock）。
- **真机冒烟（必做、不可裁剪）**：
  - 点「分析」按钮 → DISM 跑 → 显示可清理空间；
  - 点「清理」按钮 → UAC 弹窗 → 系统还原点创建 → DISM 跑 → 进度显示；
  - 硬编码禁 `/ResetBase`（尝试传参被拒绝）；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 3 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（WinSxS 分析结果 + 清理按钮 + 进度显示，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| DISM XML 解析 | 单测输出（字段映射正确） |
| 白名单拒绝 /ResetBase | 单测输出（返回错误） |
| 分析结果显示 | 真机截图：Dashboard 显示 WinSxS 可清理空间 |
| 清理流程 | 真机证据：UAC 弹窗 → 还原点创建 → DISM 进度 → 完成 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 3 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-065-winsxs-cleanup`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - DISM 输出 XML 解析字段；
   - `/ResetBase` 硬编码禁止实现；
   - 还原点自动创建时机；
   - `cargo test --workspace` 通过输出（含新增 ≥4 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（3 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（M1 第 2 项：E 模块大文件地图；D/F 模块排队）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务065-M1 C模块 WinSxS清理 (#task-065)`，
   该提交只含 `tasks/065-winsxs-cleanup.md` 一个文件，落在 `feature/task-065-winsxs-cleanup` 分支最前端。
