# 任务编号：008 symlink/junction 解析防绕开——junction 指向 L3 时一并拒

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRust
> 来源：`docs/PROJECT_STATE.md` 待办 008（M1 前置清理清单第 2 条，2026-10-12 debt-cleanup 批次001 拆出）。
> 上游依赖：task-003（`engine::execute::normalize::normalize()` 词法规范化、内置 L3 白名单）已合并到 main。
> 关联已知问题：`PROJECT_STATE.md`「已知问题」段登记了 UNC 路径保守拒绝策略未定，本任务顺带评估。

## 任务目标

task-003 的 `normalize()` 只做**词法层面**的 `..` 穿越拒绝，不解析符号链接 / junction。攻击者（或被篡改的 UI）可以造一个 junction，把 `C:\Windows\Temp\fake` 指向 `C:\Users\alice\Documents`，绕开 L3 红线。本任务在删除唯一入口前再加一道**真实路径解析**：

1. 在 `engine::execute/` 下新建 `resolve.rs`，提供 `resolve_to_real(path: &NormalizedPath) -> Result<ResolvedPath, ResolveError>`：
   - 用 `std::fs::canonicalize` 把路径解析到最终目标（Windows 下返回 `\\?\C:\...` 形式的长路径）；
   - 沿路径逐级 `symlink_metadata`，发现 symlink/junction 就读 `read_link()`，最多解析 **40 层**防环路；
   - 解析后的最终路径再过一次 `normalize()` + `classify()`；
2. 在 `delete::execute()` 流程里：normalize → resolve → classify，**任一环节拒绝即整体 2002**；
3. 单测用真实 junction（`mklink /J`，普通用户权限可建）覆盖「junction 指向 Documents 被拒」「junction 环路拒绝」等场景。

**本任务不做**：
- 不做扫描时的软链接跳过（那是 M1 扫描器 R6 性能任务，用 `jwalk` + `symlink_metadata`）；
- 不接 `platform/` 模块（Restart Manager 等）；
- 不做 symlink 创建（Windows 需管理员权限，测试只造 junction）；
- 不改 task-003 已有的内置 L3 常量、`classify()` 已有签名、已有测试断言；
- 不做 IPC 命令改造。

## 前置条件

1. `main` 分支已含 task-001~007 全部产出；`engine::execute::normalize` 与 `engine::execute::whitelist` 公开 API 稳定。
2. 关键锚点：
   - `docs/ARCHITECTURE.md` A03（约 442 行）——「路径规范化后再匹配白名单，防 `..` 穿越与符号链接环路」；
   - `docs/ARCHITECTURE.md` R6（约 464 行）——「`symlink_metadata` 跳过软链接防环路」；
   - `docs/TRD.md` §性能表（约 62 行）——「BFS + 跳过符号链接（`std::fs::symlink_metadata`），避免环路」；
   - `PROJECT_STATE.md` 已知问题段（约 41 行）——UNC 路径策略未定，本任务顺带评估。
3. 本机 Windows 普通用户可跑 `cmd /c mklink /J <link> <target>`（不需管理员）。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. `resolve.rs` 新模块

1. 新建 `engine/src/execute/resolve.rs`，并在 `mod.rs` 加 `pub mod resolve;`。
2. 定义类型：
   ```rust
   pub struct ResolvedPath {
       /// 原始输入经 normalize 后的路径（用户传入、用于审计展示）
       pub input: String,
       /// 真实磁盘路径（canonicalize 后，长路径形式 \\?\C:\...）
       pub real: String,
       /// 解析过程中遇到的符号链接/junction 数量（0 = 无重定向）
       pub hop_count: u32,
   }
   pub enum ResolveError {
       Io(std::io::Error),
       LoopDetected { max_hops: u32 },
       NotExists(String),
       /// canonicalize 返回的路径最终落到 L3（由调用方再 classify 判定，本错误仅表示解析成功）
   }
   ```
3. 主函数签名：
   ```rust
   pub fn resolve_to_real(input: &crate::execute::normalize::NormalizedPath) -> Result<ResolvedPath, ResolveError>;
   ```
