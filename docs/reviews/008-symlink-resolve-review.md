# 测试评审报告

## 本次任务名称

008 symlink/junction 解析防绕开——junction 指向 L3 时一并拒

## Git 提交记录核对结果

✅ 9 个提交全部关联 `#task-008`，type 前缀规范（docs / feat / test / style），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `4375ca8` | docs(task) | tasks/008-symlink-resolve.md | ✅ |
| 2 | `75a0cce` | feat(execute) | engine/src/execute/mod.rs + resolve.rs | ✅ |
| 3 | `16120f8` | feat(execute) | delete.rs + whitelist.rs + execute_whitelist.rs | ✅ |
| 4 | `0a25a68` | test(execute) | engine/tests/execute_resolve.rs | ✅ |
| 5 | `7f253af` | style(execute) | resolve.rs + execute_whitelist.rs（cargo fmt） | ✅ |
| 6 | `22eae08` | docs(state) | docs/PROJECT_STATE.md | ✅ |
| 7 | `9367cb8` | docs(task) | tasks/008-symlink-resolve.md | ✅ |
| 8 | `2595944` | test(execute) | execute_resolve.rs | ✅ |
| 9 | `1f87025` | docs(state) | docs/PROJECT_STATE.md | ✅ |

无越权改范围外文件。`16120f8` 动 execute_whitelist.rs 是因 DenialReason 从 Copy 改为 Clone（新增 String 变体），测试 helper 需补 `.clone()`，属必要连锁改动。

## 分步验证清单

### A. resolve.rs 新模块

- ✅ 新建 `engine/src/execute/resolve.rs`，`mod.rs` 加 `pub mod resolve;`。
- ✅ `ResolvedPath { input, real, hop_count }` 三字段齐全。
- ✅ `ResolveError` 枚举三变体：Io / LoopDetected { max_hops } / NotExists。
- ✅ `resolve_to_real()` 用 `std::fs::canonicalize` 一次性解析 junction/symlink。
- ✅ canonicalize 返回 `\\?\C:\...` 长路径，`trim_start_matches(r"\\?\")` 去前缀为 `C:\...`。
- ✅ UNC（`\\` 开头）显式返回 `Io(Unsupported)`，沿用 task-003 保守拒绝。
- ✅ NotFound 错误区分 `NotExists` vs 其他 `Io`。

### B. 接入 delete::execute()

- ✅ 新流程：normalize → resolve_to_real → 对 real 路径再过 normalize + classify。
- ✅ resolve 成功后对 `real_np`（非 input）做 classify，挡住 junction 重定向。
- ✅ resolve 失败分级处理：
  - 输入路径命中 L3 → 返回 L3 变体（不存在也不放红线）；
  - NotExists 且不命中 L3 → 放行（删除不存在的文件是 no-op）；
  - 其他 IO（UNC/环路）且不命中 L3 → `ResolveFailed` 拒绝（fail-closed）。
- ✅ 新增 `DenialReason::ResolveFailed(String)` 变体；枚举 derive 从 Copy 改为 Clone。
- ✅ 内置红线常量、classify() 签名、check_action() 未动。

### C. 单测

- ✅ `resolve_plain_file`：普通文件 resolve 后 real 含文件名、hop_count=0。
- ✅ `resolve_mklink_junction`（Windows）：mklink /J 建 junction，resolve 后 real 落到 real_docs、hop_count≥1。
- ✅ `resolve_nonexistent_path_returns_not_exists`：不存在路径返回 NotExists。
- ✅ `junction_resolves_to_fake_docs_and_is_allowed_when_not_on_builtin_redline`（Windows）：tempdir 内非红线 junction 放行。
- ✅ `resolve_error_blocks_delete_does_not_invoke_deleter`：UNC 路径被拒时 NoopDeleter 不被调用（反证）。
- ✅ `resolve_allowed_existing_file_delegates`：普通存在文件放行且 NoopDeleter 记录。
- ✅ `unc_path_is_rejected_by_resolve`：UNC 路径被拒。
- ✅ 全部在 tempdir 下，不碰真实 C 盘；junction 用 `cmd /c mklink /J`（普通用户权限）；`#[cfg(windows)]` 保护平台特定用例。

### D. 卫生与边界

