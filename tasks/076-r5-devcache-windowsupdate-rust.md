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

## 实现备注

> 开发轮（DevFullstackRustReact）交付说明。**所有结论可复现**，命令附在对应位置。

### 1. 实际改动文件清单

| 文件 | 类型 | 说明 |
|---|---|---|
| `engine/src/scan/modules/dev_cache.rs` | **新建** | dev_cache 扫描 + 10 条内联单测 |
| `engine/src/scan/modules/windows_update.rs` | **新建** | windows_update 扫描 + 5 条内联单测 |
| `engine/src/scan/modules/mod.rs` | 改 | 注册两个新模块（`pub mod` 两行，**分两次提交**——见偏差说明） |
| `engine/src/scan/orchestrator.rs` | 改 | 两个模块接线 + 五个开关名抽常量 + 单测从 10 → 12 |
| `engine/tests/devcache_windowsupdate_test.rs` | **新建** | 6 条集成测试 + 1 条 `#[ignore]` 真机冒烟 |
| `docs/PROJECT_STATE.md` | 改 | 按第 5 条提交规范更新（单独提交） |
| `tasks/076-r5-devcache-windowsupdate-rust.md` | 改 | 本段「实现备注」（输出要求 2，单独 `docs(task)` 提交） |

**与白名单的差异：0 处。** 本任务是 R5 五连迁的最后一环，白名单这次写得完整——没有出现前几轮那种「不改就跑不起来 / CI 会红」的白名单外文件。特别地：

- `engine/tests/scan_orchestrator.rs` 与 `scan_progress_percent.rs` **不需要动**：task-075 引入的 `CCLEAN_RUST_ALL=0` 总开关已经把它们和「又多了两个 Rust 模块」解耦，这次加模块后直接全绿（这正是 task-075 引入总开关的预期收益）；
- `scripts/scan_dev_cache.ps1` / `scan_windows_update.ps1` **按任务书要求未改**，保留作回退。

**唯一的形式偏差**：任务书的提交计划把 `modules/mod.rs` 只列在单元 1 下，但 `windows_update.rs` 的注册也得写进同一个文件。为让**每个 commit 都能单独编译**，`mod.rs` 拆成两次提交（单元 1 加 `dev_cache`、单元 2 加 `windows_update`）。合并成一次的话单元 1 会引用尚不存在的文件，commit 级 `cargo check` 必红。

### 2. Rust 实现与 ps1 输出对比（字段 / 格式 / 筛选逻辑）

#### 2.1 dev_cache

| 字段 | ps1 | Rust |
|---|---|---|
| `module_code` | `'dev_cache'` | `MODULE_CODE = "dev_cache"` |
| `category` | `'npm_cache'` / `'pip_cache'` / `'pnpm_store'` | 同（`CATEGORY_NPM` / `CATEGORY_PIP` / `CATEGORY_PNPM`） |
| `path` | `$_.FullName` | `to_string_lossy()` |
| `size_bytes` | `$_.Length` | `md.len()` |
| `risk_level` | `1`（`$riskCache`） | `RISK_L1 = 1` |
| 遍历 | `Get-ChildItem -Recurse -Force -File` | `super::scan_dir` |
| 符号链接 | `if ($_.LinkType) { return }` | `is_reparse_point()` |
| 筛选 | **无** | **无** |

**四个根，顺序与 ps1 的 `$roots` 完全一致**（顺序不影响正确性，但一致才能在调试对比时逐行对上）。

#### 2.2 windows_update

| 字段 | ps1 | Rust |
|---|---|---|
| `module_code` | `'windows_update'` | `MODULE_CODE = "windows_update"` |
| `category` | `'update_download'` | `CATEGORY_UPDATE_DOWNLOAD` |
| `risk_level` | `2`（`$riskUpdateDownload`） | `RISK_L2 = 2` |
| 其余 | 同上 | 同上 |

**一处刻意修正**：ps1 硬编码 `C:\Windows\SoftwareDistribution\Download`，Rust 读 `%SystemRoot%`。系统装在 D 盘或改过 `SystemRoot` 的机器上 ps1 会扫错地方；`SystemRoot` 缺失时才退回 `C:\Windows`（与 ps1 一致，属兜底）。

### 3. npm / pip / pnpm 缓存路径确认

**任务书「现状」段与实际脚本不符**，按脚本为准：

