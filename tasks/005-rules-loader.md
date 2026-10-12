# 任务编号：005 落地初始规则库 cache_rules.json + rules_meta.json + SHA-256 校验加载器

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRust
> 来源：`docs/PROJECT_STATE.md` 待办 005 /「下一步计划」段（由 task-004 收尾时写入）。
> 上游依赖：task-001（monorepo 骨架）、task-002（`engine::db`）、task-003（`engine::execute` 内置 L3 白名单）、task-004（`engine::ipc` 信封）均已合并到 main。

## 任务目标

在仓库 `rules/` 目录落地**初始规则库静态文件**，并在 `engine/` 内新建 `rules/` 子模块，把「JSON 规则 → 路径展开 → SHA-256 校验 → 喂给白名单匹配器」的纯本地加载链路跑通。本任务交付以下四件事：

1. **`rules/cache_rules.json`**：按 TRD §4.10 schema 写的初始规则库，含 `version`、`modules`、`whitelist_deny` 三段；初始只放 1~2 个示例条目（如 `npm_cache`），六大模块 A~F 的完整规则清单留给后续规则 PR；
2. **`rules/rules_meta.json`**：与 cache_rules.json 配对的元数据文件（version / checksum_sha256 / source_url / downloaded_at / custom_rules），checksum 字段必须是 cache_rules.json 真实字节的 SHA-256；
3. **`engine::rules` Rust 模块**：
   - serde 模型：`CacheRules` / `RuleEntry` / `RulesMeta`；
   - 路径变量白名单展开：`%LOCALAPPDATA%` / `%APPDATA%` / `%TEMP%` / `%PROGRAMDATA%` / `%USERPROFILE%` / `~` 六个变量 + 波浪号，其他环境变量一律拒绝；
   - SHA-256 校验加载器：读 cache_rules.json 字节 → 算 SHA-256 → 与 rules_meta.checksum_sha256 比对 → 不符即拒绝加载并保留旧版；
4. **与 task-003 白名单的并集**：把 `whitelist_deny` 数组展开后注入 task-003 的白名单匹配器，与引擎内置 L3 红线**取并集**生效；内置红线永不被规则文件移除。

**本任务不做**：
- 不做网络拉取（GitHub Raw 联网更新 / `rules.checkUpdate` / `rules.applyUpdate` IPC 命令留给后续任务）；
- 不写 DB（`rules_meta` 表的运行时落库、`audit_log: rules_update` 留后续任务）；
- 不做规则条目扫描执行（不调 `jwalk`、不统计目录大小）——只加载与展开，不发起扫描；
- 不接 Tauri command（前端规则更新 UI 留后续任务）；
- 不塞完整六大模块规则清单——只放 1~2 条示例把 schema 跑通。

## 前置条件

1. `main` 分支已含 task-001~004 全部产出；`engine::execute::whitelist` 已暴露可注入外部红线的入口（若 task-003 未留该 trait，DevAgent 在备注里说明并在 `whitelist.rs` 上做最小扩展——但不得改动 task-003 已通过测试的内置红线行为）。
2. 关键锚点（DevAgent 只读这些段落，不通读全文）：
   - `docs/TRD.md` §4.9 `rules_meta` 表字段（约 220~229 行）——version / checksum_sha256 / source_url / downloaded_at / custom_rules；
   - `docs/TRD.md` §4.10 `cache_rules.json` 完整 schema（约 231~261 行）——modules 数组字段、whitelist_deny 数组、路径变量白名单；
   - `docs/TRD.md` §3 依赖表（约 96 行）——`sha2` 用于 checksum 校验；
   - `docs/ARCHITECTURE.md` §3.3 规则库更新数据流（约 200~212 行）——SHA-256 不符即保留旧版、code=3002；
   - `docs/ARCHITECTURE.md` §6 A08（约 447 行）——规则仅 JSON、永不执行远端代码；
   - `docs/ARCHITECTURE.md` R3（约 461 行）——内置红线与规则红线取并集、规则更新不得移除内置红线。
3. 本机可跑 `cargo test -p engine`。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. 仓库静态规则文件

