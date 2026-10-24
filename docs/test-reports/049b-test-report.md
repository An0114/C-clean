# 测试报告：task-049b 护栏本地化绕过修复（重测轮）

> 测试 Agent（提示词 9），独立于 049b 修复轮执行：不复用修复轮自验数字，全部门禁本地重跑。
> 被测分支：`feature/task-049-rm-shutdown-locking` @ `5550f61`（含 049b 修复合并 + 测试轮修复合并）。
> 上游：`docs/reviews/049-review.md`（评审轮 1 不通过）→ `tasks/049b-bugfix-guard-image-name.md`（修复轮）→ 本报告（重测轮）。
> 测试日期：2026-10-24（本机 Windows 11 / 用户 `ANFER-PC\An`，非管理员）。

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
|---|---|---|---|
| dev（本地启动） | `cargo build -p cdisk-cleaner`（debug，`target/debug/cdisk-cleaner.exe`）+ `vite` 5173（HTTP 200）；WebView2 运行时 155.0.4283.62 | 是 | **不通过（环境阻塞）**：窗口句柄与标题 `C-Disk Cleaner` 短暂创建，12s 内 WebView2 渲染进程崩溃（Crashpad 新增 `68f5da47-….dmp`，11:47:22），应用随即退出；无任何属于本应用的 `msedgewebview2.exe` 存活进程；CDP 9222 不监听 |
| 打包安装（干净目录） | `pnpm tauri build` → NSIS `C-DiskCleaner_0.1.0_x64-setup.exe` 4.01 MiB / MSI 6.95 MiB；`/S /D=%TEMP%\cclean-smoke-049b` | 是 | **部分通过**：安装 exit 0、payload 9 文件、数据落 `%APPDATA%\com.and0114.cclean`、安装目录 **9/9 SHA-256 零变化**；GUI 渲染同上崩溃（新增 `1d64ac46-….dmp`，11:55:02） |
| CI | workflow `ci`；run **40059150999**（feature `5550f61`）/ **40057717120**（bugfix `448275a`）/ **40056574881**（feature `0c02485`，049b 合并后基线） | 是 | 绿 |

> 打包命令本地退出码为 1，原因是 `TAURI_SIGNING_PRIVATE_KEY` 未设置（`Error A public key has been found, but no private key`）——**产物已生成**（NSIS 4.01 MiB / MSI 6.95 MiB），仅 `.sig` 缺失；CI 用临时密钥完成签名与产物校验（步骤 15–17 全绿）。属本地环境限制，非代码问题。

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据（日志/截图/报告路径） |
|---|---|---|---|---|
| L1 单元 | engine 全量（非只跑改动文件）+ 前端全量 | `cargo fmt --all -- --check`；`cargo clippy --workspace --all-targets -- -D warnings`；`cargo test -p engine --no-fail-fast`；`pnpm --filter app lint`；`pnpm --filter app exec vitest run --pool=forks --no-file-parallelism`；`pnpm --filter app build` | **全绿**：fmt 0 / clippy 0 / cargo **188 passed, 0 failed, 8 ignored（合计 196）** / lint 0 / vitest **34 passed（6 files）** / build 0（**176.39 kB，gzip 56.25 kB**） | 合并前后各跑一遍，数字一致：`.tmp-test/l1-test.log`、`.tmp-test/regress-test.log`、`.tmp-test/postmerge-test.log`；基线 196 条与修复轮自验一致 |
| L2 集成/契约 | `shutdown_locking_processes` 真机全链路 + 前后端契约 | `cargo test -p engine --test locking_shutdown -- --include-ignored --test-threads=1`（连跑 5 次） | **通过**：5 次均 **8 passed / 0 failed**；`shutdown_refuses_protected_image_holder` 实测 `closed=[]`、`refused=[{pid:37836, appName:"explorer.exe", reason:"protected process"}]`；`shutdown_only_touches_registered_path` 5/5 稳定 | 单跑输出见 §3 A5/C6；连跑 5 次输出见 §3 C7 |
| L3 端到端/UI（dev） | 清理页失败明细 / 占用进程 / 重试 / confirm / 三态互斥 | `vite`(5173) + `target/debug/cdisk-cleaner.exe` | **阻塞（第三档）**：见 §1 dev 行；本轮新增证据为「窗口标题短暂存在 → 12s 内 Crashpad 落盘 → 进程消失」 | Crashpad `68f5da47-416b-4ed2-bb30-30f407535b02.dmp`（11:47:22，6.76 MB）；`MY_WV2=0`；`CDP_9222_LISTEN=0`。**未取得任何截图/Console 证据，不伪造** |
| L4 真机/打包安装 | NSIS 干净目录安装 + 数据目录隔离 + 安装目录零写入 | `pnpm tauri build`；`setup.exe /S /D=%TEMP%\cclean-smoke-049b`；启动安装版；9 文件 SHA-256 前后比对 | **部分通过**：安装 `INSTALLER_EXIT=0`；payload = exe + uninstall.exe + `_up_\_up_\{rules,scripts}`（2 json + 5 ps1）；启动后 `%APPDATA%\com.and0114.cclean\data\cdiskcleaner.db`（438272 B）为数据落点，安装目录 **`INSTALL_DIR_FILES=9 CHANGED=0` → `NO_DIFF`** | 安装目录 `C:\Users\An\AppData\Local\Temp\cclean-smoke-049b`；GUI 渲染崩溃（`1d64ac46-….dmp`，11:55:02） |
| L5 CI 门禁 | 本任务链 3 次运行 | GitHub Actions API 实查 | **绿**：`0c02485` / `448275a` / `5550f61` 全部 `conclusion=success` | run 40056574881、40057717120、40059150999（均 windows-latest 单 job `build`，20 步全绿） |

