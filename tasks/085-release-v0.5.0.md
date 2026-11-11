# 任务编号：085 发版 v0.5.0——UI 体验批量优化 + 软件感知地基 + 大文件地图收官（release-prep）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：版本线规则——v0.5.0 全部 backlog（082~107）完成后发版 v0.5.0。
> 上游依赖：task-084 已合并 main（merge `d52b06f`，批次五六七 12 项收官）。
> **版本线**：v0.5.0 = UI 体验批量优化（task-082）+ 软件感知地基（task-083，批次二三四 8 项）+ 大文件地图与决策补全（task-084，批次五六七 12 项）。
> **本任务范围（4 个功能单元）**：版本号六处升级 + CHANGELOG v0.5.0 条目 + README/USER_GUIDE 同步 + 打 tag 触发 Release workflow（含 ARM64 矩阵）。
> **为什么合并**：发版准备是标准收尾任务，一次 PR 完成版本号 / CHANGELOG / 文档 / tag。
> **不做**：新功能开发（本任务只做发版准备）。

## 状态字段

pending

## 任务目标

### 1. 版本号六处升级（吸取 task-080 教训）

- **现状**：当前版本 v0.4.0（task-080 已发版）。
- **修法**（**注意 task-080 白名单遗漏教训，以下 6 处必须全部覆盖**）：
  - `app/src-tauri/tauri.conf.json`：`version` → `0.5.0`；
  - `app/src-tauri/Cargo.toml`：`version` → `0.5.0`；
  - `app/package.json`：`version` → `0.5.0`；
  - 根 `package.json`：`version` → `0.5.0`；
  - `cli/Cargo.toml`：`version` → `0.5.0`（**task-080 遗漏过，导致 `c-clean-cli --version` 出不来新版**）；
  - `Cargo.lock`：版本号变更的必然产物（`cargo build` 自动更新）。
- **不修改**：根 `Cargo.toml`（纯 workspace，无 version 字段）；`engine/Cargo.toml`（0.1.0 内部库版本独立）；`elevation/Cargo.toml`（0.1.0 独立）。

### 2. CHANGELOG v0.5.0 条目

- **现状**：`CHANGELOG.md` 最新为 v0.4.0。
- **修法**：
  - `CHANGELOG.md` 新增 v0.5.0 条目，四段：
    - **新增**：
      - UI 体验批量优化（task-082：仪表盘布局重排 + 命令/路径列横向滚动 + 托盘菜单 5 项 + 逐项影响说明）；
      - 软件感知地基（task-083：规则 schema `matches` + 内置模块改读规则库 + 软件枚举器 + 软件缓存识别 + L3 反向断言 + 多 Profile + UI 软件列 + 规则去重）；
      - 大文件地图深化（task-084：目标空间 Downloads/Documents/Desktop/Temp + 重复文件 SHA-256 + 文件夹分布 Top20 + 残留一键定位）；
      - 决策补全（task-084：自动清理 L1 拍板 + L0 一键清理 + 非系统盘产出 + 规则导入导出 + Gradle/Maven 缓存识别）；
      - P2 长尾（task-084：多用户 Profile + 草稿暂存 + ARM64 产物）；
    - **架构优化**：
      - 双份真值源收敛：5 个内置模块硬编码路径全部迁入规则库（task-083 A2）；
      - L3 红线反向断言：规则加载时防呆 panic（task-083 B3）；
      - 多 Profile 浏览器缓存识别（Chrome/Edge/Firefox，task-083 C1）；
    - **测试**：
      - 新增 ≥30 条单测（规则 schema / 软件枚举 / 重复文件 / 文件夹分布 / 草稿 / 规则 IO）；
      - 累计 Rust ≥668 passed + 前端 ≥214 passed；
    - **已知问题**：
      - 代码签名不做（SmartScreen 警告可接受）；
      - 品牌图标不做（Tauri 默认占位）；
      - 跨盘恢复 fallback / RmShutdown 三功能维持现状。

### 3. README / 用户手册同步

