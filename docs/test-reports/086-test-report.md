# 测试报告：task-086 v0.6.0 PRD 全闭环 + 安全加固 + 体验深化（29 项大合并）

- 分支：`feature/task-086-v060-prd-closure-hardening`
- 测试 HEAD：`7fc7d87`（评审报告提交后）
- 测试时间：2026-11-13（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）
- 任务性质：6 组 29 项打包（PRD 闭环 3 / 安全加固 3 / 体验深化 5 / 工程化清理 4 / P2 长尾 8 / 文档合规 6）；命中「前端任务判定清单」+「安全红线」，真机/打包冒烟任务书标 **必做、不可裁剪**
- 本次为评审打回后的**独立补测轮**：评审报告 `docs/reviews/task-086-...-review.md` 列 5 项缺口（#1 报告缺失阻塞 / #2 NSIS 安装冒烟高 / #3 通知历史中 / #4 右键菜单中 / #5 200% 缩放中），本报告逐条闭环或正式降级登记

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| 构建门禁 | `cargo fmt/clippy/test` + `pnpm lint/test/build` | 是 | 通过 |
| CLI 无头真机 | `c-clean-cli` version / help / list-rules / scan json | 是 | 通过 |
| GUI dev 真机 | `pnpm tauri dev` | 部分（后端起 + WebView2 附着；窗口锁屏态不可交互） | 后端证据见 §2，UI 交互归第三档 |
| 打包 + 安装冒烟 | NSIS 干净目录静默安装 → 安装态跑通 → 卸载 | **是**（缺口 #2 闭环） | 通过 |
| 通知历史真机 | 安装态 `--auto-scan` 无头触发 → DB 落库核验 | **是**（缺口 #3 闭环） | 通过 |
| Explorer 右键点击 | 桌面人工右键 | 否（锁屏态不可人工交互） | 第三档 + 替代证据 |
| 200% 全页截图 | 其余 7 页 200% | 否（锁屏态 + CDP 未开） | 第三档 + 替代证据 |
| ARM64 产物 | CI runner | 否（本地 x64） | 归 CI 矩阵 |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区 | `cargo test --workspace` | 通过 | `cargo-test.log`：63 batches / **769 passed / 0 failed / 15 ignored**（与 dev 自报 769 一致；任务书下限 ≥688，超 81） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm test` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（输出 `cdisk-cleaner v0.6.0` + `c-clean-cli v0.6.0` + elevation v0.1.0）；`pnpm.log`：lint=0（4 条既有 react-refresh warning，`--max-warnings 10` 通过）、test=0（**17 files / 233 passed**，与 dev 一致；任务书下限 ≥224，超 9）、build=0（**282.59 kB / gzip 86.99 kB**） |
| L3a CLI 无头真机 | 终端契约 | `--version` / `--help` / `list-rules` / `scan --format json --limit 1` | 通过 | `cli-smoke.log`：①`c-clean-cli 0.6.0`；②5 子命令（scan/dry-run/clean/list-rules/help）+ 7 选项 + 退出码 0/1/2；③内置规则库 v1.3.0 **13 条**（L1/L2 分级），用户规则 0 条；④scan 临时 data-dir：session=1，**7661 项 / 701.2 MB**，JSON 字段 `items[].module/path/riskLevel/sizeBytes` + `shown/totalBytes/totalItems` 正确，`id` 不外泄，exit=0 |
| L3b GUI 后端 | `pnpm tauri dev` | 后端启动 + 路径 + 进程存活 | 通过（后端层） | `tauri-dev.log`：`[paths] data_root / rules_dir / scripts_dir / box_dir` 四路径正确；主进程存活，空闲 WS **49.3 MB**；WebView2 子进程（pid 16332）附着，`--webview-exe-name=cdisk-cleaner.exe --webview-exe-version=0.6.0`，user-data-dir 为本 app EBWebView；**锁屏态下窗口句柄/标题枚举为空，UI 交互与截图不可取证**（见第三档） |
| L3c 通知历史真机 | 安装态 exe `--auto-scan` 无头 | 扫描 → notify → DB 落库 | **通过（缺口 #3 闭环）** | `notify-smoke.log` + `db_after.py`：stderr `[auto-scan-task] 计划任务完成 session=7 ok=true items=7784 bytes=833827339`；DB `notification_history` 从 **0 → 1**，条目 `title=C-Disk Cleaner 自动扫描`、`body=自动扫描完成：发现 7784 项可清理，约 795.2 MB（仅扫描，未删除）`、`read=0`；测试后 `auto_scan_enabled` 已恢复 0 |
| L4 打包 | NSIS + MSI 出包 | `tauri build` | 通过 | `tauri-build.log`：**NSIS `C-DiskCleaner_0.6.0_x64-setup.exe`（4.65 MiB / 4,877,675 B）** + MSI `C-DiskCleaner_0.6.0_x64_en-US.msi`（6.78 MiB / 7,110,656 B）；签名 exit=1 为已知预期（私钥在 CI secret） |
| L4b 安装冒烟 | 干净目录静默安装 + 卸载 | NSIS `/S /D=<dir>` → 核验产物 → `uninstall.exe /S` | **通过（缺口 #2 闭环）** | `install-smoke.log` / `install-uninstall.log`：安装 **exit=0**；干净目录产物 `cdisk-cleaner.exe`（19,157,504 B）/ `cclean-elevation.exe`（285,184 B）/ `uninstall.exe`（79,404 B）/ `rules\cache_rules.json`（6,577 B，规则库 v1.3.0）+ `rules_meta.json`（259 B）；安装态 exe `FileVersion/ProductVersion = 0.6.0`；卸载 **exit=0**，安装目录整体清除、无注册表残渣（`HKCU\...\Directory\shell\CClean` 仍为未注册） |

## 3. 验收标准逐条核对（29 项）

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | 报告 HTML/CSV 导出到指定路径 | `commands.rs:947 report_export`（async）+ `:1001 report_export_tests`（路径解析/HTML/CSV/文件名清洗）；`Items.tsx` 导出按钮与结果条；`lib.rs` 已注册 `report_export` | 通过（单测 + 接线；真机导出对话框归第三档） |
| 2 | 计划任务频率 + 阈值（默认 5GB） | `auto_scan.rs` 读 `prefs.auto_scan_interval` / `auto_clean_l1_threshold`；prefs 单测覆盖四档位 + 默认 5GiB；Settings 频率 30min/1h/天/周/月 + 阈值 UI；DB 实测 `auto_scan_interval=daily` | 通过（链路单测 + 配置落库；真机改配置跑一轮归第三档） |
| 3 | 目标推荐组合 + 差额提示 | `recommend.rs:47 recommend_plan` + `pick` 贪心（L1 累加不足纳入 L2，L2 按大小降序）；单测 `l1_only_reaches_target` / `l1_insufficient_includes_l2` / `l2_picked_in_size_desc_order`；前端 `PlanTemplates.test.tsx` 17 条 | 通过 |
| 4 | 进度条随模块推进（非恒 0%） | `runner.rs` `percent_span` / `interpolate_percent_*`（task-060 既有）+ `engine/tests/scan_progress_percent.rs`；前端进度条宽度绑定 | 通过（既有实现复核） |
| 5 | 符号链接/junction 拦截（code 2009） | `executor.rs:225` 判最终组件本身 reparse point；`:230` 审计 `SymlinkRefused`；`:237` 返回 `code 2009`；`resolve.rs:47` `FILE_ATTRIBUTE_REPARSE_POINT 0x400`；dry_run 预检同拦；负向单测存在 | 通过（单测 + 代码走查；真机删 symlink 负向用例归第三档） |
| 6 | 8.3 短名解析 `SYSTEM~1→System32` | `resolve.rs:60 GetLongPathNameW` 展开；单测 `long_path_name_roundtrips_short_to_long`；展开失败不拒绝合法清理（增强项） | 通过 |
| 7 | History 折线图 7/30/90 天 | `Dashboard.tsx` `TrendChart` SVG polyline（task-073 既有）；`Dashboard.test.tsx` 三档切换 + `data-testid=trend-line` | 通过（既有实现复核） |
| 8 | BigFiles 矩形树图 | `app/src/utils/treemap.ts`（4,348 B 纯函数）+ 6 条单测；`BigFiles.test.tsx` 切「文件夹分布」→ treemap | 通过 |
| 9 | 影响说明规则库热更新 | `runner.rs:237/238/272/283` `impact_note` / `rebuild_note` 入库；`temp_scan_test.rs` 校验文案来自规则库；`cache_rules.json` 带 impact 字段 | 通过 |
| 10 | 高分屏 150%/200% 清晰 | CSS `clamp` + rem 响应式；12 张截图在 150% 采集；`10-dashboard-200pct.png` 为 200%；**200% 其余 7 页未采**（缺口 #5，第三档） | 部分通过（150% 全页 + 200% 仪表盘；200% 全页第三档） |
| 11 | 更新下载进度反馈 | `App.tsx` chunk 累计 / 百分比 / 速度 EMA（task-055 既有）+ 既有测试 | 通过（既有实现复核） |
| 12 | release.yml `target_commitish` | `.github/workflows/release.yml` `target_commitish: ${{ github.sha }}`（task-054 既有，注释标注来源） | 通过 |
| 13 | 审计 session_id 会话筛选 | v8 `v8__audit_log_session.sql` 加 `session_id`；`audit_log_list` 接入参；History 下拉；`04-history.png` | 通过 |
| 14 | ps1 删除 + 回退清理 | `scripts/` 实查仅剩 `.gitkeep` / `build-elevation.mjs` / `PROTOCOL.md` / `release-lib.*` / `tauri-resources.test.mjs`，**无 `scan_*.ps1`**；`CCLEAN_RUST_*` 回退链路无残留逻辑；CLI/auto-scan 日志显示模块均「走 Rust 实现」 | 通过 |
| 15 | DenialReason 来源拆分 | `whitelist.rs:53/55/57` `BuiltinRedline(BuiltinRedlineReason)` / `ExternalRedline` / `UserRule`；`builtin()` 构造器；来源单测 | 通过 |
| 16 | 跨盘恢复 copy+remove fallback | `execute/safe_box.rs:218` 注释「跨盘时 `move_path` 退回 copy+remove」；`:219` 调 `move_path`；目录递归、拒绝 reparse 穿透；`engine/tests/safe_box.rs` 3,902 B 覆盖 | 通过（单测 + 走查；真机 C→D 恢复归第三档） |
| 17 | RmShutdown 等待进度事件 | task-049b/060 既有 1s ticker（优于任务书 5s）；前端等待状态 | 通过（既有实现复核） |
| 18 | 兄弟进程连带风险提示 | confirm 弹窗文案「同一控制台内的相关进程可能一并关闭」；`Items.test.tsx` 断言 | 通过 |
| 19 | UNC 自定义规则 L2 | `scan/modules/user_rules.rs:99` 盘符识别不出来（UNC）不跳过；`:135` 规范化失败不放行；`:262` 断言用户规则条目全为 L2；内置规则不扫网络盘 | 通过（单测 + 走查） |
| 20 | 增量扫描 | v9 `v9__longtail.sql` `scan_cache`；`scan/cache.rs`（7,366 B）按「模块 + 根目录 mtime」签名；DB 实测 `scan_cache` **5 行**；`browser_cache` / `software_cache` 刻意不缓存（正确性优先） | 通过（取舍已登记；二次扫描提速不显著，观察项） |
| 21 | 多语言英文 | `i18n/zh-CN.json` / `en-US.json` 各 **29 keys** 对齐 + `index.ts` + `index.test.ts` 5 条；Settings 语言卡；`09-settings-en-us.png` 英文导航 | 通过 |
| 22 | 右键菜单集成 | `context_menu.rs`（6,626 B）：`install`/`uninstall`/`is_installed`/`build_command`/`collect_items`；4 条单测 + `lib.rs` 内 `--scan-path` 解析 2 条；IPC `context_menu_status` / `context_menu_set_enabled` / `context_scan_path` / `take_pending_scan_path` 已注册；**Explorer 真机右键点击未取证**（缺口 #4，第三档） | 部分通过（链路/单测/接线齐备；人工点击第三档） |
| 23 | 通知中心历史 | `notification_history.rs`（5,776 B）`record/list/unread_count/mark_all_read/clear` 5 条单测；`auto_scan::notify` 先落库再 toast；**真机落库 1 条已核验**（见 L3c，缺口 #3 闭环） | 通过 |
| 24 | SECURITY.md | `docs/SECURITY.md` 6,255 B：杀软误报说明 + 安全披露流程 | 通过（文档层） |
| 25 | CONTRIBUTING.md | `CONTRIBUTING.md` 5,440 B：环境 / 代码风格 / 安全红线 / PR 流程 | 通过（文档层） |
| 26 | 测试覆盖率 Rust ≥80% / 前端 ≥70% | **未在本测试轮独立复跑 llvm-cov / coverage-v8**（耗时长，归评审引用 dev 数据）：dev 报 engine 行覆盖 84.4%（函数 79.8%）、engine+cli+elevation 81.7%、全 workspace 71.9%（欠账在 `commands.rs` 1318 行命令薄封装）；前端 statements/lines 86.49%、branches 82.74%、functions 74.21% | 观察项（口径见 §6 第二档） |
| 27 | 性能基准 ≤180s / RSS | **未在本测试轮独立复测基准**：dev 报 GUI 全扫 5s / Rust 峰值 51.1 MB / 空闲 47.8 MB；CLI 11.6s / 16.1 MB；WebView2 子进程树空闲 ~435 MB、峰值 ~508 MB（Chromium 固有开销）。本轮 CLI 无头扫描 7661 项跑通、GUI 后端空闲 WS 49.3 MB 与 dev 口径吻合 | 观察项（整组口径见 §6 第二档） |
| 28 | 8 页 GUI 截图 | `docs/screenshots/` **12 张**全部非空（35.5–98.8 KB）：8 页核心 + 语言/右键通知/英文/200% 补充；采集方式为真实窗口 `PrintWindow`（dev 备注） | 通过（150% 全页；200% 全页第三档） |
| 29 | 不回归：cargo ≥688 / pnpm ≥224 | 实测 **769 passed**（超下限 81）/ **233 passed**（超下限 9），双 0 failed | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮未发现需代码修复的阻断缺陷 | L1/L2 全绿 + CLI/安装/通知链路真机跑通 + NSIS 装卸闭环 | — | — | — |

## 5. 测试期修复记录（bugfix）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| — | 本轮缺口以补测 / 取证为主，测试轮未发现代码缺陷，未开 bugfix 子分支 | — | — | — | — |

## 6. 遗留问题（三档分流）

### 第一档（本轮已取证 / 闭环）
- **缺口 #1 独立测试报告**：本报告 `docs/test-reports/086-test-report.md` 即独立测试轮交付物；状态由 `testing` 置 `awaiting_review`（见提交）。
- **缺口 #2 NSIS 干净目录安装冒烟**：真机闭环——静默安装 exit=0、产物齐全、安装态版本 0.6.0、安装态无头 auto-scan 跑通、卸载 exit=0 无残渣。
- **缺口 #3 通知中心真机历史**：真机闭环——安装态 `--auto-scan` 触发，`notification_history` 落库 1 条（7784 项 / 795.2 MB，未读），DB 行级证据；写入链路另有 5 条单测兜底。
- **双回归数字达标**：Rust 769 / 前端 233，双 0 failed。
- **CLI 无头契约**：版本 0.6.0、规则库 v1.3.0 13 条、JSON 字段契约正确、无 id 外泄。

### 第二档（需评审 / 用户拍板）
1. **覆盖率口径**：Rust 以 engine 口径 84.4% 判 ≥80% 达标；全 workspace 71.9%，欠账集中在 `commands.rs` 的 `#[tauri::command]` 薄封装（1318 行未覆盖，参数/状态依赖 AppHandle，单测成本高）。建议后续补命令层契约测试。
2. **整组内存口径**：Rust 进程口径达标（空闲 47.8 / 峰值 51.1 MB）；WebView2 子进程树空闲 ~435 MB、峰值 ~508 MB，超 PRD 300 MB 整组目标。此为 Tauri + WebView2 架构固有开销，单纯优化 Rust 无法达成；**需在 v0.6.0 发版说明中明确「Rust 进程口径 vs 任务管理器整组口径」**。
3. **增量缓存收益有限**：`browser_cache` / `software_cache` 刻意不缓存（安装根 mtime 不随缓存文件变化，启用会长期返回旧结果）；命中的 temp/prefetch/dev_cache/windows_update 占比小，二次扫描未显著提速。正确性优先取舍，非缺陷。
4. **实现偏差（dev 已登记，测试复核属实）**：①1.2 复用既有 `prefs` KV，未新建任务书预期的 v8 迁移（v8 让给 audit_log.session_id）；②4.1 `target_commitish` 为 task-054 既有，本分支 release.yml 改动是 ARM runner 名 + 去 `scripts/` 分发；③组 5 新增独立模块 `cache.rs` / `notification_history.rs` / `context_menu.rs` + v9 迁移；④`@vitest/coverage-v8` 为覆盖率测量新增 devDependency（不进产物）。

