# 任务编号：011 ps1 兼容层从零编写——5 个清理脚本 + NDJSON 输出协议

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevPowerShell
> 来源：`docs/PROJECT_STATE.md` 待办 011（M1 前置清理清单第 5 条 / 最后一项，2026-10-12 debt-cleanup 批次001 拆出）。
> **用户澄清（2026-10-12）**：立项决策里说的「现有 5 个 PowerShell 脚本」实际不存在，本任务**从零编写**，而非改造已有脚本。
> **NDJSON 用途澄清**：不是 ps1 自己想流式吐，而是 **Rust 主进程 spawn ps1 子进程、逐行读 stdout、边写 cleanup_item 表、边 emit scan://progress 给前端**——所以每行 JSON 字段必须严格对齐 Rust 侧消费需求。

## 任务目标

在 `scripts/` 下从零产出：

1. **NDJSON 输出协议文档** `scripts/PROTOCOL.md`——定义 ps1 → Rust 的行协议 schema；
2. **5 个清理 ps1 脚本**——每个脚本独立可跑，stdout 逐行输出合法 NDJSON，stderr 只写日志不污染 stdout；
3. **冒烟测试**——用 `powershell -File xxx.ps1` 实跑 5 个脚本，断言 stdout 每行可被 `ConvertFrom-Json` 解析、字段齐全、`type` ∈ {item, done}。

