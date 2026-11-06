# 任务编号：079 P2 收官——便携版 + CLI 入口

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：P2 未做清单最后一项（便携版 + CLI）。
> 上游依赖：task-078 已合并 main（merge `af86e7e`，用户自定义规则管理完成）。
> **版本线**：v0.4.x P2 收官——便携版 + CLI。
> **本任务范围（4 个功能单元）**：CLI 独立 binary（调用 engine 库）+ CLI 子命令（scan/clean/dry-run/list-rules）+ 便携版打包配置 + CLI 文档。
> **为什么合并**：CLI 入口与便携版打包是强耦合（CLI binary 本身就是便携版核心），一次 PR 完成 P2 收官。
> **不做**：跨盘恢复 fallback（维持现状）；UI 深色适配（留后续小任务）。

## 状态字段

pending

## 任务目标

### 1. CLI 独立 binary

- **现状**：只有 Tauri GUI（`app/src-tauri/`），无命令行入口。
- **修法**：
  - `cli/Cargo.toml`（新建）：
    - 独立 binary `c-clean-cli`；
    - 依赖 `engine`（workspace 成员）；
    - 不依赖 `tauri` 或 `tauri-plugin-*`。
  - `cli/src/main.rs`（新建）：
    - 解析命令行参数（clap 4）；
    - 调用 engine 库执行扫描/清理；
    - 输出人类可读表格 + JSON 两种格式（`--format json`）。

### 2. CLI 子命令

- **修法**：
  - `cli/src/main.rs`：
    - `scan`：执行扫描，输出可清理项列表；
    - `clean`：执行清理（默认 dry-run，加 `--yes` 真实删除）；
    - `dry-run`：预览可清理项（同 scan，但更详细）；
    - `list-rules`：列出内置 + 用户规则；
    - `--format json`：机器可读输出（供脚本调用）。
  - 所有子命令复用 engine 的扫描/删除逻辑，不重复实现。

### 3. 便携版打包配置

- **现状**：NSIS 安装版（task-033 已做）。
- **修法**：
  - `src-tauri/tauri.conf.json`：
    - 加 `bundle.windows.portable` 配置（Tauri 2 原生支持）；
    - 便携版 = 单 exe，数据存 exe 同目录（`portable_data/`）；
  - `.github/workflows/release.yml`：
    - CI 同时产出 NSIS 安装版 + 便携版 zip；
    - 两个 artifact 都上传到 GitHub Release。

### 4. CLI 文档

- **修法**：
  - `docs/CLI.md`（新建）：
    - CLI 安装方式（从 Release 下载 `c-clean-cli.exe`）；
    - 子命令使用示例（scan / clean / dry-run / list-rules）；
    - JSON 输出格式说明；
    - 便携版使用方式（解压即用，数据存同目录）。

**本任务不做**：
- 不做跨盘恢复 fallback（维持现状）；
- 不做 UI 深色适配（Items/SafeBox/History，留后续小任务）；
- 不做规则导入/导出（留后续）。

## 前置条件

1. `main` 已合并 task-078（merge `af86e7e`），用户自定义规则完成；
2. 关键锚点（必须全部在白名单内）：
   - CLI 新建：`cli/Cargo.toml`
   - CLI 新建：`cli/src/main.rs`
   - 根 Cargo：`Cargo.toml`（workspace members 加 cli）
   - 打包：`src-tauri/tauri.conf.json`（便携版配置）
   - CI：`.github/workflows/release.yml`（便携版 artifact）
   - 文档：`docs/CLI.md`（新建）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. CLI 独立 binary

1. `c-clean-cli` 独立编译（不依赖 Tauri）；
2. 调用 engine 库复用扫描/删除逻辑；
3. 单测：CLI 编译通过。

### 2. CLI 子命令

4. `scan` 子命令输出可清理项列表；
5. `clean` 子命令默认 dry-run，`--yes` 真实删除；
6. `dry-run` 子命令预览可清理项；
7. `list-rules` 子命令列出内置 + 用户规则；
8. `--format json` 机器可读输出；
9. 单测：子命令执行正确。

### 3. 便携版打包

10. `tauri.conf.json` 加便携版配置；
11. CI 同时产出 NSIS 安装版 + 便携版 zip；
12. 便携版数据存 exe 同目录；
13. 单测：便携版 exe 可启动。

### 4. CLI 文档