### 第三档（环境不可得，正式登记 + 替代证据）
- **缺口 #4 Explorer 右键菜单人工点击**：当前会话为锁屏态，无法在资源管理器人工右键触发「用 C-Disk Cleaner 清理此文件夹」；应用 `install()` 写注册表也需 app 在线调 IPC，锁屏窗口不可交互。**替代证据**：`build_command` 引号/占位符单测、注册表键路径 per-user 单测、受保护目录拒绝单测、临时目录 L2 扫描单测共 4 条 + `--scan-path` 解析 2 条；`collect_items` 复用常规扫描与同一套红线判定；4 个右键相关 IPC 已注册并出现在 Envelope 契约；安装态 exe 就位。**留待有活动桌面的机器补人工右键取证。**
- **缺口 #5 200% 缩放其余 7 页**：切换系统缩放需注销会话；CDP 调试端口未开，无法程序化导航补采。**替代证据**：150% 下 8 页核心截图已存（12 张）；仪表盘 200% 1 张；CSS `clamp`/rem 响应式 + 前端 233 passed 兜底。**留待有活动桌面的机器补其余 7 页 200% 截图。**
- **GUI 窗口 UI 交互 / CDP console 0 红错**：后端 Rust 壳与 WebView2 子进程已附着，但锁屏态窗口不可见、不可点击；前端交互以 233 条 vitest + 12 张既有截图为证据，未抓 CDP console。
- **真机端到端 UI 项**：报告导出对话框、Settings 改频率/阈值后按新配置跑、20GB 推荐勾选、英文界面切换目视、跨盘 C→D 恢复、UNC 规则添加、symlink 删除负向用例——均以单测 + 代码走查 + 无头链路证据替代，留活动桌面补做。
- **系统 toast 目视**：应用内通知历史已落库并核验；Windows 通知中心弹窗在锁屏态未目视确认（异步投递，dev 备注已给 2s 等待）。
- **ARM64 产物**：本地 x64，由 CI 矩阵（task-084 C3）产出。
- **代码签名**：死路不做（本地签名 exit=1 已知预期；SmartScreen 警告可接受，CHANGELOG 已明写）。
- **CI 五道闸**：PR #51 run **43431889382** 评审已核 conclusion **success**（windows-latest：fmt / clippy -D warnings / cargo test / pnpm lint+test+build / tauri build + 产物校验 / 规则库 checksum）；本测试轮未重复跑 CI。
- **长期遗留**：`commands.rs` 命令层契约测试欠账；规则库 `modules` 段双份真值源；Items/SafeBox/History 深色适配；`tasks/060-shutdown-experience.md` 工作区已删除但 HEAD 存在。

