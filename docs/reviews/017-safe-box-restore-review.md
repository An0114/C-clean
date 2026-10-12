# 测试评审报告

## 本次任务名称

017 safe_box restore——从保险箱按 id 恢复文件到原路径，禁止覆盖目标占用

## Git 提交记录核对结果

✅ 7 个提交全部关联 `#task-017`，type 前缀规范（docs / feat / test），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `4714798` | docs(task) | tasks/017-safe-box-restore.md | ✅ |
| 2 | `9fb97ea` | feat(execute) | safe_box.rs | ✅ |
| 3 | `26d19d1` | feat(tauri) | commands.rs + lib.rs | ✅ |
| 4 | `6372f6c` | test(execute) | safe_box_restore.rs | ✅ |
| 5 | `1c65d5e` | docs(state) | PROJECT_STATE.md | ✅ |
| 6 | `acc8503` | docs(task) | tasks/017-safe-box-restore.md | ✅ |
| 7 | `2053525` | docs(state) | PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. restore_from_safe_box（safe_box.rs）

- ✅ `SELECT original_path, safe_rel_path, size_bytes FROM safe_box WHERE id=? AND status='in_box'`。
- ✅ 无行 → `NotFound`。
- ✅ `orig.exists()` → `TargetOccupied`（**禁止静默覆盖**）。
- ✅ `parent().exists()` → `ParentMissing`（不自动 mkdir）。
- ✅ `rename(box_abs, orig)` 把文件移回原路径。
- ✅ `DELETE FROM safe_box WHERE id=?`。
- ✅ `INSERT INTO audit_log (ts, actor, action, target, detail, result)` action='restore'。
- ✅ 返回 RestoreResult { safe_box_id, original_path, size_bytes }。

### B. Tauri safe_box_restore 命令

- ✅ 入参 safe_box_id: i64。
- ✅ open DB + box_dir = data_root.join("safe_box")。
- ✅ 调 restore_from_safe_box。
- ✅ 成功 → Envelope::ok。
- ✅ TargetOccupied → 2003 FILE_OCCUPIED。
- ✅ NotFound / ParentMissing / Io / Db → 5001 INTERNAL（eprintln 兜底）。
- ✅ lib.rs generate_handler 注册。

### C. 单测（safe_box_restore.rs）

- ✅ `restore_returns_file_to_original_path`：原文件回来、box 里没了、safe_box 表 0 行、audit_log 1 行 restore。
- ✅ `restore_target_occupied_returns_error`：原路径写新文件 → TargetOccupied，box 文件没动、DB 行还在。
- ✅ `restore_parent_missing_returns_error`：original_path 指向不存在子目录 → ParentMissing。
- ✅ `restore_purged_row_returns_not_found`：status='purged' 行 → NotFound。
- ✅ `set_quota_still_works`：防止未使用警告。

### D. 卫生与边界

- ✅ 未修改 SafeBoxDeleter / enforce_quota 已有逻辑（只追加 restore 函数）。
- ✅ 未修改 schema（不加 restored_at 列）。
- ✅ 未修改 db/**、scan/**、ipc/**。
- ✅ 未修改 app/src/**（前端 React）。
- ✅ 未新增依赖。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 104 passed / 0 failed（99 已有 + 5 restore 新增） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0 |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **原子性偏差**：rename 成功后 DELETE safe_box 行失败 → 文件已回原路径但 DB 还有行（用户可能重复 restore）。与 task-016 同类。**第三档。**
2. **TOCTOU**：`orig.exists()` 检查后、rename 前原路径可能被新文件占用。但 Rust std::fs::rename 在 Windows 上目标存在时会失败，不会静默覆盖。**第三档。**
3. **restore 不自动建父目录**：原目录被删时报错让用户手动处理。DevAgent 已记录。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ restore 按 id 查 DB，id 由后端反查；不接受前端传 path。 |
| A03 注入 | ✅ SQL 全部参数化；original_path 来自 DB 反查。 |
| A04 不安全设计 | ✅ **禁止覆盖**：orig.exists() → TargetOccupied，不静默覆盖用户新文件；父目录不存在报错不自动 mkdir。 |
| A08 数据完整性 | ✅ restore 写 audit_log（action='restore', target=original_path, result='success'）；DELETE safe_box 行。 |

### 专项安全分析

- **restore 会不会覆盖用户新文件？** 不会。`orig.exists()` 检查 → TargetOccupied，box 文件没动、DB 行还在。✅
- **已剔除的文件能恢复吗？** 不能。`status='in_box'` 条件筛掉 purged 行 → NotFound。✅
- **原目录被删了怎么办？** 报错 ParentMissing，让用户手动建目录。不自动 mkdir（防路径注入）。✅
- **restore 会不会绕过白名单？** restore 是把文件移回用户指定的原路径（删除前用户确认过的路径），不是新删除操作，不需要白名单。✅
- **box 文件不在磁盘上怎么办？** rename 会报错 Io，返回 5001。box_dir 文件被手动删了但 DB 还有行——后续 list 命令可清理。✅

## 评审结论：**通过**

restore_from_safe_box 落地：按 id 从 safe_box 表查行（status='in_box'）→ 检查原路径不被占用 → 检查父目录存在 → rename 回原路径 → DELETE 行 → 写 audit_log；Tauri safe_box_restore 命令注册；104 测试全绿（含 5 个新增：正常恢复/目标占用/父目录缺失/已剔除/quota 兼容）；clippy/fmt/pnpm build 全过；7 个提交全部关联 #task-017 无混杂。保险箱闭环完成（删除→移入→后悔→恢复）。

## Git 操作记录

- **merge commit**：`ac30fd9`（`git merge --no-ff feature/task-017-safe-box-restore`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-017-safe-box-restore`，was 2053525）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务017评审报告 (#task-017)`

## 后续动作建议

1. **task-018**：safe_box.list 命令——列出当前保险箱里所有未剔除条目（前端 UI 用）。
2. **后续**：批量恢复、原父目录自动 mkdir、restored_at 审计列、TTL 自动清空。
3. **M1 收尾**：scan.listItems 命令、前端勾选 UI、打包时 data_dir 改 app_data_dir。
