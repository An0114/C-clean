# 任务编号：038 safe_box 批量 purge——多选 checkbox + 一键永久删除（后端 batch 命令 + 前端批量按钮）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-037 合并后写入）——单条 purge 已就绪，下一步批量操作。
> 上游依赖：task-037（`safe_box_purge` 单条命令已注册）、task-030（SafeBox 表格已就位）。
> **克制范围**：只做批量 purge，**不做**批量恢复（留 task-039）、**不做**批量 FIFO 自动清理。

## 任务目标

目前 SafeBox 表格只能逐行点「永久删除」，文件多时效率低。本任务补批量 purge：

1. **后端**新增 `safe_box_purge_many(item_ids: Vec<i64>) -> PurgeManyResult`：
   ```rust
   pub struct PurgeManyResult {
       pub deleted: usize,      // 成功删除的条数
       pub failed: usize,      // 失败的条数
       pub errors: Vec<String>, // 失败原因（id: err）
   }
   ```
   - 循环调现有 `purge_item`，单个失败不中断，收集到 errors；
   - 空 Vec 直接返 `PurgeManyResult { deleted: 0, failed: 0, errors: vec![] }`。
2. **前端** SafeBox 表格：
   - 加 checkbox 列（每行一个 checkbox + 表头全选 checkbox）；
   - 顶部操作栏加「批量删除已选(N)」红色按钮（N=0 时禁用）；
   - 点击 → `window.confirm("确定永久删除 N 个文件？此操作不可恢复。")`；
   - confirm 通过 → `invoke('safe_box_purge_many', { item_ids: [...] })`；
   - 结果弹绿条「成功 N 条」+（如有失败）红条「失败 M 条：<errors>」；
   - 自动重拉列表。

**本任务不做**：
- 不做批量恢复（留 task-039）；
- 不做进度条（批量 purge 很快，不需要）；
- 不做「全选当前页」以外的全选跨页（列表本身无分页，task-018 已知问题）。

## 前置条件

1. `main` 分支已含 task-001~037 全部产出；
2. 关键锚点：
   - `engine/src/execute/safe_box.rs`（task-037 已有 `purge_item`）；
   - `app/src/pages/SafeBox.tsx`（task-030/037 已有表格 + 单条按钮）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. 后端：safe_box_purge_many

1. 在 `engine/src/execute/safe_box.rs` 加：
   ```rust
   pub fn purge_many(
       conn: &rusqlite::Connection,
       box_dir: &Path,
       item_ids: &[i64],
   ) -> Result<PurgeManyResult, String>;
   ```
2. 内部：
   - 遍历 `item_ids`，每个调 `purge_item(conn, box_dir, id)`；
   - 成功 → `deleted += 1`；
   - 失败 → `failed += 1; errors.push(format!("id={id}: {err}"))`；
   - 单个失败不中断整体循环。
3. Tauri 命令 `safe_box_purge_many` 接到 `commands.rs`，入参 `item_ids: Vec<i64>`，注册到 `lib.rs`。

### B. 后端单测

4. 在 `engine/tests/safe_box_purge_many.rs` 覆盖：
   - `purge_many_all_success`：3 个 pending id 全 purge → deleted=3, failed=0；
   - `purge_many_partial_fail`：2 个有效 id + 1 个随机 id → deleted=2, failed=1, errors 含 "id=随机: ..."；
   - `purge_many_empty`：空 Vec → deleted=0, failed=0。

### C. 前端：SafeBox 加 checkbox + 批量按钮

5. `app/src/pages/SafeBox.tsx`：
   - state 加 `selected: Set<number>`；
   - 表格首列加 checkbox；
   - 表头 checkbox：当前全选时 checked、否则 unchecked；点击切换全选/全不选；
   - 顶部操作栏（汇总行右侧）加「批量删除已选(N)」红色按钮（N=0 禁用）；
   - 点击 → confirm → invoke `safe_box_purge_many`；
   - 结果弹绿条/红条，清空 selected，重拉列表。

### D. 卫生

6. 不改单条 purge_item 逻辑；
7. 不做批量恢复。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/safe_box.rs`（加 purge_many + PurgeManyResult）
- `engine/tests/safe_box_purge_many.rs`（新建）
- `app/src-tauri/src/commands.rs`（新增 `safe_box_purge_many`）
- `app/src-tauri/src/lib.rs`（注册命令）
- `app/src/pages/SafeBox.tsx`（加 checkbox + 批量按钮）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-038-safebox-batch-purge`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(execute): 新增 purge_many——循环 purge_item 收集成功/失败计数 (#task-038)`
   —— 含 `engine/src/execute/safe_box.rs`。
2. `feat(tauri): 新增 safe_box_purge_many 命令并注册 (#task-038)`
   —— 含 `app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`。
3. `test(execute): 补 purge_many 集成测试（全成功/部分失败/空 Vec） (#task-038)`
   —— 含 `engine/tests/safe_box_purge_many.rs`。
4. `feat(frontend): SafeBox 加 checkbox 列 + 批量删除已选按钮 (#task-038)`
   —— 含 `app/src/pages/SafeBox.tsx`。
5. 自验全绿后，最后一次提交：`docs(state): 任务038完成，safe_box 批量 purge 就绪 (#task-038)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 038: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-039：safe_box 批量恢复（复用 checkbox + 批量 restore_many 命令）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-14  task-038  safe_box.purge_many 批量永久删除（后端 batch + 前端 checkbox 列）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 140+ 条 + 本任务 3 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-038-safebox-batch-purge`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - `cargo test -p engine` 通过输出；
   - `pnpm build` 输出（无 error）；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（批量恢复、进度条、跨页全选留 task-039+）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务038-safe_box 批量 purge (#task-038)`，
   该提交只含 `tasks/038-safebox-batch-purge.md` 一个文件，落在 `feature/task-038-safebox-batch-purge` 分支最前端。
