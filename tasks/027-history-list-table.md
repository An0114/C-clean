# 任务编号：027 前端历史列表——audit_log.list 接 React 表格（分页 / session 过滤）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFrontendReact
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-026 合并后写入）。
> 上游依赖：task-024（`audit_log.list` Tauri 命令已注册，入参 `{ session_id?, limit?, offset? }`，出参 `Vec<AuditLogEntry>`）、task-026（Dashboard 已就位，前端工程模式已跑通）。

## 任务目标

Dashboard 已经能看 4 个汇总数字，但「每次清理做了什么」还得翻 DB 才知道。本任务补历史明细页：

1. 新建 `app/src/pages/History.tsx`：
   - 启动时 `invoke('audit_log_list', { limit: 20, offset: 0 })`；
   - 渲染表格 4 列：时间（`created_at` unix → 本地时间字符串）、事件类型、路径、详情；
   - 分页：上一页 / 下一页（每页 20 条，offset 累加）；
   - 加载骨架 / 错误红条 + 重试；
2. App.tsx 用 `useState<'dashboard' | 'history'>` 加简单 tab 切换（顶部两个按钮），不引 React Router；
3. session 过滤先留个下拉占位（默认「全部」），真实 session_id 入参透传后端，但**不做 session 列表拉取**（后端没列 session 命令，留 task-028）。

**本任务不做**：
- 不做 scan.start / cleanup.execute 交互按钮（留 task-028+）；
- 不做 safe_box / rules 更新 UI；
- 不引 UI 库 / Router / 状态管理；
- 不写 Vitest。

## 前置条件

1. `main` 分支已含 task-001~026 全部产出；
2. 关键锚点：
   - `app/src/pages/Dashboard.tsx`（task-026 模式参考）；
   - `app/src/utils/formatBytes.ts`（task-026）；
   - `engine/src/audit_log/query.rs`（task-024）——AuditLogEntry 字段。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. 时间格式化工具

1. 新建 `app/src/utils/formatTimestamp.ts`：
   ```ts
   export function formatTs(unixSec: number): string;
   ```
2. 规则：`new Date(unixSec * 1000).toLocaleString('zh-CN', { hour12: false })`；unixSec 非法返 `"-"`。

### B. History 页面

3. 新建 `app/src/pages/History.tsx`：
   - state：`{ status: 'loading'|'ok'|'error', rows: AuditLogEntry[], offset: number, msg?: string }`；
   - `PAGE_SIZE = 20`；
   - 每次 offset 变化时重新 invoke；
   - 表格列：时间 / event_type / path / detail（detail 空时显示 `-`）；
   - 分页栏：「上一页」按钮（offset=0 时禁用）+ 「下一页」按钮（返回行数 < PAGE_SIZE 时禁用）+ 当前偏移提示；
   - 加载骨架 / 错误红条 + 重试。
4. 类型：
   ```ts
   interface AuditLogEntry {
     id: number;
     session_id: number;
     event_type: string;
     path: string;
     detail: string | null;
     created_at: number;
   }
   ```

### C. App.tsx tab 切换

5. `App.tsx`：
   - `const [tab, setTab] = useState<'dashboard'|'history'>('dashboard');`
   - 顶部两个按钮：「仪表盘」「历史」；
   - 切到对应 tab 渲染 Dashboard 或 History。

### D. 卫生与边界

6. 不修改 Rust 代码；
7. 不新增 npm 依赖；
8. 不写路由。

## 允许修改范围

- `app/src/utils/formatTimestamp.ts`（新建）
- `app/src/pages/History.tsx`（新建）
- `app/src/App.tsx`（加 tab）
- `docs/PROJECT_STATE.md`（任务收尾）

## Git 分支

`feature/task-027-history-list-table`，从 main 拉出。

## Git 提交要求

1. `feat(frontend): 新增 formatTs 时间格式化工具 (#task-027)`
2. `feat(frontend): 新增 History 页面——audit_log.list 表格 + 分页 (#task-027)`
3. `feat(frontend): App.tsx 加 Dashboard/History tab 切换 (#task-027)`
4. `docs(state): 任务027完成，历史明细页就绪 (#task-027)` —— 单独提交，只动 PROJECT_STATE.md，「下一步计划」改为「派发 task-028：scan.start 交互——前端开始扫描按钮 + 进度条 listen」。

**自验**：`pnpm --filter app build` 退出码 0；tauri dev 真机切 tab 看到表格。

## 输出要求

1. 3 个 feat commit + 1 个 docs(state) commit；
2. 任务末尾追加「## 实现备注」段（自验输出、截图文字描述、偏差、遗留）；
3. PROJECT_STATE.md 更新。
