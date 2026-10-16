# 任务编号：046 safe_box list session 过滤——后端可选 session_id 参数 + 前端 session 下拉

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：`docs/PROJECT_STATE.md` 已知问题（task-018 段）——「无 session 过滤（先返回全部）」。task-045 已做分页，本任务补「按 session 筛选」：用户想看「最近一次清理进保险箱的文件」时，不用翻全表。
> 上游依赖：task-016（safe_box 表含 session_id 列）、task-045（list_paged + SafeBoxPage）、task-030/038/039/045（SafeBox.tsx 表格 + checkbox + 分页栏）。
> **克制范围**：只加 session 等值过滤（下拉选一个 session），**不做**时间范围筛选、**不做**多选 session、**不做**与 audit_log 联表。

## 状态字段

pending

## 任务目标

当前 `safe_box_list(page, page_size)` 不区分 session，所有进保险箱的文件混在一起。用户在调试某次清理误删时，想只看「那次清理进了哪些文件」。

本任务补：

1. **后端**：
   - `list_paged` 入参加 `session_id: Option<i64>`；
   - `None` → 全表（向后兼容）；
   - `Some(id)` → SQL 加 `WHERE session_id = ?`，total 也跟着过滤；
   - 新增 `list_sessions() -> Vec<SessionSummary>`：
     ```rust
     pub struct SessionSummary {
         pub session_id: i64,
         pub item_count: i64,      // 该 session 进了多少条
         pub created_ts: i64,       // 该 session 最早一条的 ts
     }
     ```
     - SQL：`SELECT session_id, COUNT(*) AS item_count, MIN(ts) AS created_ts FROM safe_box GROUP BY session_id ORDER BY session_id DESC`；
     - 供前端下拉选 session。
2. **前端** SafeBox.tsx 顶部加分页栏左边：
   - 下拉 `<select>`：第一项「全部 session」，后续每一项「session #N（M 条 · YYYY-MM-DD）」；
   - 默认选「全部 session」；
   - 切换下拉 → page 重置为 1 → 重新拉 list；
   - selected Set 清空。

**本任务不做**：
- 不做时间范围筛选；
- 不做多选 session；
- 不做与 audit_log 联表（不显示 session 对应扫描来源路径）；
- 不改 purge/restore 逻辑。

## 前置条件

1. `main` 分支已含 task-001~045 全部产出；
2. 关键锚点：
   - `engine/src/execute/safe_box.rs`（task-045 `list_paged` + `SafeBoxPage`）；
   - `app/src-tauri/src/commands.rs`（`safe_box_list` 已注册）；
   - `app/src/pages/SafeBox.tsx`（task-045 分页栏）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. 后端：list_paged 加 session_id 可选参数

1. 在 `engine/src/execute/safe_box.rs` 改：
   ```rust
   pub fn list_paged(
       conn: &rusqlite::Connection,
       page: u32,
       page_size: u32,
       session_id: Option<i64>,
   ) -> Result<SafeBoxPage, String>;
   ```
2. SQL 动态拼接：
   - 基础：`SELECT * FROM safe_box`；
   - `WHERE session_id = ?` 当 session_id 是 Some；
   - `ORDER BY id DESC LIMIT ? OFFSET ?`；
   - COUNT 同步加 WHERE。
3. 新增 `list_sessions(conn) -> Result<Vec<SessionSummary>, String>`。
4. Tauri 命令：
   - `safe_box_list` 入参加 `session_id: Option<i64>`（前端 null = None）；
   - 新增 `safe_box_list_sessions` 返回 `Vec<SessionSummary>`。

### B. 后端单测

5. 在 `engine/tests/safe_box_session_filter.rs` 覆盖：
   - `list_paged_no_session_returns_all`：造 5 行（session 1 有 3 条、session 2 有 2 条），session_id=None → items=5, total=5；
   - `list_paged_filter_by_session`：session_id=Some(1) → items=3, total=3；
   - `list_paged_filter_no_match`：session_id=Some(999) → items=0, total=0；
   - `list_sessions_groups_by_session`：返回 2 项，session 1 item_count=3、session 2 item_count=2，按 session_id DESC 排序。

### C. 前端：SafeBox session 下拉

6. `app/src/pages/SafeBox.tsx`：
   - state 加 `sessionFilter: number | null`（null = 全部）、`sessions: SessionSummary[]`；
   - 首次加载调 `safe_box_list_sessions()` 填下拉；
   - 分页栏左边加 `<select>`：
     ```html
     <option value="">全部 session</option>
     <option value="1">session #1 (3 条)</option>
     <option value="2">session #2 (2 条)</option>
     ```
   - 切换下拉 → setSessionFilter → setPage(1) → 重新拉 `safe_box_list({ page, pageSize, sessionId })`；
   - selected Set 清空。

### D. 卫生

7. 不改 purge/restore 逻辑；
8. 不改分页栏布局（只在左边加 select，不上下移位）；
9. session 列在 safe_box 表已存在（task-016 schema），不加列。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/safe_box.rs`（list_paged 加 session_id + list_sessions）
- `engine/tests/safe_box_session_filter.rs`（新建）
- `app/src-tauri/src/commands.rs`（safe_box_list 入参改 + 新增 safe_box_list_sessions）
- `app/src-tauri/src/lib.rs`（注册新命令）
- `app/src/pages/SafeBox.tsx`（加 session 下拉）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-046-safebox-session-filter`，从最新 `main` 拉出。
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-046-<描述>`，**只允许一层**；bugfix 修复后直接合并回 feature，不进 main。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(execute): list_paged 加可选 session_id 过滤 + 新增 list_sessions (#task-046)`
   —— 含 `engine/src/execute/safe_box.rs`。
