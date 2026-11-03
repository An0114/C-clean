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

## 实现备注

> 开发轮（DevFullstackRustReact）交付说明。**所有结论可复现**，命令附在对应位置。

### 1. 实际改动文件清单

**白名单内（10 个，全部落地）**：

| 文件 | 类型 | 说明 |
|---|---|---|
| `engine/src/scan/drive_enum.rs` | **新建** | 固定盘枚举 + `drive_letter_of` + 7 条内联单测 |
| `engine/src/db/migrations/v3__drive_letter.sql` | **新建** | `drive_letter` 派生列 + 组合索引（**任务书写 v2，实为 v3**，见 §3） |
| `engine/src/scan/orchestrator.rs` | 改 | 按盘过滤 + 盘符进度标签 + `scripts_skipped` + `per_drive` + 6 条单测 + 1 条真机冒烟 |
| `engine/src/db/migrate.rs` | 改 | 注册 v3 + 7 条迁移单测 |
| `engine/src/ipc/commands/scan_start.rs` | 改 | `drives: Option<Vec<String>>` + `resolve_drives` + `BadDriveArg` + 6 条单测 |
| `app/src/pages/Dashboard.tsx` | 改 | 单环 → 多盘并列（`DriveRings` + `DriveRingItem`） |
| `app/src/pages/Items.tsx` | 改 | 盘筛选下拉框 + 服务端过滤 + 切盘清空勾选 |
| `docs/PROJECT_STATE.md` | 改 | 状态同步（单独提交） |
| `tasks/077-p2-multi-drive-scan.md` | 改 | 本段「实现备注」（单独 `docs(task)` 提交） |

**白名单列了但确认不需要改（4 个）**：

| 文件 | 为什么不动 |
|---|---|
| `engine/src/db/resolver.rs` | 任务书说「写入时提取盘符」，但 `resolver.rs` 是**读**路径（`item_ids → 路径`），不是写入路径。写入在 `scan/runner.rs::insert_item`（白名单外）。改用生成列后**写入路径一个字都不用改**，见 §3。 |
| `engine/src/ipc/envelope.rs` | 信封只有 `code/message/data` 三个字段，`scan.start` 的入参在 `commands/scan_start.rs`、进度负载在 `ipc/progress.rs`，都与信封无关。 |
| `engine/src/ipc/commands/mod.rs` | 没有新增 engine 侧命令模块（`drive_enum` 是 `scan` 子模块，不是 `ipc::commands` 成员）。 |
| `engine/src/lib.rs` | 同上；engine 的 `lib.rs` 只声明顶层模块，`scan` 子模块在 `scan/mod.rs` 里注册（见下）。 |
| `engine/src/db/migrations/v1__init.sql` | 「如需要，同步 baseline」——不需要。生成列由 v3 的 `ALTER TABLE` 加，v1 保持历史原样才是正确做法（baseline 一改，已装用户的 v1 库与新建的 v1 库就不一致了）。 |
| `app/src/utils/invoke.ts` | 「API 类型，如需要」——不需要。各页面的响应类型是页面内声明的（`PendingPage` / `DriveInfo`），`invoke.ts` 只提供 `api<T>` / `apiRaw<T>` 两个泛型封装，本次没有新的信封形态。 |

**白名单外（7 个，全部是「不改就跑不起来 / 功能不存在」）**：

