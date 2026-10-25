# 评审报告：task-051 保险箱时间区间筛选 / 多 session / 审计历史联表

> 评审角色：Review Lead（提示词 10）｜评审时间：2026-10-25
> 评审对象：`feature/task-051-safebox-filter-history` @ `7bfc130`（合并态纯代码 `c1a1af6`）
> 职责声明：**不重复跑功能测试、不修改业务源码**。仅核对证据链 + 代码走查 + 3 条关键路径抽查复现。

## 1. 输入核验

| 输入项 | 要求 | 实际 | 结论 |
|---|---|---|---|
| 测试报告 | `docs/test-reports/051-test-report.md` 存在且结论明确 | 存在（159 行，7 节齐全：环境矩阵 / L1–L5 / 验收 28 条 / 缺陷清单 / 修复记录 / 三档分流 / 结论） | ✅ |
| CI 运行号与结论 | 分支最新 run，`conclusion=success` | run **40168680216**（head `7bfc130`，4m24s，PR #16 synchronize）→ 页面 `Status: Success`；run **40168310323**（`c1a1af6`，合并态纯代码，4m54s）→ `Success` | ✅ 双 run 实测 |
| feature 分支最新 commit | 明确且可定位 | `7bfc130`（tip）；合并态代码 `c1a1af6`。本地 = origin，`rev-list --left-right --count` → `0 0` | ✅ |

**输入核验结论：三样齐全，可进入评审。**

### 1.1 事实核对中发现的一处非本任务问题

`main` 上存在一个**不属于 task-051 分支**的不规范提交：

```
5c91d3d  An0114  2026-10-25 11:36:47  ".."
  .workbuddy-ai/memory/2026-10-24.md | 33 +
  .workbuddy-ai/memory/2026-10-25.md | 48 +
```

- 提交信息为 `..`，**无 type / scope / #task 关联**，违反 `docs/AGENTS.md` 的提交规范；
- 内容是工作区记忆文件（上一轮 task-050 评审的日志），不涉及业务代码；
- **不在 `feature/task-051-*` 的 16 个提交内**，不构成本任务的一票否决（否决项针对「本次提交记录混杂/含敏感文件」）；
- 已登记为仓库卫生问题（见 §10.3），建议后续统一规范，**不得用 reset 改写 main**。

另：切换分支时该提交与工作区未跟踪的 `2026-10-25.md` 冲突，评审已先备份为 `2026-10-25.premerge-bak.md` 再切换，未丢失任何内容。

## 2. Git 提交记录核对

### 2.1 逐提交清单（16 个）

| # | Commit | 信息 | 文件 | 原子性 |
|---|---|---|---|---|
| 1 | `6eb7ba6` | `feat(engine): list_paged 改为动态 WHERE…` | `safe_box.rs` + 2 测试 + `commands.rs`(6 行适配) | ⚠️ 见 2.2 |
| 2 | `c1eba8d` | `feat(engine): 保险箱条目暴露 source_task_id…` | 2 文件 | ✅ |
| 3 | `2e05177` | `feat(ipc): safe_box_list 参数升级并注册…` | 2 文件 | ✅ |
| 4 | `0c4162c` | `test(engine): 覆盖时间区间边界与多 session…` | 3 测试文件 | ✅ |
| 5 | `ce7f7f0` | `feat(ui): 保险箱页支持时间预设/多选…` | `SafeBox.tsx` | ✅ |
| 6 | `142acc7` | `test(ui): 新增 SafeBox 组件用例…` | `SafeBox.test.tsx` | ✅ |
| 7 | `72c737b` | `docs(task): 补记任务051实现备注…` | `tasks/051-*.md` | ✅ |
| 8 | `3d00901` | `docs(state): 任务051完成，状态置 testing…` | `PROJECT_STATE.md` | ✅ |
| 9 | `b217b61` | `docs(state): 回填…40163134926 全绿` | `PROJECT_STATE.md` | ✅（碎） |
| 10 | `ad404df` | `docs(state): 记录任务051两轮 CI 均全绿` | `PROJECT_STATE.md` | ✅（碎） |
| 11 | `ef054aa` | `fix(ui): 刷新按钮不再把 click 事件当作 page…` | `SafeBox.tsx`（3+/1-） | ✅ 单行修复 |
| 12 | `05d5475` | `test(ui): 覆盖刷新按钮实参与筛选条件保持` | `SafeBox.test.tsx` | ✅ 测试独立 |
| 13 | `c1a1af6` | `merge: task-051 刷新按钮实参缺陷修复` | merge（2 文件） | ✅ `--no-ff` 合回 |
| 14 | `58ce7ba` | `docs(test): 任务051测试报告` | `test-reports/051-*.md` | ✅ |
| 15 | `7bfc130` | `docs(state): 任务051测试通过，状态 awaiting_review` | `PROJECT_STATE.md` | ✅ |
| 16 | `d020022` | `docs(test): 回填任务051 CI run…全绿` | 报告 + `PROJECT_STATE.md` | ⚠️ 见 2.3 |

