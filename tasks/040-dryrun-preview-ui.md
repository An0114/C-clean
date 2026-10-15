# 任务编号：040 dry-run 预览 UI——Items 页「预览可清理项」按钮 + cleanup.dryRun 汇总/红线明细面板

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFrontendReact
> 来源：`docs/PROJECT_STATE.md`【下一步计划】第 ① 项——「派发 task-040：dry-run 预览 UI——前端「预览可清理项」按钮，调 cleanup.dryRun 显示大小汇总」；`docs/reviews/task-039-batch-restore-review.md` §后续动作建议第 1 条同样指向本任务。
> 上游依赖：task-004（`cleanup.dryRun` 纯逻辑 + `DryRunResp`）、task-010（外部红线并入 `handle_dry_run`）、task-014（`cleanup_dry_run` 已切 `DbItemPathResolver` 按 id 反查真实路径）、task-026（`formatBytes`）、task-029（Items 表格 + 勾选 + 清理已选）、task-032（Vitest 2.1 + jsdom）。
> **克制范围**：只做「点按钮 → `cleanup.dryRun` → 只读展示」。**不做**把 dryRun 变成「清理已选」的强制前置校验（留 task-041）、不做确认弹窗改造、不做进度条、不做预览结果持久化。

## 状态字段

`pending`

- 初始值 `pending`；流转：`pending` → `in_progress`（DevAgent 认领）→ `review`（提交待评审）→ `done`（评审 Approve 且合并 main 后由 DocsKeeper 置位）。
- 状态变更只在本任务文档与 `docs/PROJECT_STATE.md` 两处同步，不在别处另记一份进度。

## 任务目标

后端 `cleanup.dryRun`（task-004/010/014）已经能对一批 `cleanup_item.id` 逐条跑「内置 L3 红线 ∪ 外部红线」，返回 `totalItems / totalBytes / breakdown / blockedByL3`，但**前端从未接入**：`app/src/pages/Items.tsx` 目前只有「刷新」+「清理已选」两个按钮，用户点「清理已选」就直接 `cleanup_execute` 真删，看不到「将要释放多少空间、按模块怎么分、有没有条目会命中红线被拦」。

本任务补最小闭环：Items 页加「预览可清理项」按钮 —— 对当前勾选项调 `cleanup_dry_run`，只读展示大小汇总；命中红线时展示被拦明细。**只预览，不执行**：预览面板不新增第二个执行入口，用户确认后仍走既有「清理已选」按钮。

1. 新增前端纯函数层 `app/src/utils/dryRun.ts`（类型 + 文案/排序/红线原因映射），配 Vitest 单测；
2. `app/src/utils/invoke.ts` **仅新增** `apiRaw()`（原样返回 `{code,message,data}` 信封），使预览能读到 2002 的 `blockedByL3` 明细；`api()` 语义零改动；
3. `app/src/pages/Items.tsx` 加「预览可清理项」按钮 + 预览面板（汇总 / 分类明细 / 红线明细 / 空清单提示 / 失败提示）。

**本任务不做**：

- 不把 dryRun 接成「清理已选」的前置校验（点「清理已选」仍直接调 `cleanup_execute`）——留 task-041；
- 不做预览结果缓存 / 落库 / 写 `audit_log`（dryRun 全程只读，不产生任何持久化副作用）；
- 不改 Items 表格列、勾选逻辑、`cleanup_list_pending` 调用；
- 不改 Rust 侧任何代码（`cleanup_dry_run` 命令与 `handle_dry_run` 已满足需求）；
- 不新增 npm 依赖；不改 `package.json` / `Cargo.toml` / 锁文件。

## 前置条件