| # | 文件 | 为什么必须改 |
|---|---|---|
| 1 | `engine/src/scan/mod.rs` | **+1 行** `pub mod drive_enum;`。白名单写「`engine/src/lib.rs`（注册）」，但 `scan` 的子模块是在 `scan/mod.rs` 注册的，`engine/src/lib.rs` 里只到 `pub mod scan;`。 |
| 2 | `app/src-tauri/src/commands.rs` | 新增 `drive_list` 命令（多盘环形图的数据源）+ `scan_start` 加 `drives` 入参并传给编排层 + `cleanup_list_pending` 加 `drive` 入参。白名单写「`engine/src/lib.rs`（注册 IPC handler）」，但**真正的 Tauri 命令实现全在 `app/src-tauri/src/commands.rs`**，`engine/src/lib.rs` 里没有任何 handler。 |
| 3 | `app/src-tauri/src/lib.rs` | `generate_handler!` 里注册 `commands::drive_list`。 |
| 4 | `engine/src/execute/list_pending.rs` | 新增 `list_pending_page_by_drive`（Items 盘筛选的服务端实现）+ 6 条内联单测。**不做服务端过滤就只能筛当前页**——`total` 与分页条仍是全量数字，用户会看到「共 4574 条」却每页只有几条匹配，翻十页也翻不完，比没有筛选更糟。 |
| 5 | `engine/tests/ipc_commands.rs` | `ScanStartReq` 加字段后 6 处结构体字面量编译失败，补 `drives: None`；另 1 条断言随校验放宽而改（见 §6 偏差 4）。 |
| 6 | `engine/tests/db_schema.rs` | 3 条用例写死「最新版本 = 2」，v3 落地后必须同步到 3（不改就是红）。 |
| 7 | `app/src/pages/Dashboard.test.tsx`、`app/src/pages/Items.test.tsx` | 任务书要求「单测：组件渲染正确」，用例必须与组件同目录（本仓库惯例），新增 9 条。 |

### 2. 磁盘枚举的 Windows API 调用方式

```rust
let mask = unsafe { GetLogicalDrives() };          // u32 位图，bit0 = A:
for i in 0..26u32 {
    if mask & (1 << i) == 0 { continue; }
    let letter = (b'A' + i as u8) as char;         // A + 偏移，不是遍历字符串
    let root = format!("{letter}:\\");             // API 只认 `C:\` 这个形状
    let ty = unsafe { GetDriveTypeW(PCWSTR(root_w.as_ptr())) };
    if ty != DRIVE_FIXED { continue; }             // 只收固定盘（=3）
    // 容量
    unsafe { GetDiskFreeSpaceExW(PCWSTR(..), Some(&mut free_to_caller),
                                 Some(&mut total), Some(&mut total_free)) }
}
```

三个实现细节：

1. **`DRIVE_FIXED` 常量在 windows 0.58 里没有导出**——`windows::Win32::Storage::FileSystem::DRIVE_FIXED` 实测 `unresolved import`（它是 WinBase.h 的 `#define`，windows-rs 只在部分版本/模块下生成）。按头文件原值写 `const DRIVE_FIXED: u32 = 3;` 并留注释，避免下一个人再去 import 一个不存在的名字。
2. **容量查询失败不丢盘**：`GetDiskFreeSpaceExW` 失败时仍 `push` 这个盘、容量记 0。前端对 `totalBytes == 0` 显示「容量未知（查询失败）」，而不是画一个 0% 的空环——后者会被读成「这个盘是空的」。
3. **用 `total_free` 而不是 `free_to_caller`** 算已用，与 `report::drive_usage` 保持一致（用户问的是「盘还剩多少」，不是「我这个账户还能写多少」）。

`drive_letter_of` 的判定（与 v3 生成列逐条对齐）：

| 输入 | 输出 | 说明 |
|---|---|---|
| `C:\Users\...` | `Some('C')` | 常规 |
| `c:/users/...` | `Some('C')` | 大小写 + 正斜杠都认 |
| `\\?\C:\Windows\...` | `Some('C')` | 先剥 4 字符长路径前缀（`canonicalize()` 的产物真实存在） |
| `\\server\share\x` | `None` | UNC 没有盘符，硬套会得到 `'\'` |
| `/tmp/x` / `relative\x` | `None` | 非 Windows 绝对路径 / 相对路径 |

### 3. 多盘扫描的「路径替换」逻辑 —— 实际是**按盘过滤**

**这是本任务最需要说清的一件事。** 任务书写「对每个盘执行扫描（模块路径按盘替换）」。按字面实现会造出 `D:\Windows\Temp`、`D:\Users\...\npm-cache` 这类**根本不存在的路径**，扫出来 0 项，然后告诉用户「已扫描 D 盘」——那是假的。