### L2 契约核对表（逐字段）

| 核对项 | 后端 | 前端 | 结论 |
|---|---|---|---|
| 命令签名 ↔ 实参 | `cleanup_retry_locked(item_ids: Vec<i64>, paths: State<AppPaths>)`（本任务**未改** `commands.rs`） | `api("cleanup_retry_locked", { itemIds })` | 对齐（049b 零改动，回归确认） |
| serde 注解 | `LockingProcess` / `ShutdownOutcome` / `RefusedProcess` / `RetryLockedResp` 全部 `#[serde(rename_all = "camelCase")]`（`locking.rs:37/53/63/78`） | `interface LockingProcess / RetryLockedResp / RefusedProcess` 全 camelCase | 对齐 |
| **`reason` 取值域（049b 变更）** | 5 个稳定文案：`windows service` / `system process` / `self` / `protected process` / **`unknown process`（新增）** | `reason: string` 原样展示（`${r.appName}（${r.reason}）`），无枚举映射 | 对齐；**前端注释此前缺 `unknown process`，本轮已补（`448275a`）** |
| Envelope 解包 | `Result<Envelope<RetryLockedResp>, String>`，空 ids → `DRYRUN_EMPTY` | `api<T>()` 解信封，`code!==0` 抛错 | 对齐（未改） |
| 事件 payload | 本任务未新增/改动 Tauri 事件 | 无新增监听 | 无影响 |
| **mixed 语义（049b 变更）** | `apply_mixed_fail_closed`：`closable` 非空且 `refused` 非空 → `closed=[]`、不调 `RmShutdown` | 黄条「以下进程受保护未关闭」+ headline「已关闭 0 个进程，仍失败 N 项」（tone=fail） | 对齐（走查 `Items.tsx:260-270,392-406`；mixed 时不会误报绿条） |

## 3. 验收标准逐条核对（每条必须有运行时证据）

