# 测试报告：task-049 占用进程 RmShutdown

> 测试 Agent（提示词 9），独立于开发轮执行：不复用开发轮自验结论，全部门禁本地重跑；GUI 相关结论以本轮实测为准。
> 分支：`feature/task-049-rm-shutdown-locking`（合并 bugfix 后）；缺陷修复子分支：`bugfix/task-049-retry-btn-mutex`。
> 测试日期：2026-10-24（本机 Windows 11 / 用户 `ANFER-PC\An`，非管理员）。

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
|---|---|---|---|
| dev（本地启动） | `cargo build`（debug，`target/debug/cdisk-cleaner.exe`）+ `vite` 5173；WebView2 运行时 155.0.4283.62 | 是（进程与 dev server 均正常启动） | **不通过（环境阻塞）**：WebView2 渲染进程无法创建，页面不渲染、CDP 无端口、窗口无 DOM 可访问 |
| 打包安装（干净目录） | `pnpm tauri build` → NSIS `C-DiskCleaner_0.1.0_x64-setup.exe` 4.01 MiB / MSI 6.94 MiB；`/S /D=C:\Users\An\AppData\Local\Temp\cclean-smoke-049-nsis` | 是（安装成功 exit 0） | **部分通过**：安装、payload、数据目录隔离、安装目录零写入均通过；GUI 点击路径同上被环境阻塞 |
| CI | workflow `ci`；run **39967340368**（feature 0541235）/ **40045174671**（bugfix 8b84cf4）/ **40045926460**（bugfix f667c70）/ **40047649773**（merge be60f55）/ **40047903556**（HEAD b1d6118） | 是 | 绿 |

