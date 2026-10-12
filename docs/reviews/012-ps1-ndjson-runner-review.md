# 测试评审报告

## 本次任务名称

012 Rust 侧 ps1 NDJSON 消费器——spawn 子进程、逐行解析、写 cleanup_item 表、emit 进度

## Git 提交记录核对结果

✅ 7 个提交全部关联 `#task-012`，type 前缀规范（docs / feat / test），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `9f283c2` | docs(task) | tasks/012-ps1-ndjson-runner.md | ✅ |
| 2 | `0e5e35e` | feat(scan) | lib.rs + scan/mod.rs + scan/parser.rs | ✅ |
| 3 | `3096cc7` | feat(scan) | scan/mod.rs + scan/runner.rs | ✅ |
| 4 | `4222d83` | test(scan) | scan/runner.rs（微调）+ scan_runner.rs | ✅ |
| 5 | `0721b99` | docs(state) | docs/PROJECT_STATE.md | ✅ |
| 6 | `8565024` | docs(task) | tasks/012-ps1-ndjson-runner.md | ✅ |
| 7 | `33c4bcb` | docs(state) | docs/PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. 模块结构

- ✅ 新建 `engine/src/scan/mod.rs`、`parser.rs`、`runner.rs`。
- ✅ `engine/src/lib.rs` 加 `pub mod scan;`。

### B. Parser（parser.rs）

- ✅ `NdjsonEvent` serde 枚举：`#[serde(tag = "type", deny_unknown_fields)]`。
- ✅ Item 变体：module_code / category / path / size_bytes / risk_level。
- ✅ Done 变体：module_code / total_items / total_bytes。
- ✅ `ParseError`：Json(serde_json::Error)。
- ✅ `parse_line(line: &str)`：trim 后空行报错，否则 `serde_json::from_str`。
- ✅ 5 个单测：item_ok / done_ok / missing_field / unknown_field / bad_json。

### C. Runner（runner.rs）

- ✅ `ScanRunner { session_id, conn, emitter, scanned_count: AtomicI64 }`。
- ✅ `run_script` spawn `powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File <script>`。
- ✅ stdout/stderr 均 piped。
- ✅ BufReader::lines() 逐行读 stdout。
- ✅ Item → 参数化 INSERT cleanup_item（rusqlite::params!）。
- ✅ INSERT 成功后 emit ScanProgress（percent=0, current_path=path, scanned_items=n）。
- ✅ Done → done_seen=true, total_bytes=tb, break。
- ✅ 解析失败 → eprintln 跳过，不 panic。
- ✅ DB INSERT 失败 → RunError::Db 致命 return。
- ✅ stderr 整段 read_to_string 后 eprintln。
- ✅ 非 0 退出但有 done → 成功；无 done 且非 0 → RunError::ScriptFailed。
- ✅ risk_level int → TEXT 映射（0→L0, 1→L1, 2→L2, other→L2 clamp）。
- ✅ description NOT NULL 给空串兜底。

### D. 单测（scan_runner.rs）

- ✅ `runner_inserts_items_and_emits`：2 item + done，DB 行数=2，emit=2。
- ✅ `runner_skips_bad_lines`：坏 JSON 跳过，继续处理后面合法行。
- ✅ `runner_script_failure_without_done_returns_err`：exit 1 无 done → ScriptFailed，item 已入库。
- ✅ 3 个 runner 测试均 `#[cfg(windows)]`。

### E. 卫生与边界

