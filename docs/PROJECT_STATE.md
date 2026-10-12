# PROJECT_STATE.md

> **特别声明**：本文件是项目唯一真实数据源，也是「每次任务结束必更新、必单独提交」的文件。后续所有 Agent 必须遵守：任何任务完成后都要先更新本文件，再单独提交一次（commit message 形如 `docs(state): ... (#task-00X)`），不得与业务代码混在同一个 commit。

## 当前目标
M1 业务模块 A 进行中：task-012 Rust 侧 ps1 NDJSON 消费器已就绪，下一步把 ScanRunner 接到 Tauri scan.start 命令。

## 最近一次变更
2026-10-11：完成 PRD / TRD / ARCHITECTURE / AGENTS / 立项决策记录全套长期基线文档，并初始化本状态白板。
2026-10-11：task-001 完成——落地 app/engine/elevation/rules/scripts 五大目录骨架、Cargo workspace + pnpm workspace 双工作区，Tauri 2 最小空窗口（标题「C-Disk Cleaner」）端到端跑通，Cargo.lock 已入库。
2026-10-11：task-002 完成——engine 内新建 db/ 子模块，rusqlite 0.40.2（bundled）连接入口、v1__init.sql 九张表 schema、user_version 手写迁移器，8 条集成测试全绿（含级联删除 / SET NULL / 单行表 CHECK / 迁移幂等）。
2026-10-11：task-003 完成——engine 内新建 execute/ 子模块：Windows 路径规范化器（反斜杠归一、大写盘符、`..` 一律拒）、内置 L3 红线路径匹配器（个人目录/浏览器数据/系统路径/动作级禁令）、`delete::execute()` 唯一入口（全拒不调 Deleter，拒绝信封码 2002）；24 条集成测试 + 9 条单元测试全绿。
2026-10-11：task-004 完成——engine 内新建 ipc/ 子模块：统一响应信封 `{code,message,data}`、TRD §5.3 全 13 个错误码、`scan.start`（递增 sessionId 入参校验）/ `cleanup.dryRun`（接 task-003 白名单 + MockResolver）纯逻辑、ProgressEmitter trait + 100ms 节流；app/src-tauri 注册两条命令。**显式偏离 TRD §5.1：L3 拒绝（2002）时 data 不置 null，回传 blocked_by_l3 明细**；7+3 条新测试全绿（engine 累计 52 条）。
2026-10-12：task-005 完成——落地 `rules/cache_rules.json`（npm_cache 示例 + 4 条外部红线）与配对 `rules_meta.json`（真实 SHA-256）；engine 新建 rules/ 子模块：serde 模型 / 白名单环境变量展开（5 变量 + `~`）/ SHA-256 校验加载器（篡改即拒不写盘）/ `classify_with_external` 与内置红线取并集；9 条集成测试全绿（engine 累计 61 条）。
2026-10-12：task-006 完成——M0 收官。新增 `rust-toolchain.toml`（channel=stable + rustfmt/clippy）与 `.github/workflows/ci.yml`：windows-latest 单 job 跑五道闸（fmt --check → clippy -D warnings → cargo test → pnpm install --frozen-lockfile → pnpm --filter app build），另加 rules/*.json SHA-256（去 CR 归一 LF）与 JSON 合法性两个自洽守卫；app/package.json 补 lint/test echo 占位。PR #1 触发 GitHub Actions 实测经 2 轮修复（Node 20→22、pnpm 9→11；checksum 去 CR）后 ci #3 全绿（5m56s）。M0 公共底座（001~006）全部就绪。
2026-10-12：task-007 完成——真实进度通道落地：新增 `app/src-tauri/src/progress.rs`（`TauriProgressEmitter` 包装 `AppHandle`，emit 失败仅吞；`ProgressState = Mutex<ThrottledEmitter<TauriProgressEmitter>>`）；Builder 的 setup 闭包内 `app.manage(...)` 注入；`scan_start` 命令返回前按真实 sessionId emit 一条 `percent=0, current_path="starting"` demo 进度。engine trait 零改动；cargo clippy/fmt/test、pnpm build 全绿。devtools listen 真机复核见任务实现备注。
2026-10-12：task-008 完成——新增 `engine::execute::resolve`（`resolve_to_real`：canonicalize 去 `\\?\` 前缀、UNC 保守拒、NotExists/Io/LoopDetected 三态错误）；delete 流程改为 normalize → resolve → 对**真实路径**再过 normalize+classify；新增 `DenialReason::ResolveFailed(String)`；NotExists 时回退对输入字符串判 L3（非存在路径不放宽红线、也不误杀 L1 单测）。6 条新集成测试（tempfile + mklink /J junction）全绿，engine 累计 67 条。
2026-10-12：task-009 完成——新增 `engine::db::resolver::DbItemPathResolver`（持有 `&Connection`，参数化按 id 查 `cleanup_item`，查不到跳过、SQL 失败返回空 Vec + eprintln）；4 条集成测试全绿（空库/匹配/未知 id/dryRun 端到端 2002）。Tauri 命令仍用 MockResolver，切换留 M1 扫描任务。
2026-10-12：task-010 完成——`handle_dry_run` 签名加 `external_deny: &[String]`，内部换 `classify_with_external`；`cleanup_dry_run` Tauri command 启动时 `current_dir()/rules` 加载规则库，`external_deny_list` 注入，加载失败保守返 5001；3 条新集成测试全绿（tempdir 自算 SHA-256 配 meta）。
2026-10-12：task-011 完成——从零产出 `scripts/PROTOCOL.md`（item/done 两种 NDJSON 事件 schema）+ 5 个扫描脚本（temp/windows_update/prefetch/browser_cache/dev_cache），全部 `#requires -Version 5.1`、UTF-8 无 BOM、stdout 只出 NDJSON、跳过 LinkType、不删除；`smoke_test.ps1` 实跑 5 脚本逐行 ConvertFrom-Json 全绿（temp=154/wu=6/prefetch=0/browser=10/dev=33 行）。M1 前置清理清单收官。
2026-10-12：task-012 完成——`engine::scan` 模块上线：`NdjsonEvent` serde 枚举（item/done + deny_unknown_fields）、`parse_line` 单行解析器；`ScanRunner` spawn `powershell.exe -File`，逐行写 `cleanup_item`（risk_level int→L0/L1/L2 映射、description 空串兜底）并 emit 进度；3 条假 ps1 集成测试全绿（含坏行跳过 / exit 1 无 done 报错）。

## 已完成事项
- [x] 项目立项与仓库初始化（commit 67990f5 / 4972f9d）
- [x] 固化立项阶段 8 项关键决策：Tauri 无 Plan B / Cargo.lock 入库 / 保险箱 5GB FIFO / 规则库 GitHub Raw+SHA-256 / 不买 EV 签名 / 仅 Windows / 按业务模块 A~F 切分 / PROJECT_STATE 由 M0 当天首写（commit c0bdf05）
- [x] 完成产品需求文档 PRD v0.1（commit caae920）
- [x] 完成技术需求文档 TRD v0.1（commit 261b24f）
- [x] 完成系统架构文档 ARCHITECTURE v0.1（commit c1f027a）
- [x] 完成多 Agent 协作与 Git 权限规范 AGENTS.md（commit 3a1bb2c）
- [x] 初始化项目状态白板 PROJECT_STATE.md（本次提交）

## 待办事项
- [x] 001：初始化 monorepo 目录骨架（app/ Tauri+React+Vite、engine/ Rust workspace、elevation/、rules/、scripts/），跑通最小空窗口
- [x] 002：落地公共数据层——SQLite 单文件 schema（settings/scan_session/cleanup_item/cleanup_task/safe_box/audit_log/rules_meta/registry_backup/cleanup_plan）与 rusqlite 连接、迁移
- [x] 003：落地删除唯一入口 engine::execute::delete::execute() + L3 红线白名单匹配器 + 单元测试（对 PRD P0-7 全部 L3 路径 100% 拦截）
- [x] 004：落地 IPC 网关骨架——统一响应信封 {code,message,data}、scan.start/cleanup.dryRun 命令、进度事件通道
- [x] 005：落地 rules/cache_rules.json 初始规则库 + rules_meta.json + SHA-256 校验加载逻辑
- [x] 006：搭好 CI（GitHub Actions：cargo clippy/fmt/test + pnpm lint/test + 规则 JSON schema 校验）
- [x] 007：真实 ProgressEmitter（AppHandle::emit）——scan://progress 真机可被前端 listen 收到（依赖 task-004、真实扫描任务；约 1 人天）
- [x] 008：symlink/junction 解析防绕开——规范化器接 symlink_metadata，junction 指向 L3 时一并拒（依赖 task-003、platform 模块；约 1 人天）
- [x] 009：DB 版 ItemPathResolver——cleanup_item 表按 id 取路径/大小/模块，替换 MockResolver（依赖 task-002、cleanup.execute；约 1 人天）
- [x] 010：外部红线集注入——rules JSON 加载后与内置 L3 红线取并集（依赖 task-005；约 0.5 人天）
- [x] 011：ps1 兼容层来源对齐与 NDJSON 改造（scripts/ 五个脚本；约 1 人天）
- [x] 012：Rust 侧 ps1 NDJSON 消费器——parser + ScanRunner（spawn/写库/emit）

## 已知问题
- [B 低] task-009 `DbItemPathResolver` 已就绪并被 dryRun 单测端到端覆盖，但 Tauri 命令 `cleanup.dryRun` 仍注入 `MockResolver`（真实扫描还没产生 cleanup_item 数据）；切换时机留给 M1 扫描任务。
- [B 低] UNC 路径 `\\server\share\...` task-008 评估结论：**沿用保守拒绝**（normalize 层 NotAbsolute + resolve 层 Io 双保险）；网络盘 L3 红线策略留待真实 NAS/企业盘场景出现时再开。
- [B 低] task-007 WebView2 devtools console 的 `listen('scan://progress')` 真机截图未在本轮自动化环境内取得（tauri dev 窗口启动后 GUI 自动化无法稳定聚焦 console）；通道本身已由编译期类型 + engine 节流单测保证，留待 UI 联调任务补截图。
- [B 低] task-005 `rules_meta` 落 DB（rules_meta 表运行时持久化）、GitHub Raw 联网拉取、`rules.checkUpdate`/`rules.applyUpdate` IPC 命令未做——本任务明确「不联网 / 不写 DB / 不接 Tauri command」，留待 M1 规则更新任务。补偿：静态 rules_meta.json 已含真实 SHA-256，loader 校验逻辑单测覆盖。
- [B 低] task-005 外部红线命中统一记为 `DenialReason::L3SystemPath`（遵守「whitelist.rs 仅新增函数、不改枚举/常量」约束）；后续审计若需区分内置/外部红线来源，再拆 DenialReason 变体。
- [B 低] task-006 CI 未接 Dependabot / cargo audit / npm audit（任务边界明确不做）；package.json 的 lint/test 为 echo 占位，CI 也不跑这两个命令，待前端组件落地（ESLint/Vitest）后再启用。CI 仅 windows-latest 单平台，跨平台矩阵（macOS/Linux）本项目明确不做；NSIS 出包矩阵未配，留待打包任务。

## 下一步计划
派发 task-013：把 ScanRunner 接到 Tauri scan.start 命令——spawn 5 个 ps1 串跑、写真实 session、前端 listen scan://progress 端到端。

## 当前负责人 Agent
DocsKeeper（文档与状态管家）

## Git 提交记录
| Commit | 任务 | 说明 |
|--------|------|------|
| 67990f5 | #task-000 | Initial commit |
| 4972f9d | #task-000 | chore: 初始化项目仓库与文档骨架 |
| c0bdf05 | #task-000 | docs: 固化立项阶段 8 项关键决策并修正 Cargo.lock 策略 |
| caae920 | #task-000 | docs(prd): 新增产品需求文档 v0.1 |
| 261b24f | #task-000 | docs(trd): 新增技术需求文档 v0.1 |
| c1f027a | #task-000 | docs(arch): 新增系统架构文档 v0.1 |
| 3a1bb2c | #task-000 | docs(agents): 新增多Agent协作与Git权限规范 |
| 09adc52 | #task-000 | docs(state): 初始化项目状态白板 |
| bdeeea4 | #task-001 | docs(task): 新增任务001-初始化monorepo脚手架（派发基线） |
| 42cc54f | #task-001 | chore(workspace): 初始化 Cargo workspace 与 pnpm workspace 配置 |
| 5742151 | #task-001 | chore(scaffold): 新增 engine 空 lib 与 elevation 空 bin 占位 crate |
| ecea8cb | #task-001 | feat(ui): 初始化 app/src Vite + React 18 + TypeScript 前端骨架 |
| 496568f | #task-001 | feat(tauri): 接入 app/src-tauri Tauri 2 壳并链接 engine |
| f041803 | #task-001 | chore(scaffold): 补全 rules/ scripts/ 占位与 README 启动说明 |
| （本次） | #task-001 | docs(state): 任务001完成，更新进度与下一步 |
| e62b586 | #task-002 | build(deps): 为 engine 引入 rusqlite bundled 依赖 |
| 8dea10f | #task-002 | feat(db): 新增 engine::db 模块骨架与连接入口 conn::open |
| bac3400 | #task-002 | feat(db): 编写 v1__init.sql 落地 9 张业务表 schema |
| d48933a | #task-002 | feat(db): 实现基于 user_version 的手写迁移器 |
| 2ecc248 | #task-002 | test(db): 补 schema 与迁移器单元/集成测试 |
| e6f18a1 | #task-002 | style(db): 修复 clippy needless_borrow 告警并按 rustfmt 统一格式 |
| （本次） | #task-002 | docs(state): 任务002完成，更新进度与下一步 |
| ec9af02 | #task-003 | build(deps): 为 engine::execute 引入 thiserror 依赖 |
| 05f5e8f | #task-003 | feat(execute): 新增 engine::execute 模块骨架与 Deleter trait 抽象 |
| b4928ac | #task-003 | feat(execute): 实现 Windows 路径规范化器（防 .. 穿越、统一斜杠、大小写不敏感） |
| 78b0a33 | #task-003 | feat(execute): 实现内置 L3 红线路径匹配器 |
| 141b8ed | #task-003 | feat(execute): 落地 delete::execute() 唯一入口，全拒即不调用 Deleter |
| 478674d | #task-003 | test(execute): 补 PRD P0-7 全部 L3 路径拦截与放行用例 |
| e22b404 | #task-003 | style(execute): 按 rustfmt 统一 if 表达式换行 |
| （本次） | #task-003 | docs(state): 任务003完成，更新进度与下一步 |
| 6c57410 | #task-004 | build(deps): 为 engine::ipc 引入 serde 与 serde_json |
| 8714c1f | #task-004 | feat(ipc): 实现统一响应信封与 TRD §5.3 错误码常量表 |
| f931517 | #task-004 | feat(ipc): 实现 ProgressEmitter trait 与 100ms 节流包装 |
| db1f9aa | #task-004 | feat(ipc): 实现 scan.start 命令纯逻辑（入参校验 + 递增 sessionId） |
| f4e48b2 | #task-004 | feat(ipc): 实现 cleanup.dryRun 命令纯逻辑（接 task-003 白名单匹配器） |
| 339c604 | #task-004 | feat(tauri): 在 app/src-tauri 注册 scan.start / cleanup.dryRun 命令 |
| c7a706e | #task-004 | test(ipc): 补信封序列化、错误码、命令分支与节流器测试 |
| （本次） | #task-004 | docs(state): 任务004完成，更新进度与下一步 |
| 649c005 | #task-005 | build(deps): 为 engine::rules 引入 sha2 与 hex |
| e7477e6 | #task-005 | feat(rules): 编写初始 cache_rules.json 与 rules_meta.json（含真实 SHA-256） |
| 5ba121a | #task-005 | feat(rules): 定义 CacheRules / RuleEntry / RulesMeta serde 模型 |
| 312683e | #task-005 | feat(rules): 实现路径变量白名单展开（6 个环境变量 + ~） |
| d218c80 | #task-005 | feat(rules): 实现 SHA-256 校验加载器，校验失败拒绝并保留旧版 |
| ccf6c69 | #task-005 | feat(rules): 把 whitelist_deny 接入白名单匹配器，与内置红线取并集 |
| 480022d | #task-005 | test(rules): 补 JSON 反序列化、变量展开、篡改拒绝、并集生效测试 |
| （本次） | #task-005 | docs(state): 任务005完成，更新进度与下一步 |
| 7ce5578 | #task-006 | chore(ci): 新增 rust-toolchain.toml 固定 Rust 工具链版本 |
| 565353e | #task-006 | chore(ci): 新增 GitHub Actions CI 工作流（fmt/clippy/test + pnpm build） |
| ce08901 | #task-006 | chore(ci): 在 CI 追加 rules/*.json SHA-256 与 JSON 合法性校验 |
| 95771fb | #task-006 | chore(ci): 在 app/package.json 补 lint/test 占位脚本 |
| e6e1505 | #task-006 | fix(ci): Node 22 + pnpm 11，对齐本地工具链（修 ERR_UNKNOWN_BUILTIN_MODULE） |
| 6ad5296 | #task-006 | fix(ci): checksum 校验先去 CR 归一 LF（windows runner autocrlf mismatch） |
| （本次） | #task-006 | docs(state): 任务006完成，M0 阶段收官 |
| 3e9235c | #task-007 | feat(tauri): 实现 TauriProgressEmitter（AppHandle::emit scan://progress） |
| 45c505d | #task-007 | feat(tauri): 在 Builder 注册 ProgressState 并改造 scan_start 命令 emit demo 进度 |
| （本次） | #task-007 | docs(state): 任务007完成，真实进度通道接通 |
| 75a0cce | #task-008 | feat(execute): 新增 resolve.rs——canonicalize 解析 junction/symlink 真实路径 |
| 16120f8 | #task-008 | feat(execute): 在 delete 流程串入 resolve，新增 DenialReason::ResolveFailed |
| 0a25a68 | #task-008 | test(execute): 补 junction 解析与 L3 拒绝集成测试 |
| 7f253af | #task-008 | style(execute): cargo fmt 统一格式 |
| （本次） | #task-008 | docs(state): 任务008完成，symlink/junction 防绕开 |
| 9ec8224 | #task-009 | feat(db): 新增 DbItemPathResolver 实现 ItemPathResolver trait |
| 47b542f | #task-009 | test(db): 补 DbItemPathResolver 集成测试（含 dryRun 端到端） |
| 6bc9fb2 | #task-009 | docs(state): 任务009完成，真实 DB resolver 就绪 |
| （本次） | #task-009 | docs(task): 回填任务009实现备注 |
| 4346211 | #task-010 | feat(ipc): handle_dry_run 加 external_deny 参数，换 classify_with_external |
| 28738fb | #task-010 | feat(tauri): cleanup_dry_run 加载 rules/ 注入外部红线，失败返 5001 |
| 6caf900 | #task-010 | test(ipc): 外部红线接入 dryRun 端到端单测 |
| （本次） | #task-010 | docs(state): 任务010完成，外部红线集已接入 dryRun |
| ff83b3c | #task-011 | feat(scripts): scan_temp 与 scan_windows_update（NDJSON） |
| 22d1e37 | #task-011 | feat(scripts): scan_prefetch / scan_browser_cache / scan_dev_cache |
| e7c06e4 | #task-011 | test(scripts): 5 脚本 NDJSON 冒烟全绿 |
| （本次） | #task-011 | docs(state): 任务011完成，M1 前置清理清单全部就绪 |
| 0e5e35e | #task-012 | feat(scan): NdjsonEvent serde 模型与 parse_line 单行解析器 |
| 3096cc7 | #task-012 | feat(scan): ScanRunner——spawn powershell、写 cleanup_item、emit |
| 4222d83 | #task-012 | test(scan): parser 与 runner 集成测试（假 ps1） |
| （本次） | #task-012 | docs(state): 任务012完成，Rust 侧 ps1 NDJSON 消费器就绪 |

---
变更轨迹：
- 2026-10-11  task-000  初始化 PROJECT_STATE.md，记录立项基线完成状态与 M0 待办
- 2026-10-11  task-001  初始化 monorepo 脚手架与 Tauri 2 最小空窗口
- 2026-10-11  task-002  落地 SQLite schema（9 张表）+ rusqlite 连接 + user_version 迁移器
- 2026-10-11  task-003  落地删除唯一入口与 L3 白名单匹配器
- 2026-10-11  task-004  落地 IPC 网关骨架（信封/错误码/scan.start/cleanup.dryRun/进度事件）
- 2026-10-12  debt-cleanup-批次001  A类口径回写 TRD §5.1（2002 data 例外）与 task-003 B.4（.. 一律拒）；拆出 task-007~011；B类已知限制登记
- 2026-10-12  task-005  落地初始规则库与 SHA-256 校验加载器
- 2026-10-12  task-005  评审前清障：独立复跑 cargo check/clippy/fmt/test 全绿、复核 checksum 一致；无第一档修复项；3 条第三档延期项登记入已知问题
- 2026-10-12  task-006  搭好 GitHub Actions CI（windows-latest 五道闸 + 规则自洽守卫），PR #1 经 2 轮修复后全绿，M0 阶段收官
- 2026-10-12  task-006  评审前清障：发现 loader.rs 算 checksum 未做 CRLF→LF 归一（Windows autocrlf 会把合法 rules 误判篡改），已修复并补 CRLF 容忍用例；M0 遗留 5 项均已登记已知问题
- 2026-10-12  task-007  落地真实 ProgressEmitter 与 scan://progress 前端 listen 端到端通道（demo emit 已通，真实扫描接入留 M1）
- 2026-10-12  task-007  评审前清障：独立复跑 clippy/test/pnpm build 全绿，核验 TauriProgressEmitter/ProgressState/scan_start 签名与 engine 常量对齐；无第一档修复项；devtools 截图缺口已登记已知问题
- 2026-10-12  task-008  junction/symlink 真实路径解析接入 delete 唯一入口（canonicalize + 对真实路径二次 L3 判定）
- 2026-10-12  task-008  评审前清障：修正 junction 测试名与断言矛盾（原名为 rejected 实断言 allowed），补 resolve 失败不调 NoopDeleter 反证用例；clippy/test 全绿
- 2026-10-12  task-009  DB 版 ItemPathResolver 落地并单测覆盖
- 2026-10-12  task-009  评审前清障：独立复跑 clippy/fmt/test 全绿（77 条），修正提交表 hash 笔误（test commit a535b6e→47b542f）；无第一档代码修复项
- 2026-10-12  task-010  外部 whitelist_deny 接入 cleanup.dryRun
- 2026-10-12  task-010  评审前清障：修正外部红线集成用例（原用 %USERPROFILE%\Videos 命中的是内置 L3 而非外部红线），改用 %LOCALAPPDATA%\SomeSensitiveCache 并断言 L3SystemPath，证明外部红线独立生效；clippy/test 全绿
- 2026-10-12  task-011  ps1 兼容层从零编写（5 脚本 + NDJSON 协议），M1 前置清理收官
- 2026-10-12  task-011  评审前清障：实跑冒烟测试 5/5 通过（temp=157/win_update=6/prefetch=0/browser=13/dev=33），cargo check exit 0；无第一档修复项
- 2026-10-12  task-012  Rust 侧 ps1 NDJSON 消费器（parser + runner）落地，单测覆盖
- 2026-10-12  task-012  评审前清障：独立复跑 clippy/fmt/test 全绿（88 条，含 ps1 spawn 3/3）；无第一档修复项；Tauri 接线/5脚本编排/scan.cancel 留 task-013
