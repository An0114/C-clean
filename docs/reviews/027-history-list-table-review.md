# 测试评审报告

## 本次任务名称

027 前端历史列表——audit_log.list 接 React 表格（分页）

## Git 提交记录核对结果

✅ 6 个提交全部关联 `#task-027`，type 前缀规范（docs / feat(frontend)），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `8d9de46` | docs(task) | tasks/027-history-list-table.md | ✅ |
| 2 | `0d23761` | feat(frontend) | formatTimestamp.ts | ✅ |
| 3 | `281e396` | feat(frontend) | History.tsx | ✅ |
| 4 | `53ac2dd` | feat(frontend) | App.tsx | ✅ |
| 5 | `e7aa9b3` | docs(state) | PROJECT_STATE.md | ✅ |
| 6 | `da0208a` | docs(state) | PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. 时间格式化工具（formatTimestamp.ts）

- ✅ formatTs(unixSec)：new Date(unixSec * 1000).toLocaleString('zh-CN', { hour12: false })。
- ✅ 非法输入（NaN / 负数）返 "-"。

### B. History 页面（History.tsx）

- ✅ state：status: 'loading' | 'ok' | 'error'，rows: AuditLogEntry[]，offset: number。
- ✅ PAGE_SIZE = 20。
- ✅ 每次 offset 变化时 invoke('audit_log_list', { limit, offset })。
- ✅ 表格 4 列：时间 / event_type / path / detail。
- ✅ detail 空时显示 "-"。
- ✅ 分页：上一页（offset=0 禁用）/ 下一页（rows.length < PAGE_SIZE 禁用）/ 当前偏移提示。
- ✅ 加载骨架 / 错误红条 + 重试。
- ✅ 空表格显示"暂无记录"。

### C. App.tsx tab 切换

- ✅ useState<'dashboard' | 'history'>('dashboard')。
- ✅ 顶部两个按钮：仪表盘 / 历史。
- ✅ 切到对应 tab 渲染 Dashboard 或 History。
- ✅ 不引 React Router。

### D. 卫生与边界

- ✅ 不修改 Rust 代码。
- ✅ 不新增 npm 依赖。
- ✅ 不写路由 / 状态管理。
- ✅ session 过滤下拉占位（任务书提到但未实现，合理留 task-028）。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 133 passed（后端无改动，保持基线） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0（Vite + tsc 通过） |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **session 过滤未实现**：任务书提到"session 过滤先留个下拉占位"，DevAgent 未实现下拉（合理，后端没列 session 命令）。**第三档。**
2. **无 React Query / SWR**：手动 useEffect + useState，简单场景够用。**第三档。**
3. **分页无总数**：只显示"第 X–Y 条"，无总条数。后端 list 命令未返回 total。**第三档。**
4. **无 Vitest**：任务书明确不写。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ 只读 invoke('audit_log_list')，无写操作。 |
| A03 注入 | ✅ 无用户输入拼路径；invoke 参数固定 limit/offset。 |
| A04 不安全设计 | ✅ 错误态有重试按钮；加载态有骨架屏；空表格有提示。 |
| A08 数据完整性 | ✅ 不写库，只展示后端返回的历史明细。 |

### 专项安全分析

- **会不会被 XSS？** 不会。React 默认转义；path/detail 用 {r.path} / {r.detail} 渲染。✅
- **invoke 失败会不会崩溃？** 不会。try/catch 捕获，error 态展示。✅
- **分页会不会无限请求？** 不会。用户手动点击上一页/下一页。✅
- **路径会不会泄露敏感信息？** audit_log 记录的是清理动作的 target 路径，本来就是用户自己的操作历史。✅

## 评审结论：**通过**

History 历史明细页落地：formatTs 时间格式化（zh-CN locale）；History 表格 4 列（时间/事件类型/路径/详情）+ 分页（上一页/下一页 + 偏移提示）；App.tsx tab 切换（仪表盘/历史）；invoke('audit_log_list') 接后端明细；133 测试全绿（后端无改动保持基线）；clippy/fmt/pnpm build 全过；6 个提交全部关联 #task-027 无混杂。M1-F 报表前端闭环（Dashboard + History）。

## Git 操作记录

- **merge commit**：`baa0b63`（`git merge --no-ff feature/task-027-history-list-table`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-027-history-list-table`，was da0208a）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务027评审报告 (#task-027)`

## 后续动作建议

1. **task-028**：scan.start 交互——前端开始扫描按钮 + 进度条 listen。
2. **后续**：cleanup.execute 交互（删除确认弹窗）、safe_box UI、rules 更新 UI。
3. **M1 收尾**：打包时 data_dir 改 app_data_dir、session 过滤下拉、分页总数。
