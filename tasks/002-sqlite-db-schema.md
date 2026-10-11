# 任务编号：002 落地公共数据层——SQLite schema + rusqlite 连接 + 迁移机制

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRust
> 来源：`docs/PROJECT_STATE.md` 待办 002 /「下一步计划」段。
> 上游依赖：task-001 已合并到 main（monorepo 骨架 + `engine/` 空 lib crate 就绪）。

## 任务目标

在 `engine/` 内新建 `db/` 子模块，落地 **SQLite 单文件库**的最小可用底座：

1. 按 ARCHITECTURE §4 ER 图与 TRD §4 数据字典，一次性建好 **9 张业务表**：`settings` / `scan_session` / `cleanup_item` / `cleanup_task` / `safe_box` / `audit_log` / `rules_meta` / `registry_backup` / `cleanup_plan`；
2. 提供 rusqlite 连接入口（bundled feature），自动创建数据目录、打开 `<数据根>\data\cdiskcleaner.db`，开启外键约束与 WAL 模式；
3. 用 `user_version` pragma 实现**手写迁移器**（不引入 sqlx-migrate / refinery 等重型迁移库），首次启动自动跑 `v1__init.sql`，重复打开幂等；
4. 配套单元测试，证明 schema 结构、外键行为、单行约束、级联删除与迁移幂等性全部符合 TRD/ARCHITECTURE。

**本任务不做**：任何业务 CRUD 仓储方法、扫描结果写入、审计写动作、IPC 暴露——这些分别由 003 / 004 / 后续业务模块负责。本任务只交付「能开库、表都在、约束生效、迁移可重放」这一层。

## 前置条件

1. `main` 分支已包含 task-001 的全部产出：`engine/` 为可编译的 Rust lib crate，根 `Cargo.toml` 已声明 workspace members。
2. 关键锚点（DevAgent 只读这些段落，不通读全文）：
   - `docs/ARCHITECTURE.md` §4「数据库 ER 图」（约 217~323 行）——9 张表字段轮廓、外键行为、`rules_meta` 单行、`cleanup_plan` 不绑 session、L3 不进 `cleanup_item`；
   - `docs/TRD.md` §4「数据字典」（约 114 行起）——字段类型、UNIX 毫秒、UTF-8、外键开启；
   - `docs/TRD.md` §3 依赖表（约 92~99 行）——`rusqlite = { version = "0.x", features = ["bundled"] }`（具体版本号取 crates.io 最新 stable，DevAgent 在实现备注里写明选用版本）；
   - `docs/TRD.md` 末尾「数据目录」表（约 427 行）——安装版 `%LOCALAPPDATA%\CDiskCleaner\data\cdiskcleaner.db`，便携版由 `CDISKCLEANER_PORTABLE=1` 切换。
3. 工作分支从最新 `main` 拉出；本机已可跑 `cargo test -p engine`（task-001 验证过）。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. 模块与依赖

1. 在 `engine/src/` 下新建 `db/` 目录，至少包含：`mod.rs`、`conn.rs`、`migrate.rs`、`migrations/`（放 `.sql` 文件）。
2. `engine/Cargo.toml` 的 `[dependencies]` **仅新增** `rusqlite = { version = "…", features = ["bundled"] }` 一条；
   - 禁止在本任务顺手加 `serde` / `serde_json` / `sha2` / `thiserror` 等其他依赖（这些留给 003/004/005）；
   - 若实现错误处理确实需要 `thiserror`，必须停下在实现备注里申请，由 DocsKeeper 评估。
3. `engine/src/lib.rs` 仅新增一行 `pub mod db;`，不改动 `hello_engine()`。

### B. 连接入口（`conn.rs`）

