# 任务编号：039 safe_box 批量恢复——复用 checkbox + restore_many 后端命令 + 前端批量恢复按钮

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-038 合并后写入）——批量 purge 已就绪，复用同一套 checkbox UI 做批量恢复。
> 上游依赖：task-017（单条 `restore_item`）、task-038（`purge_many` + checkbox 列已就位）。
> **克制范围**：只做批量恢复，**不做**「全部恢复」/「全部 purge」一键操作（留 task-040）。

## 任务目标

task-038 已经把 SafeBox 表格的 checkbox 列 + 全选 + selected Set UI 模式跑通，但只有「批量删除」按钮。本任务复用同一套 UI 加「批量恢复」按钮：

1. **后端**新增 `restore_many(item_ids: Vec<i64>) -> BatchOpResult`：
   ```rust
   pub struct BatchOpResult {
       pub succeeded: usize,
       pub failed: usize,
       pub errors: Vec<String>, // "id=X: <err>"
   }
   ```
   - 循环调现有 `restore_item`，单个失败不中断，收集 errors；
   - 空 Vec 直接返 `{ succeeded: 0, failed: 0, errors: vec![] }`。
2. **前端** SafeBox 顶部操作栏加「批量恢复已选(N)」按钮（绿色，在「批量删除已选」左边）：
   - 点击 → `window.confirm("确定恢复 N 个文件到原路径？")`；
   - confirm 通过 → `invoke('safe_box_restore_many', { item_ids: [...] })`；
   - 结果：绿条「成功 N 条」+（如有失败）红条「失败 M 条：<errors>」；
   - 自动重拉列表。

**本任务不做**：
- 不做「全部恢复」/「全部 purge」一键操作（留 task-040）；
- 不做跨页全选（列表本身无分页）；
- 不做进度条。

## 前置条件

1. `main` 分支已含 task-001~038 全部产出；
2. 关键锚点：
   - `engine/src/execute/safe_box.rs`（task-017 `restore_item`、task-038 `purge_many` + `BatchOpResult` 已存在）；
   - `app/src/pages/SafeBox.tsx`（task-038 已有 checkbox 列 + selected Set + 批量删除按钮）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. 后端：restore_many

1. 在 `engine/src/execute/safe_box.rs` 加：
   ```rust
   pub fn restore_many(
       conn: &rusqlite::Connection,
       item_ids: &[i64],
   ) -> Result<BatchOpResult, String>;
   ```
2. 遍历 `item_ids`，每个调 `restore_item(conn, id)`；
   - 成功 → `succeeded += 1`；
   - 失败（如 ParentMissing）→ `failed += 1; errors.push(format!("id={id}: {err}"))`；
   - 单个失败不中断。
3. Tauri 命令 `safe_box_restore_many` 接到 `commands.rs`，入参 `item_ids: Vec<i64>`，注册到 `lib.rs`。
4. **复用** task-038 的 `BatchOpResult` 结构（如果 task-038 用的是 `PurgeManyResult`，DevAgent 可以抽公共 `BatchOpResult` 或直接用同名结构，备注说明）。

### B. 后端单测

5. 在 `engine/tests/safe_box_restore_many.rs` 覆盖：
   - `restore_many_all_success`：2 个 pending id 全恢复 → succeeded=2, failed=0；
   - `restore_many_partial_fail`：1 个有效 id + 1 个随机 id → succeeded=1, failed=1, errors 含 "id=随机"；
   - `restore_many_empty`：空 Vec → succeeded=0, failed=0。

### C. 前端：SafeBox 加批量恢复按钮

6. `app/src/pages/SafeBox.tsx`：
   - 顶部操作栏（「批量删除已选(N)」左边）加绿色「批量恢复已选(N)」按钮（N=0 时禁用）；
   - 复用现有 `selected: Set<number>`；
   - 点击 → confirm → invoke `safe_box_restore_many`；
   - 结果弹绿/红条，清空 selected，重拉列表。

### D. 卫生

