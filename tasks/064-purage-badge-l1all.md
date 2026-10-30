# 任务编号：064 保险箱 7 天自动 purge + 风险徽标/影响说明 + 全选 L1 一键按钮（P0 收尾）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：P0 未做清单第 5 项（P0-4 保险箱 7 天自动 purge + P0-2 风险徽标/影响说明 + P0-3 全选 L1 一键按钮）。
> 上游依赖：task-063 已合并 main（merge `62cb7cd`，报告导出 + 前后对比 + 环形图 + 取消扫描完成）。
> **版本线**：v0.2.0 开发线 P0 收尾——本任务完成后 P0 全部清完，进入 M1 业务模块缺口（C/D/E/F）。
> **本任务范围（3 个功能单元）**：保险箱 7 天自动 purge + 风险徽标/影响说明 + 全选 L1 一键按钮。
> **为什么合并**：都是 P0 收尾体验项，保险箱 purge（单元 1）是后端定时任务，风险徽标（单元 2）+ 全选 L1（单元 3）是前端 Items 页体验增强。按「任务量尽量大」一次 PR 收尾 P0。
> **不做**：跨盘恢复 fallback / RmShutdown 三功能（维持现状）；M1 C/D/E/F 模块（留 task-065+）。

## 状态字段

pending

## 任务目标

### 1. P0-4 保险箱 7 天自动永久 purge

- **PRD §396 描述**：保险箱默认开启，延迟删除 7 天，7 天后自动永久 purge（不可恢复）。
- **现状**：保险箱已有 `expires_at` 字段，但无自动 purge 机制。
- **修法**：
  - 后端启动时（或每小时）跑一次 `purge_expired()`：DELETE `status='in_box' AND expires_at < now()` 的行 + 删除磁盘文件；
  - purge 前 audit_log 记 `action='auto_purge'`；
  - 前端保险箱页显示「7 天后自动永久删除」提示。

### 2. P0-2 风险徽标/影响说明

- **PRD §104 描述**：每个清理项显示 L0/L1/L2/L3 风险徽标，hover 显示影响说明。
- **现状**：风险等级已在后端，但前端可能未显示徽标/hover 说明。
- **修法**：
  - Items 页每项显示风险徽标（L0 灰 / L1 绿 / L2 黄 / L3 红）；
  - hover 显示 tooltip：「L1 安全缓存，删除不影响系统」/「L2 需确认，删除可能影响 X」；
  - 徽标样式用 CSS 变量（适配深色主题）。

### 3. P0-3 全选 L1 一键按钮

- **PRD §116 描述**：一键全选所有 L1 安全项，用户快速清理。
- **现状**：Items 页有 checkbox 逐项选择，但无「全选 L1」快捷按钮。
- **修法**：
  - Items 页顶部加「全选 L1（安全）」按钮；
  - 点击后勾选所有 L1 项，L2/L3 不选；
  - 按钮旁显示「已选 N 项 L1，约 X GB」。

**本任务不做**：
- 不做 M1 C/D/E/F 模块（WinSxS / 大文件 / 残留 / 注册表）；
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 前置条件

1. `main` 已合并 task-063（merge `62cb7cd`）；
2. 关键锚点：
   - `engine/src/execute/safe_box.rs`（加 `purge_expired()`）
   - `engine/src/lib.rs` 或 `main.rs`（启动时跑 purge）
   - `app/src/pages/Items.tsx`（风险徽标 + 全选 L1 按钮）
   - `app/src/pages/SafeBox.tsx`（7 天 purge 提示）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 保险箱 7 天自动 purge

1. 后端启动时跑 `purge_expired()`：DELETE 过期行 + 删除磁盘文件；
2. purge 前 audit_log 记 `action='auto_purge'`；
3. 前端保险箱页显示「7 天后自动永久删除」提示；
4. 单测：`purge_expired()` 删除过期行 + 保留未过期行。

### 2. 风险徽标/影响说明