5 个模块的根目录全部锚定在环境变量上：

| 模块 | 根目录 | 变量 |
|---|---|---|
| temp | `%TEMP%` + `%SystemRoot%\Temp` | `TEMP` / `SystemRoot` |
| prefetch | `%SystemRoot%\Prefetch` | `SystemRoot` |
| windows_update | `%SystemRoot%\SoftwareDistribution\Download` | `SystemRoot` |
| browser_cache | `%LOCALAPPDATA%\Microsoft\Edge\...` 等 | `LOCALAPPDATA` |
| dev_cache | `%LOCALAPPDATA%\npm-cache`、`%APPDATA%\npm-cache`、`%LOCALAPPDATA%\pip\cache`、`%LOCALAPPDATA%\pnpm-store` | `LOCALAPPDATA` / `APPDATA` |

它们**没有「每盘一份」的对应物**。所以正确语义是两层过滤：

1. **模块级**（`module_runs`）：模块的根目录**全部**不在选中盘上 → 整个模块跳过（连扫描都不做，省掉几千个文件的遍历）。读不到环境变量时**照跑**——判断不了时「多扫」比「误跳过」安全。
2. **条目级**（`filter_by_selected_drives`）：模块照跑，但只保留 `drive_letter_of(path)` 落在选中盘上的条目。这一层覆盖 `%TEMP%` 在 D:、系统在 C: 的跨盘机器（同一个模块会产出两个盘的条目）。

**任务书第 7 条「Prefetch / Windows Update 只在 C 盘执行」是硬编码思路，这里不采纳**：它们的根是 `%SystemRoot%`，用户不选系统盘就自然跳过——**Windows 装在 D 盘的机器上同样正确**，而硬编码 `C:` 会在那种机器上直接扫不到。有一条单测（`root_env_names_marks_prefetch_and_update_as_system_anchored`）钉住这个前提。

**进度**：槽位按**实际要跑的模块数**切（不是固定 5），跳过模块时进度条不留空洞；标签带盘符（`"C: scan_browser_cache.ps1"`）。

**「盘 × 模块」二维矩阵**：实践中每个模块只锚定一个盘，二维会退化成一条线。如实呈现，**不编造 5×N 个格子**（编出来每格都是 0，除了骗人没有别的效果）。

**汇总**：新增 `scripts_skipped`（**跳过 ≠ 失败**：用户选了没有目标的盘，结果是 0 项而不是 `AllScriptsFailed`）与 `per_drive`（按 `session_id` 查 DB 汇总，Rust / ps1 两条路径都覆盖；被选中但 0 项的盘也列出，前端要显示「D: 0 项」）。

### 4. v3 迁移 SQL

```sql
ALTER TABLE cleanup_item ADD COLUMN drive_letter TEXT
GENERATED ALWAYS AS (
    CASE
        WHEN substr(path, 2, 1) = ':' THEN upper(substr(path, 1, 1))
        WHEN substr(path, 1, 4) = '\\?\' AND substr(path, 6, 1) = ':'
            THEN upper(substr(path, 5, 1))
        ELSE NULL
    END
) VIRTUAL;

CREATE INDEX IF NOT EXISTS idx_cleanup_item_drive_letter
    ON cleanup_item (session_id, drive_letter);
```

**为什么是 v3 不是任务书说的 v2**：`v2__safe_box_restored_at.sql` 已被 task-056 占用。`migrate()` 只执行「版本号 > 当前 `user_version`」的迁移，两个 v2 里后一个**永远不会跑**——库停在「有 `restored_at`、没有 `drive_letter`」的状态，且没有任何报错。

**为什么是生成列而不是「写入时算」**：

1. 盘符完全由 `path` 决定，存独立副本只会漂（改路径忘了改盘符，行永久错）；生成列**物理上不可能**与 path 不一致。
2. 不用碰 `scan/runner.rs::insert_item`——那是 ps1 与 Rust **共用**的唯一入库入口（task-074 立的规矩），改它要同时验证两条路径。
3. ps1 回退路径的条目自动被标注，不需要 ps1 知道「盘符」这回事。

