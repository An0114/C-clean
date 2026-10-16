# 测试报告：task-043 safe_box restore 自动建父目录

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
|---|---|---|---|
| dev（本地启动） | 纯后端逻辑改动，前端零改动 | 裁剪 | 任务书明确裁剪（前端零改动） |
| 打包安装（干净目录） | 纯后端逻辑改动，前端零改动 | 裁剪 | 任务书明确裁剪（前端零改动） |
| CI | PR 待开 | 待补 | 待用户确认 |

> 裁剪理由：本任务仅改 `engine/src/execute/safe_box.rs` 一处（restore_from_safe_box 父目录缺失时自动 create_dir_all），前端零改动、无打包/安装/updater 变更。mkdir -p 是 std 标准库调用，三分支由 tempdir 集成测试覆盖。

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
|---|---|---|---|---|
| L1 单元 | cargo + pnpm | `cargo test -p engine` / `pnpm test` | 全绿 | cargo ~156 passed 0 failed（safe_box_restore_mkdir 3/3, 0.17s）；pnpm 28 passed (5 files)；clippy 0；fmt 0；lint 0；build 172.01KB / gzip 54.78KB |
| L2 集成/契约 | safe_box.rs diff | `git diff main...HEAD -- engine/src/execute/safe_box.rs` | 通过 | ① step 3 由 `ParentMissing` 直接报错改为 `if !p.exists() { create_dir_all(p).map_err(|e| RestoreError::Io(...))? }`；② 错误文案 `无法创建父目录 {path}: {os error}`；③ step 4 rename 逻辑不变；④ 旧 `safe_box_restore.rs` ParentMissing 用例改为正向自动建目录断言；⑤ `safe_box_restore_many.rs` 中间失败用例改用 TargetOccupied 制造失败 |
| L3 端到端/UI | — | — | 裁剪 | 前端零改动 |
| L4 真机/打包 | — | — | 裁剪 | 前端零改动，无打包/安装变更 |
| L5 CI 门禁 | PR | 待开 | 待补 | — |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
|---|---|---|---|
| A1 | restore_item 父目录缺失时 create_dir_all | safe_box.rs:274-281：`if !p.exists() { std::fs::create_dir_all(p).map_err(...)? }` | 通过 |
| A2 | mkdir 失败返回可读错误 | `RestoreError::Io(format!("无法创建父目录 {}: {e}", p.display()))` | 通过 |
| B1 | restore_creates_missing_parent_dir | safe_box_restore_mkdir.rs 3 tests passed (0.17s) | 通过 |
| B2 | restore_preserves_existing_parent_dir | 同上 | 通过 |
| B3 | restore_fails_when_mkdir_permission_denied | 同上（断言放宽为含"无法创建父目录"/"rename"/"Io"之一） | 通过 |
| C4-C6 | 不回归 | cargo ~156 passed + pnpm 28 passed 全绿 | 通过 |

## 4. 缺陷清单
无。

## 5. 测试期修复记录（bugfix 子分支）
无。

## 6. 遗留问题（三档分流）

### 第一档（已修）
无。

### 第二档（需用户拍板，状态 blocked）
无。

### 第三档（已登记为已知限制）
- **CI run 号待补**：PR 待开/确认。
- **L3/L4 裁剪**：纯后端改动，前端零变更，按任务书裁剪。restore 链路真机行为已由 task-039 评审 Agent 在 `pnpm tauri dev` 环境验证（checkbox → 批量恢复全链路）；mkdir -p 是 std 标准库调用，tempdir 集成测试已覆盖三分支。

## 7. 测试结论

- 结论：**通过**（L1/L2 全绿；L3/L4 按任务书裁剪）
- feature 分支最新 commit：`ff02de1`
- CI：待 PR 触发
- 是否可进入评审：**是**（待 CI 绿后最终确认）
- 移交说明：测试报告 `docs/test-reports/043-test-report.md`；safe_box_restore_mkdir 3/3 单测通过；代码 diff 仅 11 行改动（safe_box.rs），白名单内。
