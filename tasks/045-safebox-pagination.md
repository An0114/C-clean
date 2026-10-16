# 任务编号：045 safe_box list 分页——后端 LIMIT/OFFSET + total 计数 + 前端分页控件

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：`docs/PROJECT_STATE.md` 已知问题（task-018 段）——「无分页（保险箱总量 ≤5GB，条目数有限）」。当前 safe_box_list 一次返回全部行，条目多时前端表格渲染卡。
> 上游依赖：task-016（safe_box 表 + box_dir）、task-018（`safe_box_list` 命令）、task-030（SafeBox.tsx 表格）、task-038/039（checkbox + 批量操作）。
> **克制范围**：只做单表分页，**不做** session 过滤（task-018 已知问题第二项，留 task-046）、**不做**排序选择（按 id DESC 默认）。

## 状态字段

pending

## 任务目标

当前 `safe_box_list()` 无参直接返回全表，保险箱条目数多时（比如 500+ 行）前端一次性渲染所有 `<tr>`，首次加载卡。

本任务补最小分页：

1. **后端** `safe_box_list` 签名改为：
   ```rust
   pub struct SafeBoxPage {
       pub items: Vec<SafeBoxItem>,
       pub total: i64,
       pub page: i64,          // 当前页（1 起）
       pub page_size: i64,
   }
   ```
   - 入参 `page: u32, page_size: u32`（默认 page=1, page_size=50，前端不传也兼容）；
   - SQL：`SELECT * FROM safe_box ORDER BY id DESC LIMIT ? OFFSET ?`；
   - 同时跑 `SELECT COUNT(*) FROM safe_box` 拿 total；
   - page_size 上限 200（防前端传 99999）；page < 1 时按 1 处理。
2. **前端** SafeBox.tsx：
   - state 加 `page`（默认 1）、`pageSize`（默认 50）、`total`；
   - 表格下方加分页栏：`[上一页] 第 X / Y 页 (共 N 条) [下一页]`；
   - 上一页 disabled 当 page=1；下一页 disabled 当 page*page_size >= total；
   - 翻页时重新 invoke `safe_box_list(page, pageSize)`；
   - 批量操作后清空 selected（避免跨页选中状态残留）。

**本任务不做**：
- 不做 session 过滤（留 task-046）；
- 不做列排序选择（默认 id DESC 就够）；
- 不做 URL/路由同步（state 内存态）；
- 不改 purge/restore 逻辑。

## 前置条件

1. `main` 分支已含 task-001~044 全部产出；
2. 关键锚点：
   - `engine/src/execute/safe_box.rs`（task-018 list 查询）；
   - `app/src-tauri/src/commands.rs`（`safe_box_list` 已注册）；
   - `app/src/pages/SafeBox.tsx`（task-030/038/039 已有表格 + checkbox + 批量按钮）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. 后端：safe_box_list 分页

1. 在 `engine/src/execute/safe_box.rs` 加：
   ```rust
   pub fn list_paged(conn: &rusqlite::Connection, page: u32, page_size: u32) -> Result<SafeBoxPage, String>;
   ```
2. 内部：
   - `page_size = page_size.clamp(1, 200)`；`page = page.max(1)`；
   - `OFFSET = (page - 1) * page_size`；
   - `SELECT COUNT(*) FROM safe_box` 拿 total；
   - `SELECT * FROM safe_box ORDER BY id DESC LIMIT ? OFFSET ?`；
   - 包装成 `SafeBoxPage { items, total, page: page as i64, page_size: page_size as i64 }`。
3. Tauri 命令 `safe_box_list` 入参加 `page: u32, page_size: u32`（前端必传）；返回 `Envelope<SafeBoxPage>`。

### B. 后端单测

4. 在 `engine/tests/safe_box_pagination.rs` 覆盖：
   - `list_paged_returns_first_page`：造 60 行 safe_box 数据，page=1 page_size=50 → items.len=50, total=60, page=1；
   - `list_paged_returns_second_page`：page=2 page_size=50 → items.len=10, total=60, page=2；
   - `list_paged_clamps_page_size`：page_size=99999 → 实际返回 ≤200 条；
   - `list_paged_empty_when_beyond_last_page`：page=99 → items.len=0, total=60；
   - `list_paged_id_desc_order`：第 1 页第一条 id 是 max(id)。

### C. 前端：SafeBox 分页栏

5. `app/src/pages/SafeBox.tsx`：
   - state：`page=1, pageSize=50, total=0`；
   - 首次加载 + 翻页时 `invoke('safe_box_list', { page, pageSize })`；
   - 表格下方加分页栏（flex 居中）：
     ```
     [上一页]   第 {page} / {Math.ceil(total/pageSize)} 页 (共 {total} 条)   [下一页]
     ```
   - 上一页 disabled 当 page===1；下一页 disabled 当 page*pageSize >= total；
   - 翻页后清空 `selected` Set（避免跨页残留）。

### D. 卫生

6. 不改 purge_item / purge_many / restore_item / restore_many；
7. 不改 checkbox / 批量按钮 UI；
8. audit_log schema 不变。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/safe_box.rs`（list_paged + SafeBoxPage）
- `engine/tests/safe_box_pagination.rs`（新建）
- `app/src-tauri/src/commands.rs`（safe_box_list 入参/返回改）
- `app/src/pages/SafeBox.tsx`（加分页栏）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-045-safebox-pagination`，从最新 `main` 拉出。
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-045-<描述>`，**只允许一层**；bugfix 修复后直接合并回 feature，不进 main。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(execute): safe_box.list_paged——LIMIT/OFFSET + COUNT total + page_size clamp (#task-045)`
   —— 含 `engine/src/execute/safe_box.rs`。
