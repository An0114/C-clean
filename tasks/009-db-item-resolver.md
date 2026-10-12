# 任务编号：009 DB 版 ItemPathResolver——cleanup_item 表按 id 取路径/大小/模块，替换 MockResolver

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRust
> 来源：`docs/PROJECT_STATE.md` 待办 009（M1 前置清理清单第 3 条，2026-10-12 debt-cleanup 批次001 拆出）。
> 上游依赖：task-002（`engine::db::conn::open` + 9 张表 schema 含 cleanup_item）、task-004（`engine::ipc::ItemPathResolver` trait + `MockResolver`）均已合并到 main。

## 任务目标

task-004 的 `cleanup.dryRun` 命令当前用 `MockResolver`（内存塞 `Vec<ResolvedItem>`），只为跑通 IPC 契约。本任务落地**真实 DB 版 resolver**：

1. 在 `engine/` 内新建 `db/resolver.rs`（或 `ipc/db_resolver.rs`，DevAgent 自选并备注理由），实现 `engine::ipc::ItemPathResolver` trait：
   - 入参持有 `&rusqlite::Connection`；
   - 按 `item_ids` 查 `cleanup_item` 表，返回 `Vec<ResolvedItem>`（含 `item_id / path / module_code / category / size_bytes`）；
   - 查不到的 id 直接跳过（不报错）；
2. 用 `tempfile::tempdir()` 造临时 DB，通过 task-002 的迁移器建表，INSERT 若干测试 `cleanup_item` 行，调 resolver 断言返回正确；
3. **不在 Tauri command 里替换 MockResolver**——真实扫描（M1 A 模块）还没产生数据，替换时机留给 M1 扫描任务。本任务只交付「可被调用的真实 resolver」+ 单测。

**本任务不做**：
- 不接 Tauri State、不改 `app/src-tauri/`；
- 不写真实扫描、不写 cleanup.execute 命令；
- 不改 task-002 已有 schema、迁移器；
- 不改 task-004 已有 `ItemPathResolver` trait 签名、`MockResolver` 实现、已有测试。

## 前置条件

1. `main` 分支已含 task-001~008 全部产出。
2. 关键锚点：
   - `engine/src/ipc/commands/dry_run.rs`（task-004）——`ItemPathResolver` trait、`ResolvedItem` 结构、`MockResolver`；
   - `engine/src/db/conn.rs`、`engine/src/db/migrate.rs`（task-002）——`open()` 返回 `rusqlite::Connection`；
   - `docs/TRD.md` §4.2 `cleanup_item` 表字段（约 144~155 行）——`id / session_id / module_code / category / path / size_bytes / risk_level / ...`；
   - `docs/ARCHITECTURE.md` §4 ER 图——cleanup_item → scan_session 级联删除。
3. 本机可跑 `cargo test -p engine`。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. 文件落点与结构

1. 新建 `engine/src/db/resolver.rs`，在 `engine/src/db/mod.rs` 加 `pub mod resolver;`。
2. 定义结构：
   ```rust
   pub struct DbItemPathResolver<'conn> {
       conn: &'conn rusqlite::Connection,
   }
   impl<'conn> DbItemPathResolver<'conn> {
       pub fn new(conn: &'conn rusqlite::Connection) -> Self;
   }
   ```
3. 对 `engine::ipc::ItemPathResolver` trait 落 impl：
   ```rust
   impl<'conn> engine::ipc::ItemPathResolver for DbItemPathResolver<'conn> {
       fn resolve(&self, item_ids: &[i64]) -> Vec<engine::ipc::ResolvedItem>;
   }
   ```

### B. 查询逻辑

4. `resolve()` 内部 SQL（参数化，禁止字符串拼接 ids）：
   ```sql
   SELECT id, module_code, category, path, size_bytes
   FROM cleanup_item
   WHERE id = ?1
   ```
   对 `item_ids` 里每个 id 单独一次 prepared statement（或用 `IN` 占位符动态构造，但必须用 ` rusqlite::params_from_iter`）；
   - 查到一行 → push `ResolvedItem { item_id: row.get(0), path: row.get(3), size_bytes: row.get(4), module_code: row.get(1), category: row.get(2) }`；
   - 查不到 → 跳过，不报错；
   - SQL 执行失败（如表不存在）→ 在 `resolve()` 内 `unwrap_or_default()` 返回空 Vec（**或**返回空 Vec 并在实现备注里说明为何不 propagrate 错误——由 DevAgent 选，建议 propagrate 成 `Result<Vec, rusqlite::Error>`，但 trait 签名是无 Result 的，所以本任务保持 trait 签名不变、查 DB 错误时返回空 Vec + `eprintln!` 临时日志）。
5. **不** JOIN `scan_session`（本 resolver 只关心 item 自身字段；后续需要 session 元信息再扩展）。

### C. 单测（`engine/tests/db_resolver.rs`）

