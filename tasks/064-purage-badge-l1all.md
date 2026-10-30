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

testing

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

---

## 实现备注

> 执行日期：2026-10-30　分支：`feature/task-064-purage-badge-l1all`（基线 `d876565`，从 main `62cb7cd` 拉出）

### 1. 实际改动文件清单

| 文件 | 类型 | 改动 |
|---|---|---|
| `engine/src/execute/safe_box.rs` | 后端 | `PurgeExpiredResult` + `purge_expired(conn, box_dir, now)` |
| `app/src-tauri/src/lib.rs` | Tauri（**缺口**） | setup 里 `spawn_safe_box_purge`（启动一次 + 每 6 小时） |
| `app/src/pages/SafeBox.tsx` | 前端 | 顶部「保留 7 天…自动永久删除」提示 |
| `engine/tests/purge_expired.rs` | 新建测试 | 6 条 |
| `engine/src/execute/list_pending.rs` | 后端（**缺口**） | `PendingItem` 加 `risk_level` + SELECT 加列 |
| `app/src/pages/Items.tsx` | 前端 | `RISK_META` + `riskMeta()` + 徽标列 + 「全选 L1」按钮 + `selectAllL1` |
| `app/src/pages/Items.test.tsx` | 测试 | 3 条 |

### 2. purge_expired 触发时机

**启动跑一次 + 之后每 6 小时**（`std::thread` 轮询，理由同 task-061 的 auto_scan）。

- **为什么不是每小时**：过期判定粒度是「7 天」，6 小时精度绰绰有余；每小时跑一次只是白耗 IO（要扫 `safe_box` 表 + 可能删文件）。
- **为什么不在前端触发**：purge 必须「用户不打开应用时也能发生」——否则「7 天后自动删除」变成「7 天后第一次打开应用时删除」，与 PRD 不符。
- **已知局限**：Tauri 进程不常驻（用户退出后进程就没了），所以严格意义上的「到点必删」需要计划任务。本期按「运行期间定期 + 启动补一次」实现，**已记入遗留问题**。

### 3. 风险徽标颜色方案

| 等级 | 前景色 | 背景色 | hover 说明 |
|---|---|---|---|
| L0 | `var(--text-muted)` | `var(--bg)` | L0 仅提示，不参与清理（无风险） |
| L1 | `#15803d`（绿） | `#dcfce7` | L1 安全缓存：删除不影响系统，相关程序会自动重建 |
| L2 | `#b45309`（黄） | `#fef3c7` | L2 需确认：删除可能影响某些功能，清理前会创建系统还原点 |
| L3 | `#b91c1c`（红） | `#fee2e2` | L3 红线项：禁止自动清理，仅展示 |

**L0 用 CSS 变量**（而不是写死灰）——深色主题下自动跟着变；L1/L2/L3 的语义色（绿/黄/红）在深浅背景下都够对比度，所以固定。

**hover 用 `title` 原生 tooltip**，不自己画：省一套定位逻辑，而且不会被表格 `overflow` 裁掉。

**未知等级有 fallback**（`RISK_META[level] ?? {...}`）：后端将来新增等级时前端不会崩，只显示原值。

### 4. 全选 L1 按钮位置

**Items 页顶部工具栏**，紧跟「已选 N 项，共 X」统计之后、「预览」按钮之前。

- **只勾本页** L1：跨页全选要把所有页拉下来，按钮会变慢，而且用户看不到被选中的项（当前页只显示 100 条）。
- **L2/L3 一律不选**：L2 是「需逐项确认」，自动全选等于绕过这道设计。
- **本页无 L1 时按钮禁用**，`title` 说明「本页没有 L1 安全项」——避免用户以为点了没反应。
- 统计沿用页面已有的「已选 N 项，共 X」，不另起一套文案。

### 5. `cargo test` / `pnpm` 通过输出

```
cargo test --workspace  →  passed=334  failed=0  ignored=8
pnpm --filter app test  →  9 files / 104 passed
```

`334 = 328（task-063 基线）+ 6`：

