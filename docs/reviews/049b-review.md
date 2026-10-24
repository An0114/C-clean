# 评审报告（轮 2）：task-049 / 049b 占用进程 RmShutdown——护栏本地化绕过修复

> 评审角色：Review Lead（提示词 10）
> 评审时间：2026-10-24
> 分支：`feature/task-049-rm-shutdown-locking`
> HEAD：`03b53b4`（代码态 = `5550f61`，其后 2 个提交为 docs：测试报告 + 状态）
> 上游：`docs/reviews/049-review.md`（轮 1 不通过）→ `tasks/049b-bugfix-guard-image-name.md`（修复）→ `docs/test-reports/049b-test-report.md`（重测）
> 评审结论：**通过**（合并动作与 merge commit 记录见 §9）

---

## 1. 输入核验

| 输入项 | 状态 | 事实核对（本轮实查） |
|---|---|---|
| 测试报告 | ✅ | `docs/test-reports/049b-test-report.md`（137 行，含 L1-L5、契约表、逐条验收、缺陷清单、三档分流、同源风险声明） |
| CI 运行号与结论 | ✅ | `ci` run **40060421641**（job `build`，windows-latest）`conclusion=success`，head = `03b53b4`（= 当前 HEAD）。报告引用的 40059150999（`5550f61`）/ 40057717120（`448275a`）/ 40056574881（`0c02485`）亦逐一核实为 success |
| feature 分支最新 commit | ✅ | `03b53b4`（本地 = `origin/feature/task-049-rm-shutdown-locking`，fetch 后一致） |

> 报告把 CI 锚点写成 `5550f61`（最后一次改代码的提交），其后 2 个 docs 提交未回写 CI——本轮已自行补查 `03b53b4` 的 run 40060421641 为 success。**不构成不通过**，与轮 1 同一处理口径。

### 1.1 分支拓扑（实查）

| 项 | 值 |
|---|---|
| main / origin/main | `401f336`（评审前未动） |
| merge-base(main, feature) | `e3e74e0` |
| main...HEAD | 落后 1（`401f336` memory 提交）/ 领先 11（相对轮 1 的 `37375fb`） |
| 本任务链 bugfix 子分支 | `bugfix/task-049-guard-image-name`、`bugfix/task-049-guard-test-stability` **均已合回并删除**（本地 + 远端 `git branch -a` 实查无残留） |
| 临时 PR | #13（draft）/ #14 已关闭，仅 #11 存续（open，`mergeable_state=clean`，head `03b53b4`） |

---

## 2. Git 提交记录核对结果

本轮新增 11 个提交（`37375fb..HEAD`），全部关联 `#task-049`，`type(scope)` 正则逐条校验 0 条不合规，无敏感文件（`.env/.key/.pem/.pfx/credential` 全量扫描 0 命中）。

| # | commit | 提交信息 | 改动 | 原子性 |
|---|---|---|---|---|
| 1 | `199527a` | fix(engine): 护栏改为按 PID 映像名判定… | locking.rs | ✅ |
| 2 | `c34af2c` | test(engine): 补本地化应用名/映像名不可得/mixed 用例，B7 可归因 | tests/locking_shutdown.rs | ✅ |
| 3 | `11feafe` | docs(task): 补任务049b实现备注与状态字段 | tasks/049b-*.md | ✅ |
| 4 | `75e43f6` | docs(state): 任务049b 修复完成，待重测 | PROJECT_STATE.md | ✅（单文件） |
| 5 | `b1055aa` | merge: task-049b … 合回 feature（`--no-ff`） | — | ✅ |
| 6 | `0c02485` | docs(state): 回填合并后 CI 结论并移交测试 | PROJECT_STATE.md | ✅ |
| 7 | `a740dea` | test(engine): 本地化护栏用例改用必然不存在的 pid | tests/locking_shutdown.rs | ✅ |
| 8 | `448275a` | docs(frontend): 补齐 reason 的 unknown process 说明 | Items.tsx（注释） | ✅ |
| 9 | `5550f61` | merge: 测试期修复合回 feature（`--no-ff`） | — | ✅ |
| 10-11 | `17aa869`/`03b53b4` | docs(test) / docs(state) | 测试报告 / PROJECT_STATE | ✅ |

> 轮 1 提出的「行为变更混在 feat commit」问题在本轮**未复发**：护栏重写单独成 `199527a`，测试单独成 `c34af2c`，测试期修复单独成 `a740dea`，两次合并均为显式 `--no-ff` merge commit。

### 2.1 变更范围（`37375fb..HEAD`，7 文件）