### A. 护栏改为映像名判定（评审 Bug #1）

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
|---|---|---|---|
| A1 | 新增 `fn image_file_name(pid: u32) -> Option<String>`，`OpenProcess` + `QueryFullProcessImageNameW`，句柄 RAII | `grep -n "fn image_file_name"` → `locking.rs:238`（Windows 实现）/ `:296`（非 Windows 桩）；`OwnedHandle(HANDLE)` + `impl Drop { CloseHandle }`（`:247-256`）；缓冲区 `32768 u16` | 通过 |
| A2 | 判定顺序 `self → pid<=4 → service → 映像名命中 → 取不到 fail-closed → 空名 → 友好名兜底` | `decide()`（`:159-194`）顺序与任务书逐条一致；`None => return Some(REASON_UNKNOWN)` 在 `:184` | 通过 |
| A3 | 不得靠「往名单里加中文名」修复 | `grep -c "资源管理器" engine/src/execute/locking.rs` → **7**，逐行核对：`:24`、`:124` 为注释，`:664/:669/:672/:681/:707` 为测试断言字符串，**生产判定逻辑 0 处** | 通过 |
| A4 | 保护名单不变（12 条，小写比较） | 解析 `PROTECTED_APP_NAMES` → **12 条**：`explorer.exe / svchost.exe / lsass.exe / csrss.exe / wininit.exe / services.exe / winlogon.exe / smss.exe / dwm.exe / registry / msmpeng.exe / cdisk-cleaner.exe`；`matches_protected_name` 内 `to_lowercase` | 通过 |
| A5 | 新增 `refuses_localized_explorer_description` / `refuses_when_image_name_unavailable`，既有 10 条不回归 | `cargo test -p engine --lib execute::locking::tests` → **15 passed**（10 旧 + 5 新），含 `refuses_localized_explorer_description`、`refuses_when_image_name_unavailable`、`mixed_batch_is_fail_closed`、`batch_without_protected_process_closes_all`、`batch_all_refused_stays_refused`；真机单跑 `shutdown_refuses_protected_image_holder` → `fake explorer outcome: ShutdownOutcome { closed: [], refused: [RefusedProcess { pid: 37836, app_name: "explorer.exe", reason: "protected process" }] }` | 通过 |

### B. mixed 场景语义（评审 Bug #2，选 A fail-closed）

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
|---|---|---|---|
| B | 存在被拒进程时整批不调 `RmShutdown`，`closed=[]` + 完整 `refused`，并新增用例 | `apply_mixed_fail_closed`（`:218-226`）实现 `if !closable.is_empty() && !refused.is_empty() { return (Vec::new(), refused) }`；`shutdown_locking_processes` 在 `closable.is_empty()` 时提前返回（`:356-362`），**不调 `session.shutdown()`**；单测 `mixed_batch_is_fail_closed` / `batch_without_protected_process_closes_all` / `batch_all_refused_stays_refused` 全绿；`ShutdownOutcome` 文档注释写明理由 | 通过 |

### C. B7 用例稳定性（评审 Bug #3）

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
|---|---|---|---|
| C6 | 用例能区分「holder 自然退出」与「被 RM 关掉」 | `Holder` 助手含 `.ready` / `.done` 标记 + `CREATE_NO_WINDOW` 独立 console + `poll()` 拿退出码；失败信息带 `b_before` / `b_after` / `holder_b_alive` / `holder_b_exit` / `holder_b_done_marker`（`locking_shutdown.rs:260-325`） | 通过 |
| C7 | 连跑 5 次全绿（`--include-ignored --test-threads=1`） | 5 次均 `test result: ok. 8 passed; 0 failed`（45.57s / 45.37s / 45.22s / 45.47s / 45.32s）；合并回 feature 后**再连跑 5 次**，仍 5×8 passed（45.38 / 45.12 / 45.41 / 45.22 / 45.26s） | 通过（共 10 次连跑，0 失败） |

### D. 卫生

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
|---|---|---|---|
| D8 | 不新增 npm / cargo 依赖；如加 windows feature 须说明 | `git diff 6705493 HEAD -- engine/Cargo.toml` → 仅追加 `"Win32_System_Threading"`（含注释）；`Cargo.lock` / `pnpm-lock.yaml` **零改动** | 通过 |
| D9 | 不改 `who_is_locking` 签名 / `LockingProcess` 字段 / `executor.rs` | `git diff --name-only 6705493 HEAD` 中无 `executor.rs` / `commands.rs` / `lib.rs` / `tauri.conf.json`；`LockingProcess` 三字段（`pid` / `app_name` / `service_name`）未变 | 通过 |
| D10 | 六门禁全绿，engine 条数 ≥ 183+新增 | fmt 0 / clippy 0 / **196 条**（188 passed + 8 ignored）/ lint 0 / vitest 34 / build 176.39 kB gzip 56.25 kB | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理（已修 / 待拍板 / 已登记） |
|---|---|---|---|---|---|
| 1 | `refuses_localized_explorer_description` 用 **PID 1234**（合法 PID）调 `is_protected_process`，断言结果取决于「1234 是否被某个普通进程占用」：未被占用 → `OpenProcess` 失败 → fail-closed 通过；一旦被普通进程占用 → 映像名取到、中文名兜底不命中 → 返回 `None` → 用例失败 | ① 让 PID 1234 被任意普通进程占用（或直接在另一台机器跑）；② `cargo test -p engine --lib execute::locking::tests`；③ 观察 `refuses_localized_explorer_description` 失败 | 用例结论只由被测逻辑决定，不随环境 PID 分布漂移 | 中（护栏唯一「本地化名」用例，机器一变就假红/假绿） | **已修**（`bugfix/task-049-guard-test-stability`，`a740dea`）：pid 改 `u32::MAX - 1`（必然不存在），断言收紧为 `Some(REASON_UNKNOWN)` |
| 2 | 前端 `RefusedProcess.reason` 注释只列 4 种取值，缺 049b 新增的 `unknown process` | 读 `app/src/pages/Items.tsx:52` | 注释与后端取值域一致（5 种） | 低（注释过期，易误导后续维护） | **已修**（同分支，`448275a`） |

