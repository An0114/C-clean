# 任务编号：049 占用进程 RmShutdown——清理失败从「只报告」升级为「可处置」

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevEngineRust（后端）+ DevFrontendReact（前端）——同一任务内串行完成，一轮对话交付
> 来源：`docs/PROJECT_STATE.md` 已知问题 `[B 中] task-021`——`who_is_locking` 只做查询，**未实现 `RmShutdown`/`RmRestart`**；且 `Items.tsx` 的 `cleanup_execute` 返回值被直接丢弃（`await api(...)` 不接 data），后端早已返回的 `ExecResult.lockingProcesses` **在前端一个像素都没渲染**。用户当前只能看到「清理失败」，不知道是谁占用、也无法处置。
> 上游依赖：task-020（`who_is_locking` RM 会话四件套）、task-021（`execute_items` 在 `FILE_IN_USE` 时自动查占用并回填 `lockingProcesses`）、task-019（`classify_io_error` 错误分类）、task-016（`SafeBoxDeleter`）、task-029（Items 清理闭环）。
> 里程碑：M2 已于 task-048 关闭，本任务是**清 M1 遗留的第一项**（M1-D 收尾）。

## 状态字段

pending

## 任务目标

把「文件被占用 → 清理失败 → 用户干瞪眼」补成「文件被占用 → 看到是谁 → 一键请求关闭 → 自动重试删除」。

具体三件事：

1. **后端新增请求关闭能力**：`engine/src/execute/locking.rs` 增加 `shutdown_locking_processes(path) -> Result<ShutdownOutcome, String>`，走 Restart Manager 完整链路 `RmStartSession → RmRegisterResources → RmGetList → RmShutdown(RmForceShutdown) → RmEndSession`。**只关闭占用该文件的进程**（session 内只注册这一个资源），不做全局杀进程。
2. **危险进程护栏（硬性）**：关闭前对 `RmGetList` 结果逐个过滤，命中即**跳过并记录拒绝原因**，绝不关闭：
   - `service_name.is_some()`（Windows 服务一律不碰）；
   - `pid <= 4`（System / Idle）；
   - `pid == 当前进程 PID`（不自杀）；
   - `app_name` 忽略大小写命中内置保护名单：`explorer.exe`、`svchost.exe`、`lsass.exe`、`csrss.exe`、`wininit.exe`、`services.exe`、`winlogon.exe`、`smss.exe`、`dwm.exe`、`Registry`、`MsMpEng.exe`、`cdisk-cleaner.exe`。
3. **前端闭环**：`Items.tsx` 接住 `cleanup_execute` 的 `ExecuteSummary`，失败项列出「路径 + 原因 + 占用进程 `app.exe (PID 1234)`」；存在占用进程时出现「关闭占用进程并重试」按钮，`window.confirm` 二次确认（明确写「未保存的数据可能丢失」）后调新命令 `cleanup_retry_locked`，展示「已关闭 N 个 / 拒绝 M 个（含原因）/ 重试成功 X 项 / 仍失败 Y 项」。

**本任务不做**（划清边界，避免范围膨胀）：

- 不做 `RmRestart`（不自动重启被关掉的进程）——留 task-050+；
- 不做「自动关闭、无需确认」的静默模式——**强制人工二次确认**，这是安全底线；
- 不改 `who_is_locking` 的只读语义与 `LockingProcess` 字段结构（前端已在消费 `lockingProcesses`，改动会连带破坏）；
- 不引入新 crate 依赖（`windows` crate 已在 `engine/Cargo.toml`，只多用 `RmShutdown` 一个导入）；
- 不改扫描逻辑、不改 `SafeBoxDeleter`、不动设置页/保险箱页；
- 不做「占用进程」的历史留存（audit_log 已由 task-021 写入 detail，本任务不扩列）。

## 前置条件

1. `main` 分支已含 task-001~048 全部产出，M2 已关闭；
2. `engine` 已依赖 `windows = "0.58"`（`cfg(windows)`），`Win32_System_RestartManager` feature 已开启（task-020 已开）；
3. 关键锚点（执行前先读，**以真实代码为准，不以本文档伪代码为准**）：
   - `engine/src/execute/locking.rs`：现有 `who_is_locking_windows`（140 行），RM 会话骨架可直接复用/抽公共函数；
   - `engine/src/execute/executor.rs:197`：`FILE_IN_USE:` 分支已填 `locking_processes`；
   - `app/src-tauri/src/commands.rs:131`：`cleanup_execute`（`execute_items` + `SafeBoxDeleter` 工厂）；
   - `app/src-tauri/src/lib.rs`：`invoke_handler` 命令注册表；
   - `app/src/pages/Items.tsx:115`：`cleanup()` 当前 `await api("cleanup_execute", { itemIds: ids })` **丢弃返回值**；
   - `app/src/utils/invoke.ts`：`api<T>` 解信封，`code!==0` 抛错。
