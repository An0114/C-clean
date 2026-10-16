# 任务编号：047 rules applyUpdate 前端确认弹窗——点「应用更新」先 confirm 再调命令

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFrontendReact
> 来源：`docs/PROJECT_STATE.md` 已知问题（task-005 段）——「前端 UI 『确认更新』弹窗」仍未做。当前 Settings 规则库卡片点「应用更新」直接 invoke `rules_apply_update`，万一手滑或新版本规则有问题直接覆盖本地 cache_rules.json，无兜底确认。
> 上游依赖：task-023（`rules.applyUpdate` 后端已就位）、task-031（Settings 规则库卡片）、task-042（Offline 三态，newVersion 蓝条时才显示应用更新按钮）。
> **克制范围**：只加 confirm 弹窗，**不做**更新进度条/差量更新/回滚自动提示（task-044 已做手动回滚）。

## 状态字段

pending

## 任务目标

当前 Settings 规则库卡片在 `newVersion` 状态下显示「应用更新」按钮，点击直接 invoke `rules_apply_update`。补 confirm 弹窗：

1. 点「应用更新」→ `window.confirm(...)`：
   ```
   即将应用规则库 vX.Y.Z（当前本地 vA.B.C）。
   应用后旧版将备份为 cache_rules.json.bak，可随时通过「回滚到上一版」恢复。
   是否继续？
   ```
2. 用户点「取消」→ 不调命令，按钮恢复可点；
3. 用户点「确定」→ 才 invoke `rules_apply_update`；
4. 更新成功后按钮状态自动切回 UpToDate 绿条（task-042 已就位）。

**本任务不做**：
- 不做自定义 modal（用 `window.confirm` 最简实现，Tauri WebView2 原生支持）；
- 不做更新进度条（applyUpdate 是同步下载+校验，秒级完成）；
- 不做「不再提示」记忆；
- 不改后端 applyUpdate 逻辑。

## 前置条件

1. `main` 分支已含 task-001~046 全部产出；
2. 关键锚点：
   - `app/src/pages/Settings.tsx`（task-031/042 规则库卡片，newVersion 蓝条时显示应用更新按钮）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. 前端：applyUpdate 前 confirm

1. 在 `app/src/pages/Settings.tsx` 的 `handleApplyUpdate`（或等效函数）里：
   ```ts
   async function handleApplyUpdate() {
     const ok = window.confirm(
       `即将应用规则库 v${updateInfo.remoteVersion}（当前本地 v${updateInfo.localVersion}）。\n` +
       `应用后旧版将备份为 cache_rules.json.bak，可随时通过「回滚到上一版」恢复。\n` +
       `是否继续？`
     );
     if (!ok) return;
     // 原有 invoke('rules_apply_update') 逻辑
   }
   ```
2. 文案里嵌入实际版本号（从 CheckUpdateResult 取 localVersion / remoteVersion）。

### B. 卫生

3. 不改后端任何代码；
4. 不改 checkUpdate / rollback 按钮逻辑；
5. 不加 npm 依赖（`window.confirm` 原生）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `app/src/pages/Settings.tsx`（handleApplyUpdate 前加 confirm）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-047-apply-update-confirm`，从最新 `main` 拉出。
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-047-<描述>`，**只允许一层**；bugfix 修复后直接合并回 feature，不进 main。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(frontend): Settings 点「应用更新」前 window.confirm 嵌入版本号 (#task-047)`
   —— 含 `app/src/pages/Settings.tsx`。
2. 自验全绿后，最后一次提交：`docs(state): 任务047完成，rules applyUpdate 前端确认弹窗就绪 (#task-047)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 047: ...`；
   - 「最近一次变更」追加一行；
   - 「已知问题」段 task-005 那条把「前端 UI 『确认更新』弹窗」划掉；
   - 「下一步计划」改为「派发 task-048：v0.1.1 真机升级验证收尾——用户在 v0.1.0 机器上启动应用观察 updater banner，验证后正式关闭 M2；后续 safe_box 时间范围筛选 / 多选 session / audit_log 联表 留 task-049+」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-16  task-047  rules.applyUpdate 前端 window.confirm 弹窗（嵌入版本号 + 回滚提示）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0（后端零改动）；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（170+ 条全绿，不回归）；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（不强制）**：本任务只加 `window.confirm` 一行，无新纯函数可测；前端 Vitest 不 mock window.confirm。
- **集成测试（必做）**：`cargo test -p engine` 全绿，确保后端不回归。
- **端到端（不强制）**：本任务无新业务流程。
- **真机冒烟（必做、不可裁剪）**：
  - 造一个「有新版本」状态（手动改 DB rules_meta 让 remoteVersion > localVersion）；
  - 点「应用更新」→ 弹原生 confirm 对话框，文案含「vX.Y.Z」「vA.B.C」「回滚」；
  - 点「取消」→ 不调后端（CDP 抓 network/IPC 无 rules_apply_update 请求）；
  - 再点「应用更新」→ 点「确定」→ 调后端，更新成功；
  - CDP 抓 console 0 红错。
- **打包安装冒烟（必做、不可裁剪）**：
  - NSIS 干净目录安装 → Settings 规则库卡片：默认 UpToDate 状态不弹 confirm；有新版本时弹 confirm。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（Settings.tsx 改动 + 真机可观察行为），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| A1-A2 confirm 弹窗 + 版本号嵌入 | CDP 截图：confirm 对话框文案含 localVersion/remoteVersion/回滚提示 |
| B3-B5 不回归 | `cargo test -p engine` 全绿 + `pnpm test` 全绿 |
| 真机冒烟 | CDP 抓点「取消」无 IPC 请求 + 点「确定」成功更新；console 0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + UpToDate 状态不弹 confirm 证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 1 个代码提交 + 1 个 `docs(state)` 提交，共 2 个 commit，全部 push 到 `feature/task-047-apply-update-confirm`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（应只有 Settings.tsx 一处）；
   - confirm 文案最终版本；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（CDP 截图路径：confirm 对话框 / 取消无 IPC / 确定成功）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（差量更新 / 更新进度条 留 task-048+）；
3. `docs/PROJECT_STATE.md` 已按第 2 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务047-applyUpdate 确认弹窗 (#task-047)`，
   该提交只含 `tasks/047-apply-update-confirm.md` 一个文件，落在 `feature/task-047-apply-update-confirm` 分支最前端。
