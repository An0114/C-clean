# 任务编号：007 落地真实 ProgressEmitter——Tauri AppHandle::emit，前端 listen 收得到 scan://progress

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevTauriBridge
> 来源：`docs/PROJECT_STATE.md` 待办 007（M1 前置清理清单第 1 条，2026-10-12 debt-cleanup 批次001 拆出）。
> 上游依赖：task-004（`engine::ipc::progress::ProgressEmitter` trait + `ScanProgress` payload + `ThrottledEmitter` 已就绪）已合并到 main。

## 任务目标

task-004 已经把进度通道的纯逻辑骨架（trait / payload / 100ms 节流 / `VecEmitter` 测试实现）落在 engine 里，但真实 emit 通道（`tauri::AppHandle::emit`）当时明确不做。本任务把这条通道接通：

1. 在 `app/src-tauri/` 侧实现 `TauriProgressEmitter`（包装 `tauri::AppHandle`，调 `app.emit("scan://progress", payload)`），**不**把 tauri 依赖引到 `engine/`；
2. 在 Tauri Builder 里用 `app.manage(...)` 注册一个全局 `ThrottledEmitter<TauriProgressEmitter>` State；
3. 修改 `scan_start` Tauri command：返回 sessionId 之前，从 State 里取出 emitter、emit 一个 `percent=0, current_path="starting"` 的 demo 进度——**不**启动真实扫描，只证明通道通；
4. 在 WebView2 devtools 里验证：`await listen('scan://progress', ...)` 能收到事件。

**本任务不做**：
- 不做真实扫描（M1 业务模块 A~F 的事）；
- 不接 `scan.cancel` / `scan.listItems` 等其他命令；
- 不改 `engine/ipc/progress.rs` 的已有 trait、payload、ThrottledEmitter 行为（已有测试必须全绿）；
- 不改前端 React 组件代码（`app/src/**`）——验证只在 devtools console 手动 listen；
- 不写 audit_log、不写 DB。

## 前置条件

1. `main` 分支已含 task-001~006 全部产出；`engine::ipc::progress` 已暴露 `ProgressEmitter` trait、`ScanProgress` 结构、`ThrottledEmitter`、`NoopEmitter`、`VecEmitter`。
2. 关键锚点：
   - `engine/src/ipc/progress.rs`（task-004 产出）——trait 签名与 payload 字段；
   - `app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`（task-004 产出）——当前 `scan_start` 命令签名；
   - `docs/TRD.md` §5.2（约 300 行起）——事件名 `scan://progress` 与 payload 字段（`percent / current_path`）；
   - `docs/ARCHITECTURE.md` §3 扫描流程图（约 163 行）——「`scan://progress` 事件 ≤1次/100ms 节流」。
3. 本机可跑 `pnpm tauri dev`，WebView2 devtools 可开。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. 架构边界（硬约束）

1. **`engine/` 不允许新增 tauri 依赖**：检查 `engine/Cargo.toml` 不出现 `tauri` / `tauri-runtime` 字样；
2. `ProgressEmitter` trait 保持在 `engine::ipc::progress`，签名不变；
3. `TauriProgressEmitter` 实现放在 `app/src-tauri/src/commands.rs`（或新建 `app/src-tauri/src/progress.rs`），impl 块对 `engine::ipc::progress::ProgressEmitter` trait 落具体类型。

### B. `TauriProgressEmitter` 实现

4. 定义结构（建议）：
   ```rust
   pub struct TauriProgressEmitter {
       app: tauri::AppHandle,
   }
   impl engine::ipc::progress::ProgressEmitter for TauriProgressEmitter {
       fn emit(&self, payload: &ScanProgress) {
           // 调 self.app.emit("scan://progress", payload)；
           // emit 失败（如窗口已关）仅吞掉，不 panic、不返回 Result；
       }
   }
   ```
   - 事件名常量 `"scan://progress"` 与 engine 侧 `SCAN_PROGRESS_EVENT` 字符串一致（最好直接引用 engine 常量，避免两处硬编码漂移）；
   - payload 用 engine 的 `ScanProgress`，serde 序列化后由 AppHandle 自动转成 JS 对象。

### C. State 注册

5. 在 `app/src-tauri/src/lib.rs` 的 `tauri::Builder::default()` 链上：
   - `app.manage(Mutex::new(ThrottledEmitter::new(TauriProgressEmitter { app: handle })))`；
   - `handle` 在 `setup` 闭包里拿到；或在命令里通过 `tauri::AppHandle` 参数现取（DevAgent 自选，备注里说明）；
   - State 类型建议包一层 `pub struct ProgressState(pub Mutex<ThrottledEmitter<TauriProgressEmitter>>)`。
6. **不**在 State 里塞 DB 连接、不塞真实 resolver——保持本任务最小。

### D. `scan_start` 命令改造

