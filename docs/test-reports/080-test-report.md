# 测试报告：task-080 发版 v0.4.0——R5 架构债收官 + P2 全部完成（release-prep）

- 分支：`feature/task-080-release-v0.4.0`
- 测试 HEAD：`7242962`
- 测试时间：2026-11-07（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）
- **v0.4.0 = R5 架构债（5 个 ps1 Rust 重写）+ P2（多盘扫描 / 用户自定义规则 / 便携版 CLI）**

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| 构建验证 | `cargo fmt/clippy/test` + `pnpm lint/test/build` | 是 | 通过（无代码改动，回归保护） |
| 版本号验证 | `c-clean-cli --version` + 6 处版本号文件 | **是**（本任务核心验收） | 通过（全部 0.4.0） |
| GUI dev 真机 | `pnpm tauri dev` | **否**（无前端代码改动，按 2026-11-03 新规归第三档） | 见第三档 |
| 打包 | NSIS `C-DiskCleaner_0.4.0_x64-setup.exe`（4.54 MiB） | 是 | 通过（文件名版本号正确） |
| 干净目录安装 | MSI per-user 静默安装 | dev 已做（FileVersion=0.4.0 / 卸载成功） | 通过（dev 端证据） |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区 | `cargo test --workspace` | 通过 | `cargo-test.log`：65 batches / **632 passed / 0 failed / 15 ignored**（与 079 完全相同，无代码改动） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm test` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（clippy 输出已显示 `cdisk-cleaner v0.4.0` 和 `c-clean-cli v0.4.0`）；`pnpm.log`：lint=0、test=0（14 files / **197 passed**）、build=0（**250.34 kB / gzip 77.59 kB**，与 079 完全相同） |
| L3 版本号真机 | 终端输出 + 文件确认 | `c-clean-cli --version` + 6 处版本号文件 | **通过** | `version-smoke.log`：①`c-clean-cli --version` → `c-clean-cli 0.4.0`；②tauri.conf.json = 0.4.0；③app/package.json = 0.4.0；④根 package.json = 0.4.0；⑤cli/Cargo.toml = 0.4.0；⑥src-tauri/Cargo.toml = 0.4.0 |
| L4 打包 | NSIS 出包 | `tauri build` | 通过 | `tauri-build.log`：**NSIS `C-DiskCleaner_0.4.0_x64-setup.exe`（4.54 MiB）** + MSI `C-DiskCleaner_0.4.0_x64_en-US.msi`（6.70 MiB）；签名 exit=1 为已知预期 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `tauri.conf.json` version = `0.4.0` | 文件确认 = 0.4.0 | 通过 |
| 2 | `src-tauri/Cargo.toml` version = `0.4.0` | 文件确认 = 0.4.0（clippy 输出 `cdisk-cleaner v0.4.0`） | 通过 |
| 3 | `app/package.json` version = `0.4.0` | 文件确认 = 0.4.0 | 通过 |
| 4 | 根 `Cargo.toml`（如统一版本）同步 | 根 Cargo.toml 是纯 workspace 清单（无 [package] 段），无 version 字段，不需要改 | 通过（实现偏差，见第二档偏差 3） |
| 5 | `CHANGELOG.md` 新增 v0.4.0 条目 | dev 已写四段（新增 / 架构优化 / 测试 / 已知问题） | 通过（文档层） |
| 6 | 包含新增功能 / 架构优化 / 测试 / 已知问题四部分 | dev 已写 | 通过 |
| 7 | 版本号格式正确（`## [0.4.0] - 2026-11-06`） | 实际写 `2026-11-07`（实际准备日） | 通过（日期偏差，见第二档偏差 6） |
| 8 | `README.md` 版本号 → v0.4.0 | dev 已更新（版本号 / 功能列表 / 安装表 / 目录结构 / 技术栈 / 开发命令） | 通过 |
| 9 | 功能列表加多盘扫描 / 自定义规则 / CLI / 便携版 | dev 已加 | 通过 |
| 10 | `docs/USER_GUIDE.md` 同步 | dev 已同步（版本号 / 3.1 多盘 / 4.1~4.4 / 5.1 / 12 章 / 新增 15 章） | 通过 |
| 11 | `cargo build --workspace` 输出版本号 0.4.0 | clippy 输出 `cdisk-cleaner v0.4.0` + `c-clean-cli v0.4.0` | 通过 |
| 12 | `pnpm --filter app build` 输出版本号 0.4.0 | pnpm build 本身不打印版本（Vite 只打产物大小）；版本由 tauri.conf.json 决定，经 tauri build 体现为产物文件名（NSIS `C-DiskCleaner_0.4.0_x64-setup.exe`） | 通过（实现偏差，见第二档） |
| 13 | PR 合并后可打 tag v0.4.0 触发 Release | tag 未打（按任务书由 DocsKeeper 在 PR 合并 main 后手动打） | 待 PR 合并后执行 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮未发现需代码修复的缺陷 | L1-L2 全绿 + 版本号 6 处全部 0.4.0 + NSIS 文件名 0.4.0 | — | — | — |

## 5. 测试期修复记录（bugfix）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮无需修复 | — | — | — | — |

## 6. 遗留问题（三档分流）