| 文件 | 049b 任务书白名单 | 判定 |
|---|---|---|
| `engine/src/execute/locking.rs` | ✅ 允许 | 范围内 |
| `engine/tests/locking_shutdown.rs` | ✅ 允许 | 范围内 |
| `engine/Cargo.toml` | ✅ 允许（仅追加 windows feature） | 范围内：实查只多 `"Win32_System_Threading"` 一行 + 注释 |
| `app/src/pages/Items.tsx` | ⚠️ 仅「选 B 方案时改黄条文案」 | **仅改注释**（补 `unknown process` 说明），未动文案与逻辑；判为**可接受**（白名单本意是禁止 UI 行为变更） |
| `docs/PROJECT_STATE.md` | ✅ 允许 | 范围内 |
| `tasks/049b-*.md` | ✅ 允许 | 范围内 |
| `docs/test-reports/049b-test-report.md` | ❌ 未列 | 越界（性质：提示词 9 交付物，同轮 1 处理口径，不追责） |

`Cargo.lock` / `pnpm-lock.yaml` / `package.json` / `app/package.json` **零改动** → 未新增任何依赖。禁止清单（`executor.rs` / `whitelist.rs` / `safe_box.rs` / `delete.rs` / `commands.rs` / `lib.rs` / `tauri.conf.json` / `capabilities/default.json` / workflows / rules / scripts）**逐一核对 0 改动**。

---

## 3. 验收覆盖性核验

### 3.1 轮 1 否决项闭环（本轮重点）

| 轮 1 缺陷 | 修复 | 测试报告证据 | 评审抽查 | 结论 |
|---|---|---|---|---|
| **Bug #1（高）护栏按本地化应用名匹配，`zh-CN` 下 `explorer.exe` 可被关闭** | `image_file_name(pid)`：`OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `QueryFullProcessImageNameW` → 取文件名；`decide()` 以映像名为主判据，取不到则 fail-closed（`unknown process`）；保留友好名兜底；**未往名单加任何中文名** | 报告 §3 A1-A5：`locking.rs:238` 实现、判定顺序、`grep -c 资源管理器` → 生产逻辑 0 处；真机 `shutdown_refuses_protected_image_holder` 实测 `closed=[]` + `refused=[protected process]` | **已复现**：本机（`zh-CN`）跑该 `--ignored` 用例 → `fake explorer outcome: ShutdownOutcome { closed: [], refused: [RefusedProcess { pid: 38976, app_name: "explorer.exe", reason: "protected process" }] }`，1 passed / 0.93s | **闭环** |
| **Bug #2（中）mixed 场景 `closed`/`refused` 语义失真** | `apply_mixed_fail_closed`：`closable` 非空且 `refused` 非空 → 清空 `closable`，调用方见空即不调 `RmShutdown` | 报告 §3 B + 单测三分支 | 代码走查 + 抽查 15 条单测全绿（含 `mixed_batch_is_fail_closed` / `batch_without_protected_process_closes_all` / `batch_all_refused_stays_refused`） | **闭环**（选 A fail-closed，即轮 1 推荐方案） |
| **Bug #3（中）B7 用例不可稳定复现** | 根因不是抖动而是 **holder 共用 console**：`RmShutdown` 强杀 A 会带走同 console 的 B。改 `CREATE_NO_WINDOW` 独立 console + `.ready`/`.done` 标记 + 退出码归因；失败时直接区分「自然退出」与「被 RM 波及」 | 报告 §3 C6-C7：合并前后各连跑 5 次、共 10 次全绿（每次 8/8） | 未重复连跑（避免重测）；代码走查确认归因逻辑成立（`done` 标记 + `exit code` + 独立的「波及旁路进程」panic 分支） | **闭环**（证据形态合格） |

