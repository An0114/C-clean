# 任务编号：084 v0.5.0 收尾大合并（批次五大文件地图 + 批次六决策补全 + 批次七 P2 长尾）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：v0.5.0 能力线 backlog，用户拍板「批次五 + 批次六 + 批次七 12 项不拆分，打包成一个任务派发」。
> 上游依赖：task-083 已合并 main（merge `680eac1`，软件感知地基 8 项完成）。
> **版本线**：v0.5.0 收尾（本任务完成后 v0.5.0 发版）。
> **本任务范围（12 个 backlog 项打包，分 3 阶段）**：
> - **批次五（大文件地图与差异化，4 项）**：096 目标空间 / 097 重复文件 / 098 文件夹分布 / 099 残留一键定位
> - **批次六（决策与 P1/P2 补全，5 项）**：100 自动清理拍板 / 101 L0 拍板 / 102 非系统盘产出 / 103 规则导入导出 / 104 Gradle&Maven
> - **批次七（P2 长尾，3 项）**：105 多用户 Profile / 106 草稿暂存 / 107 ARM64 产物
> **为什么合并**：用户明确拍板「批次五六七打包成一个任务」；这是 v0.5.0 收官任务，完成后直接发版 v0.5.0。
> **不做**：跨盘恢复 fallback / RmShutdown 三功能（维持现状）；代码签名 / 品牌图标（死路）；G6（设置页首调红错，豁免）。

## 状态字段

pending

## 任务目标

### 阶段 A：批次五 大文件地图与差异化（096~099）

#### A1. 目标空间扫描（096）

- **现状**：`engine/src/scan/big_files.rs` 只扫 hiberfil.sys / Windows.old / 回收站 / Top100。
- **修法**：
  - 新增「目标空间」类别：
    - `%USERPROFILE%\Downloads`（下载目录，按大小倒序 Top20）；
    - `%USERPROFILE%\Documents` 大文件（>100MB）；
    - `%USERPROFILE%\Desktop` 大文件（>100MB）；
    - `%LOCALAPPDATA%\Temp` 大文件（>50MB）；
  - 所有目标空间只读不删（沿用 E 模块定位）；
  - 结果按 category 分组返回前端。

#### A2. 重复文件检测（097）

- **现状**：无重复文件检测。
- **修法**：
  - 新建 `engine/src/scan/modules/dup_files.rs`：
    - 扫描用户目录（Downloads/Documents/Desktop）下 >10MB 文件；
    - 按大小分组 → 同大小文件算 SHA-256 → 同哈希判定重复；
    - 每个重复组返回「保留一个 + 其余标记可清理」（保守策略，UI 上标注「保留最新修改时间的那份」）；
    - 不自动删，只报告。
  - 性能：只对 >10MB 文件算哈希，避免小文件风暴。

#### A3. 文件夹分布（098）

- **现状**：大文件地图只列单个文件，不告诉你「哪个文件夹占最多」。
- **修法**：
  - 新建 `engine/src/scan/modules/folder_stats.rs`：
    - 对用户目录（Downloads/Documents/Desktop/AppData）做文件夹大小递归统计；
    - 输出 Top20 文件夹（按总大小倒序）；
    - 用 walkdir 并行递归（rayon）；
  - 前端 BigFiles 页加「文件夹分布」tab。

#### A4. 残留一键定位（099）

- **现状**：`engine/src/scan/residual.rs` 检测残留（注册表卸载列表 ↔ 文件目录交叉比对），但 UI 上只能看到「有残留」，不能「打开该目录」。
- **修法**：
  - 新增 IPC 命令 `residual_locate(path: String)`：
    - 用 `explorer.exe /select,<path>` 打开资源管理器并选中该文件/目录；
    - 错误码 2008（路径不存在）；
  - 前端 Residual 页每项加「定位」按钮（文件夹图标）。

### 阶段 B：批次六 决策与 P1/P2 补全（100~104）

#### B1. 自动清理拍板（100）

- **现状**：task-061 已做自动扫描（30 分钟轮询 + 只扫不删 + 系统通知），但没有「自动清理 L1」开关。
- **修法**：
  - Settings 页新增「自动清理 L1 安全项」开关：
    - 关闭（默认）：只扫不删；
    - 开启：扫描完成后自动删除 L1 项（L2/L3 不动）；
  - 后端 `auto_scan.rs` 加 `auto_clean_l1: bool` 配置；
  - 自动清理前必须先建系统还原点（task-062 已做）；
  - 清理结果发系统通知（「已自动清理 X MB」）。