> 缺陷 1 的实测依据：修复前本机 `Get-Process -Id 1234` → `PID1234_NOT_EXISTS`，即该用例当时**靠「PID 1234 恰好空闲」通过**，属偶然前提。

## 5. 测试期修复记录（bugfix 子分支）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
|---|---|---|---|---|---|
| 1 | 本地化护栏用例依赖环境 PID | `bugfix/task-049-guard-test-stability` | `a740dea` | `test(engine): 本地化护栏用例改用必然不存在的 pid，消除环境依赖 (#task-049)` | 全量回归：fmt 0 / clippy 0 / **196 条**（188+8）/ vitest 34 / lint 0 / build 176.39 kB；CI run 40057717120 绿 |
| 2 | `reason` 注释缺 `unknown process` | 同上 | `448275a` | `docs(frontend): 补齐 RefusedProcess.reason 的 unknown process 说明 (#task-049)` | 同上 |
| — | 合并回 feature | `feature/task-049-rm-shutdown-locking` | `5550f61` | `merge: 测试期修复（护栏用例去环境依赖 + reason 注释）合回 feature (#task-049)`（`--no-ff`） | 合并后重跑全量：fmt 0 / clippy 0 / 196 条 / vitest 34 / build 0；**真机连跑 5 次全绿**；CI run 40059150999 绿 |

> 约束遵守：未修改/删除/弱化任何既有断言；缺陷 1 的断言是**收紧**（`is_some()` → `Some(REASON_UNKNOWN)`），非放宽；无 skip/xfail；子分支已删除（本地 + 远端），仅一层。
> 该修复属「测试资产缺陷」，按约束 1 单列 `test(...)` 提交并在此说明理由，供评审复核。

## 6. 遗留问题（三档分流）

### 第一档（已修）
- 缺陷 1（护栏用例 PID 环境依赖）、缺陷 2（`reason` 注释过期）。

### 第二档（需用户拍板，状态 blocked）
- **无。** 本轮未发现 TRD/PRD 口径冲突、接口语义变更或方案取舍问题。

### 第三档（已登记为已知限制）

1. **GUI 真机点击与 DevTools Console 证据未取得（L3 / L4 的交互部分）**
   - 缺失条件：本会话**无活动桌面合成**。本轮实测：应用启动后窗口标题 `C-Disk Cleaner` 与句柄短暂存在，但 12s 内 WebView2 渲染进程崩溃——Crashpad 分别在 `11:47:22`（dev）与 `11:55:02`（安装版）落盘 6.7 MB / 6.8 MB `.dmp`；崩溃后无任何属于本应用的 `msedgewebview2.exe`（`MY_WV2=0`），应用进程退出；CDP 9222 不监听（wry 0.57 强制 `additionalBrowserArgs`，`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` 被忽略）。
   - 现有补偿：① C8–C12 由 `app/src/pages/Items.test.tsx` 6 条组件用例（jsdom + RTL，mock IPC 信封）覆盖，C12 用例在修复前代码上实测失败、修复后通过；② 后端真机用例 `shutdown_refuses_protected_image_holder` / `shutdown_only_touches_registered_path` / `execute_retry_after_shutdown_deletes_locked_file` 用真实外部进程跑通护栏与「失败 → 关闭 → 重试删除」；③ `Items.tsx` 逐条代码走查（含 mixed 场景 headline/tone 推演）。
   - 未覆盖：真实点击手感、`window.confirm` 原生弹窗渲染、Console 红色 error 采集、安装态实际点击。**建议在具备活动桌面的机器上补做后放行发版。**

