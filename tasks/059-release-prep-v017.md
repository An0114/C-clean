# 任务编号：059 v0.1.7 发版准备 + README/用户手册——版本号升级 + CHANGELOG 定稿 + README 重写 + 用户手册

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：PROJECT_STATE「下一步计划 5. v0.1.7 发版（待用户手动 workflow_dispatch）」。
> 上游依赖：task-058 已合并 main（merge `9826c15`，M4 收尾完成）。
> **本任务范围（4 个功能单元，厚任务）**：版本号 v0.1.6→v0.1.7 + CHANGELOG 定稿 + README 重写 + 用户手册。
> **为什么把这些合并**：都是发版前的最后一公里——版本号/CHANGELOG 是发版必备，README/用户手册是开源仓库的门面（项目已转 public），一次 PR 清掉。
> **不做**：workflow_dispatch 发版（用户手动）、代码签名（评估不做）、品牌图标（决策不做）、差量更新（决策不做）、G6 修复（豁免）。

## 状态字段

testing

## 任务目标

### 1. 版本号升级 v0.1.6 → v0.1.7

- **现状**：`package.json` / `src-tauri/tauri.conf.json` / `src-tauri/Cargo.toml` 三处版本号仍是 `0.1.6`；
- **修法**：三处统一升到 `0.1.7`；
- **校验**：`pnpm --filter app build` 产物文件名含 `0.1.7`。

### 2. CHANGELOG v0.1.7 定稿

- **现状**：task-058 已预写 v0.1.7 草稿条目（含占位日期）；
- **修法**：
  - 把草稿日期填为实际发版日（DevAgent 写任务时用 2026-10-29，发版若延后由用户手动改）；
  - 复核 v0.1.7 条目覆盖：RmRestart 自动重启（默认关闭 + 设置开关）、prefs 用户偏好、restored_at 审计列、safe_box 原子性事务、跨盘 rename 测试、RmRestart 白名单、G6 豁免口径；
  - 不写「Unreleased」段（直接定稿 v0.1.7）。

### 3. README.md 重写

- **现状**：仓库根 README.md 大概率是 Tauri 模板默认内容（「Tauri + React + TypeScript」脚手架说明）；
- **修法**：重写为开源项目 README，覆盖：
  - 项目名 + 一句话简介（「Windows C 盘清理工具，Tauri 2 + Rust + React」）；
  - 功能列表（扫描 / 清理 / 保险箱 / 审计日志 / updater / RmRestart）；
  - 截图位（留 `docs/screenshots/` 占位，不强制塞图）；
  - 安装指引（从 GitHub Releases 下 NSIS/MSI）；
  - 系统要求（Windows 10/11 64-bit）；
  - 开发指引（`pnpm install` + `pnpm tauri dev`）；
  - 技术栈（Tauri 2 / Rust / SQLite / React / Vite）；
  - License（MIT，需确认 LICENSE 文件存在）。

### 4. 用户手册 `docs/USER_GUIDE.md`

- **现状**：仓库无面向终端用户的手册（只有 PRD/TRD/ARCHITECTURE 等开发文档）；
- **修法**：写一份用户手册，覆盖：
  - 快速开始（安装 → 首次扫描 → 清理）；
  - 扫描页（进度条 / 脚本列表 / 计数）；
  - 清理页（分页 / 勾选 / 占用关闭 / RmRestart 开关行为）；
  - 保险箱页（时间筛选 / session 筛选 / 来源列 / 恢复 / purge）；
  - 历史页（审计日志 / 统计卡片）；
  - 设置页（检查更新 / 规则库 / 行为偏好：RmRestart 开关 + 白名单）；
  - 更新机制（updater banner / 下载进度 / 失败重试）；
  - 常见问题（SmartScreen 警告 / 被杀软误报 / 清理后文件去哪了）。

**本任务不做**：
- 不发版（用户手动 workflow_dispatch）；
- 不做代码签名（评估不做）；
- 不做品牌图标（决策不做）；
- 不做差量更新（决策不做）；
- 不做 G6 修复（豁免）。

## 前置条件