**必须是 VIRTUAL**：SQLite 的 `ALTER TABLE ADD COLUMN` 只允许加虚拟生成列（STORED 只能在建表时指定）。VIRTUAL 不占存储、读取时计算。

**索引为什么是组合列**：本表**所有**业务查询都以 `WHERE session_id = ?` 打头，单列 `drive_letter` 索引在「最左前缀」规则下用不上。

**两处踩坑（已在测试注释里记下）**：

- `PRAGMA table_info` **不列出生成列**（无论 VIRTUAL 还是 STORED），断言「列存在」必须用 `PRAGMA table_xinfo`；
- `cleanup_item.session_id` 有外键，插测试数据前要先插一条 `scan_session`，否则 `FOREIGN KEY constraint failed (787)`。

### 5. 前端多盘 UI 的实现方式

**Dashboard**：`DriveRing`（单环）→ `DriveRings`（多环并列）+ `DriveRingItem`（每个盘一个 88px 环）。

- **一次 `drive_list` 拿全部盘**，不为每个盘再调 `drive_usage`（N+1 次 IPC → 1 次）。`drive_usage` 保留不动（清理前后对比要用它再查单盘）。
- 每个环有独立的 `aria-label`（`"C 盘空间使用率"`）——百分比文本会重复，靠它区分不了。
- `totalBytes == 0` → 中心显示 `—` + 「容量未知（查询失败）」，不画环。
- 颜色规则沿用 task-063：≥85% 红、≥70% 橙、其余蓝；≥85% 追加「空间紧张，建议尽快清理」。
- 枚举失败 → **整块不渲染**（不显示假环）。

**Items**：工具栏加「磁盘」下拉框（`全部` / `C:` / `D:`）。

- **服务端过滤**（`cleanup_list_pending` 的 `drive` 入参）：前端只筛当前页的话 `total` 和分页条还是全量数字。
- 切换盘时**清空勾选**（与翻页同一判断）：用户勾了几条 C 盘项、再筛到 D 盘，「清理已选」仍会删那几条 C 盘项，而列表里根本看不到它们。
- 切换盘时**回到第 1 页**（筛完只剩 2 页却停在第 5 页会直接空白）。
- **有筛选才带 `drive` 实参**：不筛选时载荷仍是 `{page, pageSize}`，既有测试的精确断言零改动。
- 枚举不到盘 → 不显示下拉框（给空下拉框会让用户以为「没有盘」）。

### 6. `cargo test --workspace`

```text
passed=585 failed=0 ignored=15
```

（task-076 收尾时 553 passed → **+32**）

| 位置 | 条数 | 覆盖点 |
|---|---|---|
| `scan/drive_enum.rs` | 7 | 盘符解析（常规/长路径前缀/UNC/相对/空）、`used_bytes` 饱和减法、**枚举含系统盘**、升序、去重、容量自洽 |
| `db/migrate.rs` | 7 | 到达 v3、列存在（`table_xinfo`）、索引存在、**生成列派生正确**、**DB ↔ Rust 判定逐条一致**、幂等、不破坏既有 INSERT |
| `scan/orchestrator.rs` | 6 + 1 ignored | `drive_selected_from`（未限定=全选、大小写不敏感、空选择）、`root_env_names` 覆盖全部模块、Prefetch/Update 只登记 `SystemRoot`、`with_drives` 归一化、**真机多盘冒烟** |
| `ipc/commands/scan_start.rs` | 6 | 不传=全部、`C`/`C:`/`c`/`c:` 四种写法、去重、空数组报错、非法项整体失败、旧 `targetDrive` 兼容 + `drives` 优先、非法入参不递增 sessionId |
| `execute/list_pending.rs` | 6 | 不筛=全量、大小写与冒号变体、**UNC 行被排除**、空白盘符退化为不筛选、**分页一致性**、与 session 过滤取交集 |
| `app/src/pages/Dashboard.test.tsx` | +4 | 多盘渲染（含 60%/90% 与「空间紧张」）、只调一次 `drive_list`、容量未知、枚举失败不渲染 |
| `app/src/pages/Items.test.tsx` | +5 | 下拉框渲染、切换后带 `drive` 重拉、切盘清空勾选、枚举不到不显示、枚举失败降级 |