2. **打包态资源路径错位（预存在，非 task-049/049b 引入，仍未修）**
   - 现象：安装目录出现 `_up_\_up_\rules` / `_up_\_up_\scripts`，而 `AppPaths` 只查 `resource_dir()/rules` → 安装版 `scripts_dir=.\scripts`。
   - 归属：引入于 `16b30e2`；修复需改 `tauri.conf.json` 或 `paths.rs`（均在本任务链禁止清单内）。**已登记，建议另开任务。**

3. **`RmRestart` 未做**：被关闭的进程不会自动重启（任务书明确划出范围）。

4. **`RmShutdown` 阻塞最坏 ~30s**：目标不响应关闭请求时等 RM 超时，期间前端只有「关闭并重试中…」无进度反馈。

5. **`RmShutdown` 会连带关闭同 console / 进程组的兄弟进程**（049b 实测定位）：已写入 `locking.rs` 文件头已知限制 4；集成用例用 `CREATE_NO_WINDOW` 隔离 console 后才稳定。

6. **mixed fail-closed 的用户可见代价（本轮观察，未改代码）**：mixed 占用时点「重试」，`confirm` 里列了可关进程（如 notepad），实际**一个都不关**（`closed=[]`），前端只显示「已关闭 0 个进程，仍失败 N 项」+ 黄条受保护列表，**没有「因存在受保护进程，本次未关闭任何进程」的解释文案**。任务书 A 方案已接受该代价（理由是「宁可不关」且失败明细不丢），文案是否补属产品决策，故不改代码，登记为观察项。

7. **本地打包签名**：`TAURI_SIGNING_PRIVATE_KEY` 未配置，本地 `pnpm tauri build` 产物可生成但退出码 1、无 `.sig`；CI 用临时密钥完成签名校验。

### 同源测试风险声明

- 049b 的三项缺陷修复由**另一个 Agent 轮次**完成（`199527a` / `c34af2c`），本轮测试独立重跑，未采信其自验数字（比对：engine 196 条与其备注一致，vitest 34 / build 176.39 kB 一致，均本轮实测复现）。
- **但缺陷 1、2 的修复由本测试轮自行完成（`a740dea` / `448275a`），其验证属同源**：修复后用例通过、全量回归 196 条、CI 绿，均由同一会话执行。建议评审对 `a740dea` 的断言收紧单独复核（理由：原 `is_some()` 在 fail-closed 下对任意无效 pid 恒真，收紧到 `Some(REASON_UNKNOWN)` 才能区分「认不出」与「识别后拒绝」）。
- 环境级结论（WebView2 崩溃）与修复轮同机同工具链，可能存在同源偏差；该结论已按第三档登记，未作为通过依据。

## 7. 测试结论

- 结论：**通过（附第三档环境限制）**——049b 三项评审否决/阻塞缺陷（护栏本地化绕过 / mixed 语义失真 / B7 不稳定）均已实测闭环；代码、契约、单元与真机集成、打包安装、CI 全绿；GUI 真机点击与 Console 采集因环境无法执行，已在 §6 单列，未伪造任何证据。
- feature 分支最新 commit：`5550f61`（`0c02485` + 测试轮修复 `a740dea`/`448275a` 的 `--no-ff` 合并）
- CI 运行号与结论：`ci` run **40059150999**（`5550f61`）`conclusion=success`；另 40057717120（`448275a`）、40056574881（`0c02485`）、40055549993（`b1055aa`，049b 合并后）均 success
- 是否可进入评审：**是**（评审需知悉 §6 第三档 1、2、6 三项；第 2 项属预存在缺陷，是否随本任务链一并处理由评审/调度裁决）
- 移交说明：
  - 分支：`feature/task-049-rm-shutdown-locking` @ `5550f61`；bugfix 子分支 `bugfix/task-049-guard-test-stability` 已合并并删除（本地 + 远端），临时 PR #14 已关闭
  - 测试报告：`docs/test-reports/049b-test-report.md`（本文件）；上一轮报告 `docs/test-reports/049-test-report.md`
  - 关键证据：真机 `--include-ignored --test-threads=1` **10 次连跑（合并前后各 5 次）全绿**；`shutdown_refuses_protected_image_holder` 实测 `closed=[]` + `refused=[protected process]`；NSIS 干净目录安装 + 安装目录 9/9 文件 SHA-256 零变化
  - 待补：GUI 真机冒烟（需活动桌面机器）
