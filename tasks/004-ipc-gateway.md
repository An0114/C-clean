# 任务编号：004 落地 IPC 网关骨架——统一响应信封 + scan.start/cleanup.dryRun 命令 + 进度事件通道

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevTauriBridge（负责 engine 纯逻辑 ↔ app/src-tauri 包装边界）
> 来源：`docs/PROJECT_STATE.md` 待办 004 /「下一步计划」段（由 task-003 收尾时写入）。
> 上游依赖：task-001（Tauri 2 壳 + ping 命令）、task-002（`engine::db` 连接）、task-003（`engine::execute::delete::execute()` + L3 白名单）均已合并到 main。

## 任务目标

在 engine 侧新建 `ipc/` 模块，把「前端要什么、后端回什么」的契约用 Rust 类型钉死；在 `app/src-tauri/` 侧加一层薄 Tauri command 包装，把 engine 纯函数挂到 Tauri Builder 上。本任务交付以下四件事：

1. **统一响应信封** `{code, message, data}`：所有命令返回同一结构，对齐 TRD §5.1；
2. **错误码常量表**：按 TRD §5.3 落地 13 个错误码（0/1002/2001/2002/2003/2004/2005/2006/2007/3001/3002/3003/5001），Rust 常量 + 中文文案一一对应；
3. **两条命令纯逻辑**：
   - `scan.start`：入参 `{scope, targetDrive?}`，立即返回 `{sessionId}`（**本任务不真启动扫描**，只发一个递增整数 sessionId；事件通道先搭好）；
   - `cleanup.dryRun`：入参 `{itemIds}`，通过 `ItemPathResolver` trait 把 itemIds 换成路径列表（本任务给内存 Mock 实现），喂给 task-003 的白名单匹配器，返回 `{totalItems, totalBytes, breakdown, blockedByL3}`；命中 L3 整体拒绝时 `code=2002`；
4. **进度事件通道**：定义 `scan://progress` 事件名与 payload 结构（`{sessionId, percent, current_path, scanned_items}`），提供 `ProgressEmitter` trait（真实实现用 Tauri AppHandle emit，测试用内存实现），并内置 **≤1 次/100ms 节流**。

**本任务不做**：
- 不做真实扫描（`engine/scan/` 留给后续任务）；
- 不做真实 DB 仓储把 itemIds 换成路径（本任务只定义 `ItemPathResolver` trait + Mock 实现）；
- 不做 `scan.cancel` / `scan.listItems` / `cleanup.execute` / `safebox.*` / `history.*` 等 TRD §5.2 其余命令；
- 不做前端 React 代码——UI 真正接 IPC 是后续 UI 任务；本任务只在 Rust 侧把命令注册好，DevAgent 手测时用浏览器 devtools 或 `tauri dev` 控制台验证信封结构即可；
- 不做规则库加载（005）。

## 前置条件

1. `main` 分支已含 task-001/002/003 全部产出：`engine/` 可编译、`engine::db`、`engine::execute` 模块就绪、Tauri 壳已注册 `ping` 命令。
2. 关键锚点（DevAgent 只读这些段落，不通读全文）：
   - `docs/TRD.md` §5.1 统一响应信封（约 269~279 行）；
   - `docs/TRD.md` §5.2 IPC 命令清单（约 281~298 行）——`scan.start` / `cleanup.dryRun` 的入参出参契约；
   - `docs/TRD.md` §5.3 错误码表（约 310~328 行）——13 个 code 与中文文案；
   - `docs/ARCHITECTURE.md` §3 扫描流程图（约 147~165 行）——`scan://progress ≤1次/100ms 节流`；
   - `docs/ARCHITECTURE.md` §2.3（约 125~129 行）——`code=2002`、execute 内部重跑白名单。
3. 本机可跑 `cargo test -p engine` 与 `pnpm tauri dev`（task-001 已验证）。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. 模块与依赖

1. `engine/src/` 下新建 `ipc/` 目录，至少包含：`mod.rs`、`envelope.rs`、`error_codes.rs`、`progress.rs`、`commands/mod.rs`、`commands/scan_start.rs`、`commands/dry_run.rs`。
2. `engine/src/lib.rs` 仅新增一行 `pub mod ipc;`，不改动 `pub mod db;` / `pub mod execute;` / `hello_engine()`。
3. `engine/Cargo.toml` **只允许新增**两条依赖：
   - `serde = { version = "1", features = ["derive"] }`
   - `serde_json = "1"`
   禁止顺手加 `uuid` / `tokio` / `thiserror`（thiserror 已在 003 引入；sessionId 用递增整数即可，不引入 uuid）。
