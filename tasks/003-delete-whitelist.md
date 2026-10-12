# 任务编号：003 落地删除唯一入口 engine::execute::delete::execute() + L3 红线白名单匹配器

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRust
> 来源：`docs/PROJECT_STATE.md` 待办 003 /「下一步计划」段（由 task-002 收尾时写入）。
> 上游依赖：task-002 已合并到 main（`engine::db` 模块、rusqlite 连接、迁移器就绪）。

## 任务目标

在 `engine/` 内新建 `execute/` 子模块，落地 **删除动作的代码级唯一入口** 与 **L3 红线白名单匹配器**，让任何来源（UI / CLI / ps1 返回 / 提权回包）要删文件时都必须经过同一个函数。本任务交付以下四件事：

1. **路径规范化器**：把任意字符串路径统一成 Windows 绝对路径、反斜杠、大小写不敏感形式，并在规范化阶段拒绝 `..` 穿越与异常前缀；
2. **内置 L3 红线路径匹配器**：按 PRD P0-7 硬编码三类 L3 路径（用户个人目录 / 浏览器个人数据 / 系统关键路径）+ 一类动作级禁令（WinSxS `/ResetBase`），全部命中即返回结构化拒绝原因；
3. **唯一入口 `engine::execute::delete::execute()`**：接收待删路径列表 → 规范化 → 跑内置红线 → 重跑外部红线集（trait 注入，本任务先为空实现）→ 返回整体决策报告；
4. **单元测试**：对 PRD P0-7 列出的所有 L3 路径与典型穿越变体 **100% 拦截**，对典型 L1 缓存路径 **100% 放行**。

**关于模块路径的对齐说明**：PROJECT_STATE 待办字面写 `engine::delete::execute()`，ARCHITECTURE §1.x 目录树写 `engine/execute/  # 执行引擎（删除唯一入口 delete::execute + 白名单匹配器）`。本任务以 **ARCHITECTURE 目录树为准**——模块目录为 `engine/src/execute/`，其中 `delete.rs` 暴露 `pub fn execute(...)`，最终调用路径为 `engine::execute::delete::execute()`。后续 004/005/业务模块都按此路径引用。

**本任务不做**：
- 不做真实文件删除（`std::fs::remove_file` 等）——真实 deleter 由后续 safebox/platform 任务通过 `Deleter` trait 注入；本任务只提供 `NoopDeleter`（测试用，记录"本应被删"的路径，不碰磁盘）；
- 不做审计写 `audit_log` 表——审计模块是 `engine/audit/` 的事，本任务只在返回报告中带上每条拒绝的 `denial_reason`，由上层（004 IPC / 后续 audit 任务）落库；
- 不接规则库红线（005 才加载 `rules/cache_rules.json`），本任务只把「外部红线集」做成 trait 边界，005 落地时往里塞；
- 不做 IPC 暴露（004 才做 Tauri command）；
- 不做保险箱移入、占用检测、还原点创建——这些是后续业务任务。

## 前置条件

1. `main` 分支已含 task-002 全部产出：`engine/` 可编译、`engine::db` 模块存在。
2. 关键锚点（DevAgent 只读这些段落，不通读全文）：
   - `docs/PRD.md` P0-7「L3 红线硬拦截」（约 174~187 行）——三类路径 + 动作级禁令的权威清单；
   - `docs/PRD.md` §5.1 风险分级表（约 382~387 行）——L0/L1/L2/L3 语义；
   - `docs/ARCHITECTURE.md` §2.3 公共能力表（约 123~132 行）——`code=2002`、`audit_log: whitelist_denied`、execute 内部重跑白名单；
   - `docs/ARCHITECTURE.md` §4 末段（约 317~323 行）——「L3 记录永不写入 cleanup_item」；
   - `docs/ARCHITECTURE.md` §6 A01/A03（约 440/442 行）——路径规范化防 `..` 穿越、二次校验。
3. 本机可跑 `cargo test -p engine`（task-002 已验证）。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. 模块与依赖

