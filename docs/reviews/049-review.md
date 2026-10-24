# 评审报告：task-049 占用进程 RmShutdown（清理失败从「只报告」升级为「可处置」）

> 评审角色：Review Lead（提示词 10）
> 评审时间：2026-10-24
> 分支：`feature/task-049-rm-shutdown-locking`
> HEAD：`6705493`（代码态 = `be60f55`，其后 4 个提交均为 docs）
> 评审结论：**不通过**（保留 feature 分支，不合并 main）

---

## 1. 输入核验

| 输入项 | 状态 | 事实核对（本轮实查，非引用报告） |
|---|---|---|
| 测试报告 | ✅ | `docs/test-reports/049-test-report.md`（含 L1-L5 分层、契约核对表、缺陷清单、三档分流） |
| CI 运行号与结论 | ✅ | `ci` run **40048746500**（job `build`，windows-latest）`conclusion=success`，head = `6705493`（= 当前 HEAD）。报告内引用的 run 40048375380 对应 `e23cdce`，亦为 success；两者之间仅 2 个 docs 文件差异（`git diff --stat e23cdce..HEAD` = PROJECT_STATE + 测试报告各 2/6 行） |
| feature 分支最新 commit | ✅ | `6705493`（本地 = 远端 `refs/heads/feature/task-049-rm-shutdown-locking`，已 fetch 核对一致） |

> 输入齐全，不构成「缺任一项直接打回」的情形。但注意：报告 §7 自述「后续仅记录本报告的 docs 提交不再回写 CI 结论」，而 HEAD 已前进 1 个 docs 提交（`6705493`）；本轮已自行补查该提交的 CI 结论（run 40048746500 success），不因此判不通过，但**建议后续测试报告以「最后一次改动代码的 commit」为 CI 锚点，并把纯 docs 提交单列**，避免评审侧二次核对。

### 1.1 分支拓扑

| 项 | 值 |
|---|---|
| main / origin/main | `401f336`（`Create 2026-10-23.md`，仅动 `.workbuddy-ai/memory/2026-10-23.md`，与本任务无关） |
| merge-base | `e3e74e0` |
| main...HEAD | 落后 1 / 领先 17 |
| main 是否被改动 | **否**（评审前未合并，符合 Git 权限约束） |
| 本任务 bugfix 子分支 | `bugfix/task-049-retry-btn-mutex` 已合回 feature 并删除（本地 + 远端均无残留，`git ls-remote` 实查） |

> main 比 feature 多 1 个提交（memory 日志），与本任务文件零交集；合并时不会冲突，不构成阻碍。但按任务书 §Git 分支「从最新 main 拉出」的要求，feature 应在合并前 rebase 到 `401f336`（仅影响后续 merge commit 的可读性，非否决项）。

---

## 2. Git 提交记录核对结果

17 个提交，全部关联 `#task-049`，`type(scope)` 全部规范（脚本逐条正则校验，0 条不合规），无敏感文件（`.env/.key/.pem/.pfx/credential/id_rsa` 全量文件名扫描，0 命中）。

| # | commit | 提交信息 | 改动文件 | 原子性 |
|---|---|---|---|---|
| 1 | `20d1b04` | feat(engine): 新增 shutdown_locking_processes 与危险进程护栏 | locking.rs | ⚠️ 见 §2.1 |
| 2 | `081d0ea` | test(engine): 补 RmShutdown 护栏与无占用用例 | tests/locking_shutdown.rs | ✅ |
| 3 | `f3a6828` | feat(tauri): 新增 cleanup_retry_locked 命令 | commands.rs, lib.rs | ✅ |
| 4 | `69b6b33` | feat(frontend): 清理失败展示占用进程并支持关闭后重试 | Items.tsx | ✅ |
| 5 | `7cfe53e` | test(engine): 补关闭后重试删除的端到端用例 | tests/locking_shutdown.rs | ✅ |
| 6 | `3b5cddd` | test(engine): 输出 RetryLockedResp 真实 JSON 便于契约核对 | tests/locking_shutdown.rs | ✅ |
| 7 | `1b467ba` | docs(task): 补任务049实现备注 | tasks/049-*.md | ✅ |
| 8 | `2063303` | docs(state): 任务049完成，占用进程 RmShutdown 就绪 | PROJECT_STATE.md | ✅（单文件） |
| 9 | `0541235` | docs(task): 补记任务049 CI 全绿 | tasks/049-*.md | ✅ |
| 10 | `0129de9` | fix(frontend): 重试按钮纳入三态互斥 | Items.tsx | ✅（bugfix 子分支） |
| 11 | `8b84cf4` | test(frontend): 补 Items 清理失败与关闭重试交互用例 | Items.test.tsx | ✅ |
| 12 | `f667c70` | test(engine): 补只关闭注册资源不误伤旁路文件的用例 | tests/locking_shutdown.rs | ✅ |
| 13 | `be60f55` | merge: 测试期缺陷修复合回 feature | —（merge commit，--no-ff） | ✅ |
| 14-17 | `3ed6fb6`/`b1d6118`/`e23cdce`/`6705493` | docs(test) / docs(state) / docs(test) / docs(test) | 测试报告 / PROJECT_STATE | ✅ |

