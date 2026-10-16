# 任务编号：042 rules 离线模式——checkUpdate 失败不影响本地规则库，UI 显示离线态

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevRustBackend
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-041 合并后写入）——task-041 已修 reqwest 超时（10s 收敛报错），但网络不通时用户仍被「检查更新失败」挡住，无法继续清理。本任务补降级：checkUpdate 失败 = 离线模式，本地 bundled cache_rules.json 照常工作。
> 上游依赖：task-011（外部红线集注入 + ps1 兼容层）、task-022（`rules.checkUpdate`）、task-023（`rules.applyUpdate`）、task-031（Settings 规则库卡片）、task-041（reqwest 超时 + 中文错误文案）。
> **克制范围**：只做「离线模式降级」，**不做** updater token header / 仓库转 public（用户决策，留 task-043）、**不做** rules 自动缓存热更新。

## 状态字段

pending

## 任务目标

当前 bug：用户断网或 GitHub 不可达时，Settings 规则库卡片显示「检查失败：超时」，用户以为规则库没了/不能用。实际本地 bundled `cache_rules.json` 一直能用（task-033 NSIS 已 bundle 进 resources），只是 UI 没说清楚。

本任务补最小降级：

1. **后端**：`rules.checkUpdate` 在 reqwest 失败（超时/404/连接失败）时，**不返回错误**，而是返回一个特殊结果：
   ```rust
   pub struct CheckUpdateResult {
       pub status: CheckStatus,        // UpToDate / NewVersion { version } / Offline
       pub local_version: String,       // 本地当前版本
       pub remote_version: Option<String>,
       pub message: String,             // 中文文案
   }
   ```
   - `Offline`：reqwest 错误时返回，前端据此显示「离线模式，使用本地规则库 vX.Y.Z」；
   - 不影响后续 scan/cleanup（本地规则库已加载）。
2. **前端** Settings 规则库卡片：
   - `Offline` 状态 → 黄色提示条「离线模式：使用本地规则库 vX.Y.Z（<message>）」；
   - 按钮恢复为「检查更新」可点（task-041 已做）；
   - 不报错红条，不卡死。

**本任务不做**：
- 不做 updater token header / 仓库转 public（用户决策项）；
- 不做 rules 自动后台轮询；
- 不做 rules_meta 缓存到 DB 的复杂降级（task-011 已注入 DB，本任务只改 checkUpdate 返回结构）。

## 前置条件

1. `main` 分支已含 task-001~041 全部产出；
2. 关键锚点：
   - `engine/src/rules/remote.rs`（task-022/023 reqwest 外呼，task-041 已加超时）；
   - `engine/src/ipc/commands/` 下 rules 命令的返回形状；
   - `app/src/pages/Settings.tsx`（task-031 规则库卡片）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. 后端：CheckUpdateResult 加 Offline 分支

1. 在 `engine/src/rules/remote.rs`（或 `engine/src/ipc/commands/` 下 rules 命令处）把 checkUpdate 返回类型改为 `CheckUpdateResult`：
   - 远程可访问 + 本地最新 → `status=UpToDate, message="规则库已是最新"`；
   - 远程可访问 + 有新版本 → `status=NewVersion { version }, message="发现新版本 vX.Y.Z"`；
   - reqwest 错误（含超时/404/连接失败）→ `status=Offline, message="<task-041 的中文错误文案>"`；
2. Tauri 命令 `rules_checkUpdate` 返回 shape 同步改（前端 camelCase 对齐）。

### B. 后端单测

3. 在 `engine/tests/rules_offline_mode.rs` 覆盖：
   - `check_update_offline_when_request_fails`：mock 一个不可达端口，断言返回 `status=Offline, local_version=非空, remote_version=None`；
   - `check_update_uptodate_when_mock_server_returns_same_version`：本地起 tiny HTTP server 返回相同 version，断言 `status=UpToDate`；
   - `check_update_new_version_when_mock_server_returns_higher`：返回更高 version，断言 `status=NewVersion{version}`。

### C. 前端：Settings 规则库卡片离线态

