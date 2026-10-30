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

testing

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

---

## 实现备注

> 执行日期：2026-10-30　分支：`feature/task-062-elevation-srpoint-tray`（基线 `4b66972`，从 main `b92a763` 拉出）

### 1. 实际改动文件清单

| 文件 | 类型 | 改动 |
|---|---|---|
| `elevation/Cargo.toml` | 配置 | bin 改名 `cclean-elevation`；**零运行时依赖**（只加 `tempfile` 作 dev-dep） |
| `elevation/src/main.rs` | 后端 | 从 M0 stub 填成真实现：白名单 + 参数解析 + 结果 JSON + `SRSetRestorePointW` FFI；7 条单测 |
| `engine/src/elevation/mod.rs` | 新建 | `run_elevated`（ShellExecuteExW + 等待 + 读结果）、`validate_command`、`build_args_string`、`parse_result_json`、`ensure_restore_point`、`restore_point_name`、`format_utc`；13 条单测 |
| `engine/src/execute/executor.rs` | 后端 | `needs_restore_point`（L2 判定）+ `log_restore_point`（audit_log） |
| `engine/src/lib.rs` | 后端（**缺口**） | `pub mod elevation;` |
| `engine/Cargo.toml` | 依赖（**缺口**） | windows features 加 `Win32_UI_Shell` / `Win32_UI_WindowsAndMessaging` / `Win32_System_Registry` |
| `app/src-tauri/src/commands.rs` | Tauri（**缺口**） | `cleanup_execute` 加 `skip_restore_point` + 还原点编排；`locate_elevation_exe`；`quit_app` / `get_autostart_enabled` / `set_autostart_enabled` |
| `app/src-tauri/src/lib.rs` | Tauri | 托盘 `setup_tray` + `CloseRequested` 拦截 + `show_main_window`；注册 3 个新命令 |
| `app/src-tauri/Cargo.toml` | 依赖 | `tauri` 加 `tray-icon` feature；`tauri-plugin-autostart` |
| `app/src-tauri/tauri.conf.json` | 配置 | `beforeBuildCommand` 加编译提权 exe；`bundle.resources` 加 `binaries/cclean-elevation.exe` |
| `scripts/build-elevation.mjs` | 新建 | 编译 + 复制 + 自检（无参数调用必须退出 2） |
| `scripts/tauri-resources.test.mjs` | 测试（**缺口**） | 资源契约 7 条 → 8 条 |
| `.gitignore` | 配置（**缺口**） | 忽略 `app/src-tauri/binaries/`（构建产物） |
| `app/src/pages/Items.tsx` | 前端 | `cleanup_execute` 改走 `apiRaw`，处理 2007 → confirm → `skipRestorePoint` 重调 |
| `app/src/pages/Settings.tsx` | 前端 | 「启动」卡片 + 开机自启开关 |
| `app/src/App.tsx` | 前端 | 监听 `tray://scan` / `tray://quit-request` |
| `app/src/pages/Dashboard.tsx` | 前端 | `scanSignal` prop → 托盘「立即扫描」触发 |
| `engine/tests/restore_point.rs` | 新建测试 | 5 条 L2 判定 |
| `app/src/pages/Settings.test.tsx` | 测试 | 3 条自启 |

### 2. 提权 exe 命令白名单

```rust
const ALLOWED_COMMANDS: &[&str] = &["create_restore_point"];
```

**只有一条**。后续 M1 的 WinSxS / 注册表任务会加 `dism_cleanup` / `reg_export`，但每次新增必须同步改三处：`elevation/src/main.rs` 的白名单 + `match` 分支、`engine/src/elevation/mod.rs` 的 `ALLOWED_COMMANDS`、以及两侧的单测。

**安全模型的三个要点**：
1. 白名单校验在**任何参数解析之前**——未知命令连参数都不看，直接 `EXIT_UNKNOWN_CMD`；
2. 每个命令只读它认识的 `--key`，多余参数一律忽略；
3. **绝不拼接命令行去调 `cmd.exe` / `powershell`**——这是命令注入的经典入口。

### 3. UAC 触发方式：`ShellExecuteExW("runas")`，不是 `ShellExecuteW`

任务书写的是 `ShellExecuteW`，但**它不返回进程句柄**，拿不到退出码、也没法等它结束。改用：

```rust
SHELLEXECUTEINFOW {
    cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
    fMask: SEE_MASK_NOCLOSEPROCESS,   // ← 要 hProcess
    lpVerb: PCWSTR(HSTRING::from("runas").as_ptr()),
    lpFile: PCWSTR(exe_path.as_ptr()),
    lpParameters: PCWSTR(args.as_ptr()),
    nShow: SW_SHOWNORMAL.0,
    ..Default::default()
}
```

**坑**：`SHELLEXECUTEINFOW` 在 windows crate 里被 gate 在 `Win32_System_Registry` 后面（因为结构体有 `hkeyClass` 字段）。只加 `Win32_UI_Shell` 会报「no SHELLEXECUTEINFOW in Win32::UI::Shell」。