### 2.1 混杂提交（唯一一处）

`20d1b04` 同时包含两件事：① 新增 `shutdown_locking_processes` + 护栏（本任务主体）；② **修改既有 `who_is_locking` 的 `RmGetList` probe 判断**（`Error::code()` HRESULT → `probe.0` 原始码）。

②是**改变已上线行为**的 bug 修复（修复前 task-020/021 的占用检测在真机上从未生效），却：
- 未在提交信息中披露（`git log` 里看不出这行改动）；
- 未单独成 commit，也没有独立的回归用例（现有用例是通过 `execute_retry_after_shutdown_deletes_locked_file` 间接覆盖的）。

PROJECT_STATE 的「Git 提交记录」表里补注了「（含 who_is_locking probe bug 修复）」，属于事后补救，但 git 历史本身仍不可追溯。**严重度：低**（不否决，但需在后续提交中避免）。

### 2.2 变更范围

`git diff --name-status main...HEAD` 共 9 个文件：

| 文件 | 任务书白名单 | 判定 |
|---|---|---|
| `engine/src/execute/locking.rs` | ✅ 允许 | 范围内 |
| `engine/tests/locking_shutdown.rs` | ✅ 允许（新建） | 范围内 |
| `app/src-tauri/src/commands.rs` | ✅ 允许 | 范围内 |
| `app/src-tauri/src/lib.rs` | ✅ 允许（注册 1 条） | 范围内 |
| `app/src/pages/Items.tsx` | ✅ 允许 | 范围内 |
| `docs/PROJECT_STATE.md` | ✅ 允许 | 范围内 |
| `tasks/049-*.md` | 未列，但「输出要求 2」明确要求追加实现备注 | 合规 |
| `app/src/pages/Items.test.tsx` | ❌ **未列入白名单** | 越界（性质：测试资产） |
| `docs/test-reports/049-test-report.md` | ❌ 未列入白名单 | 越界（性质：提示词 9 的交付物） |

明确禁止清单（`executor.rs` / `whitelist.rs` / `safe_box.rs` / `delete.rs` / 各页面 / `tauri.conf.json` / `capabilities/default.json` / 两个 `Cargo.toml` / `package.json` / workflows / rules / scripts）**逐一核对，0 改动**；未新增任何 npm / cargo 依赖（`git diff --stat` 对四个依赖清单为空）。

> 两处「越界」都是测试侧必要资产，实质无风险。**建议**：任务书白名单把 `*.test.tsx` 与 `docs/test-reports/` 显式列入（本次任务书漏项，非 Agent 过错）。

---

## 3. 验收覆盖性核验

