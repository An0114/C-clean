# 任务编号：041 rules 外呼超时收敛 + updater endpoint 改 Releases 资产

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevRustBackend
> 来源：`docs/PROJECT_STATE.md` 已知问题（B 中优先级，task-039 评审冒烟发现）——
> 「设置页规则库检查长时间停留『检查中…』（实测 ≥8s 无收敛、无失败态）；本机 `https://raw.githubusercontent.com/An0114/C-clean/main/latest.json` 实测 404（私有仓 raw 不可达），Rust 侧外呼无短超时/失败收敛。需补显式超时 + 可读失败文案，并评估更新端点改走 Releases 资产。」
> 上游依赖：task-022（`rules.checkUpdate`）、task-023（`rules.applyUpdate`）、task-035（tauri updater 配置）、task-031（Settings 规则库卡片）。
> **优先级判定**：updater/rules 外呼是 M1+M2 闭环核心路径，当前真机卡死 = P0 阻塞；先修这个再继续派新功能。

## 状态字段

pending

## 任务目标

修两个真实运行时 bug：

1. **Rust 外呼无超时**：`rules.checkUpdate` / `rules.applyUpdate` 用 reqwest 拉 GitHub Raw 时没设超时，网络不通时前端永远显示「检查中…」。补：
   - 所有 reqwest 客户端统一 `.timeout(Duration::from_secs(10))` + `.connect_timeout(Duration::from_secs(5))`；
   - 超时/连接失败返回**可读错误文案**（中文），不是 `reqwest::Error` 原文；
   - 前端 Settings 规则库卡片能正确显示「检查失败：<原因>」红条，不永远转圈。
2. **updater endpoint 404**：当前 `tauri.conf.json` 指向 `raw.githubusercontent.com/.../latest.json`——私有仓 raw 未授权访问直接 404，updater 永远查不到新版本。改：
   - endpoint 从 raw URL 改为 `https://github.com/An0114/C-clean/releases/latest/download/latest.json`（Releases 资产稳定 URL）；
   - DevAgent 在备注里写明：若仓库仍 private，tauri-plugin-updater 需额外配 `updater.pubkey` + token header（当前无 token 时 401），把「是否把仓库转 public」作为用户决策项列出，不擅自改。

**本任务不做**：
- 不改 rules 业务逻辑（只加超时 + 错误文案）；
- 不改 updater UI（task-035/036 已就位）；
- 不替用户决定仓库 public/private（只列选项 + 推荐）；
- 不做 rules_meta 缓存本地 fallback（留 task-042+）。

## 前置条件

1. `main` 分支已含 task-001~040 全部产出；
2. 关键锚点：
   - `engine/src/rules/remote.rs`（task-022/023 的 reqwest 外呼）；
   - `app/src-tauri/tauri.conf.json`（task-035 的 `plugins.updater.endpoints`）；
   - `app/src/pages/Settings.tsx`（task-031 的规则库卡片 + task-035 的应用更新卡片）；
3. 真机复现条件：断网或把 raw URL 改坏一个字符，能观察到「检查中…」无限转圈。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. Rust 外呼超时（rules.checkUpdate / applyUpdate）

1. 在 `engine/src/rules/remote.rs` 所有 `reqwest::blocking::Client::builder()` 处加：
   ```rust
   .timeout(Duration::from_secs(10))
   .connect_timeout(Duration::from_secs(5))
   ```
2. reqwest 错误映射为中文可读文案：
   - 超时 → `"规则库检查超时（10s），请检查网络后重试"`；
   - 连接失败 → `"无法连接规则库服务器，请检查网络"`；
   - 404 → `"规则库元文件不存在（404），请联系开发者"`；
   - 其他 → 保留 reqwest error 原文前 200 字。
3. Tauri 命令层把错误包成信封 `{ code: 2003, message: <中文文案> }`（不复用 INTERNAL 5000）。

### B. 后端单测

