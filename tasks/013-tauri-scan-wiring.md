# 任务编号：013 Tauri scan.start 真接 ScanRunner——串跑 5 个 ps1、写真实 session、前端 listen 端到端

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevTauriBridge
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-012 合并后由 DocsKeeper 写入）——M1 业务模块 A 的第二个任务。
> 上游依赖：task-004（`scan_start` 命令骨架 + `ScanStartResp`）、task-007（`TauriProgressEmitter` + `ProgressState`）、task-009（`DbItemPathResolver`）、task-012（`engine::scan::ScanRunner` + `NdjsonEvent`）均已合并到 main。

## 任务目标

task-012 已经在 engine 层把「spawn 单个 ps1 → 逐行 NDJSON → 写 cleanup_item → emit 进度」的最小闭环跑通，但 Tauri 的 `scan_start` 命令仍然只返回 demo sessionId + emit 一条 demo 进度（task-007 留下的）。本任务把这根线接通：

1. 在 engine 层加一个 `ScanOrchestrator::run_all(session_id, conn, emitter, scripts_dir)`——**串跑** 5 个 ps1（temp / windows_update / prefetch / browser_cache / dev_cache），每个 script 复用 task-012 的 `ScanRunner`；
2. 改造 Tauri `scan_start` 命令：
   - 真实 `INSERT INTO scan_session(started_at, status) VALUES (unixepoch(), 'running')` 拿 session_id；
   - 从 State 里取 `ProgressState`（即 `ThrottledEmitter<TauriProgressEmitter>`）；
   - 调 `ScanOrchestrator::run_all`；
   - `scan_session.status` 更新为 `'completed'`（或 `'failed'`）；
   - 返回真实 `session_id`（不再是 demo）；
3. 端到端验证：DevAgent 本地 `pnpm tauri dev`，开 WebView2 devtools console，`await listen('scan://progress', e => console.log(e.payload))` 然后 `invoke('scan_start', { scope: 'full' })`，把 console 输出贴进实现备注。

**本任务不做**：
- 不做并行跑 5 个 ps1（先串跑，性能优化留后续）；
- 不做 scan.cancel（取消命令留后续 task）；
- 不做「按 scope 选跑子集」（本任务硬编码跑全部 5 个；scope 参数暂忽略、仅做入参校验）；
- 不做 cleanup.dryRun 切换 MockResolver → DbItemPathResolver（留后续，因为 cleanup_item 现在是 scan 写完的真实数据）；
- 不改前端 React 组件（不写进度条 UI，只在 console 验证）；
- 不写数据库查询接口（scan.listItems 留后续 task）。

## 前置条件

1. `main` 分支已含 task-001~012 全部产出；
2. 关键锚点：
   - `engine/src/scan/runner.rs`（task-012）——`ScanRunner::run_script(script_path)`；
   - `engine/src/db/conn.rs`——`open()` 返回 `Connection`；
   - `app/src-tauri/src/commands.rs`、`app/src-tauri/src/progress.rs`（task-007）——`TauriProgressEmitter`、`ProgressState`；
   - `app/src-tauri/src/lib.rs`——Builder 的 `manage(...)`；
   - `scripts/` 下 5 个 ps1 + `.gitkeep`（task-011）。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. Engine 层：`ScanOrchestrator`

1. 新建 `engine/src/scan/orchestrator.rs`，在 `mod.rs` 加 `pub mod orchestrator;`。
2. 定义：
   ```rust
   pub struct OrchestratorSummary {
       pub total_items: i64,
       pub total_bytes: i64,
       pub scripts_succeeded: Vec<String>,
       pub scripts_failed: Vec<String>,
   }
   pub struct ScanOrchestrator<'conn, 'emitter> {
       session_id: i64,
       conn: &'conn rusqlite::Connection,
       emitter: &'emitter dyn crate::ipc::progress::ProgressEmitter,
   }
   impl<'conn, 'emitter> ScanOrchestrator<'conn, 'emitter> {
       pub fn new(session_id: i64, conn: &'conn rusqlite::Connection, emitter: &'emitter dyn ProgressEmitter) -> Self;
       pub fn run_all(&self, scripts_dir: &std::path::Path) -> Result<OrchestratorSummary, OrchestratorError>;
   }
   ```
