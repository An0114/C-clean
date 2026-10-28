# 任务编号：056 M3 技术评估 + 遗留小修——差量更新评估 + restored_at 审计列 + 跨盘 rename 测试 + G6 排查

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRustReact + DevFrontendReact
> 来源：PROJECT_STATE「下一步计划 2（差量更新评估）」+「已知问题」段 B 低/B 中遗留项。
> 上游依赖：task-055 已合并 main（merge `4231cbd`，banner 下载进度闭环）。
> **本任务范围（4 个功能单元，厚但低风险）**：差量更新技术评估文档、`restored_at` 审计列、跨盘 rename 测试用例补全、G6 设置页 IPC 红错排查与门禁豁免口径明确化。
> **为什么不是纯开发任务**：M3 下一项「差量更新」PROJECT_STATE 明确说「先做技术评估再立项」；同时清掉几个 B 低/B 中遗留小修，避免它们长期挂在「已知问题」段。
> **不做**：差量更新实际开发（评估完再立项 task-057）、RmRestart（产品未拍板）、代码签名（未购证书）、品牌图标（决策不做）。

## 状态字段

pending

## 任务目标

### 1. 差量更新技术评估文档（M3 下一项立项前置）

- **现状**：banner 下载进度已落地（task-055），但 NSIS 包 ~4 MB、MSI ~6 MB，每次更新全量下载。PROJECT_STATE「下一步计划 2」明确「差量更新是独立大块，建议先做技术评估再立项」。
- **修法**：写 `docs/research/delta-update-evaluation.md`，覆盖：
  - **候选方案 A：Tauri 2 官方 updater 差量支持**——查 `@tauri-apps/plugin-updater` 是否原生支持 bsdiff/xdelta；若不支持，Tauri 2 是否有官方 plugin 或社区方案；
  - **候选方案 B：bsdiff4（Rust crate `bsdiff` / `bzip2`）**——服务端发版时对相邻版本生成 `.bsdiff` 差量包，客户端下载差量包 + 旧版安装包 → 本地 patch 成新版；
  - **候选方案 C：xdelta3**——与 bsdiff 对比（压缩率 / 速度 / Rust 绑定成熟度）；
  - **关键决策点**：
    - 差量包签名（不能直接复用整包签名，需单独签）；
    - 客户端旧版本校验（patch 前必须确认本地是 v0.1.6，否则不能 patch 到 v0.1.7）；
    - 服务端产物矩阵（每相邻版本对一个差量包，v0.1.5→0.1.6、v0.1.6→0.1.7...）；
    - 回退路径（patch 失败自动下载整包）；
  - **结论**：推荐方案 + 预估工作量（人日）+ 风险 + 是否值得做（对 4 MB 包差量是否真省流量）；
  - **输出**：评估文档 + task-057 立项建议（或明确「不做」）。

### 2. `restored_at` 审计列（B 低，task-017 遗留）

- **现状**：`safe_box` 表有 `purged_at`（purge 时写），但 restore 时只 DELETE 行 + audit_log `action='restore'`，**没有时间列**——用户问「这个文件什么时候恢复的」查不到。
- **修法**：
  - **方案 A（推荐）**：`safe_box` 加 `restored_at TEXT NULL` 列（迁移器 v2 加 `ALTER TABLE safe_box ADD COLUMN restored_at TEXT`）；restore 时 UPDATE 该行 `restored_at = NOW()`（DELETE 改为 UPDATE status='restored' + restored_at）；
  - **方案 B（兜底）**：不动 safe_box，audit_log 已经有 `created_at`，前端从 audit_log 反查——但这是 N+1 查询，不推荐；
  - 选方案 A：迁移器 v2 + 后端 `restore` / `restore_many` 改 UPDATE 而非 DELETE + 单测覆盖。

### 3. 跨盘 rename 测试用例补全（B 低，task-016 遗留）

- **现状**：`engine/src/execute/safe_box.rs` 跨盘 rename 走 copy+remove fallback，但 CI 只跑单盘（C 盘），EXDEV 路径未实测。
- **修法**：
  - 写一个集成测试 `safe_box_cross_disk.rs`：
    - 用 `tempfile::TempDir` 在不同盘建两个目录（如 C 盘 + 如果有 D 盘就用 D 盘，否则 skip）；
    - 断言：跨盘 rename 不返回 EXDEV Err，走 copy+remove fallback，文件内容一致；
  - CI 环境只有 C 盘（windows-latest），测试用 `#[cfg_attr(not(windows), ignore)]` + 运行时检测「只有一个盘则 skip」，避免 CI 红。

