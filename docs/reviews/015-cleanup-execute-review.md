# 测试评审报告

## 本次任务名称

015 cleanup.execute 真实删除——FsDeleter + delete::execute 二次白名单 + resolve_to_real junction 防护 + audit_log 落库

## Git 提交记录核对结果

✅ 9 个提交全部关联 `#task-015`，type 前缀规范（docs / feat / test / fix），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `57ea8ca` | docs(task) | tasks/015-cleanup-execute.md | ✅ |
| 2 | `aa5abd7` | feat(execute) | fs_deleter.rs + mod.rs | ✅ |
| 3 | `319d55e` | feat(execute) | executor.rs + mod.rs | ✅ |
| 4 | `125c385` | feat(tauri) | commands.rs + lib.rs | ✅ |
| 5 | `897b0c1` | test(execute) | execute_items.rs | ✅ |
| 6 | `8aec3a0` | docs(state) | PROJECT_STATE.md | ✅ |
| 7 | `d366b4b` | docs(task) | tasks/015-cleanup-execute.md | ✅ |
| 8 | `b80cd86` | fix(execute) | executor.rs（补 resolve_to_real junction 防护） | ✅ |
| 9 | `e813c89` | docs(state) | PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. FsDeleter（fs_deleter.rs）

- ✅ 实现 `Deleter` trait。
- ✅ `delete_file`：metadata → 检查目录 → remove_file。
- ✅ NotFound → Ok(()) 幂等（已删过就算成功）。
- ✅ PermissionDenied / SharingViolation / 其他 IO → Err。
- ✅ 目录 → Err（task-015 只删文件）。
- ✅ `delete_dir` → Err（明确未实现）。

### B. execute_items 编排（executor.rs）

- ✅ 对每个 item_id：SELECT path FROM cleanup_item WHERE id = ?（参数化）。
- ✅ 查不到 → skip，不写 audit_log。
- ✅ `normalize(&path)` 规范化。
- ✅ **`resolve_to_real(&np)`**——task-008 junction/symlink 防护（b80cd86 补的关键修复）。
- ✅ `classify_with_external(&classify_target, external_deny)`——二次白名单校验。
- ✅ L3 命中 → audit_log result='denied'，不删文件。
- ✅ `deleter.delete_file(&np)` 真实删除。
- ✅ 成功 → audit_log result='success'。
- ✅ 失败 → audit_log result='fail'。
- ✅ 最后写一行 cleanup_task 聚合（success/fail/skip/freed_bytes）。

### C. Tauri cleanup_execute 命令

- ✅ 入参 item_ids 非空校验（空 → DRYRUN_EMPTY）。
- ✅ load_external_deny() fail-closed。
- ✅ open(&data_root) + DbItemPathResolver 同一 data_dir。
- ✅ 调 execute_items(&item_ids, &conn, &external_deny, &FsDeleter)。
- ✅ 返回 Envelope<ExecuteSummary>。
- ✅ lib.rs generate_handler 注册。

### D. 单测（execute_items.rs）

- ✅ `execute_items_deletes_real_files`：3 个 tempdir 文件真删，cleanup_task=1, audit_log success=3。
- ✅ `execute_items_blocked_by_l3`：Desktop 路径 → deleted=0, denied audit=1。
- ✅ `execute_items_skips_missing_id`：不存在 id → skip，不影响其他。
- ✅ `execute_items_idempotent_on_already_deleted`：二次删 → Ok(0)，两次 success audit。

### E. 卫生与边界

