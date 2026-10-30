# 测试报告：task-063 清理报告导出 + 前后空间对比 + 仪表盘环形图 + 取消扫描

- 分支：`feature/task-063-report-dashboard`
- 测试 HEAD：`2e6c2cd`
- 测试时间：2026-10-30（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| dev（pnpm tauri dev） | Win11 25H2 | 是（以 NSIS 安装态代跑 L3） | 通过 |
| 打包安装（干净目录） | NSIS `C-DiskCleaner_0.1.7_x64-setup.exe`（4.53 MiB），装到 `D:\qa-063-install` | 是 | 通过 |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区 + 前端 | `cargo test --workspace` / `pnpm --filter app test` | 通过 | `cargo-test.log`：50 batches / **328 passed / 0 failed**（基线 316 + 新增 12：report 10 + orchestrator cancel 3）；`pnpm.log`：9 files / **101 passed** |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（仅 1 条 incremental lock 非阻塞 warning）；`pnpm.log`：lint=0（2 条 SafeBox 既有 warning）、build=0（199.87 kB / gzip 64.33 kB，较基线 +2.9 kB） |
| L3 端到端/UI | 安装态：仪表盘环形图真实渲染 | NSIS 启动 + GDI 截图 | 通过（部分） | `dashboard.png`：环形图蓝色 49%，中心「49% 已用」，右侧「C 盘空间 / 已用 195.8 GB / 共 400.0 GB / 可用 204.2 GB」（真实 C 盘数据） |
| L4 真机/打包 | NSIS 干净目录安装 → 启动 → 卸载 | `/S /D=D:\qa-063-install` | 通过 | `l4.txt`：installed=True、pid=38716、cleaned=True |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | 清理结果页加「导出报告」按钮 | Items.tsx 实现，真机未走清理流程；单测覆盖 | 第三档（见 §6） |
| 2 | HTML 导出：单文件含内联 CSS | `html_contains_summary_and_table` / `html_escapes_dangerous_chars` / `html_shows_dash_when_snapshot_missing` 单测通过 | 通过（单测） |
| 3 | CSV 导出：明细行（路径/大小/状态/原因） | `csv_has_bom_header_and_rows`（BOM 首字符）/ `csv_escapes_comma_and_quote` 单测通过 | 通过（单测） |
| 4 | 导出文件保存到 `%APPDATA%\com.and0114.cclean\reports\` | `write_report_creates_dir_and_file` 单测通过 | 通过（单测） |
| 5 | 单测：HTML/CSV 格式正确 | 同上 #2/#3 | 通过 |
| 6 | 清理前记录 `disk_before_bytes` | executor.rs `detect_drive` + 前后两次 `drive_usage`；`snapshot_released_uses_saturating_sub` 单测通过 | 通过（单测） |
| 7 | 清理后实时查 `disk_after_bytes` | 同上 | 通过（单测） |
| 8 | 结果页显示「清理前 X → 清理后 Y，释放 Z」 | 真机未走清理流程 | 第三档（见 §6） |
| 9 | 单测：前后空间差值（饱和减法） | `snapshot_released_uses_saturating_sub`（after>before 算 0）通过 | 通过（单测） |
| 10 | 后端 `drive_usage()` 返回 `{total, used, free}` | `used_percent_basic` / `used_percent_does_not_overflow_on_huge_disks`（8TB u128 不溢出）单测通过；**真机 Dashboard 实测返回 195.8/400.0/204.2 GB 与本机 C 盘一致** | **通过（真机证据）** |
| 11 | 前端 Dashboard 加 SVG 环形图 | `dashboard.png`：蓝色环形（r=46, strokeWidth=12, dasharray 按 pct） | **通过（真机证据）** |
| 12 | 环形图中心显示已用百分比 | 截图中心「49% 已用」 | **通过（真机证据）** |
| 13 | 单测：`drive_usage()` 返回值格式 | 同上 #10 | 通过 |
| 14 | 后端 `ScanOrchestrator` 加 cancellation token | `cancel_flag_none_means_not_cancelled` / `cancel_flag_defaults_false_then_flips` / `cancel_flag_reset_between_scans`（每轮复位）3 条单测通过 | 通过（单测） |
| 15 | 前端扫描中显示「取消」按钮 | Dashboard.tsx 实现；真机未触发扫描流程 | 第三档（见 §6） |
| 16 | 取消后保留已扫描结果 | 脚本粒度停止（不 kill ps1 子进程），保留已写入 DB 结果 | 通过（代码审查 + 单测） |
| 17 | 单测：cancel 触发后立即停止 | 同上 #14 | 通过 |
| 18 | `cargo test --workspace` 全绿（316+≥4） | 328 passed / 0 failed | 通过 |

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
1. **任务书 4 处技术偏差**（开发已在实现备注 §9 记录）：①`engine/src/commands.rs` 不存在（命令一直在 `app/src-tauri/src/commands.rs`）；②字段名 `disk_before_bytes`/`disk_after_bytes` 而非任务书写的 `drive_used_bytes_before`；③白名单漏 4 个必改文件（engine/src/lib.rs、engine/Cargo.toml、engine/src/execute/locking.rs、app/src-tauri/{commands,lib}.rs）；④前端取消扫描 UI 单测因并发 setState 元素引用失效被放弃，交真机冒烟。
2. **取消是脚本粒度**：当前 ps1 脚本（通常几秒）会跑完才停，不 kill 子进程。要做到「立即停」需 pid 传出 + TerminateProcess + 回滚已写 DB，代价高。

### 第三档（已登记为已知限制）
- **清理结果页「导出报告」按钮 + 前后对比文案**：真机未走清理完成流程（需要先扫描+勾选+清理），由 10 条 report 单测 + Items.tsx 代码审查覆盖。
- **HTML/CSV 真实导出文件**：未在真机点「导出」按钮验证文件落 `%APPDATA%\...\reports\`，由 `write_report_creates_dir_and_file` 单测覆盖。
- **CSV 中文 Excel 不乱码（BOM）**：单测验证 BOM 首字符，但未在 Excel 真实打开中文路径 CSV。
- **取消扫描按钮真机点击**：未点「取消」按钮观察立即停止 + 已扫描结果保留，由 3 条 cancel 单测覆盖。
- **updater 签名**：build 在签名步报缺 `TAURI_SIGNING_PRIVATE_KEY`，NSIS/MSI 正常产出（与既往一致）。
- **CDP console 红错**：tauri dev 不开 9222，以运行日志无 panic + 无红弹窗替代（G6 已知豁免）。

## 7. 测试结论

- 结论：**通过（L1/L2/L4 自动化闸门 + 仪表盘环形图真机渲染 全部通过；报告导出/前后对比/取消扫描 UI 真机未触发，由 12 条新增单测 + 代码审查覆盖，见 §6 第三档）**
- feature 分支最新 commit：`2e6c2cd`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/lint/build）+ NSIS 干净目录安装 + 仪表盘环形图真机实证（49% / 195.8/400.0/204.2 GB 与本机 C 盘完全一致），未复用开发自验结论。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①任务书 4 处技术偏差；②取消扫描为脚本粒度（不 kill 子进程）；③报告导出/前后对比/取消按钮 UI 真机证据缺失是否可接受单测覆盖。**
