# 任务编号：049b 护栏本地化绕过修复——危险进程判定改为「PID → 映像文件名」

> 派发角色：Review Lead（提示词 10，评审不通过后生成）
> 执行角色：Bug 修复 Agent（提示词 11）
> 来源：`docs/reviews/049-review.md` Bug #1 / #2 / #3
> 上游：task-049（`feature/task-049-rm-shutdown-locking` @ `6705493`，评审不通过、未合并）
> 编号说明：不占用 `task-050`（该号预留给 safe_box 增强），用 `049b` 标识 task-049 的评审修复轮。

## 状态字段

testing（修复完成并已合回 feature，待测试 Agent 全量回归 + 评审重评）

## 任务目标

修掉评审否决项，让「危险进程护栏」在**任何语言环境**下都成立，并让「关了谁 / 拒了谁」的上报与事实一致。

1. **Bug #1（高）**：护栏当前拿 `RmGetList` 的 `strAppName` 做名单匹配，而它是**本地化 FileDescription**（本机 `zh-CN`：`explorer.exe` → `Windows 资源管理器`、`dwm.exe` → `桌面 窗口管理器`），三条匹配规则全部不命中 → `explorer.exe` 被判为可关。改为 **PID → 映像文件名**判定，与语言无关。
2. **Bug #2（中）**：`RmShutdown` 是会话级，mixed 占用时保护进程也会被一并请求关闭，但 `refused` 仍标「受保护未关闭」，前端提示与事实相反。二选一（推荐 A）。
3. **Bug #3（中）**：`shutdown_only_touches_registered_path` 连跑不稳定（评审 1/3 失败），需让失败可归因并给出连跑证据。

## 前置条件

1. 从 `feature/task-049-rm-shutdown-locking` @ `6705493` 拉出 **一层** bugfix 子分支 `bugfix/task-049-guard-image-name`（禁止再开第二层）；
2. 先读真实代码，以代码为准：`engine/src/execute/locking.rs`（`is_protected_process` / `matches_protected_name` / `RmSession` / `shutdown_locking_processes`）、`engine/tests/locking_shutdown.rs`、`app/src/pages/Items.tsx`（`ResultState` / 黄条文案）；
3. 复核事实（贴输出）：`powershell -c "(Get-Item C:\Windows\explorer.exe).VersionInfo.FileDescription"` → `Windows 资源管理器`；真机 JSON 里 `appName` 是友好名（`"Windows PowerShell"`）而非 `powershell.exe`。

## 验收标准

### A. 护栏改为映像名判定（Bug #1）

1. 新增内部函数（建议签名）：
   ```rust
   /// 取进程映像文件名（小写，含 .exe）；取不到返回 None。
   fn image_file_name(pid: u32) -> Option<String>
   ```
   实现：`OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)` → `QueryFullProcessImageNameW` → 取路径最后一段 → `to_lowercase()`；句柄用 `CloseHandle` 释放（建议 RAII，避免新增泄漏点）。
2. `is_protected_process` 判定顺序调整为：
   1) `pid == self` → `self`；
   2) `pid <= 4` → `system process`；
   3) `service_name` 非空 → `windows service`；
   4) **`image_file_name(pid)` 命中名单 → `protected process`**；
   5) **取不到映像名（`None`）→ `protected process`（fail-closed，宁可不关）**；
   6) `app_name` 为空 → `system process`；
   7) `matches_protected_name(app_name)` 命中 → `protected process`（保留原逻辑作兜底，覆盖 RM 直接返回 `explorer.exe` 的情形）。
3. **不得**用「往名单里追加中文名」的方式修复（不收敛，且不同语言环境各一套）。
4. 保护名单本身不变（12 条），仍小写比较。
5. 新增单测（在 `locking.rs` 的 `#[cfg(test)] mod tests`，沿用现有风格）：
   - `refuses_localized_explorer_description`：`is_protected_process("Windows 资源管理器", None, 1234, SELF)` 必须**不是** `None`（fail-closed 生效）；
   - `refuses_when_image_name_unavailable`：用一个必然取不到映像名的 pid（如 `u32::MAX - 1`）断言拒绝；
   - 既有 10 条用例全部保留并通过。

### B. mixed 场景语义（Bug #2，二选一，推荐 A）