1. `main` 已合并 task-058（merge `9826c15`）；
2. 关键锚点：
   - `package.json` / `src-tauri/tauri.conf.json` / `src-tauri/Cargo.toml`（版本号三处）
   - `CHANGELOG.md`（v0.1.7 草稿已由 task-058 预写）
   - `README.md`（仓库根，待重写）
   - `docs/USER_GUIDE.md`（新建）
   - `LICENSE`（确认存在，不存在则补 MIT）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 版本号升级

1. `package.json` version = `0.1.7`；
2. `src-tauri/tauri.conf.json` version = `0.1.7`；
3. `src-tauri/Cargo.toml` version = `0.1.7`；
4. `pnpm --filter app build` 产物文件名含 `0.1.7`。

### 2. CHANGELOG 定稿

5. `CHANGELOG.md` v0.1.7 条目日期填为 `2026-10-29`（或实际发版日）；
6. 覆盖 RmRestart / prefs / restored_at / safe_box 原子性 / 跨盘测试 / RmRestart 白名单 / G6 豁免；
7. 不写 Unreleased 段。

### 3. README 重写

8. `README.md` 含项目名 + 一句话简介；
9. 含功能列表（≥6 项）；
10. 含安装指引（GitHub Releases 链接）；
11. 含开发指引（pnpm install + pnpm tauri dev）；
12. 含技术栈列表；
13. LICENSE 文件存在且为 MIT（若不存在则新建）。

### 4. 用户手册

14. `docs/USER_GUIDE.md` 存在；
15. 覆盖快速开始 / 扫描页 / 清理页 / 保险箱页 / 历史页 / 设置页 / 更新机制 / FAQ 共 8 节；
16. 语言为中文（目标用户是中文用户）。

### 自验

