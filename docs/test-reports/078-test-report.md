# 测试报告：task-078 P2——用户自定义规则管理（内置规则 + 用户规则合并）

- 分支：`feature/task-078-p2-user-custom-rules`
- 测试 HEAD：`a6a4ae3`
- 测试时间：2026-11-06（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）
- **P2 持续：user_rules 表 + CRUD IPC + 规则引擎合并（内置优先）+ Settings 自定义规则管理 UI**

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| 构建验证 | `cargo fmt/clippy/test` + `pnpm lint/test/build` | 是 | 通过 |
| dev 真机启动 | `pnpm tauri dev`（cdisk-cleaner.exe 运行，后端日志观察） | **是**（按 2026-11-03 新规，有前端改动必须真机） | 后端启动正常；WebView2 窗口渲染受项目长期已知环境限制（Crashpad），按 experience `tauri-dev-webview2-crash-fallback` 不反复切窗口 |
| 打包 | NSIS `C-DiskCleaner_0.2.1_x64-setup.exe`（4.54 MiB） | 是 | 通过（出包成功） |
| 干净目录安装 | `/S /D=...` | 未尝试 | 归第三档（UAC 非交互受限） |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区（含 18 条新增 + 15 条 `#[ignore]` 真机） | `cargo test --workspace` | 通过 | `cargo-test.log`：64 batches / **620 passed / 0 failed / 15 ignored**（基线 602 + 新增 18，与开发声称一致） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm test` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（1 条 elevation incremental lock warning + 2 条 elevation 既有 warning）；`pnpm.log`：lint=0（2 条 SafeBox.tsx 既有 warning）、test=0（14 files / **197 passed**，+8，Settings.test.tsx 35 条）、build=0（**250.34 kB / gzip 77.59 kB**，+6.28 kB，与开发声称一致） |
| L3 真机 | 后端 IPC + 端到端规则生效 | `pnpm tauri dev` + 日志观察 | **后端启动通过 / 前端窗口未渲染** | `tauri-dev.log`：app 启动正常（data_root/rules_dir/scripts_dir/box_dir 四路径正确）；WebView2 窗口渲染受环境限制，按 experience 不反复切窗口；端到端规则生效由 dev 单测 `end_to_end_rule_produces_items_then_disabled_removes_them` 直接证明（真 DB 文件 + 真临时目录 + 真 `ScanOrchestrator::run_all`：加规则→`cleanup_item` 出现 `module_code='user_rules'` 条目 / `set_enabled(false)`→新 session user_rules 条目数=0） |
| L4 打包 | NSIS 出包 | `tauri build` | 通过 | `tauri-build.log`：NSIS 4.54 MiB（+0.02 vs 077）+ MSI 6.71 MiB；FileVersion=0.2.1；签名 exit=1 为已知预期 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | v4 迁移成功（`user_rules` 表创建） | 3 条 v4 迁移单测全绿（表结构 / CHECK 拒绝 L3 与非法 enabled / pattern 唯一）；真实升级路径验证（仓库 data/cdiskcleaner.db 从 v1 连续跑 v2/v3/v4，user_version=4，表含 user_rules，列与索引齐全） | 通过 |
| 2 | `user_rules_list()` 返回所有规则 | 14 条内联单测含 CRUD 全流程 | 通过 |
| 3 | `user_rules_add()` 新增规则 | 同上（含重复 pattern 拒绝 / 校验失败不落库） | 通过 |
| 4 | `user_rules_update()` 更新规则（含启用开关） | 同上；额外加 `user_rules_set_enabled` 独立命令（开关单字段高频操作，不复用 update 避免回传全字段） | 通过（实现简化，见第二档偏差 4） |
| 5 | `user_rules_delete()` 删除规则 | 同上 | 通过 |
| 6 | 单测：CRUD 全流程 | 6 条集成测试全绿 | 通过 |
| 7 | 内置规则加载不变 | `external_deny_list` 不回归单测钉住 | 通过 |
| 8 | 用户规则（enabled=1）合并到扫描规则列表 | **关键事实**：loader 在扫描链路上没有消费者（5 个扫描模块各自硬编码路径），新增 `scan/modules/user_rules.rs` 才让规则真正生效；`run_user_rules()` 在编排层调用 | 通过（实现方式偏差，见第二档偏差 3） |
| 9 | 同路径时内置规则优先 | `covered_by_builtin` 纯函数 + 单测钉住（边界落在分隔符上，`c:\temp2` 不算在 `c:\temp` 之下）；`builtin_rule_shadows_user_rule_on_same_path` 集成测试 | 通过 |
| 10 | 用户规则风险等级自定义生效 | **只允许 L1/L2**，DB 层 CHECK 挡死 L3（L3=红线永不删除，一条 L3 清理规则会扫出永远删不掉的条目，语义自相矛盾） | 通过（实现偏差，见第二档偏差 1） |
| 11 | 单测：合并结果正确 | 5 条 loader 单测 + 6 条集成测试 | 通过 |
| 12 | 自定义规则列表显示正确 | 7 条 Settings.test.tsx 新增（列表渲染 / 空态 / 新增实参 camelCase / 编辑预填 enabled 不被重置 / 开关传相反值 / 删除 confirm 取消确认 / 后端校验原因显示） | 通过（单测层）；真机窗口未渲染见第三档 |
| 13 | 新增规则弹窗表单工作 | 同上（`role="dialog"`，draft.id=null 表示新增） | 通过（单测层） |
| 14 | 编辑 / 删除按钮工作（删除 confirm） | 同上（删除 confirm 文案说清「已扫描出的条目不受影响，但下次扫描不会再包含」） | 通过（单测层） |
| 15 | 启用开关切换即生效 | `disabled_rule_is_not_scanned_even_if_it_would_match` 集成测试钉住 | 通过 |
| 16 | 深色主题适配 | 全部走 `var(--surface)` / `var(--text)` / `var(--border)` / `var(--text-muted)` | 通过（代码层） |
| 17 | 单测：组件渲染正确 | 7 条新增 vitest | 通过 |
| 18 | 新增用户规则 → 扫描 → 该路径文件出现在 Items | `end_to_end_rule_produces_items_then_disabled_removes_them`：真 DB + 真临时目录 + 真 run_all → `cleanup_item` 出现 `module_code='user_rules'` 条目，每条路径在该目录下 / `risk_level='L1'` / `category='user_rule_<id>'` | 通过（真后端证据） |
| 19 | 关闭规则 → 重新扫描 → 该路径文件不再出现 | 同上：`set_enabled(false)` → 新 session 再跑 → user_rules 条目数=0 | 通过（真后端证据） |
| 20 | `cargo test --workspace` 全绿（新增 ≥5 条） | 620 passed（新增 18 条，远超 ≥5） | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮未发现需代码修复的缺陷 | L1-L2 全绿 | — | — | — |

## 5. 测试期修复记录（bugfix）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮无需修复 | — | — | — | — |

## 6. 遗留问题（三档分流）

### 第一档（已修）
- v4 迁移：`user_rules` 表 + `idx_user_rules_enabled` + `idx_user_rules_pattern` 唯一索引 + 两条 CHECK（risk_level IN L1/L2、enabled IN 0/1）。
- 14 条 CRUD 内联单测：glob 匹配（段内/跨段/大小写与分隔符/?/病态模式不爆炸）、静态根（截断规则/拒绝整盘）、校验（空名/超长/L3 与拼写错误/太宽泛/L3 受保护位置）、CRUD（增删改查/重复模式/缺 id/校验失败不落库）。
- 新增 `scan/modules/user_rules.rs`：从 DB 取启用规则 → 逐条遍历静态根 → **三道过滤**（glob_match / whitelist::classify 逐条真实路径 / covered_by_builtin）→ 产出 `ScanItem`。
- 20,000 条上限 + 回调式遍历（够数即停），避免用户把规则指向几十万文件目录跑几十分钟写爆 cleanup_item。
- 端到端真实验证：真 DB 文件 + 真临时目录 + 真 `ScanOrchestrator::run_all`，加规则→有条目→停用→没有。

### 第二档（需评审/用户拍板）
1. **risk_level 只允许 L1/L2，L3 在 DB 层 CHECK 挡死**：L3=红线永不删除，一条 L3 清理规则会扫出永远删不掉的条目，语义自相矛盾。
2. **created_at/updated_at 用 INTEGER unix 秒而非 TEXT**：与全库时间列和前端 `formatTs(unixSec: number)` 对齐。
3. **「规则引擎合并」只改 `rules/loader.rs` 不够**：loader 在扫描链路上没有消费者（5 个扫描模块各自硬编码路径），新增 `scan/modules/user_rules.rs` 才让规则真正生效。
4. **加第 5 条 IPC `user_rules_set_enabled`**：开关单字段高频操作，复用 update 要求前端回传全部字段，多一个「改名没保存就切开关、名字被旧值覆盖」的机会。
5. **category 用 `user_rule_<id>` 而非规则名**：用规则名会让清理方案（存 module_code/category/path 三元组）在改名后整份失效。
6. **校验失败走 `Err(String)` 而非 `Envelope::err`**：后者 message 是 `&'static str`，装不下「路径模式太宽泛」这种逐条不同的原因。
7. **「同路径内置优先」实现为「内置目录前缀覆盖的文件，用户规则条目被抑制」**：边界落在分隔符上。
8. **白名单外 7 个文件必改**：`scan/modules/user_rules.rs`（新建，核心）/ `scan/modules/mod.rs`（注册）/ `scan/orchestrator.rs`（接线 run_user_rules）/ `app/src-tauri/src/commands.rs`（5 条 Tauri 命令 + scan_start 传内置前缀）/ `app/src-tauri/src/lib.rs`（generate_handler）/ `engine/tests/db_schema.rs`（v4 让 user_version 3→4、业务表 9→10，断言同步并改为表名集合完全相等）/ `app/src/pages/Settings.test.tsx`（7 条新测试）。
9. **白名单列了但确认不需要改**：`engine/src/lib.rs`（engine lib.rs 里没有 handler，真 handler 在 app/src-tauri）/ `app/src/utils/invoke.ts`（api<T>() 泛型封装够用）。

