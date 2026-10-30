# 任务编号：062 提权 exe 底座 + 系统还原点 + 系统托盘常驻（v0.2.0 起步）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：P0 未做清单第 0 项（提权 exe 底座）+ 第 1 项（P0-10 系统还原点）+ 第 2 项（P0-9 系统托盘常驻）。
> 上游依赖：task-061 已合并 main（merge `b92a763`，M5 起步完成）。
> **版本线**：v0.2.0 开发线起步——提权 exe 底座是 C 模块(WinSxS) / F 模块(注册表) 的共同前置。
> **本任务范围（3 个功能单元）**：提权 exe 底座 + 系统还原点自动创建 + 系统托盘常驻。
> **为什么合并**：提权 exe 底座（单元 1）与系统还原点（单元 2）强耦合——还原点需要管理员权限，必须走提权 exe；系统托盘常驻（单元 3）是独立模块，但都是 P0 首批功能，按「任务量尽量大」合并一次 PR 推进。
> **不做**：跨盘恢复 fallback / RmShutdown 连带兄弟进程 / RmShutdown 30s 无进度（维持现状）；WinSxS / 注册表（留后续 M1 模块任务，依赖本任务的提权 exe 底座）。

## 状态字段

pending

## 任务目标

### 1. 提权 exe 底座（TRD §430）

- **现状**：当前应用以普通权限运行，无法执行需要管理员权限的操作（系统还原点、WinSxS 清理、注册表修改）。
- **修法**：
  - 新建独立二进制 `elevation/`（Rust crate），编译为单独的 `cclean-elevation.exe`；
  - **UAC 单次触发**：主 exe 通过 `ShellExecuteW("runas", ...)` 启动提权 exe，UAC 弹窗一次；
  - **命令白名单**：提权 exe 只接受固定命令列表（`create_restore_point` / `dism_cleanup` / `reg_export`），**不接受任意命令拼接**（防命令注入）；
  - **结果回传**：提权 exe 执行完后退出，通过退出码 + 临时 JSON 文件回传结果；
  - **主 exe 封装**：`engine/src/elevation/mod.rs` 封装 `run_elevated(cmd: &str, args: &[&str]) -> Result<ElevatedResult, String>`。

### 2. P0-10 清理前自动创建系统还原点（TRD §83）

- **PRD 描述**：清理前自动创建系统还原点，L2/WinSxS/注册表操作前必做；失败询问用户是否继续。
- **修法**：
  - 调用 `SRSetRestorePoint`（通过 WMI 或直接 FFI）；
  - 在 `execute_items` 执行前，如果当前任务包含 L2 + 风险项，自动触发系统还原点创建；
  - 还原点名称：`C-clean cleanup #<task_id> <timestamp>`；
  - 失败处理：还原点创建失败 → 弹 confirm「系统还原点创建失败，是否继续清理？」；
  - 提权 exe 命令：`create_restore_point --description "..."`。

### 3. P0-9 系统托盘常驻（TRD §433）

- **PRD 描述**：关闭窗口 = 最小化到托盘，不退出；右键菜单含「立即扫描 / 查看结果 / 打开保险箱 / 退出」；退出二次确认；开机自启开关。
- **修法**：
  - Tauri 2 内置 tray 支持（`tauri-plugin-tray` 或原生 `WindowEvent::CloseRequested` 拦截）；
  - 关闭窗口时拦截 `CloseRequested` → 隐藏窗口（`window.hide()`），显示托盘图标；
  - 托盘右键菜单：立即扫描 / 打开主窗口 / 退出（退出时二次确认）；
  - 设置页加「开机自启」开关（`tauri-plugin-autostart` 或写注册表 `HKCU\...\Run`）。

**本任务不做**：
- 不做 WinSxS 清理（C 模块，留后续任务）；
- 不做注册表修改（F 模块，留后续任务）；
- 不做提权 exe 的其他命令（只做 `create_restore_point`）；
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 前置条件

1. `main` 已合并 task-061（merge `b92a763`）；
2. 关键锚点：
   - `elevation/`（新建独立 crate，编译为 `cclean-elevation.exe`）
   - `engine/src/elevation/mod.rs`（新建，封装 `run_elevated`）
   - `engine/src/execute/executor.rs`（清理前触发还原点）
   - `src-tauri/src/lib.rs` 或 `main.rs`（托盘拦截 + 自启）
   - `src-tauri/Cargo.toml`（加 `tauri-plugin-tray` / `tauri-plugin-autostart`）
   - `app/src/pages/Items.tsx`（还原点失败 confirm UI）
   - `app/src/pages/Settings.tsx`（开机自启开关）
   - `src-tauri/tauri.conf.json`（bundle.resources 加 `cclean-elevation.exe`）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 提权 exe 底座

1. `elevation/` 独立 crate 编译为 `cclean-elevation.exe`；
2. 命令白名单：只接受 `create_restore_point`（后续可扩展），拒绝任意命令；
3. 主 exe 通过 `ShellExecuteW("runas")` 启动提权 exe，UAC 弹窗一次；
4. 提权 exe 执行完后退出，通过退出码 + 临时 JSON 回传结果；
5. `engine/src/elevation/mod.rs` 封装 `run_elevated(cmd, args)` API；
6. 单测：命令白名单拒绝未知命令（返回错误）。

### 2. 系统还原点自动创建