- **规范符合性**：16/16 均为 `<type>(<scope>): <描述> (#task-051)`，type（feat/fix/test/docs/merge）与 scope（engine/ipc/ui/task/state/test）语义准确。**无关联缺失**。
- **bugfix 子分支**：`bugfix/task-051-refresh-click-arg` 已按 `--no-ff` 合回（`c1a1af6`），本地与远端**均无残留**（`git branch -a | grep 051` 只剩 feature 一条）。
- **敏感文件**：16 提交**零命中** `*.key` / `*.pem` / `*.db` / `*.sig`（`git diff --name-only main...HEAD` 实查）。
- **依赖**：`Cargo.lock` / `pnpm-lock.yaml` / `app/package.json` / `app/src-tauri/Cargo.toml` → **空 diff**（E22 成立）。

### 2.2 `6eb7ba6` 夹带 `commands.rs` 的判定

该提交标 `feat(engine)` 却改了 `app/src-tauri/src/commands.rs` 6 行。实查改动内容：

```rust
// task-051：`list_paged` 改为接收过滤结构；此处先把旧的单 session 入参
// 适配成 filter（命令签名升级见同任务的 safe_box_list 改动）。
let filter = engine::execute::safe_box::SafeBoxFilter {
    session_ids: session_id.into_iter().collect(),
    ..Default::default()
};
```

**判定：合理折中，不扣分。** 理由：
1. 目的是保证**该提交自身可编译**（否则中间提交是红的，违反「每个提交前门禁全绿」）；
2. 只有 6 行、无行为变更，且**提交信息正文已显式声明**这一点（未隐瞒）；
3. 真正的签名升级仍在 `2e05177`，职责未混淆。

### 2.3 `d020022` 文档混提的判定

标 `docs(test)` 但同时改了 `docs/PROJECT_STATE.md`（4 行）与测试报告（6 行）。

**判定：轻微规范瑕疵，不构成否决。** 任务书的硬约束是「不得把 `PROJECT_STATE.md` 混进**业务提交**」——该提交不含任何业务代码，属文档与状态之间的边界模糊（CI 号回填天然同时涉及报告与状态）。建议后续把状态回填统一放进 `docs(state)`，本条仅登记。

### 2.4 变更范围越界核对

`git diff --name-only main...HEAD` → 14 个文件：

| 文件 | 任务书允许 | 判定 |
|---|---|---|
| `engine/src/execute/safe_box.rs` | ✅ | ✅ |
| `engine/src/audit_log/query.rs` | ✅（仅新增 `list_by_target`） | ✅ 实查未动 `list_audit_log` / `AuditLogFilter` |
| `engine/tests/safe_box_{time_range,multi_session,item_history}.rs` | ✅（新增） | ✅ |
| `engine/tests/safe_box_{pagination,session_filter}.rs` | ✅（仅同步签名） | ✅ 断言逐字未动（§4.2 复核） |
| `app/src-tauri/src/commands.rs` | ✅ | ✅ |
| `app/src-tauri/src/lib.rs` | ✅（仅 1 行注册） | ✅ 实查仅 `+commands::safe_box_item_history` |
| `app/src/pages/SafeBox.tsx` / `.test.tsx` | ✅ | ✅ |
| `docs/PROJECT_STATE.md` / `tasks/051-*.md` | ✅ | ✅ |
| `docs/test-reports/051-test-report.md` | 流程产物 | ✅ |

**禁止清单逐项实查未触碰**：`App.tsx` / `Items.tsx` / `Dashboard.tsx` / `History.tsx` / `Settings.tsx` / `app/src/utils/**` / `paths.rs` / `progress.rs` / `tauri.conf.json` / `app/src-tauri/Cargo.toml` / `engine/src/execute/{executor,delete,whitelist,locking,error_kind}.rs` / `engine/src/db/**` / `engine/src/rules/**` / `engine/src/ipc/**` / `elevation/**` / `rules/**` / `scripts/**` / `.github/workflows/**` / `capabilities/default.json`。

