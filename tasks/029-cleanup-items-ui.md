# 任务编号：029 清理闭环——cleanup.listPending 后端命令 + 前端 Items 列表（checkbox + 一键清理按钮）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-028 合并后写入）。
> 上游依赖：task-015（cleanup.execute 命令已注册，入参 `item_ids: Vec<i64>`）、task-014（cleanup_item 表结构、DbItemPathResolver）、task-028（scan_start 按钮已就位）、task-024/025（audit_log.list/stats 模式参考）。
> **闭环意义**：从「能扫描 + 能看进度」升级为「能看扫描结果 + 能勾选 + 能真正删除 + 数字自动刷新」。

## 任务目标

目前 scan.start 跑完后，DB 里写了 cleanup_item 行，但前端**看不到**、也**不能挑着删**。本任务补最小闭环：

1. **后端**新增 `cleanup.listPending` Tauri 命令：
   ```rust
   #[tauri::command]
   async fn cleanup_list_pending(
       session_id: Option<i64>,
       limit: Option<i64>,
       offset: Option<i64>,
       app: tauri::AppHandle,
   ) -> Result<Envelope<Vec<PendingItem>>, String>;
   // PendingItem { id, path, size_bytes, module_code, session_id, scanned_at }
   // SQL: SELECT ... FROM cleanup_item WHERE status='pending' [AND session_id=?]
   //      ORDER BY size_bytes DESC LIMIT ? OFFSET ?
   ```
2. **前端**新建 `app/src/pages/Items.tsx`：
   - 顶部「刷新」按钮；
   - 表格 5 列：checkbox / 路径 / 大小（formatBytes）/ 模块 / 时间；
   - 顶部「已选 N 项，共 X MB」+ 「清理已选」按钮（未选时禁用）；
   - 点击「清理已选」→ `invoke('cleanup_execute', { item_ids: [...] })`；
   - 完成后自动刷新列表 + stats（Dashboard）+ history；
3. **App.tsx** 加第三个 tab「清理」。

**本任务不做**：
- 不做 dry-run 预览（task-014 已有后端命令，UI 留 task-030）；
- 不做 safe_box restore UI（留 task-031）；
- 不做 rules 更新 UI；
- 不做取消清理（后端无 cancel）；
- 不引 UI 库。

## 前置条件

1. `main` 分支已含 task-001~028 全部产出；
2. 关键锚点：
   - `engine/src/db/migrations/v1__init.sql`——`cleanup_item` 表字段（id / path / size_bytes / module_code / session_id / status / scanned_at）；
   - `engine/src/audit_log/query.rs`（task-024）——只读查询模式参考；
   - `app/src-tauri/src/commands.rs`——cleanup_execute 已注册；
   - `app/src/pages/Dashboard.tsx` / `History.tsx`（task-026/027 模式）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. 后端：`cleanup_list_pending`

1. 新建 `engine/src/execute/list_pending.rs`：
   ```rust
   #[derive(serde::Serialize, Debug, Clone)]
   pub struct PendingItem {
       pub id: i64,
       pub path: String,
       pub size_bytes: i64,
       pub module_code: String,
       pub session_id: i64,
       pub scanned_at: i64,
   }
   pub fn list_pending_items(
       conn: &rusqlite::Connection,
       session_id: Option<i64>,
       limit: i64,
       offset: i64,
   ) -> Result<Vec<PendingItem>, String>;
   ```
2. SQL 按 session_id 可选过滤、`status='pending'`、`ORDER BY size_bytes DESC`、参数化。
3. Tauri 命令 `cleanup_list_pending` 接到 `app/src-tauri/src/commands.rs`，入参全可选（session_id?/limit?/offset?，默认 limit=100），注册到 `lib.rs`。

### B. 后端单测（`engine/tests/list_pending_items.rs`）

4. 至少覆盖：
   - `list_pending_filters_status_pending`：INSERT 2 pending + 1 deleted → list 只返回 2；
   - `list_pending_filters_by_session`：session=1 pending 1 行、session=2 pending 1 行 → 按 session_id=1 过滤只返回 1 行；
   - `list_pending_orders_by_size_desc`：3 行 size 100/300/200 → 返回顺序 300/200/100。

### C. 前端：Items 页面

5. 新建 `app/src/pages/Items.tsx`：
   - 启动 `invoke('cleanup_list_pending', { limit: 100 })`；
   - state：`{ rows: PendingItem[], selected: Set<number>, cleaning: boolean, msg?: string }`；
   - 表格列：checkbox / 路径 / 大小（formatBytes）/ 模块 / 时间（formatTs）；
   - 底部操作栏：「已选 N 项，共 X MB」+ 「清理已选」按钮（selected 空时 disabled）+ 「刷新」按钮；
   - 清理中按钮禁用 + 文案「清理中…」；
   - 清理完成：弹红色错误条（如有）或绿色成功条（「已清理 N 项，释放 X MB」），自动重拉列表。

### D. App.tsx 第三 tab

6. `app/src/App.tsx`：tab 加「清理」，渲染 `<Items />`。

### E. 卫生