- ✅ 未修改 delete.rs 已有签名。
- ✅ 未修改 db/**、ipc/**、scan/**、rules/**。
- ✅ 未修改 app/src/**（前端 React）。
- ✅ 未新增依赖（std::fs 够用）。
- ✅ 不删目录、不做保险箱、不做 Restart Manager。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 99 passed / 0 failed（95 已有 + 4 execute_items 新增） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0 |

## 代码评审发现问题

### Bug

无。b80cd86 已补 junction 防护（resolve_to_real）。

### 技术债务（轻微）

1. **TOCTOU**：normalize + classify 通过后、delete_file 前，文件可能被替换/junction 被换。M1 可接受（攻击者需在毫秒级替换且已有本机写权限）。**第三档。**
2. **重复 metadata 调用**：executor.rs 第 162 行调一次 `std::fs::metadata` 算 bytes_before，FsDeleter.delete_file 内部又调一次。无害但冗余。**第三档。**
3. **cleanup_task.session_id = NULL**：executor 不知道当前 session_id（item_ids 跨 session）。DevAgent 已记录 schema 偏差。**第三档。**
4. **文件占用时删除失败**：SharingViolation 直接 Err，不重试。留 Restart Manager。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ execute 时**再过一次**白名单（normalize + resolve_to_real + classify_with_external），不依赖 dryRun 结果；L3 路径即使 dryRun 通过也会在 execute 时被拦。 |
| A03 注入 | ✅ SQL 全部参数化（rusqlite::params!）；path 来自 DB 反查，前端只传 item_ids（i64）。 |
| A04 不安全设计 | ✅ NotFound 幂等；目录删除拒绝；单 item 失败不中断；失败也写 audit_log。 |
| A08 数据完整性 | ✅ 每次删除都写 audit_log（ts/actor/action/target/detail/result/denial_reason）；cleanup_task 聚合行记录 success/fail/skip。 |

### 专项安全分析（真实删除任务重点）

- **前端能不能指定路径？** 不能。前端只传 itemIds（i64），path 由 DB 反查。✅
- **junction 能不能绕过 L3？** 不能。`resolve_to_real` 解析 canonicalize 后的真实路径，再对真实路径做 L3 判定。即使 Temp 里有个 junction 指向 Desktop，真实路径是 Desktop，会被 L3PersonalDir 拦。✅（b80cd86 补的关键修复）
- **dryRun 通过后路径被改怎么办？** execute 时重新 normalize + resolve_to_real + classify，不缓存 dryRun 结果。✅
- **能不能删目录？** 不能。FsDeleter.delete_dir 返回 Err；delete_file 检查 `meta.is_dir()` 返回 Err。✅
- **重复删除会不会报错？** 不会。NotFound → Ok(()) 幂等。✅
- **删除失败会不会静默？** 不会。失败写 audit_log result='fail'，返回 ExecResult.error。✅
- **L3 路径会不会被删？** 不会。Desktop/Documents/Downloads/Pictures/System32 等在 classify_with_external 阶段被拦。测试 `execute_items_blocked_by_l3` 验证。✅
- **rules 加载失败？** 仍 fail-closed 返 5001，不执行删除。✅

## 评审结论：**通过**

真实删除闭环落地：FsDeleter（std::fs::remove_file）+ execute_items 编排（二次白名单 + resolve_to_real junction 防护 + audit_log 落库）+ cleanup.execute Tauri 命令；99 测试全绿（含 4 个新增：真删/L3拦/缺id/幂等）；clippy/fmt/pnpm build 全过；9 个提交全部关联 #task-015 无混杂；b80cd86 补的 junction 防护是关键安全修复。

## Git 操作记录

- **merge commit**：`7c359de`（`git merge --no-ff feature/task-015-cleanup-execute`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-015-cleanup-execute`，was e813c89）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务015评审报告 (#task-015)`

## 后续动作建议

1. **task-016**：safe_box 保险箱——删除前先移到 5GB FIFO 目录，覆盖则按时间最旧剔除。
2. **后续**：Restart Manager（文件占用重试）、目录删除、批量进度事件、elevation 提权。
3. **安全关注**：TOCTOU 在 M2 可考虑（删除前重新 stat + 对 inode/hash 校验）；cleanup_task.session_id 关联到当前 session。
