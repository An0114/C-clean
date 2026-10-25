# 任务编号：051 保险箱增强——时间区间筛选 / 多选 session / 审计历史联表

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevEngineRust（后端）+ DevFrontendReact（前端）——同一任务内串行完成，一轮对话交付
> 来源：`docs/PROJECT_STATE.md`「下一步计划 ①」+「已知问题 `[B 低] task-018 list 边界`」——`list_paged` 已支持单 session + 分页（task-045/046），仍缺**时间区间筛选**、**多 session 组合**、**按条目的审计历史联表**。`SafeBox.tsx` 目前只有一个单选 `<select>`，且表格里看不到「这条来自哪次清理」。
> 上游依赖：task-018（list）、task-045（分页）、task-046（session 过滤 + `list_sessions`）、task-024（`audit_log/query.rs` 的 `AuditLogEntry`）、task-037/038/039（purge / 批量操作，会写 audit_log）。
> 里程碑：M2 已关闭；本任务是 **task-051**（编号由原 050 顺延，050 已让位给打包资源路径缺陷）。

## 状态字段

pending

## 任务目标

把保险箱页从「只能翻页看全量」升级为「能按时间窗 + 多批清理记录交叉筛，并能查单个条目的来龙去脉」。四个原子单元，按 A→B→C→D 顺序做、按顺序提交：

1. **A｜时间区间筛选（后端）**：`list_paged` 增加 `from_ts` / `to_ts`（Unix **秒**，闭区间），与既有分页 / session 过滤可任意组合。**同时把现有「`match session_id` 两套硬编码 SQL」重构成动态 WHERE**——否则加两个时间参数会炸成 8 个分支。
2. **B｜多选 session（后端）**：`session_id: Option<i64>` 升级为 `session_ids: Vec<i64>`（空 = 全部），SQL 用 `IN (...)` 参数化。
3. **C｜审计历史联表（后端）**：新增「按 safe_box 条目查该原路径的 audit_log 历史」只读能力，复用 `audit_log/query.rs` 的 `AuditLogEntry`，不新建同义结构。
4. **D｜前端闭环**：时间预设按钮组（全部 / 24 小时 / 3 天 / 7 天）+ 原生多选 session + 表格新增「来源 session」列 + 行内「历史」展开。

**本任务不做**（划清边界）：

- **不做**自定义日期区间（`datetime-local`）——本地时区 ↔ UTC 秒换算会引入时区 bug，单独任务做更稳；本任务只做「距现在 N 小时/天」的预设，天然无时区问题。
- **不做**跨页多选 / 全选所有筛选结果（只保留现有「当前页全选」语义）；
- **不做**审计历史的写入侧改造（`quarantine` 目前**不写** audit_log，本任务只读不补写）；
- **不改** `safe_box` 表结构、不加列、不做迁移（`source_task_id` 已有，只是没暴露给前端）；
- **不改** `purge` / `restore` / `restore_many` / `purge_many` / `enforce_quota` 任何行为；
- **不改** `executor.rs` / `whitelist.rs` / `delete.rs` / `locking.rs` / `paths.rs` / `tauri.conf.json`；
- **不做**导出 CSV、不做图表、不做 `RmRestart`、不做图标。

## 前置条件

1. `main` 已含 task-001~050 全部产出（task-050 合并提交 `cadb286`，状态白板 `2d59b5d`），工作区干净；
2. 分支 `feature/task-051-safebox-filter-history` **已由派发方从 `main` 拉出并推送**。执行方直接 `git fetch origin && git checkout feature/task-051-safebox-filter-history` 起手，**不要**重建同名分支；若本地已有同名旧分支，先确认 base 含任务书 051 的提交，否则删除重建；
3. 关键锚点（**先读真实代码，以代码为准，不以本文档伪代码为准**）：
   - `engine/src/execute/safe_box.rs:402` `list_paged(conn, page, page_size, session_id)`——两套硬编码 SQL + 两套 params（`match session_id`）；
   - `engine/src/execute/safe_box.rs:321` `SafeBoxEntry { id, original_path, size_bytes, quarantined_at }`（camelCase 序列化）；
   - `engine/src/execute/safe_box.rs:518` `SessionSummary` + `list_sessions`；
   - `engine/src/audit_log/query.rs` `AuditLogEntry { id, event_type, path, detail, created_at, result }`（对外 camelCase：`eventType/path/detail/createdAt/result`）；
   - `app/src-tauri/src/commands.rs:267` `safe_box_list`、`:291` `safe_box_list_sessions`；
   - `app/src-tauri/src/lib.rs:41-45` 命令注册表；
   - `app/src/pages/SafeBox.tsx`（490 行，`load()` / `onChangeSession` / 分页栏 / 表格）；
   - `app/src/pages/Items.test.tsx`（jsdom 组件用例的既有写法，含 mock `api`）。
