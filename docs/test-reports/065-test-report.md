# 测试报告：task-065 M1 C 模块——WinSxS 清理（DISM 分析+清理，走提权 exe）

- 分支：`feature/task-065-winsxs-cleanup`
- 测试 HEAD：`e099a6d`（含测试轮 fmt 修复）
- 测试时间：2026-10-31（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| dev（pnpm tauri dev） | Win11 25H2 | 是（以 NSIS 安装态代跑 L3） | 通过 |
| 打包安装（干净目录） | NSIS `C-DiskCleaner_0.1.7_x64-setup.exe`（4.36 MiB / 4574010 字节），装到 `D:\qa-065-install` | 是 | 通过 |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区 + 前端 | `cargo test --workspace` / `pnpm --filter app test` | 通过 | `cargo-test.log`：54 batches / **364 passed / 0 failed**（基线 334 + 新增 30：elevation +6 / engine::elevation +5 / elevation_whitelist 3 / winsxs_analyze 9 / winsxs_cleanup 7）；`pnpm.log`：10 files / **113 passed**（基线 104 + Dashboard.test 9） |
| L2 集成/契约 | 门禁 | `cargo fmt`（测试轮修复后=0）/ `cargo clippy -D warnings` / `pnpm lint` / `pnpm build` | 通过 | `gate.log`：clippy=0（2 条 incremental lock 非阻塞 warning）；`pnpm.log`：lint=0（2 条 SafeBox 既有 warning）、build=0（206.66 kB / gzip 66.31 kB，+5.18 kB） |
| L3 端到端/UI | 安装态：Dashboard WinSxS 卡片真机渲染 | NSIS 启动 + GDI 截图 | 通过 | `dashboard.png`：「WinSxS 组件存储」卡片 + 说明文案 + 蓝色「分析组件存储」按钮真机可见；仪表盘其他模块（4 卡 / 环形图 / 趋势图）无回归 |
| L4 真机/打包 | NSIS 干净目录安装 → 启动 → 卸载 | `/S /D=D:\qa-065-install`（首次 UAC 自动取消，改 `-Verb RunAs` 后通过） | 通过 | `l4.txt`：installed=True、elev_exe=True、elev_size=297472 字节（290 KB，与开发声称 297 KB 一致）、pid=2568、cleaned=True |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | 提权 exe 加命令 `dism_analyze` | elevation 白名单实现；`dism_action_args_never_contain_resetbase` 等单测；真机安装态 `cclean-elevation.exe` 297472 字节（较 062 的 153 KB +144 KB DISM 模块增量） | 通过（单测 + 真机包内体积） |
| 2 | 后端解析 DISM 输出（任务书写 XML，实为文本） | `/analyzecomponentstore` 无 XML 输出，改为 `/English` + 「标签:值」逐行解析；`winsxs_analyze.rs` 9 条单测覆盖字段映射（Actual Size / Backups / Cache / Reclaimable / Recommended=Yes→bool） | 通过（单测，开发已登记偏差） |
| 3 | 返回 `{reusable_packets, backports, recommended_cleanup}` | 字段名按 DISM 真实标签更正为 `reclaimable_packages` / `backups_and_disabled_bytes`；同单测覆盖 | 通过（单测，字段名偏差已登记） |
| 4 | 单测：DISM 输出解析正确 | 9 条 winsxs_analyze 单测通过 | 通过 |
| 5 | 提权 exe 加命令 `dism_cleanup` | elevation 白名单实现；同 #1 | 通过 |
| 6 | 硬编码禁 `/ResetBase` | 实现升级为「无传参入口」：`Action::args()` 编译期常量表，整条链无 `&str` 能流进 DISM；`dism_action_args_never_contain_resetbase` / `extra_argv_cannot_inject_dism_flags` / `resetbase_is_never_a_command` 3 条单测 | 通过（单测，实现强于任务书要求） |
| 7 | 清理前自动创建系统还原点 | `winsxs::cleanup` 顺序：`skip_restore_point==false` → 先 `ensure_restore_point` 再 DISM；`restore_point_runs_before_dism` / `skip_restore_point_goes_straight_to_dism` 用「提权 exe 不存在」可控失败点反推顺序 | 通过（单测） |
| 8 | 实时转发 DISM 输出进度 | `run_elevated_streaming` + `tail_progress` + `WINSXS_PROGRESS_EVENT`；winsxs_cleanup 7 条单测覆盖 lineCount/outputTail/restorePointSeq | 通过（单测） |
| 9 | 单测：白名单拒绝 `/ResetBase` | 同 #6 | 通过 |
| 10 | 前端显示分析结果（可清理空间 / 建议清理） | Dashboard WinsxsCard 组件；`dashboard.png` 真机显示卡片（未点分析，故未显示数值）；Dashboard.test 9 条覆盖渲染 | 通过（真机卡片存在 + 单测） |
| 11 | 「立即清理」按钮弹 confirm | Dashboard.test 覆盖交互 | 通过（单测） |
| 12 | 清理中显示进度 | 同 #8 流式转发；Dashboard.test 覆盖 | 通过（单测） |
| 13 | 单测：UI 渲染正确 | Dashboard.test.tsx 9 条通过 | 通过 |
| 14 | 白名单加 `dism_analyze` / `dism_cleanup` | 同 #1/#5 | 通过 |
| 15 | 拒绝 `/ResetBase` 参数 | 同 #6 | 通过 |
| 16 | 单测：白名单拒绝未知命令 + `/ResetBase` | elevation_whitelist.rs 3 条（两侧白名单一致性，`include_str!` 读提权 exe 源码）+ elevation 6 条 | 通过 |
| 17 | `cargo test --workspace` 全绿（334+≥4） | 364 passed / 0 failed | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| 1 | `cargo fmt --all -- --check` 在 `engine/tests/winsxs_cleanup.rs` 报两处长行换行 diff | 测试轮跑 `cargo fmt --all -- --check` | 退出码 0 | 低 | 已修（测试轮 `cargo fmt --all` 自动格式化，commit `e099a6d`） |

