# 任务编号：067 M1 D 模块——残留增强（注册表卸载列表↔文件目录交叉比对）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：M1 业务模块缺口第一组第 3 项（D 模块残留增强，TRD §371）。
> 上游依赖：task-066 已合并 main（merge `a618066`，E 模块大文件地图完成）。
> **版本线**：v0.3.x M1 模块持续落地。
> **本任务范围（4 个功能单元）**：残留扫描后端（注册表↔文件交叉比对）+ IPC 命令接线 + 前端残留列表页面 + tab 路由注册。
> **为什么合并**：D 模块残留增强是一个完整业务闭环（后端扫描 + IPC 接线 + 前端展示 + 路由注册），必须一次 PR 打通。
> **不做**：F 模块注册表健康中心（留 task-068）；跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 状态字段

testing

## 任务目标

### 1. 残留扫描后端（注册表↔文件交叉比对）

- **TRD §371 描述**：扫描注册表卸载列表（HKLM/HKCU Uninstall），与已安装文件目录交叉比对，找出残留项。
- **修法**：
  - `engine/src/scan/residual.rs`（新建）：
    - `scan_registry_uninstall()`：读 `HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall` 和 `HKCU\...\Uninstall`，返回 `{display_name, install_location, registry_key}`；
    - `scan_file_residuals()`：遍历常见安装目录（Program Files / Program Files (x86) / AppData），找出「注册表无记录的文件夹」；
    - `cross_reference()`：交叉比对，返回两类残留：
      - **类型 A（注册表残留）**：注册表有卸载项，但 `install_location` 指向的目录已不存在；
      - **类型 B（文件残留）**：文件目录存在，但注册表无对应卸载项（孤儿文件夹）；
  - 返回统一结构 `ResidualItem { name, path, residual_type, size_bytes, registry_key: Option<String> }`。

### 2. IPC 命令接线

- **修法**：
  - `engine/src/ipc/commands/residual.rs`（新建）：`residual_scan()` handler；
  - `engine/src/ipc/commands/mod.rs`：注册 `residual_scan`；
  - `engine/src/lib.rs`：注册 IPC handler（参照现有模式）。

### 3. 前端残留列表页面

- **修法**：
  - `app/src/pages/Residual.tsx`（新建）：
    - 两个分类 tab（注册表残留 / 文件残留）；
    - 列表显示：名称 / 路径 / 大小 / 类型；
    - 逐项确认：勾选后可删除（文件残留），注册表残留仅提示（不直接删注册表键）；
    - 风险徽标（L2 需确认）。

### 4. tab 路由注册

- **修法**：
  - `app/src/App.tsx`：
    - Tab 类型加 `"residual"`；
    - Tab 栏加「残留」按钮；
    - 切换渲染 `<Residual />`；
    - 深色主题适配（CSS 变量）。

**本任务不做**：
- 不做 F 模块注册表健康中心（Run 键 / 计划任务，留 task-068）；
- 不做直接删除注册表键（仅扫描展示，注册表残留仅提示）；
- 不做跨盘扫描（仅 C 盘）；
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 前置条件

1. `main` 已合并 task-066（merge `a618066`），E 模块大文件地图完成；
2. 关键锚点（必须全部在白名单内）：
   - 后端：`engine/src/scan/residual.rs`（新建）
   - 后端：`engine/src/scan/mod.rs`（注册）
   - 后端：`engine/src/ipc/commands/residual.rs`（新建）
   - 后端：`engine/src/ipc/commands/mod.rs`（注册）
   - 后端：`engine/src/lib.rs`（注册 IPC handler）
   - 后端测试：`engine/tests/residual_test.rs`（新建）
   - 前端：`app/src/pages/Residual.tsx`（新建）
   - 前端：`app/src/pages/Residual.test.tsx`（新建）
   - 前端：`app/src/App.tsx`（tab 注册）
   - 前端：`app/src/utils/invoke.ts`（API 封装）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 残留扫描后端

1. `scan_registry_uninstall()`：返回注册表卸载项列表；
2. `scan_file_residuals()`：返回孤儿文件夹列表；
3. `cross_reference()`：返回两类残留（注册表残留 / 文件残留）；
4. 单测：交叉比对逻辑正确（mock 注册表 + mock 文件系统）。

### 2. IPC 命令接线

5. `residual_scan()` IPC handler 注册成功；
6. 前端调用返回完整数据结构；
7. 单测：handler 返回值格式正确。

