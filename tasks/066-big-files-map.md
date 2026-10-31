# 任务编号：066 M1 E 模块——大文件地图（hiberfil/Windows.old/回收站/Top100，只读不删）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：M1 业务模块缺口第一组第 2 项（E 模块大文件地图，TRD §372）。
> 上游依赖：task-065 已合并 main（merge `509aa1f`，C 模块 WinSxS 清理完成）。
> **版本线**：v0.3.x M1 模块持续落地。
> **本任务范围（4 个功能单元）**：大文件扫描后端 + IPC 命令接线 + 前端大文件地图页面 + tab 路由注册。
> **为什么合并**：E 模块大文件地图是一个完整业务闭环（后端扫描 + IPC 接线 + 前端展示 + 路由注册），必须一次 PR 打通。
> **不做**：D 模块残留 / F 模块注册表（留后续任务）；大文件删除（只读不删，仅展示）；跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 状态字段

pending

## 任务目标

### 1. 大文件扫描后端（只读）

- **TRD §372 描述**：扫描 hiberfil.sys、Windows.old、回收站、Top100 大文件，**只读不删**（仅展示，引导用户手动处理）。
- **修法**：
  - `engine/src/scan/big_files.rs`（新建）：
    - `scan_hiberfil()`：查 `C:\hiberfil.sys` 大小 + 是否存在；
    - `scan_windows_old()`：查 `C:\Windows.old` 目录总大小；
    - `scan_recycle_bin()`：查回收站总大小（`SHGetDiskFreeSpace` 或遍历）；
    - `scan_top100()`：遍历 C:\ 下 Top 100 大文件（按大小排序，跳过系统目录）；
  - 返回统一结构 `BigFileItem { path, size_bytes, category, recommendation }`。

### 2. IPC 命令接线

- **修法**：
  - `engine/src/ipc/commands/big_files.rs`（新建）：`big_files_scan()` handler；
  - `engine/src/ipc/commands/mod.rs`：注册 `big_files_scan`；
  - `engine/src/lib.rs` 或入口：注册 IPC handler（参照现有 `scan_start` 模式）。

### 3. 前端大文件地图页面

- **修法**：
  - `app/src/pages/BigFiles.tsx`（新建）：
    - 四个分类卡片（hiberfil / Windows.old / 回收站 / Top100）；
    - 每个卡片显示大小 + 处理建议（如「休眠文件：可关闭休眠释放 X GB」）；
    - Top100 列表：文件路径 + 大小 + 类别；
    - **只读模式**：无删除按钮（引导用户手动处理）；
  - `app/src/utils/invoke.ts`：加 `big_files_scan()` API 封装（参照现有模式）。

### 4. tab 路由注册

- **修法**：
  - `app/src/App.tsx`：
    - Tab 类型加 `"bigfiles"`；
    - Tab 栏加「大文件」按钮；
    - 切换渲染 `<BigFiles />`；
    - 深色主题适配（如果 BigFiles 页面用 CSS 变量）。

**本任务不做**：
- 不做 D 模块残留 / F 模块注册表（留后续任务）；
- 不做大文件删除（只读不删，仅展示建议）；
- 不做跨盘扫描（仅 C 盘）；
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 前置条件

1. `main` 已合并 task-065（merge `509aa1f`），C 模块 WinSxS 清理完成；
2. 关键锚点（必须全部在白名单内）：
   - 后端：`engine/src/scan/big_files.rs`（新建）
   - 后端：`engine/src/scan/mod.rs`（注册）
   - 后端：`engine/src/ipc/commands/big_files.rs`（新建）
   - 后端：`engine/src/ipc/commands/mod.rs`（注册）
   - 后端：`engine/src/lib.rs` 或入口（注册 IPC handler）
   - 后端测试：`engine/tests/big_files_test.rs`（新建）
   - 前端：`app/src/pages/BigFiles.tsx`（新建）
   - 前端：`app/src/pages/BigFiles.test.tsx`（新建）
   - 前端：`app/src/App.tsx`（tab 注册）
   - 前端：`app/src/utils/invoke.ts`（API 封装）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 大文件扫描后端

1. `scan_hiberfil()`：返回 hiberfil.sys 大小 + 存在状态；
2. `scan_windows_old()`：返回 Windows.old 目录总大小；
3. `scan_recycle_bin()`：返回回收站总大小；
4. `scan_top100()`：返回 Top 100 大文件列表（路径 + 大小）；
5. 单测：各扫描函数返回格式正确（mock 文件系统）。

