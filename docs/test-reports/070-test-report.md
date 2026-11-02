# 测试报告：task-070 v0.2.0 真机验收 3 个 Bug 修复（WinSxS 退出码 3 + 扫描只出 L1 + 残留入箱永久删除失败）

- 分支：`feature/task-070-v0.2.1-bugfixes`
- 测试 HEAD：见 `target/qa-070/head.txt`
- 测试时间：2026-11-02（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）
- **v0.2.1 patch 发版（3 个 Bug 同批修复）**

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| 构建验证 | `cargo check/clippy/test` + `pnpm lint/test/build` | 是 | 通过 |
| 打包安装（干净目录） | NSIS `C-DiskCleaner_0.2.0_x64-setup.exe`（4.45 MiB / 4664761 字节），装到 `D:\qa-070-install` | 是 | 通过（启动无回归） |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区（含 13 条新增）+ 前端回归 | `cargo test --workspace` / `pnpm --filter app test` | 通过 | `cargo-test.log`：58 batches / **444 passed / 0 failed**（基线 431 + 新增 13，与开发声称一致）；`pnpm.log`：13 files / **148 passed**（前端未改动，与基线一致） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（1 条 incremental lock「拒绝访问 os error 5」warning + 2 条 elevation 既有 warning）；`pnpm.log`：lint=0（2 条 SafeBox 既有 warning）、build=0（230.32 kB / gzip 71.78 kB） |
| L3 真机 | 安装态仪表盘启动 + 8 tab 无回归 | 启动 `cdisk-cleaner.exe`、GDI 最大化截图 | 通过 | `l4.txt`：installed=True、FileVersion=0.2.0、pid=3404；`dashboard.png`：8 个 tab（仪表盘/清理/大文件/残留/注册表/保险箱/历史/设置）全在，WinSxS 卡片/4 卡/环形图无回归 |
| L4 打包 | NSIS 干净目录安装/卸载 | `/S /D=D:\qa-070-install`（`-Verb RunAs`） | 通过 | `l4.txt`：cleaned=True；NSIS 4.45 MiB + MSI 6.56 MiB；签名 exit=1 为已知预期（TAURI_SIGNING_PRIVATE_KEY 未设） |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | WinSxS「分析」按钮正常返回结果（不再退出码 3） | 未真机点（UAC 自动化受限）；**单测覆盖**：Bug-1 四条新用例（`argv_starts_with_command_name` 等）+ 开发取证「两种 argv 形状对比」（`--result x.json`→exit 3 vs `dism_analyze --result x.json`→exit 4+DISM 740）证明 argv 拼装已修 | 单测通过，真机点验归第三档 |
| 2 | 分析结果显示可清理空间 | 同上（DISM 真跑需 UAC） | 同上 |
| 3 | 单测：DISM 命令参数正确 | `argv_starts_with_command_name` / `argv_without_args_is_just_the_command` / `argv_first_token_is_always_whitelisted` / `flag_as_first_token_is_rejected_not_treated_as_command` 全绿 | 通过 |
| 4 | 扫描结果出现 L0/L1/L2 三级 | **只做到两级（L1+L2）**：开发真跑 `scan_temp.ps1` 实测 risk=1:61 + risk=2:29；L0 语义是「只读/仅提示不参与清理」，5 个可清理模块里无天然 L0 候选，硬塞会让可清理模块变不可清理，属产品决策 | 部分通过（L1+L2 两级），L0 待产品决策 |
| 5 | L2 项正确显示风险徽标 | 单测 `system_directories_are_l2`（windows_temp/update_download=L2）绿；ps1 已改（windows_temp / update_download=L2） | 单测通过，真机徽标归第三档 |
| 6 | 单测：ps1 输出风险等级正确 | `no_script_hardcodes_risk_level` / `system_directories_are_l2` / `user_owned_caches_are_l1` / `detector_catches_the_old_hardcoded_shape`（含检测器自检）全绿 | 通过 |
| 7 | 残留项移入保险箱后「永久删除」正常完成 | 未真机点；**单测覆盖**：`purge_item_removes_directory_entries` / `purge_expired_removes_directory_entries` / `purge_item_is_idempotent_when_directory_already_gone` 绿（`remove_entry()` 用 `symlink_metadata` 判类型，目录 `remove_dir_all`、文件 `remove_file`） | 单测通过，真机点验归第三档 |
| 8 | 残留入箱项来源列正确显示来源 | 未真机点；**单测覆盖**：`residual_move_records_source_task_id` / `residual_empty_batch_creates_no_task_row` 绿（移入前建 `cleanup_task` 行，id 传给 deleter） | 单测通过，真机点验归第三档 |
| 9 | 单测：目录永久删除 / source_task_id 写入正确 | 见 #7/#8 | 通过 |
| 10 | `cargo test --workspace` 全绿（新增 ≥3 条） | 444 passed（新增 13 条，远超 ≥3） | 通过 |
| 11 | `pnpm test` 全绿 | 148 passed（13 files） | 通过 |
| 12 | 真机冒烟：3 个 Bug 现象消失 | 安装态启动无回归，但三个 Bug 的最终点验（WinSxS 分析 / 扫描徽标 / 残留永久删除）未在测试轮自动点（UAC + DPI 坐标偏移） | 部分通过，归第三档 |

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
- 三个 Bug 代码修复均有单测覆盖：Bug-1 argv 拼装（`build_argv`）、Bug-2 ps1 风险等级（5 个脚本）、Bug-3 `remove_entry()` 目录删除 + `source_task_id` 写入。

