# 任务编号：083 软件感知地基大合并（批次二 P0 地基 + 批次三软件感知核心 + 批次四软件感知收尾）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：v0.5.0 能力线 backlog，用户拍板「批次二 + 批次三 + 批次四 8 项不拆分，打包成一个任务派发」。
> 上游依赖：task-082 已合并 main（merge `eaf533d`，UI 批量优化完成）。
> **版本线**：v0.5.0 能力线（软件感知 = v0.5.0 核心交付）。
> **本任务范围（8 个 backlog 项打包）**：
> - 批次二 P0 地基：**088** 双份真值源收敛（内置模块改读规则库）+ **089** 规则 schema 扩展 `matches`
> - 批次三软件感知核心：**090** 软件枚举器 + **091** 扩展软件清单（软件缓存识别）+ **092** L3 红线反向断言
> - 批次四软件感知收尾：**093** 多 Profile（Chrome/Edge/Firefox）+ **094** UI 软件识别展示 + **095** 规则去重联调
> **为什么合并**：用户明确拍板「批次二三四打包成一个任务」；技术上 088 规则库是 090/091 的前置、092 是 091 的安全护栏、093 是 091 的多用户扩展、094 是 091 的前端呈现、095 是收尾联调——8 项强耦合，拆开会浪费 PR 轮次。
> **不做**：批次五大文件地图（096~099）；批次六决策与 P1/P2 补全（100~104）；批次七 P2 长尾（105~107）；跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 状态字段

pending

## 任务目标

### 阶段 A：批次二 P0 地基（088 + 089）

#### A1. 规则 schema 扩展 `matches`（089，先做，是 A2 的前置）

- **现状**：`rules/cache_rules.json` 现有字段 `id/name/category/level/risk/description/paths/extensions/recursive/size_min_mb`，但路径匹配是「目录前缀匹配」，不支持复杂 glob / 文件名匹配。
- **修法**：
  - `rules/cache_rules.json` schema 新增 `matches` 字段（数组，每项 `{ "type": "glob" | "regex" | "suffix", "pattern": "..." }`）；
  - 兼容：无 `matches` 字段的旧规则仍走原 `paths` 前缀匹配；
  - `engine/src/rules/mod.rs`（或新建 `engine/src/rules/schema.rs`）：serde 反序列化 + 校验；
  - 规则引擎在命中时先按 `paths` 筛目录候选，再按 `matches` 精筛文件名；
  - 新增单元测试：glob/regex/suffix 三种 type 的命中与不命中。

#### A2. 双份真值源收敛（088，内置模块改读规则库）

- **现状**：5 个内置 Rust 模块（`engine/src/scan/modules/{temp,prefetch,browser_cache,dev_cache,windows_update}.rs`）各自硬编码了路径、扩展名、阈值，与 `rules/cache_rules.json` 是「双份真值源」——改一处忘改另一处。
- **修法**：
  - 5 个内置模块的硬编码路径/扩展名/阈值全部迁移到 `rules/cache_rules.json`（作为内置规则的一部分，category 字段标记为 `system`）；
  - Rust 模块从规则库读取配置，**不再硬编码**；
  - 保留模块「硬编码禁用 /ResetBase」（WinSxS）、「白名单命令」（提权 exe）等安全护栏；
  - 环境变量 `CCLEAN_RUST_*`（Rust 优先 / ps1 回退）逻辑不变；
  - 迁移完成后：同一规则只在 JSON 里维护一份，Rust 模块只是「执行器」。

### 阶段 B：批次三软件感知核心（090 + 091 + 092）

#### B1. 软件枚举器（090）

- **现状**：当前只扫描已知缓存目录，不知道用户装了哪些软件——没装 Chrome 就不扫 Chrome 缓存，浪费且误报。
- **修法**：
  - 新建 `engine/src/scan/software_enum.rs`：
    - 读注册表 `HKLM/HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\*`（DisplayName/DisplayVersion/Publisher/InstallLocation）；
    - 读 WMI `Win32_Product`（可选，慢，默认关，环境变量 `CCLEAN_ENUM_WMI=1` 开启）；
    - 输出 `Vec<SoftwareEntry { name, version, publisher, install_location, registry_key }>`；
  - 去重：同一软件 HKLM + HKCU 都有时按 HKLM 优先；
  - 单元测试：构造注册表 hive 快照做命中测试（不真读系统注册表）。