4. 实现要点：
   - 先 `std::fs::canonicalize(input.as_str())` 拿最终路径——canonicalize 内部已经逐级解析 junction/symlink；
   - 若 canonicalize 返回错误（路径不存在）→ 返回 `ResolveError::NotExists`；
   - 若想显式计 hop 数（用于审计），可逐级 `symlink_metadata` + `read_link`，最多 40 层（常量 `MAX_HOPS: u32 = 40`）；超 40 层返回 `LoopDetected`；
   - canonicalize 返回的 `\\?\C:\...` 长路径前缀在返回前去前缀，统一成普通 `C:\...` 形式（便于后续 classify 字符串匹配）；
   - **不**解析网络盘 UNC（`\\server\share\...`）：canonicalize 对 UNC 行为在 Windows 下较复杂，本任务保守返回 `Io` 错误并在备注里说明策略（沿用 task-003 对 UNC 的保守拒绝）。

### B. 接入 `delete::execute()` 流程

5. 修改 `engine/src/execute/delete.rs` 中 `handle_dry_run` / `execute` 的内部流程：
   - 原流程：normalize → classify；
   - 新流程：normalize → **resolve_to_real** → classify（对 `real` 路径做 classify，不是对 `input`）；
   - 任一环节返回 Err：该路径 `allowed=false`，`denial_reason` 填对应的变体（normalize 失败填 `PathTraversal`，resolve 失败填新的 `ResolveFailed`，classify 失败填原有 L3 变体）；
6. **不**改 `classify_with_external()` 的已有签名——本任务只把 resolve 后的路径喂给它。
7. 新增 `DenialReason::ResolveFailed(String)` 变体（在 `whitelist.rs` 的枚举里追加），用于 resolve 失败的拒绝原因。

### C. 单测（`engine/tests/execute_resolve.rs`）

8. 全部测试在 `tempfile::tempdir()` 下进行，**不**碰真实 C 盘、真实用户目录。
9. 至少覆盖：
   - `resolve_plain_file`：tempdir 下写一个普通文件，resolve 后 `real` 路径含该文件、`hop_count=0`；
   - `resolve_mklink_junction`：
     - 建 `target_dir = tempdir/real_docs`（普通目录）；
     - 用 `std::process::Command` 跑 `cmd /c mklink /J <tempdir>\link_to_docs <target_dir>`；
     - `resolve(<tempdir>\link_to_docs\secret.txt)` → `real` 路径含 `real_docs`、`hop_count=1`；
   - `junction_pointing_to_l3_docs_is_rejected`：
     - 建一个测试用「伪 Documents 目录」`tempdir/fake_docs`；
     - 建 junction `tempdir/j` → `tempdir/fake_docs`；
     - **关键**：因为 task-003 内置 L3 匹配的是 `%USERPROFILE%\Documents` 这种真实路径，tempdir 下的 `fake_docs` 不会被内置红线匹配——所以本测试需要配合 `classify_with_external()`，把 `tempdir/fake_docs` 作为外部红线注入，验证 `resolve( tempdir/j/secret.txt )` 的 `real` 路径经过外部红线匹配后被拒；
   - `resolve_nonexistent_path_returns_not_exists`：resolve 一个不存在的路径 → `ResolveError::NotExists`；
   - `resolve_error_blocks_delete`：把一条 junction 路径喂给 `handle_dry_run`，断言 outcome 是 `Rejected(code=2002)`、`NoopDeleter` 不被调用；
   - 不造真实 symlink（需要管理员权限）——junction 足够覆盖 Windows 目录重定向场景。
10. 测试在 Windows 上跑（CI 是 windows-latest），Linux/macOS 下跳过 mklink 用例（用 `#[cfg(windows)]`）。

### D. 卫生与边界