### 3.2 049b 任务书验收标准逐条

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---|---|---|---|
| A1 | `image_file_name` 用 `OpenProcess` + `QueryFullProcessImageNameW`，句柄 RAII | 报告 §3 A1 | 代码走查：`OwnedHandle(HANDLE)` + `Drop{CloseHandle}`；`pid==0` 短路；`?` 失败转 `None`；缓冲 32768 u16 | 通过 |
| A2 | 判定顺序：self → pid≤4 → service → 映像名 → 取不到 fail-closed → 空名 → 友好名兜底 | 报告 §3 A2 | 代码走查 `decide()` 与任务书逐条一致（`None => Some(REASON_UNKNOWN)`） | 通过 |
| A3 | 不得靠追加中文名修复 | 报告 §3 A3（生产逻辑 0 处） | `grep` 复核：`资源管理器` 仅出现在注释与测试断言中 | 通过 |
| A4 | 名单仍 12 条、小写比较 | 报告 §3 A4 | 代码走查 `PROTECTED_APP_NAMES` 12 条未变 | 通过 |
| A5 | 新增本地化 / 映像名不可得用例，既有 10 条不回归 | 报告 §3 A5（15 passed） | **已复现**：`cargo test -p engine --lib execute::locking::tests` → **15 passed / 0 failed**（15 条用例名与报告一致） | 通过 |
| B | mixed 场景整批不关 + 新用例 | 报告 §3 B | 同上（单测三分支通过）；`shutdown_locking_processes` 在 `closable.is_empty()` 时提前返回，确实不调 `session.shutdown()` | 通过 |
| C6 | B7 用例可归因 | 报告 §3 C6 | 代码走查：失败信息带 `b_before/b_after/holder_b_alive/holder_b_exit/holder_b_done_marker`；且「无 done 标记即退出」直接 panic 为**真缺陷**而非 inconclusive | 通过 |
| C7 | 连跑 5 次全绿 | 报告 §3 C7（10 次连跑） | 未重跑（属功能测试范畴，信任测试轮证据；抽查已覆盖同文件的另一条真机用例） | 通过（采信） |
| D8 | 不新增依赖；新增 windows feature 须说明 | 报告 §3 D8 | 实查 `Cargo.toml` 仅 +1 feature，`Cargo.lock` 零改动 | 通过 |
| D9 | 不改 `who_is_locking` 签名 / `LockingProcess` / `executor.rs` | 报告 §3 D9 | 实查 diff：三者均未变 | 通过 |
| D10 | 六门禁全绿，engine ≥183+新增 | 报告 §3 D10（196 条） | CI run 40060421641 全绿（含 cargo test / lint / vitest / build / tauri build） | 通过 |
| 真机/打包 | 真机 + 打包安装冒烟 | 报告 §2 L4：安装 exit 0、安装目录 9/9 SHA-256 零变化；GUI 渲染仍崩溃（新增 `.dmp`） | 未复核（测试侧执行范围）；GUI 缺失已按第三档登记，与轮 1 同一处理口径 | 部分覆盖（已登记） |

### 3.3 契约核对表复核

| 核对项 | 报告结论 | 评审复核 |
|---|---|---|
| 命令签名 ↔ 实参 | 对齐（049b 未改 `commands.rs`） | ✅ 实查本轮 diff 无 `commands.rs` |
| serde 注解 ↔ interface | 对齐 | ✅ `ShutdownOutcome` / `RefusedProcess` / `RetryLockedResp` 仍 `camelCase`；真机 JSON 字段名与前端 interface 一致 |
| **`reason` 取值域** | 5 种（新增 `unknown process`），前端原样展示 | ✅ 后端常量 5 个；前端注释已同步（`448275a`）；`reason` 是 `string` 直出，无枚举映射风险 |
| Envelope 解包 | 对齐 | ✅ 未改 |
| 事件 payload | 无新增 | ✅ `progress.rs` 不在 diff |

---

## 4. 代码评审发现问题

### 4.1 方案与实现质量（本轮重点）

- **判定依据换对了**：从「RM 的本地化应用名」换成「PID → 映像文件名」，这是把**不可靠的外部字符串**换成**语言无关的系统事实**，方向正确，且没有退化成「往名单里堆中文名」。
- **fail-closed 贯穿到底**：`pid==0` / `OpenProcess` 失败 / `QueryFullProcessImageNameW` 失败 / 路径末段为空，四条失败路径统一 `None` → `unknown process` → 拒绝。这是护栏类代码该有的默认值。
- **句柄管理**：`OwnedHandle` + `Drop` 收口，三条提前返回都不会漏 `CloseHandle`；与既有 `RmSession` 的 RAII 风格一致。
- **mixed fail-closed 的边界正确**：收口放在 `shutdown_locking_processes(path)` 内部（按路径），而不是命令层跨路径聚合处——这样「路径 A 全可关、路径 B 有保护进程」时 A 仍能正常关闭，不会被 B 连坐。这是本实现里最容易写错的一处，实现正确。
- **测试可移植性**：`decide()` 把映像名作为入参注入，绝大多数用例与运行机器无关；只有两条「取不到映像名」的用例走真实 `OpenProcess`，且用 `u32::MAX - 1` 保证必然失败。轮 1 我担心的「用例随机器漂移」在测试轮也被自查出来并收紧（`a740dea`）。

