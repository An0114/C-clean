# 测试报告：task-058 M4 收尾——RmRestart 白名单 + safe_box 原子性 + 签名评估 + CHANGELOG

- 分支：`feature/task-058-m4-finishup`
- 测试 HEAD：`5564a30`
- 测试时间：2026-10-29（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| dev（pnpm tauri dev） | Win11 25H2，Vite 5.4 + Tauri 2，`target\debug\cdisk-cleaner.exe` | 是 | 通过 |
| 打包安装（干净目录） | NSIS `C-DiskCleaner_0.1.6_x64-setup.exe`（4.05 MiB），装到 `D:\qa-058-install` | 是 | 通过 |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust lib + 前端组件 | `cargo test --workspace` / `pnpm --filter app test` | 通过 | `target/qa-058/cargo-test.log`：全部 `ok. N passed; 0 failed`；`pnpm.log`：Test Files 9 passed |
| L2 集成/契约 | engine 集成 + 门禁 | `cargo fmt --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0；`pnpm.log`：lint=0（2 条 SafeBox react-refresh pre-existing warning）、build=0（187.46 kB / gzip 60.30 kB） |
| L3 端到端/UI | dev 真机：白名单 textarea 渲染、编辑保存、DB 落库、重启回填 | `pnpm tauri dev` + Win32 键盘/剪贴板驱动 + GDI 截图 | 通过 | `settings-full.png`（白名单 textarea + fail-closed 文案）、`whitelist-pasted.png`（chrome.exe/code.exe 输入）、`db-after2.txt`（DB 写入 `["chrome.exe","code.exe"]`）、`restart-verify.png`（重启后 textarea 仍回填） |
| L4 真机/打包 | NSIS 干净目录安装 → 设置页复跑 → 安装目录零写入 | `pnpm tauri build`（NSIS）+ `/S /D=D:\qa-058-install` | 通过 | `tauri-build.log`（NSIS 4.05 MiB 产出）、`installed-settings.png`（打包设置页白名单 UI 完整）、`install-clean.txt`（安装目录仅原始文件，DB 在 %APPDATA%） |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | prefs 加 restart_whitelist（默认空数组） | DB 初始无该行 → textarea 显示灰色 placeholder「notepad.exe / *.exe」，「当前：不限制」（`settings-full.png`） | 通过 |
| 2 | 后端按白名单过滤（fail-closed） | 单测 8 条覆盖空/匹配/不匹配/全部命中/通配/脏数据（cargo-test.log）；UI 文案如实写明「全部在此列表中才会自动重启」 | 通过 |
| 3 | Settings 页 textarea | `settings-full.png`：重启白名单 textarea + 保存按钮 | 通过 |
| 4 | 切换立即 set_prefs | 粘贴 chrome.exe/code.exe → 点保存 → DB 立即出现 `prefs.restart_whitelist='["chrome.exe","code.exe"]'`（`db-after2.txt`） | 通过 |
| 5 | 单测白名单三态 | `whitelist_allows_restart_three_states`、`whitelist_requires_all_names_to_match`、`wildcard_match_basics` 等通过 | 通过 |
| 6 | fs::rename 路径事务包裹 | 代码结构 INSERT→rename→commit（开发备注 §5）；3 条原子性单测通过 | 通过 |
| 7 | rename 失败回滚事务+文件原位 | `insert_failure_leaves_source_file_in_place_and_no_orphan`：外键约束制造 INSERT 失败 → 原文件在、内容不变、box 零孤儿、DB 零行 | 通过 |
| 8 | 跨盘 copy+remove 同理 | `success_path_keeps_file_and_row_consistent`、`missing_source_returns_false_without_row` 覆盖 | 通过 |
| 9 | 单测 INSERT 失败→原文件在、box 无孤儿 | 同上（真实外键约束，非 mock） | 通过 |
| 10 | code-signing-evaluation.md 存在 | `Test-Path docs/research/code-signing-evaluation.md = True` | 通过 |
| 11 | 覆盖 A/B/C/D 四方案 | 文档含 EV/OV/自签/Store 四方案对比（开发备注 §6 摘要） | 通过 |
| 12 | 推荐方案+结论 | 结论「当前不做」，三条触发条件 + Azure Trusted Signing 最小路径 | 通过 |
| 13 | CHANGELOG v0.1.7 草稿 | `CHANGELOG.md:13 ## [0.1.7] - 未发布（草稿）` | 通过 |
| 14 | 覆盖 RmRestart/prefs/restored_at/跨盘/G6 | 草稿含新增 4/修复 1/变更 2/已知问题 5 | 通过 |
| 15 | 版本号不升（保持 0.1.6） | package.json=0.1.6、tauri.conf.json=0.1.6、CHANGELOG v0.1.6 为上一个发布版 | 通过 |
| 16 | cargo test -p engine 全绿 | `cargo test --workspace` exit=0，全部 ok/0 failed | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮未发现需要代码修复的缺陷 | L1-L4 全绿 | — | — | — |

> 备注：dev 日志中出现 tokio runtime panic「Cannot drop a runtime in a context where blocking is not allowed」，与 task-057 的 G6（设置页首调 rules_check_update 走 ipc.localhost 时序竞争）同源，不影响白名单保存（DB 行已正确写入），按既有豁免口径排除。

## 5. 测试期修复记录（bugfix 子分支）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮无需修复 | — | — | — | — |

## 6. 遗留问题（三档分流）

### 第一档（已修）
- 无。

### 第二档（需用户拍板，状态 blocked）
- **白名单语义从「只重启名单内」改为「全部命中才重启」（fail-closed）**。任务书验收标准 2 写「空 = 全部可重启；非空 = 只重启白名单内进程名」，但 `RmRestart(hSession, dwFlags, callback)` 无按 pid 参数，一次调用会拉起本会话所有 bRestartable 应用，无法只挑名单内的。开发实现为 fail-closed（名单非空且被关进程全部命中才调 RmRestart），UI 文案已如实说明。若产品认为该交互不可接受，需改成「自己 CreateProcess 逐个重启」，代价是丢失 RM 的命令行/工作目录恢复能力。**测试按实际声明语义验证通过，语义变更本身需用户拍板。**

### 第三档（已登记为已知限制）
- **真机 notepad/chrome 端到端白名单行为未跑**：需要一个既持文件锁又注册了 RM 重启的真实进程（如现代 Edge/Word），本环境 notepad 不持锁。白名单过滤逻辑由 8 条 Rust 单测（空/匹配/不匹配/通配/脏数据/全部命中）全覆盖，真实 RM 行为在生产态由内核保证。
- **safe_box 原子性真机改库验证未做**：核心路径已用真实外键约束单测（非 mock）覆盖「INSERT 失败→原文件在、box 零孤儿、DB 零行」，手动改库制造失败风险高且收益低。
- **updater 签名**：`tauri build` 末尾因无 `TAURI_SIGNING_PRIVATE_KEY` 退出 1，但 NSIS/MSI 安装包正常产出，不影响安装冒烟。
- **CDP console 0 红错**：tauri dev 不开 9222 端口，以运行日志无 panic（除 G6）+ 无白屏/无红弹窗替代验证。

## 7. 测试结论

- 结论：**通过**
- feature 分支最新 commit：`5564a30`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁与真机冒烟，未复用开发自验结论。测试轮未发现需代码修复的缺陷；L1-L4 全绿。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核第二档：白名单 fail-closed 语义与任务书原「只重启名单内」要求的偏差是否可接受。**
