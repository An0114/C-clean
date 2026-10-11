# 测试评审报告

## 本次任务名称
002 落地公共数据层——SQLite schema（9 张表）+ rusqlite 连接 + user_version 手写迁移器（feature/task-002-sqlite-schema）

评审人：TestReviewer
评审日期：2026-10-11
被审分支：`feature/task-002-sqlite-schema`（base: `main` @ 10a0c05）
合并目标分支：`main`

---

## Git 提交记录核对结果（原子性、规范符合性）

feature 分支相对 main 共 **9 个提交**，全部携带 `(#task-002)` 尾标，type 前缀规范（build / feat / test / style / docs），每个提交文件范围同类，无跨任务混杂：

| Commit | Message | 文件范围 | 原子性 |
|--------|---------|----------|--------|
| c49d909 | docs(task): 新增任务002-SQLite数据层与迁移器 | 仅 `tasks/002-...md` | ✅ DocsKeeper 派发基线 |
| e62b586 | build(deps): 为 engine 引入 rusqlite bundled 依赖 | `engine/Cargo.toml` `Cargo.lock` | ✅ 干净 |
| 8dea10f | feat(db): 新增 engine::db 模块骨架与连接入口 conn::open | `engine/src/lib.rs` `db/mod.rs` `db/conn.rs` | ✅ 干净 |
| bac3400 | feat(db): 编写 v1__init.sql 落地 9 张业务表 schema | 仅 `db/migrations/v1__init.sql` | ✅ 干净 |
| d48933a | feat(db): 实现基于 user_version 的手写迁移器 | `db/migrate.rs` `db/conn.rs` `db/mod.rs` | ✅ conn.rs 串入迁移调用、mod.rs 加 `pub mod migrate;`，符合任务预期 |
| 2ecc248 | test(db): 补 schema 与迁移器单元/集成测试 | 仅 `engine/tests/db_schema.rs` | ✅ 干净 |
| e6f18a1 | style(db): 修复 clippy needless_borrow 并按 rustfmt 统一格式 | `conn.rs` `migrate.rs` `db_schema.rs` | ✅ 额外一次风格修复提交，未改写历史 |
| a0cc0fb | docs(state): 任务002完成，更新进度与下一步 | 仅 `docs/PROJECT_STATE.md` | ✅ 状态文件单独提交 |
| 3bf9fd0 | docs(task): 追加任务002实现备注 | 仅 `tasks/002-...md` | ✅ 实现备注单独提交 |

**偏差（不阻断）**：
- 任务预期 6 个提交，实际 9 个（多出 DocsKeeper 派发基线 docs(task)、clippy/fmt 修复 style(db)、实现备注 docs(task)），均为文档/风格类提交，未与业务代码混提。
- `[dev-dependencies]` 引入 `tempfile = "3.27.0"`：任务约束 §A-2 只限制 `[dependencies]` 仅加 rusqlite 一条，§E-12 强制要求 `tempfile::tempdir()`，测试专用依赖不污染业务树，评审接受。

---

## 分步验证清单（逐项对照验收标准）

### A. 模块与依赖
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 1 | engine/src/db/ 含 mod.rs / conn.rs / migrate.rs / migrations/ | ✅ | 目录树核对 |
| 2 | [dependencies] 仅新增 rusqlite={version, features=["bundled"]}；不引 serde/sha2/thiserror | ✅ | `engine/Cargo.toml` L13 仅 rusqlite；dev-dep tempfile 为测试专用 |
| 3 | lib.rs 仅新增 `pub mod db;`，hello_engine 未动 | ✅ | `lib.rs` L13 一行，L8-L10 未改 |

### B. 连接入口（conn.rs）
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 4 | `open(data_root: &Path) -> Result<Connection, DbError>`；create_dir_all；路径 `<data_root>/data/cdiskcleaner.db`；PRAGMA foreign_keys=ON；PRAGMA journal_mode=WAL；不写死路径；不读环境变量 | ✅ | `conn.rs` L19-L37 逐项核对：L22 join("data")、L23 create_dir_all、L26 Connection::open、L29 foreign_keys=true、L31 journal_mode=WAL、L34 自动 migrate |