- **现状**：README / USER_GUIDE 描述 v0.4.0 功能。
- **修法**：
  - `README.md`：
    - 版本号 → v0.5.0；
    - 功能列表加：软件感知（识别已装软件缓存）/ 大文件地图深化（目标空间 + 重复文件 + 文件夹分布）/ 自动清理 / 规则导入导出 / 多用户 Profile / 草稿暂存 / ARM64 产物；
    - 安装表加 ARM64 一行；
  - `docs/USER_GUIDE.md`：
    - 版本号 → v0.5.0；
    - 新增章节：软件感知说明 / 大文件地图深化 / 自动清理设置 / 规则导入导出 / 多用户 Profile / 草稿暂存；
    - 交叉引用同步。

### 4. 打 tag 触发 Release workflow（含 ARM64 矩阵）

- **现状**：Release workflow 已支持 tag 触发（task-034）+ ARM64 matrix（task-084 C3）。
- **修法**：
  - 发版 PR 合并到 main 后，由 DocsKeeper 手动打 tag：
    - `git tag -a v0.5.0 -m "v0.5.0: UI 体验批量优化 + 软件感知地基 + 大文件地图收官"`
    - `git push origin v0.5.0`
  - tag 触发 `.github/workflows/release.yml`，自动构建 x64 + ARM64 产物。

**本任务不做**：
- 不做新功能开发（发版准备任务）；
- 不做代码签名 / 品牌图标（死路）。

## 前置条件

1. `main` 已合并 task-084（merge `d52b06f`），v0.5.0 全部 backlog 完成；
2. 关键锚点（必须全部在白名单内，**吸取 task-080 遗漏教训**）：
   - `app/src-tauri/tauri.conf.json`（版本号）
   - `app/src-tauri/Cargo.toml`（版本号）
   - `app/package.json`（版本号）
   - 根 `package.json`（版本号，**task-080 遗漏过**）
   - `cli/Cargo.toml`（版本号，**task-080 遗漏过**）
   - `Cargo.lock`（自动更新）
   - `CHANGELOG.md`（v0.5.0 条目）
   - `README.md`（功能列表同步）
   - `docs/USER_GUIDE.md`（新增章节）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 版本号六处升级

1. `app/src-tauri/tauri.conf.json` version = `0.5.0`；
2. `app/src-tauri/Cargo.toml` version = `0.5.0`；
3. `app/package.json` version = `0.5.0`；
4. 根 `package.json` version = `0.5.0`；
5. `cli/Cargo.toml` version = `0.5.0`；
6. `Cargo.lock` 同步更新（`cargo build` 自动）。

### 2. CHANGELOG v0.5.0 条目

7. `CHANGELOG.md` 新增 v0.5.0 条目；
8. 包含新增 / 架构优化 / 测试 / 已知问题四部分；
9. 版本号格式正确（`## [0.5.0] - 2026-11-11`）。

### 3. README / 用户手册同步

10. `README.md` 版本号 → v0.5.0；
11. 功能列表加软件感知 / 大文件地图深化 / 自动清理 / 规则导入导出 / 多用户 / 草稿 / ARM64；
12. 安装表加 ARM64；
13. `docs/USER_GUIDE.md` 新增 6 个章节 + 交叉引用同步。

### 4. 发版验证

14. `cargo build --workspace` 输出版本号 0.5.0；
15. `cargo build --release -p c-clean-cli` + `c-clean-cli --version` 输出 `0.5.0`；
16. `pnpm --filter app build` 成功；
17. PR 合并后可打 tag v0.5.0 触发 Release（含 x64 + ARM64）。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> ⚠️ 吸取 task-080 教训：白名单已包含根 `package.json`、`cli/Cargo.toml`、`Cargo.lock`。

**版本号（6 处）**：
- `app/src-tauri/tauri.conf.json`
- `app/src-tauri/Cargo.toml`
- `app/package.json`
- `package.json`（根）
- `cli/Cargo.toml`
- `Cargo.lock`（自动更新）

**文档**：
- `CHANGELOG.md`
- `README.md`
- `docs/USER_GUIDE.md`
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

