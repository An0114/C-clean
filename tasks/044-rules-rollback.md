# 任务编号：044 rules 手动回滚命令——从 .bak 恢复上一版 cache_rules.json + 前端回滚按钮

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：`docs/PROJECT_STATE.md` 已知问题（task-005 段）——「applyUpdate 已备份替换但**手动回滚命令（从 .bak 恢复）** 仍未做」。用户更新规则后发现新版本误删/加严规则时，需要一键回退到上一版。
> 上游依赖：task-023（`rules.applyUpdate` 已做备份替换 + .bak 落盘）、task-031（Settings 规则库卡片）、task-041（reqwest 超时）、task-042（Offline 三态）。
> **克制范围**：只做单版回滚（.bak → cache_rules.json），**不做**多版本历史/差量更新/UI「确认更新」弹窗（留 task-045+）。

## 状态字段

pending

## 任务目标

task-023 `rules.applyUpdate` 逻辑（推断）：
- 下载新 cache_rules.json → SHA-256 校验；
- 把当前 cache_rules.json 备份为 `cache_rules.json.bak`；
- 写入新 cache_rules.json；
- upsert rules_meta 到 DB。

缺的能力：用户发现新版规则有问题时，没法一键退回上一版。

本任务补：

1. **后端**新增 `rules_rollback() -> RollbackResult`：
   ```rust
   pub struct RollbackResult {
       pub previous_version: String,   // .bak 里的 rules_meta version
       pub restored_sha256: String,
       pub message: String,
   }
   ```
   - 检查 app_data_dir 下 `cache_rules.json.bak` 是否存在；
   - 不存在 → 返回错误「没有可回滚的备份（首次更新前）」；
   - 存在 → 读 .bak 的 SHA-256 + version；
   - 校验 .bak 文件完整性（格式合法）；
   - 把 .bak 内容覆盖写回 `cache_rules.json`；
   - 重新加载 rules 到 engine 内存；
   - upsert DB rules_meta；
   - 写 audit_log action='rules_rollback'。
2. **后端**新增 `rules_has_backup() -> bool`：前端用来决定是否显示「回滚」按钮。
3. **前端** Settings 规则库卡片：
   - 启动时调 `rules_has_backup()`；
   - true 时在「应用更新」按钮下方显示灰色「回滚到上一版」按钮；
   - 点击 → confirm「确定回滚到上一版规则库？」；
   - confirm → invoke `rules_rollback()` → 绿条「已回滚到 vX.Y.Z」；
   - 回滚后按钮自动隐藏（因为 cache_rules.json 已经是 .bak 的内容，.bak 还是同一文件——其实仍然存在，DevAgent 按实际行为决定是否隐藏）。

**本任务不做**：
- 不做多版本历史（只保留单版 .bak，task-023 已固定）；
- 不做差量更新；
- 不做「确认更新」弹窗（applyUpdate 直接执行，已是现状）；
- 不改 applyUpdate 逻辑。

## 前置条件

1. `main` 分支已含 task-001~043 全部产出；
2. 关键锚点：
   - `engine/src/rules/remote.rs`（task-023 applyUpdate，.bak 落盘路径）；
   - `engine/src/rules/loader.rs`（SHA-256 校验 + 加载）；
   - `app/src-tauri/src/commands.rs`（rules 命令已注册）；
   - `app/src/pages/Settings.tsx`（task-031/041/042 规则库卡片）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. 后端：rules_rollback + rules_has_backup

1. 在 `engine/src/rules/` 下新增 `rollback.rs`（或在 remote.rs 加函数）：
   ```rust
   pub fn rollback(app_data_dir: &Path) -> Result<RollbackResult, String>;
   pub fn has_backup(app_data_dir: &Path) -> bool;
   ```
2. `rollback` 逻辑：
   - `.bak` 不存在 → `Err("没有可回滚的备份（首次更新前）")`；
   - 读 .bak 文本 → serde_json 解析 rules 模型（格式不合法 → `Err("备份文件损坏")`）；
   - 计算 .bak 的 SHA-256；
   - 把 .bak 内容复制覆盖到 `cache_rules.json`；
   - 重新加载 engine 内存 rules 集；
   - upsert rules_meta 到 DB；
   - 写 audit_log。
3. Tauri 命令 `rules_rollback` + `rules_has_backup` 注册到 `commands.rs` / `lib.rs`。

### B. 后端单测

4. 在 `engine/tests/rules_rollback.rs` 覆盖：
   - `rollback_restores_from_bak`：tempdir 造 `cache_rules.json`（v1）+ `cache_rules.json.bak`（v0 内容）→ rollback → cache_rules.json 内容变成 v0；
   - `rollback_no_bak_returns_err`：无 .bak → Err「没有可回滚的备份」；
   - `rollback_corrupted_bak_returns_err`：.bak 写垃圾 JSON → Err「备份文件损坏」；
   - `has_backup_true_when_bak_exists` / `has_backup_false_when_absent`。

