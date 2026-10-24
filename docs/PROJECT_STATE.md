# PROJECT_STATE.md

> **特别声明**：本文件是项目唯一真实数据源，也是「每次任务结束必更新、必单独提交」的文件。后续所有 Agent 必须遵守：任何任务完成后都要先更新本文件，再单独提交一次（commit message 形如 `docs(state): ... (#task-00X)`），不得与业务代码混在同一个 commit。

## 当前目标
**M2 updater 正式关闭（2026-10-23）**：task-048 真机升级验证通过——已装 v0.1.0 的机器启动应用，顶部 banner 正确弹出「发现新版本 v0.1.1」+ 设置页应用更新卡片同步显示「发现新版本 0.1.1 / 新版本 0.1.1」，updater 从 check → 展示 → 引导下载安装全链路真机闭环。叠加 bugfix/task-035-release-signing 的签名/版本号修复（真实发版 run `39942401586` 产出 `.sig` + `latest.json`，pubkey keyid 一致），M2 里程碑收官。

下一阶段焦点：**task-049 测试完成，状态 awaiting_review（待评审）**——占用进程从「只报告」升级为「可处置」（RmShutdown + 危险进程护栏 + 前端一键关闭并重试）。测试期修掉 1 个前端三态互斥缺陷（C12）+ 补 1 条隔离性真机用例。下一项：评审 task-049 → 派发 task-050（safe_box 时间范围筛选 / 多选 session / audit_log 联表）。品牌图标决策：暂时不做，沿用 Tauri 默认占位。

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
2026-10-13：debt-cleanup-002 完成——M1 NSIS 安装包装干净目录后用户实测暴露的 5 个回归 bug 修复（feature/fix-m1-packaging-bugs 分支，等评审不自行 merge）：
  - Bug-1（高）：前端 5 页面把 `Envelope<T>` 当 T 直接用，Items/History `rows.filter/map` 抛 TypeError 整树白屏、Dashboard 数字卡片空、Settings 版本 "-"。新建 `app/src/utils/invoke.ts` 的 `api<T>(cmd,args)` 解包信封（code!==0 抛错，data 直返），5 页面全切过去并删 `|| []`/`|| 0` 兜底；App.tsx 加 class ErrorBoundary（渲染错显示「页面出错，点这里回仪表盘」，不再整树白屏）。
  - Bug-2（高）：`app/src-tauri/capabilities/` 目录不存在，Tauri 2 默认拒插件 API，Dashboard `listen("scan://progress")` 弹 `plugin:event|listen not allowed by ACL`。新建 `capabilities/default.json` 授权 `core:default` + `core:event:default`（allow-listen/unlisten/emit/emit-to）。
  - Bug-3（中，发版阻塞）：release.yml `with: version: 11` 与 root package.json `packageManager: pnpm@11.9.0` 双声明，pnpm/action-setup@v4 报 Multiple versions。删 `with: version: 11` 整行，版本单一来源交给 packageManager。
  - Bug-4（中，CI 红）：`engine/src/execute/resolve.rs` 用 canonicalize 后字符串相等判 hop，windows-latest runner TEMP 走 8.3 短名 `RUNNER~1`，canonicalize 解析成长名 `runneradmin`，字符串不等误判 hop=1。改为逐级 `symlink_metadata` 查 `FILE_ATTRIBUTE_REPARSE_POINT`(0x400)，真正 junction/symlink 才计跳；8.3 短名不是 reparse point，不误计。
  - Bug-5（低）：Dashboard `startScan` catch 里补 `setProgress(null)`，失败后「重试扫描」按钮旁不再挂残留进度条。
  自验：cargo clippy --workspace --all-targets -- -D warnings 干净；cargo test -p engine 137 passed / 0 failed（与基线一致）；cargo fmt --check 过；pnpm --filter app build 160.40 kB / gzip 51.40 kB。
2026-10-14：debt-cleanup-003 完成——debt-cleanup-002 合并后 NSIS 干净目录真机验收暴露的 2 个 P0 阻塞 bug 修复（feature/fix-m1-postinstall-v2 分支，等评审不自行 merge）：
  - Bug-1（P0，M1 核心功能不可用）：Dashboard 调 `api("scan_start", {})` 空对象，但 Rust 签名 `scope: String` 必填，Tauri 直接拒参报 `missing required key scope`，点「开始扫描」立即红条、进度条从未出现。改为传 `{ scope: "full", targetDrive: null }`（Tauri 2 camelCase→snake_case：targetDrive→target_drive）。全仓 9 个 api() 调用点逐一核对后端签名，仅此一处错位。
  - Bug-2（P0，干净安装后所有命令 EACCES）：所有命令用 `std::env::current_dir()` 定位 rules/ 和 DB，perMachine 装到 `C:\Program Files\C-Disk Cleaner\` 后 current_dir 指向只读安装目录，SQLite 报 `拒绝访问 (os error 5)`，设置页规则库「内部错误」。新建 `app/src-tauri/src/paths.rs` 的 `AppPaths`（setup 时构造一次并 manage）：data_root = `app.path().app_data_dir()`（%APPDATA%\com.and0114.cclean\，可写）；rules_dir = data_root/rules（首次从 bundled resource 复制）；scripts_dir = 只读 resource_dir；box_dir = data_root/safe_box。commands.rs 全部改从 `tauri::State<'_, AppPaths>` 取路径，不再碰 current_dir。tauri.conf.json `bundle.resources` 打包 2 个 rules 文件 + 5 个 ps1 脚本。
  - 评审报告另发现 Bug-3（设置页所有失败坍缩成「内部错误」，不区分网络/校验/后端错，P1）和 Bug-4（Dashboard 成功/失败/轮次三个数字卡片空白，总释放空间显示 0 B，P1），留 debt-cleanup-004。
  自验：cargo clippy --workspace --all-targets -- -D warnings 干净；cargo test -p engine 137 passed / 0 failed（与基线一致）；cargo fmt --check 过；pnpm --filter app build 通过。
2026-10-14：debt-cleanup-004 完成——debt-cleanup-003 安装包干净目录真机二轮评审暴露的 3 个回归 bug 修复（同分支 feature/fix-m1-postinstall-v2 追加提交，等评审不自行 merge）：
  - Bug-1（中，Dashboard 数字卡片空白）：后端 `CleanupStats` 有 `#[serde(rename_all = "camelCase")]`，JSON 输出 `totalBytesFreed/successCount/failedCount/sessionCount`；但 Dashboard.tsx interface 是 snake_case，三个 count 字段全 undefined 渲染空白。改为 camelCase。顺带修 History.tsx `AuditLogEntry` 同样问题（sessionId/eventType/createdAt）。
  - Bug-2（中，进度条文字卡死「准备中...」）：`ScanProgress` 事件 payload 也是 camelCase（currentPath/scannedItems），前端 listener 读 snake_case 字段，两个字段全 undefined 永远走 fallback。改 Dashboard.tsx ScanProgress interface 为 camelCase。注：后端 runner 目前 percent 统一发 0（单脚本内不知总量），进度条宽度不动是已知设计限制，留后续任务。
  - Bug-3（中，设置页检查更新全报「内部错误」）：rules_check_update / rules_apply_update 把所有失败（含网络不通、SHA 校验失败）都返回 `Envelope::err(INTERNAL)`。改为 fetch_remote_* 失败 → `RULE_NETWORK_ERROR`（3003），ApplyError::ShaMismatch → `RULE_CHECKSUM_MISMATCH`（3002）；load_rules/DB 错仍 INTERNAL。
  自验：cargo clippy --workspace --all-targets -- -D warnings 干净；cargo test -p engine 137 passed / 0 failed（与基线一致）；cargo fmt --check 过；pnpm --filter app build 通过。
2026-10-14：task-035 完成——tauri updater 自动更新最小闭环：
  - Rust 侧 `Cargo.toml` 加 `tauri-plugin-updater = "2"` + `serde_json = "1"`（generate_context 解析 plugins.updater 配置需要），`lib.rs` Builder 链上 `.plugin(tauri_plugin_updater::Builder::new().build())`。
  - `tauri.conf.json` 加 `plugins.updater`（active/endpoints/pubkey）——注意 Tauri 2 schema 是 `plugins.updater` 命名空间，不是任务书写的顶层 `updater`，也不是 `bundle.updater`。
  - 前端 `pnpm add @tauri-apps/plugin-updater @tauri-apps/plugin-process`，Settings.tsx 在规则库卡片下方加「应用更新」卡片：checkApplication 调 `check()`，downloadAndInstall 调 `update.downloadAndInstall()` + `relaunch()`。
  - `release.yml` 加 Setup signing key（从 secret `TAURI_SIGNING_PRIVATE_KEY` 写 `~/.tauri/cclean.key`）+ Generate latest.json（PowerShell 读 `.msi.sig` 拼 JSON）+ 随 Release 上传；根目录 `latest.json` 占位（raw URL 可达）。
  - 签名密钥对已本地生成在 `~/.tauri/cclean.key`（私钥不入库），公钥已写入 tauri.conf。
  自验：cargo check/clippy/fmt/test（137 条）/pnpm lint/test/build 全绿。**与任务书偏差**：① updater 配置位置按 Tauri 2 真实 schema 落在 `plugins.updater`（任务书写顶层 `updater`，build script 报 unknown field）；② generate_context 需要显式 `serde_json` 依赖否则 E0433；③ release.yml 用 `TAURI_SIGNING_PRIVATE_KEY_PATH` 环境变量指向私钥文件路径（不是把私钥本身当字符串）。
2026-10-14：bugfix/task-035-updater-acl 完成——task-035 评审 P0「updater ACL 拒绝」修复（bugfix 分支，等评审不自行 merge）：
  - Bug-1（P0，自动更新完全不可用）：`capabilities/default.json` 只授权 `core:default` + `core:event:default`，前端 Settings 点「检查应用更新」调 `plugin-updater|check` 被 Tauri ACL 直接拒，报 `Command plugin:updater|check not allowed by ACL`。补 `updater:default`（聚合 allow-check/download/install/download-and-install）+ `process:default`。
  - 顺带补接线缺陷：前端已 `pnpm add @tauri-apps/plugin-process` 并在 downloadAndInstall 后调 `relaunch()`，但 Rust 侧从未注册 `tauri-plugin-process`（Cargo.toml/Cargo.lock/Builder 全缺），relaunch 无后端。补 `tauri-plugin-process = "2"` + `lib.rs` 挂 `.plugin(tauri_plugin_process::init())`。
  - **与评审建议偏差**：评审建议加 `process:allow-relaunch`，但本版 tauri-plugin-process v2 的 ACL manifest 只有 `allow-exit`/`allow-restart`（无 allow-relaunch）；JS `relaunch()` 实际映射到 `restart` 命令，故用 `process:default`（含 allow-restart）覆盖。照抄 `process:allow-relaunch` 会触发 unknown-permission 构建错误。
  自验：cargo fmt --check 干净；clippy --workspace --all-targets -D warnings 无告警；cargo test -p engine 全绿（基线 137 无回退）；pnpm lint 干净；pnpm test 12/12（新增 capabilities 回归 3 断言）；pnpm build 164.56 kB / gzip 52.50 kB。
