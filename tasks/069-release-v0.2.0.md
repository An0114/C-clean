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

pending

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