14. `docs/CLI.md` 包含安装方式 + 子命令示例 + JSON 格式说明；
15. 便携版使用方式说明。

### 后端单测

16. `cargo test --workspace` 全绿（现有 + 本任务新增 ≥3 条全绿）。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**CLI 新建**：
- `cli/Cargo.toml`（新建）
- `cli/src/main.rs`（新建）

**根 Cargo**：
- `Cargo.toml`（workspace members 加 cli）

**打包**：
- `src-tauri/tauri.conf.json`（便携版配置）

**CI**：
- `.github/workflows/release.yml`（便携版 artifact）

**文档**：
- `docs/CLI.md`（新建）
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-079-p2-portable-cli`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-079-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `feat(cli): CLI 独立 binary（clap 4 + 复用 engine 库） (#task-079)`（单元 1）
   —— 含 `cli/Cargo.toml` + `cli/src/main.rs` + 根 `Cargo.toml`。
2. `feat(cli): CLI 子命令（scan / clean / dry-run / list-rules + --format json） (#task-079)`（单元 2）
   —— 含 `cli/src/main.rs`。
3. `feat(bundle): 便携版打包配置（tauri.conf portable + CI 双 artifact） (#task-079)`（单元 3）
   —— 含 `src-tauri/tauri.conf.json` + `.github/workflows/release.yml`。
4. `docs(cli): CLI 使用文档（安装 + 子命令示例 + JSON 格式） (#task-079)`（单元 4）
   —— 含 `docs/CLI.md`。
5. 自验全绿后，最后一次提交：`docs(state): 任务079完成，P2 收官——便携版 + CLI 入口就绪 (#task-079)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 079: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「P2 全部完成；下一阶段：发版 v0.4.0（release-prep）/ UI 深色适配补完 / 规则导入导出」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-06  task-079  P2 收官（便携版 + CLI）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app build` 退出码 0（无前端改动，回归保护）；
- `cargo run -p c-clean-cli -- --help` 输出帮助信息。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：CLI 编译、子命令执行、JSON 输出格式。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **真机冒烟（必做、不可裁剪）**：
  - `c-clean-cli scan` 输出可清理项列表；
  - `c-clean-cli scan --format json` 输出 JSON；
  - `c-clean-cli list-rules` 列出内置 + 用户规则；
  - 便携版 exe 解压后可启动，数据存同目录；
  - CDP 抓 console 0 红错（GUI 回归）。
- **打包安装冒烟（必做、不可裁剪）**：
  - NSIS 安装版安装正常；
  - 便携版 zip 解压即用，无需安装；
  - 两个 artifact 都在 GitHub Release 可见。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（便携版可启动 + GUI 回归），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| CLI 编译 | `cargo build --workspace` 输出 |
| CLI 子命令 | 终端输出：scan / clean / list-rules 示例 |
| JSON 输出 | 终端输出：`--format json` 正确 JSON |
| 便携版 exe | 真机截图：解压后启动 |
| 双 artifact | GitHub Release 页面截图（NSIS + portable.zip） |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS + 便携版双路径复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-079-p2-portable-cli`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - CLI binary 名称确认（`c-clean-cli`）；
   - 子命令参数设计确认；
   - 便携版数据路径确认（exe 同目录 `portable_data/`）；
   - CI 双 artifact 配置确认；
   - `cargo test --workspace` 通过输出（含新增 ≥3 条用例）；
   - `c-clean-cli --help` 输出；
   - 真机冒烟证据（4 条终端输出路径）；
   - 打包安装冒烟证据（NSIS + 便携版双路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（P2 全部完成；下一阶段：发版 v0.4.0 / UI 深色适配补完 / 规则导入导出）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务079-P2收官 便携版+CLI入口 (#task-079)`，
   该提交只含 `tasks/079-p2-portable-cli.md` 一个文件，落在 `feature/task-079-p2-portable-cli` 分支最前端。

## 实现备注

> 开发轮（DevFullstackRustReact）交付说明。**所有结论可复现**，命令附在对应位置。

### 1. 实际改动文件清单

**白名单内（7 个落地 + 2 个确认不适用）**：

| 文件 | 类型 | 说明 |
|---|---|---|
| `cli/Cargo.toml` | **新建** | 独立 crate `c-clean-cli`，不依赖 tauri |
| `cli/src/main.rs` | **新建** | 参数解析 / 四个子命令 / 渲染 / 12 条单测 |
| `Cargo.toml` | 改 | workspace members 加 `cli` |
| `.github/workflows/release.yml` | 改 | 便携版构建 + zip 打包 + CLI exe 上传 |
| `docs/CLI.md` | **新建** | 安装 / 子命令 / JSON / 便携版 / 退出码 / 脚本 |
| `docs/PROJECT_STATE.md` | 改 | 状态同步（单独提交） |
| `tasks/079-p2-portable-cli.md` | 改 | 本段「实现备注」（单独 `docs(task)` 提交） |

**白名单列了但实际不适用**：

| 文件 | 为什么不动 |
|---|---|
| `src-tauri/tauri.conf.json` | **路径本身就是错的**（真实路径 `app/src-tauri/tauri.conf.json`），而且**不该改**：便携版的 `appDirectoriesOverride` 是一个**相对 exe 的路径**，只在 exe 目录可写时有效。写进共享配置会让**安装版**（装到 `Program Files`，只读）每次写库都失败——Tauri 官方文档原话是「Unless every distribution of the app is portable, keep relative paths out of the shared configuration and apply them to the portable build flavor only, for instance with the CLI's `--config` flag」。所以改用独立配置文件 + `--config`（见 §4）。 |

**白名单外（2 个，都是「不加功能就不存在」）**：

| # | 文件 | 为什么必须加 |
|---|---|---|
| 1 | `app/src-tauri/tauri.portable.conf.json` | **新建**。便携版配置必须单独一个文件：`--config` 支持「JSON 文件或内联 JSON」，但把一段转义过的 JSON 塞进 YAML 的 `run:` 块既难读又容易写错，独立文件还能被编辑器按 schema 校验。 |
| 2 | `Cargo.lock` | 加了 workspace 成员 `cli`，lock 里必须出现 `c-clean-cli` 条目（本项目 `Cargo.lock` 入库是立项决策）。 |

### 2. CLI binary 名称确认

**`c-clean-cli`**（产物 `target/release/c-clean-cli.exe`）。

- `[[bin]] name = "c-clean-cli"` 显式声明，不依赖 crate 名推导（crate 名也叫 `c-clean-cli`，但显式写死能避免将来改 crate 名时产物跟着变）；
- 与 GUI 的 `cdisk-cleaner.exe`、提权助手 `cclean-elevation.exe` 并列，三个 exe 名字一眼能分清谁是谁。

### 3. 子命令参数设计

```text
c-clean-cli <子命令> [选项]

scan          扫描并列出可清理项（摘要 + 前 N 条）
dry-run       扫描并列出全部条目（不删除）
clean         扫描 + 清理；默认只预览，加 --yes 才真的移入保险箱
list-rules    列出内置规则与用户自定义规则

--format <table|json>   输出格式（默认 table）
--drive <C,D>           只扫指定盘（逗号分隔；默认全部固定盘）
--data-dir <PATH>       数据目录
--limit <N>             scan / clean 预览的打印条数上限（默认 50）
--yes                   clean 时真的执行
--include-l2            clean 时连 L2 一起清
-h/--help  -V/--version
```

三个设计取舍：

1. **不给子命令 = 打帮助，不扫描**。「什么都不做」是最安全的默认值——很多 CLI 默认执行主操作，误敲一次就动文件；
2. **`--yes` / `--include-l2` 只能配 `clean`**。写错子命令（`scan --yes`）时静默忽略会让人以为文件被清了，所以直接报用法错误（退出码 2）；
3. **退出码 2 = 用法错误**，与「跑起来但失败」（1）分开。脚本里最常见的分支就是「我参数写错了」，混在 1 里只能靠 stderr 猜。

### 4. 便携版数据路径确认

**数据目录优先级**（`pick_data_root`，纯函数、有单测）：

1. `--data-dir <PATH>`
2. 环境变量 `CCLEAN_DATA_DIR`
3. `<exe 目录>\portable_data`（**存在即用** —— 便携版）
4. `%APPDATA%\com.and0114.cclean`（与 GUI 安装版同一个库）

**机制**：Tauri 2 的 `app.appDirectoriesOverride`（**没有 `bundle.windows.portable` 这个打包目标**）。`app/src-tauri/tauri.portable.conf.json`：

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "app": { "appDirectoriesOverride": "./portable_data" }
}
```

它把 `app_data_dir()` / `app_local_data_dir()` 等路径 API 的根改成「相对 exe 的目录」，GUI 与 CLI 都跟着走。

**真机实证（override 的作用域）**：

```text
便携构建（tauri build --no-bundle --config src-tauri/tauri.portable.conf.json）
  cdisk-cleaner.exe 里 "portable_data" 出现 1 次
