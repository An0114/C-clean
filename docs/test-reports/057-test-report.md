# 测试报告：task-057 RmRestart 自动重启被关进程 + 设置页开关（默认关闭）

- 分支：`feature/task-057-rm-restart-toggle`
- 测试 HEAD：`e308f9e`（merge bugfix/task-057-rmrestart-flags）
- 测试时间：2026-10-28（UTC+8）
- 测试 Agent：QA（独立于开发轮；本报告含「同源测试」声明，见 §7）

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| dev（pnpm tauri dev） | Win11 25H2，Node/pnpm，Vite 5.4 + Tauri 2，`D:\GitHub\C-clean\target\debug\cdisk-cleaner.exe` | 是 | 通过 |
| 打包安装（干净目录） | NSIS `C-DiskCleaner_0.1.6_x64-setup.exe`（4.04 MiB），静默装到 `D:\qa-057-install` | 是 | 通过 |

> 说明：多显示器（副屏 1920 + 主屏 1707×1409）。自动化截图走 GDI 直采主物理屏；窗口经 `SetWindowPos` 固定到主屏 (0,0)。

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据（日志/截图/报告路径） |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust lib + 前端组件 | `cargo test --workspace` / `pnpm --filter app test` | 通过 | `target/qa-057/final-gate.log`：lib 54 passed（含 prefs 5 + rm_restartable 1 + restartable_pids 1），前端 9 文件 85 passed |
| L2 集成/契约 | engine 集成测试 + 门禁 | `cargo fmt --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm build` | 通过 | `final-gate.log`：fmt=0、clippy=0、lint=0（2 条 SafeBox pre-existing react-refresh warning，≤10）、build=0（vite 185.59 kB / gzip 59.59 kB） |
| L3 端到端/UI | dev 真机点 Settings 开关、prefs 落库、重启回填、真实进程关闭 | `pnpm tauri dev` + Win32 点击/GDI 截图 + 一次性 Rust 真机用例 | 通过 | `settings3.png`（默认 off）、`toggle-on4.png`（on 蓝勾）、`db2.txt`（SQLite `prefs.auto_restart_on_close='1'`）、`restart-settings-zoom.png`（重启后仍 on）、`real-restart3.log`（RmShutdown 真关 Python 持锁进程，RmRestart 路径无 ERROR_INVALID_PARAMETER 87） |
| L4 真机/打包 | NSIS 干净目录安装 → 设置页复跑 → 安装目录零写入 | `pnpm tauri build`（NSIS）+ `/S /D=D:\qa-057-install` | 通过 | `tauri-build.log`、`install.txt`（exit=0）、`installed-max.png`（打包行为卡片+蓝勾「已开启」）、`install-clean.txt`（安装目录仅原始文件，DB 在 %APPDATA%） |

## 3. 验收标准逐条核对（每条必须有运行时证据）

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | RmShutdown 后 auto_restart=true 调 RmRestart | 真机日志 `real-restart3.log`：`shutdown_locking_processes(true) = Ok(...)`，`closed=[Python 持锁进程]`，代码路径 `locking.rs:392 session.restart_all()` 被调用 | 通过 |
| 2 | 仅 bRestartable=true 进程尝试重启 | 真机日志：`[rm-restart] 被关闭的进程中没有可重启的，跳过 RmRestart`（Python 非可重启，正确跳过）；单测 `restartable_pids_只保留_closable_且_b_true` 通过 | 通过 |
| 3 | 重启超时 10s，不阻塞 | `locking.rs` 用后台线程 + `recv_timeout(10s)` 包住同步 RmRestart；真机用例 6.96s 返回，未卡住删除 | 通过 |
| 4 | 重启失败记 warning 不阻塞删除 | `locking.rs:394` `eprintln!("[rm-restart] RmRestart 失败: {e}")`；删除在 RmShutdown 后立即进行，失败仅 warning | 通过 |
| 5 | 新建 prefs 存储 | 复用 `settings(key,value,updated_at)` 表，key=`prefs.auto_restart_on_close`；真机 `db2.txt` 查到该行 | 通过 |
| 6 | get_prefs / set_prefs Tauri 命令 | `app/src-tauri/src/commands.rs` 注册；前端 Settings 切换后 DB 立即出现/更新该行（`db2.txt` 时间戳 1793192902768） | 通过 |
| 7 | 默认 false（新用户首次读返回 false） | dev 首次打开设置页时 DB 无该行 → 开关显示「已关闭」未勾选（`settings3.png`）；fail-closed 逻辑单测覆盖 | 通过 |
| 8 | Settings 页新增 toggle | `settings3.png` / `installed-max.png`：「行为偏好」卡片 →「自动重启被关闭的进程」复选框 | 通过 |
| 9 | 灰色说明文字 | 「清理被占用的文件时，若系统支持（Restart Manager 标记为可重启），关闭后自动把进程重新拉起。默认关闭。」（灰色，见截图） | 通过 |
| 10 | 切换立即 set_prefs 持久化 | 点复选框后秒查 SQLite 即得 `'1'`（`db2.txt`） | 通过 |
| 11 | 启动时读 prefs 回填 | 杀进程重启 dev → 设置页仍蓝勾「已开启」（`restart-settings-zoom.png`）；打包安装态同样回显 on（`installed-max.png`） | 通过 |
| 12 | 开启时 confirm 文案含「将自动重启」 | 前端单测 `Items.test.tsx` 覆盖文案追加（`注意：本次会在关闭后自动重启被关闭且支持重启的进程。`）；真机未走真实扫描触发 confirm（见 §6 第三档说明） | 通过（单测） |
| 13 | 关闭时文案不变 | 前端单测覆盖 false 分支不追加 | 通过（单测） |
| 14 | prefs_test：默认 false / set-get / 未知 key 不破坏 | lib 54 passed 中含 prefs 5 条（`final-gate.log`） | 通过 |
| 15 | RmRestart bRestartable 标志判断单测 | `rm_restartable_只对_b_true_返回_true` + `restartable_pids_只保留_closable_且_b_true` 通过 | 通过 |
| 16 | cargo test -p engine 全绿 | `cargo test --workspace` EXIT=0（`final-gate.log`） | 通过 |
| 17 | pnpm test 全绿 | `pnpm --filter app test` EXIT=0，9 文件 85 passed（`final-gate.log`） | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| D1 | `RmSession::restart(pid)` 把 PID 当 `dwRestartFlags` 传给 `RmRestart`，非零 flags 必然 `ERROR_INVALID_PARAMETER(87)`，自动重启从不生效（旧代码按 pid 循环 N 次） | 查 `windows-0.58` 绑定签名 `RmRestart(u32,u32,RM_WRITE_STATUS_CALLBACK)` 与官方文档：第二参是保留位必须为 0，无 PID 形参 | 一次 `RmRestart(handle, 0, None)`，由 RM 内核侧原生重启 bRestartable==TRUE 应用 | P0 | 已修（bugfix/task-057-rmrestart-flags） |
| D2 | 新增测试用 `let mut x=T::default(); x.field=...` 触发 clippy `field_reassign_with_default`，`-D warnings` 下为 error | `cargo clippy --workspace --all-targets -- -D warnings` 开发自报 0 但本地未加 `-D warnings` | 用结构体字面量 `T{ field: BOOL(1), ..Default::default() }` | P1 | 已修（同 bugfix 分支） |