### 4. G6 设置页 IPC 红错排查与门禁豁免口径明确化（B 中，已知项）

- **现状**：每次文档加载后首次进「设置」tab，`rules_check_update` 走 `http://ipc.localhost/rules_check_update` 报 2 红 + 2 黄，功能经 postMessage 回退可用。task-039 评审时已发现，至今未修。
- **修法**：
  - **排查**：读 Tauri 2.x `useCustomProtocol` 回退策略源码（或 GitHub issue），定位为什么首调走 ipc.localhost 而非 postMessage；
  - **尝试性修复（可选）**：在 `Settings.tsx` 首调前 warmup 一下 IPC 通道（如调一次无副作用的 `app_info` 命令），触发 postMessage 回退；
  - **如果修不动**：明确门禁豁免口径写进 `docs/test-reports/gui-walkthrough-round3-checklist.md`：「G6 为 Tauri 2.x 已知项，发版门禁口径为排除 G6 后 Console 零红，G6 本身不阻塞发版」；
  - **结论**：要么修掉（提交 fix），要么明确豁免口径（提交 docs），二选一，不要挂着。

**本任务不做**：
- 不做差量更新实际开发（评估完再立项 task-057）；
- 不做 RmRestart（产品未拍板）；
- 不做代码签名（未购 EV/OV 证书）；
- 不做品牌图标（决策不做）；
- 不做 Restart Manager 集成（M1-D 大块，留后续）。

## 前置条件

1. `main` 已合并 task-055（merge `4231cbd`）；
2. 关键锚点：
   - `engine/src/db/migrate.rs`（迁移器，加 v2）
   - `engine/src/execute/safe_box.rs`（restore / restore_many 改 UPDATE）
   - `engine/src/execute/safe_box.rs`（跨盘 rename fallback 位置）
   - `app/src/pages/Settings.tsx:87, 104`（G6 首调点）
   - `docs/test-reports/gui-walkthrough-round3-checklist.md`（G6 豁免口径写入位置）
   - 仓库根（新建 `docs/research/delta-update-evaluation.md`）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 差量更新评估

1. `docs/research/delta-update-evaluation.md` 存在；
2. 覆盖 A/B/C 三个候选方案对比；
3. 含关键决策点（签名 / 旧版本校验 / 产物矩阵 / 回退）；
4. 给出推荐方案 + 预估工作量 + 是否值得做的明确结论；
5. 结论明确「立项 task-057」或「不做，全量下载够用」。

### 2. restored_at 审计列

6. 迁移器 v2：`ALTER TABLE safe_box ADD COLUMN restored_at TEXT NULL`（不破坏 v1 已有数据）；
7. `restore` / `restore_many` 改 UPDATE（status='restored' + restored_at=NOW()），不再 DELETE；
8. `list` 查询过滤 `status='in_box'` 时不返回 restored 行（行为不变）；
9. 单测覆盖：restore 后该行 `restored_at` 非 NULL、`status='restored'`。

### 3. 跨盘 rename 测试

10. `engine/tests/safe_box_cross_disk.rs` 存在；
11. 运行时检测多盘环境，单盘 skip（CI windows-latest 只一个盘，测试应 skip 不红）；
12. 多盘环境下断言跨盘 rename 走 copy+remove fallback，文件内容一致。

### 4. G6 排查与豁免

13. 排查结论写进任务文档「实现备注」：要么「已修，根因是 X，修法 Y」；要么「修不动，根因是 Tauri 2.x ipc.localhost 首调与 useCustomProtocol 回退策略，属于上游 bug」；
14. `gui-walkthrough-round3-checklist.md` 末尾加 G6 豁免口径说明（如果选择修不动路线）；
15. 如果选择修掉：`Settings.tsx` 加 warmup IPC 调用 + 单测覆盖。

### 后端单测