> 对照任务书 §需求 & 验收标准 A1-D16 逐条核对；「抽查复现」为评审侧独立执行（编译 `--test locking_shutdown` 后跑，不重跑前端全量）。

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---|---|---|---|
| A1 | `ShutdownOutcome` / `RefusedProcess` 结构 + camelCase | 报告 §3 A1 + `grep` 输出 | 代码核对：`locking.rs:35/45` 均带 `#[serde(rename_all="camelCase")]`；`RetryLockedResp` 多出（偏差 2 已说明） | 通过 |
| A2 | 签名 / 幂等 / 会话不泄漏 | 报告 §3 A2 + `RmSession::drop` | 代码走查：`RmSession` 的 `RmEndSession` 在 `Drop`，`start()/register()/list()/shutdown()` 任一 Err 均走 drop；抽查 `shutdown_no_lockers_is_idempotent` **PASS**（连调两次空结果） | 通过 |
| A3 | 护栏纯函数 8 条（实为 10 条） | 报告 §3 A3（10 passed） | 抽查 `shutdown_never_closes_self` **PASS**，实际输出 `refused:[{pid:10064, app_name:"locking_shutdown-....exe", reason:"self"}]`——**证明 self 判定走 pid 而非名字，正确** | **部分通过**：见 Bug #1（名单匹配在中文环境下失效） |
| A4 | 已知限制注释 | 报告 §3 A4 | 文件头「## 已知限制」3 条齐全，第 1 条明写「必须重新尝试删除」 | 通过 |
| B5 | 命令实现 + 空 ids 复用 DRYRUN_EMPTY | 报告 §3 B5 | 代码核对 `commands.rs:168`，空数组早返回，`load_external_deny` 失败在关闭前返回 INTERNAL（顺序正确，不会「规则没加载就先关进程」） | 通过 |
| B6 | `lib.rs` 注册 | 报告 §3 B6 | `lib.rs:39` 实查存在 | 通过 |
| B7 | 不误伤其他路径 | 报告 §2「连续 3 次稳定 PASS」 | **抽查不可稳定复现**：连跑 6 条 `--include-ignored` 时该用例 **FAILED**（`b after=[] holder_b_alive=false`，B 侧 holder 进程消失）；单独重跑 2 次均 PASS。见 Bug #3 | **不通过**（证据不足） |
| C8 | `cleanup_execute` 返回值接住 + 失败明细 | 报告 §3 C8（组件用例） | 代码走查 `Items.tsx`：`api<ExecuteSummary>` 接住，`failures = results.filter(!deleted)`，明细渲染「路径 — 原因 — 占用：app.exe (PID x)」 | 通过（组件级） |
| C9 | 无占用时不显示重试按钮 | 报告 §3 C9 | 代码走查：按钮条件 `result.failures.some(f => f.lockingProcesses.length > 0)`；组件用例断言 `queryByText(...) === null` | 通过（组件级） |
| C10 | confirm 文案含清单 + 风险提示；取消不调命令 | 报告 §3 C10 | 代码走查：`window.confirm` 文案含「即将请求关闭以下进程」「未保存的数据可能丢失」；`if (!confirmed) return;` 在 `setRetrying` 之前 | 通过（组件级） |
| C11 | 绿条 + 受保护提示 + 只传失败项 id | 报告 §3 C11 | 代码走查：`itemIds: locked.map(f => f.itemId)`；组件用例断言实参 `{itemIds:[1]}` | 通过（组件级），但提示文案失真见 Bug #2 |
| C12 | 三态互斥 | 报告 §4 缺陷 1（已修 `0129de9`） | 代码走查：`busy = cleaning \|\| retrying \|\| previewing`，四个按钮（刷新/预览/清理/重试）统一 `disabled={busy}`，三个 callback 入口守卫均为 `busy` | 通过 |
| D13 | 不新增依赖 | 报告 §3 D13 | `git diff` 四份依赖清单 0 改动 | 通过 |
| D14 | 不改 `who_is_locking` 签名与 `LockingProcess` 字段 | 报告 §3 D14 | 实查：签名 `pub fn who_is_locking(path: &str) -> Result<Vec<LockingProcess>, String>` 未变；`LockingProcess` 三字段未变；仅内部 probe 判断修正（抽查真机输出 `app_name:"Windows PowerShell"` 证明修复生效） | 通过 |
| D15 | 不改 `executor.rs` | 报告 §3 D15 | `git diff --name-status` 无该文件 | 通过 |
| D16 | 六门禁全绿 + engine ≥173 | 报告 §3 D16（183/6） | CI run 40048746500 全绿（含 cargo test / lint / test / build / tauri build）；本轮抽查 test 二进制 6 条（5 通过 1 失败，见 B7） | 通过（门禁）/ B7 例外 |
| 真机冒烟 | **必做、不可裁剪**：notepad 场景 + explorer 拒绝场景 + CDP 0 红错 | 报告 §6 第三档 1：**未取得**（无活动桌面 → WebView2 渲染进程崩溃） | 已核实环境限制属实（本会话同为无桌面合成；`wry 0.57` 无条件设置 `additionalBrowserArgs` 亦为事实） | **未覆盖**（已登记，见 §7 说明） |
| 打包安装冒烟 | **必做**：NSIS 干净目录安装 + 安装态点击 | 报告 §2 L4：安装 exit 0 / 数据目录隔离 / 安装目录 9 文件哈希 `NO_DIFF`；**GUI 点击部分未取得** | 未复核安装（属测试侧执行范围），接受其「安装与隔离通过、点击阻塞」的分层结论 | 部分覆盖 |
| CI | 五道闸 run id + success | 报告 §2 L5 | 已独立实查 6 次 run 全 success，含 HEAD 的 40048746500 | 通过 |