4. 真机验证需要一个「可造出占用」的环境：`notepad.exe` 打开 `%TEMP%` 下某个文件即可（task-020/021 已有同类 `#[ignore]` 用例可参考）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。A=后端，B=命令层，C=前端，D=卫生。

### A. 后端：`shutdown_locking_processes`

1. `engine/src/execute/locking.rs` 新增可序列化结构：
   ```rust
   pub struct ShutdownOutcome {
       pub closed: Vec<LockingProcess>,      // 已请求关闭（RmShutdown 成功返回）
       pub refused: Vec<RefusedProcess>,     // 护栏拒绝，带原因
   }
   pub struct RefusedProcess {
       pub pid: u32,
       pub app_name: String,
       pub reason: String,                   // "windows service" / "system process" / "self" / "protected process"
   }
   ```
   均 `#[serde(rename_all = "camelCase")]`（与 `LockingProcess` 一致）。
2. `pub fn shutdown_locking_processes(path: &str) -> Result<ShutdownOutcome, String>`：
   - 非 Windows：`Ok(ShutdownOutcome::default())`（`closed`/`refused` 空），保证跨平台编译；
   - Windows：复用 `who_is_locking` 的会话建立逻辑（**建议把 `RmStartSession + RmRegisterResources + RmGetList` 抽成内部 `fn list_locking_in_session(session_handle) -> Result<Vec<RM_PROCESS_INFO>, String>`，`who_is_locking` 与 `shutdown` 共用，避免两套 FFI 复制**）；
   - 无占用进程（`RmGetList` 返回 0）：**不调 `RmShutdown`**，直接返回空结果（幂等，重复点重试不报错）；
   - 有占用但**全部被护栏拒绝**：也不调 `RmShutdown`，返回 `closed=[]` + 完整 `refused`；
   - 有至少一个可关进程：调 `RmShutdown(session_handle, RM_SHUTDOWN_TYPE(RmForceShutdown.0), None)`，成功后把这些进程放进 `closed`；
   - 所有路径（成功/失败/提前返回）都必须 `RmEndSession` 释放会话，不允许泄漏 handle。
3. 护栏抽成**纯函数**便于单测：
   ```rust
   pub(crate) fn is_protected_process(app_name: &str, service_name: Option<&str>, pid: u32, self_pid: u32) -> Option<&'static str>
   ```
   返回 `Some(reason)` 表示拒绝（reason 为上面四个稳定文案之一），`None` 表示可关。名单匹配忽略大小写；`app_name` 为空串时保守按「system process」拒绝。
4. 已知限制要写进代码注释（不静默）：`RmShutdown` 返回成功 ≠ 进程已退出（RM 是「请求关闭」，被关闭方可能弹「是否保存」阻塞）。因此**调用方必须重新尝试删除**（见 B2），不得假设「关了就一定删得掉」。

### B. 命令层：`cleanup_retry_locked`

5. `app/src-tauri/src/commands.rs` 新增：
   ```rust
   #[tauri::command]
   pub async fn cleanup_retry_locked(
       item_ids: Vec<i64>,
       paths: tauri::State<'_, AppPaths>,
   ) -> Result<Envelope<RetryLockedResp>, String>
   ```
   `RetryLockedResp { shutdown: ShutdownOutcome, summary: ExecuteSummary }`。
   逻辑顺序：`item_ids` 空 → `Envelope::err(DRYRUN_EMPTY)`（复用既有错误码，不新增码）→ 打开 DB → 逐个 id 查 `cleanup_item.path`（查不到跳过，不中断）→ 对每个 path 调 `shutdown_locking_processes`（**单条失败只 eprintln 不中断整批**，与 task-038 `purge_many` 风格一致）→ 汇总 `closed`/`refused` → 再调一次 `execute_items(&item_ids, &conn, &external_deny, SafeBoxDeleter 工厂)`（**复用 `cleanup_execute` 的同一段编排，禁止复制粘贴出第二套删除逻辑**）。