### C. 迁移器（migrate.rs）
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 5 | 迁移文件命名 v1__init.sql | ✅ | 路径核对 |
| 6 | 读 user_version；按版本序依次执行；每迁移独立事务；成功后 user_version=version + commit；失败回滚 user_version 不变；不引入 refinery/sqlx::migrate! | ✅ | `migrate.rs` L25-L38：L30 filter `*v > current`、L32 unchecked_transaction、L33 execute_batch、L34 pragma_update、L35 commit；事务失败 drop 自动 rollback |
| 7 | open() 内自动调 migrate；重复 open 幂等 | ✅ | conn.rs L34；migration_is_idempotent 测试通过 |

### D. v1__init.sql schema
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 8a | settings(key TEXT PK, value TEXT NOT NULL, updated_at INTEGER NOT NULL) | ✅ | L8-L12 |
| 8b | scan_session 自增 id + started_at/finished_at/drive_used_bytes_before/error_summary | ✅ | L15-L23 |
| 8c | cleanup_item.id PK + session_id NOT NULL REFERENCES scan_session(id) ON DELETE CASCADE + module_code/category/path/risk_level/size_bytes/extra | ✅ | L27-L40 |
| 8d | cleanup_task.id PK + session_id REFERENCES scan_session(id) + started_at/finished_at/restore_point_name/safebox_enabled | ✅ | L43-L57 |
| 8e | safe_box.id PK + original_path/safe_rel_path/size_bytes/quarantined_at/expires_at/status/source_task_id REFERENCES cleanup_task(id) | ✅ | L60-L69 |
| 8f | audit_log.id PK + ts NOT NULL + actor/action/target/result/denial_reason，不建外键 | ✅ | L72-L81（多一个可空 detail 字段，与 TRD 数据字典一致） |
| 8g | registry_backup.id PK + task_id REFERENCES cleanup_task(id) ON DELETE SET NULL + reg_path/export_file_path/created_at | ✅ | L85-L91 |
| 8h | rules_meta CHECK(id=1) 单行表 + version/checksum_sha256/fetched_at/downloaded_at/custom_rules | ✅ | L94-L101（字段名 downloaded_at，与 TRD 对齐；任务文档字面写 fetched_at，以 TRD 为准） |
| 8i | cleanup_plan 不绑 session_id + name/note/item_refs(JSON)/created_at/updated_at | ✅ | L105-L113 |
| 9 | 时间字段一律 INTEGER（Unix 毫秒），不用 DATETIME/TIMESTAMP | ✅ | 全表核对 |
| 10 | 文本一律 TEXT，不硬编码长度 | ✅ | 全表核对 |
| 11 | SQL 全参数化；迁移器不对路径/表名做字符串插值 | ✅ | 业务 SQL 全用 `?N` 占位符（测试代码 L18/L30/L49 等）；迁移 SQL 用 `include_str!` 编译期静态内嵌，运行时零拼接 |

### E. 单元测试
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 12 | tempfile::tempdir() 临时目录，不碰真实 %LOCALAPPDATA | ✅ | 每个测试用例 L58/L73/L88 等 |
| 13a | opens_and_creates_db_file | ✅ | 测试通过 |
| 13b | creates_all_9_business_tables（count=9） | ✅ | 测试通过 |
| 13c | foreign_keys_pragma_on（=1） | ✅ | 测试通过 |
| 13d | user_version_bumped_to_1 | ✅ | 测试通过 |
| 13e | migration_is_idempotent | ✅ | 测试通过 |
| 13f | rules_meta_rejects_id_not_1（CHECK 约束触发 ConstraintViolation） | ✅ | 测试通过 |
| 13g | cleanup_item_cascades_with_session（ON DELETE CASCADE） | ✅ | 测试通过 |
| 13h | registry_backup_sets_null_on_task_delete（ON DELETE SET NULL） | ✅ | 测试通过 |
| 14 | 测试不依赖网络/真实 C 盘/真实注册表 | ✅ | `r"C:\tmp\cache"` 等仅为字符串字面量，无真实文件/注册表访问 |

