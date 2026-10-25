# 任务编号：052 GUI 走查 P1+P2 修复——G1 进度条 + G2 列表分页 + G3 source_task_id 接线 + G4 轮次口径 + G5 零字节语义

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：`docs/test-reports/gui-walkthrough-2026-10-25.md`（2026-10-25 真机走查）P1×3 + P2×2 共 5 项发版阻塞。
> 上游依赖：task-050（扫描/清理链路）、task-051（safe_box 时间筛选/多 session/来源列）、task-049（占用关闭）。
> **本任务范围（5 个修复单元）**：G1/G2/G3（P1）+ G4/G5（P2）；G6（P3 设置页 IPC 红错，已知项）不阻塞发版，留 task-053；RmShutdown/RmRestart 立项决策不做。
> **流程教训（来自走查报告 §四.3）**：凡新增「从 DB 字段派生 UI」的能力，验收必须包含**真实端到端写入**（点按钮 → 查 DB → 查 UI），不能只构造 DB 行。G3 就是因为 task-051 26 条全是构造 DB 行的「读路径」测试，没覆盖「写路径」才漏。

## 状态字段

pending

## 任务目标

修 5 个真机缺陷（3 P1 + 2 P2）：

### G1【P1】扫描进度条恒 0%
- **现象**：扫描全程进度条宽度 0，下方路径/计数正常刷新；结束后仍空。
- **根因**：`engine/src/scan/runner.rs:118-124` 每条 item emit 时 `percent: 0` 硬编码；`ScanOrchestrator` 从未做脚本级百分比换算。
- **修法**：在 `ScanOrchestrator` 按「脚本序号 / 脚本总数」给出基线百分比（如 5 个脚本各占 20%），脚本内按 `scanned_items` 做亚线性插值；任意时刻 `percent ∈ [0,100]`，所有脚本跑完时单调递增到 100。

### G2【P1】清理列表固定 100 条截断
- **现象**：`%TEMP%` 新建小文件（被锁）重新扫描后，清理列表找不到（实际排在 100 名之后）。
- **根因**：前端 `Items.tsx:176` 硬编码 `api("cleanup_list_pending", { limit: 100 })`，无 offset/分页/总数；后端 `list_pending.rs:39-42` 排序 `ORDER BY size_bytes DESC`（同 size 时顺序未定义，翻页跳行）。
- **修法**：
  - 后端 `list_pending` 入参加 `page, page_size`，返回 `{ items, total, page, page_size }`（复用 task-045 SafeBoxPage 模式）；
  - 排序改 `ORDER BY size_bytes DESC, id DESC`（稳定）；
  - 前端 `Items.tsx` 加分页栏（复用 History/SafeBox 分页 UI），默认 page_size=100。

### G3【P1】safe_box.source_task_id 恒 NULL
- **现象**：保险箱 Session 下拉完全空白、来源列全 `—`。
- **根因**：`app/src-tauri/src/commands.rs:149`（`cleanup_execute`）和 `:227`（`cleanup_retry_locked`）构造 `SafeBoxDeleter::new(&box_dir, &conn, None)`——第三参写死 None；`list_sessions` SQL 是 `WHERE source_task_id IS NOT NULL GROUP BY source_task_id` → 恒空集。
- **修法**：`execute_items` 在 `cleanup_task` INSERT 后拿到 `last_insert_rowid()`（`executor.rs:79-93` 已有），通过 deleter 工厂闭包把 `Some(task_id)` 传入 `SafeBoxDeleter::new(...)`；`cleanup_retry_locked` 复用同一次执行的 task 行。
- **验收硬要求**：必须有一条**真实端到端**用例——点「清理已选」入保险箱 → 查 `safe_box.source_task_id` 非 NULL → 查 UI 来源列出现 `#N`、Session 下拉出现 `session #N (M 条)`。**不允许再只构造 DB 行**。

### G4【P2】仪表盘「清理轮次」恒为 0
- **现象**：成功 78 / 失败 32 / 释放 921.9 MB 均正常，唯独「清理轮次」永远 0。
- **根因**：`engine/src/execute/executor.rs:79-82` 插入 `cleanup_task` 时 `session_id` 写死 NULL；`audit_log/stats.rs` 口径是 `COUNT(DISTINCT session_id)`——`COUNT(DISTINCT x)` 忽略 NULL，故恒为 0。
- **修法（按走查报告推荐方案，DevAgent 直接执行，不等产品拍板）**：把 `stats.rs` 的口径从 `COUNT(DISTINCT session_id)` 改为 `COUNT(*)`——「轮次」的语义本就是「清理批次」，一次执行一行 cleanup_task，COUNT(*) 更贴语义；同时避免补 session_id 的跨 session 聚合复杂度。验收：连续两次清理后卡片显示 2。

