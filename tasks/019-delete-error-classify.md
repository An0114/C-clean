# 任务编号：019 删除失败错误分类——识别 SharingViolation（文件占用），audit_log 写具体原因

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRust
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-018 合并后由 DocsKeeper 写入）——M1 业务模块 D（文件占用）的第一个任务。
> 上游依赖：task-015（FsDeleter）、task-016（SafeBoxDeleter 已在 task-016 评审时加了 raw_os_error 17/18 判定）均已合并到 main。

## 任务目标

当前 FsDeleter/SafeBoxDeleter 删除失败时只是笼统返回 `Err(String)`，audit_log detail 也只是原始 io error。本任务把**Windows 文件占用错误**识别出来，给前端一个明确的「文件被 N 个进程占用」信号：

1. 在 engine 层新增 `classify_io_error(err: &io::Error) -> DeleteFailureKind`：
   - `NotFound`（errno 2）→ 幂等，不视为失败；
   - `PermissionDenied`（errno 5）→ 需要提权；
   - `SharingViolation`（Windows raw_os_error 32）→ 文件被占用；
   - `AccessDenied`（Windows raw_os_error 5）→ 同 PermissionDenied；
   - 其他 → `Unknown`；
2. SafeBoxDeleter 的 delete_file 抛 SharingViolation 时，返回 `Err("FILE_IN_USE: ...")`（明确前缀）；
3. `execute_items` 拿到 `FILE_IN_USE` 错误时，audit_log 写 `event_type='delete_failed', detail='file in use by another process'`；
4. 端到端：DevAgent 用 notepad 打开一个 tempdir 文件不关闭，跑 cleanup.execute，断言该条目失败且 detail 是 FILE_IN_USE。

**本任务不做**：
- 不做完整 Windows Restart Manager FFI（查询占用进程 PID、请求关闭进程——留 task-020）；
- 不做提权重试（elevation crate 落地后再做）；
- 不做前端 UI 提示（前端 task 留后续）；
- 不写 ps1 查询脚本（Restart Manager 命令行工具留后续）。

## 前置条件

1. `main` 分支已含 task-001~018 全部产出；
2. 关键锚点：
   - `engine/src/execute/fs_deleter.rs`（task-015）；
   - `engine/src/execute/safe_box.rs`（task-016，已在评审时加了 raw_os_error 17/18 判定——17=CrossesDevices/EXDEV、18=CrossesVolumes，不是本任务的 32）；
   - Windows errno 参考：32 = ERROR_SHARING_VIOLATION、5 = ERROR_ACCESS_DENIED、2 = ERROR_FILE_NOT_FOUND。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. Engine 层：错误分类

1. 新建 `engine/src/execute/error_kind.rs`，在 `mod.rs` 加 `pub mod error_kind;`：
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum DeleteFailureKind {
       NotFound,          // 幂等，不算失败
       PermissionDenied,  // 需提权
       SharingViolation,  // 文件被占用
       Unknown,
   }
   pub fn classify_io_error(err: &std::io::Error) -> DeleteFailureKind;
   ```
2. 实现（Windows）：
   - `err.kind() == std::io::ErrorKind::NotFound` → `NotFound`；
   - `err.raw_os_error() == Some(32)` → `SharingViolation`；
   - `err.raw_os_error() == Some(5)` 或 `err.kind() == PermissionDenied` → `PermissionDenied`；
   - 其他 → `Unknown`；
   - 非 Windows 平台用 `err.kind()` 粗分。

### B. SafeBoxDeleter 抛错带明确前缀

3. 在 `engine/src/execute/safe_box.rs` 的 `delete_file` 里：
   - `std::fs::rename` 失败时，先 `classify_io_error(&e)`；
   - `SharingViolation` → 返回 `Err(format!("FILE_IN_USE: {}", path))`；
   - `PermissionDenied` → 返回 `Err(format!("PERMISSION_DENIED: {}", path))`；
   - 其他 → 返回 `Err(e.to_string())`（保持原行为）。

### C. audit_log detail 更具体

4. 在 `engine/src/execute/executor.rs` 的 `execute_items` 里：
   - 失败时写 audit_log：
     - error 以 `FILE_IN_USE:` 开头 → detail = `"file in use by another process"`；
     - error 以 `PERMISSION_DENIED:` 开头 → detail = `"permission denied, needs elevation"`；
     - 其他 → detail = 原始 error 字符串。

### D. 单测（`engine/tests/delete_error_kind.rs`）

5. 至少覆盖：
   - `classify_not_found`：`io::Error::from(ErrorKind::NotFound)` → `NotFound`；
   - `classify_sharing_violation`：构造 `io::Error::from_raw_os_error(32)` → `SharingViolation`；
   - `classify_permission_denied`：构造 `io::Error::from_raw_os_error(5)` → `PermissionDenied`；
   - `classify_unknown`：`io::Error::from(ErrorKind::Other)` → `Unknown`；
6. 端到端（DevAgent 手动，**tempdir 沙箱**）：
   - tempdir 写一个 `locked.tmp`；
   - 用 `notepad.exe locked.tmp &` 打开不关闭；
   - 跑 cleanup.execute 该 id；
   - 断言：返回 `deleted=false`、error 以 `FILE_IN_USE:` 开头；
   - audit_log 表 detail = `"file in use by another process"`；
   - 关闭 notepad 后再跑一次 → 删除成功。

### E. 卫生与边界

7. 不引入 `windows` crate（本任务只用 raw_os_error 数字比较，不做 FFI）；
8. 不修改 SafeBoxDeleter / FsDeleter 已有成功路径；
9. 不修改 Tauri 命令层；
10. 不新增依赖。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/error_kind.rs`（新建）
- `engine/src/execute/mod.rs`（加 `pub mod error_kind;`）
- `engine/src/execute/safe_box.rs`（delete_file 抛错前分类）
- `engine/src/execute/executor.rs`（audit_log detail 规则）
- `engine/tests/delete_error_kind.rs`（新建）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-019-delete-error-classify`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(execute): 新增 classify_io_error——区分 NotFound/PermissionDenied/SharingViolation/Unknown (#task-019)`
   —— 含 `engine/src/execute/error_kind.rs`、`engine/src/execute/mod.rs`。
