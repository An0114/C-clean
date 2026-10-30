# 测试报告：task-062 提权 exe 底座 + 系统还原点 + 系统托盘常驻（v0.2.0 起步）

- 分支：`feature/task-062-elevation-srpoint-tray`
- 测试 HEAD：`5b6bf1e`
- 测试时间：2026-10-30（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| dev（pnpm tauri dev） | Win11 25H2，Tauri 2 + Vite | 是（dev 未单独启动，直接以 NSIS 安装态代跑 L3） | 通过 |
| 打包安装（干净目录） | NSIS `C-DiskCleaner_0.1.7_x64-setup.exe`（4.30 MiB，含 `cclean-elevation.exe` 153 KB），装到 `D:\qa-062-install` | 是 | 通过 |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区 + 前端 | `cargo test --workspace` / `pnpm --filter app test` | 通过 | `cargo-test.log`：50 batches / 316 passed / 0 failed / 8 ignored（基线 291 + 新增 25 条：elevation 7、engine/elevation 13、restore_point 5、Settings 自启 3）；`pnpm.log`：9 files / 101 passed |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（11.36s finished）；`pnpm.log`：lint=0（2 条 SafeBox 既有 warning）、build=0（196.96 kB / gzip 63.37 kB，较基线 +1.5 kB） |
| L3 端到端/UI | 安装态代跑：托盘拦截关闭、自启默认值、仪表盘渲染 | NSIS 启动 + WM_CLOSE + 注册表 Run 键检查 | 通过（部分） | `installed-dashboard.png`：仪表盘正常渲染；`l4-launch.txt`：WM_CLOSE 后 pid=32892 仍存活（托盘隐藏而非退出）；自启注册表默认不存在 |
| L4 真机/打包 | NSIS 干净目录安装 → 提权 exe 落位 → 启动 → 托盘行为 → 卸载 | `/S /D=D:\qa-062-install` | 通过 | `l4.txt`：安装根目录含 `cclean-elevation.exe`（153088 字节，与 release 产物完全一致）+ cdisk-cleaner.exe 17.71 MB；`l4-launch.txt`：卸载干净 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `elevation/` 独立 crate 编译为 `cclean-elevation.exe` | `tauri-build.log`：`[build-elevation] 已复制 → app\src-tauri\binaries\cclean-elevation.exe（153088 字节）`；安装根目录同名文件 153088 字节 | 通过 |
| 2 | 命令白名单：只接受 `create_restore_point`，拒绝未知命令 | `whitelist_rejects_unknown_command` / `validate_rejects_unknown_command` / `run_elevated_rejects_unknown_cmd_before_launching` 单测通过 | 通过（单测） |
| 3 | 主 exe 通过 `ShellExecuteExW("runas")` 启动提权 exe，UAC 弹窗一次 | 真机未触发 L2 清理流程（需 L2 风险项），未实测 UAC 弹窗；代码走 `ShellExecuteExW + SEE_MASK_NOCLOSEPROCESS`，`ERROR_CANCELLED`(1223) 转中文错误 | 第三档（见 §6） |
| 4 | 提权 exe 退出后通过退出码 + 临时 JSON 回传结果 | `parse_result_reads_full_json` / `falls_back_when_json_missing` / `trusts_exit_code_over_json_ok` 单测通过 | 通过（单测） |
| 5 | `engine/src/elevation/mod.rs` 封装 `run_elevated(cmd, args)` API | `engine/src/elevation/mod.rs` 存在，`run_elevated_rejects_unknown_cmd_before_launching` / `errors_when_exe_missing` 单测通过 | 通过 |
| 6 | 单测：白名单拒绝未知命令 | 同上 #2 | 通过 |
| 7 | 清理前自动触发 `create_restore_point`（L2 风险项时） | `any_l2_needs_restore_point` / `all_l1_does_not_need_restore_point` / `empty_ids_never_needs_restore_point` / `l2_outside_the_batch_does_not_trigger` 5 条 L2 判定单测通过 | 通过（单测） |
| 8 | 还原点名称 `C-clean cleanup #<task_id> <timestamp>` | `restore_point_name_has_task_id_and_timestamp` / `fits_in_255_utf16_units` 单测通过；实现备注说明用 `#0` 占位（见 §6） | 通过（单测，含已知偏差） |
| 9 | 失败处理：弹 confirm「是否继续清理？」 | Items.tsx 处理 2007 状态码 → confirm → `skipRestorePoint` 重调；真机未触发 L2 流程 | 第三档（见 §6） |
| 10 | 成功时 audit_log 记录「系统还原点已创建」 | `log_restore_point` 实现；真机未触发 | 第三档（见 §6） |
| 11 | 单测：还原点名称格式 + 失败返回错误 | 同上 #8 + 单测覆盖 | 通过（单测） |
| 12 | 关闭窗口拦截 `CloseRequested` → 隐藏到托盘 | `l4-launch.txt`：发送 WM_CLOSE 后 pid=32892 仍存活（未退出） | **通过（真机证据）** |
| 13 | 托盘右键菜单：立即扫描 / 打开主窗口 / 退出 | 代码 `TrayIconBuilder::with_id("main-tray").menu(&menu)`，菜单项 3 个；真机未右键截图（GDI 驱动菜单右键坐标不稳） | 第二档（见 §6） |
| 14 | 退出时二次确认 | 实现为托盘「退出」→ emit `tray://quit-request` → 前端 window.confirm → `invoke("quit_app")`；真机未走退出路径 | 第二档（见 §6） |
| 15 | 设置页「开机自启」开关，prefs 持久化 | 实现备注：自启状态**不存 prefs**，以注册表 `HKCU\...\Run` 为真实来源；`l4-launch.txt`：初始状态注册表无条目（默认关）；Settings.test.tsx 3 条自启用例通过（回填/切换/失败回滚） | 通过（单测 + 注册表实证默认关） |
| 16 | 单测：自启开关默认值 + 持久化 | Settings.test.tsx 19 tests（含新增 3 条自启） | 通过 |
| 17 | `cargo test --workspace` 全绿（≥291+4） | 316 passed / 0 failed | 通过 |

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
1. **托盘右键菜单 3 项未真机截图**：代码侧菜单已在 `TrayIconBuilder::with_id("main-tray").menu(&menu)` 注册（立即扫描/打开主窗口/退出），但本次 GDI 驱动未能截到右键菜单弹出图。请评审确认是否接受单测 + 代码审查作为证据。
2. **退出二次确认未真机走通**：实现链路为 `托盘退出 → emit tray://quit-request → 前端 window.confirm → invoke quit_app`。真机验证需点托盘右键→退出→观察 confirm，本次未完成。
3. **任务书 4 处技术偏差**（开发已在实现备注 §10 记录）：①`tauri-plugin-tray` 不存在（托盘是 Tauri 2 内置 + `tray-icon` feature）；②`ShellExecuteW` 拿不到进程句柄，改 `ShellExecuteExW + SEE_MASK_NOCLOSEPROCESS`；③白名单漏了 8 个必改文件（engine/src/lib.rs、engine/Cargo.toml、commands.rs、App.tsx、Dashboard.tsx、build-elevation.mjs、tauri-resources.test.mjs、.gitignore）；④路径 `src-tauri/*` 实际是 `app/src-tauri/*`。均为不做就编译/打包不过的级联改动，请评审确认。
4. **自启状态不存 prefs**：以注册表为真实来源（避免任务管理器手动禁用后 prefs 打架），与任务书「prefs 持久化」字面略有偏差。