用户点 UAC「否」→ `ShellExecuteExW` 返回 `ERROR_CANCELLED`(1223)，转成中文「用户取消了 UAC 提权请求」。

### 4. 结果回传方式

| 通道 | 内容 | 为什么需要两条 |
|---|---|---|
| 退出码 | 0 / 2 / 3 / 4 | 即使 JSON 写失败也能判断成败 |
| 临时 JSON | `{ok, cmd, message, sequenceNumber}` | 带错误原文（「系统还原未开启」这种） |

**为什么不用 stdout**：`ShellExecuteExW` 拉起的进程拿不到 stdout（那是 `CreateProcess` 的能力）。所以由主进程生成临时路径、用 `--result` 传进去、读完即删。

**以退出码为准**：`parse_result_json` 会用 OS 给的退出码覆盖 JSON 里的 `ok`——退出码来自操作系统，比子进程自报更可信。

### 5. 系统还原点 API：直接 FFI `SRSetRestorePointW`（不走 WMI）

**为什么不用 WMI**：`Win32_SystemRestore.CreateRestorePoint` 要走 COM 初始化 + WMI 查询，代码量和失败模式都更多；`SRSetRestorePointW` 一次调用就够，且**能在提权 exe 里零依赖实现**（手写 `#[link(name = "sfc")]`）。

**调用序列**：`BEGIN_SYSTEM_CHANGE`（建点）→ 立刻 `END_SYSTEM_CHANGE`（关闭变更期，避免系统认为「有变更未结束」影响后续还原点合并）。

**失败的最常见原因**：系统还原被关闭。此时 `SRSetRestorePointW` 返回 0、`status.status` 给错误码，我们把错误码带进 message 返回给上层。

### 6. 托盘实现方式：Tauri 2 内置（任务书说的 `tauri-plugin-tray` 不存在）

**Tauri 2 没有 `tauri-plugin-tray` 这个插件**——托盘是内置能力，只是默认不开 `tray-icon` feature。所以：

```toml
tauri = { workspace = true, features = ["tray-icon"] }
```

```rust
TrayIconBuilder::with_id("main-tray")
    .menu(&menu)                       // 立即扫描 / 打开主窗口 / 退出
    .show_menu_on_left_click(false)    // 左键唤窗口，右键出菜单
    .on_menu_event(...)
    .on_tray_icon_event(...)           // 左键单击 → show_main_window
    .build(app)
```

**关闭拦截**：`.on_window_event` 里 `CloseRequested { api, .. }` → `api.prevent_close()` + `window.hide()`。

**「退出」的二次确认怎么做**：托盘菜单回调里**没有 WebView**，弹不了 `confirm`。所以：

```
托盘「退出」→ show_main_window() + emit("tray://quit-request")
           → 前端 window.confirm(...)
           → 确认 → invoke("quit_app") → app.exit(0)
```

### 7. 自启实现方式：`tauri-plugin-autostart`，状态以注册表为准

用插件而不是手写 `HKCU\...\Run`：插件处理了「安装路径变化后条目失效」「卸载时清理」这些边界。

**关键设计**：**不把自启状态存进 prefs**。真实来源是系统注册表——用户在「任务管理器 → 启动」里手动禁用了，我们的 prefs 还写着 true，两边会打架。所以 `get_autostart_enabled` 直接查 `app.autolaunch().is_enabled()`，`set_autostart_enabled` 写完再**回读一次**返回真实状态。

### 8. `cargo test` / `pnpm` 通过输出

```
cargo test --workspace  →  passed=316  failed=0  ignored=8
pnpm --filter app test  →  9 files / 101 passed
tauri build             →  exit 0（NSIS 4.22 MiB / MSI 6.17 MiB + 2 .sig）
```

`316 = 291（task-061 基线）+ 25`，新增明细：

| 用例 | 位置 | 覆盖 |
|---|---|---|
| `whitelist_rejects_unknown_command` | elevation | **未知命令必须拒绝**（验收 6） |
| `missing_command_is_bad_usage` / `known_command_without_result_is_bad_usage` | elevation | 用法错误 |
| `take_flag_reads_value_and_ignores_extras` | elevation | 参数解析 + 悬空 key 不 panic |
| `json_escape_handles_quotes_and_control_chars` | elevation | JSON 转义（含中文不转义） |
| `result_json_has_expected_shape` / `result_json_null_sequence_when_none` | elevation | 结果 JSON 形态 |
| `validate_rejects_unknown_command` / `validate_accepts_whitelisted_command` | engine | 主进程侧白名单（弹 UAC 前就拒） |
| `args_string_quotes_values_with_spaces` / `neutralizes_embedded_quotes` | engine | 参数引号处理（防注入） |
| `parse_result_reads_full_json` / `falls_back_when_json_missing` / `trusts_exit_code_over_json_ok` | engine | 结果解析与兜底 |
| `run_elevated_rejects_unknown_cmd_before_launching` / `errors_when_exe_missing` | engine | 早失败 |
| `format_utc_known_timestamps` | engine | 含**闰日 2024-02-29** 验证 |
| `restore_point_name_has_task_id_and_timestamp` / `fits_in_255_utf16_units` | engine | 名称格式（验收 8） |
| `all_l1_does_not_need_restore_point` / `any_l2_needs_restore_point` / `empty_ids_never_needs_restore_point` / `unknown_ids_do_not_trigger` / `l2_outside_the_batch_does_not_trigger` | restore_point | L2 判定五态 |
| 自启 3 条 | Settings.test | 回填 / 切换 / 失败回滚（验收 16） |