7. 把现有 `scan_start` 命令签名改为：
   ```rust
   #[tauri::command]
   async fn scan_start(
       scope: String,
       target_drive: Option<String>,
       app: tauri::AppHandle,
       state: tauri::State<'_, ProgressState>,
   ) -> Result<Envelope<ScanStartResp>, String>;
   ```
   - 原有入参校验逻辑（scope ∈ {full,custom}、targetDrive 格式）保持不变；
   - 校验通过、拿到 `sessionId` 后，在返回前调 `state.0.lock().unwrap().emit(&ScanProgress { session_id, percent: 0, current_path: "starting".into(), scanned_items: 0 })`；
   - **只 emit 这一条 demo 进度**——不写循环、不启动线程、不模拟扫描；
   - 任何 emit 失败不影响命令返回（demo 事件没发出去也返回 Ok）。

### E. 前端 listen 端到端验证

8. DevAgent 在 `pnpm tauri dev` 启动后，开 WebView2 devtools console，执行：
   ```js
   await window.__TAURI__.event.listen('scan://progress', (e) => console.log('[scan://progress]', e.payload));
   await window.__TAURI__.core.invoke('scan_start', { scope: 'full' });
   ```
   - console 必须打印一条 `[scan://progress]` 对象，含 `sessionId`、`percent: 0`、`current_path: "starting"`、`scannedItems: 0`；
   - DevAgent 把 console 截图或粘贴输出贴进实现备注；
9. 前端 React 代码（`app/src/App.tsx` 等）**不**改——不写 useEffect listen、不写进度条 UI。

### F. 节流与回归

10. engine 侧已有测试（`ThrottledEmitter` 100ms 内丢重复、`VecEmitter` 记录）必须全绿；
11. 在 `app/src-tauri` 侧**不写**测试（Tauri AppHandle 单测成本高），验证靠 §E 端到端手动；
12. `cargo clippy -p cdisk-cleaner --all-targets -- -D warnings` 全绿、`cargo test -p engine` 全绿、`pnpm --filter app build` 全绿。

### G. 卫生与边界

13. 不修改 `engine/**` 任何文件（trait 与已有实现保持不动）；
14. 不修改 `app/src/**`（前端 React）；
15. 不修改 `rules/**`、`scripts/**`、`elevation/**`；
16. 不新增依赖（tauri 已在 app/src-tauri 依赖里，无需新增）；
17. 不写真实扫描逻辑、不 spawn 线程、不接 DB。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `app/src-tauri/src/lib.rs`（Builder 链加 `manage(...)`）
- `app/src-tauri/src/commands.rs`（改造 `scan_start` 签名、新增 `TauriProgressEmitter` 与 `ProgressState`）
- 可选：`app/src-tauri/src/progress.rs`（新建，把 emitter 与 state 拆出去；DevAgent 自选）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`（由最后一次 `docs(state)` 提交处理）

## Git 分支

- 分支名：`feature/task-007-tauri-progress-emitter`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(tauri): 实现 TauriProgressEmitter（AppHandle::emit scan://progress） (#task-007)`
   —— 含 `app/src-tauri/src/commands.rs` 或 `progress.rs` 新增结构与 impl。
2. `feat(tauri): 在 Builder 注册 ProgressState 并改造 scan_start 命令 emit demo 进度 (#task-007)`
   —— 含 `app/src-tauri/src/lib.rs`（manage）、`app/src-tauri/src/commands.rs`（签名改造）。
3. 自验全绿后，最后一次提交：`docs(state): 任务007完成，真实进度通道接通 (#task-007)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 把待办 007 从 `- [ ]` 改为 `- [x]`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-008：symlink/junction 解析防绕开——规范化器接 symlink_metadata，junction 指向 L3 时一并拒」；
   - 「Git 提交记录」表追加本任务 commit 摘要；
   - 「已知问题」段删除 task-004 未做真机联调那条（本任务已补）；
   - 文件末尾「变更轨迹」追加 `2026-10-12  task-007  落地真实 ProgressEmitter 与 scan://progress 前端 listen 端到端`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（engine 已有测试不回退）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0；
- WebView2 devtools 端到端验证通过（§E-8 的 console 输出）。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 2 个代码提交 + 1 个 `docs(state)` 提交，共 3 个 commit，全部 push 到 `feature/task-007-tauri-progress-emitter`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - State 注入方式（setup 闭包 vs 命令参数）；
   - 自验命令与退出码；
   - devtools console listen + invoke 的粘贴输出；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（如：真实扫描任务何时接 emitter、scanned_items 计数器何时从 engine 侧传入）；
3. `docs/PROJECT_STATE.md` 已按第 3 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务007-真实ProgressEmitter (#task-007)`，
   该提交只含 `tasks/007-tauri-progress-emitter.md` 一个文件，落在 `feature/task-007-tauri-progress-emitter` 分支最前端。
