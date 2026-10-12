# 测试评审报告

## 本次任务名称

014 cleanup.dryRun 切换——MockResolver → DbItemPathResolver，真实 item 走白名单

## Git 提交记录核对结果

✅ 5 个提交全部关联 `#task-014`，type 前缀规范（docs / feat），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `f9eaa17` | docs(task) | tasks/014-dryrun-db-resolver.md | ✅ |
| 2 | `0c32251` | feat(tauri) | app/src-tauri/src/commands.rs | ✅ |
| 3 | `3af5bbb` | docs(state) | docs/PROJECT_STATE.md | ✅ |
| 4 | `5ba77fb` | docs(task) | tasks/014-dryrun-db-resolver.md | ✅ |
| 5 | `be99619` | docs(state) | docs/PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. cleanup_dry_run 命令改造

- ✅ 删除 `MockResolver` import（第 7 行）。
- ✅ 保留 `load_external_deny()`（task-010 fail-closed 加载）。
- ✅ 新增 `open(&data_root)` 打开真实 DB（与 scan_start 同一 data_dir）。
- ✅ 构造 `DbItemPathResolver::new(&conn)`。
- ✅ 删除 `MockResolver::new(vec![])`。
- ✅ 调 `handle_dry_run(req, &resolver, &external_deny)`。
- ✅ handle_dry_run 签名未改（task-010 定型）。
- ✅ DbItemPathResolver 实现未改（task-009 就绪）。

### B. 输入输出契约

- ✅ 入参 `itemIds: Vec<i64>` 不变。
- ✅ 返回 `Envelope<DryRunResp>` 不变。
- ✅ 查不到的 id → resolver 跳过（task-009 单测已覆盖）。

### C. 端到端验证

- ⚠️ DevAgent 未跑 `pnpm tauri dev` GUI（与 task-007/013 已知问题一致）；engine 层 task-009 已覆盖「DB 有 item → dryRun 返回真实 path / 未知 id 跳过 / 2002 拒 L3」，本任务是 Tauri 接线薄包装。**可接受。**

### D. 卫生与边界

- ✅ 未修改 engine/src/ipc/commands/dry_run.rs。
- ✅ 未修改 engine/src/db/resolver.rs。
- ✅ 未修改 rules/**、scan/**、execute/**。
- ✅ 未修改 app/src/**（前端 React）。
- ✅ 未新增依赖。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 95 passed / 0 failed（无回退） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0 |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **每次 cleanup_dry_run 都 open 新 DB 连接**：与 scan_start 一致，每次调用开新连接。SQLite 多连接读没问题；M1 可在 Tauri State 里缓存连接。**第三档。**
2. **每次 cleanup_dry_run 都重新 load_external_deny()**：重复读盘 + 算 SHA-256。当前调用频率低，无性能问题。**第三档。**
3. **data_dir 硬编码 current_dir()**：与 task-013 一致，打包后改 app_data_dir。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ dryRun 仍走 task-003 白名单 + task-010 外部红线；resolver 从 DB 取真实 path，不接受前端传 path。 |
| A03 注入 | ✅ resolver 内部 SQL 参数化；item_ids 是 i64，不拼字符串。 |
| A04 不安全设计 | ✅ rules 加载失败仍 fail-closed 返 5001；查不到的 id 静默跳过。 |
| A08 数据完整性 | ✅ path 来自 DB（scan_start 写入），前端无法伪造 path。 |

### 专项安全分析

- **前端能不能伪造 path？** 不能。前端只传 itemIds（i64），path 由 DbItemPathResolver 从 DB 反查。✅
- **查不到的 id 会怎样？** resolver.resolve() 过滤掉，不报错、不返回到 items 数组。task-009 单测覆盖。✅
- **rules 加载失败？** 仍返 5001 INTERNAL，不静默放行。✅
- **L3 路径会不会被 dryRun 放过？** 不会。handle_dry_run 仍走 classify_with_external（内置 L3 ∪ 外部红线）。✅

## 评审结论：**通过**

cleanup.dryRun 从 MockResolver（空 Vec 永远查不到）切换到 DbItemPathResolver（真实 DB 反查）；95 测试无回退；clippy/fmt/pnpm build 全过；5 个提交全部关联 #task-014 无混杂；dryRun 闭环真实数据。

## Git 操作记录

- **merge commit**：`03561de`（`git merge --no-ff feature/task-014-dryrun-db-resolver`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-014-dryrun-db-resolver`，was be99619）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务014评审报告 (#task-014)`

## 后续动作建议

1. **task-015**：cleanup.execute 真实删除——dryRun 之后用户确认，调 `engine::execute::delete::execute()` 走白名单 + Deleter trait。
2. **后续**：scan.listItems 查询接口（前端列出可清理项）；前端勾选 UI；打包时 data_dir 改 app_data_dir；Tauri State 缓存 DB 连接和 rules。