**`pnpm build`**：`196.96 kB │ gzip 63.37 kB`（基线 195.42，+1.5 kB）。

**`tauri build` 关键证据**：NSIS 从 **4.05 MiB → 4.22 MiB**（+0.17 MiB ≈ 提权 exe 的 153 KB），证明 `bundle.resources` 真的把 exe 打进包了。

### 9. 真机 / 安装态冒烟 / CI

| 项 | 状态 | 说明 |
|---|---|---|
| UAC 弹窗一次 | **移交测试 Agent** | 按新流程开发轮不做 GUI/真机 |
| 系统还原点创建（面板可见） | **移交测试 Agent** | 需系统还原已开启 |
| 还原点失败 → confirm | **移交测试 Agent** | 可关掉系统还原构造 |
| 关闭窗口 → 托盘常驻 | **移交测试 Agent** | 同上 |
| 托盘菜单三项 + 退出二次确认 | **移交测试 Agent** | 同上 |
| 开机自启（查注册表 + 重启） | **移交测试 Agent** | 同上 |
| NSIS 安装态复跑 | **移交测试 Agent** | **重点：提权 exe 必须在安装目录根** |
| CI（PR + 五道闸） | **移交测试 Agent** | 开发分支不跑 CI；**未建 PR** |

开发轮替代验证：`tauri build` 出包体积增量证明 exe 进包；`build-elevation.mjs` 的**自检**（无参数调用必须退出 2）证明产物可执行且白名单链路通；25 条单测覆盖白名单 / 参数 / 解析 / 时间格式 / L2 判定。

### 10. 与任务书不符的偏差（4 项）

1. **`tauri-plugin-tray` 不存在**。Tauri 2 的托盘是内置能力 + `tray-icon` feature，没有这个插件。见 §6。
2. **`ShellExecuteW` 拿不到退出码**，改 `ShellExecuteExW` + `SEE_MASK_NOCLOSEPROCESS`。见 §3。
3. **白名单漏了 8 个必改文件**：`engine/src/lib.rs`（mod 声明）、`engine/Cargo.toml`（windows features）、`app/src-tauri/src/commands.rs`（新命令 + 编排）、`app/src/App.tsx` + `Dashboard.tsx`（托盘事件）、`scripts/build-elevation.mjs` + `scripts/tauri-resources.test.mjs` + `.gitignore`。都属于「不做就编译不过 / 打包不了 / 资源契约测试红」的级联改动。
4. **路径层级**：任务书写 `src-tauri/*`，实际是 `app/src-tauri/*`（老问题，task-054 起就存在）。

### 11. 遗留问题

- **`ensure_restore_point` 用的是 `#0` 而不是真实 task_id**：真正的 `cleanup_task` 行要等 `execute_items` 才建，而还原点必须在它**之前**建。名称里用 `#0` + UTC 时间戳区分批次。若将来要精确对应，得改成「先建 task 行（状态 pending）→ 建还原点 → 执行」。
- **还原点名称用 UTC**：不引 chrono 的代价。用户在还原点面板看到的顺序是对的，只是时间比本地差几个时区。
- **托盘「立即扫描」在设置页时**：会切到仪表盘并触发扫描。如果用户正在改设置，切走会丢焦点——可接受但不算优雅。
- **`quit_app` 没有超时保护**：如果后台扫描正在跑，`app.exit(0)` 会强杀线程。SQLite 有 WAL，不会损坏，但正在进行的清理会中断（保险箱里的文件仍可恢复）。
- **开机自启的路径绑定**：autostart 插件写的是当前 exe 的绝对路径。如果用户把应用从 Program Files 挪走，注册表条目会失效（插件不会自动修）。卸载时插件会清理。
- **M1 的 C 模块（WinSxS）/ F 模块（注册表）** 依赖本任务的提权底座，后续只需往 `ALLOWED_COMMANDS` 加命令 + 实现对应的 `match` 分支。

### 三句话结论

1. 提权链路两端都通了：exe 侧白名单 + 零依赖 FFI，主进程侧 `ShellExecuteExW` + 退出码/JSON 双通道；`tauri build` 的**体积增量**（4.05→4.22 MiB）是「真的进包了」的硬证据。
2. 任务书有三处技术错误：`tauri-plugin-tray` 不存在、`ShellExecuteW` 拿不到退出码、白名单漏了 8 个级联文件——都按事实改了。
3. 还原点只对 L2 触发（全 L1 弹 UAC 会逼用户关功能），托盘「退出」走「emit → 前端 confirm → `quit_app`」因为托盘回调里没有 WebView。

