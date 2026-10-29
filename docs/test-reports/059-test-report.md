# 测试报告：task-059 v0.1.7 发版准备（版本号升级 + CHANGELOG 定稿 + README 重写 + 用户手册）

- 分支：`feature/task-059-release-prep-v017`
- 测试 HEAD：`b4aa2d0`
- 测试时间：2026-10-29（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）
- 任务性质：纯文档 / 版本号改动，无业务代码、无前端可观察行为

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| dev（pnpm tauri dev） | Win11 25H2 | 裁剪（理由：本任务无 UI 改动，dev 真机无新行为可点） | N/A |
| 打包安装（干净目录） | NSIS `C-DiskCleaner_0.1.7_x64-setup.exe`（4.05 MiB），装到 `D:\qa-059-install` | 是 | 通过 |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust lib + 前端组件 | `cargo test --workspace` / `pnpm --filter app test` | 通过 | `target/qa-059/cargo-test.log`：全部 `ok`，合计 273 passed / 8 ignored / 0 failed；`pnpm.log`：Test Files 9 passed / Tests 89 passed |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（cdisk-cleaner v0.1.7 编译通过）；`pnpm.log`：lint=0（2 条 SafeBox react-refresh 既有 warning）、build=0（187.46 kB / gzip 60.30 kB） |
| L3 端到端/UI | dev 真机 | — | 裁剪 | 本任务无前端代码改动（仅版本号、文档），dev 真机行为与 task-058 一致，无新可测点 |
| L4 真机/打包 | NSIS 干净目录安装 → 启动 → 卸载 | `/S /D=D:\qa-059-install` + 启动 + 静默卸载 | 通过 | `l4.txt`：安装包 FileVersion=0.1.7、安装后 exe FileVersion=0.1.7；`l4-launch.txt`：进程启动 title=C-Disk Cleaner、卸载干净 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `package.json` version = `0.1.7` | `package.json:3 "version": "0.1.7"`（`static.txt`） | 通过 |
| 2 | `app/src-tauri/tauri.conf.json` version = `0.1.7` | `tauri.conf.json:4 "version": "0.1.7"` | 通过 |
| 3 | `app/src-tauri/Cargo.toml` version = `0.1.7` | `Cargo.toml:3 version = "0.1.7"` | 通过 |
| 4 | 产物文件名含 `0.1.7` | `target/release/bundle/nsis/C-DiskCleaner_0.1.7_x64-setup.exe`（4.24 MB）、`.../msi/C-DiskCleaner_0.1.7_x64_en-US.msi`（6.16 MB）；安装包 FileVersion=0.1.7 | 通过 |
| 5 | CHANGELOG v0.1.7 日期 = 2026-10-29 | `CHANGELOG.md:11 ## [0.1.7] - 2026-10-29` | 通过 |
| 6 | 覆盖 RmRestart / prefs / restored_at / safe_box 原子性 / 跨盘 / 白名单 / G6 | 开发备注 §4 摘要：新增 4 / 修复 1 / 变更 2 / 已知问题 5，逐条覆盖 | 通过 |
| 7 | 不写 Unreleased 段 | `Select-String "Unreleased"` 无命中 | 通过 |
| 8 | README 含项目名 + 一句话简介 | `README.md:1 # C-Disk Cleaner` + 首段定位 | 通过 |
| 9 | 功能列表 ≥6 项 | README「## 功能」7 项：扫描 / 清理 / 保险箱 / 占用进程 / 审计 / 自动更新 / 规则库热更新 | 通过 |
| 10 | 含安装指引（GitHub Releases） | README「## 安装」NSIS + MSI 对照表 + SmartScreen 绕过 | 通过 |
| 11 | 含开发指引（pnpm install + pnpm tauri dev） | README「## 开发」前置依赖 + 常用命令 | 通过 |
| 12 | 含技术栈列表 | README「## 技术栈」Tauri 2 / React / Rust / SQLite / Restart Manager / JSON 规则库 | 通过 |
| 13 | LICENSE 存在且为 MIT | `LICENSE` 存在，首行 `MIT License`，`Copyright (c) 2026 An0114` | 通过 |
| 14 | `docs/USER_GUIDE.md` 存在 | `Test-Path docs/USER_GUIDE.md = True` | 通过 |
| 15 | 覆盖 8 节（快速开始/扫描/清理/保险箱/历史/设置/更新/FAQ） | 标题列表：1 快速开始 / 2 扫描 / 3 清理 / 4 保险箱 / 5 历史与审计 / 6 设置 / 7 更新机制 / 8 常见问题 | 通过 |
| 16 | 语言为中文 | 全文中文（标题/正文/FAQ） | 通过 |
| 17 | `cargo check --workspace --all-targets` 退出码 0 | clippy 编译通过（cdisk-cleaner v0.1.7），cargo test --workspace exit=0 | 通过 |
| 18 | `pnpm lint / test / build` 全绿 | lint=0、test=89 passed、build=0（187.46 kB） | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮未发现缺陷 | L1-L4 全绿 | — | — | — |

## 5. 测试期修复记录（bugfix 子分支）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮无需修复 | — | — | — | — |

## 6. 遗留问题（三档分流）

### 第一档（已修）
- 无。

### 第二档（需用户拍板，状态 blocked）
- 无本任务新增项。task-058 遗留的「白名单 fail-closed 语义」偏差仍由评审 Agent 复核。

### 第三档（已登记为已知限制）
- **L3 dev 真机 UI 裁剪**：本任务纯文档/版本号改动，无前端代码 diff，dev 真机行为与 task-058 一致。裁剪理由：无新可测 UI 行为；影响面：零。
- **README 截图位未填**：开发备注 §11 已说明——空目录 Git 不跟踪、挂破图链接比不放更糟，待真实截图产出后补「## 截图」段。
- **USER_GUIDE 真人走查**：手册 8 节按代码与实际 UI 文案 grep 后撰写，但「按手册从头点一遍」的真人走查未在测试轮执行（本环境只能验证功能存在，不能验证手册叙述与按钮位置的 100% 一致）。建议用户发版后首跑时顺手核对。
- **updater 签名**：build 仍需 `TAURI_SIGNING_PRIVATE_KEY`，本次 NSIS/MSI 为未签名包（与 0.1.6 一致）。
- **v0.1.7 发版**：待用户手动 `workflow_dispatch`（version=0.1.7），不在本任务范围。

## 7. 测试结论

- 结论：**通过**
- feature 分支最新 commit：`b4aa2d0`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁与 L4 打包冒烟，未复用开发自验结论。测试轮未发现需代码修复的缺陷；L1/L2/L4 全绿，L3 按理由裁剪。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审复核：①L3 dev 真机裁剪是否认可；②USER_GUIDE 真人走查未做是否接受作为第三档遗留；③task-058 白名单 fail-closed 语义偏差仍待拍板。**
