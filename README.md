# C-Disk Cleaner

> 一款面向普通用户与开发者的 Windows C 盘安全清理工具：**先扫描诊断、按风险分级、用户自主选择、执行可追溯**。
> 把「会清理」变成「敢清理、看得见、能回滚」。

**每一个删除决定都有依据，每一次清理都能追溯。**

当前版本：**v0.1.7**（[更新日志](CHANGELOG.md)）· 平台：Windows 10 / 11 x64 · 许可证：MIT

---

## 为什么还要再造一个清理工具

| 对比 | 常见做法 | 本项目的做法 |
|---|---|---|
| 大众清理工具（CCleaner / 360 / 火绒） | 一键全清、默认激进、夹带推广 | 默认保守、**可回滚**、无推广、无遥测 |
| 专业工具（Dism++ / SpaceSniffer / TreeSize） | 功能强但门槛高，每步要自己判断 | 每项**有解释、有影响说明、有审计日志** |

核心差异是三件事：**保险箱（删了能拿回来）、dry-run 预演（删之前先看清单）、审计日志（删了什么、什么时候删的）**。

## 功能

- **扫描**：五类目标——浏览器缓存、开发工具缓存（npm/pnpm/Cargo 等）、预读取文件、系统临时文件、Windows 更新残留。按脚本分阶段推进，实时进度与计数。
- **清理**：按风险分级（L1 可直接清 / L2 需逐项确认 / L3 硬编码禁止）；执行前可 **dry-run 预览**，看到「将被删除的文件数 / 大小 / 路径清单」以及会被拦下的项与原因。
- **保险箱**：所有删除**先移入保险箱**（默认 7 天可恢复）。支持按时间区间、多 session 组合筛选；每条记录显示「来源」批次；可批量恢复（父目录缺失自动创建）或永久删除。
- **占用进程处置**：文件被程序占用导致删不掉时，列出占用进程并可「关闭占用进程并重试」；可选**清理后自动重启被关闭的进程**（默认关闭，支持进程名白名单）。
- **审计日志**：每次扫描 / 清理 / 恢复 / 永久删除都有记录，历史页可按条件查看，统计卡片显示累计释放空间与清理轮次。
- **自动更新**：启动时非阻塞检查新版本，顶部 banner 提示；下载时显示进度条、百分比、已下载大小与实时速度；失败按原因给中文提示并可重试。
- **规则库热更新**：规则从 GitHub 拉取并校验 checksum，支持回滚到上一版；离线时自动降级使用本地规则库。

## 安装

从 [GitHub Releases](https://github.com/An0114/C-clean/releases) 下载最新版本：

| 包 | 文件名 | 说明 |
|---|---|---|
| NSIS（推荐） | `C-DiskCleaner_x.y.z_x64-setup.exe` | 安装向导，体积更小 |
| MSI | `C-DiskCleaner_x.y.z_x64_en-US.msi` | 适合企业环境批量部署（`msiexec /i`） |

> **首次运行会被 SmartScreen 拦截**（安装包未做代码签名）。点「更多信息 → 仍要运行」即可。
> 为什么不做签名、什么条件下会做，见 [`docs/research/code-signing-evaluation.md`](docs/research/code-signing-evaluation.md)。

**系统要求**：Windows 10 1809+ 或 Windows 11，64 位。需要 WebView2 Runtime（Win11 自带，Win10 通常已随 Edge 安装）。

## 开发

前置依赖：

```powershell
rustc --version    # Rust stable（rustup）
node --version     # Node.js >= 20
pnpm --version     # pnpm >= 9（corepack enable 后可用）
# 另需：Microsoft C++ Build Tools、WebView2 Runtime
```

常用命令（仓库根目录执行）：

```powershell
pnpm install                        # 安装前端 workspace 依赖
pnpm tauri dev                      # 开发模式：编译 Rust 壳 + 拉起 Vite dev server
pnpm tauri build                    # 打包（产物在 target/release/bundle/）
pnpm --filter app test              # 前端单测（vitest）
cargo test --workspace              # Rust 全量测试
cargo clippy --workspace --all-targets -- -D warnings
```

打包需要 updater 签名私钥（缺失会以「public key found but no private key」失败）：

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content .\cclean-new.key -Raw
pnpm tauri build
```

## 技术栈

| 层 | 选型 |
|---|---|
| 桌面框架 | [Tauri 2](https://tauri.app/)（Rust 核心 + 系统 WebView，安装包约 4 MB） |
| 前端 | React 18 + TypeScript + Vite |
| 核心引擎 | Rust（扫描 / 分级 / 执行 / 保险箱 / 审计） |
| 本地存储 | SQLite（`rusqlite` bundled，带手写迁移器） |
| 系统集成 | Windows Restart Manager（占用进程处置）、UAC 提权独立 exe |
| 规则库 | JSON（远端拉取 + checksum 校验 + 本地回滚） |

## 目录结构

```
C-clean/
├── app/                  Tauri 应用（React 前端 + src-tauri 主进程壳）
├── engine/               Rust 核心（db / scan / execute / ipc）
├── elevation/            独立提权 exe（按需 UAC，完成即退出）
├── rules/                JSON 规则库（与 scripts/ 共用同一规则源）
├── scripts/              扫描脚本（5 个 scan_*.ps1 + smoke_test.ps1）+ 发版工具（release-lib.mjs）
├── docs/                 设计文档、评审报告、测试报告、技术评估、项目状态白板
├── tasks/                任务文档（按编号拆分）
├── CHANGELOG.md          更新日志
└── LICENSE               MIT
```

## 安全与隐私基线

这是产品生命线，不是宣传语：

- **L3 红线硬编码**：系统关键路径的禁止项在 UI 不出现，代码层二次校验路径白名单。
- **保险箱默认开启**：所有清理先移入保险箱，7 天内可恢复。
- **dry-run 预演**：执行前展示将被删除的完整清单。
- **最小权限**：主进程普通权限，仅写操作按需 UAC 提权，无常驻提权进程。
- **全本地处理**：无遥测、无云上传、无广告 SDK。唯一的网络请求是规则库更新与版本检查，两者失败都不影响核心功能。

## 贡献

欢迎提 Issue 与 PR。改动前建议先读：

- [`docs/PROJECT_STATE.md`](docs/PROJECT_STATE.md)——项目当前状态与已知问题
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)——模块边界与关键约束
- [`docs/USER_GUIDE.md`](docs/USER_GUIDE.md)——面向用户的功能说明（改 UI 时请同步更新）

提交前请确保 `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`、`pnpm --filter app lint/test/build` 全部通过。

## 许可证

[MIT](LICENSE) © 2026 An0114
