# 测试评审报告

## 本次任务名称

024 audit_log.list——查询清理历史（ts DESC 分页、limit/offset 防御）

## Git 提交记录核对结果

✅ 9 个提交全部关联 `#task-024`，type 前缀规范（docs / feat / test / style），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `0d00d42` | docs(task) | tasks/024-audit-log-list.md | ✅ |
| 2 | `79677da` | feat(audit_log) | mod.rs + query.rs + lib.rs | ✅ |
| 3 | `701caf8` | feat(tauri) | commands.rs + lib.rs | ✅ |
| 4 | `dd24e78` | test(audit_log) | audit_log_query.rs | ✅ |
| 5 | `e7dcbf8` | style(audit_log) | query.rs + audit_log_query.rs（fmt） | ✅ |
| 6 | `7e27f4b` | docs(state) | PROJECT_STATE.md | ✅ |
| 7 | `8f1bfed` | docs(task) | tasks/024-audit-log-list.md | ✅ |
| 8 | `09f95a2` | docs(state) | PROJECT_STATE.md | ✅ |
| 9 | `e368e62` | docs(state) | PROJECT_STATE.md（session_id 方案二口径更新） | ✅ |

## 分步验证清单

### A. Engine 层：audit_log/query.rs

- ✅ AuditLogEntry { id, event_type, path, detail, created_at, result }（对外业务语义命名）。
- ✅ AuditLogFilter { session_id: Option<i64>, limit: i64, offset: i64 }。
- ✅ list_audit_log：`SELECT id, action, target, detail, ts, result FROM audit_log ORDER BY ts DESC, id DESC LIMIT ? OFFSET ?`。
- ✅ 参数化（params!），无字符串拼接。
- ✅ limit<=0 兜底 100、offset<0 兜底 0。
- ✅ 列名映射：action→event_type、target→path、ts→created_at（真实 schema 偏差，DevAgent 已记录）。

### B. Tauri audit_log_list 命令

- ✅ 入参 session_id: Option<i64>, limit: Option<i64>, offset: Option<i64>。
- ✅ limit 默认 100、offset 默认 0。
- ✅ DB 错误返 INTERNAL。
- ✅ lib.rs generate_handler 注册。

### C. 单测（audit_log_query.rs）

- ✅ list_empty_returns_empty：空 DB → 空 Vec。
- ✅ list_all_returns_desc_order：5 行 ts 递增 → 返回按 ts DESC，最新在前。
- ✅ list_by_action_filters：5 行混合 action → 全量返回 5 条（session_id 未启用，验证 action 字段透传）。
- ✅ list_pagination_limit_offset：10 行 → limit=3 offset=0 返回前 3 条、offset=3 返回接下来 3 条。

### D. 卫生与边界

- ✅ 不修改 db/execute/rules/scan/ipc 已有逻辑。
- ✅ 不做聚合统计（SUM/GROUP BY）。
- ✅ 不修改前端 React。
- ✅ 不新增依赖。

### E. session_id 过滤（方案二）

- ✅ 入参保留 `session_id: Option<i64>`，当前 schema 无对应列，**不进 WHERE**。
- ✅ 代码注释明确标记"预留未启用，待后续 schema 加列后启用"。
- ✅ PROJECT_STATE.md 口径已更新（用户拍板方案二）。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 129 passed / 0 failed / 1 ignored（125 已有 + 4 audit_log_query 新增） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0 |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **session_id 过滤未实现**：真实 schema 无 session_id 列，入参保留但不进 WHERE。**用户已拍板方案二（入参保留标记预留未启用）**，待后续 schema 加列后启用。**第二档口径，已登记。**
2. **列名映射偏差**：任务书要求 event_type/path/created_at，真实 schema 是 action/target/ts。DevAgent 已做对外业务语义命名映射，前端无感。**第二档口径，已登记。**
3. **无 action/result 过滤**：当前只支持全量查询 + 分页，不支持 WHERE action=... 或 WHERE result=...。任务书未要求，留 task-025。**第三档。**
4. **无导出 CSV/JSON**：任务书明确不做。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ 只读查询，不修改 audit_log；无删除/清空能力。 |
| A03 注入 | ✅ SQL 参数化（params!），limit/offset 用占位符；无字符串拼接。 |
| A04 不安全设计 | ✅ limit<=0 兜底 100、offset<0 兜底 0，防异常分页。 |
| A08 数据完整性 | ✅ 不写 audit_log，只查；created_at/result 透传不篡改。 |

### 专项安全分析

- **会不会被 SQL 注入？** 不会。所有参数用 `params!` 占位符。✅
- **会不会一次拉全表？** 不会。LIMIT 默认 100，limit<=0 兜底 100。✅
- **会不会泄露敏感路径？** audit_log 记录的是清理动作的 target 路径，本来就是用户自己的操作历史，前端历史页要展示。✅
- **session_id 未启用会不会被前端误判？** 不会。入参保留但忽略，前端传了也不报错；代码注释 + PROJECT_STATE 口径已标记"预留未启用"。✅

## 评审结论：**通过**

audit_log.list 只读查询命令落地：engine 层 list_audit_log 按 ts DESC/id DESC 分页返回，limit/offset 防御（<=0 兜底）；列名映射（action→event_type、target→path、ts→created_at）对外业务语义命名；Tauri 命令 audit_log_list 注册；129 测试全绿（含 4 个新增 audit_log_query 测试：空/倒序/action 透传/分页）；clippy/fmt/pnpm build 全过；9 个提交全部关联 #task-024 无混杂。M1-F 报表后端基础就绪。

## Git 操作记录

- **merge commit**：`70e6285`（`git merge --no-ff feature/task-024-audit-log-list`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-024-audit-log-list`，was e368e62）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务024评审报告 (#task-024)`

## 后续动作建议

1. **task-025**：audit_log.stats 聚合——按 action/result 汇总 total_bytes、total_items、success/fail 计数。
2. **后续**：session_id 列加入 schema 后启用过滤；action/result WHERE 过滤；导出 CSV/JSON；前端 React 历史页 UI。
3. **M1 收尾**：scan.listItems 命令、前端勾选 UI、打包时 data_dir 改 app_data_dir。