> 环境限制说明（本轮实测，非引用开发轮说法）：本会话**没有活动桌面合成**，`Graphics.CopyFromScreen` 返回「句柄无效」；应用进程启动后**子进程数为 0**、系统内无属于本应用的 `msedgewebview2.exe`；`%LOCALAPPDATA%\com.and0114.cclean\EBWebView\Crashpad\reports` 在每次启动后新增 `.dmp`（08:12 / 08:16 / 08:20 / 08:21 / 08:56 各一份）。UIA 枚举该窗口只得到一个无内容 `Pane`。另：wry 0.57 无条件设置 `additionalBrowserArgs`（`wry-0.57.0/src/webview2/mod.rs:300`），因此 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` 被 WebView2 忽略；改用 `tauri dev --config` 注入 `--remote-debugging-port=9222` 后二进制内已含该参数，但渲染进程本身起不来，端口仍未监听。

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据（日志/截图/报告路径） |
|---|---|---|---|---|
| L1 单元 | engine 全量（非只跑改动文件）+ 前端全量 | `cargo fmt --all -- --check`；`cargo clippy --workspace --all-targets -- -D warnings`；`cargo test -p engine --no-fail-fast`；`pnpm --filter app lint`；`pnpm --filter app exec vitest run --pool=forks --no-file-parallelism`；`pnpm --filter app build` | **全绿**：fmt 0 / clippy 0 / cargo **183 passed, 0 failed, 6 ignored** / lint 0 / vitest **34 passed（6 files）** / build 0（**176.39 kB，gzip 56.25 kB**） | 本轮终端输出（见 §3 D13-D16 与 §5）；基线 173 条，本轮 +10 |
| L2 集成/契约 | `cleanup_retry_locked` 全链路 + 前后端契约 | `cargo test -p engine --test locking_shutdown -- --ignored`（真机占用）；逐字段核对（见下） | **通过**：真实占用 → 首次删除失败并回填 `lockingProcesses` → `RmShutdown` 关闭 → 重试删除成功；契约 4 项全部对齐 | 真机输出：`first: deleted=false, error=FILE_IN_USE, locking_processes=[{pid:32220, app_name:"Windows PowerShell"}]` → `shutdown: closed=[同进程], refused=[]` → `second: deleted=true, bytesFreed=7`；`retry_locked_resp_json: {"shutdown":{"closed":[{"pid":32220,"appName":"Windows PowerShell","serviceName":null}],"refused":[]},"summary":{...}}` |
| L3 端到端/UI（dev） | 清理页：失败明细 / 占用进程 / 重试按钮 / confirm / 重试结果 / 三态互斥 | `vite`(5173) + `target/debug/cdisk-cleaner.exe`；CDP 9222 与 UIA 双路尝试 | **阻塞（第三档）**：应用进程启动正常（`[paths] data_root=...\com.and0114.cclean`），但 WebView2 渲染进程崩溃 → 页面不渲染、无 DOM、无 Console 可采集 | 启动日志 `/tmp/tauri-dev*.log`；`[paths]` 行；Crashpad `.dmp`；UIA 仅 1 个空 `Pane`。**未取得任何截图/Console 证据，不伪造** |
| L4 真机/打包安装 | NSIS 安装到干净目录 + 打包态数据目录隔离 + 安装目录零写入 | `pnpm tauri build`；`setup.exe /S /D=<干净目录>`；启动安装版；安装目录 9 文件 SHA-256 前后比对 | **部分通过**：安装 exit 0；payload = exe + uninstall.exe + `_up_\_up_\{rules,scripts}`（2 json + 5 ps1）；安装版启动后 `data_root=%APPDATA%\com.and0114.cclean`、`box_dir=%APPDATA%\...\safe_box`；安装目录 **9/9 文件哈希完全一致（零写入）** | 安装目录 `C:\Users\An\AppData\Local\Temp\cclean-smoke-049-nsis`；哈希比对结果 `NO_DIFF`；安装版 `[paths]` 日志。GUI 点击路径被环境阻塞（同 L3） |
| L5 CI 门禁 | 本分支 5 次运行 | GitHub Actions API 实查 | **绿**：0541235 / 8b84cf4 / f667c70 / be60f55 / b1d6118 全部 `conclusion=success` | run 39967340368、40045174671、40045926460、40047649773、40047903556 |

### L2 契约核对表（逐字段）

| 核对项 | 后端 | 前端 | 结论 |
|---|---|---|---|
| 命令签名 ↔ 实参 | `cleanup_retry_locked(item_ids: Vec<i64>, paths: State<AppPaths>)`（`commands.rs:168`） | `api("cleanup_retry_locked", { itemIds })`（Tauri v2 默认 camelCase→snake_case） | 对齐 |
| serde 注解 | `RetryLockedResp` / `ShutdownOutcome` / `RefusedProcess` / `ExecuteSummary` / `ExecResult` 全部 `#[serde(rename_all = "camelCase")]` | `interface RetryLockedResp / ExecuteSummary / ExecResult / LockingProcess / RefusedProcess` 全 camelCase | 对齐（真机 JSON 与 interface 逐字段一致） |
| Envelope 解包 | `Result<Envelope<RetryLockedResp>, String>`，空 `item_ids` → `Envelope::err(DRYRUN_EMPTY)` | `api<T>()` 解 `{code,message,data}`，`code!==0` 抛错；前端仅在 `locked.length>0` 时调用，不会触发空数组分支 | 对齐 |
| 事件 payload | 本任务**未新增/改动**任何 Tauri 事件（`progress.rs` 未动） | 无新增监听 | 无影响 |

### L2 真机隔离性（B7，本轮新增用例）

`shutdown_only_touches_registered_path`（`--ignored`）：A、B 两个文件各由一个外部进程独占持有 → 只对 A 调 `shutdown` → A 占用消失、**B 占用与 B 的进程原样保留**（`b before=[pid 14484] b after=[pid 14484] holder_b_alive=true`）。连续 3 次稳定 PASS。