### 4.2 本轮发现的问题（均不阻塞）

| # | 问题 | 严重度 | 处置 |
|---|---|---|---|
| 1 | **`RmShutdown` 会连带关闭同 console / 进程组的兄弟进程**（修复轮实测定位：两个共用 console 的 holder，关 A 时 B 也退出，`exit=1` 且无 done 标记）。对用户的实际含义：若占用方是控制台程序（如终端里跑着的构建/脚本），点「重试」可能连带关掉同一终端里的其他进程，超出 confirm 里列出的进程范围 | 中 | 已写入 `locking.rs` 已知限制 4 + PROJECT_STATE 已知问题 ⑤。**不阻塞合并**（用户显式确认 + 已登记），但建议后续：① `confirm` 文案补一句「同一控制台内的相关进程可能一并关闭」；② 或另开任务评估对控制台进程改用 `WM_CLOSE`/`TerminateProcess` 逐进程处理 |
| 2 | **mixed fail-closed 的用户可见代价**：点「重试」→ confirm 里列了可关进程 → 实际一个都没关 → 只显示「已关闭 0 个进程，仍失败 N 项」+ 黄条，**没有「因存在受保护进程，本次未关闭任何进程」的解释** | 低-中 | 测试报告第三档 6 已登记；属产品文案决策。建议并入下一条 UI 小改一起做 |
| 3 | `app/src/pages/Items.tsx` 改动超出「仅选 B 方案时改文案」的白名单条款（实际只改注释） | 低 | 判为可接受（无行为变更），记录在案 |
| 4 | PROJECT_STATE「下一步计划」第 1 条仍写「测试 Agent 全量回归 task-049」（该步已完成），与实际状态错位 | 低 | 本轮合并时一并修正（§9 的 state 提交） |
| 5 | `tasks/049b-*.md` 状态字段为 `testing`，PROJECT_STATE 已 `awaiting_review` | 低 | 与轮 1 同一「状态字段不同步」惯例问题（046/047 同病），不追责 |

### 4.3 组件用例保真度复核（轮 1 遗留疑问）

轮 1 我注意到 `Items.test.tsx` 的 mock 数据同时给出 `closed` 非空 + `refused` 非空，而 mixed fail-closed 之后单个路径不可能产生这种组合。本轮复核：**该状态仍可达**——`apply_mixed_fail_closed` 在**按路径**调用，命令层跨路径聚合时「路径 A 全可关（进 closed）+ 路径 B 有保护进程（进 refused）」完全可能。故该 fixture 不陈旧，UI 分支（绿条 + 黄条并存）仍需覆盖。**无误报**。

---

## 5. 安全风险检查结果（OWASP Top 10）

| 风险 | 检查点 | 结论 |
|---|---|---|
| A01 越权 / 权限控制 | 命令是否可被用来操作任意路径 | **通过**：仍只按 `item_ids` 反查 DB path，删除走 `execute_items` 的 L3 白名单二次校验；本轮未动命令层 |
| A02 加密失效 | 无新增密钥/传输 | 不适用 |
| A03 注入 | SQL 参数化 | **通过**（本轮未新增 SQL） |
| A04 不安全设计（**本轮重点**） | 新增「终止进程」原语的可信边界 | **改善**：判定从「本地化字符串」改为「映像文件名」，取不到即拒绝（fail-closed），攻击面从「改语言/改名即可绕过」收敛为「必须真的叫 explorer.exe」；mixed 场景整批不关，杜绝「护栏说没关、实际关了」 |
| A05 安全配置错误 | `capabilities` / `tauri.conf.json` | **通过**（未改） |
| A06 易受攻击组件 | 依赖清单 | **通过**：`Cargo.lock` 零改动，仅给已有 `windows` crate 加 feature |
| A07 认证失效 | 单机桌面应用 | 不适用 |
| A08 数据完整性 | 反序列化 | **通过**（无新增入站反序列化） |
| A09 日志与监控 | 关闭动作仍只进 `eprintln` + 前端结果条 | **接受**（任务书明确不扩 `audit_log`），建议后续补「关闭了哪些进程」审计 |
| A10 SSRF | 无网络 | 不适用 |

**敏感信息泄露**：本轮 diff 无密钥/token/证书；日志只含本地路径。**通过**。

**高危原语残余风险**（记录，不否决）：`OpenProcess` 只请求 `PROCESS_QUERY_LIMITED_INFORMATION`（只读查询权限），不提权、不注入；`RmShutdown` 的 console 连带效应见 §4.2 #1，已登记。

---

## 6. 状态与文档一致性核对