1. `engine/src/` 下新建 `execute/` 目录，至少包含：`mod.rs`、`normalize.rs`、`whitelist.rs`、`delete.rs`。
2. `engine/src/lib.rs` 仅新增一行 `pub mod execute;`，不改动 `pub mod db;` 与 `hello_engine()`。
3. `engine/Cargo.toml` **只允许新增** `thiserror = "1"` 一条（用于错误与拒绝原因枚举）；
   - 禁止顺手加 `serde` / `regex` / `walkdir` 等其他依赖；
   - L3 路径匹配**不允许引入 regex 依赖**，用 `std::path::Path` 前缀匹配 + 字符串比较即可（性能足够，且避免 regex 注入面）。

### B. 路径规范化（`normalize.rs`）

4. 暴露公开函数（建议签名）：`pub fn normalize(raw: &str) -> Result<NormalizedPath, NormalizeError>`；
   - 输入可能是：`C:\Windows\Temp\a.log`、`C:/Windows/Temp/a.log`、`C:\Windows\Temp\..\System32\x.dll`、`\\?\C:\...`、相对路径、空串；
   - 输出是一个 newtype 包装的绝对路径（反斜杠、无尾部、无 `..`、无 `.`、统一大写盘符），并在内部保留「原始输入」与「规范化结果」以便审计日志记录；
   - 拒绝以下输入并返回 `NormalizeError`：
     - 含 `..` 且规范化后逃逸到盘符根之外（例如 `C:\Windows\..\..\evil`）；
     - **口径回写（debt-cleanup 批次001）**：实际实现为「见到任何 `..` 组件即一律拒绝」，不做词法折叠——删除网关上游传入全是已解析绝对路径，`..` 无合法用途，一律拒最安全；E-14 用例 `Windows\Temp\..\..\Users\...` 即此语义。
     - 非绝对路径（未带盘符 / UNC 之外的相对路径）；
     - 空串、纯空白、含 NUL；
     - 本任务**不**解析符号链接（`std::fs::symlink_metadata` 留到后续 platform 模块）——规范化只处理词法层面的 `..` / `.` / 斜杠。
5. 路径比较一律大小写不敏感（Windows 文件系统语义）：`C:\Windows` 与 `c:\WINDOWS` 视为同一前缀。

### C. 内置 L3 红线匹配器（`whitelist.rs`）

6. 暴露枚举：
   ```rust
   pub enum DenialReason {
       L3PersonalDir,      // 桌面/文档/下载/图片/视频/音乐
       L3BrowserProfile,   // 浏览器 User Data 主目录
       L3BrowserSensitive, // 书签/密码/历史记录文件
       L3SystemPath,       // System32 / Windows\Installer / pagefile.sys / swapfile.sys
       L3ReservedAction,   // WinSxS /ResetBase 等动作级禁令
       PathTraversal,      // 规范化阶段已拒（不应出现在 execute 返回里，仅作保留）
   }
   ```
7. 内置 L3 列表（**硬编码**，不读 JSON；005 加载外部规则后与之取并集，但**本内置列表永不被外部规则移除**，对应 ARCHITECTURE R3「规则更新不得移除内置红线」）：
   - **用户个人目录**（按 known folder 语义，用可识别的文件夹名匹配，不写死 `C:\Users\<name>`）：
     - 任意盘符下任意用户 profile 内的 `Desktop` / `Documents` / `Downloads` / `Pictures` / `Videos` / `Music`；
     - 同时匹配中文本地化文件夹名：`桌面` / `文档` / `下载` / `图片` / `视频` / `音乐`（Windows 显示名与真实文件系统名可能不同，本任务以**文件系统真实名**为主，中文名作为补充匹配，DevAgent 在备注里说明取舍）。
   - **浏览器个人数据**：
     - Chrome / Edge / Firefox 的 `User Data` / `Profiles` 主目录（前缀匹配，命中即拒）；
     - 任意路径下名为 `Bookmarks` / `Login Data` / `History` 的文件（精确文件名匹配，不分父目录）。
   - **系统关键路径**：
     - `<盘符>:\Windows\System32` 及其任意后代；
     - `<盘符>:\Windows\Installer` 及其任意后代；
     - `<盘符>:\pagefile.sys`、`<盘符>:\swapfile.sys`（文件本身，不匹配后代）。
   - **动作级禁令**（不是路径，是"不允许出现的操作字符串"）：
     - DISM `/ResetBase` 字符串——本任务通过一个独立的 `check_action(action: &str) -> Result<(), DenialReason>` 函数处理，命中即返回 `L3ReservedAction`；代码库中**不允许出现 `/ResetBase` 的正向调用**（只允许出现在测试与拒绝分支里）。