4. `app/src/pages/Settings.tsx`：
   - 根据 `status` 渲染：
     - `UpToDate` → 绿条「规则库已是最新（vX.Y.Z）」；
     - `NewVersion` → 蓝条「发现新版本 vX.Y.Z，点『应用更新』」；
     - `Offline` → 黄条「离线模式：使用本地规则库 vX.Y.Z（<message>）」；
   - 按钮始终可点（不卡转圈，task-041 已修）。

### D. 卫生

5. 不改 scan/cleanup 业务逻辑（本地规则库加载路径不变）；
6. 不改 applyUpdate 命令（远程新版本应用流程不变）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/rules/remote.rs`（checkUpdate 返回结构）
- `engine/src/ipc/commands/` 下 rules 相关命令（返回 shape）
- `engine/tests/rules_offline_mode.rs`（新建）
- `app/src/pages/Settings.tsx`（规则库卡片离线态渲染）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-042-rules-offline-mode`，从最新 `main` 拉出。
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-042-<描述>`，**只允许一层**；bugfix 修复后直接合并回 feature，不进 main。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(rules): checkUpdate 返回 CheckUpdateResult 加 Offline 分支 (#task-042)`
   —— 含 `engine/src/rules/remote.rs`、`engine/src/ipc/commands/` 相关文件。
2. `test(rules): 补离线模式 / UpToDate / NewVersion 三分支单测 (#task-042)`
   —— 含 `engine/tests/rules_offline_mode.rs`。
3. `feat(frontend): Settings 规则库卡片按 status 渲染三态（绿/蓝/黄） (#task-042)`
   —— 含 `app/src/pages/Settings.tsx`。
4. 自验全绿后，最后一次提交：`docs(state): 任务042完成，rules 离线模式降级就绪 (#task-042)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 042: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-043：等用户决定仓库 public/private 后落地 updater token header（或转 public），验证 v0.1.1 真实升级流程」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-16  task-042  rules.checkUpdate 加 Offline 三态，断网时本地规则库照常工作 + UI 黄条提示`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 150+ 条 + 本任务 3 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：`rules_offline_mode.rs` 三分支（Offline / UpToDate / NewVersion）。
- **集成测试（必做）**：`cargo test -p engine` 全绿，确保现有不回归。
- **端到端（不强制）**：本任务只改返回结构和 UI 渲染，不做新业务流程。
- **真机冒烟（必做、不可裁剪）**：
  - 断网或 hosts 屏蔽 github.com → 打开 Settings → 点「检查更新」→ 10s 内黄条显示「离线模式：使用本地规则库 vX.Y.Z」；
  - 恢复网络 → 再点 → 绿条或蓝条；
  - CDP 抓 console 0 红错。
- **打包安装冒烟（必做、不可裁剪）**：
  - NSIS 干净目录安装 → 断网 → Settings 显示离线黄条；本地规则库 vX.Y.Z 文案正确。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（Settings.tsx 三态渲染 + 真机可观察行为），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| A1-A2 CheckUpdateResult 三分支 | `cargo test -p engine` 输出（含 rules_offline_mode 用例 + passed） |
| B1-B3 三分支单测 | `cargo test rules_offline_mode -- --nocapture` 完整输出 |
| C4 前端三态渲染 | CDP 截图：断网黄条 / 联网绿条 / 有新版本蓝条 |
| D5-D6 不回归 | `cargo test -p engine` 全绿 + `pnpm test` 全绿 |
| 真机冒烟 | CDP `http://localhost:9222/json` 抓 page + consoleAPICalled 0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 断网时 Settings 黄条证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-042-rules-offline-mode`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - `CheckUpdateResult` 结构最终定义（字段名 + 类型）；
   - `cargo test -p engine` 通过输出；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（CDP 截图路径 / 断网黄条文案）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（updater token header / 仓库 public/private 留 task-043+）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务042-rules 离线模式 (#task-042)`，
   该提交只含 `tasks/042-rules-offline-mode.md` 一个文件，落在 `feature/task-042-rules-offline-mode` 分支最前端。