| 任务书说法 | `scan_dev_cache.ps1` 实际 `$roots` | 本次实现 |
|---|---|---|
| npm cache（`%APPDATA%\npm-cache`） | `%LOCALAPPDATA%\npm-cache` + `%APPDATA%\npm-cache` **两条** | 两条都扫，共用类别 `npm_cache` |
| pip cache | `%LOCALAPPDATA%\pip\cache` | 同 |
| pnpm store | `%LOCALAPPDATA%\pnpm-store` | 同 |
| **Gradle cache** | **不存在** | **不加**（见下） |
| **Maven cache** | **不存在** | **不加**（见下） |
| **node_modules** | **不存在** | **不加**（见下） |

**为什么 Gradle / Maven / node_modules 一个都不加**（这是本任务唯一「违背任务书字面要求」的地方，理由必须写清）：

1. **验收标准第 14 条自相矛盾**：任务书要求「Rust 实现与 ps1 输出抽样对比（格式一致）」。多扫三个目录，两边条目数必然不同，那条对比就只能靠在测试里过滤差异来过关——**把一个真实行为差异藏进测试白名单，比不加更糟**；
2. **Gradle / Maven 删错代价大**：`~/.gradle/caches`、`~/.m2/repository` 里混着依赖包与版本缓存，删了下次构建要**重新下载几百 MB**。纳不纳入、纳哪几个子目录，需要独立任务 + 独立开关；
3. **`node_modules` 是 L3**：它是项目依赖不是缓存，删了用户项目当场跑不起来。属 `execute::whitelist` 的用户数据红线范畴，不该出现在清理列表里。

有一条负向单测 `scan_roots_never_include_node_modules_or_gradle` 把「不扫」钉成契约——将来谁想加，必须先改这条测试，也就必须先回答上面三个问题。

npm 为什么扫两个位置：npm 6 及更早放 `%APPDATA%\npm-cache`，npm 7 起改到 `%LOCALAPPDATA%\npm-cache`，老机器升级后两个可能并存。

### 4. SoftwareDistribution\Download 路径确认

`%SystemRoot%\SoftwareDistribution\Download`，类别 `update_download`，**L2**。

L2 的理由（v0.2.1 Bug-2 就是这里被写死成 L1）：目录下的文件**大多属 SYSTEM**，普通权限删不掉；而且**更新正在安装时删它会中断安装**，可能让系统停在「已解压组件 + 未提交事务」的半装状态。它不是「删了自动重建」那么轻，必须逐项确认、不参与「全选 L1」。

**本机实测这个目录居然可读**（9 项 / 29.6 MB），所以 ps1 与 Rust 的对比是**真实比对**，不是空集对空集。

### 5. Orchestrator 接线方式（环境变量回退）

| 模块 | 开关 | 回退后跑的脚本 |
|---|---|---|
| temp | `CCLEAN_RUST_TEMP=0` | `scan_temp.ps1` |
| windows_update | `CCLEAN_RUST_WINDOWS_UPDATE=0` | `scan_windows_update.ps1` |
| prefetch | `CCLEAN_RUST_PREFETCH=0` | `scan_prefetch.ps1` |
| browser_cache | `CCLEAN_RUST_BROWSER_CACHE=0` | `scan_browser_cache.ps1` |
| dev_cache | `CCLEAN_RUST_DEV_CACHE=0` | `scan_dev_cache.ps1` |
| **全部** | `CCLEAN_RUST_ALL=0` | 5 个脚本全跑 |

`env_flag_from(all, module)` = 总开关 **与** 模块开关都为真；假值集合 `0 / false / off / no`（不区分大小写、允许空格），空串视为「恢复默认」。

**顺带做的一件事**：把五个环境变量名从散在 `rust_*_enabled()` 里的字面量抽成 `ENV_*` 常量。原来只有 `CCLEAN_RUST_ALL` 是常量，另外五个是字符串字面量——漏写一个的表现是**那个模块永远回退不了，且没有任何报错**。现在有 `every_script_has_an_own_switch` 断言「脚本名 ↔ 开关 ↔ 启用判断」三件配套。

### 6. R5 收官确认

**5 个模块全部 Rust 优先，ps1 仅作为回退保留。**

| # | 模块 | Rust 模块 | 迁移任务 |
|---|---|---|---|
| 1 | temp | `modules/temp.rs` | task-074 |
| 2 | windows_update | `modules/windows_update.rs` | **task-076** |
| 3 | prefetch | `modules/prefetch.rs` | task-075 |
| 4 | browser_cache | `modules/browser_cache.rs` | task-075 |
| 5 | dev_cache | `modules/dev_cache.rs` | **task-076** |

