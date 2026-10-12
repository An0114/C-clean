# 测试评审报告

## 本次任务名称

005 落地初始规则库 cache_rules.json + rules_meta.json + SHA-256 校验加载器（与内置 L3 白名单并集）

## Git 提交记录核对结果

✅ 11 个提交均关联 `#task-005`，type 前缀规范（docs / build / feat / test），无 `#debt-cleanup` 或其他任务号混杂。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `2b4c8b8` | docs(task) | tasks/005-rules-loader.md | ✅ |
| 2 | `649c005` | build(deps) | Cargo.lock + engine/Cargo.toml | ✅ |
| 3 | `e7477e6` | feat(rules) | rules/cache_rules.json + rules_meta.json | ✅ |
| 4 | `5ba121a` | feat(rules) | lib.rs + rules/mod.rs + models.rs | ✅ |
| 5 | `312683e` | feat(rules) | rules/env_expand.rs + mod.rs | ✅ |
| 6 | `d218c80` | feat(rules) | rules/loader.rs + mod.rs | ✅ |
| 7 | `ccf6c69` | feat(rules) | whitelist.rs + loader.rs + mod.rs | ✅ |
| 8 | `480022d` | test(rules) | tests/rules_loader.rs + whitelist.rs + env_expand.rs | ⚠️ 见下 |
| 9 | `08dc855` | docs(state) | PROJECT_STATE.md | ✅ |
| 10 | `650fca9` | docs(task) | tasks/005-rules-loader.md | ✅ |
| 11 | `9d9a3a4` | docs(state) | PROJECT_STATE.md | ✅ |

**偏差记录（轻微，不阻断）**：
- `480022d` 测试提交中混入了两处业务源码改动：① `whitelist.rs` 函数签名折行（纯 fmt，无逻辑变化）；② `env_expand.rs` 将 `while let Some(nc) = chars.next()` 改为 `for nc in chars.by_ref()`（修 clippy `while_let_on_iterator` 告警，无逻辑变化）。这两处本应在对应的 feat 提交里完成，放在 test 提交里属于原子性瑕疵，但改动内容经 diff 逐行确认**零业务逻辑变更**，仅 fmt/clippy 修正。
- 未越权回写 TRD/PRD/ARCHITECTURE（与 task-004 不同，本次无流程越权）。

## 分步验证清单

### A. 仓库静态规则文件

- ✅ `rules/cache_rules.json` 合法 JSON，顶层三字段齐全，`version="1.0.0"`。
- ✅ `modules.A[0]` = npm_cache，必填 9 字段齐全，`paths` 含 `%LOCALAPPDATA%\npm-cache` 与 `~/.npm/_cacache`，`risk_level="L1"`，`elevated=false`。
- ✅ `whitelist_deny` 4 条齐全（Desktop / Documents / System32 / pagefile.sys）。
- ✅ JSON 内无可执行代码、`$()`、`${}`、命令字符串；无 `%WINDIR%` / `%PROGRAMFILES%` 等白名单外变量。
- ✅ `rules_meta.json` 5 字段齐全，`checksum_sha256` 经评审人 `Get-FileHash` 现场复算逐字符一致：
  - 实际 = `bec1ab744ce2c3adab2c20e5397715340d9ffb35f8f733fcd410b3670aaebfd9`
  - meta = `bec1ab744ce2c3adab2c20e5397715340d9ffb35f8f733fcd410b3670aaebfd9`
  - MATCH = True。小写 hex，无 `0x` 前缀。

### B. Rust 模块与依赖

- ✅ `engine/src/rules/` 含 mod.rs / models.rs / env_expand.rs / loader.rs 四文件。
- ✅ `engine/src/lib.rs` 仅新增 `pub mod rules;`（原 db / execute / ipc 模块声明未动）。
- ✅ `engine/Cargo.toml` 仅新增 `sha2 = "0.10.9"` + `hex = "0.4.3"`；未引入 reqwest / tokio / serde_yaml / ureq。

### C. serde 模型

- ✅ `CacheRules` / `RuleEntry` / `RulesMeta` 字段与任务文档逐一对齐，snake_case，`custom_rules` 为 `serde_json::Value`。
- ✅ loader 遍历 modules 识别 `risk_level == "L3"` 即返回 `L3InRules(entry.id)`。