## 7. 测试结论

- 结论：**有条件通过**——L1/L2 门禁全绿（769 passed / 233 passed / build 282.59 kB）；L3 无头 CLI、后端启动、通知落库、L4 打包 + 干净目录安装/卸载全部真机跑通；评审 5 项缺口中 #1（报告）、#2（安装冒烟）、#3（通知历史）本轮已闭环，#4（右键人工点击）、#5（200% 全页）正式登记第三档并附替代证据。**是否合并 main 由评审 Agent（提示词 10）判定，测试 Agent 不下合并结论。**
- feature 分支最新 commit：`7fc7d87`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮**独立重跑**全部门禁（fmt / clippy / cargo test / lint / vitest / build）+ CLI 无头真机 + 安装态 auto-scan 落库核验 + NSIS 干净目录安装/卸载 + 文件层与代码走查，未复用开发自验结论；覆盖率与性能基准两项耗时指标未独立复测，如实标注为 dev 自报 + 评审观察项。
- **新规执行说明**：本任务含大量前端改动（导出按钮 / 频率阈值 UI / 目标输入 / 折线图 / treemap / i18n / 右键菜单 / 通知中心），按 2026-11-03「有前端更改必须真机测试，不可跳过」指示，测试轮已起 `tauri dev` 取证后端与 WebView2 附着；受锁屏态环境限制，窗口级 UI 交互与截图归第三档并逐条给出替代证据，未以「CI 绿 / 单测过」通融跳过。
- 移交说明：报告随单独 commit 推送到 `feature/task-086-v060-prd-closure-hardening`，`PROJECT_STATE.md` 状态置 `awaiting_review`。**请评审重点复核：①缺口 #2 安装冒烟本轮真机闭环（装 exit=0 / 卸 exit=0 / 安装态跑通 / 无残渣）；②缺口 #3 通知历史 DB 行级证据（0→1）；③#4/#5 第三档替代证据是否充分；④覆盖率与整组内存两口径是否需在发版说明明示；⑤29 项中以单测 + 走查替代真机 UI 的条目清单（§3 标「第三档」处）。**