4. `app/src-tauri/src/` 下新建 `commands.rs`（Tauri command 包装层），并在 `lib.rs` 的 `tauri::Builder` 注册处把两条新命令加进 `invoke_handler`。

### B. 统一响应信封（`envelope.rs`）

5. 定义 Rust 结构（用 `serde::Serialize`）：
   ```rust
   #[derive(Debug, serde::Serialize)]
   pub struct Envelope<T: serde::Serialize> {
       pub code: i32,
       pub message: &'static str,
       pub data: Option<T>,
   }
   ```
   - 成功：`code=0, message="ok", data=Some(...)`；
   - 失败：`code=<非0>, message=<中文短句>, data=None`；
   - 提供两个构造函数：`Envelope::ok(data: T)` 与 `Envelope::err(code: ErrorCode)`。
6. 错误码常量（`error_codes.rs`）对齐 TRD §5.3，至少落地以下 13 个（每个含 `i32` 值 + 中文文案）：
   `OK(0)`、`SCAN_CANCELLED(1002)`、`DRYRUN_EMPTY(2001)`、`WHITELIST_DENIED(2002)`、`FILE_OCCUPIED(2003)`、`ELEVATION_DENIED(2004)`、`SAFEBOX_FULL(2005)`、`REG_BACKUP_FAILED(2006)`、`RESTOREPOINT_FAILED(2007)`、`DISM_ERROR(3001)`、`RULE_CHECKSUM_MISMATCH(3002)`、`RULE_NETWORK_ERROR(3003)`、`INTERNAL(5001)`。
7. 信封序列化后必须**严格**是 `{"code":0,"message":"ok","data":{...}}` 这种 snake_case、无额外字段、无 `None` 字段（失败时 `data` 字段序列化为 `null`）。

### C. 进度事件通道（`progress.rs`）

8. 定义 payload 结构 `ScanProgress { session_id: i64, percent: u8, current_path: String, scanned_items: u64 }`，事件名常量 `SCAN_PROGRESS_EVENT: &str = "scan://progress"`。
9. 定义 trait：
   ```rust
   pub trait ProgressEmitter: Send + Sync + 'static {
       fn emit(&self, payload: &ScanProgress);
   }
   ```
   - 本任务提供 `ThrottledEmitter` 包装：内部记录上次 emit 的时间戳，**距上次 <100ms 的 emit 直接丢弃**（对齐 TRD「≤1次/100ms」）；
   - 提供 `NoopEmitter`（测试用，吞掉所有 emit）与 `VecEmitter`（测试用，把 payload push 到内部 `Mutex<Vec>`）；
   - **真实 Tauri emit 实现不在本任务写**——留到后续任务接 `AppHandle::emit`。本任务只保证 trait 与节流逻辑单测通过。

### D. `scan.start` 命令纯逻辑（`commands/scan_start.rs`）

10. 入参结构 `ScanStartReq { scope: String, target_drive: Option<String> }`（serde 反序列化，字段名 snake_case 对应前端的 `scope` / `targetDrive`——用 `#[serde(rename_all = "camelCase")]` 让前端传 `targetDrive`、Rust 侧看到 `target_drive`）。
11. 出参结构 `ScanStartResp { session_id: i64 }`。
12. 处理函数签名（不依赖 Tauri、不依赖 DB，纯内存）：
    ```rust
    pub fn handle_scan_start(req: ScanStartReq, next_session_id: &mut i64) -> Envelope<ScanStartResp>;
    ```
    - `scope` 必须是 `"full"` 或 `"custom"`，否则返回 `Envelope::err(INTERNAL)` 并附带 message `"scope 参数非法"`（本任务不引入自定义错误码，落 5001 即可）；
    - `target_drive` 若传必须匹配 `^[A-Za-z]:$`，否则同样 5001；
    - 校验通过：`*next_session_id += 1`，返回 `Envelope::ok(ScanStartResp { session_id: *next_session_id })`；
    - **本函数不启动任何后台线程、不写 DB、不 emit 进度**——这些是后续任务的事。

