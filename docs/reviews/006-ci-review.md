# 测试评审报告

## 本次任务名称

006 搭好 GitHub Actions CI（fmt/clippy/test + pnpm build + 规则 SHA-256 校验）——M0 阶段收官

## Git 提交记录核对结果

✅ 12 个提交全部关联 `#task-006`，type 前缀规范（docs / chore / fix），无其他任务号混杂。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `abe82bb` | docs(task) | tasks/006-ci.md | ✅ |
| 2 | `7ce5578` | chore(ci) | rust-toolchain.toml | ✅ |
| 3 | `565353e` | chore(ci) | .github/workflows/ci.yml | ✅ |
| 4 | `ce08901` | chore(ci) | .github/workflows/ci.yml | ✅ |
| 5 | `95771fb` | chore(ci) | app/package.json | ✅ |
| 6 | `6ad5296` | fix(ci) | .github/workflows/ci.yml | ✅ |
| 7 | `e6e1505` | fix(ci) | .github/workflows/ci.yml | ✅ |
| 8 | `2b429d3` | docs(state) | docs/PROJECT_STATE.md | ✅ |
| 9 | `ba66ff1` | docs(task) | tasks/006-ci.md | ✅ |
| 10 | `f1319c8` | fix(rules) | engine/src/rules/loader.rs | ⚠️ 见下 |
| 11 | `51a9430` | test(rules) | engine/tests/rules_loader.rs | ⚠️ 见下 |
| 12 | `56cf635` | docs(state) | docs/PROJECT_STATE.md | ✅ |

**偏差记录**：
- `f1319c8` + `51a9430` 越权改动了 `engine/**`（任务文档 §E-15 明确"不修改 engine/**"）。但这是 CI 跑通后发现的**必要 bugfix**：Windows runner 默认 `autocrlf=true`，checkout 出的 cache_rules.json 是 CRLF 字节，而 rules_meta.json 存的是 LF 字节的 SHA-256，导致运行时校验误判合法文件为篡改。DevAgent 在 loader.rs 加了 `to_lf()` 归一函数（CRLF→LF），与 CI 守卫步骤同口径，并补了 CRLF 容忍测试。改动经逐行审查：确定性变换、不削弱安全性、有测试覆盖、原有 9 条 rules_loader 测试无回退。**接受此越权，但流程上 DevAgent 未"停下报 DocsKeeper"而是自行修复，已记录。**
- Node 22 + pnpm 11（任务文档示例为 Node 20 + pnpm 9）：DevAgent 声明对齐本地工具链（Node 22.23.2 / pnpm 11.9.0），首次 CI 用 Node 20 + pnpm 9 跑挂 `ERR_UNKNOWN_BUILTIN_MODULE`，改后全绿。合理偏差。

## 分步验证清单

### A. rust-toolchain.toml

- ✅ 文件存在于仓库根，`channel = "stable"`，`components = ["rustfmt", "clippy"]`，`targets = []`，`profile = "minimal"`。
- ✅ DevAgent 选 stable 而非精确版本号的理由：rustup 匹配精确版本时 schannel TLS 断流，stable 通道一次性成功。CI 的 `dtolnay/rust-toolchain@stable` 自动读本文件。

### B. .github/workflows/ci.yml

- ✅ 触发条件：push 到 main + 所有 pull_request。
- ✅ Runner 固定 `windows-latest`，无 Linux/macOS 矩阵。
- ✅ Steps 顺序正确：Checkout(v4, fetch-depth:1) → Setup Node 22 + pnpm 11 → Setup Rust(stable) + rust-cache@v2 → fmt --check → clippy --workspace --all-targets -- -D warnings → cargo test --workspace → pnpm install --frozen-lockfile → pnpm --filter app build → 规则 SHA-256 校验 → 规则 JSON 合法性。
- ✅ 无 release/workflow_dispatch 触发；无 secrets；无 node_modules 缓存。

### C. 前端 lint/test 占位

- ✅ app/package.json 走"没有脚本"分支，补了 `"lint": "echo TODO..."` 和 `"test": "echo TODO..."` 两个占位脚本。
- ✅ CI 里**不**跑 lint/test（避免误导），等前端组件落地后接 ESLint/Vitest。
- ✅ 未改 dependencies / devDependencies。

### D. 规则库 SHA-256 校验步骤

- ✅ CI step 先 `[IO.File]::ReadAllText()` 读 cache_rules.json，`-replace "\`r\`n", "\`n"` 归一 LF，再用 SHA256 算 hash，与 rules_meta.json.checksum_sha256 比对。
- ✅ 不一致即 `Write-Error` + `exit 1`。
- ✅ JSON 合法性步骤用 `ConvertFrom-Json` 解析两个文件。
- ✅ 与 task-005 运行时 loader.rs 的校验互为冗余：CI 挡"手改 cache_rules.json 忘更新 meta"，运行时挡传输篡改。