## 3. 验收覆盖性核验

任务书 A/B/C/D/E/F 共 **28 条编号、26 条实质验收项**（F27 为 N/A 项），逐条对照：

| # | 验收标准（摘要） | 测试报告证据 | 抽查结果 | 结论 |
|---|---|---|---|---|
| A1 | `SafeBoxFilter` + `list_paged(…, &filter)` | §3-A1 + `safe_box.rs:339-369` | 代码实查：结构 + `normalized()` + `is_empty_range()` 齐备 | 通过 |
| A2 | 动态 WHERE，值一律占位符绑定 | §3-A2 + `:473-494` | 代码实查：只拼 `?` 数量，无字符串插值 | 通过 |
| A3 | items / COUNT / SUM 同一 WHERE 与参数 | §3-A3 + 变异 2 | 代码实查三条共用 `where_clause` + `filter_params` | 通过 |
| A4 | 闭区间 `>= from` `<= to`，单边也生效 | §3-A4 | **抽查复现**：`closed_interval_includes_both_bounds` 等 6 条全绿 | 通过 |
| A5 | `from>to` 空页不 Err；`<=0` 视为未设置 | §3-A5 | 抽查复现：`from_greater_than_to_returns_empty_page_not_error` ok | 通过 |
| A6 | 分页 clamp 与 `ORDER BY id DESC` 不变 | §3-A6 | 既有 5 条全绿（报告 + 代码实查 SQL 未变） | 通过 |
| A7 | 时间区间用例 ≥5 条 | §3-A7：6 条 | **抽查复现：6 passed / 0 failed** | 通过 |
| B8 | 空=不过滤；非空 `IN (...)`；去重 | §3-B8 | 代码实查 `normalized()` 保序去重 | 通过 |
| B9 | 与时间区间可叠加 | §3-B9 | 用例 `multi_session_with_time_range_and_pagination` | 通过 |
| B10 | 多 session 用例 ≥4 条 | §3-B10：6 条 | 报告逐条列出用例名 | 通过 |
| B11 | 既有 9 条只改签名、断言不放宽 | §3-B11 | diff 实查：仅 `None`→`&SafeBoxFilter::default()` 等，断言逐字未动、无 skip | 通过 |
| C12 | `source_task_id` → `sourceTaskId`，两条路径都填 | §3-C12 | 代码实查 `list_safe_box` 与 `list_paged` 均 SELECT 该列 | 通过 |
| C13 | `list_by_target`，NOCASE，limit clamp [1,100] | §3-C13 | 代码实查 SQL 与 `limit.clamp(1,100)` | 通过 |
| C14 | `item_history`，不存在 → Err 含 not found | §3-C14 | 代码实查；抽查复现 `missing_item_returns_not_found_error` ok | 通过 |
| C15 | 历史用例 ≥4 条 | §3-C15：6 条 | **抽查复现：6 passed / 0 failed** | 通过 |
| D16 | 4 档时间预设，切页重置+清空选中 | §3-D16 + 真浏览器实参 | 代码实查 `onChangePreset` → `setSelected(new Set())` + `load(1, …)` | 通过 |
| D17 | 多选 session + 全选/清空 | §3-D17 + 截图 2 | 代码实查 `<select multiple size={4}>` + 两个按钮 | 通过 |
| D18 | 来源列 `#N` / `—`，宽度 ≤90 | §3-D18 | 代码实查 `width: 90` + `r.sourceTaskId == null ? "—"` | 通过 |
| D19 | 行内历史展开：中文化/加载中/空/失败/单行/收起 | §3-D19 | 代码实查五态齐备；`toggleHistory` 用 `prev.id` 校验防竞态 | 通过 |
| D20 | 既有交互不破坏；刷新沿用筛选 | §3-D20 + 缺陷 1 修复记录 | 代码实查 `onClick={() => void load()}` + 注释说明 | 通过（修后） |
| D21 | `SafeBox.test.tsx` ≥6 条 | §3-D21：12 条 | **抽查复现：12 passed（12 tests）** | 通过 |
| E22 | 不新增依赖、不动锁文件 | §3-E22 | lock 文件空 diff（实查） | 通过 |
| E23 | 只改允许文件 | §3-E23 | 14 文件全部对齐（§2.4） | 通过 |
| E24 | 七条门禁 | §3-E24：fmt 0 / clippy 0 / 220+8 / lint 0 error / vitest 46 / build 179.94 kB / 契约 6/6 | 条数口径自洽（基线 197+8 → +23） | 通过 |
| F25 | dev 真机交互 | §3-F25：**未取得**（第三档） | 见 §3.1 专项判定 | 降级通过（任务书预设路径） |
| F26 | 安装包冒烟 | §3-F26：NSIS `/S /D=` exit 0 + 9 项布局 + 启动探针 | 报告含逐项清单与卸载记录 | 通过 |
| F27 | 造数据 | 未向用户库写入测试行（复用真实库只读） | 开发轮构造数据已清理，测试轮未新增 | N/A（合规） |
| F28 | GUI 缺口登记 | §6 第三档含窗口存活 / WV2 曲线 / dmp / CDP | 登记完整，无伪造 | 通过 |

