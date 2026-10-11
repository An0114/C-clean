# 测试评审报告

## 本次任务名称
003 落地删除唯一入口 `engine::execute::delete::execute()` + L3 红线白名单匹配器（feature/task-003-delete-whitelist）

评审人：TestReviewer
评审日期：2026-10-11
被审分支：`feature/task-003-delete-whitelist`（base: `main` @ 647e1c0）
合并目标分支：`main`
**风险等级：最高**（本任务是产品生命线——L3 误拦=安全，漏拦=删用户文件/系统崩溃）

---

## Git 提交记录核对结果（原子性、规范符合性）

feature 分支相对 main 共 **10 个提交**，全部携带 `(#task-003)` 尾标，type 前缀规范（build / feat / test / style / docs）：

| Commit | Message | 文件范围 | 原子性 |
|--------|---------|----------|--------|
| 66a2074 | docs(task): 新增任务003-删除唯一入口与L3白名单 | 仅 `tasks/003-...md` | ✅ DocsKeeper 派发基线 |
| ec9af02 | build(deps): 为 engine::execute 引入 thiserror 依赖 | `engine/Cargo.toml` `Cargo.lock` | ✅ 干净 |
| 05f5e8f | feat(execute): 新增模块骨架与 Deleter trait 抽象 | `lib.rs` `execute/{mod,delete,normalize,whitelist}.rs` | ⚠️ 骨架提交预创建了 normalize/whitelist 空文件（后续提交填充），非业务代码混杂 |
| b4928ac | feat(execute): 实现 Windows 路径规范化器 | 仅 `normalize.rs` | ✅ 干净 |
| 78b0a33 | feat(execute): 实现内置 L3 红线路径匹配器 | 仅 `whitelist.rs` | ✅ 干净 |
| 141b8ed | feat(execute): 落地 delete::execute() 唯一入口 | 仅 `delete.rs` | ✅ 干净 |
| 478674d | test(execute): 补 PRD P0-7 全部 L3 拦截与放行用例 | `whitelist.rs`（微调）+ `tests/execute_whitelist.rs` | ⚠️ 测试提交微调了 whitelist.rs，可接受 |
| e22b404 | style(execute): 按 rustfmt 统一 if 表达式换行 | 仅 `normalize.rs` | ✅ 干净 |
| 69af8b9 | docs(state): 任务003完成 | 仅 `docs/PROJECT_STATE.md` | ✅ 干净 |
| 1c66ad3 | docs(task): 追加实现备注 | 仅 `tasks/003-...md` | ✅ 干净 |

**偏差（不阻断）**：
- 任务预期 7 个提交，实际 10 个（多出 DocsKeeper 基线、style 修复、实现备注），均为文档/风格类。
- 骨架提交预创建了 normalize/whitelist 空文件，后续提交再填充——常见做法，不影响最终代码。
- 测试提交微调了 whitelist.rs（可能是测试驱动的小调整），可接受。

---

## 分步验证清单（逐项对照验收标准）

### A. 模块与依赖
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 1 | execute/ 含 mod.rs / normalize.rs / whitelist.rs / delete.rs | ✅ | 目录树核对 |
| 2 | lib.rs 仅新增 `pub mod execute;`，不动 db 与 hello_engine | ✅ | `lib.rs` L16 一行 |
| 3 | [dependencies] 仅新增 thiserror="1"；不加 serde/regex/walkdir；L3 匹配不用 regex | ✅ | `Cargo.toml` L14；whitelist.rs 全用字符串比较 + starts_with/contains |