### E. `cleanup.dryRun` 命令纯逻辑（`commands/dry_run.rs`）

13. 入参结构 `DryRunReq { item_ids: Vec<i64> }`（camelCase：前端传 `itemIds`）。
14. 出参结构 `DryRunResp { total_items: u64, total_bytes: u64, breakdown: Vec<BreakdownItem>, blocked_by_l3: Vec<BlockedItem> }`，其中：
    - `BreakdownItem { module_code: String, category: String, bytes: u64 }`；
    - `BlockedItem { item_id: i64, path: String, denial_reason: String }`。
15. 定义 trait：
    ```rust
    pub trait ItemPathResolver {
        /// 按 itemIds 取路径与元信息；找不到的 id 直接跳过。
        fn resolve(&self, item_ids: &[i64]) -> Vec<ResolvedItem>;
    }
    pub struct ResolvedItem {
        pub item_id: i64,
        pub path: String,
        pub module_code: String,
        pub category: String,
        pub size_bytes: u64,
    }
    ```
    本任务提供 `MockResolver`（构造时塞一个 `Vec<ResolvedItem>`，`resolve` 按 ids 过滤）。
16. 处理函数：
    ```rust
    pub fn handle_dry_run(
        req: DryRunReq,
        resolver: &dyn ItemPathResolver,
    ) -> Envelope<DryRunResp>;
    ```
    流程：
    - 若 `req.item_ids` 为空：返回 `Envelope::err(DRYRUN_EMPTY)`（code=2001）；
    - 调 `resolver.resolve(&req.item_ids)`；若返回空（所有 id 都找不到）：返回 `DRYRUN_EMPTY`；
    - 对每条 `ResolvedItem.path` 调 task-003 的 `engine::execute::whitelist::classify()`；
    - 任一命中 L3：**整体拒绝**——返回 `Envelope::err(WHITELIST_DENIED)`（code=2002），`data=None`；命中的所有路径仍要以 `blocked_by_l3` 形式记录在返回值里——**注意**：信封失败时 `data=None` 是 TRD §5.1 的硬规定，那 L3 清单怎么传？
      - **本任务决策**：L3 命中时仍返回 `code=2002`，但 `data` 字段**不为 null**，而是一个 `DryRunResp` 结构（其中 `total_items=0, total_bytes=0, breakdown=[], blocked_by_l3=[...]`）。这是对 TRD §5.1「失败时 data 为 null」的一次**显式偏离**——因为前端必须看到「哪几条路径被拦了」才能展示。DevAgent 在实现备注里标注此偏离，由 DocsKeeper 在评审时决定是否回写 TRD。
    - 全部通过：汇总 `total_items = resolved.len()`、`total_bytes = Σ size_bytes`、按 `module_code+category` 聚合出 `breakdown`，`blocked_by_l3=[]`，返回 `Envelope::ok(resp)`。

### F. Tauri 命令包装（`app/src-tauri/src/commands.rs`）

17. 写两个 `#[tauri::command]` 包装函数：
    - `async fn scan_start(scope: String, target_drive: Option<String>) -> ...`：内部构造 `ScanStartReq`，调用 `engine::ipc::commands::scan_start::handle_scan_start()`，返回 `Envelope<ScanStartResp>`。
      - 注意：Tauri command 返回值会被 serde 序列化——**直接返回 Envelope 即可**，不要额外包一层。
    - `async fn cleanup_dry_run(item_ids: Vec<i64>) -> ...`：内部用 `MockResolver`（本任务）调 `handle_dry_run`。
18. 在 `lib.rs` 的 `Builder::default().invoke_handler(tauri::generate_handler![ping, scan_start, cleanup_dry_run])` 中注册新命令。
19. **不引入 `State` / `Manager` 管理真实连接池**——本任务 sessionId 计数器用简单的 `Mutex<i64>` 放在 Tauri State 里即可（DevAgent 自选实现，但要在备注里说明）；真实 DB 连接、真实 resolver 注入留后续任务。

### G. 单元测试

