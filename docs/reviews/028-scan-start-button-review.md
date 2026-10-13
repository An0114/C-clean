# 测试评审报告

## 本次任务名称

028 前端 Scan 交互——「开始扫描」按钮 + listen('scan://progress') 实时进度条

## Git 提交记录核对结果

✅ 5 个提交全部关联 `#task-028`，type 前缀规范（docs / feat(frontend)），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `c935889` | docs(task) | tasks/028-scan-start-button.md | ✅ |
| 2 | `90eec5c` | feat(frontend) | Dashboard.tsx | ✅ |
| 3 | `783b45b` | docs(state) | PROJECT_STATE.md | ✅ |
| 4 | `42d71f2` | docs(task) | tasks/028-scan-start-button.md | ✅ |
| 5 | `8f8e8cc` | docs(state) | PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. Scan 按钮 + 进度条（Dashboard.tsx）

- ✅ 「开始扫描」按钮，scanning 时禁用。
- ✅ ScanStatus: idle | scanning | done | error。
- ✅ Progress: { session_id, percent, current_path, scanned_items }（真实 shape 对齐）。
- ✅ click → listen('scan://progress') → invoke('scan_start') → finally unlisten。
- ✅ 进度条：灰底 #e5e7eb + 绿底 #10b981，宽度 = percent%。
- ✅ 文字：current_path — scanned_items 项。
- ✅ done 后自动调 load() 刷新 stats。
- ✅ error 红条 + scanMsg。
- ✅ done 后按钮文案「再次扫描」。

### B. 边界

- ✅ 扫描中按钮 disabled。
- ✅ done 后按钮文案「再次扫描」。
- ✅ error 态按钮恢复可点（文案「重试扫描」）。

### C. 卫生与边界

- ✅ 不修改 Rust 代码。
- ✅ 不新增 npm 依赖。
- ✅ 不接 cleanup.execute。

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

1. **ProgressEvent shape 偏差**：任务书假设 { module, done, total }，实际是 { session_id, percent, current_path, scanned_items }。DevAgent 已按真实 shape 对接（进度条宽度 = percent，文字 = current_path + scanned_items）。**第二档口径，已登记。**
2. **提交粒度偏差**：任务书要求 3 个 feat commit（按钮 / listen 进度条 / 完成刷新 + 错误红条），DevAgent 合并为 1 个 feat commit。合理（同一文件强耦合）。**第二档口径，已登记。**
3. **无取消扫描**：后端无 cancel 命令。**第三档。**
4. **无扫描结果列表**：cleanup_item 表格留 task-030。**第三档。**
5. **tauri dev 真机未跑**：自动化环境无 GUI。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ scan_start 是后端已注册命令，前端只是触发；无越权操作。 |
| A03 注入 | ✅ 无用户输入拼路径；invoke 参数固定 {}。 |
| A04 不安全设计 | ✅ scanning 时按钮禁用防重复；finally unlisten 防内存泄漏；error 态有重试。 |
| A08 数据完整性 | ✅ 扫描完成后自动刷新 stats，不篡改后端数据。 |

### 专项安全分析

- **会不会重复触发扫描？** 不会。scanning 时按钮 disabled。✅
- **listen 会不会泄漏？** 不会。finally unlisten() 调用。✅
- **扫描失败会不会崩溃？** 不会。try/catch 捕获，error 态展示。✅
- **进度事件会不会伪造？** 不会。listen 只订阅后端 emit 的 scan://progress 事件。✅

## 评审结论：**通过**

Dashboard 开始扫描交互落地：「开始扫描」按钮 + scan_start invoke + listen('scan://progress') 实时进度条（percent 宽度 + current_path/scanned_items 文字）；scanning 时按钮禁用；done 后自动刷新 stats；error 红条 + 重试；133 测试全绿（后端无改动保持基线）；clippy/fmt/pnpm build 全过；5 个提交全部关联 #task-028 无混杂。前端从「只读展示」迈向「可执行操作」。

## Git 操作记录

- **merge commit**：`c1750a2`（`git merge --no-ff feature/task-028-scan-start-button`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-028-scan-start-button`，was 8f8e8cc）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务028评审报告 (#task-028)`

## 后续动作建议

1. **task-029**：cleanup.execute 交互——前端「清理已选」按钮，调 cleanup.execute Tauri 命令，成功后刷新 stats / history。
2. **task-030**：扫描结果列表——cleanup_item 表格（勾选 + 预览）。
3. **后续**：取消扫描命令、safe_box UI、rules 更新 UI。