### 3.1 契约核对表复核

| 核对项 | 报告结论 | 评审复核 |
|---|---|---|
| 命令签名 ↔ 实参 | 对齐 | ✅ `cleanup_retry_locked(item_ids: Vec<i64>, paths: State<AppPaths>)` ↔ `api("cleanup_retry_locked", { itemIds })`（Tauri v2 默认 camelCase→snake_case） |
| serde 注解 ↔ interface | 对齐 | ✅ 逐字段实查：`locking.rs:24/35/45/60` 与 `executor.rs:22/37` 全部 `rename_all="camelCase"`；`ExecResult.locking_processes` 带 `#[serde(default)]`（老数据不缺字段） |
| Envelope 解包 | 对齐 | ✅ `Result<Envelope<RetryLockedResp>, String>`；`api<T>` 判 `code!==0` 抛错；空数组分支前端不可达 |
| 事件 payload | 无新增 | ✅ `progress.rs` 未在 diff 内 |

> 契约四项齐备，与报告一致。

---

## 4. 代码评审发现问题

### 4.1 架构与 TRD 符合度

- 分层清晰：`engine`（RM FFI + 护栏纯函数）→ `app/src-tauri`（命令编排）→ `Items.tsx`（展示与确认）。删除编排**确实复用** `execute_items`，没有第二套删除逻辑（抽查 `commands.rs:222`）。
- `RmSession` 用 RAII 收口 `RmEndSession`，比原来的「每条 return 手写 EndSession」更稳，是本轮质量最好的一处。
- `RetryLockedResp` 落在 engine（偏差 2）：理由成立（app crate 无 serde），但**层次上有味道**——命令响应 DTO 定义在领域模块里。可接受，建议后续把 `Envelope` 的泛型约束或 app 侧 serde 依赖理顺后迁回。
- `spawn_blocking` 隔离阻塞 FFI、按 pid 去重，两处都是超出任务书要求的正确判断（偏差 6），认可。

### 4.2 模块边界与可维护性

- `matches_protected_name` / `is_protected_process` 拆成纯函数 + 单测，可维护性好；拒绝原因用常量收敛为四个稳定文案，前端直接展示不翻译，接口稳定。
- 隐患：护栏的**判定依据是字符串**（见 Bug #1），扩展名单只能靠「加名字」，在本地化环境下不收敛。
- `Items.tsx` 从 1 个 `{ok,text}` 结果态扩成 `ResultState{tone,headline,failures,refused}`，结构合理；`describeExecError` 把 `FILE_IN_USE:` / `PERMISSION_DENIED:` 前缀翻成人话、其余原样展示（不吞信息），符合「不静默」原则。

### 4.3 错误处理与日志

- `commands.rs`：单条 shutdown 失败 `eprintln` 不中断整批（与 `purge_many` 一致）；id 查不到跳过并打日志；`load_external_deny` 失败在**关闭进程之前**返回 INTERNAL——顺序正确。
- `locking.rs`：`list()` 的 probe 用 `probe.0` 与 `ERROR_MORE_DATA` 比较（并附注释说明为何不能用 `Error::code()`），fetch 用 `.ok()`，错误信息带 code + OS error 文案，可排查。
- 缺口：`shutdown()` 只看 `RmShutdown` 的返回值，**不校验「哪些进程真的被关了」**，直接把护栏放行的 `closable` 当 `closed` 上报（见 Bug #2）。
- 缺口：`audit_log` 未记录「关了谁」（任务书明确不扩列，接受，但排查时只能看前端结果条）。

### 4.4 并发与资源释放