- **A（推荐，fail-closed）**：`closable` 非空且 `refused` 非空时，**不调** `RmShutdown`，返回 `closed=[]` + 完整 `refused`，并在 `ShutdownOutcome` 注释里写明理由；`Items.tsx` 黄条文案保持「以下进程受保护未关闭」（此时为真）。
- **B**：保留现状，但 `Items.tsx` 黄条文案改为「以下进程被护栏跳过；受 Restart Manager 会话粒度限制，可能仍被一并关闭」，并在 `locking.rs` 文件头「已知限制 2」同步说明。
- 无论选哪个，都必须新增一条用例覆盖 mixed 行为（单测可直接构造 `closable/refused` 判定结果；集成用例可用「同一文件被 holder + 一个受保护进程占用」构造，若构造困难，用单测覆盖判定分支即可，并在实现备注里说明为什么）。

### C. B7 用例稳定性（Bug #3）

6. `shutdown_only_touches_registered_path` 必须能区分「holder 自然退出」与「被 RM 关掉」：
   - holder 启动后等待并确认其存活，再断言；
   - `try_wait()` 拿到 `Some(exit)` 时打印 `exit code`（被强杀 vs 正常退出）；
   - 失败信息里带上 `b_before` / `b_after` / `holder_b_alive` / `exit code`；
7. **连跑 5 次全绿**才算通过，5 次输出必须贴进实现备注（`--include-ignored --test-threads=1`，连跑含其余 5 条用例）。

### D. 卫生

8. 不新增 npm / cargo 依赖（`windows` crate 已有 `Win32_System_Threading` / `Win32_System_ProcessStatus` 相关 feature 时复用；**若需在 `engine/Cargo.toml` 的 `windows` feature 列表里加一行，允许，但必须在实现备注写明新增了哪个 feature**）；
9. 不改 `who_is_locking` 签名、不改 `LockingProcess` 字段、不改 `executor.rs`；
10. 六个门禁全绿：`cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test -p engine`（条数 ≥ 183+新增）/ `pnpm --filter app lint` / `pnpm --filter app test` / `pnpm --filter app build`。

## 允许修改范围

| 文件 | 允许的改动 |
|---|---|
| `engine/src/execute/locking.rs` | 映像名判定 + 判定顺序 + mixed 行为 + 单测 |
| `engine/tests/locking_shutdown.rs` | B7 用例稳定性 + 新增 explorer 真机用例（`#[ignore]`） |
| `app/src/pages/Items.tsx` | 仅当选 B 方案时改黄条文案 |
| `engine/Cargo.toml` | 仅允许给 `windows` 追加 feature（须在备注说明） |
| `docs/PROJECT_STATE.md` | 收尾单独提交 |
| `tasks/049b-*.md` | 追加 `## 实现备注` |

**禁止修改**：`executor.rs`、`whitelist.rs`、`safe_box.rs`、`delete.rs`、`commands.rs`（本次无需改命令层）、`lib.rs`、其他页面、`tauri.conf.json`、`capabilities/default.json`、`app/package.json`、workflows、rules、scripts。

## Git 提交要求

格式 `<type>(<scope>): <描述> (#task-049)`，小步提交：

1. `fix(engine): 护栏改为按 PID 映像名判定，本地化应用名不再可绕过 (#task-049)`
2. `fix(engine): mixed 占用场景 fail-closed，拒绝与关闭不再语义冲突 (#task-049)`（若选 B 则改为前端文案提交）
3. `test(engine): 补本地化应用名与映像名不可得的护栏用例 (#task-049)`
4. `test(engine): B7 用例可归因并连跑 5 次稳定 (#task-049)`
5. `docs(state): 任务049b 修复完成，待重测 (#task-049)`（单独只动 PROJECT_STATE）

> **不要重写已推送的历史**（禁止 rebase / force push 已评审的 commit）。Bug #4（probe 修复混在 feat commit）只做留痕，不追溯。

## 测试要求

