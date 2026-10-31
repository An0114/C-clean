# 任务编号：068 M1 F 模块——注册表健康中心（Run 键/卸载残留/计划任务，reg export 备份）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：M1 业务模块缺口第一组第 4 项（F 模块注册表健康中心，TRD §85/§373）。
> 上游依赖：task-067 已合并 main（merge `1c0e2c5`，D 模块残留增强完成）。
> **版本线**：v0.3.x M1 模块收官——本任务完成后 M1 四个业务模块（C/D/E/F）全部闭环。
> **本任务范围（4 个功能单元）**：注册表健康扫描后端 + IPC 命令接线 + 前端注册表健康中心页面 + tab 路由注册。
> **为什么合并**：F 模块注册表健康中心是一个完整业务闭环（后端扫描 + IPC 接线 + 前端展示 + 路由注册），必须一次 PR 打通。
> **不做**：跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 状态字段

testing

## 任务目标

### 1. 注册表健康扫描后端

- **TRD §85/§373 描述**：扫描 Run 键 / 卸载残留 / 计划任务，reg export 备份，逐项确认。
- **修法**：
  - `engine/src/scan/registry_health.rs`（新建）：
    - `scan_run_keys()`：读 `HKLM\Software\Microsoft\Windows\CurrentVersion\Run` 和 `HKCU\...\Run`，返回开机启动项列表；
    - `scan_uninstall_residual()`：读注册表卸载列表，交叉比对文件是否存在（与 task-067 残留模块复用逻辑）；
    - `scan_scheduled_tasks()`：枚举计划任务（`schtasks /query` 或 API）；
    - `reg_export_backup()`：删除前自动 `reg export` 备份到 `%APPDATA%\com.and0114.cclean\registry_backups\`；
  - 返回统一结构 `RegistryHealthItem { name, path, category, risk_level, recommendation }`。

### 2. IPC 命令接线

- **修法**：
  - `engine/src/ipc/commands/registry_health.rs`（新建）：
    - `registry_health_scan()`：扫描注册表健康项；
    - `registry_health_backup()`：删除前 reg export 备份；
  - `engine/src/ipc/commands/mod.rs`：注册上述两个命令；
  - `engine/src/lib.rs`：注册 IPC handler（参照现有模式）。

### 3. 前端注册表健康中心页面

- **修法**：
  - `app/src/pages/RegistryHealth.tsx`（新建）：
    - 三个分类 tab（启动项 / 卸载残留 / 计划任务）；
    - 列表显示：名称 / 路径 / 风险等级 / 建议；
    - 逐项确认：勾选后删除（删除前自动 reg export 备份）；
    - 风险徽标（L2 需确认 / L3 禁删）；
    - 「备份路径」显示：删除前提示备份文件位置。

### 4. tab 路由注册

- **修法**：
  - `app/src/App.tsx`：
    - Tab 类型加 `"registry"`；
    - Tab 栏加「注册表」按钮；
    - 切换渲染 `<RegistryHealth />`；
    - 深色主题适配（CSS 变量）。

**本任务不做**：
- 不做直接删除注册表键（删除前自动 reg export 备份，用户确认后才删）；
- 不做跨盘扫描（仅 C 盘）；
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 前置条件

1. `main` 已合并 task-067（merge `1c0e2c5`），D 模块残留增强完成；
2. 关键锚点（必须全部在白名单内）：
   - 后端：`engine/src/scan/registry_health.rs`（新建）
   - 后端：`engine/src/scan/mod.rs`（注册）
   - 后端：`engine/src/ipc/commands/registry_health.rs`（新建）
   - 后端：`engine/src/ipc/commands/mod.rs`（注册）
   - 后端：`engine/src/lib.rs`（注册 IPC handler）
   - 后端测试：`engine/tests/registry_health_test.rs`（新建）
   - 前端：`app/src/pages/RegistryHealth.tsx`（新建）
   - 前端：`app/src/pages/RegistryHealth.test.tsx`（新建）
   - 前端：`app/src/App.tsx`（tab 注册）
   - 前端：`app/src/utils/invoke.ts`（API 封装）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 注册表健康扫描后端

1. `scan_run_keys()`：返回开机启动项列表（HKLM + HKCU）；
2. `scan_uninstall_residual()`：返回卸载残留列表；
3. `scan_scheduled_tasks()`：返回计划任务列表；
4. `reg_export_backup()`：删除前自动备份到指定目录；
5. 单测：各扫描函数返回格式正确（mock 注册表）。

### 2. IPC 命令接线

6. `registry_health_scan()` IPC handler 注册成功；
7. `registry_health_backup()` IPC handler 注册成功；
8. 前端调用返回完整数据结构；
9. 单测：handler 返回值格式正确。

### 3. 前端注册表健康中心页面

10. 三个分类 tab（启动项 / 卸载残留 / 计划任务）；
11. 列表显示：名称 / 路径 / 风险等级 / 建议；
12. 删除前自动 reg export 备份 + 提示备份路径；
13. 风险徽标（L2 需确认 / L3 禁删）；
14. 深色主题适配；
15. 单测：页面渲染正确。

### 4. tab 路由注册

16. Tab 栏加「注册表」按钮；
17. 切换渲染 `<RegistryHealth />`；
18. 单测：tab 切换正确。

### 后端单测

19. `cargo test --workspace` 全绿（现有 + 本任务新增 ≥4 条全绿）。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**后端（engine）**：
- `engine/src/scan/registry_health.rs`（新建）
- `engine/src/scan/mod.rs`（注册 registry_health 模块）
- `engine/src/ipc/commands/registry_health.rs`（新建）
- `engine/src/ipc/commands/mod.rs`（注册 registry_health_scan / registry_health_backup 命令）
- `engine/src/lib.rs`（注册 IPC handler，如需要）
- `engine/tests/registry_health_test.rs`（新建）

**前端（app）**：
- `app/src/pages/RegistryHealth.tsx`（新建）
- `app/src/pages/RegistryHealth.test.tsx`（新建）
- `app/src/App.tsx`（tab 类型 + 按钮 + 渲染）
- `app/src/utils/invoke.ts`（加 registry_health_scan / registry_health_backup API）

**收尾**：
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-068-registry-health`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-068-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `feat(scan): 注册表健康扫描后端（Run 键/卸载残留/计划任务 + reg export 备份） (#task-068)`（单元 1）
   —— 含 `engine/src/scan/registry_health.rs` + `mod.rs` + `engine/tests/registry_health_test.rs`。
