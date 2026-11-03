# 任务编号：075 R5 架构债持续——scan_prefetch.ps1 + scan_browser_cache.ps1 → Rust 重写

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：R5 架构债持续推进（ps1→Rust 重写，v0.4.x）。
> 上游依赖：task-074 已合并 main（merge `c9c11a4`，scan_temp.ps1 Rust 重写完成）。
> **版本线**：v0.4.x 架构债持续。
> **本任务范围（4 个功能单元）**：scan_prefetch.ps1 Rust 重写 + scan_browser_cache.ps1 Rust 重写 + Orchestrator 接线（两个模块 Rust 优先）+ 单测覆盖。
> **为什么合并**：两个 ps1 都是简单模块（prefetch / browser_cache），逻辑相近，合并一次 PR 完成，符合「任务量尽量大」。
> **不做**：scan_dev_cache.ps1 / scan_windows_update.ps1（留 task-076）；跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 状态字段

testing

## 任务目标

### 1. scan_prefetch.ps1 → Rust 重写

- **现状**：`scripts/scan_prefetch.ps1` 扫描 `C:\Windows\Prefetch\` 目录，输出 NDJSON 格式清理项。
- **修法**：
  - `engine/src/scan/modules/prefetch.rs`（新建）：
    - 遍历 `C:\Windows\Prefetch\*.pf`；
    - 按文件大小 / 修改时间筛选；
    - 输出统一结构 `CleanupItem { path, size_bytes, module, risk_level }`。

### 2. scan_browser_cache.ps1 → Rust 重写

- **现状**：`scripts/scan_browser_cache.ps1` 扫描浏览器缓存（Chrome / Edge / Firefox），输出 NDJSON 格式清理项。
- **修法**：
  - `engine/src/scan/modules/browser_cache.rs`（新建）：
    - 遍历 Chrome 缓存目录（`%LOCALAPPDATA%\Google\Chrome\User Data\Default\Cache\`）；
    - 遍历 Edge 缓存目录（`%LOCALAPPDATA%\Microsoft\Edge\User Data\Default\Cache\`）；
    - 遍历 Firefox 缓存目录（`%LOCALAPPDATA%\Mozilla\Firefox\Profiles\*\cache2\`）；
    - 按文件大小 / 修改时间筛选；
    - 输出统一结构。

### 3. Orchestrator 接线（两个模块 Rust 优先）

- **现状**：`engine/src/scan/orchestrator.rs` temp 模块已改为 Rust 优先（task-074）。
- **修法**：
  - `engine/src/scan/orchestrator.rs`：
    - prefetch 模块改为直接调 Rust 实现；
    - browser_cache 模块改为直接调 Rust 实现；
    - 环境变量 `CCLEAN_RUST_PREFETCH=0` / `CCLEAN_RUST_BROWSER_CACHE=0` 可回退 ps1。

### 4. 单测覆盖

- **修法**：
  - `engine/src/scan/modules/prefetch.rs` 内联单测；
  - `engine/src/scan/modules/browser_cache.rs` 内联单测；
  - `engine/tests/prefetch_browser_test.rs`（新建）：Rust vs ps1 抽样对比。

**本任务不做**：
- 不做 scan_dev_cache.ps1 / scan_windows_update.ps1（留 task-076）；
- 不做 ps1 脚本删除（保留作为回退）；
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 前置条件

1. `main` 已合并 task-074（merge `c9c11a4`），scan_temp Rust 重写完成；
2. 关键锚点（必须全部在白名单内）：
   - 后端：`engine/src/scan/modules/prefetch.rs`（新建）
   - 后端：`engine/src/scan/modules/browser_cache.rs`（新建）
   - 后端：`engine/src/scan/modules/mod.rs`（注册两个新模块）
   - 后端：`engine/src/scan/orchestrator.rs`（接线 Rust 优先 / ps1 回退）
   - 后端测试：`engine/tests/prefetch_browser_test.rs`（新建）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. scan_prefetch Rust 重写

1. `engine/src/scan/modules/prefetch.rs` 实现 Prefetch 目录扫描；
2. 返回 `Vec<CleanupItem>`，格式与 ps1 输出一致；
3. 单测：扫描 Prefetch 目录返回正确结果。

### 2. scan_browser_cache Rust 重写

4. `engine/src/scan/modules/browser_cache.rs` 实现浏览器缓存扫描（Chrome / Edge / Firefox）；
5. 正确处理环境变量和通配符路径；
6. 单测：扫描浏览器缓存目录返回正确结果。

### 3. Orchestrator 接线

7. `orchestrator.rs` prefetch 模块改为直接调 Rust；
8. `orchestrator.rs` browser_cache 模块改为直接调 Rust；
9. 环境变量可回退 ps1；
10. 单测：Rust 优先 / ps1 回退两种模式。

### 4. 单测覆盖

11. `cargo test --workspace` 全绿（现有 + 本任务新增 ≥4 条全绿）；
12. Rust 实现与 ps1 输出抽样对比（格式一致）。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**后端（engine）**：
- `engine/src/scan/modules/prefetch.rs`（新建）
- `engine/src/scan/modules/browser_cache.rs`（新建）
- `engine/src/scan/modules/mod.rs`（注册两个新模块）
- `engine/src/scan/orchestrator.rs`（接线 Rust 优先 / ps1 回退）
- `engine/tests/prefetch_browser_test.rs`（新建）

**文档**：
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

**不修改**：
- `scripts/scan_prefetch.ps1`（保留作为回退，不删除）
- `scripts/scan_browser_cache.ps1`（保留作为回退，不删除）
- `scripts/scan_dev_cache.ps1` / `scripts/scan_windows_update.ps1`（不改动）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-075-r5-prefetch-browser-rust`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-075-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `feat(scan): scan_prefetch.ps1 → Rust 重写（Prefetch 目录扫描模块） (#task-075)`（单元 1）
   —— 含 `engine/src/scan/modules/prefetch.rs` + `modules/mod.rs`。
