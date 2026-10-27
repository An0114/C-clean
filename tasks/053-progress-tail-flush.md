# 任务编号：053 进度条尾部冲刷 + 保险箱历史来源渲染——G7 收尾 100% + G8 NULL 来源文案

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：`docs/test-reports/gui-walkthrough-2026-10-25.md` §6.3（G7）、§6.2（G8）；PROJECT_STATE「下一步计划 ⑤」。
> 上游依赖：task-052（G1 进度条脚本级换算已合并 `d92db1b`，但真机复验发现收尾不满）。
> **本任务范围（5 个功能单元）**：G7（P1，唯一发版阻塞）+ G8（P2，随 bugfix 一起带）。
> **流程教训（走查报告 §6.4）**：① 测试替身不能替代生产链路——`VecEmitter` 断言 100 与 `ThrottledEmitter` 生产行为是两件事，凡经过节流/队列/去抖的通道必须有走真实包装层的用例；② 「数据为空」不等于「功能失效」，判定类缺陷前先确认观察对象是不是本次变更产生的数据。

## 状态字段

pending

## 任务目标

### G7【P1，唯一发版阻塞】扫描结束时进度条不满

- **现象**：扫描结束（按钮已变「再次扫描」）时进度条仍停在一段距离内，未到 100%。
- **根因（三重叠加）**：
  1. `ThrottledEmitter` 节流窗口内最后一条 `percent=100` 被丢弃；
  2. `ScanOrchestrator` 收尾事件没走强制发送；
  3. 前端 `Dashboard.tsx` 在 `scan_start` resolve 后立刻退订，最后一条事件来不及到。
- **测试为什么没抓住**：task-052 的 `scan_progress_percent` 集成测试用 `VecEmitter`（不经过 `ThrottledEmitter`），收尾 100 必然被记录 → 断言通过。**测试替身绕过了生产链路上的节流器**。
- **修法（三处都要动，缺一不可）**：
  - **后端 A**：`ThrottledEmitter` 加 trailing 补偿（被丢的最后事件暂存为 pending，窗口到点补发）或提供 `emit_force()` 供编排层收尾使用；
  - **后端 B**：`ScanOrchestrator` 所有脚本跑完后，调用 `emit_force()` 强制发送 `percent=100`；
  - **前端**：`scan_start` resolve 后主动 `setProgress(p => p && {...p, percent: 100})` 作为 UI 兜底；`unlisten` 延后 ~200ms 或改为「收到 100 或超时后再退订」；
  - **测试**：新增一条**经过 `ThrottledEmitter`** 的收尾事件用例（缩短 `MIN_INTERVAL` 或注入时钟），断言「最后到达的 percent === 100」。

### G8【P2】保险箱历史行 NULL 来源渲染为「—」造成误判

- **现象**：task-052 修复后，老数据（task-052 上线前入箱的 78 条）依然显示来源列 `—`、Session 下拉筛不到，用户以为「修了还是全 `—`」。
- **根因**：`source_task_id` 一旦写入 NULL 就补不回来（信息在当时就丢了）。
- **修法**：
  - 前端 `SafeBox.tsx` 来源列：`sourceTaskId == null` 时渲染「历史（无来源）」（淡灰色，区别于真 `—`）；
  - 筛选框旁加一句说明文案：「早于 v0.1.6 的条目无来源信息」；
  - **不改 DB**（历史数据无法追溯，强行回填反而引入风险）。

**本任务不做**：
- 不做 G6（设置页 IPC 红错，Tauri 2.x 已知项，发版不阻塞）；
- 不做 RmShutdown/RmRestart（立项决策不做）；
- 不做 banner 下载进度反馈 / 差量更新（M3+）。

## 前置条件

