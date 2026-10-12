# 任务编号：022 rules_meta 落 DB + rules.checkUpdate——查 GitHub Raw 比对版本（不下载）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRust
> 来源：`docs/PROJECT_STATE.md` 下一步计划 + task-005 已知问题段（「rules_meta 落 DB、GitHub Raw 联网拉取、rules.checkUpdate/rules.applyUpdate IPC 命令未做」）。
> 上游依赖：task-002（`rules_meta` 表 schema）、task-005（`rules_meta.json` 静态文件 + SHA-256 校验加载器）均已合并到 main。
> 立项决策 #4：「规则库 GitHub Raw+SHA-256」。
> **本任务范围克制**：只做「落 DB + 查 GitHub Raw 比对版本」，**不**下载替换（applyUpdate 留 task-023）。

## 任务目标

task-005 的规则库是仓库内静态文件，每次启动重新算 SHA-256，DB 里 `rules_meta` 表是空的，也不知道远端有没有新版本。本任务补这两块：

1. 启动时（或首次调用时）把当前 `rules_meta.json` 的内容写入 `rules_meta` 表（version / sha256 / downloaded_at / is_active）；
2. 新增 Tauri 命令 `rules.checkUpdate`：
   - HTTP GET `https://raw.githubusercontent.com/An0114/C-clean/main/rules/rules_meta.json`；
   - 解析远端 version / sha256；
   - 与 DB 当前 active 行比对；
   - 返回 `{ "has_update": bool, "current_version": ..., "remote_version": ..., "current_sha256": ..., "remote_sha256": ... }`；
   - **不**下载 cache_rules.json、**不**替换本地文件。

**本任务不做**：
- 不做 applyUpdate（下载新规则 + 替换）——留 task-023；
- 不做 GitHub Token / 私有仓库（公共仓库匿名拉取）；
- 不做失败重试 / 离线降级（直接返 INTERNAL）；
- 不做前端 UI（UI 提示"有更新"留后续）；
- 不做 RmShutdown（M1-D 可选遗留，本任务绕开）。

## 前置条件

1. `main` 分支已含 task-001~021 全部产出；
2. 关键锚点：
   - `engine/src/db/migrations/v1__init.sql`——`rules_meta` 表（version / sha256 / downloaded_at / is_active）；
   - `engine/src/rules/loader.rs`（task-005）——读本地 rules_meta.json；
   - 远程 URL：`https://raw.githubusercontent.com/An0114/C-clean/main/rules/rules_meta.json`（硬编码常量）。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. Engine 层：rules_meta 落 DB

1. 在 `engine/src/rules/` 新建 `meta_store.rs`：
   ```rust
   pub fn upsert_active_rules_meta(
       conn: &rusqlite::Connection,
       version: &str,
       sha256: &str,
   ) -> Result<(), String>;
   pub fn get_active_rules_meta(conn: &rusqlite::Connection) -> Result<Option<RulesMetaRow>, String>;
   ```
2. `upsert_active_rules_meta`：
   - 先 `UPDATE rules_meta SET is_active = 0 WHERE is_active = 1`；
   - 再 `INSERT INTO rules_meta (version, sha256, downloaded_at, is_active) VALUES (?, ?, unixepoch(), 1)`；
   - 幂等（重复调用不报错）。
3. Tauri 启动时（或首次调 rules.checkUpdate 前）调一次，把本地 rules_meta.json 的 version/sha256 写进去。

### B. Engine 层：远端比对

4. 引入 `reqwest`（blocking feature，因为 Tauri 命令里直接跑）：
   ```toml
   # engine/Cargo.toml
   [dependencies]
   reqwest = { version = "0.12", default-features = false, features = ["blocking", "json", "rustls-tls"] }
   ```
5. 在 `engine/src/rules/` 新建 `remote.rs`：
   ```rust
   pub const REMOTE_RULES_META_URL: &str = "https://raw.githubusercontent.com/An0114/C-clean/main/rules/rules_meta.json";
   #[derive(serde::Deserialize, Debug)]
   pub struct RemoteRulesMeta { pub version: String, pub sha256: String }
   pub fn fetch_remote_meta() -> Result<RemoteRulesMeta, String>;
   ```
6. `fetch_remote_meta`：
   - `reqwest::blocking::get(REMOTE_RULES_META_URL)?.error_for_status()?.json::<RemoteRulesMeta>()?`；
   - 失败返 `Err(String)`（网络不通 / JSON 错）。

