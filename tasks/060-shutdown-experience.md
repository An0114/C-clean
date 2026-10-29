# 任务编号：060 占用关闭体验——RmShutdown 连带兄弟进程 + 30s 进度反馈 + 跨盘恢复文档化

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：PROJECT_STATE「下一步计划 3. 跨盘恢复 fallback、RmShutdown 连带兄弟进程、RmShutdown 30s 无进度 → 登记为日后新功能点」。
> 上游依赖：v0.1.7 已发布（merge `360df74`），M2/M3/M4 全部收官。
> **本任务范围（3 个功能单元）**：RmShutdown 连带兄弟进程验证/实现、30s 进度反馈、跨盘恢复 fallback 文档化。
> **为什么把这些合并**：都是「占用关闭」相关的体验优化，同一模块（engine/src/execute/locking.rs + 前端 Items.tsx），按新提示词「同一模块前后端闭环必须合并派发」原则，一次 PR 清掉。
> **不做**：代码签名/品牌图标/G6（已决策不做）、差量更新/release 瘦身（已决策不做）。

## 状态字段

pending

## 任务目标

### 1. RmShutdown 连带兄弟进程验证 + 实现

- **现状**：task-049 在 confirm 文案里提示「同一控制台内的相关进程可能一并关闭」，但实际 RmShutdown 是否真的关了兄弟进程需验证；
- **Windows Restart Manager 行为**：`RmShutdown` 默认只关**直接锁文件的进程**，不自动关同一控制台的兄弟进程（除非用 `RmRegisterResources` 注册整个进程树）；
- **修法**：
  - **验证**：在 `RmRegisterResources` 时除了注册文件句柄，再注册进程本身（`RmRegisterResources(handle, ..., &[pid], ...)`）；
  - **实现**：如果当前没注册进程本身，补上；这样 RmShutdown 会同时关该进程及其兄弟；
  - **风险**：误杀兄弟进程可能导致用户数据丢失——confirm 文案已经提示了「同一控制台内的相关进程可能一并关闭」，用户已知情；
  - **单测**：mock 两个进程（父子），验证 RmShutdown 后两个都退出。

### 2. RmShutdown 30s 进度反馈

- **现状**：RmShutdown 最长等 30s，期间前端无进度反馈——用户以为卡死；
- **修法**：
  - 后端 `RmShutdown` 改异步（spawn 线程），每 1s emit 一次进度事件（已等 Xs / 最多 30s）；
  - 前端 Items.tsx 占用关闭 confirm 后，显示「正在关闭进程... 已等 Xs」进度条（宽度 = X/30）；
  - 30s 超时后放弃，记 warning，不阻塞删除流程（现状）。

### 3. 跨盘恢复 fallback 文档化

- **现状**：task-056 已补跨盘 rename 测试（CI 单盘 skip），但跨盘 copy+remove fallback 的实际行为没有文档说明；
- **修法**：在 `docs/USER_GUIDE.md` 或 `docs/ARCHITECTURE.md` 补一段「跨盘恢复行为」：
  - 当保险箱目录与原文件不在同一盘时，用 copy+remove fallback；
  - 跨盘恢复比同盘慢（需复制整个文件）；
  - 跨盘恢复失败时文件留在保险箱，不删除原位置。

**本任务不做**：
- 不做代码签名/品牌图标/G6（已决策不做）；
- 不做差量更新/release 瘦身（已决策不做）；
- 不做 RmRestart 白名单 UI（已在 task-058 做）。

## 前置条件

1. `main` 已合并 task-059（merge `360df74`），v0.1.7 已发布；
2. 关键锚点：
   - `engine/src/execute/locking.rs`（RmRegisterResources + RmShutdown 位置）
   - `app/src/pages/Items.tsx`（占用关闭 confirm 调用点 + 进度反馈 UI）
   - `docs/USER_GUIDE.md`（补跨盘恢复说明）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. RmShutdown 连带兄弟进程