#### B2. 扩展软件清单 / 软件缓存识别（091）

- **现状**：浏览器缓存（Chrome/Edge/Firefox）路径硬编码；其他软件（VSCode/Slack/Discord/Teams/WeChat/QQ 等）的缓存目录不识别。
- **修法**：
  - 新建 `engine/src/scan/modules/software_cache.rs`：
    - 基于 B1 枚举结果，按软件名匹配「缓存路径模板表」（这张表写在 `rules/software_rules.json`，新文件，schema：`{ software_name, cache_paths[], level, risk }`）；
    - 命中已安装软件 → 扫其缓存目录；未安装 → 跳过；
    - 示例条目：VSCode (`%APPDATA%\Code\Cache`)、Slack、Discord、Teams、WeChat (`%APPDATA%\Tencent\WeChat\...`)、QQ；
  - 多 Profile 在 B3 处理。

#### B3. L3 红线反向断言（092，安全护栏）

- **现状**：L3 规则（系统文件/受保护路径）是「命中就拒绝」，但没有「反向校验」——新增软件缓存规则时可能误把 L3 路径写进 L1。
- **修法**：
  - 规则引擎加载时做一次「反向断言」：
    - 所有 L1/L2 规则的 `paths` 必须不在 L3 红线清单内；
    - 命中冲突 → 启动时 panic 并打印冲突规则 id（防呆）；
  - 新增 L3 红线清单（`rules/l3_redlines.json`，新文件：System32/WinSxS/Program Files/ProgramData/Microsoft-Windows/...）；
  - 单元测试：故意构造一条与 L3 冲突的 L1 规则，断言启动时 panic。

### 阶段 C：批次四软件感知收尾（093 + 094 + 095）

#### C1. 多 Profile 支持（093）

- **现状**：Chrome/Edge/Firefox 缓存路径硬编码为单一 `Default` profile。
- **修法**：
  - 枚举 `%LOCALAPPDATA%\Google\Chrome\User Data\*` 下所有 profile 目录（Default/Profile 1/Profile 2/...）；
  - 对每个 profile 扫其 `Cache/Code Cache/GPUCache`；
  - Firefox：读 `%APPDATA%\Mozilla\Firefox\Profiles\profiles.ini` 列出所有 profile；
  - 命中结果带 `profile_name` 字段（UI 展示用）。

#### C2. UI 软件识别展示（094）

- **现状**：Items 页只列「路径 + 大小 + 风险」，不告诉用户「这是哪个软件的缓存」。
- **修法**：
  - 扫描结果新增 `software_name` 字段（B2 识别到时填，否则 null）；
  - Items 页：
    - 加「软件」列（在路径列之前），显示软件名 + 版本；
    - 同软件多项可折叠（按 software_name 分组）；
    - 深色主题用 CSS 变量（沿用 task-081/082 体系）；
  - 后端 IPC：`scan_start` 返回结果带 software_name（serde 字段）。

#### C3. 规则去重联调（095）

- **现状**：5 个内置模块迁移到规则库后（A2），可能与软件规则（B2）路径重叠（如浏览器缓存既在 system 规则又在 software 规则）。
- **修法**：
  - 规则加载时做一次「路径去重」：
    - 同一文件被多条规则命中时，按「更具体路径优先」+「更高 level 优先」；
    - 去重后只算一次大小（不重复统计）；
  - 单元测试：构造两条重叠规则，断言只命中一次。

**本任务不做**：
- 不做批次五大文件地图（096~099，留后续）；
- 不做批次六决策与 P1/P2 补全（100~104，留后续）；
- 不做批次七 P2 长尾（105~107，留后续）；
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）；
- 不做代码签名 / 品牌图标（死路）。

## 前置条件

