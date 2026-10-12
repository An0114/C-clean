# 任务编号：012 Rust 侧 ps1 NDJSON 消费器——spawn 子进程、逐行解析、写 cleanup_item 表、emit 进度

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRust
> 来源：`docs/PROJECT_STATE.md`「下一步计划」段（2026-10-12 task-011 合并后写入）——M1 业务模块 A 的第一个任务。
> 上游依赖：task-002（`cleanup_item` 表 schema）、task-004（`ProgressEmitter` trait / `ScanProgress` / `ThrottledEmitter`）、task-009（`DbItemPathResolver`）、task-011（5 个 ps1 脚本 + `scripts/PROTOCOL.md` NDJSON 协议）均已合并到 main。
> 关联已知问题：PROJECT_STATE 已知问题第 1 条（task-011 Rust 消费器未实现，留 M1-A）。

## 任务目标

把 task-011 已就绪的 ps1 NDJSON 输出接到 Rust 侧：

1. 在 `engine/src/scan/` 新建模块：
   - `NdjsonEvent` serde 枚举（`Item { module_code, category, path, size_bytes, risk_level }` / `Done { module_code, total_items, total_bytes }`）；
   - `parse_line(line: &str) -> Result<NdjsonEvent, ParseError>`——单行解析；
   - `ScanRunner::run(script_path, session_id, conn, emitter)`——spawn `powershell.exe -NoProfile -File <script>`，逐行读 stdout，item 行写 cleanup_item 表并 emit 进度，done 行结束；
2. 单测用**喂字符串流**的方式测 parser（不真跑 ps1），DB 写用 tempdir；
3. **不**接 Tauri command（留后续 task）、**不**跑 5 个真实 ps1（CI 不跑 powershell，DevAgent 本地可选跑一次验证）。

**本任务不做**：
- 不接 Tauri State / 不改 `app/src-tauri/`；
- 不做「扫完 5 个脚本」的编排（一次只跑一个 script）；
- 不做真实删除（只扫描入库）；
- 不做取消（Ctrl+C / scan.cancel IPC）；
- 不做 stderr 日志解析（ps1 stderr 只转发到 Rust eprintln）；
- 不改 task-011 已有 ps1 脚本与 PROTOCOL.md。

## 前置条件

1. `main` 分支已含 task-001~011 全部产出；
2. 关键锚点：
   - `scripts/PROTOCOL.md`（task-011）——NDJSON 行协议 schema；
   - `engine/src/db/migrations/v1__init.sql`（task-002）——`cleanup_item` 表字段：`session_id / module_code / category / path / size_bytes / risk_level / discovered_at`；
   - `engine/src/ipc/progress.rs`（task-004）——`ProgressEmitter` trait、`ScanProgress` payload；
   - `engine/src/db/conn.rs`（task-002）——`open()` 返回 `Connection`。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. `engine/src/scan/` 模块结构

1. 新建 `engine/src/scan/mod.rs`、`engine/src/scan/parser.rs`、`engine/src/scan/runner.rs`；
2. 在 `engine/src/lib.rs` 加 `pub mod scan;`。

### B. Parser（`parser.rs`）

3. 定义 serde 结构：
   ```rust
   #[derive(serde::Deserialize, Debug, Clone)]
   #[serde(tag = "type")]
   pub enum NdjsonEvent {
       #[serde(rename = "item")]
       Item {
           module_code: String,
           category: String,
           path: String,
           size_bytes: i64,
           risk_level: i64,
       },
       #[serde(rename = "done")]
       Done {
           module_code: String,
           total_items: i64,
           total_bytes: i64,
       },
   }
   ```
4. 错误类型：
   ```rust
   #[derive(thiserror::Error, Debug)]
   pub enum ParseError {
       #[error("JSON 解析失败: {0}")]
       Json(#[from] serde_json::Error),
       #[error("未知事件类型或字段缺失")]
       Schema,
   }
   ```
5. 主函数：
   ```rust
   pub fn parse_line(line: &str) -> Result<NdjsonEvent, ParseError>;
   ```
   - 用 `serde_json::from_str`；
   - 字段缺失时 serde 自动报错（用 `#[serde(deny_unknown_fields)]` 加在 enum 上，发现多余字段也报错）。

### C. Runner（`runner.rs`）

6. 定义：
   ```rust
   pub struct ScanRunner<'conn, 'emitter> {
       session_id: i64,
       conn: &'conn rusqlite::Connection,
       emitter: &'emitter dyn crate::ipc::progress::ProgressEmitter,
       scanned_count: std::sync::atomic::AtomicI64,
   }
   ```
7. 主函数：
   ```rust
   impl<'conn, 'emitter> ScanRunner<'conn, 'emitter> {
       pub fn new(session_id: i64, conn: &'conn rusqlite::Connection, emitter: &'emitter dyn ProgressEmitter) -> Self;
       pub fn run_script(&self, script_path: &std::path::Path) -> Result<ScanSummary, RunError>;
   }
   pub struct ScanSummary {
       pub items_inserted: i64,
       pub total_bytes: i64,
       pub done_seen: bool,
   }
   ```
