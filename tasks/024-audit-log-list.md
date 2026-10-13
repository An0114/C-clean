# 任务编号：024 audit_log.list——查询清理历史（按 session 过滤、分页、时间倒序）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRust
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-023 合并后由 DocsKeeper 写入）——M1 业务模块 F（报表）的第一个任务。
> 上游依赖：task-002（`audit_log` 表 schema：id/session_id/event_type/path/detail/created_at）、task-015/016/017/021（audit_log 已有写入点）均已合并到 main。

## 任务目标

目前所有清理动作都写了 audit_log（delete_success / delete_failed / restore / file in use 等），但**查不出来**。本任务补只读查询命令，作为 M1-F 报表/历史页的后端基础：

1. engine 层 `audit_log/query.rs`：
   - `AuditLogEntry { id, session_id, event_type, path, detail, created_at }`；
   - `list_audit_log(conn, filter: AuditLogFilter) -> Vec<AuditLogEntry>`；
   - filter 支持：`session_id: Option<i64>`、`limit: i64`（默认 100）、`offset: i64`（默认 0）；
   - SQL：`ORDER BY created_at DESC, id DESC LIMIT ? OFFSET ?`；
2. Tauri 命令 `audit_log.list`：入参 `{ session_id?: number, limit?: number, offset?: number }`，返回条目数组；
3. 单测：写 5 行 audit_log → list 全部 → list 按 session 过滤 → list 分页。

**本任务不做**：
- 不做统计聚合（total_bytes / 按模块汇总——留 task-025）；
- 不做前端 UI（历史页 React 组件留后续）；
- 不做导出 CSV/JSON；
- 不做删除/清空 audit_log。

## 前置条件

1. `main` 分支已含 task-001~023 全部产出；
2. 关键锚点：
   - `engine/src/db/migrations/v1__init.sql`——`audit_log` 表字段；
   - `engine/src/db/` 已有 conn.rs / migrate.rs / resolver.rs 模式参考。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. Engine 层：`audit_log/query.rs`

1. 新建 `engine/src/audit_log/mod.rs` 与 `engine/src/audit_log/query.rs`，在 `engine/src/lib.rs` 加 `pub mod audit_log;`。
2. 定义：
   ```rust
   #[derive(serde::Serialize, Debug, Clone)]
   pub struct AuditLogEntry {
       pub id: i64,
       pub session_id: i64,
       pub event_type: String,
       pub path: String,
       pub detail: Option<String>,
       pub created_at: i64,
   }
   #[derive(Debug, Default)]
   pub struct AuditLogFilter {
       pub session_id: Option<i64>,
       pub limit: i64,   // 默认 100
       pub offset: i64,  // 默认 0
   }
   pub fn list_audit_log(
       conn: &rusqlite::Connection,
       filter: &AuditLogFilter,
   ) -> Result<Vec<AuditLogEntry>, String>;
   ```
3. SQL：
   - 有 session_id：
     ```sql
     SELECT id, session_id, event_type, path, detail, created_at
     FROM audit_log
     WHERE session_id = ?1
     ORDER BY created_at DESC, id DESC
     LIMIT ?2 OFFSET ?3
     ```
   - 无 session_id：去掉 WHERE 子句，其余同上；
   - 参数化，禁止字符串拼接。

### B. Tauri 命令 `audit_log.list`

4. 在 `app/src-tauri/src/commands.rs`：
   ```rust
   #[tauri::command]
   async fn audit_log_list(
       session_id: Option<i64>,
       limit: Option<i64>,
       offset: Option<i64>,
       app: tauri::AppHandle,
   ) -> Result<Envelope<Vec<AuditLogEntry>>, String>;
   ```
5. 内部：打开 DB → 构造 filter（limit 默认 100、offset 默认 0）→ 调 `list_audit_log` → 返 Envelope::ok；DB 错误返 INTERNAL。
6. 在 `lib.rs` 的 `generate_handler!` 注册 `audit_log_list`。

### C. 单测（`engine/tests/audit_log_query.rs`）

7. 至少覆盖：
   - `list_empty_returns_empty`：空 DB → 空 Vec；
   - `list_all_returns_desc_order`：手动 INSERT 5 行（created_at 递增）→ list 返回按时间倒序，最新在前；
   - `list_by_session_id_filters`：INSERT 2 条 session=1 + 3 条 session=2 → filter session_id=2 返回 3 条；
   - `list_pagination_limit_offset`：INSERT 10 行 → limit=3 offset=0 返回前 3 条、offset=3 返回接下来 3 条。

### D. 卫生与边界

8. 不修改 `engine/src/db/**`、`engine/src/execute/**`、`engine/src/rules/**`、`engine/src/scan/**`、`engine/src/ipc/**` 已有逻辑；
9. 不做聚合统计（SUM / GROUP BY）；
10. 不修改 `app/src/**`（前端 React）；
11. 不新增依赖。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/audit_log/mod.rs`（新建）
- `engine/src/audit_log/query.rs`（新建）
- `engine/src/lib.rs`（加 `pub mod audit_log;`）
- `engine/tests/audit_log_query.rs`（新建）
- `app/src-tauri/src/commands.rs`（新增 `audit_log_list`）
- `app/src-tauri/src/lib.rs`（注册命令）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-024-audit-log-list`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(audit_log): 新增 list_audit_log——按 session 过滤 + 分页 + 时间倒序 (#task-024)`
   —— 含 `engine/src/audit_log/mod.rs`、`engine/src/audit_log/query.rs`、`engine/src/lib.rs`。
2. `feat(tauri): 新增 audit_log.list 命令并注册 (#task-024)`
   —— 含 `app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`。
3. `test(audit_log): 补 list 集成测试（空/倒序/session过滤/分页） (#task-024)`
   —— 含 `engine/tests/audit_log_query.rs`。
4. 自验全绿后，最后一次提交：`docs(state): 任务024完成，audit_log.list 就绪 (#task-024)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 024: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-025：audit_log.stats 聚合——按 event_type / module 汇总 total_bytes、total_items、success/fail 计数」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-13  task-024  audit_log.list 只读查询命令落地，历史页后端基础`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 125 条 + 本任务 4 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-024-audit-log-list`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 自验命令与退出码；
   - `cargo test -p engine` 通过输出；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（聚合统计、导出 CSV、前端 UI、清空历史留 task-025+）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务024-audit_log.list (#task-024)`，
   该提交只含 `tasks/024-audit-log-list.md` 一个文件，落在 `feature/task-024-audit-log-list` 分支最前端。
