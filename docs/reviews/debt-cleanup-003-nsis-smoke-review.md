# 测试评审报告：debt-cleanup-003 NSIS 干净目录真机验收（第二轮）

## 分支与 commit

- 分支：`main`
- 基线 commit：`2e52c5d`（debt-cleanup-003 两个 P0 post-install bug 修复完成）
- 修复提交：
  - `c5cf1d3` fix(frontend): pass scope='full' to scan_start command
  - `16b30e2` fix(tauri): use app_data_dir() for DB/safe_box, bundle rules+scripts as resources
- 安装包：`C-Disk Cleaner_0.1.0_x64-setup.exe`（3.62 MiB）
- 安装位置：`C:\Program Files\C-Disk Cleaner\`（perMachine NSIS `/S`）
- 被测进程：`cdisk-cleaner.exe`，WebView2 remote debugging @ :9222

---

## 分步验证清单（每条必须有运行时现象）

| # | 验收点 | 现象（实际看到的） | 结论 |
|---|--------|------------------|------|
| 1 | NSIS 安装包产出并静默安装 | `pnpm tauri build` 成功；`/S` 静默安装到 `C:\Program Files\C-Disk Cleaner\` | ✅ |
| 2 | Dashboard 打开不白屏 | 标题「C-Disk Cleaner」、5 个 tab、绿色「开始扫描」按钮、4 个数字卡片渲染骨架后填充 | ✅ |
| 3 | 点「开始扫描」不红条 | 点后按钮变「扫描中...」，约 12 秒后扫描完成变「再次扫描」。**不再报 `missing required key scope`**（Bug-1 修复确认） | ✅ |
| 4 | 扫描进度条走动 | 进度条区域文字一直显示「准备中... — 项」，12 秒内 percent/scanned_items 未变化。按钮状态从「扫描中...」切到「再次扫描」说明扫描实际完成，但用户看不到实时进度 | ❌ |
| 5 | 扫描后数字卡片有值 | 总释放空间显示「0 B」；**成功次数/失败次数/清理轮次后面空白**（无数字，不是 0）。DB 查询确认 scan_session 有 2 条 completed 记录，但 cleanup_task 表无数据（未执行过清理） | ❌ |
| 6 | 清理 tab 列出待清理项 | 扫描后列出 **100+ 条待清理项**，含 windows_update（C:\Windows\SoftwareDistribution\）、temp（C:\Users\An\AppData\Local\Temp\）、dev_cache（pip cache）等模块，每条带路径/大小/模块/Session 编号 | ✅ |
| 7 | 保险箱 tab 空状态 | 「刷新」「共 0 项 / 总大小 0 B」「保险箱为空」 | ✅ |
| 8 | 历史 tab 空状态 | 表头「时间/事件类型/路径/详情」，「暂无记录」，分页「第 1–0 条」 | ✅ |
| 9 | 设置页检查更新显示版本号 | 点「检查更新」后红条：`加载失败：Error: 内部错误（附日志 ID）`，版本全 `-` | ❌ |
| 10 | DB 可写（Bug-2 修复确认） | DB 位于 `%APPDATA%\com.and0114.cclean\data\cdiskcleaner.db`（90KB），rules 复制到 `data_root\rules\`，safe_box 目录在 `data_root\safe_box\`。**不再报 `os error 5 拒绝访问`** | ✅ |
| 11 | DevTools Console 零红色 error | CDP 收集 idle + 点开始扫描后的 Console：**零 error、零 exception** | ✅ |

---

## 评审结论：**不通过**

debt-cleanup-003 的两个 P0 bug（scope 参数 + app_data_dir）修复到位，M1 核心链路（扫描→列出待清理项）已打通。但 Dashboard 数字卡片无值（验收点 5 不通过），且进度条不更新（验收点 4 不通过）。这两个问题都与前后端数据契约有关，不应放过。

---

## Bug 清单

### Bug-1（中）：Dashboard 数字卡片字段名不匹配，成功/失败/轮次空白

- **现象**：Dashboard 4 个数字卡片中，「总释放空间」显示「0 B」，但「成功次数」「失败次数」「清理轮次」后面无数字（空白）。DB 查询确认 `scan_session` 有 2 条 completed 记录，但前端拿不到 `success_count`/`failed_count`/`session_count`。
- **根因**：后端 `engine/src/audit_log/stats.rs` 的 `CleanupStats` 用 `#[serde(rename_all = "camelCase")]`，JSON 输出 `totalBytesFreed/successCount/failedCount/sessionCount`；但前端 `app/src/pages/Dashboard.tsx` 的 interface 是 snake_case（`total_bytes_freed/success_count/failed_count/session_count`）。`api()` helper 解包后字段对不上，三个数字字段为 `undefined`。
- **复现步骤**：
  1. 安装 NSIS 包并启动
  2. Dashboard 加载完成后观察数字卡片
  3. 成功次数/失败次数/清理轮次空白
- **期望行为**：4 个数字都应显示 0（空 DB 的 COALESCE 结果）或真实聚合值。前后端字段命名需对齐（要么前端改 camelCase，要么后端去掉 `rename_all = "camelCase"`）。
- **严重度**：中（显示问题，不影响扫描/清理功能，但 Dashboard 是用户首页）

### Bug-2（中）：扫描进度条文字不更新，一直显示「准备中...」

- **现象**：点「开始扫描」后按钮变「扫描中...」，进度条区域文字 12 秒内一直显示「准备中... — 项」，percent 和 scanned_items 未变化。扫描实际完成（按钮切到「再次扫描」，清理 tab 列出 100+ 项），但用户看不到实时进度。
- **复现步骤**：
  1. Dashboard 点「开始扫描」
  2. 观察进度条文字
  3. 12 秒内一直「准备中... — 项」
- **期望行为**：进度条应实时显示当前扫描路径、已扫描项数、百分比（`scan://progress` 事件的 `current_path/scanned_items/percent`）。
- **严重度**：中（扫描实际在跑，但用户看不到进度，体验差）

### Bug-3（中，上轮已登记）：设置页「检查更新」报「内部错误」而非网络错

- **现象**：Settings tab 点「检查更新」，红条显示 `加载失败：Error: 内部错误（附日志 ID）`，版本全 `-`。
- **复现步骤**：
  1. 切到「设置」tab
  2. 点「检查更新」
- **期望行为**：网络不通报「网络错误，请检查连接」；规则库损坏报「规则库校验失败」；只有未预期 panic 才报「内部错误」。
- **严重度**：中（不阻塞核心清理链路，但用户无法区分错误类型）

---

## Git 操作记录

- **未合并新代码**（debt-cleanup-003 已在 main 上，本次是合并后第二轮真机验收）。
- **Revert 记录**：无。
- **报告 commit**：本文件单独提交。

---

## 后续动作建议

1. **新建 debt-cleanup-004（P1）**：修复 Bug-1（前后端字段命名对齐）+ Bug-2（进度条事件订阅/渲染）+ Bug-3（设置页错误分类）。三个都是数据契约/错误处理问题，修复量小。
2. **修复后重跑 NSIS 干净目录冒烟**：确认 Dashboard 数字卡片有值、进度条走动、设置页错误信息分类。
3. **核心链路已通**：扫描→列出待清理项→不白屏→DB 可写→Console 零红错。M1 功能可用，上述 bug 为体验层问题，不阻塞发版但应在下个批次修掉。
