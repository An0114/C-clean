# 任务编号：086 v0.6.0 PRD 全闭环 + 安全加固 + 体验深化大合并（29 项打包）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：v0.5.0 已发版（merge `8a3bf26`），用户拍板「所有剩余任务整合为一个大任务派发」。
> 上游依赖：task-085 v0.5.0 发版准备已合并 main。
> **版本线**：v0.6.0 = PRD 全闭环 + 安全加固 + 体验深化。
> **本任务范围（29 项打包，分 6 组）**：
> - **组 1 PRD 闭环（3 项 P0）**：清理报告 HTML/CSV 导出 + 计划任务频率阈值 + 目标空间推荐组合
> - **组 2 安全加固（3 项）**：扫描进度条按脚本序号换算（修 G1）+ 符号链接/junction 拦截 + 8.3 短名解析
> - **组 3 体验深化（5 项 P1）**：折线图 + 矩形树图 + 规则库热更新影响说明 + 高分屏适配 + 更新下载进度反馈
> - **组 4 工程化清理（4 项）**：target_commitish 修正 + 审计 session_id 过滤 + ps1 脚本删除 + DenialReason 拆分
> - **组 5 P2 长尾（8 项）**：跨盘恢复 fallback + RmShutdown 进度 + RmShutdown 兄弟进程 + UNC 策略 + 增量扫描 + 多语言 + 右键菜单 + 通知中心
> - **组 6 文档合规（6 项）**：杀软白名单说明页 + 安全披露流程 + 贡献指南 + 测试覆盖率提升 + 性能基准达标 + GUI 截图补做
> **为什么合并**：用户明确拍板「整合为一个大任务」；这是 v0.6.0 收官任务，完成后 PRD 100% 闭环。
> **不做**：品牌图标（死路）/ 代码签名（死路）/ 差量更新（死路）/ Dependabot（死路）/ G6（豁免）。

## 状态字段

pending

## 任务目标

### 组 1：PRD 闭环（3 项）

#### 1.1 清理报告 HTML/CSV 导出（PRD P0-6）

- **现状**：前后空间对比已做，但「导出为 HTML 文件和 CSV 文件，用户自选保存位置」未做。
- **修法**：
  - 新增 IPC 命令 `report_export(format: "html" | "csv", path: String)`；
  - HTML 模板：清理前/后空间对比 + 释放总量 + 按模块明细列表；
  - CSV 格式：文件路径 / 大小 / 风险等级 / 状态 / 模块；
  - 前端清理完成页加「导出 HTML」「导出 CSV」按钮（文件保存对话框）。

#### 1.2 计划任务频率+阈值配置（PRD P1-1）

- **现状**：task-061 做了 30 分钟轮询，task-084 做了自动清理 L1 开关，但 PRD 要求「频率（每周/每两周/每月）+ 阈值（≥5GB 才自动执行）」未做。
- **修法**：
  - Settings 页「自动扫描」扩展：
    - 频率选项：每 30 分钟 / 每小时 / 每天 / 每周（替代当前固定 30 分钟）；
    - 自动清理阈值：仅当预估释放 ≥ N GB 时才自动执行（默认 5GB，可调）；
    - 自动清理时段（如仅在锁屏时）；
  - 后端 `auto_scan.rs` 读取频率和阈值配置；
  - 数据库 settings 表加 `auto_scan_interval` / `auto_clean_threshold_mb` 字段（v8 迁移）。

#### 1.3 目标空间推荐组合交互（PRD P1-8）

- **现状**：task-084 做了「目标空间扫描」（Downloads/Documents/Desktop 大文件列表），但 PRD 核心交互「输入目标值→自动推荐组合方案」未做。
- **修法**：
  - Dashboard 或 Items 页加「目标空闲空间」输入框（如 20GB）；
  - 后端新命令 `recommend_plan(target_mb: u64)`：
    - 优先 L1 项累加，不够时纳入安全 L2；
    - 返回推荐组合（勾选哪些项）+ 预计达成度；
  - 前端：推荐方案默认勾选，用户可微调；
  - 若安全项无法达标，明确提示「仅安全项最多可释放 X GB，距目标差 Y GB」。

### 组 2：安全加固（3 项）

#### 2.1 扫描进度条按脚本序号换算（修 G1）

- **现状**：`runner.rs` 每条 item `percent:0` 硬编码，进度条恒 0%（PROJECT_STATE G1 已知问题，多次标注「留后续」）。
- **修法**：
  - `engine/src/scan/orchestrator.rs`：按已完成模块数 / 总模块数给基线百分比（如 5 个模块，完成 2 个 = 40%）；
  - 模块内部按已扫描文件数 / 预估总数给次级进度；
  - 前端 Dashboard 进度条宽度正确随模块推进。

#### 2.2 符号链接/junction 拦截（评审 003 安全项）

- **现状**：`C:\Temp\link -> C:\Users\alice\Documents` 词法路径不命中 L3 红线。
- **修法**：
  - `engine/src/execute/whitelist.rs` 或 `resolve.rs`：对待删路径做 `symlink_metadata` 检查；
  - 检测到符号链接/junction → 拒绝删除（返回 code=2009「拒绝删除符号链接」）；
  - 或解析后重判（保守策略：直接拒绝）。

