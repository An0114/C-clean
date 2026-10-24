# 任务编号：049b 护栏本地化绕过修复——危险进程判定改为「PID → 映像文件名」

> 派发角色：Review Lead（提示词 10，评审不通过后生成）
> 执行角色：Bug 修复 Agent（提示词 11）
> 来源：`docs/reviews/049-review.md` Bug #1 / #2 / #3
> 上游：task-049（`feature/task-049-rm-shutdown-locking` @ `6705493`，评审不通过、未合并）
> 编号说明：不占用 `task-050`（该号预留给 safe_box 增强），用 `049b` 标识 task-049 的评审修复轮。

## 状态字段

bugfix_pending

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