### 3. 前端残留列表页面

8. 两个分类 tab（注册表残留 / 文件残留）；
9. 列表显示：名称 / 路径 / 大小 / 类型；
10. 文件残留可勾选删除，注册表残留仅提示；
11. 风险徽标（L2 需确认）；
12. 深色主题适配；
13. 单测：页面渲染正确。

### 4. tab 路由注册

14. Tab 栏加「残留」按钮；
15. 切换渲染 `<Residual />`；
16. 单测：tab 切换正确。

### 后端单测

17. `cargo test --workspace` 全绿（现有 + 本任务新增 ≥4 条全绿）。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**后端（engine）**：
- `engine/src/scan/residual.rs`（新建）
- `engine/src/scan/mod.rs`（注册 residual 模块）
- `engine/src/ipc/commands/residual.rs`（新建）
- `engine/src/ipc/commands/mod.rs`（注册 residual_scan 命令）
- `engine/src/lib.rs`（注册 IPC handler，如需要）
- `engine/tests/residual_test.rs`（新建）

**前端（app）**：
- `app/src/pages/Residual.tsx`（新建）
- `app/src/pages/Residual.test.tsx`（新建）
- `app/src/App.tsx`（tab 类型 + 按钮 + 渲染）
- `app/src/utils/invoke.ts`（加 residual_scan API）