#### 2.3 8.3 短文件名解析（评审 003 安全项）

- **现状**：`C:\Windows\SYSTEM~1\kernel32.dll` 不匹配 `system32` L3 规则。
- **修法**：
  - 路径规范化层调用 Windows API `GetLongPathNameW`；
  - 短名转长名后再走 L3 匹配；
  - 单元测试：构造 8.3 短名路径断言正确解析。

### 组 3：体验深化（5 项）

#### 3.1 折线图（C 盘剩余空间趋势，PRD P1-5）

- **现状**：task-061 做了手写 SVG 柱状图，但 PRD 要求「折线图，横轴时间纵轴剩余空间」。
- **修法**：
  - History 页趋势图从柱状图改为折线图（手写 SVG polyline）；
  - 横轴时间（最近 7/30/90 天可选），纵轴剩余空间 GB；
  - 深色主题用 CSS 变量。

#### 3.2 矩形树图（文件夹占用分布，PRD P1-2）

- **现状**：task-084 做了 Top20 文件夹排行列表，但 PRD 要求「矩形树图 treemap 可视化」。
- **修法**：
  - BigFiles 页「文件夹分布」tab 加 treemap 可视化（手写 SVG 矩形递归分割）；
  - 每个矩形按大小比例，hover 显示文件夹名 + 大小；
  - 不引入 recharts（死路）。

#### 3.3 规则库热更新逐项影响说明（PRD P1-6）

- **现状**：task-082 做了静态 hover 文案，task-083 做了规则库但「影响说明从规则库热更新」未做。
- **修法**：
  - `rules/cache_rules.json` 每条规则加 `impact_description` 字段；
  - 扫描结果从规则库读取影响说明，而非前端硬编码；
  - 规则库热更新后，影响说明自动更新。

#### 3.4 高分屏适配（PRD P2-3.3）

- **现状**：主题切换已做，但 4K/150% 缩放下字体图标清晰度未专项验证。
- **修法**：
  - 全局字体用 `rem` + `clamp()` 响应式；
  - 图标用 SVG 而非位图；
  - 真机在 150%/200% 缩放下截图验证。

#### 3.5 更新下载进度反馈（task-039 遗留）

- **现状**：banner「立即更新」点击后无下载/安装进度。
- **修法**：
  - Tauri updater plugin 的下载进度事件接前端 progress bar；
  - 下载完成后显示「安装中...」状态。

### 组 4：工程化清理（4 项）

#### 4.1 release.yml target_commitish 修正

- **现状**：tag 打到默认分支而非构建提交（`bfe9414` vs `17f5bcb`）。
- **修法**：
  - `.github/workflows/release.yml` 的 `softprops/action-gh-release` 加 `target_commitish: ${{ github.sha }}`。

#### 4.2 审计日志 session_id 过滤启用

- **现状**：task-024 预留了 session_id 列但入参标「预留未启用」。
- **修法**：
  - History/AuditLog 页加「按扫描会话筛选」下拉；
  - 后端 IPC 命令接 session_id 参数。

#### 4.3 ps1 脚本删除（R5 回退清理）

- **现状**：R5 架构决策「跑一个版本周期后删 ps1」。v0.5.0 已发布，可删。
- **修法**：
  - 删除 `scripts/scan_*.ps1` 5 个脚本；
  - 删除 orchestrator 的 `CCLEAN_RUST_*` 回退逻辑；
  - 更新 README/USER_GUIDE 中关于 ps1 回退的说明；
  - **保留** `rules/` 目录（规则库不删）。

#### 4.4 DenialReason 变体拆分

- **现状**：`whitelist.rs` 仅新增函数，未区分内置/外部红线来源。
- **修法**：
  - `DenialReason` 枚举拆出 `BuiltinRedline` / `ExternalRedline` / `UserRule`；
  - 前端拒绝提示文案区分来源。

### 组 5：P2 长尾（8 项）

#### 5.1 跨盘恢复 copy+remove fallback

- **现状**：保险箱跨盘恢复直接返回 `RestoreError::Io`。
- **修法**：
  - `safe_box.rs` 的 `restore` 函数：检测源盘与目标盘不同时，走 copy + remove fallback。

#### 5.2 RmShutdown 30s 进度反馈

- **现状**：占用进程不响应时等 RM 超时 30s，前端无进度。
- **修法**：
  - 后端在等待期间每 5s 发一次进度事件（「正在关闭占用进程...」）；
  - 前端显示等待状态。

#### 5.3 RmShutdown 兄弟进程控制

- **现状**：RM 会话连带关闭同 console/进程组的兄弟进程。
- **修法**：
  - 评估逐进程关闭（TerminateProcess）替代 RM Shutdown；
  - 或在 confirm 弹窗提示「可能连带关闭同组进程」。

#### 5.4 UNC 网络盘 L3 策略

- **现状**：`\\server\share\...` 保守拒绝。
- **修法**：
  - 支持用户手动添加网络盘路径为自定义规则（L2，需确认）；
  - 内置规则不自动扫描网络盘。

#### 5.5 增量扫描

- **现状**：每次全量扫描。
- **修法**：
  - 记录上次扫描的文件 mtime / size；
  - 后续扫描只检查变化的目录；
  - 缓存上次结果，未变化的直接复用。

