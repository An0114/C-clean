# AGENTS.md 多Agent角色与协作规范

> 适用项目：C-Disk Cleaner（Windows C 盘安全清理工具，Rust + Tauri + Web 前端）
> 生效范围：本文档发布后立项阶段收尾之后的所有开发任务。
> 本文档本身为 bootstrap 提交（#task-000），自下一个任务起所有规则强制执行。

---

## 1. 协作总规则

1. **最小上下文原则**：任何 Agent 启动时只允许读取三样东西——
   - `docs/PROJECT_STATE.md`（项目当前状态切片）
   - 当前被派发的任务文件 `tasks/NNN-<slug>.md`
   - 任务文件【涉及文件】清单内列出的源码
2. **禁止全量扫描仓库**：
   - 禁止 `Get-ChildItem -Recurse` / `find .` / `rg` 不带路径前缀地扫全仓；
   - 禁止通读 `docs/PRD.md`、`docs/TRD.md`、`docs/ARCHITECTURE.md` 全文——只允许按 `PROJECT_STATE.md` 中给出的锚点（章节号 / 行号）阅读对应片段；
   - 禁止主动翻 `git log` 历史中与当前任务无关的提交。
3. **单一职责**：一个 Agent 在一次任务中只扮演一个角色，不跨角色代行（例如 DevAgent 不自己批自己的 MR）。
4. **状态单一事实源**：`docs/PROJECT_STATE.md` 是唯一允许同步"做到哪了、下一步做什么、谁在做、哪些模块被锁住"的地方。任何 Agent 不得在自己脑子里维护另一套进度。
5. **任务编号制**：所有提交、分支、评审、交接都必须带 `#task-00X` 编号，未编号的变更一律拒绝合并。
6. **文件写锁**：`PROJECT_STATE.md` 的【模块写锁】段登记当前被哪个任务占用的模块；新任务与已有锁冲突时必须先协调，不得强推。
7. **安全敏感**：本项目涉及真实文件删除，任何清理规则的变更都必须经过 `ReviewerAgent` 的安全审查（dry-run / 风险路径黑名单），不允许 DevAgent 自评通过。

---

## 2. Agent 角色清单

### 2.1 DocsKeeper（文档与状态管家）

- **职责**
  1. 维护 `docs/PROJECT_STATE.md` 的结构与时效性；
  2. 登记新任务到 `tasks/` 目录，分配 `#task-00X` 编号；
  3. 记录架构/需求变更决策到 `docs/0X-*.md` 决策日志；
  4. 汇总 Reviewer 报告，更新状态看板。
- **允许修改文件**
  - `docs/**`（含 `docs/reviews/**` 的索引，不含评审报告正文本身）
  - `tasks/**`（任务台账、任务模板）
- **禁止行为**
  - 修改 `src-tauri/**`、`src/**`、`package.json`、`Cargo.toml`、`Cargo.lock`、`pnpm-lock.yaml` 等任何业务代码与构建清单；
  - 直接 push 到 `main`；
  - 在未与 DevAgent / Reviewer 对齐的情况下擅自改写 PRD/TRD 的结论。
- **输出规范**
  - 任务文件模板：`tasks/NNN-<slug>.md`，必须包含【目标】【涉及文件】【允许修改范围】【验收标准】【依赖任务】五段；
  - PROJECT_STATE 每次更新需在末尾追加一行 `YYYY-MM-DD HH:mm  task-00X  变更摘要`。
- **Git 权限**
  - 可在 `feature/task-00X-docs-*` 分支提交 `docs/` 与 `tasks/` 下的文件；
  - commit 必须显式列文件，禁止 `git add .`；
  - 通过 MR 合并到 `main`，不直接 push。

---

### 2.2 DevAgent（开发 Agent）

按技术方向分三个子方向，权限模型一致：

| 子方向 | 负责目录 |
|---|---|
| DevBackendRust | `src-tauri/src/**`（清理引擎、规则、沙箱、审计） |
| DevFrontendWeb | `src/**`、`web/**`（React/Vite UI、状态管理） |
| DevTauriBridge | `src-tauri/src/**/commands.rs`、`src/**/api/**`（前后端 IPC 边界） |

- **职责**
  1. 严格按 `tasks/NNN-<slug>.md` 的【允许修改范围】实现功能；
  2. 为改动补充/更新单元测试（同目录 `*_test.rs` 或 `__tests__/`）；
  3. 自验：`cargo test` / `pnpm test` / `cargo clippy` / `pnpm lint` 必须全绿；
  4. 任务完成后更新 `docs/PROJECT_STATE.md` 的进度段。
- **允许修改文件**
  - 仅任务文件【允许修改范围】中显式列出的路径；
  - 同目录下与改动强相关的测试文件。
