# 测试报告：task-069 v0.2.0 发版准备（版本号升级 + CHANGELOG + README + 用户手册同步）

- 分支：`feature/task-069-release-v0.2.0`
- 测试 HEAD：`b8dcd1d`
- 测试时间：2026-10-31（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）
- **v0.2.0 发版准备任务（M1 四模块全部闭环后的版本收尾）**

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| 构建验证 | `cargo check` + `pnpm build` | 是 | 通过 |
| 打包安装（干净目录） | NSIS `C-DiskCleaner_0.2.0_x64-setup.exe`（4.45 MiB / 4662571 字节），装到 `D:\qa-069-install` | 是 | 通过（安装态版本号 0.2.0 实证） |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区 + 前端（无代码改动，回归保护） | `cargo test --workspace` / `pnpm --filter app test` | 通过 | `cargo-test.log`：57 batches / **431 passed / 0 failed**（与 068 基线完全一致，符合纯文档预期）；`pnpm.log`：13 files / **148 passed**（与 068 基线完全一致） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy -D warnings` / `pnpm lint` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（4 条既有 warning）；`pnpm.log`：lint=0（2 条 SafeBox 既有 warning）、build=0（230.32 kB / gzip 71.78 kB，与 068 完全一致） |
| L3 文档检查 | 版本号四处 + CHANGELOG + README + USER_GUIDE | grep 版本号 / 文件 diff | 通过 | `version.txt`：app/package.json=0.2.0、根 package.json=0.2.0、tauri.conf.json=0.2.0、src-tauri Cargo.toml=0.2.0；engine/elevation Cargo.toml 正确保持 0.1.0；配置文件中 0.1.7 零残留；CHANGELOG 顶部已加 `## [0.2.0] - 2026-10-31` |
| L4 真机/打包 | NSIS 0.2.0 干净目录安装 → 版本号实证 → 卸载 | `/S /D=D:\qa-069-install`（`-Verb RunAs`） | 通过 | `l4.txt`：installed=True、**installed FileVersion=0.2.0、ProductVersion=0.2.0**、pid 在案、cleaned=True；`dashboard.png`：8 个 tab 全在无回归；NSIS 文件名已变为 `C-DiskCleaner_0.2.0_x64-setup.exe`（旧 0.1.7/0.1.6 安装包仍在 bundle 目录但不影响） |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `app/package.json` 版本号 `0.1.7` → `0.2.0` | `app/package.json:4` `"version": "0.2.0"` | 通过 |
| 2 | 其他版本号字段同步升级 | 根 package.json=0.2.0、tauri.conf.json=0.2.0、src-tauri/Cargo.toml=0.2.0、Cargo.lock 自动同步；engine/elevation 正确保持 0.1.0（内部构件不单独发版） | 通过 |
| 3 | `pnpm build` 成功无版本不一致报错 | `pnpm build`=0；pnpm 包名已变为 `app@0.2.0`（tauri-build.log 36 行） | 通过 |
| 4 | `CHANGELOG.md` 顶部加 `## [0.2.0] - 2026-10-31` | CHANGELOG 顶部已加（开发备注 3454 字符 / 新增 17 / 变更 4 / 修复 2 / 已知问题 10） | 通过 |
| 5 | 「新增」段含 task-062~068 全部 14 项功能 | 开发备注：17 条 = 任务书 14 项 + 补入 task-061 三项（主题/趋势图/自动扫描，因 061 合并在 v0.1.7 发版之后） | 通过（超集，见 §6 第二档） |
| 6 | Keep a Changelog 格式正确 | `## [0.2.0] - 2026-10-31` + `### 新增/变更/修复/已知问题` | 通过 |
| 7 | `README.md` 功能列表加 v0.2.0 新功能 | 开发备注：功能段从 7 条平铺改为三组 17 条（核心清理 6 / 系统深度清理 4 / 体验与集成 7），含八个页签总览 | 通过 |
| 8 | 与实际功能一致（无夸大、无遗漏） | README 每条写「为什么不做某操作」（WinSxS 禁 /ResetBase、大文件页只读等），技术栈如实写「手写 windows crate FFI 不引 winreg」 | 通过 |
| 9 | `docs/USER_GUIDE.md` 加 12 项新功能使用说明 | 开发备注：8 章 331 行 → 15 章 766 行，新增 7 章 + 5 条 FAQ；章节顺序与界面页签顺序一致；内链 30 条锚点全部有效 | 通过 |
| 10 | 普通用户可按说明操作（无技术术语堆砌） | 开发备注：修掉三处与实现不符的说法（还原点序号显示、释放 X 与磁盘差值、UAC 弹起的 exe 名） | 通过 |
| 11 | `pnpm --filter app build` 退出码 0 | build=0 | 通过 |
| 12 | `cargo check --workspace` 退出码 0 | clippy 编译通过（含 check 阶段）=0 | 通过 |

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
1. **白名单漏 2 个文件**：根 `package.json`（也有 version 字段，不改会留「根包 0.1.7 / 应用 0.2.0」矛盾版本号）、`Cargo.lock`（改 Cargo.toml 的必然结果，不提交会让 `--locked` 的 CI 红）。开发按任务书「确认无遗漏」条款处理，合理。
2. **CHANGELOG 新增 14 项 → 17 项**：补入 task-061 三项（深浅色主题/30 天趋势图/自动扫描）。依据：061 合并在 v0.1.7 发版之后（merge-base 实证），不补就是「代码里有、CHANGELOG 里没有」。验收标准第 5 条「含 task-062~068 全部 14 项」已满足，17 是超集。
3. **任务书第 7 条与项目流程冲突**：任务书写「CI（必做）：开 PR 触发 windows-latest 五道闸全绿」，但项目现行流程是「开发轮不跑、测试轮跑」。开发按现行流程执行，建议派发方后续统一任务书模板里的 CI 条款。
4. **`engine/Cargo.toml`、`elevation/Cargo.toml` 保持 0.1.0 未同步**：内部构件不单独发版，v0.1.7 发版时同样保持 0.1.0，合理但需评审确认。
5. **`latest.json` 保持 0.1.0 未手工改**：updater 索引文件由 `release.yml` 在 `workflow_dispatch` 时用 `scripts/release-lib.mjs generate-latest` 整份覆盖（真实签名 + 下载地址），手工改会造出「版本 0.2.0 / 签名占位符」的假清单，合理。