**覆盖率：25/26 实测通过 + 1 条（F25）按任务书预设路径降级登记。全覆盖，无跳项。**

### 3.1 F25 GUI 取证的专项判定

F25 未取得运行时证据。任务书 F28 原文预设了降级路径：

> 「若本轮仍如此：必须**实测记录现象**（窗口标题存活时长 / Crashpad `.dmp` 时间戳 / CDP 探测结果），按第三档登记，用 D21 的 jsdom 组件用例 + 后端集成用例 + 安装包文件系统级证据补偿，**禁止伪造截图或日志**。」

逐条核对：

| 条件 | 实际 | 判定 |
|---|---|---|
| 实测记录现象 | dev：`hwnd=198844`、`responding=True`、WV2 子进程 0、Crashpad `13:27:44` 6.4 MB dmp、CDP `NETSTAT_9222_HITS=0`；安装版：`hwnd=592778`、WV2 `0 → 4(t=6s) → 0(t=9s)`、带调试参数时 CDP **t=1.5s 返回 HTTP 200/460 B** 但 attach 后 `Runtime.evaluate` 超时、target 停在 `about:blank` | ✅ 现象具体、含时间戳与量化采样 |
| 禁止伪造 | 明确写「未取得截图/Console，未伪造」；所贴截图有真实产物路径 | ✅ |
| D21 jsdom 补偿 | 12 条用例覆盖四交互 + 空态 + 收起 + 刷新实参 | ✅ |
| 后端集成补偿 | 18 条（时间区间 6 / 多 session 6 / 历史 6） | ✅ |
| 安装包文件系统补偿 | 9 项布局 / 零写入 / 数据目录重建 / 防劫持 | ✅ |
| 环境归因 | `main` 上同样现象（049b / 050 连续记录）→ 非本任务引入 | ✅ 可采信 |

**本轮新增补偿手段（评审认为显著优于前几轮）**：Edge `--headless=new` 真 Chromium 加载 Vite dev server + 真实 `SafeBox.tsx`（stub `__TAURI_INTERNALS__.invoke`），取得 **2 张截图 + DOM dump + 7 条 IPC 实参序列（`fromTs:1792819931`、`sessionIds:[9001,9002]`、`safeBoxId:71`）+ Console 0 错误**。

这把「组件在 jsdom 里能跑」升级为「**真实 Chromium 渲染真实组件 + 真实事件驱动 + 实参序列可核对**」，是对 GUI 缺口最有价值的一次补偿。

**判定：F25 按第三档降级成立。但 GUI 真机走查仍是发版前硬门禁（与 task-050 的 E20 合并为同一次走查）。**

### 3.2 契约核对表齐备性

测试报告 §2.1 给了 **9 行逐字段核对表**：列表命令名、列表入参、列表出参、条目字段、历史命令名、历史入参、历史出参、信封解包、事件 payload（N/A）。

评审独立复核了三处最易出错的：

| 契约点 | 后端 | 前端 | 复核 |
|---|---|---|---|
| 入参命名 | `session_ids` / `from_ts` / `to_ts`（snake） | `sessionIds` / `fromTs`（camel，Tauri 2 自动映射） | ✅ 实参序列证明映射生效 |
| 出参字段 | `SafeBoxEntry.source_task_id` + `#[serde(rename_all="camelCase")]` | `sourceTaskId: number \| null` | ✅ `Option<i64>` → `null` 正确 |
| 信封 | `Result<Envelope<T>, String>` | `api()` 判 `code!==0` 抛错 | ✅ 新增命令沿用同一模式，未另造 |

**契约核对表齐备，无缺项。**

### 3.3 本次评审的 3 条抽查复现（唯一执行的命令）