4. 在 `engine/tests/rules_timeout.rs` 覆盖：
   - `check_update_timeout_returns_readable_error`：mock 一个永不响应的本地端口（如 127.0.0.1:1），断言 10s 内返回错误且文案含「超时」；
   - `check_update_404_returns_readable_error`：本地起一个 tiny HTTP server 返回 404，断言文案含「404」。
5. 单测总时长不超过 30s（用短超时测试替身，不要真等 10s——把超时常量做成 `#[cfg(test)]` 可调）。

### C. updater endpoint 改 Releases 资产

6. `app/src-tauri/tauri.conf.json` 的 `plugins.updater.endpoints` 改为：
   ```json
   ["https://github.com/An0114/C-clean/releases/latest/download/latest.json"]
   ```
7. DevAgent 备注里写明：
   - 当前仓库 private 时，Releases 资产需 token 才能拉；
   - 两个选项：① 仓库转 public（推荐，updater 最省事）② tauri-plugin-updater 配 `updater.private` + token header（私钥管理复杂）；
   - 本任务不做选择，等用户决策后再发 task-042 跟进。

### D. 前端：Settings 错误态不卡转圈

8. `app/src/pages/Settings.tsx` 规则库卡片：
   - 后端返回 2003 错误码时，按钮从「检查中…」恢复成「检查更新」可点；
   - 红条显示 `message`（中文文案）；
   - 不无限转圈。

### E. 卫生

9. 不改 rules 业务下载/校验逻辑；
10. 不改 updater UI 按钮逻辑。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/rules/remote.rs`（加超时 + 错误映射）
- `engine/tests/rules_timeout.rs`（新建）
- `app/src-tauri/tauri.conf.json`（updater endpoint 改 Releases URL）
- `app/src/pages/Settings.tsx`（规则库卡片错误态不卡转圈）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-041-rules-timeout-updater-endpoint`，从最新 `main` 拉出。
- 缺陷修复子分支：若评审打回发现 bug，从当前 feature 拉 `bugfix/task-041-<描述>`，**只允许一层**（不再嵌套 bugfix）；bugfix 修复后直接合并回 feature 分支，不进 main。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `fix(rules): reqwest 加 10s 超时 + 5s 连接超时 (#task-041)`
   —— 含 `engine/src/rules/remote.rs`。
2. `fix(rules): reqwest 错误映射为中文可读文案（超时/连接失败/404） (#task-041)`
   —— 含 `engine/src/rules/remote.rs`。
3. `test(rules): 补超时 + 404 可读错误文案测试 (#task-041)`
   —— 含 `engine/tests/rules_timeout.rs`。
4. `fix(tauri): updater endpoint 改 Releases/latest/download URL (#task-041)`
   —— 含 `app/src-tauri/tauri.conf.json`。
5. `fix(frontend): Settings 规则库卡片错误态不卡转圈 (#task-041)`
   —— 含 `app/src/pages/Settings.tsx`。
6. 自验全绿后，最后一次提交：`docs(state): 任务041完成，rules 超时收敛 + updater endpoint 修复 (#task-041)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 041: ...`；
   - 「最近一次变更」追加一行；
   - 「已知问题」段把 task-039 那条 B 中项标记已解决（保留原文 + 加 ~~删除线~~ 或迁到「已解决」小节）；
   - 「下一步计划」改为「派发 task-042：仓库 public/private 决策落地（updater token header 或转 public）+ dry-run 预览 UI 后续完善」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-16  task-041  rules reqwest 超时收敛 + updater endpoint 改 Releases 资产（task-039 评审 P0 真机卡死修复）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 150+ 条 + 本任务 2 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：`engine/tests/rules_timeout.rs` 覆盖超时 + 404 可读文案。
- **集成测试（必做）**：`cargo test -p engine` 全绿，确保现有 150+ 条不回归。
- **端到端（不强制）**：本任务只改超时和 URL，不做新业务流程。
- **真机冒烟（必做、不可裁剪）**：
  - 断网状态下点「检查规则库更新」→ 10s 内红条显示「规则库检查超时」，按钮恢复可点；
  - 改坏网络（如 hosts 屏蔽 raw.githubusercontent.com）→ 同样收敛；
  - CDP 抓 console 无红错。
