# 测试评审报告

## 本次任务名称

022 rules_meta 落 DB + rules.checkUpdate——查 GitHub Raw 比对版本（不下载）

## Git 提交记录核对结果

✅ 10 个提交全部关联 `#task-022`，type 前缀规范（docs / build / feat / test / style），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `9024ef8` | docs(task) | tasks/022-rules-check-update.md | ✅ |
| 2 | `53c4c92` | build(deps) | Cargo.lock + engine/Cargo.toml | ✅ |
| 3 | `08d4d23` | feat(rules) | meta_store.rs + mod.rs | ✅ |
| 4 | `fb849e7` | feat(rules) | remote.rs | ✅ |
| 5 | `7f0c1bf` | feat(tauri) | commands.rs + lib.rs + mod.rs + remote.rs | ✅ |
| 6 | `757f5c3` | test(rules) | rules_meta_store.rs | ✅ |
| 7 | `48287a8` | style(rules) | commands.rs + meta_store.rs（fmt） | ✅ |
| 8 | `d876cf5` | docs(state) | PROJECT_STATE.md | ✅ |
| 9 | `755da44` | docs(task) | tasks/022-rules-check-update.md | ✅ |
| 10 | `c4f1259` | docs(state) | PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. rules_meta 落 DB（meta_store.rs）

- ✅ RulesMetaRow { version, checksum_sha256, source_url, downloaded_at }。
- ✅ upsert_active_rules_meta：INSERT OR REPLACE INTO rules_meta (id=1, ...)。
- ✅ get_active_rules_meta：SELECT WHERE id=1，空表返 None。
- ✅ 单行表偏差：v1__init.sql 的 rules_meta 是 CHECK(id=1) 单行表，没有 is_active 列。DevAgent 已记录。

### B. 远端比对（remote.rs）

- ✅ REMOTE_RULES_META_URL 硬编码（GitHub Raw）。
- ✅ RemoteRulesMeta { version, checksum_sha256 }。
- ✅ CheckUpdateResp { has_update, current_version, remote_version, current_sha256, remote_sha256 }。
- ✅ fetch_remote_meta：reqwest::blocking::get → error_for_status → json。
- ✅ 网络错误 / 非 2xx / JSON 错 → Err(String)，不重试。

### C. Tauri rules_check_update 命令

- ✅ load_rules 拿本地 version / checksum_sha256。
- ✅ upsert_active_rules_meta 到 DB（幂等）。
- ✅ fetch_remote_meta 拉远端。
- ✅ has_update = local_sha != remote_sha。
- ✅ 失败返 Envelope::err(INTERNAL)。
- ✅ lib.rs generate_handler 注册。

### D. 单测（rules_meta_store.rs）

- ✅ upsert_then_get_active：upsert → get_active 返回同一行。
- ✅ upsert_new_version_replaces_old：v1 → v2 → get_active 返回 v2，DB 只有 1 行。
- ✅ get_active_empty_returns_none：空 DB → None。
- ✅ fetch_remote_meta 不写 CI 测试（依赖网络）。

### E. 卫生与边界

- ✅ 不下载 cache_rules.json（不调 applyUpdate）。
- ✅ 不替换本地 rules/ 目录文件。
- ✅ 不写前端 UI。
- ✅ 网络错误直接返 Err，不重试 / 离线降级。
- ✅ reqwest blocking + rustls-tls，default-features=false。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 122 passed / 0 failed / 1 ignored（119 已有 + 3 meta_store 新增） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0 |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **get_active_rules_meta 调用后未使用**：`let _ = get_active_rules_meta(&conn); // 留作后续扩展`。无害但冗余。**第三档。**
2. **fetch_remote_meta 未写 CI 测试**：依赖网络 + GitHub。DevAgent 已记录。**第三档。**
3. **单行表偏差**：v1__init.sql 的 rules_meta 是 CHECK(id=1) 单行表，没有 is_active 列。DevAgent 已记录。**第三档。**
4. **无重试 / 离线降级**：网络不通直接返 INTERNAL。任务书明确不做。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ 只读查询，不下载替换；rules_meta 是单行表，不可伪造。 |
| A03 注入 | ✅ SQL 参数化；reqwest GET 固定 URL，无用户输入拼路径。 |
| A04 不安全设计 | ✅ HTTPS GitHub Raw；只查版本不下载；失败不重试不降级。 |
| A08 数据完整性 | ✅ 比对 SHA-256，不自动信任远端；本地 rules_meta.json 已有 SHA-256 校验（task-005）。 |

### 专项安全分析

- **会不会下载恶意规则？** 不会。本任务只查版本（rules_meta.json），不下载 cache_rules.json。applyUpdate 留 task-023。✅
- **会不会被 MITM？** 不会。HTTPS GitHub Raw + rustls-tls。✅
- **本地规则会不会被替换？** 不会。upsert 只写 DB，不改 rules/ 目录文件。✅
- **网络不通会不会崩溃？** 不会。fetch_remote_meta 失败返 INTERNAL，不阻塞本地使用。✅
- **rules_meta 单行表会不会被并发写坏？** INSERT OR REPLACE WHERE id=1，单行表 SQLite 串行写。✅

## 评审结论：**通过**

rules_meta 落 DB + rules.checkUpdate 落地：meta_store upsert/get_active 单行表（id=1）；fetch_remote_meta GET GitHub Raw rules_meta.json；rules.checkUpdate 命令比对本地/远端 sha256 返 has_update（不下载替换）；122 测试全绿（含 3 个新增 meta_store 测试）；clippy/fmt/pnpm build 全过；10 个提交全部关联 #task-022 无混杂。规则版本检查闭环。

## Git 操作记录

- **merge commit**：`c81d228`（`git merge --no-ff feature/task-022-rules-check-update`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-022-rules-check-update`，was c4f1259）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务022评审报告 (#task-022)`

## 后续动作建议

1. **task-023**：rules.applyUpdate——下载新 cache_rules.json 并替换本地文件，SHA-256 校验通过后 upsert rules_meta 表。
2. **后续**：GitHub Token / 私有仓库、失败重试 / 离线降级、前端 UI 提示"有更新"。
3. **M1 收尾**：scan.listItems 命令、前端勾选 UI、打包时 data_dir 改 app_data_dir。