### D. 路径变量白名单展开

- ✅ `expand()` 支持 5 个白名单环境变量（大小写不敏感）+ `~` 前缀。
- ✅ `%WINDIR%` / `%ProgramFiles%` → `UnknownVar`。
- ✅ 环境变量未设置 → `VarUnset`。
- ✅ `~foo` → `TildeMasquerade`（评审人读码确认 L39-42：strip_prefix('~') 后 rest 非空且不以 `\` 或 `/` 开头即拒绝）。
- ✅ 未闭合 `%` 原样保留；正斜杠归一为反斜杠。
- ✅ 不做 `..` 规范化（交由 task-003 normalize）。

### E. SHA-256 校验加载器

- ✅ `LoadedRules` / `LoadError` 枚举五变体（Io / Json / ChecksumMismatch / L3InRules / Expand）签名与文档一致。
- ✅ `load()` 流程严格按序：读两文件字节 → 反序列化 meta → 算 SHA-256 → 大小写敏感比对 → 反序列化 cache → L3 检查 → 变量展开校验。
- ✅ checksum 不符即返回 `ChecksumMismatch { expected, actual }`，不写盘、不返回半成品。
- ✅ 不做备份恢复（留后续任务）。

### F. 与 task-003 白名单并集

- ✅ `external_deny_list()` 返回展开后的 whitelist_deny 绝对路径列表。
- ✅ `classify_with_external()` 先跑内置 `classify(p)?`，再跑 external 前缀匹配；原 `classify()` 与内置红线常量一字未动（L105-115 为新增函数，L1-97 未改）。
- ✅ 前缀匹配要求路径等于外部红线或以 `\` 延伸（`lower == ext_lower || lower.starts_with("{ext}\\")`），防止 `C:\Windows\System32Evil` 这类前缀误匹配。
- ✅ 并集语义：内置红线 + external 任一命中即拒；external 不能放宽内置（单测 `external_deny_list_union_with_builtin` 验证 System32 仅靠内置红线仍拒、空 external 内置仍生效）。

### G. 单元测试

- ✅ `cargo test -p engine` 全绿（评审人复跑）：
  - 12 passed  lib（含 env_expand 3 条新增）
  - 8 passed   db_schema（task-002 无回退）
  - 24 passed  execute_whitelist（task-003 无回退）
  - 7 passed   ipc_commands（task-004 无回退）
  - 3 passed   ipc_envelope
  - 9 passed   rules_loader（本任务 §G 9 条用例全覆盖）
  - 1 passed   smoke
  - **合计 64 passed / 0 failed**。
- ✅ 9 条集成测试覆盖：合法加载、篡改 cache 拒绝、meta 错 checksum 拒绝、未知环境变量拒绝、L3 规则拒绝、6 个白名单变量展开、~伪装拒绝、并集语义、内置红线回归保护。
- ✅ 用 tempfile 造目录，程序算 SHA-256，不硬编码 hash；不发网络请求、不读真实 rules/。

### H. 卫生与边界

- ✅ 未创建 scan/、safebox/、audit/、platform/ 等其他子模块。
- ✅ 未修改 db/**、app/**、elevation/**、scripts/**。
- ✅ 未修改 PRD/TRD/ARCHITECTURE/AGENTS。
- ✅ 未引入网络栈依赖。
- ✅ cache_rules.json 内容从文件读，未硬编码到 Rust。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 64 passed / 0 failed |
| `cargo clippy -p engine --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `Get-FileHash rules/cache_rules.json` vs `rules_meta.json.checksum_sha256` | ✅ MATCH |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **`external_deny_list` 用 `filter_map(|raw| expand(raw).ok())` 静默吞错**（loader.rs L97-104）。注释已说明"load() 已保证全部可展开，unwrap 不会失败"，逻辑正确；但如果未来有人绕过 `load()` 直接构造 `LoadedRules`，展开失败会被静默过滤掉而非报错。当前无实际风险（`LoadedRules` 字段为 pub 但构造路径唯一），建议后续改为 `expect("load() invariant")` 或在 `LoadedRules` 上加 `#[non_exhaustive]` 防外部构造。**第三档，不阻断。**