### C. 前端：Settings 回滚按钮

5. `app/src/pages/Settings.tsx`：
   - useEffect 调 `rules_has_backup()`，state `hasBak: boolean`；
   - hasBak=true 时规则库卡片加灰色「回滚到上一版」按钮；
   - 点击 → `window.confirm("确定回滚到上一版规则库？")`；
   - confirm → invoke `rules_rollback()` → 绿条「已回滚到 v<previous_version>」；
   - 失败红条显示 err。

### D. 卫生

6. 不改 applyUpdate 逻辑；
7. 不改 Offline/UpToDate/NewVersion 三态渲染（task-042 已就位）；
8. .bak 路径常量与 applyUpdate 复用同一常量，不硬编码两处。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/rules/remote.rs`（或新建 `rollback.rs`）
- `engine/src/rules/loader.rs`（如需导出重新加载函数）
- `engine/tests/rules_rollback.rs`（新建）
- `app/src-tauri/src/commands.rs`（新增 rollback / has_backup）
- `app/src-tauri/src/lib.rs`（注册命令）
- `app/src/pages/Settings.tsx`（加回滚按钮）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-044-rules-rollback`，从最新 `main` 拉出。
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-044-<描述>`，**只允许一层**；bugfix 修复后直接合并回 feature，不进 main。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `feat(rules): 新增 rollback + has_backup——从 .bak 恢复 cache_rules.json (#task-044)`
   —— 含 `engine/src/rules/remote.rs`（或 rollback.rs）、`engine/src/rules/loader.rs`。
2. `feat(tauri): 新增 rules_rollback / rules_has_backup 命令并注册 (#task-044)`
   —— 含 `app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`。
3. `test(rules): 补 rollback 集成测试（有 bak/无 bak/损坏 bak/has_backup） (#task-044)`
   —— 含 `engine/tests/rules_rollback.rs`。
4. `feat(frontend): Settings 规则库卡片加「回滚到上一版」按钮 (#task-044)`
   —— 含 `app/src/pages/Settings.tsx`。
5. 自验全绿后，最后一次提交：`docs(state): 任务044完成，rules 手动回滚命令就绪 (#task-044)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 044: ...`；
   - 「最近一次变更」追加一行；
   - 「已知问题」段 task-005 那条把「手动回滚命令（从 .bak 恢复）」划掉；
   - 「下一步计划」改为「派发 task-045：safe_box list 分页（LIMIT/OFFSET + 前端分页控件）——task-018 已知问题闭环」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-16  task-044  rules_rollback 单版回滚（后端命令 + 前端灰色按钮 + confirm）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 159+ 条 + 本任务 5 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：`rules_rollback.rs` 4 条覆盖有 bak/无 bak/损坏 bak/has_backup。
- **集成测试（必做）**：`cargo test -p engine` 全绿，确保现有不回归。
- **端到端（不强制）**：本任务不改网络/更新流程，纯本地文件操作。
- **真机冒烟（必做、不可裁剪）**：
  - 先 applyUpdate 一次（制造 .bak）；
  - 再点「回滚到上一版」→ 绿条「已回滚到 v<previous>」；
  - 回滚后规则库版本号对应回上一版；
  - CDP 抓 console 0 红错。
- **打包安装冒烟（必做、不可裁剪）**：
  - NSIS 干净目录安装 → Settings 规则库卡片回滚按钮在有 .bak 时显示、无 .bak 时隐藏。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（Settings.tsx 新增按钮 + 真机可观察行为），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| A1-A3 rollback / has_backup 逻辑 | `cargo test -p engine` 输出（含 rules_rollback 用例 + passed） |
| B1-B4 后端单测 | `cargo test rules_rollback -- --nocapture` 完整输出 |
| C5 前端回滚按钮三态 | CDP 截图：有 bak 显示按钮 / 无 bak 隐藏 / 回滚后绿条文案 |
| D6-D8 不回归 | `cargo test -p engine` 全绿 + `pnpm test` 全绿 |
| 真机冒烟 | CDP `http://localhost:9222/json` 抓 page + consoleAPICalled 0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 回滚按钮显示/隐藏证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-044-rules-rollback`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - .bak 文件路径最终格式（与 applyUpdate 复用常量）；
   - `RollbackResult` 结构最终定义；
   - `cargo test -p engine` 通过输出；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（CDP 截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（多版本历史 / 差量更新 / 确认更新弹窗留 task-045+）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务044-rules 手动回滚 (#task-044)`，
   该提交只含 `tasks/044-rules-rollback.md` 一个文件，落在 `feature/task-044-rules-rollback` 分支最前端。