- **禁止行为**
  - 修改范围外任何文件（包括格式化全仓、顺手改别人的模块、升级无关依赖）；
  - `git add .` / `git commit -a`；
  - 直接 push `main`；
  - 自行编写清理规则后绕过 Reviewer 安全审查；
  - 在代码里硬编码真实用户路径（`C:\Users\...`）、凭据、token；
  - 擅自修改 `.gitignore`、CI 配置、版本号。
- **输出规范**
  - 代码遵循项目既有 lint 规则；
  - 任务文件末尾追加【实现备注】段：列出实际改动文件、自验命令与结果、遗留问题；
  - 在 `PROJECT_STATE.md` 把任务状态从 `进行中` 改为 `待评审`，并登记到【待评审队列】。
- **Git 权限**
  - 分支命名：`feature/task-00X-<slug>`；
  - commit 格式：`feat(scope): ... (#task-00X)` / `fix(scope): ... (#task-00X)` / `refactor(scope): ... (#task-00X)`；
  - 只能 push 自己的 feature 分支；
  - **禁止直接 push main**；合并由 Reviewer 通过后由 DocsKeeper 或维护者执行。

---

### 2.3 ReviewerAgent（测试与评审 Agent）

- **职责**
  1. 拉取 DevAgent 的 feature 分支，跑全量测试与构建（`cargo test --all`、`pnpm test`、`cargo clippy -D warnings`）；
  2. 评审代码：正确性、边界条件、错误处理、与 TRD/ARCHITECTURE 的一致性；
  3. **安全审查（本项目专属）**：核对清理规则是否符合 L1/L2/L3 风险分级、是否触碰黑名单路径（系统目录、已注册程序文件、用户文档）、dry-run 是否默认开启；
  4. 输出评审报告，给出 `Approve / Request Changes / Block` 结论。
- **允许修改文件**
  - `docs/reviews/task-00X-<slug>.md`（评审报告）
  - 仅当结论为 Request Changes 时，可在报告中提出必须修改项，但**不直接改业务代码**。
- **禁止行为**
  - 修改 `src-tauri/**`、`src/**`、测试用例（禁止通过删/改测试来"通过"评审）；
  - 修改 `Cargo.toml`、`package.json` 依赖版本；
  - 直接 push `main`；
  - 跳过安全审查直接 Approve 涉及清理规则的 MR。
- **输出规范**
  - 评审报告必须包含：
    - 【评审范围】本次 MR 涉及的 commit 与文件清单；
    -【测试结果】逐条命令 + 通过/失败；
    -【安全检查】清理规则分级、黑名单路径、dry-run 默认值；
    -【问题清单】按 `Blocker / Major / Minor` 分级；
    -【结论】Approve / Request Changes / Block；
  - 报告命名：`docs/reviews/task-00X-YYYYMMDD.md`。
- **Git 权限**
  - 代码只读；
  - 可提交评审报告到 `docs/reviews/`，走 `feature/task-00X-review` 分支；
  - 在 MR 中给出 Approve 意见；**不执行 merge 操作本身**（merge 由维护者/DocsKeeper 按规则执行）。

---

## 3. Agent 交接流程

每次 Agent 之间的工作移交，**交接包严格限定为三样，不得多传**：

1. **`docs/PROJECT_STATE.md` 当前版本**（切片，不是 git 历史）；
2. **当前任务文件** `tasks/NNN-<slug>.md`；
3. **任务文件【涉及文件】清单内的源码**（按路径逐个提供，不打包整目录）。

### 3.1 标准交接步骤

```
DocsKeeper            DevAgent                ReviewerAgent
   |                     |                        |
   |--派发任务文件------>|                        |
   |  (含允许修改范围)   |                        |
   |                     |--实现 + 自验---------->|
   |                     |  (push feature 分支)   |
   |                     |                        |--跑测试/安全审查
   |                     |                        |--写评审报告到 docs/reviews/
   |<--更新状态看板------|<--Approve 意见----------|
   |--维护者 merge main-->|                        |
   |--释放模块写锁------->|                        |
```

### 3.2 交接硬约束

- 禁止口头"顺便看一下 PRD"——任何 PRD/TRD 引用必须来自 `PROJECT_STATE.md` 中登记的锚点；
- 禁止把整个 `src-tauri/src/` 目录交给下一个 Agent；只交任务涉及的具体文件；
- 交接时如发现任务文件【涉及文件】与实际代码不符，DevAgent 应**停下并报告 DocsKeeper**，不得自行扩大范围；
- 评审不通过时，DevAgent 在原 feature 分支追加修复 commit（仍带 `#task-00X`），不新开分支。

---

## 4. 任务完成输出标准

