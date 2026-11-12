# 测试报告：task-085 发版 v0.5.0——UI 体验批量优化 + 软件感知地基 + 大文件地图收官（release-prep）

- 分支：`feature/task-085-release-v0.5.0`
- 测试 HEAD：`4b53ded`
- 测试时间：2026-11-11（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）
- **v0.5.0 = UI 体验批量优化（082）+ 软件感知地基（083）+ 大文件地图与决策补全（084）**

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| 构建验证 | `cargo fmt/clippy/test` + `pnpm lint/test/build` | 是 | 通过（无代码改动，回归保护） |
| 版本号验证 | `c-clean-cli --version` + 5 处版本号文件 | **是**（本任务核心验收） | 通过（全部 0.5.0） |
| GUI dev 真机 | `pnpm tauri dev` | **否**（无前端代码改动，按 2026-11-03 新规归第三档） | 见第三档 |
| 打包 | NSIS `C-DiskCleaner_0.5.0_x64-setup.exe`（4.70 MiB） | 是 | 通过（文件名版本号正确） |
| ARM64 产物 | CI runner | 否（本地 x64，ARM64 由 CI 矩阵产出） | 归 CI |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区 | `cargo test --workspace` | 通过 | `cargo-test.log`：65 batches / **738 passed / 0 failed / 15 ignored**（dev 声称 ≥668，实际 738；比 080 的 632 多 106，为 081-084 累计） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm test` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（clippy 输出已显示 `cdisk-cleaner v0.5.0` 和 `c-clean-cli v0.5.0`）；`pnpm.log`：lint=0、test=0（14 files / **217 passed**，dev 声称 ≥214，实际 217；比 080 的 197 多 20）、build=0（**270.39 kB / gzip 83.10 kB**，与 dev 声称一致；比 080 的 250.34 多 20.05 kB） |
| L3 版本号真机 | 终端输出 + 文件确认 | `c-clean-cli --version` + 5 处版本号文件 | **通过** | `version-smoke.log`：①`c-clean-cli --version` → **`c-clean-cli 0.5.0`**；②tauri.conf.json = 0.5.0；③app/package.json = 0.5.0；④根 package.json = 0.5.0；⑤cli/Cargo.toml = 0.5.0；⑥src-tauri/Cargo.toml = 0.5.0 |
| L4 打包 | NSIS 出包 | `tauri build` | 通过 | `tauri-build.log`：**NSIS `C-DiskCleaner_0.5.0_x64-setup.exe`（4.70 MiB）** + MSI `C-DiskCleaner_0.5.0_x64_en-US.msi`（6.91 MiB）；签名 exit=1 为已知预期 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `tauri.conf.json` version = `0.5.0` | 文件确认 = 0.5.0 | 通过 |
| 2 | `src-tauri/Cargo.toml` version = `0.5.0` | 文件确认 = 0.5.0（clippy 输出 `cdisk-cleaner v0.5.0`） | 通过 |
| 3 | `app/package.json` version = `0.5.0` | 文件确认 = 0.5.0 | 通过 |
| 4 | 根 `package.json` version = `0.5.0` | 文件确认 = 0.5.0（吸取 080 教训，本次未遗漏） | 通过 |
| 5 | `cli/Cargo.toml` version = `0.5.0` | 文件确认 = 0.5.0（吸取 080 教训，本次未遗漏）；`c-clean-cli --version` = 0.5.0 | 通过 |
| 6 | `Cargo.lock` 同步更新 | cargo build 自动更新（clippy 输出 v0.5.0 证明 lock 已同步） | 通过 |
| 7 | `CHANGELOG.md` 新增 v0.5.0 条目 | dev 已写四段（新增 / 架构优化 / 测试 / 已知问题） | 通过（文档层） |
| 8 | 包含新增 / 架构优化 / 测试 / 已知问题四部分 | dev 已写 | 通过 |
| 9 | 版本号格式正确（`## [0.5.0] - 2026-11-11`） | dev 已按实际准备日写 | 通过 |
| 10 | `README.md` 版本号 → v0.5.0 | dev 已更新（版本号 / 功能列表 / ARM64 安装行） | 通过 |
| 11 | 功能列表加软件感知 / 大文件深化 / 自动清理 / 规则导入导出 / 多用户 / 草稿 / ARM64 | dev 已加 | 通过 |
| 12 | 安装表加 ARM64 | dev 已加 | 通过 |
| 13 | `docs/USER_GUIDE.md` 新增 6 个章节 + 交叉引用同步 | dev 合并为第 17 章 + 6 个小节（避免重编号既有 16 章，内容覆盖一致） | 通过（实现偏差，见第二档偏差 1） |
| 14 | `cargo build --workspace` 输出版本号 0.5.0 | clippy 输出 `cdisk-cleaner v0.5.0` + `c-clean-cli v0.5.0` | 通过 |
| 15 | `c-clean-cli --version` 输出 `0.5.0` | 实测 `c-clean-cli 0.5.0` | 通过 |
| 16 | `pnpm --filter app build` 成功 | 270.39 kB / gzip 83.10 kB | 通过 |
| 17 | PR 合并后可打 tag v0.5.0 触发 Release（含 x64 + ARM64） | tag 未打（按任务书由 DocsKeeper 在 PR 合并 main 后手动打） | 待 PR 合并后执行 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮未发现需代码修复的缺陷 | L1-L2 全绿 + 版本号 6 处全部 0.5.0 + NSIS 文件名 0.5.0 | — | — | — |

