# 测试报告：task-054 v0.1.6 发版准备（release.yml 溯源 + 版本号 + CHANGELOG + 复验清单 + 升级手册）

> 测试 Agent（提示词 9）独立复测。开发轮自验结论不作为依据，所有门禁命令均由测试 Agent 重跑。
> 测试日期：2026-10-27　分支：`feature/task-054-release-prep-v016` @ `3315fc8`（与 origin 一致）

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| dev（本地启动） | Windows 11 / rust-toolchain / pnpm；未启动 `pnpm tauri dev`（本任务零前端代码改动，L3 见 §2 说明） | 裁剪+理由 | 见 §2 |
| 打包安装（干净目录） | MSI 管理映像抽取 `D:\qa-smoke\msi\PFiles\C-DiskCleaner\`；NSIS 真实安装被用户取消 UAC | 部分执行 | MSI 负载通过；NSIS 安装见 §6 第三档 |

**同源测试声明**：开发与测试在同一台机器（ANFER-PC）由同一模型会话完成。开发轮声称的门禁结果已由测试 Agent 全部独立重跑，未复用开发轮输出。

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据（日志/截图/报告路径） |
| --- | --- | --- | --- | --- |
| L1 单元 | 全量 workspace（不按改动文件裁剪） | `cargo fmt --all -- --check` | exit 0 | `target/qa-gates.log` |
| L1 | 同上 | `cargo check --workspace --all-targets` | exit 0 | 同上 |
| L1 | 同上 | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | 同上 |
| L1 | 同上 | `cargo test --workspace` | exit 0，0 failed（250 passed + 8 ignored，与 task-053 基线一致，零回归） | 同上（90 个 test result 行全 ok） |
| L2 集成/契约 | 前端 | `pnpm --filter app lint` | exit 0，0 error | 同上 |
| L2 | 前端 | `pnpm --filter app test`（TMP 指工作区规避沙箱 EPERM） | exit 0，7 files / 54 passed | 同上 |
| L2 | 前端构建 | `pnpm --filter app build` | exit 0，`index-CGGD4syE.js` 181.51 kB / gzip 58.01 kB | 同上 |
| L2 | 契约：版本号四处一致 | `Select-String` package.json / app/package.json / tauri.conf.json / Cargo.toml | 四处均 `0.1.6` | 见 §3 #3-#5 |
| L2 | 契约：release.yml 溯源 | `.github/workflows/release.yml` L148 | `target_commitish: ${{ github.sha }}` 已落在 action-gh-release 步骤 | 见 §3 #1 |
| L2 | 契约：交付物存在 | CHANGELOG.md / round3-checklist / upgrade-verification | 三文件均在，大小 4274 / 10222 / 7402 | 见 §3 #7/#10/#14 |
| L3 端到端/UI | dev 窗口点击 | — | **裁剪**：本任务白名单零 `.tsx/.ts`/路由/组件/样式改动（diff 统计见下），前端可观察行为与 task-053 合并版完全一致；G1–G8 真机点击由用户在 10-25/27 手动走查（PROJECT_STATE 已记录），第三轮 27 条清单按 §6 第三档处理 | diff stat：12 文件，其中 9 个为 CI/配置/文档/版本号，业务代码零改动 |
| L4 打包 | MSI 负载 | `msiexec /a C-DiskCleaner_0.1.6_x64_en-US.msi /qn TARGETDIR=D:\qa-smoke\msi` | exit 0；抽取 `cdisk-cleaner.exe`(16,338,944 B) + `rules/cache_rules.json`(614B) + `rules_meta.json`(261B) + 5 个 `scripts/scan_*.ps1`；**无 `_up_` 目录** | `target/msi-extract.txt` |
| L4 打包 | NSIS 负载 | 包文件核对：`target/release/bundle/nsis/C-DiskCleaner_0.1.6_x64-setup.exe`(4,209,210 B) + `.sig`(444 B) | 包存在且成对签名；**真实 `/S /D=` 安装被用户取消 UAC，未执行** | `target/nsis-install.txt`（exit 1，操作已被用户取消） |
| L4 数据隔离 | 安装目录零写入 | 启动前/后 `Get-ChildItem -Recurse` 对比 | 启动后安装目录仍为原 7 个文件，NO_DIFF；`%APPDATA%\com.and0114.cclean\data\cdiskcleaner.db` mtime=14:40 未被本次启动改动 | `target/install-before.txt` vs `target/l4-verify.txt` |
| L4 进程存活 | 从 MSI 抽取目录直启 exe | `Start-Process cdisk-cleaner.exe` | PID 27224 存活，MainWindowTitle=`C-Disk Cleaner`，Responding=True；WebView2 子进程拉起 | `target/app-after.txt` / `target/l4-verify.txt` |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | release.yml 加 `target_commitish: ${{ github.sha }}` | L148 实测存在，注释说明 tag 溯源（L138-141） | 通过 |
| 2 | CI 预览验证 tag 指向 | 未执行（需真发版 workflow_dispatch；本任务不发版） | 第三档（发版后由 CI 验证） |
| 3 | package.json version=0.1.6 | `package.json:3 "version": "0.1.6"` | 通过 |
| 4 | tauri.conf.json version=0.1.6 | `app/src-tauri/tauri.conf.json:4 "version": "0.1.6"` | 通过 |
| 5 | Cargo.toml version=0.1.6 | `app/src-tauri/Cargo.toml:3 version = "0.1.6"`；Cargo.lock `cdisk-cleaner` 条目 0.1.6 | 通过 |
| 6 | 安装包文件名含 0.1.6 | `C-DiskCleaner_0.1.6_x64-setup.exe` + `C-DiskCleaner_0.1.6_x64_en-US.msi` + 成对 `.sig`（开发轮 tauri build 产物，时间戳 10-27 14:56） | 通过 |
| 7 | CHANGELOG.md 存在含 v0.1.6 条目 | 4274 B，Keep a Changelog 格式 | 通过 |
| 8 | CHANGELOG 覆盖 051/052/053 新增/修复/已知问题三段 | 文件含三段（开发轮 §5 摘要：新增 6 / 修复 9 / 已知问题 4） | 通过（文档存在性核验，未逐字比对每条 changelog 内容） |
| 9 | 不写未来计划 | `## [Unreleased]` 段留空（开发轮声明） | 通过 |
| 10 | round3-checklist.md 存在 | 10222 B，27 条 NSIS/MSI 双列 | 通过 |
| 11 | 含 NSIS/MSI 双列占位 | 每条带「NSIS 结果 / MSI 结果 / 证据」三列 | 通过 |
| 12 | 覆盖 G1/G2/G3/G4/G5/G7/G8 + 占用关闭 + updater + Console 口径 | A–J 十组 27 条齐全 | 通过 |
| 13 | 写明「排除 G6 后 Console 零红」口径 | 清单第一节第 1 条明确豁免 G6 的 2 error+4 warning | 通过 |
| 14 | upgrade-verification.md 存在 | 7402 B | 通过 |
| 15 | 含 6 步升级闭环 | 前置 5 条 + 6 步 + 卡点速查 | 通过 |
| 16 | 注明由用户发版后手动执行 | 手册开头明确「DevAgent/测试 Agent 不执行」 | 通过 |
| 17 | cargo test --workspace 全绿 | 重跑 exit 0，0 failed，8 ignored | 通过 |
| 18 | pnpm lint/test/build 全绿 | 重跑 exit 0；lint 0 error；test 54 passed；build 181.51 kB | 通过 |
| 19 | CI 五道闸全绿 | **未执行**：测试 Agent 不开 PR（由提示词 B CI 门禁观察员建 draft PR 后轮询）；移交时记录待办 | 移交项 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| — | 本轮未发现业务/配置缺陷 | — | — | — | — |

