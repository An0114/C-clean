# 测试报告：task-076 R5 架构债收官——scan_dev_cache + scan_windows_update → Rust 重写

- 分支：`feature/task-076-r5-devcache-windowsupdate-rust`
- 测试 HEAD：`789a2bc`
- 测试时间：2026-11-03（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）
- **R5 五连迁收官：dev_cache + windows_update Rust 模块，5 个模块全部 Rust 优先，ps1 仅作回退保留**

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| 构建验证 | `cargo check/clippy/test` + `pnpm build`（无前端改动） | 是 | 通过 |
| 打包 | NSIS `C-DiskCleaner_0.2.1_x64-setup.exe`（4.50 MiB） | 是 | 通过（出包成功） |
| 干净目录安装 | `/S /D=...`（`-Verb RunAs`） | 未尝试 | **归第三档**（UAC 非交互受限；本任务无前端改动，按用户 2026-11-03 新规仍可归第三档） |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区（含 23 条新增 + 14 条 `#[ignore]` 真机） | `cargo test --workspace` | 通过 | `cargo-test.log`：63 batches / **553 passed / 0 failed / 14 ignored**（基线 530 + 新增 23，与开发声称一致） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（4 条既有 warning：db_schema 1 + lib test 1 + elevation 2）；`pnpm.log`：build=0（243.01 kB / gzip 75.55 kB，与 073/074/075 完全一致，无前端改动） |
| L3 真机 | 扫描结果含 dev_cache/windows_update 项 / 5 模块 Rust 优先 / 环境变量回退 | 安装态截图 + 环境变量切换 | **未完成**（归第三档） | 由 23 条单测 + 1 条 `#[ignore]` 真机冒烟（dev 已跑：dev_cache 21 项 / 0.1 MB（pip_cache），windows_update 9 项 / 29.6 MB 全 L2）覆盖；硬证据 `orchestrator_runs_five_rust_modules_without_any_ps1`（空 scripts_dir 下 5 模块全成功 + 总开关关后 AllScriptsFailed(5)） |
| L4 打包 | NSIS 出包 | `tauri build` | 通过 | `tauri-build.log`：NSIS 4.50 MiB + MSI 6.65 MiB；FileVersion=0.2.1；签名 exit=1 为已知预期 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `engine/src/scan/modules/dev_cache.rs` 实现开发缓存扫描（npm/pip/Gradle/Maven） | 10 条内联单测全绿（四根递归收集 / 按根打 category / module+risk / 缺根返空 / 根与 ps1 路径一致 / **永不扫 node_modules 或 gradle** / 空 bases 返空 / 真机不 panic / dir_from 拒空 / env_dir 缺失返 None） | 通过（但见第二档偏差 1：实际只扫 npm×2/pip/pnpm，不加 Gradle/Maven/node_modules） |
| 2 | 正确处理环境变量和用户目录 | `env_dir_missing_variable_is_none` + `scan_roots_match_ps1_paths` 单测钉住 | 通过 |
| 3 | 返回 `Vec<CleanupItem>`，格式与 ps1 输出一致 | 与前几轮同一 `ScanItem` 结构 + 同一 `insert_batch`；6 条集成测试含 2 条真实 Rust vs ps1 对比 | 通过（语义等价；命名用 ScanItem 非 CleanupItem，见第二档偏差 3） |
| 4 | 单测：扫描开发缓存目录返回正确结果 | 10 条内联 + 4 条集成（含空 bases 一致 / 永不扫 gradle） | 通过 |
| 5 | `engine/src/scan/modules/windows_update.rs` 实现 Windows Update 缓存扫描 | 5 条内联单测全绿（递归收集 / module+category+L2 risk / 缺目录返空 / **根基于 SystemRoot 而非硬编码 C:\Windows** / 真机不 panic） | 通过 |
| 6 | 返回 `Vec<CleanupItem>`，格式与 ps1 输出一致 | 同上 | 通过 |
| 7 | 单测：扫描 SoftwareDistribution\Download 返回正确结果 | 5 条内联 + 2 条集成（真实机器 9 项 / 29.6 MB + 伪造树打 L2） | 通过 |
| 8 | `orchestrator.rs` dev_cache 模块改为直接调 Rust | `rust_module_items` match 分支 + `orchestrator_runs_five_rust_modules_without_any_ps1` 空 scripts_dir 硬证据 | 通过 |
| 9 | `orchestrator.rs` windows_update 模块改为直接调 Rust | 同上 | 通过 |
| 10 | 环境变量可回退 ps1 | 5 个模块开关名抽常量 `ENV_*` + `every_script_has_an_own_switch` 单测钉住三件配套 | 通过 |
| 11 | 全部 5 个模块默认 Rust 优先 | `all_five_module_switches_default_on` 单测（原 all_three 升级） | 通过 |
| 12 | 单测：Rust 优先 / ps1 回退两种模式 | 6 条集成含 2 条真实对比 + 总开关关后 AllScriptsFailed(5) | 通过 |
| 13 | `cargo test --workspace` 全绿（新增 ≥4 条） | 553 passed（新增 23 条，远超 ≥4） | 通过 |
| 14 | Rust 实现与 ps1 输出抽样对比（格式一致） | 2 条真实对比单测（dev_cache 4 条逐条相等 + windows_update 真实机器 9 项） | 通过 |
| 15 | R5 架构债全部 5 个 ps1 Rust 重写完成 | temp(074) / prefetch+browser_cache(075) / dev_cache+windows_update(076)；硬证据空 scripts_dir 下 5 模块全成功 | 通过 |

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
- dev_cache.ps1 → Rust 等价重写（`%LOCALAPPDATA%\npm-cache` + `%APPDATA%\npm-cache` + `%LOCALAPPDATA%\pip\Cache` + `%LOCALAPPDATA%\pnpm-store`，四根顺序与 ps1 完全一致；符号链接走 `is_reparse_point` 跳过）。
- windows_update.ps1 → Rust 重写（读 `%SystemRoot%\SoftwareDistribution\Download` 而非硬编码 `C:\Windows`，SystemRoot 缺失才兜底 `C:\Windows`；类别 `update_download`，**L2**）。
- Orchestrator 接线：5 个模块全部 Rust 优先，ps1 仅作回退；5 个环境变量名抽成 `ENV_*` 常量（原来 4 个是字符串字面量，漏写一个表现为「永远回退不了且无报错」）。
- R5 收官硬证据：空 scripts_dir（一个 ps1 都没有）下 `run_all` 仍 5 模块全成功、`scripts_succeeded.len()==5`；总开关 `CCLEAN_RUST_ALL=0` 同目录必须 `AllScriptsFailed(5)`。
- 负向契约：`scan_roots_never_include_node_modules_or_gradle` 把「不扫 node_modules / Gradle / Maven」钉成测试。

