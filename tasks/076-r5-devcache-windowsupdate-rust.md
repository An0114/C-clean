# 任务编号：076 R5 架构债收官——scan_dev_cache.ps1 + scan_windows_update.ps1 → Rust 重写

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：R5 架构债收官（ps1→Rust 重写，v0.4.x）。
> 上游依赖：task-075 已合并 main（merge `66eceb1`，prefetch + browser_cache Rust 重写完成）。
> **版本线**：v0.4.x 架构债收官——全部 5 个 ps1 Rust 重写完成。
> **本任务范围（4 个功能单元）**：scan_dev_cache.ps1 Rust 重写 + scan_windows_update.ps1 Rust 重写 + Orchestrator 接线（两个模块 Rust 优先）+ 单测覆盖 + R5 收官确认。
> **为什么合并**：最后两个 ps1 合并一次 PR 完成 R5 架构债全部 5 个模块的 Rust 迁移，符合「任务量尽量大」。
> **不做**：跨盘恢复 fallback / RmShutdown 三功能（维持现状）；ps1 脚本删除（保留作为回退）。

## 状态字段

pending

## 任务目标

### 1. scan_dev_cache.ps1 → Rust 重写

- **现状**：`scripts/scan_dev_cache.ps1` 扫描开发工具缓存（Node_modules / npm cache / pip cache / Gradle / Maven 等），输出 NDJSON 格式清理项。
- **修法**：
  - `engine/src/scan/modules/dev_cache.rs`（新建）：
    - 遍历 npm cache（`%APPDATA%\npm-cache\`）；
    - 遍历 pip cache（`%LOCALAPPDATA%\pip\Cache\`）；
    - 遍历 Gradle cache（`%USERPROFILE%\.gradle\caches\`）；
    - 遍历 Maven cache（`%USERPROFILE%\.m2\repository\`）；
    - 按文件大小 / 修改时间筛选；
    - 输出统一结构 `CleanupItem { path, size_bytes, module, risk_level }`。

### 2. scan_windows_update.ps1 → Rust 重写

- **现状**：`scripts/scan_windows_update.ps1` 扫描 Windows Update 缓存（`C:\Windows\SoftwareDistribution\Download\`），输出 NDJSON 格式清理项。
- **修法**：
  - `engine/src/scan/modules/windows_update.rs`（新建）：
    - 遍历 `C:\Windows\SoftwareDistribution\Download\`；
    - 按文件大小 / 修改时间筛选；
    - 输出统一结构。

### 3. Orchestrator 接线（两个模块 Rust 优先）

- **现状**：`engine/src/scan/orchestrator.rs` temp / prefetch / browser_cache 三个模块已改为 Rust 优先（task-074/075）。
- **修法**：
  - `engine/src/scan/orchestrator.rs`：
    - dev_cache 模块改为直接调 Rust 实现；
    - windows_update 模块改为直接调 Rust 实现；
    - 环境变量 `CCLEAN_RUST_DEV_CACHE=0` / `CCLEAN_RUST_WINDOWS_UPDATE=0` 可回退 ps1；
    - 全部 5 个模块默认 Rust 优先。

### 4. 单测覆盖 + R5 收官确认

- **修法**：
  - `engine/src/scan/modules/dev_cache.rs` 内联单测；
  - `engine/src/scan/modules/windows_update.rs` 内联单测；
  - `engine/tests/devcache_windowsupdate_test.rs`（新建）：Rust vs ps1 抽样对比；
  - 确认全部 5 个模块 Rust 优先，ps1 仅作为回退保留。

**本任务不做**：
- 不做 ps1 脚本删除（保留作为回退）；
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 前置条件

1. `main` 已合并 task-075（merge `66eceb1`），prefetch + browser_cache Rust 重写完成；
2. 关键锚点（必须全部在白名单内）：
   - 后端：`engine/src/scan/modules/dev_cache.rs`（新建）
   - 后端：`engine/src/scan/modules/windows_update.rs`（新建）
   - 后端：`engine/src/scan/modules/mod.rs`（注册两个新模块）
   - 后端：`engine/src/scan/orchestrator.rs`（接线 Rust 优先 / ps1 回退）
   - 后端测试：`engine/tests/devcache_windowsupdate_test.rs`（新建）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. scan_dev_cache Rust 重写

1. `engine/src/scan/modules/dev_cache.rs` 实现开发缓存扫描（npm / pip / Gradle / Maven）；
2. 正确处理环境变量和用户目录；
3. 返回 `Vec<CleanupItem>`，格式与 ps1 输出一致；
4. 单测：扫描开发缓存目录返回正确结果。

### 2. scan_windows_update Rust 重写

5. `engine/src/scan/modules/windows_update.rs` 实现 Windows Update 缓存扫描；
6. 返回 `Vec<CleanupItem>`，格式与 ps1 输出一致；
7. 单测：扫描 SoftwareDistribution\Download 返回正确结果。

### 3. Orchestrator 接线

8. `orchestrator.rs` dev_cache 模块改为直接调 Rust；
9. `orchestrator.rs` windows_update 模块改为直接调 Rust；
10. 环境变量可回退 ps1；
11. 全部 5 个模块默认 Rust 优先；
12. 单测：Rust 优先 / ps1 回退两种模式。

### 4. 单测覆盖 + R5 收官

13. `cargo test --workspace` 全绿（现有 + 本任务新增 ≥4 条全绿）；
14. Rust 实现与 ps1 输出抽样对比（格式一致）；
15. R5 架构债全部 5 个 ps1 Rust 重写完成。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**后端（engine）**：
- `engine/src/scan/modules/dev_cache.rs`（新建）
- `engine/src/scan/modules/windows_update.rs`（新建）
- `engine/src/scan/modules/mod.rs`（注册两个新模块）
- `engine/src/scan/orchestrator.rs`（接线 Rust 优先 / ps1 回退）
- `engine/tests/devcache_windowsupdate_test.rs`（新建）

**文档**：
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

**不修改**：
- `scripts/scan_dev_cache.ps1`（保留作为回退，不删除）
- `scripts/scan_windows_update.ps1`（保留作为回退，不删除）
- 其他 3 个 ps1 脚本（不改动）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-076-r5-devcache-windowsupdate-rust`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-076-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `feat(scan): scan_dev_cache.ps1 → Rust 重写（npm/pip/Gradle/Maven 缓存扫描） (#task-076)`（单元 1）
   —— 含 `engine/src/scan/modules/dev_cache.rs` + `modules/mod.rs`。