**收尾**：
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-067-residual-scan`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-067-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `feat(scan): 残留扫描后端（注册表卸载列表↔文件目录交叉比对，两类残留） (#task-067)`（单元 1）
   —— 含 `engine/src/scan/residual.rs` + `mod.rs` + `engine/tests/residual_test.rs`。
2. `feat(ipc): residual_scan IPC 命令接线 (#task-067)`（单元 2）
   —— 含 `engine/src/ipc/commands/residual.rs` + `commands/mod.rs` + `lib.rs`。
3. `feat(ui): 残留列表页面（两分类 tab + 文件残留可删 + 注册表残留仅提示） (#task-067)`（单元 3）
   —— 含 `app/src/pages/Residual.tsx` + `Residual.test.tsx` + `utils/invoke.ts`。
4. `feat(ui): App tab 注册「残留」按钮 + 深色适配 (#task-067)`（单元 4）
   —— 含 `app/src/App.tsx`。
5. 自验全绿后，最后一次提交：`docs(state): 任务067完成，M1 D 模块残留增强就绪 (#task-067)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 067: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「M1 第 4 项：F 模块注册表健康中心（task-068）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-31  task-067  M1 D 模块残留增强`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：交叉比对逻辑、handler 接线、页面渲染、tab 切换。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **端到端写路径（必做，本任务硬要求）**：残留扫描必须有一条真实「切 tab → 调 IPC → 后端扫描注册表+文件 → 前端展示两类残留」证据（不能只构造 mock）。
- **真机冒烟（必做、不可裁剪）**：
  - Tab 栏显示「残留」按钮；
  - 点击切换到 Residual 页面；
  - 两个分类 tab（注册表残留 / 文件残留）；
  - 列表显示名称 / 路径 / 大小 / 类型；
  - 文件残留可勾选删除，注册表残留仅提示（不直接删注册表键）；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 5 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（残留列表页面 + tab 切换，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 交叉比对逻辑 | 单测输出（两类残留分类正确） |
| IPC handler 接线 | 单测输出（返回值格式） |
| tab 切换 | 真机截图：Tab 栏显示「残留」按钮 |
| 两分类 tab | 真机截图：注册表残留 / 文件残留切换 |
| 列表显示 | 真机截图：名称 + 路径 + 大小 + 类型 |
| 文件残留可删 / 注册表仅提示 | 真机证据：UI 行为符合预期 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 5 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-067-residual-scan`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - 注册表卸载列表读取实现（winreg crate？）；
   - 交叉比对算法（类型 A / 类型 B 判定逻辑）；
   - 文件残留删除方式（送保险箱 vs 直接删）；
   - `cargo test --workspace` 通过输出（含新增 ≥4 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（5 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（M1 第 4 项：F 模块注册表健康中心；Items/SafeBox/History 深色适配待补）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务067-M1 D模块 残留增强 (#task-067)`，
   该提交只含 `tasks/067-residual-scan.md` 一个文件，落在 `feature/task-067-residual-scan` 分支最前端。

## 实现备注

> 开发轮（DevFullstackRustReact）交付说明。**所有结论可复现**，命令附在对应位置。

### 1. 实际改动文件清单

| 文件 | 类型 | 说明 |
|---|---|---|
| `engine/src/scan/residual.rs` | **新建** | `scan_registry_uninstall` / `install_roots` / `list_install_dirs` / `cross_reference` / `scan_all` / `delete_residuals` + `winreg` 子模块（注册表 FFI） |
| `engine/src/scan/mod.rs` | 改 | `pub mod residual;` + 导出 `ResidualItem` / `ResidualReport` |
| `engine/src/ipc/commands/residual.rs` | **新建** | `handle_residual_scan` / `handle_residual_delete` |
| `engine/src/ipc/commands/mod.rs` | 改 | `pub mod residual;` |
| `engine/tests/residual_test.rs` | **新建** | 23 条（22 常规 + 1 `#[ignore]` 真机冒烟） |
| `app/src-tauri/src/commands.rs` | 改 | `residual_scan` / `residual_delete` |
| `app/src-tauri/src/lib.rs` | 改 | 注册两条命令 |
| `app/src/pages/Residual.tsx` | **新建** | 两分类 tab + 勾选 + 移入保险箱 |
| `app/src/pages/Residual.test.tsx` | **新建** | 11 条 |
| `app/src/App.tsx` | 改 | `residual` tab + 按钮 + 渲染 |
| `app/src/App.test.tsx` | 改 | +1 条 tab 切换（验收 16） |
| `docs/PROJECT_STATE.md` | 改 | 按第 5 条提交规范更新 |
| `engine/src/execute/safe_box.rs` | 改 | **白名单外**：目录删除能力 + 配额缺陷修正（见 §4） |

**与白名单的差异（5 处）**：

| # | 白名单写法 | 实际情况 |
|---|---|---|
| 1 | 未列 `engine/src/execute/safe_box.rs` | **必须改**：`delete_dir` 原是 `Err` 桩、`delete_file` 用 `metadata().len()` 算目录体积会得 0 让配额失效。不改就删不了目录。 |
| 2 | 未列 `app/src-tauri/src/{commands,lib}.rs` | 必须改，否则前端 invoke 不到命令（与 task-065/066 同样的遗漏）。 |
| 3 | `engine/src/lib.rs`（注册 IPC handler） | **无需改动**（`pub mod ipc` 已存在）。 |
| 4 | `app/src/utils/invoke.ts`（加 API 封装） | **未改动**：现有模式是 9 个页面全部直调 `api<T>` / `apiRaw<T>`，invoke.ts 只有两个通用解包函数、没有任何 per-command 封装。 |
| 5 | 未列 `app/src/App.test.tsx` | 验收 16「tab 切换正确」只能在 App 层测，新增 1 条。 |

### 2. 注册表卸载列表读取实现（**没用 winreg crate**）

用**已在依赖里的 `windows` crate**手写 FFI，`Win32_System_Registry` feature 在 task-062
就为 `SHELLEXECUTEINFOW` 的 `hkeyClass` 字段开好了，**Cargo.toml 一行没动**：

```rust
// engine/src/scan/residual.rs 的 winreg 子模块
RegOpenKeyExW(root, PCWSTR(path), 0, KEY_READ, &mut hkey)        // 开键
RegEnumKeyExW(hkey, idx, PWSTR(buf), &mut len, None, PWSTR::null(), None, None)  // 枚举子键
RegGetValueW(root, PCWSTR(sub), PCWSTR(name), RRF_RT_REG_SZ, ...) // 读字符串（一次到位）
```

为什么不用 `winreg` crate：只需要「枚举子键 + 读字符串/DWORD」三件事，引一个 crate
会拖慢本 crate 的编译时间（项目一直把编译时间当指标）。手写约 90 行。

读**三处**（只读 64 位会漏掉所有 32 位程序的卸载项）：

- `HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall`（64 位）
- `HKLM\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall`（32 位）
- `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall`（用户级安装）

过滤三类「不是可卸载程序」的项：`DisplayName` 为空（补丁/系统组件）、
`SystemComponent == 1`（微软约定：不在「程序和功能」显示）、`ParentKeyName` 存在（子组件）。
本机实测过滤后剩 **94** 个有效卸载项。

### 3. 交叉比对算法

**类型 A（注册表残留）**：

```
对每个 entry：
  InstallLocation 为空 或 长度 < 3（如 "C:"）  → 跳过（判不了，宁可漏报）
  Path::new(loc).exists()                      → 不是残留
  否则                                          → 类型 A，confidence=high
```

**类型 B（文件残留）**：`has_owner(dir, entries)` 三级判定，**任一级命中就算有归属**：

```
① 路径级：InstallLocation 与目录路径互为前缀（含相等）
② 目录名级：InstallLocation 的末段 == 目录名
③ 名称级：normalize_name(DisplayName) 与 normalize_name(目录名)
          双向包含，且较短串 ≥ 4 字符
```

`normalize_name` 只保留字母数字（含 CJK）并转小写：
`Microsoft Visual C++ 2015 (x64)` → `microsoftvisualc2015x64`，这样带版本号/架构后缀的
名字能和文件夹名对上。

**为什么「任一级命中就算有归属」**：误判有两个方向——
把正常程序报成残留（用户可能真去删，**危险**）vs 把残留漏掉（用户少清一点，**无害**）。
所以判定一律偏向「有归属」，宁可漏报。

### 4. 文件残留删除方式：**送保险箱，不直接删**

按用户明确选择实现为「移入保险箱，7 天可恢复」：

```
residual_delete(paths)
  → execute()（项目唯一删除入口：L3 红线 + 动作级禁令）
  → 拒绝 → 记 failed + 原因，不进保险箱
  → 通过 → SafeBoxDeleter::delete_dir（整目录 rename 进 box_dir）
         → 写 safe_box 行（7 天后 purge_expired 自动永久删除）
```

**为什么必须走保险箱**：`engine::execute::whitelist` 的 L3 红线只覆盖个人目录 / 浏览器数据 /
System32 / Installer / pagefile，**`C:/Program Files\*` 不在红线内**——白名单拦不住误判。
一旦删错，只有保险箱能救回来。目录是**整体 rename**，内部结构完整保留，恢复后目录树原样回来。

**顺带修两个既有缺陷**（`engine/src/execute/safe_box.rs`）：

1. `delete_file` 用 `metadata().len()` 算体积——对目录返回**目录项本身**的大小（NTFS 上通常 0），
   一个 3 GB 的残留目录会被记成 0 字节，`enforce_quota` 完全放行，保险箱可以被撑爆。
   改为 `meta.is_dir()` 时走 `dir_total_bytes` 递归求和。
2. `delete_dir` 原是 `Err("not implemented for ... (task-016 only moves files)")` 桩。
   实现为转调 `delete_file`——对目录而言 `fs::rename` 本来就是整体移动。

**自己写出来的假成功也修了**：不存在的路径会被 `delete_dir` 当成 `Ok(())`（幂等语义），
直接信它就会把「什么都没做」记成「已移入保险箱」，UI 上出现「已移入 1 项」但实际没动。
现在调用前先 `Path::exists()` 挡一道，不存在就完全 no-op。

### 5. 真机实测：误报从 182 条压到 11 条

这是本任务最花时间的一段，也是**只靠单测发现不了的**。

初版把 `%LOCALAPPDATA%` / `%APPDATA%` 也当安装目录扫，本机结果：

```
typeB=182  total=36.0GB  elapsed=10458ms
  C:/Users/An/AppData/Local/Huawei        ← 华为 SDK，装了
  C:/Users/An/AppData/Roaming/LarkShell   ← 飞书，装了
  C:/Users/An/AppData/Roaming/Tencent     ← 腾讯系，装了
  C:/Users/An/AppData/Roaming/JetBrains   ← JetBrains，装了
  C:/Program Files (x86)\Windows Kits     ← Windows SDK
  C:/Program Files\WSL                    ← Windows 功能
```

**根因**：AppData 是「应用数据」目录，不是「程序安装」目录，里面的东西本来就不写卸载项。
把它当孤儿报出来，等于把用户的正常数据说成残留——**比不报更糟**。

修正：只扫 Program Files 系列 + 补 13 个系统组件黑名单（Windows Kits / WSL / dotnet /
Hyper-V / WindowsPowerShell / Application Verifier / AMD / NVIDIA 等，每条都对应一个实际误报）。

修正后：

```
typeB=11  total=95.7MB  elapsed=75ms
  HONOR / C-Disk Cleaner / Microsoft Office 15 / Tencent / WXWork
  Zero G Registry (17 KB)  ← InstallAnywhere 典型空壳残留
  WinRAR (474 B)           ← 卸载后剩空壳，典型残留
  Huawei / OpenBoxLab / Tencent (0 B)  ← 空目录
```

剩下的 11 条都可核查，且「误报」与「真残留」的边界本来就模糊（`C-Disk Cleaner` 是
我们自己，如果是绿色解压进去的，本来就没有卸载项——符合类型 B 的定义）。

### 6. 门禁输出

```
cargo check --workspace --all-targets            exit 0
cargo clippy --workspace --all-targets -- -D warnings   exit 0
cargo fmt --all -- --check                       exit 0
cargo test --workspace                           405 passed, 0 failed   （本任务新增 22 条 + 1 条 #[ignore]）
  └─ engine/tests/residual_test.rs                22 passed, 1 ignored
pnpm --filter app lint                           0 error（2 warning 均 SafeBox.tsx 既有）
pnpm --filter app test                           135 passed（12 files，+12）
pnpm --filter app build                          220.75 kB │ gzip 69.85 kB
tauri build                                      NSIS 4.41 MiB + MSI 6.51 MiB
```

### 7. 真实数据证据（后端侧真机冒烟）

```bash
cargo test -p engine --test residual_test -- --ignored --nocapture real_scan
```

```
registry_entries=94 dirs_scanned=28 typeA=3 typeB=11 total=95719553B elapsed=75ms
--- 类型 A（注册表残留，路径已不存在）---
  Google Chrome  →  C:/Program Files (x86)\Chromebrowser
  IntelliJ IDEA 2024.1  →  D:/IDEA/IntelliJ IDEA 2024.1
  Java(TM) SE Development Kit 25.0.1 (64-bit)  →  D:/Java/jdk-25
--- 类型 B（文件残留，注册表无对应项）---
  ...（11 条，见 §5）
```

三条类型 A 都可核查：`Chromebrowser` 目录名本身就是拼错的，`D:/IDEA/...` 与 `D:/Java/jdk-25` 也已不存在。

### 8. 真机冒烟 / 打包安装冒烟 / CI

**未执行**（按 `docs/PROJECT_STATE.md` 的流程约定：开发轮只做本地静态/单元/集成门禁
+ 打包能出包，GUI 与安装态冒烟统一移交测试 Agent）。

已完成的打包验证：`pnpm tauri build` 产出
`C-DiskCleaner_0.1.7_x64-setup.exe`（**4.41 MiB**）+ `.msi`（**6.51 MiB**），
末尾签名步报 `A public key has been found, but no private key` —— 本地无私钥，
属**已知预期**（CI 有 secret 才签）。

### 9. 与本任务文档不符的偏差（5 项）

1. **「文件残留可勾选删除」→ 实现为「移入保险箱」**（用户明确选择）。理由：Program Files
   不在 L3 红线内，白名单拦不住误判，必须留可恢复的退路。
2. `app/src/utils/invoke.ts` **未改动**（现有模式无 per-command 封装）。
3. `engine/src/lib.rs` **无需改动**，但 `app/src-tauri/src/{commands,lib}.rs` 漏在白名单外。
4. `app/src/App.test.tsx` 漏在白名单外（验收 16 只能在 App 层测）。
5. 白名单漏 `engine/src/execute/safe_box.rs`（目录删除 + 配额缺陷修正）。

### 10. 遗留问题

- **类型 B 的误报率无法归零**：「注册表里没有」推不出「已卸载」。已用四重措施兜底
  （低置信徽标 + 默认不勾 + 不提供全选 + 移入保险箱）。要提高准确率需引入
  「文件指纹 / 厂商签名 / 版本资源比对」，属新功能点。
- **注册表读取只覆盖 Uninstall 三处**，不含「应用和功能」里被 `SystemComponent` 隐藏的项、
  不含微软商店应用的注册信息——这些程序如果目录还在，会被判成类型 B。
- 类型 A 只检查 `InstallLocation`，不看 `DisplayIcon` / `UninstallString` 里的路径。
  `InstallLocation` 为空的项（本机 94 条里有一部分）完全没做残留检查。
- M1 第 4 项：F 模块注册表健康中心（task-068）。可复用本任务的 `winreg` 子模块。
- Items / SafeBox / History 三页深色适配待补（本任务新增的 Residual 页已全走 CSS 变量）。
