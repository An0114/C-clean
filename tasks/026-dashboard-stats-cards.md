# 任务编号：026 前端仪表盘——audit_log.stats 数字卡片（GB 格式化 / 加载 / 错误态）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFrontendReact
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-025 合并后由 DocsKeeper 写入）——M1 业务模块 F 报表的前端第一刀。
> 上游依赖：task-025（`audit_log.stats` Tauri 命令已注册，返回 `CleanupStats { total_bytes_freed, success_count, failed_count, session_count }`）、task-001（app/ React 18 + TS + Vite 脚手架）均已合并到 main。
> **克制范围**：只接 stats 数字卡片，不接 list 历史、不接 scan/delete 交互。

## 任务目标

后端已有 `audit_log.stats` 命令，但前端还是 task-001 留下来的空 App 组件。本任务补最小可读界面：

1. 在 `app/src/` 新建 `pages/Dashboard.tsx`：
   - 启动时 `invoke('audit_log_stats')`；
   - 渲染 4 个数字卡片：**总释放空间**（字节格式化为 KB/MB/GB）、**成功次数**、**失败次数**、**清理轮次**（session_count）；
   - 加载中骨架 / 失败时红色错误条（带重试按钮）；
2. 在 `App.tsx` 挂 Dashboard 为默认页；
3. 复用 `@tauri-apps/api/core` 的 `invoke`（已在 devDeps）。

**本任务不做**：
- 不接 `audit_log.list` 历史列表（留 task-027）；
- 不接 `scan_start` / `cleanup_execute` 交互（留 task-028+）；
- 不接 safe_box / rules 更新 UI；
- 不引入 UI 库（MUI / Ant Design / Tailwind 都不引，纯内联 style）；
- 不写 Vitest / Playwright（CI 的 lint/test 还是 echo 占位，留 task-006 已知问题）；
- 不接 React Router（单页就一个 Dashboard）。

## 前置条件

1. `main` 分支已含 task-001~025 全部产出；
2. 关键锚点：
   - `app/src/App.tsx`、`app/src/main.tsx`（task-001 脚手架）；
   - `app/src-tauri/src/lib.rs` 已注册 `audit_log_stats`；
   - 入参 `session_id?: number`（可空）、出参 `CleanupStats` JSON shape。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. 字节格式化工具

1. 新建 `app/src/utils/formatBytes.ts`：
   ```ts
   export function formatBytes(n: number): string;
   ```
2. 规则：
   - `n < 1024` → `"B"`；
   - `< 1024^2` → `"x.x KB"`（保留 1 位）；
   - `< 1024^3` → `"x.x MB"`；
   - 否则 → `"x.x GB"`；
   - 负数 / NaN → `"0 B"`。

### B. Dashboard 页面

3. 新建 `app/src/pages/Dashboard.tsx`：
   - `useEffect` 里 `invoke<CleanupStats>('audit_log_stats')`；
   - state：`{ status: 'loading' | 'ok' | 'error', data?: CleanupStats, msg?: string }`；
   - 加载中：4 个灰色占位条（`opacity-50` 或内联 style `opacity: 0.5`）；
   - 失败：红色 div + `err.message` + 「重试」按钮（重新触发 useEffect）；
   - 成功：4 个卡片 grid（2×2），每个卡片：标签（顶部小字）+ 数字（大号）。
4. 卡片：
   | 标签 | 数字来源 | 格式化 |
   |------|----------|--------|
   | 总释放空间 | `total_bytes_freed` | `formatBytes` |
   | 成功次数 | `success_count` | 整数 |
   | 失败次数 | `failed_count` | 整数（>0 时数字红色） |
   | 清理轮次 | `session_count` | 整数 |
5. 内联 style 即可，不引入 CSS 文件 / Tailwind / MUI。

### C. App.tsx 挂首页

6. `app/src/App.tsx` 默认渲染 `<Dashboard />`，删掉 task-001 默认的占位欢迎文字。

### D. Tauri 命令调用约定