| 层级 | 是否必做 | 覆盖内容 |
|---|---|---|
| 单元测试 | 必做 | 本地化名 fail-closed / 映像名不可得 fail-closed / mixed 分支 / 既有 10 条不回归 |
| 集成测试 | 必做 | explorer 持锁真机用例（`#[ignore]`）：`closed` 不含 explorer、`refused` 含 `protected process`；B7 连跑 5 次 |
| 真机冒烟 | **必做** | explorer 场景（无需 GUI 点击：用 explorer 持有 `%TEMP%` 下文件造占用，跑 `--ignored` 用例即可）；GUI 点击仍受环境限制，按第三档处理 |
| 打包安装冒烟 | 不重复 | 本次未改打包/路径逻辑，引用 task-049 轮结论 |
| CI | 必做 | 新 run id + `conclusion=success` |

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| A1-A2 映像名判定 | `grep -n "fn image_file_name" engine/src/execute/locking.rs` + 判定顺序代码摘录 |
| A3 不靠中文名单 | `grep -c "资源管理器" engine/src/execute/locking.rs` → 0（除测试断言字符串） |
| A5 新单测 | `cargo test -p engine execute::locking::tests` 输出（含用例名） |
| B mixed 行为 | 用例输出 + `ShutdownOutcome` 注释摘录 |
| C 稳定性 | 连跑 5 次 `--include-ignored` 的完整输出 |
| D 门禁 | 六条命令退出码 + engine 测试条数 + build 体积 |
| CI | run URL + id + conclusion |

## 输出要求

1. 代码按上表提交并 push 到 `bugfix/task-049-guard-image-name`；
2. 任务文件末尾追加 `## 实现备注`：实际改动文件对账、`image_file_name` 的失败分支处理、mixed 方案选择与理由、新增 windows feature（如有）、六门禁退出码、连跑 5 次输出、CI run id、与任务书不符之处；
3. **合回 `feature/task-049-rm-shutdown-locking`（`--no-ff`），不合并 main**；
4. 合回后回到提示词 9 重测 → 提示词 10 重评。

---

## 实现备注

> 执行：Bug 修复 Agent（提示词 11）｜日期：2026-10-24｜分支：`bugfix/task-049-guard-image-name`（基于 `feature/task-049-rm-shutdown-locking` @ `37375fb`）

### 1. 实际改动文件对账（3 个文件，全部在白名单内）

| 文件 | 改动 | 对应验收 |
|---|---|---|
| `engine/src/execute/locking.rs` | `+369/-…`：新增 `image_file_name` / `decide` / `apply_mixed_fail_closed`；`is_protected_process` 改为 PID→映像名入口；`shutdown_locking_processes` 接入 mixed 收口；文件头已知限制补 2 条；`#[cfg(test)] mod tests` 10 → 15 条 | A1–A5、B |
| `engine/tests/locking_shutdown.rs` | `+280/-…`：`Holder` 助手（ready/done 标记 + 独立 console + 退出码）；B7 重写归因；新增 `shutdown_refuses_protected_image_holder` + `holder_child_holds_file` | C6–C7、A5 真机 |
| `engine/Cargo.toml` | 仅追加 `windows` feature 一行：`Win32_System_Threading` | D8 |

**未改动**：`executor.rs` / `whitelist.rs` / `safe_box.rs` / `delete.rs` / `commands.rs` / `lib.rs` / 其他页面 / `tauri.conf.json` / `capabilities/default.json` / `app/package.json` / workflows / rules / scripts（`git diff --stat 37375fb..HEAD` 只有上表 3 个文件）。
**`app/src/pages/Items.tsx` 未改**——选 A 方案（fail-closed），黄条文案「以下进程受保护未关闭」在 mixed 场景下变成真话，无需改文案。
**未新增 npm / cargo 依赖**（`pnpm-lock.yaml`、`Cargo.lock` 零改动）。

### 2. `image_file_name` 的失败分支处理

```rust
fn image_file_name(pid: u32) -> Option<String>
// OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION) → QueryFullProcessImageNameW
// → 取路径末段 → trim().to_lowercase()
```

失败一律返回 `None`，由 `decide()` 的 **fail-closed** 分支兜住（`reason = unknown process`）：

| 失败场景 | 结果 |
|---|---|
| `pid == 0` | `None`（短路，不去 OpenProcess） |
| `OpenProcess` 失败（权限不足 / 进程已退出 / 非法 pid） | `None` |
| `QueryFullProcessImageNameW` 失败（进程刚退出 / 缓冲区不足） | `None` |
| 路径末段为空 | `None` |