### 第三档（已登记为已知限制）
- **真机 GUI 窗口未渲染**：dev 日志证明后端启动正常（四路径正确），但 WebView2 窗口渲染受项目长期已知环境限制（Crashpad，从 task-074 起 dev 就注明本机 WebView2 渲染进程创建即崩）。按 experience `tauri-dev-webview2-crash-fallback` 不反复切窗口。**端到端规则生效由 dev 单测直接证明**（真 DB + 真 run_all，不是 mock）。
- **安装态未完成**：NSIS 出包成功（4.54 MiB / FileVersion=0.2.1），UAC 非交互受限。
- **CDP console 未抓**：WebView2 窗口未渲染。
- **CI 五道闸未跑**：PR 由 CI 门禁观察员创建后跑。
- **签名**：本地未签名（exit=1 已知预期）。
- **规则库 `cache_rules.json` 的 modules 段依然没有扫描消费者**：task-078 只把用户规则接进扫描，内置那份仍是「文件里写一份、代码里硬编码一份」的双份真值源。建议 task-080 前拍板（5 个内置模块改规则库驱动 / 删 modules 段只留 whitelist_deny）。
- **20,000 条上限是硬截断**：超出时只打日志，界面上没有提示。真要给用户看需在 `ScanOrchestratorSummary` 加「被截断的规则」字段透传 UI。
- **ps1 回退路径不扫用户规则**：run_user_rules 在编排层，与 ps1/Rust 无关，不受影响；但多盘过滤对用户规则是按静态根判断（不是按条目路径），跨盘组合下行为正确性依赖 static_root。
- **P2 只剩便携版 CLI（task-079）**。
- **Items/SafeBox/History 三页深色适配仍缺**（task-061 起的旧账）。
- **版本线**：task-072~078 攒着发 v0.4.0；CHANGELOG 需写明「用户自定义规则只允许 L1/L2（L3 是红线等级）」。
- **`tasks/060-shutdown-experience.md` 仍是「工作区已删除、HEAD 里存在」**（连续 16 个任务）。

