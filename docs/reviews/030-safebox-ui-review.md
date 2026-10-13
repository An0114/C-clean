# 测试评审报告

## 本次任务名称

030 保险箱 UI——SafeBox 表格 + 单条「恢复」按钮

## Git 提交记录核对结果

✅ 6 个提交全部关联 `#task-030`，type 前缀规范（docs / feat(frontend)），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `7bb625e` | docs(task) | tasks/030-safebox-ui.md | ✅ |
| 2 | `1f24104` | feat(frontend) | SafeBox.tsx | ✅ |
| 3 | `317258c` | feat(frontend) | App.tsx | ✅ |
| 4 | `c7c1882` | docs(state) | PROJECT_STATE.md | ✅ |
| 5 | `05bd0a6` | docs(task) | tasks/030-safebox-ui.md | ✅ |
| 6 | `5d42370` | docs(state) | PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. SafeBox 页面

- ✅ `invoke<SafeBoxListResp>('safe_box_list', {})`。
- ✅ state：entries / totalItems / totalBytes / restoringId / result。
- ✅ 表格 4 列：原路径 / 大小（formatBytes）/ 移入时间（formatTs）/ 操作。
- ✅ 顶部汇总：「共 N 项 / 总大小 X MB」。
- ✅ 「刷新」按钮。
- ✅ 「恢复」按钮：点击 → 该行按钮变「恢复中…」禁用；`invoke('safe_box_restore', { safeBoxId })`；成功 → 绿条「已恢复：originalPath」+ 重拉列表；失败 → 红条「恢复失败：<err>」，按钮恢复可点。
- ✅ 空列表：居中灰色「保险箱为空」。

### B. App.tsx 第四 tab

- ✅ tab 加「保险箱」，渲染 `<SafeBox />`。
- ✅ 四个 tab：仪表盘 / 清理 / 保险箱 / 历史。

### C. 卫生与边界

- ✅ 不修改 Rust 代码。
- ✅ 不新增 npm 依赖。
- ✅ 不接 purge / 批量恢复 / quota UI。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 137 passed / 0 failed（基线无回退，后端无改动） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0（Vite + tsc 通过） |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **响应 shape 偏差**：任务书假设 `invoke<SafeBoxItem[]>('safe_box_list')`，实际是 `SafeBoxListResp { entries, totalItems, totalBytes }`。DevAgent 已按真实 shape 对接（汇总用响应字段 totalItems/totalBytes，不再前端 sum）。**第二档口径，已登记。**
2. **字段名 camelCase**：任务书假设 `path / size_bytes / moved_at / original_path`，实际是 `originalPath / sizeBytes / quarantinedAt`。DevAgent 已对齐。**第二档口径，已登记。**
3. **restore 入参**：任务书假设 `item_id`，实际是 `safeBoxId`。DevAgent 已对齐。**第二档口径，已登记。**
4. **无 purge UI**：后端没命令，留 task-031。**第三档。**
5. **无批量恢复**：后端没批量命令，留 task-031。**第三档。**
6. **无 quota 进度条**：5GB FIFO 已在后端 enforce，UI 展示留 task-031。**第三档。**
7. **tauri dev 真机未跑**：自动化环境无 GUI。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ safe_box_list 是只读查询；safe_box_restore 是 task-017 已注册命令，有 TargetOccupied 防护（orig.exists() → 拒绝覆盖）。 |
| A03 注入 | ✅ 无用户输入拼路径；invoke 参数固定（safeBoxId 是 number）。 |
| A04 不安全设计 | ✅ restoringId 时所有恢复按钮禁用防重复；try/catch 捕获 invoke 失败；失败后按钮恢复可点。 |
| A08 数据完整性 | ✅ safe_box_restore 走 task-017 的 rename 回原路径流程，不篡改后端数据。 |

### 专项安全分析

- **会不会恢复到占用路径？** 不会。后端 task-017 有 TargetOccupied 防护（orig.exists() → 拒绝覆盖）。✅
- **会不会重复触发恢复？** 不会。restoringId 时所有恢复按钮 disabled。✅
- **恢复失败会不会崩溃？** 不会。try/catch 捕获，error 态展示。✅
- **会不会 XSS？** 不会。React 默认转义；originalPath 用 `{r.originalPath}` 渲染。✅

## 评审结论：**通过**

M1 安全网闭环打通：SafeBox 表格（原路径 + 大小 + 移入时间 + 恢复按钮）+ 顶部汇总（共 N 项 / 总大小）+ 单条恢复按钮（成功绿条 / 失败红条 + 自动重拉）+ App.tsx 第四 tab；137 测试全过（基线无回退，后端无改动）；clippy/fmt/pnpm build 全过；6 个提交全部关联 #task-030 无混杂。用户删错了能从保险箱恢复。

## Git 操作记录

- **merge commit**：`1434f72`（`git merge --no-ff feature/task-030-safebox-ui`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-030-safebox-ui`，was 5d42370）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务030评审报告 (#task-030)`

## 后续动作建议

1. **task-031**：rules 更新 UI——前端「检查更新」按钮 + 「应用更新」按钮，调 rules_checkUpdate / rules_applyUpdate。
2. **后续**：purge（永久删除）/ 批量恢复 / 5GB FIFO quota 进度条 / dry-run 预览 UI。