4. **时间单位是 Unix 秒**（不是毫秒）：`safe_box.quarantined_at` 来自 `SystemTime::as_secs()`，`audit_log.ts` 同源，`expires_at = quarantined_at + 7*86400`。`v1__init.sql` 注释写「Unix 毫秒」属历史笔误，**本任务不改 schema 注释、不改历史数据**，一律按秒处理。
5. 基线（本任务不得回退）：`cargo test --workspace` **197 passed / 8 ignored / 0 failed**（engine 188+8，app `paths::` 9）；vitest **34 passed（6 files）**；`pnpm build` **176.39 kB │ gzip 56.25 kB**；`node --test scripts/tauri-resources.test.mjs` **6/6**。

## 需求 & 验收标准

> 执行 Agent 自验时逐条勾选并贴证据。A/B/C=后端，D=前端，E=卫生，F=端到端。

### A. 时间区间筛选（后端）

1. `list_paged` 签名改为接收过滤结构（**推荐**下列形态，字段名可微调，语义固定）：
   ```rust
   #[derive(Debug, Clone, Default)]
   pub struct SafeBoxFilter {
       /// 空 = 不限 session；非空 = IN (...) 组合过滤（内部先去重）。
       pub session_ids: Vec<i64>,
       /// 含下界；Unix 秒；<= 0 视为未设置。
       pub from_ts: Option<i64>,
       /// 含上界；Unix 秒；<= 0 视为未设置。
       pub to_ts: Option<i64>,
   }

   pub fn list_paged(
       conn: &Connection,
       page: u32,
       page_size: u32,
       filter: &SafeBoxFilter,
   ) -> Result<SafeBoxPage, String>
   ```
2. **动态 WHERE**：`status='in_box'` 固定打底，其余条件按需追加，值一律走占位符绑定（`rusqlite::params_from_iter` / `Vec<Box<dyn ToSql>>` 均可），**禁止把任何用户输入拼进 SQL 字符串**。
3. `items` / `COUNT(*)` / `SUM(size_bytes)` 三条查询必须使用**同一套** WHERE 与参数——`total`、`totalBytes` 与筛选条件一致（这是本任务最容易漏的点，见 A7 用例）。
4. 时间语义：`quarantined_at >= from_ts` 且 `quarantined_at <= to_ts`（**闭区间**）；只给 `from` 或只给 `to` 也要生效。
5. 边界：`from_ts > to_ts` 返回**空页**（`items=[]`、`total=0`、`totalBytes=0`、`page`/`pageSize` 原样回显），**不返回 Err**；`from_ts`/`to_ts` `<= 0` 视为未设置。
6. 分页 clamp 行为不变（`page<1` 按 1、`page_size` clamp 到 [1,200]），`ORDER BY id DESC` 不变。
7. 新增集成测试 `engine/tests/safe_box_time_range.rs`（≥5 条，自行造 `quarantined_at` 不同的行）：
   - 只给 `from`：只返回边界内条目，且 `total` 与 `items.len()` 一致；
   - 只给 `to`：同上；
   - 闭区间：`from == quarantined_at` 与 `to == quarantined_at` 的条目**都被包含**；
   - `from > to` → 空页且 `total=0`、`totalBytes=0`（不 Err）；
   - 时间区间 + 分页组合：第 2 页结果正确、`total` 是全区间总数（不是全表总数）。

### B. 多选 session（后端）

