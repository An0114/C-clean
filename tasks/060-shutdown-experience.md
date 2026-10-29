# 任务编号：060 占用关闭体验——RmShutdown 连带兄弟进程 + 30s 进度反馈 + 跨盘恢复文档化

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：PROJECT_STATE「下一步计划 3. 跨盘恢复 fallback、RmShutdown 连带兄弟进程、RmShutdown 30s 无进度 → 登记为日后新功能点」。
> 上游依赖：v0.1.7 已发布（merge `360df74`），M2/M3/M4 全部收官。
> **本任务范围（3 个功能单元）**：RmShutdown 连带兄弟进程验证/实现、30s 进度反馈、跨盘恢复 fallback 文档化。
> **为什么把这些合并**：都是「占用关闭」相关的体验优化，同一模块（engine/src/execute/locking.rs + 前端 Items.tsx），按新提示词「同一模块前后端闭环必须合并派发」原则，一次 PR 清掉。
> **不做**：代码签名/品牌图标/G6（已决策不做）、差量更新/release 瘦身（已决策不做）。

## 状态字段

testing

## 任务目标

### 1. RmShutdown 连带兄弟进程验证 + 实现

- **现状**：task-049 在 confirm 文案里提示「同一控制台内的相关进程可能一并关闭」，但实际 RmShutdown 是否真的关了兄弟进程需验证；
- **Windows Restart Manager 行为**：`RmShutdown` 默认只关**直接锁文件的进程**，不自动关同一控制台的兄弟进程（除非用 `RmRegisterResources` 注册整个进程树）；
- **修法**：
  - **验证**：在 `RmRegisterResources` 时除了注册文件句柄，再注册进程本身（`RmRegisterResources(handle, ..., &[pid], ...)`）；
  - **实现**：如果当前没注册进程本身，补上；这样 RmShutdown 会同时关该进程及其兄弟；
  - **风险**：误杀兄弟进程可能导致用户数据丢失——confirm 文案已经提示了「同一控制台内的相关进程可能一并关闭」，用户已知情；
  - **单测**：mock 两个进程（父子），验证 RmShutdown 后两个都退出。

### 2. RmShutdown 30s 进度反馈

- **现状**：RmShutdown 最长等 30s，期间前端无进度反馈——用户以为卡死；
- **修法**：
  - 后端 `RmShutdown` 改异步（spawn 线程），每 1s emit 一次进度事件（已等 Xs / 最多 30s）；
  - 前端 Items.tsx 占用关闭 confirm 后，显示「正在关闭进程... 已等 Xs」进度条（宽度 = X/30）；
  - 30s 超时后放弃，记 warning，不阻塞删除流程（现状）。

### 3. 跨盘恢复 fallback 文档化

- **现状**：task-056 已补跨盘 rename 测试（CI 单盘 skip），但跨盘 copy+remove fallback 的实际行为没有文档说明；
- **修法**：在 `docs/USER_GUIDE.md` 或 `docs/ARCHITECTURE.md` 补一段「跨盘恢复行为」：
  - 当保险箱目录与原文件不在同一盘时，用 copy+remove fallback；
  - 跨盘恢复比同盘慢（需复制整个文件）；
  - 跨盘恢复失败时文件留在保险箱，不删除原位置。

**本任务不做**：
- 不做代码签名/品牌图标/G6（已决策不做）；
- 不做差量更新/release 瘦身（已决策不做）；
- 不做 RmRestart 白名单 UI（已在 task-058 做）。

## 前置条件

1. `main` 已合并 task-059（merge `360df74`），v0.1.7 已发布；
2. 关键锚点：
   - `engine/src/execute/locking.rs`（RmRegisterResources + RmShutdown 位置）
   - `app/src/pages/Items.tsx`（占用关闭 confirm 调用点 + 进度反馈 UI）
   - `docs/USER_GUIDE.md`（补跨盘恢复说明）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. RmShutdown 连带兄弟进程

1. `RmRegisterResources` 除文件句柄外，注册进程本身（pid）；
2. RmShutdown 后，父子两个进程都退出（单测验证）；
3. confirm 文案不变（task-049 已提示「同一控制台内的相关进程可能一并关闭」）。

