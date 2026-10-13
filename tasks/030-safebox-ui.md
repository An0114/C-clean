# 任务编号：030 保险箱 UI——safe_box.list 接前端 + 单条「恢复」按钮

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFrontendReact
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-029 合并后写入）——M1 业务闭环打通后的下一刀。
> 上游依赖：task-018（`safe_box.list` Tauri 命令已注册，返回条目数组）、task-017（`safe_box_restore` Tauri 命令已注册，入参 `item_id: i64`）、task-029（Items 表格 + tab 模式参考）。
> **产品价值**：用户删错了能从保险箱恢复——M1 安全网闭环。

## 任务目标

后端 safe_box 已经能 list / restore，但前端没入口。本任务补最小 UI：

1. 新建 `app/src/pages/SafeBox.tsx`：
   - 启动 `invoke('safe_box_list')`；
   - 表格 4 列：路径 / 大小（formatBytes）/ 移入时间（formatTs）/ 操作（「恢复」按钮）；
   - 顶部「刷新」按钮 + 「共 N 项 / 共 X MB」汇总；
   - 点击「恢复」→ `invoke('safe_box_restore', { item_id })` → 成功后自动重拉列表 + 弹绿色成功条；
   - 恢复失败（如原路径父目录已不存在）→ 红条显示错误，不阻塞其他行；
2. App.tsx 加第四个 tab「保险箱」。

**本任务不做**：
- 不做单条 purge（永久删除）（后端没命令，留 task-031）；
- 不做批量恢复（后端没批量命令，留 task-031）；
- 不做 quota 进度条（5GB FIFO 已在后端 enforce，UI 展示留 task-031）；
- 不接 dry-run 预览（留 task-031）；
- 不引 UI 库。

## 前置条件

1. `main` 分支已含 task-001~029 全部产出；
2. 关键锚点：
   - `app/src/pages/Items.tsx`（task-029 模式参考）；
   - `app/src-tauri/src/commands.rs`——`safe_box_list` / `safe_box_restore` 已注册；
   - `app/src/utils/formatBytes.ts` / `formatTimestamp.ts`（task-026/027）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. SafeBox 页面

1. 新建 `app/src/pages/SafeBox.tsx`：
   - state：
     ```ts
     interface SafeBoxItem { id: number; path: string; size_bytes: number; moved_at: number; original_path: string; }
     type Status = "loading" | "ok" | "error";
     ```
   - `load()`：`invoke<SafeBoxItem[]>('safe_box_list')`；
   - 表格：
     | 路径（original_path） | 大小 | 移入时间 | 操作 |
   - 顶部汇总：`共 N 项 / 总大小 formatBytes(sum)`；
   - 「恢复」按钮：
     - 点击 → 该行按钮变「恢复中…」禁用；
     - `invoke('safe_box_restore', { item_id: id })`；
     - 成功：绿色 toast「已恢复：original_path」，重拉列表；
     - 失败：红条「恢复失败：<err>」，按钮恢复可点；
   - 空列表：居中灰色「保险箱为空」。

### B. App.tsx 第四 tab

2. `app/src/App.tsx`：tab 加「保险箱」，渲染 `<SafeBox />`。

### C. 卫生

3. 不修改 Rust 代码（命令已注册）；
4. 不新增 npm 依赖；
5. 不接 purge / 批量 / quota UI。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `app/src/pages/SafeBox.tsx`（新建）
- `app/src/App.tsx`（加第四 tab）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-030-safebox-ui`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(frontend): 新增 SafeBox 页面——safe_box_list 表格 + 汇总 (#task-030)`
   —— 含 `app/src/pages/SafeBox.tsx`。
2. `feat(frontend): SafeBox 行加「恢复」按钮 + safe_box_restore 接线 + 成功/失败提示 (#task-030)`
   —— 含 `app/src/pages/SafeBox.tsx`。
3. `feat(frontend): App.tsx 加「保险箱」tab 切换 (#task-030)`
   —— 含 `app/src/App.tsx`。
4. 自验全绿后，最后一次提交：`docs(state): 任务030完成，保险箱恢复 UI 就绪 (#task-030)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 030: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-031：rules 更新 UI——前端「检查更新」按钮 + 「应用更新」按钮，调 rules_checkUpdate / rules_applyUpdate」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-13  task-030  SafeBox 表格 + 单条恢复按钮，M1 安全网闭环`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0（后端无改动）；
- `pnpm --filter app build` 退出码 0；
- DevAgent 本地 `tauri dev`：删一个文件 → 切「保险箱」tab 看到该行 → 点「恢复」→ 文件回到原路径。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个 feat 提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-030-safebox-ui`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - SafeBoxItem 真实字段（贴 shape）；
   - `pnpm --filter app build` 输出（无 error）；
   - `tauri dev` 真机端到端流程文字描述；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（purge / 批量恢复 / quota UI / rules 更新 UI 留 task-031+）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务030-保险箱恢复 UI (#task-030)`，
   该提交只含 `tasks/030-safebox-ui.md` 一个文件，落在 `feature/task-030-safebox-ui` 分支最前端。