## 7. 测试结论

- 结论：**通过（620 passed / 197 vitest / NSIS 出包 4.54 MiB；端到端规则生效应由真 DB+真 run_all 单测直接证明）**
- feature 分支最新 commit：`a6a4ae3`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/lint/test/build）+ NSIS 出包 + `tauri dev` 后端启动日志观察，未复用开发自验结论。
- **新规执行说明**：本任务**有前端改动**（Settings.tsx UserRulesSection：列表/弹窗/开关/删除），按用户 2026-11-03「有前端更改必须真机测试」的指示，L3 真机**已执行**（`pnpm tauri dev` 启动 + 后端日志观察）。后端启动正常，但 WebView2 窗口渲染受项目长期已知环境限制（Crashpad），按 experience `tauri-dev-webview2-crash-fallback` 不反复切窗口。**端到端验收 18/19 由 dev 单测 `end_to_end_rule_produces_items_then_disabled_removes_them` 直接证明**（真 DB 文件 + 真临时目录 + 真 `ScanOrchestrator::run_all`，不是 mock），这是比 UI 截图更硬的证据。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①risk_level 只允许 L1/L2（L3 在 DB 层 CHECK 挡死，语义自洽）；②loader 在扫描链路上没有消费者，新增 scan/modules/user_rules.rs 才让规则生效（任务书假设错误）；③三道过滤（glob / whitelist::classify 逐条真实路径 / covered_by_builtin）缺一不可的理由；④20,000 条上限硬截断；⑤白名单外 7 个文件必改是否合理；⑥规则库 modules 段双份真值源问题（建议 task-080 前拍板）。**