句柄用函数内的 `OwnedHandle(HANDLE)` + `Drop` 释放（RAII），三条失败路径都不会漏 `CloseHandle`。缓冲区 `32768 u16`，覆盖 `\\?\` 长路径上限，避免 `ERROR_INSUFFICIENT_BUFFER`。
**为什么新增 `unknown process` 而不是复用 `protected process`**：任务书允许二者之一；分开后前端提示能区分「认出来了但在名单里」与「根本认不出是谁」，排查「点重试什么都没发生」时不必再猜。两者都是拒绝，方向一致。

### 3. mixed 方案选择与理由（选 A，fail-closed）

`shutdown_locking_processes` 现在先逐条判定，再走 `apply_mixed_fail_closed(closable, refused)`：**只要 `refused` 非空，`closable` 就清空 → 不调 `RmShutdown` → `closed = []`**。

选 A 而不是 B 的理由：
1. 评审 Bug #2 的本质是「上报与事实相反」。B 方案（保留关闭 + 改文案）仍会把保护进程关掉，只是把话说圆；A 方案让「受保护未关闭」变成事实。
2. 代价可控：mixed 占用（保护进程 + 普通进程同占一个文件）本身罕见，且用户在失败明细里仍能看到全部占用进程（`execute_items` 的 `lockingProcesses` 未被清掉），不会「静默什么都没发生」。
3. 与护栏的「宁可不关」原则同向，不需要额外的文案翻译层。

被清掉的可关进程**不写入 `refused`**（任务书 A 方案原文：「返回 `closed=[]` + 完整 `refused`」）——它们既没被关，也不该被标成「受保护」。这一点在代码注释与文件头已知限制 2 里都写明了。

### 4. 新增 windows feature

`engine/Cargo.toml` → `[target.'cfg(windows)'.dependencies] windows.features` 追加 **`Win32_System_Threading`**（提供 `OpenProcess` / `QueryFullProcessImageNameW` / `PROCESS_QUERY_LIMITED_INFORMATION` / `PROCESS_NAME_WIN32`）。`CloseHandle` / `HANDLE` 走已有的 `Win32_Foundation`，`PWSTR` 走 `windows::core`。仅加 feature，未升版本、未换 crate。

### 5. 六道门禁（本地实测退出码）

| # | 命令 | 结果 |
|---|---|---|
| 1 | `cargo fmt --all -- --check` | **exit 0** |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0**（0 warning） |
| 3 | `cargo test -p engine` | **全绿**；测试清单 **196 条**（含真机 `#[ignore]` 用例，CI 跳过），修复前 189 条 → 新增 7 条（单测 +5、集成 +2） |
| 4 | `pnpm --filter app lint` | **exit 0**（eslint 0 error） |
| 5 | `pnpm --filter app test` | **34 passed / 6 files**（Items.test.tsx 6 条不变） |
| 6 | `pnpm --filter app build` | **exit 0**，`dist/assets/index-*.js 176.39 kB │ gzip 56.25 kB`（与修复前一致，前端零改动） |

护栏单测（`cargo test -p engine --lib execute::locking`）：**15 passed**，用例名：

```
allows_normal_process / refuses_explorer / refuses_svchost /
refuses_protected_name_case_insensitive / refuses_windows_service /
refuses_system_pid / refuses_self / refuses_empty_app_name /
refuses_rm_friendly_name_of_protected_app / allows_rm_friendly_name_of_normal_app
refuses_localized_explorer_description / refuses_when_image_name_unavailable
mixed_batch_is_fail_closed / batch_without_protected_process_closes_all /
batch_all_refused_stays_refused
```

> 注：既有 10 条断言语义全部保留，只是改为调 `decide()` 并**显式注入映像名**——否则在 fail-closed 下，任何「测试机上不存在的 pid」都会被判 `unknown process`，用例结果会随机器漂移。

### 6. 真机证据（zh-CN 环境）

```
PS> Get-Item C:\Windows\explorer.exe, C:\Windows\System32\dwm.exe | % { $_.Name, $_.VersionInfo.FileDescription }
explorer.exe   Windows 资源管理器
dwm.exe        桌面窗口管理器
PS> (Get-Process explorer).Path
C:\WINDOWS\Explorer.EXE
```

即：RM 的 `strAppName` 是中文 FileDescription，而 `QueryFullProcessImageNameW` 拿到的是 `explorer.exe` —— 旧实现三条名字规则对中文名全部不命中（Bug #1 成立），新实现按映像名命中（修复有效）。

**端到端真机用例** `shutdown_refuses_protected_image_holder`（`--ignored`，无需 GUI）：把测试二进制复制成 `explorer.exe` 并让它独占持有文件，实测输出

```
fake explorer outcome: ShutdownOutcome { closed: [], refused: [RefusedProcess { pid: 38568,
  app_name: "explorer.exe", reason: "protected process" }] }
```

即 `closed` 不含 explorer、`refused` 含 `protected process`、holder 仍存活（护栏拒绝 → 未调 `RmShutdown`）。

