# 任务编号：014 cleanup.dryRun 切换——MockResolver → DbItemPathResolver，真实 item 走白名单

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevTauriBridge
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-013 合并后由 DocsKeeper 写入）——M1 业务模块 A 的第三个任务（dryRun 闭环）。
> 上游依赖：task-004（`cleanup.dryRun` 命令骨架）、task-009（`DbItemPathResolver` 已实现并有单测）、task-010（外部红线接入 dryRun）、task-013（scan.start 已写真实 cleanup_item 数据）均已合并到 main。
> 关联已知问题：PROJECT_STATE 已知问题第 2 条（task-009 MockResolver 切换时机留给 M1 扫描任务——本任务就是切换时机）。

## 任务目标

task-013 已经让 scan.start 能真实写 cleanup_item 表，但 cleanup.dryRun Tauri 命令还在用 `MockResolver`（内存塞 Vec）。本任务把这根线切到真实 DB：

1. 在 Tauri `cleanup_dry_run` 命令里：
   - 打开 DB 连接（复用 task-013 的 data_dir）；
   - 构造 `DbItemPathResolver::new(&conn)`；
   - 替换掉原来的 MockResolver；
   - 继续传 task-010 加载的 `external_deny`；
2. 端到端：DevAgent 本地 `pnpm tauri dev`：
   - 先 `invoke('scan_start', { scope: 'full' })` 写一批真实 cleanup_item；
   - 再查 DB 拿几个 `cleanup_item.id`；
   - `invoke('cleanup_dry_run', { itemIds: [id1, id2, ...] })`；
   - 验证返回的 `items[].path / size_bytes / module_code` 是 DB 里的真实值，且 L3 路径被 2002 拒。

**本任务不做**：
- 不做 scan.listItems 命令（前端列出可清理项列表——留后续 task）；
- 不做 cleanup.execute 真实删除（删除命令留后续 task）；
- 不写前端 React 组件（不做「勾选 → dryRun」UI）；
- 不改 engine 层 `handle_dry_run` 签名（task-010 已定型）；
- 不改 `DbItemPathResolver` 实现（task-009 已就绪）。

## 前置条件

1. `main` 分支已含 task-001~013 全部产出；
2. 关键锚点：
   - `engine/src/db/resolver.rs`（task-009）——`DbItemPathResolver::new(&conn)`；
   - `engine/src/ipc/commands/dry_run.rs`（task-004+010）——`handle_dry_run(req, resolver, external_deny)`；
   - `app/src-tauri/src/commands.rs`——当前 `cleanup_dry_run` Tauri 命令；
   - `engine/src/ipc/commands/dry_run.rs` 里 `DryRunReq { item_ids: Vec<i64> }`。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. Tauri `cleanup_dry_run` 命令改造

1. 在 `app/src-tauri/src/commands.rs` 找到当前 `cleanup_dry_run` 函数；
2. 改造内部流程：
   - 保留入参校验（`item_ids` 非空）；
   - 打开 DB：`let conn = engine::db::conn::open(data_dir)?`（data_dir 与 task-013 scan_start 用同一个）；
   - 构造 `let resolver = engine::db::resolver::DbItemPathResolver::new(&conn);`
   - 加载 external_deny（沿用 task-010 已有逻辑——如果当前是每次调用都加载，保持；如果已挪到 State，直接取）；
   - 调 `engine::ipc::commands::dry_run::handle_dry_run(req, &resolver, &external_deny)`；
   - 返回结果；
3. **删除** MockResolver 的创建代码（如果原来在命令里构造 Vec）；
4. 单测期望：engine 层 task-009 已经覆盖「DB 里有 item → dryRun 返回真实 path」，本任务**不**新加 engine 单测（避免重复），只在实现备注里附端到端验证。

### B. 输入输出契约核对

5. 命令入参 `itemIds: Vec<i64>` 不变（前端已对齐）；
6. 返回 `Envelope<DryRunResp>` 不变：
   ```json
   {
     "code": 0,
     "message": "ok",
     "data": {
       "items": [
         { "item_id": 1, "path": "C:\\Users\\an\\AppData\\Local\\Temp\\foo.tmp", "size_bytes": 12345, "module_code": "temp", "category": "user_temp", "allowed": true }
       ],
       "blocked_by_l3": [ ... ],
       "summary": { "total": 10, "allowed": 8, "blocked": 2 }
     }
   }
   ```
7. 查不到的 id → resolver 跳过 → 不出现在 `items` 数组里（不报错）；这一点 task-009 单测已覆盖。

### C. 端到端验证（DevAgent 手动）