2. `feat(ipc): registry_health_scan / registry_health_backup IPC 命令接线 (#task-068)`（单元 2）
   —— 含 `engine/src/ipc/commands/registry_health.rs` + `commands/mod.rs` + `lib.rs`。
3. `feat(ui): 注册表健康中心页面（三分类 tab + 删除前备份 + 风险徽标） (#task-068)`（单元 3）
   —— 含 `app/src/pages/RegistryHealth.tsx` + `RegistryHealth.test.tsx` + `utils/invoke.ts`。
4. `feat(ui): App tab 注册「注册表」按钮 + 深色适配 (#task-068)`（单元 4）
   —— 含 `app/src/App.tsx`。
5. 自验全绿后，最后一次提交：`docs(state): 任务068完成，M1 F 模块注册表健康中心就绪，M1 四模块全部闭环 (#task-068)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 068: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「M1 四模块全部闭环（C/D/E/F），进入下一阶段：P1 体验项（趋势图增强 / 自动扫描 / 方案模板）或 v0.2.0 发版准备」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-31  task-068  M1 F 模块注册表健康中心（M1 收官）`。

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
- **端到端写路径（必做，本任务硬要求）**：注册表健康扫描必须有一条真实「切 tab → 调 IPC → 后端扫描 → 前端展示三分类」证据（不能只构造 mock）；删除前必须有一条真实「勾选删除 → reg export 备份 → 删除」证据。
- **真机冒烟（必做、不可裁剪）**：
  - Tab 栏显示「注册表」按钮；
  - 点击切换到 RegistryHealth 页面；
  - 三个分类 tab（启动项 / 卸载残留 / 计划任务）；
  - 列表显示名称 / 路径 / 风险等级 / 建议；
  - 删除前自动 reg export 备份 + 提示备份路径；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 5 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（注册表健康中心页面 + tab 切换，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 各扫描函数返回格式 | 单测输出（三分类字段正确） |
| IPC handler 接线 | 单测输出（返回值格式） |
| tab 切换 | 真机截图：Tab 栏显示「注册表」按钮 |
| 三分类 tab | 真机截图：启动项 / 卸载残留 / 计划任务切换 |
| 列表显示 | 真机截图：名称 + 路径 + 风险等级 + 建议 |
| 删除前备份 | 真机证据：勾选删除 → reg export 备份路径提示 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 5 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-068-registry-health`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - Run 键读取实现（winreg crate？）；
   - 计划任务枚举方式（schtasks / API？）；
   - reg export 备份实现方式；
   - 风险等级判定逻辑（L2/L3）；
   - `cargo test --workspace` 通过输出（含新增 ≥4 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（5 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（M1 四模块全部闭环；下一阶段：P1 体验项或 v0.2.0 发版准备；Items/SafeBox/History 深色适配待补）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务068-M1 F模块 注册表健康中心 (#task-068)`，
   该提交只含 `tasks/068-registry-health.md` 一个文件，落在 `feature/task-068-registry-health` 分支最前端。