1. `main` 已合并 task-082（merge `eaf533d`），UI 批量优化完成；
2. 全局 CSS 变量体系已建立（task-061/081/082）；
3. 现有规则库 `rules/cache_rules.json` 已可被 Rust 加载（task-078 用户自定义规则已接线）；
4. 关键锚点（必须全部在白名单内）：
   - 规则 schema：`rules/cache_rules.json`（扩展 matches 字段）
   - 规则库新文件：`rules/software_rules.json`（新建）、`rules/l3_redlines.json`（新建）
   - Rust 规则引擎：`engine/src/rules/mod.rs` 或 `engine/src/rules/schema.rs`
   - 5 个内置模块：`engine/src/scan/modules/{temp,prefetch,browser_cache,dev_cache,windows_update}.rs`（改读规则库）
   - 新建软件枚举：`engine/src/scan/software_enum.rs`
   - 新建软件缓存模块：`engine/src/scan/modules/software_cache.rs`
   - 数据库迁移：`engine/src/db/migrations/v5__software_awareness.sql`（新建，软件清单缓存表）
   - IPC：`engine/src/ipc/commands/scan_start.rs`（返回 software_name）
   - Tauri 接线：`app/src-tauri/src/commands.rs` + `app/src-tauri/src/lib.rs`
   - 前端：`app/src/pages/Items.tsx`（软件列 + 折叠分组）+ `app/src/utils/invoke.ts`
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 阶段 A：P0 地基

1. **A1. matches schema**：`rules/cache_rules.json` 支持 `matches` 字段（glob/regex/suffix）；serde 反序列化通过；无 matches 旧规则仍走 paths 前缀匹配；
2. **A1. 单元测试**：glob/regex/suffix 三种 type 命中 + 不命中 ≥6 条全绿；
3. **A2. 内置模块改读规则库**：5 个内置模块硬编码路径/扩展名/阈值全部迁移到 JSON；Rust 模块从规则库读取；同一规则只在 JSON 维护一份；
4. **A2. 不回归**：Rust 优先 / ps1 回退环境变量逻辑不变；WinSxS 禁用 /ResetBase 保留；
5. **A2. 单元测试**：迁移后扫描结果与迁移前一致（快照对比 ≥5 条用例）。

### 阶段 B：软件感知核心

6. **B1. 软件枚举器**：注册表 Uninstall 键枚举成功；去重逻辑正确；
7. **B1. 单元测试**：构造注册表 hive 快照命中测试 ≥3 条全绿；
8. **B2. 软件缓存识别**：基于枚举结果匹配 software_rules.json；未安装软件跳过；
9. **B2. 软件规则文件**：`rules/software_rules.json` 含 ≥5 个常见软件（VSCode/Slack/Discord/Teams/WeChat）；
10. **B3. L3 反向断言**：规则加载时校验 L1/L2 不与 L3 红线冲突；冲突时 panic 并打印冲突 id；
11. **B3. 单元测试**：构造冲突规则断言 panic ≥2 条全绿。

### 阶段 C：软件感知收尾

12. **C1. 多 Profile**：Chrome/Edge 枚举 `User Data\*` 所有 profile；Firefox 读 profiles.ini；
13. **C1. 单元测试**：多 profile 枚举 ≥2 条全绿；
14. **C2. UI 软件列**：Items 页加「软件」列显示名称+版本；同软件可折叠分组；
15. **C2. 深色主题**：软件列用 CSS 变量，不引入硬编码色；
16. **C3. 规则去重**：同一文件被多条规则命中时只算一次；
17. **C3. 单元测试**：重叠规则去重 ≥2 条全绿。

### 后端 + 前端整体

18. **后端单测**：`cargo test --workspace` 全绿（本任务新增 ≥15 条全绿，总数从 638 → ≥653）；
19. **前端单测**：`pnpm --filter app test` 全绿（现有 206 不回归 + 新增 ≥3 条）；
20. **Lint**：`cargo clippy -D warnings` + `pnpm lint` 0 error；
21. **构建**：`cargo build --release` + `pnpm build` 成功；
22. **真机关联扫描**：真机跑一次完整扫描，识别到 ≥3 个已安装软件的缓存并正确归类。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**规则库（新建 + 修改）**：
- `rules/cache_rules.json`（扩展 matches 字段 + 5 个内置模块路径迁入）
- `rules/software_rules.json`（**新建**，软件缓存路径模板表）
- `rules/l3_redlines.json`（**新建**，L3 红线清单）
- `rules/rules_meta.json`（版本号 + schema 版本升级）