7. 不修改 `engine/src/execute/executor.rs` / `fs_deleter.rs` / `safe_box.rs` 已有逻辑；
8. 不接 cleanup.dryRun UI；
9. 不新增 npm 依赖。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/mod.rs`（加 `pub mod list_pending;`）
- `engine/src/execute/list_pending.rs`（新建）
- `engine/tests/list_pending_items.rs`（新建）
- `app/src-tauri/src/commands.rs`（新增 `cleanup_list_pending`）
- `app/src-tauri/src/lib.rs`（注册命令）
- `app/src/pages/Items.tsx`（新建）
- `app/src/App.tsx`（加第三 tab）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-029-cleanup-items-ui`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(execute): 新增 list_pending_items——按 session 过滤 pending 项，size 倒序 (#task-029)`
   —— 含 `engine/src/execute/mod.rs`、`engine/src/execute/list_pending.rs`。
2. `feat(tauri): 新增 cleanup.listPending 命令并注册 (#task-029)`
   —— 含 `app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`。
3. `test(execute): 补 list_pending_items 集成测试（status/session/size排序） (#task-029)`
   —— 含 `engine/tests/list_pending_items.rs`。
4. `feat(frontend): 新增 Items 页面——pending 表格 + checkbox + 清理已选按钮 (#task-029)`
   —— 含 `app/src/pages/Items.tsx`。
5. `feat(frontend): App.tsx 加「清理」tab 切换 (#task-029)`
   —— 含 `app/src/App.tsx`。
6. 自验全绿后，最后一次提交：`docs(state): 任务029完成，清理闭环打通 (#task-029)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 029: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「M1 业务闭环（扫描 → 看结果 → 勾选 → 删除 → 看数字）已打通；下一轮进入 safe_box UI（恢复列表 + 恢复按钮）或 rules 更新 UI」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-13  task-029  cleanup.listPending + Items 表格 + checkbox 清理按钮，M1 业务闭环`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 133 条 + 本任务 3 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0；
- DevAgent 本地 `tauri dev`：扫一次 → 切「清理」tab 看到 pending 列表 → 勾几项 → 点「清理已选」→ 回到 Dashboard 看到数字涨上去。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 5 个代码提交 + 1 个 `docs(state)` 提交，共 6 个 commit，全部 push 到 `feature/task-029-cleanup-items-ui`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - `cargo test -p engine` 通过输出；
   - `pnpm --filter app build` 输出；
   - `tauri dev` 真机端到端流程文字描述（扫描 → 勾选 → 删除 → 数字上涨）；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（dry-run 预览 UI、safe_box restore UI、rules 更新 UI 留 task-030+）；
3. `docs/PROJECT_STATE.md` 已按第 6 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务029-清理闭环Items UI (#task-029)`，
   该提交只含 `tasks/029-cleanup-items-ui.md` 一个文件，落在 `feature/task-029-cleanup-items-ui` 分支最前端。

## 实现备注（DevAgent 落地后回填）
- **改动文件**：
  - 后端：新建 `engine/src/execute/list_pending.rs`、`engine/tests/list_pending_items.rs`；`engine/src/execute/mod.rs` 加 `pub mod list_pending;`；`app/src-tauri/src/commands.rs` 加 `cleanup_list_pending`；`app/src-tauri/src/lib.rs` 注册。
  - 前端：新建 `app/src/pages/Items.tsx`；`app/src/App.tsx` 加第三 tab「清理」。
- **PendingItem shape**（serde rename_all=camelCase，前端消费）：
  ```ts
  interface PendingItem {
    id: number; path: string; sizeBytes: number; moduleCode: string; sessionId: number;
  }
  ```
- **与任务书偏差（重要）**：
  1. `cleanup_item` 表**无 status 列**（真实列为 id/session_id/module_code/category/path/size_bytes/risk_level/...），「pending」= 表内所有行，无 `WHERE status='pending'` 过滤。
  2. 表**无 scanned_at 列**，PendingItem 去掉该字段；前端 Items 表格不显示时间列，改显示 `sessionId`（#N）。
  3. cleanup.execute 不删 cleanup_item 行（task-015 边界，executor 不允许改）；清理后行仍在 DB（历史快照），Items 页刷新后仍会列出——这是已知限制，后续可加 `WHERE id NOT IN (SELECT item_id FROM audit_log WHERE action='delete')` 或加 status 列。
- **自验命令与退出码**：cargo check=0 / clippy -D warnings=0（首次报 manual_clamp，已改 `limit.clamp(1,500)` 修复）/ cargo test -p engine=0（133+4=137 条全绿，含 list_pending 4/4）/ cargo fmt --check=0 / pnpm --filter app build=0。
- **提交粒度**：按任务书 5 个代码 commit 推进（execute 功能 / tauri 命令 / 测试 / Items 页 / App tab），外加 1 个 clippy style 修复 commit。
- **遗留**：cleanup_item 清理后不删行的问题、dry-run 预览 UI（task-014 已有后端）、safe_box restore UI、rules 更新 UI；tauri dev 真机「扫一次→勾选→清理→Dashboard 数字涨」端到端未在自动化环境跑（GUI 截图留评审手动复核）。