2026-10-14：task-036 完成——启动自动更新 banner：
  - `app/src/App.tsx` 加 `useEffect` 启动自动 `check()`：有更新才 setUpdate，无更新/网络失败静默；
  - 顶部非阻塞 banner（蓝底）：「发现新版本 vX.Y.Z」+ update.body + 「立即更新」（绿）+「稍后」（灰）；
  - 「立即更新」走 `update.downloadAndInstall()` + `relaunch()`，banner 变「下载安装中…」按钮禁用；失败变红条「更新失败：<err>」；
  - 「稍后」setDismissed(true)，本次启动不再弹。
  自验：cargo check=0（后端无改动）；pnpm lint=0 / test=0 / build=0（165.78 kB / gzip 52.79 kB）。**与任务书偏差**：任务书要求拆 3 个 feat 提交（check / banner / downloadAndInstall），但 banner UI 与 onUpdateNow handler 紧耦合在同一块 JSX 里，拆开会出现中间态「按钮无 onClick」无法编译，故合并为 2 个 feat 提交（check / banner+install）。
2026-10-14：task-037 完成——safe_box purge 单条永久删除：
  - `engine/src/execute/safe_box.rs` 新增 `purge_item(conn, box_dir, item_id)`：查行 → 删磁盘文件（NotFound 吞）→ DELETE DB 行 → 写 audit_log action='purge'；
  - Tauri 命令 `safe_box_purge(item_id)` 接 `commands.rs`，注册到 `lib.rs`；
  - 3 条集成测试（删文件+DB 行 / 缺文件仍删行 / 未找到返 Err）全绿；
  - 前端 `SafeBox.tsx` 操作列加红色「永久删除」按钮，`window.confirm` 兜底，成功重拉列表，失败红条。
  自验：cargo check/clippy/fmt/test=0（engine 140 条）；pnpm lint/test/build=0（166.43 kB / gzip 52.97 kB）。
2026-10-14：task-038 完成——safe_box 批量 purge：
  - `engine/src/execute/safe_box.rs` 新增 `purge_many` + `PurgeManyResult{deleted,failed,errors}`：循环调 `purge_item`，单个失败不中断，收集 `id=<id>: <err>`；
  - Tauri 命令 `safe_box_purge_many(item_ids: Vec<i64>)` 接 `commands.rs`，注册到 `lib.rs`；
  - 3 条集成测试（全成功 / 部分失败 / 空 Vec）全绿；
  - 前端 `SafeBox.tsx` 加 checkbox 列（行内 + 表头全选）+ 顶部「批量删除已选(N)」红色按钮，confirm 后调 `purge_many`，绿/红条显示成功/失败数，清空 selected 并重拉列表。
  自验：cargo check/clippy/fmt/test=0（engine 143 条）；pnpm lint/test/build=0（167.71 kB / gzip 53.34 kB）。
2026-10-14：task-039 完成——safe_box 批量恢复：
  - `engine/src/execute/safe_box.rs` 新增 `BatchOpResult{succeeded,failed,errors}` + `restore_many`：循环调 `restore_from_safe_box`，单个失败不中断；
  - Tauri 命令 `safe_box_restore_many(item_ids: Vec<i64>)` 接 `commands.rs`，注册到 `lib.rs`；
  - 3 条集成测试（全成功 / 部分失败 / 空 Vec）全绿；
  - 前端 `SafeBox.tsx` 顶部加绿色「批量恢复已选(N)」按钮（在红色批量删除按钮左边），复用现有 selected Set，confirm 后调 `restore_many`，绿/红条显示成功/失败数，清空 selected 重拉列表。
  自验：cargo check/clippy/fmt/test=0（engine 146 条）；pnpm lint/test/build=0（168.49 kB / gzip 53.43 kB）。
2026-10-15：task-039 评审前清障——修复 3 项第一档问题：① `cargo fmt --check` 在 HEAD/main 双红灯（task-038 遗留的多行签名/断言格式，其评审「fmt 全绿」为误报）；② SafeBox 行内单条「恢复/永久删除」按钮禁用条件漏了批量操作状态，存在竞态；③ 补 `restore_many` 中间失败不阻断后续 id 的测试用例（原 partial_fail 失败项在末尾无法证明）；另修正 `BatchOpResult` 误导性注释、登记批量恢复真机端到端为已知限制。修复后 check/clippy/fmt/test（147 条）/pnpm lint/fe-test/build 全绿。
2026-10-16：task-040 首次评审**不通过**（dry-run 预览 UI）——评审结论：代码质量本身合格（dryRun.ts 纯函数无副作用、apiRaw 不破坏 api、Items 四分支渲染与只读保证代码层确认、Rust 零改动、白名单 7 文件无越界、无新增依赖），但触发一票否决的是**测试证据缺失**，不是业务代码 bug，故不开 bugfix 子分支改业务代码：
  - R1（阻塞）：真机 Tauri + WebView2 CDP(9222) 冒烟未执行（覆盖 code=0 汇总 / 2002 红线 / 2001 空清单 / 刷新清预览 / Console 0 红错）——路由测试 Agent（提示词 9）在 GUI 环境补做，不接受口头「本地绿」。
  - R2（阻塞）：NSIS `pnpm tauri build` → perMachine `/S` 装干净目录 → 安装版预览按钮可点/面板渲染/Console 零红错/DB 落 app_data_dir()——同样路由测试 Agent 补做。
  - R3（阻塞）：ci.yml 只在 `push:main` 与 PR 触发，本分支未开 PR、无真实 run 结论——需开 PR 取 windows-latest 五道闸 run URL + id + success。
  - R4/R5（低，本提交落地）：补本 docs(state) 记录；`tasks/040-dryrun-preview-ui.md` 状态字段由 `pending` 流转到 `review`。
  注：task-040 **尚未勾选完成**，待 R1–R3 真机/打包/CI 证据补齐后由评审（提示词 10）复核通过再勾选并合并 main。
2026-10-16：task-040 **补测完成**（评审打回后二轮）——R1/R2 真机证据已补齐，R3 CI 待开 PR：
  - R1（CDP 真机冒烟，已通过）：`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 --remote-allow-origins=*` 启动 NSIS 安装版（干净目录 `C:\Users\An\AppData\Local\Temp\cclean-smoke-040`），CDP `http://localhost:9222/json` 连到 `http://tauri.localhost/` 页面。验证：① 五 tab（仪表盘/清理/保险箱/历史/设置）全部渲染；② 清理页表格头（路径/大小/模块/Session）正确；③「预览可清理项」按钮存在，0 选中时正确禁用（C1）；④「清理已选」同步禁用；⑤ `Runtime.consoleAPICalled` + `exceptionThrown` 采集 **0 红错 0 exception**。
  - R2（NSIS 干净目录安装冒烟，已通过）：`/S` 静默安装到全新 temp 目录（13 文件：cdisk-cleaner.exe 16MB + uninstall.exe + rules/scripts 资源）；安装目录零写入（9 个静态文件，无 DB）；DB 落在 `%APPDATA%\com.and0114.cclean\data\cdiskcleaner.db`（57KB SQLite），rules 复制到 `%APPDATA%\com.and0114.cclean\rules\`——数据目录隔离验证通过。
  - R3（CI，待补）：需开 PR 到 main 触发 ci.yml windows-latest 五道闸（fmt/clippy/cargo test/pnpm lint+test/pnpm build+tauri build），当前无 gh CLI 鉴权，需用户手动开 PR 或授权。
  - 已知限制：干净目录安装后扫描未产出待清理项（安装目录 `_up_` 资源路径与运行时 `app_data_dir/rules/` 首次复制逻辑需真实环境验证，非 task-040 代码问题）；因此预览按钮的「点击→面板渲染」全链路因无数据行未能触发，但按钮存在性、禁用逻辑、页面渲染、Console 零红错均已验证。
2026-10-16：task-040 **R3 CI 缺口已补开 PR**（第二轮评审仍因 C3-C6 面板未真机渲染 + CI 未跑打回）：
  - 经 GitHub 插件开 **draft PR #3**（`feature/task-040-dryrun-preview-ui` → `main`，base 不合并）：https://github.com/An0114/C-clean/pull/3
  - CI run：https://github.com/An0114/C-clean/actions/runs/38839740840 （id `38839740840`，job `build`，记录时 status=in_progress；最终 conclusion 待 run 结束，以 PR #3 checks 页为准）。
  - 代码层面第二轮评审确认无 bug、不改业务代码、不开 bugfix 子分支。
2026-10-16：task-040 **三轮补测完成**——R3 CI 已绿，C3-C6 面板 innerText 因 CDP/WebView2 自动化限制登记第三档：
  - R3 CI（已通过）：PR #3 run 38839740840 五道闸全绿。
  - C3-C6 面板渲染：向 DB 插入 2 条测试数据后，清理 tab 表格正确渲染两行；预览按钮存在且 1 项选中时 enabled；Console 0 红错。但 CDP `.click()` 无法触发嵌入式 WebView2 的 React 合成事件（按钮文案未变「预览中…」、面板未渲染、0 console 事件）——这是 CDP 自动化对 Tauri WebView2 嵌入式控件的已知限制，非代码 bug。补偿：后端四分支契约 147+ Rust 测试 + 前端四分支渲染 28 Vitest 断言覆盖；面板 innerText 需人工鼠标点击验证。
  - 测试报告：`docs/test-reports/040-test-report.md` §8 含完整证据。
  - 仍待测试 Agent（提示词 9）补：C3-C6 四个面板分支（code=0 汇总 / 2002 红线 / 2001 空 / error）真机 innerText 证据——需 CDP 调 `scan_start(scope='full')` 造数据行后点预览抓面板文案（评审 §R1-R3）。
  - 注：本地一个评审 markdown 提交 `1ac7182` 经远端 PR head 已可见（PR head sha = 1ac7182）。
2026-10-16：task-041 完成——rules reqwest 超时收敛 + updater endpoint 改 Releases 资产（task-039 评审 P0 真机卡死修复）：
  - `engine/src/rules/remote.rs`：reqwest 统一走 `build_client()`，timeout=10s / connect_timeout=5s（env `RULES_HTTP_TIMEOUT_SECS`/`RULES_HTTP_CONNECT_SECS` 可覆盖，单测压 1s）；`map_http_error()` 按 is_timeout/is_connect/404/其他状态码分档返回中文文案；拆出 `fetch_remote_meta_with_url`/`fetch_remote_rules_bytes_with_url` 注入 URL 供单测。
  - `engine/tests/rules_timeout.rs` 3 条：404 文案含「404」×2、永不响应端口 1s 超时文案含「超时」，总耗时 1.1s。
  - `app/src-tauri/tauri.conf.json`：updater endpoints 从 `raw.githubusercontent.com/.../latest.json` 改为 `https://github.com/An0114/C-clean/releases/latest/download/latest.json`。
  - `app/src/pages/Settings.tsx`：规则库卡片错误态按钮文案从「检查更新」切为「重试」（phase=error 时已自动恢复可点，原「检查中…」卡死由后端超时根治）。
  自验：cargo check/clippy/fmt/test=0（engine 153 条）；pnpm lint/test/build=0（171.25 kB / gzip 54.59 kB）。偏差：错误码沿用 3003（任务书 §A3 要求 2003），动态中文原因走 eprintln 日志；Envelope.message 是 &'static str 不能传动态文案。仓库 public/private 决策留 task-042+。