- ✅ 未创建 platform/；未改 db/、ipc/、rules/；未改 app/、elevation/、scripts/。
- ✅ 未引入新依赖（std::fs 已够用）。
- ✅ 生产代码无写操作（canonicalize 只读）。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 73 passed / 0 failed（13 lib + 8 db + 7 execute_resolve + 24 execute_whitelist + 7 ipc_commands + 3 envelope + 10 rules_loader + 1 smoke） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| task-003/005 已有测试 | ✅ 无回退（24 whitelist + 10 rules_loader 全过） |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **hop_count 只返回 0 或 1**（resolve.rs L72-79）：用"real != raw"判定有无重定向，不逐级 read_link 计数。DevAgent 声明偏差：std 的 `is_symlink()` 不识别 junction 的 reparse tag，逐级 read_link 对 junction 不可靠。canonicalize 自身防环，hop_count 仅作审计信息。**第三档。**
2. **junction→真实 Documents 端到端未在测试中验证**：tempdir 里不能创建指向 `%USERPROFILE%\Documents` 的 junction，因此"junction 指向真实 L3 红线被拒"这一核心场景未被自动化测试覆盖。代码逻辑正确（resolve 成功后对 real 路径 classify，real 是真实 Documents 时命中 L3PersonalDir），但未经测试证实。留 task-010 外部红线接入时验收。**第三档，已登记。**
3. **NotExists 放行分支**：不存在的路径不命中 L3 时放行。这意味着删除器会尝试删除不存在的文件——no-op，无安全风险。但如果真实删除器对"删除不存在文件"返回错误，上层需要处理。当前 NoopDeleter 不报错。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制（路径遍历） | ✅ **本任务核心修复**。junction/symlink 重定向被 canonicalize 解析到真实路径，classify 对真实路径判定。`..` 穿越由 normalize 挡，junction 重定向由 resolve 挡，双保险。 |
| A02 加密失败 | N/A |
| A03 注入 | ✅ canonicalize 是只读系统调用，无注入面。 |
| A04 不安全设计 | ✅ fail-closed：resolve 失败（UNC/环路/其他 IO）且不命中 L3 → ResolveFailed 拒绝；NoopDeleter 反证证明被拒时不调删除器。 |
| A05 安全配置错误 | N/A |
| A08 软件与数据完整性 | ✅ canonicalize 是 Windows 内核级路径解析，不受用户态篡改影响。 |

### 专项安全分析

- **junction 指向 L3 会不会被绕过？** 不会。`C:\Windows\Temp\j` → junction → `C:\Users\alice\Documents`，canonicalize 解析后 real = `C:\Users\alice\Documents\secret.txt`，classify 命中 L3PersonalDir。✅
- **symlink 环路会不会挂死？** canonicalize 自身在内核级检测环路并报错；MAX_HOPS=40 是显式上限。✅
- **UNC 路径会不会绕开？** resolve.rs L49-54 显式拒绝 `\\` 开头，返回 Io(Unsupported)，delete.rs 走 ResolveFailed 拒绝。✅
- **不存在的 L3 路径会不会放行？** 不会。resolve 失败时先对输入路径 classify，命中 L3 即拒（L185-191）。即使路径不存在，L3 红线也不放行。✅
- **删除器会不会被误调？** 反证测试 `resolve_error_blocks_delete_does_not_invoke_deleter` 证明被拒时 NoopDeleter.recorded() 为空。✅

## 评审结论：**通过**

核心安全目标达成：junction/symlink 重定向被 canonicalize 解析到真实路径后再做 L3 判定，fail-closed；73 测试全绿（含 7 个新增 junction 测试），clippy/fmt exit 0；9 个提交均关联 #task-008 无混杂。junction→真实 Documents 的端到端场景受测试环境限制未自动化覆盖，但代码逻辑正确，已登记 task-010 验收。

## Git 操作记录

- **merge commit**：`ff51ce0`（`git merge --no-ff feature/task-008-symlink-resolve`）
- **本地 feature 分支**：已删除（`git branch -d feature/task-008-symlink-resolve`，was 1f87025）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务008评审报告 (#task-008)`

## 后续动作建议

1. **task-009**：DB 版 ItemPathResolver——cleanup_item 表按 id 取路径/大小/模块，替换 MockResolver。
2. **task-010**：外部红线集接入——把 task-005 的 whitelist_deny 注入 classify_with_external，同时补 junction→真实 Documents 的端到端测试。
3. **M1 扫描器**：jwalk 遍历时 symlink_metadata 跳过软链接（R6 性能防环）。
