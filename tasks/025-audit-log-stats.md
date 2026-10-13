# 任务编号：025 audit_log.stats——清理结果聚合（总释放字节 / 成功数 / 失败数）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRust
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-024 合并后由 DocsKeeper 写入）——M1 业务模块 F（报表）的第二个任务。
> 上游依赖：task-002（`cleanup_task` 表 schema：action / status / bytes_freed / item_id / session_id）、task-015/016（cleanup_task 写入点）、task-024（audit_log.list 查询）均已合并到 main。
> **注意**：audit_log 表只有自由文本，聚合数字从 `cleanup_task` 表取（结构化字段），不从 audit_log 文本解析。

## 任务目标

task-024 已经能列出历史明细，但前端「清理了多少 GB、多少次成功/失败」这种数字还拿不到。本任务补聚合查询：

1. engine 层 `audit_log/stats.rs`：
   ```rust
   pub struct CleanupStats {
       pub total_bytes_freed: i64,   // SUM(bytes_freed) WHERE status='success'
       pub success_count: i64,       // COUNT(*) WHERE status='success'
       pub failed_count: i64,        // COUNT(*) WHERE status='failed'
       pub session_count: i64,       // COUNT(DISTINCT session_id)
   }
   pub fn cleanup_stats(conn: &rusqlite::Connection, session_id: Option<i64>) -> Result<CleanupStats, String>;
   ```
2. 三条 SQL（或合并成一条）：
   - `SELECT COALESCE(SUM(bytes_freed), 0) FROM cleanup_task WHERE status='success'`
   - `SELECT COUNT(*) FROM cleanup_task WHERE status='success'`
   - `SELECT COUNT(*) FROM cleanup_task WHERE status='failed'`
   - `SELECT COUNT(DISTINCT session_id) FROM cleanup_task`
   - 有 session_id 时加 `AND session_id = ?`；
3. Tauri 命令 `audit_log.stats`：入参 `session_id?: number`，返回 CleanupStats。

**本任务不做**：
- 不做按 event_type / module 维度拆分（cleanup_task 表没有 module_code，要 JOIN cleanup_item 才能拿——留 task-026）；
- 不做时间区间过滤（今天/本周——留后续）；
- 不做前端 UI 图表（留后续）；
- 不做 audit_log 文本聚合。

## 前置条件

1. `main` 分支已含 task-001~024 全部产出；
2. 关键锚点：
   - `engine/src/db/migrations/v1__init.sql`——`cleanup_task` 表字段；
   - `engine/src/audit_log/query.rs`（task-024）——查询模式参考。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. Engine 层：`cleanup_stats`

1. 新建 `engine/src/audit_log/stats.rs`，在 `mod.rs` 加 `pub mod stats;`。
2. 结构与函数签名见「任务目标」段。
3. 实现要点：
   - 用 4 条独立 prepared statement（简单清晰），或合并成 `SELECT SUM(...) FILTER (WHERE ...), COUNT(*) FILTER (...), ... FROM cleanup_task`（DevAgent 选，备注风格）；
   - 参数化 session_id 过滤；
   - 空表返回全 0（不是 NULL）。

### B. Tauri 命令 `audit_log.stats`

4. 在 `app/src-tauri/src/commands.rs`：
   ```rust
   #[tauri::command]
   async fn audit_log_stats(
       session_id: Option<i64>,
       app: tauri::AppHandle,
   ) -> Result<Envelope<CleanupStats>, String>;
   ```
5. 内部：打开 DB → 调 `cleanup_stats` → 返 Envelope::ok；DB 错误返 INTERNAL。
6. 在 `lib.rs` 的 `generate_handler!` 注册 `audit_log_stats`。

### C. 单测（`engine/tests/audit_log_stats.rs`）

7. 至少覆盖：
   - `stats_empty_returns_zero`：空 cleanup_task 表 → 全 0；
   - `stats_counts_success_failed`：INSERT 3 success（bytes 100+200+300）+ 2 failed → total_bytes_freed=600、success_count=3、failed_count=2、session_count=1（同一 session）；
   - `stats_filters_by_session`：session=1 有 2 success、session=2 有 1 success → filter session_id=1 返回 success_count=2、session_count=1；
   - `stats_distinct_session_count`：3 个不同 session 各 1 行 → session_count=3。

### D. 卫生与边界

8. 不修改 `engine/src/audit_log/query.rs` 已有逻辑；
9. 不 JOIN cleanup_item（module 维度留 task-026）；
10. 不修改 `app/src/**`；
11. 不新增依赖。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/audit_log/mod.rs`（加 `pub mod stats;`）
- `engine/src/audit_log/stats.rs`（新建）
- `engine/tests/audit_log_stats.rs`（新建）
- `app/src-tauri/src/commands.rs`（新增 `audit_log_stats`）
- `app/src-tauri/src/lib.rs`（注册命令）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-025-audit-log-stats`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(audit_log): 新增 cleanup_stats——总释放字节 / 成功数 / 失败数 / session 数 (#task-025)`
   —— 含 `engine/src/audit_log/mod.rs`、`engine/src/audit_log/stats.rs`。
2. `feat(tauri): 新增 audit_log.stats 命令并注册 (#task-025)`
   —— 含 `app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`。
3. `test(audit_log): 补 stats 集成测试（空/计数/session过滤/去重） (#task-025)`
   —— 含 `engine/tests/audit_log_stats.rs`。
4. 自验全绿后，最后一次提交：`docs(state): 任务025完成，清理统计数字就绪 (#task-025)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 025: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「M1-F 报表基础（list + stats）已就绪；下一轮进入前端 UI 接入（React 历史页 + 仪表盘）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-13  task-025  cleanup_stats 聚合查询（total_bytes / success / fail / sessions）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 129 条 + 本任务 4 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-025-audit-log-stats`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - SQL 风格（4 条独立 vs 1 条 FILTER）；
   - 自验命令与退出码；
   - `cargo test -p engine` 通过输出；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（module 维度拆分、时间区间、前端 UI 图表留 task-026+）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务025-audit_log.stats (#task-025)`，
   该提交只含 `tasks/025-audit-log-stats.md` 一个文件，落在 `feature/task-025-audit-log-stats` 分支最前端。