8. `run_script` 内部流程：
   - `std::process::Command::new("powershell.exe")`
     .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", script_path])
     .stdout(Stdio::piped())
     .stderr(Stdio::piped())
     .spawn()?;
   - 拿子进程 stdout，包 `std::io::BufReader`，按行 `lines()` 迭代；
   - 每一行：调 `parse_line`：
     - `Item` → 执行 SQL：
       ```sql
       INSERT INTO cleanup_item (session_id, module_code, category, path, size_bytes, risk_level, discovered_at)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, unixepoch())
       ```
       参数化；insert 成功后 `scanned_count += 1`，调 `emitter.emit(&ScanProgress { session_id, percent: -1 /* 未知 */, current_path: item.path.clone(), scanned_items: scanned_count })`；
     - `Done` → 记录 `done_seen = true`，`total_bytes = ...`，跳出循环；
     - 解析失败 → `eprintln!` 跳过该行，不 panic、不中断整个扫描；
   - 等待子进程退出，拿 exit status；非 0 退出但已解析到 done 事件 → 视为成功（脚本部分失败但 NDJSON 完整）；无 done 事件且非 0 退出 → 返回 `RunError::ScriptFailed`；
   - stderr 一次性 `read_to_string` 后 `eprintln!` 打印（不解析、不入库）。

### D. 单测（`engine/tests/scan_runner.rs`）

9. Parser 单测（不 spawn 子进程，纯字符串）：
   - `parse_item_line_ok`：合法 item JSON → 反序列化为 `NdjsonEvent::Item`，字段一致；
   - `parse_done_line_ok`：合法 done JSON → `NdjsonEvent::Done`；
   - `parse_missing_field_err`：缺 `path` 字段 → 返回 `ParseError::Json`；
   - `parse_unknown_field_err`：多了个不认识的字段 → 报错（因 `deny_unknown_fields`）；
   - `parse_bad_json_err`：非 JSON 字符串 → 报错。
10. Runner 单测（用**假 script** 方式）：
    - `runner_inserts_items_and_emits`：
      - 用 `tempfile::tempdir()` 造临时 DB（跑迁移）；
      - 先 `INSERT INTO scan_session(started_at) VALUES (1000)` 拿 session_id；
      - 写一个临时 ps1 文件 `test.ps1`，内容是直接 `Write-Output '{"type":"item","module_code":"test","category":"cat","path":"C:\\temp\\a.tmp","size_bytes":100,"risk_level":1}'` 两行 + `Write-Output '{"type":"done","module_code":"test","total_items":2,"total_bytes":200}'`；
      - 用 `NoopEmitter`（task-004 已有）或 `VecEmitter`；
      - 调 `run_script(test.ps1)` → 断言 `items_inserted=2`、`done_seen=true`；
      - 直接查 DB：`SELECT count(*) FROM cleanup_item WHERE session_id=?` → 2 行；
    - `runner_skips_bad_lines`：ps1 里插一行坏 JSON → runner 不 panic、跳过、继续处理后面的合法行；
    - `runner_script_failure_without_done_returns_err`：ps1 里 `exit 1` 不输出 done → `run_script` 返回 `Err`。
11. 所有单测在 windows-latest CI 跑；Linux/macOS 下用 `#[cfg(windows)]` 跳过 ps1 集成测试（parser 单测跨平台跑）。

### E. 卫生与边界

12. 不修改 `engine/src/db/**`、`engine/src/ipc/**`、`engine/src/execute/**` 已有逻辑；
13. 不修改 `app/**`、`elevation/**`、`rules/**`、`scripts/**`；
14. 不引入新 crate（`serde_json / rusqlite / thiserror / tempfile` 都已在）；
15. 不在生产代码里写 `println!`（用 `eprintln!` 打 stderr）；
16. 不实现 scan.cancel（留后续 task）；
17. 不实现 5 个脚本编排（一次跑一个）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/scan/mod.rs`（新建）
- `engine/src/scan/parser.rs`（新建）
- `engine/src/scan/runner.rs`（新建）
- `engine/src/lib.rs`（仅加 `pub mod scan;`）
- `engine/tests/scan_runner.rs`（新建）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-012-ps1-ndjson-runner`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(scan): 新增 NdjsonEvent serde 模型与 parse_line 单行解析器 (#task-012)`
   —— 含 `engine/src/scan/mod.rs`、`engine/src/scan/parser.rs`、`engine/src/lib.rs`（挂 mod）。
2. `feat(scan): 实现 ScanRunner——spawn powershell 子进程、逐行写 cleanup_item、emit 进度 (#task-012)`
   —— 含 `engine/src/scan/runner.rs`。
3. `test(scan): 补 parser 与 runner 集成测试（含假 ps1 脚本） (#task-012)`
   —— 含 `engine/tests/scan_runner.rs`。
4. 自验全绿后，最后一次提交：`docs(state): 任务012完成，Rust 侧 ps1 NDJSON 消费器就绪 (#task-012)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加一条 `- [ ] 012: ...` 并立即勾上 `- [x]`（或在已完成段补）；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-013：把 ScanRunner 接到 Tauri scan.start 命令——spawn 5 个 ps1 串跑、写真实 session、前端 listen scan://progress 端到端」；
   - 「已知问题」段删除 task-011 Rust 消费器未实现那条；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-12  task-012  Rust 侧 ps1 NDJSON 消费器（parser + runner）落地，单测覆盖`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 77 条 + 本任务 8 条全绿）；
- `cargo fmt --all -- --check` 退出码 0。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-012-ps1-ndjson-runner`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 遇到的 ps1 spawn 实际行为（PowerShell 启动耗时、exit code 语义）；
   - DB INSERT 失败时的取舍（跳过该行 vs 整体失败）；
   - 自验命令与退出码；
   - `cargo test -p engine` 通过输出；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（Tauri command 接线、5 脚本编排、scan.cancel 留后续）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务012-Rust侧ps1 NDJSON消费器 (#task-012)`，
   该提交只含 `tasks/012-ps1-ndjson-runner.md` 一个文件，落在 `feature/task-012-ps1-ndjson-runner` 分支最前端。