3. `run_all` 硬编码 5 个脚本文件名（常量数组）：
   ```rust
   const SCRIPTS: &[&str] = &[
       "scan_temp.ps1",
       "scan_windows_update.ps1",
       "scan_prefetch.ps1",
       "scan_browser_cache.ps1",
       "scan_dev_cache.ps1",
   ];
   ```
4. 逐个 `script_path = scripts_dir.join(name)`：
   - 文件不存在 → 记 `scripts_failed.push(name)`，继续下一个（不 panic）；
   - 调 `ScanRunner::new(session_id, conn, emitter).run_script(&script_path)`；
   - 成功 → 累加 `items_inserted / total_bytes` 到汇总，记 `scripts_succeeded`；
   - 失败 → 记 `scripts_failed`，继续下一个；
   - 5 个全部失败 → 返回 `Err(OrchestratorError::AllScriptsFailed)`；至少 1 个成功 → 返回 `Ok(summary)`。

### B. Tauri `scan_start` 命令改造

5. 在 `app/src-tauri/src/commands.rs`：
   - 命令签名保持：`async fn scan_start(scope: String, target_drive: Option<String>, app: AppHandle, state: State<'_, ProgressState>) -> Result<Envelope<ScanStartResp>, String>`；
   - 内部流程：
     1. 入参校验（scope ∈ {full, custom}）——沿用 task-004 已有逻辑；
     2. 打开 DB：`let conn = engine::db::conn::open(data_dir)?`（data_dir 用 `app.path().app_data_dir()?` 或临时沿用 `current_dir()`——DevAgent 选并备注）；
     3. `INSERT INTO scan_session(started_at, status) VALUES (unixepoch(), 'running') RETURNING id` 拿 `session_id`；
     4. 构造 `ThrottledEmitter<TauriProgressEmitter>`（直接用 state 里的）；
     5. scripts_dir = `current_dir()?.join("scripts")`（开发期路径，加注释「打包后改资源目录」）；
     6. 调 `ScanOrchestrator::new(session_id, &conn, &state.0).run_all(&scripts_dir)`；
     7. 成功 → `UPDATE scan_session SET status='completed', finished_at=unixepoch() WHERE id=?`；
     8. 失败 → `status='failed'`，返回 `Envelope::err(INTERNAL)`；
     9. 返回 `Envelope::ok(ScanStartResp { session_id, estimated_seconds: None })`；
   - **不**在命令里 spawn 线程（Tauri async command 内部用 `tauri::async_runtime::spawn_blocking` 包住 `run_all`，避免阻塞 async runtime）。

### C. 单测（`engine/tests/scan_orchestrator.rs`）

6. 新建 `engine/tests/scan_orchestrator.rs`：
   - `orchestrator_runs_all_scripts`：
     - tempdir 造 DB + 跑迁移；
     - 建 scan_session 行；
     - 写 3 个临时 ps1 到 tempdir（合法 NDJSON 输出）；
     - 跑 `ScanOrchestrator::run_all(&tempdir)` → 断言 `scripts_succeeded.len() == 3`、`scripts_failed.is_empty()`；
   - `orchestrator_skips_missing_scripts`：
     - tempdir 只放 2 个 ps1，另外 3 个缺失 → `scripts_succeeded.len() == 2`、`scripts_failed.len() == 3`、返回 Ok（不是 Err）；
   - `orchestrator_all_failed_returns_err`：
     - tempdir 不放任何 ps1 → `Err(AllScriptsFailed)`；
   - `orchestrator_writes_cleanup_item_rows`：
     - 跑 1 个合法 ps1 后，直接 `SELECT count(*) FROM cleanup_item WHERE session_id=?` → 与 ps1 输出的 item 行数一致。

### D. 端到端验证（DevAgent 手动）