### 第一档（已修）
- **版本号 6 处全部升级到 0.4.0**：tauri.conf.json / src-tauri/Cargo.toml / app/package.json / 根 package.json / cli/Cargo.toml / Cargo.lock。
- **c-clean-cli --version = 0.4.0**（读 `env!("CARGO_PKG_VERSION")`，验收标准 12 的直接证据）。
- **NSIS 文件名 = `C-DiskCleaner_0.4.0_x64-setup.exe`**（版本号经 tauri build 体现为产物文件名）。
- **dev 已做 MSI per-user 静默安装实测**：装到 `target\install-smoke`，exit 0；`cdisk-cleaner.exe` FileVersion=0.4.0 / ProductVersion=0.4.0 / 17.95 MB；安装目录不含 `portable_data`（正确——安装版不该被便携 override 污染）；卸载 exit 0，目录消失。
- **dev 已做便携版实测**：便携配置构建的 exe 里 `portable_data` 字符串出现 1 次，安装态 exe 0 次（证明 appDirectoriesOverride 只作用于便携版那一次构建）；便携布局下 `c-clean-cli --version` = 0.4.0，DB 落在 `portable_data/data/cdiskcleaner.db`。

### 第二档（需评审/用户拍板）
1. **当前版本不是 v0.1.7，是 v0.2.1**（任务书「现状」段写错）。本次从 0.2.1 升到 0.4.0。
2. **`src-tauri/` 实际在 `app/src-tauri/`**（任务书全文的路径都少了 `app/`）。
3. **根 `Cargo.toml` 没有 version**（纯 workspace），不需要改。
4. **白名单漏了 3 个必改文件**：根 `package.json`、`cli/Cargo.toml`、`Cargo.lock`。其中 `cli/Cargo.toml` 属于**不改就无法通过验收标准 12** 的硬遗漏。
5. **版本号实际改了 5 个文件（6 处，含 Cargo.lock）而不是任务书说的「四处」**。
6. **CHANGELOG 日期写 2026-11-07**（实际准备日），任务书示例是 11-06（派发日）。
7. **`docs/USER_GUIDE.md` 顺带修正了原文档的错误**：3.1 节原文写「超过 85% 转橙，超过 95% 转红」，而代码（`Dashboard.tsx`）是 `>=85% 红 / >=70% 橙`。既然这一节本来就要改成多盘描述，一并改对，不留已知错误。
8. **章节顺延**：USER_GUIDE 新增 12.2「自定义规则」使原 12.2~12.4 顺延为 12.3~12.5；新增第 15 章「命令行（CLI）与便携版」使原第 15 章「常见问题」顺延为 16。两处交叉引用（`#124-行为偏好`、`#15-常见问题`）已同步。
9. **`latest.json` 未改**：它当前是 `0.1.0` 占位，文件内注释明确写「首次 workflow_dispatch 触发后，release.yml 会用真实签名覆盖此文件」——属 CI 产物，手改无意义且会被覆盖。也不在白名单内。
10. **tag 未打**：按任务书「由 DocsKeeper 在 PR 合并 main 后手动打」，开发轮不做。

### 第三档（已登记为已知限制）
- **GUI dev 真机未起**：本任务无前端代码改动（纯版本号 + 文档），按 2026-11-03「有前端更改必须真机」新规归第三档。pnpm test=197 / build=250.34 kB 与 079 完全相同，回归保护通过。**关于页 / 设置页显示 v0.4.0 的窗口截图未取证**（WebView2 崩溃，从 task-074 起 dev 就注明本机 WebView2 渲染进程创建即崩）。dev 已给替代证据：安装态 exe 的版本资源 = 0.4.0、`getVersion()` 的唯一来源（tauri.conf.json）= 0.4.0、前端无硬编码版本（已 grep 确认）。
- **CDP console 未抓**：GUI 真机未起。
- **CI 五道闸未跑**：PR 由 CI 门禁观察员创建后跑。已知风险：task-074/075 的 CI 曾在「Rust 测试」步骤红而本地全绿，本分支推上去需测试轮定位（纯文档改动，理论上不应引入新红）。
- **签名**：本地未签名（exit=1 已知预期）。
- **v0.4.0 尚未真正发布**：需 PR 合并 main → DocsKeeper 打 tag → `release.yml` 出四个产物（NSIS / MSI / 便携版 zip / CLI exe）。
- **下一阶段**：UI 深色适配补完（Items / SafeBox / History，task-081）/ 规则导入导出。
- **规则库 `rules/cache_rules.json` 的 `modules` 段仍是双份真值源**（task-078 起遗留），建议 task-081 前拍板。
- **`tasks/060-shutdown-experience.md` 仍是「工作区已删除、HEAD 里存在」**（连续 18 个任务）。

## 7. 测试结论

- 结论：**通过（632 passed / 197 vitest / 版本号 6 处全部 0.4.0 / NSIS 文件名 0.4.0 / CLI --version 0.4.0）**
- feature 分支最新 commit：`7242962`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/lint/test/build）+ 版本号真机验证（CLI --version + 6 处文件确认）+ NSIS 出包，未复用开发自验结论。
- **新规执行说明**：本任务**无前端代码改动**（纯版本号 + 文档，pnpm build 与 079 完全相同 250.34 kB），按用户 2026-11-03「有前端更改必须真机测试」的指示，GUI dev 真机归第三档。**版本号本身就是本任务的核心验收**，测试轮已独立验证：①`c-clean-cli --version` = 0.4.0；②6 处版本号文件全部 0.4.0；③NSIS 文件名 = `C-DiskCleaner_0.4.0_x64-setup.exe`。dev 已做 MSI per-user 静默安装实测（FileVersion=0.4.0 / 卸载成功）+ 便携版实测（portable_data 字符串出现 1 次 vs 安装态 0 次）。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①白名单漏了 3 个必改文件（根 package.json / cli/Cargo.toml / Cargo.lock）；②根 Cargo.toml 是纯 workspace 无 version 字段；③USER_GUIDE 顺带修正了 Dashboard 颜色阈值的原文档错误；④latest.json 是 CI 产物不该手改；⑤tag 由 DocsKeeper 在 PR 合并后手动打。**
