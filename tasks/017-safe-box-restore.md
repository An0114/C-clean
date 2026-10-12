# 任务编号：017 safe_box restore——从保险箱按 id 恢复文件到原路径

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRust
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-016 合并后由 DocsKeeper 写入）——M1 业务模块 C（保险箱）的第二个任务。
> 上游依赖：task-016（`SafeBoxDeleter` + `safe_box` 表写入 + 5GB FIFO 已落地）已合并到 main。

## 任务目标

task-016 已经把「删除」升级成「移入保险箱」，但用户后悔时还**取不回来**。本任务落地反向操作：

1. 在 engine 层写 `restore_from_safe_box(conn, safe_box_id) -> Result<RestoreResult, RestoreError>`：
   - 从 `safe_box` 表按 id 查行（`purged_at IS NULL` 才能恢复）；
   - 检查 `original_path` 是否已被新文件占用——是则报错（**禁止静默覆盖**）；
   - `rename(box_path → original_path)`；
   - 删除 safe_box 表行（文件已回到原位置，audit_log 留痕）；
   - 写 `audit_log(event_type='restore', path=original_path)`；
2. 新增 Tauri 命令 `safe_box.restore`：入参 `safe_box_id: i64`，返回恢复结果；
3. 单测：tempdir 移入 → restore → 断言原文件回来、box 里没了、DB 行没了。

**本任务不做**：
- 不做「列出保险箱所有条目」命令（留后续 task，前端 UI 用）；
- 不做「批量恢复」；
- 不做「原目录已删时自动 mkdir」——直接报错让用户手动处理；
- 不改 task-016 的 SafeBoxDeleter / enforce_quota；
- 不加 `restored_at` 列（当前 schema 没有，DELETE 行 + audit_log 已足够审计）。

## 前置条件

1. `main` 分支已含 task-001~016 全部产出；
2. 关键锚点：
   - `engine/src/db/migrations/v1__init.sql`——`safe_box` 表字段（id/session_id/item_id/original_path/box_path/size_bytes/moved_at/purged_at）；
   - `engine/src/execute/safe_box.rs`（task-016）——SafeBoxDeleter 写入模式；
   - `engine/src/db/migrations/v1__init.sql`——`audit_log` 表。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. Engine 层：`restore_from_safe_box`

1. 在 `engine/src/execute/safe_box.rs` 追加：
   ```rust
   #[derive(Debug)]
   pub struct RestoreResult {
       pub safe_box_id: i64,
       pub original_path: String,
       pub size_bytes: u64,
   }
   #[derive(thiserror::Error, Debug)]
   pub enum RestoreError {
       #[error("保险箱条目不存在或已被永久剔除: id={0}")]
       NotFound(i64),
       #[error("原路径已被占用，拒绝覆盖: {0}")]
       TargetOccupied(String),
       #[error("原父目录不存在，请先手动创建: {0}")]
       ParentMissing(String),
       #[error("IO 错误: {0}")]
       Io(String),
       #[error("DB 错误: {0}")]
       Db(String),
   }
   pub fn restore_from_safe_box(
       conn: &rusqlite::Connection,
       safe_box_id: i64,
   ) -> Result<RestoreResult, RestoreError>;
   ```
2. 内部流程：
   - `SELECT original_path, box_path, size_bytes, session_id FROM safe_box WHERE id=? AND purged_at IS NULL`；无行 → `NotFound`；
   - 检查 `original_path`：`std::fs::metadata(original_path)` 能成功 → `TargetOccupied`；
   - 检查 `original_path` 父目录存在：`parent().exists()`？否则 → `ParentMissing`；
   - `std::fs::rename(box_path, original_path)?`；
   - `DELETE FROM safe_box WHERE id=?`；
   - `INSERT INTO audit_log (session_id, event_type, path, detail, created_at) VALUES (?, 'restore', ?, ?, unixepoch())`；
   - 返回 `RestoreResult`。

### B. Tauri 命令 `safe_box.restore`

3. 在 `app/src-tauri/src/commands.rs` 新增：
   ```rust
   #[tauri::command]
   async fn safe_box_restore(
       safe_box_id: i64,
       app: tauri::AppHandle,
   ) -> Result<Envelope<RestoreResp>, String>;
   ```
4. 内部：打开 DB → 调 `restore_from_safe_box(&conn, safe_box_id)`：
   - 成功 → `Envelope::ok(RestoreResp { ... })`；
   - `NotFound` / `TargetOccupied` / `ParentMissing` → 返回业务错误 code（用 TRD §5.3 已有的错误码，DevAgent 选最接近的并备注；不新增错误码）；
   - `Io` / `Db` → `Envelope::err(INTERNAL)`。
5. 在 `lib.rs` 的 `generate_handler!` 注册 `safe_box_restore`。

### C. 单测（`engine/tests/safe_box_restore.rs`）