2. `feat(scan): scan_windows_update.ps1 → Rust 重写（SoftwareDistribution 缓存扫描） (#task-076)`（单元 2）
   —— 含 `engine/src/scan/modules/windows_update.rs`。
3. `feat(orchestrator): dev_cache / windows_update 模块 Rust 优先 / ps1 回退接线（R5 收官） (#task-076)`（单元 3）
   —— 含 `engine/src/scan/orchestrator.rs`。
4. `test(scan): devcache_windowsupdate 单测覆盖（Rust vs ps1 抽样对比） (#task-076)`（单元 4）
   —— 含 `engine/tests/devcache_windowsupdate_test.rs`。
5. 自验全绿后，最后一次提交：`docs(state): 任务076完成，R5 架构债收官——全部 5 个 ps1 Rust 重写就绪 (#task-076)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 076: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「R5 架构债全部完成；下一阶段进入 P2 项（多盘扫描 / 自定义规则 / 便携版 CLI）或发版 v0.4.0」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-03  task-076  R5 收官（dev_cache + windows_update Rust 重写）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0（无前端改动，回归保护）。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：dev_cache / windows_update 扫描逻辑、Rust/ps1 切换、输出格式对比。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **真机冒烟（必做、不可裁剪）**：
  - 扫描结果包含 dev_cache 模块项（Rust 实现）；
  - 扫描结果包含 windows_update 模块项（Rust 实现）；
  - 全部 5 个模块 Rust 优先（不再 spawn ps1 进程）；
  - 环境变量回退 ps1 时结果一致；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 4 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（扫描结果可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| dev_cache 扫描逻辑 | 单测输出（npm/pip/Gradle/Maven 路径正确） |
| windows_update 扫描逻辑 | 单测输出（SoftwareDistribution\Download 正确） |
| Rust/ps1 切换 | 单测输出（两种模式结果一致） |
| 扫描结果含 dev_cache/windows_update 项 | 真机截图：Items 页显示两个模块项 |
| 全部 5 模块 Rust 优先 | 真机证据：进程列表无 ps1 子进程 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 4 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-076-r5-devcache-windowsupdate-rust`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - Rust 实现与 ps1 输出对比（字段 / 格式 / 筛选逻辑）；
   - npm/pip/Gradle/Maven 缓存路径确认；
   - SoftwareDistribution\Download 路径确认；
   - Orchestrator 接线方式（环境变量回退）；
   - R5 收官确认：全部 5 个模块 Rust 优先，ps1 仅作为回退保留；
   - `cargo test --workspace` 通过输出（含新增 ≥4 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（4 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（R5 架构债全部完成；下一阶段 P2 项 / 发版 v0.4.0；Items/SafeBox/History 深色适配待补）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务076-R5收官 dev_cache+windows_update Rust重写 (#task-076)`，
   该提交只含 `tasks/076-r5-devcache-windowsupdate-rust.md` 一个文件，落在 `feature/task-076-r5-devcache-windowsupdate-rust` 分支最前端。