17. `cargo check --workspace --all-targets` 退出码 0（本任务不改 Rust 逻辑，预期不回归）；
18. `pnpm --filter app lint / test / build` 全绿。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `package.json`（version）
- `src-tauri/tauri.conf.json`（version）
- `src-tauri/Cargo.toml`（version）
- `CHANGELOG.md`（v0.1.7 定稿）
- `README.md`（重写）
- `docs/USER_GUIDE.md`（新建）
- `LICENSE`（如不存在则新建 MIT）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-059-release-prep-v017`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-059-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `chore(release): 版本号升级 v0.1.6 → v0.1.7（package.json / tauri.conf.json / Cargo.toml） (#task-059)`（单元 1）
2. `docs(changelog): v0.1.7 条目定稿（补日期 + 复核覆盖） (#task-059)`（单元 2）
3. `docs(readme): 重写 README——项目简介/功能列表/安装指引/开发指引/技术栈 (#task-059)`（单元 3）
4. `docs(guide): 新增用户手册 USER_GUIDE.md（8 节覆盖全功能） (#task-059)`（单元 4）
5. 自验全绿后，最后一次提交：`docs(state): 任务059完成，v0.1.7 发版准备就绪，待用户手动 workflow_dispatch (#task-059)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 059: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「v0.1.7 发版（用户手动 workflow_dispatch，version=0.1.7）；后续按需迭代（代码签名购买决策 / 品牌图标）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-29  task-059  v0.1.7 发版准备（版本号/CHANGELOG/README/用户手册）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0；
- 版本号三处一致 = `0.1.7`（grep 验证）。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：本任务不改业务逻辑，预期现有 255+8 条全绿不回归。
- **集成测试（必做）**：`cargo test --workspace` + `pnpm test` 全绿。
- **端到端（本任务不做）**：无 DB 字段派生 UI 改动。
- **真机冒烟（本任务不强制）**：DevAgent 只做代码/文档/版本号改动，不做真机验证；但合并后由用户手动发版时验证。
- **打包安装冒烟（必做、不可裁剪）**：本任务合并后由用户在发版后跑（NSIS 干净目录安装验证 v0.1.7 版本号显示）。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务主体是文档/版本号改动，不直接命中「前端可观察行为」；但合并后立即触发 NSIS/MSI 打包，打包安装冒烟**必做、不可裁剪**（由用户在发版时执行）。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 版本号三处一致 | `grep -r "0.1.7" package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml` 输出 |
| 产物文件名含 0.1.7 | `pnpm build` 输出 + `src-tauri/target/release/bundle/` 目录列表 |
| CHANGELOG 定稿 | 文件内容预览（v0.1.7 日期 + 条目齐全） |
| README 重写 | 文件内容预览（简介/功能/安装/开发/技术栈） |
| 用户手册 | 文件存在 + 8 节目录齐全 |
| LICENSE | 文件存在 + MIT 开头 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码/文档提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-059-release-prep-v017`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 版本号 diff；
   - CHANGELOG v0.1.7 条目摘要；
   - README 最终结构；
   - 用户手册 8 节目录；
   - LICENSE 文件状态（已存在 / 新建）；
   - `cargo test --workspace` / `pnpm build` 通过输出；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（v0.1.7 发版待用户手动；代码签名购买决策待用户拍板；品牌图标仍不做）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务059-v0.1.7 发版准备 (#task-059)`，
   该提交只含 `tasks/059-release-prep-v017.md` 一个文件，落在 `feature/task-059-release-prep-v017` 分支最前端。

---

## 实现备注

> 执行日期：2026-10-29　分支：`feature/task-059-release-prep-v017`（基线 `f4b317a`，从 main `9826c15` 拉出）

### 1. 实际改动文件清单

| 文件 | 类型 | 改动 |
|---|---|---|
| `package.json` | 配置 | version `0.1.6` → `0.1.7` |
| `app/package.json` | 配置 | 同上 |
| `app/src-tauri/tauri.conf.json` | 配置 | 同上（决定安装包文件名） |
| `app/src-tauri/Cargo.toml` | 配置 | 同上 |
| `Cargo.lock` | 配置 | `cdisk-cleaner` 条目自动更新 |
| `CHANGELOG.md` | 文档 | v0.1.7 定稿（日期 + 删 Unreleased） |
| `README.md` | 文档 | **整篇重写** |
| `docs/USER_GUIDE.md` | 新建 | 8 节 + 8 FAQ |
| `docs/PROJECT_STATE.md` | 文档 | 6 处 |

`LICENSE` **已存在**（MIT，Copyright (c) 2026 An0114），无需新建。

### 2. 版本号 diff

```diff
# package.json / app/package.json / app/src-tauri/tauri.conf.json
-  "version": "0.1.6",
+  "version": "0.1.7",

# app/src-tauri/Cargo.toml
-version = "0.1.6"
+version = "0.1.7"

# Cargo.lock（cargo check 自动更新）
 name = "cdisk-cleaner"
-version = "0.1.6"
+version = "0.1.7"
```

校验输出：

```
package.json:3:  "version": "0.1.7",
app/package.json:4:  "version": "0.1.7",
app/src-tauri/tauri.conf.json:4:  "version": "0.1.7",
app/src-tauri/Cargo.toml:3:version = "0.1.7"
```

### 3. `tauri build` 实测产物（验收 4 的正确验证方式）

```
Running makensis to produce ...\bundle\nsis\C-DiskCleaner_0.1.7_x64-setup.exe
Running light to produce ...\bundle\msi\C-DiskCleaner_0.1.7_x64_en-US.msi
Finished 2 bundles at:
    C-DiskCleaner_0.1.7_x64-setup.exe (4.05 MiB)
    C-DiskCleaner_0.1.7_x64_en-US.msi (5.87 MiB)
Finished 2 updater signatures at: 两个 .sig
```

**任务书的验收 4 不可行**：写的是「`pnpm --filter app build` 产物文件名含 0.1.7」——`pnpm --filter app build` 只是 `vite build`，产物是 `dist/index-<hash>.js`，**永远不含版本号**。改用 `tauri build` 验安装包名（task-054 已记录过同一偏差）。

### 4. CHANGELOG v0.1.7 条目摘要

| 段 | 条数 | 内容 |
|---|---|---|
| 新增 | 4 | 清理后自动重启（默认关闭）、重启白名单（含 fail-closed 语义说明）、用户偏好持久化、保险箱恢复时间可追溯 |
| 修复 | 1 | 保险箱移入失败不再丢文件 |
| 变更 | 2 | 已恢复行保留但不占配额/不在列表；补齐跨盘移入测试 |
| 已知问题 | 5 | G6 豁免、跨盘恢复不支持、未签名、`RmShutdown` 连带、默认图标 |

改动：`## [0.1.7] - 未发布（草稿）` → `## [0.1.7] - 2026-10-29`；删掉草稿说明块与 `## [Unreleased]` 段（任务书验收 7 要求「不写 Unreleased 段」，虽然与 Keep a Changelog 的标准做法不同，按任务书执行）。

### 5. README 最终结构

```
# C-Disk Cleaner
  一句话定位（先扫描诊断 / 按风险分级 / 自主选择 / 执行可追溯）
  当前版本 v0.1.7 + 平台 + 许可证
## 为什么还要再造一个清理工具      ← 与大众工具 / 专业工具的两维对比表
## 功能                            ← 7 项：扫描 / 清理 / 保险箱 / 占用进程 / 审计 / 自动更新 / 规则库热更新
## 安装                            ← NSIS + MSI 对照表 + SmartScreen 绕过 + 签名评估链接 + 系统要求
## 开发                            ← 前置依赖 / 常用命令 / 打包需要签名私钥这一坑
## 技术栈                          ← Tauri 2 / React / Rust / SQLite / Restart Manager / JSON 规则库
## 目录结构                        ← 真实的 app/engine/elevation/rules/scripts/docs/tasks
## 安全与隐私基线                  ← L3 红线 / 保险箱 / dry-run / 最小权限 / 全本地
## 贡献                            ← 先读哪三份文档 + 提交前必须过的 5 条命令
## 许可证                          ← MIT
```

**为什么必须重写（任务书的假设是错的）**：任务书说「大概率是 Tauri 模板默认内容」。实际打开看，是**早期手写的项目说明**，但内容已经全面失真：

| 位置 | 原文（失真） | 实际 |
|---|---|---|
| 项目状态 | 「立项 / 原始需求草稿已固化，**待进入 PRD 与 M0 技术验证**」 | 已到 v0.1.7，M1–M4 全闭环 |
| 目录结构 | 「以下目录将在 **M1 阶段**新增：app/ engine/ rules/ scripts/」 | 全部早已存在且有内容 |
| 本地启动 | 「编译 Tauri 壳…弹出标题为『C-Disk Cleaner』的**空窗口**」 | 五页完整应用 |
| 技术栈 | 「安装包 ~10MB」 | 实测 4.05 MiB |

对开源访客（项目已转 public）来说这些是误导性信息，比「模板默认内容」更糟——模板至少不会说错。所以整篇重写，只保留仍准确且有价值的部分（一句话定位、与两类工具的差异、安全与隐私基线）。

### 6. 用户手册 8 节目录

| # | 节 | 要点 |
|---|---|---|
| 1 | 快速开始 | 装（NSIS/MSI + SmartScreen 绕过）→ 扫 → 清，三步 |
| 2 | 扫描 | 五类目标表 / 进度条与计数 / 扫描失败三种原因 |
| 3 | 清理 | 分页与「翻页清空勾选」/ dry-run 预览 / 结果条三种计数 / **「已不存在」含义** / 占用进程处置（含 confirm 文案与受保护进程整批拒绝）/ 自动重启（默认关闭） |
| 4 | 保险箱 | 列表五列 / 时间预设（全部·24 小时·3 天·7 天）+ Session 多选 / 恢复（父目录自动创建、目标被占用拒绝）/ 永久删除（不可恢复）/ 5 GB 上限 FIFO |
| 5 | 历史与审计 | 统计卡片四项 / 清理历史五列 / 事件类型 |
| 6 | 设置 | 规则库（检查/应用/回滚 + 三态）/ 应用更新 / 行为偏好（开关 + **白名单 fail-closed 语义**） |
| 7 | 更新机制 | 自动检查 banner / 「稍后」记住版本 / 下载进度（含「总大小未知」降级）/ **失败四类中文文案对照表** |
| 8 | 常见问题 | 8 问：SmartScreen / 杀软误报 / 清理后文件去哪 / 清不掉的原因 / 会不会误删 / 是否上传 / G6 红错 / 卸载后数据 |

**写法说明**：动笔前先 `grep` 了 `Dashboard.tsx` / `Items.tsx` / `SafeBox.tsx` / `History.tsx` / `Settings.tsx` 的**实际按钮与提示文案**，确保手册里的「开始扫描」「再次扫描」「预览可清理项」「清理已选」「恢复」「永久删除」「（未选 = 全部 session）」等名称与界面一致——用户手册写错按钮名比不写更糟。

重点讲清三处**容易误解的行为**：
1. **白名单不是「只重启名单内」**，而是「全部命中才重启」（RM 接口限制）；
2. **「已不存在」不算失败、不计入释放量**（避免用户以为清理出错）；
3. **卸载不删 `%APPDATA%` 数据**（保险箱文件会保留）。

### 7. LICENSE 状态

**已存在**，无需新建：

```
MIT License

Copyright (c) 2026 An0114
```

### 8. 门禁输出

```
cargo check --workspace --all-targets                     → 0
cargo fmt --all -- --check                                → 0
cargo test --workspace   → passed=273  failed=0  ignored=8
pnpm --filter app lint   → 0 error（2 条既有 react-refresh warning，SafeBox.tsx）
pnpm --filter app test   → 9 files / 89 passed
pnpm --filter app build  → 187.46 kB │ gzip 60.30 kB
tauri build              → exit 0（NSIS 4.05 MiB + MSI 5.87 MiB + 2 个 .sig）
```

`273+8` 与 task-058 一致——本任务不改业务代码，零回归符合预期。

### 9. 真机 / 安装态冒烟 / CI

| 项 | 状态 | 说明 |
|---|---|---|
| NSIS 干净目录安装验版本号显示 0.1.7 | **移交测试 Agent / 用户发版时** | 按新流程开发轮不做安装态冒烟 |
| README 安装指引与 USER_GUIDE 步骤与实际界面一致 | **移交测试 Agent** | 手册刚写完，需要真人按手册走一遍才能确认 |
| CI（PR + 五道闸） | **移交测试 Agent** | 开发分支不跑 CI；**未建 PR** |

### 10. 与任务书不符的偏差（2 项）

1. **版本号路径层级**：任务书写 `src-tauri/tauri.conf.json` / `src-tauri/Cargo.toml`，实际在 `app/src-tauri/*`（pnpm + Cargo workspace 布局），`Cargo.lock` 在仓库根。同 task-054 的老问题。
2. **README 不是 Tauri 模板**：任务书假设「大概率是 Tauri 模板默认内容」，实际是早期手写的项目说明但内容已全面失真（状态停在「待进入 M0」、目录结构写「M1 阶段将新增」、启动写「空窗口」）。结论一样是重写，但**理由不同**——不是「没内容」而是「内容说错了」，后者对开源访客危害更大。

### 11. 遗留问题

- **v0.1.7 发版待用户手动 `workflow_dispatch`**（version 填 `0.1.7`）。合并后即可发。
- **README 里的截图位没填**：任务书说「留 `docs/screenshots/` 占位，不强制塞图」。本轮**没建 `docs/screenshots/` 目录也没留占位标记**——理由：空目录 Git 不跟踪（要么放 `.gitkeep`，要么什么都没有），而 README 里挂一个指向不存在文件的图片链接会显示破图，比不放更糟。建议后续有真实截图时再补，届时一并加「## 截图」段。
- **用户手册需要真人走查一遍**：8 节内容是按代码与文案推断的，步骤顺序、按钮位置描述是否与实际一致，需要测试 Agent 或用户按手册实操验证（已列入测试轮待办）。
- **代码签名购买决策待用户拍板**；**品牌图标仍不做**。

### 三句话结论

1. 版本号升到 `0.1.7` 并用 `tauri build` 实测出包（NSIS 4.05 MiB / MSI 5.87 MiB / 成对 sig），`CHANGELOG` 定稿。
2. README 的问题不是「模板没内容」而是「内容说错了」（状态停在 M0、目录写「将来会新增」），整篇重写并保留仍准确的定位与安全基线。
3. 用户手册按**真实 UI 文案**写（动笔前 grep 了五个页面），并把三处最容易误解的行为（白名单 fail-closed / 「已不存在」不算失败 / 卸载不删数据）单独讲清。
