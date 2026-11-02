# 任务编号：074 架构债 R5 起步——scan_temp.ps1 → Rust 重写（临时文件扫描）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：P2 与架构债清单 R5（ps1→Rust 重写，v0.4.x 起步）。
> 上游依赖：task-073 已合并 main（merge `70fd70c`，P1 体验项全部收官）。
> **版本线**：v0.4.x 架构债起步——ps1→Rust 重写。
> **本任务范围（3 个功能单元）**：scan_temp.ps1 逻辑 Rust 重写 + Orchestrator 接线（Rust 优先 / ps1 回退）+ 单测覆盖。
> **为什么合并**：第一个 ps1 重写必须完整闭环（Rust 实现 + Orchestrator 接线 + 单测），作为后续 4 个 ps1 重写的模板。
> **不做**：其他 4 个 ps1（scan_browser_cache / scan_dev_cache / scan_prefetch / scan_windows_update，留 task-075+）；跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 状态字段

pending

## 任务目标

### 1. scan_temp.ps1 逻辑 Rust 重写

- **现状**：`scripts/scan_temp.ps1` 扫描 `%TEMP%` 目录，输出 NDJSON 格式清理项。
- **修法**：
  - `engine/src/scan/modules/temp.rs`（新建）：
    - 遍历 `%TEMP%` 目录；
    - 按文件大小 / 修改时间筛选；
    - 输出统一结构 `CleanupItem { path, size_bytes, module, risk_level }`；
    - 与 ps1 输出格式一致（NDJSON），但直接返回 Vec，不走进程。
  - `engine/src/scan/modules/mod.rs`（新建）：模块注册。

### 2. Orchestrator 接线（Rust 优先 / ps1 回退）

- **现状**：`engine/src/scan/orchestrator.rs` 通过 `ScanRunner` spawn ps1 进程。
- **修法**：
  - `engine/src/scan/orchestrator.rs`：
    - temp 模块改为直接调 Rust 实现（不再 spawn ps1）；
    - 其他 4 个模块保持 ps1 方式（逐步迁移）；
    - 加 feature flag 或环境变量 `CCLEAN_RUST_TEMP=1` 控制（默认开启，可回退 ps1）。

### 3. 单测覆盖

- **修法**：
  - `engine/src/scan/modules/temp.rs` 内联单测：
    - 创建临时目录，放入测试文件；
    - 调用 Rust 扫描函数，验证返回结果格式；
    - 与 ps1 输出对比（抽样）。

**本任务不做**：
- 不做其他 4 个 ps1 重写（留 task-075+）；
- 不做 ps1 脚本删除（保留作为回退）；
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 前置条件

1. `main` 已合并 task-073（merge `70fd70c`），P1 全部收官；
2. 关键锚点（必须全部在白名单内）：
   - 后端：`engine/src/scan/modules/temp.rs`（新建）
   - 后端：`engine/src/scan/modules/mod.rs`（新建）
   - 后端：`engine/src/scan/orchestrator.rs`（接线 Rust 优先）
   - 后端测试：`engine/tests/temp_scan_test.rs`（新建）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. scan_temp Rust 重写

1. `engine/src/scan/modules/temp.rs` 实现临时文件扫描；
2. 返回 `Vec<CleanupItem>`，格式与 ps1 输出一致；
3. 正确处理 `%TEMP%` 环境变量；
4. 单测：扫描临时目录返回正确结果。

### 2. Orchestrator 接线

5. `orchestrator.rs` temp 模块改为直接调 Rust；
6. 其他 4 个模块保持 ps1 方式；
7. 环境变量 `CCLEAN_RUST_TEMP=0` 可回退 ps1；
8. 单测：Rust 优先 / ps1 回退两种模式。

### 3. 单测覆盖

9. `cargo test --workspace` 全绿（现有 + 本任务新增 ≥3 条全绿）；
10. Rust 实现与 ps1 输出抽样对比（格式一致）。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**后端（engine）**：
- `engine/src/scan/modules/temp.rs`（新建）
- `engine/src/scan/modules/mod.rs`（新建）
- `engine/src/scan/orchestrator.rs`（接线 Rust 优先 / ps1 回退）
- `engine/tests/temp_scan_test.rs`（新建）

**文档**：
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

**不修改**：
- `scripts/scan_temp.ps1`（保留作为回退，不删除）
- 其他 4 个 ps1 脚本（不改动）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-074-r5-temp-rust-rewrite`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-074-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 3 个功能单元拆 3 个原子提交：

1. `feat(scan): scan_temp.ps1 → Rust 重写（临时文件扫描模块） (#task-074)`（单元 1）
   —— 含 `engine/src/scan/modules/temp.rs` + `modules/mod.rs`。
2. `feat(orchestrator): temp 模块 Rust 优先 / ps1 回退接线 (#task-074)`（单元 2）
   —— 含 `engine/src/scan/orchestrator.rs`。
3. `test(scan): temp_scan 单测覆盖（Rust vs ps1 抽样对比） (#task-074)`（单元 3）
   —— 含 `engine/tests/temp_scan_test.rs`。
4. 自验全绿后，最后一次提交：`docs(state): 任务074完成，R5 架构债起步——scan_temp Rust 重写就绪 (#task-074)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 074: ...`；
   - 「最近一次变更」追加一行，含 3 个单元摘要 + commit hash；
   - 「下一步计划」改为「R5 架构债持续：scan_browser_cache / scan_dev_cache / scan_prefetch / scan_windows_update 逐步 Rust 重写（task-075+）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-02  task-074  R5 架构债起步（scan_temp Rust 重写）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0（无前端改动，回归保护）。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：temp 扫描逻辑、Rust/ps1 切换、输出格式对比。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **真机冒烟（必做、不可裁剪）**：
  - 扫描结果包含 temp 模块项（Rust 实现）；
  - 环境变量 `CCLEAN_RUST_TEMP=0` 时回退 ps1（结果一致）；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 2 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（扫描结果可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| temp 扫描逻辑 | 单测输出（返回格式正确） |
| Rust/ps1 切换 | 单测输出（两种模式结果一致） |
| 扫描结果含 temp 项 | 真机截图：Items 页显示 temp 模块项 |
| 回退 ps1 模式 | 真机证据：环境变量切换后结果一致 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 2 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-074-r5-temp-rust-rewrite`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - Rust 实现与 ps1 输出对比（字段 / 格式 / 筛选逻辑）；
   - Orchestrator 接线方式（feature flag / 环境变量）；
   - 环境变量回退机制；
   - `cargo test --workspace` 通过输出（含新增 ≥3 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（2 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（R5 架构债持续：其余 4 个 ps1 重写 task-075+；Items/SafeBox/History 深色适配待补）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务074-R5架构债起步 scan_temp Rust重写 (#task-074)`，
   该提交只含 `tasks/074-r5-temp-rust-rewrite.md` 一个文件，落在 `feature/task-074-r5-temp-rust-rewrite` 分支最前端。
