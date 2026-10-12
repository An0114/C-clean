# 测试评审报告

## 本次任务名称

009 DB 版 ItemPathResolver——cleanup_item 表按 id 取路径/大小/模块，替换 MockResolver

## Git 提交记录核对结果

✅ 6 个提交全部关联 `#task-009`，type 前缀规范（docs / feat / test），无混杂提交，无越权改范围外文件。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `0083839` | docs(task) | tasks/009-db-item-resolver.md | ✅ |
| 2 | `9ec8224` | feat(db) | engine/src/db/mod.rs + resolver.rs | ✅ |
| 3 | `47b542f` | test(db) | engine/tests/db_resolver.rs | ✅ |
| 4 | `6bc9fb2` | docs(state) | docs/PROJECT_STATE.md | ✅ |
| 5 | `fe73eac` | docs(task) | tasks/009-db-item-resolver.md | ✅ |
| 6 | `0a95a59` | docs(state) | docs/PROJECT_STATE.md | ✅ |

无偏差。

## 分步验证清单

### A. 文件落点与结构

- ✅ 新建 `engine/src/db/resolver.rs`，`mod.rs` 加 `pub mod resolver;`。
- ✅ `DbItemPathResolver<'conn> { conn: &'conn Connection }` 生命周期正确。
- ✅ `new(conn)` 构造函数。
- ✅ `impl ItemPathResolver for DbItemPathResolver` trait 落具体类型。

### B. 查询逻辑

- ✅ SQL 参数化（`WHERE id = ?1` + `params![id]`），无字符串拼接 ids。
- ✅ prepare 失败 eprintln + 返回空 Vec，不 panic。
- ✅ 逐 id 查询，`QueryReturnedNoRows` 跳过不报错；其他 SQL 错误 eprintln 后继续下一个 id。
- ✅ `size_bytes` INTEGER(i64) → `max(0) as u64` 对齐 ResolvedItem。
- ✅ 不 JOIN scan_session。

### C. 单测

- ✅ `resolver_returns_empty_when_no_items`：空 DB 返回空 Vec。
- ✅ `resolver_returns_matching_items`：INSERT 3 行，resolve([10,12]) 返回 2 条，字段对齐。
- ✅ `resolver_skips_unknown_ids`：INSERT id=10，resolve([10,999]) 只返回 1 条。
- ✅ `resolver_dry_run_end_to_end`：L3 Desktop 条目被拒，outcome=2002，blocked_by_l3 含 item_id=2 且 reason 含 L3PersonalDir。
- ✅ 全部 tempfile，不碰真实用户目录。

### D. 卫生与边界

- ✅ 未改 conn.rs / migrate.rs / v1__init.sql。
- ✅ 未改 dry_run.rs 的 trait / MockResolver / handle_dry_run 签名。
- ✅ 未改 app/、elevation/、rules/、scripts/。
- ✅ 未新增依赖。
- ✅ 未在 Tauri command 里替换 MockResolver（留 M1 接线）。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 77 passed / 0 failed（13 lib + 4 db_resolver + 8 db_schema + 7 execute_resolve + 24 execute_whitelist + 7 ipc_commands + 3 envelope + 10 rules_loader + 1 smoke） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| task-002/004 已有测试 | ✅ 无回退（8 db_schema + 7 ipc_commands + 3 envelope 全过） |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **DB 错误静默吞掉**（resolver.rs L31-34, L53）：prepare/query 失败仅 eprintln，返回空 Vec。trait 签名无 Result，这是任务文档允许的取舍。但生产环境 eprintln 不会被日志系统收集。建议 M1 接线时把 eprintln 换成项目的日志 facade。**第三档。**
2. **逐 id 查询而非 IN 批量**：每个 id 一次 query_row。对少量 item_ids（dryRun 场景 typically < 20）无性能问题；M1 扫描器如果一次 resolve 成百上千 id，可改为 `IN (?, ?, ...)` + `params_from_iter`。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A03 注入 | ✅ SQL 全参数化（`?1` + `params![]`），无字符串拼接 ids。 |
| A04 不安全设计 | ✅ DB 错误不 panic、不 propagrate 到主流程；查不到的 id 跳过而非报错。 |
| A08 数据完整性 | ✅ 只读 SELECT，无写操作；不修改 schema。 |

### 专项安全分析

- **SQL 注入**：id 是 i64 类型，Rust 类型系统保证不是字符串注入面；SQL 用 `?1` 占位符。✅
- **路径越权**：resolver 只从 DB 取路径字符串，不直接操盘；路径交给 task-003/008 的 normalize+resolve+classify 管线判定。✅
- **Tauri 命令未切换**：当前 Tauri command 仍用 MockResolver，DbItemPathResolver 仅在单测中被验证。M1 接线时需确保切换后 L3 拦截路径与单测一致。✅

## 评审结论：**通过**

DB resolver 实现干净，SQL 参数化，4 条集成测试全过（含 dryRun 端到端 L3 拦截）；77 测试无回退，clippy/fmt exit 0；6 个提交均关联 #task-009 无混杂。Tauri command 切换留 M1，符合任务边界。

## Git 操作记录

- **merge commit**：`17c5d05`（`git merge --no-ff feature/task-009-db-item-resolver`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-009-db-item-resolver`，was 0a95a59）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务009评审报告 (#task-009)`

## 后续动作建议

1. **task-010**：外部红线集注入——rules JSON 加载后与内置 L3 红线取并集，接入 cleanup.dryRun。
2. **M1 A 模块**：真实扫描器写入 cleanup_item 后，在 Tauri command 里把 MockResolver 切换为 DbItemPathResolver。
3. **日志**：把 eprintln 换成项目日志 facade。
