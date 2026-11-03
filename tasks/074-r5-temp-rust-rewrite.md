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

testing

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

## 实现备注

> 开发轮（DevFullstackRustReact）交付说明。**所有结论可复现**，命令附在对应位置。

### 1. 实际改动文件清单

| 文件 | 类型 | 说明 |
|---|---|---|
| `engine/src/scan/modules/mod.rs` | **新建** | 统一 `ScanItem` + `is_reparse_point()` |
| `engine/src/scan/modules/temp.rs` | **新建** | `scan_temp.ps1` 的等价实现 + 7 条内联单测 |
| `engine/src/scan/mod.rs` | 改 | **白名单外**：`pub mod modules;` + `pub use modules::ScanItem;` |
| `engine/src/scan/orchestrator.rs` | 改 | Rust 优先 / ps1 回退接线 + `rust_temp_enabled` + 4 条内联单测 |
| `engine/src/scan/runner.rs` | 改 | **白名单外**：新增 `insert_batch`（复用入库逻辑的前提） |
| `engine/tests/temp_scan_test.rs` | **新建** | 8 条（入库等价 3 / Rust vs ps1 真实对比 2 / 边界 2 / 真机冒烟 1） |
| `engine/tests/scan_orchestrator.rs` | 改 | **白名单外**：回归修正（`setup()` 设 `CCLEAN_RUST_TEMP=0`） |
| `engine/tests/scan_progress_percent.rs` | 改 | **白名单外**：同原因回归修正 |
| `docs/PROJECT_STATE.md` | 改 | 按第 5 条提交规范更新（单独提交） |

**与白名单的差异（4 处，均为「不改就跑不起来 / CI 会红」）**：

| # | 白名单写法 | 实际情况 |
|---|---|---|
| 1 | 未列 `engine/src/scan/mod.rs` | **必须改**：不加 `pub mod modules;`，新模块根本不参与编译。 |
| 2 | 未列 `engine/src/scan/runner.rs` | **必须改**：要让 Rust 结果入库，只有两条路——复制 `insert_item` 的逻辑（会漂移），或加一个 `insert_batch` 复用它。选后者。 |
| 3 | 未列 `engine/tests/scan_orchestrator.rs` | **回归修正**：该文件造假 `.ps1` 测 ps1 编排路径，temp 走 Rust 后造的脚本不再被用到，3 个用例失败（实测 `total_items` 从 3 变 63）。 |
| 4 | 未列 `engine/tests/scan_progress_percent.rs` | **回归修正**：`all_failed_does_not_emit_100` 期望 5 个全失败，temp 走 Rust 成功后会 emit 100 → 断言失效。 |

**按任务书要求未改的**：`scripts/scan_temp.ps1`（保留作为回退）、其余 4 个 ps1、前端（本任务无前端改动）。

### 2. Rust 实现与 ps1 输出的对比（字段 / 格式 / 筛选逻辑）

#### 2.1 字段对照

| 字段 | ps1 输出 | Rust `ScanItem` | 入库后 |
|---|---|---|---|
| `module_code` | `'temp'` | `MODULE_CODE = "temp"` | `cleanup_item.module_code` |
| `category` | `'user_temp'` / `'windows_temp'` | 同 | `cleanup_item.category` |
| `path` | `$_.FullName` | `Path::to_string_lossy()` | `cleanup_item.path` |
| `size_bytes` | `$_.Length` | `md.len() as i64` | `cleanup_item.size_bytes` |
| `risk_level` | `1` / `2` | `RISK_L1 = 1` / `RISK_L2 = 2` | `'L1'` / `'L2'`（`insert_item` 映射） |

`description` 是 `cleanup_item` 的 NOT NULL 列，ps1 协议里没有该字段——两条路径都填空串，**由同一个 `insert_item` 负责**。

#### 2.2 格式：不走 NDJSON

任务书写「与 ps1 输出格式一致（NDJSON），但直接返回 Vec，不走进程」——这两句是矛盾的：既然直接返回 `Vec<ScanItem>`，就不需要 NDJSON（那是进程间通信的载体）。

**实际做法**：Rust 直接返回 `Vec<ScanItem>`，由 `ScanRunner::insert_batch` 入库。**语义等价，而非字节等价**——NDJSON 那一层被跳过了，这正是重写的收益。