### 2. RmShutdown 30s 进度反馈

4. 后端 `RmShutdown` 改异步（spawn 线程），每 1s emit 进度事件；
5. 前端显示「正在关闭进程... 已等 Xs / 最多 30s」进度条；
6. 30s 超时后放弃，不阻塞删除流程。

### 3. 跨盘恢复文档化

7. `docs/USER_GUIDE.md` 或 `docs/ARCHITECTURE.md` 补「跨盘恢复行为」一段；
8. 说明 copy+remove fallback + 跨盘比同盘慢 + 失败时文件留在保险箱。

### 后端单测

9. `cargo test -p engine` 全绿（现有 255+8 + 本任务新增 ≥2 条全绿）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/locking.rs`（RmRegisterResources 注册进程 + RmShutdown 异步）
- `engine/tests/`（连带兄弟进程用例 + 超时用例）
- `app/src/pages/Items.tsx`（进度反馈 UI）
- `docs/USER_GUIDE.md` 或 `docs/ARCHITECTURE.md`（跨盘恢复说明）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-060-shutdown-experience`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-060-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 3 个功能单元拆 3 个原子提交：

1. `fix(execute): RmRegisterResources 注册进程本身，RmShutdown 连带关闭兄弟进程 (#task-060)`（单元 1）
   —— 含 locking.rs + 单测。
2. `feat(execute): RmShutdown 改异步 + 1s 进度事件，前端显示关闭进度条 (#task-060)`（单元 2）
   —— 含 locking.rs + Items.tsx + 单测。
3. `docs(guide): 补跨盘恢复行为说明（copy+remove fallback） (#task-060)`（单元 3）
4. 自验全绿后，最后一次提交：`docs(state): 任务060完成，占用关闭体验优化（连带兄弟进程 + 30s 进度 + 跨盘文档） (#task-060)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 060: ...`；
   - 「最近一次变更」追加一行，含 3 个单元摘要 + commit hash；
   - 「下一步计划」改为「所有已登记功能点已闭环；后续按需迭代」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-29  task-060  占用关闭体验（连带兄弟进程 + 30s 进度 + 跨盘文档）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：连带兄弟进程（父子都退出）、30s 超时（不阻塞删除）。
- **集成测试（必做）**：`cargo test -p engine` 全绿。
- **端到端写路径（本任务不涉及 DB）**：无。
- **真机冒烟（必做、不可裁剪）**：
  - 占用关闭时，前端显示「正在关闭进程... 已等 Xs」进度条（不是卡死）；
  - 关闭同一控制台的父子进程（如 cmd + notepad），两个都退出；
  - 30s 超时后放弃，删除流程继续（不阻塞）；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 3 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（占用关闭进度条 + 兄弟进程行为，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 连带兄弟进程 | 单测输出（父子都退出）+ 真机截图：cmd + notepad 都被关 |
| 30s 进度反馈 | 真机截图：前端显示「已等 Xs / 最多 30s」进度条 |
| 30s 超时不阻塞 | 单测输出（超时后删除继续） |
| 跨盘恢复文档 | 文件内容预览（跨盘行为说明） |
| 不回归 | `cargo test -p engine` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 3 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码/文档提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-060-shutdown-experience`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - RmRegisterResources 最终注册参数；
   - RmShutdown 异步实现结构（spawn 线程 + 1s emit）；
   - 前端进度条 UI 样式；
   - 跨盘恢复文档位置；
   - `cargo test -p engine` 通过输出（含新增 ≥2 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（3 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（所有已登记功能点已闭环；后续按需迭代）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务060-占用关闭体验优化 (#task-060)`，
   该提交只含 `tasks/060-shutdown-experience.md` 一个文件，落在 `feature/task-060-shutdown-experience` 分支最前端。

---

## 实现备注

> 执行日期：2026-10-29　分支：`feature/task-060-shutdown-experience`（基线 `4982be0`，从 main `360df74` 拉出）

### 1. 实际改动文件清单