- `RmSession::drop` 覆盖成功/失败/提前返回三条路径；`RmShutdown` 放 `spawn_blocking`；三态互斥 `busy` 覆盖四个按钮与三个入口守卫。**未发现资源泄漏或竞态**。
- 残余风险：`retryLocked` / `cleanup` 的入口守卫读的是渲染时闭包的 `busy`，同一帧内极快双击理论上仍可能进两次（React 状态异步）。与既有 `cleanup` 同构，属既有模式，不单独计缺陷。
- 残余风险：`RmShutdown` 最坏 ~30s，多路径串行无总时长上限（报告已登记）。

### 4.5 新增依赖

无。`windows` crate 复用既有 `Win32_System_RestartManager` feature。

---

## 5. 安全风险检查结果（OWASP Top 10 视角）

| 风险 | 检查点 | 结论 |
|---|---|---|
| A01 越权 / 权限控制 | `cleanup_retry_locked` 是否可被前端用来操作任意路径？ | **通过**：只按 `item_ids` 反查 DB 里的 path，不接受前端传路径；删除仍走 `execute_items` 的 L3 白名单二次校验。capabilities 未放宽 |
| A02 加密失效 | 无新增密钥/传输 | 不适用 |
| A03 注入 | `SELECT path FROM cleanup_item WHERE id = ?1` + `params![id]` | **通过**（参数化，无拼接） |
| A04 不安全设计 | **新增「终止进程」这一高危原语** | **部分通过**：有 confirm 二次确认、有护栏、有幂等与去重；但护栏在中文环境下可被绕过（Bug #1），且 `closed/refused` 上报与实际关闭范围不一致（Bug #2） |
| A05 安全配置错误 | `capabilities/default.json` / `tauri.conf.json` 未改 | **通过** |
| A06 易受攻击组件 | 依赖清单 0 改动 | **通过** |
| A07 认证失效 | 单机桌面应用，无会话体系 | 不适用 |
| A08 数据/代码完整性 | 无新增反序列化入口；`Envelope<T>` 仅出站序列化 | **通过** |
| A09 日志与监控缺失 | 关闭动作只进 `eprintln` + 前端结果条，未进 `audit_log` | **接受**（任务书明确不扩列），建议后续补「关闭了哪些进程」审计 |
| A10 SSRF | 无网络请求 | 不适用 |

**敏感信息泄露**：diff 内无密钥/token/证书；`eprintln` 只含本地路径，属本机日志。**通过**。

**高危原语评估**：本任务把应用从「只读查询占用」升级为「可请求强制关闭他进程」。护栏是唯一防线，因此**护栏的正确性属于安全红线**——这正是 Bug #1 被判为高严重度的原因。

---

## 6. 状态与文档一致性核对

| 核对项 | 断言 | 实查结果 | 结论 |
|---|---|---|---|
| PROJECT_STATE 状态 | 「task-049 测试完成，状态 awaiting_review」 | 与测试报告 §7 一致；feature 自 `be60f55` 后无代码改动 | 一致 |
| 已知问题 · task-021 条目 | 「~~已由 task-049 解决~~」+ probe bug 说明 | 代码与真机抽查（`app_name:"Windows PowerShell"` 非空）证明占用检测确实恢复 | 一致 |
| 已知问题 · task-049 遗留 ①②③④ | 四条 | ①②③ 属范围外/已登记；④ GUI 证据缺失属实 | 一致 |
| 已知问题 · 打包资源路径错位 | 「2026-10-24 测试轮发现，非本任务引入，引入于 16b30e2」 | `git log -S` 归属与「非本任务引入」相符（本任务 diff 未触及 `tauri.conf.json` / `paths.rs`） | 一致（登记合格） |
| 第三档遗留是否正式登记 | 报告 §6 共 7 项 | 已知问题覆盖 ①②③④ + 新增打包路径项；第 6 项（vitest EPERM）、第 7 项（同源测试风险）属环境/流程说明，未登记亦可接受 | 基本一致 |
| `tasks/049-*.md` 状态字段 | 应为 awaiting_review | **仍为 `pending`**（未更新）。注：`tasks/046`、`tasks/047` 同样停留在 `pending`，属仓库既有惯例缺口，非本任务独有 | **不一致（低）** |
| 下一步计划 · commit 指针 | 写「`feature/...` @ `be60f55`」 | 实际 HEAD `6705493`（差 4 个 docs 提交） | **轻微失真（低）** |
| 分支残留 | — | 本地/远端仍存 `feature/task-035-tauri-updater`、`feature/task-036-auto-update-banner`、`feature/task-037-safebox-purge`、`bugfix/task-035-release-signing`、`bugfix/task-035-updater-acl`，`git branch --merged main` 确认**均已并入 main** | 历史遗留未清理（非本任务责任） |