### G5【P2】「清理成功，释放 0 B」
- **现象**：部分批次「已清理 1 项，释放 0 B」，历史行 `bytes_freed: 0`、`err: null`。
- **根因**：`executor.rs:176-179` 先取 `fs::metadata(...).unwrap_or(0)` 作为 `bytes_before`；`SafeBoxDeleter::delete_file` 对 `NotFound` 幂等返回 `Ok(())`。于是「扫描后、清理前文件已被系统/其他程序删掉」时：删除成功 + 释放 0 B。
- **修法**：
  - `ExecResult`（或清理结果结构）增 `already_gone: bool` 字段（deleter 返回 `Ok(true)` 表示真删、`Ok(false)` 表示 NotFound 幂等）；
  - 后端聚合时 `already_gone` 不计入 `bytes_freed`，但单独计数 `already_gone_count`；
  - 前端结果条文案改为：「已清理 N 项，释放 X MB；M 项已不存在（未计入释放量）」（M=0 时不显示后半句）。
  - **不**算成 fail（语义上不算失败）。

**本任务不做**：
- 不做 G6（设置页首调 `rules_check_update` 红错，Tauri 2.x ipc.localhost 已知项，发版不阻塞）；
- 不做 RmShutdown/RmRestart（立项决策不做）；
- 不做 banner 下载进度反馈 / 差量更新（M3+，非阻塞）。

## 前置条件

1. `main` 分支已含 task-001~051 + ③ 关闭体验文案（merge `801af22`）全部产出；
2. 关键锚点：
   - `engine/src/scan/runner.rs:118-124`（G1 percent:0 硬编码）
   - `engine/src/scan/orchestrator.rs`（G1 编排层，需加脚本级换算）
   - `engine/src/execute/list_pending.rs:39-42`（G2 排序 + limit）
   - `app/src/pages/Items.tsx:176`（G2 前端硬编码 limit:100）
   - `app/src-tauri/src/commands.rs:149, 227`（G3 SafeBoxDeleter::new 第三参 None）
   - `engine/src/execute/executor.rs:79-93`（G3 cleanup_task INSERT + last_insert_rowid；G4 session_id 写死 NULL）
   - `engine/src/execute/safe_box.rs:122-127, 198-200, 573-577`（G5 SafeBoxDeleter NotFound 幂等；G3 insert_row + list_sessions SQL）
   - `engine/src/audit_log/stats.rs`（G4 COUNT(DISTINCT session_id) 口径）
   - `app/src/pages/Dashboard.tsx`（G4 sessionCount 卡片）
   - `app/src/pages/Items.tsx` / 清理结果条（G5 文案）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### G1. ScanOrchestrator 脚本级百分比

1. 在 `engine/src/scan/orchestrator.rs` 维护 `script_index / script_total` 状态；
2. 每脚本开始时基线 = `(script_index / script_total) * 100`；脚本内 item 事件把 `percent` 从「上一脚本完成值」插值到「下一脚本开始值」；
3. 所有脚本完成后 `percent=100`；
4. 前端 `Dashboard.tsx` 不改（直接取 `progress.percent`，已就位）。

### G2. cleanup_list_pending 分页

5. 后端 `engine/src/execute/list_pending.rs`：
   - 入参加 `page: u32, page_size: u32`（默认 page=1, page_size=100，clamp 上限 500）；
   - 返回 `{ items, total, page, page_size }`；
   - 排序改 `ORDER BY size_bytes DESC, id DESC`；
6. Tauri 命令 `cleanup_list_pending` shape 同步改；
7. 前端 `Items.tsx`：
   - state 加 `page, pageSize, total`；
   - 表格下方加分页栏 `[上一页] 第 X / Y 页 (共 N 条) [下一页]`（复用 SafeBox 分页栏样式）；
   - 翻页时清空 selected。

### G3. safe_box.source_task_id 真实接线

8. `engine/src/execute/executor.rs`：`execute_items` 把 `cleanup_task.id` 通过闭包传入 deleter 工厂（或在函数签名加参数）；
9. `app/src-tauri/src/commands.rs` 两处调用点（`cleanup_execute:149` / `cleanup_retry_locked:227`）同步改为 `Some(task_id)`；
10. **端到端验收**（不允许只构造 DB 行）：
    - 真实点「清理已选」→ 查 SQLite `SELECT source_task_id FROM safe_box ORDER BY id DESC LIMIT 1` 非 NULL；
    - 前端保险箱「来源」列显示 `#N`；
    - Session 下拉出现 `session #N (M 条)`；
    - 选该 session 后列表被正确过滤。