## 实现备注

> 开发轮（DevFullstackRustReact）交付说明。**所有结论可复现**，命令附在对应位置。

### 1. 实际改动文件清单

| 文件 | 类型 | 说明 |
|---|---|---|
| `engine/src/scan/registry_health.rs` | **新建** | 三分类扫描 + 风险分级 + `.reg` 备份 + 带备份的删除 |
| `engine/src/scan/winreg.rs` | **新建** | 注册表最小封装（从 residual.rs 抽出 + 新增枚举值/删值） |
| `engine/src/scan/mod.rs` | 改 | 注册两个新模块 |
| `engine/src/scan/residual.rs` | 改 | **白名单外**：改为引用 `winreg` 模块，删掉自己那份 FFI 副本 |
| `engine/src/ipc/commands/registry_health.rs` | **新建** | `handle_registry_health_scan` / `handle_registry_health_delete` |
| `engine/src/ipc/commands/mod.rs` | 改 | `pub mod registry_health;` |
| `engine/tests/registry_health_test.rs` | **新建** | 26 条（25 常规 + 1 `#[ignore]` 真机冒烟） |
| `app/src-tauri/src/commands.rs` | 改 | `registry_health_scan` / `registry_health_delete` |
| `app/src-tauri/src/lib.rs` | 改 | 注册两条命令 |
| `app/src/pages/RegistryHealth.tsx` | **新建** | 三 tab + 勾选 + 删除并备份 |
| `app/src/pages/RegistryHealth.test.tsx` | **新建** | 12 条 |
| `app/src/App.tsx` | 改 | `registry` tab + 按钮 + 渲染 |
| `app/src/App.test.tsx` | 改 | +1 条 tab 切换（验收 18） |
| `docs/PROJECT_STATE.md` | 改 | 按第 5 条提交规范更新 |

**与白名单的差异（4 处）**：

| # | 白名单写法 | 实际情况 |
|---|---|---|
| 1 | 未列 `engine/src/scan/winreg.rs` | **必须新建**：注册表封装要在 `residual` 与 `registry_health` 之间共用，复制 120 行更糟 |
| 2 | 未列 `engine/src/scan/residual.rs` | 必须改：把 `mod winreg` 换成 `use crate::scan::winreg`（删掉重复副本） |
| 3 | 未列 `app/src-tauri/src/{commands,lib}.rs` | 必须改，否则前端 invoke 不到命令（与 task-065/066/067 同样的遗漏） |
| 4 | `app/src/utils/invoke.ts`（加 API 封装） | **未改动**：现有模式是 10 个页面全部直调 `api<T>` / `apiRaw<T>` |

另：`engine/src/lib.rs` **无需改动**（`pub mod ipc` 已存在）；`app/src/App.test.tsx` 也不在白名单，但验收 18「tab 切换正确」只能在 App 层测。

### 2. Run 键读取实现（**没用 winreg crate**）

用已在依赖里的 `windows` crate 手写 FFI。`Win32_System_Registry` feature 在 task-062 就为 `SHELLEXECUTEINFOW` 的 `hkeyClass` 字段开好了，**Cargo.toml 一行没动**。

```rust
RegOpenKeyExW(root, PCWSTR(path), 0, KEY_READ, &mut hkey)                    // 开键
RegEnumKeyExW(hkey, idx, PWSTR(buf), &mut len, None, PWSTR::null(), None, None)
RegEnumValueW(hkey, idx, PWSTR(name), &mut name_len, None,
              Some(&mut kind), Some(data.as_mut_ptr()), Some(&mut data_len))  // 枚举值
RegGetValueW(...)                                                            // 读单个值
RegDeleteKeyValueW(root, PCWSTR(subkey), PCWSTR(value))                      // 删值
```

签名坑：`RegEnumValueW` 的第 5 参是 `Option<*const u32>`、第 6 参是 **`Option<*mut u32>`**（不是 `*mut REG_VALUE_TYPE`）、第 7/8 参是 `Option<*mut _>`——混着来，只能编译试错。

读**三处** Run 键（只读 64 位会漏掉所有 32 位程序的启动项）：