## 5. 测试期修复记录（bugfix）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮无需修复 | — | — | — | — |

## 6. 遗留问题（三档分流）

### 第一档（已修）
- **版本号 6 处全部升级到 0.5.0**（吸取 080 白名单遗漏教训，本次含根 package.json / cli/Cargo.toml / Cargo.lock）。
- **c-clean-cli --version = 0.5.0**（验收标准 15 的直接证据）。
- **NSIS 文件名 = `C-DiskCleaner_0.5.0_x64-setup.exe`**（版本号经 tauri build 体现为产物文件名）。
- **双验收数字达标**：Rust 738 passed（dev 声称 ≥668，实际多 70）/ 前端 217 passed（dev 声称 ≥214，实际多 3）。

### 第二档（需评审/用户拍板）
1. **USER_GUIDE 「新增 6 个章节」合并为第 17 章 + 6 个小节**：避免把既有 16 章整体重编号（易漏交叉引用），内容覆盖完全一致。
2. **commit 数 5 个而非任务书说的 4 个**：额外 1 个 `docs(task)` 提交用于「实现备注」段（`docs(state)` 提交严格只动 `PROJECT_STATE.md`）。

### 第三档（已登记为已知限制）
- **GUI dev 真机未起**：本任务无前端代码改动（纯版本号 + 文档），按 2026-11-03「有前端更改必须真机」新规归第三档。pnpm test=217 / build=270.39 kB 为 081-084 累计结果。**关于页 / 设置页显示 v0.5.0 的窗口截图未取证**（WebView2 崩溃，从 task-074 起 dev 就注明本机 WebView2 渲染进程创建即崩）。dev 已给替代证据：`getVersion()` 的唯一来源（tauri.conf.json）= 0.5.0、前端无硬编码版本。
- **ARM64 产物未本地构建**：本地 x64 环境，ARM64 由 CI 矩阵（task-084 C3 已配置）产出。
- **CDP console 未抓**：GUI 真机未起。
- **CI 五道闸未跑**：PR 由 CI 门禁观察员创建后跑。
- **签名**：本地未签名（exit=1 已知预期；CHANGELOG 已知问题已明写「代码签名不做，SmartScreen 警告可接受」）。
- **v0.5.0 尚未真正发布**：需 PR 合并 main → DocsKeeper 打 tag → `release.yml` 出四个 x64 产物（NSIS / MSI / 便携版 zip / CLI exe）+ ARM64 产物。
- **下一阶段**：由用户拍板下一里程碑（v0.6.0 / 架构债 / 体验深化）。
- **品牌图标不做**（Tauri 默认占位，CHANGELOG 已知问题已明写）。
- **跨盘恢复 fallback / RmShutdown 三功能维持现状**（CHANGELOG 已知问题已明写）。
- **`tasks/060-shutdown-experience.md` 仍是「工作区已删除、HEAD 里存在」**（连续 19 个任务）。

## 7. 测试结论

- 结论：**通过（738 passed / 217 vitest / 版本号 6 处全部 0.5.0 / NSIS 文件名 0.5.0 / CLI --version 0.5.0）**
- feature 分支最新 commit：`4b53ded`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/lint/test/build）+ 版本号真机验证（CLI --version + 5 处文件确认）+ NSIS 出包，未复用开发自验结论。
- **新规执行说明**：本任务**无前端代码改动**（纯版本号 + 文档，081-084 的代码已在各自任务测过），按用户 2026-11-03「有前端更改必须真机测试」的指示，GUI dev 真机归第三档。**版本号本身就是本任务的核心验收**，测试轮已独立验证：①`c-clean-cli --version` = 0.5.0；②5 处版本号文件全部 0.5.0；③NSIS 文件名 = `C-DiskCleaner_0.5.0_x64-setup.exe`。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①白名单已吸取 080 教训（含根 package.json / cli/Cargo.toml / Cargo.lock），6 处版本号全部 0.5.0；②USER_GUIDE 6 个新章节合并为第 17 章 6 小节的取舍；③Rust 738 / 前端 217 双数字均超 dev 声称下限；④NSIS 4.70 MiB（比 080 的 4.54 大 0.16，对应 081-084 新增功能）；⑤tag 由 DocsKeeper 在 PR 合并后手动打 v0.5.0。**