## 3. 验收标准逐条核对（每条必须有运行时证据）

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
|---|---|---|---|
| A1 | `ShutdownOutcome` / `RefusedProcess` 结构 + camelCase | `grep -n` 输出：`locking.rs:36 pub struct ShutdownOutcome`、`:46 pub struct RefusedProcess`、`:61 pub struct RetryLockedResp`；真机 JSON `{"closed":[{"pid":32220,"appName":"Windows PowerShell","serviceName":null}],"refused":[]}` | 通过 |
| A2 | `shutdown_locking_processes` 签名 + 幂等 + 全会话收尾 | `locking.rs:181 pub fn shutdown_locking_processes(path: &str) -> Result<ShutdownOutcome, String>`；`shutdown_no_lockers_is_idempotent`（连调两次均空结果）PASS；`RmEndSession` 置于 `RmSession::drop`（成功/失败/提前返回统一释放）；真机连续调用无 RM 错误 | 通过 |
| A3 | 护栏纯函数 8 条（实为 10 条） | `cargo test -p engine execute::locking::tests` → **10 passed**：`allows_normal_process` / `refuses_explorer` / `refuses_svchost` / `refuses_protected_name_case_insensitive` / `refuses_windows_service` / `refuses_system_pid` / `refuses_self` / `refuses_empty_app_name` / `refuses_rm_friendly_name_of_protected_app` / `allows_rm_friendly_name_of_normal_app` | 通过 |
| A4 | 已知限制注释（RmShutdown≠已退出） | `locking.rs` 文件头「## 已知限制」三条，第 1 条明写「必须重新尝试删除」 | 通过 |
| B5 | `cleanup_retry_locked` 命令实现 + 空 ids 复用 DRYRUN_EMPTY | `commands.rs:168`；`cargo clippy --workspace --all-targets` 0（含 app crate 全量类型检查） | 通过 |
| B6 | `lib.rs` 注册 | `lib.rs:39 commands::cleanup_retry_locked,` | 通过 |
| B7 | 不误伤其他路径 | 新增真机用例 `shutdown_only_touches_registered_path` PASS（A 关闭 / B 占用与进程均保留） | 通过（本轮补测） |
| C8 | `cleanup_execute` 返回值被接住 | 组件用例「失败项展示路径 + 原因 + 占用进程」通过：红条 `清理完成：成功 0 项 / 失败 1 项` + `C:\Temp\locked.txt` + `notepad.exe (PID 1234)` | 通过（组件级，见 §6 第三档） |
| C9 | 失败明细 + 无占用时不显示重试按钮 | 组件用例「没有占用进程时不显示重试按钮」通过（`PERMISSION_DENIED` 场景无按钮） | 通过（组件级） |
| C10 | confirm 文案含进程清单 + 风险提示；取消不调命令 | 组件用例断言文案含「即将请求关闭以下进程」「notepad.exe (PID 1234)」「未保存的数据可能丢失」；取消后 `cleanup_retry_locked` 未被调用且按钮仍可点 | 通过（组件级） |
| C11 | 重试结果绿条 + 受保护进程提示 + 只传失败项 id | 组件用例：绿条「已关闭 1 个进程，重试成功 1 项」+「以下进程受保护未关闭」+「explorer.exe（protected process）」；实参 `{ itemIds: [1] }` | 通过（组件级） |
| C12 | 三态互斥（cleaning/retrying/previewing 互斥） | **本轮发现缺陷并修复**：重试按钮此前仅 `disabled={retrying}`。修复后组件用例「预览进行中重试按钮禁用，预览结束后恢复」通过；该用例在修复前代码上**失败**（`× 三态互斥（C12）`），修复后通过 | 通过（已修） |
| D13 | 不新增 npm/cargo 依赖 | `git diff --stat origin/main...HEAD` 仅 8 个文件（3 代码 + 2 测试 + 2 文档 + 任务书），无 `Cargo.toml` / `package.json` | 通过 |
| D14 | 不改 `who_is_locking` 签名与 `LockingProcess` 字段 | `git diff` 未见 `LockingProcess` 字段变化；`who_is_locking` 签名不变（仅修 probe 判断的 `Error::code()`→`probe.0`，task-020 bug 修复） | 通过 |
| D15 | 不改 `executor.rs` | `git diff --stat` 无 `engine/src/execute/executor.rs` | 通过 |
| D16 | 六门禁全绿 + engine 测试数 ≥173 | fmt 0 / clippy 0 / cargo test **183 passed + 6 ignored**（基线 173，+10）/ lint 0 / vitest 34 / build 176.39 kB（gzip 56.25 kB，基线 173.98/55.41，+2.41 kB） | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理（已修 / 待拍板 / 已登记） |
|---|---|---|---|---|---|
| 1 | 清理失败后，`cleaning` / `previewing` 进行中「关闭占用进程并重试」按钮仍可点，可并发触发 RmShutdown + 重删（违反 C12 三态互斥） | ① 造占用 → 清理已选 → 红条出现重试按钮；② 勾选任意项 → 点「预览可清理项」（`previewing=true`）或点「清理已选」（`cleaning=true`）；③ 观察重试按钮仍 enabled | 三态任一为真时重试按钮禁用 | 中（可并发触发危险操作，但需用户连续误点） | **已修**（`bugfix/task-049-retry-btn-mutex`，`0129de9`） |
| 2 | 任务书证据清单要求的「只传 1 个 id 时其他文件不受影响」无对应用例（测试用例遗漏） | 代码走查 `engine/tests/locking_shutdown.rs` | 有可复现的隔离性用例 | 低 | **已修**（同分支，`f667c70`） |

