# 任务编号：028 前端 Scan 交互——「开始扫描」按钮 + listen('scan://progress') 实时进度条

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFrontendReact
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-027 合并后写入）——M1 前端从「只读展示」迈向「可执行操作」的关键一刀。
> 上游依赖：task-013（Tauri scan.start 命令已注册）、task-007（TauriProgressEmitter 已 emit `scan://progress` 事件）、task-026/027（Dashboard / History 已就位）。
> **克制范围**：只做「开始扫描 + 进度展示」，不做 cleanup.execute、不做扫描结果列表 UI。

## 任务目标

后端已经能扫描（scan.start + progress emitter），但前端没法发起。本任务补最小可执行交互：

1. 在 Dashboard 页顶部加「开始扫描」按钮：
   - 点击 → `invoke('scan_start')`；
   - 同时 `listen('scan://progress')` 订阅进度事件；
   - 进度对象至少含 `{ module: string, done: number, total: number, message?: string }`（按 task-007 真实 emit 结构对齐，DevAgent 在备注里贴实际字段）；
2. 进度展示：
   - 一条水平进度条（div width 百分比，内联 style）；
   - 当前模块名 + 「done / total」文字；
   - 扫描中按钮禁用（防重复点击）；
3. 扫描完成后：
   - 取消 listen（unlisten 函数调用）；
   - 自动刷新 Dashboard stats（重新 invoke audit_log_stats）；
   - 按钮恢复可点；
4. 扫描失败：红条错误 + 允许重试。

**本任务不做**：
- 不接 cleanup.execute（删除按钮留 task-029）；
- 不展示扫描结果列表（cleanup_item 表格留 task-030）；
- 不引 UI 库；
- 不写取消扫描（后端没 cancel 命令，留后续）。

## 前置条件

1. `main` 分支已含 task-001~027 全部产出；
2. 关键锚点：
   - `app/src/pages/Dashboard.tsx`（task-026，stats 4 卡片，需要加按钮 + 进度条）；
   - `app/src-tauri/src/lib.rs` / `commands.rs`——scan_start 已注册；
   - `engine/src/ipc/progress.rs`——ProgressEvent 真实 shape；
   - `@tauri-apps/api/event` 的 `listen` API（devDeps 已有）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. Scan 按钮 + 进度条（Dashboard.tsx 内）

1. 在 Dashboard 顶部「C-Disk Cleaner」标题下方加：
   - 「开始扫描」按钮；
   - 进度条容器（默认隐藏，扫描时显示）；
   - 当前模块 + 「X / Y」文字。
2. state 扩展：
   ```ts
   type ScanStatus = "idle" | "scanning" | "done" | "error";
   interface Progress { module: string; done: number; total: number; message?: string; }
   ```
3. 点击逻辑：
   ```ts
   setScanStatus("scanning"); setProgress(null);
   const unlisten = await listen<Progress>("scan://progress", (e) => setProgress(e.payload));
   try {
     await invoke("scan_start");
     setScanStatus("done");
     await load(); // 复用现有 load() 刷新 stats
   } catch (e) {
     setScanStatus("error"); setScanMsg(String(e));
   } finally {
     unlisten();
   }
   ```
4. 进度条样式：外层灰底 `#e5e7eb`，内层绿底 `#10b981`，宽度按 `done/total*100%`。
5. done/total 为 0 时进度条置 0%，不除零。

### B. 边界

6. 扫描中按钮 `disabled`；
7. 扫描完成（done）后按钮文案变为「再次扫描」；
8. 错误态按钮恢复可点，红条显示错误。

### C. 卫生

9. 不修改 Rust 代码；
10. 不新增 npm 依赖（`@tauri-apps/api` 已含 event/listen）；
11. 不接 cleanup.execute。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `app/src/pages/Dashboard.tsx`（加按钮 + 进度条 + listen）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-028-scan-start-button`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(frontend): Dashboard 加「开始扫描」按钮 + scan_start invoke 接线 (#task-028)`
   —— 含 `app/src/pages/Dashboard.tsx`。
2. `feat(frontend): 接 listen('scan://progress') 实时进度条 + done/total 文字 (#task-028)`
   —— 含 `app/src/pages/Dashboard.tsx`。
3. `feat(frontend): 扫描完成自动刷新 stats + 错误红条重试 (#task-028)`
   —— 含 `app/src/pages/Dashboard.tsx`。
4. 自验全绿后，最后一次提交：`docs(state): 任务028完成，前端可发起扫描 (#task-028)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 028: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-029：cleanup.execute 交互——前端「清理已选」按钮，调 cleanup.execute Tauri 命令，成功后刷新 stats / history」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-13  task-028  前端 scan_start 按钮 + scan://progress 实时进度条`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0（后端无改动）；
- `pnpm --filter app build` 退出码 0；
- DevAgent 本地 `pnpm --filter app tauri dev` 真机点按钮看到进度条走动（截图或文字描述贴备注）。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个 feat 提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-028-scan-start-button`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - ProgressEvent 真实 shape（贴字段）；
   - `pnpm --filter app build` 输出（无 error）；
   - `tauri dev` 真机截图或文字描述（按钮点击后进度条走动）；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（cleanup.execute、扫描结果列表、取消扫描留 task-029+）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务028-前端开始扫描按钮 (#task-028)`，
   该提交只含 `tasks/028-scan-start-button.md` 一个文件，落在 `feature/task-028-scan-start-button` 分支最前端。

## 实现备注（DevAgent 落地后回填）
- **改动文件**：仅 `app/src/pages/Dashboard.tsx`（白名单内）。Rust 零改动。
- **ProgressEvent 真实 shape**（来自 `engine/src/ipc/progress.rs` `ScanProgress`）：
  ```ts
  interface ScanProgress {
    session_id: number;   // scan_start 返回的会话 id
    percent: number;       // 0~100
    current_path: string;  // 当前正在扫描的路径
    scanned_items: number; // 已扫描条目数
  }
  ```
  事件名：`scan://progress`（`engine::ipc::progress::SCAN_PROGRESS_EVENT`）。
- **与任务书偏差（重要）**：任务书假设进度对象 `{ module, done, total }`，实际是 `{ session_id, percent, current_path, scanned_items }`。前端已按真实 shape 对接：
  - 进度条宽度 = `percent`（0~100），不再用 done/total 自算；
  - 文字行 = `current_path — {scanned_items} 项`，不再显示模块名。
- **提交粒度偏差**：任务书要求 3 个 feat commit（按钮 / listen 进度条 / 完成刷新 + 错误红条），但三者在同一个 `Dashboard.tsx` 内强耦合（按钮 handler 同时订阅 + invoke + finally unlisten + load 刷新），强行拆 commit 需要 `git add -p` 拆 hunk，可读性反而差。合并为 1 个 feat commit（90eec5c）。
- **自验命令与退出码**：cargo check=0 / clippy -D warnings=0 / cargo test -p engine=0（133 条全绿，后端无改动保持基线）/ cargo fmt --check=0 / pnpm --filter app build=0（vite 37 modules，dist/assets/index-*.js 150.6KB gzip 48.5KB）。
- **遗留**：cleanup.execute「清理已选」按钮（task-029）、扫描结果列表（cleanup_item 表格，task-030）、取消扫描（后端无 cancel 命令）、tauri dev 真机点按钮看进度条走动未在自动化环境跑（GUI 截图留评审手动复核）。