1. `main` 分支已含 task-001~039 全部产出，工作区干净；
2. 关键锚点（只读这些文件，不做全仓扫描）：
   - `app/src/pages/Items.tsx`（task-029 产出：`selected: Set<number>` + 表格 + 「清理已选」）；
   - `app/src/utils/invoke.ts`（`api<T>()`：`code !== 0` 抛 `Error(message)`，成功返回 `data`）；
   - `app/src/utils/formatBytes.ts`（`formatBytes(bytes: number): string`，task-026）；
   - `engine/src/ipc/commands/dry_run.rs`（`DryRunReq { item_ids }` / `DryRunResp { total_items, total_bytes, breakdown, blocked_by_l3 }`，`serde(rename_all = "camelCase")`）；
   - `engine/src/ipc/error_codes.rs`（`2001 待清理清单为空` / `2002 检测到受保护路径，操作已拒绝（L3 红线）`）、`engine/src/execute/whitelist.rs`（`DenialReason` 变体：`L3PersonalDir` / `L3BrowserProfile` / `L3BrowserSensitive` / `L3SystemPath` / `L3ReservedAction` / `PathTraversal` / `ResolveFailed(String)`）；
   - `app/src-tauri/src/commands.rs`（`cleanup_dry_run` 入参 `item_ids: Vec<i64>`，返回 `Result<Envelope<DryRunResp>, String>`）。
3. 已知约束：`cleanup_dry_run` 的 2002 分支**显式偏离 TRD §5.1**（`err_with_data`），即 `code=2002` 时 `data` 非 null 且含 `blockedByL3` —— 这正是本任务要展示的信息，前端不得丢弃。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选；评审逐条对照运行时证据。

### A. 纯函数层 `app/src/utils/dryRun.ts`（新建）

1. `A1` 导出与后端 camelCase 对齐的类型（`BreakdownItem { moduleCode, category, bytes }`、`BlockedItem { itemId, path, denialReason }`、`DryRunResp { totalItems, totalBytes, breakdown, blockedByL3 }`）。
2. `A2` `formatDryRunHeadline(r)` → 复用 `formatBytes`，形如 `将清理 3 项，共 1.2 GB`；`totalItems === 0` 时返回 `没有可预览的条目`。
3. `A3` `sortBreakdown(rows)` → 按 `bytes` 倒序；`bytes` 相同时按 `moduleCode`、再按 `category` 字典序（结果稳定，不改入参数组）。
4. `A4` `formatBreakdownLine(row)` → `npm_cache / cache · 1.2 GB`（模块 / 类别 · 大小，复用 `formatBytes`）。
5. `A5` `describeDenial(reason)` → 已知 `DenialReason` Debug 串映射为中文，至少覆盖：`L3PersonalDir`→`个人目录（桌面/文档/下载等）`、`L3BrowserProfile`→`浏览器用户数据目录`、`L3BrowserSensitive`→`浏览器敏感文件（书签/密码/历史）`、`L3SystemPath`→`系统关键路径`、`L3ReservedAction`→`动作级禁令`、`PathTraversal`→`路径穿越/非法路径`；`ResolveFailed(...)` 前缀命中 → `真实路径解析失败（<原文>）`；**未知串回退为原始字符串**（不吞信息、不抛错）。
6. `A6` 新建 `app/src/utils/dryRun.test.ts`，Vitest 断言 ≥ 8 条：headline 0 项 / 非 0 项（与 `formatBytes` 口径一致）、`sortBreakdown` 倒序 + 同值稳定 + 不改入参、`formatBreakdownLine` 文案、`describeDenial` 六个已知变体 + 未知串回退。

### B. `app/src/utils/invoke.ts`（仅新增，零破坏）

7. `B1` 导出 `Envelope<T>` 类型，并新增 `apiRaw<T>(cmd, args?)`：直接返回 `invoke` 得到的信封，**`code !== 0` 也不抛错**（含 `data` 非 null 的 2002）。
8. `B2` `api<T>()` 的签名、抛错语义、成功返回值**完全不变**（5 个既有页面的调用点零改动，`String(e)` 文案不发生任何变化）；仅允许把文件内 `interface Envelope` 改为 `export interface Envelope`。
9. `B3` 新建 `app/src/utils/invoke.test.ts`（`vi.mock("@tauri-apps/api/core")`）：① `code=0` 时 `api` 返回 `data`、`apiRaw` 返回整个信封；② `code=2002` 时 `api` 抛 `Error` 且 message 为信封 message、`apiRaw` **不抛**且 `data.blockedByL3` 可读；③ `code=2001` 时 `apiRaw.data === null`。

### C. Items 页面接入 `app/src/pages/Items.tsx`

