# 测试评审报告

## 本次任务名称

018 safe_box.list——列出当前保险箱未剔除条目（前端 UI 用）

## Git 提交记录核对结果

✅ 7 个提交全部关联 `#task-018`，type 前缀规范（docs / feat / test），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `a8daaf6` | docs(task) | tasks/018-safe-box-list.md | ✅ |
| 2 | `abb43dc` | feat(execute) | safe_box.rs | ✅ |
| 3 | `5827efe` | feat(tauri) | commands.rs + lib.rs | ✅ |
| 4 | `774e1dd` | test(execute) | safe_box_list.rs | ✅ |
| 5 | `2faf5da` | docs(state) | PROJECT_STATE.md | ✅ |
| 6 | `4df3418` | docs(task) | tasks/018-safe-box-list.md | ✅ |
| 7 | `9e1a756` | docs(state) | PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. list_safe_box（safe_box.rs）

- ✅ `SELECT id, original_path, size_bytes, quarantined_at FROM safe_box WHERE status='in_box' ORDER BY quarantined_at DESC`。
- ✅ 过滤掉 purged 行（status='in_box'）。
- ✅ 累计 total_items = entries.len()、total_bytes = sum(size_bytes)。
- ✅ DB 错误 → Err(String)。
- ✅ SafeBoxEntry { id, original_path, size_bytes: u64, quarantined_at }。
- ✅ SafeBoxListResp { entries, total_items, total_bytes: u64 }。

### B. Tauri safe_box_list 命令

- ✅ 无入参。
- ✅ open DB → list_safe_box → Envelope::ok。
- ✅ DB 错误 → Envelope::err(INTERNAL)（eprintln 兜底）。
- ✅ lib.rs generate_handler 注册。

### C. 单测（safe_box_list.rs）

- ✅ `list_empty_returns_zero`：空 DB → entries=[]、total_items=0、total_bytes=0。
- ✅ `list_returns_inserted_entries`：移入 3 个文件（100+200+300=600 字节）→ 3 条、total_bytes=600。
- ✅ `list_excludes_purged`：标 purged 一行 → 只剩 1 条 200 字节。
- ✅ `list_excludes_restored`：restore 一条 → 该条目消失，剩 1 条 b.tmp。

### D. 卫生与边界

- ✅ 未修改 SafeBoxDeleter / enforce_quota / restore 已有逻辑（只追加 list 函数）。
- ✅ 未修改 schema。
- ✅ 未修改 app/src/**（前端 React）。
- ✅ 未新增依赖。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 108 passed / 0 failed（104 已有 + 4 list 新增） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0 |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **无分页**：保险箱总量 ≤5GB，条目数有限。DevAgent 已记录。**第三档。**
2. **无 session 过滤**：先返回全部。**第三档。**
3. **size_bytes i64→u64 转换**：真实列 INTEGER 可正可负但业务上非负，`v as u64` 直接转（若有负会变巨大正数）。M1 dev 可接受。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ 只读操作，无入参，不接受前端传 id/path。 |
| A03 注入 | ✅ SQL 无参数（全静态语句）。 |
| A04 不安全设计 | ✅ 只返回 status='in_box' 条目；不暴露 safe_rel_path（内部路径），只暴露 original_path（用户原始路径）。 |
| A08 数据完整性 | ✅ list 是只读，不改 DB。 |

### 专项安全分析

- **会不会泄露内部路径？** 不会。SafeBoxEntry 只暴露 id / original_path（用户原始路径）/ size_bytes / quarantined_at；不暴露 safe_rel_path（box_dir 内部相对路径）。✅
- **purged 条目会出现在列表里吗？** 不会。`WHERE status='in_box'` 过滤。✅
- **restored 条目会出现在列表里吗？** 不会。restore 已 DELETE safe_box 行。✅
- **DB 错误会泄露堆栈吗？** 不会。返回 Envelope::err(INTERNAL)，详情 eprintln。✅

## 评审结论：**通过**

list_safe_box 落地：SELECT status='in_box' 条目按 quarantined_at DESC，返回 entries + total_items + total_bytes 汇总；Tauri safe_box_list 命令注册；108 测试全绿（含 4 个新增：空箱/多条汇总/排除 purged/排除 restored）；clippy/fmt/pnpm build 全过；7 个提交全部关联 #task-018 无混杂。保险箱三件套（入/回/列）齐备。

## Git 操作记录

- **merge commit**：`196889c`（`git merge --no-ff feature/task-018-safe-box-list`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-018-safe-box-list`，was 9e1a756）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务018评审报告 (#task-018)`

## 后续动作建议

1. **M1-D**：Restart Manager 处理文件占用删除失败（SharingViolation 重试）。
2. **后续**：分页、按 session 过滤、单条 purge 命令、前端 React 列表 UI。
3. **M1 收尾**：scan.listItems 命令、前端勾选 UI、打包时 data_dir 改 app_data_dir。