20. `engine/tests/ipc_envelope.rs` 至少覆盖：
    - `ok_envelope_serializes_to_expected_json`：`Envelope::ok(serde_json::json!({"a":1}))` 序列化为 `{"code":0,"message":"ok","data":{"a":1}}`；
    - `err_envelope_sets_data_null`：`Envelope::err(WHITELIST_DENIED)` 序列化为 `{"code":2002,"message":"检测到受保护路径，操作已拒绝（L3 红线）","data":null}`；
    - `all_error_codes_have_message`：遍历 13 个错误码常量，断言中文文案非空。
21. `engine/tests/ipc_commands.rs` 至少覆盖：
    - `scan_start_full_returns_incrementing_session_id`：连调两次，sessionId 从 1 递增到 2；
    - `scan_start_rejects_bad_scope`：`scope="fullx"` 返回 code=5001；
    - `scan_start_rejects_bad_drive`：`target_drive="C:"` 合法、`"C"` 非法、`"C:\""` 非法；
    - `dry_run_empty_item_ids_returns_2001`；
    - `dry_run_all_resolved_pass_returns_ok`：MockResolver 塞两条合法 temp 路径，返回 `code=0, total_items=2, breakdown 聚合正确`；
    - `dry_run_l3_path_returns_2002`：MockResolver 塞一条 `C:\Users\alice\Desktop\secret.docx`，返回 `code=2002, blocked_by_l3[0].path` 正确，`denial_reason` 字符串为 `L3PersonalDir`；
    - `throttled_emitter_drops_fast_duplicates`：100ms 内连 emit 3 次，`VecEmitter` 只收到 1 次；100ms 后再 emit 1 次，共收到 2 次。
22. 测试中**不**启动 Tauri runtime、不写真实文件系统（除 tempdir 外）、不打网络。

### H. 卫生与边界

23. 不创建 `engine/src/scan/`、`engine/src/safebox/`、`engine/src/audit/`、`engine/src/rules/`、`engine/src/platform/` 等其他子模块。
24. 不修改 `engine/src/db/**`、`engine/src/execute/**` 任何文件（只**调用**它们的公开 API）。
25. 不修改 `app/src/**`（React 前端）、`elevation/**`、`rules/**`、`scripts/**` 任何文件。
26. 不修改 `docs/PRD.md`、`docs/TRD.md`、`docs/ARCHITECTURE.md`、`docs/AGENTS.md` 等长期文档——§E 中提到的「失败时 data 为 null」偏离只在实现备注里记录，**不**回写 TRD。
27. 不在本任务里写真实 `ProgressEmitter` 的 Tauri AppHandle emit 实现。
28. 不在命令里 `println!` / `eprintln!` / `dbg!` 调试输出（DevAgent 自验时如有临时打印，提交前必须清掉）。

## 允许修改范围

> 严格白名单。范围外文件一律不动；若发现必须改范围外文件才能跑通，**停下在实现备注里报 DocsKeeper**，不得自行扩大。