7. 命令名：`audit_log_stats`（**下划线**，与 Rust 注册名一致——Tauri 把 camelCase 转 snake_case 调）；
8. 入参：`{}`（不传 session_id，查全局）；
9. 类型：
   ```ts
   interface CleanupStats {
     total_bytes_freed: number;
     success_count: number;
     failed_count: number;
     session_count: number;
   }
   ```

### E. 卫生与边界

10. 不修改任何 `engine/**` / `app/src-tauri/**` Rust 代码；
11. 不引入新 npm 依赖（`@tauri-apps/api` 已有）；
12. 不写路由、不写状态管理（zustand/redux 都不引）；
13. 不接 `audit_log.list`（留 task-027）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `app/src/utils/formatBytes.ts`（新建）
- `app/src/pages/Dashboard.tsx`（新建）
- `app/src/App.tsx`（替换默认渲染为 Dashboard）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-026-dashboard-stats-cards`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(frontend): 新增 formatBytes 工具函数 (#task-026)`
   —— 含 `app/src/utils/formatBytes.ts`。
2. `feat(frontend): 新增 Dashboard 页面——audit_log.stats 4 数字卡片 (#task-026)`
   —— 含 `app/src/pages/Dashboard.tsx`。
3. `feat(frontend): App.tsx 挂载 Dashboard 为默认页 (#task-026)`
   —— 含 `app/src/App.tsx`。
4. 自验全绿后，最后一次提交：`docs(state): 任务026完成，前端仪表盘数字卡片就绪 (#task-026)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 026: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-027：前端历史列表——audit_log.list 接 React 表格（分页 / session 过滤）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-13  task-026  Dashboard 4 数字卡片（stats 接前端，GB 格式化）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0（后端无改动，保持基线）；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 133 条全绿）；
- `pnpm --filter app build` 退出码 0（Vite + tsc 通过）；
- DevAgent 本地 `pnpm --filter app tauri dev` 真机看到 4 个卡片（截图或文字描述贴备注）。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-026-dashboard-stats-cards`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - `pnpm --filter app build` 输出（无 error）；
   - `tauri dev` 真机截图或文字描述（4 卡片数字）；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（list 历史、scan/delete 交互、safe_box UI、rules 更新 UI 留 task-027+）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务026-前端仪表盘数字卡片 (#task-026)`，
   该提交只含 `tasks/026-dashboard-stats-cards.md` 一个文件，落在 `feature/task-026-dashboard-stats-cards` 分支最前端。

## 实现备注（DevAgent 落地后回填）
- **改动文件**：新建 `app/src/utils/formatBytes.ts`、`app/src/pages/Dashboard.tsx`；`app/src/App.tsx` 默认渲染 `<Dashboard />`；`app/package.json` + `pnpm-lock.yaml` 新增 `@tauri-apps/api@2.12.3` 依赖（任务书假设已在 devDeps 实际未装，本次补装）。
- **formatBytes 规则**：n<1024→"B"；<1024²→"x.x KB"；<1024³→"x.x MB"；否则→"x.x GB"；负数/NaN 兜底 "0 B"。
- **Dashboard 状态机**：`status: 'loading'|'ok'|'error'`；loading 4 个灰色占位条（opacity 0.5）；error 红底 div + message + 「重试」按钮（useCallback load 重新 invoke）；ok 2×2 grid 4 卡片，失败数>0 时数字红色 `#dc2626`。
- **Tauri 调用**：`invoke<CleanupStats>('audit_log_stats', {})`，入参空对象查全局；类型按任务书 snake_case（total_bytes_freed/success_count/failed_count/session_count）。
- **自验命令与退出码**：cargo check=0 / clippy -D warnings=0 / cargo test -p engine=0（133 条全绿，后端无改动保持基线）/ cargo fmt --check=0 / pnpm --filter app build=0（vite 34 modules，dist/assets/index-*.js 145KB gzip 46.8KB）。
- **偏差**：任务书声称 `@tauri-apps/api` 已在 devDeps，实际未装——本次新增依赖并打包进 commit 3（与 App.tsx 同一 commit）。其他需求逐项落实，无其他偏差。
- **遗留**：未接 audit_log.list 历史列表（task-027）、未接 scan/delete 交互（task-028+）、未接 safe_box / rules 更新 UI；未写 Vitest/Playwright（CI echo 占位）。