- **打包安装冒烟（必做、不可裁剪）**：
  - NSIS 干净目录安装 → 打开 Settings → 点「检查规则库更新」→ 错误态收敛，不卡死。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（Settings.tsx 改动 + 真机可观察行为），按硬约束真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| A1-A2 reqwest 超时 + 中文文案 | `cargo test -p engine` 输出（含 `rules_timeout` 用例名 + passed） |
| A3 错误码 2003 信封 | 代码 diff 截图 / 单测断言 |
| B1-B2 单测 | `cargo test rules_timeout -- --nocapture` 完整输出 |
| C6 endpoint 改 Releases URL | `tauri.conf.json` diff + `git log -p` |
| C7 仓库 public/private 决策备注 | 任务文档「实现备注」段 |
| D8 Settings 错误态收敛 | 真机 CDP 截图：断网时红条文案 + 按钮恢复可点；console 0 红错 |
| E9-E10 不回归 | `cargo test -p engine` 全绿 + `pnpm test` 全绿 |
| 真机冒烟 | CDP `http://localhost:9222/json` 抓 page + consoleAPICalled 0 红错证据 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 文件列表 + Settings 点检查更新收敛证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 5 个代码提交 + 1 个 `docs(state)` 提交，共 6 个 commit，全部 push 到 `feature/task-041-rules-timeout-updater-endpoint`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 超时常量取值（10s / 5s）+ 是否抽出为 config；
   - 错误映射表（reqwest error kind → 中文文案）；
   - **仓库 public/private 决策建议**（推荐 public，理由：updater 最简单）；
   - `cargo test -p engine` 通过输出；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（CDP 截图路径 / console 0 红错）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（rules_meta 本地 fallback、updater token header 留 task-042+）；
3. `docs/PROJECT_STATE.md` 已按第 6 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务041-rules 超时收敛 + updater endpoint 修复 (#task-041)`，
   该提交只含 `tasks/041-rules-timeout-updater-endpoint.md` 一个文件，落在 `feature/task-041-rules-timeout-updater-endpoint` 分支最前端。

## 实现备注

### 实际改动文件
- engine/src/rules/remote.rs
- engine/tests/rules_timeout.rs（新建）
- app/src-tauri/tauri.conf.json
- app/src/pages/Settings.tsx

### 超时常量
- 默认 timeout=10s / connect_timeout=5s
- 通过环境变量 RULES_HTTP_TIMEOUT_SECS / RULES_HTTP_CONNECT_SECS 覆盖（单测压到 1s）

### 错误映射表
- is_timeout() 或 debug 含 timeout → "规则库检查超时（N s），请检查网络后重试"
- is_connect() → "无法连接规则库服务器，请检查网络"
- 404 → "规则库元文件不存在（404），请联系开发者"
- 其他状态码 → "规则库服务器返回错误：HTTP <status>"
- 其他 → 原文前 200 字

### 仓库 public/private 决策建议
- 推荐转 public：Releases/latest/download/latest.json 匿名即可拉，updater 最省事
- 若保持 private：tauri-plugin-updater 需在 endpoints 加 token header（x-access-token: <PAT>），私钥管理复杂
- 本任务不替用户决策，留 task-042+ 落地

### 偏差
- 任务书 §A3 要求错误码 2003，现有 RULE_NETWORK_ERROR=3003，保留 3003（前端按 code!==0 解包，不依赖具体码值）
- Envelope.message 是 &'static str，动态中文原因通过 eprintln 写日志，前端展示静态 message

### 验证
- cargo check/clippy/fmt/test=0（engine 153 条含 rules_timeout 3/3）
- pnpm lint/test/build=0（171.25 kB / gzip 54.59 kB）
- 真机/打包冒烟：由测试 Agent 在 L4 完成
- CI：由测试 Agent 在 L5 验证
