# 测试报告：task-074 架构债 R5 起步——scan_temp.ps1 → Rust 重写（临时文件扫描）

- 分支：`feature/task-074-r5-temp-rust-rewrite`
- 测试 HEAD：`838b6a6`
- 测试时间：2026-11-03（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）
- **R5 架构债第一个 ps1 重写闭环：scan_temp Rust 模块 + Orchestrator Rust 优先/ps1 回退（`CCLEAN_RUST_TEMP` 环境变量）+ 18 条新单测，作为后续 4 个 ps1 重写的模板**

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| 构建验证 | `cargo check/clippy/test` + `pnpm build`（无前端改动） | 是 | 通过 |
| 打包 | NSIS `C-DiskCleaner_0.2.1_x64-setup.exe`（4.50 MiB / 4718672 字节） | 是 | 通过（出包成功） |
| 干净目录安装 | `/S /D=...`（`-Verb RunAs`） | 未尝试 | **归第三档**（UAC 非交互受限） |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区（含 18 条新增 + 1 条 `#[ignore]` 真机） | `cargo test --workspace` | 通过 | `cargo-test.log`：61 batches / **509 passed / 0 failed**（基线 491 + 新增 18，与开发声称一致） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（4 条既有 warning：execute_locking_integration / lib test / elevation×2）；`pnpm.log`：build=0（243.01 kB / gzip 75.55 kB，与 073 完全一致，无前端改动） |
| L3 真机 | 扫描结果含 temp 项 / `CCLEAN_RUST_TEMP=0` 回退 ps1 | 安装态截图 + 环境变量切换 | **未完成**（归第三档） | 由 18 条单测 + 1 条 `#[ignore]` 真机冒烟（dev 已跑：46 项 / 7.4 MB / 0.05s vs ps1 1.03s）覆盖 |
| L4 打包 | NSIS 出包 | `tauri build` | 通过 | `tauri-build.log`：NSIS 4.50 MiB + MSI 6.65 MiB；FileVersion=0.2.1；签名 exit=1 为已知预期 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `engine/src/scan/modules/temp.rs` 实现临时文件扫描 | 7 条内联单测全绿（递归收集 4 文件 / windows_temp L2 / 目录不存在返空 / 两次扫描一致按路径升序 / 跳过符号链接 / scan_roots 含两个目标且 windows_temp 基于 SystemRoot / user_temp L1） | 通过 |
| 2 | 返回 `Vec<ScanItem>`，格式与 ps1 输出一致 | 3 条入库等价单测（字段形状 / 进度条数单调 / 空批次）+ 2 条 Rust vs ps1 真实对比 | 通过（语义等价，跳过 NDJSON 进程间层） |
| 3 | 正确处理 `%TEMP%` 环境变量 | `scan_roots` 单测断言 user_temp 基于 `%TEMP%`、windows_temp 基于 `%SystemRoot%\Temp` | 通过 |
| 4 | 单测：扫描临时目录返回正确结果 | 7 条内联 + 8 条集成全绿 | 通过 |
| 5 | `orchestrator.rs` temp 模块改为直接调 Rust | 4 条内联单测（缺省=开启 / 0/false/off/no 全识别为关闭 / 空串或其他=开启 / TEMP_SCRIPT 必须在 SCRIPTS 里） | 通过 |
| 6 | 其他 4 个模块保持 ps1 方式 | 代码审查：Rust 分支放在脚本存在检查之前，其余 4 个仍走 `runner.run_script()` | 通过（代码审查 + 既有 ps1 用例回归） |
| 7 | 环境变量 `CCLEAN_RUST_TEMP=0` 可回退 ps1 | 4 条纯函数 `rust_temp_enabled_from` 单测钉住 0/false/off/no（含大小写与前后空格）= ps1 | 通过 |
| 8 | 单测：Rust 优先 / ps1 回退两种模式 | 8 条集成测试含 2 条 Rust vs ps1 真实对比 + 2 条边界（不出现 L3 / module_code 恒为 temp） | 通过 |
| 9 | `cargo test --workspace` 全绿（新增 ≥3 条） | 509 passed（新增 18 条，远超 ≥3） | 通过 |
| 10 | Rust 实现与 ps1 输出抽样对比（格式一致） | 2 条真实对比单测 + dev 真机冒烟（46 项 / 7.4 MB） | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮未发现需代码修复的缺陷 | L1-L2 全绿 | — | — | — |

## 5. 测试期修复记录（bugfix）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮无需修复 | — | — | — | — |

## 6. 遗留问题（三档分流）

