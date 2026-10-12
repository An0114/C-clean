# 测试评审报告

## 本次任务名称

010 外部红线集注入——rules JSON 的 whitelist_deny 接入 cleanup.dryRun

## Git 提交记录核对结果

✅ 8 个提交全部关联 `#task-010`，type 前缀规范（docs / feat / test），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `49dcf65` | docs(task) | tasks/010-external-deny-wiring.md | ✅ |
| 2 | `4346211` | feat(ipc) | dry_run.rs + db_resolver.rs + ipc_commands.rs | ✅ |
| 3 | `28738fb` | feat(tauri) | app/src-tauri/src/commands.rs | ✅ |
| 4 | `6caf900` | test(ipc) | engine/tests/ipc_dry_run_external.rs | ✅ |
| 5 | `2f7f9d6` | docs(state) | docs/PROJECT_STATE.md | ✅ |
| 6 | `b882688` | docs(task) | tasks/010-external-deny-wiring.md | ✅ |
| 7 | `6ae6d22` | test(ipc) | ipc_dry_run_external.rs（修正用例） | ✅ |
| 8 | `3797d89` | docs(state) | docs/PROJECT_STATE.md | ✅ |

`4346211` 动 db_resolver.rs / ipc_commands.rs 是签名变更后已有测试调用点需加 `&[]`，属必要连锁改动。

## 分步验证清单

### A. handle_dry_run 签名

- ✅ 签名加 `external_deny: &[String]`。
- ✅ 函数内 `classify(path)` → `classify_with_external(path, external_deny)`。
- ✅ 已有测试调用点（ipc_commands.rs 3 处 + db_resolver.rs 1 处）均传 `&[]`，保持原有语义。

### B. Tauri command 接线

- ✅ `load_external_deny()` 辅助函数：`current_dir()/rules` → `load_rules()` → `external_deny_list()`。
- ✅ 加载失败 → `Envelope::err(INTERNAL)`=5001，不返回 dryRun 结果（fail-closed）。
- ✅ rules 路径硬编码 `current_dir()/rules`，注释标「M1 打包后改为 Tauri 资源目录」。
- ✅ MockResolver::new(vec![]) 仍空，M1 切 DbItemPathResolver。

### C. 单测

- ✅ `dry_run_external_deny_blocks_path`：外部红线 `%LOCALAPPDATA%\SomeSensitiveCache`（非内置 PERSONAL_DIRS）命中，code=2002，denial_reason=L3SystemPath。
- ✅ `dry_run_no_external_deny_falls_back_to_builtin`：`&[]` 时 Desktop 仍被内置 L3PersonalDir 拒。
- ✅ `dry_run_builtin_still_blocks_when_rules_empty`：System32 被内置红线拒。
- ✅ 测试用 tempfile 造临时 rules/，程序算 SHA-256，不硬编码 hash。

### D. 卫生与边界

- ✅ 未改 rules/**、execute/whitelist.rs、db/**、progress.rs、envelope.rs。
- ✅ 未改 app/src/**。
- ✅ 未新增依赖。
- ✅ 未写 Tauri 资源协议解析（留 M1 打包）。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 80 passed / 0 failed（13 lib + 4 db_resolver + 8 db_schema + 7 execute_resolve + 24 execute_whitelist + 7 ipc_commands + 3 ipc_dry_run_external + 3 envelope + 10 rules_loader + 1 smoke） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0 |
| task-003/004/005/008/009 已有测试 | ✅ 无回退 |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **rules 路径硬编码 current_dir()/rules**：开发期可用，打包后需改 Tauri 资源目录。已注释标记，留 M1。**第三档。**
2. **cleanup_dry_run 每次调用都重新 load_rules()**：未缓存已加载的规则集。频繁调用时重复读盘+算 hash。当前调用频率低（用户手动触发），无性能问题；M1 可在 Tauri State 里缓存。**第三档。**
3. **外部红线命中统一归 L3SystemPath**（task-005 已知偏差）：无法区分内置 System32 与外部 whitelist_deny。已在已知问题段保留。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ 并集语义：内置 L3 + external 任一命中即拒；external 不能放宽内置红线（测试 2/3 验证 &[] 时内置仍生效）。 |
| A03 注入 | ✅ rules JSON 经 serde 反序列化，不执行；路径经 normalize + classify_with_external 判定。 |
| A04 不安全设计 | ✅ rules 加载失败 → 5001 INTERNAL，fail-closed，不静默放行。 |
| A08 数据完整性 | ✅ cache_rules.json 的 SHA-256 校验在 loader.rs（task-005），篡改即拒。 |

### 专项安全分析

- **外部红线会不会绕过内置红线？** 不会。`classify_with_external` 先跑内置 `classify()`，再跑 external。即使 rules 里 whitelist_deny 少写了 Desktop，内置 L3PersonalDir 仍生效。测试 `dry_run_no_external_deny_falls_back_to_builtin` 验证。✅
- **rules 文件被篡改会怎样？** loader.rs 的 SHA-256 校验挡住（task-005）。checksum mismatch → load() 失败 → load_external_deny() 返回 Err → cleanup_dry_run 返回 5001。✅
- **rules 文件缺失会怎样？** load() 返回 Io 错误 → 5001。fail-closed。✅
- **外部红线命中记什么 reason？** L3SystemPath（外部桶），不影响安全判断。✅

## 评审结论：**通过**

外部红线集已正确接入 cleanup.dryRun；并集语义正确（内置不被 external 放宽）；加载失败 fail-closed 返 5001；80 测试全绿（含 3 个新增外部红线端到端测试）；clippy/fmt/pnpm build 全过；8 个提交均关联 #task-010 无混杂。

## Git 操作记录

- **merge commit**：`9331522`（`git merge --no-ff feature/task-010-external-deny-wiring`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-010-external-deny-wiring`，was 3797d89）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务010评审报告 (#task-010)`

## 后续动作建议

1. **task-011**：ps1 兼容层来源对齐与 NDJSON 改造（scripts/ 五个脚本）。
2. **M1 打包**：把 rules 路径从 current_dir()/rules 改为 Tauri 资源目录；在 Tauri State 里缓存已加载规则。
3. **M1 A 模块**：真实扫描器写入 cleanup_item 后，把 cleanup_dry_run 的 MockResolver 切换为 DbItemPathResolver。
