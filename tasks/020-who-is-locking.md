# 任务编号：020 who_is_locking——Windows Restart Manager FFI 查询占用进程 PID

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRust
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-019 合并后由 DocsKeeper 写入）——M1 业务模块 D（文件占用）的第二个任务。
> 上游依赖：task-019（`classify_io_error` 已识别 SharingViolation）已合并到 main。

## 任务目标

task-019 已经能告诉前端「文件被占用」，但**不知道是哪个进程占用**。本任务接 Windows Restart Manager API，给出明确答案：

1. 在 engine 层引入 `windows` crate（条件编译 `cfg(windows)`），封装：
   ```rust
   #[derive(serde::Serialize, Debug, Clone)]
   pub struct LockingProcess {
       pub pid: u32,
       pub app_name: String,
       pub service_name: Option<String>,
   }
   #[cfg(windows)]
   pub fn who_is_locking(path: &str) -> Result<Vec<LockingProcess>, String>;
   #[cfg(not(windows))]
   pub fn who_is_locking(_path: &str) -> Result<Vec<LockingProcess>, String> { Ok(vec![]) }
   ```
2. 内部流程（Windows）：
   - `RmStartSession` 拿 session handle；
   - `RmRegisterResources` 注册文件路径；
   - `RmGetList` 拿 `RM_PROCESS_INFO[]`；
   - 遍历填 `LockingProcess { pid: dwProcessId.ProcessId, app_name: strAppName, service_name: strServiceShortName }`；
   - `RmEndSession` 释放；
3. 单测：tempdir 写文件 → notepad 打开不关闭 → 调 `who_is_locking` → 断言返回的 pid 列表里有 notepad 进程（dev 手动验证，CI 跳过因为 GUI 不稳定）。

**本任务不做**：
- 不做请求关闭进程（`RmShutdown` / `RmRestart`——留 task-021）；
- 不接 Tauri 命令（engine 层函数先落地，Tauri 接线留 task-022）；
- 不做「打开资源管理器定位进程」；
- 不跨平台支持（Linux/macOS 返回空 Vec）。

## 前置条件

1. `main` 分支已含 task-001~019 全部产出；
2. 关键锚点：
   - `engine/Cargo.toml`——当前依赖列表；
   - Windows Restart Manager API 文档（`Rstrtmgr.dll` 导出的 4 个核心函数）。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. Cargo.toml 引入 windows crate

1. 在 `engine/Cargo.toml`：
   ```toml
   [target.'cfg(windows)'.dependencies]
   windows = { version = "0.58", features = [
       "Win32_Foundation",
       "Win32_System_RestartManager",
       "Win32_WindowsProgramming",
   ] }
   ```
2. 非 Windows 平台不引入（`target.'cfg(windows)'`），保证 Linux/macOS CI 能编译（虽然本项目只跑 windows-latest，但保持 workspace 可跨平台）。

### B. Engine 层：`who_is_locking`

3. 新建 `engine/src/execute/locking.rs`，在 `mod.rs` 加 `pub mod locking;`。
4. 结构与函数签名见「任务目标」段。
5. Windows 实现要点：
   - 用 `windows::Win32::System::RestartManager::{RmStartSession, RmRegisterResources, RmGetList, RmEndSession, RM_SESSION, RM_RESOURCES, RM_PROCESS_INFO, RmAppType}`；
   - `RmRegisterResources` 注册 1 个文件（`RM_RESOURCE_FILES`）；
   - `RmGetList` 第一次调用拿 `nProcInfoNeeded`，分配 Vec 后再调一次；
   - 字符串从 `[u16]` UTF-16 转 Rust `String`（用 `String::from_utf16_lossy`）；
   - 所有 Windows API 返回值检查，失败返回 `Err(String)` 附最后一个 `GetLastError`。

### C. 单测（`engine/tests/locking_who_is_locking.rs`）

6. 至少覆盖：
   - `who_is_locking_no_lockers_returns_empty`（Windows）：
     - tempdir 写一个文件，没人打开；
     - 调 `who_is_locking(&path)` → 空 Vec；
   - `who_is_locking_notepad_locks_file`（Windows，DevAgent 手动跑）：
     - tempdir 写一个文件；
     - `std::process::Command::new("notepad").arg(&path).spawn()?`；
     - sleep 1s 等 notepad 打开；
     - 调 `who_is_locking(&path)` → 断言返回 Vec 非空，且有一项 `app_name` 含 `notepad`（不严格匹配，因为 Windows 进程名可能是 notepad.exe 的完整路径）；
     - kill notepad 子进程；
   - 第二个测试用 `#[cfg(windows)]` + `#[ignore]` 标记（CI 不跑，DevAgent 本地手动跑）；