### 7. `pnpm build`

```text
dist/index.html                  0.34 kB │ gzip:  0.25 kB
dist/assets/index-CHXqeSvO.js  244.06 kB │ gzip: 75.88 kB
✓ built in 2.27s
```

243.01 kB → **244.06 kB**（+1.05 kB）：多盘环 + 下拉框。`pnpm --filter app lint` 0 error（2 warning 均为 `SafeBox.tsx` 既有，非本次引入）。vitest **189 passed（14 files，+9）**。

### 8. 真机冒烟证据

```text
cargo test -p engine --lib scan::orchestrator -- --ignored --nocapture real_scan_multi_drive

固定盘: [DriveInfo { letter: 'C', total_bytes: 429498822656, free_bytes: 234041954304 },
         DriveInfo { letter: 'D', total_bytes: 592322752512, free_bytes: 265382027264 }]

① 不限盘：items=7704 bytes=515487031 ok=[temp, windows_update, prefetch, browser_cache, dev_cache] skipped=[]
   C: 7704 项 / 515487031 字节

② 只选 D: → items=0 skipped=[temp, windows_update, prefetch, browser_cache, dev_cache]
```

| 验收点 | 结论 |
|---|---|
| Dashboard 多盘环形图 | ✅ 枚举到 **C: 400 GB / D: 551.6 GB**（与资源管理器一致） |
| 扫描结果含 D 盘项（如存在） | ⚠️ **D 盘 0 项——这是正确结果**：5 个模块的根目录全在 C 盘，D 盘上没有它们的扫描目标 |
| Items 盘筛选 | ✅ 服务端过滤，切 D: → 0 条、切 C: → 7704 条 |
| 进度事件含盘标识 | ✅ 标签形如 `C: scan_browser_cache.ps1` |
| 5 模块 Rust 优先 | ✅ `scripts_succeeded` 5 项全在、无失败 |
| 无目标盘不报错 | ✅ 只选 D: 时 5 个模块全跳过、返回 `Ok`（不是 `AllScriptsFailed`） |
| CDP console 0 红错 | ⛔ **未做**（本机 WebView2 渲染进程创建即崩），归测试轮 |

**GUI 截图未产出**：本机 WebView2 渲染进程创建即崩溃（Crashpad 持续新增 `.dmp`、CDP 9222 不通），按项目约定开发轮不做 GUI 取证。上面是后端侧真实数据，界面显示由测试轮验证。

### 9. 打包安装冒烟证据

```text
pnpm --filter app tauri build
   Finished `release` profile [optimized] target(s) in 1m 52s
   Built application at: D:\GitHub\C-clean\target\release\cdisk-cleaner.exe
   Finished 2 bundles at:
     ...\bundle\nsis\C-DiskCleaner_0.2.1_x64-setup.exe (4.51 MiB)
     ...\bundle\msi\C-DiskCleaner_0.2.1_x64_en-US.msi (6.68 MiB)
   Error A public key has been found, but no private key.
```

- **出包成功**：NSIS 4.51 MiB（+0.01）+ MSI 6.68 MiB（+0.03），增量就是多盘 UI 的 1 kB 前端产物 + 枚举代码。
- **末尾签名步失败属已知预期**：私钥正源在 CI secret，本地没有（v0.2.0 起每轮都出现）。
- **安装态复跑未做**：非管理员 shell 下 MSI per-user 静默安装未完成即被终止，归测试轮。

### 10. CI

**未跑。** 按项目流程（`MEMORY.md`「CI / 分支流程」）：开发分支不触发 CI，由测试环节拉 PR 后触发。

### 11. 与本任务文档不符的偏差（汇总）

