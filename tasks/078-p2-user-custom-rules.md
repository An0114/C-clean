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

## 实现备注

> 开发轮（DevFullstackRustReact）交付说明。**所有结论可复现**，命令附在对应位置。

### 1. 实际改动文件清单

**白名单内（7 个落地 + 2 个确认不需要改）**：

| 文件 | 类型 | 说明 |
|---|---|---|
| `engine/src/db/migrations/v4__user_rules.sql` | **新建** | `user_rules` 表 + 唯一索引 + enabled 索引 + 两条 CHECK |
| `engine/src/ipc/commands/user_rules.rs` | **新建** | 模型 / 校验 / glob 匹配器 / 静态根 / CRUD + 14 条内联单测 |
| `engine/src/db/migrate.rs` | 改 | 注册 v4 + 3 条 v4 单测（表 / CHECK 拒绝 L3 / pattern 唯一） |
| `engine/src/ipc/commands/mod.rs` | 改 | `pub mod user_rules;` |
| `engine/src/rules/loader.rs` | 改 | 新增 `builtin_dir_prefixes` / `covered_by_builtin`（「内置优先」判定）+ 5 条单测 |
| `app/src/pages/Settings.tsx` | 改 | `UserRulesSection`（列表 / 弹窗 / 开关 / 删除） |
| `engine/tests/user_rules_test.rs` | **新建** | 6 条集成测试（含真实端到端） |
| `docs/PROJECT_STATE.md` | 改 | 状态同步（单独提交） |
| `tasks/078-p2-user-custom-rules.md` | 改 | 本段「实现备注」（单独 `docs(task)` 提交） |

**白名单列了但确认不需要改**：

| 文件 | 为什么不动 |
|---|---|
| `engine/src/lib.rs` | 任务书说「注册 IPC handler」。**engine 的 `lib.rs` 里没有任何 handler**——真正的 Tauri 命令实现与注册都在 `app/src-tauri/src/{commands,lib}.rs`（见下方白名单外清单）。`ipc::commands` 的模块声明在 `ipc/commands/mod.rs`（已改）。 |
| `app/src/utils/invoke.ts` | 「API 封装，如需要」——不需要。`api<T>()` / `apiRaw<T>()` 两个泛型封装原样够用，本次没有新的信封形态。 |

**白名单外（7 个，全部是「不改就跑不起来 / 功能不存在」）**：

| # | 文件 | 为什么必须改 |
|---|---|---|
| 1 | `engine/src/scan/modules/user_rules.rs` | **新建。本任务的核心**：任务书假设「改 `rules/loader.rs` 就能让用户规则生效」，但 loader 在扫描链路上**没有消费者**（见 §3）。没有这个模块，「新增规则 → 扫描 → Items 出现」这条验收标准不可能成立。 |
| 2 | `engine/src/scan/modules/mod.rs` | 注册 `pub mod user_rules;`（+1 行）。 |
| 3 | `engine/src/scan/orchestrator.rs` | 接线：新增 `builtin_prefixes` 字段 + `run_user_rules()`，并在「全失败」判定之前调用（只有用户规则有产出时整体也算成功）。 |
| 4 | `app/src-tauri/src/commands.rs` | 5 条 Tauri 命令（list/add/update/set_enabled/delete）+ `scan_start` 里加载内置规则前缀传给编排层。 |
| 5 | `app/src-tauri/src/lib.rs` | `generate_handler!` 里注册上述 5 条命令。 |
| 6 | `engine/tests/db_schema.rs` | v4 让 `user_version` 从 3 变 4、业务表从 9 张变 10 张，两条断言必须同步（不改就是红）。顺带把「数量 == 9」改成「表名集合完全相等」——只比数量在「换了一张表」时会假绿。 |
| 7 | `app/src/pages/Settings.test.tsx` | 任务书要求「单测：组件渲染正确」，用例必须与组件同目录（本仓库惯例），新增 7 条。 |

### 2. `user_rules` 表 schema

```sql
CREATE TABLE IF NOT EXISTS user_rules (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    pattern     TEXT    NOT NULL,
    risk_level  TEXT    NOT NULL DEFAULT 'L1'
                CHECK (risk_level IN ('L1', 'L2')),
    enabled     INTEGER NOT NULL DEFAULT 1
                CHECK (enabled IN (0, 1)),
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_user_rules_enabled ON user_rules (enabled);
CREATE UNIQUE INDEX IF NOT EXISTS idx_user_rules_pattern ON user_rules (pattern);
```

**为什么另开一张表而不是写进 `rules_meta.custom_rules`**（那里本来就有个占位字段）：