2. `feat(tauri): safe_box_list 入参加 session_id，新增 safe_box_list_sessions 命令 (#task-046)`
   —— 含 `app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`。
3. `test(execute): 补 session 过滤集成测试（全部/按 session/无匹配/list_sessions 分组） (#task-046)`
   —— 含 `engine/tests/safe_box_session_filter.rs`。
4. `feat(frontend): SafeBox 加分页栏左侧 session 下拉（全部/session #N） (#task-046)`
   —— 含 `app/src/pages/SafeBox.tsx`。
5. 自验全绿后，最后一次提交：`docs(state): 任务046完成，safe_box session 过滤就绪 (#task-046)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 046: ...`；
   - 「最近一次变更」追加一行；
   - 「已知问题」段 task-018 那条把「无 session 过滤」划掉（task-018 两项已知问题至此全部闭环）；
   - 「下一步计划」改为「派发 task-047：v0.1.1 真机升级验证收尾——用户在 v0.1.0 机器上启动应用观察 updater banner，验证后正式关闭 M2」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-16  task-046  safe_box list session 过滤（可选 session_id 参数 + list_sessions + 前端下拉）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 169+ 条 + 本任务 4 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：`safe_box_session_filter.rs` 4 条覆盖全部/按 session/无匹配/list_sessions 分组。
- **集成测试（必做）**：`cargo test -p engine` 全绿，确保现有不回归。
- **端到端（不强制）**：本任务只加查询过滤，不改写逻辑。
- **真机冒烟（必做、不可裁剪）**：
  - 造 5 行 safe_box 数据（2 个 session）；
  - SafeBox 下拉显示「全部 session / session #1 (3 条) / session #2 (2 条)」；
  - 切到 session #1 → 表格只显示 3 条、分页 total=3；
  - 切回「全部 session」→ 显示 5 条；
  - CDP 抓 console 0 红错。
- **打包安装冒烟（必做、不可裁剪）**：
  - NSIS 干净目录安装 → SafeBox 空表下拉只显示「全部 session」，选中后表格空。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（SafeBox.tsx 新增下拉 + 真机可观察行为），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| A1-A4 list_paged session_id + list_sessions | `cargo test -p engine` 输出（含 safe_box_session_filter 用例 + passed） |
| B1-B4 后端单测 | `cargo test safe_box_session_filter -- --nocapture` 完整输出 |
| C6 前端 session 下拉 | CDP 截图：下拉选项 / 切 session 后表格行数变化 |
| D7-D9 不回归 | `cargo test -p engine` 全绿 + `pnpm test` 全绿 |
| 真机冒烟 | CDP `http://localhost:9222/json` 抓 page + consoleAPICalled 0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 空表下拉证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-046-safebox-session-filter`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - `SessionSummary` 结构最终定义；
   - session_id 参数为 None 时的行为（向后兼容 task-045）；
   - `cargo test -p engine` 通过输出；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（CDP 截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（时间范围筛选 / 多选 session / audit_log 联表留 task-047+）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务046-safe_box session 过滤 (#task-046)`，
   该提交只含 `tasks/046-safebox-session-filter.md` 一个文件，落在 `feature/task-046-safebox-session-filter` 分支最前端。

## 实现备注

### 实际改动文件
- engine/src/execute/safe_box.rs：list_paged 入参加 session_id: Option<i64>（按 source_task_id 过滤）；新增 SessionSummary{session_id,item_count,created_ts} + list_sessions(conn)（按 source_task_id GROUP BY，ORDER BY source_task_id DESC）。
- engine/tests/safe_box_pagination.rs：5 条旧测试调用点补 None 实参。
- engine/tests/safe_box_session_filter.rs（新建 4 条）。
- pp/src-tauri/src/commands.rs：safe_box_list(page,page_size,session_id,state)；新增 safe_box_list_sessions(state) -> Envelope<Vec<SessionSummary>>。
- pp/src-tauri/src/lib.rs：注册 safe_box_list_sessions。
- pp/src/pages/SafeBox.tsx：state 加 sessionFilter/sessions；首次挂载拉一次 sessions 填下拉；分页栏左侧加 <select>；切换时 page=1、清空 selected、重拉 list。

### SessionSummary 最终定义
`ust
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub session_id: i64,
    pub item_count: i64,
    pub created_ts: i64,
}
`

### 与任务书偏差（重要）
- 任务书写「safe_box 表已有 session_id 列（task-016 schema）」，**实际 schema 没有**：safe_box 只有 source_task_id INTEGER REFERENCES cleanup_task(id)。
- 实现选择：把 source_task_id 当作前端语义上的「session」——一个 cleanup_task 对应一次清理执行，等价于一次 session。list_sessions 按 source_task_id GROUP BY，list_paged(session_id) 过滤 source_task_id = ?。
- 未新增列、未做 scan_session 联表（保留「显示扫描来源路径」给 task-047+）。

### None 时行为（向后兼容 task-045）
- session_id=None → 不加 WHERE，全表 COUNT/SUM/分页；
- session_id=Some(id) → 同步加 AND source_task_id = ? 到 COUNT/SUM/items 三处。

### 验证
- cargo test -p engine 全绿（safe_box_session_filter 4/4；safe_box_pagination 5/5 仍绿）。
- cargo clippy --workspace --all-targets -- -D warnings = 0；cargo fmt --check = 0。
- pnpm lint/test/build = 0（vite 173.78 kB / gzip 55.27 kB）。

### 真机/打包冒烟
由测试 Agent 在 L4 完成（造 5 行 2 session → 下拉切 session #1 只显 3 条 / 切回全部显 5 条 / NSIS 空表下拉只显「全部 session」）。

### 遗留
- 时间范围筛选 / 多选 session / audit_log 联表 留 task-047+。