8. 暴露函数 `pub fn classify(p: &NormalizedPath) -> Result<(), DenialReason>`：通过则返回 `Ok(())`，命中即返回对应的 `DenialReason`。

### D. 唯一入口 `delete::execute()`

9. 定义 `Deleter` trait（本任务只做抽象，真实实现留后续）：
   ```rust
   pub trait Deleter {
       /// 真实删除一个已通过白名单的路径；返回 Ok(()) 表示删除动作已完成（或 Noop 记录）。
       fn delete_file(&self, p: &NormalizedPath) -> Result<(), String>;
       fn delete_dir(&self, p: &NormalizedPath) -> Result<(), String>;
   }
   ```
   本任务提供 `NoopDeleter` 结构体：实现 `delete_file` / `delete_dir`，把路径 push 到内部 `Vec`，返回 `Ok(())`，**不碰磁盘**。
10. 主入口签名（建议）：
    ```rust
    pub struct ExecuteRequest {
        pub paths: Vec<String>,
        pub actions: Vec<String>,   // 例如 ["dism", "registry_delete"]；用于动作级检查
    }
    pub struct PathDecision {
        pub input: String,
        pub normalized: Option<String>,
        pub allowed: bool,
        pub denial_reason: Option<DenialReason>,
    }
    pub enum ExecuteOutcome {
        Allowed { decisions: Vec<PathDecision> },
        Rejected { decisions: Vec<PathDecision>, code: u16 },  // code = 2002
    }
    pub fn execute(req: &ExecuteRequest, deleter: &dyn Deleter) -> Result<ExecuteOutcome, ExecuteError>;
    ```
11. `execute()` 内部流程（严格按序，任何一步失败即整体返回 `Rejected`，不调用 `deleter`）：
    - 对每条 `path`：先 `normalize::normalize()`；规范化失败 → `PathDecision { allowed:false, denial_reason: PathTraversal }`；
    - 对规范化成功的路径：跑 `whitelist::classify()`；命中 L3 → 对应 `DenialReason`；
    - 对 `actions` 列表：跑 `whitelist::check_action()`；命中 → 整体拒绝；
    - **只要有任何一条路径被拒，或任何一个 action 被禁**：返回 `ExecuteOutcome::Rejected { code: 2002, decisions: 全部决策 }`，**完全不调用 `deleter`**；
    - 全部通过：返回 `ExecuteOutcome::Allowed { decisions }`，本任务**不**自动调用 `deleter`——由上层（后续 safebox/IPC 任务）拿到 Allowed 后再决定是否真删。这样保持本任务只做「判定」，不与真实删除耦合。
12. 不写 SQLite、不写审计日志、不读规则文件、不 spawn 子进程。

### E. 单元测试（`engine/tests/execute_whitelist.rs`）

13. 全部用纯内存测试，不依赖真实 C 盘、真实用户目录、真实浏览器安装。
14. **L3 拦截用例**（每条一个 `#[test]`，必须全部 `assert!(!allowed)`）：
    - `blocks_personal_desktop`：输入 `C:\Users\alice\Desktop\secret.docx` → 拒，`L3PersonalDir`；
    - `blocks_personal_documents`：`C:\Users\alice\Documents\tax.pdf` → 拒；
    - `blocks_personal_downloads`：`D:\Users\bob\Downloads\installer.exe` → 拒；
    - `blocks_personal_pictures`：`C:\Users\alice\Pictures\birthday.jpg` → 拒；
    - `blocks_personal_videos`：`C:\Users\alice\Videos\clip.mp4` → 拒；
    - `blocks_personal_music`：`C:\Users\alice\Music\song.mp3` → 拒；
    - `blocks_browser_user_data`：`C:\Users\alice\AppData\Local\Google\Chrome\User Data\Default\Cache` → 拒，`L3BrowserProfile`；
    - `blocks_browser_edge_user_data`：`C:\Users\alice\AppData\Local\Microsoft\Edge\User Data` → 拒；
    - `blocks_browser_bookmarks_file`：任意路径下 `...\Bookmarks` 文件 → 拒，`L3BrowserSensitive`；
    - `blocks_browser_login_data`：任意路径下 `...\Login Data` 文件 → 拒；
    - `blocks_browser_history`：任意路径下 `...\History` 文件 → 拒；
    - `blocks_system32`：`C:\Windows\System32\kernel32.dll` → 拒，`L3SystemPath`；
    - `blocks_windows_installer`：`C:\Windows\Installer\{GUID}.msi` → 拒；
    - `blocks_pagefile`：`C:\pagefile.sys` → 拒；
    - `blocks_swapfile`：`C:\swapfile.sys` → 拒；
    - `blocks_resetbase_action`：`actions = ["dism /ResetBase"]` → 整体 Rejected，`L3ReservedAction`；
    - `blocks_path_traversal`：`C:\Windows\Temp\..\..\Users\alice\Documents\bank.pdf` → 规范化阶段拒；
    - `blocks_case_insensitive`：`c:\wInDoWs\sYsTeM32\x.dll` → 仍拒（证明大小写不敏感）；
    - `blocks_mixed_separators`：`C:/Windows/System32/x.dll` → 仍拒（证明斜杠归一）。
