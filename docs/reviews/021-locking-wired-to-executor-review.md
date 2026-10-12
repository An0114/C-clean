# 测试评审报告

## 本次任务名称

021 占用进程自动接线——FILE_IN_USE 时自动调 who_is_locking 写 audit_log 并返回前端

## Git 提交记录核对结果

✅ 7 个提交全部关联 `#task-021`，type 前缀规范（docs / feat / test / style），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `dd088e5` | docs(task) | tasks/021-locking-wired-to-executor.md | ✅ |
| 2 | `a4dbee4` | feat(execute) | executor.rs | ✅ |
| 3 | `841bf87` | test(execute) | execute_locking_integration.rs | ✅ |
| 4 | `022de36` | style(execute) | executor.rs + execute_locking_integration.rs（clippy + fmt） | ✅ |
| 5 | `7d1ba15` | docs(state) | PROJECT_STATE.md | ✅ |
| 6 | `1d32a13` | docs(task) | tasks/021-locking-wired-to-executor.md | ✅ |
| 7 | `cf130ea` | docs(state) | PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. ExecResult 加字段

- ✅ ExecResult 加 `locking_processes: Vec<LockingProcess>`。
- ✅ 成功路径：locking_processes = Vec::new()。
- ✅ 默认空 Vec，只有 FILE_IN_USE 时才填。

### B. execute_items 接线

- ✅ deleter 返回 Err 且以 `FILE_IN_USE:` 开头 → 调 `who_is_locking(&path)`。
- ✅ who_is_locking 非空 → detail = `"file in use by: <app> (PID <pid>), ..."`，locking_processes 填结果。
- ✅ who_is_locking 空或失败 → detail 回退 `"file in use by another process (unable to identify process)"`。
- ✅ `PERMISSION_DENIED:` → 走 task-019 文案，**不调 who_is_locking**。
- ✅ 其他错误 → 原文兜底。

### C. 单测

- ✅ `execute_items_unlocked_file_no_locking_processes`：正常删除 → locking_processes 空。
- ✅ `execute_items_permission_denied_skips_who_is_locking`：MockDeleter 抛 PERMISSION_DENIED → locking_processes 空、detail 走 task-019 文案。
- ✅ `execute_items_locked_file_reports_processes`：#[ignore] notepad 端到端（GUI 不稳定）。

### D. 卫生与边界

- ✅ 未修改 SafeBoxDeleter / FsDeleter / who_is_locking / classify_io_error。
- ✅ 未修改 Tauri 命令签名（只是响应结构多一个字段）。
- ✅ 未新增依赖。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 119 passed / 0 failed / 1 ignored（117 已有 + 2 CI 新增，notepad #[ignore]） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0 |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **who_is_locking 失败时吞掉错误**：RM 失败时返回空 Vec + 回退文案。合理（不阻塞删除失败报告）。**第三档。**
2. **notepad 测试 #[ignore]**：DevAgent 已记录（GUI 不稳定）。CI 两条必跑。**第三档。**
3. **RmShutdown / RmRestart 未做**：请求关闭进程留 task-022。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ who_is_locking 是只读查询，不关闭进程；PERMISSION_DENIED 不触发 FFI。 |
| A03 注入 | ✅ path 来自 DB 反查，who_is_locking 内部用 PCWSTR 传递。 |
| A04 不安全设计 | ✅ RM 失败不阻塞删除失败报告，回退稳定文案；PERMISSION_DENIED 不调 who_is_locking（避免不必要的 FFI）。 |
| A08 数据完整性 | ✅ audit_log detail 写稳定文案 + 进程名；ExecResult.locking_processes 透传前端。 |

### 专项安全分析

- **会不会误关闭进程？** 不会。who_is_locking 只读查询，不调 RmShutdown / RmRestart。✅
- **PERMISSION_DENIED 会不会误触发 who_is_locking？** 不会。按前缀分流，PERMISSION_DENIED 走 task-019 文案。✅
- **who_is_locking 失败会不会崩溃？** 不会。`match` 失败分支回退空 Vec + 稳定文案。✅
- **前端能不能拿到进程列表？** 能。ExecResult.locking_processes 透传，serde 自动序列化。✅

## 评审结论：**通过**

FILE_IN_USE 自动接线落地：deleter 返回 FILE_IN_USE 时自动调 who_is_locking 查占用进程，拼进 audit_log detail（"file in use by: notepad.exe (PID 1234)"）并填到 ExecResult.locking_processes 返回前端；PERMISSION_DENIED 不触发 FFI；119 测试全过（含 2 个 CI 新增：正常删除无 locking / PERMISSION_DENIED 不触发）；clippy/fmt/pnpm build 全过；7 个提交全部关联 #task-021 无混杂。M1-D 文件占用模块闭环。

## Git 操作记录

- **merge commit**：`5fc8962`（`git merge --no-ff feature/task-021-locking-wired-to-executor`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-021-locking-wired-to-executor`，was cf130ea）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务021评审报告 (#task-021)`

## 后续动作建议

1. **M1-D 收官**：RmShutdown 请求关闭占用进程（可选，默认不做）。
2. **M1 下一阶段**：M1-E 规则更新 或 M1-F 报表。
3. **M1 收尾**：scan.listItems 命令、前端勾选 UI、打包时 data_dir 改 app_data_dir。