### 第二档（需评审/用户拍板）
1. **白名单 2 处偏差**：①Bug-3 任务书指向 `engine/src/ipc/commands/residual.rs`，实际改在 `engine/src/scan/residual.rs`（ipc 层只是转调）；②`engine/tests/safe_box_test.rs` 不存在，目录删除用例加进已有的 `engine/tests/purge_expired.rs`，来源写入用例加进 `engine/tests/residual_test.rs`。合理但需评审确认。
2. **`rules/cache_rules.json` 未改**：开发取证该文件 `modules` 段在扫描链路上无消费者（`loader.rs` 只用于 `whitelist_deny` 与格式校验，`grade/` 从未实现），风险等级唯一来源是 ps1。改它还要连带改 `rules_meta.json` 的 SHA-256，patch 版不引入这个风险。若要「风险等级单一真值源」是 P1 重构，非本 bugfix 范围。
3. **验收第 4 条「L0/L1/L2 三级」只做到两级**：L0 语义是「只读/仅提示不参与清理」，5 个可清理模块无天然 L0 候选，硬塞会让可清理模块变不可清理，属产品决策。
4. **Bug-1 根因与任务书猜的三方向都不同**：不是提权通道/DISM 参数/本机环境，而是 `run_elevated` 的 `build_args_string` 只拼 `--result "path"`、`cmd` 从未进 argv；提权 exe 收 `--result` 当首 token → `EXIT_UNKNOWN_CMD(3)`。开发三条独立证据（argv 对比 / dism.log 无痕迹 / 退出码 3=常量值）。
5. **两个 Bug 互相掩盖**：`create_restore_point` 走同一条坏路径，但还原点只在清理含 L2 项时才建，而 Bug-2（所有项都是 L1）让该路径从没被走到——WinSxS 是第一个真正需要提权通道的功能，所以最先暴露。

### 第三档（已登记为已知限制）
- **3 个 Bug 的真机最终点验未在测试轮自动完成**：①WinSxS「分析」按钮（需 UAC）；②扫描结果 L1/L2 徽标；③残留入箱永久删除 + 来源列 `#N`。UAC 无法自动化 + DPI tab 坐标偏移老问题，由 13 条新单测 + 开发取证（argv 对比 / scan_temp.ps1 risk 分布 / 用户真实 DB 5 行 safe_box 取证）覆盖。**建议用户装 v0.2.1 后手动点一遍这三处确认。**
- **CI 五道闸未跑**：开发轮不建 PR，PR 由 CI 门禁观察员创建后跑。
- **签名**：本地未签名（`A public key has been found, but no private key`），exit=1 不算失败。
- **`tasks/060-shutdown-experience.md` 「工作区已删除、HEAD 里存在」连续 8 个任务未决**（流程噪音）。
- **下一步 = v0.2.1 patch 发版准备**（版本号 0.2.0 → 0.2.1 + CHANGELOG 删三条已知问题）；原 P1-7 清理方案模板延后到 task-071。

## 7. 测试结论

- 结论：**通过（444 passed / 0 failed / 148 pnpm / NSIS 出包 4.45 MiB / 安装态启动无回归）**
- feature 分支最新 commit：见 `target/qa-070/head.txt`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/lint/build）+ NSIS 干净目录安装 + 启动截图，未复用开发自验结论。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①白名单 2 处偏差（ipc/commands/residual.rs 未改、safe_box_test.rs 不存在）；②rules/cache_rules.json 未改是否认可；③验收第 4 条 L0 只做到两级是否接受（需产品决策）；④Bug-1 根因（argv 漏传 cmd）与任务书猜的三方向不符。3 个 Bug 均有单测覆盖，建议 v0.2.1 发版后用户手动点验 WinSxS 分析 / 扫描徽标 / 残留永久删除三处真机行为。**