### G4. 清理轮次口径改 COUNT(*)

11. `engine/src/audit_log/stats.rs`：把「清理轮次」对应 SQL 从 `COUNT(DISTINCT session_id)` 改为 `COUNT(*)`（cleanup_task 行数）；
12. 连续两次清理后 Dashboard「清理轮次」卡片显示 2（不再恒 0）；
13. 不改 cleanup_task schema（不动 session_id 列，避免跨 session 聚合复杂度）。

### G5. 已不存在 vs 已清理语义

14. `engine/src/execute/safe_box.rs` 的 `SafeBoxDeleter::delete_file` 返回值从 `Result<(), String>` 改为 `Result<bool, String>`（true=真删、false=NotFound 幂等）；
15. `engine/src/execute/executor.rs` 聚合清理结果时统计 `already_gone_count`，不计入 `bytes_freed`；
16. Tauri 清理命令返回结构加 `already_gone_count: usize` 字段；
17. 前端清理结果条文案：
    - `already_gone_count == 0` → 「已清理 N 项，释放 X MB」（现状）；
    - `already_gone_count > 0` → 「已清理 N 项，释放 X MB；M 项已不存在（未计入释放量）」。

### 后端单测（新增）

18. 在 `engine/tests/` 下补：
    - G1：orchestrator 跑完 N 个脚本后 percent 单调递增到 100，任意时刻 ∈ [0,100]；
    - G2：list_pending 分页 total 正确、跨页不跳行（同 size 时 id DESC 稳定）；
    - G3：`execute_items` 真实跑一遍 → safe_box.source_task_id 非 NULL（关键回归测试，防再漏）；
    - G4：stats 查询返回的 sessionCount 等于 cleanup_task 行数（≥2 行时不再为 0）；
    - G5：deleter 对 NotFound 返回 Ok(false)；executor 聚合时 already_gone 不计入 bytes_freed。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/scan/orchestrator.rs`（G1 脚本级百分比）
- `engine/src/scan/runner.rs`（G1 配合 emit）
- `engine/src/execute/list_pending.rs`（G2 分页 + 稳定排序）
- `engine/src/execute/executor.rs`（G3 传 task_id；G5 聚合 already_gone）
- `engine/src/execute/safe_box.rs`（G3 已有；G5 delete_file 返回 bool）
- `engine/src/audit_log/stats.rs`（G4 COUNT(*) 口径）
- `app/src-tauri/src/commands.rs`（G3 两处调用点；G5 清理命令返回 shape）
- `app/src/pages/Items.tsx`（G2 前端分页栏；G5 结果条文案）
- `app/src/pages/Dashboard.tsx`（G4 sessionCount 显示，仅核对字段名，预期零改动）
- `engine/tests/` 下新增测试文件
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-052-gui-p1p2-fixes`，从最新 `main` 拉出。
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-052-<描述>`，**只允许一层**；bugfix 修复后直接合并回 feature，不进 main。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

按 5 个修复拆 5 个原子提交（每个功能点独立提交，禁止攒批）：

1. `fix(scan): ScanOrchestrator 加脚本级百分比换算，进度条单调递增到 100 (#task-052)`（G1）
   —— 含 `engine/src/scan/orchestrator.rs`、`engine/src/scan/runner.rs`、对应测试。
2. `fix(execute): cleanup_list_pending 分页 + total + size_bytes DESC, id DESC 稳定排序 (#task-052)`（G2 后端）
   —— 含 `engine/src/execute/list_pending.rs`、`app/src-tauri/src/commands.rs` 命令 shape、对应测试。
3. `fix(execute): execute_items 传 task_id 给 SafeBoxDeleter，safe_box.source_task_id 真实写入 (#task-052)`（G3 后端）
   —— 含 `engine/src/execute/executor.rs`、`app/src-tauri/src/commands.rs` 两处调用点、端到端测试。
4. `feat(frontend): Items 清理列表加分页栏（上一页/下一页/页码/total） (#task-052)`（G2 前端）
   —— 含 `app/src/pages/Items.tsx`。
5. `fix(stats): 清理轮次口径改 COUNT(*)，Dashboard 轮次卡片不再恒 0 (#task-052)`（G4）
   —— 含 `engine/src/audit_log/stats.rs`、对应测试。
6. `fix(execute): deleter 区分真删 vs 已不存在，结果条单列 already_gone (#task-052)`（G5 后端+前端）
   —— 含 `engine/src/execute/safe_box.rs`、`engine/src/execute/executor.rs`、`app/src-tauri/src/commands.rs`、`app/src/pages/Items.tsx`、对应测试。
7. 自验全绿后，最后一次提交：`docs(state): 任务052完成，GUI 走查 P1+P2（G1-G5）修复就绪 (#task-052)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 052: ...`；
   - 「最近一次变更」追加一行，含 G1-G5 修复摘要 + commit hash；
   - 「已知问题」段把走查报告 G1-G5 五条标记已解决；G6 保留为已知项；
   - 「下一步计划」改为「派发 task-053：复验走查（同一份清单重跑，NSIS/MSI 各一遍）+ release.yml target_commitish 小修 + v0.1.6 发版」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-25  task-052  GUI 走查 P1+P2 修复（G1 进度条/G2 分页/G3 source_task_id/G4 轮次口径/G5 零字节语义）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 170+ 条 + 本任务新增 ≥8 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：G1 百分比单调性、G2 分页 total + 稳定排序、G3 source_task_id 真实写入（关键回归）、G4 轮次口径、G5 already_gone 区分。
- **集成测试（必做）**：`cargo test -p engine` 全绿，确保现有不回归。
- **端到端（必做，本任务硬要求）**：G3 必须有一条**真实端到端**用例（点清理 → 查 DB → 查 UI），不能只构造 DB 行。
- **真机冒烟（必做、不可裁剪）**：
  - G1：点扫描 → 进度条单调递增，结束时到 100%（不再恒 0）；
  - G2：造 >100 条（含 1 个小文件）→ 第 2 页能找到小文件并勾选清理；
  - G3：点「清理已选」入保险箱 → 保险箱来源列显示 `#N`、Session 下拉出现 `session #N (M 条)`；
  - G4：连续两次清理 → Dashboard「清理轮次」显示 2（不再恒 0）；
  - G5：清理一个已被外部删除的文件 → 结果条显示「M 项已不存在」，不再「释放 0 B」；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，G1-G5 五条在安装态复跑一遍。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（Items.tsx 分页栏/结果条文案、Dashboard 轮次卡片、SafeBox 来源列/Session 下拉、Dashboard 进度条，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| G1 脚本级百分比 | 单测输出（monotonic + ∈[0,100] + final=100）+ 真机截图：扫描过程进度条非 0 |
| G2 后端分页 | 单测输出（total 正确 + 跨页不跳行） |
| G2 前端分页栏 | 真机截图：>100 条时第 1/2 页 + 小文件在第 2 页可见 |
| G3 source_task_id 真实写入 | **端到端证据**：① SQLite 查询 `SELECT source_task_id FROM safe_box ORDER BY id DESC LIMIT 1` 非 NULL；② 保险箱来源列截图显示 `#N`；③ Session 下拉截图显示 `session #N (M 条)` |
| G3 回归测试 | `cargo test` 输出含 `execute_items_writes_source_task_id`（或同名用例）passed |
| G4 轮次口径 | 单测输出（sessionCount = cleanup_task 行数）+ 真机截图：两次清理后轮次=2 |
| G5 already_gone 区分 | 单测输出（NotFound 返回 Ok(false)、already_gone 不计入 bytes_freed）+ 真机截图：结果条含「M 项已不存在」 |
| 不回归 | `cargo test -p engine` 全绿 + `pnpm test` 全绿 |
| 真机冒烟 | CDP 抓 page + consoleAPICalled（排除 G6 已知项后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + G1-G5 五条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 6 个代码提交 + 1 个 `docs(state)` 提交，共 7 个 commit，全部 push 到 `feature/task-052-gui-p1p2-fixes`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - G1 百分比换算最终算法（基线/插值公式）；
   - G2 page_size 默认值 / clamp 上限；
   - G3 端到端真实写入证据（SQL 查询结果 + 截图路径）；
   - G4 COUNT(*) 口径迁移前后对比；
   - G5 ExecResult/AlreadyGone 字段最终定义；
   - `cargo test -p engine` 通过输出；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（G1-G5 五条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（G6 设置页 IPC 红错留 task-053，已知项不阻塞）；
3. `docs/PROJECT_STATE.md` 已按第 7 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务052-GUI 走查 P1+P2 修复 (#task-052)`，
   该提交只含 `tasks/052-gui-p1p2-fixes.md` 一个文件，落在 `feature/task-052-gui-p1p2-fixes` 分支最前端。
