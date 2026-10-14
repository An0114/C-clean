# 任务编号：036 启动自动检查应用更新——非阻塞 banner 提示新版本

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFrontendReact
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-035 合并后写入）——updater 已能在 Settings 页手动触发，下一步让应用启动时自动检查一次。
> 上游依赖：task-035（`@tauri-apps/plugin-updater` 已接入，`check()` / `downloadAndInstall()` / `relaunch()` 已就绪）、task-026~031（App.tsx tab 切换 + 5 个页面已就位）。
> **克制范围**：只做「启动自动 check + 非阻塞 banner」，**不做**强制更新 / 定时轮询 / 下载进度条。

## 任务目标

目前用户得自己切到「设置」tab 点「检查应用更新」才知道有新版本。本任务补最小自动化：

1. App 启动时 `useEffect` 自动调一次 `check()`：
   - 无更新 / 检查失败 → 静默，不打扰用户；
   - 有更新 → 在主界面顶部显示**非阻塞 banner**（不是 modal，不挡住用户操作）；
2. Banner 内容：
   - 「发现新版本 v0.1.1」+ 版本说明（update.body 或 notes）；
   - 「立即更新」按钮（绿）+ 「稍后」按钮（灰）；
3. 点「立即更新」：
   - banner 变「下载中…（不阻塞，用户继续用）」；
   - `downloadAndInstall()` + `relaunch()`；
4. 点「稍后」：banner 消失，本次启动不再弹（下次启动再检查）。

**本任务不做**：
- 不做强制更新（用户能忽略）；
- 不做下载进度条（downloadAndInstall 自带简单日志，不展示）；
- 不做定时轮询（只启动时检查一次）；
- 不做"不再提示此版本"持久化（每次启动有新版本就弹）；
- 不改 Settings 页已有的手动检查按钮。

## 前置条件

1. `main` 分支已含 task-001~035 全部产出；
2. 关键锚点：
   - `app/src/App.tsx`（task-029 已加 5 tab 切换）；
   - `app/src/pages/Settings.tsx`（task-035 已有「应用更新」卡片）；
   - `@tauri-apps/plugin-updater` API（`check()` → `Update | null`）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. 启动自动检查

1. 在 `app/src/App.tsx` 加：
   ```ts
   const [update, setUpdate] = useState<Awaited<ReturnType<typeof check>> | null>(null);
   const [dismissed, setDismissed] = useState(false);
   useEffect(() => {
     (async () => {
       try {
         const u = await check();
         if (u) setUpdate(u);
       } catch { /* 静默 */ }
     })();
   }, []);
   ```

### B. 非阻塞 Banner

2. 在 App 顶部（`<h1>` 下方、tab 按钮上方）渲染：
   ```tsx
   {update && !dismissed && (
     <div style={{ background: '#dbeafe', border: '1px solid #93c5fd', padding: '10px 14px', borderRadius: 8, marginBottom: 12, display: 'flex', gap: 12, alignItems: 'center' }}>
       <div style={{ flex: 1 }}>
         <div style={{ fontWeight: 600 }}>发现新版本 v{update.version}</div>
         {update.body && <div style={{ fontSize: 12, color: '#374151' }}>{update.body}</div>}
       </div>
       <button onClick={onUpdateNow} style={{ background: '#10b981', color: '#fff', border: 0, padding: '6px 12px', borderRadius: 6, cursor: 'pointer' }}>
         立即更新
       </button>
       <button onClick={() => setDismissed(true)} style={{ background: '#fff', border: '1px solid #d1d5db', padding: '6px 12px', borderRadius: 6, cursor: 'pointer' }}>
         稍后
       </button>
     </div>
   )}
   ```

### C. 立即更新逻辑

3. `onUpdateNow`：
   - banner 文案改「下载中…」（按钮禁用）；
   - `await update.downloadAndInstall()`；
   - `await relaunch()`（来自 `@tauri-apps/plugin-process`）；
   - 出错：banner 变红条「更新失败：<err>」。

### D. 卫生

4. 不改 Rust 代码；
5. 不改 Settings 页手动检查按钮；
6. 不新增 npm 依赖（plugin-updater / plugin-process task-035 已装）；
7. 不在 tab 切换时重复 check。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `app/src/App.tsx`（加启动 check + banner）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-036-auto-update-banner`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(frontend): App 启动自动 check() 应用更新 (#task-036)`
   —— 含 `app/src/App.tsx`。
2. `feat(frontend): 加非阻塞 banner——新版本提示 + 立即更新/稍后按钮 (#task-036)`
   —— 含 `app/src/App.tsx`。
3. `feat(frontend): banner 立即更新走 downloadAndInstall + relaunch (#task-036)`
   —— 含 `app/src/App.tsx`。
4. 自验全绿后，最后一次提交：`docs(state): 任务036完成，启动自动更新 banner 就绪 (#task-036)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 036: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「M2 updater 闭环完成（手动 + 自动）；下一轮进入：①首次 v0.1.1 发版验证完整 updater 流程 ②品牌图标设计（用户手动）③safe_box purge 命令 / 批量恢复 / dry-run 预览 UI 等 M1 遗留」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-14  task-036  启动自动 check + 非阻塞 banner 提示新版本，updater 最后一公里`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0（后端无改动）；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0（已有纯函数单测全绿）；
- `pnpm --filter app build` 退出码 0。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个 feat 提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-036-auto-update-banner`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - `pnpm build` 输出（无 error）；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（强制更新、下载进度条、定时轮询、"不再提示此版本"持久化留 task-037+）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务036-启动自动更新 banner (#task-036)`，
   该提交只含 `tasks/036-auto-update-banner.md` 一个文件，落在 `feature/task-036-auto-update-banner` 分支最前端。

## 实现备注

### 实际改动文件清单
- `app/src/App.tsx`：加 `useEffect` 启动自动 `check()`、非阻塞 banner UI、`onUpdateNow` 走 `downloadAndInstall + relaunch`
- `docs/PROJECT_STATE.md`：状态同步

### 自验输出
- `cargo check --workspace --all-targets`：0（后端无改动）
- `pnpm --filter app lint`：0
- `pnpm --filter app test`：0（12 断言无回退）
- `pnpm --filter app build`：0（165.78 kB / gzip 52.79 kB）

### 与本任务文档不符的偏差
- 任务书要求拆 3 个 feat 提交（check / banner / downloadAndInstall），但 banner UI 与 onUpdateNow handler 紧耦合在同一块 JSX 里，拆开会出现中间态「按钮无 onClick」无法编译，故合并为 2 个 feat 提交（2d959b3 check / f460c42 banner+install）。

### 遗留问题
- 强制更新 / 下载进度条 / 定时轮询 / 「不再提示此版本」持久化：明确不做，留 task-037+
- 真机端到端（启动 → banner 弹出 → 立即更新 → 自动重启）：待首次 v0.1.1 发版后验证
