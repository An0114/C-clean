# 任务编号：054 发版前最后一公里——release.yml 溯源 + 版本号 v0.1.6 + 第三轮复验走查清单 + 升级闭环验证

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact + 测试 Agent + 用户手动发版
> 来源：PROJECT_STATE「下一步计划 ⑥」；走查报告 §6.4。
> 上游依赖：task-053（merge `351704f`，G7/G8 已合并）。
> **本任务范围（5 个功能单元）**：release.yml 溯源、版本号升级、第三轮复验走查清单、升级闭环验证脚本、G6 门禁豁免口径（或可选修复）。
> **本任务是 v0.1.6 发版前最后一个代码/文档任务**；发版动作本身（workflow_dispatch）由用户手动执行，不在本任务内。
> **流程教训（走查报告 §6.4）**：测试替身不能替代生产链路；「数据为空」不等于「功能失效」——本任务的复验清单必须包含「造数据后再验」的条目。

## 状态字段

pending

## 任务目标

### 1. release.yml 补 `target_commitish`（tag 溯源）

- **现状**：`release.yml` 触发后建 tag / Release，但没指定 tag 指向哪个 commit；workflow_dispatch 若在 main 有新 commit 后触发，tag 可能错指到旧 commit。
- **修法**：在 build 步骤后加 `target_commitish: ${{ github.sha }}`（或在 softprops/action-gh-release 步骤加 `commitish: ${{ github.sha }}`），确保 tag 精确指向触发发版的那个 commit。

### 2. 版本号升级 v0.1.5 → v0.1.6

- **现状**：`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`（`version` 字段）三处版本号需同步；
- **修法**：三处统一升到 `0.1.6`，`pnpm version minor` 或手动改；
- **校验**：发版后 NSIS/MSI 安装包文件名含 `0.1.6`，安装后「关于」页显示 `0.1.6`。

### 3. CHANGELOG 草稿（v0.1.6 条目）

- **现状**：仓库无 CHANGELOG.md；
- **修法**：新建 `CHANGELOG.md`，按 Keep a Changelog 格式写 v0.1.6 条目，覆盖：
  - 新增：safe_box 时间筛选 / 多 session / 来源列（task-051）、批量 purge / 恢复（task-038/039）、dry-run 预览（task-040）、updater 自动检查 + Offline 三态 + 非阻塞 banner（task-035/036/042）、占用关闭 confirm 文案（task-049）、items 分页（task-052 G2）；
  - 修复：扫描进度条脚本级换算 + 收尾 100%（task-052 G1 + task-053 G7）、清理列表 100 条截断（task-052 G2）、safe_box.source_task_id 接线（task-052 G3）、清理轮次口径（task-052 G4）、已不存在 vs 已清理语义（task-052 G5）、历史来源文案（task-053 G8）、rules reqwest 超时（task-041）、updater endpoint 改 Releases/latest/download（task-041）；
  - 已知问题：G6 设置页首调 ipc.localhost 红错（Tauri 2.x 已知项，功能经 postMessage 回退可用）。

### 4. 第三轮复验走查清单（NSIS / MSI 各一遍）

- **现状**：前两轮走查都是 dev 窗口，NSIS/MSI 安装态只跑过部分条目；
- **修法**：新建 `docs/test-reports/gui-walkthrough-round3-checklist.md`，按前两轮清单重写一遍，每条加「NSIS 结果 / MSI 结果 / 证据链接」三列；必须覆盖：
  - G1/G7 进度条：连扫 3 次每次停 100%；
  - G2 分页：造 >100 条（含小文件）验证第 2 页可见；
  - G3 safe_box：新删文件入箱 → 来源列 `#N` + Session 下拉 `session #N (M 条)`；
  - G4 轮次：连清 2 次卡片显示 2；
  - G5 already_gone：清理已被外部删除的文件 → 结果条显示「M 项已不存在」；
  - G8 历史来源：老行显示「历史（无来源）」；
  - G6 门禁豁免口径：明确写「排除 G6 已知项后 Console 零红」，G6 本身不阻塞；
  - 占用关闭 confirm 文案含「同一控制台内相关进程可能一并关闭」；
  - updater 自动检查 banner（首次启动后非阻塞出现）。

### 5. 升级闭环验证脚本（装 v0.1.5 → 检查更新 → 装 v0.1.6）