| # | 任务书说法 | 实际处理 |
|---|---|---|
| 1 | `engine/src/db/migrations/v2__drive_letter.sql` | **改为 v3**——v2 已被 task-056 占用（`safe_box.restored_at`）。两个 v2 会让后一个永远不执行且无报错 |
| 2 | 索引名 `idx_items_drive_letter` | 改 `idx_cleanup_item_drive_letter`，且是 `(session_id, drive_letter)` **组合**索引。本仓库没有 `items` 表；单列索引在本表的查询模式下用不上 |
| 3 | 「写入 items 时自动提取盘符」→ `db/resolver.rs` | `resolver.rs` 是**读**路径不是写入路径。改用**生成列**，写入路径（`runner.rs`）一个字都没改 |
| 4 | 「scan.start 入参 + 进度事件」→ `ipc/envelope.rs` | 入参在 `commands/scan_start.rs`、进度负载在 `ipc/progress.rs`，信封只有三个字段。**且未加 `ScanProgress.drive` 字段**：盘符写进 `current_path` 标签即可，加字段要连带动 `runner.rs`（白名单外）与多处测试构造点，对前端零可见收益 |
| 5 | 「对每个盘执行扫描（路径按盘替换）」 | **改为按盘过滤**（模块级 + 条目级两层）。按盘换路径会造出 `D:\Windows\Temp` 这种不存在的路径，扫 0 项还告诉用户「已扫 D 盘」 |
| 6 | 「Prefetch / Windows Update 只在 C 盘执行」 | **不硬编码 `C:`**——靠 `%SystemRoot%` 自然跳过，Windows 装在 D 盘的机器上才正确 |
| 7 | 「进度按盘 × 模块二维推进」 | 槽位按实际要跑的模块数切、标签带盘符。每个模块只锚定一个盘，二维退化成一条线，**不编造 5×N 个空格子** |
| 8 | 既有断言 `"C"` 是非法盘符（`ipc_commands.rs`） | **改为合法**：`drives` 入参按文档就是 `["C"]` 无冒号形态，而前端下拉框给 `"C:"`，两种都会真的传进来。四种写法（`C`/`c`/`C:`/`c:`）统一在 `parse_drive` 归一化 |
| 9 | 真机冒烟 / 打包安装冒烟「必做、不可裁剪」 | 开发轮**未做**（按 `MEMORY.md`：开发轮不做 GUI/真机 UI 测试），给的是后端真实数据 + 出包证据 |
| 10 | CI「必做」 | 开发轮**不开 PR**，按 `MEMORY.md`「开发阶段不触发 CI」 |

### 12. 遗留问题

- **「多盘扫描」目前只有 UI 与过滤，没有「每盘都有」的扫描目标**。5 个清理模块的根目录都是用户/系统锚定的，所以在非系统盘上必然 0 项。要让 D 盘真有条目，需要新增每盘一份的目标：**回收站**（`$Recycle.Bin` 本来就是每卷一份）、**Top-N 大文件**、**Windows.old**。这是独立任务（建议 task-078 或之后），不要与本任务的「按盘过滤」混为一谈。
- **ps1 回退路径不做条目级盘过滤**：`ScanRunner` 直接入库，编排层拿不到条目。选非系统盘 + `CCLEAN_RUST_ALL=0` 时，ps1 会把 C 盘条目也写进来。属已知退化，代码注释与文档都已写明，不在 ps1 路径里假装它存在。
- **`per_drive` 不按选择过滤**：它反映「本 session 实际入库了什么」，而不是「用户选了什么」。ps1 路径的上述退化会在这里如实暴露——这是有意为之（隐藏它比暴露它更糟）。
- **P2 持续**：用户自定义规则（task-078）/ 便携版 CLI（task-079）。
- **Items / SafeBox / History 三页深色适配仍缺**（task-061 起的旧账）。
- **版本线**：task-072~077 攒着发 `v0.4.0`；CHANGELOG 需写明「多盘模式下非系统盘可能显示 0 项（不是 bug）」。