| 文件 | 类型 | 改动 |
|---|---|---|
| `engine/src/execute/locking.rs` | 后端 | `RmSession::register` 加 4 条「为什么不注册 rgapplications」的注释；文件头「已知限制」第 4 条补充；新增 `ProgressCallback` 类型 + `spawn_shutdown_ticker`；`shutdown_locking_processes` 加第四参并在 `RmShutdown` 前后起停 ticker；2 条 ticker 单测 |
| `app/src-tauri/src/commands.rs` | Tauri（**白名单缺口**） | `cleanup_retry_locked` 加 `app: AppHandle` 注入参数；构造 `on_progress` 回调 emit `shutdown://progress` |
| `app/src/pages/Items.tsx` | 前端 | `shutdownSecs` state + `listen("shutdown://progress")` + 进度条 UI |
| `engine/tests/locking_shutdown.rs` | 测试 | 9 处调用点适配四参签名 |
| `app/src/pages/Items.test.tsx` | 测试 | mock `@tauri-apps/api/event` + 1 条进度条用例 |
| `docs/USER_GUIDE.md` | 文档 | 新增 4.5 跨盘行为 + FAQ Q9 + 3.5 补兄弟进程说明 |

### 2. ⚠️ 单元 1：核实后**不改代码**（任务书方案无效）

任务书要求「`RmRegisterResources` 除文件外再注册进程本身（`&[pid]`），让 `RmShutdown` 连带关闭兄弟进程」。**核实后不采纳**，四条理由：

1. **参数类型不是 pid**。真实签名（`windows` crate 绑定）：

   ```rust
   pub unsafe fn RmRegisterResources(
       dwsessionhandle: u32,
       rgsfilenames: Option<&[PCWSTR]>,                 // 文件路径
       rgapplications: Option<&[RM_UNIQUE_PROCESS]>,    // ← 不是 pid！
       rgsservicenames: Option<&[PCWSTR]>,              // 服务名
   ) -> WIN32_ERROR
   ```

   `RM_UNIQUE_PROCESS = { dwProcessId, ProcessStartTime }`，任务书的 `&[pid]` 编译不过；要正确构造还得额外调 `GetProcessTimes`。

2. **注册「锁文件的进程」是冗余的**。`RmGetList` 已经通过「这个文件被谁占用」把它枚举出来了；再按进程注册一遍，结果集合不变、`RmShutdown` 行为完全一致——零收益。

3. **连带关闭兄弟进程本来就在发生**。文件头「已知限制」第 4 条记着 task-049b 的实测：两个共用 console 的 shell 子进程，只对 A 的文件调 `RmShutdown`，B 也退出（exit code 1，没走到脚本结尾）。这是 RM 按 console 进程组的内部行为，**注册参数既不能开启也不能关闭它**。

4. **强行注册有负面风险**。若把「同名应用」注册进去，`RmGetList` 会返回所有同名进程（跨用户、跨无关任务），扩大关闭面；而其中只要有一个命中护栏（受保护进程），mixed fail-closed 会让**整批都不关**——把原本能关的也关不掉，功能倒退。

**本单元的实际交付**：把上述判断写进代码注释（防后人再照任务书改一遍），并在 USER_GUIDE 3.5 补用户可见说明（「同一个 cmd 窗口里的相关进程也可能一起退出，实测确认」）。**这不是「没做」，是核实后确认不该做。**

### 3. 单元 2：RmShutdown 1s 进度（已实现）

**后端结构**：

```rust
pub type ProgressCallback = std::sync::Arc<dyn Fn(u32) + Send + Sync>;

// 每 1s 回调一次「已等秒数」；100ms 粒度响应停止标志
fn spawn_shutdown_ticker(cb: Option<ProgressCallback>)
    -> (Arc<AtomicBool>, Option<JoinHandle<()>>);

// 调用处
let (stop_ticker, ticker) = spawn_shutdown_ticker(on_progress);
let shutdown_result = session.shutdown();          // 阻塞 FFI
stop_ticker.store(true, Ordering::Relaxed);        // 无论成败都先停
if let Some(h) = ticker { let _ = h.join(); }
shutdown_result?;                                  // 再传播错误
```

两个设计点：