| 抽查项 | 命令 | 结果 |
|---|---|---|
| C15 历史查询（含 NOCASE / 排序 / 上限 / not found） | `cargo test -p engine --test safe_box_item_history` | **6 passed / 0 failed** |
| A4/A5/A7 时间区间（含闭区间、`from>to`、total 一致性） | `cargo test -p engine --test safe_box_time_range` | **6 passed / 0 failed** |
| D21 + D20 前端（含刷新实参用例） | `vitest run --pool=threads src/pages/SafeBox.test.tsx` | **12 passed（12 tests）** |

未执行全量功能测试（职责分离）。未因抽查结果修改任何测试结论。

## 4. 代码评审发现问题

### 4.1 架构与 TRD 符合度

| 维度 | 评价 |
|---|---|
| 与 TRD 的查询契约 | ✅ `SafeBoxFilter` 作为值对象承载过滤语义，`list_paged` 只消费不解释；`normalized()` 把「去重 / `<=0` 视为未设置」收敛到一处，调用方无需重复判断 |
| 重构方向 | ✅ 把 `match session_id` 的两套硬编码 SQL（加时间区间会变 8 分支）改为动态 WHERE + 顺序绑定，是**正确的重构**而非临时打补丁 |
| `total` 一致性 | ✅ 三条查询共用同一个 `where_clause` 与同一份 `filter_params`，从实现层消灭了「列表筛了、汇总没筛」这一类最常见 bug |
| 模块边界 | ✅ `audit_log/query.rs` 只加通用查询 `list_by_target`，`safe_box.rs` 的 `item_history` 负责「id → path → 历史」的业务编排，**依赖方向正确**（execute 依赖 audit_log，未反向） |
| 复用而非新建 | ✅ 复用 `AuditLogEntry`，未造同义结构（任务书 C13 明确要求） |
| 前端状态管理 | ✅ 筛选条件变化统一走 `setSelected(new Set())` + `load(1, …)` + 收起历史，避免「第 2 页 + 新筛选」空页与跨筛选误批量操作——这是**主动设计的正确性保障**，不是碰巧 |

### 4.2 关键实现复核（评审逐点验过）

**SQL 注入面（A2/A3 的核心）**：

```rust
let mut clauses: Vec<String> = vec!["status='in_box'".to_string()];
if !session_ids.is_empty() {
    let holders = vec!["?"; session_ids.len()].join(", ");
    clauses.push(format!("source_task_id IN ({holders})"));
}
```

拼进 SQL 的只有**固定片段**与**占位符个数**；`session_ids` 与时间戳全部经 `params_from_iter` 绑定。`from`/`to` 用 `>=` / `<=` 硬编码，未做任何字符串插值。**无注入面。**

**参数顺序正确性**：`filter_params` 的构造顺序（session ids → from → to）与 WHERE 中占位符出现顺序严格一致，分页参数追加在末尾对应 `LIMIT ? OFFSET ?`。**若顺序错位，6 条时间区间用例会立即红**（已抽查复现全绿）。

**`from > to` 短路**：在两条聚合查询之前返回空页，`page` / `page_size` 用 clamp 后的值原样回显（A5 要求「原样回显」）。✅

**既有测试「只改签名」的复核**：`git diff main...HEAD -- engine/tests/safe_box_pagination.rs engine/tests/safe_box_session_filter.rs` 实查——改动仅为 `None` → `&SafeBoxFilter::default()` / `Some(1)` → `SafeBoxFilter { session_ids: vec![1], .. }` 与 import，**断言逐字未动、无用例删除、无 `#[ignore]`**。B11 成立。

**前端竞态处理**：`toggleHistory` 在 `await` 后回填时用 `prev.id === item.id` 校验——请求期间用户切到别的行时，旧响应不会污染新展开行。这是**很多人会漏的细节**，实现正确。

**刷新按钮修复**：`onClick={() => void load()}`，`load` 默认参数取当前 `page` / `pageSize` / `timePreset` / `sessionIds` → 既修了「事件对象当 page」，又满足 D20「沿用筛选条件」。附带注释解释为什么不能直接 `onClick={load}`，**防复发意图明确**。

### 4.3 错误处理与日志