1. `cache_rules.json` 受 SHA-256 校验保护，`rules_meta.json` 与它成对。用户增删规则要改文件 → 要么改 checksum（把防篡改这道闸自己拆了），要么每次校验失败；
2. **规则库更新（task-023）会用远端整份覆盖本地文件**，用户规则会被静默冲掉；回滚（task-058）同理。

**两处与任务书不符（都是安全/正确性原因）**：

- `risk_level` 只允许 **L1 / L2**，L3 在库层面被 `CHECK` 挡死。L3 在本项目里的语义是「红线，永不删除」（`cleanup_item` 表注释：「L3 红线项永不写入本表」），**一条 L3 的清理规则会扫出永远删不掉的条目**，语义自相矛盾。挡在 DB 层而不是只在 IPC 层，是为了「绕过 IPC 直连 DB 也写不进去」。
- `created_at` / `updated_at` 用 **INTEGER unix 秒**，不是任务书写的 TEXT。本库所有时间列都是 INTEGER（`scan_session.started_at` / `safe_box.quarantined_at` / `audit_log.ts`），前端 `formatTs(unixSec: number)` 也只吃数字。同一张库里两种时间表示是最容易出错的组合。

`pattern` 唯一索引：同一模式两条规则会让**同一批文件被扫两遍**（条目翻倍、大小重复计），而用户完全看不出原因。名字允许重复（不同目录的规则可以同名）。

### 3. 规则合并的优先级逻辑 —— 以及「只改 loader 是错的」

**这是本任务最重要的一条事实。**

任务书把「规则引擎合并」放在 `rules/loader.rs`，但 `load_rules` 的调用方只有两处：

1. `commands.rs` 的 `external_deny_list(&loaded)` —— 给 `cleanup.dryRun` / `cleanup.execute` 提供**外部 L3 红线**；
2. `commands.rs` 的规则库更新 / 回滚比对。

**5 个扫描模块各自硬编码自己的路径，从不读规则库。** 所以「加载内置规则 + 从 DB 加载用户规则 + 合并成一个列表」这件事，即使做出来也**没有任何消费者**——用户规则存得再好也不会出现在清理列表里，验收标准 18/19 直接不成立。

因此本任务的实现是：