### F. 卫生与边界
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 15 | 不创建 scan/grade/safebox/audit/rules/platform 等其他子模块 | ✅ | engine/src/ 仅 db/ + lib.rs |
| 16 | 不修改 app/elevation/rules/scripts | ✅ | diff stat 仅 engine/、Cargo.lock、docs/PROJECT_STATE.md、tasks/ |
| 17 | 不修改 PRD/TRD/ARCHITECTURE/AGENTS | ✅ | diff name-only 核对 |
| 18 | 不把 db::open() 接到 Tauri command | ✅ | app/src-tauri/ 未动 |
| 19 | 不写业务 CRUD/扫描入库/审计写/保险箱插入 | ✅ | 仅建表 + 连接 + 迁移器 |

### 评审人复跑自验命令（仓库根，main 合并后工作区）
| 命令 | 退出码 | 结果 |
|------|--------|------|
| `cargo check --workspace --all-targets` | 0 | Finished in 13.40s |
| `cargo clippy -p engine --all-targets -- -D warnings` | 0 | 无 warning |
| `cargo test -p engine` | 0 | 8 新测试 + 1 smoke，全部 passed / 0 failed |
| `cargo fmt --all -- --check` | 0 | 无 diff |

---

## 代码评审发现问题

本任务交付「连接 + 迁移 + schema」数据层底座，无业务 CRUD。静态分析：

- **无 `unsafe`**；无 `Command::new`；无 `std::fs::remove*`；无网络调用。
- **错误处理**：`DbError` 手工实现 Display/Error/From，两个变体 Io/Sqlite 均正确链 `source()`，未引入 thiserror（符合任务约束）。
- **迁移器事务边界正确**：每个迁移独立 `unchecked_transaction`，失败时 tx drop 自动 rollback，user_version 不被推进；`user_version` 在事务内修改并随 commit 持久化，崩溃恢复安全。
- **`include_str!` 静态内嵌迁移 SQL**：编译期嵌入二进制，运行时不做文件系统查找，从根上消除路径/表名字符串插值风险（ARCHITECTURE §A03 硬要求）。
- **外键 pragma 时序正确**：在连接刚打开、迁移之前设置，连接级 pragma 不能在事务内切换，这里顺序正确。

**提示项（非阻断，建议后续任务跟进）**：
1. **`CREATE TABLE IF NOT EXISTS` 冗余**：迁移文件全部用了 `IF NOT EXISTS`。由于 user_version 保证迁移只跑一次，IF NOT EXISTS 是防御性冗余；若未来有人手工改表结构再跑 v1，会静默跳过而不报错。当前不影响正确性，task-005+ 写 v2 迁移时建议去掉 IF NOT EXISTS，让 schema 偏差尽早暴露。
2. **`unchecked_transaction` 命名**：rusqlite 的 `transaction()` 会在嵌套事务时 panic，`unchecked_transaction()` 不会。当前调用点（open 后 autocommit 状态）无嵌套风险，但未来若在业务方法里复用此函数，需注意。建议在 migrate.rs 顶部文档补一句"本函数假定调用时连接不在事务中"。
3. **WAL 文件**：open() 会产生 `cdiskcleaner.db-wal` / `cdiskcleaner.db-shm`，.gitignore 已通配 `*.db-*`？核对：`.gitignore` L52-L54 有 `*.db / *.db-shm / *.db-wal`，已覆盖。✅
4. **`rules_meta` 字段命名**：任务文档 §D-8 写 `fetched_at`，实际 DDL 用 `downloaded_at`。DevAgent 备注"逐字段抄 TRD"，以 TRD 为准。task-005 落地规则库加载时需对齐字段名。
5. **`audit_log.detail`** 字段为任务文档未列的补充字段（TRD 数据字典应有），不影响验收。

