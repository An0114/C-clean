# 任务编号：031 规则更新 UI——「检查更新」按钮 + 「应用更新」按钮（rules_checkUpdate / rules_applyUpdate）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFrontendReact
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-030 合并后写入）——M1 业务模块 E（规则更新）的前端收尾。
> 上游依赖：task-022（`rules_check_update` Tauri 命令已注册，返回 `{ has_update, current_version, remote_version, current_sha256, remote_sha256 }`）、task-023（`rules_apply_update` Tauri 命令已注册，返回 `{ new_version, new_sha256 }`）、task-029/030（前端 tab 模式参考）。
> **闭环意义**：用户能在前端看到「有新版本规则」并一键应用——M1-E 规则更新前后端打通。

## 任务目标

后端 rules.checkUpdate / applyUpdate 早已就绪，但前端没入口。本任务补最小 UI：

1. 新建 `app/src/pages/Settings.tsx`：
   - 顶部「规则库」卡片：
     - 显示当前版本（启动时调一次 `rules_check_update` 拿 current_version）；
     - 「检查更新」按钮 → invoke `rules_check_update` → 显示当前版本 / 远端版本 / 是否有更新；
     - 有更新时显示「应用更新」按钮（绿色高亮）→ invoke `rules_apply_update` → 成功显示新版本号 + 绿 toast；
     - 应用中按钮禁用 + 「应用中…」文案；
     - 网络失败红条显示错误；
2. App.tsx 加第五个 tab「设置」。

**本任务不做**：
- 不做手动回滚（从 .bak 恢复，后端没命令，留 task-032）；
- 不做 GitHub Token / 私有仓配置 UI；
- 不做失败重试 / 离线降级；
- 不做差量更新提示；
- 不引 UI 库。

## 前置条件

1. `main` 分支已含 task-001~030 全部产出；
2. 关键锚点：
   - `app/src/pages/SafeBox.tsx`（task-030 模式参考）；
   - `app/src-tauri/src/commands.rs`——`rules_check_update` / `rules_apply_update` 已注册。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. Settings 页面

1. 新建 `app/src/pages/Settings.tsx`：
   - state：
     ```ts
     interface CheckUpdateResp {
       has_update: boolean;
       current_version: string;
       remote_version: string;
       current_sha256: string;
       remote_sha256: string;
     }
     type ActionStatus = "idle" | "checking" | "applying" | "error" | "updated";
     ```
   - 启动 `invoke<CheckUpdateResp>('rules_check_update', {})` 拉一次，显示当前版本；
   - 「检查更新」按钮：
     - 点击 → status='checking'，按钮禁用；
     - invoke rules_check_update → 显示当前版本 / 远端版本 / 「已是最新」或「有新版本 X 可用」；
   - 「应用更新」按钮（仅当 has_update=true 时显示）：
     - 点击 → status='applying'，按钮禁用 + 「应用中…」；
     - invoke rules_apply_update → 成功：绿 toast「已更新到版本 X」+ 重拉当前版本；
     - 失败：红条显示错误（如 SHA 不匹配）。

### B. App.tsx 第五 tab

2. `app/src/App.tsx`：tab 加「设置」，渲染 `<Settings />`。

### C. 卫生

3. 不修改 Rust 代码；
4. 不新增 npm 依赖；
5. 不做回滚 UI。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `app/src/pages/Settings.tsx`（新建）
- `app/src/App.tsx`（加第五 tab）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-031-rules-settings-ui`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(frontend): 新增 Settings 页面——规则库卡片 + 启动拉当前版本 (#task-031)`
   —— 含 `app/src/pages/Settings.tsx`。
2. `feat(frontend): 接「检查更新」按钮 + rules_check_update 显示远端版本对比 (#task-031)`
   —— 含 `app/src/pages/Settings.tsx`。
3. `feat(frontend): 接「应用更新」按钮 + rules_apply_update 成功/失败提示 (#task-031)`
   —— 含 `app/src/pages/Settings.tsx`。
4. `feat(frontend): App.tsx 加「设置」tab 切换 (#task-031)`
   —— 含 `app/src/App.tsx`。
5. 自验全绿后，最后一次提交：`docs(state): 任务031完成，规则更新 UI 就绪 (#task-031)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 031: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「M1 业务模块 A~F 前端 UI 全部接完；下一轮进入：NSIS 打包 / ESLint+Vitest 接入 / safe_box purge 命令 / 端到端真机验收」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-13  task-031  Settings 规则库卡片 + 检查更新/应用更新按钮，M1-E 前后端打通`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0（后端无改动）；
- `pnpm --filter app build` 退出码 0；
- DevAgent 本地 `tauri dev`：切「设置」tab → 点「检查更新」看到当前/远端版本 → （如有更新）点「应用更新」看到新版本号。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个 feat 提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-031-rules-settings-ui`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - `pnpm --filter app build` 输出（无 error）；
   - `tauri dev` 真机截图或文字描述；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（手动回滚 / Token / 重试 / 差量 / NSIS 打包留 task-032+）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务031-规则更新 UI (#task-031)`，
   该提交只含 `tasks/031-rules-settings-ui.md` 一个文件，落在 `feature/task-031-rules-settings-ui` 分支最前端。