硬证据（`orchestrator_runs_five_rust_modules_without_any_ps1`）：

- 给一个**空的 scripts_dir**（一个 ps1 都没有）跑 `run_all` → `scripts_succeeded.len() == 5`、`scripts_failed` 为空。**默认路径完全不依赖脚本文件**；
- 同一个空目录 + `CCLEAN_RUST_ALL=0` → 必须 `AllScriptsFailed(5)`。**回退开关真的有效**；
- 入库行的 `module_code` 只能是这 5 个之一（空目录下不可能有 ps1 产出）；
- `s.total_items` == `SELECT count(*) FROM cleanup_item`（两条路径共用 `insert_batch` 的记账不漂）。

这条比「结果里出现了某模块的条目」硬：它证明的是**代码路径**，不受「本机恰好有缓存」影响。

### 7. `cargo test --workspace` 输出

```text
passed=553 failed=0 ignored=14
```

（task-075 收尾时 530 passed → **+23**）

| 位置 | 条数 | 用例 |
|---|---|---|
| `dev_cache.rs` 内联 | 10 | `scan_at_collects_all_four_roots` / `scan_at_tags_category_by_root` / `scan_at_tags_module_and_risk` / `scan_at_skips_missing_roots` / `scan_roots_match_ps1_paths` / `scan_roots_never_include_node_modules_or_gradle` / `scan_at_returns_empty_for_empty_bases` / `scan_does_not_panic_on_real_machine` / `dir_from_rejects_missing_and_blank` / `env_dir_missing_variable_is_none` |
| `windows_update.rs` 内联 | 5 | `scan_at_collects_files_recursively` / `scan_at_tags_module_category_and_risk` / `scan_at_returns_empty_when_missing` / `update_root_is_based_on_system_root` / `scan_does_not_panic_on_real_machine` |
| `orchestrator.rs` 内联 | +2 | 新增 `every_script_has_an_own_switch` / `module_switch_env_names_are_distinct_and_scoped`；`all_three_module_switches_default_on` → `all_five_module_switches_default_on`（原版是空转断言 `[...].len() == 3`，改为真实断言五个开关默认全开 + 环境已有开关时跳过） |
| `devcache_windowsupdate_test.rs` | 6 + 1 ignored | `dev_cache_rust_matches_ps1` / `dev_cache_rust_agrees_with_ps1_on_empty_bases` / `dev_cache_never_scans_gradle_or_maven` / `windows_update_rust_matches_ps1_on_real_machine` / `windows_update_rust_tags_l2_on_fabricated_tree` / `orchestrator_runs_five_rust_modules_without_any_ps1` / `real_scan_smoke`（ignored） |

**ps1 对比测试确认真的执行了**（不是静默跳过）：带 `--nocapture` 重跑两条对比用例，**无 `[跳过]` 日志**，耗时 3.00s（PowerShell 冷启动两次），`dev_cache_rust_matches_ps1` 断言 `ps1 产出 == 4 条` 且与 Rust 逐条相等。

### 8. `pnpm build` 输出

```text
dist/index.html                  0.34 kB │ gzip:  0.25 kB
dist/assets/index-I9UDeEER.js  243.01 kB │ gzip: 75.55 kB
✓ built in 2.70s
```

243.01 kB，与 task-073/074/075 **完全一致**——本任务无前端改动，这条只是回归保护。

### 9. 真机冒烟证据

```text
cargo test -p engine --test devcache_windowsupdate_test -- --ignored --nocapture real_scan

dev_cache: 共 21 项，合计 0.1 MB
  npm_cache: 0 项 / 0.0 MB
  pip_cache: 21 项 / 0.1 MB
  pnpm_store: 0 项 / 0.0 MB
windows_update: 9 项 / 29.6 MB（普通权限下通常为 0：目录 ACL 只给管理员）
test real_scan_smoke ... ok
```

| 验收点 | 结论 |
|---|---|
| 扫描结果包含 dev_cache 模块项（Rust） | ✅ `pip_cache` 21 项（本机 `npm_cache` / `pnpm_store` 为 0，是真实的——没装/已清过） |
| 扫描结果包含 windows_update 模块项（Rust） | ✅ 9 项 / 29.6 MB，类别 `update_download`，全部 L2 |
| 全部 5 个模块 Rust 优先（不再 spawn ps1） | ✅ 空 scripts_dir 下 5 个模块全成功（单测硬证据） |
| 环境变量回退 ps1 时结果一致 | ✅ 总开关关掉后同目录 `AllScriptsFailed(5)`；dev_cache 两套实现产出逐条相等 |
| CDP 抓 console 0 红错 | ⛔ **未做**（本机 WebView2 渲染进程创建即崩），归测试轮 |