16. `cargo test -p engine` 全绿（现有 250+8 + 本任务新增 ≥3 条全绿）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `docs/research/delta-update-evaluation.md`（新建）
- `engine/src/db/migrate.rs`（v2 迁移）
- `engine/src/db/schema.rs`（safe_box 结构加 restored_at）
- `engine/src/execute/safe_box.rs`（restore 改 UPDATE）
- `engine/tests/safe_box_cross_disk.rs`（新建）
- `app/src/pages/Settings.tsx`（G6 可选修复）
- `docs/test-reports/gui-walkthrough-round3-checklist.md`（G6 豁免口径）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-056-research-and-cleanup`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-056-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `docs(research): 新增差量更新技术评估（bsdiff/xdelta3/Tauri 2 对比） (#task-056)`（单元 1）
2. `feat(db): safe_box 加 restored_at 列，restore 改 UPDATE 不再 DELETE (#task-056)`（单元 2）
   —— 含迁移器 v2 + 后端 + 单测。
3. `test(execute): 补跨盘 rename 集成测试（单盘环境 skip） (#task-056)`（单元 3）
4. `fix(ui) | docs(test): G6 设置页 IPC 红错排查——二选一（warmup 修复或门禁豁免口径） (#task-056)`（单元 4）
   —— 根据排查结果选 fix 或 docs，commit type 对应。
5. 自验全绿后，最后一次提交：`docs(state): 任务056完成，差量更新评估 + restored_at + 跨盘测试 + G6 闭环 (#task-056)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 056: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「已知问题」段把 task-017（restored_at）/ task-016 跨盘测试 / G6 三条标记已解决或明确豁免；
   - 「下一步计划」改为「根据差量更新评估结论决定是否立项 task-057；RmRestart 待产品拍板」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-28  task-056  M3 评估 + 遗留小修（差量评估 / restored_at / 跨盘测试 / G6）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（现有 250+8 + 本任务新增 ≥3 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0（如果动了 Settings.tsx）；
- `pnpm --filter app test` 退出码 0（如果动了 Settings.tsx）；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：restored_at 列写入 + 跨盘 rename fallback + G6 warmup（如修）。
- **集成测试（必做）**：`cargo test -p engine` 全绿，确保现有不回归；跨盘测试在 CI 单盘环境 skip（不红）。
- **端到端写路径（必做，本任务硬要求）**：restored_at 必须有一条真实「点恢复按钮 → 查 DB restored_at 非 NULL → 查 UI 不再显示该条」证据——不能只构造 DB 行。
- **真机冒烟（可选，非必做）**：本任务主体是 DB 迁移 + 测试 + 文档，UI 行为变化小（restored 条不再出现在保险箱列表），但合并后 NSIS 安装态需验证：
  - 迁移器 v2 在已装 v0.1.6（无 restored_at 列）的数据库上跑不报错；
  - 旧数据（in_box 行）保留。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，验证迁移器 v2 正确执行（不破坏旧数据）。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（restored 行不再出现在保险箱列表——用户可观察），打包安装冒烟**必做、不可裁剪**；真机冒烟可选（DevAgent 自验时跑一遍即可）。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 差量更新评估文档 | 文件存在 + 结论段落（推荐方案 + 工作量 + 是否值得做） |
| 迁移器 v2 | `cargo test -p engine` 输出含 migration 用例 passed |
| restored_at 写入 | 单测输出（restore 后该行 restored_at 非 NULL）+ 真机 SQL 查询 `SELECT restored_at FROM safe_box WHERE status='restored' ORDER BY rowid DESC LIMIT 1` 非 NULL |
| restored 行不显示 | 真机截图：保险箱列表不再显示已恢复行 |
| 跨盘 rename 测试 | `cargo test -p engine` 输出（CI 单盘 skip，本地多盘 passed） |
| G6 排查 | 实现备注含「已修 / 修不动」二选一结论 + commit type 对应 |
| 不回归 | `cargo test -p engine` + `pnpm test` 全绿 |
| 打包安装冒烟 | NSIS 干净目录安装 + 迁移器 v2 执行日志（不破坏旧数据） |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码/文档提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-056-research-and-cleanup`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 差量更新评估最终结论（推荐方案 + 是否立项 task-057）；
   - restored_at 迁移器 v2 diff；
   - 跨盘测试在 CI skip / 本地 passed 的输出；
   - G6 排查最终结论（fix 或 docs 路线）；
   - `cargo test -p engine` 通过输出（含新增 ≥3 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（如有）；
   - 打包安装冒烟证据（NSIS 迁移器 v2 执行日志）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（差量更新是否立项 task-057 / RmRestart 待产品拍板 / 代码签名未购证书）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务056-差量更新评估与遗留小修 (#task-056)`，
   该提交只含 `tasks/056-research-and-cleanup.md` 一个文件，落在 `feature/task-056-research-and-cleanup` 分支最前端。