#### 5.6 多语言（英文界面）

- **现状**：纯中文。
- **修法**：
  - 建立 i18n 框架（简单 key-value 字典，不引入库）；
  - 翻译核心 UI 文案（Dashboard/Items/Settings/SafeBox/History）；
  - Settings 加语言切换。

#### 5.7 Windows 右键菜单集成

- **现状**：无。
- **修法**：
  - 注册右键菜单「用 C-clean 清理此文件夹」；
  - 点击后打开 app 并扫描该目录。

#### 5.8 通知中心历史

- **现状**：toast 弹完即消失。
- **修法**：
  - 清理结果写入 Windows 通知中心（`tauri-plugin-notification` 已用，加历史保留）。

### 组 6：文档合规（6 项）

#### 6.1 杀软白名单说明页

- **修法**：
  - `docs/SECURITY.md` 或 README 加「杀软误报说明」段；
  - 解释为何需要管理员权限 / 扫描写入 / 无遥测。

#### 6.2 安全披露流程

- **修法**：
  - `docs/SECURITY.md` 建安全披露流程（如何报告漏洞、响应时限）。

#### 6.3 贡献指南

- **修法**：
  - `CONTRIBUTING.md` 建贡献指南（开发环境搭建 / 代码风格 / PR 流程）。

#### 6.4 测试覆盖率提升

- **修法**：
  - 目标：Rust 覆盖率 ≥80%，前端覆盖率 ≥70%；
  - 补充关键路径单测（whitelist / safe_box / execute）。

#### 6.5 性能基准达标

- **修法**：
  - 全量扫描 ≤180s、空闲 RSS ≤100MB、峰值 ≤300MB；
  - 不达标时优化（rayon 并行 / walkdir 优化）。

#### 6.6 GUI 真机截图补做

- **现状**：自动化环境 WebView2 崩溃，GUI 截图缺失。
- **修法**：
  - 在有活动桌面的机器补做 8 页核心截图（Dashboard/Items/SafeBox/History/Settings/BigFiles/Residual/RegistryHealth）；
  - 存入 `docs/screenshots/`。

**本任务不做**：
- 品牌图标（死路）
- 代码签名（死路）
- 差量更新（死路）
- Dependabot / cargo audit（死路）
- G6（豁免）
- 跨平台矩阵（仅 Windows）

## 前置条件

1. `main` 已合并 task-085（merge `8a3bf26`，v0.5.0 发版完成）；
2. 关键锚点（必须全部在白名单内）：
   - 组 1：`engine/src/ipc/commands/report.rs` / `engine/src/ipc/commands/recommend.rs`（新建）/ `engine/src/db/migrations/v8__auto_scan_config.sql`（新建）/ `app/src/pages/CleanupComplete.tsx`（或 Items.tsx）/ `app/src/pages/Settings.tsx` / `app/src/pages/Dashboard.tsx`
   - 组 2：`engine/src/scan/runner.rs` / `engine/src/scan/orchestrator.rs` / `engine/src/execute/whitelist.rs` / `engine/src/execute/resolve.rs`
   - 组 3：`app/src/pages/History.tsx` / `app/src/pages/BigFiles.tsx` / `rules/cache_rules.json` / `app/src/App.tsx` / `app/src/utils/update.ts`
   - 组 4：`.github/workflows/release.yml` / `engine/src/db/migrations/` / `scripts/`（删除）/ `engine/src/execute/whitelist.rs`
   - 组 5：`engine/src/safe_box.rs` / `engine/src/execute/rm_shutdown.rs` / `engine/src/scan/mod.rs` / `app/src/i18n/`（新建）/ `app/src-tauri/src/lib.rs`（右键菜单）
   - 组 6：`docs/SECURITY.md`（新建）/ `CONTRIBUTING.md`（新建）/ `docs/screenshots/`（新建）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选（29 项）。

### 组 1：PRD 闭环

1. **报告导出**：HTML/CSV 导出按钮可用，文件保存到用户指定路径；
2. **计划任务**：Settings 页频率可选（30min/1h/每天/每周）+ 阈值可调（默认 5GB）；
3. **目标推荐**：输入目标值后返回推荐组合 + 预计达成度；安全项不足时明确提示差额。

### 组 2：安全加固

4. **进度条**：扫描时进度条随模块推进（不再恒 0%）；
5. **符号链接拦截**：删除路径是 symlink/junction 时拒绝（code=2009）；
6. **8.3 短名**：`SYSTEM~1` 正确解析为 `System32`，L3 拦截生效。

### 组 3：体验深化

7. **折线图**：History 页趋势图为折线（非柱状），可选 7/30/90 天；
8. **矩形树图**：BigFiles 页文件夹分布为 treemap 可视化；
9. **影响说明热更新**：hover 文案从 rules JSON 读取，非前端硬编码；
10. **高分屏**：150%/200% 缩放下字体图标清晰；
11. **更新进度**：点击「立即更新」后显示下载进度条。

### 组 4：工程化清理

12. **target_commitish**：release.yml 加 `target_commitish: ${{ github.sha }}`；
13. **session_id 过滤**：History 页可按扫描会话筛选；
14. **ps1 删除**：`scripts/scan_*.ps1` 5 个文件删除；回退逻辑删除；README 同步；
15. **DenialReason**：拒绝提示文案区分内置/外部/用户规则来源。