2026-10-16：task-042 完成——rules.checkUpdate 加 Offline 三态，断网时本地规则库照常工作 + UI 黄条提示：
  - `engine/src/rules/remote.rs`：新增 `CheckStatus{UpToDate,NewVersion,Offline}` 枚举 + `CheckUpdateResult{status,localVersion,localSha256,remoteVersion?,remoteSha256?,message}`，提供 `up_to_date/new_version/offline/classify` 构造器。
  - `app/src-tauri/src/commands.rs`：`rules_check_update` 返回 `Envelope<CheckUpdateResult>`，reqwest 失败不再返回错误信封，而是 `status=Offline, message=<task-041 中文错误文案>`。
  - `engine/tests/rules_offline_mode.rs` 3 条：Offline（永不响应端口 1s 超时）/ UpToDate（mock server 返回同 sha）/ NewVersion（返回更高 version），总耗时 1.1s。
  - `app/src/pages/Settings.tsx`：按 status 渲染三态提示条——upToDate 绿条「规则库已是最新」、newVersion 蓝条「发现新版本 vX.Y.Z」、offline 黄条「离线模式：使用本地规则库 vX.Y.Z（<message>）」；应用更新按钮仅在 newVersion 时出现。
  自验：cargo check/clippy/fmt/test=0（engine 156 条）；pnpm lint/test/build=0（172.01 kB / gzip 54.78 kB）。applyUpdate 命令的网络错误仍返回错误信封（本任务不改）。
2026-10-16：task-043 完成——safe_box restore 自动 create_dir_all 父目录（task-017 已知问题闭环）：
  - `engine/src/execute/safe_box.rs` `restore_from_safe_box` step 3：原 `ParentMissing(p) => Err(...)` 改为 `if !parent.exists() { std::fs::create_dir_all(parent).map_err(|e| Io("无法创建父目录 {p}: {e}"))? }`；`ParentMissing` 枚举变体保留但不再返回。
  - `engine/tests/safe_box_restore_mkdir.rs` 3 条新建：缺目录自动建多层 / 已存在目录 sibling 文件不被覆盖 / 父路径不可用（父组件是文件）返回错误。
  - 旧用例适配：`safe_box_restore.rs` 的 `restore_parent_missing_returns_error` 改为 `restore_parent_missing_auto_creates_dirs`（正向断言）；`safe_box_restore_many.rs` 的中间失败注入从 ParentMissing 改为 TargetOccupied（预置同名文件）。
  自验：cargo check/clippy/fmt/test=0（engine 159 条）；pnpm lint/test/build=0（前端零改动）。真机/打包冒烟按任务书裁剪。
2026-10-16：task-044 完成——rules_rollback 单版回滚（后端命令 + 前端灰色按钮 + confirm）：
  - `engine/src/rules/rollback.rs`（新建）：`RollbackResult{previousVersion,restoredSha256,message}` + `has_backup(rules_dir) -> bool` + `rollback(rules_dir, conn)`；.bak 路径 `<rules_dir>/cache_rules.json.bak` 与 apply_update 复用。
  - `engine/tests/rules_rollback.rs` 5 条：has_backup 真/假、无 .bak Err、回滚成功（v1→v0）、损坏 .bak Err。
  - `app/src-tauri/src/commands.rs`：`rules_has_backup() -> Envelope<bool>` + `rules_rollback() -> Envelope<RollbackResult>`，错误用 INTERNAL 信封。
  - `app/src/pages/Settings.tsx`：启动时调 has_backup，true 时显示灰色「回滚到上一版」按钮；confirm 后 invoke rollback → 绿条「已回滚到 vX.Y.Z」。
  自验：cargo check/clippy/fmt/test=0（engine 164 条）；pnpm lint/test/build=0（172.68 kB / gzip 54.92 kB）。真机/打包冒烟由测试 Agent 在 L4 完成。
2026-10-16：task-045 完成——safe_box list 分页（LIMIT/OFFSET + total + 前端分页栏）：
  - `engine/src/execute/safe_box.rs`：新增 `SafeBoxPage{items,total,page,pageSize,totalBytes}` + `list_paged(conn,page,page_size)`；page_size clamp [1,200]、page<1 按 1；SQL `ORDER BY id DESC LIMIT ? OFFSET ?` + `COUNT(*)` + `SUM(size_bytes)`。
  - `engine/tests/safe_box_pagination.rs` 5 条：首页 50 条 / 次页 10 条 / page_size=99999 clamp 到 200 / 越界页空 / id DESC 跨页递减。
  - `app/src-tauri/src/commands.rs`：`safe_box_list(page:u32,page_size:u32) -> Envelope<SafeBoxPage>`。
  - `app/src/pages/SafeBox.tsx`：state 加 page/pageSize/total；表格下加分页栏（上一页 / 第 X / Y 页（共 N 条） / 下一页）；翻页清空 selected。
  自验：cargo check/clippy/fmt/test=0（engine 169 条）；pnpm lint/test/build=0（173.18 kB / gzip 55.10 kB）。真机/打包冒烟由测试 Agent 在 L4 完成。
2026-10-16：task-046 完成——safe_box list session 过滤（可选 session_id 参数 + list_sessions 分组 + 前端下拉）：
  - `engine/src/execute/safe_box.rs`：`list_paged(conn,page,page_size,session_id:Option<i64>)`；None 全表、Some(id) 同步加 `AND source_task_id=?` 到 COUNT/SUM/items；新增 `SessionSummary{sessionId,itemCount,createdTs}` + `list_sessions(conn)` 按 source_task_id GROUP BY。
  - 偏差：真实 safe_box schema 没有 session_id 列，只有 `source_task_id` 外键；实现上把 source_task_id 当作「session」语义，未加列、未做 scan_session 联表。
  - `engine/tests/safe_box_session_filter.rs` 4 条：全部返回 / 按 session 过滤 / 无匹配 / list_sessions 分组。
  - `app/src-tauri/src/commands.rs`：`safe_box_list(page,page_size,session_id,state)`；新增 `safe_box_list_sessions(state) -> Envelope<Vec<SessionSummary>>`；lib.rs 注册。
  - `app/src/pages/SafeBox.tsx`：state 加 sessionFilter/sessions；分页栏左侧加 `<select>`（全部 session / session #N (M 条)）；切换时 page=1、清空 selected、重拉 list。
  自验：cargo check/clippy/fmt/test=0（engine 173 条）；pnpm lint/test/build=0（173.78 kB / gzip 55.27 kB）。真机/打包冒烟由测试 Agent 在 L4 完成。
2026-10-16：task-047 完成——rules.applyUpdate 前端 window.confirm 弹窗：
  - `app/src/pages/Settings.tsx`：`apply` useCallback 开头加 `window.confirm(...)`，文案嵌入 `info.localVersion / info.remoteVersion`；取消直接 return，不进 setPhase/不调 invoke；依赖数组补 `info`。
  - confirm 文案：「即将应用规则库 v{remoteVer}（当前本地 v{localVer}）。应用后旧版将备份为 cache_rules.json.bak，可随时通过「回滚到上一版」恢复。是否继续？」。
  自验：cargo check/clippy/fmt/test=0（后端零改动，engine 173 条）；pnpm lint/test/build=0（173.98 kB / gzip 55.41 kB）。真机/打包冒烟由测试 Agent 在 L4 完成。
2026-10-16：用户决策落地（DocsKeeper 同步状态，无代码改动）：
  - **项目仓库已转 public**：updater endpoint（task-041 已改 `releases/latest/download/latest.json`）直拉可达，无需 token header；task-039 评审遗留的「私有仓 raw 404」正式关闭。
  - **品牌图标决策：暂时不做**：沿用 Tauri 默认占位，v0.1.x 发版不阻塞；待后续有 1024×1024 源 PNG 再跑 `tauri icon`。
  - **首次 v0.1.1 发版已手动 workflow_dispatch 执行**：release workflow 已产出安装包 + latest.json（含签名），updater 端到端待用户在已装 v0.1.0 的机器上启动应用真机验证 banner 是否弹 v0.1.1。
  - 同期用户侧已做的发版准备 commit（未走任务流程）：`3fe5268 chore: 更换 updater 签名公钥`、`8f65b6b fix(release): 修正签名环境变量名，未配置 secret 时优雅跳过 latest.json`、`25aafb6 fix(release): 移除 pnpm/action-setup 的 version 字段`、`37c5c97 Update .gitignore`。
2026-10-16：task-046 rustfmt CI 红灯修复（bugfix 子分支闭环）：
  - 现象：PR #8（feature/task-046-safebox-session-filter）CI run 38875797117 的「Rustfmt 检查」步骤 exit 1——`engine/src/execute/safe_box.rs:434` `Some(s) => conn` 折多行压回单行；`engine/tests/safe_box_pagination.rs` 文件头 BOM + 文件尾多余空行。
  - 修复：跑 `cargo fmt --all` 自动收敛（纯格式/空白改动，无逻辑变更）；bugfix 子分支 `bugfix/task-046-rustfmt-ci-red`，提交 `659013c`，已 `--no-ff` 合回 feature（merge `bf8a133`）。
  - 自验：`cargo fmt --all -- --check` exit 0；clippy -D warnings 无告警；safe_box_pagination 5/5、safe_box_session_filter 4/4 测试通过。
  - 结果：PR #8 重跑 CI run 38876741430 **conclusion=success**（08:53→08:58，五道闸全绿）。状态置 testing，待测试/评审复核。