- `list_paged` 的 `map_err` 分别标注 `count safe_box` / `sum safe_box` / `prepare list_paged` / `query_map` / `row`，排障可定位到具体语句 ✅
- `list_by_target` 同理（`prepare` / `query_map` / `row`）✅
- ⚠️ **P3 观察（本次唯一实质代码问题）**：`item_history` 里

  ```rust
  .map_err(|_| format!("safe_box item not found: id={safe_box_id}"))
  ```

  把 `query_row` 的**所有**错误都映射成 `not found`，包括 DB 打不开、表结构异常、锁冲突等。虽然当前 schema 稳定、实际风险低，但一旦真实 DB 出问题，日志会误报「条目不存在」，掩盖真因。**建议**：只对 `rusqlite::Error::QueryReturnedNoRows` 映射成 not found，其余 `format!("query safe_box: {e}")` 透传。
- ⚠️ **P4 观察**：`commands.rs` 的 `safe_box_item_history` 把所有 engine 错误统一返回 `Envelope::err(INTERNAL)`，前端只能显示「加载失败：internal error」，拿不到「条目不存在」的具体原因。属**既有命令层统一模式**（本任务未引入），非本任务缺陷；若将来要做精细错误码，需整体设计。

### 4.4 并发与资源释放

- `stmt` 与 `rows` 的生命周期正确（`rows` 迭代完才构造返回值），无借用逃逸。
- `item_params` 复用 `filter_params`（move），避免重复 clone ✅
- 前端 `load` / `toggleHistory` 均用 `useCallback` 且依赖数组完整（`[page, pageSize, timePreset, sessionIds]` / `[history.id]`），无陈旧闭包。
- 无新增线程 / 无异步资源泄漏面。

### 4.5 新增依赖

**零新增**（lock 文件空 diff 实查）。前端纯函数单测未引入任何测试库，复用既有 vitest + jsdom。

### 4.6 其他观察（不阻塞）

| # | 观察 | 说明 | 严重度 |
|---|---|---|---|
| O1 | `normalized()` 用 `ids.contains(id)` 去重 | O(n²)；session 数量级为个位/十位，实测无影响 | P4 |
| O2 | `COLLATE NOCASE` 会使 `target` 列上的 BINARY 索引失效 | `audit_log` 数据量小（单机应用），全表扫可接受；若将来数据量大，可建 `target COLLATE NOCASE` 索引 | P3（性能） |
| O3 | 2 条 `react-refresh/only-export-components` warning | `presetFromTs` / `actionLabel` 从 `SafeBox.tsx` 导出供单测；`app/src/utils/**` 在禁止修改范围，无法另建文件。0 error，在 `--max-warnings 10` 内 | P4（已登记） |
| O4 | `colSpan=6` 与任务书写 7 不符 | 表格实际 6 列（勾选/原路径/大小/移入时间/来源/操作），写 7 会多跨一列。**实现正确，任务书笔误** | 非问题 |
| O5 | `session_ids` 非空时 `source_task_id IS NULL` 的行不会被命中 | SQL NULL 语义正确行为，前端「来源」列显示 `—`，符合预期；实现备注已说明 | 非问题 |

## 5. 安全风险检查（OWASP Top 10）

| 类别 | 结论 | 依据 |
|---|---|---|
| A01 越权 / 访问控制 | ✅ 无新增面 | 本地单用户桌面应用，无账号体系；`safe_box_item_history` 按 id 只读，不涉及跨用户数据。未引入任何「按参数绕过权限」的路径 |
| A02 加密机制失效 | ✅ 不涉及 | 未触碰签名 / 密钥 / TLS；测试用临时密钥 `%TEMP%` 未入库 |
| A03 **注入**（本任务核心风险面） | ✅ **安全** | 动态 SQL 只拼固定片段与 `?` 占位符；session id / 时间戳全部 `params_from_iter` 绑定；`from`/`to` 用硬编码运算符。`list_by_target` 的 `target` 也是 `?1` 绑定。**逐条代码走查 + 6 条时间区间用例实跑，无注入路径** |
| A04 不安全设计 | ✅ 改善 | `from>to` 显式返回空页而非静默回落全量；`<=0` 视为未设置（避免「时间戳 0 被当成 1970 年筛选」的隐性行为）；筛选变化统一重置页码与选中，消除「跨筛选误批量删除」这类危险操作路径 |
| A05 安全配置错误 | ✅ 无变化 | 未动 CI / capabilities / tauri.conf |
| A06 易受攻击组件 | ✅ 无变化 | 零新增依赖 |
| A07 认证与会话 | ✅ 不涉及 | — |
| A08 不安全反序列化 | ✅ 无新增 | `SafeBoxFilter` 由命令层显式构造，未直接反序列化任意用户 JSON；`AuditLogEntry` 结构未变 |
| A09 日志与监控 | ⚠️ 可接受 | 错误日志含原始文件路径（本地 stderr，不上报远端）。`item_history` 的错误归因过宽（§4.3 P3），会**降低**排障准确度但不泄露信息 |
| A10 SSRF | ✅ 不涉及 | 无网络请求 |