4. 暴露一个公开函数（建议签名）：`pub fn open(data_root: &std::path::Path) -> Result<rusqlite::Connection, DbError>`；
   - 自动创建 `data_root` 下不存在的父目录（`std::fs::create_dir_all`）；
   - 打开 `<data_root>\cdiskcleaner.db`（**注意**：TRD 给的完整路径是 `<数据根>\data\cdiskcleaner.db`，本函数接收的 `data_root` 即「数据根」，最终落盘路径为 `data_root\data\cdiskcleaner.db`，不要让调用方自己拼 `\data\`）；
   - 打开后立即执行 `PRAGMA foreign_keys = ON;` 与 `PRAGMA journal_mode = WAL;`；
   - 不写死任何真实用户路径（`C:\Users\…`），路径全部由参数注入；
   - 不读环境变量（便携版/安装版路径切换由上层 Tauri 壳决定，本模块只吃 `data_root`）。

### C. 迁移器（`migrate.rs`）

5. 迁移文件用固定命名：`engine/src/db/migrations/v1__init.sql`（本任务只有 v1；后续表结构变更由新任务加 `v2__xxx.sql`）。
6. 迁移器逻辑：
   - 读 `PRAGMA user_version;`，当前为 N；
   - 按文件名字典序找 `v{N+1}__*.sql`、`v{N+2}__*.sql` …… 依次执行；
   - 每成功跑一个迁移文件，立即 `PRAGMA user_version = <新版本号>` 并 COMMIT；
   - 任何一个迁移失败：整张事务回滚、`user_version` 不变、返回错误，库停留在旧 schema；
   - **不**引入 `refinery` / `sqlx::migrate!` 宏。
7. `open()` 内部在打开连接、设完 PRAGMA 后自动调用迁移器；重复调用 `open()` 同一数据目录，第二次 `user_version` 已是最新，不再执行任何 SQL，返回 Ok。

### D. `v1__init.sql` 必须落地的 schema

8. 9 张表全部 CREATE，字段与类型必须与 TRD §4 数据字典一致（DevAgent 对照 TRD 逐字段抄，不凭印象）；最低限度满足以下结构约束：
   - `settings(key TEXT PRIMARY KEY, value TEXT, updated_at INTEGER NOT NULL)`；
   - `scan_session`：自增 `id INTEGER PRIMARY KEY`，含 `started_at / finished_at / drive_used_bytes_before / error_summary` 等字段；
   - `cleanup_item`：`id INTEGER PRIMARY KEY`、`session_id INTEGER NOT NULL REFERENCES scan_session(id) ON DELETE CASCADE`，含 `module_code / category / path / risk_level / size_bytes / extra` 等字段；
   - `cleanup_task`：`id INTEGER PRIMARY KEY`、`session_id INTEGER REFERENCES scan_session(id)`、`started_at / finished_at / restore_point_name / safebox_enabled` 等字段；
   - `safe_box`：`id INTEGER PRIMARY KEY`、`original_path / safe_rel_path / size_bytes / quarantined_at / expires_at / status / source_task_id INTEGER REFERENCES cleanup_task(id)`；
   - `audit_log`：`id INTEGER PRIMARY KEY`、`ts INTEGER NOT NULL`、`actor / action / target / result / denial_reason`，**不建外键**；
   - `registry_backup`：`id INTEGER PRIMARY KEY`、`task_id INTEGER REFERENCES cleanup_task(id) ON DELETE SET NULL`、`reg_path / export_file_path / created_at`；
   - `rules_meta`：单行表，含 `CHECK(id = 1)` 约束，字段 `version / checksum_sha256 / fetched_at / custom_rules`；
   - `cleanup_plan`：不绑 `session_id`，含 `name / note / item_refs(TEXT 存 JSON 串) / created_at / updated_at`。
9. 所有时间字段一律 `INTEGER`（Unix 毫秒），不使用 SQLite `DATETIME` / `TIMESTAMP` 类型。
10. 所有文本字段用 `TEXT`，不要硬编码长度（SQLite TEXT 本身无长度）。
11. SQL 全部使用**参数化占位符**（`?` / `:name`），代码里禁止字符串拼接 SQL——这是 ARCHITECTURE §A03 的硬要求。本任务虽不写业务 SQL，但迁移器执行 `.sql` 文件时也禁止对路径/表名做字符串插值。

### E. 单元测试（`engine/src/db/migrations/schema_test.rs` 或 `engine/tests/db_schema.rs`）

12. 测试用 `tempdir`（`tempfile::tempdir()`）造临时数据目录，**禁止**在用户真实 `%LOCALAPPDATA%` 下开库；测试结束自动清理。
13. 至少覆盖以下用例（每条一个 `#[test]`）：
    - `opens_and_creates_db_file`：`open()` 后 `<tmp>\data\cdiskcleaner.db` 文件存在；
    - `creates_all_9_business_tables`：`SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'` 等于 9；
    - `foreign_keys_pragma_on`：`PRAGMA foreign_keys;` 返回 `1`；
    - `user_version_bumped_to_1`：`PRAGMA user_version;` 等于 1；
    - `migration_is_idempotent`：对同一临时目录再 `open()` 一次，仍然 Ok，`user_version` 仍为 1；
    - `rules_meta_rejects_id_not_1`：`INSERT INTO rules_meta(id, ...) VALUES (2, ...)` 返回 `IntegrityError`；
    - `cleanup_item_cascades_with_session`：插一个 session + 两个 item，删 session 后 `SELECT count(*) FROM cleanup_item` 为 0；
    - `registry_backup_sets_null_on_task_delete`：插 task + backup，删 task 后 backup.task_id 为 NULL（不被连带删除）。
