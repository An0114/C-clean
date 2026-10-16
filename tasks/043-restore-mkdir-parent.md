# 任务编号：043 safe_box restore 自动建父目录——ParentMissing 时 mkdir -p 后再恢复

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevRustBackend
> 来源：`docs/PROJECT_STATE.md` 已知问题（task-017 restore 边界）——「不自动建原父目录（ParentMissing 直接报错让用户手动处理）」。用户恢复保险箱文件时，如果原父目录已被删（典型场景：清理了某个空文件夹后又想恢复其中被删的文件），当前直接报错，体验差。
> 上游依赖：task-017（`restore_item`）、task-039（`restore_many`）。
> **克制范围**：只改 restore 一处（父目录缺失时自动 mkdir -p），**不做** safe_box list 分页、**不做** rules 手动回滚命令（留 task-044+）。

## 状态字段

pending

## 任务目标

当前 `restore_item` 行为：
- 查 box_dir 文件存在；
- 拼 target_path = 原路径；
- `fs::rename(box_file, target_path)`；
- 若 target_path 父目录不存在 → rename 失败 → 返回 `Err(ParentMissing)`。

本任务改：
- rename 前先检查 target_path.parent() 是否存在；
- 不存在 → `fs::create_dir_all(parent)`（自动建多层父目录）；
- 建完再 rename；
- mkdir 失败（权限不足等）才报错。

**收益**：用户恢复文件时不用手动去资源管理器建空目录。

**本任务不做**：
- 不改 safe_box_list 分页逻辑；
- 不改前端 UI（按钮/文案不变）；
- 不做「恢复前 confirm 父目录会新建」的提示（自动行为，不打扰）；
- 不做 rules 手动回滚命令。

## 前置条件

1. `main` 分支已含 task-001~042 全部产出（含 2026-10-16 用户决策同步 commit `f30efb0`）；
2. 关键锚点：
   - `engine/src/execute/safe_box.rs`（task-017 `restore_item` / task-039 `restore_many`）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. 后端：restore_item 自动建父目录

1. 在 `engine/src/execute/safe_box.rs` 的 `restore_item`（或 `restore_from_safe_box`）里：
   ```rust
   let target = /* 从 DB 查的 original_path 拼出 */;
   if let Some(parent) = target.parent() {
       if !parent.exists() {
           fs::create_dir_all(parent).map_err(|e| format!("无法创建父目录 {}: {}", parent.display(), e))?;
       }
   }
   fs::rename(box_file, target)?;
   ```
2. 错误语义：
   - mkdir 失败（权限不足/磁盘满）→ 返回可读错误「无法创建父目录 <path>: <os error>」；
   - rename 仍失败（占用/权限）→ 保留原错误；
   - 成功 → 写 audit_log action='restore'，detail 可加 `created_dirs=true` 字段。

### B. 后端单测

3. 在 `engine/tests/safe_box_restore_mkdir.rs` 覆盖：
   - `restore_creates_missing_parent_dir`：tempdir 下模拟 safe_box 文件 + 原路径位于一个不存在的多层子目录 `a/b/c/file.txt` → restore 后 `a/b/c/` 目录存在 + file.txt 存在；
   - `restore_preserves_existing_parent_dir`：父目录已存在（里面放个无关文件）→ restore 后该无关文件仍在，不被覆盖；
   - `restore_fails_when_mkdir_permission_denied`：构造一个不可写父目录（如 tempdir 下用只读父目录路径），断言返回错误文案含「无法创建父目录」。

### C. 卫生

4. 不改 `purge_item` / `restore_many`（restore_many 内部调 restore_item，自动受益）；
5. 不改前端 UI；
6. 不改 audit_log schema（复用现有 action='restore'，detail 可选字段）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/safe_box.rs`（restore_item 加 mkdir -p）
- `engine/tests/safe_box_restore_mkdir.rs`（新建）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-043-restore-mkdir-parent`，从最新 `main` 拉出。
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-043-<描述>`，**只允许一层**；bugfix 修复后直接合并回 feature，不进 main。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `fix(execute): restore_item 在父目录缺失时自动 create_dir_all (#task-043)`
   —— 含 `engine/src/execute/safe_box.rs`。
2. `test(execute): 补 restore 自动建父目录单测（缺目录/已存在目录/权限拒绝） (#task-043)`
   —— 含 `engine/tests/safe_box_restore_mkdir.rs`。
3. 自验全绿后，最后一次提交：`docs(state): 任务043完成，safe_box restore 自动建父目录就绪 (#task-043)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 043: ...`；
   - 「最近一次变更」追加一行；
   - 「已知问题」段把 task-017 那条「不自动建原父目录」项标记已解决（保留原文 + 加 ~~删除线~~ 或迁到「已解决」小节）；
   - 「下一步计划」改为「派发 task-044：rules 手动回滚命令（从 .bak 恢复）——applyUpdate 已留 .bak，补回滚命令 + Settings 按钮」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-16  task-043  safe_box.restore_item 自动 mkdir -p 父目录（task-017 已知问题闭环）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 156 条 + 本任务 3 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0（前端零改动，但跑一遍确认未破）；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：`safe_box_restore_mkdir.rs` 3 条覆盖缺目录/已存在/权限拒绝。
- **集成测试（必做）**：`cargo test -p engine` 全绿，确保现有 156 条不回归。
- **端到端（不强制）**：本任务只改后端 restore 一处，不做新业务流程。
- **真机冒烟（不强制）**：
  - 后端纯逻辑改动，前端零改动；已有 task-039 真机证据覆盖 checkbox → restore 链路；
  - 裁剪理由：mkdir -p 是标准库调用，单测在 tempdir 已覆盖三分支，真机行为与单测一致，不强制重复。
- **打包安装冒烟（不强制）**：同上，前端零改动。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务**不命中**前端任务判定清单（前端零改动），真机/打包冒烟按裁剪规则不强制，CI 必做。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| A1-A2 restore_item 自动 mkdir + 错误文案 | `cargo test -p engine` 输出（含 safe_box_restore_mkdir 用例名 + passed） |
| B1-B3 三条单测 | `cargo test safe_box_restore_mkdir -- --nocapture` 完整输出 |
| C4-C6 不回归 | `cargo test -p engine` 全绿 + `pnpm test` 全绿 |
| 真机/打包冒烟 | 裁剪（前端零改动，理由见上） |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 2 个代码提交 + 1 个 `docs(state)` 提交，共 3 个 commit，全部 push 到 `feature/task-043-restore-mkdir-parent`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - mkdir 错误文案最终格式；
   - `cargo test -p engine` 通过输出；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（safe_box list 分页 / rules 手动回滚 / RmShutdown 留 task-044+）；
3. `docs/PROJECT_STATE.md` 已按第 3 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务043-restore 自动建父目录 (#task-043)`，
   该提交只含 `tasks/043-restore-mkdir-parent.md` 一个文件，落在 `feature/task-043-restore-mkdir-parent` 分支最前端。