1. `main` 已合并 task-052（merge `d92db1b`，G1 脚本级换算、G2 分页、G3 source_task_id 接线、G4 轮次口径、G5 already_gone 已就位）；
2. 关键锚点：
   - `engine/src/scan/runner.rs`（ThrottledEmitter 封装位置，需看实现）
   - `engine/src/scan/orchestrator.rs`（收尾点，脚本循环结束后）
   - `app/src/pages/Dashboard.tsx:116-119`（进度条渲染 + scan_start resolve / unlisten）
   - `app/src/pages/SafeBox.tsx`（来源列 `sourceTaskId` 渲染 + Session 筛选框）
   - `engine/tests/scan_progress_percent.rs`（现有 VecEmitter 测试，**被绕过**——需新增 ThrottledEmitter 版本）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### G7.1 后端 ThrottledEmitter 尾部冲刷

1. 在 `ThrottledEmitter`（具体实现文件由 DevAgent 定位）加 trailing 补偿：被节流丢弃的最后事件暂存为 `pending`，下一个窗口起点补发；或暴露 `emit_force(event)` 方法；
2. 选择「trailing 补偿」或「emit_force()」任一方案均可，但必须保证**生产链路最后一条 percent=100 不会被吞**；
3. 现有 `VecEmitter` 测试保持通过（不破坏已有测试替身用法）。

### G7.2 ScanOrchestrator 收尾强制发送

4. 所有脚本跑完后，Orchestrator 显式发送一次 `percent=100` 事件（走 `emit_force()` 或 trailing 路径）；
5. 该事件**不能**被节流吞掉（单元测试断言）。

### G7.3 前端 Dashboard UI 兜底 + 延迟退订

6. `scan_start` Promise resolve 后：
   - 主动 `setProgress(p => p && {...p, percent: 100})`；
   - `unlisten` 延后 ~200ms 执行（或改为「收到 percent=100 事件后立刻退订 + 200ms 超时兜底退订」）；
7. 不破坏现有 `scannedItems` / `foundSize` 渲染。

### G7.4 经过 ThrottledEmitter 的尾部用例（关键回归）

8. 新增测试文件或用例：
   - 构造真实 `ThrottledEmitter`（注入缩短的 `MIN_INTERVAL` 或 fake clock）；
   - 模拟「Orchestrator 连续 emit 多个 percent 事件，最后一条是 100」；
   - 断言：**最后到达 subscriber 的事件 percent === 100**（不是 96/88）；
   - 同时断言事件总数 ≤ 节流前事件数（节流仍生效，不是全放行）。

### G8.1 保险箱 NULL 来源文案区分

9. `SafeBox.tsx` 来源列：
   - `sourceTaskId != null` → 显示 `#N`（现状）；
   - `sourceTaskId == null` → 显示「历史（无来源）」（淡灰色，如 `text-gray-400`）；
10. Session 筛选框旁加说明文案：「早于 v0.1.6 的条目无来源信息」（小字、灰色）。

### 后端单测（新增）

11. `cargo test -p engine` 全绿：现有 245+8 条不回归 + 本任务新增 ≥2 条（G7.4 关键回归 + 任意后端改动配套）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/scan/runner.rs`（ThrottledEmitter trailing / emit_force）
- `engine/src/scan/orchestrator.rs`（收尾强制发送）
- `engine/tests/` 下新增或修改 scan_progress_percent 测试（G7.4）
- `app/src/pages/Dashboard.tsx`（G7.3 UI 兜底 + 延迟退订）
- `app/src/pages/SafeBox.tsx`（G8.1 NULL 来源文案）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`bugfix/task-053-progress-tail-flush`（**从最新 `main` 拉出**，不是从 task-052 分支拉——task-052 已合并 main）；
  - 命名沿用 PROJECT_STATE「下一步计划 ⑤」的 bugfix 命名，但本质上是从 main 拉的新任务分支，走完整 PR 流程；
- 缺陷修复子分支：若评审打回，从当前 bugfix 分支拉 `bugfix/task-053-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `fix(scan): ThrottledEmitter 加 trailing 补偿（或 emit_force），最后事件不被节流吞 (#task-053)`（G7.1 后端）
   —— 含 `engine/src/scan/runner.rs` + 配套单测。
2. `fix(scan): ScanOrchestrator 收尾强制发送 percent=100 (#task-053)`（G7.2 后端）
   —— 含 `engine/src/scan/orchestrator.rs` + 配套单测。