- **现状**：updater 端到端闭环（装旧版 → 检查更新 → 下载安装 → 重启到新版）从未真机跑过；
- **修法**：写一份 `docs/release/v0.1.6-upgrade-verification.md` 操作手册（不是代码），包含：
  - 前置：v0.1.5 安装包已在 GitHub Release（用户手动 workflow_dispatch 发过）；
  - 步骤 1：在干净目录装 v0.1.5；
  - 步骤 2：启动 → 设置页 → 「检查更新」→ 看到 v0.1.6 可更新 banner；
  - 步骤 3：点「立即更新」→ 下载进度（如有）→ 安装 → 重启；
  - 步骤 4：重启后「关于」页显示 v0.1.6；
  - 步骤 5：safe_box / 扫描 / 清理基本功能回归一遍。

**本任务不做**：
- 不做 G6 修复（Tauri 2.x 已知项，本任务只列门禁豁免口径；若 DevAgent 在 1 个提交内能修掉，可选做，不修不阻塞发版）；
- 不做 RmShutdown/RmRestart（立项决策不做）；
- 不做 banner 下载进度反馈 / 差量更新（M3+）；
- **不执行 workflow_dispatch 发版**（用户手动）。

## 前置条件

1. `main` 已合并 task-053（merge `351704f`，G7/G8 全绿）；
2. v0.1.5 已在 GitHub Release（用户 2026-10-23 手动发过，`.exe.sig` keyid `52ACA0C8DAA19EB3` 验证通过）；
3. 关键锚点：
   - `.github/workflows/release.yml`（补 target_commitish）
   - `package.json` / `src-tauri/tauri.conf.json` / `src-tauri/Cargo.toml`（版本号三处）
   - 仓库根（新建 `CHANGELOG.md`）
   - `docs/test-reports/`（新建 round3 checklist）
   - `docs/release/`（新建升级验证手册）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. release.yml 溯源

1. `.github/workflows/release.yml` 加 `target_commitish: ${{ github.sha }}`（或 softprops/action-gh-release 的 `commitish` 参数）；
2. 本地 `act` 或 CI 预览跑过（不强制真发版），能看到 tag 指向正确 commit。

### 2. 版本号升级

3. `package.json` version = `0.1.6`；
4. `src-tauri/tauri.conf.json` version = `0.1.6`；
5. `src-tauri/Cargo.toml` version = `0.1.6`（`src-tauri/Cargo.lock` 由 `cargo build` 自动更新）；
6. `pnpm --filter app build` 成功，产物文件名含 `0.1.6`。

### 3. CHANGELOG

7. `CHANGELOG.md` 存在，含 v0.1.6 条目（Keep a Changelog 格式）；
8. 条目覆盖 task-051/052/053 主要变更（新增 / 修复 / 已知问题 三段）；
9. 不写未来计划（unreleased 段可留空）。

### 4. 第三轮复验走查清单

10. `docs/test-reports/gui-walkthrough-round3-checklist.md` 存在；
11. 清单含 NSIS / MSI 两列结果占位；
12. 清单覆盖 G1/G2/G3/G4/G5/G7/G8 + 占用关闭 + updater banner + Console 零红（排除 G6）；
13. 明确写出门禁口径：「排除 G6 已知项后 Console 零红」。

### 5. 升级闭环验证手册

14. `docs/release/v0.1.6-upgrade-verification.md` 存在；
15. 手册含装 v0.1.5 → 检查更新 → 装 v0.1.6 → 重启 → 关于页显示 v0.1.6 → 基本功能回归 共 6 步；
16. 手册注明「本手册由用户在发版后手动执行，DevAgent 不执行」。

### 测试要求

17. `cargo test --workspace` 全绿（本任务不改业务代码，预期 253+13 条不回归）；
18. `pnpm --filter app lint / test / build` 全绿；
19. CI 五道闸（fmt / clippy / test / lint / build）全绿。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `.github/workflows/release.yml`（加 target_commitish）
- `package.json`（version）
- `src-tauri/tauri.conf.json`（version）
- `src-tauri/Cargo.toml`（version）
- `src-tauri/Cargo.lock`（cargo build 自动更新）
- `CHANGELOG.md`（新建）
- `docs/test-reports/gui-walkthrough-round3-checklist.md`（新建）
- `docs/release/v0.1.6-upgrade-verification.md`（新建）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-054-release-prep-v016`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-054-<描述>`，**只允许一层**；
- 禁止直接 push `main`；
- **本任务合并后**，用户手动 workflow_dispatch 发 v0.1.6（不在本任务 PR 内）。

