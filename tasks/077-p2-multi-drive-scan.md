# 任务编号：077 P2 起步——多盘扫描支持（C 盘 → 所有固定磁盘）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：P2 未做清单第一项（多盘扫描）。
> 上游依赖：task-076 已合并 main（merge `b4f40df`，R5 架构债全部收官）。
> **版本线**：v0.4.x P2 起步——多盘扫描。
> **本任务范围（4 个功能单元）**：磁盘枚举（Windows API 检测所有固定盘）+ 扫描引擎多盘接线 + DB/IPC 盘标识透传 + 前端多盘 UI（Dashboard 环形图多盘 + Items 按盘筛选）。
> **为什么合并**：多盘扫描是完整闭环（后端枚举 → 扫描接线 → IPC 透传 → 前端 UI），一次 PR 完成。
> **不做**：用户自定义规则 / 便携版 CLI（留 task-078+）；跨盘恢复 fallback（维持现状）。

## 状态字段

pending

## 任务目标

### 1. 磁盘枚举（Windows API）

- **现状**：扫描硬编码 C 盘（`%TEMP%` / `C:\Windows\*`）。
- **修法**：
  - `engine/src/scan/drive_enum.rs`（新建）：
    - 调用 Windows API `GetLogicalDrives()` + `GetDriveTypeW()`；
    - 筛选固定磁盘（`DRIVE_FIXED`），排除可移动 / CD-ROM / 网络盘；
    - 返回 `Vec<DriveInfo { letter: char, total_bytes: u64, free_bytes: u64 }>`；
    - `#[cfg(windows)]` 条件编译（非 Windows 返回空）。

### 2. 扫描引擎多盘接线

- **现状**：`engine/src/scan/orchestrator.rs` 对单盘执行扫描。
- **修法**：
  - `engine/src/scan/orchestrator.rs`：
    - 先枚举所有固定盘；
    - 对每个盘执行扫描（temp / prefetch / browser_cache / dev_cache / windows_update 模块，路径按盘替换）；
    - 进度按「盘 × 模块」二维推进（如 `[2/5] D: browser_cache`）；
    - 扫描结果按盘分组返回。
  - 注意：Prefetch / Windows Update 模块只在 C 盘执行（系统目录固定），其他模块按盘执行。

### 3. DB / IPC 盘标识透传

- **现状**：items 表 path 字段已含盘符，但未按盘索引。
- **修法**：
  - `engine/src/db/migrations/v2__drive_letter.sql`（新建）：
    - items 表加 `drive_letter TEXT` 字段（如 `C` / `D`）；
    - 加索引 `idx_items_drive_letter`；
  - `engine/src/ipc/envelope.rs`：
    - scan.start 入参加 `drives: Option<Vec<char>>`（默认 None = 全部固定盘）；
    - 进度事件加 `drive: char` 字段；
  - `engine/src/db/resolver.rs`：
    - 写入 items 时自动提取盘符。

### 4. 前端多盘 UI

- **修法**：
  - `app/src/pages/Dashboard.tsx`：
    - 环形图改为多盘并列（每个盘一个小环形图，显示已用/总容量）；
    - 扫描按钮点击后显示多盘进度（如 `扫描 D: ...`）；
  - `app/src/pages/Items.tsx`：
    - 加盘筛选下拉框（全部 / C: / D: / ...）；
    - 表格按盘分组显示。

**本任务不做**：
- 不做用户自定义规则（留 task-078）；
- 不做便携版 CLI（留 task-079）；
- 不做跨盘恢复 fallback（维持现状）。

## 前置条件

1. `main` 已合并 task-076（merge `b4f40df`），R5 全部收官；
2. 关键锚点（必须全部在白名单内）：
   - 后端新建：`engine/src/scan/drive_enum.rs`
   - 后端新建：`engine/src/db/migrations/v2__drive_letter.sql`
   - 后端修改：`engine/src/scan/orchestrator.rs`（多盘接线）
   - 后端修改：`engine/src/db/migrate.rs`（注册 v2 迁移）
   - 后端修改：`engine/src/db/migrations/v1__init.sql`（如需要，同步 baseline）
   - 后端修改：`engine/src/db/resolver.rs`（写入时提取盘符）
   - 后端修改：`engine/src/ipc/envelope.rs`（scan.start 入参 + 进度事件）
   - 后端修改：`engine/src/ipc/commands/scan_start.rs`（多盘支持）
   - 前端修改：`app/src/pages/Dashboard.tsx`（多盘环形图）
   - 前端修改：`app/src/pages/Items.tsx`（盘筛选）
   - 前端修改：`app/src/utils/invoke.ts`（API 类型，如需要）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 磁盘枚举

1. `drive_enum.rs` 正确枚举所有固定磁盘；
2. 返回 `DriveInfo { letter, total_bytes, free_bytes }`；
3. 非 Windows 平台返回空（条件编译）；
4. 单测：枚举结果包含 C 盘（Windows 环境）。

### 2. 扫描引擎多盘接线