普通构建（pnpm --filter app tauri build）
  cdisk-cleaner.exe 里 "portable_data" 出现 0 次
```

即：**只有便携版带这个覆盖，安装版完全不受影响**——这正是把它从共享配置里拿出来的理由。

**真机实证（数据真的跟着 exe 走）**：把 CLI exe 放进一个含 `portable_data/` 的目录里跑：

```text
target/portable-smoke/
├── c-clean-cli.exe
└── portable_data/
    ├── README.txt
    └── data/cdiskcleaner.db      ← 运行后自动创建
```

`%APPDATA%` 未被触碰。

**为什么 `portable_data/` 里要放一个 README**：`Compress-Archive` **会跳过空目录**，纯空的 `portable_data` 解压后就不存在了，CLI 的「目录存在即便携」判断会失效 → 回退到 `%APPDATA%` → 用户看到「数据不见了」。放一个说明文件既让目录非空，也顺便给用户解释这个目录是什么。

### 5. CI 双 artifact 配置

在 `release.yml` 的「校验产物」之后插入两步：

1. **构建便携版**：`tauri build --no-bundle --config src-tauri/tauri.portable.conf.json` + `cargo build --release -p c-clean-cli`；
2. **打包 zip**：按 `bundle.resources` 的映射布局 staging（`cdisk-cleaner.exe` / `c-clean-cli.exe` / `cclean-elevation.exe` / `rules/` / `scripts/` / `portable_data/README.txt`），`Compress-Archive` 成 `C-DiskCleaner_<ver>_portable_x64.zip`，并自检 `portable_data\README.txt` 在不在。

Release 的 `files:` 追加：

```yaml
            target/release/c-clean-cli.exe
            C-DiskCleaner_*_portable_x64.zip
