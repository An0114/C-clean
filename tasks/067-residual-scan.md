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

pending

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