---

## 7. 评审结论

**不通过。**

理由（按一票否决项对齐）：

1. **验收标准 A2/A3 在目标语言环境下未真正成立**（Bug #1，高）：护栏以「RM 返回的应用名」做名单匹配，而 RM 返回的是**本地化 FileDescription**（真机证据：`powershell.exe` → `"Windows PowerShell"`，非 `powershell.exe`）。本机 `zh-CN`，`explorer.exe` 的 FileDescription 实测为 **`Windows 资源管理器`**、`dwm.exe` 为 **`桌面 窗口管理器`**，三条匹配规则（全名 / 去 `.exe` 相等 / 以基名结尾）全部不命中 → **`explorer.exe` 会被判为「可关」并进入 `closed`**，与任务书「explorer.exe 命中即跳过，绝不关闭」的硬性要求相反。
   该缺陷本应由「真机 explorer 拒绝场景」发现，但该场景因 GUI 环境阻塞**从未执行**（开发轮、测试轮均未取得），单元测试只喂了英文名，故两轮都没暴露。
2. **B7「不误伤其他路径」的唯一证据不可稳定复现**（Bug #3，中）：评审侧连跑 1/3 失败，报告称「连续 3 次稳定 PASS」。

其余为「可带条件通过」的项，但上述两条已足以否决。

> **不构成否决的部分（明确记录）**：
> - GUI 真机点击 / Console 采集缺失：**属实的环境限制**（无活动桌面合成 → WebView2 渲染进程崩溃），两轮独立复现，且已用组件级用例 + 后端真实占用进程端到端 + 代码走查补偿，并登记第三档。**不作为本轮否决理由**，但按任务书「必做、不可裁剪」的口径，**必须在具备活动桌面的机器上补齐后才能发版**（见 §10）。
> - 打包态资源路径错位：预存在问题、已登记、不属本任务引入，**不作为否决理由**。
> - 会话粒度导致的 `closed/refused` 语义失真：已登记为已知限制，**不单独否决**，但与 Bug #1 同属「护栏可信度」问题，建议在同一次 bugfix 里一并处理文案（见 Bug #2）。

---

## 8. Bug 清单（不通过）

> 修复一律在当前 feature 分支下开 **一层** bugfix 子分支：`bugfix/task-049-guard-image-name`（从 `6705493` 拉出），修完合回 feature → 回提示词 9 重测 → 再回本提示词重评。

### Bug #1（高）护栏依赖本地化应用名，中文 Windows 下 `explorer.exe` 可被关闭

