# 任务编号：016 safe_box 保险箱——删除前移入 5GB FIFO 目录，超容按时间最旧剔除

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRust
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-015 合并后由 DocsKeeper 写入）——M1 业务模块 C（保险箱）的第一个任务。
> 上游依赖：task-002（`safe_box` 表 schema：id/session_id/item_id/original_path/box_path/size_bytes/moved_at/purged_at）、task-015（`FsDeleter` + `execute_items` 编排已落地）均已合并到 main。
> 立项决策 #4：「保险箱 5GB FIFO」——删除前先移入保险箱，用户后悔可恢复；超容时按时间最旧永久剔除。

## 任务目标

task-015 的 FsDeleter 是直接 `remove_file`——删错了就找不回来。本任务落地**保险箱**：

1. 新建 `SafeBoxDeleter` 实现 `engine::execute::delete::Deleter` trait：
   - 不直接删，而是 `std::fs::rename(original, box_path)` 把文件移到 `<data_dir>/safe_box/`；
   - 文件名格式：`<unix_ts>_<original_filename>`（避免同名冲突）；
2. 移入后写 `safe_box` 表（original_path / box_path / size_bytes / moved_at）；
3. 移入前先调 `enforce_quota()`：算 safe_box 目录当前总大小，超 5GB 上限时按 `moved_at ASC` 永久删除最旧的文件，直到回到 5GB 以下；
4. Tauri `cleanup.execute` 命令切换：从 FsDeleter 换成 SafeBoxDeleter（**所有删除操作都走保险箱**）；
5. 单测用 tempdir + 小容量上限（比如 100 字节，通过常量/参数注入）验证 FIFO 剔除逻辑。

**本任务不做**：
- 不做「从保险箱恢复」命令（restore 留后续 task）；
- 不做保险箱 UI 列表（前端 task 留后续）；
- 不做「保险箱自动清空 30 天」TTL（留后续）；
- 不改 task-015 的 FsDeleter 代码（保留，作为「永久删除」的底层原语）；
- 不做原子事务（rename 失败、safe_box 表写入失败的回滚策略——先简单失败即报错，留后续优化）。

## 前置条件

1. `main` 分支已含 task-001~015 全部产出；
2. 关键锚点：
   - `engine/src/db/migrations/v1__init.sql`——`safe_box` 表字段；
   - `engine/src/execute/fs_deleter.rs`（task-015）——FsDeleter 实现模式参考；
   - `engine/src/execute/executor.rs`（task-015）——`execute_items` 当前用 FsDeleter；
   - 立项决策记录：保险箱 5GB FIFO。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. Engine 层：`SafeBoxDeleter`

1. 新建 `engine/src/execute/safe_box.rs`，在 `mod.rs` 加 `pub mod safe_box;`。
2. 常量：
   ```rust
   /// 保险箱容量上限（字节）。5 GB。
   pub const SAFE_BOX_QUOTA_BYTES: u64 = 5 * 1024 * 1024 * 1024;
   ```
3. 结构：
   ```rust
   pub struct SafeBoxDeleter<'conn> {
       box_dir: std::path::PathBuf,
       conn: &'conn rusqlite::Connection,
       session_id: i64,
       item_id: i64,
   }
   impl<'conn> SafeBoxDeleter<'conn> {
       pub fn new(box_dir: &std::path::Path, conn: &'conn rusqlite::Connection, session_id: i64, item_id: i64) -> Self;
       /// 移入前先调：超 5GB 时按 moved_at 最旧永久删除
       pub fn enforce_quota(&self) -> Result<u64, String>;
   }
   ```
4. `Deleter` trait 实现：
   ```rust
   impl<'conn> crate::execute::delete::Deleter for SafeBoxDeleter<'conn> {
       fn delete_file(&self, path: &str) -> Result<u64, String> {
           // 1. 算文件大小：std::fs::metadata(path)?.len()
           // 2. 先调 self.enforce_quota() 确保保险箱有空间
           // 3. 构造 box_path = box_dir.join(format!("{}_{}", unix_ts, Path::new(path).file_name()?))
           // 4. std::fs::rename(path, &box_path)  —— 跨盘时 rename 会失败，此时 fallback 到 copy+remove，DevAgent 自选并备注
           // 5. INSERT INTO safe_box (session_id, item_id, original_path, box_path, size_bytes, moved_at) VALUES (...)
           // 6. 返回 Ok(size)
       }
   }
   ```

### B. `enforce_quota()` 逻辑

5. 内部：
   - `SELECT COALESCE(SUM(size_bytes), 0) FROM safe_box WHERE purged_at IS NULL`；
   - 若 `sum + new_file_size <= SAFE_BOX_QUOTA_BYTES` → 直接返回当前总大小；
   - 否则：
     - `SELECT id, box_path, size_bytes FROM safe_box WHERE purged_at IS NULL ORDER BY moved_at ASC`；
     - 逐条：`std::fs::remove_file(box_path)`，成功后 `UPDATE safe_box SET purged_at=unixepoch() WHERE id=?`；
     - 直到剩余空间足够放新文件；
   - 返回剔除后剩余总大小。

