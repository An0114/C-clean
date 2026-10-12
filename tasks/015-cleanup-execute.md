# 任务编号：015 cleanup.execute 真实删除——FsDeleter + delete::execute 唯一入口 + audit_log 落库

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevTauriBridge
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-014 合并后由 DocsKeeper 写入）——M1 业务模块 B（删除）的第一个任务。
> 上游依赖：task-003（`engine::execute::delete::execute()` 唯一入口 + L3 白名单 + Deleter trait + NoopDeleter 测试实现）、task-014（cleanup.dryRun 已走真实 DB）、task-002（`cleanup_task` / `audit_log` 表 schema）均已合并到 main。

## 任务目标

dryRun 闭环（task-014）已经能告诉用户「哪些能删、哪些被 L3 拦」，但**真实删除**这一步还没接：task-003 的 `delete::execute()` 唯一入口已经存在，但只在单测里被 NoopDeleter 调过。本任务落地真实删除：

1. 在 engine 层写 `FsDeleter`（实现 `engine::execute::delete::Deleter` trait，调 `std::fs::remove_file`）；
2. 新建 Tauri 命令 `cleanup.execute`：
   - 入参 `item_ids: Vec<i64>`；
   - 从 DB 查每个 id 对应的 path；
   - 对每个 path **再过一次** `delete::execute(path, &FsDeleter, &external_deny)`（不能因为 dryRun 通过就跳白名单——防 dryRun 到 execute 之间路径被改/junction 被换）；
   - 删除成功的写一行 `cleanup_task` + `audit_log`；
3. 单测：tempdir 造真实文件，跑 FsDeleter 删，断言文件没了 + DB 行写入；
4. 端到端：DevAgent 手动验证时**只删 tempdir 里的测试文件**，禁止删真实系统文件。

**本任务不做**：
- 不做保险箱（safe_box 表 / FIFO 5GB——M1-C 留后续）；
- 不做 Restart Manager（文件占用时无法删——M1-D 留后续）；
- 不做批量回滚/批量进度（每个 item 单独处理，不发进度事件）；
- 不写前端 React 组件（不做「确认删除」UI）；
- 不删目录（只删文件；目录留后续）；
- 不接 elevation 提权（普通用户权限可删 %TEMP%、浏览器缓存；需要提权的目录留 elevation crate 落地后）。

## 前置条件

1. `main` 分支已含 task-001~014 全部产出；
2. 关键锚点：
   - `engine/src/execute/delete.rs`（task-003）——`Deleter` trait、`delete::execute()` 唯一入口；
   - `engine/src/db/migrations/v1__init.sql`（task-002）——`cleanup_task`（id / session_id / item_id / action / status / bytes_freed / executed_at）、`audit_log`（id / session_id / event_type / path / detail / created_at）表；
   - `app/src-tauri/src/commands.rs`——已有 `scan_start`、`cleanup_dry_run` 命令注册模式。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. Engine 层：`FsDeleter`

1. 新建 `engine/src/execute/fs_deleter.rs`，在 `mod.rs` 加 `pub mod fs_deleter;`。
2. 定义：
   ```rust
   pub struct FsDeleter;
   impl crate::execute::delete::Deleter for FsDeleter {
       fn delete_file(&self, path: &str) -> Result<u64, String> {
           // 1. 先取文件大小（bytes_freed 写 DB 用）：std::fs::metadata(path)?.len()
           // 2. 再 std::fs::remove_file(path)?
           // 3. 返回 Ok(bytes)
       }
   }
   ```
3. **错误处理**：
   - 文件不存在（`io::ErrorKind::NotFound`）→ 返回 `Ok(0)`（幂等，已删过就算成功）；
   - 权限不足（`PermissionDenied`）→ 返回 `Err(format!("permission denied: {}", path))`；
   - 文件占用（Windows 下 ` SharingViolation`）→ 返回 `Err(...)`；
   - 其他 IO 错误 → 返回 `Err(...)`。

### B. Engine 层：删除编排函数

4. 新建 `engine/src/execute/executor.rs`（或在 delete.rs 里加函数，DevAgent 选并备注）：
   ```rust
   pub struct ExecResult {
       pub item_id: i64,
       pub path: String,
       pub deleted: bool,
       pub bytes_freed: u64,
       pub error: Option<String>,
   }
   pub fn execute_items(
       item_ids: &[i64],
       conn: &rusqlite::Connection,
       external_deny: &[String],
       deleter: &dyn Deleter,
   ) -> Vec<ExecResult>;
   ```
5. 内部流程对每个 item_id：
   - `SELECT path FROM cleanup_item WHERE id = ?`；查不到 → `deleted=false, error="not found"`；
   - 调 `crate::execute::delete::execute(path, deleter, external_deny)`——**这一步再过白名单**；
   - 白名单拒（2002）→ `deleted=false, error="blocked by L3"`；
   - deleter 失败 → `deleted=false, error=err`；
   - 成功 → `deleted=true, bytes_freed=...`，写：
     - `INSERT INTO cleanup_task (session_id, item_id, action, status, bytes_freed, executed_at) VALUES (?, ?, 'delete', 'success', ?, unixepoch())`；
     - `INSERT INTO audit_log (session_id, event_type, path, detail, created_at) VALUES (?, 'delete_success', ?, ?, unixepoch())`；
   - 失败也写 audit_log：`event_type='delete_failed', detail=err`。

### C. Tauri 命令 `cleanup.execute`

