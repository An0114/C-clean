# 测试评审报告

## 本次任务名称

025 audit_log.stats——清理结果聚合（总释放字节 / 成功数 / 失败数 / session 数）

## Git 提交记录核对结果

✅ 7 个提交全部关联 `#task-025`，type 前缀规范（docs / feat / test），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `8fa4546` | docs(task) | tasks/025-audit-log-stats.md | ✅ |
| 2 | `9f0c0c5` | feat(audit_log) | mod.rs + stats.rs | ✅ |
| 3 | `7052e14` | feat(tauri) | commands.rs + lib.rs | ✅ |
| 4 | `d6a9fa1` | test(audit_log) | audit_log_stats.rs | ✅ |
| 5 | `b741579` | docs(state) | PROJECT_STATE.md | ✅ |
| 6 | `0516bee` | docs(task) | tasks/025-audit-log-stats.md | ✅ |
| 7 | `270a83c` | docs(state) | PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. Engine 层：cleanup_stats

- ✅ CleanupStats { total_bytes_freed, success_count, failed_count, session_count }。
- ✅ cleanup_stats(conn, session_id: Option<i64>)。
- ✅ 单条 SQL 一次拿 4 个聚合值：COALESCE(SUM(freed_bytes),0) / SUM(success_count) / SUM(fail_count) / COUNT(DISTINCT session_id)。
- ✅ session_id 有则加 WHERE，无则全表。
- ✅ COALESCE 防空表 NULL。
- ✅ 参数化（params!）。

### B. Tauri audit_log_stats 命令

- ✅ 入参 session_id: Option<i64>。
- ✅ DB 错误返 INTERNAL。
- ✅ lib.rs generate_handler 注册。

### C. 单测（audit_log_stats.rs）

- ✅ stats_empty_returns_zero：空表 → 全 0。
- ✅ stats_counts_success_failed：3 行（freed 100+200+300，success 3，fail 2）→ total=600/success=3/failed=2/session=1。
- ✅ stats_filters_by_session：session=1 (success 2) + session=2 (success 1) → filter session=1 返回 success=2/session=1。
- ✅ stats_distinct_session_count：3 个不同 session → session_count=3。
- ✅ 测试前置：先 INSERT scan_session（FK 约束）。

### D. 卫生与边界

- ✅ 不修改 query.rs 已有逻辑。
- ✅ 不 JOIN cleanup_item（module 维度留 task-026）。
- ✅ 不修改前端 React。
- ✅ 不新增依赖。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 133 passed / 0 failed / 1 ignored（129 已有 + 4 audit_log_stats 新增） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0 |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **cleanup_task schema 偏差**：任务书假设是行级 success/failed 表（status 列），实际是「一次清理任务」级聚合行（freed_bytes/success_count/fail_count 整数列）。DevAgent 已适配为 SUM(freed_bytes) / SUM(success_count) / SUM(fail_count)。**第二档口径，已登记。**
2. **无 module 维度拆分**：需 JOIN cleanup_item，留 task-026。**第三档。**
3. **无时间区间过滤**：今天/本周，留后续。**第三档。**
4. **无前端图表**：留后续。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ 只读聚合查询，不修改 cleanup_task；无删除/清空能力。 |
| A03 注入 | ✅ SQL 参数化（params!）；session_id 用占位符；无字符串拼接。 |
| A04 不安全设计 | ✅ COALESCE 防空表 NULL，返全 0 不崩。 |
| A08 数据完整性 | ✅ 不写 cleanup_task，只聚合；SUM/COUNT 不篡改原始数据。 |

### 专项安全分析

- **会不会被 SQL 注入？** 不会。所有参数用 `params!` 占位符。✅
- **空表会不会崩溃？** 不会。COALESCE 返全 0。✅
- **session_id 过滤会不会越权？** 不会。session_id 是用户自己的清理会话，前端历史页要展示。✅
- **聚合数字会不会被篡改？** 不会。只读 SELECT，不写库。✅

## 评审结论：**通过**

cleanup_stats 聚合查询落地：单条 SQL 一次拿 4 个聚合值（SUM(freed_bytes) / SUM(success_count) / SUM(fail_count) / COUNT(DISTINCT session_id)），COALESCE 防空表 NULL，session_id 可选过滤；Tauri 命令 audit_log_stats 注册；133 测试全绿（含 4 个新增 audit_log_stats 测试：空/计数/session 过滤/去重）；clippy/fmt/pnpm build 全过；7 个提交全部关联 #task-025 无混杂。M1-F 报表基础（list + stats）就绪。

## Git 操作记录

- **merge commit**：`748618e`（`git merge --no-ff feature/task-025-audit-log-stats`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-025-audit-log-stats`，was 270a83c）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务025评审报告 (#task-025)`

## 后续动作建议

1. **前端 UI 接入**：React 历史页（audit_log.list）+ 仪表盘卡片（audit_log.stats）。
2. **task-026**：module 维度拆分（JOIN cleanup_item）、时间区间过滤。
3. **M1 收尾**：scan.listItems 命令、前端勾选 UI、打包时 data_dir 改 app_data_dir。