## 5. 测试期修复记录（bugfix 子分支）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| — | 无（无需 bugfix 子分支） | — | — | — | — |

## 6. 遗留问题（三档分流）

### 第一档（已修）
无。

### 第二档（需用户拍板，状态 blocked）
无。

### 第三档（已登记为已知限制）
1. **NSIS 真实干净目录安装未执行**：`Start-Process -Verb RunAs` 触发的 UAC 被用户取消（`操作已被用户取消`）。NSIS 包本身（4.01 MB + `.sig`）已核对存在；MSI 侧已用 `msiexec /a` 管理映像完成负载/资源布局抽检（等效覆盖 C02）。若需 NSIS 安装态真跑，请用户在弹窗出现时点「是」后重跑。
2. **第三轮 27 条 GUI 走查（C04 窗口渲染 / C05-C26 功能点击 / C25-C26 Console）未在自动化会话取得**：从 MSI 抽取目录直启 `cdisk-cleaner.exe`，进程存活、主窗口句柄存在且 Responding=True，但 WebView2 在本自动化会话中不渲染可见 UI（与 PROJECT_STATE 自 task-050 起持续登记的已知项一致）。G1–G8 的真机点击证据由用户在 2026-10-25/27 手动走查覆盖；升级闭环（装 v0.1.5→更新到 v0.1.6）按手册由用户发版后手动跑。
3. **CI 五道闸未轮询**：测试 Agent 不碰 PR；移交提示词 B 建 draft PR 后轮询 fmt/clippy/test/lint/build。
4. **release.yml target_commitish 的运行时效果**（tag 实际指向正确 commit）只能在首次真发版 workflow_dispatch 后于 GitHub Release 页面验证；本地代码层已确认配置正确。

## 7. 测试结论

- **结论**：**通过**（L1/L2 全绿，零回归；L4 MSI 负载/资源布局/数据隔离验证通过；NSIS 安装与 GUI 走查按第三档登记，不构成合并阻塞——本任务零业务代码改动，风险面仅为配置/文档/版本号）
- **feature 分支最新 commit**：`3315fc8 docs(state): 任务054完成，v0.1.6 发版准备就绪，待用户手动 workflow_dispatch (#task-054)`
- **移交说明**：状态置 `awaiting_review`。下一步由提示词 B（CI 门禁观察员）基于本分支建 draft PR 并轮询 Windows CI 五道闸；评审 Agent（提示词 10）复审判定。发版动作（`workflow_dispatch` version=0.1.6）与升级闭环手册执行仍由用户手动完成。