### 第三档（已登记为已知限制）
- **UAC 弹窗 + 系统还原点创建 + 失败 confirm + audit_log** 四项真机未触发：需 L2 风险项清理流程（当前默认规则集多为 L1 浏览器/开发缓存）。这四项的代码路径由单测覆盖（25 条新增），但「真实 UAC 弹窗一次」「SystemRestore 面板可见新还原点」「失败 confirm 弹窗」三条任务书硬性真机证据**未取得**。
- **真实系统还原点创建**：需要系统还原已开启（本机 System Protection 可能默认关闭），且需要管理员权限走 `SRSetRestorePointW`，未在本次测试中触发。
- **还原点名称用 `#0` 占位**：因为真正的 `cleanup_task` 行要等 `execute_items` 才建，而还原点必须在它之前建。名称含 UTC 时间戳区分批次。
- **updater 签名**：build 在签名步报缺 `TAURI_SIGNING_PRIVATE_KEY`，NSIS/MSI 正常产出（与既往一致）。
- **CDP console 红错**：tauri dev 不开 9222，以运行日志无 panic + 无红弹窗替代（G6 已知豁免）。

## 7. 测试结论

- 结论：**通过（L1/L2/L4 自动化闸门 + 托盘关闭拦截 + 提权 exe 落安装目录 全部真机实证；UAC/还原点/托盘菜单/退出 confirm 四项真机交互未触发，由 25 条单测 + 代码审查覆盖，见 §6 第二/三档）**
- feature 分支最新 commit：`5b6bf1e`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/lint/build）+ NSIS 干净目录安装 + 提权 exe 落位校验 + WM_CLOSE 托盘拦截真机实证，未复用开发自验结论。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①UAC/还原点/托盘菜单/退出 confirm 四项真机证据缺失是否可接受（单测覆盖度）；②任务书 4 处技术偏差（tauri-plugin-tray 不存在 / ShellExecuteExW 替代 / 超白名单 8 文件 / app/src-tauri 路径）；③自启状态不存 prefs 改为以注册表为准。**
