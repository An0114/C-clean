# 任务编号：037 safe_box purge——单条永久删除（后端命令 + 前端红色按钮 + confirm）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-036 合并后写入）——M2 updater 闭环完成，回头清 M1 遗留。task-018 已知问题明确「无单条 purge 命令」。
> 上游依赖：task-016（safe_box FIFO + box_dir）、task-018（`safe_box_list` 命令已注册）、task-030（SafeBox 前端表格已就位）。
> **克制范围**：只做单条 purge，**不做**批量 purge、**不做**quota 自动清理（FIFO 已在 task-016 做）。

## 任务目标

目前保险箱里的旧文件只能「恢复」到原路径，不能彻底删掉。用户想清掉占空间的旧文件时没入口。本任务补最小 purge：

1. **后端**新增 `safe_box_purge(item_id: i64)` 命令：
   - 从 `safe_box` 表按 id 查行，拿 `box_path`；
   - `std::fs::remove_file(box_path)` 删磁盘文件（文件不存在不报错）；
   - `DELETE FROM safe_box WHERE id = ?`；
   - 写 audit_log action='purge'；
2. **前端** SafeBox 表格每行加「永久删除」按钮（红色，confirm 后才执行）：
   - 点击 → `window.confirm("确定永久删除该文件？此操作不可恢复。")`；
   - confirm 通过 → invoke `safe_box_purge` → 重拉列表；
   - 失败红条提示。

**本任务不做**：
- 不做批量 purge（多选 + 一键删除，留 task-038）；
- 不做 quota 自动清理（task-016 FIFO 已在做）；
- 不做 Trash 回收站（删了就真没了，confirm 已经兜底）；
- 不改 safe_box_list 逻辑。

## 前置条件

1. `main` 分支已含 task-001~036 全部产出；
2. 关键锚点：
   - `engine/src/execute/safe_box.rs`（task-016/017 已有 list/restore）；
   - `app/src-tauri/src/commands.rs`——`safe_box_list` / `safe_box_restore` 已注册；
   - `app/src/pages/SafeBox.tsx`（task-030 已有表格 + 恢复按钮）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. 后端：safe_box_purge

1. 在 `engine/src/execute/safe_box.rs` 加：
   ```rust
   pub fn purge_item(conn: &rusqlite::Connection, box_dir: &Path, item_id: i64) -> Result<(), String>;
   ```
2. 内部：
   - 查 `SELECT box_path FROM safe_box WHERE id = ?`；
   - 找不到 → `Err("safe_box item not found")`；
   - 拼 `box_dir.join(box_path)`，`fs::remove_file(...)`（文件不存在忽略错误）；
   - `DELETE FROM safe_box WHERE id = ?`；
   - 写 audit_log（action='purge', path=原路径, detail=box_path）。
3. Tauri 命令 `safe_box_purge` 接到 `app/src-tauri/src/commands.rs`，入参 `item_id: i64`，注册到 `lib.rs`。

### B. 后端单测

4. 在 `engine/tests/safe_box_purge.rs` 覆盖：
   - `purge_removes_file_and_db_row`：tempdir 造 box_dir + 写文件 + INSERT safe_box 行 → purge → 文件不存在、DB 无行；
   - `purge_missing_file_ok`：DB 有行但文件已被手动删 → purge 不报错，DB 行被删；
   - `purge_not_found_returns_err`：随机 id → Err。

### C. 前端：SafeBox 行加「永久删除」

5. `app/src/pages/SafeBox.tsx`：
   - 操作列加第二个按钮「永久删除」（红色，在「恢复」按钮旁边）；
   - 点击 → `window.confirm("确定永久删除该文件？此操作不可恢复。")`；
   - confirm 通过：
     - 该行按钮变「删除中…」禁用；
     - `invoke('safe_box_purge', { item_id: id })`；
     - 成功：重拉列表；
     - 失败：红条「永久删除失败：<err>」。

### D. 卫生

6. 不改 safe_box_list / safe_box_restore 逻辑；
7. 不改 FIFO quota 逻辑；
8. 不做批量。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/safe_box.rs`（加 purge_item）
- `engine/tests/safe_box_purge.rs`（新建）
- `app/src-tauri/src/commands.rs`（新增 `safe_box_purge`）
- `app/src-tauri/src/lib.rs`（注册命令）
- `app/src/pages/SafeBox.tsx`（加永久删除按钮）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-037-safebox-purge`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(execute): 新增 safe_box.purge_item——删磁盘文件 + DELETE DB 行 + audit_log (#task-037)`
   —— 含 `engine/src/execute/safe_box.rs`。
2. `feat(tauri): 新增 safe_box_purge 命令并注册 (#task-037)`
   —— 含 `app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`。
3. `test(execute): 补 safe_box_purge 集成测试（删文件+DB/缺文件/未找到） (#task-037)`
   —— 含 `engine/tests/safe_box_purge.rs`。
4. `feat(frontend): SafeBox 行加「永久删除」红色按钮 + confirm + 调用 purge (#task-037)`
   —— 含 `app/src/pages/SafeBox.tsx`。
5. 自验全绿后，最后一次提交：`docs(state): 任务037完成，safe_box purge 就绪 (#task-037)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 037: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-038：safe_box 批量恢复 / 批量 purge（多选 + 一键操作）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-14  task-037  safe_box.purge 单条永久删除（后端命令 + 前端红色按钮 + confirm）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 137+ 条 + 本任务 3 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-037-safebox-purge`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - `cargo test -p engine` 通过输出；
   - `pnpm build` 输出（无 error）；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（批量 purge / 批量恢复 / quota UI 留 task-038+）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务037-safe_box purge (#task-037)`，
   该提交只含 `tasks/037-safebox-purge.md` 一个文件，落在 `feature/task-037-safebox-purge` 分支最前端。