7. DevAgent 跑 `pnpm tauri dev`（首次可能要编译几分钟）；
8. WebView2 devtools console：
   ```js
   await window.__TAURI__.event.listen('scan://progress', (e) => console.log('[scan]', e.payload));
   const resp = await window.__TAURI__.core.invoke('scan_start', { scope: 'full' });
   console.log('[scan_start resp]', resp);
   ```
9. 预期：
   - console 有多条 `[scan]` 事件（5 个 ps1 跑出来的每个文件一行，节流到 ~100ms 一条）；
   - 最后一条 `[scan_start resp]` 含真实 `sessionId`（数字 ≥1）；
   - 跑完后用 SQLite 工具查 `<data_dir>/cdisk-cleaner.db`：
     - `scan_session` 表有一行 `status='completed'`；
     - `cleanup_item` 表有 N 行（N > 0，取决于本机 Temp/Prefetch/浏览器缓存里实际有多少文件）；
10. DevAgent 把 console 输出（前 10 条 + 最后一条）+ DB 查询结果贴进实现备注。
   - **GUI 自动化不稳定时不强求截图**——task-007 已知问题里已登记「dev 窗口启动后 console 自动化难以稳定聚焦」，贴 console 文本输出即可。

### E. 卫生与边界

11. 不修改 `engine/src/scan/parser.rs`、`engine/src/scan/runner.rs` 已有逻辑；
12. 不修改 `engine/src/ipc/**`、`engine/src/db/**`、`engine/src/execute/**` 已有逻辑；
13. 不修改 `app/src/**`（前端 React）；
14. 不修改 `scripts/**` 已有 ps1；
15. 不新增依赖；
16. 不做并行 spawn（先串跑）；
17. 不做按 scope 选跑子集。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/scan/mod.rs`（仅加 `pub mod orchestrator;`）
- `engine/src/scan/orchestrator.rs`（新建）
- `engine/tests/scan_orchestrator.rs`（新建）
- `app/src-tauri/src/commands.rs`（改造 `scan_start` 命令）
- `app/src-tauri/src/lib.rs`（如需补 manage DB connection State）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-013-tauri-scan-wiring`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(scan): 新增 ScanOrchestrator——串跑 5 个 ps1 并汇总 (#task-013)`
   —— 含 `engine/src/scan/mod.rs`、`engine/src/scan/orchestrator.rs`。
2. `feat(tauri): 改造 scan_start 命令——真实建 session、跑 Orchestrator、更新 status (#task-013)`
   —— 含 `app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`。
3. `test(scan): 补 Orchestrator 集成测试（全跑/缺脚本/全失败/写库行数） (#task-013)`
   —— 含 `engine/tests/scan_orchestrator.rs`。
4. 自验全绿后，最后一次提交：`docs(state): 任务013完成，scan.start 端到端跑通真实扫描 (#task-013)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 013: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-014：cleanup.dryRun 切换 MockResolver → DbItemPathResolver（现在 cleanup_item 有真实数据了）」；
   - 「已知问题」段补/更新：task-009 MockResolver 切换时机已到；devtools console 截图仍由 DevAgent 手动贴文本；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-12  task-013  scan.start 端到端接线：5 ps1 串跑 + 真实 session + 前端 listen`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 88 条 + 本任务 4 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0；
- DevAgent 端到端验证（§D）通过，console 输出 + DB 查询结果已贴备注。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-013-tauri-scan-wiring`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - DB data_dir 实际选择（`app.path().app_data_dir()` vs `current_dir()`）；
   - `spawn_blocking` 使用情况；
   - console listen 输出（前 10 条 + 最后一条 scan_start resp）；
   - DB 查询结果（scan_session 行、cleanup_item 行数、总字节）；
   - 5 个 ps1 在本机的真实成功率（哪些 scripts_succeeded、哪些 failed）；
   - 自验命令与退出码；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（并行、按 scope 子集、scan.cancel、scan.listItems、cleanup.dryRun 切真实 resolver 留后续）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务013-scan.start端到端接线 (#task-013)`，
   该提交只含 `tasks/013-tauri-scan-wiring.md` 一个文件，落在 `feature/task-013-tauri-scan-wiring` 分支最前端。