1. `rules/cache_rules.json` 必须是合法 JSON（不是 JSONC），结构严格符合 TRD §4.10：
   - 顶层字段：`version` (string)、`modules` (object)、`whitelist_deny` (array of string)；
   - `version` 固定为 `"1.0.0"`；
   - `modules` 至少含一个键 `"A"`，其值为数组，数组里至少一个对象：
     - 必填字段：`id` / `category` / `display_name` / `paths` / `risk_level` / `description` / `impact_note` / `rebuild_note` / `elevated`；
     - 示例条目直接抄 TRD §4.10 的 `npm_cache`（`paths` 含 `%LOCALAPPDATA%\npm-cache` 与 `~/.npm/_cacache`，`risk_level="L1"`，`elevated=false`）；
   - `whitelist_deny` 数组至少含 TRD §4.10 示例的 4 条：`%USERPROFILE%\Desktop`、`%USERPROFILE%\Documents`、`C:\Windows\System32`、`C:\pagefile.sys`；
   - **不得**在 JSON 里放任何可执行代码、`$(`、`${}`、命令字符串；
   - **不得**在 JSON 里放 `%WINDIR%` / `%PROGRAMFILES%` 等不在白名单内的环境变量（本任务先验证白名单变量能跑通）。
2. `rules/rules_meta.json` 必须是合法 JSON，含以下字段：
   ```json
   {
     "version": "1.0.0",
     "checksum_sha256": "<cache_rules.json 的真实 SHA-256 十六进制小写>",
     "source_url": "https://raw.githubusercontent.com/An0114/C-clean/main/rules/cache_rules.json",
     "downloaded_at": 0,
     "custom_rules": []
   }
   ```
   - `checksum_sha256` 必须**用脚本现场算**（DevAgent 提交前用 `sha256sum rules/cache_rules.json` 或 PowerShell `Get-FileHash` 算出真实值再回填），不得手填或留占位；
   - 字段值大小写、十六进制小写、无 `0x` 前缀。

### B. Rust 模块与依赖

3. `engine/src/` 下新建 `rules/` 目录，至少包含：`mod.rs`、`models.rs`、`env_expand.rs`、`loader.rs`。
4. `engine/src/lib.rs` 仅新增一行 `pub mod rules;`，不改动已有 `pub mod db;` / `pub mod execute;` / `pub mod ipc;`。
5. `engine/Cargo.toml` 只允许新增：
   - `sha2 = "0.10"`
   - `hex = "0.4"`
   禁止顺手加 `reqwest` / `tokio` / `serde_yaml`（本任务纯本地，不联网）。

### C. serde 模型（`models.rs`）

6. 定义结构（用 `serde::Deserialize`，字段名 snake_case 与 JSON 对齐）：
   ```rust
   pub struct CacheRules {
       pub version: String,
       pub modules: std::collections::HashMap<String, Vec<RuleEntry>>,
       pub whitelist_deny: Vec<String>,
   }
   pub struct RuleEntry {
       pub id: String,
       pub category: String,
       pub display_name: String,
       pub paths: Vec<String>,
       pub risk_level: String,        // "L0" / "L1" / "L2"
       pub description: String,
       pub impact_note: String,
       pub rebuild_note: String,
       pub elevated: bool,
   }
   pub struct RulesMeta {
       pub version: String,
       pub checksum_sha256: String,
       pub source_url: String,
       pub downloaded_at: i64,
       pub custom_rules: serde_json::Value,  // 本任务保持 []，不展开
   }
   ```
7. `risk_level` 本任务不做枚举强校验（字符串即可），但 loader 必须能识别 `"L0"/"L1"/"L2"`，遇到 `"L3"` 直接报错（L3 不允许出现在可清理规则里，对应 ARCHITECTURE §4「L3 记录永不写入 cleanup_item」）。

### D. 路径变量白名单展开（`env_expand.rs`）

8. 暴露函数 `pub fn expand(input: &str) -> Result<String, ExpandError>`：
   - 支持的变量（大小写不敏感，匹配 `%NAME%`）：
     `%LOCALAPPDATA%` / `%APPDATA%` / `%TEMP%` / `%PROGRAMDATA%` / `%USERPROFILE%`；
   - 支持 `~` 仅作为路径前缀（`~/.npm/_cacache`），展开为当前用户 home（`std::env::var("USERPROFILE")`）；
   - 遇到其他 `%XXX%`（如 `%WINDIR%`、`%PROGRAMFILES%`、`%PATH%`）→ 返回 `ExpandError::UnknownVar("WINDIR")`；
   - 环境变量未设置 → 返回 `ExpandError::VarUnset("LOCALAPPDATA")`；
   - 展开后路径统一反斜杠（正斜杠 → 反斜杠）；
   - **不做**词法层面的 `..` 规范化（那是 task-003 `normalize.rs` 的事），但本函数返回的字符串在进入白名单匹配前会由 task-003 的 `normalize` 再过一遍。
