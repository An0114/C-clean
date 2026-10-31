# 测试报告：task-067 M1 D 模块——残留增强（注册表卸载列表↔文件目录交叉比对）

- 分支：`feature/task-067-residual-scan`
- 测试 HEAD：`434c23c`
- 测试时间：2026-10-31（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| dev（pnpm tauri dev） | Win11 25H2 | 是（以 NSIS 安装态代跑 L3） | 通过 |
| 打包安装（干净目录） | NSIS `C-DiskCleaner_0.1.7_x64-setup.exe`（4.41 MiB / 4623358 字节），装到 `D:\qa-067-install` | 是 | 通过 |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区 + 前端 | `cargo test --workspace` / `pnpm --filter app test` | 通过 | `cargo-test.log`：56 batches / **405 passed / 0 failed**（基线 382 + 新增 22 + 1 `#[ignore]` 真机冒烟用例；residual_test.rs 22 passed 1 ignored）；`pnpm.log`：12 files / **135 passed**（基线 123 + Residual.test 11 + App.test 1） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy -D warnings` / `pnpm lint` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（4 条既有 warning）；`pnpm.log`：lint=0（2 条 SafeBox 既有 warning）、build=0（220.75 kB / gzip 69.85 kB） |
| L3 端到端/UI | 安装态：Tab 栏「残留」按钮真机可见 | NSIS 启动 + GDI 截图 | 通过 | `dashboard.png`：tab 栏在「大文件」与「保险箱」之间新增「残留」按钮；仪表盘其他模块（4 卡 / 环形图 50% / WinSxS 卡片 / 趋势图）无回归 |
| L4 真机/打包 | NSIS 干净目录安装 → 启动 → 卸载 | `/S /D=D:\qa-067-install`（`-Verb RunAs` 绕 UAC 自动取消） | 通过 | `l4.txt`：installed=True、pid 在案、cleaned=True |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `scan_registry_uninstall()` 返回注册表卸载项列表 | 手写 `windows` crate FFI（未引 winreg），读 HKLM 64位 / WOW6432Node 32位 / HKCU 三处；过滤 DisplayName 空 / SystemComponent=1 / ParentKeyName；开发真机过滤后 94 条 | 通过（单测 + 开发真机数据） |
| 2 | `scan_file_residuals()` 返回孤儿文件夹列表 | 只扫 Program Files / Program Files (x86) + 13 个系统组件黑名单（Windows Kits/WSL/dotnet/Hyper-V 等）；开发真机 dirs_scanned=28 | 通过（单测 + 开发真机数据） |
| 3 | `cross_reference()` 返回两类残留 | 类型 A（注册表有卸载项但 InstallLocation 目录不存在）；类型 B（Program Files 目录存在但注册表无对应项，三级归属判定偏向「有归属」）；开发真机 typeA=3 / typeB=11 / 95.7 MB | 通过（单测 + 开发真机数据） |
| 4 | 单测：交叉比对逻辑正确 | residual_test.rs 22 条通过 | 通过 |
| 5 | `residual_scan()` IPC handler 注册成功 | `app/src-tauri/src/commands.rs` + `lib.rs` 注册；前端 invoke 可调用 | 通过（代码审查 + 单测） |
| 6 | 前端调用返回完整数据结构 | Residual.test 11 条覆盖渲染 | 通过（单测） |
| 7 | 单测：handler 返回值格式正确 | 同 #4/#6 | 通过 |
| 8 | 两个分类 tab（注册表残留 / 文件残留） | Residual.tsx 内部分类 tab；Residual.test 断言两 tab 切换 | 通过（单测） |
| 9 | 列表显示：名称 / 路径 / 大小 / 类型 | Residual.tsx 列表列定义；Residual.test 覆盖 | 通过（单测） |
| 10 | 文件残留可勾选删除，注册表残留仅提示 | Residual.test 断言「注册表残留 tab 里没有勾选框，只给键路径让人手动处理」；文件残留勾选后走 `residual_delete` IPC → 移入保险箱 7 天可恢复（非直接删，因 Program Files 不在 L3 红线内） | 通过（单测） |
| 11 | 风险徽标（L2 需确认） | Residual.tsx 列表项带 L2 徽标 | 通过（代码审查 + 单测） |
| 12 | 深色主题适配（CSS 变量） | Residual 全页走 CSS 变量 | 通过（代码审查） |
| 13 | 单测：页面渲染正确 | Residual.test.tsx 11 条通过 | 通过 |
| 14 | Tab 栏加「残留」按钮 | `dashboard.png` 真机实证：tab 栏「大文件」与「保险箱」之间新增「残留」 | **通过（真机截图）** |
| 15 | 切换渲染 `<Residual />` | App.test 新增 1 条 tab 切换用例（27 total，较基线 +1） | 通过（单测） |
| 16 | 单测：tab 切换正确 | 同 #15 | 通过 |
| 17 | `cargo test --workspace` 全绿（382+≥4） | 405 passed / 0 failed | 通过 |

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
1. **白名单 5 处偏差**（开发已登记）：①`engine/src/execute/safe_box.rs` 漏列（`delete_dir` 从 Err 桩实现为转调 delete_file，`delete_file` 对目录走 `dir_total_bytes` 递归求和修配额缺陷，不存在路径 no-op——不改就删不了目录）；②`app/src-tauri/src/{commands,lib}.rs` 漏列（与 065/066 同样的遗漏）；③`engine/src/lib.rs` 无需改；④`app/src/utils/invoke.ts` 未改（现有模式无 per-command 封装）；⑤`app/src/App.test.tsx` 漏列（验收 16 只能在 App 层测）。
2. **「文件残留可勾选删除」实现为「移入保险箱 7 天可恢复」**（非直接删）：理由是 Program Files 不在 L3 红线内，白名单拦不住误判，必须留可恢复退路。与任务书「可删除」表述有偏差。
3. **类型 B 误报率无法归零**：开发初版把 AppData 当安装目录导致 182 条误报（华为/飞书/腾讯/JetBrains 等正常程序），修正为只扫 Program Files 系列 + 13 个系统组件黑名单后压到 11 条；但「注册表里没有」推不出「已卸载」，要提高准确率需引入文件指纹/厂商签名/版本资源比对，属新功能点。
4. **注册覆盖面有限**：只覆盖 Uninstall 三处（HKLM 64位/WOW6432Node/HKCU），不含 `SystemComponent` 隐藏项、不含微软商店应用注册信息；类型 A 只看 `InstallLocation`，不看 `DisplayIcon`/`UninstallString` 里的路径（InstallLocation 为空的项完全没做残留检查）。

### 第三档（已登记为已知限制）
- **点「残留」tab 真机渲染两分类列表 + 勾选删除**：未点 tab（DPI 坐标偏移老问题），由 Residual.test 11 条 + App.test 1 条 tab 切换 + 22 条 rust 单测覆盖；真机仅证实 tab 按钮存在。
- **「切 tab → 调 IPC → 后端扫描注册表+文件 → 前端展示两类残留」端到端写路径**：由开发 `cargo test -- --ignored real_scan` 数据覆盖（registry_entries=94 / typeA=3 / typeB=11 / 95.7 MB / 75 ms；3 条类型 A 分别为 Chromebrowser 拼错路径、D:/IDEA 已不存在、D:/Java/jdk-25 已不存在）。
- **文件残留勾选后移入保险箱的真机行为**：未点入页面勾选删除，由 Residual.test + `safe_box.rs` 单元测试覆盖。
- **updater 签名**：build 末尾报缺 `TAURI_SIGNING_PRIVATE_KEY`，NSIS/MSI 正常产出（与既往一致）。
- **CDP console 红错**：tauri dev 不开 9222，以运行日志无 panic + 无红弹窗替代（G6 已知豁免）。
- **Items/SafeBox/History 三页深色适配待补**（本任务新增 Residual 页已全走 CSS 变量）。

## 7. 测试结论

- 结论：**通过（L1/L2/L4 自动化闸门 + Tab 栏「残留」按钮真机可见 全部通过；Residual 页面两分类列表/勾选删除/移入保险箱 真机未点入，由 33 条新增单测（22 rust + 11 前端）+ 开发真机 real_scan 数据覆盖，见 §6 第三档）**
- feature 分支最新 commit：`434c23c`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/lint/build）+ NSIS 干净目录安装 + Tab 栏新按钮真机截图，未复用开发自验结论。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①白名单 5 处偏差（尤其 safe_box.rs 目录删除能力 + 配额缺陷修正为何必须改）；②「移入保险箱」替代「直接删」是否可接受；③类型 B 误报率 11 条是否可接受；④注册覆盖面有限是否可接受。M1 D 模块落地，下一步 F 模块注册表健康中心（task-068）。**