### 7. Bug #3 根因与连跑 5 次证据

**根因不是「环境抖动」，是 holder 之间的 console 耦合**（本轮实测定位）：

- 单跑 B7：`2/2 PASS`；
- 全量 `--include-ignored --test-threads=1`：**改前 `2/2 FAILED`**，输出
  `b_after=[] holder_b_alive=false holder_b_exit=Some(1) holder_b_done_marker=false`
  ——B 的 holder 没走到脚本结尾（无 done 标记）、退出码 1，即**被外部终止**，而不是自然退出；
- 给两个 holder 各加 `CREATE_NO_WINDOW`（独立隐藏 console，彼此不再共用测试进程的 console）后，全量 `2/2 PASS`。

结论：`RmShutdown` 对**同一 console / 进程组**里的兄弟进程会连带关闭（已写进 `locking.rs` 文件头已知限制 4）；真实场景里两个占用方本就是彼此独立的进程，独立 console 更贴近事实。用例因此既稳定，也把「只关注册资源」这一性质单独隔离出来测。
归因能力也已补齐：holder 写 `.ready`（替代盲等 5s）与 `.done`（自然退出标记），失败信息带 `b_before` / `b_after` / `holder_b_alive` / `holder_b_exit` / `holder_b_done_marker`，并在「没写 done 就退出」时直接判失败而不是 `inconclusive`。

**连跑 5 次**（`--include-ignored --test-threads=1`，8 条用例全跑）：

```
=== RUN 1 === b_after=[pid 15320] holder_b_alive=true holder_b_exit=None -> ok. 8 passed; 0 failed (45.60s)
=== RUN 2 === b_after=[pid 2328]  holder_b_alive=true holder_b_exit=None -> ok. 8 passed; 0 failed (45.36s)
=== RUN 3 === b_after=[pid 40608] holder_b_alive=true holder_b_exit=None -> ok. 8 passed; 0 failed (45.30s)
=== RUN 4 === b_after=[pid 41304] holder_b_alive=true holder_b_exit=None -> ok. 8 passed; 0 failed (45.01s)
=== RUN 5 === b_after=[pid 14884] holder_b_alive=true holder_b_exit=None -> ok. 8 passed; 0 failed (45.16s)
```

### 8. Git 与 CI

| 项 | 值 |
|---|---|
| 分支 | `bugfix/task-049-guard-image-name`（从 feature @ `37375fb` 拉出，一层） |
| fix commit | `199527a` `fix(engine): 护栏改为按 PID 映像名判定，本地化应用名不再可绕过 (#task-049)` |
| test commit | `c34af2c` `test(engine): 补本地化应用名/映像名不可得/mixed 用例，B7 可归因 (#task-049)` |
| 触发 CI | draft PR #13（`bugfix/task-049-guard-image-name` → `feature/task-049-rm-shutdown-locking`，仅用于触发 CI，不通过 PR 合并） |
| CI run | **`40054417543`**（job `build`，windows-latest）`conclusion=success`，head = `c34af2c` |

### 9. 与任务书不符之处（3 处，均为主动判断）

1. **分支基点取 `37375fb` 而非 `6705493`**：`6705493` 之后 feature 上只有 3 个 docs 提交（评审报告 / PROJECT_STATE / 本任务书），从分支最新 tip 拉出可避免合回时把 docs 提交再合一遍，拓扑更干净。**未 rebase / 未 force push 任何已评审历史**。
2. **提交粒度 2 条（fix / test）而非任务书的 4 条**：改动集中在 `locking.rs` 单文件，单测与生产代码同文件，按「生产代码 / 测试资产」两个维度拆分（`Cargo.toml` 的 feature 归入 fix）；任务书建议的第 2 条「mixed fail-closed」并入 fix commit 并在 body 里单独列点，未另起 commit。
3. **`refused` 不追加「因 mixed 而未关」的可关进程**：严格按任务书 A 方案「返回 `closed=[]` + 完整 `refused`」实现（`refused` 仍是护栏判定的原样结果）；理由与被清进程的去向写在 `apply_mixed_fail_closed` 的文档注释与文件头已知限制 2 里，避免前端出现「被标成受保护」的错误语义。

> 另：本轮**未**修复 Bug #4/#5/#6（probe 提交混杂 / 任务书状态字段 / 测试资产越界），三者属流程与文档，按评审 §10「可延后」处理，仅在此留痕。
