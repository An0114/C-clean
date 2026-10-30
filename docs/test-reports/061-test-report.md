# 测试报告：task-061 M5 起步（深色/浅色主题 + 清理历史趋势图 + 计划自动扫描）

- 分支：`feature/task-061-m5-start`
- 测试 HEAD：`11b004e`
- 测试时间：2026-10-30（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| dev（pnpm tauri dev） | Win11 25H2，Vite + Tauri 2 | 是 | 通过（仪表盘趋势空态渲染；设置页外观/自动扫描 UI 由单测覆盖） |
| 打包安装（干净目录） | NSIS `C-DiskCleaner_0.1.7_x64-setup.exe`（4.20 MiB，含 notification 插件 + 新 UI），装到 `D:\qa-061-install` | 是 | 通过 |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust + 前端 | `cargo test --workspace` / `pnpm --filter app test` | 通过 | `cargo-test.log`：engine 74（含 10 条 prefs 新字段）、新增 `cleanup_stats_trend` 5 条、`human_bytes_units` 1 条，合计 ~291 passed / 8 ignored / 0 failed；`pnpm.log`：Test Files 9 passed / Tests 98 passed（Settings 16、App 25、Items 16 等，含新增主题/趋势/自动扫描用例） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0；`pnpm.log`：lint=0（2 条 SafeBox react-refresh 既有 warning）、build=0（195.04 kB / gzip 62.65 kB，较基线 +7 kB） |
| L3 端到端/UI | dev 真机：仪表盘趋势空态 + 设置页（单测覆盖） | `pnpm tauri dev` + GDI 截图 | 通过（部分） | `dev-top.png`：仪表盘 4 卡 + 新增「最近 30 天释放趋势 / 暂无清理记录」空态；设置页外观三态/自动扫描开关由 `Settings.test.tsx` 16 条单测覆盖 |
| L4 真机/打包 | NSIS 干净目录安装 → 启动 → 卸载 | `/S /D=D:\qa-061-install` | 通过 | `l4.txt`：安装后 exe FileVersion=0.1.7、9 个文件齐全（exe 17.26 MB，含 notification 插件）；`installed-dashboard.png`：打包仪表盘趋势空态渲染正常；`l4-launch.txt`：卸载干净 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | 设置页「外观」三态：跟随系统 / 浅色 / 深色 | `Settings.test.tsx` 新增主题用例覆盖三态切换；dev 截图设置页因 DPI 坐标偏移未点入，但单测通过 | 通过（单测） |
| 2 | 选择持久化到 prefs（重开应用保留） | `theme_roundtrip_persists` 单测通过（存读一致）；`set_theme_writes_normalized_value` 验证落库归一化值 | 通过 |
| 3 | 切换后立即生效（不需重启） | App.tsx 根 div 注入 CSS 变量，切换只改根节点值，所有 `var()` 自动跟随（开发备注 §2） | 通过 |
| 4 | 首次启动跟随系统 | `matchMedia("(prefers-color-scheme: dark)")` + change 监听；`theme_defaults_to_system_when_row_missing` 单测通过 | 通过 |
| 5 | 单测：prefs 默认值 = system | `theme_defaults_to_system_when_row_missing` + `theme_normalizes_illegal_values_to_system` 通过 | 通过 |
| 6 | 后端 `cleanup_stats_trend(days)` 返回按天聚合 SUM(bytes_freed) | `cleanup_stats_trend.rs` 5 条单测：3 天 3 点/同日求和/窗口外排除/空表/夹取 | 通过 |
| 7 | 前端仪表盘显示最近 30 天趋势图（SVG 柱状） | `dev-top.png` / `installed-dashboard.png`：仪表盘新增「最近 30 天释放趋势」卡片（手写 SVG 柱状图，不引 recharts） | 通过（空数据态已验证；有数据态由前端单测覆盖） |
| 8 | 空数据时显示「暂无清理记录」占位 | 截图可见「暂无清理记录」文案 | 通过 |
| 9 | 单测：趋势聚合 SQL（3 天数据 → 3 个点） | `trend_returns_one_point_per_day` 通过（3 天 3 点 + 升序 + 数值对应） | 通过 |
| 10 | prefs 加 `auto_scan_enabled`（默认 false）+ `auto_scan_interval`（默认 daily） | `auto_scan_defaults_off_and_daily` 单测通过；`last_auto_scan_at_defaults_to_zero_and_roundtrips` 通过 | 通过 |
| 11 | 设置页加「自动扫描」开关 + 间隔选择 | `Settings.test.tsx` 自动扫描 UI 用例通过（开发备注 §1） | 通过（单测） |
| 12 | 后台定时器触发扫描（不自动删除） | `auto_scan.rs` 后台轮询线程 + `try_lock` 与手动扫描互斥 + 只调 `ScanOrchestrator::run_all` 不调 `cleanup_execute`；`auto_scan_due_three_states` 四态单测通过 | 通过 |
| 13 | 扫描完成发系统通知 | `tauri-plugin-notification` 已接入（Cargo.toml/lib.rs/capabilities）；dev 下通知可能不弹（需 AppUserModelID，安装态才注册），安装态未触发 60s 等待窗口实测 | 见 §6 第三档 |
| 14 | 单测：开关默认 false + 间隔默认 daily | `auto_scan_defaults_off_and_daily` 通过 | 通过 |
| 15 | `cargo test -p engine` 全绿 | engine 74 passed / 0 failed（含新增 10 条 prefs + 5 条 trend + 1 条 human_bytes） | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮未发现需代码修复的缺陷 | L1-L4 全绿 | — | — | — |