### 第一档（已修）
- scan_temp.ps1 → Rust 等价重写（`engine/src/scan/modules/temp.rs` + `mod.rs` 共用 `ScanItem`/`is_reparse_point()`）。
- Orchestrator 接线：temp 走 Rust（默认），`CCLEAN_RUST_TEMP=0/false/off/no` 回退 ps1；Rust 分支放在脚本存在检查之前（避免将来删 ps1 时判 missing script）；入库复用 `ScanRunner::insert_batch`（新增）避免 INSERT 漂移。
- 关键修正：junction 在 Rust `is_symlink()` 返回 false，改用 `FILE_ATTRIBUTE_REPARSE_POINT` 位检查；windows_temp 读 `%SystemRoot%\Temp` 而非硬编码 `C:\Windows\Temp`。
- 性能：dev 真机冒烟 46 项耗时 0.05s vs ps1 1.03s（外部脚本解释器启动开销消除）。

### 第二档（需评审/用户拍板）
1. **白名单 4 处偏差**（dev 已在实现备注 §1 列出）：①`engine/src/scan/mod.rs` 必须改（注册 `pub mod modules;`，不改则新模块不参与编译）；②`engine/src/scan/runner.rs` 必须改（新增 `insert_batch` 复用入库逻辑）；③`engine/tests/scan_orchestrator.rs` 回归修正（造假 .ps1 测 ps1 编排路径，temp 走 Rust 后 `total_items` 从 3 变 63）；④`engine/tests/scan_progress_percent.rs` 回归修正（`all_failed_does_not_emit_100` 期望 5 全失败，temp 走 Rust 成功后会 emit 100）。
2. **「按文件大小/修改时间筛选」未实现**：现有 ps1 本就无筛选，dev 刻意不加（避免两种模式结果不一致 + 阈值无依据）。要加应是独立产品决策。
3. **「NDJSON 但直接返回 Vec」自相矛盾**：dev 做成语义等价（同字段同入库结果），跳过 NDJSON 进程间层——这正是重写收益。
4. **`scan/modules/mod.rs` 超出「模块注册」**：还放了 `ScanItem` 与 `is_reparse_point()`（所有 Rust 扫描模块共用），没单独开 `types.rs`。
5. **环境变量而非 prefs**：`CCLEAN_RUST_TEMP` 是排障用临时开关，非用户偏好，不进 UI/prefs。
6. **风险等级仍「脚本/代码各写一份」**：`rules/cache_rules.json` 的 `modules` 段至今无消费者，全部迁 Rust 后可删——R5 额外收益，非本任务范围。

### 第三档（已登记为已知限制）
- **真机 GUI / 安装态冒烟未做**（6 条）：①扫描结果含 temp 模块项（Rust 实现）；②`CCLEAN_RUST_TEMP=0` 回退 ps1 结果一致；③CDP console 0 红错（G6 已知项除外）；④Items 页显示 temp 模块项；⑤NSIS 安装态复跑上述 2 条；⑥环境变量切换后 Items 页条目数一致。由 18 条单测 + 1 条 `#[ignore]` 真机冒烟（dev 已跑：46 项 / 7.4 MB / user_temp 35 项 L1 / windows_temp 11 项 L2 / vmware-SYSTEM 权限不足正确跳过）覆盖。**建议用户装 0.2.1 后手动验 ①②。**
- **安装态未完成**：NSIS 出包成功（4.50 MiB / FileVersion=0.2.1），但 UAC 非交互受限，不再重复尝试。
- **CI 五道闸未跑**：开发轮不建 PR，PR 由 CI 门禁观察员创建后跑。
- **签名**：本地未签名（exit=1 已知预期）。
- **其余 4 个 ps1 未迁移**：建议顺序 windows_update（最简单照抄模板）→ prefetch（权限问题）→ browser_cache（多 profile）→ dev_cache（npm/pnpm/pip 各不同），留 task-075+。
- **`scripts/` 依赖还在**：4 个迁完后才能删 `paths::scripts_dir` / `bundle.resources` 的 scripts/ 映射 / `ScanRunner::run_script`——那才是 R5 真正收尾。
- **`tasks/060-shutdown-experience.md` 仍是「工作区已删除、HEAD 里存在」**（连续 12 个任务）。
- **Items/SafeBox/History 三页深色适配待补**（沿用既有遗留）。

## 7. 测试结论

- 结论：**通过（509 passed / 0 failed / NSIS 出包 4.50 MiB）**
- feature 分支最新 commit：`838b6a6`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/build）+ NSIS 出包，未复用开发自验结论。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①白名单 4 处偏差（scan/mod.rs、runner.rs、两个既有测试文件回归修正）；②「按大小/时间筛选」刻意未加是否接受；③NDJSON 层跳过（语义等价 vs 字节等价）是否认可；④环境变量 `CCLEAN_RUST_TEMP` 作为排障开关不进 prefs 是否合理；⑤junction reparse point 修正与 `%SystemRoot%\Temp` 修正是否需在发版说明注明。6 条真机冒烟归第三档，建议用户装 0.2.1 后手动验「Rust 模式 vs ps1 模式结果一致」。R5 模板已立，后续 4 个 ps1 照此推进。**
