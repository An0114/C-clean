# 任务编号：010 外部红线集注入——rules JSON 的 whitelist_deny 接入 cleanup.dryRun

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevTauriBridge（engine 纯逻辑 + app/src-tauri 薄包装）
> 来源：`docs/PROJECT_STATE.md` 待办 010（M1 前置清理清单第 4 条，2026-10-12 debt-cleanup 批次001 拆出）。
> 上游依赖：task-004（`engine::ipc::commands::dry_run::handle_dry_run` + `ItemPathResolver`）、task-005（`engine::rules::loader::load` + `external_deny_list` + `classify_with_external`）均已合并到 main。

## 任务目标

task-005 已经把「外部红线与内置 L3 取并集」的纯逻辑（`classify_with_external` / `external_deny_list`）和单测写好，但 task-004 的 `handle_dry_run` 当前只跑内置 `classify()`，外部红线**没有接线**。本任务把这根线接上：

1. 修改 `handle_dry_run` 签名，加一个 `external_deny: &[String]` 参数（已展开的绝对路径列表）；
2. 命令内部对每条 item 路径调 `classify_with_external(normalized, external_deny)`，不再调 `classify()`；
3. 在 `app/src-tauri/src/commands.rs` 的 `cleanup_dry_run` Tauri command 里：
   - 启动时（或每次调用时）用 `engine::rules::loader::load(<rules_dir>)` 加载仓库内 `rules/`；
   - 取出 `external_deny_list(&loaded)` 传给 `handle_dry_run`；
   - 加载失败时（checksum 不匹配等）**保守拒绝**：返回 5001 INTERNAL，不静默放行。

**本任务不做**：
- 不做联网拉取（`rules.checkUpdate` / GitHub Raw）；
- 不写 DB `rules_meta` 表；
- 不接 Tauri 资源路径解析（开发期直接用仓库根 `rules/` 相对路径，运行期路径留给后续打包任务）；
- 不改 task-005 已有 loader、`classify_with_external`、已有测试；
- 不改前端 React 代码。

## 前置条件

1. `main` 分支已含 task-001~009 全部产出；
2. `engine::rules::loader::load()`、`engine::rules::external_deny_list()`、`engine::execute::whitelist::classify_with_external()` 公开 API 可用；
3. `engine::ipc::commands::dry_run::handle_dry_run` 当前签名已稳定。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. 修改 `handle_dry_run` 签名

1. 在 `engine/src/ipc/commands/dry_run.rs`：
   - 函数签名从
     ```rust
     pub fn handle_dry_run(req: DryRunReq, resolver: &dyn ItemPathResolver) -> Envelope<DryRunResp>
     ```
     改为
     ```rust
     pub fn handle_dry_run(
         req: DryRunReq,
         resolver: &dyn ItemPathResolver,
         external_deny: &[String],
     ) -> Envelope<DryRunResp>
     ```
   - 函数内部把 `classify(path)` 换成 `classify_with_external(path, external_deny)`；
   - 已有单测（task-004 写的 8 条）**全部要改**：每个调用点传 `&[]` 作为 external_deny（保持原有测试语义不变，等价于「无外部红线」）。

### B. Tauri command 接线

2. 在 `app/src-tauri/src/commands.rs` 的 `cleanup_dry_run` 包装函数里：
   - 解析 rules 目录路径（本任务硬编码为仓库根 `rules/`，用 `std::env::current_dir()?.join("rules")`；DevAgent 加注释「M1 打包后改为资源目录」）；
   - 调 `engine::rules::loader::load(rules_dir)`：
     - 成功 → 拿 `external_deny_list(&loaded)`，传给 `handle_dry_run`；
     - 失败（checksum mismatch / IO / JSON 错）→ 返回 `Envelope::err(INTERNAL)`（code=5001），**不**返回 dryRun 结果；
   - 把加载逻辑包成一个小函数 `fn load_external_deny() -> Result<Vec<String>, ...>` 便于测试。

### C. 单测

3. 在 `engine/tests/ipc_dry_run_external.rs`（新建）：
   - `dry_run_external_deny_blocks_path`：
     - 用 `tempfile::tempdir()` 造临时 rules/ 目录，写 `cache_rules.json` 含 `whitelist_deny: ["%USERPROFILE%\\Videos"]`，写配对 `rules_meta.json`（真实 SHA-256）；
     - 调 `engine::rules::loader::load(rules_dir)` 拿 external_deny_list；
     - MockResolver 塞一条 `C:\Users\alice\Videos\movie.mp4`；
     - 调 `handle_dry_run(req, &mock, &external_deny)` → 断言 `code=2002`、`blocked_by_l3` 含该路径、`denial_reason` 非空；
   - `dry_run_no_external_deny_falls_back_to_builtin`：
     - `external_deny = &[]`，一条 `C:\Users\alice\Desktop\secret.docx` → 仍被内置红线拒（code=2002）；
   - `dry_run_builtin_still_blocks_when_rules_empty`：
     - 即使 rules 文件缺失（load 失败），纯内置红线仍能在 engine 单测里拒 L3（这一条用 `&[]` external 验证内置红线独立生效）。