8. `session_ids` 为空 → 不过滤；非空 → `AND source_task_id IN (?n...)`；内部 `dedup`（重复 id 不报错、不影响结果）。
9. 与时间区间**可叠加**（A+B 同时生效，`total` 仍与条件一致）。
10. 新增集成测试（≥4 条，可并入 `safe_box_time_range.rs` 或新建 `safe_box_multi_session.rs`）：
    - 选 2 个 session → 只返回这 2 个的条目，`total` 正确；
    - 选 1 个不存在的 session → 空页；
    - 空 vec → 等价于全部；
    - 多 session + 时间区间 + 分页三者叠加 → 结果正确。
11. 既有用例同步改签名，**断言不得放宽、不得删用例、不得 skip**：`engine/tests/safe_box_pagination.rs`（5 条）、`engine/tests/safe_box_session_filter.rs`（4 条，含 `list_sessions` 那条保持不变）。

### C. 审计历史联表（后端）

12. `SafeBoxEntry` 增加字段：
    ```rust
    /// 来源清理批次（safe_box.source_task_id），NULL 表示历史遗留/未关联。
    pub source_task_id: Option<i64>,
    ```
    序列化后前端字段为 `sourceTaskId`；`list_safe_box`（旧全量接口）与 `list_paged` 都要填。
13. `engine/src/audit_log/query.rs` 新增：
    ```rust
    /// 按 target（路径）查该路径的审计历史，ts DESC, id DESC，最多 limit 条。
    /// 路径比较用 COLLATE NOCASE（Windows 路径大小写不敏感；本应用写入走 normalize，
    /// 但历史数据/手工写入可能大小写不一致）。
    pub fn list_by_target(conn: &Connection, target: &str, limit: i64) -> Result<Vec<AuditLogEntry>, String>
    ```
    复用既有 `AuditLogEntry`，**不新建同义结构**；`limit` 由调用方传入，内部 clamp 到 [1, 100]。
14. `engine/src/execute/safe_box.rs` 新增：
    ```rust
    /// 按 safe_box id 查该条目原路径的审计历史（只读）。
    /// 条目不存在 → Err("safe_box item not found")。
    pub fn item_history(conn: &Connection, safe_box_id: i64) -> Result<Vec<AuditLogEntry>, String>
    ```
    实现：先 `SELECT original_path FROM safe_box WHERE id=?`，再调 `list_by_target(conn, &path, 20)`。
15. 新增集成测试 `engine/tests/safe_box_item_history.rs`（≥4 条）：
    - 同一路径有 `quarantine`/`permanent_delete`/`restore` 多条 → 按 `ts DESC` 返回、条数正确；
    - 路径大小写不同（`C:\Temp\A.txt` vs `c:\temp\a.txt`）仍能命中（NOCASE 生效）；
    - 无历史 → 返回空 vec（不 Err）；
    - `safe_box_id` 不存在 → `Err`，消息含 `not found`。

### D. 前端闭环（`SafeBox.tsx`）

16. **时间预设**：筛选栏加 4 个互斥按钮/下拉：`全部 / 24 小时 / 3 天 / 7 天`。切换时 `fromTs = Math.floor(Date.now()/1000) - N`（`全部` → 不传），`page` 重置 1、清空 `selected`、重拉列表。
17. **多选 session**：原单选 `<select>` 换成 `<select multiple>`（`size={4}`，Windows 原生多选），列出 `session #N (M 条)`；旁边加「全选 / 清空」两个小按钮。选中集合为空 → 不传 `sessionIds`（等价全部）。切换后同样 `page=1` + 清空 `selected` + 重拉。
18. **来源 session 列**：表格新增一列「来源」，渲染 `sourceTaskId == null ? "—" : "#" + sourceTaskId`，宽度 ≤ 90px。
19. **历史展开**：每行「历史」按钮 → 展开一行（`<tr><td colSpan={7}>`），调 `safe_box_item_history({ safeBoxId })` 懒加载并展示 `时间 / 动作 / 结果 / 详情`：
    - 动作中文化映射：`permanent_delete → 永久删除`、`restore → 恢复`、`purge → 清除`、`quarantine → 移入保险箱`、`scan → 扫描`，未知名原样显示；
    - 加载中显示「加载中…」；空结果显示「该路径暂无审计记录」；失败显示红字原因；
    - 同一时刻只展开一行，再次点击同一行收起；展开状态在翻页/筛选变化时清空。
