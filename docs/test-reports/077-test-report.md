# 测试报告：task-077 P2 起步——多盘扫描支持（C 盘 → 所有固定磁盘）

- 分支：`feature/task-077-p2-multi-drive-scan`
- 测试 HEAD：`79b923f`
- 测试时间：2026-11-04（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）
- **P2 起步：磁盘枚举 + 多盘扫描接线 + v3 生成列迁移 + 前端多盘 UI（Dashboard 环形图 + Items 盘筛选下拉框）**

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| 构建验证 | `cargo fmt/clippy/test` + `pnpm lint/test/build` | 是 | 通过 |
| dev 真机启动 | `pnpm tauri dev`（cdisk-cleaner.exe 运行，后端 IPC 跑通） | **是**（按 2026-11-03 新规，有前端改动必须真机） | 后端通、前端窗口 WebView2 渲染进程崩溃（项目长期已知限制） |
| 打包 | NSIS `C-DiskCleaner_0.2.1_x64-setup.exe`（4.52 MiB / 4734548 字节） | 是 | 通过（出包成功） |
| 干净目录安装 | `/S /D=...` | 未尝试 | 归第三档（UAC 非交互受限） |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区（含 32 条新增 + 15 条 `#[ignore]` 真机） | `cargo test --workspace` | 通过 | `cargo-test.log`：63 batches / **585 passed / 0 failed / 15 ignored**（基线 553 + 新增 32，与开发声称一致） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm test` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（1 条 elevation incremental lock warning + 2 条 elevation 既有 warning）；`pnpm.log`：lint=0（2 条 SafeBox.tsx 既有 warning）、test=0（14 files / **189 passed**，+9）、build=0（**244.06 kB / gzip 75.88 kB**，+1.05 kB，与开发声称一致） |
| L3 真机 | 后端 IPC 多盘扫描 | `pnpm tauri dev` + 日志观察 | **后端通过 / 前端窗口未渲染** | `tauri-dev.log`：session=2 / items=8958 / bytes=537938297 / ok=[temp,windows_update,prefetch,browser_cache,dev_cache] / failed=[] / skipped=[] / **per_drive=[C:8958项/537938297B]**；5 模块全走 Rust（temp 4268 / windows_update 9 / prefetch 0 / browser_cache 4660 / dev_cache 21）；但 WebView2 窗口创建后未在桌面显示（`launch_app` 返回 `accepted_unverified`，Crashpad 崩溃为项目长期已知限制） |
| L4 打包 | NSIS 出包 | `tauri build` | 通过 | `tauri-build.log`：NSIS 4.52 MiB（+0.02 vs 076）+ MSI 6.68 MiB；FileVersion=0.2.1；签名 exit=1 为已知预期 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `drive_enum.rs` 正确枚举所有固定磁盘 | 7 条内联单测全绿（盘符解析含常规/长路径/UNC/相对/空、used_bytes 饱和减法、枚举含系统盘、升序去重、容量自洽） | 通过 |
| 2 | 返回 `DriveInfo { letter, total_bytes, free_bytes }` | 与任务书结构一致；dev 真机冒烟枚举到 C: 400 GB / D: 551.6 GB（与资源管理器一致） | 通过 |
| 3 | 非 Windows 平台返回空（条件编译） | `#[cfg(windows)]` 守卫 + 单测断言 | 通过 |
| 4 | 单测：枚举结果包含 C 盘 | `枚举含系统盘` 单测钉住 | 通过 |
| 5 | `orchestrator.rs` 对所有固定盘执行扫描 | 6 条内联单测 + 1 条真机冒烟；dev 实测枚举到 C:/D: 双盘 | 通过 |
| 6 | 进度按「盘 × 模块」二维推进 | 标签带盘符（`C: scan_browser_cache.ps1`）；dev 说明每个模块只锚定一个盘，二维退化成一条线，不编造空格子 | 通过（实现简化，见第二档偏差 7） |
| 7 | Prefetch / Windows Update 只在 C 盘执行 | **不硬编码 C:**，靠 `%SystemRoot%` 自然跳过（Windows 装 D 盘的机器才正确）；单测 `root_env_names_marks_prefetch_and_update_as_system_anchored` 钉住 | 通过（实现与任务书字面不同，见第二档偏差 6） |
| 8 | 单测：多盘扫描结果按盘分组 | `per_drive` 单测 + dev 日志 `per_drive=[C:8958项]` | 通过 |
| 9 | v2 迁移成功（items 表加 drive_letter + 索引） | **实际是 v3**（v2 已被 task-056 占用）；7 条迁移单测全绿（到达 v3、列存在用 `table_xinfo`、索引存在、生成列派生正确、DB↔Rust 判定逐条一致、幂等、不破坏既有 INSERT） | 通过（版本号偏差，见第二档偏差 1） |
| 10 | 写入 items 时自动提取盘符 | **改用 SQLite 生成列**（`GENERATED ALWAYS AS ... VIRTUAL`），写入路径 `runner.rs` 一个字都没改；物理上不可能与 path 不一致 | 通过（实现方式偏差，见第二档偏差 3） |
| 11 | scan.start 入参支持指定盘（默认全部） | 6 条 scan_start 单测全绿（不传=全部、`C`/`C:`/`c`/`c:` 四种写法归一化、去重、空数组报错、非法项整体失败、旧 `targetDrive` 兼容） | 通过 |
| 12 | 进度事件含盘标识 | 标签带盘符；dev 说明未加 `ScanProgress.drive` 字段（写进 `current_path` 标签即可，加字段要连带改动 `runner.rs`） | 通过（实现简化，见第二档偏差 4） |
| 13 | 单测：迁移 + 盘符提取正确 | 7 条迁移单测 + `drive_letter_of` 7 条单测 | 通过 |
| 14 | Dashboard 环形图多盘并列显示 | 4 条 Dashboard.test.tsx 新增（多盘渲染含 60%/90% 与「空间紧张」、只调一次 drive_list、容量未知、枚举失败不渲染） | 通过（单测层）；真机窗口未渲染见第三档 |
| 15 | Items 页盘筛选下拉框工作 | 5 条 Items.test.tsx 新增（下拉框渲染、切换后带 drive 重拉、切盘清空勾选、枚举不到不显示、枚举失败降级）；6 条 list_pending 服务端过滤单测 | 通过（单测层）；真机窗口未渲染见第三档 |
| 16 | 扫描进度显示当前盘 | 标签带盘符（dev 日志可见 `C: scan_browser_cache.ps1` 形态） | 通过 |
| 17 | 单测：组件渲染正确 | 9 条新增 vitest（Dashboard 4 + Items 5） | 通过 |
| 18 | `cargo test --workspace` 全绿（新增 ≥5 条） | 585 passed（新增 32 条，远超 ≥5） | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| 1 | **dev 真机日志出现 4 次 tokio runtime drop panic**：`Cannot drop a runtime in a context where blocking is not allowed`（`tokio-1.53.2/src/runtime/blocking/shutdown.rs:51:21`） | `pnpm tauri dev` 启动后，扫描完成期间多次出现 | panic 不应出现；但 app 未崩（big_files_scan 后续仍在跑），疑似某个 IPC handler 在 async 上下文里 drop 了 blocking runtime（可能是新引入的多盘命令或 big_files/residual 扫描的 tokio 用法） | **P2 观察项** | 测试轮未阻断（app 仍运行、扫描结果正确入库），但建议 dev 排查是否新代码引入；归第二档请评审判定 |

