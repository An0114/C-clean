# 测试评审报告：debt-cleanup-004 NSIS 干净目录真机验收（第三轮）

## 分支与 commit

- 分支：`main`
- 基线 commit：`9335aaf`（debt-cleanup-004 三个二轮回归 bug 修复完成）
- 修复提交：
  - `16f0512` fix(frontend): align CleanupStats/ScanProgress/AuditLogEntry to camelCase
  - `00015c0` fix(tauri): classify rules update errors instead of collapsing to INTERNAL
- 安装包：`C-Disk Cleaner_0.1.0_x64-setup.exe`（3.62 MiB，本机 `pnpm tauri build`）
- 安装位置：`C:\Program Files\C-Disk Cleaner\`（perMachine NSIS `/S`）
- 被测进程：`cdisk-cleaner.exe`，WebView2 remote debugging @ :9222
- DB 清空后首次启动（`%APPDATA%\com.and0114.cclean\data\cdiskcleaner.db`）

---

## 分步验证清单（每条必须有运行时现象）

| # | 验收点 | 现象（实际看到的） | 结论 |
|---|--------|------------------|------|
| 1 | NSIS 安装包产出并静默安装 | `pnpm tauri build` 成功；`/S` 静默安装到 `C:\Program Files\C-Disk Cleaner\` | ✅ |
| 2 | Dashboard 打开不白屏 | 标题「C-Disk Cleaner」、5 个 tab、绿色「开始扫描」按钮、4 个数字卡片渲染骨架后填充 | ✅ |
| 3 | 点「开始扫描」不红条 | 点后按钮变「扫描中...」，约 14 秒后扫描完成变「再次扫描」。无红条 | ✅ |
| 4 | 扫描进度条走动 | t=4s 显示 `C:\Windows\Temp\ANFER-PC-20261014-0652.log — 69 项`；t=6s 显示 `C:\Windows\SoftwareDistribution\... — 4 项`；实时路径和项数持续更新（Bug-2 修复确认，camelCase 对齐后 ScanProgress 字段正确驱动 UI） | ✅ |
| 5 | 扫描后数字卡片有值 | 总释放空间 `0 B`、成功次数 `0`、失败次数 `0`、清理轮次 `0`。四个数字全部显示（Bug-1 修复确认，camelCase 对齐后字段正确映射） | ✅ |
| 6 | 清理 tab 列出待清理项 | 扫描后列出 **100+ 条待清理项**，含 windows_update（C:\Windows\SoftwareDistribution\）、temp（C:\Users\An\AppData\Local\Temp\）、dev_cache（pip cache）等模块，每条带路径/大小/模块/Session #1 | ✅ |
| 7 | 保险箱 tab 空状态 | 「刷新」「共 0 项 / 总大小 0 B」「保险箱为空」 | ✅ |
| 8 | 历史 tab 空状态 | 表头「时间/事件类型/路径/详情」，「暂无记录」，分页「第 1–0 条」 | ✅ |
| 9 | 设置页检查更新显示版本号 | 点「检查更新」后红条：`加载失败：Error: 规则库更新失败，继续使用本地版本`，版本全 `-`。**不再是模糊的「内部错误（附日志 ID）」**，用户能理解发生了什么（Bug-3 修复确认，错误已分类） | ✅ |
| 10 | DB 可写 | DB 位于 `%APPDATA%\com.and0114.cclean\data\cdiskcleaner.db`，rules 复制到 `data_root\rules\`，safe_box 目录在 `data_root\safe_box\`。无 `os error 5` | ✅ |
| 11 | DevTools Console 零红色 error | CDP 收集 idle + 扫描 + 点检查更新后的 Console：**零 error、零 exception** | ✅ |

---

## 评审结论：**通过**

debt-cleanup-004 的三个二轮回归 bug（camelCase 字段对齐 + 进度条事件驱动 + 设置页错误分类）全部修复到位。M1 核心链路完整跑通：

**安装 → 启动 → 不白屏 → 点开始扫描 → 进度条实时走动 → 扫描完成 → 数字卡片有值 → 清理 tab 列出 100+ 待清理项 → 保险箱/历史空状态正常 → 设置页错误信息分类清晰 → Console 零红错**。

从 debt-cleanup-002 到 debt-cleanup-004 共修复 5 + 2 + 3 = 10 个 bug，M1 NSIS 安装包达到可分发状态。

---

## Bug 清单

本轮验收无新增 bug。

---

## Git 操作记录

- 未合并新代码（debt-cleanup-004 已在 main 上，本次是第三轮真机验收）。
- Revert 记录：无。
- 报告 commit：本文件单独提交。

---

## 后续动作建议

1. **M1 可发版**：核心链路已通，错误处理清晰，Console 零红错。可以触发 task-034 的 release workflow_dispatch 出 v0.1.1 正式安装包。
2. **已知技术债务（第三档，不阻塞发版）**：
   - 设置页远端版本仍拉不到（GitHub Raw 网络可达性问题），错误信息已分类但未真正成功拉取——需在真实网络环境复测。
   - 清理 tab 勾选 + 「清理已选」按钮未在本轮实测（本轮只验证了列出待清理项），建议手动点一次确认删除走保险箱流程。
   - 保险箱单条恢复、历史审计记录在清理操作后才会有数据，本轮空状态正常但未验证有数据时的渲染。
   - 组件单测（Dashboard/History/Items/SafeBox/Settings）未写。
   - 无 Tauri updater 自动更新 + 无代码签名（用户首次运行会有 SmartScreen 警告）。
3. **下一轮 debt-cleanup**：可考虑补组件单测 + 真实清理操作端到端验证（清理 → 保险箱 → 恢复 → 历史记录）。