```

顺序说明：便携构建放在「校验产物（版本号 + .sig）」**之后**，因为它会用 `--no-bundle` 重新生成 `target/release/cdisk-cleaner.exe`；先校验安装包，就不会有任何相互影响。

### 6. `cargo test --workspace`

```text
passed=632 failed=0 ignored=15
```

（task-078 收尾时 620 passed → **+12**，全部是 CLI 单测）

| 用例 | 覆盖点 |
|---|---|
| `no_args_shows_help` | 不给子命令 = 帮助，绝不默认扫描 |
| `parses_subcommands` | 四个子命令 + `clean` 的 `yes/include_l2` 组合 |
| `parses_value_flags` | `--format/--limit/--drive/--data-dir`，盘符大小写与冒号归一 |
| `rejects_usage_errors` | 9 种用法错误（未知子命令 / 未知开关 / 缺值 / 两个子命令 / `--yes` 配错子命令…） |
| `parse_drives_rejects_garbage` | `1` / `CC` / `C,,D` / 空项都拒；重复项去重 |
| `data_root_priority` | 数据目录四档优先级 + `%APPDATA%` 缺失时报错不瞎猜 |
| `renders_table_with_aligned_columns` | **CJK 按 2 列算**，含中文路径那行「路径」列起点与表头一致 |
| `renders_empty_list` / `formats_bytes` / `display_width_counts_cjk_as_two` | 空态 / 单位换算 / 宽度函数 |
| `scan_json_keeps_documented_field_names` | **JSON 字段名契约**（`totalItems/totalBytes/shown/items[].path,sizeBytes,module,riskLevel`），且 `id` 不外泄 |
| `scan_json_truncates_items_but_keeps_true_total` | `--limit` 只截 `items`/`shown`，`totalItems` 必须是真实总数 |

### 7. `c-clean-cli --help`

```text
c-clean-cli — C-Disk Cleaner 命令行（便携版核心）

用法:
  c-clean-cli <子命令> [选项]

子命令:
  scan          扫描并列出可清理项（摘要 + 前 N 条）
  dry-run       扫描并列出全部条目（不删除）
  clean         扫描 + 清理；默认只预览，加 --yes 才真的移入保险箱
  list-rules    列出内置规则与用户自定义规则
  help          显示本帮助