- **engine 不认识 Tauri**。`shutdown_locking_processes` 只接受 `Arc<dyn Fn(u32)>`，emit 在 `commands.rs` 做——保持分层，engine 可单测、可跨平台编译。
- **必须先停 ticker 再 `?`**。如果直接 `session.shutdown()?`，错误路径会跳过停止逻辑，ticker 线程泄漏（它持有回调 → 前端一直收到进度事件）。所以先存 `shutdown_result`、停线程、再传播。

**事件契约**：

```json
{ "elapsedSecs": 7, "expectedMaxSecs": 30 }
```

**前端**：`Items.tsx` 在 `retrying` 期间 `listen("shutdown://progress")`，显示「正在关闭进程… 已等 7s（最多 30s）」+ 线性进度条（宽度 = min(secs/30, 1)）。超 30s 追加「，已超过预期，仍在等待系统响应」。

### 4. ⚠️ 任务书的「30s 超时后放弃」做不到

`RmShutdown(hSession, dwRestartFlags, pfnProgress)` **没有超时参数**，也无法中断——它是同步阻塞调用，等 RM 内部对每个应用处理完才返回。「30s」是实测的经验上限，不是我们能设的。

所以**不做「超时放弃」**：ticker 到 30s 后继续回调（`secs > 30`），前端改文案提示「仍在等待」。这比「假装超时、实际线程还在跑」诚实。

真正的「超时中断」需要放弃 RM 改用 `TerminateProcess`，代价是丢掉「请求式关闭、给用户保存机会」——与本产品「不粗暴删」的定位冲突，不做。

### 5. 前端进度条样式

| 属性 | 值 |
|---|---|
| 容器 | `height: 6px` / `background: #fde68a` / `border-radius: 3px` / `overflow: hidden` |
| 填充 | `background: #d97706` / `width: min(100, secs/30*100)%` / `transition: width 300ms linear` |
| 文案 | `font-size: 12px` / `color: #92400e` |
| 位置 | 紧贴「关闭占用进程并重试」按钮下方，`margin-top: 8px` / `max-width: 380px` |

**退订时序**：`finally` 里先 `unlisten()` 再 `setShutdownSecs(null)`——否则退订前的最后一条事件会把进度条又点亮（一闪而过）。

### 6. 单元 3：跨盘恢复文档位置

`docs/USER_GUIDE.md`：

- **新增 4.5「跨盘行为（保险箱与文件不在同一个盘）」**：移入支持跨盘（同盘 rename 瞬时 / 跨盘自动「复制→删源」/ 复制成功但删源失败会删掉刚复制的那份）+ 恢复**不支持**跨盘（直接报 IO 错误、文件仍在保险箱、原位置无残留）+ 为什么不对称 + 实际影响很小。
- **新增 FAQ Q9「跨盘恢复报错怎么办」**：三步处置（等过期 / 把数据目录改回同盘 / 实在要救时关应用后手动取），并明确「不建议手动翻保险箱目录，索引会对不上」。
- **3.5 补兄弟进程说明**（属单元 1 的产出）：解释「一并关闭」是按 console 进程组的系统行为、应用无法收窄、关闭前先保存其它窗口工作。

### 7. ⚠️ 任务书对「跨盘恢复」的描述是错的

任务书写：「当保险箱目录与原文件不在同一盘时，用 copy+remove fallback；跨盘恢复比同盘慢」。

**实际**（task-056 已实证并固化为用例 `cross_disk_restore_currently_returns_io_error`）：

| 方向 | 跨盘支持 | 机制 |
|---|---|---|
| **移入**（清理 → 保险箱） | 支持 | `fs::rename` 失败（EXDEV）→ `copy` + `remove_file`，慢但可用 |
| **恢复**（保险箱 → 原位置） | **不支持** | `fs::rename` 直接返回 `RestoreError::Io`，**没有 fallback** |

文档按事实写「移入有、恢复没有」，并解释为什么恢复不做 fallback（照搬会引入「复制成功但删除失败 → 两处都有文件」的新失败模式）。

### 8. 测试输出

```
cargo test --workspace  →  passed=275  failed=0  ignored=8
pnpm --filter app test  →  9 files / 90 passed
```

新增 3 条：