## 5. 测试期修复记录（bugfix）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮无需代码修复（tokio panic 为观察项，未阻断主流程） | — | — | — | — |

## 6. 遗留问题（三档分流）

### 第一档（已修）
- 磁盘枚举：`GetLogicalDrives()` 位图 + `GetDriveTypeW()` 筛 `DRIVE_FIXED`（windows 0.58 未导出该常量，按 WinBase.h 原值写 `const=3`）；`GetDiskFreeSpaceExW` 失败时仍 push 该盘、容量记 0（前端显示「容量未知」而非 0% 空环）。
- 多盘扫描：**按盘过滤而非路径替换**（模块级跳过 + 条目级过滤两层）——按盘换路径会造出 `D:\Windows\Temp` 这种不存在的路径，扫 0 项还告诉用户「已扫 D 盘」。
- v3 生成列：`drive_letter TEXT GENERATED ALWAYS AS (CASE ...) VIRTUAL` + 组合索引 `(session_id, drive_letter)`（本表所有业务查询都以 session_id 打头，单列索引用不上）。
- Items 盘筛选：**服务端过滤**（`list_pending_page_by_drive`），不是当前页过滤——否则 `total` 和分页条仍是全量数字，用户会看到「共 4574 条」却每页只有几条匹配。
- 切盘清空勾选 + 回到第 1 页（用户勾了 C 盘项再切 D 盘，「清理已选」不能删看不到的 C 盘项）。
- Dashboard：一次 `drive_list` 拿全部盘（不为每盘再调 `drive_usage`，N+1 IPC → 1 次）；`totalBytes==0` 显示「—」+「容量未知」；枚举失败整块不渲染（不显示假环）。