15. **放行用例**（每条 `#[test]`，必须 `assert!(allowed)`）：
    - `allows_windows_temp_log`：`C:\Windows\Temp\update.log` → 通过；
    - `allows_user_appdata_local_temp`：`C:\Users\alice\AppData\Local\Temp\foo.tmp` → 通过（注意：AppData\Local\Temp 不是个人目录，是 L1 缓存）；
    - `allows_simple_cache`：`C:\Users\alice\AppData\Local\Google\Chrome\User Data\Default\Cache\data_1` → **预期被拒**（User Data 主目录前缀命中）——本用例作为反例验证 L3 不是只拦精确文件，而是拦前缀；
    - `execute_rejected_does_not_call_deleter`：构造一条 L3 路径 + 一条合法路径，跑 `execute()` 用 `NoopDeleter`，断言 outcome 是 `Rejected` 且 `NoopDeleter` 内部 Vec **长度为 0**（证明拒绝时真的不调用 deleter）；
    - `execute_allowed_returns_all_decisions`：构造两条合法路径，跑 `execute()`，断言 outcome 是 `Allowed` 且 decisions 长度 = 2。
16. 测试中**禁止**触碰真实文件系统（除 `tempfile::tempdir()` 造空目录外，但本任务纯路径判定，原则上不需要 tempdir）。

### F. 卫生与边界

17. 不创建 `scan/`、`grade/`、`safebox/`、`audit/`、`rules/`、`platform/` 等其他 engine 子模块目录（留给后续任务）。
18. 不修改 `app/src-tauri/`、`app/src/`、`elevation/`、`rules/`、`scripts/`、`engine/src/db/**` 任何文件。
19. 不修改 `docs/PRD.md`、`docs/TRD.md`、`docs/ARCHITECTURE.md`、`docs/AGENTS.md` 等长期文档；
20. 不在代码中硬编码真实用户名（`C:\Users\alice\...` 只允许出现在测试用例字符串里，不允许出现在生产代码的 L3 列表中——L3 列表必须用文件夹名匹配，不限定具体用户）；
21. 不在本任务里调用 `std::fs::remove_file` / `std::fs::remove_dir`；
22. 不在本任务里写任何 `audit_log` SQL / Tauri command / 规则 JSON 解析。

## 允许修改范围

> 严格白名单。范围外文件一律不动；若发现必须改范围外文件才能跑通，**停下在实现备注里报 DocsKeeper**，不得自行扩大。