### C. Tauri 命令切换

6. 在 `app/src-tauri/src/commands.rs` 的 `cleanup_execute` 里：
   - 把原来传 `&FsDeleter` 的地方，换成循环构造 `SafeBoxDeleter::new(box_dir, &conn, session_id, item_id)`；
   - 注意：task-015 的 `execute_items` 签名是 `deleter: &dyn Deleter`，现在要每个 item 用不同的 deleter（因为 SafeBoxDeleter 持有 item_id / session_id）——所以本任务**要改 execute_items 签名**，从「传一个 deleter」改成「传一个闭包/工厂 `Fn(item_id) -> Box<dyn Deleter>`」；
   - `box_dir = data_dir.join("safe_box")`，不存在则 `std::fs::create_dir_all`。

### D. 单测（`engine/tests/safe_box.rs`）

7. 至少覆盖：
   - `safe_box_moves_file_and_writes_row`：
     - tempdir 造 DB + 跑迁移 + scan_session；
     - 写 1 个真实文件 `a.tmp`（100 字节）；
     - 调 `SafeBoxDeleter::new(box_dir, &conn, 1, 1).delete_file(".../a.tmp")`；
     - 断言：原文件没了、box_dir 里有新文件、safe_box 表 1 行、返回 size=100；
   - `safe_box_quota_evicts_oldest`：
     - 用**测试常量**（DevAgent 加 `#[cfg(test)]` 覆盖 `SAFE_BOX_QUOTA_BYTES = 300`）；
     - 先移入 3 个文件各 100 字节（moved_at 模拟：用 SQL 直接塞不同的 moved_at 值）；
     - 再移入第 4 个 100 字节文件 → 触发 quota；
     - 断言：最早的那个文件在 box_dir 里没了、safe_box 表里最早的行 `purged_at` 非空；
   - `safe_box_rename_across_disk_fallback`：
     - 如果 DevAgent 选了 copy+remove fallback，测一条跨盘用例（CI 是单盘，可跳过；DevAgent 本地备注）；
   - `safe_box_enforce_quota_noop_when_under_quota`：
     - 保险箱空，移入 100 字节，不触发任何剔除。

### E. 卫生与边界

8. 不修改 `engine/src/execute/fs_deleter.rs`（保留 FsDeleter 作为永久删除原语）；
9. 不修改 `engine/src/execute/delete.rs` 已有 delete::execute 签名；
10. 不修改 `engine/src/db/**` 已有 schema（safe_box 表已就绪）；
11. 不实现 restore 命令、不做 UI、不做 TTL；
12. 不新增依赖（std::fs 够用）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/safe_box.rs`（新建）
- `engine/src/execute/mod.rs`（加 `pub mod safe_box;`）
- `engine/src/execute/executor.rs`（改签名：从单 deleter 改成 deleter 工厂闭包）
- `engine/tests/safe_box.rs`（新建）
- `app/src-tauri/src/commands.rs`（cleanup_execute 改用 SafeBoxDeleter）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-016-safe-box`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(execute): 新增 SafeBoxDeleter——删除前移入保险箱目录并写 safe_box 表 (#task-016)`
   —— 含 `engine/src/execute/safe_box.rs`、`engine/src/execute/mod.rs`。
2. `feat(execute): 实现 enforce_quota——超 5GB 按 moved_at 最旧永久剔除 (#task-016)`
   —— 在 `engine/src/execute/safe_box.rs` 追加。
3. `refactor(execute): execute_items 改签名——deleter 工厂闭包以支持 per-item SafeBoxDeleter (#task-016)`
   —— 含 `engine/src/execute/executor.rs`。
4. `feat(tauri): cleanup_execute 切换到 SafeBoxDeleter，所有删除走保险箱 (#task-016)`
   —— 含 `app/src-tauri/src/commands.rs`。
5. `test(execute): 补 safe_box 移入/quota 剔除/空 noop 集成测试 (#task-016)`
   —— 含 `engine/tests/safe_box.rs`。
6. 自验全绿后，最后一次提交：`docs(state): 任务016完成，5GB FIFO 保险箱落地 (#task-016)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 016: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-017：restore 命令——从保险箱按 id 恢复文件到原路径」；
   - 「已知问题」段补：跨盘 rename fallback 策略、原子性（rename 成功但 DB 写失败）留后续；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-12  task-016  safe_box 5GB FIFO 保险箱落地，删除不再直接 remove_file`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 96 条 + 本任务 4 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 5 个代码提交 + 1 个 `docs(state)` 提交，共 6 个 commit，全部 push 到 `feature/task-016-safe-box`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 跨盘 rename 处理策略（rename 失败时是否做 copy+remove）；
   - quota 测试常量如何覆盖（`#[cfg(test)]` 还是运行时参数）；
   - 自验命令与退出码；
   - `cargo test -p engine` 通过输出；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（restore、TTL、原子性、跨盘留后续）；
3. `docs/PROJECT_STATE.md` 已按第 6 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务016-safe-box保险箱 (#task-016)`，
   该提交只含 `tasks/016-safe-box.md` 一个文件，落在 `feature/task-016-safe-box` 分支最前端。
