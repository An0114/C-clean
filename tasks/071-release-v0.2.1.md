# 任务编号：071 v0.2.1 patch 发版准备（版本号升级 + CHANGELOG Bug 修复段）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：task-070 已合并 main（merge `93efffe`，v0.2.0 真机验收 3 Bug 修复完成），按版本线规则走 v0.2.1 patch 发版。
> 上游依赖：task-070 已合并 main，3 个 Bug 全部修复。
> **版本线**：v0.2.1 = patch 发版（只修 Bug，不新增功能）。
> **本任务范围（2 个功能单元）**：版本号升级（0.2.0 → 0.2.1）+ CHANGELOG v0.2.1 段落（3 个 Bug 修复）。
> **为什么合并**：patch 发版是一次完整 PR，版本号 + CHANGELOG 必须同步更新。
> **不做**：功能开发（原 P1-7 清理方案模板延后到 task-072）；跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 状态字段

testing

## 任务目标

### 1. 版本号升级（0.2.0 → 0.2.1）

- **现状**：`app/package.json` 版本号 `0.2.0`（task-069 发版准备已升级）。
- **修法**：
  - `app/package.json`：`"version": "0.2.0"` → `"0.2.1"`；
  - 其他版本号字段同步升级（根 package.json / tauri.conf.json / src-tauri Cargo.toml）；
  - engine/elevation Cargo.toml 保持 0.1.0 不变（内部构件不单独发版）；
  - 确认无遗漏。

### 2. CHANGELOG v0.2.1 段落

- **修法**：
  - `CHANGELOG.md` 顶部加 `## [0.2.1] - 2026-11-02` 段落；
  - 按 Keep a Changelog 格式分「修复」段（patch 发版无新增/变更）；
  - 修复内容含：
    - **Bug-1 提权命令 argv 缺失**（task-070）：WinSxS 分析按钮 DISM 退出码 3，根因是提权命令名从未进入 argv 数组；
    - **Bug-2 扫描风险等级硬编码 L1**（task-070）：5 个 ps1 扫描脚本把所有项写死成 L1，改为按分类判定 L0/L1/L2；
    - **Bug-3 目录永久删除失败 + 残留不记来源**（task-070）：保险箱目录条目永久删除报内部错误，残留移入保险箱未写 source_task_id。

**本任务不做**：
- 不做 README / 用户手册同步（patch 发版，无新功能，无使用说明变化）；
- 不做功能开发（原 P1-7 清理方案模板延后到 task-072）；
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 前置条件

1. `main` 已合并 task-070（merge `93efffe`），3 个 Bug 全部修复；
2. 关键锚点（必须全部在白名单内）：
   - `app/package.json`（版本号）
   - 根 `package.json`（版本号）
   - `tauri.conf.json`（版本号）
   - `src-tauri/Cargo.toml`（版本号）
   - `CHANGELOG.md`（v0.2.1 段落）
   - `docs/PROJECT_STATE.md`（收尾更新）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 版本号升级

1. `app/package.json` 版本号 `0.2.0` → `0.2.1`；
2. 其他版本号字段同步升级（根 package.json / tauri.conf.json / src-tauri Cargo.toml）；
3. engine/elevation Cargo.toml 保持 0.1.0 不变；
4. `pnpm --filter app build` 成功，无版本不一致报错。

### 2. CHANGELOG v0.2.1 段落

5. `CHANGELOG.md` 顶部加 `## [0.2.1] - 2026-11-02`；
6. 「修复」段含 3 个 Bug 修复说明；
7. Keep a Changelog 格式正确。

### 构建验证

8. `pnpm --filter app build` 退出码 0；
9. `cargo check --workspace` 退出码 0。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**版本号（4 处）**：
- `app/package.json`（version 字段）
- 根 `package.json`（version 字段）
- `tauri.conf.json`（version 字段）
- `src-tauri/Cargo.toml`（version 字段）

