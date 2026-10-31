# 任务编号：069 v0.2.0 发版准备（版本号升级 + CHANGELOG + README + 用户手册同步）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：M1 四模块全部闭环（task-065~068），按版本线规则走 v0.2.0 发版准备。
> 上游依赖：task-068 已合并 main（merge `2a6a82f`，M1 F 模块注册表健康中心完成）。
> **版本线**：v0.2.0 = 提权底座（task-062）+ P0 补完（task-063~064）+ M1 四模块（task-065~068）。
> **本任务范围（4 个功能单元）**：版本号四处升级 + CHANGELOG v0.2.0 段落 + README 同步 + 用户手册同步。
> **为什么合并**：发版准备是一次完整 PR，版本号 / CHANGELOG / README / 用户手册必须同步更新，一次性提交。
> **不做**：代码功能开发（M1 已全部完成）；跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 状态字段

testing

## 任务目标

### 1. 版本号升级（0.1.7 → 0.2.0）

- **现状**：`app/package.json` 版本号 `0.1.7`。
- **修法**：
  - `app/package.json`：`"version": "0.1.7"` → `"0.2.0"`；
  - 检查其他版本号字段（如 `tauri.conf.json` / `Cargo.toml` workspace 版本），同步升级；
  - 确认无遗漏。

### 2. CHANGELOG v0.2.0 段落

- **修法**：
  - `CHANGELOG.md` 顶部加 `## [0.2.0] - 2026-10-31` 段落；
  - 按 Keep a Changelog 格式分「新增 / 变更 / 修复」；
  - 新增内容含：
    - **提权 exe 底座**（task-062）：UAC 单次触发，白名单命令；
    - **系统还原点自动创建**（task-062）：L2/WinSxS/注册表操作前自动建；
    - **系统托盘常驻**（task-062）：关闭最小化 + 右键菜单 + 开机自启；
    - **清理报告导出**（task-063）：HTML 单文件 + CSV 明细；
    - **前后空间对比**（task-063）：清理前/后 C 盘已用差值显示；
    - **C 盘环形图**（task-063）：仪表盘 SVG 环形图 + 已用百分比；
    - **取消扫描按钮**（task-063）：cancellation token + 保留已扫描结果；
    - **保险箱 7 天自动 purge**（task-064）：过期行自动删除 + audit_log；
    - **风险徽标/影响说明**（task-064）：L0-L3 四色徽标 + tooltip；
    - **全选 L1 一键按钮**（task-064）：只选安全项 + 已选统计；
    - **WinSxS 清理**（task-065）：DISM 分析 + 清理（硬编码禁 /ResetBase）；
    - **大文件地图**（task-066）：hiberfil/Windows.old/回收站/Top100，只读不删；
    - **残留增强**（task-067）：注册表↔文件交叉比对，两类残留；
    - **注册表健康中心**（task-068）：Run 键/卸载残留/计划任务 + reg export 备份。

### 3. README 同步

- **修法**：
  - `README.md` 功能列表加 v0.2.0 新功能（提权 / 托盘 / 报告导出 / 环形图 / 取消扫描 / WinSxS / 大文件 / 残留 / 注册表）；
  - 截图或图标如有更新同步。

### 4. 用户手册同步

- **修法**：
  - `docs/USER_GUIDE.md` 加新功能使用说明：
    - 提权 exe 何时弹窗 / 还原点如何查看；
    - 托盘右键菜单各选项；
    - 报告导出格式 / 保存位置；
    - 环形图如何看；
    - 取消扫描行为；
    - 保险箱 7 天 purge 机制；
    - 风险徽标含义；
    - 全选 L1 按钮；
    - WinSxS 清理注意事项；
    - 大文件地图只读说明；
    - 残留扫描结果解读；
    - 注册表健康中心备份恢复。

**本任务不做**：
- 不做代码功能开发（M1 已全部完成）；
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）。

## 前置条件

1. `main` 已合并 task-068（merge `2a6a82f`），M1 四模块全部闭环；
2. 关键锚点（必须全部在白名单内）：
   - `app/package.json`（版本号）
   - `CHANGELOG.md`（v0.2.0 段落）
   - `README.md`（功能列表）
   - `docs/USER_GUIDE.md`（使用说明）
   - `docs/PROJECT_STATE.md`（收尾更新）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 版本号升级

1. `app/package.json` 版本号 `0.1.7` → `0.2.0`；
2. 其他版本号字段同步升级（如 tauri.conf.json / Cargo.toml）；
3. `pnpm --filter app build` 成功，无版本不一致报错。

