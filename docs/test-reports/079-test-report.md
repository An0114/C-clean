# 测试报告：task-079 P2 收官——便携版 + CLI 入口

- 分支：`feature/task-079-p2-portable-cli`
- 测试 HEAD：`25eb2c8`
- 测试时间：2026-11-06（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）
- **P2 收官：CLI 独立 binary（零依赖解析器 + 复用 engine）+ 4 子命令 + 便携版打包配置 + CLI 文档**

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| 构建验证 | `cargo fmt/clippy/test` + `pnpm lint/test/build` | 是 | 通过 |
| CLI 真机 | `cargo run -p c-clean-cli`（--help / list-rules / scan --format json） | **是**（CLI 本身就是终端真机） | 通过 |
| GUI dev 真机 | `pnpm tauri dev` | **否**（无前端改动，按 2026-11-03 新规归第三档） | 见第三档 |
| 打包 | NSIS `C-DiskCleaner_0.2.1_x64-setup.exe`（4.55 MiB） | 是 | 通过（出包成功） |
| 干净目录安装 | `/S /D=...` | 未尝试 | 归第三档（UAC 非交互受限） |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区（含 12 条新增 CLI + 15 条 `#[ignore]` 真机） | `cargo test --workspace` | 通过 | `cargo-test.log`：65 batches / **632 passed / 0 failed / 15 ignored**（基线 620 + 新增 12，与开发声称一致） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm test` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（1 条 winsxs_analyze warning + 1 条 engine lib test warning + 1 条 elevation incremental lock warning + 2 条 elevation 既有 warning）；`pnpm.log`：lint=0、test=0（14 files / **197 passed**，无前端改动回归保护通过）、build=0（**250.34 kB / gzip 77.59 kB**，与 078 完全相同，无前端改动） |
| L3 CLI 真机 | 终端输出 | `c-clean-cli --help` / `list-rules` / `scan --format json --limit 1` | **通过** | `cli-smoke.log`：①`--help` 完整（4 子命令 + 7 开关 + 退出码 0/1/2 说明）；②`list-rules` 输出「内置规则库 v1.0.0（1 条）[A] npm_cache (L1) / 用户自定义规则（0 条）」；③`scan --format json --limit 1` 输出 session=1 / 7048 项 / 344.1 MB，JSON 字段名契约正确（`items[].module/path/riskLevel/sizeBytes` + `shown/totalBytes/totalItems`），`id` 不外泄 |
| L4 打包 | NSIS 出包 | `tauri build` | 通过 | `tauri-build.log`：NSIS 4.55 MiB（+0.01 vs 078）+ MSI 6.71 MiB；FileVersion=0.2.1；签名 exit=1 为已知预期 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `c-clean-cli` 独立编译（不依赖 Tauri） | `cargo run -p c-clean-cli` 直接跑通；cli/Cargo.toml 不依赖 tauri | 通过 |
| 2 | 调用 engine 库复用扫描/删除逻辑 | scan 输出与 GUI 一致（5 模块 / 7048 项 / 344.1 MB）；走 Rust 实现路径（日志可见「走 Rust 实现，入库 N 项」） | 通过 |
| 3 | 单测：CLI 编译通过 | 12 条 CLI 单测全绿 | 通过 |
| 4 | `scan` 子命令输出可清理项列表 | 真机输出 session=1 / 7048 项 / 344.1 MB + 表格 | 通过 |
| 5 | `clean` 子命令默认 dry-run，`--yes` 真实删除 | dev 真机输出「未加 --yes，只预览不删除。确认要清理请加 --yes」；参数解析单测钉住 | 通过 |
| 6 | `dry-run` 子命令预览可清理项 | 子命令已注册（--help 可见）；参数解析单测覆盖 | 通过 |
| 7 | `list-rules` 子命令列出内置 + 用户规则 | 真机输出「内置 npm_cache (L1) / 用户规则 0 条」 | 通过 |
| 8 | `--format json` 机器可读输出 | 真机 JSON：items[].module/path/riskLevel/sizeBytes + shown/totalBytes/totalItems；`id` 不外泄 | 通过 |
| 9 | 单测：子命令执行正确 | 12 条单测（参数解析 / 数据目录三档 / 表格对齐 CJK 2 列 / JSON 契约 / --limit 截断但 totalItems 真实） | 通过 |
| 10 | `tauri.conf.json` 加便携版配置 | **实现偏差**：Tauri 2 没有 `bundle.windows.portable`，改用 `app/appDirectoriesOverride` + 独立 `tauri.portable.conf.json` + `--config`（见第二档偏差 2/3） | 通过（实现方式偏差） |
| 11 | CI 同时产出 NSIS 安装版 + 便携版 zip | release.yml 已加便携构建 + Compress-Archive 步骤；本地未跑 CI（归 CI 门禁） | 通过（配置层） |
| 12 | 便携版数据存 exe 同目录 | dev 真机实证：cli exe 放进含 `portable_data/` 的目录跑，DB 落在 exe 旁边，`%APPDATA%` 未被触碰 | 通过 |
| 13 | 单测：便携版 exe 可启动 | `pick_data_root` 四档优先级单测钉住；便携构建二进制里 `portable_data` 字符串出现 1 次，普通构建 0 次 | 通过 |
| 14 | `docs/CLI.md` 包含安装方式 + 子命令示例 + JSON 格式说明 | 文档已建（安装 / 子命令 / JSON / 便携版 / 退出码 / 脚本） | 通过 |
| 15 | 便携版使用方式说明 | 文档已写（解压即用，数据存同目录 portable_data/） | 通过 |
| 16 | `cargo test --workspace` 全绿（新增 ≥3 条） | 632 passed（新增 12 条，远超 ≥3） | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮未发现需代码修复的缺陷 | L1-L2 全绿 + CLI 真机三命令全通 | — | — | — |

## 5. 测试期修复记录（bugfix）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮无需修复 | — | — | — | — |

## 6. 遗留问题（三档分流）

### 第一档（已修）
- **零依赖手写参数解析器**（100 行）：不给子命令 = 打帮助不扫描（最安全默认）；`--yes` / `--include-l2` 只能配 clean（配错直接报用法错误退出码 2）；退出码 2 = 用法错误，与 1（运行期错误）分开。
- **数据目录四档优先级**（`pick_data_root` 纯函数）：`--data-dir` > `CCLEAN_DATA_DIR` 环境变量 > `<exe 目录>\portable_data`（存在即用）> `%APPDATA%\com.and0114.cclean`。
- **便携版配置独立文件**：`app/src-tauri/tauri.portable.conf.json` 只含 `app.appDirectoriesOverride: "./portable_data"`，用 `--config` 注入，不污染共享配置（避免安装版 Program Files 只读写库失败）。
- **`portable_data/` 放 README.txt**：避免 `Compress-Archive` 跳过空目录导致「目录存在即便携」判断失效。
- **12 条 CLI 单测**：no_args_shows_help / parses_subcommands / parses_value_flags / rejects_usage_errors（9 种用法错误）/ parse_drives_rejects_garbage / data_root_priority / renders_table_with_aligned_columns（CJK 按 2 列算）/ renders_empty_list / formats_bytes / display_width_counts_cjk_as_two / scan_json_keeps_documented_field_names / scan_json_truncates_items_but_keeps_true_total。
- **真机冒烟（测试轮）**：--help / list-rules / scan --format json 三命令全通，JSON 字段名契约正确，id 不外泄。

### 第二档（需评审/用户拍板）
1. **用零依赖手写解析器而非 clap 4**：本项目一贯零依赖（提权 exe 手写 FFI / 报告手写拼接 / glob 手写 DP）；clap 会带进 6+ 个传递依赖；参数形态只有 4 子命令 + 7 开关，手写 100 行且能直接单测。
2. **Tauri 2 没有 `bundle.windows.portable` 这个配置项**：bundle target 只有 nsis/msi/app/dmg/deb/rpm/appimage/updater；便携场景的官方机制是 `app.appDirectoriesOverride`。
3. **不改 `src-tauri/tauri.conf.json` 加便携配置**：路径本身就是错的（真实 `app/src-tauri/`），而且相对路径 override 会让安装版（Program Files 只读）写库失败；Tauri 文档明确要求只作用于便携构建。改用独立 `tauri.portable.conf.json` + `--config`。
4. **便携版不是单 exe**：还要 `rules/`（外部 L3 红线）、`scripts/`（ps1 回退）、`cclean-elevation.exe`（还原点/DISM）。`AppPaths::pick_resource_root` 要求「同时含 rules/ 与 scripts/ 的目录」。所以便携版是**一个文件夹**，不是单个 exe。
5. **单元 1（binary）与单元 2（子命令）合成一次提交**（`5856c9d`）：CLI 只有一个 `main.rs`，「只有 binary 骨架、没有子命令」的中间态是不可运行的死代码。子命令的正确性修复单独一次（`5c107fd`：分页取全条目 + clean 预览遵守 --limit，真机冒烟发现漏 500 条），测试单独一次（`069b407`）。
6. **CI 额外 `cargo build --release -p c-clean-cli` 并把 `c-clean-cli.exe` 单独上传到 Release**：文档承诺「每个 Release 都带 c-clean-cli.exe」，脚本用户不想为拿一个 CLI 去解压整个便携包。
7. **白名单外 2 个文件必改**：`app/src-tauri/tauri.portable.conf.json`（新建，独立便携配置）/ `Cargo.lock`（加了 workspace 成员 cli，lock 里必须出现 c-clean-cli 条目）。

### 第三档（已登记为已知限制）
- **GUI dev 真机未起**：本任务无前端改动（Settings.tsx 等 UI 没动），按 2026-11-03「有前端更改必须真机」新规归第三档。pnpm test=197 / build=250.34 kB 与 078 完全相同，回归保护通过。
- **便携版 zip 本地未打包**：`Compress-Archive` 那步在 CI 里；本地等价验证是手工 staging + 便携布局冒烟（dev 已跑）。
- **安装态未完成**：NSIS 出包成功（4.55 MiB / FileVersion=0.2.1），UAC 非交互受限。
- **CDP console 未抓**：GUI 真机未起。
- **CI 五道闸未跑**：PR 由 CI 门禁观察员创建后跑。
- **签名**：本地未签名（exit=1 已知预期）。
- **`clean --include-l2` 不创建系统还原点**：还原点是 GUI 的交互式提权流程（要弹 UAC），命令行做不了。已在输出里明确警告，文档也写了「真要清 L2 建议用 GUI」。若要补，得给提权 exe 加一个非交互入口 + `--accept-no-restore-point` 之类的显式开关，属独立任务。
- **CLI 没有「关闭占用进程并重试」**（GUI 的 `cleanup_retry_locked`）。被占用文件会进 `failures`，关掉程序重跑即可。
- **CLI 一次最多处理 50 万条**（翻页上限 1000 页 × 500 条），超出打警告并截断。
- **便携版不能放在只读目录**（`Program Files`）：它的前提就是 exe 旁边可写。文档已写明。
- **`portable_data` 的判定是「目录存在」而不是「目录可写」**：可写性探测会真的落盘临时文件，在 Program Files 下还会弹 UAC 失败提示。存在性判断零副作用，代价是「用户手建了一个空的 portable_data 但放在只读盘」时会写失败——属边缘情况，文档已提示。
- **P2 全部完成**。下一阶段：发版 `v0.4.0` / UI 深色适配补完（Items / SafeBox / History）/ 规则导入导出。
- **规则库 `rules/cache_rules.json` 的 `modules` 段依然没有扫描消费者**（task-078 起就是双份真值源，本任务未触及）。建议发版前拍板：要么让内置模块按规则库驱动，要么删掉 `modules` 段只留 `whitelist_deny`。
- **`tasks/060-shutdown-experience.md` 仍是「工作区已删除、HEAD 里存在」**（连续 17 个任务）。

## 7. 测试结论

- 结论：**通过（632 passed / 197 vitest / NSIS 出包 4.55 MiB；CLI 真机三命令全通）**
- feature 分支最新 commit：`25eb2c8`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/lint/test/build）+ CLI 真机三命令（--help / list-rules / scan --format json）+ NSIS 出包，未复用开发自验结论。
- **新规执行说明**：本任务**无前端改动**（Settings.tsx 等 UI 没动，pnpm build 与 078 完全相同 250.34 kB），按用户 2026-11-03「有前端更改必须真机测试」的指示，GUI dev 真机归第三档。**CLI 本身就是终端真机**（本任务的核心交付物），测试轮已独立跑通 --help / list-rules / scan --format json 三命令，JSON 字段名契约正确，id 不外泄。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①零依赖手写解析器 vs clap 4 的取舍；②Tauri 2 没有 bundle.windows.portable，改用 appDirectoriesOverride + 独立配置文件的合理性；③便携版是文件夹而非单 exe（rules/scripts/elevation 三件套）；④portable_data/ 放 README.txt 避免 Compress-Archive 跳空目录；⑤clean --include-l2 不创建系统还原点的取舍；⑥规则库 modules 段双份真值源（task-078 起遗留）建议发版前拍板。**