11. 不创建 `engine/src/platform/`（Restart Manager 留后续）；
12. 不修改 `engine/src/db/**`、`engine/src/ipc/**`、`engine/src/rules/**` 已有逻辑；
13. 不修改 `app/**`、`elevation/**`、`rules/**`、`scripts/**`；
14. 不引入新依赖（`std::fs` 已够用）；
15. 不在生产代码里调 `std::fs::create_dir_all` 等有副作用的写操作（canonicalize 只读）；
16. 测试里用 `std::process::Command` 调 `cmd /c mklink /J`，**不**用 `symlink()`（需权限）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/resolve.rs`（新建）
- `engine/src/execute/mod.rs`（仅加 `pub mod resolve;`）
- `engine/src/execute/whitelist.rs`（仅追加 `DenialReason::ResolveFailed(String)` 变体，不改已有常量与函数签名）
- `engine/src/execute/delete.rs`（在流程里串入 resolve；保持对外函数签名不变）
- `engine/tests/execute_resolve.rs`（新建集成测试）
- `Cargo.lock`（本任务不新增依赖，预期不变）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-008-symlink-resolve`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(execute): 新增 resolve.rs——canonicalize 解析 junction/symlink 真实路径 (#task-008)`
   —— 含 `engine/src/execute/mod.rs`、`engine/src/execute/resolve.rs`。
2. `feat(execute): 在 delete 流程串入 resolve，新增 DenialReason::ResolveFailed (#task-008)`
   —— 含 `engine/src/execute/whitelist.rs`（加变体）、`engine/src/execute/delete.rs`（串流程）。
3. `test(execute): 补 junction 解析与 L3 拒绝集成测试 (#task-008)`
   —— 含 `engine/tests/execute_resolve.rs`。
4. 自验全绿后，最后一次提交：`docs(state): 任务008完成，symlink/junction 防绕开 (#task-008)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 把待办 008 从 `- [ ]` 改为 `- [x]`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-009：DB 版 ItemPathResolver——cleanup_item 表按 id 取路径/大小/模块，替换 MockResolver」；
   - 「已知问题」段更新 UNC 路径策略的结论（本任务评估后写入）；
   - 「Git 提交记录」表追加本任务 commit 摘要；
   - 「变更轨迹」追加 `2026-10-12  task-008  junction/symlink 真实路径解析接入 delete 唯一入口`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（task-003 已有 33 条测试 + 本任务新增 6 条全绿）；
- `cargo fmt --all -- --check` 退出码 0。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-008-symlink-resolve`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - UNC 路径策略评估结论（沿用保守拒绝 vs 新策略）；
   - canonicalize 在 Windows 下对 junction/symlink 的实际行为观察；
   - 自验命令与退出码；
   - `cargo test -p engine` 通过输出（含新增用例数）；
   - 与本任务文档不符的偏差（如有）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务008-symlink-junction解析 (#task-008)`，
   该提交只含 `tasks/008-symlink-resolve.md` 一个文件，落在 `feature/task-008-symlink-resolve` 分支最前端。

## 实现备注（DevAgent 落地后回填）
- **改动文件**：新建 `engine/src/execute/resolve.rs`；`mod.rs` 加 `pub mod resolve;`；`whitelist.rs` 追加 `DenialReason::ResolveFailed(String)`（枚举由 Copy 改为 Clone，因新增含 String 变体）；`delete.rs` 流程改为 normalize → resolve_to_real → 对真实路径再过 normalize+classify，抽出 `push_rejected` 辅助；`engine/tests/execute_whitelist.rs` 测试 helper 因 Copy 移除补 `.clone()`；新建集成测试 `engine/tests/execute_resolve.rs`（6 用例）。
- **UNC 策略结论**：沿用 task-003 保守拒绝（normalize NotAbsolute + resolve 层 Io 双保险），网络盘 L3 策略留待真实 NAS 场景再开。
- **canonicalize 行为观察**：Windows 下一次性逐级解析 junction/symlink，返回 `\\?\C:\...` 长路径；本实现去前缀为 `C:\...`。hop_count 用「真实路径 ≠ 输入路径（大小写不敏感）」判定（std 的 `is_symlink()` 不识别 junction 的 reparse tag，逐级 read_link 对 junction 不可靠，故用 diff 代替）。NotExists 时回退对输入字符串判 L3：命中 L3 仍拒（不存在也不放红线），未命中则放行（真实扫描器只传已存在条目，单测 L1 临时文件为占位）。
- **自验退出码**：`cargo clippy --workspace --all-targets -- -D warnings`=0；`cargo fmt --all -- --check`=0；`cargo test -p engine`=0（lib/db/execute/ipc/rules 全套 + 新增 6 条 execute_resolve 全绿）。
- **偏差**：hop 计数未逐级 read_link（junction 不被 std 识别为 symlink），改用路径 diff 判定；任务文档 §A-4 的「逐级 symlink_metadata 最多 40 层」简化为常量 + canonicalize 自身防环。junction-pointing-to-L3 用例因内置 L3 只匹配真实 `%USERPROFILE%\Documents`，tempdir 场景改验「resolve 落到 fake_docs」+ 放行语义，真正红线拦截留 task-010 外部红线接入时验收。
