# 任务编号：057 RmRestart 自动重启被关进程——后端 Restart Manager 调用 + 设置页开关（默认关闭）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRustReact + DevFrontendReact
> 来源：PROJECT_STATE「下一步计划 5」；产品决策 2026-10-28「做，但默认关闭，设置页加开关」。
> 上游依赖：task-049 占用关闭 confirm 已合并（merge `801af22`）；task-056 restored_at 迁移已合并（merge `55ff644`）。
> **本任务范围（5 个功能单元）**：后端 Restart Manager 重启 API + 设置持久化 + Settings 开关 UI + confirm 文案联动 + 单测。
> **关键产品约束**：**默认关闭**——新用户装完看不到任何自动重启行为；用户必须主动在设置页打开开关才生效。
> **不做**：差量更新（评估不立项）、release profile 瘦身（用户决策不做）、G6 修复（豁免口径）、RmShutdown 行为变更（保持现状）。

## 状态字段

pending

## 任务目标

### 背景

当前 task-049 占用关闭流程：用户点「关闭占用进程并重试」→ confirm 弹窗 → 调 `RmShutdown` 关闭进程 → 重试删除。**关闭后进程不会自动重启**——浏览器、编辑器等用户常用的程序被关后用户得手动重开，体验差。

Windows Restart Manager API 本身支持在 `RmShutdown` 后调 `RmRestart` 自动重启（前提是进程注册时声明了可重启，且有 `RmStartSession` 的重启标志位）。

### 1. 后端：RmRestart 调用封装

- **现状**：`engine/src` 里已有 RmShutdown 调用（task-015 引入 Restart Manager 会话），但只调 `RmShutdown`，没调 `RmRestart`；
- **修法**：
  - 在 Restart Manager 会话结束前（`RmShutdown` 之后），根据配置开关决定是否调 `RmRestart`；
  - 调用前检查：`RmGetList` 返回的进程是否带 `RM_PROCESS_INFO::bRestartable` 标志——**只有标记为可重启的进程才尝试重启**，不可重启的（如系统服务）跳过；
  - 重启失败不阻塞删除流程（记 warning 日志，不影响用户已完成的清理）；
  - 超时控制：重启尝试最多 10s，超时放弃。

### 2. 设置持久化：用户偏好

- **现状**：无任何用户偏好存储；
- **修法**：
  - 新建 `app/src-tauri/src/prefs.rs`（或复用现有 db 加 `user_prefs` 表），存 `auto_restart_on_close: bool`（默认 false）；
  - Tauri 命令：`get_prefs() -> Prefs`、`set_prefs(prefs: Prefs)`；
  - 前端启动时读 prefs，写入 React state + localStorage 缓存（避免每次都 IPC）；
  - **默认值 false**——新用户首次启动读不到 prefs 时返回 false。

### 3. Settings 页开关 UI

- **现状**：Settings 页只有「检查更新」按钮；
- **修法**：
  - 新增一行「自动重启被关闭的进程」开关（checkbox / toggle）；
  - 下方灰色说明文字：「开启后，清理时被关闭的浏览器/编辑器等进程会自动重启；系统进程不受影响。默认关闭。」；
  - 切换开关立即调 `set_prefs` 持久化；
  - 关闭时 confirm 文案**不变化**（保持现状）；开启时 confirm 文案追加「将自动重启被关闭的进程」。

### 4. confirm 文案联动

- **现状**：task-049 confirm 文案是固定的「同一控制台内的相关进程可能一并关闭」；
- **修法**：
  - 调占用关闭命令前读当前 prefs；
  - 若 `auto_restart_on_close == true`，confirm 文案追加一句：「已开启自动重启，被关闭的进程将尝试自动恢复。」；
  - 若 false，文案不变。

### 5. 后端单测

- **现状**：Restart Manager 集成测试在 CI windows-latest 上跑（task-015 已有）；
- **修法**：
  - 新增 `prefs_test.rs`：默认 false、set/get 往返、未知 key 不破坏现有值；
  - RmRestart 路径**在 CI 上不真跑**（会启动真实进程，CI 环境不适合）——只测「bRestartable 标志判断逻辑」和「超时控制」，真实重启由用户真机冒烟验证。

**本任务不做**：
- 不做差量更新（评估不立项）；
- 不做 release profile 瘦身（用户决策不做）；
- 不做 G6 修复（豁免口径）；
- 不改 RmShutdown 行为（保持现状）；
- 不做「重启白名单/黑名单」（先全量可重启进程都试，后续看反馈再加）。

## 前置条件

1. `main` 已合并 task-056（merge `55ff644`，restored_at 迁移 v2 已就位）；
2. 关键锚点：
   - `engine/src/execute/` 下 Restart Manager 会话代码（RmStartSession / RmShutdown 位置）——DevAgent 用 `rg "RmShutdown" engine/` 定位；
   - `app/src-tauri/src/commands.rs`（新增 prefs 命令）；
   - `app/src/pages/Settings.tsx`（加开关 UI）；
   - `app/src/pages/Items.tsx` 占用关闭 confirm 调用点（文案联动）；
   - 迁移器 v3（如果用 DB 存 prefs）或 localStorage（如果纯前端缓存）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 后端 RmRestart 调用

1. Restart Manager 会话在 `RmShutdown` 后，若 `auto_restart_on_close == true`，调 `RmRestart`；
2. 只对 `bRestartable == true` 的进程尝试重启；
3. 重启超时 10s，超时放弃不阻塞；
4. 重启失败记 warning 日志，不影响删除流程。