6. 在 `app/src-tauri/src/commands.rs` 新增：
   ```rust
   #[tauri::command]
   async fn cleanup_execute(
       item_ids: Vec<i64>,
       app: tauri::AppHandle,
   ) -> Result<Envelope<CleanupExecuteResp>, String>;
   ```
7. 内部：
   - 打开 DB（复用 task-013 的 data_dir）；
   - 加载 external_deny（复用 task-010 逻辑）；
   - 调 `engine::execute::executor::execute_items(&item_ids, &conn, &external_deny, &FsDeleter)`；
   - 汇总：`deleted_count`、`bytes_freed_total`、`failed_count`；
   - 返回 `Envelope::ok(CleanupExecuteResp { results, summary })`。
8. 在 `lib.rs` 的 `tauri::generate_handler!` 里注册 `cleanup_execute`。

### D. 单测（`engine/tests/execute_items.rs`）

9. 至少覆盖：
   - `execute_items_deletes_real_files`：
     - tempdir 造 DB + 跑迁移 + 建 scan_session；
     - 写 3 个真实文件 `a.tmp / b.tmp / c.tmp` 到 tempdir，各 100 字节；
     - INSERT 3 行 cleanup_item（path 指向这 3 个文件）；
     - 调 `execute_items(&[1,2,3], &conn, &[], &FsDeleter)`；
     - 断言 3 个文件都没了；
     - 断言 cleanup_task 表 3 行 success；
     - 断言 audit_log 表 3 行 delete_success；
   - `execute_items_blocked_by_l3`：
     - 一条 cleanup_item 的 path = `C:\Users\alice\Desktop\secret.docx`（命中内置 L3）；
     - 调 execute_items → `deleted=false, error="blocked by L3"`；
     - cleanup_task 表不写 success、audit_log 写一条 delete_failed；
   - `execute_items_skips_missing_id`：
     - item_ids = [1, 999]，999 不存在 → result 里第二条 `deleted=false, error="not found"`，不影响第一条；
   - `execute_items_idempotent_on_already_deleted`：
     - 先删一次，再删一次 → 第二次 `Ok(0)`（NotFound 幂等），audit_log 仍写 success。

### E. 端到端验证（DevAgent 手动，**严格沙箱**）

10. DevAgent **禁止**用真实系统路径端到端验证。改为：
    - 在 tempdir 手动造 3 个 .tmp 文件；
    - 用 sqlite3 命令行直接 INSERT cleanup_item 行（不跑 ps1 扫描）；
    - devtools console：`await invoke('cleanup_execute', { itemIds: [1,2,3] })`；
    - 验证：tempdir 里 3 个文件消失、DB 里 cleanup_task/audit_log 有行；
11. DevAgent 把 console 输出贴进实现备注。

### F. 卫生与边界

12. 不修改 `engine/src/execute/delete.rs` 已有签名（只在 executor.rs 里调它）；
13. 不修改 `engine/src/db/**`、`engine/src/ipc/**`、`engine/src/scan/**`、`engine/src/rules/**`；
14. 不修改 `app/src/**`（前端 React）；
15. 不做目录删除、不做保险箱、不做 Restart Manager、不做 elevation；
16. 不新增依赖（`std::fs` 够用）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/fs_deleter.rs`（新建）
- `engine/src/execute/executor.rs`（新建）
- `engine/src/execute/mod.rs`（仅加 `pub mod fs_deleter; pub mod executor;`）
- `engine/tests/execute_items.rs`（新建）
- `app/src-tauri/src/commands.rs`（新增 `cleanup_execute` 命令）
- `app/src-tauri/src/lib.rs`（注册命令到 generate_handler）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-015-cleanup-execute`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(execute): 新增 FsDeleter——std::fs::remove_file 真实删除实现 Deleter trait (#task-015)`
   —— 含 `engine/src/execute/fs_deleter.rs`、`engine/src/execute/mod.rs`。
2. `feat(execute): 新增 execute_items 编排——过白名单、删文件、写 cleanup_task/audit_log (#task-015)`
   —— 含 `engine/src/execute/executor.rs`。
3. `feat(tauri): 新增 cleanup.execute 命令，注册到 generate_handler (#task-015)`
   —— 含 `app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`。
4. `test(execute): 补 execute_items 集成测试（删真文件/L3拦/缺id/幂等） (#task-015)`
   —— 含 `engine/tests/execute_items.rs`。
5. 自验全绿后，最后一次提交：`docs(state): 任务015完成，真实删除闭环 (#task-015)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 015: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-016：safe_box 保险箱——删除前先移到 5GB FIFO 目录，覆盖则按时间最旧剔除」；
   - 「已知问题」段补：文件占用时删除失败（SharingViolation），留 Restart Manager 任务；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-12  task-015  cleanup.execute 真实删除（FsDeleter + delete::execute + audit_log）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 92 条 + 本任务 4 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0；
- DevAgent 端到端验证（§E）在 tempdir 沙箱内通过。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-015-cleanup-execute`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - FsDeleter 对 Windows SharingViolation / PermissionDenied 的错误文案；
   - 端到端验证 tempdir 路径（DevAgent 自己本机的临时目录，不提交）；
   - console cleanup_execute 输出；
   - DB 查询结果（cleanup_task / audit_log 行数）；
   - 自验命令与退出码；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（保险箱、Restart Manager、目录删除、进度事件、提权留后续）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务015-cleanup.execute真实删除 (#task-015)`，
   该提交只含 `tasks/015-cleanup-execute.md` 一个文件，落在 `feature/task-015-cleanup-execute` 分支最前端。
