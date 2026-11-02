# 测试报告：task-071 v0.2.1 patch 发版准备（版本号 0.2.0→0.2.1 + CHANGELOG Bug 修复段）

- 分支：`feature/task-071-release-v0.2.1`
- 测试 HEAD：见 `target/qa-071/head.txt`
- 测试时间：2026-11-02（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）
- **v0.2.1 patch 发版（纯版本号 + CHANGELOG，无功能改动）**

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| 构建验证 | `cargo check/clippy/test` + `pnpm lint/test/build` | 是 | 通过 |
| 打包安装（干净目录） | NSIS `C-DiskCleaner_0.2.1_x64-setup.exe`（4.46 MiB / 4676295 字节），装到 `D:\qa-071-install` | 是 | 通过（安装态版本号 0.2.1 实证） |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区 + 前端（纯版本号，回归保护） | `cargo test --workspace` / `pnpm --filter app test` | 通过 | `cargo-test.log`：58 batches / **444 passed / 0 failed**（与 070 基线完全一致，符合纯版本号预期）；`pnpm.log`：13 files / **148 passed**（与 070 基线完全一致） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（4 条既有 warning）；`pnpm.log`：lint=0（2 条 SafeBox 既有 warning）、build=0（230.32 kB / gzip 71.78 kB，与 070 完全一致） |
| L3 文档检查 | 版本号四处 + CHANGELOG v0.2.1 | grep 版本号 / 文件 diff | 通过 | `count.txt`：app/package.json=0.2.1、根 package.json=0.2.1、tauri.conf.json=0.2.1、src-tauri/Cargo.toml=0.2.1；engine/elevation 正确保持 0.1.0；CHANGELOG 顶部已加 `## [0.2.1] - 2026-11-02`（修复 3 条 + 已知问题 2 条） |
| L4 真机/打包 | NSIS 0.2.1 干净目录安装 → 版本号实证 → 卸载 | `/S /D=D:\qa-071-install`（`-Verb RunAs`） | 通过 | `l4.txt`：installed=True、**installed FileVersion=0.2.1、ProductVersion=0.2.1**、pid 在案、cleaned=True；`dashboard.png`：8 个 tab 全在、WinSxS 卡片显示「可清理 10.7 GB / 建议清理」（用户真实 DB 数据）；NSIS 文件名已变为 `C-DiskCleaner_0.2.1_x64-setup.exe` |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `app/package.json` 版本号 `0.2.0` → `0.2.1` | `app/package.json` `"version": "0.2.1"` | 通过 |
| 2 | 其他版本号字段同步升级 | 根 package.json=0.2.1、tauri.conf.json=0.2.1、src-tauri/Cargo.toml=0.2.1、Cargo.lock 自动同步 | 通过 |
| 3 | engine/elevation Cargo.toml 保持 0.1.0 | engine=0.1.0、elevation=0.1.0 | 通过 |
| 4 | `pnpm --filter app build` 成功，无版本不一致报错 | build=0；pnpm 包名已变为 `app@0.2.1` | 通过 |
| 5 | `CHANGELOG.md` 顶部加 `## [0.2.1] - 2026-11-02` | CHANGELOG 顶部已加（1183 字符 / 修复 3 / 已知问题 2） | 通过 |
| 6 | 「修复」段含 3 个 Bug 修复说明 | Bug-1 提权 argv 缺失 / Bug-2 ps1 硬编码 L1 / Bug-3 目录永久删除+来源 | 通过 |
| 7 | Keep a Changelog 格式正确 | `## [0.2.1] - 2026-11-02` + `### 修复/已知问题` | 通过 |
| 8 | `pnpm --filter app build` 退出码 0 | build=0 | 通过 |
| 9 | `cargo check --workspace` 退出码 0 | clippy 编译通过 =0 | 通过 |

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
1. **白名单 3 处偏差**：①`Cargo.lock`（白名单未列，改 Cargo.toml 的必然结果，不提交会让 `--locked` CI 红）；②`README.md`（任务书写「不修改」，但只改版本展示行 `v0.2.0→v0.2.1`，功能列表未动，不改会仓库自相矛盾）；③`docs/USER_GUIDE.md`（同上，只改「适用版本」行）。合理但需评审确认。
2. **CHANGELOG 未照抄任务书 3 处**：①Bug-1 写用户能懂的话（不写「argv 数组」天书）；②Bug-2 不写「L0/L1/L2」虚假承诺（实际只做到 L1/L2，L0 待产品决策）；③补写「系统还原点自动创建在 0.2.0 里其实也从未成功过」（与 Bug-1 同路径，此前无 L2 项可清没被走到）。
3. **`latest.json` 保持 0.1.0 未手工改**：updater 索引由 `release.yml` 的 `generate-latest` 在 `workflow_dispatch` 时整份覆盖（真实签名+下载地址），手改会造假清单。
4. **版本号全局搜索**：配置/构建文件中 0.2.0 零残留；剩余命中都是历史 CHANGELOG 段 / docs/tasks 历史描述 / release.yml 与 release-lib.mjs 注释示例 / release-lib.test.mjs 测试夹具，均不该改。

### 第三档（已登记为已知限制）
- **发版动作本身要人来点**：GitHub → Actions → Release → Run workflow，`version` 填 `0.2.1`。本地只能验到「能出包 + 版本号对」，签名与 `latest.json` 由 CI 完成。
- **CI 五道闸未跑**：开发轮不建 PR，测试轮本轮也未跑 CI（按既往流程，PR 由 CI 门禁观察员创建后跑）。
- **签名**：本地未签名（exit=1 已知预期），签名正源是 CI secret。
- **v0.2.0 三个 Bug 的真机最终点验**（WinSxS 分析按钮 / L1+L2 徽标 / 残留永久删除+来源列）仍未在测试轮自动完成（UAC + DPI 坐标偏移），由 070 的 13 条新单测 + 开发取证覆盖。**建议用户发版后手动点一遍这三处确认。**
- **`tasks/060-shutdown-experience.md` 「工作区已删除、HEAD 里存在」连续 9 个任务未决**（流程噪音）。
- **下一步**：P1-7 清理方案模板（task-072）或 P1 体验项 / 架构债（深色补齐 Items/SafeBox/History 三页）。

## 7. 测试结论

- 结论：**通过（版本号四处全部 0.2.1 + 安装态 FileVersion/ProductVersion=0.2.1 实证 + CHANGELOG v0.2.1 段落 + 444/148 回归零波动 + NSIS 出包 4.46 MiB）**
- feature 分支最新 commit：见 `target/qa-071/head.txt`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/lint/build）+ NSIS 0.2.1 干净目录安装 + 安装态版本号实证，未复用开发自验结论。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①白名单 3 处偏差（Cargo.lock / README.md 版本行 / USER_GUIDE 适用版本行）是否认可；②CHANGELOG 未照抄任务书 3 处（用户化 Bug-1 写法 / 不承诺 L0 / 补写还原点从未成功）是否认可。v0.2.1 patch 发版准备就绪，下一步等用户手动 workflow_dispatch 发版（version=0.2.1），后续进入 P1-7 清理方案模板（task-072）或架构债。**