### 2. IPC 命令接线

6. `big_files_scan()` IPC handler 注册成功；
7. 前端调用返回完整数据结构；
8. 单测：handler 返回值格式正确。

### 3. 前端大文件地图页面

9. 四个分类卡片显示大小 + 处理建议；
10. Top100 列表显示文件路径 + 大小；
11. **只读模式**：无删除按钮；
12. 深色主题适配（CSS 变量）；
13. 单测：页面渲染正确。

### 4. tab 路由注册

14. Tab 栏加「大文件」按钮；
15. 切换渲染 `<BigFiles />`；
16. 单测：tab 切换正确。

### 后端单测

17. `cargo test --workspace` 全绿（现有 + 本任务新增 ≥4 条全绿）。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**后端（engine）**：
- `engine/src/scan/big_files.rs`（新建）
- `engine/src/scan/mod.rs`（注册 big_files 模块）
- `engine/src/ipc/commands/big_files.rs`（新建）
- `engine/src/ipc/commands/mod.rs`（注册 big_files_scan 命令）
- `engine/src/lib.rs`（注册 IPC handler，如需要）
- `engine/tests/big_files_test.rs`（新建）

**前端（app）**：
- `app/src/pages/BigFiles.tsx`（新建）
- `app/src/pages/BigFiles.test.tsx`（新建）
- `app/src/App.tsx`（tab 类型 + 按钮 + 渲染）
- `app/src/utils/invoke.ts`（加 big_files_scan API）

**收尾**：
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-066-big-files-map`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-066-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `feat(scan): 大文件扫描后端（hiberfil/Windows.old/回收站/Top100，只读） (#task-066)`（单元 1）
   —— 含 `engine/src/scan/big_files.rs` + `mod.rs` + `engine/tests/big_files_test.rs`。
2. `feat(ipc): big_files_scan IPC 命令接线 (#task-066)`（单元 2）
   —— 含 `engine/src/ipc/commands/big_files.rs` + `commands/mod.rs` + `lib.rs`。
3. `feat(ui): 大文件地图页面（四分类卡片 + Top100 列表，只读模式） (#task-066)`（单元 3）
   —— 含 `app/src/pages/BigFiles.tsx` + `BigFiles.test.tsx` + `utils/invoke.ts`。
4. `feat(ui): App tab 注册「大文件」按钮 + 深色适配 (#task-066)`（单元 4）
   —— 含 `app/src/App.tsx`。
5. 自验全绿后，最后一次提交：`docs(state): 任务066完成，M1 E 模块大文件地图就绪 (#task-066)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 066: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「M1 第 3 项：D 模块残留增强（task-067）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-31  task-066  M1 E 模块大文件地图`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：各扫描函数返回格式、handler 接线、页面渲染、tab 切换。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **端到端写路径（必做，本任务硬要求）**：大文件地图必须有一条真实「切 tab → 调 IPC → 后端扫描 → 前端展示四分类卡片 + Top100 列表」证据（不能只构造 mock）。
- **真机冒烟（必做、不可裁剪）**：
  - Tab 栏显示「大文件」按钮；
  - 点击切换到 BigFiles 页面；
  - 四个分类卡片显示大小 + 处理建议；
  - Top100 列表显示文件路径 + 大小；
  - **只读模式**：无删除按钮（引导手动处理）；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 5 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（大文件地图页面 + tab 切换，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 扫描函数返回格式 | 单测输出（各函数字段正确） |
| IPC handler 接线 | 单测输出（返回值格式） |
| tab 切换 | 真机截图：Tab 栏显示「大文件」按钮 |
| 四分类卡片 | 真机截图：hiberfil / Windows.old / 回收站 / Top100 大小显示 |
| Top100 列表 | 真机截图：文件路径 + 大小列表 |
| 只读模式 | 真机证据：页面无删除按钮 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 5 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-066-big-files-map`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - hiberfil.sys / Windows.old / 回收站 扫描实现方式；
   - Top100 遍历算法（跳过系统目录策略）；
   - 只读模式 UI 设计；
   - `cargo test --workspace` 通过输出（含新增 ≥4 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（5 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（M1 第 3 项：D 模块残留增强；F 模块注册表排队；Items/SafeBox/History 深色适配待补）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务066-M1 E模块 大文件地图 (#task-066)`，
   该提交只含 `tasks/066-big-files-map.md` 一个文件，落在 `feature/task-066-big-files-map` 分支最前端。