2. `feat(execute): SafeBoxDeleter 抛错时按分类加 FILE_IN_USE/PERMISSION_DENIED 前缀 (#task-019)`
   —— 含 `engine/src/execute/safe_box.rs`。
3. `feat(execute): execute_items 写 audit_log 时按错误分类填 detail (#task-019)`
   —— 含 `engine/src/execute/executor.rs`。
4. `test(execute): 补 classify_io_error 单元测试 (#task-019)`
   —— 含 `engine/tests/delete_error_kind.rs`。
5. 自验全绿后，最后一次提交：`docs(state): 任务019完成，删除失败错误分类就绪 (#task-019)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 019: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-020：Windows Restart Manager FFI——查询占用进程 PID 列表，供前端提示」；
   - 「已知问题」段补：完整 Restart Manager（请求关闭进程）留 task-020；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-12  task-019  删除失败错误分类（FILE_IN_USE/PERMISSION_DENIED）落 audit_log`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 108 条 + 本任务 4 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-019-delete-error-classify`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - notepad 端到端验证输出（locked.tmp 删除失败 + 关闭后成功）；
   - 自验命令与退出码；
   - `cargo test -p engine` 通过输出；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（Restart Manager FFI、提权重试、前端 UI 提示留 task-020）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务019-删除失败错误分类 (#task-019)`，
   该提交只含 `tasks/019-delete-error-classify.md` 一个文件，落在 `feature/task-019-delete-error-classify` 分支最前端。

## 实现备注（DevAgent 落地后回填）
- **改动文件**：新建 `engine/src/execute/error_kind.rs`（DeleteFailureKind 枚举 + classify_io_error + 4 个单元测试）、`engine/tests/delete_error_kind.rs`（外部集成测试 4 条）；修改 `engine/src/execute/mod.rs`（挂 mod）、`engine/src/execute/safe_box.rs`（rename/copy+remove 失败分类加前缀）、`engine/src/execute/executor.rs`（fail 分支按前缀翻译 audit_log.detail）。
- **notepad 端到端验证**：本任务自动化环境未跑 notepad 真机（需 GUI 交互），但 classify 逻辑由 4 个单元测试（raw_os_error 32/5 + ErrorKind::NotFound/Other）覆盖；rename 失败路径在 SafeBoxDeleter 里直接 match classify_io_error 结果返回前缀错误。
- **自验命令与退出码**：cargo check=0 / clippy -D warnings=0 / cargo test -p engine=0（含 classify 4/4，累计 112 条） / cargo fmt --check=0 / pnpm --filter app build=0。
- **偏差**：SafeBoxDeleter rename 遇 NotFound（分类为 NotFound）时直接 `Ok(())` 幂等返回（与 task-015 FsDeleter 一致），不视为失败；任务书未明确此分支，但保持幂等语义合理。
- **遗留**：完整 Restart Manager FFI（查占用 PID、请求关闭进程）、提权重试、前端 UI toast 提示留 task-020。