2. `feat(tauri): safe_box_list 入参加 page/page_size，返回 SafeBoxPage (#task-045)`
   —— 含 `app/src-tauri/src/commands.rs`。
3. `test(execute): 补 safe_box 分页集成测试（首页/次页/clamp/越界/ID DESC） (#task-045)`
   —— 含 `engine/tests/safe_box_pagination.rs`。
4. `feat(frontend): SafeBox 表格下加分页栏（上一页/下一页/页码/total） (#task-045)`
   —— 含 `app/src/pages/SafeBox.tsx`。
5. 自验全绿后，最后一次提交：`docs(state): 任务045完成，safe_box list 分页就绪 (#task-045)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 045: ...`；
   - 「最近一次变更」追加一行；
   - 「已知问题」段 task-018 那条把「无分页」划掉；
   - 「下一步计划」改为「派发 task-046：safe_box list session 过滤（按 cleanup session 筛选）——task-018 已知问题第二项」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-16  task-045  safe_box_list 分页（LIMIT/OFFSET + total + 前端分页栏）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 164+ 条 + 本任务 5 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：`safe_box_pagination.rs` 5 条覆盖首页/次页/clamp/越界/ID DESC。
- **集成测试（必做）**：`cargo test -p engine` 全绿，确保现有不回归。
- **端到端（不强制）**：本任务只改 list 查询 shape + 前端分页栏，不改业务流程。
- **真机冒烟（必做、不可裁剪）**：
  - 造 60 行 safe_box 数据（向 DB 直接 INSERT）→ SafeBox 表格只显示 50 行；
  - 分页栏显示「第 1 / 2 页 (共 60 条)」；
  - 点下一页 → 显示第 11~60 行（10 条）；
  - 点上一页回到第 1 页；
  - 跨页翻页后 selected Set 清空；
  - CDP 抓 console 0 红错。
- **打包安装冒烟（必做、不可裁剪）**：
  - NSIS 干净目录安装 → SafeBox 空表显示「第 1 / 1 页 (共 0 条)」，上一页/下一页均 disabled。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（SafeBox.tsx 新增分页栏 + 真机可观察行为），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| A1-A3 list_paged + SafeBoxPage | `cargo test -p engine` 输出（含 safe_box_pagination 用例 + passed） |
| B1-B5 后端单测 | `cargo test safe_box_pagination -- --nocapture` 完整输出 |
| C5 前端分页栏 | CDP 截图：60 行数据时第 1/2 页 / 下一页点击翻页 / 空表状态 |
| D6-D8 不回归 | `cargo test -p engine` 全绿 + `pnpm test` 全绿 |
| 真机冒烟 | CDP `http://localhost:9222/json` 抓 page + consoleAPICalled 0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 空表分页栏 disabled 证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-045-safebox-pagination`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - page_size 上限 / 默认值最终取值；
   - `SafeBoxPage` 结构最终定义；
   - `cargo test -p engine` 通过输出；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（CDP 截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（session 过滤 / 列排序留 task-046+）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务045-safe_box 分页 (#task-045)`，
   该提交只含 `tasks/045-safebox-pagination.md` 一个文件，落在 `feature/task-045-safebox-pagination` 分支最前端。

## 实现备注

### 实际改动文件
- engine/src/execute/safe_box.rs：新增 SafeBoxPage{items,total,page,pageSize,totalBytes} + list_paged(conn,page,page_size)；原 list_safe_box 保留未删（避免破坏其他调用点）。
- engine/tests/safe_box_pagination.rs（新建 5 条）。
- pp/src-tauri/src/commands.rs：safe_box_list(page,page_size) 返回 Envelope<SafeBoxPage>。
- pp/src/pages/SafeBox.tsx：state 加 page/pageSize/total；load 入参 (p, ps)；表格下加分页栏（上一页 / 第 X / Y 页（共 N 条） / 下一页）；翻页清空 selected。

### page_size 上限 / 默认值
- 默认 pageSize=50；后端 clamp 到 [1, 200]；page < 1 按 1 处理。

### SafeBoxPage 最终定义
`ust
#[serde(rename_all = "camelCase")]
pub struct SafeBoxPage {
    pub items: Vec<SafeBoxEntry>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_bytes: u64,
}
`

### 验证
- cargo test -p engine 全绿（含 safe_box_pagination 5/5）。
- pnpm lint/test/build=0（vite 173.18 kB / gzip 55.10 kB）。

### 偏差
- 原 list_safe_box（返回 SafeBoxListResp）保留未删，避免破坏其他潜在调用点；Tauri 命令已切到 list_paged。
- 排序按任务书用 ORDER BY id DESC（旧 list 是 quarantined_at DESC）。
- total_bytes 用全表 SUM（不是当前页），保留 UI 顶栏「总大小」显示。

### 真机/打包冒烟
由测试 Agent 在 L4 完成（造 60 行 → 第 1/2 页 → 翻页 → NSIS 空表 disabled）。

### 遗留
- session 过滤 / 列排序选择 留 task-046+。