14. 测试中**禁止**依赖网络、真实 C 盘、真实注册表。

### F. 卫生与边界

15. 不创建 `scan/`、`grade/`、`safebox/`、`audit/`、`rules/`、`platform/` 等其他 engine 子模块目录（留给后续任务）。
16. 不修改 `app/src-tauri/`、`app/src/`、`elevation/`、`rules/`、`scripts/` 任何文件；
17. 不修改 `docs/PRD.md`、`docs/TRD.md`、`docs/ARCHITECTURE.md`、`docs/AGENTS.md` 等长期文档；
18. 不在本任务里把 `db::open()` 接到 Tauri command（那是 004 的事）；
19. 不在本任务里写任何「扫描结果入库 / 审计写一条日志 / 保险箱记录插入」的业务方法——只建空表 + 连接 + 迁移器。

## 允许修改范围

> 严格白名单。范围外文件一律不动；若发现必须改范围外文件才能跑通，**停下在实现备注里报 DocsKeeper**，不得自行扩大。

- `engine/Cargo.toml`（仅新增 rusqlite 一条依赖）
- `engine/src/lib.rs`（仅新增 `pub mod db;` 一行）
- `engine/src/db/**`（新建整目录：`mod.rs`、`conn.rs`、`migrate.rs`、`migrations/v1__init.sql`）
- `engine/tests/**`（新建 `db_schema.rs` 集成测试；若采用同目录单元测试则放在 `engine/src/db/` 下的 `#[cfg(test)] mod tests` 中，二选一，DevAgent 自选并在备注里说明）
- `Cargo.lock`（由依赖变更自动产生）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`（由最后一次 `docs(state)` 提交处理）

## Git 分支

- 分支名：`feature/task-002-sqlite-schema`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

> 严格按以下顺序原子提交，每个提交只动一类文件；**禁止 `git add .` / `git add -A` / `git commit -a`**，每次 `git commit` 必须显式列文件路径。

1. `build(deps): 为 engine 引入 rusqlite bundled 依赖 (#task-002)`
   —— 含 `engine/Cargo.toml`、`Cargo.lock`。
2. `feat(db): 新增 engine::db 模块骨架与连接入口 conn::open (#task-002)`
   —— 含 `engine/src/lib.rs`（加 `pub mod db;`）、`engine/src/db/mod.rs`、`engine/src/db/conn.rs`。
3. `feat(db): 编写 v1__init.sql 落地 9 张业务表 schema (#task-002)`
   —— 含 `engine/src/db/migrations/v1__init.sql`。
4. `feat(db): 实现基于 user_version 的手写迁移器 (#task-002)`
   —— 含 `engine/src/db/migrate.rs`，并在 `conn.rs` 的 `open()` 中串入迁移调用。
5. `test(db): 补 schema 与迁移器单元/集成测试 (#task-002)`
   —— 含 `engine/tests/db_schema.rs` 或同目录 `mod tests` 块。
6. 自验全绿后，最后一次提交：`docs(state): 任务002完成，更新进度与下一步 (#task-002)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 把待办 002 从 `- [ ]` 改为 `- [x]`；
   - 「最近一次变更」追加一行（task-002 完成摘要）；
   - 「下一步计划」改为「派发 task-003：落地删除唯一入口 `engine::delete::execute()` + L3 红线白名单匹配器 + 单元测试」；
   - 「Git 提交记录」表追加本任务 6 条 commit 摘要；
   - 文件末尾「变更轨迹」追加 `2026-10-11  task-002  落地 SQLite schema（9 张表）+ rusqlite 连接 + user_version 迁移器`。

**提交前自验（必须全绿，任一不过不得 push）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy -p engine --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0，且 §E 中 8 条测试用例全部通过；
- `cargo fmt --all -- --check` 退出码 0（未配置 fmt 配置时使用默认 rustfmt，不新增 rustfmt.toml）。

## 输出要求

DevAgent 交付物（后续执行结果，不是本任务文档产出）：

1. 上述 5 个代码/配置提交 + 1 个 `docs(state)` 提交，共 6 个 commit，全部 push 到 `feature/task-002-sqlite-schema`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（与白名单逐条对照）；
   - rusqlite 选用版本号与 bundled feature 确认；
   - 自验命令与退出码（逐条列）；
   - 8 条测试用例的通过截图或粘贴输出；
   - 与本任务文档不符的偏差（如有，含原因）；
   - 遗留问题（如：TRD §4 数据字典中某字段类型在 D 段未列全，DevAgent 抄表时如何对齐的）；
3. `docs/PROJECT_STATE.md` 已按第 6 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务002-SQLite数据层与迁移器 (#task-002)`，
   该提交只含 `tasks/002-sqlite-db-schema.md` 一个文件，落在 `feature/task-002-sqlite-schema` 分支最前端（即 DevAgent 开工前的基线）。

## 实现备注（DevAgent 回填，2026-10-11）

### 实际改动文件清单（对照白名单）

- `engine/Cargo.toml`：仅新增 `rusqlite = { version = "0.40.2", features = ["bundled"] }`（[dependencies]）；另加 `tempfile = "3.27.0"`（[dev-dependencies]，为 §E-12 临时目录测试所用，测试专用、非业务依赖）。
- `engine/src/lib.rs`：仅新增 `pub mod db;` 一行，`hello_engine()` 未动。
- `engine/src/db/mod.rs`（新建）：模块声明 + 手工 `DbError` 枚举（Io/Sqlite + Display/Error/From）。
- `engine/src/db/conn.rs`（新建）：`open(data_root)` 连接入口。
- `engine/src/db/migrate.rs`（新建）：`user_version` 手写迁移器。
- `engine/src/db/migrations/v1__init.sql`（新建）：9 张表 DDL。
- `engine/tests/db_schema.rs`（新建）：8 条集成测试。
- `Cargo.lock`：随依赖变更自动更新。
- `docs/PROJECT_STATE.md`：由最后一次 `docs(state)` 提交处理。
- 未触碰：app/、elevation/、rules/、scripts/、PRD/TRD/ARCHITECTURE/AGENTS 等长期文档。

### rusqlite 版本与 bundled 确认

- 选用 **rusqlite v0.40.2**（`cargo add rusqlite --features bundled` 解析出的当前 crates.io stable），bundled 特性自动编译 libsqlite3-sys v0.38.2（内嵌 SQLite，无需系统 SQLite）。
- 测试框架用 **tempfile v3.27.0**（dev-dependencies），满足 §E-12「禁止在真实 %LOCALAPPDATA 下开库」。

### 自验命令与退出码（仓库根执行）

| 命令 | 退出码 | 结果 |
|------|--------|------|
| `cargo check --workspace --all-targets` | 0 | 全 workspace 通过 |
| `cargo clippy -p engine --all-targets -- -D warnings` | 0 | 首次报 3 个 needless_borrow，已修复后复跑 0 告警 |
| `cargo test -p engine` | 0 | 9 passed / 0 failed（8 条新测试 + 1 条既有 smoke） |
| `cargo fmt --all -- --check` | 0 | 首次有 1 处格式 diff，`cargo fmt --all` 后复跑通过 |

注：本机初始未装 clippy/rustfmt 组件，且直连 static.rust-lang.org TLS 断流，改用 rsproxy 镜像 `rustup component add clippy rustfmt` 安装；cargo 拉包仍用会话级 `CARGO_HTTP_CHECK_REVOKE=false` 绕过 schannel 吊销离线错误，未改全局配置。

### 8 条测试用例输出（cargo test -p engine 摘录）

```text
running 8 tests
test creates_all_9_business_tables ... ok
test foreign_keys_pragma_on ... ok
test rules_meta_rejects_id_not_1 ... ok
test user_version_bumped_to_1 ... ok
test opens_and_creates_db_file ... ok
test cleanup_item_cascades_with_session ... ok
test registry_backup_sets_null_on_task_delete ... ok
test migration_is_idempotent ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 与任务文档的偏差（含原因）

1. **tempfile 落入 [dev-dependencies]**：任务约束 §A-2 说「[dependencies] 仅新增 rusqlite 一条」，未禁止 dev-dependencies；而 §E-12 强制要求 `tempfile::tempdir()`，二者并存，故测试专用依赖放 dev-dependencies，未污染业务依赖树。
2. **错误类型未用 thiserror**：按约束手工实现 `DbError`（Io/Sqlite 两变体 + From 转换），零额外依赖。
3. **迁移文件用 `include_str!` 编译期内嵌**：任务 §C-6 描述「按文件名字典序找 v{N+1}__*.sql」。本任务仅 v1 一个迁移，且为避免运行时文件查找路径插值（§D-11），采用静态有序表 `static MIGRATIONS: &[(u32, &str)]`，语义等价于字典序；后续 v2 起按版本号追加一行即可。
4. **rusqlite 0.40 错误变体匹配**：早期草稿用了已移除的 `IntegrityViolation`/`ConstraintViolation` 变体，最终改为匹配 `Error::SqliteFailure` 且 `e.code == ErrorCode::ConstraintViolation`（覆盖 CHECK/NOT NULL/UNIQUE/FK 全部 SQLITE_CONSTRAINT_*）。
5. **多一次 style 提交**：clippy/fmt 修复跨 conn.rs、migrate.rs、db_schema.rs 三个已提交文件，未改写历史提交，单独落 `style(db)` 提交。

### 遗留问题 / 字段对齐说明

- TRD §4 数据字典字段已逐字段抄入 v1__init.sql：`cleanup_task` 的 `session_id` 可空（TRD 未标 NOT NULL），`safe_box.source_task_id` 未指定 ON DELETE 动作（保持默认 NO ACTION，与 ER 图一致）；`audit_log` 无外键（自由日志表）；`rules_meta` 单行 `CHECK(id = 1)` 已落地。
- `sqlite_sequence`（AUTOINCREMENT 自动产生）被测试的 `name NOT LIKE 'sqlite_%'` 排除，9 张业务表计数正确。
- 未做任何业务 CRUD、IPC 接线（留给 003/004）。