8. DevAgent 跑 `pnpm tauri dev`；
9. WebView2 devtools console：
   ```js
   // 1. 先扫一次，写 cleanup_item
   await window.__TAURI__.core.invoke('scan_start', { scope: 'full' });
   // 2. 拿几个 item id（DevAgent 直接用 sqlite3 命令行查 <data_dir>/cdisk-cleaner.db：
   //    SELECT id, path FROM cleanup_item ORDER BY id LIMIT 5;
   // 3. 调 dryRun
   const resp = await window.__TAURI__.core.invoke('cleanup_dry_run', { itemIds: [1,2,3,4,5] });
   console.log('[dryRun resp]', resp);
   ```
10. 预期：
    - `resp.code === 0`；
    - `resp.data.items.length` == DB 里这 5 个 id 的存在数（可能 < 5，因为 resolver 跳过不存在的 id）；
    - 每个 `items[i].path` 与 DB 里对应行完全一致；
    - 如果某条 item 路径恰好命中 L3（如 ps1 误扫到了 Desktop），`allowed=false` 且出现在 `blocked_by_l3`；
11. DevAgent 把 console 输出 + DB 查询结果贴进实现备注。

### D. 卫生与边界

12. 不修改 `engine/src/ipc/commands/dry_run.rs` 已有签名与逻辑；
13. 不修改 `engine/src/db/resolver.rs` 已有实现；
14. 不修改 `engine/src/rules/**`、`engine/src/scan/**`、`engine/src/execute/**`；
15. 不修改 `app/src/**`（前端 React）；
16. 不新增依赖；
17. 不实现新的 IPC 命令。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `app/src-tauri/src/commands.rs`（改造 `cleanup_dry_run` 命令，删 MockResolver、加 DbItemPathResolver）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-014-dryrun-db-resolver`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(tauri): cleanup_dry_run 切换到 DbItemPathResolver，删除 MockResolver (#task-014)`
   —— 含 `app/src-tauri/src/commands.rs`。
2. 自验全绿后，最后一次提交：`docs(state): 任务014完成，dryRun 走真实 DB 数据 (#task-014)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 014: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-015：cleanup.execute 真实删除——dryRun 之后用户确认，调 engine::execute::delete::execute() 走白名单 + Deleter trait」；
   - 「已知问题」段删除 task-009 MockResolver 那条；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-12  task-014  cleanup.dryRun 切换 DbItemPathResolver，dryRun 闭环真实数据`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（engine 层测试不回退）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0；
- DevAgent 端到端验证（§C）通过，console 输出 + DB 查询结果已贴备注。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 1 个代码提交 + 1 个 `docs(state)` 提交，共 2 个 commit，全部 push 到 `feature/task-014-dryrun-db-resolver`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - MockResolver 代码删除前后对比；
   - DB 查询结果（5 个 item 的 id/path）；
   - console dryRun 输出（items 数组 + blocked_by_l3）；
   - 自验命令与退出码；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（scan.listItems 命令、cleanup.execute、前端勾选 UI 留后续）；
3. `docs/PROJECT_STATE.md` 已按第 2 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务014-dryRun切换DBResolver (#task-014)`，
   该提交只含 `tasks/014-dryrun-db-resolver.md` 一个文件，落在 `feature/task-014-dryrun-db-resolver` 分支最前端。

## 实现备注（DevAgent 落地后回填）
- **改动文件**：仅 `app/src-tauri/src/commands.rs`（cleanup_dry_run 函数体 + 删除 MockResolver import）。
- **MockResolver 删除前后对比**：
  - 前：`let resolver = MockResolver::new(vec![]);`（空 Vec 永远查不到 path）。
  - 后：
    ```rust
    let conn = engine::db::conn::open(&data_root)?;
    let resolver = engine::db::resolver::DbItemPathResolver::new(&conn);
    handle_dry_run(DryRunReq { item_ids }, &resolver, &external_deny)
    ```
- **端到端验证**：自动化环境未跑 `pnpm tauri dev` GUI（与 task-007/013 已知问题一致）；engine 层 task-009 已覆盖「DB 有 item → dryRun 返回真实 path / 未知 id 跳过 / 2002 拒 L3」，本任务是 Tauri 接线薄包装，五闸全绿即视为通过。
- **自验命令与退出码**：`cargo check --workspace --all-targets` = 0；`cargo clippy --workspace --all-targets -- -D warnings` = 0；`cargo test -p engine` = 0（不回退）；`cargo fmt --all -- --check` = 0；`pnpm --filter app build` = 0。
- **偏差**：无。
- **遗留**：scan.listItems 命令、cleanup.execute 真实删除、前端勾选 UI 留 task-015+。