5. Items 页每项显示风险徽标（L0 灰 / L1 绿 / L2 黄 / L3 红）；
6. hover 显示 tooltip 影响说明；
7. 徽标样式适配深色主题（CSS 变量）；
8. 单测：风险等级映射正确。

### 3. 全选 L1 一键按钮

9. Items 页顶部加「全选 L1（安全）」按钮；
10. 点击后勾选所有 L1 项，L2/L3 不选；
11. 按钮旁显示「已选 N 项 L1，约 X GB」；
12. 单测：全选 L1 逻辑（只选 L1，不选 L2/L3）。

### 后端单测

13. `cargo test --workspace` 全绿（现有 291+8 + 本任务新增 ≥3 条全绿）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/execute/safe_box.rs`（`purge_expired()`）
- `engine/src/lib.rs` 或 `main.rs`（启动时跑 purge）
- `engine/tests/`（purge 用例）
- `app/src/pages/Items.tsx`（风险徽标 + 全选 L1 按钮）
- `app/src/pages/SafeBox.tsx`（7 天 purge 提示）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-064-purage-badge-l1all`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-064-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 3 个功能单元拆 3 个原子提交：

1. `feat(safe_box): 7 天自动 purge（启动时 purge_expired + audit_log 记录） (#task-064)`（单元 1）
   —— 含 `safe_box.rs` + `lib.rs` + `SafeBox.tsx` + 单测。
2. `feat(ui): 风险徽标/影响说明（L0-L3 徽标 + hover tooltip + 深色适配） (#task-064)`（单元 2）
   —— 含 `Items.tsx` + 单测。
3. `feat(ui): 全选 L1 一键按钮（只选 L1，显示已选数量/大小） (#task-064)`（单元 3）
   —— 含 `Items.tsx` + 单测。
4. 自验全绿后，最后一次提交：`docs(state): 任务064完成，P0 收尾——purge/徽标/全选L1 就绪 (#task-064)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 064: ...`；
   - 「最近一次变更」追加一行，含 3 个单元摘要 + commit hash；
   - 「下一步计划」改为「P0 全部清完，进入 M1 业务模块缺口（C 模块 WinSxS 清理，task-065）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-30  task-064  P0 收尾（7天purge/风险徽标/全选L1）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：purge_expired 只删过期行、风险等级映射、全选 L1 逻辑。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **端到端写路径（必做，本任务硬要求）**：保险箱 purge 必须有一条真实「造过期行 → 启动时 purge → DB 行消失 + 磁盘文件消失」证据（不能只构造 mock）。
- **真机冒烟（必做、不可裁剪）**：
  - 保险箱页显示「7 天后自动永久删除」提示；
  - Items 页每项显示风险徽标（L0-L3 四色）；
  - hover 风险徽标显示 tooltip 影响说明；
  - 「全选 L1」按钮：点击后只选 L1，L2/L3 不选；
  - 按钮旁显示「已选 N 项 L1，约 X GB」；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 5 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（风险徽标 + 全选 L1 按钮 + purge 提示，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| purge_expired | 单测输出（过期行删除 + 未过期保留） |
| 7 天 purge 提示 | 真机截图：保险箱页显示提示 |
| 风险徽标 | 真机截图：Items 页四色徽标 |
| tooltip 影响说明 | 真机截图：hover 显示说明 |
| 全选 L1 | 真机证据：点击按钮 → 只选 L1，L2/L3 不选 |
| 已选数量/大小 | 真机截图：按钮旁显示统计 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 5 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-064-purage-badge-l1all`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - purge_expired 触发时机（启动时 vs 每小时）；
   - 风险徽标颜色方案；
   - 全选 L1 按钮位置；
   - `cargo test --workspace` 通过输出（含新增 ≥3 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（5 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（P0 全部清完；M1 C/D/E/F 模块排队；Items/SafeBox/History 深色适配待补）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务064-P0收尾（7天purge+风险徽标+全选L1） (#task-064)`，
   该提交只含 `tasks/064-purage-badge-l1all.md` 一个文件，落在 `feature/task-064-purage-badge-l1all` 分支最前端。
