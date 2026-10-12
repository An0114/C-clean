# 任务编号：018 safe_box.list——列出当前保险箱未剔除条目（前端 UI 用）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRust
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-017 合并后由 DocsKeeper 写入）——M1 业务模块 C（保险箱）的第三个任务。
> 上游依赖：task-016（`safe_box` 表写入 + FIFO）、task-017（`restore_from_safe_box`）均已合并到 main。

## 任务目标

保险箱双向操作（入 / 回）已经闭环，但用户在 UI 上**看不到保险箱里有什么**。本任务补一个只读 list 命令：

1. engine 层写 `list_safe_box(conn) -> Vec<SafeBoxEntry>`：
   - `SELECT id, original_path, size_bytes, moved_at FROM safe_box WHERE purged_at IS NULL ORDER BY moved_at DESC`；
2. Tauri 命令 `safe_box.list`：无入参，返回条目数组 + 汇总（条目数、总字节）；
3. 单测：tempdir 移入 3 个文件 → list 返回 3 条 → restore 一条 → list 剩 2 条。

**本任务不做**：
- 不做分页（保险箱总量 ≤5GB，条目数有限；真多了再加 LIMIT）；
- 不做按 session 过滤（先返回全部，简单）；
- 不写前端 UI（列表组件留后续 React 任务）；
- 不做「永久删除某条」命令（purge 单条留后续）。

## 前置条件

1. `main` 分支已含 task-001~017 全部产出；
2. 关键锚点：
   - `engine/src/db/migrations/v1__init.sql`——`safe_box` 表字段；
   - `engine/src/execute/safe_box.rs`（task-016/017）——SafeBoxDeleter、restore 模式参考。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. Engine 层：`list_safe_box`

1. 在 `engine/src/execute/safe_box.rs` 追加：
   ```rust
   #[derive(serde::Serialize, Debug, Clone)]
   pub struct SafeBoxEntry {
       pub id: i64,
       pub original_path: String,
       pub size_bytes: i64,
       pub moved_at: i64,
   }
   #[derive(serde::Serialize, Debug, Clone)]
   pub struct SafeBoxListResp {
       pub entries: Vec<SafeBoxEntry>,
       pub total_items: i64,
       pub total_bytes: i64,
   }
   pub fn list_safe_box(conn: &rusqlite::Connection) -> Result<SafeBoxListResp, String>;
   ```
2. 内部：
   - 单条 prepared statement：`SELECT id, original_path, size_bytes, moved_at FROM safe_box WHERE purged_at IS NULL ORDER BY moved_at DESC`；
   - 累计 `total_items` / `total_bytes`；
   - DB 错误 → `Err(String)`。

### B. Tauri 命令 `safe_box.list`

3. 在 `app/src-tauri/src/commands.rs` 新增：
   ```rust
   #[tauri::command]
   async fn safe_box_list(app: tauri::AppHandle) -> Result<Envelope<SafeBoxListResp>, String>;
   ```
4. 内部：打开 DB → 调 `list_safe_box(&conn)` → 返回 `Envelope::ok(resp)`；DB 错误 → `Envelope::err(INTERNAL)`。
5. 在 `lib.rs` 的 `generate_handler!` 注册 `safe_box_list`。

### C. 单测（`engine/tests/safe_box_list.rs`）

6. 至少覆盖：
   - `list_empty_returns_zero`：空 DB → `entries=[]、total_items=0、total_bytes=0`；
   - `list_returns_inserted_entries`：移入 3 个文件 → list 返回 3 条，按 moved_at DESC 排序，total_bytes = 三者之和；
   - `list_excludes_purged`：移入后用 SQL 把一行 `purged_at=unixepoch()` → list 只剩 2 条；
   - `list_excludes_restored`：移入一条后调 `restore_from_safe_box` → list 里该条目消失。

### D. 卫生与边界

7. 不修改 `engine/src/execute/safe_box.rs` 的 SafeBoxDeleter / enforce_quota / restore 已有逻辑（只追加 list 函数）；
8. 不修改 schema；
9. 不修改 `app/src/**`（前端 React）；
10. 不新增依赖。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/safe_box.rs`（追加 list 函数 + 响应结构）
- `engine/tests/safe_box_list.rs`（新建）
- `app/src-tauri/src/commands.rs`（新增 `safe_box_list` 命令）
- `app/src-tauri/src/lib.rs`（注册命令）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-018-safe-box-list`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(execute): 新增 list_safe_box——查未剔除条目 + 汇总 (#task-018)`
   —— 含 `engine/src/execute/safe_box.rs`。
2. `feat(tauri): 新增 safe_box.list 命令并注册 (#task-018)`
   —— 含 `app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`。
3. `test(execute): 补 list 集成测试（空/多条/排除 purged/排除 restored） (#task-018)`
   —— 含 `engine/tests/safe_box_list.rs`。
4. 自验全绿后，最后一次提交：`docs(state): 任务018完成，safe_box.list 就绪 (#task-018)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 018: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「M1-C 保险箱三件套（入/回/列）齐备，下一轮派发 M1-D：Restart Manager 处理文件占用删除失败」；
   - 「已知问题」段补：list 无分页、无 session 过滤（保险箱总量小可接受）；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-12  task-018  safe_box.list 命令就绪，前端 UI 可列出保险箱条目`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 104 条 + 本任务 4 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-018-safe-box-list`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 自验命令与退出码；
   - `cargo test -p engine` 通过输出；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（分页、session 过滤、单条 purge、前端 UI 列表留后续）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务018-safe_box-list (#task-018)`，
   该提交只含 `tasks/018-safe-box-list.md` 一个文件，落在 `feature/task-018-safe-box-list` 分支最前端。