- `engine/Cargo.toml`（仅新增 `thiserror = "1"` 一条依赖）
- `engine/src/lib.rs`（仅新增 `pub mod execute;` 一行）
- `engine/src/execute/**`（新建整目录：`mod.rs`、`normalize.rs`、`whitelist.rs`、`delete.rs`、`noop_deleter.rs` 或在 `delete.rs` 内联 `NoopDeleter`）
- `engine/tests/execute_whitelist.rs`（新建集成测试）
- `Cargo.lock`（由依赖变更自动产生）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`（由最后一次 `docs(state)` 提交处理）

## Git 分支

- 分支名：`feature/task-003-delete-whitelist`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

> 严格按以下顺序原子提交，每个提交只动一类文件；**禁止 `git add .` / `git add -A` / `git commit -a`**，每次 `git commit` 必须显式列文件路径。

1. `build(deps): 为 engine::execute 引入 thiserror 依赖 (#task-003)`
   —— 含 `engine/Cargo.toml`、`Cargo.lock`。
2. `feat(execute): 新增 engine::execute 模块骨架与 Deleter trait 抽象 (#task-003)`
   —— 含 `engine/src/lib.rs`、`engine/src/execute/mod.rs`、`engine/src/execute/delete.rs`（仅 trait 与 NoopDeleter 骨架）。
3. `feat(execute): 实现 Windows 路径规范化器（防 .. 穿越、统一斜杠、大小写不敏感） (#task-003)`
   —— 含 `engine/src/execute/normalize.rs`。
4. `feat(execute): 实现内置 L3 红线路径匹配器（个人目录/浏览器数据/系统关键路径/动作级禁令） (#task-003)`
   —— 含 `engine/src/execute/whitelist.rs`。
5. `feat(execute): 落地 delete::execute() 唯一入口，全拒即不调用 Deleter (#task-003)`
   —— 完善 `engine/src/execute/delete.rs`，串起 normalize → whitelist → Deleter。
6. `test(execute): 补 PRD P0-7 全部 L3 路径拦截与放行用例 (#task-003)`
   —— 含 `engine/tests/execute_whitelist.rs`。
7. 自验全绿后，最后一次提交：`docs(state): 任务003完成，更新进度与下一步 (#task-003)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 把待办 003 从 `- [ ]` 改为 `- [x]`；
   - 「最近一次变更」追加一行（task-003 完成摘要）；
   - 「下一步计划」改为「派发 task-004：落地 IPC 网关骨架——统一响应信封、scan.start/cleanup.dryRun 命令、进度事件通道」；
   - 「Git 提交记录」表追加本任务 7 条 commit 摘要；
   - 文件末尾「变更轨迹」追加 `2026-10-11  task-003  落地删除唯一入口与 L3 白名单匹配器`。

**提交前自验（必须全绿，任一不过不得 push）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy -p engine --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0，且 §E 中至少 20 条测试用例全部通过；
- `cargo fmt --all -- --check` 退出码 0；
- 代码中 grep 检查：生产代码（非 `#[cfg(test)]`、非 `whitelist.rs` 的拒绝分支）**不允许**出现 `std::fs::remove_file` / `std::fs::remove_dir` / `/ResetBase` 正向调用——DevAgent 用 `grep` 自检并在备注里贴结果。

## 输出要求

DevAgent 交付物（后续执行结果，不是本任务文档产出）：

1. 上述 6 个代码/配置提交 + 1 个 `docs(state)` 提交，共 7 个 commit，全部 push 到 `feature/task-003-delete-whitelist`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（与白名单逐条对照）；
   - 中文本地化文件夹名（桌面/文档/下载/...）是否纳入匹配、取舍理由；
   - 自验命令与退出码（逐条列）；
   - 20 条测试用例的通过输出（粘贴 `cargo test -p engine` 末尾统计）；
   - `grep` 自检结果（证明生产代码无真实删除调用、无 `/ResetBase` 正向调用）；
   - 与本任务文档不符的偏差（如有，含原因）；
   - 遗留问题（例如：UNC 路径 `\\server\share\...` 本任务如何处理、留待哪个任务）；
3. `docs/PROJECT_STATE.md` 已按第 7 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务003-删除唯一入口与L3白名单 (#task-003)`，
   该提交只含 `tasks/003-delete-whitelist.md` 一个文件，落在 `feature/task-003-delete-whitelist` 分支最前端（即 DevAgent 开工前的基线）。

## 实现备注（DevAgent 回填，2026-10-11）

### 实际改动文件清单（对照白名单）

- `engine/Cargo.toml`：仅新增 `thiserror = "1"`（钉 1.x，解析为 1.0.69）；未加 serde/regex/walkdir。
- `engine/src/lib.rs`：仅新增 `pub mod execute;` 一行。
- `engine/src/execute/mod.rs`（新建）：模块声明 delete/normalize/whitelist。
- `engine/src/execute/normalize.rs`（新建）：NormalizeError（thiserror）+ NormalizedPath newtype + `normalize()`。
- `engine/src/execute/whitelist.rs`（新建）：DenialReason 六变体 + `classify()` + `check_action()`。
- `engine/src/execute/delete.rs`（新建）：Deleter trait、NoopDeleter、ExecuteRequest/PathDecision/ExecuteOutcome/ExecuteError、`execute()` 唯一入口、`REJECT_CODE=2002`。
- `engine/tests/execute_whitelist.rs`（新建）：24 条集成测试。
- `Cargo.lock`：随依赖自动更新。
- `docs/PROJECT_STATE.md`：由 `docs(state)` 提交处理。
- 未触碰：engine/src/db/**、app/**、elevation/**、rules/、scripts/、长期文档。

### 中文本地化文件夹名取舍

**已纳入补充匹配**：`桌面/文档/下载/图片/视频/音乐` 与英文真名同表（PERSONAL_DIRS 12 项）。理由：Windows 显示名与 NTFS 真名可能不一致（尤其某些工具直接拿 Explorer 显示路径回传），作为补充匹配代价为零、只增拦截不漏；匹配规则仍是 `users/<用户名>/<目录>` 结构，不会误伤普通目录下的同名文件夹。

### 自验命令与退出码

| 命令 | 退出码 |
|------|--------|
| `cargo check --workspace --all-targets` | 0 |
| `cargo clippy -p engine --all-targets -- -D warnings` | 0（首次 2 个告警已修：`Iterator::last`→`next_back`、needless borrow） |
| `cargo test -p engine` | 0（lib 9 + db 8 + execute 24 + smoke 1 = 42 passed / 0 failed） |
| `cargo fmt --all -- --check` | 0 |

### 测试输出摘录（cargo test -p engine 末尾统计）

```text
test result: ok. 9 passed; 0 failed (lib 单元：normalize 5 + whitelist 4)
test result: ok. 8 passed; 0 failed (task-002 db schema)
test result: ok. 24 passed; 0 failed (execute_whitelist 集成：19 拦截 + 5 放行/行为)
test result: ok. 1 passed; 0 failed (hello_engine smoke)
```

### grep 卫生自检结果

```text
Select-String engine\src\execute\*.rs -Pattern 'remove_file|remove_dir'
  → 无匹配（生产代码无任何真实删除调用）
Select-String engine\src\execute\normalize.rs,delete.rs -Pattern 'resetbase'
  → 无匹配（/ResetBase 字面量仅存在于 whitelist.rs 拒绝分支与测试代码）
```

### 与任务文档的偏差（含原因）

1. **`..` 一律直接拒绝，不做词法折叠**：任务 B.4 原文写「规范化后逃逸盘符根才拒」，但 E-14 `C:\Windows\Temp\..\..\Users\alice\Documents\bank.pdf` 明确要求规范化阶段拒（按「折叠后判逃逸」语义它折叠后是合法 C:\Users\...，会漏拒）。删除网关对「未解析上跳」没有合法用途（上游全是已解析绝对路径），故见到任何 `..` 即 TraversalEscapes，零折叠、零逃逸空间。
2. **User Data 尾匹配**：路径恰好以 `User Data` 结尾（无后代）时原 `\user data\` 子串匹配漏掉 Edge 用例，已补 `ends_with("\user data")`。
3. **thiserror 版本**：钉 `= "1"`（任务字面要求），cargo 解析为 1.0.69；首次 `cargo add` 拉到 2.0.21 已 amend 纠正（未 push）。
4. **User Data 下的敏感文件测试路径**：Bookmarks/Login Data/History 用例刻意放在 User Data 之外（`C:\Temp\stolen\...`）以验证 L3BrowserSensitive 分支——在 User Data 内会先命中 L3BrowserProfile。
5. **多一次 style 提交**：rustfmt 对 normalize.rs 的 if 换行整改，未改写历史。

### 遗留问题

- **UNC 路径 `\\server\share\...`**：本任务按 NotAbsolute 保守拒绝（无盘符）。网络盘通常是用户自己挂载的数据盘，L3 语义与本地盘不同，留待 task-004 之后的 platform 任务统一评估网络盘红线策略。
- **符号链接解析**：本任务纯词法规范化，`symlink_metadata` 留待 platform 模块（防 junction 绕开白名单）。
- **外部红线集 trait 边界**：任务要求的「重跑外部红线集」未在本任务接入（005 加载 rules/cache_rules.json 时注入），当前 execute 只跑内置红线。