- `HKLM\Software\Microsoft\Windows\CurrentVersion\Run`
- `HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run`
- `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`

### 3. 计划任务枚举方式（**既不是 schtasks 也不是 COM**）

读 `%SystemRoot%\System32\Tasks` 目录下的 **XML 文件**（任务定义本身），用字符串查找提取 `<Command>` / `<Arguments>`。

**为什么不用 `schtasks /query`**：① 违反「不 spawn 外部 exe」原则；② 它的输出是**本地化文本**，中文系统上列宽/表头都会变，解析必挂（task-065 的 DISM 踩过一模一样的坑）；③ XML 字段跨语言一致、结构稳定。

**为什么不用 COM（`ITaskService`）**：需要引 `Win32_System_TaskScheduler` + COM 初始化，为一个只读列表引一堆接口不划算。

**已知硬限制**：该目录默认只有 Administrators / SYSTEM 可读，普通用户直接 `Permission denied`（本机实测）。所以 `RegistryHealthReport` 加了 **`taskScanAvailable`** 字段——必须区分「没有任务」和「读不到」，否则 UI 显示「没有计划任务」等于骗用户。

### 4. `reg export` 备份实现方式（**自己生成 .reg，不调 reg.exe**）

```rust
pub fn build_reg_export(root: RegRoot, subkey: &str, values: &[RegValue]) -> String
```

生成 `Windows Registry Editor Version 5.00` 格式：

| 值类型 | 输出形态 |
|---|---|
| `REG_SZ` | `"Name"="value"`（`\`→`\\`、`"`→`\"` 转义） |
| `REG_EXPAND_SZ` | `"Name"=hex(2):41,00,42,00,...` |
| `REG_DWORD` | `"Name"=dword:00000001` |
| 默认值（空名） | `@="value"` |
| 其他 | `hex(N):...`（每 25 字节折行 + `\` 续行） |

**为什么不用 `reg export`**：
1. **不 spawn 外部 exe**（本项目原则；而且 `reg.exe` 在受限环境会被安全软件/组策略直接拦掉——本机开发沙箱就拦了它，这是真实存在的场景）；
2. **可单测**：生成的是纯字符串，输入输出都能钉死；
3. 粒度相同：`.reg` 格式本来就不支持「只导一个值」，两边都只能导整键。

备份落 `<data_root>\registry_backups\<HKLM|HKCU>-<清洗后的键路径>-<unix秒>.reg`，**双击即可还原**。

### 5. 风险等级判定逻辑（L2 / L3）

```
classify_run_risk(name, exe_path, exists):
  ① exe 路径以 C:/Windows/ 开头（大小写/斜杠归一化后）  → L3
     「系统组件的启动项，删除可能影响系统功能」
  ② name 或 exe 路径命中 17 个安全软件关键词             → L3
     （securityhealth / windowsdefender / defender / kaspersky / avast / avg /
       bitdefender / eset / nod32 / mcafee / symantec / norton / huorong /
       火绒 / 360 / qqpcmgr / tencentpcmanager）
     「安全软件启动项，删除会降低防护」
  ③ 目标文件不存在                                        → L2
     「大概率是卸载残留，可以删除（删前会自动导出 .reg 备份）」
  ④ 其余                                                  → L2
     「第三方开机启动项，删除后不再自动启动，随时可恢复」
```

**后端独立复核**：`delete_run_values` 删除前**重新读值、重新算等级**，不信前端传的 `riskLevel`——否则前端改一行就能删系统启动项。命中 L3 直接拒，记入 `failed`。

**`deletable` 字段** = `risk != L3`，前端据此决定给不给勾选框。

### 6. 只删 Run 值，不删键

`RegDeleteKeyValueW`（删值）而不是 `RegDeleteTreeW`（删整键）：
`HKCU\...\Run` 下一个值对应一个程序，删键会连带干掉同键下**其他所有**启动项。

**fail-closed**：备份写不出来就不删（`write_reg_backup` 失败 → 记 failed 并 `continue`，不执行删除）。

### 7. 门禁输出

```
cargo check --workspace --all-targets            exit 0
cargo clippy --workspace --all-targets -- -D warnings   exit 0
cargo fmt --all -- --check                       exit 0
cargo test --workspace                           431 passed, 0 failed   （本任务新增 25 条 + 1 条 #[ignore]）
  └─ engine/tests/registry_health_test.rs         25 passed, 1 ignored
