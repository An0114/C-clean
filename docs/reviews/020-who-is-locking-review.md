# 测试评审报告

## 本次任务名称

020 who_is_locking——Windows Restart Manager FFI 查询占用进程 PID

## Git 提交记录核对结果

✅ 8 个提交全部关联 `#task-020`，type 前缀规范（docs / build / feat / test / style），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `f1d9af6` | docs(task) | tasks/020-who-is-locking.md | ✅ |
| 2 | `a3f3a3d` | build(deps) | Cargo.lock + engine/Cargo.toml | ✅ |
| 3 | `8445d68` | feat(execute) | locking.rs + mod.rs | ✅ |
| 4 | `7910e38` | test(execute) | locking_who_is_locking.rs | ✅ |
| 5 | `47590f9` | style(execute) | locking.rs + locking_who_is_locking.rs（fmt） | ✅ |
| 6 | `c06801a` | docs(state) | PROJECT_STATE.md | ✅ |
| 7 | `614f2fb` | docs(task) | tasks/020-who-is-locking.md | ✅ |
| 8 | `1fa3805` | docs(state) | PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. Cargo.toml 引入 windows crate

- ✅ `[target.'cfg(windows)'.dependencies]` windows 0.58。
- ✅ features：Win32_Foundation + Win32_System_RestartManager。
- ✅ Win32_WindowsProgramming 已去掉（windows 0.58 中不存在，DevAgent 已记录偏差）。
- ✅ 非 Windows 平台不引入。

### B. who_is_locking（locking.rs）

- ✅ LockingProcess { pid: u32, app_name: String, service_name: Option<String> }。
- ✅ who_is_locking(path) → Windows 走 FFI，非 Windows 返回空 Vec。
- ✅ RmStartSession → RmRegisterResources → RmGetList probe → RmGetList fetch → RmEndSession。
- ✅ 第一次 RmGetList 预期 ERROR_MORE_DATA(234)，其他错视为失败。
- ✅ UTF-16 → String（String::from_utf16_lossy）。
- ✅ 错误路径 RmEndSession 清理（不泄漏 session）。
- ✅ 不做 RmShutdown / RmRestart（只读查询）。

### C. 单测

- ✅ `who_is_locking_no_lockers_returns_empty`：CI 必跑，空锁返回空 Vec。
- ✅ `who_is_locking_notepad_locks_file`：#[ignore] DevAgent 手动跑（GUI 不稳定）。

### D. 卫生与边界

- ✅ 未修改 delete.rs / fs_deleter.rs / safe_box.rs / executor.rs / error_kind.rs。
- ✅ 未接 Tauri 命令。
- ✅ 未写 RmShutdown / RmRestart。
- ✅ windows crate 最小 feature 原则。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 117 passed / 0 failed / 1 ignored（116 已有 + 1 空锁新增，notepad #[ignore]） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0 |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **RmEndSession 返回值忽略**：`let _ = RmEndSession(...)`。best effort 清理，无害。**第三档。**
2. **notepad 测试 #[ignore]**：DevAgent 已记录（GUI 不稳定）。空锁测试 CI 必跑。**第三档。**
3. **windows crate feature 偏差**：任务书写 Win32_WindowsProgramming，实际 windows 0.58 中不存在，已去掉。**第三档。**
4. **未接 Tauri 命令**：engine 层函数先落地，Tauri 接线留 task-021。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ 只读查询，不修改系统状态；不关闭进程。 |
| A03 注入 | ✅ path 来自调用方，FFI 用 PCWSTR 传递，无 shell。 |
| A04 不安全设计 | ✅ 错误路径 RmEndSession 清理 session；非 Windows 空实现兜底。 |
| A08 数据完整性 | ✅ 不写 DB / audit_log（纯查询函数）。 |

### 专项安全分析

- **会不会误关闭进程？** 不会。只做 RmStartSession / RmRegisterResources / RmGetList / RmEndSession，不调 RmShutdown / RmRestart。✅
- **session 会不会泄漏？** 不会。所有错误路径都调 RmEndSession。✅
- **非 Windows 平台能编译吗？** 能。cfg(not(windows)) 返回空 Vec。✅
- **unsafe 块用得对吗？** FFI 调用全部包裹在 unsafe 块内；buf 分配后 RmGetList 填充；遍历时手动偏移计算 RM_PROCESS_INFO 大小。✅
- **会不会泄露进程路径？** LockingProcess 只暴露 pid / app_name（文件名）/ service_name，不暴露完整路径。✅

## 评审结论：**通过**

who_is_locking 落地：Windows Restart Manager FFI 四件套（StartSession / RegisterResources / GetList / EndSession）查占用进程 PID + 进程名；只读查询不关闭进程；非 Windows 空实现；117 测试全过（空锁 CI 必跑，notepad #[ignore]）；clippy/fmt/pnpm build 全过；8 个提交全部关联 #task-020 无混杂。文件占用首次能定位到具体进程。

## Git 操作记录

- **merge commit**：`448ed7b`（`git merge --no-ff feature/task-020-who-is-locking`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-020-who-is-locking`，was 1fa3805）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务020评审报告 (#task-020)`

## 后续动作建议

1. **task-021**：cleanup.execute 失败时自动调 who_is_locking 把占用进程写进 audit_log/detail。
2. **后续**：RmShutdown / RmRestart（请求关闭进程）、Tauri 命令接线、前端 toast 提示。
3. **M1 收尾**：scan.listItems 命令、前端勾选 UI、打包时 data_dir 改 app_data_dir。
