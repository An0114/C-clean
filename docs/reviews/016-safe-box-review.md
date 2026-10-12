# 测试评审报告

## 本次任务名称

016 safe_box 保险箱——删除前 rename 移入 5GB FIFO 目录，超容按时间最旧永久剔除

## Git 提交记录核对结果

✅ 10 个提交全部关联 `#task-016`，type 前缀规范（docs / feat / refactor / test / fix），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `56868e4` | docs(task) | tasks/016-safe-box.md | ✅ |
| 2 | `58dd794` | feat(execute) | mod.rs + safe_box.rs | ✅ |
| 3 | `6a3892e` | refactor(execute) | executor.rs + execute_items.rs（适配新签名） | ✅ |
| 4 | `86e5fdf` | feat(tauri) | commands.rs | ✅ |
| 5 | `cdb89f6` | test(execute) | safe_box.rs | ✅ |
| 6 | `82e320c` | fix(execute) | safe_box.rs（raw_os_error 17/18） | ✅ |
| 7 | `ea510c8` | docs(state) | PROJECT_STATE.md | ✅ |
| 8 | `484bdf2` | docs(task) | tasks/016-safe-box.md | ✅ |
| 9 | `b613891` | test(safe_box) | executor.rs + safe_box.rs（全局串行锁） | ✅ |
| 10 | `f8b12c5` | docs(state) | PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. SafeBoxDeleter（safe_box.rs）

- ✅ 实现 `Deleter` trait。
- ✅ `delete_file`：metadata → create_dir_all → enforce_quota → rename（跨盘 fallback copy+remove）→ INSERT safe_box 行。
- ✅ 文件名格式：`<unix_ts>_<original_filename>`（避免同名冲突）。
- ✅ NotFound → Ok(()) 幂等。
- ✅ `delete_dir` → Err（只删文件）。
- ✅ 跨盘 rename：`raw_os_error()` 17 (Windows) / 18 (Linux) → fallback `fs::copy` + `fs::remove_file`。

### B. enforce_quota()

- ✅ `SELECT COALESCE(SUM(size_bytes),0) FROM safe_box WHERE status='in_box'`。
- ✅ usage + incoming <= quota → 直接返回。
- ✅ 超容 → `SELECT ... ORDER BY quarantined_at ASC` 逐条 remove_file + UPDATE status='purged'。
- ✅ remove_file NotFound → 标 purged 不阻塞。
- ✅ QUOTA_BYTES 用 AtomicU64，测试通过 `set_quota_for_test()` 覆盖。

### C. execute_items 签名改造

- ✅ 从单 `deleter: &dyn Deleter` 改成 `deleter_factory: FnMut(i64) -> Box<dyn Deleter + 'a>`。
- ✅ 每个 item_id 调 factory 构造新 SafeBoxDeleter（持有 item 上下文）。

### D. Tauri cleanup_execute 切换

- ✅ box_dir = data_root.join("safe_box") + create_dir_all。
- ✅ 闭包构造 SafeBoxDeleter::new(&box_dir, &conn, None)。
- ✅ 所有删除操作都走保险箱（不再直接 remove_file）。

### E. 单测（safe_box.rs）

- ✅ `safe_box_moves_file_and_writes_row`：原文件没了、box_dir 有新文件、safe_box 表 1 行。
- ✅ `safe_box_enforce_quota_noop_when_under_quota`：空箱 + 100 字节不触发剔除。
- ✅ `safe_box_quota_evicts_oldest`：3 个 100 字节旧文件 + 第 4 个进来 → 最旧 old_0.bin 被 purged，in_box=3。
- ✅ 全局 TEST_LOCK 串行化（QUOTA_BYTES 是进程级 AtomicU64）。

### F. 卫生与边界

- ✅ 未修改 fs_deleter.rs（保留 FsDeleter 作为永久删除原语）。
- ✅ 未修改 delete.rs 已有签名。
- ✅ 未修改 db/** schema。
- ✅ 未实现 restore 命令、UI、TTL。
- ✅ 未新增依赖。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 102 passed / 0 failed（99 已有 + 3 safe_box 新增） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0 |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **原子性偏差**：rename 成功后 INSERT safe_box 行失败 → 文件已移入保险箱但 DB 没记录。DevAgent 已记录为遗留。**第三档。**
2. **evict 时 remove_file 成功但 UPDATE status 失败**：文件没了但 DB 还显示 in_box。**第三档。**
3. **expires_at 硬编码 7 天**：但 TTL 自动清空留后续。无害。**第三档。**
4. **跨盘 copy+remove 无重试**：copy 中途失败会留半份文件。M1 dev 可接受。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ execute_items 仍过二次白名单 + resolve_to_real（task-015 不变）；SafeBoxDeleter 只是替换删除实现，不绕开白名单。 |
| A03 注入 | ✅ SQL 全部参数化；path 来自 DB 反查。 |
| A04 不安全设计 | ✅ 删除不再是不可逆 remove_file——先 rename 到保险箱，用户后悔可恢复；超容按 FIFO 最旧剔除。 |
| A08 数据完整性 | ✅ 每次移入写 safe_box 行（original_path/safe_rel_path/size_bytes/quarantined_at/status）；evict 时 UPDATE status='purged'。 |

### 专项安全分析

- **删错了能恢复吗？** 能。文件先 rename 到 `<data_dir>/safe_box/`，restore 命令留 task-017。✅
- **保险箱会不会无限膨胀？** 不会。enforce_quota 超 5GB 按 quarantined_at 最旧永久剔除。✅
- **跨盘 rename 失败怎么办？** fallback copy+remove（Windows=17 / Linux=18）。✅
- **同名文件冲突？** 文件名加 `<unix_ts>_` 前缀。✅
- **L3 路径会不会被移入保险箱？** 不会。白名单在 execute_items 阶段（task-015）就拦了，SafeBoxDeleter 只处理已通过白名单的 path。✅
- **evict 会不会误删用户还想恢复的文件？** 不会。evict 只删 status='in_box' 且 quarantined_at 最旧的，且超容时才触发。FIFO 语义明确。✅

## 评审结论：**通过**

SafeBoxDeleter 落地：删除从直接 `remove_file` 改为先 `rename` 到 5GB FIFO 保险箱；超容按 `quarantined_at ASC` 永久剔除最旧；跨盘 rename fallback copy+remove；execute_items 签名改为 deleter 工厂闭包以支持 per-item deleter；102 测试全绿（含 3 个新增：移入/quota 剔除/空 noop）；clippy/fmt/pnpm build 全过；10 个提交全部关联 #task-016 无混杂。

## Git 操作记录

- **merge commit**：`d0d450e`（`git merge --no-ff feature/task-016-safe-box`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-016-safe-box`，was f8b12c5）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务016评审报告 (#task-016)`

## 后续动作建议

1. **task-017**：restore 命令——从保险箱按 id 恢复文件到原路径。
2. **后续**：TTL 自动清空（expires_at 已写 7 天）、原子性（rename+INSERT 事务）、跨盘 copy 重试、目录删除。
3. **M1 收尾**：scan.listItems 命令、前端勾选 UI、打包时 data_dir 改 app_data_dir。