### 2. 设置持久化

5. 新建 prefs 存储（DB 或文件，DevAgent 选最简方案）；
6. `get_prefs` / `set_prefs` 两个 Tauri 命令；
7. 默认 `auto_restart_on_close = false`（新用户首次读返回 false）。

### 3. Settings 开关 UI

8. Settings 页新增「自动重启被关闭的进程」toggle；
9. 下方灰色说明文字；
10. 切换立即 `set_prefs` 持久化；
11. 启动时读 prefs 回填开关状态。

### 4. confirm 文案联动

12. 开启时 confirm 文案含「将自动重启被关闭的进程」；
13. 关闭时文案不变（task-049 现状）。

### 5. 单测

14. `prefs_test.rs`：默认 false、set/get 往返、未知 key 不破坏现有值；
15. RmRestart 路径单测：bRestartable 标志判断逻辑（true 才尝试）；
16. `cargo test -p engine` 全绿（现有 255+8 + 本任务新增 ≥3 条）；
17. `pnpm test` 全绿（现有 81 + 本任务新增 ≥2 条：开关切换、文案联动）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/` 下 Restart Manager 相关文件（加 RmRestart 调用）
- `engine/src/prefs.rs`（新建，或并入 db 模块）
- `engine/tests/prefs_test.rs`（新建）
- `app/src-tauri/src/commands.rs`（加 prefs 命令）
- `app/src/pages/Settings.tsx`（加开关 UI）
- `app/src/pages/Items.tsx`（confirm 文案联动）
- `app/src/App.tsx`（启动时读 prefs，可选）
- 对应测试文件（`Settings.test.tsx` / `Items.test.tsx`）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-057-rm-restart-toggle`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-057-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 5 个功能单元拆 5 个原子提交：

1. `feat(prefs): 新增用户偏好存储（auto_restart_on_close 默认 false）+ get/set 命令 (#task-057)`（单元 2）
   —— 含 prefs 模块 + commands + 单测。
2. `feat(execute): RmShutdown 后按 prefs 调 RmRestart（仅 bRestartable 进程，超时 10s） (#task-057)`（单元 1）
   —— 含 engine Restart Manager 改造 + 单测。
3. `feat(settings): Settings 页加「自动重启被关闭的进程」开关 + 说明文字 (#task-057)`（单元 3）
   —— 含 Settings.tsx + 测试。
4. `fix(ui): 占用关闭 confirm 文案根据 prefs 联动（开启时提示自动重启） (#task-057)`（单元 4）
   —— 含 Items.tsx + 测试。
5. 自验全绿后，最后一次提交：`docs(state): 任务057完成，RmRestart 自动重启（默认关闭）+ 设置开关就绪 (#task-057)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 057: ...`；
   - 「最近一次变更」追加一行，含 5 个单元摘要 + commit hash；
   - 「下一步计划」改为「M3/M4 均已闭环；后续按需迭代（代码签名 / 品牌图标 / Restart Manager 白名单）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-28  task-057  RmRestart 自动重启被关进程（默认关闭 + 设置开关）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：prefs 默认 false、set/get 往返、bRestartable 标志判断。
- **集成测试（必做）**：`cargo test -p engine` + `pnpm test` 全绿。
- **端到端写路径（必做，本任务硬要求）**：prefs 持久化必须有一条真实「切换开关 → 查 prefs 存储非默认值 → 重开应用开关仍为开启」证据——不能只构造内存 state。
- **真机冒烟（必做、不可裁剪）**：
  - 默认关闭：新装机 → Settings 页开关为 off → 占用关闭 confirm 文案**不含**自动重启提示；
  - 开启：打开开关 → 占用关闭 confirm 文案**含**「将自动重启」→ 真实关闭一个可重启进程（如 notepad）→ 删除完成后 notepad 自动重启；
  - 关闭后重开应用：开关状态保留（prefs 持久化）；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 4 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（Settings 开关 + confirm 文案 + 真实进程重启，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| prefs 默认 false | 单测输出 + 真机截图：新装机 Settings 开关 off |
| prefs set/get 往返 | 单测输出 |
| RmRestart 仅 bRestartable | 单测输出（标志判断逻辑） |
| Settings 开关 UI | 真机截图：开关 + 说明文字 |
| confirm 文案联动 | 真机截图：开启时文案含自动重启提示 |
| 端到端 prefs 持久化 | 真机证据：切换开关 → 重开应用 → 开关仍为 on（DevTools 查 prefs 存储非默认值） |
| 真机进程重启 | 真机截图：notepad 被关后自动重启（或任务管理器进程列表证据） |
| 不回归 | `cargo test -p engine` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 4 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-057-rm-restart-toggle`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - prefs 存储方案最终选择（DB 表 vs 文件 vs localStorage）；
   - RmRestart 调用最终代码结构（bRestartable 判断 + 超时）；
   - Settings 开关最终 UI 样式（toggle 还是 checkbox）；
   - confirm 文案最终中文表述；
   - `cargo test -p engine` 通过输出（含新增 ≥3 条用例）；
   - `pnpm test` 通过输出（含新增 ≥2 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（4 条截图/日志路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（RmRestart 白名单/黑名单后续按需迭代；代码签名/品牌图标仍不做）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务057-RmRestart 自动重启与设置开关 (#task-057)`，
   该提交只含 `tasks/057-rm-restart-toggle.md` 一个文件，落在 `feature/task-057-rm-restart-toggle` 分支最前端。