2. **外部红线命中统一归 `DenialReason::L3SystemPath`**（DevAgent 已声明偏差）。遵守"whitelist.rs 仅新增函数、不改枚举"约束，正确；但审计上无法区分"内置 System32"与"外部 Desktop"。已登记 task-010。**第三档。**

3. **`ExpandError::TildeMasquerade` 为新增变体**（任务文档未指定变体名）。DevAgent 按测试断言对齐，合理。**无问题。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制（路径遍历） | ✅ env_expand 不做 `..` 规范化，交由 task-003 normalize；normalize 已拒 `..`。规则文件路径在进入白名单匹配前会再过 normalize。 |
| A02 加密失败 | ✅ SHA-256 用于规则文件完整性校验；不涉及机密数据加密。 |
| A03 注入 | ✅ 规则文件为纯 JSON，无 `$()` / `${}` / 命令字符串；环境变量白名单拒绝 `%WINDIR%` 等任意变量，防止规则文件通过环境变量注入任意路径。 |
| A04 不安全设计 | ✅ checksum 校验失败即拒绝加载、不返回半成品、fail-closed；L3 规则不允许出现在可清理条目；内置红线永不被外部规则移除（并集而非替换）。 |
| A05 安全配置错误 | ✅ rules_meta.json 静态打包在仓库，downloaded_at=0，不联网；无默认密码/密钥。 |
| A07 身份认证失败 | N/A 本任务无认证。 |
| A08 软件与数据完整性 | ✅ SHA-256 校验链路完整：读字节 → 算 hash → 与 meta 严格比对（大小写敏感）→ 不符即拒。篡改 cache_rules.json 末尾加一个空格即被单测捕获。 |
| A09 日志监控失败 | N/A 留后续审计任务。 |
| A10 SSRF | ✅ 本任务不联网，无 reqwest/ureq；source_url 仅作记录字符串，不发起请求。 |

### 专项安全分析

- **规则文件防篡改**：cache_rules.json 被篡改（哪怕加一个空格）→ SHA-256 不匹配 → 拒绝加载 → 调用方保留旧版。链路正确。
- **环境变量注入**：规则文件只能引用 5 个白名单环境变量 + `~`；`%WINDIR%` 等被拒。防止规则文件通过环境变量间接指向任意系统路径。
- **`~` 伪装**：`~foo` 不展开为 home，防止用户把规则写成 `~foo/secret` 试图伪装成用户目录。
- **L3 规则禁止**：规则文件里 risk_level="L3" 的条目被 loader 拒绝，防止规则文件把系统关键路径标记为可清理。
- **内置红线保护**：classify_with_external 先跑内置 classify 再跑 external；即使规则文件漏写某条内置红线，内置红线依然生效。并集语义正确。
- **前缀匹配精确性**：external 前缀匹配要求 `\` 延伸，防止 `C:\Windows\System32Evil` 误命中 `C:\Windows\System32`。

## 评审结论：**通过**

24 条验收标准全部满足；SHA-256 校验链路正确；环境变量白名单防注入；L3 规则禁止；内置红线不被外部规则放宽；64 测试全绿；clippy/fmt exit 0；checksum 经评审人现场复算一致。

## Git 操作记录

- **merge commit**：`23c8154`（`git merge --no-ff feature/task-005-rules-loader`）
- **本地 feature 分支**：已删除（`git branch -d feature/task-005-rules-loader`）
- **远端 feature 分支**：保留不动（由维护者统一 push main 和清理远端）
- **评审报告 commit**：单独提交 `docs(review): 任务005评审报告 (#task-005)`

## 后续动作建议

1. **task-006**：搭 GitHub Actions CI（cargo clippy/fmt/test + pnpm lint/test + 规则 JSON schema 与 SHA-256 校验）。
2. **task-010**：外部红线集注入——将 `classify_with_external` + `external_deny_list` 接入 cleanup.dryRun；如需区分内置/外部红线来源，再拆 DenialReason 变体。
3. **M1 规则更新任务**：rules_meta 落 DB、GitHub Raw 联网拉取、`rules.checkUpdate`/`rules.applyUpdate` IPC 命令。
4. **轻微技术债务**（第三档，不阻断）：`external_deny_list` 的 `filter_map().ok()` 静默吞错建议后续改为 expect 或加 `#[non_exhaustive]`。