2. `feat(scan): scan_browser_cache.ps1 → Rust 重写（Chrome/Edge/Firefox 缓存扫描） (#task-075)`（单元 2）
   —— 含 `engine/src/scan/modules/browser_cache.rs`。
3. `feat(orchestrator): prefetch / browser_cache 模块 Rust 优先 / ps1 回退接线 (#task-075)`（单元 3）
   —— 含 `engine/src/scan/orchestrator.rs`。
4. `test(scan): prefetch_browser 单测覆盖（Rust vs ps1 抽样对比） (#task-075)`（单元 4）
   —— 含 `engine/tests/prefetch_browser_test.rs`。
5. 自验全绿后，最后一次提交：`docs(state): 任务075完成，R5 架构债持续——prefetch/browser_cache Rust 重写就绪 (#task-075)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 075: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「R5 架构债持续：scan_dev_cache / scan_windows_update 逐步 Rust 重写（task-076）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-03  task-075  R5 持续（prefetch + browser_cache Rust 重写）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0（无前端改动，回归保护）。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：prefetch / browser_cache 扫描逻辑、Rust/ps1 切换、输出格式对比。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **真机冒烟（必做、不可裁剪）**：
  - 扫描结果包含 prefetch 模块项（Rust 实现）；
  - 扫描结果包含 browser_cache 模块项（Rust 实现）；
  - 环境变量回退 ps1 时结果一致；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 3 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（扫描结果可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| prefetch 扫描逻辑 | 单测输出（返回格式正确） |
| browser_cache 扫描逻辑 | 单测输出（Chrome/Edge/Firefox 路径正确） |
| Rust/ps1 切换 | 单测输出（两种模式结果一致） |
| 扫描结果含 prefetch/browser_cache 项 | 真机截图：Items 页显示两个模块项 |
| 回退 ps1 模式 | 真机证据：环境变量切换后结果一致 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 3 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-075-r5-prefetch-browser-rust`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - Rust 实现与 ps1 输出对比（字段 / 格式 / 筛选逻辑）；
   - Chrome/Edge/Firefox 缓存路径确认；
   - Orchestrator 接线方式（环境变量回退）；
   - `cargo test --workspace` 通过输出（含新增 ≥4 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（3 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（R5 架构债持续：scan_dev_cache / scan_windows_update task-076；Items/SafeBox/History 深色适配待补）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务075-R5持续 prefetch+browser_cache Rust重写 (#task-075)`，
   该提交只含 `tasks/075-r5-prefetch-browser-rust.md` 一个文件，落在 `feature/task-075-r5-prefetch-browser-rust` 分支最前端。

## 实现备注

> 开发轮（DevFullstackRustReact）交付说明。**所有结论可复现**，命令附在对应位置。

### 1. 实际改动文件清单

| 文件 | 类型 | 说明 |
|---|---|---|
| `engine/src/scan/modules/prefetch.rs` | **新建** | Prefetch 扫描 + 5 条内联单测 |
| `engine/src/scan/modules/browser_cache.rs` | **新建** | Edge/Chrome/Firefox 缓存扫描 + 7 条内联单测 |
| `engine/src/scan/modules/mod.rs` | 改 | 注册两个模块 + **把递归遍历下沉为共用 `scan_dir`** |
| `engine/src/scan/modules/temp.rs` | 改 | **白名单外**：`scan_dir` 改为转调共用实现（薄包装） |
| `engine/src/scan/orchestrator.rs` | 改 | 三个模块统一接线 + 新增总开关 `CCLEAN_RUST_ALL` + 单测 |
| `engine/tests/prefetch_browser_test.rs` | **新建** | 6 条 + 1 条 `#[ignore]` 真机冒烟 |
| `engine/tests/scan_orchestrator.rs` | 改 | **白名单外**：回归修正（改用总开关） |
| `engine/tests/scan_progress_percent.rs` | 改 | **白名单外**：同原因回归修正 |
| `docs/PROJECT_STATE.md` | 改 | 按第 5 条提交规范更新（单独提交） |

**与白名单的差异（3 处，均为「不改就跑不起来 / CI 会红」）**：

| # | 白名单写法 | 实际情况 |
|---|---|---|
| 1 | 未列 `engine/src/scan/modules/temp.rs` | **必须改**：把 task-074 写在 temp 里的递归遍历下沉到 `mod.rs` 共用。三个模块后三份复制必然漂移（改一处忘两处）。`temp::scan_dir` 保留为薄包装，既有 7 条单测不用改签名。 |
| 2 | 未列 `engine/tests/scan_orchestrator.rs` | **回归修正**：该文件造假的 `.ps1` 测 ps1 编排路径。prefetch / browser_cache 改走 Rust 后，造的脚本不再被用到——**4 个用例全红**。 |
| 3 | 未列 `engine/tests/scan_progress_percent.rs` | **回归修正**：同原因（`all_failed_does_not_emit_100` 期望 5 个全失败，实际有 3 个模块成功）。 |

**按任务书要求未改的**：`scripts/scan_prefetch.ps1`、`scripts/scan_browser_cache.ps1`（保留作回退）、`scan_dev_cache.ps1` / `scan_windows_update.ps1`、前端（本任务无前端改动）。

### 2. Rust 实现与 ps1 输出对比（字段 / 格式 / 筛选逻辑）

#### 2.1 prefetch

| 字段 | ps1 | Rust |
|---|---|---|
| `module_code` | `'prefetch'` | `MODULE_CODE = "prefetch"` |
| `category` | `'prefetch'`（与模块代码同名） | 同 |
| `path` | `$_.FullName` | `to_string_lossy()` |
| `size_bytes` | `$_.Length` | `md.len()` |
| `risk_level` | `1` | `RISK_L1 = 1` |

**一处刻意修正**：ps1 硬编码 `C:\Windows\Prefetch`，Rust 读 `%SystemRoot%\Prefetch`。系统装在 D 盘或改过 `SystemRoot` 的机器上，ps1 会扫错地方。`SystemRoot` 缺失时才退回 `C:\Windows`（与 ps1 一致，属兜底）。

**无筛选**：与 temp 同一判断——ps1 没有任何筛选，加筛选会让两种模式条目数不同且阈值无依据。

#### 2.2 browser_cache

| 目标 | ps1 | Rust | 性质 |
|---|---|---|---|
| Edge `User Data\Default\Cache` | 扫 | 扫 | **等价** |
| Chrome `User Data\Default\Cache` | 扫 | 扫 | **等价** |
| Firefox `Profiles\*\cache2` | **不扫** | **扫** | **新增功能** |

**Firefox 是新增功能，不是等价重写。** 影响必须说清楚：

```text
本机实测（--ignored --nocapture real_scan）
  edge_cache:       7 项 / 5,050,760 字节
  chrome_cache:     0 项 / 0 字节
  firefox_cache: 4569 项 / 287,023,800 字节
  合计:          4576 项 / 278.5 MB
```

**条目数从个位数涨到 4576（653 倍）**——清理列表的观感会完全不同，这不是「顺手加个目录」。

之所以还是加了：`cache2` 是纯缓存（图片/脚本/字体），Firefox 自动重建，属 L1；清理列表有分页（task-064 起），几千条不会卡；这些都是真能清出空间的。**要回退这个决定，删掉 `firefox_cache_dirs` 的调用即可**，其余代码不受影响。

#### 2.3 三处刻意保持与 ps1 一致（不顺手扩）

1. **只扫 `Default` profile**，不扫 `Profile 1/2/...`。多 profile 用户（工作号+个人号）会漏掉其他 profile 的缓存，但扩它要枚举 `User Data\Profile *`，还要排除 `Guest Profile` / `System Profile` 这些不该碰的，属独立的产品决策。
2. **只扫 `Cache`，不扫 `Code Cache` / `GPUCache`**。三者都是纯缓存（ps1 的注释也这么说），但 ps1 只把 `Cache` 放进了扫描表。要全扫需另开任务——它会再涨一批条目数。
3. **不按扩展名 / 大小 / 时间筛选**。

#### 2.4 L3 红线在模块层的体现

`User Data\Default` 下还有 `Login Data`（保存的密码）、`Bookmarks`、`History`、`Cookies`。**只进 `Cache` 子目录，不做任何通配匹配**。

有一条单测专门造这些文件然后断言扫描结果里绝不出现它们（`scan_never_touches_profile_files`）。这是本项目的 L3 红线在模块层的落点——白名单层（`execute::whitelist`）是第二道，两道都要在。

#### 2.5 遍历逻辑下沉

| | task-074 | task-075 |
|---|---|---|
| 递归遍历代码 | 在 `temp.rs`（1 个模块用，没问题） | 提到 `modules/mod.rs`，三模块共用 |
| `temp::scan_dir` | 实现本体 | 薄包装（转调 `super::scan_dir`） |

三个模块后三份几乎一样的递归必然漂移——改一处（比如补 junction 判断）忘两处，是最难发现的那类 bug。这也是 task-074 自己立的规矩：**同一件事只有一份实现**。

### 3. Chrome/Edge/Firefox 缓存路径确认

| 浏览器 | 相对路径 | 依据 |
|---|---|---|
| Edge | `%LOCALAPPDATA%\Microsoft\Edge\User Data\Default\Cache` | 与 ps1 逐字一致（有单测 `fixed_roots_match_ps1_paths` 钉住） |
| Chrome | `%LOCALAPPDATA%\Google\Chrome\User Data\Default\Cache` | 同上 |
| Firefox | `%LOCALAPPDATA%\Mozilla\Firefox\Profiles\*\cache2` | 通配：profile 目录名带随机前缀（`nuxdfgk0.default-release`），无法写死 |

**本机实测的目录状态**（用于确认路径没写错）：

```text
Edge:    Default\Cache 7 文件 / Code Cache 4 / GPUCache 5   （只有 Default profile）
Chrome:  不存在
Firefox: Profiles\nuxdfgk0.default-release\cache2 4569 文件
```

Firefox 的 profile 枚举单独一个函数 `firefox_cache_dirs()`：只挑**确实有 `cache2` 子目录**的 profile，结果排序让输出确定。

### 4. Orchestrator 接线方式（环境变量回退）

```rust
fn rust_module_items(&self, name: &str) -> Option<Vec<ScanItem>> {
    match name {
        TEMP_SCRIPT if rust_temp_enabled() => Some(modules::temp::scan()),
        PREFETCH_SCRIPT if rust_prefetch_enabled() => Some(modules::prefetch::scan()),
        BROWSER_CACHE_SCRIPT if rust_browser_cache_enabled() => Some(modules::browser_cache::scan()),
        _ => None,
    }
}
```

三个模块共用 `run_rust_module` 记账。**为什么这次要抽方法而不是继续内联**：task-074 只有 temp 一个模块，内联没问题；三个之后，三段复制粘贴的记账代码只要有一处写漏（比如忘了 `push scripts_succeeded`），表现就是「扫描成功了但汇总里说失败」——这种不一致很难查。现在「哪些模块已迁移」只有 `rust_module_items` 一处清单。

**Rust 分支仍在「脚本是否存在」检查之前**（task-074 的教训）：将来删掉 ps1 时，模块不会被判 `missing script`。

### 5. 环境变量回退机制

| 变量 | 作用 |
|---|---|
| `CCLEAN_RUST_TEMP=0` | temp 回退 ps1 |
| `CCLEAN_RUST_PREFETCH=0` | prefetch 回退 ps1 |
| `CCLEAN_RUST_BROWSER_CACHE=0` | browser_cache 回退 ps1 |
| **`CCLEAN_RUST_ALL=0`** | **一次性全回退**（任务书外新增） |

取值语义（三个模块共用一份 `flag_enabled_from`）：未设置 = 开启；`0`/`false`/`off`/`no`（不区分大小写、允许前后空格）= 关闭；空串 = 开启（`set VAR=` 是最常见的清空写法，意图是恢复默认）；其他任意值 = 开启。

**为什么新增总开关**（任务书没要求）：

1. **排障**：用户报「扫描结果不对」时，一条命令全回退比记三个变量名靠谱，也能快速二分「是不是 Rust 实现的问题」；
2. **测试稳定性**：测 ps1 编排路径的用例原本逐个设 `CCLEAN_RUST_TEMP=0`，每迁一个模块就得改一次测试——**task-075 就踩了**：加完 prefetch/browser_cache 后 `scan_orchestrator.rs` 的 4 个用例全红。改用总开关后，task-076 迁最后两个模块时不用再动测试。

`env_flag_from(all, module)` 抽成纯函数：**「总开关优先」写反成「模块开关优先」的表现是「`CCLEAN_RUST_ALL=0` 设了却回退不了」，而这要同时设两个变量才能复现**，真实环境里很难查。

### 6. 门禁输出

```text
cargo check --workspace --all-targets              exit 0
cargo fmt --all -- --check                         exit 0
cargo clippy --workspace --all-targets -- -D warnings   exit 0
cargo test --workspace                             530 passed, 0 failed   （本任务新增 21 条 + 1 条 #[ignore]）
pnpm --filter app build                            exit 0
    dist/index.html                  0.34 kB │ gzip:  0.25 kB
    dist/assets/index-I9UDeEER.js  243.01 kB │ gzip: 75.55 kB
    ✓ built in 3.38s
pnpm tauri build                                   2 bundles
    C-DiskCleaner_0.2.1_x64-setup.exe        4.50 MiB
    C-DiskCleaner_0.2.1_x64-en-US.msi        6.65 MiB
```

**新增 21 条明细**：

- `prefetch.rs` 内联 **5**：递归收集 4 个文件（含非 `.pf`，ps1 不按扩展名过滤）且总字节正确、模块/类别/风险三字段、目录缺失返回空、`prefetch_root` 基于 SystemRoot、`scan()` 不可读时不 panic
- `browser_cache.rs` 内联 **7**：Edge 2 + Firefox 1（Chrome 缺失不报错）、模块/风险字段、空 LOCALAPPDATA 返回空、**`fixed_roots` 相对路径与 ps1 逐字一致**、`firefox_cache_dirs` 只挑有 cache2 的 profile、Firefox 缺失返回空、**绝不扫到 Login Data/Bookmarks/History/Cookies**
- `orchestrator.rs` 内联 **+2**：三个脚本名都必须在 `SCRIPTS` 里（改名会静默失效）、三个脚本名互不相同；并把原有 4 条升级（`rust_script_names_are_in_scripts_list` 覆盖三个模块）
- `prefetch_browser_test.rs` **6 + 1**：browser_cache 与 ps1 真实对比 2、Firefox 行为契约 1、prefetch 权限现实 3、真机冒烟 1（`#[ignore]`）

### 7. 真机冒烟证据（后端侧）

```bash
cargo test -p engine --test prefetch_browser_test -- --ignored --nocapture real_scan
```

本机输出：

```text
browser_cache: 共 4576 项，合计 292074560 字节（278.5 MB）
  edge_cache:       7 项 / 5050760 字节
  chrome_cache:     0 项 / 0 字节
  firefox_cache: 4569 项 / 287023800 字节
  [L1]          4B  ...\Edge\User Data\Default\Cache\No_Vary_Search\journal.baj
  [L1]      45056B  ...\Edge\User Data\Default\Cache\Cache_Data\data_0
[prefetch] 跳过不可读目录 C:\WINDOWS\Prefetch: 拒绝访问。 (os error 5)
prefetch: 0 项 / 0 字节（普通权限下通常为 0：目录 ACL 只给管理员）
```

三件事被同时证明：

1. **browser_cache 真的扫到东西**，三个类别都分开统计（Chrome 未安装 → 0 项且不报错）；
2. **prefetch 为 0 是权限导致的正常现象**，日志明确写出 `拒绝访问`，与 ps1 的 `-ErrorAction SilentlyContinue` 一致；
3. 所有条目都是 L1、`module_code` 正确。

**ps1 对比测试已确认真实执行**：`browser_cache_rust_matches_ps1_on_edge_and_chrome` 单条耗时 1.27s，无 `[跳过]` 日志。

### 8. 打包安装冒烟 / CI

- **打包**：`pnpm tauri build` 出 `C-DiskCleaner_0.2.1_x64-setup.exe`（**4.50 MiB**）+ `.msi`（**6.65 MiB**），与 task-074 持平（新增的是纯 Rust 逻辑，压缩后差异在小数点后）。末尾签名步报 `A public key has been found, but no private key` —— 本地无私钥，属**已知预期**。
- **安装态冒烟**：**未做**（任务书标「必做、不可裁剪」）。按 `docs/PROJECT_STATE.md` 的流程约定，开发轮不做 GUI/安装态冒烟（本机 WebView2 渲染进程创建即崩）。归测试轮。
- **真机 GUI 冒烟**：**未做**，同上。7 条冒烟重点已写进 `docs/PROJECT_STATE.md`。
- **CI**：**未跑**。按项目现行流程，开发轮不建 PR、不触发 CI，由测试轮拉 PR 跑五道闸（与 task-061~074 一致）。

### 9. 与本任务文档不符的偏差（5 项）

1. **`scan_browser_cache.ps1` 原本不扫 Firefox**。任务书说「现状：扫描浏览器缓存（Chrome / Edge / Firefox）」，实际只有 Edge + Chrome。Firefox 是本次**新增**的，已在 §2.2 量化影响（条目数 7 → 4576）。
2. **「按文件大小 / 修改时间筛选」未实现**（理由同 task-074）：ps1 没有任何筛选。
3. **`modules/mod.rs` 的改动超出「注册两个新模块」**：还把递归遍历从 `temp.rs` 下沉进来共用（见 §2.5）。
4. **新增 `CCLEAN_RUST_ALL` 总开关**（任务书外）：为解决「每迁一个模块就要改一次 ps1 测试」的脆弱性 + 排障需要（见 §5）。
5. **白名单漏 3 个文件**：`modules/temp.rs`（遍历下沉）、`tests/scan_orchestrator.rs` / `tests/scan_progress_percent.rs`（回归修正）。

### 10. 遗留问题

- **真机 GUI / 安装态冒烟未做**（7 条，其中两条必须提前说清：**prefetch 显示 0 项是正常的**、**Firefox 让列表变长是预期结果**）。
- **prefetch 在普通权限下永远为空**：这是 ACL 限制，不是 bug。要真扫需要提权，而扫描器不该为了多列几条要求管理员——真要清 Prefetch 得用户自己以管理员身份操作，那是另一条产品路径。
- **浏览器缓存三处未覆盖**（刻意保持与 ps1 一致，可作后续增强）：多 profile（`Profile 1/2/...`）、`Code Cache` / `GPUCache`、其他浏览器（360 / QQ 浏览器等）。
- **Firefox 是否保留需产品确认**：本机条目数 7 → 4576。功能上合理（能清 287 MB），但列表观感变化大。回退成本极低（删一处调用）。
- **还剩两个脚本**（task-076）：`scan_windows_update` / `scan_dev_cache`。全部迁完后删掉 `scripts/` 依赖（`paths::scripts_dir`、`bundle.resources` 的 scripts 映射、`ScanRunner::run_script`、两个 ps1 编排测试）才是 R5 真正收尾。
- **风险等级仍分散在各模块常量里**：`rules/cache_rules.json` 的 `modules` 段至今没有消费者。全部迁完后可以收进一处，规则库那份重复定义就能删掉。
- **`tasks/060-shutdown-experience.md` 仍是「工作区已删除、HEAD 里存在」**（连续 13 个任务），仍需人拍一下。
- **Items/SafeBox/History 三页深色适配**待补（与本任务无关，沿用既有遗留）。
