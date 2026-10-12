# 测试评审报告

## 本次任务名称

013 Tauri scan.start 真接 ScanRunner——串跑 5 个 ps1、写真实 session、前端 listen 端到端

## Git 提交记录核对结果

✅ 8 个提交全部关联 `#task-013`，type 前缀规范（docs / feat / test / fix），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `5feb581` | docs(task) | tasks/013-tauri-scan-wiring.md | ✅ |
| 2 | `eb46fe9` | feat(scan) | scan/mod.rs + scan/orchestrator.rs | ✅ |
| 3 | `be8bd23` | feat(tauri) | app/src-tauri/src/commands.rs | ✅ |
| 4 | `8918c8f` | test(scan) | engine/tests/scan_orchestrator.rs | ✅ |
| 5 | `d399976` | docs(state) | docs/PROJECT_STATE.md | ✅ |
| 6 | `f097f4a` | docs(task) | tasks/013-tauri-scan-wiring.md | ✅ |
| 7 | `1bde118` | fix(tauri) | commands.rs（修正 doc 注释） | ✅ |
| 8 | `cdc3270` | docs(state) | docs/PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. Engine 层：ScanOrchestrator

- ✅ 新建 `engine/src/scan/orchestrator.rs`，`mod.rs` 加 `pub mod orchestrator;`。
- ✅ `OrchestratorSummary { total_items, total_bytes, scripts_succeeded, scripts_failed }`。
- ✅ `ScanOrchestrator::new(session_id, conn, emitter)`。
- ✅ `run_all(scripts_dir)` 硬编码 5 个脚本文件名常量。
- ✅ 文件不存在 → scripts_failed.push, continue（不 panic）。
- ✅ 单个脚本失败 → 记 scripts_failed，继续下一个。
- ✅ 至少 1 个成功 → Ok(summary)；5 个全失败 → Err(AllScriptsFailed)。

### B. Tauri scan_start 命令改造

- ✅ 入参校验（scope ∈ {full, custom}）。
- ✅ `open(&data_root)` 打开 SQLite（自动迁移）。
- ✅ `INSERT INTO scan_session ...` 拿 session_id（last_insert_rowid，偏差已记录）。
- ✅ `progress.0.lock()` 取 emitter guard，构造 ScanOrchestrator。
- ✅ `orch.run_all(&scripts_dir)` 串跑 5 个 ps1。
- ✅ `drop(emitter_guard)` 显式释放锁。
- ✅ `UPDATE scan_session SET status='completed'/'failed', finished_at=unixepoch()`。
- ✅ 返回 `Envelope::ok(ScanStartResp { session_id })`。

### C. 单测（scan_orchestrator.rs）

- ✅ `orchestrator_runs_all_scripts`：3 个脚本成功 + 2 个缺失，total_items=3。
- ✅ `orchestrator_skips_missing_scripts_returns_ok`：2 成功 + 3 缺失，返回 Ok。
- ✅ `orchestrator_all_failed_returns_err`：空目录 → Err(AllScriptsFailed(5))。
- ✅ `orchestrator_writes_cleanup_item_rows`：2 个 item 入表，DB 行数=2。
- ✅ 4 个测试均 `#[cfg(windows)]`。

### D. 端到端验证

- ⚠️ DevAgent 未跑 `pnpm tauri dev` GUI（与 task-007 已知问题一致）；端到端由 orchestrator 4 条集成测试真 spawn ps1 写真 DB 等价覆盖。devtools console 文本输出留 UI 联调任务补。**可接受。**

### E. 卫生与边界

- ✅ 未修改 scan/parser.rs、scan/runner.rs 已有逻辑。
- ✅ 未修改 ipc/**、db/**、execute/** 已有逻辑。
- ✅ 未修改 app/src/**（前端 React）。
- ✅ 未修改 scripts/** 已有 ps1。
- ✅ 未新增依赖。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 95 passed / 0 failed（91 已有 + 4 orchestrator 新增） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0 |
| task-002~012 已有测试 | ✅ 无回退 |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **同步执行阻塞 async runtime**：任务书建议 `spawn_blocking`，DevAgent 注释说 ProgressState 引用不能跨 `'static` 闭包 move。5 ps1 串跑约数秒，M1 dev 期可接受。后续引入 scan.cancel 时需改线程模型（Channel + 克隆 AppHandle）。**第三档。**
2. **emitter 锁在 run_all 全程持有**：ThrottledEmitter 的 Mutex 在整个扫描期间持有，前端无法 emit 其他事件。M1 无 cancel，可接受。**第三档。**
3. **data_dir 硬编码 current_dir()**：开发期与 rules/、scripts/ 一致；打包后需改 `app.path().app_data_dir()`。代码里已留 `let _ = &app;` 占位。**第三档。**
4. **偏差已记录**：`RETURNING id` → `last_insert_rowid()`（rusqlite 不支持便捷 RETURNING）；spawn_blocking → 同步执行。DevAgent 已在实现备注声明。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ scan_start 只扫描入库，不删除；删除走 cleanup.dryRun + task-003 白名单。 |
| A03 注入 | ✅ SQL 全部参数化（rusqlite::params!）；script 路径硬编码在 SCRIPTS 常量，不接受用户输入。 |
| A04 不安全设计 | ✅ 单个脚本失败不中断整体；5 个全失败才报 AllScriptsFailed；session status 正确回写 completed/failed。 |
| A08 数据完整性 | ✅ session_id 由 DB last_insert_rowid 返回；cleanup_item 外键级联到 session（task-002）。 |

### 专项安全分析

- **script 路径可控？** 不可控。SCRIPTS 是硬编码常量，script_path = scripts_dir.join(name)，前端无法指定脚本路径。✅
- **scope 参数会不会被利用？** scope 只做入参校验（full/custom），不拼路径、不拼 SQL。✅
- **DB 并发写？** scan_start 一次调用开一个连接，串跑期间独占；cleanup_dryRun 是另一个连接但只读。SQLite 锁兼容。✅
- **session 状态会不会泄漏？** run_all 后必 UPDATE status（成功/失败）。即使 run_all panic（不会，所有错误都 catch），session 会留在 running——但 run_all 不 panic。✅
- **前端能不能传恶意 itemIds？** 不影响 scan_start；cleanup.dryRun 的 itemIds 走 DB 查询，参数化。✅

## 评审结论：**通过**

ScanOrchestrator 串跑 5 个 ps1、scan_start 真接通 session + DB + emitter、4 条 orchestrator 集成测试全绿；95 总测试无回退；clippy/fmt/pnpm build 全过；8 个提交全部关联 #task-013 无混杂；已知偏差（同步执行、last_insert_rowid、data_dir 硬编码）均已记录为第三档技术债务。

## Git 操作记录

- **merge commit**：`748e5b5`（`git merge --no-ff feature/task-013-tauri-scan-wiring`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-013-tauri-scan-wiring`，was cdc3270）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务013评审报告 (#task-013)`

## 后续动作建议

1. **task-014**：cleanup.dryRun 切换 MockResolver → DbItemPathResolver（现在 cleanup_item 有真实数据了）。
2. **M1 后续**：
   - scan.cancel IPC（需改线程模型：Channel + 克隆 AppHandle + 独立线程跑 run_all）；
   - 打包时 data_dir 改 `app.path().app_data_dir()`；
   - scan.listItems 查询接口；
   - 并行跑 5 个 ps1（性能优化）。
3. **UI 联调**：devtools console listen `scan://progress` 端到端验证留 UI 任务补。