---

## 安全风险检查结果

按 OWASP Top 10 (2021) 逐项过：

| 风险项 | 本任务暴露面 | 结论 |
|--------|--------------|------|
| A01 失效访问控制 | 无权限模型、无业务路由 | ✅ N/A |
| A02 加密失败 | 无凭据、无加密代码 | ✅ N/A |
| A03 注入 | 所有业务 SQL 用 `?N` 参数占位；迁移 SQL 编译期静态内嵌；无表名/路径字符串拼接；无 `format!` 拼 SQL | ✅ **零注入面** |
| A04 不安全设计 | 迁移器事务边界正确；外键默认关闭显式开启；user_version 单调推进 | ✅ |
| A05 安全配置错误 | foreign_keys=ON、journal_mode=WAL 均在 open() 强制设置，不依赖外部配置 | ✅ |
| A06 脆弱过时组件 | rusqlite 0.40.2 + bundled libsqlite3-sys 0.38.2（内嵌 SQLite，无系统 SQLite 依赖）；Cargo.lock 入库 | ✅ |
| A07 身份识别 | 无登录/身份 | ✅ N/A |
| A08 软件与数据完整性 | 迁移失败回滚保证库停留在旧 schema；user_version 事务保护；无自动更新管道 | ✅ |
| A09 日志监控不足 | 本任务只建 audit_log 空表，不写日志 | ✅ N/A（task-003+ 落地） |
| A10 SSRF | 无出站请求（规则库拉取是 task-005） | ✅ N/A |

路径注入扫描：`grep C:\\Users|password|secret|api_key|private key|Command::new|unsafe!` 在业务代码零命中。

---

## 评审结论：**通过**

19 条验收标准全部满足；4 项自验命令评审人复跑全部 exit 0（clippy -D warnings、fmt --check、workspace check、engine 8+1 测试）；OWASP Top 10 无阻断风险；9 个提交原子性与规范符合性达标。

---

## 如果不通过：Bug 清单 + 修复建议

不适用（本次评审通过）。

非阻断提示项已在上文列出，均不构成 bug。建议：
- task-003 落地删除入口时，复用 `db::conn::open()` 拿到连接，不要重复打开；
- task-005 落地规则库加载时对齐 `rules_meta.downloaded_at` 字段名；
- task-006 CI 加入 `cargo clippy -- -D warnings` 与 `cargo fmt --check` 门禁。

---

## Git 操作记录

- 合并方式：`git merge --no-ff feature/task-002-sqlite-schema`（保留分支拓扑）
- **Merge commit hash：`14b6034`**
  - message：`merge(feature/task-002): 落地 SQLite schema（9 张表）+ rusqlite 连接 + user_version 迁移器 (#task-002)`
- 本地 feature 分支：已删除（`Deleted branch feature/task-002-sqlite-schema (was 3bf9fd0)`）
- 远端分支 `origin/feature/task-002-sqlite-schema`：保留未删除、未 push main（延续 task-001 约定，远端同步策略由维护者统一决定）。
- 未触发任何 `git revert`。

---

## 后续动作建议

1. **派发 task-003**：落地删除唯一入口 `engine::delete::execute()` + L3 红线白名单匹配器 + 单元测试（对 PRD P0-7 全部 L3 路径 100% 拦截）。这是产品生命线，需重点评审：白名单匹配器不能有路径遍历绕过（`..`、符号链接、UNC 路径）、删除前必须走保险箱、所有删除动作写 audit_log。
2. 维护者择机 `git push origin main` 同步远端，并清理 `origin/feature/task-001`、`origin/feature/task-002`。
3. task-003 立项时建议把「路径规范化（canonicalize）后再做白名单匹配」写入验收标准，防止 `C:\Windows\System32\..\Windows` 这类绕过。