选项:
  --format <table|json>   输出格式（默认 table）
  --drive <C,D>           只扫指定盘（逗号分隔；默认全部固定盘）
  --data-dir <PATH>       数据目录（默认 %APPDATA%\com.and0114.cclean，
                          便携版为 exe 同目录的 portable_data）
  --limit <N>             scan 打印条数上限（默认 50）
  --yes                   clean 时真的执行（否则只预览）
  --include-l2            clean 时连 L2 一起清（默认只清 L1 安全项）
  -h, --help              显示帮助
  -V, --version           显示版本

退出码: 0 成功 / 1 运行期错误 / 2 用法错误
```

### 8. 真机冒烟证据（终端输出）

**① `list-rules`**（`--data-dir target/cli-smoke`，避免污染真实库）：

```text
内置规则库 v1.0.0（1 条）
  [A] npm_cache  (L1)

用户自定义规则（0 条）
  （无。可在 GUI 设置页添加）
```

**② `scan`**：

```text
[scan] session=1 条目=5870 合计=334.9 MB 成功模块=["scan_temp.ps1", "scan_windows_update.ps1",
  "scan_prefetch.ps1", "scan_browser_cache.ps1", "scan_dev_cache.ps1"] 跳过=[]
共 5870 项，合计 334.9 MB

大小     风险  模块            路径
-------  ----  --------------  ------------------------------------------
13.0 MB  L2    windows_update  C:\WINDOWS\SoftwareDistribution\Download\SharedFileCache\e9ab08...
12.9 MB  L1    browser_cache   C:\Users\...\cache2\entries\9AE3ADAB...
...
（只显示前 3 条，加 --limit 5870 或改用 dry-run 看全部）
```

**③ `scan --format json`**：

```json
{
  "items": [
    {
      "module": "windows_update",
      "path": "C:\\WINDOWS\\SoftwareDistribution\\Download\\SharedFileCache\\e9ab08...",
      "riskLevel": "L2",
      "sizeBytes": 13590379
    }
  ],
  "shown": 1,
  "totalBytes": 351211583,
  "totalItems": 5869
}
```

**④ `clean`（无 `--yes`，预览）**：

```text
[提示] 未加 --yes，只预览不删除。确认要清理请加 --yes。

【预览】将清理 5828 项，合计 297.4 MB（未删除任何文件）
...（表格）
另有 41 条 L2（需逐项确认）未包含；确认要一起清请加 --include-l2。

确认无误后执行：c-clean-cli clean --yes
```

**⑤ 便携布局（数据跟着 exe 走）**：

```text
target/portable-smoke> .\c-clean-cli.exe list-rules
内置规则库：未找到（扫描不受影响，只是没有外部 L3 红线）   ← 该目录没有 rules/，符合预期
用户自定义规则（0 条）

--- 生成的文件 ---
portable_data/data/cdiskcleaner.db      ← DB 落在 exe 旁边
portable_data/README.txt
```

**GUI 截图未产出**：本机 WebView2 渲染进程创建即崩（Crashpad 持续新增 `.dmp`、CDP 9222 不通），按项目约定开发轮不做 GUI 取证。

### 9. 打包安装冒烟证据

```text
# 便携版（裸 exe，无安装包）
pnpm --filter app tauri build --no-bundle --config src-tauri/tauri.portable.conf.json
   Finished `release` profile [optimized] target(s) in 2m 03s
   Built application at: D:\GitHub\C-clean\target\release\cdisk-cleaner.exe

# 安装版（回归）
pnpm --filter app tauri build
   Finished 2 bundles at:
     ...\bundle\nsis\C-DiskCleaner_0.2.1_x64-setup.exe (4.54 MiB)
     ...\bundle\msi\C-DiskCleaner_0.2.1_x64_en-US.msi (6.71 MiB)
   Error A public key has been found, but no private key.