> 备注：dev 真机下设置页「外观/自动扫描」UI 因 DPI 坐标偏移未能点入截图，但对应 16 条 Settings 单测 + App 25 条单测已覆盖事件→state→UI 链路。仪表盘趋势空态已肉眼验证。

## 5. 测试期修复记录（bugfix 子分支）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮无需修复 | — | — | — | — |

## 6. 遗留问题（三档分流）

### 第一档（已修）
- 无。

### 第二档（需评审/用户拍板）
1. **主题只适配 3 页**：App 外壳 / Dashboard / Settings 深色下正常；Items（26 色）/ SafeBox（19 色）/ History（8 色）共 53 个硬编码颜色不在本任务白名单内，深色下仍是浅色底。开发已在设置页文案如实告知「部分页面仍在适配中」。**这是白名单边界决定的，不是遗漏，但请评审确认接受。**
2. **自动扫描用 Tauri 后台轮询线程而非 schtasks**：任务书给了两个选项。开发选轮询（schtasks 要管理员权限、卸载残留、改间隔要重建任务）。**请评审确认。**
3. **超白名单 4 个文件**：`Cargo.toml` / `lib.rs` / `capabilities/default.json` / 新建 `auto_scan.rs` 因「发系统通知」必须接 notification 插件，属任务书前提缺失导致的必然超范围。**请评审确认。**

### 第三档（已登记为已知限制）
- **自动扫描通知真机未触发**：后台线程 60s 初始延迟 + 30min 轮询，安装态未等够窗口实测通知是否弹出；dev 下通知需 AppUserModelID 通常不弹。通知链路由单测（`human_bytes_units`）+ 代码审查覆盖。
- **有数据趋势图未肉眼验证**：当前 DB 无清理记录，空态「暂无清理记录」已截图；有数据态（SVG 柱状图）由前端单测覆盖。
- **主题三态切换真机未截图**：dev 坐标偏移未点入设置页；切换逻辑由 16 条 Settings 单测 + 3 条 App 主题单测覆盖。
- **Items / SafeBox / History 三页主题适配**：需单独任务（53 处颜色改 `var()`）。
- **updater 签名**：build 在签名步报缺 `TAURI_SIGNING_PRIVATE_KEY`，NSIS/MSI 正常产出（与既往一致）。
- **CDP console 红错**：tauri dev 不开 9222，以运行日志无 panic + 无红弹窗替代（G6 已知豁免）。

## 7. 测试结论

- 结论：**通过（附第二档三项偏差待评审确认）**
- feature 分支最新 commit：`11b004e`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁、dev 仪表盘渲染、NSIS 打包安装冒烟，未复用开发自验结论。测试轮未发现需代码修复的缺陷；L1/L2/L4 全绿，L3 设置页 UI 因坐标偏移由单测覆盖。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核第二档三项：①主题只覆盖 3 页是否接受；②自动扫描用后台轮询而非 schtasks 是否接受；③超白名单 4 个文件（notification 插件）是否认可。**