2026-10-23：bugfix/task-035-release-signing 完成——发版 workflow 两个 bug 修复（**状态 testing，等 CI 全绿 + 测试/评审复核，未合并**）：
  - 分支基线说明：`feature/task-035-tauri-updater` 早已并入 main（bug-001~008 的发版修复直接落在 main 上，属线上问题），故本 bugfix 分支按 hotfix 语义从 main 拉出，而非从 feature 拉。
  - **Bug-2（.sig 永不生成，根因）**：`app/src-tauri/tauri.conf.json` 缺 `bundle.createUpdaterArtifacts`。Tauri 2 该字段默认 `false`，为 false 时 bundler **压根不会去读签名私钥**，所以前面「直接传私钥内容 / 写文件传路径 / 改环境变量名 / 改换行符」全部无效且无任何报错。补 `"createUpdaterArtifacts": true` 后本地实测产出 `C-DiskCleaner_0.1.99_x64-setup.exe.sig` + `.msi.sig`（此前同一 target 目录只有 .exe/.msi，0 个 .sig）。反证：该开关打开但不给私钥时构建直接失败 `A public key has been found, but no private key`——说明开关确实是签名闸门。
  - **Bug-1（版本号永远 0.1.0）**：现版本的内联 PowerShell 正则注入本地实测是生效的（注入 0.1.99 → 产物名 `_0.1.99_`），真正的缺口是「没有校验」——注入失败/版本来源取错（如 tauri-action 时代用 tag 拼 release 名但产物版本仍读 conf；dispatch 时 `ref_name` 是分支名）都表现为 CI 绿 + 产物错。修法：版本解析抽成 `scripts/release-lib.mjs` 的 `normalizeVersion`（严格 semver，分支名直接失败），注入后立刻 `JSON.parse` 复验，构建后再按安装包文件名硬校验 `_<version>_`，对不上直接红灯。
  - **新增 `scripts/release-lib.mjs`**（纯函数 + CLI，零依赖）：`normalizeVersion` / `patchTauriConfVersion` / `findUpdaterArtifact` / `verifyArtifacts` / `buildLatestJson` / `listFilesRecursive`；CLI 子命令 `resolve-version` / `patch-conf` / `verify-artifacts` / `generate-latest`。
  - **`release.yml` 重构**：版本号单点解析写入 `RELEASE_VERSION` → `patch-conf` 注入并复验 → 私钥空值前置拦截 + LF 归一 → build → `verify-artifacts` 硬校验 → `generate-latest` → 上传；latest.json 与 Release tag 全部用同一 `RELEASE_VERSION`。
  - **`ci.yml` 配套**：`createUpdaterArtifacts=true` 后 tauri build 强制要私钥，CI 用 `tauri signer generate` 一次性临时密钥（RUNNER_TEMP，不入库）签名，并新增「校验打包产物（版本号 + .sig）」步骤 + 上传 `*.sig`；新增「发版脚本单测」步骤。
  - **单测**：`scripts/release-lib.test.mjs`（node:test，零新依赖）21 条全绿——版本解析 5 条（输入优先/v 前缀/tag 回退/分支名拒收/预发布）、版本注入 3 条（顶层替换/不误伤嵌套 version/缺字段报错）、产物挑选 3 条（NSIS 优先/msi 回退/无签名返回 null）、产物校验 5 条（正常/版本不符/无 sig/空目录/残留旧版本/旧签名不蒙混）、latest.json 1 条、契约测试 3 条（createUpdaterArtifacts 必须为 true、pubkey+endpoint 在位、release.yml 必须注入私钥+校验+上传 .sig）。
  - 自验：`node --test scripts/release-lib.test.mjs` 21/21 pass；本地 `pnpm --filter app tauri build`（临时密钥）产出 `C-DiskCleaner_0.1.99_x64-setup.exe` + `.sig`、`.msi` + `.sig`，`verify-artifacts --version 0.1.99` 通过、`generate-latest` 产出合法 latest.json（URL 指向 v0.1.99 的 nsis 包）；后端/前端源码零改动（未动 Rust/TS，故未重跑 cargo/pnpm 全量门禁，交由 CI 覆盖）。
  - **Bug-3（真机发版暴露：私钥文件多写一个换行 → base64 解码失败）**：Release run `39940302050` 的 `Tauri build` 在 msi 都打完、第 8 分钟才失败：`failed to decode base64 secret key: Invalid symbol 10, offset 348`。根因是 `Setup signing key` 步骤写的是 `($env:SECRET_KEY -replace "\r\n","\n").Trim() + "\n"`，**主动补的 `\n`** 让私钥文件从 348 字节变 349 字节，而 Tauri 是把「整个文件内容」当一整段 base64 解、不会替调用方 strip 空白，于是在最后一个字节炸掉（`tauri signer generate` 产出的 `.key` 本就是 348 字节、无 CR/LF、末字节 `=`）。
  - **Bug-3 修复**：`release-lib.mjs` 新增 `normalizeSigningKey`（剔除所有空白 → 校验字符集为 base64、长度为 4 的倍数、解码后带 minisign `untrusted comment` 头）与 CLI 子命令 `write-key`（规范化后原样写盘，不加换行/不加 BOM，并校验写盘字节数 == 内容长度）；`release.yml` 改调 `write-key`。失败点从「构建 8 分钟后」提前到「写盘秒级」。
  - **Bug-3 自验**：单测 21 → **26 条全绿**（新增私钥规范化 4 条 + 契约 1 条「必须走 write-key，不得再拼换行」）；真实密钥端到端：348 字符 → `write-key` 写出 **348 字节、无 CR/LF、可还原为 `untrusted comment: rsign encrypted secret key`** → `tauri signer sign` 成功产出 392B `.sig`；反向（文件 +`\n`）复现出与 CI **一字不差**的 `Invalid symbol 10, offset 348` 且不产出 `.sig`。
  - **端到端验收（真实发版，非本地模拟）**：Release run `39942401586`（sha `17f5bcb`，dispatch version `0.1.5`）14 步全绿；PR CI run `39942341485` 20 道门禁全绿（含发版脚本单测、Tauri build、校验打包产物）。Release v0.1.5 资产：`C-DiskCleaner_0.1.5_x64-setup.exe` + `.exe.sig`(444B)、`..._x64_en-US.msi` + `.msi.sig`(444B)、`latest.json`(718B)——**两个 bug 的现象均消失**（此前 5 个 Release 全部零 `.sig`，v0.1.1/v0.1.2 资产名还是 `_0.1.0_`）。
  - **签名配对已证实**：v0.1.5 的 `.exe.sig` 内层 keyid = `52ACA0C8DAA19EB3`，与 `plugins.updater.pubkey` 的 keyid **完全一致**（此前记的 `B39EA1DAC8A0AC52` 是字节序反读）。`latest.json` 的 `version=0.1.5`、`url` 指向 v0.1.5 的 nsis 包、`signature` 与 `.sig` 资产前 40 字符一致。
  - 遗留（本次未改，待定）：`softprops/action-gh-release` 未指定 `target_commitish`，GitHub 默认把 tag 打到默认分支，导致 `v0.1.5` tag 指向 main `bfe9414` 而非构建提交 `17f5bcb`（资产下载不受影响，属溯源问题；一行 `target_commitish: ${{ github.sha }}` 可修，但需再发一版验证，按最小改动原则本次不动）。
2026-10-23：task-048 完成（用户真机验证通过）——v0.1.1 升级验证收尾，**M2 正式关闭**：
  - 验证方式：用户本机已装 v0.1.0，直接启动应用观察 updater 行为（非 CDP 模拟，真实安装态 + 真实 GitHub Release 源）。
  - 结果（截图证据）：① 顶部蓝底 banner「发现新版本 v0.1.1 / v0.1.1 发布」正确弹出，含「立即更新」「稍后」双按钮；② 设置页「应用更新」卡片同步显示「更新状态：发现新版本 0.1.1」「新版本：0.1.1」，与 banner 一致；③ 同页规则库卡片仍为「规则库已是最新（v1.0.0）」，说明应用更新通道与规则库更新通道互不串台。
  - 结论：task-036 启动自动 check + 非阻塞 banner、task-035 plugins.updater 配置 + `latest.json` 直拉、bugfix/task-035 签名修复三者链路在真机安装态下一致成立；M2 无遗留阻塞项。
  - 后续（非阻塞）：banner「立即更新」点击后的下载安装进度反馈、差量更新留 M3+；`target_commitish` 溯源修正留发版类任务。

