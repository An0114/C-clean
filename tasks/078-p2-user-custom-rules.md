# 任务编号：078 P2——用户自定义规则管理（内置规则 + 用户规则合并）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：P2 未做清单第二项（用户自定义规则）。
> 上游依赖：task-077 已合并 main（merge `e5a1b22`，多盘扫描支持完成）。
> **版本线**：v0.4.x P2 持续——用户自定义规则。
> **本任务范围（4 个功能单元）**：用户规则 DB 表 + CRUD IPC + 规则引擎合并（内置 + 用户）+ 前端 Settings 自定义规则管理 UI。
> **为什么合并**：用户自定义规则是完整闭环（DB 存储 → IPC CRUD → 引擎合并 → 前端管理 UI），一次 PR 完成。
> **不做**：便携版 CLI（留 task-079）；跨盘恢复 fallback（维持现状）。

## 状态字段

pending

## 任务目标

### 1. 用户规则 DB 表 + CRUD IPC

- **现状**：规则库只有内置规则（`rules/cache_rules.json`，SHA-256 校验），无用户自定义规则存储。
- **修法**：
  - `engine/src/db/migrations/v4__user_rules.sql`（新建）：
    - `user_rules` 表：
      - `id INTEGER PRIMARY KEY AUTOINCREMENT`
      - `name TEXT NOT NULL`（规则名）
      - `pattern TEXT NOT NULL`（匹配路径 glob，如 `D:\MyCache\*`）
      - `risk_level TEXT NOT NULL DEFAULT 'L1'`（L1/L2/L3）
      - `enabled INTEGER NOT NULL DEFAULT 1`
      - `created_at TEXT NOT NULL`
      - `updated_at TEXT NOT NULL`
    - 索引：`idx_user_rules_enabled`
  - `engine/src/ipc/commands/user_rules.rs`（新建）：
    - `user_rules_list()` → 返回所有用户规则；
    - `user_rules_add(name, pattern, risk_level)` → 新增；
    - `user_rules_update(id, name, pattern, risk_level, enabled)` → 更新；
    - `user_rules_delete(id)` → 删除。

### 2. 规则引擎合并（内置 + 用户）

- **现状**：`engine/src/rules/loader.rs` 只加载内置规则。
- **修法**：
  - `engine/src/rules/loader.rs`：
    - 加载内置规则（现有逻辑不变）；
    - 从 DB 加载用户规则（`enabled=1`）；
    - 合并为统一规则列表，用户规则优先级低于内置规则（同路径时内置优先）；
    - 用户规则风险等级可自定义（L1/L2/L3）。

### 3. 前端 Settings 自定义规则管理 UI

- **修法**：
  - `app/src/pages/Settings.tsx`：
    - 加「自定义规则」区块；
    - 规则列表（表格：名称 / 路径模式 / 风险等级 / 启用开关）；
    - 「新增规则」按钮 → 弹窗表单（名称 / glob 路径 / 风险等级下拉）；
    - 每行「编辑」「删除」按钮（删除需 confirm）；
    - 启用开关切换即生效；
    - 深色主题适配。

### 4. 扫描时用户规则生效验证

- **修法**：
  - 新增一条用户规则（如 `D:\TestCache\*`，L1）；
  - 执行扫描，验证该路径文件出现在 Items 页；
  - 关闭该规则，重新扫描，验证该路径文件不再出现。

**本任务不做**：
- 不做便携版 CLI（留 task-079）；
- 不做跨盘恢复 fallback（维持现状）；
- 不做规则导入/导出（留后续）。

## 前置条件

1. `main` 已合并 task-077（merge `e5a1b22`），多盘扫描完成；
2. 关键锚点（必须全部在白名单内）：
   - 后端新建：`engine/src/db/migrations/v4__user_rules.sql`
   - 后端新建：`engine/src/ipc/commands/user_rules.rs`
   - 后端修改：`engine/src/db/migrate.rs`（注册 v4 迁移）
   - 后端修改：`engine/src/ipc/commands/mod.rs`（注册 user_rules 命令）
   - 后端修改：`engine/src/lib.rs`（注册 IPC handler）
   - 后端修改：`engine/src/rules/loader.rs`（合并内置 + 用户规则）
   - 前端修改：`app/src/pages/Settings.tsx`（自定义规则管理 UI）
   - 前端修改：`app/src/utils/invoke.ts`（API 封装，如需要）
   - 后端测试：`engine/tests/user_rules_test.rs`（新建）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 用户规则 DB 表 + CRUD IPC