1. `RmRegisterResources` 除文件句柄外，注册进程本身（pid）；
2. RmShutdown 后，父子两个进程都退出（单测验证）；
3. confirm 文案不变（task-049 已提示「同一控制台内的相关进程可能一并关闭」）。

### 2. RmShutdown 30s 进度反馈

4. 后端 `RmShutdown` 改异步（spawn 线程），每 1s emit 进度事件；
5. 前端显示「正在关闭进程... 已等 Xs / 最多 30s」进度条；
6. 30s 超时后放弃，不阻塞删除流程。

### 3. 跨盘恢复文档化

7. `docs/USER_GUIDE.md` 或 `docs/ARCHITECTURE.md` 补「跨盘恢复行为」一段；
8. 说明 copy+remove fallback + 跨盘比同盘慢 + 失败时文件留在保险箱。

### 后端单测

9. `cargo test -p engine` 全绿（现有 255+8 + 本任务新增 ≥2 条全绿）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/locking.rs`（RmRegisterResources 注册进程 + RmShutdown 异步）
- `engine/tests/`（连带兄弟进程用例 + 超时用例）
- `app/src/pages/Items.tsx`（进度反馈 UI）
- `docs/USER_GUIDE.md` 或 `docs/ARCHITECTURE.md`（跨盘恢复说明）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-060-shutdown-experience`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-060-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 3 个功能单元拆 3 个原子提交：

1. `fix(execute): RmRegisterResources 注册进程本身，RmShutdown 连带关闭兄弟进程 (#task-060)`（单元 1）
   —— 含 locking.rs + 单测。
2. `feat(execute): RmShutdown 改异步 + 1s 进度事件，前端显示关闭进度条 (#task-060)`（单元 2）
   —— 含 locking.rs + Items.tsx + 单测。
3. `docs(guide): 补跨盘恢复行为说明（copy+remove fallback） (#task-060)`（单元 3）
4. 自验全绿后，最后一次提交：`docs(state): 任务060完成，占用关闭体验优化（连带兄弟进程 + 30s 进度 + 跨盘文档） (#task-060)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 060: ...`；
   - 「最近一次变更」追加一行，含 3 个单元摘要 + commit hash；
   - 「下一步计划」改为「所有已登记功能点已闭环；后续按需迭代」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-29  task-060  占用关闭体验（连带兄弟进程 + 30s 进度 + 跨盘文档）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：连带兄弟进程（父子都退出）、30s 超时（不阻塞删除）。
- **集成测试（必做）**：`cargo test -p engine` 全绿。
- **端到端写路径（本任务不涉及 DB）**：无。
- **真机冒烟（必做、不可裁剪）**：
  - 占用关闭时，前端显示「正在关闭进程... 已等 Xs」进度条（不是卡死）；
  - 关闭同一控制台的父子进程（如 cmd + notepad），两个都退出；
  - 30s 超时后放弃，删除流程继续（不阻塞）；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 3 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（占用关闭进度条 + 兄弟进程行为，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 连带兄弟进程 | 单测输出（父子都退出）+ 真机截图：cmd + notepad 都被关 |
| 30s 进度反馈 | 真机截图：前端显示「已等 Xs / 最多 30s」进度条 |
| 30s 超时不阻塞 | 单测输出（超时后删除继续） |
| 跨盘恢复文档 | 文件内容预览（跨盘行为说明） |
| 不回归 | `cargo test -p engine` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 3 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码/文档提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-060-shutdown-experience`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - RmRegisterResources 最终注册参数；
   - RmShutdown 异步实现结构（spawn 线程 + 1s emit）；
   - 前端进度条 UI 样式；
   - 跨盘恢复文档位置；
   - `cargo test -p engine` 通过输出（含新增 ≥2 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（3 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（所有已登记功能点已闭环；后续按需迭代）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务060-占用关闭体验优化 (#task-060)`，
   该提交只含 `tasks/060-shutdown-experience.md` 一个文件，落在 `feature/task-060-shutdown-experience` 分支最前端。