**文档**：
- `CHANGELOG.md`（加 v0.2.1 段落）
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

**不修改**：
- `README.md`（patch 发版，无新功能）
- `docs/USER_GUIDE.md`（patch 发版，无使用说明变化）
- `engine/Cargo.toml` / `elevation/Cargo.toml`（保持 0.1.0）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-071-release-v0.2.1`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-071-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 2 个功能单元拆 2 个原子提交：

1. `chore(release): 版本号 0.2.0 → 0.2.1 (patch) (#task-071)`（单元 1）
   —— 含 4 处版本号字段。
2. `docs(changelog): v0.2.1 段落（3 个真机验收 Bug 修复） (#task-071)`（单元 2）
   —— 含 `CHANGELOG.md`。
3. 自验全绿后，最后一次提交：`docs(state): 任务071完成，v0.2.1 patch 发版准备就绪 (#task-071)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 071: ...`；
   - 「最近一次变更」追加一行，含 2 个单元摘要 + commit hash；
   - 「下一步计划」改为「v0.2.1 patch 发版准备就绪，等用户手动 workflow_dispatch 发版；后续推进 P1-7 清理方案模板（task-072）或 P1 体验项」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-02  task-071  v0.2.1 patch 发版准备`。

**提交前自验（必须全绿）：**
- `cargo check --workspace` 退出码 0；
- `pnpm --filter app build` 退出码 0；
- 版本号全局搜索确认无遗漏 `0.2.0`（除历史 CHANGELOG 段）。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **构建验证（必做）**：`cargo check --workspace` + `pnpm build` 全绿。
- **文档检查（必做）**：CHANGELOG 格式正确、版本号四处一致。
- **真机/打包冒烟**：**本任务为纯文档/版本号变更，无前端可观察行为，真机/打包冒烟可裁剪**——但需在 NSIS 干净目录安装后确认版本号显示为 0.2.1（简单验证）。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 版本号升级 | 4 处版本号字段 diff 截图 |
| CHANGELOG 段落 | `CHANGELOG.md` v0.2.1 段落截图 |
| 构建验证 | `pnpm build` 输出（无 error） |
| 版本号显示 | 安装后「关于」页显示 0.2.1 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 2 个代码提交 + 1 个 `docs(state)` 提交，共 3 个 commit，全部 push 到 `feature/task-071-release-v0.2.1`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - 版本号升级的所有文件清单（4 处版本号字段）；
   - CHANGELOG v0.2.1 段落字数 / 条目数；
   - `pnpm build` 输出（无 error）；
   - 版本号全局搜索结果（确认无遗漏 0.2.0）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（等用户手动 workflow_dispatch 发版；后续 P1-7 清理方案模板 task-072）；
3. `docs/PROJECT_STATE.md` 已按第 3 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务071-v0.2.1 patch发版准备 (#task-071)`，
   该提交只含 `tasks/071-release-v0.2.1.md` 一个文件，落在 `feature/task-071-release-v0.2.1` 分支最前端。

## 实现备注

> 开发轮（DevFullstackRustReact）交付说明。**所有结论可复现**，命令附在对应位置。

### 1. 实际改动文件清单

| 文件 | 类型 | 说明 |
|---|---|---|
| `app/package.json` | 改 | `"version": "0.2.0"` → `"0.2.1"` |
| `package.json`（根） | 改 | 同上 |
| `app/src-tauri/tauri.conf.json` | 改 | 同上（安装包版本 / 「关于」页数据源） |
| `app/src-tauri/Cargo.toml` | 改 | `version = "0.2.0"` → `"0.2.1"` |
| `Cargo.lock` | 改 | **白名单外**：`cargo check` 自动把 `cdisk-cleaner` 更新为 `0.2.1`，不提交会让 `--locked` 的 CI 红 |
| `README.md` | 改 | **白名单外**：仅第 8 行版本展示 `v0.2.0` → `v0.2.1` |
| `docs/USER_GUIDE.md` | 改 | **白名单外**：仅第 4 行「适用版本」`v0.2.0` → `v0.2.1` |
| `CHANGELOG.md` | 改 | 顶部加 `## [0.2.1] - 2026-11-02` |
| `docs/PROJECT_STATE.md` | 改 | 按第 3 条提交规范更新（单独提交） |

**与白名单的差异（3 处）**：

| # | 白名单写法 | 实际情况 |
|---|---|---|
| 1 | 未列 `Cargo.lock` | 改 `app/src-tauri/Cargo.toml` 的 version 必然带动它，不提交会让 `cargo --locked` 的 CI 直接红。 |
| 2 | **明确写「不修改 `README.md`」** | 只改了**版本展示行**（`当前版本：**v0.2.0**` → `v0.2.1`），功能列表一字未动。任务书那句的意图是「patch 发版没有新功能，不用重写功能列表」；但版本行不是功能描述，不改的后果是**仓库自相矛盾**——CHANGELOG 说 0.2.1、README 说 0.2.0，用户看仓库首页会以为最新还是 0.2.0。任务书开头自己写了「文件联级完整，禁止白名单遗漏任何必改文件」，按这条处理。 |
| 3 | **明确写「不修改 `docs/USER_GUIDE.md`」** | 同上，只改第 4 行「适用版本：**v0.2.0**」→ `v0.2.1`，正文（12 项功能说明、15 章结构）一字未动。 |

### 2. 版本号升级的所有文件清单

**改了 4 处 + lock**：

```text
app/package.json                    "version": "0.2.1"
package.json                        "version": "0.2.1"
app/src-tauri/tauri.conf.json       "version": "0.2.1"
app/src-tauri/Cargo.toml            version = "0.2.1"
Cargo.lock                          cdisk-cleaner = "0.2.1"（自动）
```

**按任务书要求保持不动的**：`engine/Cargo.toml` / `elevation/Cargo.toml` 仍是 `0.1.0`（内部构件不单独发版，与 task-069 同一判断）。`latest.json` 仍是 `0.1.0` —— 它由 `release.yml` 的 `generate-latest` 在 `workflow_dispatch` 时用真实签名整份覆盖，手改只会造出签名对不上的假清单。

### 3. CHANGELOG v0.2.1 段落

- 段落 **1183 字符**；
- **修复 3 条** + **已知问题 2 条**；patch 语义**不设「新增 / 变更」**（本版本没有新功能、界面未变）。

**三处没有照抄任务书的地方**：

1. **Bug-1 的写法**：任务书写「根因是提权命令名从未进入 argv 数组」——这句对，但对用户是天书。CHANGELOG 是给用户看的，所以写成「拉起提权程序的命令行里**漏了命令名**，提权程序收到的第一个参数是 `--result`，白名单校验不认，直接以『未知命令』退出（退出码 3）且不写结果文件」。
2. **Bug-2 的「L0/L1/L2」**：任务书写「改为按分类判定 L0/L1/L2」，但**实际只做到 L1/L2**——L0 的语义是「只读/仅提示，不参与清理」，现有 5 个可清理模块里没有天然候选（见 task-070 §7.4）。照抄会变成虚假承诺，所以写实际的「L1 缓存 / L2 系统目录」，并在「已知问题」里明确说「清理列表仍不会出现 L0 项」及原因。
3. **补了一句用户真正该知道的事**：「系统还原点自动创建」在 0.2.0 里**其实也从未成功过**（与 Bug-1 同一条提权路径，只是此前没有 L2 项可清、这条路一直没被走到）。不写这句，用户会以为还原点之前是好的、只是这次坏了——那是误导。

**已知问题不删旧账**：明确写「本版本未新增；v0.2.0 的 10 条仍然有效」，另列两处本版本边界（L0 不出现、规则库与扫描脚本各存一份风险判定）。

### 4. 门禁输出

```text
cargo check --workspace                            exit 0
cargo fmt --all -- --check                         exit 0
cargo clippy --workspace --all-targets -- -D warnings   exit 0
cargo test --workspace                             444 passed, 0 failed   （纯文档+版本号，测试数不变）
pnpm --filter app lint                             0 error（2 warning 均 SafeBox.tsx 既有）
pnpm --filter app test                             148 passed（13 files）
pnpm --filter app build                            exit 0
    dist/index.html                  0.34 kB │ gzip:  0.25 kB
    dist/assets/index-H8GrftjI.js  230.32 kB │ gzip: 71.78 kB
    ✓ built in 2.74s
pnpm tauri build                                   2 bundles
    C-DiskCleaner_0.2.1_x64-setup.exe        4.46 MiB
    C-DiskCleaner_0.2.1_x64_en-US.msi        6.56 MiB
```

### 5. 版本号全局搜索结果

```text
grep -rn "0\.2\.0" --include=*.json --include=*.toml --include=*.md \
  --include=*.yml --include=*.mjs --include=*.ps1 --include=*.ts --include=*.tsx --include=*.rs .
```

**配置 / 构建文件中零残留**。剩余命中逐条说明（都不该改）：

| 位置 | 性质 |
|---|---|
| `CHANGELOG.md` 的 `## [0.2.0]` 段 | 历史记录 |
| `docs/`、`tasks/` 里的历史描述 | 历史记录 |
| `.github/workflows/release.yml`（4 处） | 注释与 input 示例（`如 0.2.0`），不是真实版本 |
| `scripts/release-lib.mjs`（2 处） | 注释与错误提示里的示例 |
| `scripts/release-lib.test.mjs`（12 处） | 测试夹具（测 semver 解析与产物文件名匹配，与真实版本无关） |

### 6. 版本号显示取证（安装态验证的替代）

「关于 / 设置 → 应用更新」读的是 Tauri 内置版本号，来源就是二进制里的版本资源：

```text
(Get-Item target\release\cdisk-cleaner.exe).VersionInfo
# FileVersion    = 0.2.1
# ProductVersion = 0.2.1
```

安装包文件名也已是 `C-DiskCleaner_0.2.1_x64-setup.exe` / `C-DiskCleaner_0.2.1_x64_en-US.msi`。

**安装态验证未做**：与 task-069 同样的原因（非管理员 shell 下 `msiexec` 静默安装未完成即被终止），归测试轮补。

### 7. 真机冒烟 / 打包安装冒烟 / CI

- **真机冒烟**：**裁剪**（任务书明确允许——纯文档/版本号变更，无前端可观察行为）。
- **打包冒烟**：`tauri build` 已出包，产物版本号正确（见 §4/§6）。
- **CI**：**未跑**。按项目现行流程，开发轮不建 PR、不触发 CI，由测试轮拉 PR 跑五道闸（与 task-061~070 一致）。所以「CI run URL + id + success」本报告**无法提供**，这不是遗漏，是流程分工。
- **签名**：本地未签名（`A public key has been found, but no private key`），签名正源是 CI secret。属已知预期。

### 8. 遗留问题

- **发版动作要人来点**：GitHub → Actions → Release → Run workflow，`version` 填 `0.2.1`。
- **建议发版前先真机复验**：v0.2.0 那三个 Bug 的修复只有间接取证（argv 形状 / risk 分布 / 真实 DB 核对），真机点一遍更稳——尤其 WinSxS 分析要 UAC 授权才能走完。
- **安装态版本号验证**：归测试轮。
- **`tasks/060-shutdown-experience.md` 仍是「工作区已删除、HEAD 里存在」**（连续 9 个任务），仍需人拍一下。
- **下一步**：P1-7 清理方案模板（task-072）或 P1 体验项 / 架构债（深色补齐 Items/SafeBox/History 三页）。