1. v4 迁移成功（`user_rules` 表创建）；
2. `user_rules_list()` 返回所有规则；
3. `user_rules_add()` 新增规则；
4. `user_rules_update()` 更新规则（含启用开关）；
5. `user_rules_delete()` 删除规则；
6. 单测：CRUD 全流程。

### 2. 规则引擎合并

7. 内置规则加载不变；
8. 用户规则（enabled=1）合并到扫描规则列表；
9. 同路径时内置规则优先；
10. 用户规则风险等级自定义生效；
11. 单测：合并结果正确。

### 3. 前端 Settings UI

12. 自定义规则列表显示正确；
13. 新增规则弹窗表单工作；
14. 编辑 / 删除按钮工作（删除 confirm）；
15. 启用开关切换即生效；
16. 深色主题适配；
17. 单测：组件渲染正确。

### 4. 端到端验证

18. 新增用户规则 → 扫描 → 该路径文件出现在 Items；
19. 关闭规则 → 重新扫描 → 该路径文件不再出现。

### 后端单测

20. `cargo test --workspace` 全绿（现有 + 本任务新增 ≥5 条全绿）。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**后端新建**：
- `engine/src/db/migrations/v4__user_rules.sql`（新建）
- `engine/src/ipc/commands/user_rules.rs`（新建）
- `engine/tests/user_rules_test.rs`（新建）

**后端修改**：
- `engine/src/db/migrate.rs`（注册 v4 迁移）
- `engine/src/ipc/commands/mod.rs`（注册 user_rules 命令）
- `engine/src/lib.rs`（注册 IPC handler）
- `engine/src/rules/loader.rs`（合并内置 + 用户规则）

**前端修改**：
- `app/src/pages/Settings.tsx`（自定义规则管理 UI）
- `app/src/utils/invoke.ts`（API 封装，如需要）

**文档**：
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-078-p2-user-custom-rules`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-078-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `feat(db): v4 迁移 user_rules 表 + CRUD IPC (#task-078)`（单元 1）
   —— 含 `migrations/v4__user_rules.sql` + `migrate.rs` + `ipc/commands/user_rules.rs` + `commands/mod.rs` + `lib.rs`。
2. `feat(rules): 规则引擎合并内置 + 用户规则 (#task-078)`（单元 2）
   —— 含 `engine/src/rules/loader.rs`。
3. `feat(ui): Settings 自定义规则管理 UI（列表 + 新增/编辑/删除 + 启用开关） (#task-078)`（单元 3）
   —— 含 `app/src/pages/Settings.tsx` + `utils/invoke.ts`。
4. `test(rules): user_rules CRUD + 合并单测覆盖 (#task-078)`（单元 4）
   —— 含 `engine/tests/user_rules_test.rs`。
5. 自验全绿后，最后一次提交：`docs(state): 任务078完成，P2——用户自定义规则管理就绪 (#task-078)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 078: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「P2 持续：便携版 CLI（task-079）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-04  task-078  P2（用户自定义规则）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：CRUD、规则合并、前端组件。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **端到端写路径（必做，本任务硬要求）**：
  - 新增用户规则 → 扫描 → Items 页显示该路径文件；
  - 关闭规则 → 重新扫描 → Items 页不再显示。
- **真机冒烟（必做、不可裁剪）**：
  - Settings 页「自定义规则」区块显示正确；
  - 新增规则弹窗表单可提交；
  - 启用开关切换后扫描结果变化；
  - 删除规则 confirm 弹窗；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 4 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（Settings 自定义规则 UI，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| CRUD IPC | 单测输出（增删改查正确） |
| 规则合并 | 单测输出（内置 + 用户合并正确） |
| Settings UI | 真机截图：自定义规则列表 |
| 新增规则弹窗 | 真机证据：弹窗表单提交 |
| 启用开关生效 | 真机证据：开关切换后扫描结果变化 |
| 端到端 | 真机证据：新增规则 → Items 显示 / 关闭 → Items 不显示 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 4 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-078-p2-user-custom-rules`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - user_rules 表 schema 确认；
   - 规则合并优先级逻辑（内置优先于用户）；
   - 前端 Settings UI 实现方式（弹窗 / 表格 / 开关）；
   - `cargo test --workspace` 通过输出（含新增 ≥5 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（4 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（P2 持续：便携版 CLI task-079；Items/SafeBox/History 深色适配待补）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务078-P2 用户自定义规则管理 (#task-078)`，
   该提交只含 `tasks/078-p2-user-custom-rules.md` 一个文件，落在 `feature/task-078-p2-user-custom-rules` 分支最前端。
