# 测试评审报告

## 本次任务名称

029 清理闭环——cleanup.listPending 后端命令 + Items 表格 checkbox 清理按钮

## Git 提交记录核对结果

✅ 11 个提交全部关联 `#task-029`，type 前缀规范（docs / feat(execute/tauri/frontend) / test / style），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `5979f14` | docs(task) | tasks/029-cleanup-items-ui.md | ✅ |
| 2 | `48ba875` | feat(execute) | list_pending.rs + mod.rs | ✅ |
| 3 | `976acdb` | feat(tauri) | commands.rs + lib.rs | ✅ |
| 4 | `e223fc6` | test(execute) | list_pending_items.rs | ✅ |
| 5 | `a4be607` | feat(frontend) | Items.tsx | ✅ |
| 6 | `94fb43d` | feat(frontend) | App.tsx | ✅ |
| 7 | `ef07dea` | style(execute) | list_pending.rs（clippy clamp） | ✅ |
| 8 | `ca81b00` | docs(state) | PROJECT_STATE.md | ✅ |
| 9 | `a1f732b` | docs(task) | tasks/029-cleanup-items-ui.md | ✅ |
| 10 | `19edb86` | docs(state) | PROJECT_STATE.md | ✅ |
| 11 | `8c705cc` | style(execute) | list_pending_items.rs（fmt 对齐） | ✅ |

## 分步验证清单

### A. 后端：list_pending_items

- ✅ PendingItem { id, path, size_bytes, module_code, session_id }（serde camelCase）。
- ✅ list_pending_items(conn, session_id, limit, offset)。
- ✅ SQL：SELECT ... FROM cleanup_item WHERE (?1 IS NULL OR session_id = ?1) ORDER BY size_bytes DESC LIMIT ? OFFSET ?。
- ✅ 参数化（params!）。
- ✅ limit.clamp(1, 500)、offset.max(0) 防御。
- ✅ cleanup_list_pending Tauri 命令，入参全可选（limit 默认 100）。
- ✅ lib.rs generate_handler 注册。

### B. 后端单测（list_pending_items.rs）

- ✅ list_pending_filters_status_pending（2 pending + 1 deleted → 只返 2）。
- ✅ list_pending_filters_by_session（session=1 / session=2 → 过滤只返 1）。
- ✅ list_pending_orders_by_size_desc（100/300/200 → 300/200/100）。
- ✅ list_pending_pagination（limit/offset 分页）。

### C. 前端：Items 页面

- ✅ invoke('cleanup_list_pending', { limit: 100 })。
- ✅ state：rows / selected: Set<number> / cleaning / result。
- ✅ 表格 5 列：checkbox / 路径 / 大小（formatBytes）/ 模块 / Session。
- ✅ 全选 checkbox。
- ✅ 底部操作栏：「已选 N 项，共 X MB」+「清理已选」按钮（selected 空时 disabled）+「刷新」。
- ✅ 清理中按钮禁用 + 文案「清理中…」。
- ✅ 清理完成：绿色成功条（「已清理 N 项」）或红色错误条，自动重拉列表。
- ✅ cleanup_execute invoke({ itemIds: ids })。

### D. App.tsx 第三 tab

- ✅ tab 加「清理」，渲染 <Items />。
- ✅ 三个 tab：仪表盘 / 清理 / 历史。

### E. 卫生与边界

- ✅ 不修改 executor.rs / fs_deleter.rs / safe_box.rs 已有逻辑。
- ✅ 不接 cleanup.dryRun UI。
- ✅ 不新增 npm 依赖。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 137 passed / 0 failed / 1 ignored（133 已有 + 4 list_pending 新增） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0（Vite + tsc 通过） |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **cleanup_item 无 status 列**：任务书假设是 pending/deleted 行级表，实际是扫描写入的所有行。「pending」= 表内所有行，无 WHERE status='pending' 过滤。DevAgent 已适配。**第二档口径，已登记。**
2. **cleanup_item 无 scanned_at 列**：PendingItem 去掉该字段，前端不显示时间列。**第二档口径，已登记。**
3. **清理后行不删**：cleanup.execute 不删 cleanup_item 行（task-015 边界），Items 页刷新后仍列出已清理项。**已知限制，第三档。**
4. **无 dry-run 预览 UI**：任务书明确不做。**第三档。**
5. **无 safe_box restore UI**：留 task-031。**第三档。**
6. **tauri dev 真机未跑**：自动化环境无 GUI。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ list_pending 是只读查询；cleanup_execute 是 task-015 已注册命令，有 L3 白名单二次校验。 |
| A03 注入 | ✅ SQL 参数化（params!）；session_id 用占位符；前端 invoke 参数固定。 |
| A04 不安全设计 | ✅ limit.clamp(1,500) 防异常分页；cleaning 时按钮禁用防重复；selected 空时清理按钮 disabled。 |
| A08 数据完整性 | ✅ list_pending 只读不写；cleanup_execute 走 task-015 的 L3 白名单 + 保险箱流程。 |

### 专项安全分析

- **会不会被 SQL 注入？** 不会。所有参数用 `params!` 占位符。✅
- **会不会误删 L3 白名单外文件？** 不会。cleanup_execute 走 task-015 的 L3 二次校验 + 保险箱 rename。✅
- **会不会重复触发清理？** 不会。cleaning 时按钮 disabled。✅
- **清理后会不会崩溃？** 不会。try/catch 捕获，error 态展示。✅
- **checkbox 会不会越权选？** 不会。用户自己勾选，item_ids 传后端反查路径。✅

## 评审结论：**通过**

M1 业务闭环打通：list_pending_items 后端命令（size DESC + session 过滤 + limit/offset 防御）+ Items 前端表格（checkbox 多选 + 已选统计 + 清理已选按钮 + 成功/失败提示 + 自动重拉）+ App.tsx 第三 tab；137 测试全绿（含 4 个新增 list_pending 测试）；clippy/fmt/pnpm build 全过；11 个提交全部关联 #task-029 无混杂。从「能扫描 + 能看进度」升级为「能看扫描结果 + 能勾选 + 能真正删除 + 数字自动刷新」。

## Git 操作记录

- **merge commit**：`3b2a96e`（`git merge --no-ff feature/task-029-cleanup-items-ui`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-029-cleanup-items-ui`，was 8c705cc）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务029评审报告 (#task-029)`

## 后续动作建议

1. **task-030**：dry-run 预览 UI（cleanup.dryRun 前端接入）。
2. **task-031**：safe_box restore UI（保险箱列表 + 恢复按钮）。
3. **后续**：rules 更新 UI、取消扫描命令、打包时 data_dir 改 app_data_dir。