**不修改**：
- 根 `Cargo.toml`（纯 workspace，无 version）
- `engine/Cargo.toml`（0.1.0 内部库独立）
- `elevation/Cargo.toml`（0.1.0 独立）
- 所有源代码（发版任务不改代码）
- `.github/workflows/release.yml`（ARM64 matrix 已由 task-084 加好）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-085-release-v0.5.0`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-085-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 3 个功能单元拆 3 个原子提交：

1. `chore(release): 版本号升级至 v0.5.0（六处同步） (#task-085)`（单元 1）
   —— 含 `tauri.conf.json` + `app/src-tauri/Cargo.toml` + `app/package.json` + 根 `package.json` + `cli/Cargo.toml` + `Cargo.lock`。
2. `docs(changelog): v0.5.0 条目（UI 批量优化 + 软件感知地基 + 大文件地图收官） (#task-085)`（单元 2）
   —— 含 `CHANGELOG.md`。
3. `docs(readme): 功能列表同步（软件感知 + 大文件深化 + 自动清理 + ARM64） (#task-085)`（单元 3）
   —— 含 `README.md` + `docs/USER_GUIDE.md`。
4. 自验全绿后，最后一次提交：`docs(state): 任务085完成，v0.5.0 release-prep 就绪 (#task-085)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 085: ...`；
   - 「最近一次变更」追加一行，含 3 个单元摘要 + commit hash；
   - 「下一步计划」改为「v0.5.0 发版完成；下一阶段：由用户拍板下一里程碑（v0.6.0 / 架构债 / 体验深化）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-11  task-085  发版 v0.5.0（UI + 软件感知 + 大文件地图收官）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo build --workspace` 输出版本号 0.5.0；
- `cargo build --release -p c-clean-cli` 成功；
- `./target/release/c-clean-cli.exe --version` 输出 `c-clean-cli 0.5.0`；
- `pnpm --filter app build` 成功；
- `pnpm --filter app lint` 退出码 0（无代码改动，回归保护）。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（不适用）**：本任务纯文档/版本号，无代码改动。
- **集成测试（必做）**：`cargo build --workspace` + `pnpm build` 版本号正确；CLI `--version` 输出 0.5.0。
- **真机冒烟（必做、不可裁剪）**：
  - 启动应用，关于页 / 设置页显示版本号 v0.5.0；
  - CLI `c-clean-cli --version` 输出 v0.5.0。
- **打包安装冒烟（必做、不可裁剪）**：
  - NSIS x64 安装版安装后版本号 v0.5.0；
  - MSI x64 安装后版本号 v0.5.0；
  - 便携版解压后版本号 v0.5.0；
  - ARM64 产物（如 runner 可用）安装后版本号 v0.5.0。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（版本号可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 版本号六处升级 | `cargo build` + `pnpm build` + `c-clean-cli --version` 输出 |
| CHANGELOG 条目 | `CHANGELOG.md` 截图 |
| README 同步 | `README.md` 截图 |
| USER_GUIDE 新增章节 | 章节目录截图 |
| 应用版本号 | 真机截图：关于页 / 设置页 v0.5.0 |
| CLI 版本号 | 终端输出：`c-clean-cli --version` → 0.5.0 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | 版本号 v0.5.0 确认 |
| 打包安装冒烟 | NSIS x64 + MSI x64 + 便携版 + ARM64 四路径版本号确认 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-085-release-v0.5.0`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - 版本号六处确认（含 CLI --version 输出）；
   - CHANGELOG v0.5.0 条目内容摘要；
   - README 功能列表更新摘要；
   - USER_GUIDE 新增章节清单；
   - `cargo build --workspace` 输出（版本号 0.5.0）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（版本号截图路径）；
   - 打包安装冒烟证据（NSIS x64 + MSI x64 + 便携版 + ARM64 版本号）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（v0.5.0 发版完成；下一阶段：由用户拍板）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务085-发版v0.5.0（UI批量优化+软件感知+大文件地图收官） (#task-085)`，
   该提交只含 `tasks/085-release-v0.5.0.md` 一个文件，落在 `feature/task-085-release-v0.5.0` 分支最前端。