## Git 提交要求

按 5 个功能单元拆 5 个原子提交：

1. `ci(release): release.yml 补 target_commitish: ${{ github.sha }}，tag 溯源 (#task-054)`（单元 1）
2. `chore(release): 版本号升级 v0.1.5 → v0.1.6（package.json / tauri.conf.json / Cargo.toml） (#task-054)`（单元 2）
3. `docs(changelog): 新增 CHANGELOG.md，v0.1.6 条目覆盖 task-051/052/053 (#task-054)`（单元 3）
4. `docs(test): 新增第三轮复验走查清单（NSIS/MSI 双列） (#task-054)`（单元 4）
5. `docs(release): 新增 v0.1.6 升级闭环验证手册 (#task-054)`（单元 5）
6. 自验全绿后，最后一次提交：`docs(state): 任务054完成，v0.1.6 发版准备就绪，待用户手动 workflow_dispatch (#task-054)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 054: ...`；
   - 「最近一次变更」追加一行，含 5 个单元摘要 + commit hash；
   - 「下一步计划」改为「v0.1.6 发版后由用户执行升级闭环验证手册，跑第三轮复验走查清单；全绿后进入 M3（banner 下载进度 / 差量更新）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-27  task-054  v0.1.6 发版准备（release 溯源 + 版本号 + CHANGELOG + 复验清单 + 升级手册）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0；
- 版本号三处一致 = `0.1.6`（grep 验证）。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：本任务不改业务逻辑，预期现有 253+13 条全绿不回归。
- **集成测试（必做）**：`cargo test --workspace` + `pnpm test` 全绿。
- **端到端（本任务不做）**：升级闭环验证由用户在发版后手动跑（手册已写好）。
- **真机冒烟（本任务不强制）**：DevAgent 只做代码/文档改动，不做真机验证；真机验证由测试 Agent 在第三轮走查清单中执行。
- **打包安装冒烟（必做、不可裁剪）**：本任务合并后由测试 Agent 在 NSIS/MSI 干净目录各跑一遍第三轮清单（DevAgent 不直接做，但必须在 PR 描述里明确「测试 Agent 接管 NSIS/MSI 双跑」）。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务主体是配置/文档/版本号变更，不直接命中「前端可观察行为」，但合并后立即触发 NSIS/MSI 打包，打包安装冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| release.yml 加 target_commitish | 文件 diff + CI run 中 tag 指向正确 commit 的日志 |
| 版本号三处一致 | `grep -r "0.1.6" package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml` 输出 |
| 产物文件名含 0.1.6 | `pnpm build` 输出 + `src-tauri/target/release/bundle/` 目录列表 |
| CHANGELOG | 文件内容预览（v0.1.6 三段齐全） |
| 第三轮走查清单 | 文件存在 + 10 条条目齐全 |
| 升级闭环手册 | 文件存在 + 6 步齐全 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿输出 |
| CI | PR URL + run id + conclusion=success |
| NSIS/MSI 打包冒烟 | （测试 Agent 在第三轮走查中补）NSIS / MSI 安装路径 + 清单结果 |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 5 个代码/文档提交 + 1 个 `docs(state)` 提交，共 6 个 commit，全部 push 到 `feature/task-054-release-prep-v016`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - release.yml 最终 diff（target_commitish 加在哪一步）；
   - 版本号三处改动 diff；
   - CHANGELOG v0.1.6 条目摘要；
   - 第三轮走查清单条目数；
   - 升级闭环手册步骤数；
   - `cargo test --workspace` / `pnpm build` 通过输出；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（G6 设置页 IPC 红错仍为已知项，门禁口径「排除 G6 后零红」；升级闭环验证由用户发版后手动跑）；
3. `docs/PROJECT_STATE.md` 已按第 6 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务054-v0.1.6 发版准备 (#task-054)`，
   该提交只含 `tasks/054-release-prep-v016.md` 一个文件，落在 `feature/task-054-release-prep-v016` 分支最前端。