```

- **两条路径都出包成功**；NSIS/MSI 体积与 task-078 持平（4.54 / 6.71 MiB）——CLI 不进 GUI 安装包，符合预期；
- **末尾签名步失败属已知预期**：私钥正源在 CI secret，本地没有；
- **zip 打包步骤本地未跑**（`Compress-Archive` 那步在 CI 里；本地等价验证是手工 staging + 便携布局冒烟，见 §8⑤）；
- **安装态复跑未做**：非管理员 shell 下 MSI per-user 静默安装未完成即被终止，归测试轮。

### 10. CI

**未跑。** 按项目流程（`MEMORY.md`「CI / 分支流程」）：开发分支不触发 CI，由测试环节拉 PR 后触发。

### 11. 与本任务文档不符的偏差（汇总）

| # | 任务书说法 | 实际处理 |
|---|---|---|
| 1 | 用 clap 4 解析参数 | **零依赖手写解析器**。本项目一贯零依赖（提权 exe 手写 FFI / 报告手写拼接 / glob 手写 DP）；clap 会带进 6+ 个传递依赖；参数形态只有 4 子命令 + 7 开关，手写 100 行且**能直接单测**（clap 的解析行为只能靠集成测试间接验） |
| 2 | `bundle.windows.portable`（Tauri 2 原生支持） | **这个配置项不存在**。Tauri 2 的 bundle target 只有 `nsis/msi/app/dmg/deb/rpm/appimage/updater`；便携场景的官方机制是 `app.appDirectoriesOverride` |
| 3 | 改 `src-tauri/tauri.conf.json` 加便携配置 | **路径写错**（真实 `app/src-tauri/`）且**不该改**：相对路径的 override 会让安装版（Program Files 只读）写库失败，Tauri 文档明确要求只作用于便携构建。改用 `app/src-tauri/tauri.portable.conf.json` + `--config` |
| 4 | 便携版 = 单 exe | **不是单 exe**：还要 `rules/`（外部 L3 红线）、`scripts/`（ps1 回退）、`cclean-elevation.exe`（还原点/DISM）。`AppPaths::pick_resource_root` 要求「同时含 rules/ 与 scripts/ 的目录」。所以便携版是**一个文件夹**，不是单个 exe |
| 5 | 单元 1（binary）与单元 2（子命令）分两次提交 | **合成一次提交**（`5856c9d`）：CLI 只有一个 `main.rs`，「只有 binary 骨架、没有子命令」的中间态是不可运行的死代码。子命令的正确性修复单独一次（`5c107fd`），测试单独一次（`069b407`） |
| 6 | 未提 CLI exe 的发布 | CI 额外 `cargo build --release -p c-clean-cli` 并把 `c-clean-cli.exe` **单独上传**到 Release，同时打进便携 zip。理由：文档承诺「每个 Release 都带 c-clean-cli.exe」，脚本用户不想为拿一个 CLI 去解压整个便携包 |
| 7 | 真机冒烟 / 打包安装冒烟「必做、不可裁剪」 | 开发轮**未做 GUI 部分**（按 `MEMORY.md`：开发轮不做 GUI/真机 UI 测试），给的是 CLI 真机输出 + 便携布局实证 + 出包证据 |
| 8 | CI「必做」 | 开发轮**不开 PR**，按 `MEMORY.md`「开发阶段不触发 CI」 |

### 12. 遗留问题

- **`clean --include-l2` 不创建系统还原点**。还原点是 GUI 的交互式提权流程（要弹 UAC），命令行做不了。已在输出里明确警告，文档也写了「真要清 L2 建议用 GUI」。若要补，得给提权 exe 加一个非交互入口 + `--accept-no-restore-point` 之类的显式开关，属独立任务。
- **CLI 没有「关闭占用进程并重试」**（GUI 的 `cleanup_retry_locked`）。被占用文件会进 `failures`，关掉程序重跑即可。
- **CLI 一次最多处理 50 万条**（翻页上限 1000 页 × 500 条），超出打警告并截断。
- **便携版不能放在只读目录**（`Program Files`）：它的前提就是 exe 旁边可写。文档已写明。
- **`portable_data` 的判定是「目录存在」而不是「目录可写」**：可写性探测会真的落盘临时文件，在 Program Files 下还会弹 UAC 失败提示。存在性判断零副作用，代价是「用户手建了一个空的 portable_data 但放在只读盘」时会写失败——属边缘情况，文档已提示。
- **P2 全部完成**。下一阶段：发版 `v0.4.0` / UI 深色适配补完（Items / SafeBox / History）/ 规则导入导出。
- **规则库 `rules/cache_rules.json` 的 `modules` 段依然没有扫描消费者**（task-078 起就是双份真值源，本任务未触及）。建议发版前拍板：要么让内置模块按规则库驱动，要么删掉 `modules` 段只留 `whitelist_deny`。