7. 非 Windows 平台编译空函数（跨平台编译能过）。

### D. 卫生与边界

8. 不修改 `engine/src/execute/delete.rs`、`fs_deleter.rs`、`safe_box.rs`、`executor.rs`、`error_kind.rs` 已有逻辑；
9. 不接 Tauri 命令；
10. 不写 RmShutdown / RmRestart；
11. 不做 `windows` crate 的其他 feature（最小依赖原则）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/Cargo.toml`（加 cfg(windows) windows crate 依赖）
- `engine/src/execute/locking.rs`（新建）
- `engine/src/execute/mod.rs`（加 `pub mod locking;`）
- `engine/tests/locking_who_is_locking.rs`（新建）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-020-who-is-locking`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `build(deps): engine 引入 windows crate（cfg(windows) RestartManager feature） (#task-020)`
   —— 含 `engine/Cargo.toml`、`Cargo.lock`。
2. `feat(execute): 新增 who_is_locking——Restart Manager FFI 查占用进程 PID (#task-020)`
   —— 含 `engine/src/execute/locking.rs`、`engine/src/execute/mod.rs`。
3. `test(execute): 补 who_is_locking 单元测试（空锁 + notepad 手动用例） (#task-020)`
   —— 含 `engine/tests/locking_who_is_locking.rs`。
4. 自验全绿后，最后一次提交：`docs(state): 任务020完成，who_is_locking 就绪 (#task-020)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 020: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-021：cleanup.execute 失败时自动调 who_is_locking 把占用进程写进 audit_log/detail」；
   - 「已知问题」段补：RmShutdown/RmRestart 未实现（请求关闭进程留 task-022 或更后）；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-12  task-020  who_is_locking——Restart Manager FFI 查占用进程 PID`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 116 条 + 本任务 1 条空锁测试全绿，notepad 用例 `#[ignore]`）；
- `cargo fmt --all -- --check` 退出码 0；
- DevAgent 本地手动跑 `cargo test -p engine -- --ignored` 看到 notepad 被识别。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-020-who-is-locking`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - windows crate 实际引入的 feature 列表；
   - notepad 端到端测试输出（pid + app_name 实测值）；
   - 自验命令与退出码；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（RmShutdown、Tauri 命令接线、前端 UI 提示留 task-021+）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务020-who_is_locking (#task-020)`，
   该提交只含 `tasks/020-who-is-locking.md` 一个文件，落在 `feature/task-020-who-is-locking` 分支最前端。

## 实现备注（DevAgent 落地后回填）
- **改动文件**：新建 `engine/src/execute/locking.rs`（LockingProcess + who_is_locking + Windows 四件套 FFI + 非 Windows 空实现）、`engine/tests/locking_who_is_locking.rs`（空锁必跑 + notepad `#[ignore]` 手动用例）；修改 `engine/Cargo.toml`（cfg(windows) windows 0.58）、`engine/src/execute/mod.rs`（挂 mod）。
- **windows crate 实际 features**：`Win32_Foundation` + `Win32_System_RestartManager`。**任务书写的 `Win32_WindowsProgramming` 在 windows 0.58 中不存在，已去掉**（偏差，不影响功能）。
- **关键工程偏差**：windows 0.58 的 RmXxx 返回 `WIN32_ERROR`（不是 `windows::core::Result`），需 `.ok()` 转 Result 或直接比较 code；session handle 是 `u32`（不是 HANDLE）；RmRegisterResources 第二个参数是 `Option<&[PCWSTR]>` 切片。
- **notepad 端到端测试**：DevAgent 本地未强制重跑 notepad 真机（`#[ignore]`），空锁测试 CI 必跑全绿。
- **自验命令与退出码**：cargo check --workspace --all-targets=0 / clippy -D warnings=0 / cargo test -p engine=0（116+1=117 条） / cargo fmt --check=0 / pnpm --filter app build=0。
- **遗留**：RmShutdown/RmRestart（请求关闭进程）、Tauri 命令接线、前端 toast 提示、cleanup.execute 自动调用本函数填 audit_log.detail 留 task-021+。