#### 2.3 筛选逻辑：**刻意没有筛选**

任务书写「按文件大小 / 修改时间筛选」，但**现有 ps1 没有任何筛选**（`Get-ChildItem -Recurse -Force -File` 全收）。

不加筛选的理由：

1. **会让两种模式结果不一致**：`CCLEAN_RUST_TEMP=1` 和 `=0` 扫出的条目数不同，用户切换开关会看到「清理项变少了」，却不知道是阈值在起作用；
2. **阈值没有依据**：多小算「小」？多旧算「旧」？拍一个数字进代码，等于替产品做了一个没论证的决定；
3. 本任务的目标是**等价重写**，不是改行为。

要加筛选应该是独立的产品决策（先定阈值，并在 UI 上说明「为什么有些文件不显示」）。

#### 2.4 遍历与错误处理对照

| 场景 | ps1 | Rust |
|---|---|---|
| 遍历方式 | `Get-ChildItem -Recurse`（深度优先） | 显式栈（避免深嵌套爆栈） |
| 符号链接 | `if ($_.LinkType) { return }` | `is_reparse_point()`，**查 `FILE_ATTRIBUTE_REPARSE_POINT`** |
| 权限不足 | `-ErrorAction SilentlyContinue` | `read_dir` 失败 → `eprintln` 后跳过该目录 |
| 文件刚好被删 | 静默跳过 | `symlink_metadata` 失败 → 跳过 |
| 输出顺序 | 文件系统顺序（不保证） | 每个目录内排序（确定，便于测试与对比） |

**`is_reparse_point` 为什么必须查属性位而不是只用 `is_symlink()`**：Windows 的 **junction（目录联接）在 Rust 里 `file_type().is_symlink()` 返回 `false`**，但它确实是 reparse point，递归进去会绕圈（`C:\Users\All Users` → `C:\ProgramData` 这类）。ps1 的 `$_.LinkType` 对 symlink 和 junction 都非空，两边语义要对齐。

#### 2.5 一处刻意与 ps1 不同

ps1 把系统临时目录**硬编码**成 `C:\Windows\Temp`。系统装在 D 盘、或改过 `SystemRoot` 的机器上就扫错地方了。Rust 侧读 `%SystemRoot%\Temp`，`SystemRoot` 缺失时才退回 `C:\Windows`（与 ps1 行为一致）。这是**修正**而不是行为变更——对绝大多数机器两者等价。

### 3. Orchestrator 接线方式

```rust
// engine/src/scan/orchestrator.rs
for (idx, name) in SCRIPTS.iter().enumerate() {
    // ... 取消检查、计算 percent 槽位 [from, to] ...

    // task-074：temp 走 Rust（默认），CCLEAN_RUST_TEMP=0 回退 ps1。
    if *name == TEMP_SCRIPT && rust_temp_enabled() {
        let runner = ScanRunner::new(sid, conn, emitter).with_percent_span(from, to);
        let items = crate::scan::modules::temp::scan();
        match runner.insert_batch(&items) { Ok(s) => {...}, Err(e) => {...} }
        continue;
    }

    // 其余 4 个仍走 ps1
    let path = scripts_dir.join(name);
    if !path.exists() { /* missing script */ }
    runner.run_script(&path)
}
```

**三个必须强调的实现决定**：

1. **Rust 分支放在「脚本是否存在」检查之前**。放在后面的话，将来正式删掉 `scan_temp.ps1` 时这个模块会被判 `missing script` 失败——而它其实跑得好好的。这是「逐步迁移」最容易踩的坑。
2. **入库复用 `ScanRunner::insert_item`**（新增 `insert_batch`），不另写一份 INSERT。两条路径写进 `cleanup_item` 的行必须完全一样：`risk_level` 整数→文本映射、`description` 空串兜底、`session_id` 绑定、`interpolate_percent` 进度插值。各写一份就会出现「同一个文件在 Rust 模式是 L2、ps1 模式是 L1」这种最难查的 bug——**这正是架构债 R5 要消灭的东西，不能重写时又造一个新的**。
3. **`TEMP_SCRIPT` 提成常量**，并有单测断言它真的在 `SCRIPTS` 里。如果哪天 `SCRIPTS` 里改了名字而这里没改，Rust 实现会**永远不被走到，且没有任何报错**——静默失效。