**本任务不做**：
- 不做 Rust 侧子进程消费器（spawn / 逐行读 / 写库 / emit 进度）——留给 M1 扫描任务；
- 不做 elevation 提权调用（`elevation/` crate 留后续）；
- 不做真实删除（ps1 只**扫描发现**可清理项并输出，不删；删除走 engine::execute::delete 唯一入口）；
- 不接 Tauri、不写前端代码；
- 不改 engine/** 已有 Rust 代码（本任务纯 PowerShell + Markdown）。

## 前置条件

1. `main` 分支已含 task-001~010 全部产出；
2. `scripts/` 目录已存在（task-001 建的占位），当前为空；
3. 本机 Windows 可跑 `powershell.exe -NoProfile -File xxx.ps1`；
4. 关键锚点：
   - `docs/PRD.md`——M1 业务模块划分（A 扫描 / B 删除 / C 保险箱 / D 计划 / E 规则更新 / F 报表）；
   - `docs/ARCHITECTURE.md` §3 扫描流程图——BFS + 跳过符号链接；
   - `engine/src/db/migrations/v1__init.sql`——`cleanup_item` 表字段（id/session_id/module_code/category/path/size_bytes/risk_level），NDJSON 字段必须与之对齐。

## 需求 & 验收标准

> 每条都可机器复核。DevAgent 自验时逐条勾选，任一不过不得 push。

### A. NDJSON 协议文档 `scripts/PROTOCOL.md`

1. 文档须明确写出两种事件 schema：
   - **item 事件**（每行一个）：
     ```json
     {"type":"item","module_code":"temp","category":"user_temp","path":"C:\\Users\\an\\AppData\\Local\\Temp\\foo.tmp","size_bytes":12345,"risk_level":1}
     ```
     - `type` 恒为 `"item"`；
     - `module_code` ∈ 5 个脚本的 module_code（见 §B）；
     - `category`：脚本内部分类（如 `user_temp` / `windows_temp` / `update_download` / `prefetch` / `chrome_cache` / `edge_cache` / `npm_cache` / `pip_cache`）；
     - `path`：绝对路径字符串，反斜杠双转义；
     - `size_bytes`：整数（文件字节数；目录时输出 0 或递归大小，由脚本自选并在 PROTOCOL.md 注明）；
     - `risk_level`：1=L1 低风险可删 / 2=L2 中风险需用户确认 / 3=L3 红线（本兼容层**不输出 L3**——L3 由 Rust 侧 task-003 白名单二次过滤）。
   - **done 事件**（脚本最后一行）：
     ```json
     {"type":"done","module_code":"temp","total_items":42,"total_bytes":1234567}
     ```
2. 文档须写明：
   - stdout **只**输出 NDJSON，禁止任何人类可读日志（日志一律走 stderr）；
   - 每行一个 JSON 对象，行末 `\n`（Windows 下 PowerShell 默认 `\r\n`——DevAgent 必须在脚本里显式 `[Console]::Out.NewLine` 控制或用 `Write-Output` 后接 `\n`，并在冒烟测试里验证行分隔符不影响 `ConvertFrom-Json`）；
   - 编码：UTF-8 无 BOM（用 `[Console]::OutputEncoding = [System.Text.Encoding]::UTF8`）；
   - 字段缺失规则：禁止输出缺字段的行；
   - 错误处理：脚本遇到权限不足的子目录 → 跳过并写 stderr，**不**向 stdout 输出错误行。

### B. 5 个清理 ps1 脚本

3. `scripts/scan_temp.ps1`（module_code = `temp`）：
   - 扫描 `$env:TEMP`（用户临时目录）+ `C:\Windows\Temp`（系统临时目录）；
   - 跳过符号链接/junction（用 `Get-Item ... -Force` + `.LinkType` 判断，对齐 ARCHITECTURE R6）；
   - 输出每个文件一行 item 事件；
   - 末尾输出 done 事件。
4. `scripts/scan_windows_update.ps1`（module_code = `windows_update`）：
   - 扫描 `C:\Windows\SoftwareDistribution\Download`；
   - 权限不足时跳过并写 stderr。
5. `scripts/scan_prefetch.ps1`（module_code = `prefetch`）：
   - 扫描 `C:\Windows\Prefetch`；
   - 同上。
6. `scripts/scan_browser_cache.ps1`（module_code = `browser_cache`）：
   - 扫描 Edge：`$env:LOCALAPPDATA\Microsoft\Edge\User Data\Default\Cache`；
   - 扫描 Chrome：`$env:LOCALAPPDATA\Google\Chrome\User Data\Default\Cache`；
   - 任一目录不存在 → 跳过不报错。
7. `scripts/scan_dev_cache.ps1`（module_code = `dev_cache`）：
   - 扫描 npm：`$env:LOCALAPPDATA\npm-cache` 或 `$env:APPDATA\npm-cache`；
   - 扫描 pip：`$env:LOCALAPPDATA\pip\cache`；
   - 扫描 pnpm：`$env:LOCALAPPDATA\pnpm-store`；
   - 目录不存在 → 跳过。

### C. 脚本通用要求

8. 每个 ps1 头部加 `#requires -Version 5.1`；
9. 每个 ps1 头部加注释块：脚本用途 / module_code / 输出事件 schema 简述；
10. **不**写任何 `Write-Host` / `Write-Output "Processing..."` 人类可读日志；
11. **不**调用 `Remove-Item` / `Clear-RecycleBin` 等删除命令——本兼容层只扫描；
12. 用 `[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)` 强制 UTF-8 无 BOM；
13. 用 `Tee-Object` 或自写函数把所有 JSON 行直接 `[Console]::Out.WriteLine($json)`，避免 PowerShell 加额外前缀。

### D. 冒烟测试（`scripts/smoke_test.ps1` 或 PowerShell 手动）

14. DevAgent 实跑 5 个脚本，对每个脚本：
    - 捕获 stdout；
    - 按行 split，逐行 `ConvertFrom-Json`——必须全部成功；
    - 最后一行 `type` 必须是 `"done"`；
    - 前面所有行 `type` 必须是 `"item"`；
    - 每行必含 `module_code / path / size_bytes / risk_level` 5 个字段；
    - `module_code` 与脚本名对应（如 scan_temp.ps1 的所有行 module_code 都是 `"temp"`）；
15. DevAgent 把冒烟测试输出贴到实现备注（含每个脚本的 item 行数 / done 事件）；
16. 不写自动化测试框架（PowerShell Pester 留后续）——本任务用一次性的 smoke_test.ps1 即可。

### E. 卫生与边界

17. 不修改 `engine/**`、`app/**`、`elevation/**`、`rules/**`、`docs/**` 任何已有文件；
18. 不新增 Rust 依赖、不改 Cargo.toml；
19. 不写 GitHub Actions workflow（CI 跑 PowerShell 冒烟测试留后续 task）；
20. ps1 脚本**不**硬编码用户路径——一律用 `$env:TEMP` / `$env:LOCALAPPDATA` / `$env:APPDATA` 展开；
21. ps1 脚本**不**触碰 L3 红线目录（Desktop / Documents / Downloads / Pictures）——即使扫描到也跳过（对齐 task-003 L3 常量）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `scripts/PROTOCOL.md`（新建）
- `scripts/scan_temp.ps1`（新建）
- `scripts/scan_windows_update.ps1`（新建）
- `scripts/scan_prefetch.ps1`（新建）
- `scripts/scan_browser_cache.ps1`（新建）
- `scripts/scan_dev_cache.ps1`（新建）
- `scripts/smoke_test.ps1`（新建，可选；DevAgent 也可手跑后只贴输出）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-011-ps1-ndjson-compat`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `docs(scripts): 新增 NDJSON 输出协议文档（ps1→Rust 行协议 schema） (#task-011)`
   —— 含 `scripts/PROTOCOL.md`。
2. `feat(scripts): 新增 scan_temp 与 scan_windows_update 两个扫描脚本（NDJSON 输出） (#task-011)`
   —— 含 `scripts/scan_temp.ps1`、`scripts/scan_windows_update.ps1`。
3. `feat(scripts): 新增 scan_prefetch / scan_browser_cache / scan_dev_cache 三个扫描脚本 (#task-011)`
   —— 含另外三个 ps1。
4. `test(scripts): 实跑 5 个脚本做 NDJSON 冒烟验证并附输出 (#task-011)`
   —— 含 `scripts/smoke_test.ps1`（可选）或仅在实现备注里贴冒烟输出。
5. 自验全绿后，最后一次提交：`docs(state): 任务011完成，M1 前置清理清单全部就绪 (#task-011)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 把待办 011 从 `- [ ]` 改为 `- [x]`；
   - 「最近一次变更」追加一行；
   - 「当前目标」段改为「M1 前置清理（007~011）全部完成，正式进入 M1 业务模块 A~F 开发（A=扫描器，将消费 ps1 NDJSON）」；
   - 「下一步计划」改为「派发 M1 业务模块 A：Rust 主进程 spawn ps1 子进程、逐行收 NDJSON、写 cleanup_item 表、emit scan://progress」；
   - 「已知问题」段补一条：Rust 侧 ps1 消费器未实现（留 M1-A 任务）；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-12  task-011  ps1 兼容层从零编写（5 脚本 + NDJSON 协议），M1 前置清理收官`。

**提交前自验（必须全绿）：**
- 5 个 ps1 全部可跑（无语法错误）；
- 每个脚本 stdout 每行 `ConvertFrom-Json` 成功；
- 每个脚本最后一行是 done 事件；
- stdout 无人类可读日志（仅 NDJSON）；
- `cargo check --workspace` 仍退出码 0（本任务不改 Rust，预期无影响）。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-011-ps1-ndjson-compat`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际产出文件清单；
   - 每个脚本实跑的 item 行数 / done 事件原文；
   - NDJSON 行分隔符实测情况（`\r\n` vs `\n`）；
   - 遇到的权限不足目录列表（写 stderr 的内容）；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（Rust 消费器留 M1-A、CI 跑冒烟测试留后续）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新（**M1 前置清理清单收官**）；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务011-ps1兼容层NDJSON (#task-011)`，
   该提交只含 `tasks/011-ps1-ndjson-compat.md` 一个文件，落在 `feature/task-011-ps1-ndjson-compat` 分支最前端。

## 实现备注（DevAgent 落地后回填）
- **产出文件**：`scripts/PROTOCOL.md`（item/done 两种事件 schema + 编码/stderr 规则）；5 个扫描脚本 `scan_temp.ps1` / `scan_windows_update.ps1` / `scan_prefetch.ps1` / `scan_browser_cache.ps1` / `scan_dev_cache.ps1`；`scripts/smoke_test.ps1`（一次性冒烟）。
- **冒烟实跑结果**：
  - scan_temp: items=154, done=`{"type":"done","module_code":"temp","total_items":154,"total_bytes":16541385}`
  - scan_windows_update: items=6, done=`...windows_update...total_bytes=16535741`
  - scan_prefetch: items=0（非管理员权限下 Prefetch 不可读，脚本不报错、只 done）
  - scan_browser_cache: items=10, done=`...browser_cache...total_bytes=32240299`
  - scan_dev_cache: items=33, done=`...dev_cache...total_bytes=15121106`
- **行分隔符实测**：脚本 `[Console]::Out.WriteLine` 输出 `\r\n`（76 0d 0a 尾）；Rust 侧按 `\n` split 时 trim `\r` 即可，PROTOCOL.md 已注明。
- **权限不足目录**：Prefetch 在非管理员下返回 0 项（无 stderr 泄漏到 stdout）；其余目录正常枚举。
- **偏差**：冒烟脚本初版用 PS `*>` 重定向会把原生 stdout 包成 UTF-16LE 导致 ConvertFrom-Json 抖动，最终改用 `System.Diagnostics.Process` + `StandardOutputEncoding=UTF8` 字节级读取，与 Rust 消费方式一致。
- **遗留**：Rust 侧 ps1 消费器（spawn/逐行读/写库/emit）留 M1-A；CI 跑 ps1 冒烟留后续。