6. 至少覆盖：
   - `restore_returns_file_to_original_path`：
     - tempdir 造 DB + 跑迁移 + scan_session；
     - 写一个真实文件 `a.tmp`（100 字节）；
     - 先用 `SafeBoxDeleter` 把它移入保险箱（拿到 safe_box_id）；
     - 断言：原文件没了、box_dir 里有新文件；
     - 调 `restore_from_safe_box(&conn, safe_box_id)`；
     - 断言：原文件回来了、box_dir 里没了、safe_box 表没了该行、audit_log 表有 1 行 restore；
   - `restore_target_occupied_returns_error`：
     - 移入后，在 original_path 写一个新文件；
     - 调 restore → `TargetOccupied`；
     - 断言 box_dir 文件没被动、safe_box 表行还在；
   - `restore_parent_missing_returns_error`：
     - 把 original_path 设成一个不存在的子目录 `C:\nonexistent\foo\a.tmp`；
     - 调 restore → `ParentMissing`；
   - `restore_purged_row_returns_not_found`：
     - 直接在 DB 里把一行 `purged_at = unixepoch()`；
     - 调 restore → `NotFound`。

### D. 卫生与边界

7. 不修改 `engine/src/execute/safe_box.rs` 的 SafeBoxDeleter / enforce_quota 已有逻辑（只追加 restore 函数）；
8. 不修改 schema（不加 restored_at 列）；
9. 不修改 `engine/src/db/**`、`engine/src/scan/**`、`engine/src/ipc/**`；
10. 不修改 `app/src/**`（前端 React）；
11. 不新增依赖。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/safe_box.rs`（追加 restore 函数 + 错误类型）
- `engine/tests/safe_box_restore.rs`（新建）
- `app/src-tauri/src/commands.rs`（新增 `safe_box_restore` 命令）
- `app/src-tauri/src/lib.rs`（注册命令）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-017-safe-box-restore`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(execute): 新增 restore_from_safe_box——按 id 把文件 rename 回原路径 (#task-017)`
   —— 含 `engine/src/execute/safe_box.rs`。
2. `feat(tauri): 新增 safe_box.restore 命令并注册 (#task-017)`
   —— 含 `app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`。
3. `test(execute): 补 restore 集成测试（正常/目标占用/父目录缺失/已剔除） (#task-017)`
   —— 含 `engine/tests/safe_box_restore.rs`。
4. 自验全绿后，最后一次提交：`docs(state): 任务017完成，保险箱 restore 闭环 (#task-017)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 017: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-018：safe_box.list 命令——列出当前保险箱里所有未剔除条目（前端 UI 用）」；
   - 「已知问题」段补：restore 不自动建父目录、不批量、不加 restored_at 列；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-12  task-017  safe_box restore 闭环，用户后悔可恢复文件`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 99 条 + 本任务 4 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-017-safe-box-restore`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 业务错误码映射（NotFound/TargetOccupied/ParentMissing 各用了 TRD 哪个 code）；
   - 自验命令与退出码；
   - `cargo test -p engine` 通过输出；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（list 命令、批量恢复、自动建父目录、restored_at 审计字段留后续）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务017-safe_box-restore (#task-017)`，
   该提交只含 `tasks/017-safe-box-restore.md` 一个文件，落在 `feature/task-017-safe-box-restore` 分支最前端。

## 实现备注（DevAgent 落地后回填）
- **改动文件**：`engine/src/execute/safe_box.rs`（追加 RestoreResult/RestoreError/restore_from_safe_box）、`engine/tests/safe_box_restore.rs`（新建 5 用例）、`app/src-tauri/src/commands.rs`（新增 safe_box_restore 命令）、`app/src-tauri/src/lib.rs`（注册）、`docs/PROJECT_STATE.md`。
- **schema 偏差**（按 v1__init.sql 真实列适配）：
  - 查询条件：`status='in_box'`（任务书写 `purged_at IS NULL`）；
  - 路径字段：`safe_rel_path`（相对 box_dir，任务书写 `box_path` 绝对路径）——故函数签名加了 `box_dir: &Path` 参数；
  - audit_log 字段：`ts/actor/action/target/detail/result`（任务书写 `event_type/path/created_at`），映射 action='restore'、target=original_path、actor='user'、result='success'、detail=JSON。
- **业务错误码映射**（不新增错误码）：
  - `TargetOccupied` → 2003 FILE_OCCUPIED（最接近：目标已被占用）；
  - `NotFound` / `ParentMissing` / `Io` / `Db` → 5001 INTERNAL（前端 toast 兜底，详情 eprintln）。
- **自验命令与退出码**：cargo check=0 / clippy -D warnings=0 / cargo test -p engine=0（含 restore 5/5，累计 104 条） / cargo fmt --check=0 / pnpm --filter app build=0。
- **遗留**：safe_box.list（前端 UI 用）、批量恢复、原父目录自动 mkdir、restored_at 审计列——留 task-018+。