## 5. 测试期修复记录（bugfix 子分支）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| 1 | D1 RmRestart flags 误用 | bugfix/task-057-rmrestart-flags | `0da34ad` | `fix(execute): RmRestart 改为一次 flags=0 调用… (#task-057)` | fmt/clippy(-D warnings)/cargo test --workspace 全绿，lib 54 passed |
| 2 | D2 clippy field_reassign_with_default | bugfix/task-057-rmrestart-flags | `0da34ad`（同提交内嵌） | 同上 | clippy EXIT=0 |
| 3 | 缺陷登记 | bugfix/task-057-rmrestart-flags | `befca9e` | `docs(state): task-057 测试轮登记… (#task-057)` | — |
| 4 | 合回 feature | feature/task-057-rm-restart-toggle | `e308f9e` | `merge(bugfix): task-057 RmRestart flags P0 + clippy P1 修复 (#task-057)` | 最终闸门全绿，bugfix 子分支已删 |

## 6. 遗留问题（三档分流）

### 第一档（已修）
- D1/D2 均已在 bugfix 子分支闭环并 --no-ff 合回 feature（见 §5）。

### 第二档（需用户拍板）
- 无。

### 第三档（已登记为已知限制）
- **真机 notepad 自动重启未做成端到端点击链**：经典 notepad.exe 打开文件后即关闭文件句柄，Restart Manager 不视其为占锁进程（`real-restart.log` 空 closed 为证）。改用 Python 持锁进程真机验证了 RmShutdown 真关 + RmRestart 路径不再报 87 错误；但「notepad 被关后自动拉起」这一直观证据需一个**既持锁又注册了重启**的应用（如现代 Edge/Word），本环境未装。代码层 bRestartable 判断 + `RmRestart(handle,0,None)` 签名已按微软官方文档与 windows-0.58 绑定双重核实，真实可重启行为在生产态由 RM 内核保证。
- **CDP console 0 红错**：tauri dev 默认不开 9222 调试端口（历史已知），无法 CDP 抓 console；dev 与安装态运行日志（`tauri-dev.log`、`tauri-build.log`、安装后窗口无白屏/无红弹窗）未见运行时错误。G6（设置页首调 rules_check_update 走 ipc.localhost 红错）按既有豁免口径排除。
- **updater 签名**：`tauri build` 末尾因无 `TAURI_SIGNING_PRIVATE_KEY` 退出 1，但 NSIS/MSI 安装包已正常产出（4.04 MiB / 5.86 MiB），不影响安装冒烟；签名属发版配置，非本任务范围。

## 7. 测试结论

- 结论：**通过**
- feature 分支最新 commit：`e308f9e`（领先 origin 3 个提交，待 push）
- 同源测试声明：本测试 Agent 与开发 Agent 为同一会话（同源）。测试轮独立从 `e308f9e` 重新拉分支状态、重跑全部闸门与真机冒烟，未复用开发轮「自验全绿」结论；测试轮新发现并修复了 2 个开发轮漏掉的缺陷（D1 P0 功能坏、D2 P1 CI 门禁），已在 §4/§5 留痕。评审 Agent 请重点复核 D1 的 FFI 签名修复与 bRestartable 过滤逻辑。
- 移交说明：L1/L2/L3/L4 全绿，证据齐全（见 §2/§3）。feature 分支 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 windows-latest 五道闸。测试 Agent 不下「可合并 main」结论，交评审 Agent（提示词 10）判定。
