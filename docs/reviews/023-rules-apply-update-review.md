# 测试评审报告

## 本次任务名称

023 rules.applyUpdate——下载新 cache_rules.json、SHA-256 校验、替换本地、upsert rules_meta

## Git 提交记录核对结果

✅ 8 个提交全部关联 `#task-023`，type 前缀规范（docs / feat / test），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `4add0d7` | docs(task) | tasks/023-rules-apply-update.md | ✅ |
| 2 | `1db76ec` | feat(rules) | remote.rs（fetch_remote_rules_bytes） | ✅ |
| 3 | `5691f59` | feat(rules) | mod.rs + remote.rs（apply_update） | ✅ |
| 4 | `49f784a` | feat(tauri) | commands.rs + lib.rs | ✅ |
| 5 | `0bc8bfb` | test(rules) | rules_apply_update.rs | ✅ |
| 6 | `d3eee2e` | docs(state) | PROJECT_STATE.md | ✅ |
| 7 | `45f7cff` | docs(task) | tasks/023-rules-apply-update.md | ✅ |
| 8 | `225829c` | docs(state) | PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. Engine 层：远端拉取 rules 字节

- ✅ REMOTE_RULES_URL 硬编码（GitHub Raw cache_rules.json）。
- ✅ fetch_remote_rules_bytes：GET → error_for_status → bytes → Vec<u8>。

### B. Engine 层：apply_update 安全替换

- ✅ ApplyResult { new_version, new_sha256 }。
- ✅ ApplyError：ShaMismatch / Io / Db / RollbackFailed。
- ✅ SHA-256 校验：expected != actual → ShaMismatch，**不碰本地文件**。
- ✅ 备份：cache_rules.json → .bak，rules_meta.json → .bak。
- ✅ 写新文件：cache_rules.json = remote_rules_bytes；rules_meta.json = 序列化 remote_meta（含 source_url/downloaded_at/custom_rules）。
- ✅ upsert_active_rules_meta 到 DB。
- ✅ 失败回滚：从 .bak 恢复原文件。

### C. Tauri rules_apply_update 命令

- ✅ fetch_remote_meta → fetch_remote_rules_bytes → open DB → apply_update。
- ✅ 失败返 Envelope::err(INTERNAL)。
- ✅ lib.rs generate_handler 注册。

### D. 单测（rules_apply_update.rs）

- ✅ apply_update_replaces_files_and_upserts_db：文件替换 + .bak 存在 + DB active 新版本。
- ✅ apply_update_sha_mismatch_refuses：ShaMismatch + 本地文件没动 + .bak 没生成。
- ✅ apply_update_db_failure_rolls_back：DROP TABLE rules_meta → Db 错误 + 文件回滚。

### E. 卫生与边界

- ✅ 不修改 task-005 已有 loader。
- ✅ 不修改 task-022 已有 fetch_remote_meta。
- ✅ 不写回滚命令（从 .bak 恢复留后续）。
- ✅ 不引入新依赖。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 125 passed / 0 failed / 1 ignored（122 已有 + 3 apply_update 新增） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0 |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **rollback 失败吞掉**：`let _ = std::fs::copy(...)`。DevAgent 已记录（RollbackFailed 变体已定义但不抛，因为本地文件已不一致，抛了也救不回来）。**第三档。**
2. **.bak 文件不清理**：成功后 .bak 保留。合理（手动回滚用）。**第三档。**
3. **fetch_remote_rules_bytes 未写 CI 测试**：依赖网络。**第三档。**
4. **rules_meta.json 偏差**：写时补了 source_url / downloaded_at / custom_rules 字段（与 v1__init.sql 列对齐）。loader 反序列化有默认值，无兼容问题。DevAgent 已记录。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ 下载替换前必须 SHA-256 校验；不匹配立即拒绝。 |
| A03 注入 | ✅ SQL 参数化；reqwest GET 固定 URL，无用户输入拼路径。 |
| A04 不安全设计 | ✅ HTTPS GitHub Raw；备份后才写新文件；失败自动回滚。 |
| A08 数据完整性 | ✅ SHA-256 校验防篡改；.bak 备份可恢复；upsert DB 后才算成功。 |

### 专项安全分析

- **会不会下载恶意规则？** 不会。SHA-256 必须等于 remote_meta.checksum_sha256，不匹配立即拒绝。✅
- **写一半失败会不会损坏本地？** 不会。先备份 .bak，写失败自动从 .bak 恢复。✅
- **DB upsert 失败会不会文件已换？** 不会。DB 失败也回滚文件。✅
- **会不会被 MITM？** 不会。HTTPS GitHub Raw + rustls-tls。✅
- **.bak 会不会泄露敏感规则？** 不会。同目录，用户自己的规则。✅

## 评审结论：**通过**

rules.applyUpdate 闭环落地：fetch_remote_rules_bytes GET 远端 cache_rules.json；apply_update SHA-256 校验防篡改 → 备份 .bak → 写新文件 → upsert DB → 失败自动回滚；125 测试全绿（含 3 个新增 apply_update 测试：替换成功 / sha 不匹配拒绝 / DB 失败回滚）；clippy/fmt/pnpm build 全过；8 个提交全部关联 #task-023 无混杂。M1-E 规则更新闭环（checkUpdate + applyUpdate）。

## Git 操作记录

- **merge commit**：`934d96b`（`git merge --no-ff feature/task-023-rules-apply-update`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-023-rules-apply-update`，was 225829c）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务023评审报告 (#task-023)`

## 后续动作建议

1. **M1-F 报表**：audit_log 查询命令（按时间 / 类型 / 结果筛选）。
2. **前端 UI 接入**：scan.listItems 命令 + 勾选 UI + cleanup.execute 按钮。
3. **收尾**：打包时 data_dir 改 app_data_dir、失败重试 / 离线降级、GitHub Token / 私有仓。