6. 每个测试用 `tempfile::tempdir()` 造临时数据根，调 `engine::db::conn::open(tmp.path())` 建库并自动跑迁移。
7. 至少覆盖：
   - `resolver_returns_empty_when_no_items`：空 DB 下调 `resolve(&[1,2,3])` → 返回空 Vec；
   - `resolver_returns_matching_items`：先 `INSERT INTO scan_session(started_at) VALUES (1000)` 拿 `session_id`，再 INSERT 3 行 `cleanup_item`（id=10/11/12，path 各异、size_bytes 各异），调 `resolve(&[10,12])` → 返回 2 条，字段与 INSERT 一致（id/path/size_bytes/module_code/category）；
   - `resolver_skips_unknown_ids`：INSERT id=10，调 `resolve(&[10, 999])` → 只返回 1 条；
   - `resolver_dry_run_end_to_end`：INSERT 一条合法 temp 路径 + 一条 L3 路径（如 `C:\Users\alice\Desktop\secret.docx`）的 cleanup_item，调 `engine::ipc::commands::dry_run::handle_dry_run` 用 `DbItemPathResolver`，断言 outcome 是 `Rejected(code=2002)` 且 `blocked_by_l3` 含 L3 那条；
8. 测试中**不**碰真实用户目录、不发网络。

### D. 卫生与边界

9. 不修改 `engine/src/db/conn.rs`、`engine/src/db/migrate.rs`、`engine/src/db/migrations/v1__init.sql`（已有 schema 不动）；
10. 不修改 `engine/src/ipc/commands/dry_run.rs` 的已有 trait / MockResolver / `handle_dry_run` 签名；
11. 不修改 `app/**`、`elevation/**`、`rules/**`、`scripts/**`；
12. 不新增依赖（rusqlite 已在 task-002 引入）；
13. 不在生产代码里写 `println!` / `dbg!`（临时 eprintln 允许，但提交前清掉）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/db/resolver.rs`（新建）
- `engine/src/db/mod.rs`（仅加 `pub mod resolver;`）
- `engine/tests/db_resolver.rs`（新建集成测试）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-009-db-item-resolver`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(db): 新增 DbItemPathResolver 实现 ItemPathResolver trait (#task-009)`
   —— 含 `engine/src/db/mod.rs`、`engine/src/db/resolver.rs`。
2. `test(db): 补 DbItemPathResolver 集成测试（含 dryRun 端到端） (#task-009)`
   —— 含 `engine/tests/db_resolver.rs`。
3. 自验全绿后，最后一次提交：`docs(state): 任务009完成，真实 DB resolver 就绪 (#task-009)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 把待办 009 从 `- [ ]` 改为 `- [x]`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-010：外部红线集注入——rules JSON 加载后与内置 L3 红线取并集，接入 cleanup.dryRun」；
   - 「已知问题」段更新（task-004 MockResolver 标记为「已被 DbItemPathResolver 替代，但 Tauri 命令尚未切换，留 M1 扫描任务接线」）；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-12  task-009  DB 版 ItemPathResolver 落地并单测覆盖`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 60+ 条测试 + 本任务 4 条全绿）；
- `cargo fmt --all -- --check` 退出码 0。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 2 个代码提交 + 1 个 `docs(state)` 提交，共 3 个 commit，全部 push 到 `feature/task-009-db-item-resolver`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - DB 查询失败时的取舍（返回空 Vec vs propagrate）；
   - 自验命令与退出码；
   - `cargo test -p engine` 通过输出；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（Tauri command 何时切换 MockResolver → DbItemPathResolver）；
3. `docs/PROJECT_STATE.md` 已按第 3 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务009-DB版ItemPathResolver (#task-009)`，
   该提交只含 `tasks/009-db-item-resolver.md` 一个文件，落在 `feature/task-009-db-item-resolver` 分支最前端。

## 实现备注（DevAgent 落地后回填）
- **改动文件**：新建 `engine/src/db/resolver.rs`（`DbItemPathResolver<'conn>` 持有 `&rusqlite::Connection`）；`engine/src/db/mod.rs` 加 `pub mod resolver;`；新建集成测试 `engine/tests/db_resolver.rs`（4 用例）。
- **DB 查询失败取舍**：trait 签名无 Result，按任务建议「返回空 Vec + eprintln! 临时日志」，不 propagrate、不 panic；prepare 失败直接空 Vec，单行 QueryReturnedNoRows 跳过，其他 SQL 错误 eprintln 后继续下一个 id。
- **自验命令与退出码**：`cargo check --workspace --all-targets`=0；`cargo clippy --workspace --all-targets -- -D warnings`=0；`cargo fmt --all -- --check`=0；`cargo test -p engine`=0（10 个 test target 全 ok，含新增 db_resolver 4/4）。
- **偏差**：scan_session 插入时显式给了 `status='running', scope='full'`（schema NOT NULL）；size_bytes DB 为 INTEGER(i64)，读出后 `max(0) as u64` 对齐 ResolvedItem。
- **遗留问题**：Tauri 命令何时把 MockResolver 换成 DbItemPathResolver——等 M1 A 模块真实扫描写入 cleanup_item 后，在接线任务中切换。