## 5. 测试期修复记录（bugfix 子分支）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| 1 | winsxs_cleanup.rs 两处长行未按 rustfmt 换行 | 直接在 feature（格式修复，非逻辑缺陷，未开 bugfix 子分支） | `e099a6d` | `style(test): cargo fmt winsxs_cleanup.rs 长行换行 (#task-065)` | fmt_check=0；cargo test 364 passed 保持 |

## 6. 遗留问题（三档分流）

### 第一档（已修）
- `winsxs_cleanup.rs` rustfmt 格式 diff（见 §4/§5，已修并提交）。

### 第二档（需评审/用户拍板）
1. **任务书 5 处技术偏差**（开发已在实现备注 §8 记录）：①「解析 DISM 输出 XML」不成立（`/analyzecomponentstore` 无 XML，改为 `/English` 文本解析）；②字段名误译（`reusable_packets`→`reclaimable_packages`，`backports_and_drivers`→`backups_and_disabled_bytes`）；③`engine/src/commands.rs` 不存在（实际在 `app/src-tauri/src/commands.rs`）；④白名单漏 6 个文件（scan/mod.rs、elevation/mod.rs、app commands.rs、lib.rs、Dashboard.test.tsx）；⑤「拒绝 `/ResetBase`」升级为「无传参入口」（语义更强）。
2. **DISM 真实提权链路（UAC → 还原点 → `/StartComponentCleanup` → 进度滚动）未真机点「分析/清理」按钮**：需 UAC 交互（本测试轮 UAC 连续被自动取消，改 `-Verb RunAs` 仅完成安装器提权，未在 App 内点「分析组件存储」按钮触发 DISM）。开发已用非提权态探测证明白名单放行/DISM 可达/`/English` 生效/退出码 740 正确映射，真实 UAC 链路由单测覆盖顺序。
3. **`/StartComponentCleanup` 15 分钟超时 kill**：机械盘 + 超大组件存储可能超时，当前实现到点 kill 报「DISM 超时」；若真机遇到需把上限做成可配置。

### 第三档（已登记为已知限制）
- **点「分析组件存储」按钮真机显示可清理空间数值**：未点击触发（UAC 交互限制），由 9 条 winsxs_analyze 单测 + Dashboard.test 9 条覆盖；真机仅证实卡片与按钮存在。
- **点「清理」按钮 → UAC 弹窗 → 还原点创建 → DISM 进度 → 完成**：未真机点击（同上），由 7 条 winsxs_cleanup 单测钉住还原点先于 DISM 的顺序。
- **还原点真机创建**：未真机验证（需 UAC 提权跑 DISM）。
- **updater 签名**：build 末尾报缺 `TAURI_SIGNING_PRIVATE_KEY`，NSIS/MSI 正常产出（与既往一致）。
- **CDP console 红错**：tauri dev 不开 9222，以运行日志无 panic + 无红弹窗替代（G6 已知豁免）。

## 7. 测试结论

- 结论：**通过（L1/L2/L4 自动化闸门 + Dashboard WinSxS 卡片真机渲染 + 提权 exe 包内体积 全部通过；DISM 真实提权链路（UAC→还原点→DISM 进度）真机未点击触发，由 30 条新增单测（含还原点先于 DISM 顺序钉死）覆盖，见 §6 第三档）**
- feature 分支最新 commit：`e099a6d`（含测试轮 fmt 修复）
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/lint/build）+ NSIS 干净目录安装 + WinSxS 卡片真机截图，未复用开发自验结论。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①任务书 5 处技术偏差（尤其 DISM 文本解析替代 XML、字段名更正、无传参入口替代拒绝 ResetBase）；②DISM 真实 UAC 提权链路真机证据缺失是否可接受单测覆盖；③测试轮 fmt 修复 commit `e099a6d` 是否认可（格式修复非逻辑变更）。M1 C 模块落地，下一步 E 模块大文件地图（task-066）。**