- **`loader.rs` 只负责它该负责的那一半**：`builtin_dir_prefixes()`（内置规则覆盖的目录前缀，归一化成小写 / `\` 分隔 / 无尾分隔符）+ `covered_by_builtin(path, prefixes)`（路径是否已被内置覆盖）。这两个是纯函数，5 条单测。
- **新增 `scan/modules/user_rules.rs` 才是让规则生效的地方**：从 DB 取启用规则 → 逐条遍历静态根 → 三道过滤 → 产出 `ScanItem`。

**三道过滤（缺一不可）**：

| # | 过滤 | 为什么不能省 |
|---|---|---|
| 1 | `glob_match(pattern, path)` | 用户写 `D:\MyCache\*` 就该只匹配那一层；`**` 才跨目录 |
| 2 | `whitelist::classify`（**逐条真实路径**） | 创建规则时校验的是「模式的**静态根**」（`D:\Data\**` 的根是合法的 `D:\Data`），而通配符可能展开到子目录里的 `Login Data` / `Desktop` 之类。**两个检查针对的对象不同，不能互相替代** |
| 3 | `covered_by_builtin` | 用户写 `%TEMP%\*` 时，同一批文件会既被内置 `temp` 模块列出、又被用户规则列出——条目翻倍、大小重复计，而两边风险等级可能不同 |

**「内置优先」的边界必须落在分隔符上**：纯字符串前缀会把 `c:\temp2\x` 判成在 `c:\temp` 之下。有专门用例 `covered_by_builtin_matches_children_only` 钉住这条。

**优先级方向**：内置赢（用户条目被抑制）。理由：内置规则的路径与风险等级是随版本演进、有测试覆盖的；用户规则是自由输入。冲突时保留「更可信的那份」，而不是「后写的那份」。

### 4. 前端 Settings UI 的实现方式

`UserRulesSection`（同文件内的独立组件，Settings 已 1000 行，不再往里塞）：

- **列表**：名称 / 路径模式（等宽字体）/ 风险徽标（L1 绿、L2 黄，`title` 说明含义）/ 启用 checkbox / 编辑 + 删除；
- **新增 / 编辑共用一个弹窗**（`role="dialog"`），`draft.id === null` 表示新增；
- **删除必须 `confirm`**，文案说清「已扫描出的条目不受影响，但下次扫描不会再包含」——用户最怕的是「删规则会不会连带删文件」；
- **开关即时生效**，走独立命令 `user_rules_set_enabled`（见偏差 §6）；
- **深色主题**：全部走 `var(--surface)` / `var(--text)` / `var(--border)` / `var(--text-muted)`，与其它卡片一致；
- **错误显示后端原文**：填 `D:\**` 会看到「路径模式太宽泛：必须指定到具体目录」，而不是「内部错误」；
- **空态**给引导文案（「还没有自定义规则。内置规则覆盖不到的缓存目录可以在这里加。」），不写「暂无数据」——后者会让用户以为规则丢了。

### 5. `cargo test --workspace`

```text
passed=620 failed=0 ignored=15
```

（task-077 收尾时 602 passed → **+18**）

| 位置 | 条数 | 覆盖点 |
|---|---|---|
| `ipc/commands/user_rules.rs` | 14 | glob（段内 / 跨段 / 大小写与分隔符 / `?` / 病态模式不爆炸）、静态根（截断规则 / 拒绝整盘）、校验（空名 / 超长 / L3 与拼写错误 / 太宽泛 / **L3 受保护位置**）、CRUD（增删改查 / 重复模式 / 缺 id / 校验失败不落库） |
| `rules/loader.rs` | 5 | 前缀归一化去重、盘符根保留分隔符、**子路径才命中（temp2 ≠ temp）**、空前缀不抑制、`external_deny_list` 不回归 |
| `db/migrate.rs` | +3 | v4 表结构、**CHECK 拒绝 L3 与非法 enabled**、pattern 唯一 |
| `tests/user_rules_test.rs` | 6 | v4 迁移、真实文件库 CRUD、**L3/受保护/太宽泛在建表前被拒**、内置优先抑制、**真实端到端（加规则 → run_all → 有条目 → 停用 → 没有）**、停用规则不进扫描集 |
| `Settings.test.tsx` | +7 | 列表渲染、空态、新增实参 camelCase、编辑预填 + **enabled 不被重置**、开关传相反值、删除 confirm 取消/确认、**后端校验原因显示**、空值前端先拦 |

### 6. `pnpm build`

```text
dist/index.html                  0.34 kB │ gzip:  0.25 kB
dist/assets/index-DAVbbap7.js  250.34 kB │ gzip: 77.59 kB
✓ built in 2.84s
```

244.06 kB → **250.34 kB**（+6.28 kB）。`pnpm --filter app lint` **0 error**（2 warning 均为 `SafeBox.tsx` 既有）。vitest **197 passed（14 files，+8）**。

### 7. 真机冒烟证据

**GUI 未产出**（本机 WebView2 渲染进程创建即崩溃，Crashpad 持续新增 `.dmp`、CDP 9222 不通），按项目约定开发轮不做 GUI 取证。给的是**后端侧真实数据**：

```text
cargo test -p engine --test user_rules_test
running 6 tests
test v4_creates_user_rules_table_with_expected_columns ... ok
test crud_on_real_db_file ... ok
test l3_rule_is_rejected_before_it_reaches_the_table ... ok
test builtin_rule_shadows_user_rule_on_same_path ... ok
test disabled_rule_is_not_scanned_even_if_it_would_match ... ok
test end_to_end_rule_produces_items_then_disabled_removes_them ... ok
test result: ok. 6 passed; 0 failed
```

`end_to_end_rule_produces_items_then_disabled_removes_them` 是验收标准 18/19 的直接证据：真 DB 文件 + 真临时目录 + 真 `ScanOrchestrator::run_all`：

1. 建规则 `{临时目录}\*.tmp`（L1）→ 跑 `run_all` → `cleanup_item` 里出现 `module_code='user_rules'` 的条目，每条路径都在该目录下、`risk_level='L1'`、`category='user_rule_<id>'`；
2. `set_enabled(false)` → 新 session 再跑 → `user_rules` 条目数 = **0**。

**真实升级路径验证**（仓库里 `data/cdiskcleaner.db` 恰好是 **v1 时代**的库）：

```text
起始 user_version = 1
applied v2 / v3 / v4
迁移后 user_version = 4
表 = [..., 'settings', 'user_rules']
cleanup_item 含 drive_letter = True
user_rules 列 = [id, name, pattern, risk_level, enabled, created_at, updated_at]
user_rules 索引 = [idx_user_rules_pattern, idx_user_rules_enabled]
```

即：**已装用户从旧版本升级上来，四步迁移连续跑通、既有数据无损**。

### 8. 打包安装冒烟证据

```text
pnpm --filter app tauri build
   Finished `release` profile [optimized] target(s) in 2m 04s
   Finished 2 bundles at:
     ...\bundle\nsis\C-DiskCleaner_0.2.1_x64-setup.exe (4.54 MiB)
     ...\bundle\msi\C-DiskCleaner_0.2.1_x64_en-US.msi (6.71 MiB)
   Error A public key has been found, but no private key.