| 项 | 内容 |
|---|---|
| 现象 | `zh-CN` 系统上 `RmGetList` 返回的 `strAppName` 是本地化 FileDescription（`Windows 资源管理器`）。`matches_protected_name` 三条规则对中文字符串全部不命中 → `explorer.exe` 被判为可关，进入 `closed` 并被 `RmShutdown(RmForceShutdown)` 请求关闭；前端 confirm 会把它列进「即将请求关闭」，事后也不会出现在「受保护未关闭」里 |
| 复现 | ① `powershell -c "(Get-Item C:\Windows\explorer.exe).VersionInfo.FileDescription"` → `Windows 资源管理器`；② RM 返回 FileDescription 而非文件名，已由真机数据证明（`app_name:"Windows PowerShell"`）；③ 逻辑推演：`matches_protected_name("Windows 资源管理器")` 三条规则均 false → `None`（可关）。真机端到端复现：让 explorer 持有某文件（缩略图/预览场景）→ 清理失败 → 点重试 → confirm 出现 `Windows 资源管理器 (PID x)` |
| 期望 | A2 硬性要求：`explorer.exe` 命中护栏即跳过，且 `refused` 里给出 `protected process` |
| 严重度 | **高**（核心安全护栏在目标语言环境失效；虽 explorer 会被系统自动重启、无数据丢失，但违反「绝不关闭」红线） |
| 修复建议 | **改为以 PID 为准、与语言无关**：`OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `QueryFullProcessImageNameW` 取映像全路径 → 取文件名比对名单；**取不到映像路径时 fail-closed（拒绝，reason 复用 `protected process` 或新增 `unknown process`）**。保留现有名字匹配作兜底。禁止只往名单里追加中文名（不收敛）。补单测：`"Windows 资源管理器"` / `"桌面 窗口管理器"` / 空名 fail-closed；补一条真机用例：explorer 持锁 → `shutdown` → 断言 `refused` 含 `protected process` 且 `closed` 不含 explorer |
| 验证 | `cargo test -p engine execute::locking` + 真机 explorer 场景（可在无 GUI 下用「explorer 持锁的文件」构造，不依赖点击） |

### Bug #2（中）`closed` / `refused` 上报与「会话级关闭」事实不一致

| 项 | 内容 |
|---|---|
| 现象 | `RmShutdown` 是**会话级**：会话里只要有 1 个可关进程就会调用它，从而把同会话里被护栏拒绝的保护进程**也一并请求关闭**；但 `closed` 只记可关的那些、`refused` 仍把保护进程列为「受保护未关闭」。前端据此显示「以下进程受保护未关闭」——在混合占用场景下**与事实相反** |
| 复现 | 同一文件同时被 `explorer.exe` 与 `notepad.exe` 占用 → 点重试 → `refused` 含 explorer，但 explorer 实际已随会话被请求关闭 |
| 期望 | 要么「一个都不关」（mixed 场景整体 fail-closed），要么前端/文档明确「这些进程被护栏跳过，但 RM 会话级关闭可能仍波及」 |
| 严重度 | 中（安全提示失真；实际危害有限——保护名单里的进程基本不持有用户未保存数据） |
| 修复建议 | 二选一：① **fail-closed**：`closable` 非空且 `refused` 非空时，整批不调 `RmShutdown`，返回 `closed=[]` + 完整 `refused`（最保守，推荐）；② 保留现状但在 `Items.tsx` 把黄条文案改为「以下进程被护栏跳过；受 Restart Manager 会话粒度限制，可能仍被波及」。任一方案都要同步改任务书 A2/B 段说明 |
| 验证 | 组件用例断言新文案；单测/集成用例断言 mixed 场景行为 |

### Bug #3（中）`shutdown_only_touches_registered_path` 不可稳定复现

| 项 | 内容 |
|---|---|
| 现象 | 评审侧 `--include-ignored --test-threads=1` 连跑 6 条时该用例 **FAILED**：`b before=[pid 34520] b after=[] holder_b_alive=false`（B 侧 holder 进程消失），触发用例自带的 `inconclusive` 断言；单独重跑 2 次均 PASS。合计 **1/3 失败**，与报告「连续 3 次稳定 PASS」不符 |
| 复现 | `cargo test -p engine --test locking_shutdown --no-run` → 跑 `--include-ignored --test-threads=1`（连跑场景）；或反复单独跑该用例观察偶发 |
| 期望 | 用例在连跑与单跑下都稳定；若失败，必须能区分「环境抖动」与「RmShutdown 真的波及了旁路进程」 |
| 严重度 | 中（B7 是「只关注册资源」的唯一证据；不稳定 = 证据不可采信。另需排除「RmShutdown 偶发关闭同映像的其他实例」这一更严重的可能） |
| 修复建议 | ① 在 `shutdown()` 后打印/断言 `RmGetList` 的剩余占用，区分「B 的 holder 自然退出」与「被 RM 关掉」；② B 侧 holder 改为**启动后写一行 ready 标记 + 长 sleep**，并记录其退出码（`try_wait` 拿到 `Some(exit)` 时打印 code，被强杀与自然退出可区分）；③ 用例间加隔离（每条用例独立的 tempdir 已有，但 holder 进程名相同，建议给 holder 传不同参数/不同 `-Command` 以区分）；④ 连跑 5 次全绿才算通过 |
| 验证 | 连跑 5 次 `--include-ignored` 全绿，并在报告贴 5 次输出 |

### Bug #4（低）提交混杂：`who_is_locking` probe 修复藏在 feat commit 里

- 现象/期望/修复：见 §2.1。建议**不重写历史**（feature 分支已推送、已评审），在 bugfix 分支补一条 `fix(engine): 修正 who_is_locking probe 误判（HRESULT vs Win32 码）(#task-049)` 的空提交式说明不可行——改为在 PROJECT_STATE 与评审记录中留痕即可，**后续任务**要求「行为变更单独成 commit」。
- 严重度：低（不阻塞）。

### Bug #5（低）文档一致性

- `tasks/049-*.md` 的 `## 状态字段` 仍为 `pending`（PROJECT_STATE 已 `awaiting_review`）；PROJECT_STATE「下一步计划」写 `@ be60f55`（实际 `6705493`）。
- 修复：本次评审已在 PROJECT_STATE 追加评审结论；状态字段建议 DocsKeeper 统一整改（046/047 同病），并约定「以 PROJECT_STATE 为唯一状态源」或「任务书状态字段同步更新」二者择一。
- 严重度：低。

### Bug #6（低）测试资产越界（流程）

- `app/src/pages/Items.test.tsx`、`docs/test-reports/049-test-report.md` 不在任务书白名单。性质为测试交付物，**不建议追责**；建议任务书模板把这两类路径列入白名单。
- 严重度：低。

---

## 9. Git 操作记录

| 动作 | 结果 |
|---|---|
| 合并 main | **未执行**（评审不通过，禁止合并） |
| revert | 未执行（未合并过） |
| feature 分支 | **保留** `feature/task-049-rm-shutdown-locking` @ `6705493`（本地 + 远端均在） |
| main | **未改动**，仍为 `401f336`（= `origin/main`） |
| 本轮新增提交 | 见下（仅 docs，不含业务代码） |

本轮评审产生的提交（仅文档，`docs` 类型）：

| commit | 内容 |
|---|---|
| `8731c44` | `docs(review): 任务049评审报告 (#task-049)` —— 仅 `docs/reviews/049-review.md` |
| `27d8057` | `docs(state): 任务049评审不通过，登记护栏本地化缺陷 (#task-049)` —— 仅 `docs/PROJECT_STATE.md` |
| `2c91242` | `docs(task): 新增任务049b护栏映像名修复任务书 (#task-049)` —— 仅 `tasks/049b-bugfix-guard-image-name.md` |

推送记录：`git push origin HEAD:refs/heads/feature/task-049-rm-shutdown-locking` → `6705493..2c91242`（远端 feature 已同步；`origin/main` 仍为 `401f336`，未动）。
CI：代码态 `6705493` 对应 run **40048746500** success；docs 提交 `2c91242` 触发 run **40050538936**（评审时 in progress，纯 md 变更，不影响结论）。

> 未修改任何业务源码（`git diff --name-only 6705493..HEAD` = 3 个 md 文件）。

---

## 10. 后续动作建议

1. **派发 Bug 修复 Agent（提示词 11）**，分支 `bugfix/task-049-guard-image-name`（从 `6705493` 拉出，只允许一层），任务书 `tasks/049b-bugfix-guard-image-name.md`。
   - 必修：Bug #1（映像名判定 + fail-closed + 中文名单测 + explorer 真机用例）、Bug #2（mixed 场景 fail-closed 或文案纠正）、Bug #3（用例稳定性 + 连跑 5 次证据）。
   - 可延后：Bug #4/#5/#6 属流程与文档，随任务收尾处理。
2. **修完回提示词 9 重测**（重点：护栏中文环境、mixed 场景、B7 连跑 5 次、六门禁），再回提示词 10 重评。
3. **循环刹车提示**：本轮为 task-049 **第 1 次**评审不通过，未触发「同一缺陷连续 2 次」的刹车；若 Bug #1 修复后仍不通过，第 2 次将由调度 Agent 置 `blocked` 并上报用户做根因决策。
4. **发版前必做**（不阻塞合并，但阻塞发版）：在具备活动桌面的机器上补 GUI 真机冒烟（红条明细 / confirm 文案 / 绿条结果 + Console 0 红错）与安装态点击，证据回填测试报告。
5. **合并前小动作**：把 feature rebase 到 `401f336`（main 仅多 1 个 memory 提交），保证 merge commit 拓扑干净；合并后清理本任务分支，并顺手清掉 task-035/036/037 的残留已合并分支。
6. **另开任务**：打包态 `bundle.resources` 路径错位（预存在问题，影响全新安装用户首次启动），建议 `bugfix/task-050-bundle-resources-path`，优先级高于 safe_box 增强。