**安全结论：无阻塞项。** 本任务的最大风险面（动态 SQL 拼接）实现正确，未引入注入。

## 6. 状态与文档一致性核对

**核对方式**：`git log` / `git branch -a` / `git rev-list` / 文件系统实查。

| 断言 | 实查结果 | 结论 |
|---|---|---|
| 分支 `feature/task-051-safebox-filter-history` 存在 | 本地 + `origin` 均存在 | ✅ |
| 本地与 origin 同步 | `rev-list --left-right --count` → `0 0` | ✅ |
| bugfix 子分支已清理 | `git branch -a \| grep 051` → 只剩 feature 一条；`bugfix/task-051-refresh-click-arg` 本地/远端均无 | ✅ |
| `main` 未被提前污染 | 合并前 `main` = `5c91d3d`，与 `origin/main` `0 0`；不含任何 task-051 代码 | ✅ |
| 状态字段 | `PROJECT_STATE` 当前负责人 = 「评审 Agent（提示词 10）… 状态 `awaiting_review`，移交评审判定与合并」 | ✅ 与 `awaiting_review` 一致 |
| 已完成事项 | 051 已勾选，并注明「测试轮通过，状态 `awaiting_review`，未合 main」 | ✅ |
| 已知问题关闭 | `[B 低] task-018 list 边界` 的「时间范围筛选 / 多 session / audit_log 联表」三项已划删除线并写明落地方式 | ✅ |
| 第三档登记 | ① GUI 取证第 4 轮（含 hwnd / WV2 曲线 / dmp 时间戳 / CDP 探测）② 前端 vitest forks 池 `EPERM`（判定会话沙箱限制，附 CI 对照） | ✅ 两项均正式登记 |

**发现 2 处文档滞后（不阻塞）**：

1. **`Git 提交记录`表未列测试轮 6 个提交**：表里最后一条是 `72c737b`，其后的 `3d00901` / `b217b61` / `ad404df` / `ef054aa` / `05d5475` / `c1a1af6` / `58ce7ba` / `7bfc130` / `d020022` 均用「（本次）」占位或未列。属项目既有惯例（部分历史也用占位），但本次占位跨度较大，建议合并后一次性补齐。
2. **`最近一次变更`里开发轮段落仍写「状态 `testing`，未合 main」**：这是**历史记录**（记录当时状态），不改写是正确做法，不算不一致。仅提示读者按时间顺序阅读。

**任务书 `## 状态字段` 仍为 `pending`**：与 047/049/050 惯例一致（任务书状态字段不随流转维护），不判为不一致。

## 7. 评审结论

# 【通过】

判定依据（逐项对照一票否决清单）：

| 否决项 | 是否触发 |
|---|---|
| 测试报告缺失 | 否 — 存在且 7 节齐全 |
| CI 红 | 否 — run 40168680216（`7bfc130`）与 40168310323（`c1a1af6`）均 `Success`（页面实查） |
| 验收标准未逐条覆盖 | 否 — 26 条全覆盖，25 条实测 + F25 按任务书预设路径降级 |
| 存在未处理的严重缺陷 | 否 — 测试轮 1 项缺陷已修并回归；评审新增 0 阻塞、1 项 P3 建议 |
| 提交记录混杂 | 否 — 16 提交全部原子、规范、带 `#task-051`；2 处轻微瑕疵（`6eb7ba6` 保编译适配已声明、`d020022` 文档混提）不构成混杂 |
| 含敏感文件 | 否 — 本分支零命中；`latest.json` 为 task-035 历史遗留（非本任务） |

**放行条件（必须随合并记录，不得静默丢弃）**：
GUI 真机走查仍未闭环（连续第 4 轮）。**task-050 的 E20（扫描 → 清理）与本任务的保险箱四交互，合并为同一次发版前真机走查**，需在有活动桌面的机器上完成，含 DevTools Console 零红。

## 8. 不通过时：Bug 清单

**不适用（结论为通过）。**

以下为不阻塞的观察项，登记备查：

