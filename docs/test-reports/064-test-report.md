# 测试报告：task-064 保险箱 7 天 purge + 风险徽标 + 全选 L1（P0 收尾）

- 分支：`feature/task-064-purage-badge-l1all`
- 测试 HEAD：`5c378ee`
- 测试时间：2026-10-30（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| dev（pnpm tauri dev） | Win11 25H2 | 是（以 NSIS 安装态代跑 L3） | 通过 |
| 打包安装（干净目录） | NSIS `C-DiskCleaner_0.1.7_x64-setup.exe`（4.53 MiB），装到 `D:\qa-064-install` | 是 | 通过 |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区 + 前端 | `cargo test --workspace` / `pnpm --filter app test` | 通过 | `cargo-test.log`：51 batches / **334 passed / 0 failed**（基线 328 + 新增 6：purge 6 条真实 SQLite + 真实文件；前端 101→104 passed，+3 条徽标/全选用例） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（仅 4 条既有非阻塞 warning）；`pnpm.log`：lint=0（2 条 SafeBox 既有 warning）、build=0（201.48 kB / gzip 64.95 kB，+1.6 kB） |
| L3 端到端/UI | 安装态：仪表盘无回归 | NSIS 启动 + GDI 截图 | 通过 | `dashboard.png`：仪表盘 4 卡 + 环形图（50%，200.3/400.0/199.7 GB）+ 趋势图（2026-10-30 峰值 4.8 MB 绿色柱）正常渲染，无红屏 |
| L4 真机/打包 | NSIS 干净目录安装 → 启动 → 卸载 | `/S /D=D:\qa-064-install` | 通过 | `l4.txt`：installed=True、pid=25372、cleaned=True |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | 后端启动时跑 `purge_expired()`：DELETE 过期行 + 删磁盘文件 | `purges_expired_row_and_deletes_file` 单测用**真实 SQLite + 真实磁盘文件**端到端验证（过期行 DB 消失 + 磁盘文件消失 + audit_log 有 `auto_purge`） | 通过（单测，真实 DB+文件） |
| 2 | purge 前 audit_log 记 `action='auto_purge'` | 同上单测断言 audit_log 条目 | 通过（单测） |
| 3 | 前端保险箱页显示「7 天后自动永久删除」提示 | SafeBox.tsx 实现；真机未导航到保险箱页 | 第三档（见 §6） |
| 4 | 单测：`purge_expired()` 删过期行 + 保留未过期行 | 6 条单测：purge 过期+删文件 / 保留未过期 / 混合批只删过期 / 文件缺失仍删行 / `expires_at==now` 严格 `<` 保留 / `status!=in_box` 不 purge | 通过 |
| 5 | Items 页每项显示风险徽标（L0 灰/L1 绿/L2 黄/L3 红） | `RISK_META` + 徽标列实现；`risk badge renders with title` 单测通过 | 通过（单测） |
| 6 | hover 显示 tooltip 影响说明 | 用原生 `title` 属性；单测断言 title 文案 | 通过（单测） |
| 7 | 徽标样式适配深色主题（CSS 变量） | L0 用 `var(--text-muted)`；L1/L2/L3 语义色在深浅背景都够对比度 | 通过（代码审查） |
| 8 | 单测：风险等级映射正确 | 同上 #5 | 通过 |
| 9 | Items 页顶部加「全选 L1（安全）」按钮 | `selectAllL1` 实现；`selects only L1 items` 单测通过 | 通过（单测） |
| 10 | 点击后勾选所有 L1，L2/L3 不选 | 单测断言 L2/L3 未被勾选 | 通过（单测） |
| 11 | 按钮旁显示「已选 N 项 L1，约 X GB」 | 沿用页面已有「已选 N 项，共 X」统计 | 通过（代码审查） |
| 12 | 单测：全选 L1 逻辑（只选 L1，不选 L2/L3） | `selects only L1 items` + `disables when no L1 on page` 单测通过 | 通过 |
| 13 | `cargo test --workspace` 全绿（328+≥3） | 334 passed / 0 failed | 通过 |

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
1. **任务书 3 处技术偏差**（开发已在实现备注 §7 记录）：①白名单漏 `engine/src/execute/list_pending.rs`（`PendingItem` 加 `risk_level` 是徽标功能的前提）；②purge 触发点写「engine/src/lib.rs 或 main.rs」，实际在 `app/src-tauri/src/lib.rs` setup 起线程（engine 是纯库无启动概念）；③徽标与全选 L1 同改 Items.tsx，无法拆两个原子提交，合并为一次 feat。
2. **purge 不是「到点必删」**：进程退出后不再检查（当前为「启动跑一次 + 每 6 小时」）。严格实现需 schtasks 计划任务（task-062 已论证代价）。对 7 天粒度体验影响小。
3. **全选 L1 只作用于当前页**：跨页需虚拟滚动改造。

### 第三档（已登记为已知限制）
- **保险箱页「7 天后自动永久删除」提示真机截图**：未导航到保险箱页，由 SafeBox.tsx 代码审查 + 13 条 SafeBox 单测覆盖。
- **Items 页风险徽标四色 + hover tooltip 真机截图**：需先跑扫描出列表，由 3 条新增 Items 单测（19 total，较基线 +3）覆盖。
- **全选 L1 按钮真机点击**：需扫描出含 L1/L2 混合列表，由 `selects only L1 items` 单测覆盖。
- **purge 端到端真机验证（造过期行 → 重启 → DB/文件消失）**：单测已用真实 SQLite + 真实磁盘文件验证同一逻辑（6 条），未在真机重复造过期行。
- **updater 签名**：build 在签名步报缺 `TAURI_SIGNING_PRIVATE_KEY`，NSIS/MSI 正常产出（与既往一致）。
- **CDP console 红错**：tauri dev 不开 9222，以运行日志无 panic + 无红弹窗替代（G6 已知豁免）。

## 7. 测试结论

- 结论：**通过（L1/L2/L4 自动化闸门 + 仪表盘无回归 全部通过；purge/徽标/全选 L1 的 UI 真机证据由 9 条新增单测（6 purge 真实 DB+文件 + 3 UI jsdom）覆盖，见 §6 第三档）**
- feature 分支最新 commit：`5c378ee`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/lint/build）+ NSIS 干净目录安装 + 仪表盘无回归截图（含真实趋势图峰值 4.8 MB），未复用开发自验结论。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①任务书 3 处技术偏差；②purge 为「运行期间定期」非「到点必删」；③徽标/全选 L1/purge 提示 UI 真机证据缺失是否可接受单测覆盖。P0 至此全部清完。**