20. **不破坏既有交互**：批量恢复 / 批量删除 / 单条恢复 / 永久删除 / 三态互斥（`restoringId`/`purgingId`/`batchPurging`/`batchRestoring`）行为不变；「刷新」按钮沿用当前筛选条件重拉（**不得**悄悄重置筛选）。
21. 新增 `app/src/pages/SafeBox.test.tsx`（jsdom，mock `../utils/invoke` 的 `api`，照 `Items.test.tsx` 的写法）≥6 条断言：
    - 首屏调 `safe_box_list` 且 `sessionIds` 为空 / `fromTs` 未传；
    - 点「24 小时」→ 再次调用带 `fromTs`（数值 > 0）且 `page=1`；
    - 多选 2 个 session → 调用带 `sessionIds: [a, b]`；
    - 表格渲染「来源 #N」列；
    - 点「历史」→ 调 `safe_box_item_history` 并渲染返回的动作（中文化）；
    - 无历史时渲染「该路径暂无审计记录」。

### E. 卫生

22. 不新增 npm / cargo 依赖；不动 `Cargo.lock` / `pnpm-lock.yaml`；
23. 允许改动文件见下表，其余一律不动；
24. 门禁全绿：`cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test --workspace`（**≥197 passed + 新增**，engine 既有 188 条不回退）/ `pnpm --filter app lint` / `pnpm --filter app test`（**≥34 + 新增**）/ `pnpm --filter app build`（体积与基线同量级，±5 kB 内）/ `node --test scripts/tauri-resources.test.mjs`（6/6，防资源映射回归）。

### F. 端到端 / 冒烟

25. **dev 真机**：`pnpm tauri dev` → 保险箱页：切时间预设、多选 session、点历史展开，各交互有正确反馈（无 React 报错）。
26. **安装包冒烟**：`pnpm --filter app tauri build`（本地需临时签名密钥，见下）→ NSIS `/S /D=%TEMP%\cclean-smoke-051` 安装 → 启动安装版，保险箱页可正常渲染、筛选栏可见。
27. 造数据要求：用「扫描 → 清理（走保险箱）」产生真实条目；若本机 GUI 阻塞导致无法造数据，允许用 `sqlite3` 直接向 `%APPDATA%\com.and0114.cclean\data\cdiskcleaner.db` 的 `safe_box` + `audit_log` 插测试行（**必须写明插入的 SQL 与时间戳，便于复现**），并在实现备注标注「数据为构造」。
28. **GUI 证据缺口**：本机历史多轮均出现「WebView2 渲染进程 3s 有 4 个子进程 → 6s 归零、CDP 9222 不监听」。若本轮仍如此：必须**实测记录现象**（窗口标题存活时长 / Crashpad `.dmp` 时间戳 / CDP 探测结果），按第三档登记，用 D21 的 jsdom 组件用例 + 后端集成用例 + 安装包文件系统级证据补偿，**禁止伪造截图或日志**。

本地打包密钥（`createUpdaterArtifacts: true` 强制要私钥，缺了 build 退出码 1）：
```bash
pnpm --filter app exec tauri signer generate -w "$TEMP/cclean-smoke-051.key" -p "" --ci --force
export TAURI_SIGNING_PRIVATE_KEY="$TEMP/cclean-smoke-051.key"
```

## 允许修改范围

| 文件 | 允许的改动 |
|---|---|
| `engine/src/execute/safe_box.rs` | `SafeBoxFilter` + `list_paged` 动态 WHERE 重构 + `SafeBoxEntry.source_task_id` + `item_history` |
| `engine/src/audit_log/query.rs` | 仅新增 `list_by_target`（不动 `list_audit_log` / `AuditLogFilter`） |
| `engine/tests/safe_box_time_range.rs` | 新增（A7） |
| `engine/tests/safe_box_multi_session.rs` | 新增（B10，也可并入上一文件，但须在备注说明） |
| `engine/tests/safe_box_item_history.rs` | 新增（C15） |
| `engine/tests/safe_box_pagination.rs` | 仅同步 `list_paged` 调用签名，断言不放宽 |
| `engine/tests/safe_box_session_filter.rs` | 仅同步签名 + 把单 session 用例改写为 `vec![id]`，断言不放宽 |
| `app/src-tauri/src/commands.rs` | `safe_box_list` 参数升级 + 新增 `safe_box_item_history` |
| `app/src-tauri/src/lib.rs` | 仅新增一行命令注册 |
| `app/src/pages/SafeBox.tsx` | 时间预设 / 多选 session / 来源列 / 历史展开 |
| `app/src/pages/SafeBox.test.tsx` | 新增（D21） |
| `docs/PROJECT_STATE.md` | 收尾单独提交 |
| `tasks/051-safebox-filter-history.md` | 追加 `## 实现备注` |