| # | 现象 | 复现 / 位置 | 期望 | 严重度 | 建议 |
|---|---|---|---|---|---|
| O1 | `item_history` 把 DB 任意错误都报成 `not found` | `safe_box.rs` `item_history` 的 `.map_err(\|_\| …)` | 仅 `QueryReturnedNoRows` 报 not found，其余透传原始错误 | P3 | 后续小改，5 行以内 |
| O2 | `COLLATE NOCASE` 使 `target` 列 BINARY 索引失效 | `audit_log/query.rs` `list_by_target` | 数据量增长后加 NOCASE 索引或改为规范化存储 | P3（性能） | 观察，暂不动 |
| O3 | 2 条 `react-refresh` warning | `SafeBox.tsx` L79/L86（CI annotation 已标） | 纯函数移出到 `utils/`（当前在禁止修改范围） | P4 | 另开任务或放宽规则 |
| O4 | `normalized()` 去重 O(n²) | `safe_box.rs` `normalized()` | 规模无关紧要 | P4 | 不处理 |
| O5 | 3 个连续 `docs(state)` 碎片提交 | `b217b61` / `ad404df` / `3d00901` | 可合并为一次 | P4（规范） | 不追溯 |
| O6 | `main` 上 `5c91d3d ".."` 不规范提交 | §1.1 | 提交信息应符合 `type(scope): … (#task-XXX)` | P3（卫生） | 登记，不改写历史 |

**bugfix 子分支建议名：不适用**（0 阻塞缺陷）。

## 9. Git 操作记录

| 步骤 | 命令 / 动作 | 结果 |
|---|---|---|
| 0 | 处理分支切换冲突：`.workbuddy-ai/memory/2026-10-25.md` 被 `5c91d3d` 跟踪，工作区有同名未跟踪文件 | 先 `mv` 备份为 `2026-10-25.premerge-bak.md`（**未丢失内容**），再切换 |
| 1 | `git checkout main` | 切到 `main`（合并前 `5c91d3d`，与 `origin/main` `0 0`） |
| 2 | `git merge --no-ff feature/task-051-safebox-filter-history -m "merge: task-051 保险箱时间筛选与审计联表 (#task-051)"` | ✅ 无冲突，14 文件 / +1749 −244 |
| 3 | **merge commit** | **`e6ab561efcec042dee0dd448e02135ac0597a015`**（`e6ab561`，parents `5c91d3d` + `d020022`，`--no-ff` 生效） |
| 4 | `git push origin main` | 见下（报告与状态提交后一并推送） |
| 5 | 删除 `feature/task-051-safebox-filter-history`（本地 + 远端） | 见下 |
| 6 | 残留 bugfix 子分支复核 | `bugfix/task-051-refresh-click-arg` 本地/远端**均无残留**（测试轮已删） |
| 7 | revert 记录 | **无**（未发生需回滚的严重问题） |

**未执行的动作（合规声明）**：
- 未在评审通过前合并 feature；
- 未使用 `reset` / `force push` 改写历史（含 `main` 上的 `5c91d3d`）；
- 未修改任何业务源码（本次评审仅新增 `docs/reviews/051-review.md` 与 `PROJECT_STATE.md` 状态字段）。

## 10. 后续动作建议

1. **发版前真机走查（硬门禁，一次覆盖两项）**：在有活动桌面的机器上同时完成 ① task-050 的 E20「扫描 → 清理」不返 `INTERNAL`；② task-051 的保险箱四交互（时间预设 / 多 session / 来源列 / 历史展开）+ DevTools Console 零红。**此项未闭环不建议发版**。
2. **推进下一步计划 ③**（task-049 遗留观察项，约 2 行）：`confirm` 补「同一控制台内的相关进程可能一并关闭」；mixed 场景补「因存在受保护进程，本次未关闭任何进程」。这是当前唯一不依赖真机、可立即交付的小改。
3. **卫生任务（低优先，可合并处理）**：
   - `main` 上 `5c91d3d ".."` 的不规范提交信息（不改写历史，只在状态文档里留痕说明）；
   - `latest.json` 仍被 Git 跟踪（task-035 遗留，内容为占位）；
   - `Git 提交记录`表补齐 task-051 的 9 个未列提交。
4. **P3 小改（可随下一个任务顺手做）**：`item_history` 的错误映射收窄到 `QueryReturnedNoRows`（O1）。
5. **PR #16（draft）**：合并已由本地 `--no-ff` 完成，draft PR 应手动关闭（GitHub 连接器无 merge/关闭权限的先例见 task-049 / task-050）。
6. **`RmRestart` 产品决策**仍挂起（task-049 遗留），提醒不要因连续合并而遗忘。

---

**评审人**：Review Lead（提示词 10）
**评审日期**：2026-10-25
**报告路径**：`docs/reviews/051-review.md`