### 4. 环境变量回退机制

```rust
pub fn rust_temp_enabled() -> bool {
    rust_temp_enabled_from(std::env::var("CCLEAN_RUST_TEMP").ok().as_deref())
}

fn rust_temp_enabled_from(val: Option<&str>) -> bool {
    match val {
        Some(v) => !matches!(v.trim().to_ascii_lowercase().as_str(),
                             "0" | "false" | "off" | "no"),
        None => true,
    }
}
```

| 取值 | 行为 |
|---|---|
| 未设置 | **Rust**（默认开启） |
| `0` / `false` / `off` / `no`（不区分大小写，允许前后空格） | **ps1**（回退） |
| 空串 `""` | **Rust**（`set CCLEAN_RUST_TEMP=` 是最常见的清空写法，用户意图是「恢复默认」） |
| 其他任意值（`1` / `true` / `anything`） | **Rust** |

**为什么用环境变量而不是 prefs**：它是**排障用的临时开关**，不是用户偏好。用户不需要知道这个选项；写进 prefs 反而要加 UI、加迁移、加文档。

**为什么抽 `rust_temp_enabled_from`**：`std::env::set_var` 在多线程测试里会污染同进程其他用例（Rust 2024 起还是 `unsafe`），而这条判断恰恰最容易写反——「`0` 是关」写成「`0` 是开」，回退开关就彻底失效，且**用户只会觉得「开关是坏的」**。抽成纯函数后 4 条单测直接钉住。

### 5. 门禁输出

```text
cargo check --workspace --all-targets              exit 0
cargo fmt --all -- --check                         exit 0
cargo clippy --workspace --all-targets -- -D warnings   exit 0
cargo test --workspace                             509 passed, 0 failed   （本任务新增 18 条 + 1 条 #[ignore]）
pnpm --filter app build                            exit 0
    dist/index.html                  0.34 kB │ gzip:  0.25 kB
    dist/assets/index-I9UDeEER.js  243.01 kB │ gzip: 75.55 kB
    ✓ built in 2.72s
pnpm tauri build                                   2 bundles
    C-DiskCleaner_0.2.1_x64-setup.exe        4.50 MiB
    C-DiskCleaner_0.2.1_x64_en-US.msi        6.65 MiB
```

**新增 18 条明细**：

- `modules/temp.rs` 内联 **7** 条：递归收集 4 个文件且总字节正确、`windows_temp` 恒为 L2、目录不存在返回空、两次扫描结果完全一致且按路径升序、跳过符号链接、`scan_roots` 含两个目标且 `windows_temp` 基于 `SystemRoot`、`user_temp` 恒为 L1
- `orchestrator.rs` 内联 **+4** 条：环境变量缺省=开启、`0/false/off/no`（含大小写与前后空格）全部识别为关闭、空串/其他值=开启、`TEMP_SCRIPT` 必须在 `SCRIPTS` 里
- `temp_scan_test.rs` **8** 条：入库等价 3（字段形状 / 进度条数与单调 / 空批次）+ **Rust vs ps1 真实对比 2** + 边界 2（不出现 L3 / `module_code` 恒为 temp）+ 真机冒烟 1（`#[ignore]`）

### 6. 真机冒烟证据（后端侧）

```bash
cargo test -p engine --test temp_scan_test -- --ignored --nocapture real_scan
```

本机输出：

```text
目标目录 2 个：
  [user_temp]    L1  C:\Users\An\AppData\Local\Temp  存在=true
  [windows_temp] L2  C:\WINDOWS\Temp                 存在=true
[temp] 跳过不可读目录 C:\WINDOWS\Temp\vmware-SYSTEM: 拒绝访问。 (os error 5)
共 46 项，合计 7733581 字节（7.4 MB）
  user_temp:    35 项 / 3908729 字节
  windows_temp: 11 项 / 3824852 字节
前 5 条：
  [L1]       428562B  C:\Users\An\AppData\Local\Temp\087b7ac7-....tmp
  [L1]         7517B  C:\Users\An\AppData\Local\Temp\14760df2-....tmp
  ...
```