### D. 卫生与边界

4. 不修改 `engine/src/rules/**`、`engine/src/execute/whitelist.rs` 已有函数与常量；
5. 不修改 `engine/src/db/**`、`engine/src/execute/delete.rs`、`engine/src/ipc/progress.rs`、`engine/src/ipc/envelope.rs`；
6. 不修改 `app/src/**`（前端 React）；
7. 不新增依赖；
8. 不写真实 Tauri 资源解析（TAURI 资源协议、`app.path()` 等）——本任务用 `current_dir()` 硬编码路径即可。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/ipc/commands/dry_run.rs`（改签名 + 换 classify 调用 + 已有测试调用点加 `&[]`）
- `app/src-tauri/src/commands.rs`（cleanup_dry_run 包装里加 rules 加载 + 传参）
- `engine/tests/ipc_dry_run_external.rs`（新建）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-010-external-deny-wiring`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(ipc): handle_dry_run 签名加 external_deny 参数，换 classify_with_external (#task-010)`
   —— 含 `engine/src/ipc/commands/dry_run.rs`（含已有测试调用点改）。
2. `feat(tauri): cleanup_dry_run 命令加载 rules/ 并注入外部红线，加载失败返 5001 (#task-010)`
   —— 含 `app/src-tauri/src/commands.rs`。
3. `test(ipc): 补外部红线接入 dryRun 的端到端单测 (#task-010)`
   —— 含 `engine/tests/ipc_dry_run_external.rs`。
4. 自验全绿后，最后一次提交：`docs(state): 任务010完成，外部红线集已接入 dryRun (#task-010)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 把待办 010 从 `- [ ]` 改为 `- [x]`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-011：ps1 兼容层来源对齐与 NDJSON 改造（scripts/ 五个脚本）」；
   - 「已知问题」段删除 task-005 外部红线未接线那条；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-12  task-010  外部 whitelist_deny 接入 cleanup.dryRun`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 60+ 条 + 本任务 3 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-010-external-deny-wiring`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 已有测试改 `&[]` 的清单；
   - rules 目录路径解析的临时方案（current_dir + 注释）；
   - 自验命令与退出码；
   - `cargo test -p engine` 通过输出；
   - 与本任务文档不符的偏差（如有）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务010-外部红线接入dryRun (#task-010)`，
   该提交只含 `tasks/010-external-deny-wiring.md` 一个文件，落在 `feature/task-010-external-deny-wiring` 分支最前端。

## 实现备注（DevAgent 落地后回填）
- **改动文件**：`engine/src/ipc/commands/dry_run.rs`（签名加 `external_deny: &[String]`、`classify`→`classify_with_external`）；`app/src-tauri/src/commands.rs`（`load_external_deny()` 辅助 + cleanup_dry_run 接线、失败返 `Envelope::err(INTERNAL)`=5001）；新建 `engine/tests/ipc_dry_run_external.rs`（3 用例）。
- **已有测试改 `&[]` 清单**：`engine/tests/ipc_commands.rs` 3 处（empty/all_pass/l3_path）；`engine/tests/db_resolver.rs` 1 处（dry_run_end_to_end）。
- **rules 目录临时方案**：`std::env::current_dir()?.join("rules")` 硬编码仓库根，代码注释已标「M1 打包后改为 Tauri 资源目录」。
- **自验退出码**：cargo check --workspace --all-targets=0；clippy -D warnings=0；cargo fmt --check=0；cargo test -p engine=0（新增 3/3）；pnpm --filter app build=0（✓ built in 1.29s）。
- **偏差**：rules 加载失败在 command 内转成 `Ok(Envelope::err(INTERNAL))`（信封内 5001），而非 `Err(String)`，符合任务「返 5001 INTERNAL」要求；`classify_with_external` 外部红线命中仍按既有约定记 `L3SystemPath`（已知问题段保留）。
- **遗留**：生产 rules 路径解析（TAURI 资源协议）、联网拉取、rules_meta 落 DB 均未做，留 M1 规则更新任务。