2026-10-23：task-049 完成（feature/task-049-rm-shutdown-locking，状态 testing）——占用进程从「只报告」升级为「可处置」：
  - 后端 `engine/src/execute/locking.rs`：抽出 `RmSession`（`RmEndSession` 放进 `Drop`，任何路径都不泄漏会话）；新增 `shutdown_locking_processes`（单资源会话 + `RmGetList` 护栏过滤 + `RmShutdown(RmForceShutdown)`，无占用/全拒时**不调** `RmShutdown`）；新增 `is_protected_process` 纯函数（服务 / `pid<=4` / 自身 / 12 条保护名单，含 RM 应用友好名兜底）。
  - **顺带修掉一个既有 bug**：`who_is_locking` 的 probe 用 `Error::code()` 与 `234` 比较，实际拿到 HRESULT `0x800700EA`，导致「真有进程占用时直接返回 Err」——task-020/021 的占用检测在真机上从未生效，`ExecResult.lockingProcesses` 一直是空的。
  - 命令层：新增 `cleanup_retry_locked`（`RmShutdown` 阻塞 FFI 走 `spawn_blocking`，closed/refused 按 pid 去重，删除复用 `execute_items` 同一段编排，白名单二次校验照常）。
  - 前端 `Items.tsx`：`cleanup_execute` 返回值不再丢弃，失败明细展示「路径 — 原因 — 占用进程 `app.exe (PID x)`」；有占用时出现「关闭占用进程并重试」，`window.confirm` 列出进程清单 + 「未保存的数据可能丢失」；结果条展示「已关闭 N 个 / 受保护未关闭清单 / 仍失败 Y 项」；cleaning/retrying/previewing 三态互斥。
  - 门禁：fmt 0 / clippy 0 / `cargo test -p engine` 183 passed + 5 ignored（合计 188，基线 173，+15）/ pnpm lint 0 / pnpm test 28 / pnpm build 176.39 kB（gzip 56.25 kB）。`pnpm tauri build` 出包成功（NSIS 4.01 MiB / MSI 5.80 MiB）。
  - **真机证据（后端全链路）**：`engine/tests/locking_shutdown.rs::execute_retry_after_shutdown_deletes_locked_file`（`--ignored` 手动跑）PASS——首次删除 `deleted=false` + `lockingProcesses=[PID]` → `closed=[PID]` → 重试 `deleted=true, bytesFreed=7`，文件消失。
  - **未取得**：GUI 点击截图与安装态冒烟。原因：Tauri 显式传 `additionalBrowserArgs` 使 WebView2 忽略 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`（CDP 9222 开不出来，改 `tauri.conf.json` 属禁止范围）；且当前会话无活动桌面合成（`PrintWindow`/`BitBlt` 客户区全黑）；安装器执行被环境拦截（exit 126）。已登记为遗留项，交测试 Agent 在真机安装态补齐。
  - 详见 `tasks/049-rm-shutdown-locking-processes.md` 的「实现备注」。

2026-10-24：task-049 测试完成（测试 Agent / 提示词 9，状态 **awaiting_review**）——独立重跑全部门禁 + 测试期缺陷闭环：
  - **基线**：`git fetch` 后本地 = 远端 `0541235`；改动 7 文件（3 代码 + 2 测试 + 2 文档），未越界（`executor.rs` / `tauri.conf.json` / `Cargo.toml` / `package.json` 均未动）。
  - **L1 全绿**：fmt 0 / clippy 0 / `cargo test -p engine` **183 passed + 6 ignored**（基线 173，+10）/ `pnpm lint` 0 / vitest **34 passed（6 files，含本轮新增 6 条 Items 组件用例）** / `pnpm build` 176.39 kB（gzip 56.25 kB）。
  - **L2 契约**：命令签名↔实参、5 个结构体 camelCase、Envelope 解包、事件 payload 四项逐字段核对一致；真机 `--ignored` 用例输出真实 JSON 与前端 interface 完全对齐。
  - **缺陷 1（已修）**：清理失败后 `cleaning` / `previewing` 期间「关闭占用进程并重试」仍可点（违反 C12 三态互斥）→ `bugfix/task-049-retry-btn-mutex` `0129de9`，`disabled={busy}` + 入口守卫改 `busy`；新增组件用例在修复前代码上实测失败、修复后通过。
  - **缺陷 2（已修）**：任务书证据清单要求的「只传 1 个 id 不误伤其他文件」无对应用例 → 同分支 `f667c70` 新增 `shutdown_only_touches_registered_path`，真机 3/3 PASS（A 关闭、B 占用与进程原样保留）。
  - **合并**：`merge --no-ff` → `be60f55`；合并后全量回归（fmt/clippy/cargo 183/vitest 34/build 0）全绿。
  - **L4 打包安装**：`pnpm tauri build` 出包（NSIS 4.01 MiB / MSI 6.94 MiB）；`/S /D=%TEMP%\cclean-smoke-049-nsis` 安装 exit 0；安装版启动后 `data_root=%APPDATA%\com.and0114.cclean`，安装目录 9 个文件 SHA-256 前后**零变化**。
  - **未取得（第三档，已登记）**：GUI 真机点击与 DevTools Console —— 本会话无活动桌面合成，WebView2 渲染进程创建即崩溃（Crashpad 持续产出 `.dmp`、应用进程 0 子进程、CDP 9222 不监听、`CopyFromScreen` 句柄无效）。补偿：C8–C12 用 jsdom 组件用例 + 后端真实占用进程端到端 + 代码走查，**未伪造截图/日志**。
  - **新登记预存在问题（非本任务引入）**：打包态 `bundle.resources` 落在 `$INSTDIR\_up_\_up_\rules`，而 `AppPaths` 只查 `resource_dir()/rules` → 安装版 `scripts_dir=.\scripts`，全新用户首次启动取不到规则会导致 `cleanup_execute`/`cleanup_retry_locked` 返回 INTERNAL（本机因 `%APPDATA%\...\rules` 已存在而未暴露）。引入于 `16b30e2`，修复需改 `tauri.conf.json` 或 `paths.rs`（均在 task-049 禁止清单内），建议另开任务。
  - **CI**：`ci` run 39967340368（0541235）/ 40045174671（8b84cf4）/ 40045926460（f667c70）/ 40047649773（be60f55）/ 40047903556（b1d6118）/ 40048375380（e23cdce）全部 `conclusion=success`（末次 20 步全绿，含 Rust 测试 / 前端 lint+test+build / Tauri build / 产物与规则库校验）。
  - 详见 `docs/test-reports/049-test-report.md`。

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
- [x] 035：tauri updater 自动更新最小闭环——签名密钥对 + plugins.updater 配置 + Settings 应用更新卡片 + release.yml 签名/latest.json 生成
- [x] 036：启动自动 check + 非阻塞 banner——App.tsx useEffect 自动 check()，有新版本顶部蓝条提示 + 立即更新/稍后按钮
- [x] 037：safe_box purge 单条永久删除——后端 purge_item + Tauri 命令 + 前端红色按钮 + confirm
- [x] 038：safe_box 批量 purge——后端 purge_many + Tauri 命令 + 前端 checkbox 列 + 批量删除按钮
- [x] 039：safe_box 批量恢复——后端 restore_many + Tauri 命令 + 前端绿色批量恢复按钮（复用 task-038 checkbox）
- [x] 049：占用进程 RmShutdown——后端 `shutdown_locking_processes` + 危险进程护栏 + `cleanup_retry_locked` 命令 + Items 失败明细与「关闭占用进程并重试」（**测试通过，状态 awaiting_review，待评审**；测试期修 C12 三态互斥 + 补 B7 隔离用例）
- [x] 048：v0.1.1 真机升级验证收尾——已装 v0.1.0 的机器启动应用，updater banner 正确弹出「发现新版本 v0.1.1」（用户真机验证通过，2026-10-23），**M2 正式关闭**

> 注：040~047 的完成明细未回填本清单（历史遗漏），以「最近一次变更」段与 `tasks/040-*.md`~`tasks/047-*.md` 为准。

## 已知问题
- [B 中] ~~task-021 仅做「报告」（who_is_locking 结果写 audit_log + 返回前端），未实现 RmShutdown，前端 `cleanup_execute` 返回值被直接丢弃、`ExecResult.lockingProcesses` 没渲染~~ **已由 task-049 解决**（`shutdown_locking_processes` + 护栏 + `cleanup_retry_locked` + Items 失败明细/一键重试）。**并顺带修掉 task-020 的 probe 判断 bug**（`Error::code()` 是 HRESULT，与 `234` 比较恒不等 → 真有占用时直接 Err，占用列表一直是空的）。
- [B 中] task-049 遗留：① `RmRestart`（关闭后自动重启进程）未做，用户需自行重开；② `RmShutdown` 阻塞最坏 ~30s（目标不响应关闭请求时等 RM 超时），期间前端只有「关闭并重试中…」无进度；③ RM 会话是「按资源」而非「按进程」，同一文件同时被保护进程与普通进程占用时无法只关普通进程（`closed` 不会记保护进程，但 `explorer.exe` 会随会话被请求关闭，Windows 会自动重启）；④ GUI 真机截图与 NSIS 安装态冒烟未取得——**测试轮（2026-10-24）已补安装态冒烟（干净目录安装 + 数据目录隔离 + 安装目录零写入），GUI 点击/Console 仍因「本会话无活动桌面、WebView2 渲染进程创建即崩溃」未取得，登记为第三档，补偿为 jsdom 组件用例（C8–C12）+ 后端真实占用进程端到端，需在有活动桌面的机器补做**。
- [B 中] **打包态资源路径错位（2026-10-24 测试轮发现，非 task-049 引入）**：`tauri.conf.json` 的 `bundle.resources` 用 `../../rules/...` 相对路径，NSIS/WiX 按 `$INSTDIR\_up_\_up_\rules` 落盘，而 `AppPaths::from_app` 只查 `resource_dir().join("rules")` → 安装版实测 `[paths] scripts_dir=.\scripts`。影响：全新用户首次启动取不到 bundled 规则 → `load_external_deny` 失败 → `cleanup_execute` / `cleanup_retry_locked` 返回 `INTERNAL`（本机因 `%APPDATA%\com.and0114.cclean\rules` 已存在而未暴露）。引入于 `16b30e2`（debt-cleanup）；修复需改 `tauri.conf.json` 或 `paths.rs`，建议另开任务。
- [B 低] ~~task-018 list 边界：无分页（保险箱总量 ≤5GB，条目数有限）、无 session 过滤（先返回全部）、无单条 purge 命令、未接前端 React UI~~（~~无分页~~：task-045 已落地 list_paged LIMIT/OFFSET + 前端分页栏；~~无 session 过滤~~：task-046 已落地 session_id 可选参数 + list_sessions + 前端下拉）。真多了再加 时间范围筛选 / 多选 session / audit_log 联表 留 task-047+。
- [B 低] ~~task-017 restore 边界：不自动建原父目录（ParentMissing 直接报错让用户手动处理）、不支持批量恢复、未加 `restored_at` 审计列（DELETE 行 + audit_log action='restore' 已留痕）。批量恢复与父目录自动 mkdir 留后续。~~ **已由 task-043 解决**（restore_from_safe_box 在父目录缺失时自动 `std::fs::create_dir_all`，mkdir 失败返回「无法创建父目录 <path>: <os error>」；ParentMissing 枚举变体保留但不再返回）。遗留：未加 `restored_at` 审计列。
- [B 中] task-016 保险箱原子性：`fs::rename` 成功但 `INSERT INTO safe_box` 失败时，文件已在 box 目录但 DB 无行——当前简单报错，不做回滚；跨盘 rename 走 copy+remove 也非原子。补偿：下次 enforce_quota 扫到磁盘上有孤儿文件时会 NotFound 直接标 purged（不会重复占用 quota）。
- [B 低] task-016 跨盘 rename：`fs::rename` 遇 EXDEV(17)/跨盘错误时 fallback 到 copy+remove；CI 单盘未实测跨盘用例。
- [B 中] task-015 文件占用时删除失败：Windows 下 `std::fs::remove_file` 遇 SharingViolation 直接返 Err，不做重试/Restart Manager；命中占用的 item 记为 fail 并写 audit_log result='fail'。Restart Manager 集成留 M1-D 后续任务。
- [B 低] UNC 路径 `\\server\share\...` task-008 评估结论：**沿用保守拒绝**（normalize 层 NotAbsolute + resolve 层 Io 双保险）；网络盘 L3 红线策略留待真实 NAS/企业盘场景出现时再开。
- [B 低] task-007 WebView2 devtools console 的 `listen('scan://progress')` 真机截图未在本轮自动化环境内取得（tauri dev 窗口启动后 GUI 自动化无法稳定聚焦 console）；通道本身已由编译期类型 + engine 节流单测保证，留待 UI 联调任务补截图。
- [B 低] task-005 rules_meta 落 DB + checkUpdate + applyUpdate 均已在 task-022/023 落地；~~GitHub Token/私有仓、失败重试/离线降级、手动回滚命令（从 .bak 恢复）、前端 UI 「确认更新」弹窗~~（~~GitHub Token/私有仓~~：项目已转 public，无需 token；~~离线降级~~：task-042 已落地 Offline 三态；~~手动回滚~~：task-044 已落地 rules_rollback + 前端按钮；~~前端确认弹窗~~：task-047 已落地 window.confirm 嵌入版本号 + 回滚提示）、差量更新 仍未做，留后续。
- [B 低] task-005 外部红线命中统一记为 `DenialReason::L3SystemPath`（遵守「whitelist.rs 仅新增函数、不改枚举/常量」约束）；后续审计若需区分内置/外部红线来源，再拆 DenialReason 变体。
- [B 低] task-006 CI 未接 Dependabot / cargo audit / npm audit（任务边界明确不做）；~~package.json 的 lint/test 为 echo 占位，CI 也不跑这两个命令~~（task-032 已接入 ESLint 10 flat config + Vitest 2.1，CI 已加 Lint/Test 两步）。CI 仅 windows-latest 单平台，跨平台矩阵（macOS/Linux）本项目明确不做；~~NSIS 出包矩阵未配~~（task-033 已配 Windows 单平台 nsis+msi，artifact 名 cclean-windows-installer）。
- [B 中] task-033 installer 未做代码签名：未购 EV/OV 证书，用户首次运行 .exe/.msi 会有 SmartScreen「Windows 已保护你的电脑」警告，需要手动「更多信息→仍要运行」；正式发版前需评估代码签名（自签/商业证书）。
- [B 低] task-033 应用图标仍为 Tauri 默认占位（task-001 生成的 17 个文件）。**决策（2026-10-16）：品牌图标暂时不做**，沿用默认占位，v0.1.x 发版不阻塞；待后续有设计出 1024×1024 源 PNG 后再跑 `tauri icon` 重生成全套。
- [B 低] task-039 safe_box 批量恢复真机端到端未跑：自动化环境无法在 GUI 里取得保险箱数据行（无真实扫描产出的文件），checkbox 勾选 + 批量恢复按钮全链路未经真机点击验证。补偿：后端 `restore_many` 4 条集成测试（全成功 / 部分失败 / 空 Vec / 中间失败不阻断后续）+ 前端 lint/单测/build 全绿 + 代码走查（selected Set 复用、confirm、绿红条、清空 selected 重拉列表）。需装包后人工造数据验证。（2026-10-15 评审侧已补齐：评审 Agent 用造数据在 `pnpm tauri dev` 真机跑通 checkbox 勾选 → 批量恢复全链路（2 成功 / 1 失败父目录缺失），证据见 `docs/reviews/task-039-batch-restore-review.md`；安装包级干净目录安装冒烟仍留发版任务。）
- [B 中] task-039 评审运行时冒烟发现（**非本任务引入**，设置页属 task-022/031 范围）：每次文档加载后**首次**进入「设置」tab 调用 `rules_check_update` 会打印 2 条红色 console error（source=network，`http://ipc.localhost/rules_check_update`，ERR_CONNECTION_REFUSED）+ 2 条 Tauri「IPC custom protocol failed, will use postMessage」warning；同条件点历史/清理/仪表盘 IPC 均 0 红错，同文档内再次进设置页也不再复现。功能经 postMessage 回退仍可用。建议 `bugfix/task-039-settings-ipc-redconsole` 排查 Tauri 2.x ipc.localhost 首调与 `useCustomProtocol` 回退策略。
- [B 中] ~~task-039 评审运行时冒烟发现：设置页规则库检查长时间停留「检查中…」（实测 ≥8s 无收敛、无失败态）；本机 raw URL 404（私有仓 raw 不可达），Rust 侧外呼无短超时/失败收敛。~~ **已由 task-041 解决**（reqwest 加 10s/5s 超时 + 中文错误映射 + updater endpoint 改 Releases/latest/download URL）。~~遗留：仓库 public/private 决策待用户拍板（推荐转 public），token header 留 task-042+。~~ **已落地（2026-10-16）：项目仓库已转 public，无需 token header，Releases 资产直拉可达。**
- [B 高] ~~发版 workflow 两个 bug：① 安装包版本号永远 0.1.0，不随 workflow_dispatch 输入变化；② 配了签名私钥却不生成 .sig（多轮改私钥传递方式均无效）。~~ **已由 bugfix/task-035-release-signing 修复（2026-10-23，状态 testing，待 CI 全绿 + 测试/评审复核）**：根因分别是「版本注入无校验、失败静默」与「`tauri.conf.json` 缺 `bundle.createUpdaterArtifacts`（Tauri 2 默认 false，为 false 时 bundler 不读私钥）」；修法与证据见「最近一次变更」。遗留：真机端到端升级验证（装旧版 → 检查更新 → 下载安装 → 重启到新版）需发版后人工跑。~~GitHub Secret `TAURI_SIGNING_PRIVATE_KEY` / `_PASSWORD` 必须与 `plugins.updater.pubkey` 配对（本地 `~/.tauri/cclean.key.pub` 与仓内 pubkey 不是同一对，CI 侧 secret 需用户自查）~~ **已证实配对（2026-10-23）**：v0.1.5 的 `.exe.sig` keyid `52ACA0C8DAA19EB3` 与 `plugins.updater.pubkey` 一致，真实发版 run `39942401586` 全绿、产物含 `.sig` + `latest.json`。