#### B2. L0 拍板（101）

- **现状**：风险分级 L0/L1/L2/L3 已做，但 L0（完全无风险的系统临时文件）没有「全部一键清理」入口。
- **修法**：
  - Items 页顶部加「一键清理 L0 安全项」按钮：
    - L0 = Temp 目录 >7 天 / prefetch >30 天 / Windows Update 下载缓存；
    - 点击后批量选中所有 L0 项并执行删除；
  - 与现有「全选 L1」（task-064）并列。

#### B3. 非系统盘产出（102）

- **现状**：task-077 已做多盘扫描（GetLogicalDrives），但大文件地图 / 残留扫描只在 C 盘跑。
- **修法**：
  - 大文件地图 / 文件夹分布 / 重复文件检测扩展到所有固定盘（D:/E:/...）；
  - 结果带 drive_letter 字段；
  - UI 按盘分组展示。

#### B4. 规则导入导出（103）

- **现状**：task-078 已做用户自定义规则（v4 user_rules 表），但没有导入/导出。
- **修法**：
  - 新增 IPC 命令 `rules_export()` → 返回 JSON 字符串（user_rules 全部）；
  - 新增 IPC 命令 `rules_import(json: String)` → 校验 + 写入 user_rules 表（冲突按「导入时间最新」覆盖）；
  - Settings 页加「导出规则」「导入规则」按钮（文件选择对话框）。

#### B5. Gradle & Maven 缓存识别（104）

- **现状**：软件感知（task-083）已做 VSCode/Slack 等，但开发者的 Gradle/Maven 缓存没识别。
- **修法**：
  - `rules/software_rules.json` 新增两条：
    - Gradle：`%USERPROFILE%\.gradle\caches`（L1，可安全清理，下次构建自动重下）；
    - Maven：`%USERPROFILE%\.m2\repository`（L2，需确认，本地 jar 重下慢）；
  - 无需新代码，规则库加条目即可（task-083 已打通软件规则加载）。

### 阶段 C：批次七 P2 长尾（105~107）

#### C1. 多用户 Profile（105）

- **现状**：task-083 已做 Chrome/Edge/Firefox 多 Profile（同一用户下），但没有跨用户（C:\Users\Alice / C:\Users\Bob）。
- **修法**：
  - 扫描时枚举 `C:\Users\*` 下所有有桌面的用户目录；
  - 对每个用户跑用户级缓存扫描（Temp/AppData/Browser）；
  - 结果带 `username` 字段；
  - **安全护栏**：只能扫当前用户 + 其他用户的公开目录（不能读其他用户的私有 AppData），权限不足时跳过并记录。

#### C2. 草稿暂存（106）

- **现状**：用户选中待清理项后，切换 tab 或关闭 app，选择丢失。
- **修法**：
  - 新增 `engine/src/db/migrations/v7__draft.sql`：`draft_selections` 表（id / scan_id / item_path / selected_at）；
  - 用户勾选时即时写入草稿表；
  - 重新打开 app 时自动恢复上次勾选；
  - 清理完成或手动取消后清空草稿。

#### C3. ARM64 产物（107）

- **现状**：CI 只打 windows-latest（x64）。
- **修法**：
  - `.github/workflows/release.yml` 加 ARM64 matrix：
    - `platform: [windows-latest, windows-11-arm]`（GitHub 托管 Windows arm64 标准 runner 的内置标签为 `windows-11-arm`，不存在 `windows-11-arm64`；VS2026 镜像为 `windows-11-vs2026-arm`）；
    - Tauri 配置 `tauri.conf.json` 加 target `aarch64-pc-windows-msvc`；
  - 发版产物多一个 `C-clean_0.5.0_arm64-setup.exe`；
  - **不签代码**（死路，SmartScreen 警告可接受）。

**本任务不做**：
- 不做跨盘恢复 fallback（维持现状）；
- 不做 RmShutdown 三功能（维持现状）；
- 不做代码签名 / 品牌图标（死路）；
- 不做 G6（豁免）。

## 前置条件