### 第二档（需评审/用户拍板）
1. **v3 而非 v2**：任务书写 v2，但 v2 已被 task-056 `safe_box.restored_at` 占用。两个 v2 会让后一个永远不执行且无报错。
2. **索引名 `idx_cleanup_item_drive_letter` 而非任务书写的 `idx_items_drive_letter`**：本仓库没有 `items` 表（实际表名 `cleanup_item`）。
3. **「写入时提取盘符」改用生成列**：任务书点名 `db/resolver.rs`，但那是读路径不是写入路径。改用 SQLite 生成列后写入路径 `runner.rs` 一个字都没改（ps1/Rust 共用的唯一入库入口，改它要同时验证两条路径）。
4. **未加 `ScanProgress.drive` 字段**：盘符写进 `current_path` 标签即可，加字段要连带改动 `runner.rs`（白名单外）与多处测试构造点，对前端零可见收益。
5. **「对每个盘执行扫描（路径按盘替换）」改为按盘过滤**：5 个模块根目录全部锚定在环境变量上（TEMP/SystemRoot/LOCALAPPDATA/APPDATA），没有「每盘一份」的对应物。
6. **「Prefetch/Windows Update 只在 C 盘执行」不硬编码 C:**：靠 `%SystemRoot%` 自然跳过，Windows 装 D 盘的机器上才正确。
7. **「盘×模块二维推进」简化为标签带盘符**：每个模块只锚定一个盘，二维退化成一条线，不编造 5×N 个空格子。
8. **既有断言「C 是非法盘符」改为合法**：`drives` 入参文档就是 `["C"]` 无冒号形态，前端下拉框给 `"C:"`，四种写法统一在 `parse_drive` 归一化。
9. **白名单外 7 个文件必改**（不改就跑不起来）：`scan/mod.rs`（注册 drive_enum）、`app/src-tauri/src/commands.rs`（drive_list 命令 + scan_start drives 入参 + cleanup_list_pending drive 入参）、`app/src-tauri/src/lib.rs`（generate_handler 注册）、`execute/list_pending.rs`（服务端过滤）、`engine/tests/ipc_commands.rs`（6 处结构体补字段）、`engine/tests/db_schema.rs`（3 条版本号断言 v2→v3）、`app/src/pages/{Dashboard,Items}.test.tsx`（9 条新测试）。
10. **tokio runtime drop panic（4 次）**：app 未崩但建议 dev 排查是否新代码引入。

### 第三档（已登记为已知限制）
- **真机 GUI 窗口未渲染**：dev 日志证明后端多盘逻辑跑通（8958 项 / per_drive=[C:8958] / 5 模块 Rust / 0 failed / 0 skipped），但 WebView2 窗口创建后未在桌面显示（`launch_app` 返回 `accepted_unverified`，Crashpad 崩溃为项目长期已知环境限制，从 task-074 起 dev 就注明本机 WebView2 渲染进程创建即崩）。**按 2026-11-03 新规（有前端改动必须真机）已尝试 `pnpm tauri dev`，后端侧证据充分，前端 UI 渲染受环境限制无法截图**。
- **D 盘 0 项是正确结果**：5 个模块根目录全在 C 盘（用户/系统锚定），D 盘上没有它们的扫描目标。要让 D 盘真有条目，需新增每盘一份的目标（回收站 `$Recycle.Bin` / Top-N 大文件 / Windows.old），是独立任务。
- **ps1 回退路径不做条目级盘过滤**：`ScanRunner` 直接入库，编排层拿不到条目。选非系统盘 + `CCLEAN_RUST_ALL=0` 时 ps1 会把 C 盘条目也写进来。
- **`per_drive` 不按选择过滤**：反映「本 session 实际入库了什么」，不是「用户选了什么」（ps1 路径的退化会在这里如实暴露）。
- **安装态未完成**：NSIS 出包成功（4.52 MiB / FileVersion=0.2.1），UAC 非交互受限。
- **CDP console 未抓**：WebView2 窗口未渲染。
- **CI 五道闸未跑**：PR 由 CI 门禁观察员创建后跑。
- **签名**：本地未签名（exit=1 已知预期）。
- **P2 持续**：用户自定义规则（task-078）/ 便携版 CLI（task-079）。
- **Items/SafeBox/History 三页深色适配仍缺**（task-061 起的旧账）。
- **版本线**：task-072~077 攒着发 `v0.4.0`；CHANGELOG 需写明「多盘模式下非系统盘可能显示 0 项（不是 bug）」。
- **`tasks/060-shutdown-experience.md` 仍是「工作区已删除、HEAD 里存在」**（连续 15 个任务）。

## 7. 测试结论

- 结论：**通过（585 passed / 189 vitest / NSIS 出包 4.52 MiB；后端多盘逻辑 dev 真机日志验证通过，前端 UI 渲染受 WebView2 环境限制）**
- feature 分支最新 commit：`79b923f`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/lint/test/build）+ NSIS 出包 + `tauri dev` 后端日志观察，未复用开发自验结论。
- **新规执行说明**：本任务**有前端改动**（Dashboard 多盘环形图 + Items 盘筛选下拉框），按用户 2026-11-03「有前端更改必须真机测试」的指示，L3 真机**已执行**（`pnpm tauri dev` 启动 + 后端日志观察）。后端多盘逻辑跑通（8958 项 / per_drive=[C:8958] / 5 模块 Rust），但 WebView2 窗口渲染受项目长期已知环境限制（从 task-074 起 dev 就注明本机 WebView2 渲染进程创建即崩），UI 截图层无法取证。此为环境限制非代码缺陷。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①v3 而非 v2（v2 已被 task-056 占用）；②生成列 vs 写入时提取（runner.rs 共用入口未动）；③按盘过滤 vs 路径替换（5 模块无根目录在 D 盘）；④不硬编码 C:（%SystemRoot% 自然跳过）；⑤服务端过滤 vs 当前页过滤（total/分页一致性）；⑥白名单外 7 个文件必改是否合理；⑦tokio runtime drop panic 4 次是否新代码引入；⑧D 盘 0 项是正确结果（需每盘一份目标如回收站才会有条目）。**