### 第二档（需评审/用户拍板）
1. **dev_cache 实际只扫 npm×2/pip/pnpm，不加 Gradle/Maven/node_modules**（任务书字面要求扫 Gradle/Maven）。dev 给的理由：①验收 14 条要求「Rust 与 ps1 输出对比」，多扫三目录两边必然不等，把差异藏进测试白名单比不加更糟；②Gradle/Maven 删错代价大（下次构建重下几百 MB），需独立任务+独立开关；③node_modules 是 L3 用户数据红线，删了项目当场跑不起来。**需产品拍板：是否扩 Gradle/Maven（要加需先定子目录+独立开关）。**
2. **`CleanupItem` 命名不存在**：任务书写 `CleanupItem { path, size_bytes, module, risk_level }`，实际用项目既有 `ScanItem { module_code, category, path, size_bytes, risk_level }`（它是 cleanup_item 表的实际字段）。
3. **「按文件大小/修改时间筛选」未实现**（同 074/075 判断，ps1 无筛选，加阈值两边不等且无依据）。
4. **`modules/mod.rs` 拆两次提交**：任务书提交计划只列单元 1 含 mod.rs，但 windows_update 注册也在同文件。为每个 commit 单独可编译，mod.rs 拆单元 1（dev_cache）+ 单元 2（windows_update）。
5. **ps1 脚本不删**：任务书明确「保留作为回退」。dev 建议先发一版收集反馈再删——回退开关是唯一兜底，脚本一删就没了。
6. **三个待决策项**：L0 是否进清理列表；Firefox 缓存是否保留（075 让条目数涨 653 倍）；dev_cache 是否扩 Gradle/Maven。

### 第三档（已登记为已知限制）
- **真机 GUI / 安装态冒烟未做**（5 条，本任务无前端改动，按用户 2026-11-03 新规归第三档）：①扫描结果含 dev_cache 模块项；②扫描结果含 windows_update 模块项；③5 模块 Rust 优先（进程列表无 ps1 子进程）；④环境变量回退 ps1 结果一致；⑤CDP console 0 红错（G6 已知项除外）。由 23 条单测 + dev 真机冒烟（dev_cache 21 项/pip、windows_update 9 项/29.6 MB/L2）+ 空 scripts_dir 硬证据覆盖。**特别提示用户：dev_cache 显示 pip_cache 21 项是真实的（本机 npm/pnpm 为 0 是没装/已清过）；windows_update 9 项/29.6 MB 是本机实测可读（普通权限下通常为 0，ACL 限制）。**
- **安装态未完成**：NSIS 出包成功（4.50 MiB / FileVersion=0.2.1，与 074/075 完全一致），UAC 非交互受限。
- **CDP console 未抓**：dev 注明本机 WebView2 渲染进程创建即崩（Crashpad 持续新增 .dmp、CDP 9222 不通），项目长期已知环境限制。
- **CI 五道闸未跑**：PR 由 CI 门禁观察员创建后跑。
- **签名**：本地未签名（exit=1 已知预期）。
- **下一阶段**：P2 项（多盘扫描 / 自定义规则 / 便携版 CLI）或发版 v0.4.0。
- **Items/SafeBox/History 三页深色适配仍缺**（task-061 起的旧账）。
- **`tasks/060-shutdown-experience.md` 仍是「工作区已删除、HEAD 里存在」**（连续 14 个任务）。

## 7. 测试结论

- 结论：**通过（553 passed / 0 failed / NSIS 出包 4.50 MiB）**
- feature 分支最新 commit：`789a2bc`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/build）+ NSIS 出包，未复用开发自验结论。
- 新规执行说明：本任务**无前端改动**（纯后端 ps1→Rust），按用户 2026-11-03「有前端更改必须真机测试」的指示，L3 真机仍归第三档；后续任务若涉及前端改动，L3 不可跳过。
- **R5 架构债收官**：5 个 ps1 模块（temp / prefetch / browser_cache / dev_cache / windows_update）全部 Rust 优先，硬证据是空 scripts_dir 下 5 模块全成功；ps1 脚本保留作回退。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①dev_cache 不加 Gradle/Maven/node_modules 的理由（负向契约已钉死，将来要加必须先改这条测试）；②windows_update 读 %SystemRoot% 而非硬编码 C:\Windows 的修正；③L2 风险评级（更新安装中删 SoftwareDistribution 会中断安装）；④ps1 脚本保留不删的决策（回退兜底）；⑤三个待决策项（L0 / Firefox / Gradle）。**