5. `orchestrator.rs` 对所有固定盘执行扫描；
6. 进度按「盘 × 模块」二维推进；
7. Prefetch / Windows Update 只在 C 盘执行；
8. 单测：多盘扫描结果按盘分组。

### 3. DB / IPC 盘标识透传

9. v2 迁移成功（items 表加 `drive_letter` 字段 + 索引）；
10. 写入 items 时自动提取盘符；
11. scan.start 入参支持指定盘（默认全部）；
12. 进度事件含盘标识；
13. 单测：迁移 + 盘符提取正确。

### 4. 前端多盘 UI

14. Dashboard 环形图多盘并列显示；
15. Items 页盘筛选下拉框工作；
16. 扫描进度显示当前盘；
17. 单测：组件渲染正确。

### 后端单测

18. `cargo test --workspace` 全绿（现有 + 本任务新增 ≥5 条全绿）。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**后端新建**：
- `engine/src/scan/drive_enum.rs`（新建）
- `engine/src/db/migrations/v2__drive_letter.sql`（新建）

**后端修改**：
- `engine/src/scan/orchestrator.rs`（多盘接线）
- `engine/src/db/migrate.rs`（注册 v2 迁移）
- `engine/src/db/migrations/v1__init.sql`（如需要，同步 baseline）
- `engine/src/db/resolver.rs`（写入时提取盘符）
- `engine/src/ipc/envelope.rs`（scan.start 入参 + 进度事件）
- `engine/src/ipc/commands/scan_start.rs`（多盘支持）
- `engine/src/ipc/commands/mod.rs`（如需要，注册 drive_enum 命令）
- `engine/src/lib.rs`（如需要，注册 IPC handler）

**前端修改**：
- `app/src/pages/Dashboard.tsx`（多盘环形图）
- `app/src/pages/Items.tsx`（盘筛选下拉框）
- `app/src/utils/invoke.ts`（API 类型，如需要）

**文档**：
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-077-p2-multi-drive-scan`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-077-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `feat(scan): 磁盘枚举模块（GetLogicalDrives + GetDriveTypeW） (#task-077)`（单元 1）
   —— 含 `engine/src/scan/drive_enum.rs` + `commands/mod.rs` + `lib.rs`。
2. `feat(orchestrator): 多盘扫描接线（二维进度 + 按盘分组） (#task-077)`（单元 2）
   —— 含 `engine/src/scan/orchestrator.rs` + `ipc/envelope.rs` + `ipc/commands/scan_start.rs`。
3. `feat(db): v2 迁移 items 表加 drive_letter 字段 + 索引 (#task-077)`（单元 3）
   —— 含 `migrations/v2__drive_letter.sql` + `migrate.rs` + `resolver.rs`。
4. `feat(ui): 多盘 Dashboard 环形图 + Items 盘筛选 (#task-077)`（单元 4）
   —— 含 `app/src/pages/Dashboard.tsx` + `Items.tsx` + `utils/invoke.ts`。
5. 自验全绿后，最后一次提交：`docs(state): 任务077完成，P2 起步——多盘扫描支持就绪 (#task-077)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 077: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「P2 持续：用户自定义规则（task-078）/ 便携版 CLI（task-079）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-03  task-077  P2 起步（多盘扫描）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：磁盘枚举、多盘扫描接线、v2 迁移、盘符提取、前端组件。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **端到端写路径（必做，本任务硬要求）**：
  - 扫描 → DB 写入含 drive_letter → Items 页按盘筛选显示；
  - Dashboard 多盘环形图显示正确。
- **真机冒烟（必做、不可裁剪）**：
  - Dashboard 显示多盘环形图（如有 D 盘则并列显示）；
  - 扫描结果包含 D 盘项（如存在）；
  - Items 页盘筛选下拉框工作；
  - 进度事件显示当前盘；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 4 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（Dashboard 环形图 + Items 筛选，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 磁盘枚举 | 单测输出（固定盘列表） |
| 多盘扫描接线 | 单测输出（按盘分组结果） |
| v2 迁移 | 单测输出（drive_letter 字段存在） |
| Dashboard 多盘环形图 | 真机截图：Dashboard 显示多盘 |
| Items 盘筛选 | 真机证据：下拉框切换过滤结果 |
| 进度事件含盘 | 真机证据：进度条显示当前盘 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 4 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-077-p2-multi-drive-scan`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - 磁盘枚举 Windows API 调用方式；
   - 多盘扫描路径替换逻辑（temp/browser_cache/dev_cache 按盘，prefetch/windows_update 仅 C 盘）；
   - v2 迁移 SQL 语句；
   - 前端多盘 UI 实现方式；
   - `cargo test --workspace` 通过输出（含新增 ≥5 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（4 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（P2 持续：用户自定义规则 task-078 / 便携版 CLI task-079；Items/SafeBox/History 深色适配待补）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务077-P2起步 多盘扫描支持 (#task-077)`，
   该提交只含 `tasks/077-p2-multi-drive-scan.md` 一个文件，落在 `feature/task-077-p2-multi-drive-scan` 分支最前端。