### 2. CHANGELOG v0.2.0 段落

4. `CHANGELOG.md` 顶部加 `## [0.2.0] - 2026-10-31`；
5. 「新增」段含 task-062~068 全部 14 项功能；
6. Keep a Changelog 格式正确。

### 3. README 同步

7. `README.md` 功能列表加 v0.2.0 新功能；
8. 与实际功能一致（无夸大、无遗漏）。

### 4. 用户手册同步

9. `docs/USER_GUIDE.md` 加 12 项新功能使用说明；
10. 普通用户可按说明操作（无技术术语堆砌）。

### 构建验证

11. `pnpm --filter app build` 退出码 0；
12. `cargo check --workspace` 退出码 0。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**版本号**：
- `app/package.json`（version 字段）
- 如存在 `tauri.conf.json` 或 `Cargo.toml` workspace 版本，同步升级

**文档**：
- `CHANGELOG.md`（加 v0.2.0 段落）
- `README.md`（功能列表同步）
- `docs/USER_GUIDE.md`（使用说明同步）
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-069-release-v0.2.0`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-069-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `chore(release): 版本号 0.1.7 → 0.2.0 (#task-069)`（单元 1）
   —— 含 `app/package.json` + 其他版本号字段。
2. `docs(changelog): v0.2.0 段落（task-062~068 新增功能汇总） (#task-069)`（单元 2）
   —— 含 `CHANGELOG.md`。
3. `docs(readme): 功能列表同步 v0.2.0 新功能 (#task-069)`（单元 3）
   —— 含 `README.md`。
4. `docs(guide): 用户手册同步 v0.2.0 新功能使用说明 (#task-069)`（单元 4）
   —— 含 `docs/USER_GUIDE.md`。
5. 自验全绿后，最后一次提交：`docs(state): 任务069完成，v0.2.0 发版准备就绪 (#task-069)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 069: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「v0.2.0 发版准备就绪，等用户手动 workflow_dispatch 发版；后续进入 P1 体验项（趋势图增强 / 方案模板）或架构债（ps1→Rust 重写）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-31  task-069  v0.2.0 发版准备`。

**提交前自验（必须全绿）：**
- `cargo check --workspace` 退出码 0；
- `pnpm --filter app build` 退出码 0；
- 版本号全局搜索确认无遗漏 `0.1.7`（除历史 CHANGELOG 段）。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **构建验证（必做）**：`cargo check --workspace` + `pnpm build` 全绿。
- **文档检查（必做）**：CHANGELOG 格式正确、README/用户手册内容与实际功能一致。
- **真机/打包冒烟**：**本任务为纯文档/版本号变更，无前端可观察行为，真机/打包冒烟可裁剪**——但需在 NSIS 干净目录安装后确认版本号显示为 0.2.0（简单验证）。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 版本号升级 | `app/package.json` diff 截图 |
| CHANGELOG 段落 | `CHANGELOG.md` v0.2.0 段落截图 |
| README 同步 | `README.md` 功能列表截图 |
| 用户手册同步 | `docs/USER_GUIDE.md` 新功能段落截图 |
| 构建验证 | `pnpm build` 输出（无 error） |
| 版本号显示 | 安装后「关于」页显示 0.2.0 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-069-release-v0.2.0`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - 版本号升级的所有文件清单（含 tauri.conf.json / Cargo.toml 等）；
   - CHANGELOG v0.2.0 段落字数 / 条目数；
   - README 新增功能条目数；
   - 用户手册新增章节数；
   - `pnpm build` 输出（无 error）；
   - 版本号全局搜索结果（确认无遗漏 0.1.7）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（等用户手动 workflow_dispatch 发版；后续 P1 体验项 / 架构债排队）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务069-v0.2.0发版准备 (#task-069)`，
   该提交只含 `tasks/069-release-v0.2.0.md` 一个文件，落在 `feature/task-069-release-v0.2.0` 分支最前端。

## 实现备注

> 开发轮（DevFullstackRustReact）交付说明。**所有结论可复现**，命令附在对应位置。

### 1. 实际改动文件清单

| 文件 | 类型 | 说明 |
|---|---|---|
| `app/package.json` | 改 | `"version": "0.1.7"` → `"0.2.0"` |
| `package.json`（根） | 改 | **白名单外**：根包也写着 `0.1.7`，不同步会留一个自相矛盾的版本号 |
| `app/src-tauri/tauri.conf.json` | 改 | `"version": "0.1.7"` → `"0.2.0"`（安装包版本与「关于」页的数据源） |
| `app/src-tauri/Cargo.toml` | 改 | `version = "0.1.7"` → `"0.2.0"` |
| `Cargo.lock` | 改 | **白名单外**：`cargo check` 自动把 `cdisk-cleaner` 更新为 `0.2.0`，不提交会让 CI 的 `--locked` 失败 |
| `CHANGELOG.md` | 改 | 顶部加 `## [0.2.0] - 2026-10-31` |
| `README.md` | 改 | 版本行 + 功能段重写 + 技术栈 + 安全基线 |
| `docs/USER_GUIDE.md` | 改 | 8 章 331 行 → 15 章 766 行 |
| `docs/PROJECT_STATE.md` | 改 | 按第 5 条提交规范更新（单独一次提交） |

**与白名单的差异（2 处，均为「版本号字段遗漏」而非范围外改动）**：

| # | 白名单写法 | 实际情况 |
|---|---|---|
| 1 | 只列 `app/package.json` | **根 `package.json` 也有 `"version": "0.1.7"`**。不改就留一个「根包 0.1.7 / 应用 0.2.0」的矛盾版本号，而 pnpm workspace 的根包版本是发版脚本会读到的东西。任务书第 1 条自己也写了「确认无遗漏」，所以按「其他版本号字段同步升级」处理。 |
| 2 | 未列 `Cargo.lock` | `app/src-tauri/Cargo.toml` 的 version 一改，`Cargo.lock` 必然跟着变。**不提交会让 `cargo --locked` 的 CI 直接红**。 |

### 2. 版本号升级的所有文件清单

**改了 4 处 + lock**：

```text
app/package.json                    "version": "0.2.0"
package.json                        "version": "0.2.0"
app/src-tauri/tauri.conf.json       "version": "0.2.0"
app/src-tauri/Cargo.toml            version = "0.2.0"
Cargo.lock                          cdisk-cleaner = "0.2.0"（自动）
```

**刻意没改的 2 处**（不是漏了，是有理由）：

| 文件 | 当前值 | 为什么不改 |
|---|---|---|
| `engine/Cargo.toml`、`elevation/Cargo.toml` | `0.1.0` | 内部构件，不单独发布。对外只有 `cdisk-cleaner` 一个版本号；同步成 `0.2.0` 会让 `cargo metadata` 里出现两个从未发过版的「0.2.0」。v0.1.7 发版时同样保持 `0.1.0`。 |
| `latest.json` | `0.1.0` | updater 索引文件。`release.yml` 在 `workflow_dispatch` 时用 `scripts/release-lib.mjs generate-latest` **整份覆盖**（真实签名 + 真实下载地址）。手工改这里的 version 只会造出一份「版本号是 0.2.0、签名是占位符」的假清单，比不改更糟。 |

**版本号全局搜索结果**（排除 `node_modules` / `target` / CHANGELOG 历史段 / PROJECT_STATE 历史记录）：

```text
grep -rn "0\.1\.7" --include=*.json --include=*.toml --include=*.md \
  --include=*.yml --include=*.mjs --include=*.ps1 --include=*.ts --include=*.tsx --include=*.rs .
```

剩余命中全部合理，逐条说明：

| 位置 | 性质 |
|---|---|
| `app/src/App.test.tsx`（16 处） | **测试夹具的 mock 版本串**，测的是 banner「忽略某版本」的比较逻辑，与真实版本无关。改成 0.2.0 反而会让「忽略 0.1.7」这个语义失真。 |
| `app/src/pages/Settings.test.tsx`（6 处） | 同上（断言文案「当前 v0.1.6 / 发现 v0.1.7」）。 |
| `app/src/utils/update.ts:46` | 注释里的**举例**（`如 "0.1.7"`）。 |
| `CHANGELOG.md` / `docs/PROJECT_STATE.md` | 历史记录，按规矩**不能改**。 |

配置/构建文件中**零残留**。

### 3. CHANGELOG v0.2.0 段落

- 段落 **3454 字符**；
- 新增 **17** 条 / 变更 **4** 条 / 修复 **2** 条 / 已知问题 **10** 条，合计 **33** 条；
- 格式按 Keep a Changelog（`## [0.2.0] - 2026-10-31` + `### 新增/变更/修复/已知问题`）。

**新增 17 条 = 任务书要求的 14 项 + 补入的 task-061 三项**。为什么补：

```text
360df74  2026-10-29  merge: task-059 v0.1.7 发版准备
b92a763  2026-10-30  merge: task-061 M5 起步——主题切换+趋势图+自动扫描
                      ↑ git merge-base --is-ancestor 360df74 b92a763 返回真
```

task-061 的合并在 v0.1.7 发版**之后**，它是随 v0.2.0 一起交付的。任务书按「task-062~068」计数为 14 项，漏掉了 061——不补就是「代码里有、CHANGELOG 里没有」，发版说明失真。补入的三条：深浅色主题、30 天释放量趋势图、自动扫描。

**已知问题不删旧账**：v0.1.7 的 5 条（未签名 / G6 / 跨盘恢复 / RmShutdown 连带 / 默认图标）全部保留，另加 5 条本版本新增边界（Top100 截断 / 残留误报 / 计划任务需管理员 / DISM 15 分钟上限 / 深色未全覆盖）。这些是用户真会撞上的，藏起来只会变成 Issue。

### 4. README 新增功能条目数

功能段从 **7 条平铺**改为 **三组 17 条**（核心清理 6 / 系统深度清理 4 / 体验与集成 7），前面加一句「八个页签」总览。

写法上的两个决定：

1. **每条写「为什么不做某个操作」，不只写「支持 X」**。例如 WinSxS 那条明说「不提供 `/ResetBase`——该开关会让已安装更新无法卸载，本项目从结构上就没有这个参数入口」；大文件页明说「整页只读，因为这四类的正规处理方式各不相同」。这类说明比功能罗列有用得多，也是这个项目真正的差异点。
2. **技术栈如实写「不引第三方 winreg 封装」**。代码里确实是手写 `windows` crate FFI，README 不写清楚会让人以为引了 `winreg`。

### 5. 用户手册新增章节数

**8 章 331 行 → 15 章 766 行**，新增 7 个章节 + 5 条 FAQ：

| 新增章节 | 覆盖的任务书要求 |
|---|---|
| §2 权限与还原点 | 提权 exe 何时弹窗（4 种场景表）/ 还原点如何查看 / 建点失败怎么办 |
| §7 WinSxS 组件存储 | WinSxS 清理注意事项（含 15 分钟上限、两次 UAC、为什么没有 `/ResetBase`） |
| §8 大文件地图 | 大文件地图只读说明 + 「已达扫描上限」的含义 |
| §9 残留扫描 | 残留扫描结果解读（两类 / 置信度 / 风险 / 为什么注册表残留不代删） |
| §10 注册表健康中心 | 启动项删除与备份 / 备份怎么恢复 / 计划任务需管理员 |
| §13 托盘与开机自启 | 托盘右键菜单各选项 / 关窗 ≠ 退出 |
| §15 Q10~Q14（新增 FAQ） | 为什么弹 UAC / 关窗为什么没退出 / 大文件页为什么不能删 / 残留误报怎么办 / WinSxS 清完没变大 |

并入已有章节的（不单独开章）：环形图 → §3.1、自动扫描 → §3.3、取消扫描 → §4.3、风险徽标 → §5.2、全选 L1 → §5.3、报告导出 → §5.9、保险箱 7 天 purge → §6.5。

**为什么整篇重写而不是追加**：12 项里有 9 项属于「已有章节的补充」，全追加到文末会让手册变成一份没有结构的清单。重排后**章节顺序与界面页签顺序一致**，读手册和用界面是同一套心智模型。原 8 章内容全部保留（含跨盘恢复、RmShutdown 连带、占用进程处置这些血泪段落），只是重新归位。

**顺带修掉三处与实现不符的说法**（写错比不写更糟）：

1. 草稿一度写「清理结果条会显示已创建还原点 #N」——**普通清理页不显示序号**（还原点建在清理前，结果出来时那次操作已结束），只有 WinSxS 卡片显示。且还原点名是 `C-clean cleanup #<task_id> <UTC>`，时间是 UTC，不说明白用户会以为时间错了。
2. 草稿一度写「释放 X = 清理前后磁盘差值」——**正好反了**。`释放 X` 是文件大小之和（`bytes_freed_total`），磁盘差值另起一行「磁盘已用：A → B」（`disk_before_bytes` / `disk_after_bytes`），两者都有偏差来源。
3. 弹 UAC 时的程序名是 `cclean-elevation.exe` 而不是主程序名——不写清楚用户会怀疑是别的东西在要权限。

**内链自检**：30 条锚点全部有效（按 GitHub slug 规则程序化校验，含中文与 `（）？` 标点的处理）。方法：抽 `](#...)` 与 `^#{2,3} ` 标题，按 GitHub 规则归一化后比对。

### 6. 门禁输出

```text
cargo check --workspace                           exit 0
cargo fmt --all -- --check                        exit 0
cargo clippy --workspace --all-targets -- -D warnings   exit 0
cargo test --workspace                            431 passed, 0 failed
pnpm --filter app lint                            0 error（2 warning 均 SafeBox.tsx 既有）
pnpm --filter app test                            148 passed（13 files）
pnpm --filter app build                           exit 0
    dist/index.html                  0.34 kB │ gzip:  0.25 kB
    dist/assets/index-H8GrftjI.js  230.32 kB │ gzip: 71.78 kB
    ✓ built in 2.44s
pnpm tauri build                                  2 bundles
    C-DiskCleaner_0.2.0_x64-setup.exe        4.44 MiB
    C-DiskCleaner_0.2.0_x64_en-US.msi        6.55 MiB
```

### 7. 版本号显示取证（安装态验证的替代）

任务书要求「NSIS 干净目录安装后确认版本号显示为 0.2.0」。**安装这一步开发轮做不了**，但版本号本身可以更直接地取证——「关于 / 设置 → 应用更新」读的是 Tauri 内置的版本号，来源就是二进制里的版本资源：

```text
(Get-Item target\release\cdisk-cleaner.exe).VersionInfo
# FileVersion    = 0.2.0
# ProductVersion = 0.2.0
```

这比「装完看一眼」更硬：装完看到的字符串就是这个资源值。

**安装尝试记录（如实）**：`msiexec /i C-DiskCleaner_0.2.0_x64_en-US.msi /qn ALLUSERS=2 MSIINSTALLPERUSER=1 INSTALLDIR=<临时目录>` 在非管理员 shell 下**未完成即被终止**（`consent.exe` 无残留、临时目录 0 文件、系统里没装上）。与既有记录一致：本机静默安装不稳定，安装态验证归测试 Agent。

### 8. 真机冒烟 / 打包安装冒烟 / CI

- **真机冒烟**：**裁剪**（任务书允许——纯文档/版本号变更，无前端可观察行为）。
- **打包冒烟**：`tauri build` 已出包，产物版本号正确（见 §6/§7）。
- **CI**：**未跑**。按 `docs/PROJECT_STATE.md` 与项目约定，**开发轮不建 PR、不触发 CI，由测试轮拉 PR 跑五道闸**（与 task-061~068 一致）。所以「CI run URL + id + success」这一项本报告**无法提供**，这不是遗漏，是流程分工。
- **签名**：本地未签名（`A public key has been found, but no private key`）。仓库根的 `cclean-new.key` 是 base64 包着的 rsign 私钥，本地启用需额外解码处理；**签名正源是 CI 的 secret**，`release.yml` 会一并产出 `.sig` 与 `latest.json`。属已知预期。

### 9. 与本任务文档不符的偏差（3 项）

1. **新增功能 14 项 → 17 项**：补入 task-061 的三项（依据见 §3）。验收标准第 5 条「含 task-062~068 全部 14 项」已满足，17 是它的超集。
2. **白名单漏 2 个文件**：根 `package.json`（也有 version 字段）、`Cargo.lock`（改 Cargo.toml 的必然结果，不提交会让 `--locked` 的 CI 红）。见 §1。
3. **CI 未跑**：与项目现行流程（开发轮不跑、测试轮跑）冲突，按现行流程执行。任务书第 7 条与 `docs/PROJECT_STATE.md` 的流程约定不一致，**建议派发方后续把任务书模板里的 CI 条款统一掉**，否则每个任务都要解释一遍。

### 10. 遗留问题

- **发版动作本身要人来点**：GitHub → Actions → Release → Run workflow，`version` 填 `0.2.0`。本地只能验到「能出包 + 版本号对」，签名与 `latest.json` 由 CI 完成。
- **安装态版本号验证**：归测试轮（本机非管理员 shell 装不上）。
- **`tasks/060-shutdown-experience.md` 仍是「工作区已删除、HEAD 里存在」**：从 task-063 起连续 7 个任务如此，每个任务都用精确 `git add` 绕开。**需要拍一下是误删还是有意归档**——这已经成了流程噪音。
- **深色主题只覆盖 5/8 页**（Items/SafeBox/History 未适配），已在 CHANGELOG 已知问题登记。
- **发版后方向**：P1 体验项（趋势图增强 / 清理方案模板 / 自动扫描策略）或架构债（5 个 `.ps1` 扫描脚本 → Rust、深色补齐）。