1. `main` 已合并 task-083（merge `680eac1`），软件感知 8 项完成；
2. 多盘扫描（task-077）、用户自定义规则（task-078）、便携版+CLI（task-079）已就位；
3. 大文件地图模块 `engine/src/scan/big_files.rs` 已存在；
4. 残留扫描模块 `engine/src/scan/residual.rs` 已存在；
5. 自动扫描 `app/src-tauri/src/auto_scan.rs` 已存在；
6. 关键锚点（必须全部在白名单内）：
   - 大文件深化：`engine/src/scan/big_files.rs`（扩展目标空间）
   - 新建模块：`engine/src/scan/modules/dup_files.rs` / `folder_stats.rs`
   - 残留定位：`engine/src/scan/residual.rs` + 新建 IPC `residual_locate`
   - 决策补全：`app/src-tauri/src/auto_scan.rs` + `app/src/pages/Settings.tsx` + `app/src/pages/Items.tsx`
   - 非系统盘：`engine/src/scan/big_files.rs` + `residual.rs`（按 drive_letter 扩展）
   - 规则 IO：新建 `engine/src/ipc/commands/rules_io.rs` + Settings UI
   - Gradle&Maven：`rules/software_rules.json` 加条目
   - 多用户：`engine/src/scan/orchestrator.rs`（枚举 C:\Users\*）
   - 草稿暂存：`engine/src/db/migrations/v7__draft.sql` + 前端勾选持久化
   - ARM64：`.github/workflows/release.yml` + `app/src-tauri/tauri.conf.json`
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 阶段 A：大文件地图

1. **A1. 目标空间**：Downloads/Documents/Desktop/Temp 大文件按 category 返回；
2. **A2. 重复文件**：>10MB 文件 SHA-256 去重，每组保留最新修改时间的那份；
3. **A2. 单测**：构造两个同内容文件，断言重复组识别 ≥2 条；
4. **A3. 文件夹分布**：Top20 文件夹按大小倒序；
5. **A3. 单测**：构造目录树，断言 Top 排序 ≥2 条；
6. **A4. 残留定位**：`residual_locate` 命令调起 explorer /select；路径不存在返回 2008；
7. **A4. 前端**：Residual 页「定位」按钮点击后资源管理器正确弹出。

### 阶段 B：决策补全

8. **B1. 自动清理 L1**：Settings 开关开启后，自动扫描完成自动删 L1；清理前建还原点；结果发通知；
9. **B2. L0 一键清理**：Items 页顶部「一键清理 L0」按钮批量选中并删除；
10. **B3. 非系统盘产出**：大文件 / 文件夹分布 / 重复文件扩展到所有固定盘；UI 按盘分组；
11. **B4. 规则导入导出**：导出 JSON 文件可下载；导入 JSON 校验通过后写入 user_rules；
12. **B5. Gradle&Maven**：rules/software_rules.json 含两条新规则；真机扫到 ~/.gradle/caches（如存在）；
13. **B4. 单测**：导出 JSON 序列化 + 导入校验 ≥3 条全绿。

### 阶段 C：P2 长尾

14. **C1. 多用户 Profile**：枚举 C:\Users\*；权限不足跳过并记录；结果带 username；
15. **C2. 草稿暂存**：勾选写入 draft_selections 表；重开 app 自动恢复；清理后清空；
16. **C2. 单测**：草稿写入 + 恢复 + 清空 ≥3 条全绿；
17. **C3. ARM64 产物**：CI release workflow matrix 含 arm64；release 产物多一个 arm64 exe；
18. **C3. 不回归**：x64 产物仍正常打出。

### 整体

19. **后端单测**：`cargo test --workspace` 全绿（现有 653 + 新增 ≥15，≥668）；
20. **前端单测**：`pnpm --filter app test` 全绿（现有 209 + 新增 ≥5，≥214）；
21. **Lint**：`cargo clippy -D warnings` + `pnpm lint` 0 error；
22. **构建**：`cargo build --release`（x64）成功；`pnpm build` 成功。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。

**Rust 扫描（阶段 A）**：
- `engine/src/scan/big_files.rs`（扩展目标空间 + 多盘）
- `engine/src/scan/modules/mod.rs`（注册新模块）
- `engine/src/scan/modules/dup_files.rs`（**新建**，A2）
- `engine/src/scan/modules/folder_stats.rs`（**新建**，A3）
- `engine/src/scan/residual.rs`（多盘扩展）
- `engine/src/scan/orchestrator.rs`（接线新模块 + 多用户枚举 C1）

**Rust IPC（阶段 A + B）**：
- `engine/src/ipc/commands/mod.rs`（注册新命令）
- `engine/src/ipc/commands/big_files.rs`（扩展目标空间 + 文件夹分布 + 重复文件）
- `engine/src/ipc/commands/residual.rs`（**新建**或扩展，A4 定位命令）
- `engine/src/ipc/commands/rules_io.rs`（**新建**，B4 导入导出）
- `engine/src/ipc/commands/draft.rs`（**新建**，C2 草稿读写）