一个任务被视为"完成"，必须同时满足：

- [ ] 代码改动全部落在任务文件【允许修改范围】内；
- [ ] `cargo test --all`、`cargo clippy -D warnings` 通过（Rust 侧）；
- [ ] `pnpm test`、`pnpm lint` 通过（前端侧）；
- [ ] 涉及清理规则的改动，Reviewer 安全检查项全过；
- [ ] `tasks/NNN-<slug>.md` 末尾追加了【实现备注】；
- [ ] `docs/PROJECT_STATE.md` 已更新：当前进度、下一步、模块写锁、待评审队列；
- [ ] 已按 §6 规范产生 commit 并 push feature 分支；
- [ ] 评审报告落到 `docs/reviews/task-00X-*.md` 且结论为 Approve；
- [ ] MR 已合并到 `main`，远端 `main` 拉取干净。

任何一项不满足，任务状态不得改为 `已完成`。

---

## 5. 冲突处理规则

1. **同一文件被两个任务同时申请修改**：
   - 后启动的任务在 `PROJECT_STATE.md`【模块写锁】中检测到锁，必须等待先任务合并后再 rebase；
   - 如确需并行，由 DocsKeeper 合并为一个任务，不允许两个分支同时改同一文件。
2. **实现与 PRD/TRD 描述冲突**：
   - DevAgent 立即停止实现，在任务文件【实现备注】中记录冲突点；
   - 由 DocsKeeper 召集（或异步记录）决策，写入 `docs/0X-变更决策.md`；
   - 决策落地前不合并代码。
3. **评审意见与开发理解不一致**：
   - 以 TRD / ARCHITECTURE / 安全规则为准，不以人为权威；
   - 仍无法达成一致时，Block 当前任务，升级为决策记录。
4. **PROJECT_STATE.md 自身冲突（两个 Agent 同时改）**：
   - 该文件由 DocsKeeper 持有唯一写锁；其他 Agent 只能通过"提交补丁段"的方式请 DocsKeeper 合并，不直接改。

---

## 6. Git 提交与合并规范

### 6.1 分支模型

| 分支 | 用途 | 谁可 push |
|---|---|---|
| `main`（受保护） | 稳定主线 | **任何 Agent 禁止直接 push**；只允许通过 MR 合并，且 Reviewer Approve |
| `feature/task-00X-<slug>` | 单个任务的开发分支 | 任务对应的 DevAgent / DocsKeeper |
| `feature/task-00X-review` | 评审报告分支 | ReviewerAgent |

### 6.2 Commit 规则

- 格式：`<type>(<scope>): <描述> (#task-00X)`
- type 取值：`feat` / `fix` / `docs` / `test` / `refactor` / `chore`
- scope 示例：`cleaner`、`ui`、`tauri`、`security`、`docs`、`state`
- **必须显式列出文件路径**，禁止 `git add .`、`git add -A`、`git commit -a`；
- 每个任务结束必须有一次 commit 专门更新 `docs/PROJECT_STATE.md`，message 形如：
  `docs(state): 同步 task-00X 进度与写锁 (#task-00X)`。

### 6.3 各角色 Git 权限矩阵

| 操作 | DocsKeeper | DevAgent | ReviewerAgent |
|---|---|---|---|
| 提交 `docs/**` | ✅ | ❌ | 仅 `docs/reviews/**` |
| 提交 `tasks/**` | ✅ | ❌（可在任务文件末尾追加实现备注） | ❌ |
| 提交业务代码（`src-tauri/**`、`src/**`） | ❌ | ✅（限任务范围） | ❌ |
| 提交测试文件 | ❌ | ✅（与改动同目录） | ❌ |
| push feature 分支 | ✅（docs 分支） | ✅（自己的 feature） | ✅（review 分支） |
| push `main` | ❌ | ❌ | ❌ |
| 合并 MR 到 `main` | ✅（Reviewer Approve 后） | ❌ | ❌（只出 Approve 意见） |
| 改 `Cargo.toml` / `package.json` 依赖 | ❌ | 仅任务范围内、且在任务文件中声明 | ❌ |

### 6.4 合并前置条件

MR 合并到 `main` 前必须满足：
1. ReviewerAgent 报告结论为 Approve；
2. CI（测试 + lint + clippy）全绿；
3. `PROJECT_STATE.md` 已在该 MR 中同步更新；
4. 分支已 rebase 最新 `main`，无冲突。

---

## 7. Git 提交记录

| Commit | 任务 | 说明 |
|---|---|---|
| （本次 bootstrap 提交） | #task-000 | `docs(agents): 新增多Agent协作与Git权限规范 (#task-000)` |

> 后续每个任务的落地 commit 由 DocsKeeper 在此表追加一行。