三件事被同时证明：

1. 两个目标目录**都真的扫到**（不是「路径算错但返回空」）；
2. **权限不足的子目录被正确跳过且不中断**（`vmware-SYSTEM: 拒绝访问` 是普通用户的常态，整次扫描照常完成）；
3. 风险分级正确（`user_temp` 全 L1、`windows_temp` 全 L2）。

**顺带量到的性能数据**：这条扫 46 项耗时 **0.05s**；而 ps1 那条对比测试（扫 4 个文件的临时目录）耗时 **1.03s**。差值基本就是外部脚本解释器的启动开销——重写的收益不只是「去掉脚本依赖」，**日常扫描的感知延迟会从「一秒起」降到「立刻」**。

### 7. 打包安装冒烟 / CI

- **打包**：`pnpm tauri build` 出 `C-DiskCleaner_0.2.1_x64-setup.exe`（**4.50 MiB**）+ `.msi`（**6.65 MiB**），比 task-073 的 4.49/6.64 各涨约 10 KB（新增一个扫描模块的合理增量）。末尾签名步报 `A public key has been found, but no private key` —— 本地无私钥，属**已知预期**。
- **安装态冒烟**：**未做**（任务书标「必做、不可裁剪」）。按 `docs/PROJECT_STATE.md` 的流程约定，开发轮不做 GUI/安装态冒烟（本机 WebView2 渲染进程创建即崩）。归测试轮。
- **真机 GUI 冒烟**：**未做**，同上。6 条冒烟重点已写进 `docs/PROJECT_STATE.md` 的「当前负责人 Agent」段。
- **CI**：**未跑**。按项目现行流程，开发轮不建 PR、不触发 CI，由测试轮拉 PR 跑五道闸（与 task-061~073 一致）。

### 8. 与本任务文档不符的偏差（5 项）

1. **「按文件大小 / 修改时间筛选」未实现**（理由见 §2.3）：现有 ps1 没有任何筛选，加筛选会让两种模式结果不一致，且阈值没有依据。
2. **「与 ps1 输出格式一致（NDJSON），但直接返回 Vec」自相矛盾**：直接返回 Vec 就不需要 NDJSON。实际做成语义等价（同样的字段、同样的入库结果），跳过 NDJSON 那一层——这正是重写的收益。
3. **白名单漏 4 个文件**：`engine/src/scan/mod.rs`（注册模块，不改则新模块不参与编译）、`engine/src/scan/runner.rs`（`insert_batch`）、两个既有测试文件（回归修正）。
4. **没有 `engine/src/lib.rs` 改动**：任务书没列，实际也确实不需要（`pub mod scan` 早已存在）。
5. **`scan/modules/mod.rs` 的内容超出了「模块注册」**：任务书说它只做「模块注册」，实际还放了 `ScanItem` 与 `is_reparse_point()`——因为这两样是「所有 Rust 扫描模块共用」的东西，放在注册文件里比单独开一个 `types.rs` 更省文件数。

### 9. 遗留问题

- **真机 GUI / 安装态冒烟未做**（6 条，其中第 ② 条 `CCLEAN_RUST_TEMP=0` 对比是「回退开关真的有效」的验收）。
- **其余 4 个脚本未迁移**（task-075+）。建议顺序：`windows_update`（最简单，照抄模板）→ `prefetch`（权限问题）→ `browser_cache`（多 profile）→ `dev_cache`（npm/pnpm/pip 各不同）。
- **`scripts/` 依赖还在**：4 个迁完后才能删 `paths::scripts_dir` / `bundle.resources` 的 `scripts/` 映射 / `ScanRunner::run_script`。那才是 R5 真正的收尾。
- **风险等级仍是「脚本/代码各写一份」**：`rules/cache_rules.json` 的 `modules` 段至今没有消费者。全部迁到 Rust 后可以把分级收进 `ScanItem` 的构造处，规则库那份重复定义就能删掉——这是 R5 的额外收益，不是本任务范围。
- **`tasks/060-shutdown-experience.md` 仍是「工作区已删除、HEAD 里存在」**（连续 12 个任务），仍需人拍一下。
- **Items/SafeBox/History 三页深色适配**待补（与本任务无关，沿用既有遗留）。