3. `test(scan): 新增经过 ThrottledEmitter 的尾部用例，断言最后到达 percent=100 (#task-053)`（G7.4 关键回归）
   —— 含 `engine/tests/` 下测试文件。
4. `fix(frontend): Dashboard scan_start resolve 后兜底置 100 + 延迟退订；SafeBox NULL 来源渲染「历史（无来源）」 (#task-053)`（G7.3 + G8.1 前端）
   —— 含 `app/src/pages/Dashboard.tsx`、`app/src/pages/SafeBox.tsx`。
5. 自验全绿后，最后一次提交：`docs(state): 任务053完成，G7 进度条收尾 + G8 历史来源文案就绪 (#task-053)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 053: ...`；
   - 「最近一次变更」追加一行，含 G7/G8 修复摘要 + commit hash；
   - 「已知问题」段把走查报告 G7/G8 两条标记已解决；
   - 「下一步计划」改为「派发 task-054：第三轮复验走查（NSIS/MSI 各一遍）+ release.yml target_commitish + v0.1.6 发版」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-27  task-053  进度条尾部冲刷（G7）+ 历史来源文案（G8）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（245+8 现有 + 本任务新增 ≥2 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：G7.1 trailing/force、G7.2 收尾强制发送、G7.4 经过 ThrottledEmitter 的尾部用例（关键回归）。
- **集成测试（必做）**：`cargo test -p engine` 全绿，确保现有不回归。
- **端到端写路径（必做）**：本任务不涉及 DB 字段派生 UI（G8 只是 NULL 文案区分），但 G7.4 的「经过 ThrottledEmitter」用例本质上就是生产链路端到端，**不可用 VecEmitter 替代**。
- **真机冒烟（必做、不可裁剪）**：
  - G7：连续扫描 3 次，每次结束进度条都停在 100%（不是 96/88）；
  - G7：扫描结束后 `scannedItems` 计数与清理列表总数一致；
  - G8：保险箱老行（source_task_id=NULL）来源列显示「历史（无来源）」，新行仍显示 `#N`；筛选框旁有「早于 v0.1.6」说明文案；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，G7/G8 三条在安装态复跑一遍。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（Dashboard 进度条 + SafeBox 来源列/Session 筛选框，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| G7.1 trailing/force 实现 | 单测输出（被丢事件最终被补发）+ 代码 diff |
| G7.2 Orchestrator 收尾 100 强制发送 | 单测输出（脚本循环结束后收到 percent=100） |
| G7.4 经过 ThrottledEmitter 尾部用例 | 单测输出（**关键回归**：最后到达 percent === 100，且事件数 ≤ 节流前） |
| G7.3 前端兜底 + 延迟退订 | 真机截图：扫描结束进度条停在 100%（连续 3 次） |
| G8.1 历史来源文案 | 真机截图：老行显示「历史（无来源）」灰色、新行显示 `#N`、筛选框旁有说明文案 |
| 不回归 | `cargo test -p engine` 全绿 + `pnpm test` 全绿 |
| 真机冒烟 | CDP 抓 page + consoleAPICalled（排除 G6 已知项后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + G7/G8 复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `bugfix/task-053-progress-tail-flush`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - G7.1 最终方案选择（trailing 补偿 vs emit_force，二选一 + 理由）；
   - G7.4 关键回归用例代码摘录 + 通过输出；
   - G7.3 延迟退订实现细节（200ms 兜底 vs 收到 100 立刻退订）；
   - G8.1 文案最终样式（颜色、字号、位置）；
   - `cargo test -p engine` 通过输出；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（G7 三次扫描截图 + G8 老行/新行对比截图）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（G6 设置页 IPC 红错仍为已知项，发版不阻塞）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务053-进度条尾部冲刷与历史来源文案 (#task-053)`，
   该提交只含 `tasks/053-progress-tail-flush.md` 一个文件，落在 `bugfix/task-053-progress-tail-flush` 分支最前端。