### C. Tauri 命令 `rules.checkUpdate`

7. 在 `app/src-tauri/src/commands.rs`：
   ```rust
   #[tauri::command]
   async fn rules_check_update(app: tauri::AppHandle) -> Result<Envelope<CheckUpdateResp>, String>;
   ```
8. 内部：
   - 打开 DB；
   - `upsert_active_rules_meta(conn, local_version, local_sha256)`（如果 DB 空）；
   - `let current = get_active_rules_meta(conn)?`；
   - `let remote = fetch_remote_meta()?`（失败返 INTERNAL）；
   - `has_update = current.sha256 != remote.sha256`；
   - 返回 `Envelope::ok(CheckUpdateResp { ... })`。
9. 在 `lib.rs` 的 `generate_handler!` 注册 `rules_check_update`。

### D. 单测（`engine/tests/rules_meta_store.rs`）

10. 至少覆盖：
    - `upsert_then_get_active`：tempdir DB → upsert → get_active 返回同一行；
    - `upsert_new_version_deactivates_old`：upsert v1 → upsert v2 → get_active 返回 v2，DB 里 v1 is_active=0；
    - `get_active_empty_returns_none`：空 DB → None。
11. `fetch_remote_meta` **不写 CI 测试**（依赖网络 + GitHub），DevAgent 本地手动跑一次贴输出。

### E. 卫生与边界

12. 不下载 cache_rules.json（不调 applyUpdate）；
13. 不替换本地 rules/ 目录文件；
14. 不写前端 UI；
15. 网络错误时直接返 Err，不做重试 / 离线降级。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/Cargo.toml`（加 reqwest 依赖）
- `engine/src/rules/mod.rs`（加 `pub mod meta_store; pub mod remote;`）
- `engine/src/rules/meta_store.rs`（新建）
- `engine/src/rules/remote.rs`（新建）
- `engine/tests/rules_meta_store.rs`（新建）
- `app/src-tauri/src/commands.rs`（新增 `rules_check_update`）
- `app/src-tauri/src/lib.rs`（注册命令）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-022-rules-check-update`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `build(deps): engine 引入 reqwest（blocking + rustls-tls） (#task-022)`
   —— 含 `engine/Cargo.toml`、`Cargo.lock`。
2. `feat(rules): 新增 meta_store——upsert/get_active rules_meta 落 DB (#task-022)`
   —— 含 `engine/src/rules/meta_store.rs`、`engine/src/rules/mod.rs`。
3. `feat(rules): 新增 fetch_remote_meta——GET GitHub Raw rules_meta.json (#task-022)`
   —— 含 `engine/src/rules/remote.rs`。
4. `feat(tauri): 新增 rules.checkUpdate 命令（比对本地/远端 sha256） (#task-022)`
   —— 含 `app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`。
5. `test(rules): 补 meta_store 集成测试 (#task-022)`
   —— 含 `engine/tests/rules_meta_store.rs`。
6. 自验全绿后，最后一次提交：`docs(state): 任务022完成，rules.checkUpdate 就绪 (#task-022)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 022: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-023：rules.applyUpdate——下载新 cache_rules.json 并替换本地文件，SHA-256 校验通过后 upsert rules_meta 表」；
   - 「已知问题」段更新 task-005 那条（checkUpdate 已做，applyUpdate 留 task-023）；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-12  task-022  rules_meta 落 DB + rules.checkUpdate 查远端版本（不下载）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 119 条 + 本任务 3 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0；
- DevAgent 本地手动 `invoke('rules_check_update')` 看到真实响应（has_update / versions）。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 5 个代码提交 + 1 个 `docs(state)` 提交，共 6 个 commit，全部 push 到 `feature/task-022-rules-check-update`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - reqwest 引入方式（blocking vs async）；
   - fetch_remote_meta 本地实测输出（远端 version / sha256）；
   - rules_check_update 命令实测响应；
   - 自验命令与退出码；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（applyUpdate、前端 UI 提示、Token、重试留 task-023+）；
3. `docs/PROJECT_STATE.md` 已按第 6 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务022-rules.checkUpdate (#task-022)`，
   该提交只含 `tasks/022-rules-check-update.md` 一个文件，落在 `feature/task-022-rules-check-update` 分支最前端。
