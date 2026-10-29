# 测试报告：task-060 占用关闭体验（RmShutdown 进度反馈 + 跨盘文档化）

- 分支：`feature/task-060-shutdown-experience`
- 测试 HEAD：`0836019`
- 测试时间：2026-10-29（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| dev（pnpm tauri dev） | Win11 25H2，Vite + Tauri 2 | 是 | 通过（页面渲染无回归；进度条真机触发受环境限制，见 §6） |
| 打包安装（干净目录） | NSIS `C-DiskCleaner_0.1.7_x64-setup.exe`（4.05 MiB，含本任务新代码），装到 `D:\qa-060-install` | 是 | 通过 |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust + 前端 | `cargo test --workspace` / `pnpm --filter app test` | 通过 | `cargo-test.log`：engine 64（含新增 `shutdown_ticker_reports_seconds_until_stopped` / `shutdown_ticker_is_noop_without_callback`），合计 ~275 passed / 8 ignored / 0 failed；`pnpm.log`：Test Files 9 passed / Tests 90 passed（含新增进度条事件用例） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（仅增量 lock 文件 os error 5 警告，非代码问题）；`pnpm.log`：lint=0（2 条 SafeBox react-refresh 既有 warning）、build=0（188.05 kB / gzip 60.49 kB，较基线 +0.6 kB 来自进度条 UI） |
| L3 端到端/UI | dev 真机：仪表盘/保险箱/清理 tab 渲染、无红错 | `pnpm tauri dev` + GDI 截图 | 通过（部分） | `dev-dashboard.png`（仪表盘 4 卡渲染）、`dev-items2.png`（保险箱空态 + 时间/session 筛选 + 分页）；无白屏、无红色弹窗 |
| L4 真机/打包 | NSIS 干净目录安装 → 启动 → 卸载 | `/S /D=D:\qa-060-install` | 通过 | `l4.txt`：安装后 exe FileVersion=0.1.7、9 个文件齐全；`installed-dashboard.png`：打包仪表盘渲染正常；`l4-launch.txt`：卸载干净 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `RmRegisterResources` 除文件外注册进程本身 | **开发核实后未改代码**（任务书 §2 详述：参数类型是 `RM_UNIQUE_PROCESS` 不是 pid；锁文件进程已由 `RmGetList` 枚举；连带关闭是 RM 按 console 进程组的既定行为；强行注册会扩大关闭面触发 mixed fail-closed）。代码注释固化理由。 | 见 §6 第二档 |
| 2 | RmShutdown 后父子两进程都退出（单测） | 现有 `locking_shutdown.rs` 3 passed / 5 ignored（需要真实 RM 环境，CI 单盘 skip）；开发备注引用 task-049b 实测 | 通过（既有证据） |
| 3 | confirm 文案不变 | 未改 Items.tsx confirm 文案，仅新增进度条 state | 通过 |
| 4 | 后端 RmShutdown 改异步（spawn 线程），每 1s emit 进度 | `spawn_shutdown_ticker` 实现 + 2 条单测：`shutdown_ticker_reports_seconds_until_stopped`（1s 后回调 [1]，stop+join 后不再增长）、`shutdown_ticker_is_noop_without_callback`（None 回调不起线程）；commands.rs emit `shutdown://progress` | 通过 |
| 5 | 前端显示「已等 Xs / 最多 30s」进度条 | `Items.test.tsx` 新增用例：手动推 `elapsedSecs:7` → 断言文案；推 `31` → 断言「仍在等待」；样式 6px 高、amber 色、宽度 min(secs/30,1) | 通过（单测覆盖事件→state→UI；真机进度条触发见 §6） |
| 6 | 30s 超时后放弃，不阻塞删除 | **开发未实现「超时放弃」**：`RmShutdown` 无超时参数且不可中断，改为超 30s 后继续回调并提示「仍在等待」。删除流程本身不被 ticker 阻塞（ticker 只是观察者） | 见 §6 第二档 |
| 7 | USER_GUIDE 补「跨盘恢复行为」一段 | `docs/USER_GUIDE.md:162 ### 4.5 跨盘行为` 存在，含移入支持/恢复不支持/原因/FAQ Q9 | 通过 |
| 8 | copy+remove fallback + 跨盘慢 + 失败时文件留在保险箱 | 4.5 节写明：移入 = rename 失败自动 copy+remove（慢）；恢复 = 直接返回 IO 错误、文件仍在保险箱、原位置无残留；FAQ Q9 给三步处置 | 通过（按开发核实的**实际行为**写，纠正了任务书「跨盘恢复也有 fallback」的错误假设） |
| 9 | `cargo test -p engine` 全绿 | engine 64 passed / 0 failed（含新增 2 条 ticker 用例） | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮未发现需代码修复的缺陷 | L1-L4 全绿 | — | — | — |