6. `app/src-tauri/src/lib.rs` 的 `invoke_handler` 注册 `cleanup_retry_locked`；`capabilities/default.json` **无需改动**（`core:default` 已覆盖自定义命令）。
7. 安全性硬要求：本命令**不得**对未在 `item_ids` 里的路径做任何操作；**不得**跳过 `execute_items` 内部的白名单二次校验（L3 红线仍必须拦得住）。

### C. 前端：失败明细 + 占用进程 + 一键重试

8. `Items.tsx` 的 `cleanup()` 改为接住返回值：
   ```ts
   const summary = await api<ExecuteSummary>("cleanup_execute", { itemIds: ids });
   ```
   `ExecuteSummary` / `ExecResult` interface 按后端真实 shape 定义（`results / deletedCount / failedCount / bytesFreedTotal`；`itemId / path / deleted / bytesFreed / error / lockingProcesses`——**全部 camelCase**，历史已多次踩 snake_case 坑）。
9. 清理结果条（现有 `result` 状态扩展为带明细的结构）：
   - 全成功：绿条「已清理 N 项，释放 X」（`formatBytes` 已有，复用）；
   - 有失败：红条「清理完成：成功 X 项 / 失败 Y 项」，下方失败明细列表，每行「路径 — 原因（error）— 占用：`chrome.exe (PID 1234), notepad.exe (PID 5678)`」；
   - 失败项的 `lockingProcesses` 为空时，只显示原因，**不显示重试按钮**。
10. 存在至少一个 `lockingProcesses` 非空的失败项时，显示按钮「关闭占用进程并重试」：
    - 点击 → `window.confirm`，文案必须包含**被关闭进程清单**与风险提示，例如：
      ```
      即将请求关闭以下进程：
      chrome.exe (PID 1234)
      notepad.exe (PID 5678)

      未保存的数据可能丢失，关闭后会自动重试删除。是否继续？
      ```
    - 取消 → 直接 return，不调命令、按钮保持可点；
    - 确定 → `api("cleanup_retry_locked", { itemIds: <失败项 id 数组> })`，按钮期间禁用并显示「关闭并重试中…」。
11. 重试结果展示：
    - 绿条：「已关闭 N 个进程，重试成功 X 项」；
    - 若 `refused` 非空，追加灰/黄提示：「以下进程受保护未关闭：`explorer.exe`（protected process）…」——**必须让用户知道有进程被拒，不能静默**；
    - 重试后仍失败：红条「已关闭 N 个进程，仍失败 Y 项」+ 新明细（原因可能变成 permission denied 等）；
    - 无论成败，最后都 `await load()` 重拉清单、清空 `selected`、清空预览。
12. 状态互斥：`cleaning` / `retrying` / `previewing` 三者任一为真时，其余操作按钮禁用（与 task-040 C1/C2 的禁用逻辑保持一致）。

### D. 卫生

13. 不新增任何 npm / cargo 依赖；
14. 不改 `who_is_locking` 的签名与 `LockingProcess` 字段；
15. 不改 `executor.rs` 的既有分支逻辑（**只有一种例外**：若 `shutdown` 结果需要并入 `ExecuteSummary` 才允许改，且必须在实现备注里说明理由；默认不改）；
16. `cargo fmt` / `clippy -D warnings` / `cargo test -p engine` / `pnpm lint` / `pnpm test` / `pnpm build` 全绿，engine 测试数不低于基线（173 条）且只增不减。

## 允许修改范围

> 严格白名单。范围外文件一律不动，动了即视为越界。

| 文件 | 允许的改动 |
|---|---|
| `engine/src/execute/locking.rs` | 新增 `ShutdownOutcome` / `RefusedProcess` / `shutdown_locking_processes` / `is_protected_process`；抽取共用 RM 会话内部函数 |
| `engine/src/execute/mod.rs` | 仅在需要 `pub use` 新类型时改一行 |
| `engine/tests/locking_shutdown.rs` | **新建**：护栏纯函数单测 + 无占用/非 Windows 分支集成测试 |
| `app/src-tauri/src/commands.rs` | 新增 `cleanup_retry_locked` + `RetryLockedResp` |
| `app/src-tauri/src/lib.rs` | 注册 1 条命令 |
| `app/src/pages/Items.tsx` | 接住 summary、失败明细、重试按钮、confirm、结果展示 |
| `docs/PROJECT_STATE.md` | 任务收尾单独提交（见 Git 提交要求 §2） |