**Rust Tauri 接线（阶段 B + C）**：
- `app/src-tauri/src/lib.rs`（注册所有新命令）
- `app/src-tauri/src/commands.rs`（注册 handler）
- `app/src-tauri/src/auto_scan.rs`（B1 自动清理 L1 逻辑）

**Rust 数据库迁移（阶段 C）**：
- `engine/src/db/migrations/v6__bigfiles_v2.sql`（**新建**，目标空间 + 重复文件表）
- `engine/src/db/migrations/v7__draft.sql`（**新建**，草稿暂存表）

**规则库（阶段 B）**：
- `rules/software_rules.json`（B5 加 Gradle/Maven 两条）

**前端（阶段 A + B）**：
- `app/src/pages/BigFiles.tsx`（目标空间 tab + 文件夹分布 tab + 重复文件 tab）
- `app/src/pages/Residual.tsx`（定位按钮）
- `app/src/pages/Items.tsx`（L0 一键清理按钮）
- `app/src/pages/Settings.tsx`（自动清理 L1 开关 + 规则导入导出按钮）
- `app/src/utils/invoke.ts`（新命令封装：residual_locate / rules_export / rules_import / draft_*）
- `app/src/App.tsx`（**仅在需要时**加 tab；若沿用现有 tab 则不动）

**CI / 打包（阶段 C）**：
- `.github/workflows/release.yml`（C3 ARM64 matrix）
- `app/src-tauri/tauri.conf.json`（C3 加 target）

**测试**：
- `engine/src/scan/modules/dup_files_test.rs`（**新建**）
- `engine/src/scan/modules/folder_stats_test.rs`（**新建**）
- `engine/src/ipc/commands/rules_io_test.rs`（**新建**）
- `engine/src/db/migrations/draft_test.rs`（**新建**）
- `app/src/pages/__tests__/BigFiles.test.tsx`（新 tab 渲染）
- `app/src/pages/__tests__/Settings.test.tsx`（自动清理开关 + 导入导出按钮）

**文档**：
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