7. 不改单条 restore_item / purge_item 逻辑；
8. 不改 checkbox 列 / 全选逻辑（task-038 已就位）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/safe_box.rs`（加 restore_many）
- `engine/tests/safe_box_restore_many.rs`（新建）
- `app/src-tauri/src/commands.rs`（新增 `safe_box_restore_many`）
- `app/src-tauri/src/lib.rs`（注册命令）
- `app/src/pages/SafeBox.tsx`（加批量恢复按钮）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-039-safebox-batch-restore`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(execute): 新增 restore_many——循环 restore_item 收集成功/失败计数 (#task-039)`
   —— 含 `engine/src/execute/safe_box.rs`。
2. `feat(tauri): 新增 safe_box_restore_many 命令并注册 (#task-039)`
   —— 含 `app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`。
3. `test(execute): 补 restore_many 集成测试（全成功/部分失败/空 Vec） (#task-039)`
   —— 含 `engine/tests/safe_box_restore_many.rs`。
4. `feat(frontend): SafeBox 加「批量恢复已选」绿色按钮 (#task-039)`
   —— 含 `app/src/pages/SafeBox.tsx`。
5. 自验全绿后，最后一次提交：`docs(state): 任务039完成，safe_box 批量恢复就绪 (#task-039)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 039: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-040：dry-run 预览 UI——前端「预览可清理项」按钮，调 cleanup.dryRun 显示大小汇总」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-14  task-039  safe_box.restore_many 批量恢复（复用 task-038 checkbox UI）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 143+ 条 + 本任务 3 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-039-safebox-batch-restore`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - `BatchOpResult` 是复用 task-038 还是新建（备注说明）；
   - `cargo test -p engine` 通过输出；
   - `pnpm build` 输出（无 error）；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（全部恢复/全部 purge 一键、进度条、跨页全选留 task-040+）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务039-safe_box 批量恢复 (#task-039)`，
   该提交只含 `tasks/039-safebox-batch-restore.md` 一个文件，落在 `feature/task-039-safebox-batch-restore` 分支最前端。

## 实现备注（DevAgent 回填 + 评审前清障补记）

### 实际改动文件

| 文件 | 改动 |
|---|---|
| `engine/src/execute/safe_box.rs` | 新增 `BatchOpResult` + `restore_many` |
| `engine/tests/safe_box_restore_many.rs` | 新建，4 条集成测试 |
| `app/src-tauri/src/commands.rs` | 新增 `safe_box_restore_many` 命令 |
| `app/src-tauri/src/lib.rs` | 注册命令 |
| `app/src/pages/SafeBox.tsx` | 加「批量恢复已选(N)」绿色按钮 + 批量状态 state |

### BatchOpResult 是复用还是新建

**新建**（未复用 task-038 的 `PurgeManyResult`）。理由：两者语义字段不同（purge 用 `deleted`、restore 用 `succeeded`），强行合并会让字段名与行为错位。任务书 §A.4 明确允许「抽公共结构或直接用同名结构」二选一，此处取后者并保留各自结构。

### 自验输出

- `cargo check --workspace --all-targets` = 0
- `cargo clippy --workspace --all-targets -- -D warnings` = 0
- `cargo test -p engine` = 0（147 条 passed / 0 failed）
- `cargo fmt --all -- --check` = 0
- `pnpm --filter app lint` = 0
- `pnpm --filter app test` = 0（3 文件 12 断言）
- `pnpm --filter app build` = 0（168.51 kB / gzip 53.43 kB）

### 与任务书的偏差

1. `restore_many` 实际签名为 `(conn, box_dir, item_ids) -> BatchOpResult`，不是任务书 §A.1 写的 `(conn, item_ids) -> Result<BatchOpResult, String>`。对齐既有 `purge_many` / `restore_from_safe_box` 的 `box_dir` 传参约定；批量语义下单个失败不抛硬错，故不返回 `Result`（失败已收集进 `errors`）。
2. 前端「批量恢复」按钮放在「批量删除」左边（任务书 §C.6 要求），且 `N=0` 时禁用。

### 评审前清障补记（2026-10-15）

清障阶段修复 3 项第一档问题并补 1 条测试，详见 `docs/PROJECT_STATE.md` 变更轨迹 `task-039 评审前清障` 行。

### 遗留问题（留 task-040+）

- 「全部恢复」/「全部 purge」一键操作（本任务明确不做）；
- 批量操作进度条（本任务明确不做）；
- 跨页全选（列表本身无分页，本任务明确不做）；
- 真机端到端批量恢复验证（自动化环境无法取得 GUI 数据行，需装包后人工勾选多个文件恢复；已登记已知问题）。