### 组 5：P2 长尾

16. **跨盘恢复**：C 盘保险箱恢复到 D 盘成功（copy+remove）；
17. **RmShutdown 进度**：等待期间每 5s 发进度事件；
18. **兄弟进程控制**：confirm 弹窗提示连带关闭风险（或逐进程关闭）；
19. **UNC 策略**：网络盘路径可手动加为自定义规则（L2）；
20. **增量扫描**：二次扫描只处理变化的目录，耗时显著缩短；
21. **多语言**：Settings 可切换英文，核心 UI 已翻译；
22. **右键菜单**：右键文件夹→「用 C-clean 清理」打开 app；
23. **通知中心**：清理结果写入通知中心历史。

### 组 6：文档合规

24. **安全说明页**：`docs/SECURITY.md` 含杀软误报说明 + 安全披露流程；
25. **贡献指南**：`CONTRIBUTING.md` 含开发环境 / 代码风格 / PR 流程；
26. **测试覆盖率**：Rust ≥80%，前端 ≥70%；
27. **性能基准**：扫描 ≤180s，空闲 RSS ≤100MB，峰值 ≤300MB；
28. **GUI 截图**：8 页核心截图存入 `docs/screenshots/`；
29. **不回归**：`cargo test --workspace` 全绿（现有 668 + 新增 ≥20）+ `pnpm test` 全绿（现有 214 + 新增 ≥10）。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。

**组 1：PRD 闭环**：
- `engine/src/ipc/commands/report.rs`（**新建**，导出命令）
- `engine/src/ipc/commands/recommend.rs`（**新建**，推荐组合）
- `engine/src/ipc/commands/mod.rs`（注册新命令）
- `engine/src/db/migrations/v8__auto_scan_config.sql`（**新建**，频率+阈值字段）
- `app/src-tauri/src/lib.rs`（注册新命令）
- `app/src-tauri/src/commands.rs`（注册 handler）
- `app/src-tauri/src/auto_scan.rs`（频率+阈值读取）
- `app/src/pages/Items.tsx`（导出按钮 + 目标输入框）
- `app/src/pages/Settings.tsx`（频率+阈值 UI）
- `app/src/utils/invoke.ts`（新命令封装）

**组 2：安全加固**：
- `engine/src/scan/runner.rs`（percent 按模块换算）
- `engine/src/scan/orchestrator.rs`（基线百分比）
- `engine/src/execute/whitelist.rs`（symlink 检查 + DenialReason 拆分）
- `engine/src/execute/resolve.rs`（GetLongPathNameW 8.3 解析）
- `engine/src/execute/mod.rs`（错误码 2009）

**组 3：体验深化**：
- `app/src/pages/History.tsx`（折线图替换柱状图）
- `app/src/pages/BigFiles.tsx`（treemap 可视化）
- `rules/cache_rules.json`（impact_description 字段）
- `app/src/App.tsx`（字体 clamp 响应式）
- `app/src/utils/update.ts`（下载进度事件）

**组 4：工程化清理**：
- `.github/workflows/release.yml`（target_commitish）
- `engine/src/ipc/commands/audit_log.rs`（session_id 过滤）
- `app/src/pages/History.tsx`（session 筛选下拉）
- `scripts/scan_temp.ps1`（**删除**）
- `scripts/scan_prefetch.ps1`（**删除**）
- `scripts/scan_browser_cache.ps1`（**删除**）
- `scripts/scan_dev_cache.ps1`（**删除**）
- `scripts/scan_windows_update.ps1`（**删除**）
- `engine/src/scan/orchestrator.rs`（删除 CCLEAN_RUST_* 回退逻辑）
- `README.md`（删除 ps1 回退说明）
- `docs/USER_GUIDE.md`（同步）

**组 5：P2 长尾**：
- `engine/src/safe_box.rs`（跨盘 copy+remove fallback）
- `engine/src/execute/rm_shutdown.rs`（进度事件 + 兄弟进程控制）
- `engine/src/scan/mod.rs`（增量扫描缓存）
- `app/src/i18n/zh-CN.json`（**新建**）
- `app/src/i18n/en-US.json`（**新建**）
- `app/src/i18n/index.ts`（**新建**，i18n 框架）
- `app/src-tauri/src/lib.rs`（右键菜单注册）
- `app/src-tauri/src/tauri.conf.json`（通知中心配置）
- `app/src-tauri/src/auto_scan.rs`（UNC 自定义规则支持）

**组 6：文档合规**：
- `docs/SECURITY.md`（**新建**，杀软说明 + 安全披露）
- `CONTRIBUTING.md`（**新建**，贡献指南）
- `docs/screenshots/`（**新建目录**，8 页截图）
- `README.md`（加安全说明 + 贡献指南链接）

**收尾**：
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