## 5. 测试期修复记录（bugfix 子分支）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
|---|---|---|---|---|---|
| 1 | C12 三态互斥缺失 | `bugfix/task-049-retry-btn-mutex` | `0129de9` | `fix(frontend): 重试按钮纳入三态互斥 (#task-049)` | 全量回归：cargo 183 passed / 0 failed；vitest 34 passed；lint 0；build 0；CI run 40045174671 绿 |
| 2 | B7 隔离性用例遗漏 | 同上 | `f667c70` | `test(engine): 补只关闭注册资源不误伤旁路文件的用例 (#task-049)` | 真机 `--ignored` 3/3 PASS；CI run 40045926460 绿 |
| — | 合并回 feature | `feature/task-049-rm-shutdown-locking` | `be60f55` | `merge: 测试期缺陷修复（C12 三态互斥 + B7 隔离用例）合回 feature (#task-049)` | 合并后重跑全量：fmt 0 / clippy 0 / cargo 183 passed / vitest 34 passed / build 0；CI run 40047649773 绿 |

> 修复方式：`retryLocked` 入口守卫 `if (!result || busy) return;`（原 `retrying`），按钮 `disabled={busy}`（原 `retrying`），依赖数组同步改为 `[result, busy, load]`。未改任何既有断言，未用 skip/xfail。

## 6. 遗留问题（三档分流）

### 第一档（已修）
- C12 三态互斥缺失（缺陷 1）。
- B7 隔离性用例遗漏（缺陷 2）。

### 第二档（需用户拍板，状态 blocked）
无。本轮未发现 TRD/PRD 口径冲突、接口语义变更或需要方案取舍的问题。

### 第三档（已登记为已知限制）

1. **GUI 真机点击与 DevTools Console 证据未取得（L3 / L4 的交互部分）**
   - 缺失条件：本会话无活动桌面合成 → WebView2 渲染进程创建即崩溃（Crashpad 持续产出 `.dmp`，应用进程 0 子进程、无 `msedgewebview2.exe`），窗口无可访问 DOM；`CopyFromScreen` 返回「句柄无效」；CDP 9222 因此永不监听（另：wry 0.57 强制 `additionalBrowserArgs`，`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` 被忽略）。
   - 现有补偿：① C8–C12 用**组件级用例**（`app/src/pages/Items.test.tsx`，6 条，jsdom + RTL，mock IPC 信封）覆盖 DOM 行为，且 C12 用例在修复前代码上实测失败、修复后通过；② 后端全链路真机用例 `execute_retry_after_shutdown_deletes_locked_file` 用**真实外部进程占用**跑通「失败 → 关闭 → 重试删除」；③ `Items.tsx` 逐条代码走查。
   - 未覆盖：真实点击手感、confirm 原生弹窗渲染、Console 红色 error 采集、安装态下的实际点击。**建议在具备活动桌面的机器上补做后放行发版。**