**禁止修改**：`app/src/App.tsx`、`app/src/pages/Items.tsx`、`Dashboard.tsx`、`History.tsx`、`Settings.tsx`、`app/src/utils/**`、`app/src-tauri/src/paths.rs`、`progress.rs`、`app/src-tauri/tauri.conf.json`、`app/src-tauri/Cargo.toml`、`engine/src/execute/{executor,delete,whitelist,locking,error_kind}.rs`、`engine/src/db/**`（含迁移 SQL）、`engine/src/rules/**`、`engine/src/ipc/**`、`elevation/**`、`rules/**`、`scripts/**`、`.github/workflows/**`、`capabilities/default.json`、`app/package.json`、`Cargo.lock`、`pnpm-lock.yaml`。

> 若发现必须改 `commands.rs` 之外的命令层或 `lib.rs` 之外的地方才能跑通，先停下在实现备注里说明，不要擅自扩范围。

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-051-safebox-filter-history`，**已由派发方从 `main` 拉出并推送**，执行方直接 checkout。
- 缺陷修复：只允许**一层**子分支 `bugfix/task-051-<描述>`（从该 feature 分支拉出），修完 `--no-ff` 合回 feature 并删除；**禁止再嵌套第二层**。
- 完成后**不合 main**：push feature 分支 → 交测试 Agent（提示词 9）→ 评审（提示词 10）；可开 **draft PR** 触发 CI，PR 不作为合并入口。

## Git 提交要求

格式 `<type>(<scope>): <描述> (#task-051)`，**每个功能点完成即提交，禁止攒批**：

1. `feat(engine): list_paged 改为动态 WHERE，支持时间区间与多 session 组合筛选 (#task-051)`
2. `feat(engine): 保险箱条目暴露 source_task_id 并新增按路径的审计历史查询 (#task-051)`
3. `feat(ipc): safe_box_list 参数升级并注册 safe_box_item_history 命令 (#task-051)`
4. `test(engine): 覆盖时间区间边界与多 session 叠加筛选 (#task-051)`
5. `feat(ui): 保险箱页支持时间预设/多选 session/来源列/历史展开 (#task-051)`
6. `test(ui): 新增 SafeBox 组件用例覆盖筛选与历史展开 (#task-051)`
7. `docs(state): 任务051 保险箱筛选与审计联表完成 (#task-051)`（**单独只动 `docs/PROJECT_STATE.md`**）

补充约束：

- 每个提交前门禁全绿；**不得**把 `PROJECT_STATE.md` 混进业务提交；
- 不重写已推送历史（禁 rebase / force push）；
- 敏感文件（`*.key`、`latest.json`、`data/*.db`）不入库；
- 既有 9 条测试的签名同步改动，必须单独说明「只改签名不改断言」，可并入对应 feat 提交（不必单独成提交），但要在实现备注列出改前/改后签名。

## 测试要求（层级 + 真机/打包冒烟是否必做）

| 层级 | 是否必做 | 覆盖内容 |
|---|---|---|
| 单元测试 | **必做** | `SafeBoxFilter` 去重/边界归一（`<=0` 视为未设置、`from>to` 空页）；动作中文化映射（若抽成纯函数则加单测） |
| 集成测试 | **必做** | A7 时间区间 ≥5、B10 多 session ≥4、C15 历史 ≥4；既有 pagination 5 + session_filter 4 全绿不回退 |
| 前端组件测试 | **必做** | D21 `SafeBox.test.tsx` ≥6 条；既有 34 条不回退 |
| 端到端 | **必做** | F25 dev 真机交互（时间预设/多选/历史展开） |
| 真机冒烟 | **必做、不可裁剪** | 安装版保险箱页渲染 + 筛选栏可用。命中前端任务判定清单，**不得以「jsdom 绿了」代替**；环境阻塞按 F28 登记 + 补偿，禁止伪造 |
| 打包安装冒烟 | **必做、不可裁剪** | `tauri build` 出包 + NSIS 干净目录安装 + 启动；资源布局（`rules/`、`scripts/`、无 `_up_`）顺带复核，防 task-050 回归 |
| CI | **必做** | 新 run id + `conclusion=success`（含 Rust 测试 / 前端 lint+test+build / Tauri build / 资源映射契约） |

裁剪说明：**无裁剪项**。本任务动了查询语义（`total`/`totalBytes` 与筛选一致性）与前端主交互，任一层缺失都可能让「筛出来是空的」这类问题漏到发版。唯一可降级的是「GUI 点击取证」，降级须留实测现象 + D21/F26 补偿证据。

## 测试证据清单

| 验收标准 | 证据形式（命令 + 输出 / 截图 / 日志 / 报告路径） |
|---|---|
| A1-A2 动态 WHERE | `git diff` 中 `list_paged` 片段 + `grep -n "params_from_iter\|to_sql\|IN (" engine/src/execute/safe_box.rs` |
| A3 total 一致性 | `cargo test -p engine --test safe_box_time_range` 输出（含 total 断言用例名） |
| A4-A5 闭区间/边界 | 同上，用例名 + 断言值（贴 `from==ts`、`to==ts`、`from>to` 三条输出） |
| A7/B10/C15 新增用例 | 三条 `cargo test` 命令的完整输出（条数 + 用例名） |
| B11 既有用例不回退 | `cargo test -p engine --test safe_box_pagination --test safe_box_session_filter` 输出（5 + 4） |
| C13 `list_by_target` | `grep -n "fn list_by_target" -A 8 engine/src/audit_log/query.rs` + NOCASE 用例输出 |
| C14 `item_history` | `grep -n "fn item_history" -A 10 engine/src/execute/safe_box.rs` |
| D16-D19 前端 | `SafeBox.tsx` diff 片段（时间预设/多选/来源列/历史展开各一段） |
| D21 组件用例 | `pnpm --filter app test` 输出（含 `SafeBox.test.tsx` 用例名，条数 ≥40） |
| E24 门禁 | 七条命令退出码 + `cargo test` 条数 + `pnpm build` 体积 + 契约测试 6/6 |
| F25 dev 真机 | 交互截图或（GUI 阻塞时）实测现象 + jsdom 用例对照 |
| F26 安装包 | `pnpm tauri build` 退出码 + `Get-ChildItem %TEMP%\cclean-smoke-051` 布局清单 |
| F27 造数据 | 实际执行的 SQL（含 `quarantined_at` 值）或「真实扫描+清理」步骤截图 |
| F28 GUI 缺口 | 窗口存活时长 / Crashpad `.dmp` 时间戳 / `curl localhost:9222/json` 结果 + 补偿说明 |
| CI | run URL + id + `conclusion=success` + 关键步骤名 |

## 输出要求

1. 按上表提交并 push 到 `feature/task-051-safebox-filter-history`（**不合 main**）；
2. 任务文件末尾追加 `## 实现备注`，至少包含：实际改动文件对账（对照允许表）、`list_paged` 重构前后的 SQL 与参数构造方式、既有 9 条测试的签名改动清单（证明断言未放宽）、新增用例条数与名称、`total/totalBytes` 一致性如何自证、门禁退出码、安装包布局、GUI 取证结论（取得/第三档）、CI run id、与任务书不符之处；
3. 明确回答三句话：①「时间区间 + 多 session + 分页三者叠加时，`total` 与列表条数是否一致」；②「同路径历史能否跨大小写命中」；③「筛选变化后是否清空选中、重置页码」——各附证据编号；
4. 更新 `docs/PROJECT_STATE.md`：勾选待办 051、关闭 `[B 低] task-018 list 边界` 里「时间范围筛选 / 多选 session / audit_log 联表」三项、把「下一步计划」推进到 ③（关闭体验文案小改），单独提交 `docs(state)`。