**Rust 规则引擎**：
- `engine/src/rules/mod.rs`（或 `engine/src/rules/schema.rs`，**新建**）：serde schema + matches 解析 + L3 反向断言 + 路径去重
- `engine/src/rules/engine.rs`（如已存在，修改命中逻辑）

**Rust 扫描模块（改读规则库）**：
- `engine/src/scan/modules/mod.rs`（模块注册）
- `engine/src/scan/modules/temp.rs`
- `engine/src/scan/modules/prefetch.rs`
- `engine/src/scan/modules/browser_cache.rs`
- `engine/src/scan/modules/dev_cache.rs`
- `engine/src/scan/modules/windows_update.rs`
- `engine/src/scan/modules/software_cache.rs`（**新建**，B2）
- `engine/src/scan/software_enum.rs`（**新建**，B1）
- `engine/src/scan/orchestrator.rs`（接线软件枚举 + 多 profile）
- `engine/src/scan/drive_enum.rs`（不改动，多盘逻辑保留）

**数据库迁移**：
- `engine/src/db/migrations/v5__software_awareness.sql`（**新建**，软件清单缓存表 software_inventory）

**IPC 层**：
- `engine/src/ipc/commands/scan_start.rs`（返回结果带 software_name + profile_name）
- `engine/src/ipc/commands/mod.rs`（注册新命令）
- `engine/src/ipc/commands/software_inventory.rs`（**新建**，枚举软件列表命令）

**Tauri 接线**：
- `app/src-tauri/src/lib.rs`（注册新命令 + 软件枚举在启动时跑一次）
- `app/src-tauri/src/commands.rs`（注册 handler）

**前端**：
- `app/src/pages/Items.tsx`（软件列 + 折叠分组）
- `app/src/utils/invoke.ts`（新命令封装）
- `app/src/App.tsx`（**仅在需要时**增加软件 tab 入口；若不新增 tab 则不动）

**测试**：
- `engine/src/rules/schema_test.rs`（**新建**，matches + L3 断言 + 去重）
- `engine/src/scan/software_enum_test.rs`（**新建**）
- `app/src/pages/__tests__/Items.test.tsx`（软件列渲染）

**文档**：
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