## 下一步计划
M2 updater 已关闭（task-048）；task-049 测试完成（`awaiting_review`）。下一轮优先级：

1. **① 评审 task-049**（`feature/task-049-rm-shutdown-locking` @ `be60f55`）：重点复核护栏有效性、`RmShutdown` 会话粒度限制、`who_is_locking` probe bug 修复是否引入回归、以及测试轮新增的 C12 修复与 B7 用例。
2. **② 决策打包资源路径错位**（见「已知问题」）：是否另开 `bugfix/task-050-bundle-resources-path` 修 `bundle.resources` / `AppPaths` 资源定位（影响全新安装用户首次启动）。
3. **③ 派发 task-051：safe_box 增强**——时间范围筛选 / 多选 session / audit_log 联表（`list_paged` 已支持 session_id 与分页，本轮加时间区间与联表视图）。
4. ④ GUI 真机冒烟补做：task-049 的红条明细 / confirm 文案 / 绿条结果 + Console 0 红错，需在具备活动桌面的机器上执行；`RmRestart` 是否要做由产品侧拍板（当前用户需自行重开被关进程）。
5. ⑤ 发版类小修：`release.yml` 补 `target_commitish: ${{ github.sha }}`（tag 溯源，需再发一版验证）。
6. ⑥ 更新体验：banner「立即更新」的下载进度反馈 / 差量更新（M3+，非阻塞）。

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
| 6233da0 | #debt-cleanup-002 | fix(frontend): unwrap Envelope<T> in api() helper to fix white screens |
| f939780 | #debt-cleanup-002 | fix(tauri): grant core:event:default so listen passes ACL |
| c81615d | #debt-cleanup-002 | fix(ci): drop explicit pnpm version in release.yml |
| 6e149ea | #debt-cleanup-002 | fix(engine): count hops by reparse point, not canonicalize string compare |
| 577e6f5 | #debt-cleanup-002 | fix(dashboard): reset progress on scan failure |
| （本次） | #debt-cleanup-002 | docs(state): debt-cleanup-002 五个 M1 打包回归 bug 修复完成 |
| c5cf1d3 | #debt-cleanup-003 | fix(frontend): pass scope='full' to scan_start command |
| 16b30e2 | #debt-cleanup-003 | fix(tauri): use app_data_dir() for DB/safe_box, bundle rules+scripts as resources |
| （本次） | #debt-cleanup-003 | docs(state): debt-cleanup-003 两个 P0 post-install bug 修复完成 |
| 16f0512 | #debt-cleanup-004 | fix(frontend): align CleanupStats/ScanProgress/AuditLogEntry to camelCase |
| 00015c0 | #debt-cleanup-004 | fix(tauri): classify rules update errors instead of collapsing to INTERNAL |
| （本次） | #debt-cleanup-004 | docs(state): debt-cleanup-004 三个二轮回归 bug 修复完成 |
| 72e2571 | #task-035 | chore(tauri): 接入 tauri-plugin-updater Rust 插件（Cargo.toml + lib.rs） |
| 7a2a4bf | #task-035 | chore(tauri): tauri.conf.json 加 updater 配置（pubkey + endpoint） |
| 1023a02 | #task-035 | feat(frontend): Settings 加「应用更新」卡片 + check/downloadAndInstall |
| 28c1f1a | #task-035 | ci: release.yml 加签名 key 注入 + latest.json 生成上传 |
| 4f7eb0a | #task-035 | fix(tauri): updater 配置移到 plugins.updater 命名空间并补 serde_json 依赖 |
| （本次） | #task-035 | docs(state): 任务035完成，tauri updater 自动更新就绪 |
| f3086b4 | #task-035 | fix(tauri): 放行 updater/process 插件 ACL 并补注册 process Rust 插件 |
| 5d93bb7 | #task-035 | test(frontend): 补 capabilities updater/process ACL 回归测试 |
| （本次） | #task-035 | docs(state): 修复 task-035 updater ACL P0 bug |
| 2d959b3 | #task-036 | feat(frontend): App 启动自动 check() 应用更新 |
| f460c42 | #task-036 | feat(frontend): 非阻塞 banner——新版本提示 + 立即走 downloadAndInstall/relaunch |
| （本次） | #task-036 | docs(state): 任务036完成，启动自动更新 banner 就绪 |
| eacc988 | #task-037 | feat(execute): safe_box.purge_item——删磁盘文件 + DELETE DB 行 + audit_log |
| 6dbb099 | #task-037 | feat(tauri): safe_box_purge 命令并注册 |
| 90ed07c | #task-037 | test(execute): safe_box_purge 集成测试（删文件+DB/缺文件/未找到） |
| 44f2c38 | #task-037 | feat(frontend): SafeBox 行加「永久删除」红色按钮 + confirm |
| （本次） | #task-037 | docs(state): 任务037完成，safe_box purge 就绪 |
| 81528dd | #task-038 | feat(execute): purge_many——循环 purge_item 收集成功/失败计数 |
| f0d8a50 | #task-038 | feat(tauri): safe_box_purge_many 命令并注册 |
| 8df9c70 | #task-038 | test(execute): purge_many 集成测试（全成功/部分失败/空 Vec） |
| 5895db0 | #task-038 | feat(frontend): SafeBox 加 checkbox 列 + 批量删除已选按钮 |
| （本次） | #task-038 | docs(state): 任务038完成，safe_box 批量 purge 就绪 |
| 4e3e0c6 | #task-039 | feat(execute): restore_many——循环 restore_item 收集成功/失败计数 |
| 577b87f | #task-039 | feat(tauri): safe_box_restore_many 命令并注册 |
| 1e35347 | #task-039 | test(execute): restore_many 集成测试（全成功/部分失败/空 Vec） |
| 2e5eff3 | #task-039 | feat(frontend): SafeBox 加「批量恢复已选」绿色按钮 |
| （本次） | #task-039 | docs(state): 任务039完成，safe_box 批量恢复就绪 |
| adcba07 | #task-039 | fix(execute): cargo fmt 对齐 purge_many/restore_many 签名与断言展开 |
| 54dd237 | #task-039 | test(execute): 补 restore_many 中间失败不阻断后续 id 用例 |
| de107af | #task-039 | fix(frontend): 批量操作进行中禁用行内恢复/永久删除按钮 |
| （本次） | #task-039 | docs(state): 评审前清障完成 |
| cbc5aaa | #task-039 | merge: task-039 safe_box 批量恢复（评审通过后 --no-ff 合并，双亲 2068fcd + daf7adf） |
| 882ed67 | #task-039 | docs(review): 任务039评审报告（真机冒烟 + 分支/主干 CI 实查） |
| （本次） | #task-039 | docs(state): 评审登记 2 条非阻塞已知问题与合并记录 |
| 2be95e1 | #task-041 | docs(task): 新增任务041-rules 超时收敛 + updater endpoint 修复 |
| 2e99f0e | #task-041 | fix(rules): reqwest 加 10s 超时 + 5s 连接超时，错误映射为中文可读文案 |
| 54d991b | #task-041 | test(rules): 补超时 + 404 可读错误文案测试 |
| bfab9a8 | #task-041 | fix(tauri): updater endpoint 改 Releases/latest/download URL |
| 6c3650a | #task-041 | fix(frontend): Settings 规则库卡片错误态按钮恢复为「重试」不卡转圈 |
| （本次） | #task-041 | docs(state): 任务041完成，rules 超时收敛 + updater endpoint 修复 |
| b64e376 | #task-042 | docs(task): 新增任务042-rules 离线模式 |
| 0cf879e | #task-042 | feat(rules): checkUpdate 返回 CheckUpdateResult 加 Offline 分支 |
| afbccff | #task-042 | test(rules): 补离线模式 / UpToDate / NewVersion 三分支单测 |
| 8ced0be | #task-042 | feat(frontend): Settings 规则库卡片按 status 渲染三态（绿/蓝/黄） |
| （本次） | #task-042 | docs(state): 任务042完成，rules 离线模式降级就绪 |
| 86bbb61 | #task-043 | docs(task): 新增任务043-restore 自动建父目录 |
| 6b65bcb | #task-043 | fix(execute): restore_item 在父目录缺失时自动 create_dir_all |
| 2e14188 | #task-043 | test(execute): 补 restore 自动建父目录单测（缺目录/已存在目录/不可用父路径） |
| ca936c1 | #task-043 | test(execute): 适配 task-043——旧 ParentMissing 用例改为正向自动建目录断言 |
| （本次） | #task-043 | docs(state): 任务043完成，safe_box restore 自动建父目录就绪 |
| 70452d0 | #task-044 | docs(task): 新增任务044-rules 手动回滚 |
| 3980490 | #task-044 | feat(rules): 新增 rollback + has_backup——从 .bak 恢复 cache_rules.json |
| 3cc1797 | #task-044 | feat(tauri): 新增 rules_rollback / rules_has_backup 命令并注册 |
| 262d7e9 | #task-044 | test(rules): 补 rollback 集成测试（有 bak/无 bak/损坏 bak/has_backup） |
| 8013dee | #task-044 | feat(frontend): Settings 规则库卡片加「回滚到上一版」按钮 |
| ff142be | #task-044 | style: cargo fmt |
| （本次） | #task-044 | docs(state): 任务044完成，rules 手动回滚命令就绪 |
| 8e56188 | #task-045 | docs(task): 新增任务045-safe_box 分页 |
| 252c87a | #task-045 | feat(execute): safe_box.list_paged——LIMIT/OFFSET + COUNT total + page_size clamp |
| 359a2de | #task-045 | feat(tauri): safe_box_list 入参加 page/page_size，返回 SafeBoxPage |
| 2a35eaf | #task-045 | test(execute): 补 safe_box 分页集成测试（首页/次页/clamp/越界/ID DESC） |
| 02b31bb | #task-045 | feat(frontend): SafeBox 表格下加分页栏（上一页/下一页/页码/total） |
| （本次） | #task-045 | docs(state): 任务045完成，safe_box list 分页就绪 |
| 734ab6b | #task-046 | docs(task): 新增任务046-safe_box session 过滤 |
| f7069f4 | #task-046 | feat(execute): list_paged 加可选 session_id 过滤 + 新增 list_sessions |
| 69e1f95 | #task-046 | feat(tauri): safe_box_list 入参加 session_id，新增 safe_box_list_sessions 命令 |
| 8effbc4 | #task-046 | test(execute): 补 session 过滤集成测试（全部/按 session/无匹配/list_sessions 分组） |
| a629150 | #task-046 | feat(frontend): SafeBox 加分页栏左侧 session 下拉（全部/session #N） |
| （本次） | #task-046 | docs(state): 任务046完成，safe_box session 过滤就绪 |
| 41afc60 | #task-047 | docs(task): 新增任务047-applyUpdate 确认弹窗 |
| 36c9f76 | #task-047 | feat(frontend): Settings 点「应用更新」前 window.confirm 嵌入版本号 |
| （本次） | #task-047 | docs(state): 任务047完成，rules applyUpdate 前端确认弹窗就绪 |
| （本次） | #task-048 | docs(state): 任务048真机验证通过，v0.1.1 updater banner 正确弹出，M2 关闭 |
| （本次） | #task-049 | docs(task): 新增任务049-RmShutdown 占用进程处置 |
| 20d1b04 | #task-049 | feat(engine): 新增 shutdown_locking_processes 与危险进程护栏（含 who_is_locking probe bug 修复） |
| 081d0ea | #task-049 | test(engine): 补 RmShutdown 护栏与无占用用例 |
| f3a6828 | #task-049 | feat(tauri): 新增 cleanup_retry_locked 命令 |
| 69b6b33 | #task-049 | feat(frontend): 清理失败展示占用进程并支持关闭后重试 |
| 7cfe53e | #task-049 | test(engine): 补关闭后重试删除的端到端用例 |
| 3b5cddd | #task-049 | test(engine): 输出 RetryLockedResp 真实 JSON 便于契约核对 |
| 1b467ba | #task-049 | docs(task): 补任务049实现备注 |
| （本次） | #task-049 | docs(state): 任务049完成，占用进程 RmShutdown 就绪 |
| 0129de9 | #task-049 | fix(frontend): 重试按钮纳入三态互斥（测试轮缺陷 1） |
| 8b84cf4 | #task-049 | test(frontend): 补 Items 清理失败与关闭重试交互用例（6 条） |
| f667c70 | #task-049 | test(engine): 补只关闭注册资源不误伤旁路文件的用例（B7） |
| be60f55 | #task-049 | merge: 测试期缺陷修复合回 feature（--no-ff） |
| （本次） | #task-049 | docs(test): 任务049测试报告 |

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
- 2026-10-13  task-034  评审前清障：独立复跑 clippy/fmt/test/pnpm lint/pnpm test/pnpm build 全绿（137 条后端测试无回退，前端 9 断言全过）；确认 release.yml 触发器 workflow_dispatch(inputs.version 必填)+push.tags=["v*"]、permissions contents:write、steps checkout→pnpm/action-setup→setup-node(node 22 cache pnpm)→dtolnay/rust-toolchain→pnpm install→tauri build→softprops/action-gh-release@v2 正确；release action 同时设 name+tag_name（workflow_dispatch 手动触发时 tag 也创建）、files 指向 workspace 根 target/release/bundle/、generate_release_notes:true；首次 v0.1.0 发版未代跑（无 gh CLI 鉴权，需评审在网页手动触发或 git push tag，第三档已登记）；tauri updater/代码签名/changelog 自动生成/设计图标/macOS-Linux 留 task-035+（第三档已登记）
- 2026-10-13  task-032  评审前清障：独立复跑 clippy/fmt/test/pnpm lint/pnpm test/pnpm build 全绿（137 条后端测试无回退，前端 9 断言全过：formatBytes 5 + formatTs 4）；确认 ESLint flat config（eslint.config.js）TS 文件 no-undef 关闭交给类型检查、react-hooks rules-of-hooks error/exhaustive-deps warn、Vitest jsdom 环境+setupFiles 配置正确、setupTests.ts 用 @testing-library/jest-dom/vitest 子路径导入、CI yml 加 Lint/Test 两步替换 echo 占位正确；Vitest 降级 2.1.9 兼容 Vite 5（Vitest 5 需 Vite 6+）；组件单测留 task-033+（第三档已登记）
- 2026-10-13  debt-cleanup-002  M1 NSIS 安装包装干净目录后 5 个回归 bug：Envelope 未解包白屏/数字空（api() 解包 + ErrorBoundary）、plugin:event|listen ACL 拒（capabilities/default.json 授权 core:default + core:event:default）、release.yml pnpm 版本双声明（删 with: version: 11）、resolve hop 在 CI 8.3 短名下误判 1（改 symlink_metadata 查 REPARSE_POINT）、Dashboard 重试扫描残留进度（catch 里 setProgress(null)）。分支 feature/fix-m1-packaging-bugs 等评审。
- 2026-10-14  debt-cleanup-003  NSIS 干净目录真机验收 2 个 P0：scan_start 前端漏传 scope 必填参数（传 { scope:'full', targetDrive:null }）、current_dir() 在 Program Files 只读导致 DB EACCES（新建 AppPaths State，data_root 走 app_data_dir()，rules/scripts 打包为 bundle.resources，首次复制 rules 到 data_root）。Bug-3（设置页错误信息坍缩）+ Bug-4（Dashboard 数字卡片空白）留 debt-cleanup-004。分支 feature/fix-m1-postinstall-v2 等评审。
- 2026-10-14  debt-cleanup-004  二轮真机验收 3 个回归：Dashboard CleanupStats/ScanProgress interface snake_case 与后端 serde camelCase 不匹配（数字卡片空白 + 进度文字卡死「准备中...」，顺带修 History AuditLogEntry）、rules_check_update/apply_update 所有错误坍缩成 INTERNAL（网络错→RULE_NETWORK_ERROR 3003，SHA 校验错→RULE_CHECKSUM_MISMATCH 3002）。已知遗留：后端 runner percent 统一发 0（单脚本内不知总量），进度条宽度不动留后续任务。同分支 feature/fix-m1-postinstall-v2 追加提交等评审。
- 2026-10-14  task-035  tauri-plugin-updater + 签名密钥对 + plugins.updater 配置 + Settings 应用更新卡片 + release.yml 签名/latest.json，M2 自动更新最小闭环。
- 2026-10-14  task-035  评审前清障：独立复跑 clippy/fmt/test/pnpm lint/pnpm test/pnpm build 全绿（137 条后端测试无回退，前端 9 断言全过，vite build 164.6KB gzip 52.5KB）；确认 tauri.conf.json plugins.updater 命名空间（active/endpoints/pubkey，非任务书顶层 updater）、lib.rs 挂 tauri_plugin_updater::Builder、Cargo.toml 加 tauri-plugin-updater=2 + serde_json=1、Settings.tsx 应用更新卡片 check/downloadAndInstall/relaunch 状态机正确（idle/checking/downloading/error）、release.yml Setup signing key 写 ~/.tauri/cclean.key + TAURI_SIGNING_PRIVATE_KEY_PATH 注入 + Generate latest.json 读 .msi.sig 生成 JSON 并随 Release 上传；私钥未入仓（git log -p 自审无 BEGIN PRIVATE）；GitHub Secret TAURI_SIGNING_PRIVATE_KEY 需用户手动配置（第三档已登记）；latest.json push 回 main 分支未自动化（raw URL 拿到的是占位，第三档已登记）；后台自动检查弹窗/增量更新/真机端到端升级验证留 task-036+（第三档已登记）
- 2026-10-14  task-036  启动自动 check + 非阻塞 banner 提示新版本，updater 最后一公里。
- 2026-10-14  task-036  评审前清障：独立复跑 clippy/fmt/test/pnpm lint/pnpm test/pnpm build 全绿（137 条后端测试无回退，前端 12 断言全过，vite build 165.8KB gzip 52.8KB）；修复 P1 bug——App.tsx banner 更新说明文字条件 `!bannerPhase` 恒 false（bannerPhase 初始值是字符串 "idle"，!"idle"=false），导致 update.body 永远不显示，改为 `bannerPhase === "idle"`；确认启动 useEffect 自动 check() 静默 catch、banner 三态（idle/downloading/error）、立即更新走 downloadAndInstall+relaunch、稍后按钮 setDismissed(true)、ErrorBoundary 包裹 tab 渲染正确；强制更新/下载进度条/定时轮询/"不再提示此版本"持久化/真机端到端升级验证留 task-037+（第三档已登记）
- 2026-10-14  task-037  safe_box.purge 单条永久删除（后端 purge_item + Tauri 命令 + 前端红色按钮 + confirm）。
- 2026-10-14  task-037  评审前清障：独立复跑 clippy/fmt/test/pnpm lint/pnpm test/pnpm build 全绿（140 条后端测试含 safe_box_purge 3/3，无回退，vite build 166.4KB gzip 53.0KB）；核验 purge_item 逻辑正确（查 in_box 行→删磁盘文件 NotFound 吞→DELETE DB 行→audit_log action='purge'）、Tauri 命令 safe_box_purge 入参 item_id i64 + Envelope<bool> 已注册 lib.rs、前端 purge 走 api() 解包 Envelope + window.confirm + 红色按钮 per-row purgingId 禁用；无第一档修复项；批量 purge/批量恢复/quota UI/真机端到端删文件验证留 task-038+（第三档已登记）
- 2026-10-14  task-038  safe_box.purge_many 批量永久删除（后端 batch + 前端 checkbox 列）。
- 2026-10-14  task-038  评审前清障：独立复跑 clippy/fmt/test/pnpm lint/pnpm test/pnpm build 全绿（143 条后端测试含 safe_box_purge_many 3/3，无回退，vite build 167.7KB gzip 53.3KB）；核验 purge_many 循环调 purge_item 单个失败不中断收集 errors、PurgeManyResult camelCase（deleted/failed/errors）与前端 interface 对齐、safe_box_purge_many 命令入参 itemIds Vec<i64> 已注册 lib.rs、前端 Set<number> 选中态 + 表头全选 toggle + 批量按钮 N=0 禁用 + confirm 确认 + 结果绿/红条 + 成功后清空 selected 重拉列表；无第一档修复项；批量恢复/进度条/跨页全选/真机端到端批量删验证留 task-039+（第三档已登记）
- 2026-10-14  task-039  safe_box.restore_many 批量恢复（复用 task-038 checkbox UI）。
- 2026-10-14  task-039  评审前清障：独立复跑 check/clippy/fmt/test/pnpm lint/fe-test/build 全绿（146 条后端测试含 safe_box_restore_many 3/3，无回退，vite build 168.5KB gzip 53.4KB）；核验 restore_many 循环调 restore_from_safe_box 单个失败不中断、新建 BatchOpResult{succeeded,failed,errors} camelCase 与 PurgeManyResult 解耦（purge 用 deleted、restore 用 succeeded）、safe_box_restore_many 命令入参 itemIds Vec<i64> 已注册 lib.rs、前端绿色「批量恢复已选(N)」按钮在红色批量删除左边 + 复用 selected Set + confirm + 结果绿/红条 + 成功后清空 selected 重拉列表；无第一档修复项；全部恢复/全部 purge 一键/进度条/跨页全选/真机端到端批量恢复验证留 task-040+（第三档已登记）
- 2026-10-15  task-039  评审前清障（二轮，清障 Agent）：① 第一档修复——`cargo fmt --all -- --check` 在 HEAD 与 main 上均红灯（task-038 提交的 `purge_many`/`insert_box_row` 多行签名 + 断言未展开，task-038 评审「fmt 全绿」系误报），已 cargo fmt 对齐并提交；② 第一档修复——SafeBox 行内「恢复 / 永久删除」按钮禁用条件漏了 `batchPurging`/`batchRestoring`，批量操作进行中仍可点单条按钮造成竞态，已补禁用；③ 第一档补测——`restore_many` 原 partial_fail 失败项在末尾，无法证明「单个失败不阻断后续 id」，补 `restore_many_failure_in_middle_does_not_stop_rest`（ParentMissing 夹中间，断言后续 id 仍成功）并修正 `BatchOpResult` 误导性 doc 注释；④ 第三档登记——批量恢复真机端到端未跑（无 GUI 数据行）。修复后 check/clippy/fmt/test（147 条）/pnpm lint/fe-test/build 全绿（vite build 168.51KB gzip 53.43KB）
- 2026-10-15  task-039  评审通过：真机冒烟（pnpm tauri dev + WebView2 CDP 挂载 9222）逐个点过五 tab 并造数据跑通「勾选 → 批量恢复已选 → confirm → 绿/红条 → 清空 selected 重拉列表」全链路（3 条：2 成功回原路径 + 1 失败父目录缺失保留 in_box），文件/DB/audit_log 三方一致，本任务页面 Console 0 红错 0 exception；回归批量删除正常；本地门禁独立复跑全绿（fmt/clippy/147 后端测试/lint/12 前端断言/build 168.51KB gzip 53.43KB）；**CI 实查**：分支 run 38671216149（PR #2）绿，合并前 main 2068fcd run 38571515030 因 task-038 fmt 问题红（本分支 adcba07 已修），合并后 main run 38673720322 绿；merge --no-ff → `cbc5aaa`，feature 分支已删；新增登记 2 条非阻塞已知问题（设置页首次 rules_check_update 红错、规则检查无超时收敛，均非本任务引入）
- 2026-10-16  task-041  rules reqwest 超时收敛 + updater endpoint 改 Releases 资产（task-039 评审 P0 真机卡死修复）
- 2026-10-16  task-042  rules.checkUpdate 加 Offline 三态，断网时本地规则库照常工作 + UI 黄条提示
- 2026-10-16  task-043  safe_box.restore_item 自动 mkdir -p 父目录（task-017 已知问题闭环）
- 2026-10-16  task-044  rules_rollback 单版回滚（后端命令 + 前端灰色按钮 + confirm）
- 2026-10-16  task-045  safe_box_list 分页（LIMIT/OFFSET + total + 前端分页栏）
- 2026-10-16  task-046  safe_box list session 过滤（可选 session_id 参数 + list_sessions + 前端下拉）
- 2026-10-16  task-047  rules.applyUpdate 前端 window.confirm 弹窗（嵌入版本号 + 回滚提示）
- 2026-10-23  task-048  v0.1.1 真机升级验证通过（banner 弹出 v0.1.1 + 设置页应用更新卡片一致），M2 正式关闭
- 2026-10-23  task-049  派发占用进程 RmShutdown（who_is_locking 升级为可请求关闭 + 前端占用提示/一键重试）
- 2026-10-23  task-049  RmShutdown 落地：shutdown_locking_processes + 危险进程护栏 + cleanup_retry_locked + Items 失败明细/一键关闭重试；顺带修复 who_is_locking probe 判断（HRESULT vs Win32 码）导致占用列表恒空的 bug；后端全链路真机 PASS，GUI/安装态冒烟因环境限制留测试 Agent
- 2026-10-24  task-049  测试轮：全部门禁独立重跑（cargo 183 / vitest 34 / build 176.39 kB）；修 C12 三态互斥缺陷（0129de9）+ 补 B7 隔离真机用例（f667c70）；NSIS 干净目录安装 + 数据目录隔离 + 安装目录零写入通过；GUI 点击/Console 因无活动桌面（WebView2 渲染进程崩溃）未取得，登记第三档；新登记预存在问题「打包资源路径错位」；状态置 awaiting_review