2. **打包态资源路径错位（预存在，非本任务引入）**
   - 现象：NSIS 安装版启动后 `[paths] ... scripts_dir=.\scripts`，即 `resource_dir()/rules|scripts` 均不存在。
   - 根因：`tauri.conf.json` 的 `bundle.resources` 用 `../../rules/...` 相对路径，NSIS/WiX 均按 `$INSTDIR\_up_\_up_\rules`（WiX `_up_` 目录）落盘；而 `AppPaths::from_app` 只查 `resource_dir().join("rules")`。
   - 影响：全新用户首次启动拿不到 bundled 规则 → `load_external_deny` 失败 → `cleanup_execute` / `cleanup_retry_locked` 返回 `INTERNAL`（本机因 `%APPDATA%\com.and0114.cclean\rules` 早已存在而未暴露）。
   - 归属：引入于 `16b30e2`（debt-cleanup，早于本任务）；修复需改 `tauri.conf.json` 或 `paths.rs`，两者都在 task-049 的禁止修改清单内。**已登记，建议另开任务修。**

3. **`RmRestart` 未做**：被关闭的进程不会自动重启，用户需自行重开（任务书明确划出范围）。

4. **`RmShutdown` 阻塞最坏 ~30s**：目标不响应关闭请求时等 RM 超时，期间前端只有「关闭并重试中…」无进度反馈（按钮已禁用，不会重复触发）。

5. **RM 会话粒度限制**：同一文件同时被保护进程与普通进程占用时，RM 无法只关普通进程；`closed` 不会记录保护进程，但会话级关闭仍会波及（`explorer.exe` 会被 Windows 自动重启）。

6. **本地 `pnpm --filter app test`（默认 threads 池）偶发 `EPERM ... <tmp>\web\<hash>`**：vitest 写临时模块缓存被拒，随机丢文件并 exit 1（**加用例前的基线跑同样复现**，与本次改动无关）；`--pool=forks --no-file-parallelism` 稳定 34/34 通过；CI（windows-latest）默认配置绿。判为环境问题，不作为不通过依据。

7. **同源测试风险声明**：开发轮与测试轮同机、同工具链、同一模型族，环境级结论（如 WebView2 崩溃）可能同源偏差；本报告所有门禁均本轮独立重跑，未采信开发轮自验数字（比对结果：engine 183/5→本轮 183/6、pnpm 28→本轮 34，差异均可解释为新增用例）。

## 7. 测试结论

- 结论：**通过（附第三档环境限制）**——代码/契约/单元与集成/打包安装/CI 全绿；**GUI 真机点击与 Console 采集因环境无法执行，已在 §6 单列，未伪造任何证据**。
- feature 分支最新 commit：`b1d6118`（代码态 = `be60f55`，其后两次为纯 docs 提交：测试报告 + 状态更新）
- CI 运行号与结论：`ci` run **40047903556**（HEAD b1d6118）`conclusion=success`，20 个步骤全绿；另 40047649773（be60f55）/ 40045926460（f667c70）/ 40045174671（8b84cf4）/ 39967340368（0541235）均 success
- 是否可进入评审：**是**（评审需知悉 §6 第三档 1、2 两项；第 2 项属预存在缺陷，是否随本任务一并处理由评审/调度裁决）
- 移交说明：
  - 分支：`feature/task-049-rm-shutdown-locking` @ `b1d6118`；bugfix 子分支 `bugfix/task-049-retry-btn-mutex` 已合并并删除（本地 + 远端）
  - 测试报告：`docs/test-reports/049-test-report.md`
  - 关键证据：真机 `--ignored` 三例（`execute_retry_after_shutdown_deletes_locked_file` / `shutdown_locked_file_closes_holder` / `shutdown_only_touches_registered_path`）全 PASS；`app/src/pages/Items.test.tsx` 6 条组件用例；NSIS 干净目录安装 + 安装目录零写入
  - 待补：GUI 真机冒烟（需活动桌面机器）