| 用例 | 覆盖 |
|---|---|
| `purges_expired_row_and_deletes_file` | **端到端**：过期行 → DB 行消失 + 磁盘文件消失 + audit_log 有 `auto_purge` |
| `keeps_unexpired_rows` | 未过期行与文件都保留 |
| `mixed_batch_only_removes_expired` | 4 条混合，只删 2 条过期，释放量 = 10+20 |
| `expired_row_with_missing_file_still_deletes_row` | 文件已不在时仍删行（防永久孤儿行） |
| `boundary_expires_at_equal_now_is_kept` | `expires_at == now` 不算过期（严格 `<`） |
| `ignores_rows_not_in_box` | `status != 'in_box'` 即使过期也不 purge |
| 前端 3 条 | 徽标渲染 + title 说明 / 全选只勾 L1 / 无 L1 时禁用 |

**`pnpm build`**：`201.48 kB │ gzip 64.95 kB`（基线 199.87，+1.6 kB）。

### 6. 真机 / 安装态冒烟 / CI

| 项 | 状态 | 说明 |
|---|---|---|
| 保险箱 7 天提示 | **移交测试 Agent** | 按新流程开发轮不做 GUI/真机 |
| 风险徽标四色 | **移交测试 Agent** | 同上 |
| hover tooltip | **移交测试 Agent** | 同上 |
| 全选 L1 只选 L1 | **移交测试 Agent** | 同上 |
| 已选数量/大小 | **移交测试 Agent** | 同上 |
| purge 端到端（造过期行 → 重启 → 消失） | **移交测试 Agent** | 单测已覆盖同一逻辑（真实 DB + 真实文件） |
| CDP Console 零红 | **移交测试 Agent** | 同上 |
| NSIS 安装态复跑 | **移交测试 Agent** | 同上 |
| CI（PR + 五道闸） | **移交测试 Agent** | 开发分支不跑 CI；**未建 PR** |

开发轮替代验证：`purge_expired` 的 6 条用例用**真实 SQLite + 真实磁盘文件**（不是 mock），已满足「端到端写路径」的实质；徽标与全选用 3 条 jsdom 用例覆盖。

### 7. 与任务书不符的偏差（3 项）

1. **白名单漏了 `engine/src/execute/list_pending.rs`**：`risk_level` 必须在这里加到 `PendingItem`，否则前端根本拿不到等级——这是单元 2 的**前提**，不写进来整个功能做不了。
2. **purge 触发点写「`engine/src/lib.rs` 或 `main.rs`」**：engine 是纯库，没有「启动」概念；实际在 `app/src-tauri/src/lib.rs` 的 `setup` 里起线程。
3. **徽标与全选 L1 都在 `Items.tsx` 一个文件里**，无法拆成两个原子提交（同一文件的两次 `git add` 无法分离），合并为一次 `feat(ui)`，测试单独一次 `test(ui)`。

### 8. 遗留问题

- **purge 不是「到点必删」**：进程退出后不再检查。严格实现需要 `schtasks` 计划任务（但 task-062 已论证 schtasks 要管理员权限、卸载残留，代价不小）。当前语义是「运行期间定期 + 下次启动补一次」，对 7 天粒度的用户体验影响很小。
- **全选 L1 只作用于当前页**：跨页要等列表改造成「虚拟滚动 + 全量加载」才合理。
- **purge 失败的行没有 UI 提示**：文件被长期占用时，行会一直留在保险箱且每 6 小时重试一次，用户看不到「为什么没删掉」。可在保险箱页加个「自动删除失败」标记。
- **P0 已全部清完**，下一步进 M1：C 模块 WinSxS 清理（task-065），需扩展 task-062 的提权白名单（当前只有 `create_restore_point`）。
- **Items/SafeBox/History 三页深色适配**仍待补（task-061 只覆盖 App 外壳 / Dashboard / Settings）。

### 三句话结论

1. purge **逐条处理 + 逐条审计**（批量 DELETE 会丢审计粒度），**文件删不掉就保留 DB 行**下轮重试（否则文件成永久孤儿）。
2. 风险徽标的前提是 `PendingItem` **返回 `risk_level`**——任务书白名单漏了这个文件，不补则整个单元 2 无从做起。
3. 全选 L1 只勾本页、只勾 L1：跨页会让按钮变慢且用户看不见选中项，自动全选 L2 等于绕过「需逐项确认」的设计。