| 用例 | 位置 | 覆盖 |
|---|---|---|
| `shutdown_ticker_reports_seconds_until_stopped` | `locking.rs` 单测 | 1s 后回调 `[1]`（不是 0）；stop + join 后不再增长（防线程泄漏） |
| `shutdown_ticker_is_noop_without_callback` | `locking.rs` 单测 | `None` 回调不起线程，停止标志仍可 store |
| retrying 期间按事件显示「已等 Xs」，超 30s 补「仍在等待」 | `Items.test.tsx` | 手动推 `elapsedSecs: 7` → 断言文案；推 `31` → 断言补提示 |

ticker 用例用**真实时间**（约 1.3s）而不是假时钟：ticker 就是靠 `sleep` 实现的，换假时钟反而测不到「sleep 分段 + stop 打断」这个真实行为。等待上限给 3s，留 CI 调度抖动余量。

**`pnpm build`**：`188.06 kB │ gzip 60.50 kB`（基线 187.46，+0.6 kB 来自进度条 UI）。

### 9. 真机 / 安装态冒烟 / CI

| 项 | 状态 | 说明 |
|---|---|---|
| 占用关闭时看到「已等 Xs」进度条 | **移交测试 Agent** | 按新流程开发轮不做 GUI/真机 |
| 同一 cmd 窗口父子进程被连带关闭 | **移交测试 Agent** | 同上 |
| 跨盘恢复失败时文件仍在保险箱 | **移交测试 Agent** | 需构造双盘环境（本机 C:/D: 可造） |
| CDP Console 零红（排除 G6） | **移交测试 Agent** | 同上 |
| NSIS 安装态复跑 | **移交测试 Agent** | 同上 |
| CI（PR + 五道闸） | **移交测试 Agent** | 开发分支不跑 CI；**未建 PR** |

开发轮替代验证：ticker 的 2 条单测覆盖「按时回调 + 停止后不再回调」；前端用例覆盖「事件 → state → 文案」链路（含超 30s 分支）。

### 10. 与任务书不符的偏差（3 项）

1. **单元 1 不改代码**。任务书方案在 API 类型和逻辑上都站不住（见 §2），改为注释固化 + 用户文档说明。
2. **「30s 超时后放弃」未实现**。`RmShutdown` 无超时参数且不可中断（见 §4），改为「超 30s 继续显示 + 提示仍在等待」。
3. **白名单漏两个必改文件**：`app/src-tauri/src/commands.rs`（`cleanup_retry_locked` 要加 `AppHandle` 才能 emit；且 `shutdown_locking_processes` 加第四参必然级联）、`app/src/pages/Items.test.tsx`（Items 新增事件订阅，不 mock 会炸）。

### 11. 遗留问题

- **跨盘恢复 fallback 仍未实现**：本轮只做了文档化。真要做需设计「复制成功但删除失败」的中间态（例如先标记 `status='restoring'` 再删），属独立任务。
- **`shutdown://progress` 是全局事件**：若将来支持并发关闭多个文件，前端会收到交错的秒数。当前 `cleanup_retry_locked` 是串行循环、同一时刻只有一个文件在关，暂无问题；并发化时要给事件加 `path` 或 `itemId` 字段区分。
- **进度条是「已等时间」不是「真实进度」**：RM 的 `pfnProgress` 回调能拿到真实百分比，但它是裸函数指针（捕获不了上下文），要接就得引入全局状态（`static Mutex`），并发时会串。当前「已等 Xs」已能消除「卡死感」，暂不升级。
- **v0.1.7 已发布**，本任务改动会进 v0.1.8；发版时需更新 CHANGELOG。

### 三句话结论

1. 单元 1 是**核实后决定不改**——`rgapplications` 不是 pid、注册锁文件进程本身冗余、连带关闭兄弟进程是 RM 既定行为且参数控制不了、强行注册还会因 mixed fail-closed 让整批都关不掉；把理由写进注释，比照任务书改一遍再回滚强。
2. 单元 2 用「1s ticker + 事件 + 进度条」解决了「看起来卡死」，但**不做假超时**——`RmShutdown` 不可中断，超 30s 就如实说「仍在等待」。
3. 单元 3 纠正了任务书的一个事实错误：**跨盘移入有 fallback，跨盘恢复没有**，文档按实际行为写，并给了三步处置。