**不修改**：
- 托盘菜单（task-082 已完成，不改动）
- 主题 CSS 变量体系（task-061/081/082 已建立，不改动）
- engine/src/scan/winsxs.rs（DISM 逻辑不变，只是路径从规则库读）
- engine/src/scan/residual.rs（残留扫描逻辑不变）
- engine/src/scan/big_files.rs（大文件地图留批次五）
- cli/src/main.rs（CLI 不改动，若需新增软件枚举子命令留后续）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-083-software-awareness-batch234`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-083-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 8 个 backlog 项拆 ≥6 个原子提交（用户要求强耦合大块最多 6 个但必须拆 ≥4 个原子提交；本任务 8 项强耦合，按阶段合并为 6 个提交）：

1. `feat(rules): schema 扩展 matches 字段（glob/regex/suffix）+ L3 反向断言 (#task-083)`（A1 + B3）
   —— 含 `rules/cache_rules.json` + `rules/l3_redlines.json` + `engine/src/rules/*` + 单测。
2. `refactor(scan): 5 个内置模块改读规则库（双份真值源收敛） (#task-083)`（A2）
   —— 含 5 个 modules/*.rs + `rules/cache_rules.json` 内置路径迁入 + 快照对比单测。
3. `feat(scan): 软件枚举器（注册表 Uninstall + 去重） (#task-083)`（B1）
   —— 含 `engine/src/scan/software_enum.rs` + `engine/src/ipc/commands/software_inventory.rs` + hive 快照单测。
4. `feat(scan): 软件缓存识别（software_rules.json + 未安装跳过） (#task-083)`（B2）
   —— 含 `rules/software_rules.json` + `engine/src/scan/modules/software_cache.rs` + orchestrator 接线。
5. `feat(scan): 多 Profile 支持（Chrome/Edge User Data 枚举 + Firefox profiles.ini） (#task-083)`（C1 + C3 去重）
   —— 含 browser_cache.rs / software_cache.rs 多 profile 逻辑 + 去重单测。
6. `feat(ui): Items 页软件列 + 折叠分组（深色 CSS 变量） (#task-083)`（C2）
   —— 含 `app/src/pages/Items.tsx` + `app/src/utils/invoke.ts` + scan_start 返回 software_name + 前端单测。
7. 自验全绿后，最后一次提交：`docs(state): 任务083完成，软件感知地基大合并（批次二三四 8 项） (#task-083)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 088/089/090/091/092/093/094/095: ...`；
   - 「最近一次变更」追加一段，含 8 项摘要 + commit hash；
   - 「下一步计划」改为「批次二三四软件感知全部完成；下一阶段：批次五大文件地图（096~099）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-09  task-083  软件感知地基（批次二三四 8 项合并）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0（≥653 passed）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0（≥209 passed）；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：matches schema / L3 反向断言 / 软件枚举 hive 快照 / 多 profile 枚举 / 规则去重，本任务新增 ≥15 条；
- **集成测试（必做）**：`cargo test --workspace` 全绿（现有 638 不回归 + 新增 ≥15）；
- **端到端（必做）**：
  - 启动 app → 触发扫描 → 真机识别到 ≥3 个已安装软件（如 VSCode/Chrome/WeChat）的缓存；
  - Items 页「软件」列正确显示软件名 + 版本；
  - 同软件多项可折叠；
  - 深色主题下软件列样式正确；
  - L3 红线（如 System32）不被误判为 L1；
- **真机冒烟（必做、不可裁剪）**：
  - 识别到的软件数 ≥3；
  - 多 profile 用户（如有）Chrome/Edge 所有 profile 都被扫到；
  - L3 反向断言：故意在 JSON 里写一条冲突规则，app 启动时 panic（防呆验证）；
  - CDP 抓 console 0 红错（G6 已知项除外）；
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述端到端 5 条 + 真机 4 条复跑；
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（软件列 / 折叠分组 / 深色主题，全部真机可观察），且命中「从 DB/规则库派生 UI」（software_name 从后端规则引擎派生到 Items 列），**真机/打包冒烟必做、不可裁剪**，且必须含真实「点扫描按钮 → 后端枚举软件 → 查 DB/规则库 → UI 显示软件列」端到端证据。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| matches schema | `cargo test rules::schema` 通过输出 + JSON 示例片段 |
| L3 反向断言 | 故意构造冲突规则 → app 启动 panic 日志截图 |
| 5 内置模块改读规则库 | 代码 diff：硬编码路径全删 + JSON 路径片段 |
| 软件枚举器 | 真机跑 `software_inventory` 命令返回 ≥5 条已安装软件 |
| 软件缓存识别 | 扫描结果中 VSCode/Slack/WeChat 缓存被识别 |
| 多 Profile | 真机 Chrome 多 profile 截图（Profile 1/2 都出现） |
| UI 软件列 | 真机截图：Items 页「软件」列 + 折叠分组 |
| 规则去重 | `cargo test rules::dedup` 通过输出 |
| 不回归 | `cargo test --workspace` 638+ 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 端到端 5 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 6 个代码提交 + 1 个 `docs(state)` 提交，共 7 个 commit，全部 push 到 `feature/task-083-software-awareness-batch234`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - matches schema 三种 type 的实际示例；
   - 5 个内置模块硬编码路径迁移到 JSON 的对照表；
   - 软件枚举器实际识别到的软件数（真机）；
   - software_rules.json 实际收录的软件清单；
   - 多 Profile 实测（Chrome/Edge/Firefox profile 数）；
   - UI 软件列实际效果截图路径；
   - L3 反向断言 panic 防呆日志；
   - `cargo test --workspace` 通过输出（含新增 ≥15 条用例，总数 ≥653）；
   - `pnpm test` 输出（无回归 + 新增 ≥3 条，总数 ≥209）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（12 条表格逐项）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（批次二三四软件感知全部完成；下一阶段：批次五大文件地图 096~099）；
3. `docs/PROJECT_STATE.md` 已按第 7 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务083-软件感知地基大合并（批次二三四 8 项） (#task-083)`，
   该提交只含 `tasks/083-software-awareness-batch234.md` 一个文件，落在 `feature/task-083-software-awareness-batch234` 分支最前端。
