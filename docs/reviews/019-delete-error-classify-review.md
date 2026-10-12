# 测试评审报告

## 本次任务名称

019 删除失败错误分类——识别 SharingViolation（文件占用），audit_log 写具体原因

## Git 提交记录核对结果

✅ 8 个提交全部关联 `#task-019`，type 前缀规范（docs / feat / test），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `56f19b9` | docs(task) | tasks/019-delete-error-classify.md | ✅ |
| 2 | `9fc9564` | feat(execute) | error_kind.rs + mod.rs | ✅ |
| 3 | `a2c75a5` | feat(execute) | safe_box.rs | ✅ |
| 4 | `257ba68` | feat(execute) | executor.rs | ✅ |
| 5 | `ee14fd0` | test(execute) | delete_error_kind.rs | ✅ |
| 6 | `e050508` | docs(state) | PROJECT_STATE.md | ✅ |
| 7 | `adbb74d` | docs(task) | tasks/019-delete-error-classify.md | ✅ |
| 8 | `f66e46b` | docs(state) | PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. classify_io_error（error_kind.rs）

- ✅ DeleteFailureKind 枚举：NotFound / PermissionDenied / SharingViolation / Unknown。
- ✅ `err.kind() == NotFound` → NotFound（跨平台稳定）。
- ✅ `err.kind() == PermissionDenied` → PermissionDenied（跨平台稳定）。
- ✅ `raw_os_error() == Some(2)` → NotFound。
- ✅ `raw_os_error() == Some(5)` → PermissionDenied。
- ✅ `raw_os_error() == Some(32)` → SharingViolation（ERROR_SHARING_VIOLATION）。
- ✅ 其他 → Unknown。
- ✅ 不引 windows crate，只做数字比较。

### B. SafeBoxDeleter 错误前缀（safe_box.rs）

- ✅ rename 失败 → classify_io_error → SharingViolation → `Err("FILE_IN_USE: {src}")`。
- ✅ PermissionDenied → `Err("PERMISSION_DENIED: {src}")`。
- ✅ 其他 → 原始错误字符串。
- ✅ copy+remove fallback 的 remove 阶段也分类。

### C. audit_log detail 翻译（executor.rs）

- ✅ error 以 `FILE_IN_USE:` 开头 → detail = "file in use by another process"。
- ✅ error 以 `PERMISSION_DENIED:` 开头 → detail = "permission denied, needs elevation"。
- ✅ 其他 → detail = 原始 error。

### D. 单测

- ✅ 内部 4 个（error_kind.rs mod tests）：NotFound / SharingViolation(32) / PermissionDenied(5) / Unknown。
- ✅ 外部 4 个（delete_error_kind.rs）：同上覆盖。

### E. 卫生与边界

- ✅ 未引入 windows crate。
- ✅ 未修改 SafeBoxDeleter / FsDeleter 已有成功路径。
- ✅ 未修改 Tauri 命令层。
- ✅ 未新增依赖。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 116 passed / 0 failed（108 已有 + 8 classify 新增：内部4 + 外部4） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `pnpm --filter app build` | ✅ exit 0 |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **notepad 端到端未跑**：DevAgent 已记录（自动化环境无 GUI）。分类逻辑由 8 个单元测试覆盖。**第三档。**
2. **FsDeleter 未加分类**：任务书只要求 SafeBoxDeleter。FsDeleter 是 task-015 的永久删除原语，当前 cleanup_execute 已切到 SafeBoxDeleter，FsDeleter 实际未被调用。**第三档。**
3. **完整 Restart Manager FFI 未做**：查占用 PID、请求关闭进程留 task-020。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ 错误分类是只读诊断，不改删除逻辑。 |
| A03 注入 | ✅ 无 SQL / shell 变更。 |
| A04 不安全设计 | ✅ 不吞错误——明确分类给前端信号；NotFound 幂等不视为失败。 |
| A08 数据完整性 | ✅ audit_log 写稳定文案（"file in use by another process"），前端可直接 toast。 |

### 专项安全分析

- **会不会误吞 SharingViolation？** 不会。raw_os_error 32 明确分类为 SharingViolation，返回 FILE_IN_USE 前缀。✅
- **NotFound 会不会误报失败？** 不会。metadata NotFound → Ok(()) 幂等；rename NotFound 也走分类 NotFound。✅
- **raw_os_error 数字会不会跨平台错位？** 不会。先看 ErrorKind（跨平台稳定），再看 raw_os_error（Windows 专用）；非 Windows 退到 ErrorKind。✅
- **audit_log 会不会泄露内部路径？** detail 是稳定文案，不包含 path；path 列单独存。✅

## 评审结论：**通过**

classify_io_error 落地：errno 2/5/32 粗分 NotFound/PermissionDenied/SharingViolation；SafeBoxDeleter rename/copy+remove 失败加 FILE_IN_USE/PERMISSION_DENIED 前缀；executor audit_log 翻译稳定文案；116 测试全绿（含 8 个新增 classify 测试）；clippy/fmt/pnpm build 全过；8 个提交全部关联 #task-019 无混杂。文件占用错误首次有明确信号。

## Git 操作记录

- **merge commit**：`3928778`（`git merge --no-ff feature/task-019-delete-error-classify`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-019-delete-error-classify`，was f66e46b）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务019评审报告 (#task-019)`

## 后续动作建议

1. **task-020**：Windows Restart Manager FFI——查询占用进程 PID 列表，供前端提示。
2. **后续**：提权重试（elevation crate）、前端 UI toast 提示、单条 purge 命令。
3. **M1 收尾**：scan.listItems 命令、前端勾选 UI、打包时 data_dir 改 app_data_dir。