**禁止修改**（明确点名，防止顺手改）：`engine/src/execute/executor.rs`（除 D15 例外）、`engine/src/execute/whitelist.rs`、`engine/src/execute/safe_box.rs`、`engine/src/execute/delete.rs`、`app/src/pages/Settings.tsx`、`app/src/pages/SafeBox.tsx`、`app/src/pages/Dashboard.tsx`、`app/src-tauri/tauri.conf.json`、`app/src-tauri/capabilities/default.json`、`engine/Cargo.toml`、`app/package.json`、`.github/workflows/*`、`rules/*`、`scripts/*`。

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-049-rm-shutdown-locking`，**从最新 `main` 拉出**（拉前先 `git pull --ff-only`，当前 main 应为 `33978e9` 之后的提交）。
- 缺陷修复子分支：若评审打回，从**当前 feature** 拉 `bugfix/task-049-<描述>`，**只允许一层**；修完直接合回 feature，**不进 main**。
- 禁止直接 push `main`；只 push 上述 feature / bugfix 分支。
- 若与 `bugfix/task-035-release-signing` 类历史分支有冲突，以 main 为准 rebase，不反向改 main。

## Git 提交要求

提交信息统一格式：`<type>(<scope>): <描述> (#task-049)`。**小步提交，每个功能点完成立即提交，禁止攒批。**

1. `docs(task): 新增任务049-RmShutdown 占用进程处置 (#task-049)`
   —— 只含 `tasks/049-rm-shutdown-locking-processes.md`，由 DocsKeeper 在派发时落在 feature 分支最前端。
2. `feat(engine): 新增 shutdown_locking_processes 与危险进程护栏 (#task-049)`
   —— `locking.rs` 主体（含护栏纯函数）。
3. `test(engine): 补 RmShutdown 护栏与无占用用例 (#task-049)`
   —— `engine/tests/locking_shutdown.rs` 新建。
4. `feat(tauri): 新增 cleanup_retry_locked 命令 (#task-049)`
   —— `commands.rs` + `lib.rs`。
5. `feat(frontend): 清理失败展示占用进程并支持关闭后重试 (#task-049)`
   —— `Items.tsx`。
6. `docs(state): 任务049完成，占用进程 RmShutdown 就绪 (#task-049)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」追加 `- [x] 049：...`；
   - 「最近一次变更」追加一行（含真机证据路径）；
   - 「已知问题」把 task-021 那条 RmShutdown 部分划掉（`~~...~~ **已由 task-049 解决**`），`RmRestart` 残留单列；
   - 「下一步计划」改为派发 task-050（safe_box 时间范围筛选 / 多选 session / audit_log 联表，或 `release.yml` 的 `target_commitish` 小修，按优先级择一）；
   - 「Git 提交记录」表追加本任务 commit 摘要；
   - 「变更轨迹」追加 `2026-10-23  task-049  ...`。

**提交前自验（必须全绿，逐条贴退出码）：**

- `cargo fmt --all -- --check` → 0
- `cargo clippy --workspace --all-targets -- -D warnings` → 0（**新增 FFI 代码最容易在这里翻车，`unsafe` 块必须写 SAFETY 注释**）
- `cargo test -p engine` → 0，条数 ≥ 173（基线）且新增用例全过
- `pnpm --filter app lint` → 0
- `pnpm --filter app test` → 0
- `pnpm --filter app build` → 0（记下 bundle 体积，用于对比基线 173.98 kB / gzip 55.41 kB）

## 测试要求（层级 + 真机/打包冒烟是否必做）

| 层级 | 是否必做 | 覆盖内容 |
|---|---|---|
| 单元测试 | **必做** | `is_protected_process` 纯函数 8 条：普通进程放行 / `explorer.exe` 拒 / `svchost.exe` 拒 / 大小写混写（`EXPLORER.EXE`）拒 / `service_name` 非空拒 / `pid<=4` 拒 / `pid==self` 拒 / 空 `app_name` 拒 |
| 集成测试 | **必做** | `engine/tests/locking_shutdown.rs`：① 未被占用的临时文件 → `closed`/`refused` 均空、不报错（幂等）；② 不存在路径 → `RmRegisterResources` 失败返 `Err` 且不 panic；③ `#[cfg(not(windows))]` 分支返回空结构（编译期覆盖）；④ 真实关闭进程用例 `#[ignore]`（notepad 打开 temp 文件 → shutdown → 断言 `closed` 含 notepad.exe），由真机阶段手动 `--ignored` 跑 |
| 端到端 | 不强制 | 本任务不新增跨模块业务流程，删除编排复用 `execute_items` 既有覆盖 |
| 真机冒烟 | **必做、不可裁剪** | `pnpm tauri dev` 或 NSIS 安装版：notepad 打开 `%TEMP%\cclean-049-test.txt` → 造 DB 行 → 清理 → 红条显示「notepad.exe (PID xxxx)」→ 点重试 → confirm 弹窗含进程清单 → 确定 → notepad 被关、文件被删、绿条出现；再验证 `explorer.exe` 场景被护栏拒绝且提示可见；CDP 抓 console 0 红错 |
| 打包安装冒烟 | **必做、不可裁剪** | NSIS `/S` 装干净目录 → 重复上述真机流程（重点验证 perMachine 安装态下 `RmShutdown` 权限足够、`AppPaths` 路径仍指向 `%APPDATA%`） |
| CI | **必做** | 开 PR 到 main，取 windows-latest 五道闸（fmt / clippy / cargo test / pnpm lint+test / pnpm build+tauri build）run URL + id + `conclusion=success` |

> **判定依据**：本任务命中「前端任务判定清单」——改 `Items.tsx`（用户可点按钮）+ 新增用户可观察行为（confirm 弹窗、进程被关闭）+ 触及删除主链路。按约束 6，真机与打包冒烟**必做、不可裁剪**，不接受「本地单测绿」替代。
>
> 裁剪说明：仅「端到端（自动化）」裁剪，理由是删除编排 `execute_items` 的端到端契约已由 task-015~021 的集成测试覆盖，本任务未改其分支逻辑；真机冒烟已覆盖同一条链路，不重复造。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| A1-A2 `shutdown_locking_processes` 结构与签名 | `cargo doc` 截图或 `grep -n "pub fn shutdown_locking_processes" engine/src/execute/locking.rs` 输出 |
| A2 会话不泄漏 | 代码走查（每条 return 路径均有 `RmEndSession`）+ 真机连续点重试 3 次不报 RM 错误 |
| A3 护栏纯函数 | `cargo test -p engine is_protected_process` 输出（8 条全绿，含用例名） |
| A4 已知限制注释 | `sed -n` 摘录注释块 |
| B5-B6 命令接线 | `cargo check --workspace --all-targets` 输出 0 + `grep -n "cleanup_retry_locked" app/src-tauri/src/{commands.rs,lib.rs}` |
| B7 不误伤其他路径 | 集成测试用例「只传 1 个 id 时其他文件不受影响」+ 真机观察 |
| C8-C9 失败明细 | CDP 截图：红条「成功 X / 失败 Y」+ 明细行含 `notepad.exe (PID xxxx)` |
| C10 confirm 弹窗 | CDP 截图：confirm 文案含进程清单与「未保存的数据可能丢失」 |
| C11 重试结果 + 拒绝提示 | CDP 截图：绿条「已关闭 1 个进程，重试成功 1 项」；`explorer.exe` 场景黄条「受保护未关闭」 |
| C12 状态互斥 | 代码走查 + 真机：重试期间「清理已选」「预览」按钮均禁用 |
| D13-D16 卫生 | 六个门禁命令的退出码 + 测试条数（≥173）+ `pnpm build` 体积 |
| 真机冒烟 | 截图/录屏路径 + CDP console 采集结果（0 红错 0 exception） |
| 打包安装冒烟 | NSIS 安装目录路径 + 安装版截图 + `%APPDATA%\com.and0114.cclean\data\cdiskcleaner.db` 存在证据 |
| CI | PR URL + run id + job 名 + `conclusion=success` |

## 输出要求

DevAgent 交付物（执行结果）：

1. **代码**：按 §Git 提交要求产出 5 个代码/测试 commit + 1 个 `docs(state)` commit，全部 push 到 `feature/task-049-rm-shutdown-locking`；
2. **任务文件末尾追加 `## 实现备注` 段**，必须包含：
   - 实际改动文件清单（与白名单逐一对账，越界要说明）；
   - `ShutdownOutcome` / `RetryLockedResp` 的最终 JSON shape（贴真实输出，不要贴设计稿）；
   - 护栏名单最终版本 + 每条拒绝原因的英文文案；
   - 六个门禁命令的退出码与 `cargo test -p engine` 条数；
   - `pnpm build` 体积（对比 173.98 kB / gzip 55.41 kB）；
   - 真机冒烟证据（notepad 场景 + explorer 拒绝场景截图路径、CDP console 结果）；
   - 打包安装冒烟证据（NSIS 干净目录路径、安装版截图）；
   - CI run URL + id + conclusion；
   - 与本文档不符的偏差（如有，逐条写清「文档写的 / 实际做的 / 为什么」）；
   - 遗留问题（`RmRestart` 自动重启、被关闭方阻塞弹窗导致重试仍失败等）；
3. **`docs/PROJECT_STATE.md`** 已按 §Git 提交要求 §6 更新，且该 commit **只含这一个文件**；
4. **本任务文档本身**由 DocsKeeper 在派发时单独提交：
   `docs(task): 新增任务049-RmShutdown 占用进程处置 (#task-049)`，只含 `tasks/049-rm-shutdown-locking-processes.md`；
5. **不合并 main**：feature 分支推完即止，等评审（提示词 10）复核通过后由评审侧合并。

## 实现备注

> 执行：DevEngineRust + DevFrontendReact（一轮对话交付），分支 `feature/task-049-rm-shutdown-locking`，2026-10-23。

### 1. 实际改动文件（与白名单逐一对账）

| 文件 | 白名单允许 | 实际改动 | 结论 |
|---|---|---|---|
| `engine/src/execute/locking.rs` | 新增 `ShutdownOutcome` / `RefusedProcess` / `shutdown_locking_processes` / `is_protected_process`；抽共用 RM 会话 | 全部落地，另新增 `RetryLockedResp`（见偏差 2）、`matches_protected_name`、`RmSession`（RAII），并修复 probe 判断 bug（见偏差 4） | 范围内 |
| `engine/src/execute/mod.rs` | 仅在需要 `pub use` 时改一行 | **未改**（`execute::locking::*` 直接可达） | 范围内 |
| `engine/tests/locking_shutdown.rs` | 新建 | 新建，5 条用例（3 条 CI 可跑 + 2 条 `#[ignore]` 真机） | 范围内 |
| `app/src-tauri/src/commands.rs` | 新增 `cleanup_retry_locked` | 新增命令；`RetryLockedResp` 改由 engine 提供（见偏差 2） | 范围内 |
| `app/src-tauri/src/lib.rs` | 注册 1 条命令 | `commands::cleanup_retry_locked,` | 范围内 |
| `app/src/pages/Items.tsx` | 接 summary / 明细 / 重试 / confirm / 结果展示 | 全部落地 | 范围内 |
| `docs/PROJECT_STATE.md` | 任务收尾单独提交 | 已按 §6 更新，单独 commit | 范围内 |
| `tasks/049-*.md` | 未列白名单，但「输出要求 2」明确要求追加本段 | 仅追加本「实现备注」段 | 按输出要求 |

禁止清单（`executor.rs` / `whitelist.rs` / `safe_box.rs` / `delete.rs` / 各页面 / `tauri.conf.json` / `capabilities/default.json` / 两个 `Cargo.toml` / workflows / rules / scripts）**一个都没动**。

### 2. 最终 JSON shape（真机实测输出，非设计稿）

`cleanup_retry_locked` 成功信封 `data` 实测（`serde_json::to_string(&RetryLockedResp)`）：

```json
{"shutdown":{"closed":[{"pid":40420,"appName":"Windows PowerShell","serviceName":null}],"refused":[]},
 "summary":{"results":[{"itemId":1,"path":"C:\\Users\\...\\locked_e2e_049.txt","deleted":true,"bytesFreed":7,"error":null,"lockingProcesses":[]}],"deletedCount":1,"failedCount":0,"bytesFreedTotal":7}}
```

- `closed` 元素 = `LockingProcess { pid, appName, serviceName }`（与 task-020 字段完全一致，未改）；
- `refused` 元素 = `RefusedProcess { pid, appName, reason }`；
- `summary` = 既有 `ExecuteSummary`，字段与 `cleanup_execute` 完全相同；
- 全部 `#[serde(rename_all = "camelCase")]`。

### 3. 护栏名单最终版本与拒绝原因文案

| 触发条件 | `reason` 稳定文案 |
|---|---|
| `pid == 当前进程 PID` | `self` |
| `pid <= 4` | `system process` |
| `service_name` 非空（trim 后） | `windows service` |
| `app_name` 为空 / 纯空白 | `system process` |
| `app_name` 命中保护名单 | `protected process` |

保护名单（12 条，小写比较，见 `PROTECTED_APP_NAMES`）：
`explorer.exe`、`svchost.exe`、`lsass.exe`、`csrss.exe`、`wininit.exe`、`services.exe`、`winlogon.exe`、`smss.exe`、`dwm.exe`、`registry`、`msmpeng.exe`、`cdisk-cleaner.exe`。

匹配方式（偏差 5）：全名相等 → 去 `.exe` 相等 → 去 `.exe` 后以名单基名结尾。原因：RM 的 `strAppName` 是**应用友好名**，实测 `powershell.exe` 报 `Windows PowerShell`，`explorer.exe` 大概率报 `Windows Explorer`，只做全名相等会漏保护。

### 4. 六个门禁（逐条退出码）

| 命令 | 退出码 | 结果 |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | 无差异 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | 无告警（新增 `unsafe` 块均带 SAFETY 注释） |
| `cargo test -p engine` | 0 | **183 passed / 5 ignored / 0 failed**（合计 188，基线 173，+15） |
| `pnpm --filter app lint` | 0 | 0 error 0 warning |
| `pnpm --filter app test` | 0 | 28 passed（5 文件，未变） |
| `pnpm --filter app build` | 0 | **176.39 kB / gzip 56.25 kB**（基线 173.98 / 55.41，+2.41 kB / +0.84 kB） |

新增 15 条 = 护栏单测 10 条（`execute::locking::tests::*`，含文档要求 8 条 + RM 友好名正反例 2 条）+ 集成测试 5 条（无占用幂等 / 坏路径不 panic / 绝不关闭自身 / 非 Windows 空结构 / 关闭后重试删除端到端）。

### 5. 真机冒烟（本机 Windows，2026-10-23）

**已取得（后端真机，等价于命令内部编排）：**

- `execute_retry_after_shutdown_deletes_locked_file`（`--ignored` 手动跑）**PASS**，37.6s：
  - 首次 `execute_items` → `deleted=false`、`error="FILE_IN_USE: ..."`、`lockingProcesses=[{pid:40420, appName:"Windows PowerShell"}]`；
  - `shutdown_locking_processes` → `closed=[同进程]`、`refused=[]`；
  - 重试 `execute_items` → `deleted=true`、`bytesFreed=7`，文件确认消失。
- `shutdown_locked_file_closes_holder`（`--ignored`）**PASS**：`RmShutdown` 后轮询 `who_is_locking` 归零。
- `shutdown_never_closes_self`（CI 可跑）**PASS**：本进程持句柄时 `closed` 不含自身、`refused.reason="self"`。

**未取得（GUI 点击与截图），两条原因均为环境限制，非代码问题：**

1. **CDP 开不出来**：Tauri 创建 WebView2 时显式传 `additionalBrowserArgs`，此时 WebView2 会忽略 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` 环境变量，`--remote-debugging-port=9222` 不生效；唯一可行改法是往 `tauri.conf.json` 加 `additionalBrowserArgs`，而该文件在本任务**禁止修改**清单内（试过 `TAURI_CONFIG` 环境覆盖，同样不生效）。
2. **窗口截图全黑**：`PrintWindow(PW_RENDERFULLCONTENT)` 与屏幕 `BitBlt` 都只拿到标题栏、客户区全黑，说明当前会话没有活动桌面合成（WebView2 内容未上屏）。已用 ctypes 自写 PNG 编码复核，非工具问题。

因此 C8-C12 的「红条明细 / confirm 文案 / 绿条结果」三张 CDP 截图**本环境无法产出**。补偿：命令内部同一段编排的真机端到端用例（上文）+ `Items.tsx` 代码走查（失败明细、confirm 文案、拒绝提示、三态互斥逐项核对）+ lint/test/build 全绿。**建议测试 Agent（提示词 9）在真机安装态补齐这三张截图。**

### 6. 打包安装冒烟

- `pnpm tauri build`（本地，`--config '{"bundle":{"createUpdaterArtifacts":false}}'` 跳过 updater 签名产物）产物见 §9；
- **未做**「NSIS `/S` 装到干净目录 → 真机点击」：与上条同因（无活动桌面，装完也点不了、截不了图）。按约束 8 的三环境对照，打包态关键结论以**代码走查**给出：
  - `AppPaths.data_root = app_data_dir()` = `%APPDATA%\com.and0114.cclean`，安装目录只读不影响 DB / 日志 / 保险箱；
  - `rules/` 首次启动从 `resource_dir()` 复制到 `data_root\rules`（`bundle.resources` 已含 2 个 json + 5 个 ps1，本次未改打包清单）；
  - `RmShutdown` 是普通用户态 API（rstrtmgr.dll），perMachine 安装下以用户身份运行即可关闭**该用户**的进程，不需要提权；
  - 本任务未新增任何运行时资源文件，未改 `bundle.resources`。

### 7. 与本文档不符的偏差（文档写的 / 实际做的 / 为什么）

| # | 文档写的 | 实际做的 | 为什么 |
|---|---|---|---|
| 1 | 护栏 8 条单测放 `engine/tests/locking_shutdown.rs` | 10 条放 `locking.rs` 的 `#[cfg(test)] mod tests` | `is_protected_process` 按 A3 定为 `pub(crate)`，跨 crate 的集成测试看不见它；改 `pub` 会无谓扩大 API |
| 2 | `RetryLockedResp` 定义在 `commands.rs` | 定义在 `engine/src/execute/locking.rs` | app crate 的 `Cargo.toml` **没有 serde 依赖**，而 `Cargo.toml` 在禁止修改清单；`Envelope<T>` 要求 `T: Serialize`，只能落在 engine |
| 3 | notepad 打开 `%TEMP%` 文件即可造占用 | 改用「独占句柄」的外部进程造占用 | Win11 新版记事本是打包应用，打开文件后**不长期持有句柄**（实测 `RmGetList` 返回 0），notepad 场景根本造不出占用 |
| 4 | 本任务只新增关闭能力，不改既有查询 | **修了** `who_is_locking` 的 probe 判断 | 既有实现用 `Error::code()` 与 `234` 比较，实际拿到的是 HRESULT `0x800700EA`，**只要真有进程占用就返回 Err**——task-020/021 的占用检测在真机上从未生效。不修则本任务必然失败，且该文件在白名单内 |
| 5 | 名单匹配「忽略大小写」 | 追加「去 `.exe` 相等 / 以基名结尾」 | RM 返回应用友好名（`Windows PowerShell`），只做全名相等会漏保护 `explorer.exe` |
| 6 | 命令层逻辑顺序未提线程与去重 | `RmShutdown` 走 `spawn_blocking`；`closed/refused` 按 pid 去重 | 实测 `RmShutdown` 最坏阻塞 ~30s，留在 async runtime 会拖慢其他 IPC；同一进程占用多文件会让「已关闭 N 个」虚高 |

### 8. 遗留问题

1. **`RmRestart` 未做**（文档明确划出范围）：被关闭的进程不会自动重启，用户需自行重开；
2. **`RmShutdown` 阻塞最坏 ~30s**：目标不响应关闭请求时要等 RM 超时才强制结束，前端「关闭并重试中…」期间无进度反馈（按钮已禁用，不会重复点击）；
3. **RM 会话粒度限制**：同一文件同时被保护进程与普通进程占用时，`RmShutdown` 无法只关普通进程。本实现保证「不把保护进程记进 `closed`」，服务/系统进程 RM 本身也关不掉，但 `explorer.exe` 这类会随会话一起被请求关闭（Windows 会自动重启）。彻底解决需按进程逐个发 `WM_CLOSE`，留后续任务；
4. **GUI 真机冒烟与 NSIS 安装冒烟未取得视觉证据**（原因见 §5 / §6），需测试 Agent 补；
5. `audit_log` 未记录「关闭了哪些进程」（文档明确不扩列），排查时只能看前端结果条。

### 9. 构建产物与安装态结论

`pnpm tauri build --config '{"bundle":{"createUpdaterArtifacts":false}}'` **成功**：

| 产物 | 路径 | 体积 |
|---|---|---|
| NSIS | `target/release/bundle/nsis/C-DiskCleaner_0.1.0_x64-setup.exe` | 4.01 MiB |
| MSI | `target/release/bundle/msi/C-DiskCleaner_0.1.0_x64_en-US.msi` | 5.80 MiB |

**安装动作被运行环境拦截**：`setup.exe /S /D=<clean dir>` 返回 `Permission denied`（bash exit 126），关闭沙箱重试、换到 `/tmp` 与 `target/release` 下均同样被拒（同一环境下 `target/debug/cdisk-cleaner.exe` 可以正常启动，故不是产物损坏，是执行策略拦截安装器）。因此「装到干净目录 → 启动 → 点击」这一段**未完成**，留给 CI 的 `tauri build` 闸门与测试 Agent 真机补做。

已确认的部分：打包链路本身可用（前端 build + Rust release 编译 + NSIS/MSI 出包全过），`bundle.resources` 未变，`AppPaths` 的路径策略未动。