### 第三档（已登记为已知限制）
- **发版动作本身要人来点**：GitHub → Actions → Release → Run workflow，`version` 填 `0.2.0`。本地只能验到「能出包 + 版本号对」，签名与 `latest.json` 由 CI 完成。
- **CI 五道闸未跑**：开发轮不建 PR，测试轮本轮也未跑 CI（按既往流程，PR 由「提示词 B（CI 门禁观察员）」创建后跑）。
- **签名**：本地未签名（`A public key has been found, but no private key`），属已知预期；仓库根的 `cclean-new.key` 是 base64 包着的 rsign 私钥，本地启用需额外解码处理；签名正源是 CI 的 secret。
- **UI 回归截图**：`dashboard.png` 证实 8 个 tab（仪表盘/清理/大文件/残留/注册表/保险箱/历史/设置）全在、4 卡/环形图/WinSxS 卡片无回归；未点入各页面（纯文档任务无前端行为变化，任务书允许裁剪真机冒烟）。
- **`tasks/060-shutdown-experience.md` 仍是「工作区已删除、HEAD 里存在」**：从 task-063 起连续 7 个任务如此，每个任务都用精确 `git add` 绕开，需拍板是误删还是有意归档（流程噪音）。
- **深色主题只覆盖 5/8 页**（Items/SafeBox/History 未适配），已在 CHANGELOG 已知问题登记。

## 7. 测试结论

- 结论：**通过（版本号四处全部 0.2.0 + 安装态 FileVersion/ProductVersion=0.2.0 实证 + CHANGELOG/README/USER_GUIDE 同步 + 431/148 回归零波动 + NSIS 出包 4.45 MiB）**
- feature 分支最新 commit：`b8dcd1d`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/lint/build）+ NSIS 0.2.0 干净目录安装 + 安装态版本号实证，未复用开发自验结论。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①白名单漏 2 文件（根 package.json / Cargo.lock）是否认可；②CHANGELOG 17 项（补入 061 三项）是否认可；③任务书 CI 条款与现行流程冲突是否需统一模板；④engine/elevation/latest.json 保持旧版本号是否认可。v0.2.0 发版准备就绪，下一步等用户手动 workflow_dispatch 发版，后续进入 P1 体验项（趋势图增强 / 方案模板）或架构债（ps1→Rust 重写）。**