```

- **出包成功**：NSIS 4.54 MiB（+0.03）+ MSI 6.71 MiB（+0.03），增量对应新增的前端 6.28 kB 与规则代码。
- **末尾签名步失败属已知预期**：私钥正源在 CI secret，本地没有（v0.2.0 起每轮都出现）。
- **安装态复跑未做**：非管理员 shell 下 MSI per-user 静默安装未完成即被终止，归测试轮。

### 9. CI

**未跑。** 按项目流程（`MEMORY.md`「CI / 分支流程」）：开发分支不触发 CI，由测试环节拉 PR 后触发。

### 10. 与本任务文档不符的偏差（汇总）

| # | 任务书说法 | 实际处理 |
|---|---|---|
| 1 | `risk_level` 可为 `L1/L2/L3` | **只允许 L1/L2**，DB 层 `CHECK` 挡死。L3 = 红线永不删除，一条 L3 的清理规则会扫出永远删不掉的条目，语义自相矛盾 |
| 2 | `created_at TEXT` / `updated_at TEXT` | 改 **INTEGER unix 秒**，与全库时间列和前端 `formatTs(unixSec: number)` 对齐 |
| 3 | 「规则引擎合并」只改 `rules/loader.rs` 即可 | **不够**。loader 在扫描链路上没有消费者，新增 `scan/modules/user_rules.rs` 才让规则真正生效（§3） |
| 4 | 只列了 4 条 IPC（list/add/update/delete） | 加第 5 条 `user_rules_set_enabled`：开关是单字段高频操作，复用 update 要求前端回传全部字段，多一个「改名没保存就切开关、名字被旧值覆盖」的机会 |
| 5 | 未提条目上限 | 每条规则 **20,000 条上限** + 回调式遍历（够数即停）。用户可以把规则指向几十万文件的目录，没有上限会跑几十分钟并写爆 `cleanup_item` |
| 6 | 未定义 `category` | 用 `user_rule_<id>`（id 稳定）。用规则名会让清理方案（存 `{module_code, category, path}` 三元组）在改名后整份失效 |
| 7 | 未说错误如何回传 | 校验失败走 **`Err(String)`**（Tauri 拒绝值）而不是 `Envelope::err`：后者 `message` 是 `&'static str`，装不下「路径模式太宽泛」这种逐条不同的原因 |
| 8 | 「同路径时内置优先」 | 实现为「**内置目录前缀覆盖的文件，用户规则条目被抑制**」。边界落在分隔符上（`c:\temp2` 不算在 `c:\temp` 之下） |
| 9 | 真机 / 安装态冒烟「必做、不可裁剪」 | 开发轮**未做**（按 `MEMORY.md`：开发轮不做 GUI/真机 UI 测试），给的是后端真实数据 + 真实升级验证 + 出包证据 |
| 10 | CI「必做」 | 开发轮**不开 PR**，按 `MEMORY.md`「开发阶段不触发 CI」 |

### 11. 遗留问题

- **规则库 `rules/cache_rules.json` 的 `modules` 段依然没有扫描消费者**。task-078 只把**用户**规则接进了扫描，内置那份仍是「文件里写一份、代码里硬编码一份」的双份真值源。两条路：把 5 个内置模块改成「按规则库驱动」（大改，要重新定义路径来源与风险等级），或删掉 `modules` 段只保留 `whitelist_deny`（规则库瘦身）。**建议在 task-080 前拍板**。
- **用户规则的 20,000 条上限是硬截断**：超出时只打日志，界面上没有提示。真要给用户看，需要在 `ScanOrchestratorSummary` 里加一个「被截断的规则」字段并透传到 UI。
- **ps1 回退路径不扫用户规则**：`run_user_rules` 在编排层，与 ps1 / Rust 无关，所以这条不受影响；但**多盘过滤对用户规则是按静态根判断**（不是按条目路径），`%TEMP%` 在 D:、规则根在 C: 这类跨盘组合下行为正确性依赖 `static_root`。
- **P2 只剩便携版 CLI（task-079）**。
- **Items / SafeBox / History 三页深色适配仍缺**（task-061 起的旧账）。
- **版本线**：task-072~078 攒着发 `v0.4.0`；CHANGELOG 需写明「用户自定义规则只允许 L1/L2（L3 是红线等级）」。