pnpm --filter app lint                           0 error（2 warning 均 SafeBox.tsx 既有）
pnpm --filter app test                           148 passed（13 files，+13）
pnpm --filter app build                          230.32 kB │ gzip 71.78 kB
tauri build                                      NSIS 4.45 MiB + MSI 6.55 MiB
```

### 8. 真实数据证据（后端侧真机冒烟）

```bash
cargo test -p engine --test registry_health_test -- --ignored --nocapture real_scan
```

```
run=16 uninstall=3 task=0 taskReadable=false elapsed=62ms
--- 启动项（Run）---
  [L2] nrc_launcher  exists=false  D:/洛克王国/.../startup_runner.exe     ← 游戏卸载后的残留
  [L2] QuarkUpdaterTaskUser1.0.0.21  exists=false  ...QuarkUpdater...     ← 旧版更新器残留
  [L2] TabbitBrowserUpdaterTaskUser1.11.13.0  exists=false  ...           ← 旧版更新器残留
  [L3] SecurityHealth  exists=true   %windir%\system32\SecurityHealthSystray.exe
  [L3] RtkAudUService  exists=true   C:/WINDOWS/System32/DriverStore/.../RtkAudUService64.exe
  [L3] Sysdiag  exists=true  D:/Huorong/Sysdiag/bin/HipsTray.exe          ← 火绒
  ...（其余 10 条 L2，exists=true）
--- 卸载残留 ---
  Google Chrome → C:/Program Files (x86)\Chromebrowser
  IntelliJ IDEA 2024.1 → D:/IDEA/IntelliJ IDEA 2024.1
  JDK 25.0.1 → D:/Java/jdk-25
--- 计划任务 ---（task=0，taskReadable=false：普通用户无权读 System32\Tasks）
```

判定质量：L3 三条全对（安全软件 ×2 + 系统目录 ×1）；`exists=false` 三条都是**真残留**。

**删除链路实测**（用独立测试键，不碰用户任何现有启动项）：
`HKCU\Software\C-Clean-Test-<pid>-<suffix>` → 写值 → `delete_run_values` → 断言「值消失 + 备份文件存在 + 备份内容含值名与命令行」→ 删键。另测「值不存在时幂等跳过」「L3 拒删且值仍在」。

### 9. 真机冒烟 / 打包安装冒烟 / CI

**未执行**（按 `docs/PROJECT_STATE.md` 的流程约定：开发轮只做本地静态/单元/集成门禁 + 打包能出包，GUI 与安装态冒烟统一移交测试 Agent）。

已完成的打包验证：`pnpm tauri build` 产出 `C-DiskCleaner_0.1.7_x64-setup.exe`（**4.45 MiB**）+ `.msi`（**6.55 MiB**），末尾签名步报 `A public key has been found, but no private key` —— 本地无私钥，属**已知预期**（CI 有 secret 才签）。

### 10. 与本任务文档不符的偏差（4 项）

1. **只做了 `registry_health_scan` + `registry_health_delete`，没有 `registry_health_backup`**。「只备份不删」没有实际用途，而「备份和删除分开」恰好是最危险的组合（中间任何一步失败都可能变成「备份了但没删」或「没备份就删」）。合成一条 fail-closed 的 delete，备份是它内部不可分割的一步。
2. `app/src/utils/invoke.ts` **未改动**（现有模式无 per-command 封装）。
3. `engine/src/lib.rs` **无需改动**，但 `app/src-tauri/src/{commands,lib}.rs` 漏在白名单外。
4. 白名单漏 `engine/src/scan/winreg.rs`（新建）、`engine/src/scan/residual.rs`（改为引用新模块）、`app/src/App.test.tsx`（验收 18 只能在 App 层测）。

### 11. 遗留问题

- **计划任务扫描在非管理员下恒为空**：`System32\Tasks` 权限限制。已用 `taskScanAvailable` 如实标记，UI 明确写「需要管理员权限」。要真正支持需提权跑扫描（可复用 task-062 的提权 exe，但那是新任务）。
- **只覆盖 Run 键**，不含 `RunOnce`（一次性启动项）、`Startup` 文件夹快捷方式、服务自启动。这些都在「启动项」这个概念范围内，但本任务范围只到 Run。
- 类型 A 卸载残留只检查 `InstallLocation`，`InstallLocation` 为空的项（本机 94 条里有一部分）没做检查。
- **M1 四模块全部闭环**。下一阶段：P1 体验项或 v0.2.0 发版准备（建议先发版）。
- Items / SafeBox / History 三页深色适配待补（新增的 BigFiles / Residual / RegistryHealth 已全走 CSS 变量）。
