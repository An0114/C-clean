# PROJECT_STATE.md

> **特别声明**：本文件是项目唯一真实数据源，也是「每次任务结束必更新、必单独提交」的文件。后续所有 Agent 必须遵守：任何任务完成后都要先更新本文件，再单独提交一次（commit message 形如 `docs(state): ... (#task-00X)`），不得与业务代码混在同一个 commit。

## 当前目标
M1 v0.1.0 打包完成 + GitHub Release 发版 workflow 已就位；下一轮进入 tauri updater 自动更新配置（签名密钥对 + update endpoint JSON，task-035）。

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
2026-10-12：task-013 完成——新增 `ScanOrchestrator`（硬编码 5 个 ps1 串跑、失败不中断、全失败才报错）；Tauri `scan_start` 命令真接通：建 scan_session、跑 orchestrator、回写 status='completed'/'failed'、返回真实 session_id。4 条 orchestrator 集成测试全绿；cargo check/clippy/test/fmt + pnpm build 全绿。
2026-10-12：task-014 完成——`cleanup_dry_run` Tauri 命令从 `MockResolver` 切到 `DbItemPathResolver`，打开真实 DB（与 scan_start 同 data_dir），按前端 itemIds 反查 cleanup_item 路径跑 dryRun；保留 external_deny 加载逻辑。cargo 五闸 + pnpm build 全绿。
2026-10-12：task-015 完成——新增 `FsDeleter`（std::fs::remove_file，NotFound 幂等）、`execute_items` 编排（再查 DB + 再过白名单 + 真删 + 落 cleanup_task 批量行与 audit_log 逐条行）、Tauri `cleanup_execute` 命令注册；4 条集成测试全绿（删真文件/L3 拦/缺 id/幂等）。cargo 五闸 + pnpm build 全绿。
2026-10-12：task-016 完成——新增 `SafeBoxDeleter`（rename 到 `<data_dir>/safe_box/`，跨盘 fallback copy+remove）、`enforce_quota` 按 quarantined_at 最旧 FIFO 剔除；`execute_items` 改签名为 deleter 工厂闭包；Tauri `cleanup_execute` 切到 SafeBoxDeleter。3 条集成测试全绿（移入写行/空 noop/quota 剔除）。cargo 五闸 + pnpm build 全绿。
2026-10-12：task-017 完成——新增 `restore_from_safe_box`（按 id 查 in_box 行、拒目标占用/父目录缺失、rename 回原路径、删行、写 audit_log action='restore'）；Tauri `safe_box_restore` 命令注册（TargetOccupied→2003 FILE_OCCUPIED，其他→5001 INTERNAL）。4 条集成测试全绿（正常/占用/父缺/已剔除）。cargo 五闸 + pnpm build 全绿。
2026-10-12：task-018 完成——新增 `list_safe_box`（`status='in_box'` 按 quarantined_at DESC，返回 entries + total_items + total_bytes）；Tauri `safe_box_list` 命令注册。4 条集成测试全绿（空/3条汇总/排除 purged/排除 restored）。cargo 五闸 + pnpm build 全绿。M1-C 保险箱三件套（入/回/列）齐备。
2026-10-12：task-019 完成——新增 `classify_io_error`（按 Windows raw_os_error 分 NotFound/PermissionDenied/SharingViolation/Unknown）；SafeBoxDeleter rename 失败时抛 `FILE_IN_USE:`/`PERMISSION_DENIED:` 前缀错误；executor 把这两个前缀翻译成 audit_log.detail 稳定文案（"file in use by another process" / "permission denied, needs elevation"）。4 条单元测试全绿（NotFound/32/5/Other）。cargo 五闸 + pnpm build 全绿。
2026-10-12：task-020 完成——engine 引入 windows 0.58 crate（cfg(windows)）；新增 `who_is_locking(path) -> Vec<LockingProcess{pid, app_name, service_name}>`，封装 RmStartSession/RmRegisterResources/RmGetList/RmEndSession 四件套；空锁测试全绿，notepad 真机用例 `#[ignore]`。cargo 五闸 + pnpm build 全绿。
2026-10-12：task-021 完成——`ExecResult` 加 `locking_processes: Vec<LockingProcess>` 字段；executor 在 deleter 返回 `FILE_IN_USE:` 前缀时自动调 `who_is_locking(&path)`，把 `"notepad.exe (PID 1234), chrome.exe (PID 5678)"` 拼进 audit_log.detail 并填进结果返回前端；PERMISSION_DENIED 走 task-019 旧文案不调 RM。2 条 CI 集成测试全绿（空锁 + PERMISSION_DENIED 不触发），notepad 端到端 `#[ignore]`。cargo 五闸 + pnpm build 全绿。
2026-10-13：task-022 完成——engine 引入 reqwest 0.12（blocking + rustls-tls）；新建 `rules/meta_store.rs`（upsert/get_active rules_meta 单行表 id=1）与 `rules/remote.rs`（`fetch_remote_meta` GET GitHub Raw rules_meta.json）；Tauri `rules.checkUpdate` 命令注册，返回 `{has_update, current_version, remote_version, current_sha256, remote_sha256}`。3 条集成测试全绿（upsert/get_active/空库）。cargo 五闸 + pnpm build 全绿。
2026-10-13：task-023 完成——`remote.rs` 新增 `fetch_remote_rules_bytes`（GET 远端 cache_rules.json）与 `apply_update`（SHA-256 校验 + 备份 .bak + 替换 cache_rules.json/rules_meta.json + upsert rules_meta 单行表 + 失败自动回滚）；Tauri `rules.applyUpdate` 命令注册。3 条集成测试全绿（正常替换/sha不匹配拒绝/DB失败回滚）。cargo 五闸 + pnpm build 全绿。
2026-10-13：task-024 完成——新建 `audit_log/query.rs`（`list_audit_log`：按 ts DESC, id DESC + limit/offset 分页；对外字段重命名 event_type=action、path=target、created_at=ts）；Tauri `audit_log.list` 命令注册。4 条集成测试全绿（空/倒序/action 过滤/分页）。cargo 五闸 + pnpm build 全绿。
2026-10-13：task-025 完成——新建 `audit_log/stats.rs`（`cleanup_stats`：单条 SQL 一次取 total_bytes_freed / success_count / failed_count / session_count，COALESCE 防空表）；Tauri `audit_log.stats` 命令注册。4 条集成测试全绿（空/计数/session 过滤/去重）。cargo 五闸 + pnpm build 全绿。
2026-10-13：task-026 完成——前端第一刀：新建 `app/src/utils/formatBytes.ts`（B/KB/MB/GB，负数/NaN 兜底 "0 B"）与 `app/src/pages/Dashboard.tsx`（2×2 grid 4 卡片，loading 骨架 / error 红条+重试 / ok 渲染，失败数>0 红色）；`App.tsx` 默认渲染 Dashboard。内联 style，未引 UI 库。补装 `@tauri-apps/api@2.12.3`（任务书假设已在 devDeps 实际未装，本次新增）。cargo 五闸保持基线 133 条全绿 + pnpm build 全绿。
2026-10-13：task-028 完成——Dashboard 顶部加「开始扫描」按钮：点击 → `invoke('scan_start')` + `listen('scan://progress')` 订阅，绿底进度条（percent 0~100）+ 当前路径 + 已扫项数；扫描中按钮禁用，done 后文案变「再次扫描」并自动刷新 stats；失败红条 + 重试。**与任务书偏差**：真实 `ScanProgress` shape 是 `{session_id, percent, current_path, scanned_items}`（不是任务书假设的 `{module, done, total}`），进度条直接用 `percent`，文字用 `current_path` + `scanned_items`。cargo 五闸保持基线 + pnpm build 全绿。
2026-10-13：task-029 完成——清理闭环打通。后端新建 `execute/list_pending.rs`（`list_pending_items`：按 session_id 可选过滤、size_bytes DESC、参数化 limit/offset）；Tauri `cleanup_list_pending` 命令注册。前端新建 `app/src/pages/Items.tsx`（pending 表格：checkbox/路径/大小/模块/session，顶部「已选 N 项共 X」+「清理已选」按钮 → `invoke('cleanup_execute', { itemIds })`，完成后自动重拉 + 绿/红结果条）；`App.tsx` 加第三 tab「清理」。4 条集成测试全绿。**与任务书偏差**：cleanup_item 表无 status/scanned_at 列，「pending」= 表内所有行（无 status 过滤），PendingItem 去掉 scanned_at 字段。cargo 五闸 + pnpm build 全绿。
2026-10-13：task-030 完成——保险箱 UI 落地。新建 `app/src/pages/SafeBox.tsx`：`invoke('safe_box_list')` 拉列表，顶部「刷新」+「共 N 项 / 总大小 formatBytes(totalBytes)」汇总，表格 4 列（原路径/大小/移入时间 formatTs/操作「恢复」按钮），行内「恢复中…」禁用，绿条「已恢复：path」/红条「恢复失败：err」，空列表居中灰字「保险箱为空」；`App.tsx` 加第四 tab「保险箱」，顺序 dashboard→清理→保险箱→历史。**与任务书偏差**：真实 list 响应是 `SafeBoxListResp { entries, totalItems, totalBytes }`（不是裸数组），`SafeBoxEntry { id, originalPath, sizeBytes, quarantinedAt }`（不是任务书假设的 `{id, path, size_bytes, moved_at, original_path}`），restore 入参是 `safeBoxId`（不是 itemId）——前端已按真实 shape 对接。cargo 五闸保持基线 137 条全绿 + pnpm build 156.19KB gzip 49.74KB。
2026-10-13：task-031 完成——规则更新 UI 落地。新建 `app/src/pages/Settings.tsx`：规则库卡片显示当前版本/远端版本/SHA-256（前 16 位）/状态（「已是最新」或「有新版本 X 可用」）；「检查更新」按钮调 `rules_check_update`（启动时自动拉一次），有更新时显示绿色「应用更新」按钮调 `rules_apply_update`，成功绿 toast「已更新到版本 X」+ 自动重拉；应用中按钮禁用 +「应用中…」；网络失败红条。`App.tsx` 加第五 tab「设置」。**与任务书偏差**：CheckUpdateResp/ApplyResult 均 `#[serde(rename_all="camelCase")]`，前端字段用 camelCase（hasUpdate/currentVersion/...），不是任务书假设的 snake_case。cargo 五闸保持基线 137 条全绿 + pnpm build 158.90KB gzip 50.36KB。
2026-10-13：task-032 完成——前端工程化补齐。ESLint 10 flat config（`app/eslint.config.js`，@eslint/js + @typescript-eslint + react-hooks + react-refresh，no-undef 关闭交给 TS，no-unused-vars warn）；Vitest 2.1 + @testing-library/react + jest-dom + jsdom（`vite.config.ts` 加 test.environment=jsdom + setupFiles，`src/setupTests.ts` 引入 `@testing-library/jest-dom/vitest`）；formatBytes 5 个断言 + formatTs 4 个断言共 9 个单测全过；`.github/workflows/ci.yml` 前端 job 加 Lint/Test 两步替换 echo 占位。**与任务书偏差**：ESLint 10 已废弃 .eslintrc.cjs，改用 flat config `eslint.config.js`；Vitest 5.0.3 要求 Vite 6+，本项目 Vite 5.4.11 降级到 vitest@^2.1.9；setupTests 必须用 `@testing-library/jest-dom/vitest`（子路径）而非 `@testing-library/jest-dom`，否则 expect.extend 在框架注册前执行报错。cargo 五闸保持基线 + pnpm lint=0 / pnpm test=0（9/9）/ pnpm build=0。
2026-10-13：task-033 完成——NSIS v0.1.0 打包就绪。`app/src-tauri/tauri.conf.json`：identifier 改为 `com.and0114.cclean`，bundle.targets 从 "all" 收窄为 `["nsis","msi"]`，bundle.windows.nsis 加 `installMode="perMachine"`（清理 C 盘要管理员）+ `languages=["SimpChinese"]`。图标沿用 task-001 生成的 Tauri 默认占位（17 个文件已在库，不重跑 `tauri icon`，设计图标留后续）。`.github/workflows/ci.yml` 在 `pnpm build` 后加 `pnpm --filter app tauri build` + `actions/upload-artifact@v4`（artifact 名 `cclean-windows-installer`，path `target/release/bundle/{nsis/*.exe,msi/*.msi}`，`if-no-files-found: error`）。本地 `pnpm --filter app tauri build` 退出码 0，产出 `target/release/bundle/msi/C-Disk Cleaner_0.1.0_x64_en-US.msi`（5.11 MB）+ `target/release/bundle/nsis/C-Disk Cleaner_0.1.0_x64-setup.exe`（3.60 MB）。**与任务书偏差**：① Tauri 2 NSIS schema 字段是 `languages`（数组）不是任务书写的 `displayLanguage`（标量，build script 直接报 unknown field）；② workspace 模式 bundle 输出在仓库根 `target/release/bundle/` 而非 `app/src-tauri/target/release/bundle/`，CI path 已相应调整；③ 图标未重跑（沿用 task-001 默认占位）。cargo 五闸 + pnpm lint/test/build + tauri build 全绿。
2026-10-13：task-034 完成——GitHub Release 发版 workflow 就位。新建 `.github/workflows/release.yml`：触发器 `workflow_dispatch`（输入 version，如 0.1.0）+ `push.tags: v*`；permissions.contents=write；job windows-latest 跑 checkout → setup node 22/pnpm 11 → setup rust stable → pnpm install --frozen-lockfile → pnpm --filter app tauri build → softprops/action-gh-release@v2 上传 `target/release/bundle/{nsis/*.exe,msi/*.msi}`，Release 名/tag 为 `v${{ inputs.version || github.ref_name }}`，generate_release_notes=true。**与任务书偏差**：bundle path 沿用 task-033 实证的 workspace 根 `target/release/bundle/`（不是任务书假设的 `app/src-tauri/target/release/bundle/`）；首次 workflow_dispatch v0.1.0 手动触发留评审在 GitHub Actions 页执行（自动化环境无 gh auth，无法代点）。

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
- [x] 013：Tauri scan.start 真接通——Orchestrator 串跑 5 ps1 + 真实 session + 回写 status
- [x] 014：cleanup.dryRun 切 DbItemPathResolver（按 id 反查 cleanup_item 真实路径）
- [x] 015：cleanup.execute 真实删除——FsDeleter + execute_items 编排 + Tauri 命令 + audit_log
- [x] 016：safe_box 5GB FIFO 保险箱——删除前移入 box 目录 + 超容按时间最旧永久剔除
- [x] 017：safe_box restore——按 id 把文件从保险箱 rename 回原路径，不自动建父目录、不批量
- [x] 018：safe_box.list——列出当前所有未剔除条目 + 汇总（条目数/总字节）
- [x] 019：删除失败错误分类——classify_io_error 识别 SharingViolation/PermissionDenied，audit_log 写具体原因
- [x] 020：who_is_locking——Windows Restart Manager FFI 查占用进程 PID
- [x] 021：占用进程自动接线——FILE_IN_USE 时 execute_items 自动调 who_is_locking 写 audit_log 并返回前端
- [x] 022：rules_meta 落 DB + rules.checkUpdate——查 GitHub Raw 比对版本（不下载替换）
- [x] 023：rules.applyUpdate——下载新 cache_rules.json + SHA-256 校验 + 备份替换 + upsert DB
- [x] 024：audit_log.list——按 ts 倒序分页只读查询，M1-F 报表后端基础
- [x] 025：audit_log.stats——cleanup_task 聚合（total_bytes / success / fail / sessions）
- [x] 026：前端 Dashboard 4 数字卡片——formatBytes + Dashboard.tsx + App.tsx 挂载（stats 接前端，GB 格式化）
- [x] 027：前端 History 明细页——formatTs + History.tsx 表格（时间/事件/路径/详情）+ 分页 + App.tsx tab 切换（audit_log.list 接前端）
- [x] 028：前端开始扫描按钮——Dashboard 按钮 + scan_start invoke + listen(scan://progress) 实时进度条
- [x] 029：清理闭环——cleanup.listPending 后端命令 + Items 表格（checkbox + 清理已选）+ App 第三 tab
- [x] 030：保险箱 UI——safe_box_list 表格 + 汇总 + 单条「恢复」按钮 + App 第四 tab
- [x] 031：规则更新 UI——Settings 规则库卡片 + 检查更新/应用更新按钮 + App 第五 tab
- [x] 032：前端工程化补齐——ESLint 10 flat config + Vitest 2.1 + formatBytes/formatTs 单测 + CI 接 lint/test
- [x] 033：NSIS v0.1.0 打包——tauri.conf identifier/nsis + CI tauri build/upload-artifact，本地出 .msi/.exe
- [x] 034：GitHub Release workflow——release.yml 支持 workflow_dispatch + tag 触发，softprops/action-gh-release 上传 installer

## 已知问题
- [B 中] task-021 仅做「报告」（who_is_locking 结果写 audit_log + 返回前端），**未实现 RmShutdown/RmRestart（请求关闭占用进程）**，前端 UI toast 也未接；notepad 端到端真机验证 CI 用例 `#[ignore]`。请求关闭与前端展示留 task-022+。
- [B 低] task-018 list 边界：无分页（保险箱总量 ≤5GB，条目数有限）、无 session 过滤（先返回全部）、无单条 purge 命令、未接前端 React UI。真多了再加 LIMIT / 前端列表组件留后续。
- [B 低] task-017 restore 边界：不自动建原父目录（ParentMissing 直接报错让用户手动处理）、不支持批量恢复、未加 `restored_at` 审计列（DELETE 行 + audit_log action='restore' 已留痕）。批量恢复与父目录自动 mkdir 留后续。
- [B 中] task-016 保险箱原子性：`fs::rename` 成功但 `INSERT INTO safe_box` 失败时，文件已在 box 目录但 DB 无行——当前简单报错，不做回滚；跨盘 rename 走 copy+remove 也非原子。补偿：下次 enforce_quota 扫到磁盘上有孤儿文件时会 NotFound 直接标 purged（不会重复占用 quota）。
- [B 低] task-016 跨盘 rename：`fs::rename` 遇 EXDEV(17)/跨盘错误时 fallback 到 copy+remove；CI 单盘未实测跨盘用例。
- [B 中] task-015 文件占用时删除失败：Windows 下 `std::fs::remove_file` 遇 SharingViolation 直接返 Err，不做重试/Restart Manager；命中占用的 item 记为 fail 并写 audit_log result='fail'。Restart Manager 集成留 M1-D 后续任务。
- [B 低] UNC 路径 `\\server\share\...` task-008 评估结论：**沿用保守拒绝**（normalize 层 NotAbsolute + resolve 层 Io 双保险）；网络盘 L3 红线策略留待真实 NAS/企业盘场景出现时再开。
- [B 低] task-007 WebView2 devtools console 的 `listen('scan://progress')` 真机截图未在本轮自动化环境内取得（tauri dev 窗口启动后 GUI 自动化无法稳定聚焦 console）；通道本身已由编译期类型 + engine 节流单测保证，留待 UI 联调任务补截图。
- [B 低] task-005 rules_meta 落 DB + checkUpdate + applyUpdate 均已在 task-022/023 落地；**GitHub Token/私有仓、失败重试/离线降级、前端 UI 「确认更新」弹窗、手动回滚命令（从 .bak 恢复）、差量更新** 仍未做，留后续。
- [B 低] task-005 外部红线命中统一记为 `DenialReason::L3SystemPath`（遵守「whitelist.rs 仅新增函数、不改枚举/常量」约束）；后续审计若需区分内置/外部红线来源，再拆 DenialReason 变体。
- [B 低] task-006 CI 未接 Dependabot / cargo audit / npm audit（任务边界明确不做）；~~package.json 的 lint/test 为 echo 占位，CI 也不跑这两个命令~~（task-032 已接入 ESLint 10 flat config + Vitest 2.1，CI 已加 Lint/Test 两步）。CI 仅 windows-latest 单平台，跨平台矩阵（macOS/Linux）本项目明确不做；~~NSIS 出包矩阵未配~~（task-033 已配 Windows 单平台 nsis+msi，artifact 名 cclean-windows-installer）。
- [B 中] task-033 installer 未做代码签名：未购 EV/OV 证书，用户首次运行 .exe/.msi 会有 SmartScreen「Windows 已保护你的电脑」警告，需要手动「更多信息→仍要运行」；正式发版前需评估代码签名（自签/商业证书）。
- [B 低] task-033 应用图标仍为 Tauri 默认占位（task-001 生成的 17 个文件），未做品牌图标设计；待设计出 1024×1024 源 PNG 后跑 `tauri icon` 重生成全套。

## 下一步计划
v0.1.0 发版 workflow 已就位；下一轮进入 tauri updater 自动更新配置（签名密钥对 + update endpoint JSON，task-035），并行做品牌图标设计与首次 workflow_dispatch v0.1.0 手动触发。

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
| eb46fe9 | #task-013 | feat(scan): ScanOrchestrator 串跑 5 个 ps1 并汇总 |
| be8bd23 | #task-013 | feat(tauri): scan_start 真接通——建 session/跑 Orchestrator/回写 status |
| 8918c8f | #task-013 | test(scan): Orchestrator 集成测试（4 条） |
| （本次） | #task-013 | docs(state): 任务013完成，scan.start 端到端跑通真实扫描 |
| 0c32251 | #task-014 | feat(tauri): cleanup_dry_run 切 DbItemPathResolver，删 MockResolver |
| （本次） | #task-014 | docs(state): 任务014完成，dryRun 走真实 DB 数据 |
| aa5abd7 | #task-015 | feat(execute): 新增 FsDeleter——std::fs::remove_file 真实删除 |
| 319d55e | #task-015 | feat(execute): 新增 execute_items 编排——过白名单/删文件/落审计 |
| 125c385 | #task-015 | feat(tauri): 新增 cleanup.execute 命令并注册 |
| 897b0c1 | #task-015 | test(execute): execute_items 集成测试 4 条 |
| （本次） | #task-015 | docs(state): 任务015完成，真实删除闭环 |
| 58dd794 | #task-016 | feat(execute): 新增 SafeBoxDeleter——删除前移入保险箱并写 safe_box 表 |
| 6a3892e | #task-016 | refactor(execute): execute_items 改 deleter 工厂闭包签名 |
| 86e5fdf | #task-016 | feat(tauri): cleanup_execute 切 SafeBoxDeleter |
| cdb89f6 | #task-016 | test(execute): safe_box 移入/quota 剔除/空 noop 集成测试 |
| 82e320c | #task-016 | fix(execute): 去掉 MSRV 1.85 CrossesDevices variant |
| （本次） | #task-016 | docs(state): 任务016完成，5GB FIFO 保险箱落地 |
| 9fb97ea | #task-017 | feat(execute): 新增 restore_from_safe_box——按 id 把文件 rename 回原路径 |
| 26d19d1 | #task-017 | feat(tauri): 新增 safe_box.restore 命令并注册 |
| 6372f6c | #task-017 | test(execute): 补 restore 集成测试（正常/占用/父缺/已剔除） |
| （本次） | #task-017 | docs(state): 任务017完成，保险箱 restore 闭环 |
| abb43dc | #task-018 | feat(execute): 新增 list_safe_box——查未剔除条目 + 汇总 |
| 5827efe | #task-018 | feat(tauri): 新增 safe_box.list 命令并注册 |
| 774e1dd | #task-018 | test(execute): 补 list 集成测试（空/多条/排除 purged/排除 restored） |
| （本次） | #task-018 | docs(state): 任务018完成，safe_box.list 就绪 |
| 9fc9564 | #task-019 | feat(execute): 新增 classify_io_error——区分 NotFound/PermissionDenied/SharingViolation/Unknown |
| a2c75a5 | #task-019 | feat(execute): SafeBoxDeleter 抛错时按分类加 FILE_IN_USE/PERMISSION_DENIED 前缀 |
| 257ba68 | #task-019 | feat(execute): execute_items 写 audit_log 时按错误分类填 detail |
| ee14fd0 | #task-019 | test(execute): 补 classify_io_error 单元测试 |
| （本次） | #task-019 | docs(state): 任务019完成，删除失败错误分类就绪 |
| a3f3a3d | #task-020 | build(deps): engine 引入 windows crate（cfg(windows) RestartManager feature） |
| 8445d68 | #task-020 | feat(execute): 新增 who_is_locking——Restart Manager FFI 查占用进程 PID |
| 7910e38 | #task-020 | test(execute): 补 who_is_locking 单元测试（空锁 + notepad 手动用例） |
| 47590f9 | #task-020 | style(execute): cargo fmt 对齐 locking 模块与测试 |
| （本次） | #task-020 | docs(state): 任务020完成，who_is_locking 就绪 |
| a4dbee4 | #task-021 | feat(execute): ExecResult 加 locking_processes 字段，FILE_IN_USE 时自动调 who_is_locking |
| 841bf87 | #task-021 | test(execute): 补 execute_items 占用进程集成测试 |
| （本次） | #task-021 | docs(state): 任务021完成，占用进程自动报告 |
| 53c4c92 | #task-022 | build(deps): engine 引入 reqwest（blocking + rustls-tls） |
| 08d4d23 | #task-022 | feat(rules): 新增 meta_store——upsert/get_active rules_meta 落 DB |
| fb849e7 | #task-022 | feat(rules): 新增 fetch_remote_meta——GET GitHub Raw rules_meta.json |
| 7f0c1bf | #task-022 | feat(tauri): 新增 rules.checkUpdate 命令（比对本地/远端 sha256） |
| 757f5c3 | #task-022 | test(rules): 补 meta_store 集成测试 |
| （本次） | #task-022 | docs(state): 任务022完成，rules.checkUpdate 就绪 |
| 4add0d7 | #task-023 | docs(task): 新增任务023-rules.applyUpdate |
| 1db76ec | #task-023 | feat(rules): 新增 fetch_remote_rules_bytes——GET 远端 cache_rules.json |
| 5691f59 | #task-023 | feat(rules): 新增 apply_update——SHA-256 校验 + 备份 + 替换 + upsert DB + 失败回滚 |
| 49f784a | #task-023 | feat(tauri): 新增 rules.applyUpdate 命令并注册 |
| 0bc8bfb | #task-023 | test(rules): 补 apply_update 集成测试（替换/sha不匹配回滚） |
| （本次） | #task-023 | docs(state): 任务023完成，规则更新闭环 |
| 0d00d42 | #task-024 | docs(task): 新增任务024-audit_log.list |
| 79677da | #task-024 | feat(audit_log): 新增 list_audit_log——按 session 过滤 + 分页 + 时间倒序 |
| 701caf8 | #task-024 | feat(tauri): 新增 audit_log.list 命令并注册 |
| dd24e78 | #task-024 | test(audit_log): 补 list 集成测试（空/倒序/action过滤/分页） |
| （本次） | #task-024 | docs(state): 任务024完成，audit_log.list 就绪 |
| 8fa4546 | #task-025 | docs(task): 新增任务025-audit_log.stats |
| 9f0c0c5 | #task-025 | feat(audit_log): 新增 cleanup_stats——总释放字节 / 成功数 / 失败数 / session 数 |
| 7052e14 | #task-025 | feat(tauri): 新增 audit_log.stats 命令并注册 |
| d6a9fa1 | #task-025 | test(audit_log): 补 stats 集成测试（空/计数/session过滤/去重） |
| b741579 | #task-025 | docs(state): 任务025完成，清理统计数字就绪 |
| 0516bee | #task-025 | docs(task): 回填任务025实现备注 |
| 1548320 | #task-026 | docs(task): 新增任务026-前端仪表盘数字卡片 |
| 40f5202 | #task-026 | feat(frontend): 新增 formatBytes 工具函数 |
| 37219cd | #task-026 | feat(frontend): 新增 Dashboard 页面——audit_log.stats 4 数字卡片 |
| 17c8e33 | #task-026 | feat(frontend): App.tsx 挂载 Dashboard 为默认页（补装 @tauri-apps/api@2.12.3） |
| 4706afc | #task-026 | docs(state): 任务026完成，前端仪表盘数字卡片就绪 |
| baa0b63 | #task-027 | merge(feature/task-027): History 历史明细页——audit_log.list 表格 + 分页，App.tsx tab 切换 |
| c935889 | #task-028 | docs(task): 新增任务028-前端开始扫描按钮 |
| 90eec5c | #task-028 | feat(frontend): Dashboard 加开始扫描按钮 + scan_start invoke + listen(scan://progress) 实时进度条 |
| 783b45b | #task-028 | docs(state): 任务028完成，前端可发起扫描 |
| 5979f14 | #task-029 | docs(task): 新增任务029-清理闭环Items UI |
| 48ba875 | #task-029 | feat(execute): 新增 list_pending_items——按 session 过滤 pending 项，size 倒序 |
| 976acdb | #task-029 | feat(tauri): 新增 cleanup.listPending 命令并注册 |
| e223fc6 | #task-029 | test(execute): 补 list_pending_items 集成测试（列出全部/session过滤/size倒序/分页） |
| a4be607 | #task-029 | feat(frontend): 新增 Items 页面——pending 表格 + checkbox + 清理已选按钮 |
| 94fb43d | #task-029 | feat(frontend): App.tsx 加「清理」tab 切换 |
| ef07dea | #task-029 | style(execute): clippy 修复——limit 用 clamp 替代 max/min |
| ca81b00 | #task-029 | docs(state): 任务029完成，清理闭环打通 |
| 7bb625e | #task-030 | docs(task): 新增任务030-保险箱恢复 UI |
| 1f24104 | #task-030 | feat(frontend): 新增 SafeBox 页面——safe_box_list 表格 + 汇总 + 单条恢复按钮 |
| 317258c | #task-030 | feat(frontend): App.tsx 加「保险箱」tab 切换 |
| c7c1882 | #task-030 | docs(state): 任务030完成，保险箱恢复 UI 就绪 |
| c58ce28 | #task-031 | docs(task): 新增任务031-规则更新 UI |
| a46ec34 | #task-031 | feat(frontend): 新增 Settings 页面——规则库卡片 + 检查更新 + 应用更新按钮 |
| 1c3d33c | #task-031 | feat(frontend): App.tsx 加「设置」tab 切换 |
| dbe7e44 | #task-031 | docs(state): 任务031完成，规则更新 UI 就绪 |
| 3ec1065 | #task-032 | docs(task): 新增任务032-ESLint+Vitest 接入 |
| 239eef0 | #task-032 | chore(frontend): 安装 ESLint 10 flat config + TS 插件 |
| ddfe37f | #task-032 | chore(frontend): 安装 Vitest 2.1 + testing-library + jsdom + vite.config test 字段 |
| 8557c4d | #task-032 | test(frontend): 给 formatBytes / formatTs 写纯函数单测（9 个断言） |
| 996a803 | #task-032 | ci: 前端 job 接 pnpm lint + pnpm test，替换 echo 占位 |
| 173eac6 | #task-032 | docs(state): 任务032完成，ESLint+Vitest 接入 |
| 3a255d1 | #task-033 | docs(task): 新增任务033-NSIS v0.1.0 打包 |
| f0a46ec | #task-033 | chore(tauri): tauri.conf.json 完善 identifier/nsis perMachine/SimpChinese |
| 0d35fa0 | #task-033 | ci: 加 tauri build + upload-artifact 出 v0.1.0 installer |
| 96be58d | #task-033 | docs(state): 任务033完成，NSIS v0.1.0 打包就绪 |
| b360a9e | #task-034 | docs(task): 新增任务034-GitHub Release 发版通道 |
| c7a276d | #task-034 | ci: 新增 release.yml——workflow_dispatch 手动发版 + tag 自动发版 |
| （本次） | #task-034 | docs(state): 任务034完成，GitHub Release 通道就绪 |

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
- 2026-10-12  task-013  scan.start 端到端接线：5 ps1 串跑 + 真实 session + 前端 listen
- 2026-10-12  task-013  评审前清障：独立复跑 clippy/fmt/test 全绿（92 条，含 orchestrator 4/4）；修正 scan_start doc 注释（同步执行+last_insert_rowid，非 spawn_blocking/RETURNING）；devtools console 截图仍留 UI 联调
- 2026-10-12  task-014  cleanup.dryRun 切换 DbItemPathResolver，dryRun 闭环真实数据
- 2026-10-12  task-014  评审前清障：独立复跑 clippy/fmt/test 全绿（92 条不回退）；确认 MockResolver import 已删、DbItemPathResolver 接线薄包装无偏差；无第一档修复项
- 2026-10-12  task-015  cleanup.execute 真实删除（FsDeleter + delete::execute + audit_log）
- 2026-10-12  task-015  评审前清障：executor 补 resolve_to_real——junction/symlink 重定向须对 canonicalize 后真实路径做 L3 判定（原只对输入路径分类，junction 可绕过）；NotExists 回退输入路径分类兼容幂等重删；clippy/test 96 条全绿
- 2026-10-12  task-016  safe_box 5GB FIFO 保险箱落地，删除不再直接 remove_file
- 2026-10-12  task-016  评审前清障：safe_box 测试加全局串行锁——QUOTA_BYTES 是进程级 AtomicU64，并行测试互相覆盖导致 evicts_oldest 偶发失败；clippy/test 99 条全绿
- 2026-10-12  task-017  safe_box restore 闭环，用户后悔可恢复文件
- 2026-10-12  task-017  评审前清障：独立复跑 clippy/fmt/test 全绿（104 条，含 restore 5/5）；确认 restore 逻辑正确（TargetOccupied/ParentMissing/rename/delete row/audit_log），无第一档修复项
- 2026-10-12  task-018  safe_box.list 命令就绪，前端 UI 可列出保险箱条目
- 2026-10-12  task-018  评审前清障：独立复跑 clippy/fmt/test 全绿（108 条，含 list 4/4）；确认 list 查询正确（status='in_box' 过滤、quarantined_at DESC、size_bytes i64→u64），无第一档修复项
- 2026-10-12  task-019  删除失败错误分类（FILE_IN_USE/PERMISSION_DENIED）落 audit_log
- 2026-10-12  task-019  评审前清障：独立复跑 clippy/fmt/test 全绿（116 条，含 classify 4/4 单元 + 4/4 集成）；确认 SafeBoxDeleter rename 失败分类前缀和 executor audit_log detail 翻译逻辑正确；notepad 真机联调未跑（第三档已登记）
- 2026-10-12  task-020  who_is_locking——Restart Manager FFI 查占用进程 PID
- 2026-10-12  task-021  FILE_IN_USE 自动查占用进程写 audit_log 并返回前端
- 2026-10-13  task-022  rules_meta 落 DB + rules.checkUpdate 查远端版本（不下载）
- 2026-10-13  task-023  rules.applyUpdate——下载新规则 + SHA-256 校验 + 备份替换 + upsert DB
- 2026-10-13  task-024  audit_log.list 只读查询命令落地，历史页后端基础
- 2026-10-13  task-025  cleanup_stats 聚合查询（total_bytes / success / fail / sessions）
- 2026-10-13  task-026  Dashboard 4 数字卡片（stats 接前端，GB 格式化）
- 2026-10-13  task-027  History 历史明细页（audit_log.list 表格 + 分页 + tab 切换）
- 2026-10-13  task-027  评审前清障：独立复跑 clippy/fmt/test/pnpm build 全绿（133 条测试无回退，vite build 148KB gzip 47.7KB）；确认 formatTs 非法输入兜底"-"、History loading/ok/error 三态+分页（offset 累加、canNext 按 rows.length===PAGE_SIZE 判定）、App tab 切换 useState 模式正确；tauri dev 真机切 tab 未跑（第三档已登记）
- 2026-10-13  task-026  评审前清障：独立复跑 clippy/fmt/test/pnpm build 全绿（133 条测试无回退，vite build 145KB gzip 46.8KB）；确认 formatBytes 四档 1024 进制+负数/NaN 兜底、Dashboard loading/ok/error 三态状态机、失败数>0 红色高亮、useCallback 重试逻辑正确；tauri dev 真机截图未跑（第三档已登记）
- 2026-10-13  task-025  评审前清障：独立复跑 clippy/fmt/test 全绿（133 条，含 audit_log_stats 4/4）；确认单条 SQL COALESCE 聚合、session_id 参数化过滤、cleanup_task 单行字段求和逻辑正确
- 2026-10-13  task-024  评审前清障：独立复跑 clippy/fmt/test 全绿（129 条，含 audit_log_query 4/4）；确认 ts DESC/id DESC 分页、limit/offset 防御、action→event_type/target→path/ts→created_at 列名映射正确；session_id 过滤用户拍板方案二——入参保留标记"预留未启用"，待后续 schema 加列后启用
- 2026-10-13  task-023  评审前清障：独立复跑 clippy/fmt/test 全绿（125 条，含 apply_update 3/3）；确认 SHA-256 校验/备份/替换/upsert/回滚全链路正确；真实 GitHub Raw 下载联调未跑（第三档已登记）
- 2026-10-13  task-022  评审前清障：独立复跑 clippy/fmt/test 全绿（122 条，含 rules_meta_store 3/3）；确认 upsert/get_active 单行表逻辑正确、fetch_remote_meta reqwest blocking 实现正确；真实 GitHub Raw 网络联调未跑（第三档已登记）
- 2026-10-12  task-021  评审前清障：独立复跑 clippy/fmt/test 全绿（119 条，含 execute_locking 2/2 必跑 + 1 ignored）；确认 FILE_IN_USE 分支自动调 who_is_locking 拼 detail、PERMISSION_DENIED 分支不触发；notepad 端到端 #[ignore] 未跑（第三档已登记）
- 2026-10-12  task-020  评审前清障：独立复跑 clippy/fmt/test 全绿（117 条，含 who_is_locking 1/1 必跑 + 1 ignored）；确认 RmStartSession/RegisterResources/GetList/EndSession 四件套 FFI 逻辑正确；notepad 端到端 #[ignore] 未跑（第三档已登记）
- 2026-10-13  task-028  前端 scan_start 按钮 + scan://progress 实时进度条（percent/current_path/scanned_items）
- 2026-10-13  task-028  评审前清障：独立复跑 clippy/fmt/test/pnpm build 全绿（133 条测试无回退，vite build 150.6KB gzip 48.5KB）；确认 listen 订阅/finally unlisten 配对、percent 直接用 0~100 不除零、按钮四态文案（扫描中/再次扫描/重试扫描/开始扫描）、scanning 时 disabled 防重复点击、done 后自动 load() 刷新 stats 正确；ProgressEvent shape 按 engine 真实字段对齐（session_id/percent/current_path/scanned_items，非任务书假设的 module/done/total）；tauri dev 真机点按钮看进度条走动未跑（第三档已登记）
- 2026-10-13  task-029  cleanup.listPending + Items 表格 + checkbox 清理按钮，M1 业务闭环
- 2026-10-13  task-030  SafeBox 表格 + 单条恢复按钮，M1 安全网闭环
- 2026-10-13  task-031  Settings 规则库卡片 + 检查更新/应用更新按钮，M1-E 前后端打通
- 2026-10-13  task-031  评审前清障：独立复跑 clippy/fmt/test/pnpm build 全绿（137 条测试无回退，vite build 158.9KB gzip 50.4KB）；确认 CheckUpdateResp camelCase 字段（hasUpdate/currentVersion/remoteVersion/currentSha256/remoteSha256）映射正确、ApplyResult newVersion/newSha256 消费正确、应用后自动 check() 重拉刷新版本号、hasUpdate=true 时才显示「应用更新」按钮、检查中/应用中按钮禁用正确；SHA-256 slice(0,16) 截断显示；App.tsx 五 tab（仪表盘/清理/保险箱/历史/设置）useState 切换；tauri dev 真机端到端（切设置→检查更新→应用更新）未跑（第三档已登记）
- 2026-10-13  task-030  评审前清障：独立复跑 clippy/fmt/test/pnpm build 全绿（137 条测试无回退，vite build 156.2KB gzip 49.7KB）；确认 safe_box_list 返回 {entries,totalItems,totalBytes} 响应结构正确消费、SafeBoxEntry camelCase 字段（originalPath/sizeBytes/quarantinedAt）映射正确、restore 入参 safeBoxId 驼峰对应 Rust safe_box_id、restoringId 单行恢复中禁用/finally 复位、空列表「保险箱为空」占位正确；App.tsx 四 tab（仪表盘/清理/保险箱/历史）useState 切换；tauri dev 真机端到端（删→进保险箱→恢复）未跑（第三档已登记）
- 2026-10-13  task-029  评审前清障：独立复跑 clippy/fmt/test/pnpm build 全绿（137 条测试，含 list_pending 4/4）；确认 list_pending_items session_id 可选过滤/limit.clamp(1,500)/size_bytes DESC/参数化正确、Items 页面 Set<number> 选中态/全选 checkbox/清理已选按钮 disabled 条件/cleanup_execute 入参 itemIds 驼峰映射正确；cleanup_item 表无 status/scanned_at 列按真实 schema 适配（pending=表内所有行，清理后不删行）；tauri dev 真机端到端（扫→勾→清→数字涨）未跑（第三档已登记）
- 2026-10-13  task-032  ESLint+Vitest 接入，CI 跑 lint/test，task-006 echo 占位清零
- 2026-10-13  task-033  NSIS v0.1.0 配置 + 图标 + CI 出 installer artifact，M1 可分发
- 2026-10-13  task-033  评审前清障：独立复跑 clippy/fmt/test/pnpm lint/pnpm test/pnpm build 全绿（137 条后端测试无回退，前端 9 断言全过）；确认 tauri.conf.json productName/version/identifier/bundle.targets=["nsis","msi"]/windows.nsis.installMode=perMachine+languages=["SimpChinese"] 字段正确、CI yml 加 Tauri build + Upload installer artifact 两步正确（path 指向 workspace 根 target/release/bundle/，if-no-files-found: error）；Tauri 2 NSIS schema 用 languages 数组（非任务书 displayLanguage 标量）按真实 schema 适配；本地 tauri build 出 .msi 5.11MB + .exe 3.60MB；CI artifact 上传未跑（第三档已登记，需 PR 触发）；代码签名/updater/设计图标/macOS-Linux/真机安装验证留 task-034+（第三档已登记）
- 2026-10-13  task-034  GitHub Release workflow_dispatch + tag 发版通道，v0.1.0 首次发版待手动触发
- 2026-10-13  task-032  评审前清障：独立复跑 clippy/fmt/test/pnpm lint/pnpm test/pnpm build 全绿（137 条后端测试无回退，前端 9 断言全过：formatBytes 5 + formatTs 4）；确认 ESLint flat config（eslint.config.js）TS 文件 no-undef 关闭交给类型检查、react-hooks rules-of-hooks error/exhaustive-deps warn、Vitest jsdom 环境+setupFiles 配置正确、setupTests.ts 用 @testing-library/jest-dom/vitest 子路径导入、CI yml 加 Lint/Test 两步替换 echo 占位正确；Vitest 降级 2.1.9 兼容 Vite 5（Vitest 5 需 Vite 6+）；组件单测留 task-033+（第三档已登记）
