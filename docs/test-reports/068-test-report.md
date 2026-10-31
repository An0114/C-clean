# 测试报告：task-068 M1 F 模块——注册表健康中心（Run 键/卸载残留/计划任务，reg export 备份）

- 分支：`feature/task-068-registry-health`
- 测试 HEAD：`7117e6c`
- 测试时间：2026-10-31（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）
- **M1 四个业务模块（C/D/E/F）收官任务**

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| dev（pnpm tauri dev） | Win11 25H2 | 是（以 NSIS 安装态代跑 L3） | 通过 |
| 打包安装（干净目录） | NSIS `C-DiskCleaner_0.1.7_x64-setup.exe`（4.45 MiB / 4665411 字节），装到 `D:\qa-068-install` | 是 | 通过 |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区 + 前端 | `cargo test --workspace` / `pnpm --filter app test` | 通过 | `cargo-test.log`：57 batches / **431 passed / 0 failed**（基线 405 + 新增 25 + 1 `#[ignore]` 真机冒烟用例；registry_health_test 25 passed 1 ignored）；`pnpm.log`：13 files / **148 passed**（基线 135 + RegistryHealth.test 12 + App.test 1） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy -D warnings` / `pnpm lint` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（4 条既有 warning）；`pnpm.log`：lint=0（2 条 SafeBox 既有 warning）、build=0（230.32 kB / gzip 71.78 kB，+9.57 kB） |
| L3 端到端/UI | 安装态：Tab 栏「注册表」按钮真机可见 | NSIS 启动 + GDI 截图 | 通过 | `dashboard.png`：tab 栏在「残留」与「保险箱」之间新增「注册表」按钮；仪表盘其他模块（4 卡 / 环形图 51% / WinSxS 卡片 / 趋势图）无回归 |
| L4 真机/打包 | NSIS 干净目录安装 → 启动 → 卸载 | `/S /D=D:\qa-068-install`（`-Verb RunAs` 绕 UAC 自动取消） | 通过 | `l4.txt`：installed=True、pid 在案、cleaned=True |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `scan_run_keys()` 返回开机启动项列表（HKLM + HKCU） | 手写 `windows` crate FFI（未引 winreg），读 HKLM 64位 / WOW6432Node 32位 / HKCU 三处 Run 键；开发真机 run=16 | 通过（单测 + 开发真机数据） |
| 2 | `scan_uninstall_residual()` 返回卸载残留列表 | 复用 067 residual 逻辑；开发真机 uninstall=3（Chrome/IDEA/JDK 路径已不存在） | 通过（单测 + 开发真机数据） |
| 3 | `scan_scheduled_tasks()` 返回计划任务列表 | 读 `%SystemRoot%\System32\Tasks` 下 XML 文件提取 `<Command>`/`<Arguments>`（不用 schtasks 避免本地化文本解析、不用 COM 避免引 TaskScheduler 接口）；开发真机 task=0 / taskReadable=false（普通用户无权读 System32\Tasks） | 通过（单测 + 开发真机数据，已知权限限制见 §6） |
| 4 | `reg_export_backup()` 删除前自动备份 | 自己生成 `.reg` 文本（不调 reg.exe 避免 spawn 外部 exe + 安全软件拦截），落 `<data_root>\registry_backups\<HKLM\|HKCU>-<键路径>-<unix秒>.reg`；双击可还原；开发真机删除链路实测：写值 → delete_run_values → 值消失 + 备份文件存在 + 备份内容含值名与命令行 | 通过（单测 + 开发真机数据） |
| 5 | 单测：各扫描函数返回格式正确 | registry_health_test.rs 25 条通过 | 通过 |
| 6 | `registry_health_scan()` IPC handler 注册成功 | `app/src-tauri/src/commands.rs` + `lib.rs` 注册；前端 invoke 可调用 | 通过（代码审查 + 单测） |
| 7 | `registry_health_backup()` IPC handler 注册成功 | **合并为 `registry_health_delete`**（fail-closed：备份是删除内部不可分割一步，不单独暴露 backup）；开发说明「只备份不删无实际用途，备份和删除分开反而是最危险组合」 | 通过（偏差，见 §6 第二档） |
| 8 | 前端调用返回完整数据结构 | RegistryHealth.test 12 条覆盖渲染 | 通过（单测） |
| 9 | 单测：handler 返回值格式正确 | 同 #5/#8 | 通过 |
| 10 | 三个分类 tab（启动项 / 卸载残留 / 计划任务） | RegistryHealth.tsx 内部分类 tab；RegistryHealth.test 断言「挂载即扫描，三个分类 tab 带数量」 | 通过（单测） |
| 11 | 列表显示：名称 / 路径 / 风险等级 / 建议 | RegistryHealth.tsx 列表列定义；RegistryHealth.test 覆盖 | 通过（单测） |
| 12 | 删除前自动 reg export 备份 + 提示备份路径 | RegistryHealth.tsx 删除后显示备份路径；开发独立测试键实测备份文件存在且内容正确 | 通过（单测 + 开发真机数据） |
| 13 | 风险徽标（L2 需确认 / L3 禁删） | 风险分级：①exe 路径以 C:/Windows/ 开头 → L3；②命中 17 个安全软件关键词（defender/kaspersky/火绒/360 等）→ L3；③目标文件不存在 → L2；④其余 → L2。后端删除前重新读值重新算等级（不信前端传的 riskLevel），命中 L3 直接拒；开发真机 L3 三条全对（SecurityHealth/RtkAudUService/Sysdiag 火绒） | 通过（单测 + 开发真机数据） |
| 14 | 深色主题适配（CSS 变量） | RegistryHealth 全页走 CSS 变量 | 通过（代码审查） |
| 15 | 单测：页面渲染正确 | RegistryHealth.test.tsx 12 条通过 | 通过 |
| 16 | Tab 栏加「注册表」按钮 | `dashboard.png` 真机实证：tab 栏「残留」与「保险箱」之间新增「注册表」 | **通过（真机截图）** |
| 17 | 切换渲染 `<RegistryHealth />` | App.test 新增 1 条 tab 切换用例（28 total，较基线 +1） | 通过（单测） |
| 18 | 单测：tab 切换正确 | 同 #17 | 通过 |
| 19 | `cargo test --workspace` 全绿（405+≥4） | 431 passed / 0 failed | 通过 |

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
1. **白名单 4 处偏差**（开发已登记）：①`engine/src/scan/winreg.rs` 漏列（注册表封装要在 residual 与 registry_health 之间共用，避免复制 120 行 FFI）；②`engine/src/scan/residual.rs` 漏列（改为引用 `winreg` 模块，删掉自己那份 FFI 副本）；③`app/src-tauri/src/{commands,lib}.rs` 漏列（与 065/066/067 同样的遗漏）；④`app/src/utils/invoke.ts` 未改（现有模式无 per-command 封装）；另 `engine/src/lib.rs` 无需改、`app/src/App.test.tsx` 漏列（验收 18 只能在 App 层测）。
2. **任务书要求 `registry_health_backup` 独立 IPC，实现合并为 `registry_health_delete`**：理由是 fail-closed（备份写不出来就不删），备份是删除内部不可分割一步；「只备份不删」无实际用途，备份和删除分开反而是最危险组合（中间失败可能变成「备份了但没删」或「没备份就删」）。
3. **计划任务扫描在非管理员下恒为空**：`System32\Tasks` 默认只有 Administrators/SYSTEM 可读，普通用户 `Permission denied`。已用 `taskScanAvailable` 字段如实区分「没有任务」和「读不到」，UI 明确写「需要管理员权限」。要真正支持需提权跑扫描（可复用 task-062 提权 exe，属新任务）。
4. **只覆盖 Run 键**：不含 `RunOnce`（一次性启动项）、`Startup` 文件夹快捷方式、服务自启动；卸载残留只检查 `InstallLocation`，为空的项（本机 94 条里有一部分）没做检查。

### 第三档（已登记为已知限制）
- **点「注册表」tab 真机渲染三分类列表 + 勾选删除**：未点 tab（DPI 坐标偏移老问题），由 RegistryHealth.test 12 条 + App.test 1 条 tab 切换 + 25 条 rust 单测覆盖；真机仅证实 tab 按钮存在。
- **「切 tab → 调 IPC → 后端扫描 → 前端展示三分类」端到端写路径**：由开发 `cargo test -- --ignored real_scan` 数据覆盖（run=16 / uninstall=3 / task=0 taskReadable=false / 62 ms；L3 三条全对：SecurityHealth/RtkAudUService/Sysdiag 火绒；exists=false 三条都是真残留：nrc_launcher 洛克王国、QuarkUpdater、TabbitBrowserUpdater）。
- **「勾选删除 → reg export 备份 → 删除」真机行为**：未点入页面勾选删除，由开发独立测试键实测覆盖（HKCU\Software\C-Clean-Test-<pid>-<suffix>：写值 → delete_run_values → 值消失 + 备份文件存在 + 备份内容含值名与命令行 + L3 拒删且值仍在 + 值不存在时幂等跳过）。
- **updater 签名**：build 末尾报缺 `TAURI_SIGNING_PRIVATE_KEY`，NSIS/MSI 正常产出（与既往一致）。
- **CDP console 红错**：tauri dev 不开 9222，以运行日志无 panic + 无红弹窗替代（G6 已知豁免）。
- **Items/SafeBox/History 三页深色适配待补**（本任务新增 RegistryHealth 页已全走 CSS 变量）。

## 7. 测试结论

- 结论：**通过（L1/L2/L4 自动化闸门 + Tab 栏「注册表」按钮真机可见 全部通过；RegistryHealth 页面三分类列表/勾选删除/reg export 备份 真机未点入，由 37 条新增单测（25 rust + 12 前端）+ 开发真机 real_scan 数据覆盖，见 §6 第三档）**
- feature 分支最新 commit：`7117e6c`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/lint/build）+ NSIS 干净目录安装 + Tab 栏新按钮真机截图，未复用开发自验结论。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①白名单 4 处偏差（尤其 winreg.rs 共用封装与 residual.rs 重构）；②`registry_health_backup` 合并为 `registry_health_delete` 是否可接受；③计划任务非管理员恒空是否可接受；④只覆盖 Run 键不覆盖 RunOnce/Startup 文件夹/服务是否可接受。M1 四模块（C/D/E/F）全部闭环，下一步进入 P1 体验项或 v0.2.0 发版准备。**