- ✅ 未修改 engine/src/db/**、ipc/**、execute/** 已有逻辑。
- ✅ 未修改 app/**、elevation/**、rules/**、scripts/**。
- ✅ 未引入新 crate。
- ✅ 生产代码无 println!（仅 eprintln!）。
- ✅ 未实现 scan.cancel、未实现 5 脚本编排。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `cargo test -p engine` | ✅ 91 passed / 0 failed（18 lib + 4 db_resolver + 8 db_schema + 7 execute_resolve + 24 execute_whitelist + 7 ipc_commands + 3 ipc_dry_run_external + 3 envelope + 10 rules_loader + 3 scan_runner + 1 smoke） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| task-002~011 已有测试 | ✅ 无回退 |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **stderr pipe 死锁风险**：stderr 是 piped，但循环里只在读 stdout。如果子进程写大量 stderr（>64KB pipe buffer），会阻塞子进程写 stderr，而父进程在等 stdout EOF——经典 pipe deadlock。实际 ps1 stderr 只写少量日志（权限不足目录名），不会撑满。**第三档，M1-A 编排层如遇大量 stderr 需改用独立线程读 stderr。**
2. **done 后 break 但未 kill 子进程**：break 后直接 child.wait()，如果 ps1 在 done 后还有 flush 操作会等它结束。合理行为。**第三档。**
3. **percent 硬编码 0**：任务书允许（编排层跑完再换算）。**第三档。**
4. **schema 偏差（已记录）**：任务书 §C 写 `discovered_at`，实际 v1__init.sql 无该列；`risk_level` DB 端 TEXT，ps1 给 int，runner 内映射；`description` NOT NULL 给空串兜底。DevAgent 已在实现备注声明。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ item 入 cleanup_item 表后，M1-A 删除前仍需过 task-003 白名单 + task-008 路径解析；runner 只入库不删除。 |
| A03 注入 | ✅ INSERT 用 rusqlite::params! 参数化，无 SQL 注入。 |
| A04 不安全设计 | ✅ DB INSERT 失败视为致命 return，不灌半份数据；行级 JSON 解析失败才跳过。 |
| A08 数据完整性 | ✅ deny_unknown_fields 防止 ps1 多塞字段静默丢弃；done 事件 total_items/total_bytes 可对账。 |

### 专项安全分析

- **SQL 注入？** 不会。`rusqlite::params!` 参数化绑定，path/category 等用户可控字段都走绑定。✅
- **ps1 脚本路径可控？** script_path 来自 M1-A 编排层（硬编码 scripts/scan_*.ps1），不是用户输入。✅
- **stderr 会不会污染 stdout？** 不会。stdout/stderr 分别 piped，stderr 整段读后 eprintln，不入库。✅
- **坏 JSON 行会不会导致 panic？** 不会。parse_line 错误被 catch，eprintln 跳过，继续读下一行。测试 `runner_skips_bad_lines` 验证。✅
- **未知 risk_level 会不会写脏数据？** 不会。clamp 到 L2（保守），并 eprintln 警告。✅
- **子进程卡死会不会拖死主进程？** 当前无超时/cancel（任务书明确留后续）。M1-A 需加 timeout。**第三档。**

## 评审结论：**通过**

NdjsonEvent serde 模型 + parse_line 单行解析器 + ScanRunner 子进程消费器全部落地；8 条新测试（5 parser + 3 runner）全绿；91 条总测试无回退；clippy/fmt 全过；SQL 参数化、DB 失败致命、坏行跳过、deny_unknown_fields 等安全设计到位；7 个提交全部关联 #task-012 无混杂。

## Git 操作记录

- **merge commit**：`f9c452a`（`git merge --no-ff feature/task-012-ps1-ndjson-runner`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-012-ps1-ndjson-runner`，was 33c4bcb）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务012评审报告 (#task-012)`

## 后续动作建议

1. **task-013**：把 ScanRunner 接到 Tauri `scan.start` 命令——spawn 5 个 ps1 串跑、写真实 session、前端 listen `scan://progress` 端到端。
2. **M1-A 编排层注意**：
   - stderr pipe 死锁风险——大量 stderr 时需独立线程读 stderr；
   - 子进程超时/cancel——留 scan.cancel IPC；
   - percent 换算——5 脚本跑完后按总字节或总条数算百分比。
3. **M1-B 删除**：cleanup.dryRun 通过后，真实删除前再过 task-003 白名单 + task-008 canonicalize。
