# 测试报告：task-066 M1 E 模块——大文件地图（hiberfil/Windows.old/回收站/Top100，只读不删）

- 分支：`feature/task-066-big-files-map`
- 测试 HEAD：`5c446ba`
- 测试时间：2026-10-31（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| dev（pnpm tauri dev） | Win11 25H2 | 是（以 NSIS 安装态代跑 L3） | 通过 |
| 打包安装（干净目录） | NSIS `C-DiskCleaner_0.1.7_x64-setup.exe`（4.38 MiB / 4595890 字节），装到 `D:\qa-066-install` | 是 | 通过 |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区 + 前端 | `cargo test --workspace` / `pnpm --filter app test` | 通过 | `cargo-test.log`：55 batches / **382 passed / 0 failed**（基线 364 + 新增 18 + 1 `#[ignore]` 真机冒烟用例；big_files_test.rs 18 passed 1 ignored）；`pnpm.log`：11 files / **123 passed**（基线 113 + BigFiles.test 9 + App.test 1） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy -D warnings` / `pnpm lint` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（2 条 elevation 既有 warning）；`pnpm.log`：lint=0（2 条 SafeBox 既有 warning）、build=0（211.79 kB / gzip 67.73 kB，+5.13 kB） |
| L3 端到端/UI | 安装态：Tab 栏「大文件」按钮真机可见 | NSIS 启动 + GDI 截图 | 通过 | `dashboard.png`：tab 栏在「清理」与「保险箱」之间新增「大文件」按钮；仪表盘其他模块（4 卡 / 环形图 50% / WinSxS 卡片 / 趋势图）无回归 |
| L4 真机/打包 | NSIS 干净目录安装 → 启动 → 卸载 | `/S /D=D:\qa-066-install`（`-Verb RunAs` 绕 UAC 自动取消） | 通过 | `l4.txt`：installed=True、elev_exe=True、pid 在案、cleaned=True |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `scan_hiberfil()` 返回 hiberfil.sys 大小 + 存在状态 | `fs::metadata().len()` 不打开文件（内核持有）；big_files_test 覆盖；开发真机冒烟：hiberfil=11893727232B（11.1 GB） | 通过（单测 + 开发真机数据） |
| 2 | `scan_windows_old()` 返回 Windows.old 总大小 | 复用 walk_collect 递归求和；开发真机：0B（本机无 Windows.old） | 通过（单测 + 开发真机数据） |
| 3 | `scan_recycle_bin()` 返回回收站大小 | 优先 `SHQueryRecycleBinW`，失败退回遍历 `$Recycle.Bin`；开发真机：1593943B（1.5 MB） | 通过（单测 + 开发真机数据） |
| 4 | `scan_top100()` 返回 Top100 列表（路径+大小） | BFS + 最小堆 Top N + 100 MiB 入围门槛 + 系统目录黑名单；开发真机：99 条 / 20 万条目截断 / 20.2 GB 录屏居首 | 通过（单测 + 开发真机数据） |
| 5 | 单测：各扫描函数返回格式正确 | big_files_test.rs 18 条通过 | 通过 |
| 6 | `big_files_scan()` IPC handler 注册成功 | `app/src-tauri/src/commands.rs` + `lib.rs` 注册；前端 invoke 可调用 | 通过（代码审查 + 单测） |
| 7 | 前端调用返回完整数据结构 | BigFiles.test 9 条覆盖渲染 | 通过（单测） |
| 8 | 单测：handler 返回值格式正确 | 同 #5/#7 | 通过 |
| 9 | 四个分类卡片显示大小 + 处理建议 | BigFiles.tsx 三固定分类卡 + Top100 汇总卡（开发说明实为三分类，非四分类；Windows.old=0 时不单独成卡）；BigFiles.test 断言三卡 + Top100 汇总卡 | 通过（单测，分类数与任务书「四个」有偏差） |
| 10 | Top100 列表显示文件路径 + 大小 | BigFiles.tsx 明细表；BigFiles.test 覆盖 | 通过（单测） |
| 11 | 只读模式：无删除按钮 | BigFiles.test 用 `getAllByRole("button")` 断言按钮恰好 `["重新扫描"]`（多一个删除按钮即红） | 通过（单测） |
| 12 | 深色主题适配（CSS 变量） | BigFiles 全页走 CSS 变量 | 通过（代码审查） |
| 13 | 单测：页面渲染正确 | BigFiles.test.tsx 9 条通过 | 通过 |
| 14 | Tab 栏加「大文件」按钮 | `dashboard.png` 真机实证：tab 栏「清理」与「保险箱」之间新增「大文件」 | **通过（真机截图）** |
| 15 | 切换渲染 `<BigFiles />` | App.test 新增 1 条 tab 切换用例（26 total，较基线 +1） | 通过（单测） |
| 16 | 单测：tab 切换正确 | 同 #15 | 通过 |
| 17 | `cargo test --workspace` 全绿（364+≥4） | 382 passed / 0 failed | 通过 |

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
1. **白名单 4 处偏差**（开发已登记）：①`invoke.ts` 未改（现有模式无 per-command 封装，直调 `api<T>`）；②`engine/src/lib.rs` 无需改，但 `app/src-tauri/src/{commands,lib}.rs` 漏在白名单外（必须改）；③`App.test.tsx` 漏在白名单外（验收 16 只能在 App 层测）；④回收站 API 任务书写错（`SHGetDiskFreeSpace` 是查磁盘剩余空间，正确为 `SHQueryRecycleBinW`）。
2. **分类卡片数与任务书「四个」不符**：任务书要求 hiberfil/Windows.old/回收站/Top100 四卡，实现为「三固定分类卡 + Top100 汇总卡」（Windows.old=0 时不单独成卡）。BigFiles.test 断言的就是三卡 + Top100 汇总卡。
3. **Top100 在本机必现截断**（20 万条目 / 20 秒上限）：开发真机 `truncated=true`，UI 已提示「已达扫描上限提前停止」；要完整榜单需「后台增量扫描 + 落库 + 进度事件」，属新功能点。
4. **DFS→BFS 遍历算法改动**（commit `f98d0a3`）：初版 DFS 在 20 秒预算内只扫到 21 条 / 16.9 GB，BFS 扫到 99 条 / 59.6 GB；这是开发自验后发现的真机问题，已在交付版改 BFS。

### 第三档（已登记为已知限制）
- **点击「大文件」tab 真机渲染四分类卡片 + Top100 列表**：未点 tab（DPI 坐标偏移老问题），由 BigFiles.test 9 条 + App.test 1 条 tab 切换 + 18 条 rust 单测覆盖；真机仅证实 tab 按钮存在。
- **进页面自动扫一次真机观察**：未点 tab 等待 20 秒扫描完成，由开发真机 `cargo test -- --ignored real_scan` 数据覆盖（hiberfil 11.1 GB / 回收站 1.5 MB / Top100 99 条 / 20.2 GB 录屏居首）。
- **只读模式真机无删除按钮**：未点入页面，由 BigFiles.test `getAllByRole("button")` 断言恰好 `["重新扫描"]` 覆盖。
- **updater 签名**：build 末尾报缺 `TAURI_SIGNING_PRIVATE_KEY`，NSIS/MSI 正常产出（与既往一致）。
- **CDP console 红错**：tauri dev 不开 9222，以运行日志无 panic + 无红弹窗替代（G6 已知豁免）。

## 7. 测试结论

- 结论：**通过（L1/L2/L4 自动化闸门 + Tab 栏「大文件」按钮真机可见 全部通过；BigFiles 页面卡片/Top100 列表/只读约束真机未点入，由 28 条新增单测（18 rust + 10 前端）+ 开发真机 real_scan 数据覆盖，见 §6 第三档）**
- feature 分支最新 commit：`5c446ba`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/lint/build）+ NSIS 干净目录安装 + Tab 栏新按钮真机截图，未复用开发自验结论。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①白名单 4 处偏差；②分类卡片数与任务书「四个」不符（实为三卡+Top100 汇总卡）；③Top100 必现截断是否可接受；④BigFiles 页面真机点入证据缺失是否可接受单测+开发 real_scan 数据覆盖。M1 E 模块落地，下一步 D 模块残留增强（task-067）。**