**GUI 截图未产出**：本机 WebView2 渲染进程创建即崩溃（Crashpad 持续新增 `.dmp`、CDP 9222 不通），这是本项目长期已知的环境限制，开发轮按项目约定不做 GUI 取证。上面给的是**后端侧真实数据**，界面显示由测试轮验证。

### 10. 打包安装冒烟证据

```text
pnpm --filter app tauri build
   Finished `release` profile [optimized] target(s) in 1m 52s
   Built application at: D:\GitHub\C-clean\target\release\cdisk-cleaner.exe
   Finished 2 bundles at:
     ...\bundle\nsis\C-DiskCleaner_0.2.1_x64-setup.exe (4.50 MiB)
     ...\bundle\msi\C-DiskCleaner_0.2.1_x64_en-US.msi (6.65 MiB)
   Error A public key has been found, but no private key.
```

- **出包成功**：NSIS 4.50 MiB + MSI 6.65 MiB，与 task-075 收尾时**完全相同**——本任务只替换扫描实现、ps1 仍随包（回退需要），包体积本就不该变；提权 exe 297,472 字节正常进包。
- **末尾签名步失败属已知预期**：私钥正源在 CI secret（`TAURI_SIGNING_PRIVATE_KEY`），本地没有。这是项目自 v0.2.0 起每轮都出现的同一条，不是本次引入。
- **安装态复跑未做**：非管理员 shell 下 MSI per-user 静默安装未完成即被终止，归测试轮。

### 11. CI

**未跑。** 按项目流程（`MEMORY.md`「CI / 分支流程」）：开发分支不触发 CI，由测试环节拉 PR 后触发。本轮不建 PR、不 push 后等 CI。

### 12. 与本任务文档不符的偏差（汇总）

| # | 任务书说法 | 实际处理 |
|---|---|---|
| 1 | dev_cache 扫「npm / pip / Gradle / Maven」，还有 node_modules | **只扫 npm×2 / pip / pnpm**（ps1 的真实 `$roots`）。Gradle / Maven / node_modules 不加，理由见 §3——核心是「加了会让本任务自己的验收标准 14 条不成立」 |
| 2 | 单元 2 的提交只含 `windows_update.rs` | 同时含 `modules/mod.rs` 的一行注册（否则该 commit 编译不过）；`mod.rs` 因此拆两次提交 |
| 3 | 「输出统一结构 `CleanupItem { path, size_bytes, module, risk_level }`」 | 用项目既有 `ScanItem { module_code, category, path, size_bytes, risk_level }`——它是 `cleanup_item` 表的实际字段，`CleanupItem` 这个名字在本仓库不存在 |
| 4 | 「按文件大小 / 修改时间筛选」 | **不做**。ps1 没有任何筛选，加阈值会让两种模式条目数不同且阈值无依据（与 task-074/075 同一判断） |
| 5 | 真机冒烟「CDP 抓 console 0 红错」「打包安装冒烟必做」 | 开发轮**未做**，按项目流程（`MEMORY.md`：开发轮不做 GUI/真机 UI 测试）归测试轮。本轮给的是后端侧真实数据 + 出包证据 |
| 6 | 「CI（必做）：开 PR 触发 windows-latest 五道闸全绿」 | 开发轮**不开 PR**，按 `MEMORY.md`「开发阶段不触发 CI」 |

### 13. 遗留问题

- **R5 架构债本体已完成**（5 个模块全 Rust）。**但「删脚本」这一步没做**：`paths::scripts_dir`、`tauri.conf.json` 的 `bundle.resources` scripts 映射、`ScanRunner::run_script`、两个 ps1 编排测试都还在。任务书明确「ps1 不删除（保留作为回退）」，所以本任务不做。**建议先发一版收集反馈再删**——回退开关是唯一兜底手段，脚本一删就没了。
- **下一阶段**：P2 项（多盘扫描 / 自定义规则 / 便携版 CLI）或发版 `v0.4.0`。
- **Items / SafeBox / History 三页深色适配仍缺**（task-061 起的旧账，不在本任务范围）。
- **三个待决策项**：L0 是否进清理列表；Firefox 缓存是否保留（task-075 让条目数涨 653 倍）；dev_cache 是否扩 Gradle / Maven（要加需先定子目录 + 独立开关）。