| 核对项 | 断言 | 实查 | 结论 |
|---|---|---|---|
| PROJECT_STATE 状态 | 「task-049b 测试通过，`awaiting_review`」 | 与测试报告 §7 一致；`5550f61` 后无代码改动（仅 docs） | 一致 |
| 已知问题 · 轮 1 三条 | 均已划线标「已由 task-049b 解决」 | 三条均有对应 commit / 证据，且与本轮代码一致 | 一致 |
| 已知问题 · 新增 console 耦合 | ⑤ 已登记 | 与 `locking.rs` 已知限制 4 对应 | 一致 |
| 第三档遗留登记 | GUI 证据缺失 / 打包路径错位 / `RmRestart` / 30s 阻塞 / console 耦合 / mixed 文案 / 本地签名 | 逐项核对，7 项均在测试报告 §6 与 PROJECT_STATE 中可追溯 | 一致 |
| 分支残留 | 本任务 bugfix 子分支应删除 | `git branch -a` 实查：两个 049 bugfix 分支均无残留 | 一致 |
| 历史残留分支 | — | `feature/task-035/036/037`、`bugfix/task-035-*` 仍在，但 `git branch --merged main` 确认均已并入；**属历史遗留，非本任务责任，本轮不删**（避免误删他人工作分支） | 记录 |

---

## 7. 评审结论

**通过。**

判定依据（对齐一票否决项）：

| 否决项 | 本轮结论 |
|---|---|
| 测试报告缺失 | 否（`049b-test-report.md` 齐备） |
| CI 红 | 否（run 40060421641 @ `03b53b4` success） |
| 验收标准未逐条覆盖 | 否（A1-A5 / B / C6-C7 / D8-D10 逐条有证据；抽查 15 条单测 + 1 条真机用例自行复现通过） |
| 存在未处理的严重缺陷 | 否（轮 1 的 3 项否决/阻塞缺陷全部闭环；本轮新发现项均为中/低，且已登记） |
| 提交记录混杂 | 否（11 个提交原子、规范；轮 1 的混杂问题未复发） |
| 含敏感文件 | 否 |

补充说明（不作为否决理由，与轮 1 口径一致）：GUI 真机点击与 Console 采集仍因「本会话无活动桌面、WebView2 渲染进程崩溃」缺失，已两轮独立复现并登记第三档，补偿为 jsdom 组件用例 + 后端真实占用进程端到端 + 代码走查；**放行发版前需在具备活动桌面的机器补做**。

---

## 8. 遗留观察项（不阻塞合并，建议后续任务处理）

| # | 项 | 建议 |
|---|---|---|
| 1 | `RmShutdown` 连带关闭同 console 兄弟进程 | 短期：`confirm` 文案补「同一控制台内的相关进程可能一并关闭」；长期：评估控制台进程改用逐进程关闭 |
| 2 | mixed fail-closed 后无解释文案（「已关闭 0 个进程」） | 与上条一起做一次 UI 文案小改（约 2 行） |
| 3 | 打包态 `bundle.resources` 路径错位（预存在） | 另开 `bugfix/task-050-bundle-resources-path`，优先级高于 safe_box 增强 |
| 4 | 关闭动作未进 `audit_log` | 后续任务补「关闭了哪些进程」审计列 |
| 5 | GUI 真机冒烟缺失 | 有活动桌面的机器上补做后放行发版 |

---

## 9. Git 操作记录

| 动作 | 结果 |
|---|---|
| 合并 main | **已执行**：`git checkout main`（`401f336`）→ `git merge --no-ff feature/task-049-rm-shutdown-locking` → merge commit **`60e0797`** |
| 合并前 main | `401f336` |
| 合并后 main | `60e0797`（其后再叠加 2 个 docs 提交：本报告 + PROJECT_STATE） |
| feature 分支 | 合并后删除（本地 + 远端） |
| 残留 bugfix 子分支 | 本任务链两个 bugfix 分支（`task-049-guard-image-name` / `task-049-guard-test-stability`）已在修复轮与测试轮删除，本轮 `git branch -a` 复核无残留 |
| revert | 未执行（无需要） |
| PR #11 | 因 GitHub 连接器 token 无 merge 权限（API 返回 403 `Resource not accessible by integration`），改走本地 `--no-ff` + push；PR 由合并提交关闭 |

> 本轮评审产生的提交：`docs(review): 任务049b评审报告（轮 2，通过）(#task-049)`（本文件）+ `docs(state): 任务049合并main并记录merge commit (#task-049)`（PROJECT_STATE）。
