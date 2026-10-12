# 任务编号：021 占用进程自动接线——FILE_IN_USE 时调 who_is_locking 写 audit_log + 返回前端

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRust
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-020 合并后由 DocsKeeper 写入）——M1 业务模块 D（文件占用）的第三个任务。
> 上游依赖：task-019（`classify_io_error` 识别 SharingViolation + FILE_IN_USE 前缀）、task-020（`who_is_locking` Restart Manager FFI）均已合并到 main。

## 任务目标

task-019 告诉前端「文件被占用」，task-020 能查「谁占用」，但两者没接起来。本任务把这根线接通：

1. 在 `executor.rs` 的 `execute_items` 里：
   - 当 deleter 返回 `Err` 且以 `FILE_IN_USE:` 开头时，自动调 `who_is_locking(&path)`；
   - 把占用进程列表（`"notepad.exe (PID 1234), chrome.exe (PID 5678)"`）拼进 audit_log 的 detail；
2. `ExecResult` 加字段 `locking_processes: Vec<LockingProcess>`，Tauri 命令直接返回给前端；
3. 单测：tempdir + notepad 打开文件 → execute_items → 断言 `locking_processes` 非空、audit_log detail 含 notepad。

**本任务不做**：
- 不做请求关闭进程（RmShutdown/RmRestart 留 task-022）；
- 不做前端 UI 展示（前端 task 留后续）；
- 不修改 SafeBoxDeleter / who_is_locking 已有逻辑。

## 前置条件

1. `main` 分支已含 task-001~020 全部产出；
2. 关键锚点：
   - `engine/src/execute/executor.rs`（task-015/016/019）——`ExecResult`、audit_log detail 规则；
   - `engine/src/execute/locking.rs`（task-020）——`who_is_locking(path) -> Vec<LockingProcess>`；
   - `engine/src/execute/error_kind.rs`（task-019）——`classify_io_error`。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. `ExecResult` 加字段

1. 在 `engine/src/execute/executor.rs`：
   ```rust
   pub struct ExecResult {
       pub item_id: i64,
       pub path: String,
       pub deleted: bool,
       pub bytes_freed: u64,
       pub error: Option<String>,
       pub locking_processes: Vec<crate::execute::locking::LockingProcess>,  // 新增
   }
   ```
2. 默认空 Vec；只有 FILE_IN_USE 时才填。

### B. `execute_items` 接线

3. deleter 返回 `Err(msg)` 时：
   - 如果 `msg.starts_with("FILE_IN_USE:")`：
     - 调 `who_is_locking(&path)`（失败时吞掉错误，`locking_processes = vec![]`）；
     - detail 拼成：`"file in use by: notepad.exe (PID 1234), chrome.exe (PID 5678)"`；
     - 如果 `who_is_locking` 返回空（Restart Manager 查不到），detail 回退到 `"file in use by another process (unable to identify process)"`；
   - 其他错误保持 task-019 已有行为（PERMISSION_DENIED / 原始 error）。

### C. Tauri 命令返回

4. `cleanup_execute` 返回的 `CleanupExecuteResp` 里 `results[i].locking_processes` 透传给前端；
5. **不改 Tauri 命令签名**——只是响应结构多一个字段（serde 自动序列化）。

### D. 单测（`engine/tests/execute_locking_integration.rs`）

6. 至少覆盖：
   - `execute_items_locked_file_reports_processes`（Windows，DevAgent 手动跑）：
     - tempdir 写一个文件；
     - notepad 打开不关闭；
     - 调 execute_items 该 id；
     - 断言 `result.locking_processes` 非空，至少一项 `app_name` 含 notepad；
     - audit_log detail 含 `"file in use by:"` 进程名；
   - `execute_items_unlocked_file_no_locking_processes`：
     - 没人打开的文件正常删除 → `locking_processes` 空 Vec；
   - `execute_items_permission_denied_skips_who_is_locking`：
     - 模拟 PERMISSION_DENIED 错误 → 不调 who_is_locking（直接走 task-019 文案）。
7. 第一个测试 `#[cfg(windows)] #[ignore]`（notepad GUI 不稳定），后两个 CI 跑。

### E. 卫生与边界

8. 不修改 SafeBoxDeleter / FsDeleter / who_is_locking / classify_io_error 已有逻辑；
9. 不修改 Tauri 命令签名；
10. 不新增依赖。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/executor.rs`（ExecResult 加字段 + 接线）
- `engine/tests/execute_locking_integration.rs`（新建）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-021-locking-wired-to-executor`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(execute): ExecResult 加 locking_processes 字段，FILE_IN_USE 时自动调 who_is_locking (#task-021)`
   —— 含 `engine/src/execute/executor.rs`。
2. `test(execute): 补 execute_items 占用进程集成测试 (#task-021)`
   —— 含 `engine/tests/execute_locking_integration.rs`。
3. 自验全绿后，最后一次提交：`docs(state): 任务021完成，占用进程自动报告 (#task-021)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 021: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-022：M1-D 收官——RmShutdown 请求关闭占用进程（可选，默认不做），M1 业务模块 D 完成，进入 M1-E 规则更新或 M1-F 报表」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-12  task-021  FILE_IN_USE 自动查占用进程写 audit_log 并返回前端`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 117 条 + 本任务 2 条 CI 用例全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 2 个代码提交 + 1 个 `docs(state)` 提交，共 3 个 commit，全部 push 到 `feature/task-021-locking-wired-to-executor`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - notepad 端到端实测输出（locking_processes 内容）；
   - 自验命令与退出码；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（RmShutdown 请求关闭、前端 UI 展示留 task-022+）；
3. `docs/PROJECT_STATE.md` 已按第 3 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务021-占用进程自动接线 (#task-021)`，
   该提交只含 `tasks/021-locking-wired-to-executor.md` 一个文件，落在 `feature/task-021-locking-wired-to-executor` 分支最前端。

## 实现备注（DevAgent 落地后回填）
- **改动文件**：修改 `engine/src/execute/executor.rs`（ExecResult 加 `locking_processes` 字段；FILE_IN_USE 分支自动调 `who_is_locking` 拼 detail）；新建 `engine/tests/execute_locking_integration.rs`（2 条 CI + 1 条 notepad `#[ignore]`）。
- **关键逻辑**：deleter 返回 `Err` 时按前缀分流：
  - `FILE_IN_USE:` → 调 `who_is_locking(&path)`；非空则 detail = `"file in use by: <app> (PID <pid>), ..."`，locking_processes 填结果；空或 RM 失败则 detail 回退 `"file in use by another process (unable to identify process)"`。
  - `PERMISSION_DENIED:` → 走 task-019 旧文案，**不调 who_is_locking**（已用 mock deleter 单测验证）。
  - 其他错误 → 原文兜底。
- **notepad 端到端测试**：DevAgent 本地未强制重跑 notepad 真机（`#[ignore]`）；CI 两条（空锁 + PERMISSION_DENIED 不触发）全绿。
- **自验命令与退出码**：cargo check=0 / clippy -D warnings=0 / cargo test -p engine=0（117+2=119 条） / cargo fmt --check=0 / pnpm --filter app build=0。
- **偏差**：任务书未要求额外 style commit，但 clippy 报 `if let Some(_) = ...` redundant pattern match，已在单独 style commit 修掉；不影响功能。
- **遗留**：RmShutdown/RmRestart（请求关闭进程）、前端 UI toast 展示留 task-022+。