9. `~` 后面必须跟 `\` 或 `/` 或结尾，不允许 `~foo` 这种伪装。

### E. SHA-256 校验加载器（`loader.rs`）

10. 暴露函数：
    ```rust
    pub struct LoadedRules {
        pub rules: CacheRules,
        pub meta: RulesMeta,
    }
    pub enum LoadError {
        Io(std::io::Error),
        Json(serde_json::Error),
        ChecksumMismatch { expected: String, actual: String },
        L3InRules(String),       // rules 里出现 risk_level="L3"
        Expand(ExpandError),
    }
    pub fn load(rules_dir: &std::path::Path) -> Result<LoadedRules, LoadError>;
    ```
11. `load()` 流程（严格按序）：
    - 读 `<rules_dir>/cache_rules.json` 为字节；
    - 读 `<rules_dir>/rules_meta.json` 为字节，serde 反序列化为 `RulesMeta`；
    - 对 cache_rules.json 字节算 SHA-256（`sha2::Sha256`），hex 小写编码；
    - 与 `meta.checksum_sha256` 字符串严格比对（大小写敏感：必须小写）；
    - **不一致** → 返回 `LoadError::ChecksumMismatch { expected, actual }`，本函数**不**修改任何文件、**不**返回 rules 结构（调用方负责保留旧版）；
    - 一致 → serde 反序列化 cache_rules.json；
    - 遍历所有 modules[*] 条目，若 `risk_level == "L3"` → 返回 `LoadError::L3InRules(entry.id)`；
    - 对所有 `whitelist_deny` 路径与所有 `paths` 调 `env_expand::expand()`；任一失败 → 返回对应 `ExpandError`；
    - 全部通过 → 返回 `LoadedRules`。
12. 不做"加载失败时尝试从备份恢复"——那是后续任务。

### F. 与 task-003 白名单的并集

13. 暴露函数 `pub fn external_deny_list(loaded: &LoadedRules) -> Vec<String>`：返回展开后的 `whitelist_deny` 绝对路径列表（已过 `env_expand`）。
14. **不修改** task-003 `whitelist.rs` 的内置红线常量。若 task-003 未留"注入外部红线"的入口，DevAgent 在 `engine::execute::whitelist` 上做最小扩展：
    - 新增函数 `classify_with_external(path: &NormalizedPath, external: &[String]) -> Result<(), DenialReason>`：先跑内置红线，再跑 external 前缀匹配；
    - 原 `classify()` 保持签名不变（行为不变，已有测试全绿）；
    - 该扩展**必须**在任务文件「实现备注」里显式列出改动行，并在提交里单独一个 commit。
15. 并集语义：内置红线 + external 红线任一命中即拒；external 红线**不能**放宽内置红线（即使 rules 里 whitelist_deny 少写了某条，内置红线依然生效）。

### G. 单元测试（`engine/tests/rules_loader.rs`）

16. 用 `tempfile::tempdir()` 造临时 rules/ 目录，每个测试先写自己的 cache_rules.json 与 rules_meta.json（用程序算 SHA-256，避免硬编码 hash）。
17. 至少覆盖：
    - `load_ok_with_valid_files`：合法 JSON + 正确 checksum → 返回 Ok，`version="1.0.0"`；
    - `load_rejects_tampered_cache_rules`：写好合法 rules_meta.json，再往 cache_rules.json 末尾加一个空格 → 返回 `ChecksumMismatch`，expected 与 actual 字段都非空；
    - `load_rejects_bad_checksum_in_meta`：rules_meta.checksum_sha256 故意写错 → 返回 `ChecksumMismatch`；
    - `load_rejects_unknown_env_var`：cache_rules.json 里 paths 用 `%WINDIR%\Temp` → 返回 `Expand(UnknownVar("WINDIR"))`；
    - `load_rejects_l3_in_rules`：某条 rule 的 `risk_level="L3"` → 返回 `L3InRules`；
    - `expand_resolves_six_whitelist_vars`：对 `%LOCALAPPDATA%\foo` 等 6 个变量分别断言展开后非空且不含 `%`；
    - `expand_rejects_tilde_masquerade`：`~foo` 不展开为 home（作为相对路径或报错）；
    - `external_deny_list_union_with_builtin`：loaded 里 whitelist_deny 含 `%USERPROFILE%\Desktop`，调 `external_deny_list()` 得到一个绝对路径；把它传给 `classify_with_external()`，对该路径返回拒；同时对一条**不在 external 列表、但在内置红线里**的路径（如 `C:\Windows\System32`）仍返回拒（证明并集不是替换）；
    - `classify_builtin_unchanged`：直接调 task-003 已有的 `classify()`，对 L3 路径仍返回拒（回归保护）。
18. 测试中**不**发网络请求、不读真实 `%LOCALAPPDATA%\npm-cache`、不写真实 rules/ 目录（除 tempdir）。

### H. 卫生与边界

19. 不创建 `engine/src/scan/`、`engine/src/safebox/`、`engine/src/audit/`、`engine/src/platform/` 等其他子模块。
20. 不修改 `engine/src/db/**` 的已有逻辑（只可能在 task-006 之后再接 DB 落库）。
21. 不修改 `app/src/**`、`app/src-tauri/**`、`elevation/**`、`scripts/**`。
22. 不修改 `docs/PRD.md`、`docs/TRD.md`、`docs/ARCHITECTURE.md`、`docs/AGENTS.md`；
23. 不引入 `reqwest` / `ureq` / `tokio` 等网络栈；
24. 不把 cache_rules.json 的内容硬编码到 Rust 源码里——规则必须从文件读。

## 允许修改范围

> 严格白名单。范围外文件一律不动；若发现必须改范围外文件才能跑通，**停下在实现备注里报 DocsKeeper**，不得自行扩大。

- `rules/cache_rules.json`（新建）
- `rules/rules_meta.json`（新建）
- `engine/Cargo.toml`（仅新增 sha2 / hex 两条依赖）
- `engine/src/lib.rs`（仅新增 `pub mod rules;` 一行）
- `engine/src/rules/**`（新建整目录，文件清单见 §B-3）
- `engine/src/execute/whitelist.rs`（**仅**允许新增 `classify_with_external()` 函数，禁止改动已有 `classify()` 与内置红线常量）
- `engine/tests/rules_loader.rs`（新建集成测试）
- `Cargo.lock`（由依赖变更自动产生）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`（由最后一次 `docs(state)` 提交处理）

## Git 分支

- 分支名：`feature/task-005-rules-loader`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

> 严格按以下顺序原子提交，每个提交只动一类文件；**禁止 `git add .` / `git add -A` / `git commit -a`**，每次 `git commit` 必须显式列文件路径。

1. `build(deps): 为 engine::rules 引入 sha2 与 hex (#task-005)`
   —— 含 `engine/Cargo.toml`、`Cargo.lock`。
2. `feat(rules): 编写初始 cache_rules.json 与 rules_meta.json（含真实 SHA-256） (#task-005)`
   —— 含 `rules/cache_rules.json`、`rules/rules_meta.json`。
3. `feat(rules): 定义 CacheRules / RuleEntry / RulesMeta serde 模型 (#task-005)`
   —— 含 `engine/src/lib.rs`（加 `pub mod rules;`）、`engine/src/rules/mod.rs`、`engine/src/rules/models.rs`。
4. `feat(rules): 实现路径变量白名单展开（6 个环境变量 + ~） (#task-005)`
   —— 含 `engine/src/rules/env_expand.rs`。
5. `feat(rules): 实现 SHA-256 校验加载器，校验失败拒绝并保留旧版 (#task-005)`
   —— 含 `engine/src/rules/loader.rs`。
6. `feat(rules): 把 whitelist_deny 接入白名单匹配器，与内置红线取并集 (#task-005)`
   —— 含 `engine/src/execute/whitelist.rs`（仅新增 `classify_with_external()`）、`engine/src/rules/loader.rs`（补 `external_deny_list()`）。
7. `test(rules): 补 JSON 反序列化、变量展开、篡改拒绝、并集生效测试 (#task-005)`
   —— 含 `engine/tests/rules_loader.rs`。
8. 自验全绿后，最后一次提交：`docs(state): 任务005完成，更新进度与下一步 (#task-005)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 把待办 005 从 `- [ ]` 改为 `- [x]`；
   - 「最近一次变更」追加一行（task-005 完成摘要）；
   - 「下一步计划」改为「派发 task-006：搭好 GitHub Actions CI（cargo clippy/fmt/test + pnpm lint/test + 规则 JSON schema 与 SHA-256 校验）」；
   - 「Git 提交记录」表追加本任务 8 条 commit 摘要；
   - 文件末尾「变更轨迹」追加 `2026-10-12  task-005  落地初始规则库与 SHA-256 校验加载器`。

**提交前自验（必须全绿，任一不过不得 push）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy -p engine --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0，§G 中至少 9 条测试用例全部通过（task-003 已有测试不允许回退）；
- `cargo fmt --all -- --check` 退出码 0；
- **人工核对 checksum**：`powershell -c "(Get-FileHash rules/cache_rules.json -Algorithm SHA256).Hash.ToLower()"` 的输出与 `rules/rules_meta.json` 里的 `checksum_sha256` 字段逐字符一致；DevAgent 把两条命令输出粘贴到实现备注。

## 输出要求

DevAgent 交付物（后续执行结果，不是本任务文档产出）：

1. 上述 7 个代码/配置提交 + 1 个 `docs(state)` 提交，共 8 个 commit，全部 push 到 `feature/task-005-rules-loader`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（与白名单逐条对照）；
   - `engine/src/execute/whitelist.rs` 的新增函数与未改动行；
   - 自验命令与退出码（逐条列）；
   - `cargo test -p engine` 通过输出（含 task-003 已有用例数）；
   - `Get-FileHash` 与 `rules_meta.json` checksum 字段的对照输出；
   - 与本任务文档不符的偏差（如有，含原因）；
   - 遗留问题（例如：rules_meta 落 DB、联网拉取、rules.checkUpdate IPC 命令分别留给哪个任务）；
3. `docs/PROJECT_STATE.md` 已按第 8 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务005-规则库与SHA256校验加载器 (#task-005)`，
   该提交只含 `tasks/005-rules-loader.md` 一个文件，落在 `feature/task-005-rules-loader` 分支最前端（即 DevAgent 开工前的基线）。

## 实现备注（DevAgent 回填，2026-10-12）

### 实际改动文件清单（对照白名单）
- `rules/cache_rules.json`（新建，照抄 TRD §4.10 npm_cache + 4 条红线）。
- `rules/rules_meta.json`（新建，checksum 现场 Get-FileHash 算出回填）。
- `engine/Cargo.toml`（仅 +sha2 0.10 / +hex 0.4）、`Cargo.lock`。
- `engine/src/lib.rs`（仅 +`pub mod rules;`）。
- `engine/src/rules/`：mod.rs / models.rs / env_expand.rs / loader.rs（新建）。
- `engine/src/execute/whitelist.rs`：**仅新增** `classify_with_external()`（第 99~119 行），`classify()` / 常量 / `check_action()` 一字未动。
- `engine/tests/rules_loader.rs`（新建，9 条集成用例）。
- `docs/PROJECT_STATE.md`、本任务文件。
- 未触碰：db/**、app/**、elevation/**、scripts/**、PRD/TRD/ARCHITECTURE 长期文档；未引入 reqwest/tokio；未硬编码规则内容到 Rust。

### Get-FileHash 与 rules_meta.checksum 对照
```
(Get-FileHash rules\cache_rules.json -Algorithm SHA256).Hash.ToLower()
  = bec1ab744ce2c3adab2c20e5397715340d9ffb35f8f733fcd410b3670aaebfd9
rules_meta.json.checksum_sha256
  = bec1ab744ce2c3adab2c20e5397715340d9ffb35f8f733fcd410b3670aaebfd9
MATCH=True（逐字符一致）
```

### 自验命令与退出码
| 命令 | 退出码 |
|------|--------|
| `cargo check --workspace --all-targets` | 0 |
| `cargo clippy -p engine --all-targets -- -D warnings` | 0（首次 while-let-on-iterator 告警已修为 `for nc in chars.by_ref()`） |
| `cargo test -p engine` | 0 |
| `cargo fmt --all -- --check` | 0（先 `cargo fmt --all` 落盘） |

### cargo test -p engine 输出
```text
12 passed  lib（含 env_expand 新增 3 条）
8 passed   db schema（task-002 无回退）
24 passed  execute_whitelist（task-003 无回退）
7 passed   ipc_commands（task-004 无回退）
3 passed   ipc_envelope
9 passed   rules_loader（本任务 §G 全覆盖）
1 passed   hello_engine
```

### 偏差说明
- 外部红线命中统一归为 `DenialReason::L3SystemPath`（未给枚举加新变体，遵守「whitelist.rs 仅新增函数」约束）；若后续审计需要区分内置/外部，再拆变体（已在 task-010 待办内）。
- `ExpandError` 新增第三变体 `TildeMasquerade(String)`（`~foo` 拒绝），任务文档未指定变体名，按测试断言对齐。

### 遗留问题
- rules_meta 落 DB（rules_meta 表）、GitHub Raw 联网拉取、`rules.checkUpdate/applyUpdate` IPC 命令：留待后续规则更新任务（对应 task-010 / M1 规则模块）。
- `external_deny_list` 当前直接喂 `classify_with_external` 前缀匹配；真实接入 cleanup.dryRun 由后续 cleanup.execute 任务完成。