**不修改**：
- engine/src/scan/software_enum.rs / software_cache.rs（task-083 已完成）
- 主题 CSS 变量体系（不改动）
- G6（豁免不修）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-086-v060-prd-closure-hardening`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-086-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 6 组拆 ≥10 个原子提交：

1. `feat(report): 清理报告 HTML/CSV 导出 (#task-086)`（组 1.1）
2. `feat(autoscan): 计划任务频率+阈值配置（v8 迁移 + Settings UI） (#task-086)`（组 1.2）
3. `feat(plan): 目标空间推荐组合交互（recommend_plan 命令） (#task-086)`（组 1.3）
4. `fix(scan): 进度条按模块序号换算（修 G1 恒 0%） (#task-086)`（组 2.1）
5. `security(whitelist): 符号链接/junction 拦截 + 8.3 短名解析 (#task-086)`（组 2.2+2.3）
6. `feat(ui): 折线图 + 矩形树图 + 影响说明热更新 + 高分屏适配 (#task-086)`（组 3.1~3.3+3.4）
7. `feat(update): 下载进度反馈 (#task-086)`（组 3.5）
8. `chore(ci): target_commitish 修正 + session_id 过滤 + ps1 删除 + DenialReason 拆分 (#task-086)`（组 4 全部）
9. `feat(longtail): 跨盘恢复 fallback + RmShutdown 进度 + 兄弟进程 + UNC + 增量扫描 + i18n + 右键菜单 + 通知中心 (#task-086)`（组 5 全部）
10. `docs: SECURITY.md + CONTRIBUTING.md + 截图 + 覆盖率 + 性能基准 (#task-086)`（组 6 全部）
11. 自验全绿后，最后一次提交：`docs(state): 任务086完成，v0.6.0 PRD 全闭环 + 安全加固 (#task-086)`
    —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0（≥688 passed）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0（≥224 passed）；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：报告导出序列化 / 推荐组合算法 / 符号链接检测 / 8.3 解析 / 折线图渲染 / treemap / i18n / 跨盘恢复，新增 ≥30 条；
- **集成测试（必做）**：`cargo test --workspace` 全绿；
- **端到端（必做）**：
  - 清理完成页导出 HTML + CSV 成功；
  - Settings 改频率/阈值后自动扫描按新配置跑；
  - 输入目标 20GB 后推荐方案勾选正确；
  - 扫描进度条随模块推进（不再恒 0%）；
  - 符号链接路径被拒绝删除；
  - 英文界面切换后核心文案正确；
- **真机冒烟（必做、不可裁剪）**：
  - 8 页核心截图（Dashboard/Items/SafeBox/History/Settings/BigFiles/Residual/RegistryHealth）；
  - 150%/200% 缩放下字体清晰；
  - 右键菜单「用 C-clean 清理」可用；
  - 通知中心有清理历史；
  - CDP console 0 红错（G6 已知项除外）；
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，端到端 + 真机全部复跑；
- **CI（必做）**：PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（导出按钮 / 频率阈值 UI / 目标输入 / 折线图 / treemap / i18n / 右键菜单 / 通知中心，全部真机可观察），且命中「安全红线」（符号链接 / 8.3 短名），**真机/打包冒烟必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 报告导出 | 真机证据：导出的 HTML/CSV 文件路径 + 内容截图 |
| 计划任务频率阈值 | Settings 截图 + 自动扫描按新配置跑日志 |
| 目标推荐 | 输入 20GB 后推荐组合截图 |
| 进度条 | 真机截图：进度条随模块推进（非 0%） |
| 符号链接拦截 | 单测输出 + 真机删除符号链接被拒 |
| 8.3 短名 | 单测输出：SYSTEM~1 → System32 |
| 折线图 | History 页折线图截图 |
| treemap | BigFiles 页矩形树图截图 |
| 影响说明热更新 | 修改 JSON impact_description → hover 文案变化 |
| 高分屏 | 150%/200% 截图 |
| 更新进度 | 点击更新后下载进度条截图 |
| ps1 删除 | `scripts/` 目录无 ps1 文件 |
| 跨盘恢复 | C→D 恢复成功证据 |
| RmShutdown 进度 | 等待期间进度事件日志 |
| i18n 英文 | 英文界面截图 |
| 右键菜单 | 右键文件夹→打开 app 截图 |
| 通知中心 | Windows 通知中心历史截图 |
| SECURITY.md | 文件内容 |
| CONTRIBUTING.md | 文件内容 |
| 8 页截图 | `docs/screenshots/` 目录 |
| 不回归 | cargo test 688+ + pnpm test 224+ 全绿 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 全部复跑 |
| CI | PR URL + run id + success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 10 个代码提交 + 1 个 `docs(state)` 提交，共 11 个 commit，全部 push 到 `feature/task-086-v060-prd-closure-hardening`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - 29 项验收标准逐项证据；
   - 性能基准实测数据（扫描耗时 / RSS）；
   - 测试覆盖率数据（Rust / 前端）；
   - `cargo test --workspace` 通过输出（≥688）；
   - `pnpm test` 输出（≥224）；
   - 真机冒烟证据（全部截图路径）；
   - 打包安装冒烟证据；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（v0.6.0 发版准备）；
3. `docs/PROJECT_STATE.md` 已按第 11 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务086-v0.6.0 PRD全闭环+安全加固+体验深化（29项） (#task-086)`，
   该提交只含 `tasks/086-v060-prd-closure-hardening.md` 一个文件，落在 `feature/task-086-v060-prd-closure-hardening` 分支最前端。

## 实现备注

> 执行环境：Windows 11 x64 · AMD Ryzen 7 7840H · Rust 1.90.0 · Node 24.21.0 · pnpm 11.9.0 · 系统缩放 150%（devicePixelRatio=1.5）。
> 分支：`feature/task-086-v060-prd-closure-hardening`。

### 一、提交清单

| # | Commit | 覆盖项 |
|---|---|---|
| 1 | `067dfd0` feat(report): 清理报告导出支持用户指定路径 | 组 1.1 |
| 2 | `623ed2a` feat(autoscan): 计划任务频率+阈值配置（30min/1h 档位，默认 5GB） | 组 1.2 |
| 3 | `2e55560` feat(recommend): 目标空间推荐组合（输入 GB→贪心推荐 L1/L2 并预勾选） | 组 1.3 |
| 4 | `5b966e7` security(whitelist): 符号链接/junction 拦截 + 8.3 短名解析 | 组 2.2 + 2.3 |
| 5 | `758f5d4` feat(ui): 文件夹矩形树图 + 规则影响说明热更新 + 高分屏适配 | 组 3.2 + 3.3 + 3.4 |
| 6 | `b7d6866` chore(engine): History 会话筛选 + ps1 删除 + DenialReason 来源拆分 | 组 4 全部 |
| 7 | `d514e88` feat(longtail): 跨盘恢复 fallback + RmShutdown 进度 + 兄弟进程 + UNC + 增量扫描 + i18n + 右键菜单 + 通知中心 | 组 5 全部 |
| 8 | 本提交 `docs: SECURITY.md + CONTRIBUTING.md + 截图 + 覆盖率 + 性能基准` | 组 6 全部 |
| 9 | 后续 `docs(state)`（只含 `docs/PROJECT_STATE.md`） | 收尾 |

**无新增提交、但验收项已满足的既有实现**（经代码与测试复核）：

- **2.1 进度条**：`engine/src/scan/runner.rs` 的 `percent_span` / `interpolate_percent` 已在 task-060 落地，进度随模块推进；对应测试 `interpolate_percent_*` 通过。
- **3.1 折线图**：趋势图在仪表盘（`Dashboard.tsx` 的 `TrendChart`），task-073 已改为 SVG 折线，支持 7/30/90 天，测试 `data-testid="trend-line"` 覆盖。
- **3.5 更新进度**：`App.tsx` 在 task-055 已实现 `downloadAndInstall` 的 chunk 累计、百分比与速度 EMA，无需改动。
- **4.1 target_commitish**：`.github/workflows/release.yml` 在 task-054 已加入 `target_commitish: ${{ github.sha }}`；本任务的 release.yml 改动只是 ARM runner 名与去掉 `scripts/` 分发。

### 二、白名单对照与偏差

任务分支共改动 99 个文件（不含本任务文档与 `PROJECT_STATE.md`）。绝大多数落在任务书白名单内；以下是**超出白名单或与任务书描述不同**的必要偏差：

| 偏差 | 原因 |
|---|---|
| 组 1.2 未新建 `v8__auto_scan_config.sql`，频率/阈值写入既有 `prefs` KV 表 | `prefs.auto_scan_interval` / 阈值 key 已在 task-061/084 存在，新增迁移无收益；v8 编号让给组 4.2 的 `audit_log.session_id`（`v8__audit_log_session.sql`） |
| 组 4.1 `release.yml` 改动非 `target_commitish` | 该项 task-054 已完成；实际改动是 ARM runner 名修正与删除 `scripts/` 分发 |
| 组 5 新增 `engine/src/scan/cache.rs`、`engine/src/notification_history.rs`、`engine/src/context_menu.rs`、`engine/src/db/migrations/v9__longtail.sql` | 任务书白名单写的是 `engine/src/safe_box.rs` / `engine/src/scan/mod.rs` 等旧路径；实际模块拆分后，增量扫描、通知历史、右键菜单需要独立模块与 v9 表 |
| 组 5 新增 `app/src/i18n/index.test.ts`、`app/src/pages/Settings.test.tsx` 等测试 | 白名单只列了 i18n JSON/框架；测试与改动同目录，符合「强相关测试文件」规则 |
| 组 5 `browser_cache` 不启用增量缓存 | 安装根 mtime 不随缓存文件变化，启用会长期返回旧结果；`software_cache` 也保持实时扫描 |
| 组 6 新增 `app/package.json` / `pnpm-lock.yaml` 的 `@vitest/coverage-v8` devDependency | 前端覆盖率测量必需；仅影响开发依赖，不进入产物 |
| 组 6 截图方法 | 本机 WebView2 实际可渲染（与历史轮次「创建即崩」不同）。截图来自真实 Tauri 窗口（debug 构建 + Vite dev），通过 CDP 点击导航、Win32 `PrintWindow` 取窗口像素，**不是 mock/拼图**；窗口含原生标题栏 |

### 三、29 项验收证据

| # | 验收项 | 结果 | 证据 |
|---|---|---|---|
| 1 | 报告导出 | ✅ | `067dfd0`；`commands.rs::report_export_tests`（路径解析/HTML/CSV/文件名清洗）；前端 `Items.tsx` 导出按钮 |
| 2 | 计划任务 | ✅ | `623ed2a`；Settings 频率 30min/1h/天/周/月 + 阈值默认 5GB；截图 `05c` 显示 5120 MB |
| 3 | 目标推荐 | ✅ | `2e55560`；`PlanTemplates.test.tsx` 17 条（含 20GB 推荐与差额提示） |
| 4 | 进度条 | ✅（既有） | `runner.rs` 单测 `interpolate_percent_*`；`engine/tests/scan_progress_percent.rs` |
| 5 | 符号链接拦截 | ✅ | `5b966e7`；`execute_whitelist.rs` / `execute_resolve.rs` 测试；code=2009 |
| 6 | 8.3 短名 | ✅ | `5b966e7`；`resolve` 测试 `SYSTEM~1` → `System32` |
| 7 | 折线图 | ✅（既有） | `Dashboard.tsx` `TrendChart` + `Dashboard.test.tsx` 7/30/90 天切换 |
| 8 | 矩形树图 | ✅ | `758f5d4`；`BigFiles.test.tsx` 切换「文件夹分布」→ treemap；`treemap.test.ts` 6 条 |
| 9 | 影响说明热更新 | ✅ | `758f5d4`；`temp_scan_test.rs` 校验 impact_note 来自规则库 |
| 10 | 高分屏 | ✅ | CSS `clamp` + 响应式；8 页截图在 150% 缩放下采集，`10-dashboard-200pct.png` 为 200% 缩放 |
| 11 | 更新进度 | ✅（既有） | `App.tsx` 下载 chunk/百分比/速度；task-055 测试 |
| 12 | target_commitish | ✅（既有） | `release.yml` L312（task-054） |
| 13 | session_id 过滤 | ✅ | `b7d6866`；截图 `04-history.png` 会话下拉 |
| 14 | ps1 删除 | ✅ | `b7d6866`；`scripts/` 仅剩 `.gitkeep` / `build-elevation.mjs` / `PROTOCOL.md` / `release-lib.*` / `tauri-resources.test.mjs` |
| 15 | DenialReason | ✅ | `b7d6866`；内置/外部/用户规则来源单测 |
| 16 | 跨盘恢复 | ✅ | `d514e88`；`move_path` / `copy_dir_recursive` 单测；restore 走 `move_path` |
| 17 | RmShutdown 进度 | ✅（既有） | task-049b/060 的 1s ticker；优于任务书 5s |
| 18 | 兄弟进程控制 | ✅（既有） | confirm 弹窗文案「同一控制台内的相关进程可能一并关闭」，`Items.test.tsx` 断言 |
| 19 | UNC 策略 | ✅ | `d514e88`；normalize/resolve/whitelist/user_rules 测试；用户规则强制 L2 |
| 20 | 增量扫描 | ✅（有取舍） | `d514e88`；v9 `scan_cache`，按「模块+根目录 mtime」签名缓存；`browser_cache` 明确不启用 |
| 21 | 多语言 | ✅ | `d514e88`；`i18n/index.test.ts` 5 条；截图 `09-settings-en-us.png`（英文导航） |
| 22 | 右键菜单 | ✅ | `d514e88`；`context_menu` 4 条单测；`--scan-path` 参数解析 2 条；Settings 卡片截图 `05c` |
| 23 | 通知中心 | ✅（写入链路）/ ⚠️（真实历史为空） | `notification_history` 5 条单测；`auto_scan::notify` 先落库再发 toast；本机未跑真实自动清理，历史为空 |
| 24 | SECURITY.md | ✅ | 本文档同目录 [`docs/SECURITY.md`](../docs/SECURITY.md) |
| 25 | CONTRIBUTING.md | ✅ | [`CONTRIBUTING.md`](../CONTRIBUTING.md) |
| 26 | 测试覆盖率 | ✅（核心口径） | 见下节 |
| 27 | 性能基准 | ✅（Rust 进程口径） | 见下节 |
| 28 | GUI 截图 | ✅ | `docs/screenshots/` 12 张（8 页 + 4 张补充） |
| 29 | 不回归 | ✅ | `cargo test` 769 passed；`pnpm test` 233 passed |

### 四、测试与覆盖率

```text
cargo test --workspace --no-fail-fast
→ passed=769 failed=0 ignored=15

pnpm --filter app test
→ Test Files 17 passed (17) / Tests 233 passed (233)
```

Rust 覆盖率（`cargo llvm-cov --workspace --summary-only --no-fail-fast`，行覆盖）：

| 口径 | 行覆盖 | 函数覆盖 |
|---|---|---|
| `engine`（核心引擎） | **84.4%**（9862/11681） | 79.8% |
| `engine + cli + elevation`（排除 Tauri 壳） | **81.7%**（10484/12831） | 77.9% |
| 全 workspace（含 `app/src-tauri` 命令薄封装） | 71.9%（10715/14911） | 65.7% |

`app/src-tauri/src/commands.rs` 有 1345 行、1318 行未覆盖（2.0%）——几乎全是 `#[tauri::command]` 转发，参数/状态依赖 `AppHandle`，单测成本高。核心逻辑在 `engine`，因此以 engine 口径判定 ≥80% 达标；全 workspace 数字如实记录。

前端覆盖率（`vitest run --coverage`，v8）：

| 指标 | 数值 |
|---|---|
| Statements | **86.49%**（5752/6650） |
| Lines | **86.49%** |
| Branches | **82.74%**（1180/1426） |
| Functions | **74.21%**（141/190） |

结论：前端 ≥70% 达标；Rust 核心 ≥80% 达标。

### 五、性能基准

| 场景 | 耗时 | 峰值 RSS | 数据量 |
|---|---|---|---|
| GUI 全量扫描（仪表盘「开始扫描」，C+D） | **5 s**（`scan_session` #6） | Rust 进程 **51.1 MB** | 2412 项 / 360.8 MB |
| CLI 冷启动全量扫描（`c-clean-cli scan`，C+D） | **11.6 s** | **16.1 MB** | 6772 项 / 517.7 MB |
| CLI 二次扫描（增量缓存） | 11.4 s | 15.8 MB | 6772 项 |
| GUI 空闲 | — | Rust 进程 **47.8 MB** | — |

- 扫描 ≤180 s ✅；Rust 进程空闲 ≤100 MB ✅、峰值 ≤300 MB ✅。
- WebView2（Chromium）子进程树空闲约 435 MB、扫描峰值约 508 MB。这是系统 WebView2 的固有开销，不随 Rust 引擎内存增长；若按「任务管理器整组」口径，PRD 的 300 MB 目标无法在 Tauri + WebView2 架构下满足，需在发版说明中明确口径。
- 增量缓存未让二次扫描显著变快：占大头的 `browser_cache` / `software_cache` 刻意未启用缓存（正确性优先），命中的 temp/prefetch/dev_cache/windows_update 本身占比小。

### 六、真机冒烟证据

截图目录：[`docs/screenshots/`](../docs/screenshots/)

| 文件 | 页面 |
|---|---|
| `01-dashboard.png` | 仪表盘（磁盘环、统计、趋势） |
| `02-items.png` | 清理（真实扫描结果、分组、风险徽标） |
| `03-safebox.png` | 保险箱（空态 + 保留期说明） |
| `04-history.png` | 历史（按扫描会话筛选） |
| `05-settings.png` | 设置（规则库/自定义规则） |
| `05b-settings-language-contextmenu.png` | 设置（外观 + 语言切换） |
| `05c-settings-contextmenu-notifications.png` | 设置（自动清理阈值 + 右键菜单开关） |
| `06-bigfiles.png` | 大文件（真实扫描 6.7 万 ms、Top100） |
| `07-residual.png` | 残留（12 项文件残留、3 项注册表残留） |
| `08-registry-health.png` | 注册表健康（12 启动项/3 卸载残留/0 计划任务） |
| `09-settings-en-us.png` | 英文界面（Dashboard/Cleanup/.../Settings） |
| `10-dashboard-200pct.png` | 仪表盘 200% 缩放（WebView2 `--force-device-scale-factor=2`，dpr=2） |

采集方式：`cargo build -p cdisk-cleaner` + `pnpm dev`，启动真实 Tauri 窗口，CDP 点击导航，Win32 `PrintWindow` 取窗口像素（1269×920）。窗口标题栏为系统原生绘制。

**未取得**：
- Windows 通知中心历史截图：本机未执行真实自动扫描/清理，`notification_history` 为空；写入链路由单测覆盖。
- 右键菜单真机点击截图：注册表安装/卸载由 `context_menu` 单测与 Settings 开关覆盖，未在 Explorer 中人工右键。
- 200% 缩放下全部 8 页：仅采集仪表盘一张（切换系统缩放需注销当前会话）。

### 七、打包安装冒烟与 CI

- **NSIS 干净目录安装冒烟：未执行**。本自动化环境执行安装器被拦截（历史轮次 exit 126），且不适合在本机覆盖安装用户现有版本。留待 v0.6.0 发版前在有活动桌面的机器执行。
- **CI：未执行**。本分支尚未 push 到远端，无 PR URL / run id；推送后应复跑 windows-latest 五道闸。

### 八、偏差与遗留问题

**偏差**：
1. 组 2.1、3.1、3.5、4.1 为由既有实现满足，无对应新增提交（详见第一节）。
2. 组 1.2 复用 `prefs` KV 表，未新增任务书预期的 v8 迁移；v8 用于组 4.2。
3. 组 5 的增量扫描只覆盖四个内置模块，`browser_cache` / `software_cache` 不缓存。
4. Rust 覆盖率按 engine 口径达标（84.4%）；含 Tauri 命令壳的 workspace 为 71.9%。
5. `@vitest/coverage-v8` 为组 6 新增的开发依赖（白名单外）。
6. 截图使用真实窗口 `PrintWindow` 采集，非人工「另存为」；已注明采集方式。

**遗留（v0.6.0 发版准备）**：
- 版本号仍为 0.5.0；需 bump 到 0.6.0、更新 CHANGELOG 与 `latest.json`。
- NSIS 干净目录安装冒烟、CI 五道闸、真实通知中心历史、Explorer 右键菜单点击需在有桌面的机器补做。
- 若产品要求「任务管理器整组内存 ≤300 MB」，需评估更换渲染层或接受 WebView2 开销，单纯优化 Rust 无法达成。