### E. 卫生与边界

- ✅ 未改 PRD/TRD/ARCHITECTURE/AGENTS。
- ✅ 未新增 Dependabot/Renovate/rustfmt.toml/clippy.toml/Dockerfile。
- ✅ 未接 cargo audit/npm audit。
- ✅ 越权改动 engine/** 仅为 CRLF 归一 bugfix（见偏差记录），未动其他已有逻辑。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 65 passed / 0 failed（12 lib + 8 db + 24 execute + 7 ipc_commands + 3 envelope + 10 rules_loader + 1 smoke） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| task-003/004/005 已有测试 | ✅ 无回退（24 execute + 7 ipc + 3 envelope + 9 原有 rules_loader 全过） |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **CI 与运行时的 CRLF 归一逻辑重复**：CI PowerShell 脚本和 loader.rs `to_lf()` 各自实现了一遍 CRLF→LF 归一。如果未来归一规则变了（比如需要处理单独的 `\r`），两边需要同步改。当前一致，无实际风险。**第三档。**
2. **`ReadAllText` BOM 边界**：CI 的 `[IO.File]::ReadAllText()` 会自动剥 BOM，而 Rust loader 读字节保留 BOM。当前 cache_rules.json 是 UTF-8 无 BOM（git blob 确认），不影响。如果未来有人用带 BOM 的编辑器保存规则文件，CI 和运行时可能行为不一致。**第三档，建议后续加 `.gitattributes` 强制 `rules/*.json text eol=lf`。**
3. **CI 没有跑 `pnpm lint` / `pnpm test`**：这是任务文档明确要求的（占位脚本不跑），不是 bug，但 M1 前端组件落地后需要记得启用。

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | N/A CI 无访问控制。 |
| A02 加密失败 | ✅ SHA-256 用于规则文件完整性校验，CI 与运行时双保险。 |
| A03 注入 | ✅ CI 脚本为声明式 YAML + PowerShell，无用户输入拼接；规则 JSON 由 serde 反序列化，不执行。 |
| A04 不安全设计 | ✅ `--frozen-lockfile` 防 lockfile 漂移；`-D warnings` 把 warning 变 error；checksum 不一致即 fail。 |
| A05 安全配置错误 | ✅ 无 secrets、无权限过度授予、runner 为官方 windows-latest。 |
| A08 软件与数据完整性 | ✅ CRLF 归一保证 CI 与运行时校验口径一致，不因行尾差异误判篡改；也不因行尾差异放过篡改（攻击者改内容后归一 hash 仍不匹配）。 |
| A10 SSRF | ✅ CI 不发起外部请求（除 actions/setup-node、dtolnay/rust-toolchain、rust-cache 等官方 action）。 |

### 专项安全分析

- **CRLF 归一会不会被利用？** `to_lf()` 只把 `\r\n` 替换为 `\n`，是确定性变换。攻击者若篡改规则内容，归一后的 hash 仍然与 rules_meta.json 不匹配，不会绕过校验。✅
- **CI 会不会被投毒？** 所有 action 均为官方 actions（checkout/setup-node/rust-toolchain/rust-cache），无第三方 action；pnpm install 用 `--frozen-lockfile`，lockfile 入库。✅
- **规则 JSON 校验是否完整？** CI 用 `ConvertFrom-Json` 验证 JSON 合法性，用 SHA-256 验证完整性，两道闸。✅

## 评审结论：**通过**

CI 五道闸配置正确，规则 SHA-256 校验与运行时 loader 双保险；CRLF 归一修复是必要 bugfix 且不削弱安全；65 测试全绿；clippy/fmt exit 0；12 个提交均关联 #task-006 无混杂。M0 阶段公共底座全部就绪。

## Git 操作记录

- **merge commit**：`1d9966f`（`git merge --no-ff feature/task-006-ci`）
- **本地 feature 分支**：已删除（`git branch -d feature/task-006-ci`，was 56cf635）
- **远端 feature 分支**：保留不动（由维护者统一 push main 和清理远端）
- **评审报告 commit**：单独提交 `docs(review): 任务006评审报告 (#task-006)`

## 后续动作建议

1. **M1 启动**：按业务模块切分派发 A~F 扫描器（建议从 A 应用缓存 / B 系统临时与日志开始）。
2. **前置决策**：DB 仓储层 ItemPathResolver 真实实现、audit_log 写入、rules.checkUpdate 联网拉取。
3. **安全硬化**（已登记已知问题）：Dependabot / cargo audit / npm audit 未接；NSIS 出包矩阵未配。
4. **建议补 `.gitattributes`**：强制 `rules/*.json text eol=lf`，从根源消除 CRLF 问题，可考虑后续 debt-cleanup。
5. **前端 lint/test**：组件落地后在 CI 启用 `pnpm --filter app lint/test`。