7. 清理前自动触发 `create_restore_point`（L2 风险项时）；
8. 还原点名称：`C-clean cleanup #<task_id> <timestamp>`；
9. 失败处理：还原点创建失败 → 弹 confirm「是否继续清理？」；
10. 成功时 audit_log 记录「系统还原点已创建」；
11. 单测：还原点名称格式 + 失败时返回错误。

### 3. 系统托盘常驻

12. 关闭窗口时拦截 `CloseRequested` → 隐藏窗口到托盘；
13. 托盘右键菜单：立即扫描 / 打开主窗口 / 退出；
14. 退出时二次确认（confirm dialog）；
15. 设置页加「开机自启」开关，prefs 持久化；
16. 单测：自启开关默认值 + 持久化。

### 后端单测

17. `cargo test --workspace` 全绿（现有 291 passed + 本任务新增 ≥4 条全绿）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `elevation/`（新建独立 crate）
- `engine/src/elevation/mod.rs`（新建）
- `engine/src/execute/executor.rs`（清理前触发还原点）
- `engine/tests/`（提权白名单 + 还原点 + 自启用例）
- `src-tauri/src/lib.rs` 或 `main.rs`（托盘拦截 + 自启）
- `src-tauri/Cargo.toml`（加 tray/autostart 插件）
- `app/src/pages/Items.tsx`（还原点失败 confirm）
- `app/src/pages/Settings.tsx`（开机自启开关）
- `src-tauri/tauri.conf.json`（bundle.resources 加提权 exe）
- `Cargo.toml` workspace 成员加 `elevation`
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-062-elevation-srpoint-tray`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-062-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 3 个功能单元拆 3 个原子提交：

1. `feat(elevation): 提权 exe 底座（独立 crate + UAC 单次触发 + 命令白名单 + 结果回传） (#task-062)`（单元 1）
   —— 含 `elevation/` crate + `engine/src/elevation/mod.rs` + tauri.conf.json + 单测。
2. `feat(execute): 清理前自动创建系统还原点（L2 风险项触发 + 失败 confirm） (#task-062)`（单元 2）
   —— 含 `executor.rs` + `Items.tsx` + 单测。
3. `feat(tray): 系统托盘常驻（关闭最小化 + 右键菜单 + 退出二次确认 + 开机自启开关） (#task-062)`（单元 3）
   —— 含 `lib.rs` + `Cargo.toml` + `Settings.tsx` + 单测。
4. 自验全绿后，最后一次提交：`docs(state): 任务062完成，提权底座+系统还原点+托盘常驻就绪 (#task-062)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 062: ...`；
   - 「最近一次变更」追加一行，含 3 个单元摘要 + commit hash；
   - 「下一步计划」改为「P0 第 3 项：清理报告导出 + 前后空间对比（task-063）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-30  task-062  提权 exe 底座 + 系统还原点 + 托盘常驻（v0.2.0 起步）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：命令白名单拒绝未知命令、还原点名称格式、自启开关默认值。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **端到端写路径（必做，本任务硬要求）**：系统还原点必须有一条真实「点清理按钮 → 提权 exe 创建还原点 → 系统还原点列表可见」证据（不能只构造 mock）；开机自启必须有一条真实「开开关 → 查注册表 Run 键 → 重启后自启」证据。
- **真机冒烟（必做、不可裁剪）**：
  - 清理前 UAC 弹窗一次；
  - 系统还原点创建成功（`SystemRestore` 面板可见）；
  - 还原点创建失败时弹 confirm「是否继续？」；
  - 关闭窗口时最小化到托盘（不退出）；
  - 托盘右键菜单含「立即扫描 / 打开主窗口 / 退出」；
  - 退出时二次确认；
  - 开机自启开关：开启后重启应用，自启生效；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，提权 exe 随安装包部署，上述 7 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（UAC 弹窗 + 还原点 confirm + 托盘菜单 + 自启开关，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 命令白名单 | 单测输出（拒绝未知命令） |
| UAC 弹窗 | 真机截图：清理前 UAC 弹窗 |
| 系统还原点创建 | 真机证据：`SystemRestore` 面板可见新还原点 |
| 失败 confirm | 真机截图：还原点失败 → 弹 confirm |
| 托盘最小化 | 真机截图：关闭窗口后托盘图标存在 |
| 托盘菜单 | 真机截图：右键菜单含 3 项 |
| 退出二次确认 | 真机截图：退出时弹 confirm |
| 开机自启 | 真机证据：开开关 → 查注册表 Run 键 → 重启自启 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 7 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-062-elevation-srpoint-tray`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 提权 exe 命令白名单列表；
   - UAC 触发方式（ShellExecuteW runas）；
   - 结果回传方式（退出码 + 临时 JSON）；
   - 系统还原点 API 调用方式（WMI vs SRSetRestorePoint FFI）；
   - 托盘实现方式（tauri-plugin-tray vs 原生）；
   - 自启实现方式（tauri-plugin-autostart vs 注册表）；
   - `cargo test --workspace` 通过输出（含新增 ≥4 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（7 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（P0 第 3 项：清理报告导出；M1 C/F 模块依赖提权 exe 底座）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务062-提权底座+系统还原点+托盘常驻 (#task-062)`，
   该提交只含 `tasks/062-elevation-srpoint-tray.md` 一个文件，落在 `feature/task-062-elevation-srpoint-tray` 分支最前端。