### B. 路径规范化（normalize.rs）
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 4 | normalize(raw) -> Result<NormalizedPath, NormalizeError>；拒绝相对路径/空串/NUL/`..` 逃逸 | ✅ | L52-102：L54 Empty、L57 Nul、L71 NotAbsolute、L87 TraversalEscapes（见到任何 `..` 即拒，比任务要求更严格） |
| 5 | 大小写不敏感；正斜杠归一为反斜杠；剥 `\\?\` 前缀；大写盘符 | ✅ | L62 replace('/','\')、L64 strip_prefix、L76 to_ascii_uppercase、L110 to_lowercase |

### C. L3 红线匹配器（whitelist.rs）
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 6 | DenialReason 六变体：L3PersonalDir/L3BrowserProfile/L3BrowserSensitive/L3SystemPath/L3ReservedAction/PathTraversal | ✅ | L36-49 |
| 7a | 个人目录：Desktop/Documents/Downloads/Pictures/Videos/Music + 中文桌面/文档/下载/图片/视频/音乐；结构 `users/<name>/<dir>` | ✅ | L11-26 PERSONAL_DIRS 12 项；L74-78 要求 comps[i-2]=="users" |
| 7b | 浏览器 User Data / Firefox Profiles 前缀匹配；Bookmarks/Login Data/History 精确文件名 | ✅ | L66-71 User Data/Firefox；L81-85 文件名精确匹配 |
| 7c | System32/Installer 及后代；pagefile.sys/swapfile.sys 本体 | ✅ | L57-63 |
| 7d | check_action 拒绝 `/ResetBase` | ✅ | L92-97 |
| 8 | classify(p) -> Result<(), DenialReason> | ✅ | L52 |

### D. 唯一入口 delete::execute()
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 9 | Deleter trait（delete_file/delete_dir）；NoopDeleter 记录路径不碰磁盘 | ✅ | L8-42 |
| 10 | ExecuteRequest{paths,actions} / PathDecision{input,normalized,allowed,denial_reason} / ExecuteOutcome{Allowed,Rejected{code:2002}} | ✅ | L46-86 |
| 11 | 流程：normalize → classify → check_action；任一拒绝 → Rejected{2002}，不调 deleter；全过 → Allowed（本任务不自动调 deleter） | ✅ | L92-146：`_deleter` 参数带下划线，函数体完全不调用 deleter；测试 execute_rejected_does_not_call_deleter 验证 recorded().len()==0 |
| 12 | 不写 SQLite/审计/规则文件/spawn 子进程 | ✅ | 全模块无 std::fs、无 Command::new、无 db 调用 |

### E. 单元测试
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 13 | 纯内存测试，不碰真实 C 盘/用户目录/浏览器 | ✅ | 全部用字符串字面量 |
| 14a | blocks_personal_desktop | ✅ | 测试通过 |
| 14b | blocks_personal_documents | ✅ | 测试通过 |
| 14c | blocks_personal_downloads（D盘） | ✅ | 测试通过 |
| 14d | blocks_personal_pictures | ✅ | 测试通过 |
| 14e | blocks_personal_videos | ✅ | 测试通过 |
| 14f | blocks_personal_music | ✅ | 测试通过 |
| 14g | blocks_browser_user_data（Chrome） | ✅ | 测试通过 |
| 14h | blocks_browser_edge_user_data | ✅ | 测试通过 |
| 14i | blocks_browser_bookmarks_file | ✅ | 测试通过 |
| 14j | blocks_browser_login_data | ✅ | 测试通过 |
| 14k | blocks_browser_history | ✅ | 测试通过 |
| 14l | blocks_system32 | ✅ | 测试通过 |
| 14m | blocks_windows_installer | ✅ | 测试通过 |
| 14n | blocks_pagefile | ✅ | 测试通过 |
| 14o | blocks_swapfile | ✅ | 测试通过 |
| 14p | blocks_resetbase_action | ✅ | 测试通过 |
| 14q | blocks_path_traversal | ✅ | 测试通过 |
| 14r | blocks_case_insensitive | ✅ | 测试通过 |
| 14s | blocks_mixed_separators | ✅ | 测试通过 |
| 15a | allows_windows_temp_log | ✅ | 测试通过 |
| 15b | allows_user_appdata_local_temp | ✅ | 测试通过 |
| 15c | allows_simple_cache 反例（User Data 前缀命中，预期被拒） | ✅ | 测试通过（命名为 allows_ 但断言 !allowed，正确验证前缀拦截） |
| 15d | execute_rejected_does_not_call_deleter | ✅ | 测试通过（NoopDeleter.recorded().len()==0） |
| 15e | execute_allowed_returns_all_decisions | ✅ | 测试通过 |

### F. 卫生与边界
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 17 | 不创建 scan/grade/safebox/audit/rules/platform 等子模块 | ✅ | engine/src/ 仅 db/ + execute/ |
| 18 | 不修改 app/elevation/rules/scripts/engine/src/db/** | ✅ | diff stat 仅 execute/ + Cargo.toml + Cargo.lock + docs/ |
| 19 | 不修改 PRD/TRD/ARCHITECTURE/AGENTS | ✅ | diff name-only 核对 |
| 20 | 不硬编码真实用户名到生产代码 L3 列表 | ✅ | PERSONAL_DIRS 只用文件夹名，不限定用户名；`alice/bob` 仅出现在测试字符串 |
| 21 | 不调用 std::fs::remove_file/remove_dir | ✅ | grep 自检通过；Deleter 是 trait 抽象，NoopDeleter 只 push Vec |
| 22 | 不写 audit_log SQL / Tauri command / 规则 JSON 解析 | ✅ | 全模块无 SQL、无 tauri::command、无 JSON 解析 |

### 评审人复跑自验命令
| 命令 | 退出码 | 结果 |
|------|--------|------|
| `cargo test -p engine` | 0 | 42 passed / 0 failed（9 lib + 8 db + 24 execute + 1 smoke） |
| `cargo clippy -p engine --all-targets -- -D warnings` | 0 | 无 warning |
| `cargo fmt --all -- --check` | 0 | 无 diff |
| `cargo check --workspace --all-targets` | 0 | 编译通过 |

---

## 代码评审发现问题（深度安全分析）

本任务是产品生命线，评审人对 normalize/whitelist/delete 三模块做了逐行绕过分析：

### 安全设计亮点
1. **`..` 一律直接拒绝，不做词法折叠**（normalize.rs L87）：比任务要求更严格。任务原文说"折叠后逃逸盘符根才拒"，但 DevAgent 发现 E-14 用例 `C:\Windows\Temp\..\..\Users\alice\Documents\bank.pdf` 折叠后是合法路径会漏拒，故见到任何 `..` 即拒。**这是零逃逸空间的安全策略**，删除网关不需要任何未解析的相对上跳。✅
2. **拒绝时完全不调用 Deleter**：`_deleter` 参数带下划线，函数体零调用；测试 `execute_rejected_does_not_call_deleter` 断言 NoopDeleter.recorded().len()==0。✅
3. **个人目录匹配要求 `users/<name>/<dir>` 结构**（whitelist.rs L75 `comps[i-2]=="users"`）：避免误拦 `C:\Temp\Desktop\foo.tmp` 这类非个人目录下的同名文件夹。✅
4. **无 regex 依赖**：全部用字符串比较 + starts_with/contains，消除 regex 注入面。✅
5. **`\\?\` verbatim 前缀剥离**：处理 Windows 扩展路径语法。✅

### 非阻断提示项（已知限制/后续任务跟进）
1. **8.3 短文件名绕过**：`C:\Windows\SYSTEM~1\kernel32.dll` 不匹配 "system32"。纯词法检查无法识别 8.3 短名，需 `GetLongPathName` API 解析。DevAgent 已在备注说明符号链接解析留待 platform 模块，8.3 短名同理。**建议 platform 任务立项时一并处理**。
2. **符号链接/junction 绕过**：`C:\Temp\link -> C:\Users\alice\Documents`，词法路径不命中 L3。DevAgent 备注明确留待 platform。**这是上线前必须补的安全点**（M1 阶段至少要对待删路径做 `symlink_metadata` 检查，拒绝符号链接或解析后重判）。
3. **`Bookmarks.bak` / `Login Data.old` 等备份文件**：精确文件名匹配不拦截带后缀的变体。任务要求"精确文件名"，按字面实现。**建议 task-005 加载规则库时把备份文件模式补入外部红线集**。
4. **`check_action` 只匹配 `/resetbase`**：DISM 也接受 `-ResetBase` 或 `--resetbase`。本任务不做真实 DISM 调用，动作字符串只是检查。**建议接真实 DISM 时扩展为同时匹配 `/` 和 `-` 前缀**。
5. **UNC 路径保守拒绝**：`\\server\share\...` 返回 NotAbsolute。DevAgent 备注说明网络盘 L3 语义不同，留待 platform。✅ 合理。
6. **外部红线集 trait 边界未接入**：任务要求的"重跑外部红线集"本任务只做了 trait 边界，实际外部规则在 005 加载。当前 execute 只跑内置红线。✅ 符合任务边界。

---

## 安全风险检查结果

按 OWASP Top 10 (2021)：

| 风险项 | 本任务暴露面 | 结论 |
|--------|--------------|------|
| A01 失效访问控制 | 删除唯一入口强制所有路径经过 normalize+classify；Deleter 是 trait 注入，本任务 Noop 不碰磁盘 | ✅ |
| A02 加密失败 | 无凭据、无加密 | ✅ N/A |
| A03 注入 | 无 SQL（本任务不写 DB）；无 shell 调用；路径全词法规范化；`..` 一律拒；无 regex | ✅ **零注入面** |
| A04 不安全设计 | 拒绝时不调用 deleter（fail-closed）；code=2002 统一信封；built-in 红线永不被外部规则移除 | ✅ |
| A05 安全配置错误 | N/A | ✅ N/A |
| A06 脆弱过时组件 | thiserror 1.0.69、rusqlite 0.40.2 均为 stable；无新增脆弱依赖 | ✅ |
| A07 身份识别 | N/A | ✅ N/A |
| A08 软件与数据完整性 | NoopDeleter 不碰磁盘；真实 deleter 由后续任务注入，注入点唯一 | ✅ |
| A09 日志监控不足 | PathDecision 保留原始输入 + 规范化结果 + 拒绝原因，供上层审计落库 | ✅ 已为 audit 留好数据 |
| A10 SSRF | 无网络调用 | ✅ N/A |

**关键安全结论**：本任务实现了 fail-closed 设计——任何一条路径或动作命中红线，整体 Rejected 且 Deleter 零调用。这是正确的安全默认值。

---

## 评审结论：**通过**

22 条验收标准全部满足；4 项自验命令评审人复跑全部 exit 0（42 测试全过、clippy -D warnings、fmt --check、workspace check）；19 条 L3 拦截用例 + 5 条放行/行为用例全部通过；OWASP Top 10 无阻断风险。

`..` 一律拒绝的策略比任务要求更严格，是正确的安全取舍。

---

## 如果不通过：Bug 清单 + 修复建议

不适用（本次评审通过）。

非阻断提示项已在上文列出，建议：
- **platform 任务立项时**必须把「符号链接/junction 解析 + 8.3 短名解析 + canonicalize 后重跑白名单」写入验收标准——这是本任务最大的已知限制，上线前必须补。
- task-005 加载外部规则库时，把 `Bookmarks.bak` 等备份文件模式补入外部红线集。
- 接真实 DISM 时扩展 `check_action` 同时匹配 `/` 和 `-` 参数前缀。

---

## Git 操作记录

- 合并方式：`git merge --no-ff feature/task-003-delete-whitelist`
- **Merge commit hash：`c7c58ab`**
  - message：`merge(feature/task-003): 落地删除唯一入口 engine::execute::delete::execute() + L3 红线白名单匹配器 (#task-003)`
- 本地 feature 分支：已删除（`Deleted branch feature/task-003-delete-whitelist (was 1c66ad3)`）
- 远端分支 `origin/feature/task-003-delete-whitelist`：保留未删除、未 push main（延续既有约定）。
- 未触发任何 `git revert`。

---

## 后续动作建议

1. **派发 task-004**：IPC 网关骨架——统一响应信封 {code,message,data}、scan.start/cleanup.dryRun 命令、进度事件通道。注意把 `execute::execute()` 接到 Tauri command 时，必须把 `NoopDeleter` 换成真实 deleter（或先接保险箱），且所有 IPC 错误都要映射到 {code,message,data} 信封。
2. **强烈建议在 platform 任务（或 task-004 之后）立即补**：路径 canonicalize（`std::fs::canonicalize`）后重跑白名单，防符号链接/junction 绕过。这是 L3 红线目前唯一的理论漏拦面。
3. 维护者择机 push main 并清理远端 feature 分支。