## 5. 测试期修复记录（bugfix 子分支）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮无需修复 | — | — | — | — |

## 6. 遗留问题（三档分流）

### 第一档（已修）
- 无。

### 第二档（需评审/用户拍板）
1. **验收 1（注册进程本身）未按任务书实现**：开发核实任务书方案在 API 类型上不成立（`rgapplications` 是 `RM_UNIQUE_PROCESS` 不是 pid），且连带关闭兄弟进程本就是 RM 既定行为（task-049b 已实测），强行注册反而扩大关闭面。改为注释固化 + USER_GUIDE 3.5 补用户可见说明。**这是对任务书方案的合理反驳，不是漏做，但请评审确认接受。**
2. **验收 6（30s 超时放弃）未实现**：`RmShutdown` 是同步阻塞 FFI、无超时参数、无法中断，开发改为「超 30s 继续等待 + 前端提示仍在等待」。若坚持要「30s 强制中断」，需改用 `TerminateProcess` 强制杀进程（丢掉 RM「请求式关闭、给保存机会」的语义），与产品定位冲突。**请评审确认接受。**
3. **跨盘恢复实际无 fallback**：任务书假设「跨盘恢复 copy+remove」，实际代码恢复直接返回 IO 错误（已有单测 `cross_disk_restore_currently_returns_io_error` 固化）。文档按实际写。**跨盘恢复 fallback 的真正实现属独立新任务。**

### 第三档（已登记为已知限制）
- **进度条真机触发未拍到**：要触发「正在关闭进程… 已等 Xs」需一个持文件锁且注册了 RM 重启的进程（现代 Edge/Word 类），本环境 notepad 不持锁、无法制造占用失败场景。事件→state→UI 链路由 `Items.test.tsx` 单测覆盖（含 7s / 31s 两个分支）；ticker 起停由 Rust 单测覆盖。建议用户日常清理中遇到占用时自然验证。
- **`shutdown://progress` 是全局事件**：当前 cleanup_retry_locked 串行、同时只有一个文件在关，暂无问题；并发化时需加 path/itemId 字段区分（开发备注 §11）。
- **updater 签名**：build 在签名步报缺 `TAURI_SIGNING_PRIVATE_KEY`，但 NSIS/MSI 安装包正常产出（与既往一致）。
- **CDP console 红错**：tauri dev 不开 9222，以运行日志无 panic + 无红弹窗替代（G6 已知豁免）。

## 7. 测试结论

- 结论：**通过（附第二档三项偏差待评审确认）**
- feature 分支最新 commit：`0836019`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁、dev 真机页面渲染、NSIS 打包安装冒烟，未复用开发自验结论。测试轮未发现需代码修复的缺陷；L1/L2/L4 全绿，L3 进度条真机触发受环境限制（单测已覆盖）。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核第二档三项：①注册进程本身的反驳是否成立；②30s 不强制中断是否接受；③跨盘恢复无 fallback 的文档化处理。**