10. `C1` 顶部操作栏新增「预览可清理项」按钮（置于「清理已选」**左侧**，样式为中性/蓝色系，与既有红色「清理已选」区分）；`selected.size === 0 || cleaning || previewing` 时 `disabled`。
11. `C2` 点击 → `apiRaw<DryRunResp>("cleanup_dry_run", { itemIds: Array.from(selected) })`；请求进行中按钮文案变「预览中…」且禁用，完成后恢复。
12. `C3` `code === 0` → 渲染蓝底预览面板：顶部 `formatDryRunHeadline` 文案；下面按 `sortBreakdown` 逐行 `formatBreakdownLine`；面板底部一行提示 `确认无误后点上方「清理已选」执行`（**不新增执行按钮**）。
13. `C4` `code === 2002` → 红底面板：首行 `N 项命中红线，已阻止清理`（N = `blockedByL3.length`）；下面逐行 `#{itemId} {path} · {describeDenial(denialReason)}`（路径用 monospace，与表格风格一致）。
14. `C5` `code === 2001` → 黄底提示 `已选条目已不在待清理列表（可能已被清理或扫描会话已过期），请点「刷新」`。
15. `C6` 其他 `code` 或 `invoke` reject → 红底 `预览失败：{message}`；按钮恢复可点、页面不白屏（ErrorBoundary 不得被触发）。
16. `C7` 只读保证：预览路径不调用 `cleanup_execute`、不写 DB、不写 `audit_log`、不移动/删除任何文件（评审用 `audit_log` 与文件 mtime 对照验证）。
17. `C8` 结果不陈旧：点「刷新」、勾选集合发生变化、或执行「清理已选」成功后，预览面板必须清空（不得残留旧汇总）。

### D. 卫生与边界

18. `D1` Rust 侧零改动：`engine/**`、`app/src-tauri/**`、`Cargo.*`、`rules/**` 均不在 diff 中；
19. `D2` 「清理已选」按钮的文案、禁用条件、`cleanup_execute` 调用链零行为变化（回归：清理仍走保险箱）；
20. `D3` 其他四页（Dashboard / History / SafeBox / Settings）文件零改动；
21. `D4` 不新增 npm 依赖、不改 `package.json` / `pnpm-lock.yaml`；
22. `D5` 不接 dryRun 作为清理前置校验、不做「全部预览」/进度条/结果导出。

## 允许修改范围

> 严格白名单。范围外文件一律不动（含格式化全仓、顺手改别的页面）。

- `app/src/utils/dryRun.ts`（新建：类型 + 4 个纯函数）
- `app/src/utils/dryRun.test.ts`（新建：Vitest 单测）
- `app/src/utils/invoke.ts`（**仅新增** `apiRaw` + `export` 已有 `Envelope` 接口）
- `app/src/utils/invoke.test.ts`（新建：mock invoke 的信封行为测试）
- `app/src/pages/Items.tsx`（加「预览可清理项」按钮 + 预览面板）
- `tasks/040-dryrun-preview-ui.md`（DocsKeeper 派发时单独提交）
- `docs/PROJECT_STATE.md`（任务收尾单独提交）

不包含（由其他角色持有）：`docs/reviews/task-040-*.md`（ReviewerAgent）、`docs/PRD.md` / `docs/TRD.md` / `docs/ARCHITECTURE.md`。

## Git 分支（feature + bugfix 子分支规则）

- 功能分支：`feature/task-040-dryrun-preview-ui`，**从最新 `main` 拉出**（`git fetch origin` 后以 `origin/main` 为基点）。
- 缺陷修复子分支：`bugfix/task-040-<描述>`，**只允许从本任务 feature 分支拉出、只允许一层**（禁止 `bugfix` 再拉 `bugfix`，禁止从 `main` 直接拉 `bugfix/task-040-*`）；修复完合回 `feature/task-040-dryrun-preview-ui`，由 feature 分支整体评审合并。
- 禁止直接 push `main`；只 push 上述 feature / bugfix 子分支。

## Git 提交要求

1. `feat(frontend): 新增 dryRun 预览纯函数与类型（汇总文案/分类排序/红线原因映射） (#task-040)`
   —— 含 `app/src/utils/dryRun.ts`。
2. `test(frontend): 补 dryRun 纯函数单测 (#task-040)`
   —— 含 `app/src/utils/dryRun.test.ts`。
3. `feat(frontend): invoke 新增 apiRaw 原样返回信封（api 语义不变） (#task-040)`
   —— 含 `app/src/utils/invoke.ts`、`app/src/utils/invoke.test.ts`。
