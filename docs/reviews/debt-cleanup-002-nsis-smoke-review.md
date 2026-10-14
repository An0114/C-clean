# 测试评审报告：debt-cleanup-002 NSIS 干净目录真机验收

## 分支与 commit

- 分支：`main`（debt-cleanup-002 合并后）
- 基线 commit：`77e0723`（merge feature/fix-m1-packaging-bugs）+ `fc513a8`（评审报告）
- 安装包：`C-Disk Cleaner_0.1.0_x64-setup.exe`（3.62 MiB，本机 `pnpm tauri build` 产出）
- 安装位置：`C:\Program Files\C-Disk Cleaner\`（perMachine，NSIS 静默 `/S`）
- 被测进程：`cdisk-cleaner.exe` PID 34104，WebView2 remote debugging @ :9222

---

## 分步验证清单（每条必须有运行时现象）

| # | 验收点 | 现象（实际看到的） | 结论 |
|---|--------|------------------|------|
| 1 | NSIS 安装包能产出并静默安装 | `pnpm tauri build` 产出 nsis 3.62 MiB + msi 5.13 MiB；`/S` 静默安装到 `C:\Program Files\C-Disk Cleaner\`，含 `cdisk-cleaner.exe` + `uninstall.exe` | ✅ |
| 2 | Dashboard 打开不白屏 | 页面正常渲染：标题「C-Disk Cleaner」、5 个 tab 按钮、绿色「开始扫描」按钮、4 个数字卡片骨架后填充。**无白屏**（debt-cleanup-002 Bug-1 修复生效） | ✅ |
| 3 | Dashboard 数字卡片有值 | 总释放空间显示 `0 B`；但「成功次数」「失败次数」「清理轮次」后面**无数字**（空白，不是 0） | ❌ |
| 4 | 点「开始扫描」不红条、进度条走动 | 点「开始扫描」后立即显示红条：`扫描失败 / invalid args `scope` for command `scan_start`: command scan_start missing required key scope`。**进度条从未出现**，扫描根本没启动 | ❌ |
| 5 | 清理 tab 能列出待清理项、勾选、清理 | tab 正常渲染：「刷新」「已选 0 项，共 0 B」「清理已选」按钮，表头「路径/大小/模块/Session」，内容「暂无待清理项」。空状态正常，不白屏 | ⚠️ 空列表（因扫描根本跑不起来，无待清理项） |
| 6 | 保险箱 tab 能列出、单条恢复 | tab 正常渲染：「刷新」「共 0 项 / 总大小 0 B」「保险箱为空」。空状态正常 | ✅ |
| 7 | 历史 tab 能列出审计记录 | tab 正常渲染：表头「时间/事件类型/路径/详情」，「暂无记录」，分页「上一页 / 第 1–0 条 / 下一页」 | ✅ |
| 8 | 设置 tab 点「检查更新」显示版本号 | 初始：当前版本 `-`、远端版本「检查中…」、SHA-256 `-`、状态「检查中…」。点「检查更新」后显示红条：`加载失败：Error: 内部错误（附日志 ID）`，版本全 `-` | ❌ |
| 9 | 设置页网络不通时报网络错而非「内部错误」 | 报「内部错误（附日志 ID）」，未区分网络错 / 解析错 / 后端错 | ❌ |
| 10 | DevTools Console 零红色 error | CDP 收集 idle + 点开始扫描后的 Console：**零 error、零 warning、零 exception**。所有错误通过 Tauri command 返回，被前端 try/catch 捕获显示红条，未抛到 JS 层 | ✅ |
| 11 | 应用从开始菜单启动后 DB 可写 | stderr 捕获（从 Program Files 目录启动时）：`[audit_log_stats] open db: db io error: 拒绝访问。(os error 5)`。DB 路径硬编码 `current_dir()`，perMachine 安装到 Program Files 后普通用户无写权限 | ❌ |

---

## 评审结论：**不通过**

M1 核心链路（扫描 → 列出待清理项 → 清理）在第 1 步就断了：`scan_start` 命令要求 `scope` 参数但前端没传，用户点「开始扫描」立即报错，后续清理/保险箱/历史全部无数据。这是阻塞性 bug，不能发版。

debt-cleanup-002 修的 5 个回归 bug 本身验证通过（不白屏、ACL、pnpm 冲突、reparse point、进度残留），但真机冒烟暴露了之前 dev 模式没覆盖到的新问题。

---

## Bug 清单

### Bug-1（严重，阻塞 M1 核心功能）：`scan_start` 缺 `scope` 参数

- **现象**：Dashboard 点「开始扫描」，立即红条报错：
  ```
  扫描失败
  invalid args `scope` for command `scan_start`: command scan_start missing required key scope
  ```
  进度条从未出现，扫描未启动。
- **复现步骤**：
  1. 安装 NSIS 包并启动应用
  2. Dashboard tab 点绿色「开始扫描」按钮
  3. 立即看到红条
- **期望行为**：点「开始扫描」后启动扫描，进度条走动，扫描完成后 Dashboard 数字卡片刷新，清理 tab 出现待清理项。
- **严重度**：严重（M1 核心功能不可用）

### Bug-2（严重）：DB 路径硬编码 `current_dir()`，perMachine 安装后不可写

- **现象**：从 `C:\Program Files\C-Disk Cleaner\` 目录启动时，stderr 输出：
  ```
  [audit_log_stats] open db: db io error: 拒绝访问。 (os error 5)
  ```
  用户从开始菜单/桌面快捷方式启动时，working directory 可能是 Program Files 或 System32，普通用户无写权限，SQLite 无法创建/打开 DB 文件。
- **复现步骤**：
  1. NSIS perMachine 模式安装到 `C:\Program Files\C-Disk Cleaner\`
  2. 通过开始菜单快捷方式启动（working directory 为安装目录）
  3. 观察 stderr / Dashboard 数字卡片
- **期望行为**：DB 路径应使用 `app.path().app_data_dir()`（`%LOCALAPPDATA%\com.and0114.cclean\`），与 WebView2 用户数据目录一致，普通用户可写。
- **严重度**：严重（干净安装场景下所有 IPC 命令都报 DB io error）

### Bug-3（中）：设置页「检查更新」报「内部错误」而非网络错

- **现象**：Settings tab 点「检查更新」，红条显示 `加载失败：Error: 内部错误（附日志 ID）`，版本全 `-`。
- **复现步骤**：
  1. 切到「设置」tab
  2. 点「检查更新」按钮
  3. 看到红条
- **期望行为**：网络不通时应报「网络错误，请检查连接」；规则库 JSON 损坏时报「规则库校验失败」；只有未预期的后端 panic 才报「内部错误」。当前所有失败都坍缩成「内部错误」，用户无法区分。
- **严重度**：中（影响用户排障，但不阻塞核心清理链路）

### Bug-4（低）：Dashboard 三个数字卡片渲染为空

- **现象**：Dashboard 4 个数字卡片中，「总释放空间」显示 `0 B`，但「成功次数」「失败次数」「清理轮次」后面无数字（空白）。
- **复现步骤**：安装后首次启动，Dashboard 加载完成后观察数字卡片。
- **期望行为**：4 个数字都应显示 0（空 DB 的 COALESCE 结果），或后端返回字段名与前端 interface 对齐。
- **严重度**：低（空 DB 场景下显示异常，但不影响功能）

---

## Git 操作记录

- **未合并**（debt-cleanup-002 已在 main 上，本次是合并后真机验收，发现新 bug）。
- **Feature 分支**：`feature/fix-m1-packaging-bugs` 已在上轮评审后删除。
- **Revert 记录**：无（本次未合并任何新代码到 main）。
- **新 bugfix 任务**：需要新建 `bugfix/debt-cleanup-003-scan-scope` 修复 Bug-1 + Bug-2（两者都阻塞 M1 发版）。

---

## 后续动作建议

1. **新建 bugfix 任务**：`debt-cleanup-003`，优先级 P0，修复 Bug-1（scan_start scope 参数对齐）和 Bug-2（data_dir 改用 `app.path().app_data_dir()`）。
2. **Bug-3 和 Bug-4** 可放入 `debt-cleanup-004`（P1），不阻塞发版但影响用户体验。
3. **发版门禁**：Bug-1 + Bug-2 修复前，不要触发 task-034 的 release workflow_dispatch 出 v0.1.1。用户从干净目录安装后完全无法使用扫描功能。
4. **补充冒烟清单**：后续每个 debt-cleanup 批次合并后，必须跑一次 `pnpm tauri build` → 静默安装 → CDP 点 5 个 tab → 收集 Console，不能只跑 lint/test/build 就通过。本次就是 dev 模式全绿但真机炸了。