**不修改**：
- engine/src/scan/software_enum.rs / software_cache.rs（task-083 已完成，不改动）
- engine/src/scan/winsxs.rs（DISM 逻辑不变）
- 托盘菜单（task-082 已完成）
- 主题 CSS 变量体系（task-061/081/082 已建立）
- cli/src/main.rs（CLI 不改动，若需新子命令留后续）
- 跨盘恢复 fallback / RmShutdown 三功能（维持现状）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-084-v050-wrapup-batch567`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-084-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 12 个 backlog 项分 6 个原子提交（阶段合并）：

1. `feat(bigfiles): 目标空间 + 重复文件检测 + 文件夹分布（批次五 A1/A2/A3） (#task-084)`
   —— 含 big_files.rs + dup_files.rs + folder_stats.rs + IPC + BigFiles.tsx 新 tab + v6 迁移。
2. `feat(residual): 残留一键定位 + 多盘扩展（A4 + B3 残留部分） (#task-084)`
   —— 含 residual.rs + IPC residual_locate + Residual.tsx 定位按钮。
3. `feat(decision): 自动清理 L1 拍板 + L0 一键清理（B1 + B2） (#task-084)`
   —— 含 auto_scan.rs + Settings.tsx 开关 + Items.tsx L0 按钮。
4. `feat(rules): 规则导入导出 + Gradle/Maven 缓存识别（B4 + B5） (#task-084)`
   —— 含 rules_io.rs + Settings.tsx 导入导出按钮 + software_rules.json 两条。
5. `feat(crossdrive): 非系统盘大文件产出（B3 大文件部分） (#task-084)`
   —— 含 big_files.rs + folder_stats.rs + dup_files.rs 多盘扩展 + UI 按盘分组。
6. `feat(longtail): 多用户 Profile + 草稿暂存 + ARM64 产物（C1 + C2 + C3） (#task-084)`
   —— 含 orchestrator.rs 多用户枚举 + v7 草稿迁移 + draft IPC + 前端勾选持久化 + release.yml matrix + tauri.conf.json。
7. 自验全绿后，最后一次提交：`docs(state): 任务084完成，v0.5.0 收尾大合并（批次五六七 12 项） (#task-084)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 096~107: ...`；
   - 「最近一次变更」追加一段，含 12 项摘要 + commit hash；
   - 「下一步计划」改为「v0.5.0 全部 backlog（082~107）完成；下一步：v0.5.0 发版准备（参照 task-080 release-prep 模式）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-09  task-084  v0.5.0 收尾大合并（批次五六七 12 项）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0（≥668 passed）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0（≥214 passed）；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：重复文件 SHA-256 / 文件夹分布排序 / 规则导入导出序列化 / 草稿读写，本任务新增 ≥15 条；
- **集成测试（必做）**：`cargo test --workspace` 全绿（现有 653 + 新增 ≥15）；
- **端到端（必做）**：
  - 大文件地图 4 个 tab（基础 / 目标空间 / 重复文件 / 文件夹分布）全部渲染；
  - 残留页「定位」按钮点击后资源管理器弹出并选中；
  - Settings 页「自动清理 L1」开关开启后，自动扫描完成自动删 L1；
  - Items 页「一键清理 L0」按钮批量删除 L0；
  - 规则导出 → 得到 JSON 文件；导入 → 校验通过写入；
  - 勾选后关闭 app 重开，选择恢复；
- **真机冒烟（必做、不可裁剪）**：
  - 大文件地图多盘（如有 D 盘）按盘分组；
  - 多用户枚举（如系统有其他用户）正确跳过私有目录；
  - CDP 抓 console 0 红错（G6 已知项除外）；
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，端到端 6 条 + 真机 3 条复跑；
- **CI（必做）**：PR 触发 windows-latest 五道闸全绿；tag push 触发 release workflow（含 ARM64 matrix）。

> 本任务命中「前端任务判定清单」（4 个新 tab / 定位按钮 / L0 按钮 / 设置开关 / 导入导出对话框 / 草稿恢复，全部真机可观察），且命中「从 DB 字段派生 UI」（重复文件 / 文件夹大小 / 草稿勾选 都是 DB→UI），**真机/打包冒烟必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 目标空间 | 真机截图：Downloads/Documents/Desktop 大文件列表 |
| 重复文件 | 真机证据：构造两个同内容文件，重复组识别 |
| 文件夹分布 | 真机截图：Top20 文件夹按大小倒序 |
| 残留定位 | 真机证据：点击定位按钮 → explorer /select 弹出 |
| 自动清理 L1 | 真机证据：Settings 开关 + 自动扫描后 L1 被删 + 系统通知 |
| L0 一键清理 | 真机证据：点击按钮 → L0 项批量删除 |
| 非系统盘 | 真机证据：D 盘大文件按盘分组显示 |
| 规则导入导出 | 真机证据：导出 JSON 文件路径 + 导入后 user_rules 新增行 |
| Gradle/Maven | 真机证据：~/.gradle/caches 被识别（如存在） |
| 多用户 | 真机证据：C:\Users\* 枚举日志（权限跳过其他用户） |
| 草稿暂存 | 真机证据：勾选 → 关闭 app → 重开 → 选择恢复 |
| ARM64 产物 | release workflow run URL + arm64 exe 产物名 |
| 不回归 | `cargo test --workspace` 668+ 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 9 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 6 个代码提交 + 1 个 `docs(state)` 提交，共 7 个 commit，全部 push 到 `feature/task-084-v050-wrapup-batch567`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - 目标空间 4 个目录实际扫描结果（真机文件数 / 总大小）；
   - 重复文件检测阈值（10MB）+ 实际命中组数；
   - 文件夹分布 Top20 实际数据；
   - 残留定位命令实际调起 explorer 截图；
   - 自动清理 L1 开关实际行为（还原点 + 通知）；
   - L0 一键清理实际删除字节数；
   - 多盘（如有 D 盘）分组实际效果；
   - 规则导出 JSON 实际文件路径；
   - Gradle/Maven 真机命中（如本机装有）；
   - 多用户枚举实际跳过的用户列表；
   - 草稿暂存实际恢复效果；
   - ARM64 产物名 + release run URL；
   - `cargo test --workspace` 通过输出（含新增 ≥15 条，总数 ≥668）；
   - `pnpm test` 输出（无回归 + 新增 ≥5，总数 ≥214）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（16 条表格逐项）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（v0.5.0 全部 backlog 082~107 完成；下一步：v0.5.0 发版准备任务）；
3. `docs/PROJECT_STATE.md` 已按第 7 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务084-v0.5.0收尾大合并（批次五六七 12 项） (#task-084)`，
   该提交只含 `tasks/084-v050-wrapup-batch567.md` 一个文件，落在 `feature/task-084-v050-wrapup-batch567` 分支最前端。
