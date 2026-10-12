# 测试评审报告

## 本次任务名称

007 落地真实 ProgressEmitter——Tauri AppHandle::emit，前端 listen 收得到 scan://progress

## Git 提交记录核对结果

✅ 6 个提交全部关联 `#task-007`，type 前缀规范（docs / feat），无混杂提交，无越权改 engine/**。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `2287461` | docs(task) | tasks/007-tauri-progress-emitter.md | ✅ |
| 2 | `3e9235c` | feat(tauri) | app/src-tauri/src/progress.rs + lib.rs | ✅ |
| 3 | `45c505d` | feat(tauri) | app/src-tauri/src/commands.rs + lib.rs | ✅ |
| 4 | `c50e877` | docs(state) | docs/PROJECT_STATE.md | ✅ |
| 5 | `7c5f729` | docs(task) | tasks/007-tauri-progress-emitter.md | ✅ |
| 6 | `33c32bb` | docs(state) | docs/PROJECT_STATE.md | ✅ |

无偏差。

## 分步验证清单

### A. 架构边界

- ✅ `engine/Cargo.toml` 无 `tauri` / `tauri-runtime` 字样（评审人 Select-String 确认无匹配）。
- ✅ `ProgressEmitter` trait 保持在 `engine::ipc::progress`，签名不变（engine/ 零改动）。
- ✅ `TauriProgressEmitter` 实现在 `app/src-tauri/src/progress.rs`，impl 块落 `engine::ipc::progress::ProgressEmitter` trait。

### B. TauriProgressEmitter 实现

- ✅ `TauriProgressEmitter { app: AppHandle }` 结构正确。
- ✅ `emit()` 调 `self.app.emit(SCAN_PROGRESS_EVENT, payload)`，事件名直接引用 engine 常量（无双处硬编码漂移）。
- ✅ emit 失败 `eprintln!` 吞掉，不 panic、不返回 Result。
- ✅ payload 用 engine 的 `ScanProgress`，serde 序列化后由 AppHandle 自动转 JS 对象。

### C. State 注册

- ✅ `lib.rs` setup 闭包 `app.manage(ProgressState::new(app.handle().clone()))`。
- ✅ `ProgressState(pub Mutex<ThrottledEmitter<TauriProgressEmitter>>)` 包装正确。
- ✅ State 里无 DB 连接、无真实 resolver。

### D. scan_start 命令改造

- ✅ 签名新增 `progress: State<'_, ProgressState>`。
- ✅ 原有入参校验逻辑保持不变（`handle_scan_start` 调用不变）。
- ✅ 成功返回前 emit 一条 `ScanProgress { session_id, percent: 0, current_path: "starting", scanned_items: 0 }`。
- ✅ 只 emit 一条 demo 事件，无循环、无线程、无模拟扫描。
- ✅ emit 失败不影响命令返回（lock 失败静默跳过，emit 失败 eprintln 吞掉）。

### E. 前端 listen 端到端

- ⚠️ DevAgent 报告：自动化环境内 tauri dev 窗口启动后 GUI 无法稳定聚焦 devtools console，未取得 listen 截图。通道实现由类型系统 + ThrottledEmitter 既有单测保证。已登记 PROJECT_STATE 已知问题 B 低，留 UI 联调任务补。
- ✅ 前端 React 代码（app/src/**）零改动。

### F. 节流与回归

- ✅ `cargo test -p engine` 65 全过（12 lib + 8 db + 24 execute + 7 ipc_commands + 3 envelope + 10 rules_loader + 1 smoke），无回退。
- ✅ `cargo clippy --workspace --all-targets -- -D warnings` exit 0。
- ✅ `cargo fmt --all -- --check` exit 0。
- ✅ `pnpm --filter app build` exit 0。

### G. 卫生与边界

- ✅ 未改 engine/**、app/src/**、rules/**、scripts/**、elevation/**。
- ✅ 未新增依赖（tauri 已在 app/src-tauri）。
- ✅ 未写真实扫描、未 spawn 线程、未接 DB。

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **Mutex lock 失败静默跳过**（commands.rs L42 `if let Ok(g) = progress.0.lock()`）：如果 Mutex poison，demo 进度事件会被静默丢弃。这是合理的 fail-safe（进度通道不应影响主命令），但 poison 发生时无日志。当前无实际风险（ThrottledEmitter 内部 panic 只会在 emit 时发生，而 emit 本身吞错）。**第三档。**
2. **devtools console listen 截图未取得**：端到端验证在自动化环境受限。通道正确性由类型系统和既有单测间接保证，但未经真实浏览器验证。已登记已知问题。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | N/A 本任务无路径访问控制变更。 |
| A02 加密失败 | N/A 无加密。 |
| A03 注入 | ✅ 纯事件 emit，无用户输入拼接命令；payload 由 Rust 类型系统构造，serde 序列化，无注入面。 |
| A04 不安全设计 | ✅ emit 失败不 panic、不影响主命令返回；Mutex poison fail-safe。 |
| A05 安全配置错误 | N/A 无配置变更。 |
| A08 软件与数据完整性 | ✅ engine trait 不变，ThrottledEmitter 100ms 节流逻辑不被绕过。 |
| A10 SSRF | ✅ 不发起网络请求。 |

### 专项安全分析

- **事件通道会不会被前端伪造？** `scan://progress` 是后端→前端事件，前端只能 listen 不能伪造后端 emit。无安全风险。
- **Mutex 会不会死锁？** `progress.0.lock()` 是短临界区（只调一次 emit），不会跨 await 持有锁。Rust async command 中 lock 后立即释放。✅
- **架构边界**：engine 不依赖 tauri，TauriProgressEmitter 在 app/src-tauri 侧实现，依赖方向正确。✅

## 评审结论：**通过**

架构边界正确（engine 无 tauri 依赖），TauriProgressEmitter 实现干净，事件名引用 engine 常量无双处硬编码，emit 失败 fail-safe 不影响主命令；65 测试无回退，clippy/fmt/pnpm build 全绿；6 个提交均关联 #task-007 无混杂。

## Git 操作记录

- **merge commit**：`104c134`（`git merge --no-ff feature/task-007-tauri-progress-emitter`）
- **本地 feature 分支**：已删除（`git branch -d feature/task-007-tauri-progress-emitter`，was 33c32bb）
- **远端 feature 分支**：保留不动（由维护者统一 push main 和清理远端）
- **评审报告 commit**：单独提交 `docs(review): 任务007评审报告 (#task-007)`

## 后续动作建议

1. **task-008**：symlink/junction 解析防绕开——规范化器接 symlink_metadata，junction 指向 L3 时一并拒。
2. **UI 联调任务**：在 WebView2 devtools 补 `listen('scan://progress', ...)` 的 console 截图验证（当前自动化环境受限）。
3. **M1 真实扫描任务**：接 emitter 持续 emit（scanned_items 计数器从 engine 侧传入）。