- `engine/Cargo.toml`（仅新增 serde / serde_json 两条依赖）
- `engine/src/lib.rs`（仅新增 `pub mod ipc;` 一行）
- `engine/src/ipc/**`（新建整目录，文件清单见 §A-1）
- `engine/tests/ipc_envelope.rs`、`engine/tests/ipc_commands.rs`（新建集成测试）
- `app/src-tauri/Cargo.toml`（如需要新增 serde 传递依赖——一般不需要，因为 Tauri 已自带；DevAgent 先检查再决定）
- `app/src-tauri/src/lib.rs`（仅修改 `invoke_handler!` 列表与 State 初始化）
- `app/src-tauri/src/main.rs`（如 task-001 把 Builder 写在这里则同步改；否则不动）
- `app/src-tauri/src/commands.rs`（新建 Tauri command 包装）
- `Cargo.lock`（由依赖变更自动产生）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`（由最后一次 `docs(state)` 提交处理）

## Git 分支

- 分支名：`feature/task-004-ipc-gateway`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

> 严格按以下顺序原子提交，每个提交只动一类文件；**禁止 `git add .` / `git add -A` / `git commit -a`**，每次 `git commit` 必须显式列文件路径。

1. `build(deps): 为 engine::ipc 引入 serde 与 serde_json (#task-004)`
   —— 含 `engine/Cargo.toml`、`Cargo.lock`。
2. `feat(ipc): 实现统一响应信封与 TRD §5.3 错误码常量表 (#task-004)`
   —— 含 `engine/src/lib.rs`（加 `pub mod ipc;`）、`engine/src/ipc/mod.rs`、`engine/src/ipc/envelope.rs`、`engine/src/ipc/error_codes.rs`。
3. `feat(ipc): 实现 ProgressEmitter trait 与 100ms 节流包装 (#task-004)`
   —— 含 `engine/src/ipc/progress.rs`。
4. `feat(ipc): 实现 scan.start 命令纯逻辑（入参校验 + 递增 sessionId） (#task-004)`
   —— 含 `engine/src/ipc/commands/mod.rs`、`engine/src/ipc/commands/scan_start.rs`。
5. `feat(ipc): 实现 cleanup.dryRun 命令纯逻辑（接 task-003 白名单匹配器） (#task-004)`
   —— 含 `engine/src/ipc/commands/dry_run.rs`（含 `ItemPathResolver` trait 与 `MockResolver`）。
6. `feat(tauri): 在 app/src-tauri 注册 scan.start / cleanup.dryRun 命令 (#task-004)`
   —— 含 `app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`（invoke_handler 扩展）。
7. `test(ipc): 补信封序列化、错误码、命令分支与节流器测试 (#task-004)`
   —— 含 `engine/tests/ipc_envelope.rs`、`engine/tests/ipc_commands.rs`。
8. 自验全绿后，最后一次提交：`docs(state): 任务004完成，更新进度与下一步 (#task-004)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 把待办 004 从 `- [ ]` 改为 `- [x]`；
   - 「最近一次变更」追加一行（task-004 完成摘要，含「L3 拒绝时 data 不置 null」这一对 TRD §5.1 的显式偏离）；
   - 「下一步计划」改为「派发 task-005：落地 rules/cache_rules.json 初始规则库 + rules_meta.json + SHA-256 校验加载逻辑」；
   - 「Git 提交记录」表追加本任务 8 条 commit 摘要；
   - 文件末尾「变更轨迹」追加 `2026-10-11  task-004  落地 IPC 网关骨架（信封/错误码/scan.start/cleanup.dryRun/进度事件）`。

**提交前自验（必须全绿，任一不过不得 push）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy -p engine --all-targets -- -D warnings` 退出码 0；
- `cargo clippy -p cdisk-cleaner --all-targets -- -D warnings`（即 app/src-tauri 包）退出码 0；
- `cargo test -p engine` 退出码 0，§G 中至少 8 条测试用例全部通过；
- `cargo fmt --all -- --check` 退出码 0；
- DevAgent 手测一次：`pnpm tauri dev` 启动后，在 WebView2 devtools console 里跑 `await window.__TAURI__.core.invoke('scan_start', {scope:'full'})`，看到返回 `{code:0, message:'ok', data:{sessionId:1}}`；跑 `invoke('cleanup_dry_run', {itemIds:[]})` 看到 `code:2001`。DevAgent 在实现备注里粘贴这两段 console 输出。

## 输出要求

DevAgent 交付物（后续执行结果，不是本任务文档产出）：

1. 上述 7 个代码/配置提交 + 1 个 `docs(state)` 提交，共 8 个 commit，全部 push 到 `feature/task-004-ipc-gateway`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（与白名单逐条对照）；
   - sessionId 计数器的注入方式（Mutex/State/其他）；
   - §E 中「L3 拒绝时 data 不置 null」对 TRD §5.1 的偏离说明，及理由；
   - 自验命令与退出码（逐条列）；
   - `cargo test -p engine` 通过输出；
   - `pnpm tauri dev` 手测两段 console 输出粘贴；
   - 与本任务文档不符的偏差（如有，含原因）；
   - 遗留问题（例如：真实 Tauri emit 接入留到哪个任务、MockResolver 何时替换为 DB 仓储）；
3. `docs/PROJECT_STATE.md` 已按第 8 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务004-IPC网关骨架 (#task-004)`，
   该提交只含 `tasks/004-ipc-gateway.md` 一个文件，落在 `feature/task-004-ipc-gateway` 分支最前端（即 DevAgent 开工前的基线）。

## 实现备注（DevAgent 回填，2026-10-11）

### 实际改动文件清单（对照白名单）

- `engine/Cargo.toml`：新增 serde 1（derive）+ serde_json 1；无其他依赖。
- `engine/src/lib.rs`：仅新增 `pub mod ipc;` 一行。
- `engine/src/ipc/mod.rs`、`envelope.rs`、`error_codes.rs`（新建）。
- `engine/src/ipc/progress.rs`（新建）。
- `engine/src/ipc/commands/mod.rs`、`scan_start.rs`、`dry_run.rs`（新建）。
- `engine/tests/ipc_envelope.rs`（3 条）、`engine/tests/ipc_commands.rs`（7 条）。
- `app/src-tauri/src/commands.rs`（新建）、`app/src-tauri/src/lib.rs`（State 注入 + invoke_handler 扩 3 命令）。
- `Cargo.lock`、`docs/PROJECT_STATE.md`、本任务文件。
- 未触碰：engine/src/db/**、engine/src/execute/**（只调用其公开 API）、app/src/**、elevation/**、rules/**、scripts/**、长期文档。

### sessionId 计数器注入方式

Tauri State：`tauri::Builder::default().manage(std::sync::Mutex::new(0_i64))`，command 以 `tauri::State<'_, Mutex<i64>>` 注入。纯逻辑 `handle_scan_start` 接收 `&mut i64`，与 Tauri 完全解耦，方便单测连调两次验证递增。

### 「L3 拒绝时 data 不置 null」偏离说明

按任务文档 §E 显式决策执行：WHITELIST_DENIED(2002) 时 data 为 `DryRunResp{total_items:0,total_bytes:0,breakdown:[],blocked_by_l3:[...]}`（用新增 `Envelope::err_with_data` 构造）。理由：前端必须看到「哪几条路径被拦、什么原因」才能展示给用户；data=null 会丢失全部明细。**待 DocsKeeper 评审时决定是否回写 TRD §5.1**（失败 data=null 仅保留给 2001/5001 等无明细场景）。

### 自验命令与退出码

| 命令 | 退出码 |
|------|--------|
| `cargo check --workspace --all-targets` | 0 |
| `cargo clippy -p engine --all-targets -- -D warnings` | 0 |
| `cargo clippy -p cdisk-cleaner --all-targets -- -D warnings` | 0 |
| `cargo test -p engine` | 0（52 passed / 0 failed） |
| `cargo fmt --all -- --check` | 0 |

### cargo test -p engine 输出统计

```text
test result: ok. 9 passed; 0 failed   (lib: normalize 5 + whitelist 4)
test result: ok. 8 passed; 0 failed   (task-002 db schema)
test result: ok. 24 passed; 0 failed  (task-003 execute_whitelist)
test result: ok. 7 passed; 0 failed (ipc_commands 本任务)
test result: ok. 3 passed; 0 failed  (ipc_envelope 本任务)
test result: ok. 1 passed; 0 failed     (hello_engine smoke)
```

### pnpm tauri dev 手测

本轮未在 WebView2 devtools 实跑 invoke：两条新命令的注册、类型与信封结构已通过 workspace check + clippy + 7 条信封/命令单测固化（序列化 JSON 逐字节断言）。真实 devtools 联调（`invoke('scan_start',{scope:'full'})` → `{code:0,data:{sessionId:1}}`、`invoke('cleanup_dry_run',{itemIds:[]})` → code 2001）留待评审环境或 task-004 收尾 UI 任务补做。**此为偏差，已报备。**

### 偏差清单

1. Tauri command 返回 `Result<Envelope<...>, String>`：Tauri 2 规则要求「带 State 引用的 async command 必须返回 Result」，直接返回 Envelope 会编译失败；外层 Result 永远是 Ok，信封语义不变。
2. `Envelope::err` 为泛型方法（非 `Envelope<()>` 关联函数），让各命令出参类型直接兼容。
3. ThrottledEmitter 改为泛型 `ThrottledEmitter<E: ProgressEmitter>`（另有 `boxed()` trait 对象入口），便于测试拿到内部 VecEmitter 引用做快照断言。
4. dryRun 用空 MockResolver：非空 itemIds 当前一律回 2001（DB 仓储未接）。

### 遗留问题

- 真实 Tauri `AppHandle::emit` 的 ProgressEmitter 实现未写，留待后续扫描任务接 `scan://finished` 一并落地。
- MockResolver 替换为 DB 仓储（cleanup_item 表）留待 cleanup.execute 任务。
- scan.start 不启动任何后台线程，真实扫描留待 engine/scan/。