4. `feat(frontend): Items 加「预览可清理项」按钮 + dryRun 汇总/红线明细面板 (#task-040)`
   —— 含 `app/src/pages/Items.tsx`。
5. 自验全绿后，最后一次提交：`docs(state): 任务040完成，dry-run 预览 UI 就绪 (#task-040)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 【待办事项】追加 `- [x] 040：dry-run 预览 UI——「预览可清理项」按钮 + cleanup.dryRun 汇总/红线明细面板`；
   - 【最近一次变更】追加一行（含自验命令与结果、真机与打包冒烟结论）；
   - 【下一步计划】改为「①派发 task-041：dryRun 前置校验接入「清理已选」（预览通过才执行）②首次 v0.1.1 发版验证完整 updater 流程 ③品牌图标设计（用户手动）」；
   - 【Git 提交记录】表追加本任务 commit 摘要；
   - 【变更轨迹】追加 `2026-10-XX  task-040  dry-run 预览 UI（前端预览按钮 + 大小/分类汇总 + L3 被拦明细）`。
6. 提交规范：`<type>(<scope>): <描述> (#task-040)`；**每个功能点完成后立即提交，禁止攒批提交**；每条 commit **显式列出文件路径**，禁止 `git add .` / `git add -A` / `git commit -a`。
7. 提交前自验（必须全绿，命令与退出码一并回填【实现备注】）：
   - `cargo check --workspace --all-targets`（后端无改动，退出码 0）；
   - `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
   - `cargo fmt --all -- --check` 退出码 0；
   - `cargo test -p engine` 退出码 0（已有 **147 条**不得回退）；
   - `pnpm --filter app lint` 退出码 0；
   - `pnpm --filter app test` 退出码 0（既有 12 条断言 + 本任务新增全部通过）；
   - `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

> **本任务命中前端任务判定清单**（新增前端交互控件 + 新增页面上屏元素 + 新增一条 IPC 调用路径 + 涉及错误码分支渲染），因此 **真机冒烟与打包冒烟均为「必做、不可裁剪」**，不得以「已有单测」「环境不便」为由裁剪或用 CI 产物顶替。

| 层级 | 是否必做 | 覆盖内容与判定口径 |
|---|---|---|
| 单元测试（前端 Vitest） | **必做** | `dryRun.test.ts`（汇总文案 / 排序稳定性 / 红线原因映射 / 未知串回退）+ `invoke.test.ts`（`apiRaw` 不抛 2002、`api` 抛错语义不变）全绿 |
| 集成测试（后端回归） | **必做（仅回归，无新增）** | `cargo test -p engine` 147 条不得回退；本任务不改 Rust，故**不新增**后端集成测试——裁剪理由：`handle_dry_run` 的 2001/2002/汇总三条路径已有 task-004/010/014 集成测试覆盖，本任务只在消费侧 |
| 端到端（真机 Tauri） | **必做、不可裁剪** | `pnpm tauri dev` + WebView2 CDP（9222）真实点击：造数据 → 勾选 → 预览 → 面板数值与 DB 一致 → 2002 分支 → 刷新后面板清空 |
| 真机（DevTools Console） | **必做、不可裁剪** | 本任务页面（清理 tab）全流程 **0 条 console.error / 0 条 exception**，且四页回归无白屏；证据为 CDP 采集日志 + 截图 |
| 打包安装冒烟 | **必做、不可裁剪** | `pnpm tauri build` 出 NSIS `.exe` → 静默安装到干净目录（参照 debt-cleanup-004：perMachine `/S` 安装 + 清空 DB 首启）→ 预览按钮在安装版里可点、面板正常渲染、Console 零红错 |
| CI（GitHub Actions） | **必做** | `windows-latest` 单 job 五道闸（fmt / clippy / cargo test / pnpm lint+test / pnpm build + tauri build）全绿；合并前必须有该分支的真实 run 结论（本仓库 `ci.yml` 只在 `push: main` 与 PR 触发，故需开 PR 取分支 CI，不接受口头「本地绿」） |

## 测试证据清单

证据存放：截图 / CDP 日志 / 原始输出存于**仓库外**评审工作目录（沿用 task-039 做法，不向仓库引入二进制产物），路径与文件名写进任务文件【实现备注】与评审报告；仓库内只留命令与文本输出摘要。

| 验收标准 | 证据形式（命令 + 输出 / 截图 / 日志 / 报告路径） |
|---|---|
| A1–A5 纯函数与类型 | `pnpm --filter app test -- dryRun` 输出（用例名 + passed 计数）+ `app/src/utils/dryRun.test.ts` 用例清单 |
| A6 / B3 单元测试 | `pnpm --filter app test` 完整输出（新增断言 ≥ 11 条，总断言数 ≥ 23，无 skipped） |
| B1–B2 `apiRaw` 非破坏性 | `invoke.test.ts` 输出 + `git diff main -- app/src/utils/invoke.ts`（仅新增，`api` 函数体无改动 hunk） |
| C1–C2 预览按钮与调用 | 真机截图 `01-items-preview-button-disabled.png`（未选态禁用）/ `02-items-previewing.png`（「预览中…」）+ CDP Runtime 域采集到的 `cleanup_dry_run` 调用与 `code=0` 回包 |
| C3 汇总面板正确 | 真机截图 `03-items-preview-panel.png`；对照 `sqlite3 <db> "select count(*), sum(size_bytes) from cleanup_item"` 与面板「N 项 / 共 X」一致；分类行与小计对比 `group by module_code, category` 查询 |
| C4 红线分支 | 真机截图 `04-items-preview-blocked.png`（红条 `N 项命中红线，已阻止清理` + 逐行中文原因）；造一条指向 `C:\Windows\System32\` 的 `cleanup_item` 行复现 2002，**只点预览，绝不点清理** |
| C5 空清单分支 | 真机截图或 CDP 采集：对已被清理的 id 调预览 → 黄条 2001 文案（可用 CDP `Runtime.evaluate` 复现并截图） |
| C6 失败态 | CDP 采集：临时改 `rules/` 使 `load_external_deny` 失败（`code=5001`）→ 红条 `预览失败：…`，按钮恢复、无白屏、ErrorBoundary 未触发 |
| C7 只读保证 | 预览前后对照：`select count(*) from audit_log` 不变、被预览文件 mtime 不变、`safe_box` 行数不变 |
| C8 结果不陈旧 | 截图 `05-items-preview-cleared-after-refresh.png`（点刷新后预览面板消失） |
| D1–D4 白名单与零依赖 | `git diff --stat main...HEAD`（只含 5 个白名单文件）+ `git status` 干净；`pnpm-lock.yaml`、`Cargo.lock` 无 diff |
| 真机 + Console | CDP 采集日志（Runtime/Log/Page 域）：清理 tab 全流程 0 error / 0 exception；四页回归复扫截图 |
| 打包安装冒烟 | `pnpm tauri build` 产物路径 + `.exe` 体积；安装位置与首启截图；安装版预览面板截图 + 安装版 Console 零红错日志 |
| CI | GitHub Actions run URL + run id + 结论（success），并附合并后 `main` 的 run 结论 |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码/测试提交 + 1 个 `docs(state)` 提交，全部 push 到 `feature/task-040-dryrun-preview-ui`（每条 commit 显式列文件、带 `(#task-040)`）；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（与白名单逐条对照）；
   - 提交前自验 7 条命令的原始输出摘要与退出码，以及 `pnpm --filter app test` 新增断言计数；
   - 真机冒烟结论（截图文件名 + CDP 采集结论：`cleanup_dry_run` 回包 code、面板数值与 DB 对照结果、Console 红错数）；
   - 打包安装冒烟结论（installer 路径 / 体积 / 安装目录 / 安装版 Console 红错数）；
   - 与任务文档不符的偏差（如有，逐条说明理由）；
   - 遗留问题（「dryRun 前置校验接入清理已选」留 task-041、组件级测试（RTL）未写等）。
3. `docs/PROJECT_STATE.md` 已按 §Git 提交要求第 5 条单独提交更新；
4. 本任务文档本身由 DocsKeeper 在派发时单独提交一次：`docs(task): 新增任务040-dry-run 预览 UI (#task-040)`，该提交**只含 `tasks/040-dryrun-preview-ui.md` 一个文件**，落在 `feature/task-040-dryrun-preview-ui` 分支最前端；
5. ReviewerAgent 评审报告落到 `docs/reviews/task-040-dryrun-preview-ui-review.md`，结论为 Approve 且 CI 绿后，由 DocsKeeper 合并 `main`、删除 feature 分支，并回写本文件状态字段为 `done`。
