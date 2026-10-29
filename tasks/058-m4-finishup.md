# 任务编号：058 M4 收尾——RmRestart 白名单 + safe_box 原子性 + 代码签名评估 + v0.1.7 CHANGELOG 草稿

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：PROJECT_STATE「下一步计划 5. 后续按需迭代」+「已知问题」段 B 中/B 低遗留。
> 上游依赖：task-057 已合并 main（merge `e9f0e7a`，RmRestart 自动重启 + 设置开关已上线，P0 flags 误用已修）。
> **本任务范围（4 个功能单元，厚任务）**：RmRestart 白名单 UI、safe_box 原子性回滚、代码签名技术评估文档、v0.1.7 CHANGELOG 草稿。
> **为什么把这些合并**：都是 M3/M4 闭环后的「收尾小修 + 研究 + 文档」，低风险且互相不耦合；按新提示词「同一模块/同一阶段收尾的事项合并派发」原则，一次 PR 清掉。
> **不做**：差量更新（已决策不立项）、release profile 瘦身（已决策不做）、G6 修复（已豁免）、品牌图标（已决策不做）。

## 状态字段

pending

## 任务目标

### 1. RmRestart 白名单 UI（task-057 后续）

- **现状**：task-057 上线后，开启开关会自动重启**所有** bRestartable 进程——但用户可能不希望某些进程（如更新器、后台服务）被自动拉起。
- **修法**：
  - prefs 表加 `restart_whitelist: Vec<String>`（进程名，如 `["notepad.exe", "code.exe"]`，空数组 = 全部可重启进程都试）；
  - Settings 页「行为偏好」下方加一个简单的白名单编辑框（textarea，每行一个进程名，支持 `*` 通配）；
  - 后端 `RmSession::restart` 调用前，对 bRestartable 进程按白名单过滤——白名单非空时只重启白名单内的；
  - **默认行为**：白名单为空 = 不限制（保持 task-057 现状），用户主动加进程名才启用白名单过滤。

### 2. safe_box 原子性回滚（task-016 遗留，B 中）

- **现状**：`engine/src/execute/safe_box.rs` 里 `fs::rename` 成功但 `INSERT INTO safe_box` 失败时，文件已在 box 目录但 DB 无行——下次 `enforce_quota` 会把孤儿文件 purge 掉，但用户感知是「文件消失了」。
- **修法**：
  - 用事务包裹：先 `INSERT`（拿到 rowid）再 `fs::rename`；
  - 如果 `fs::rename` 失败，**回滚事务**（DELETE 刚 INSERT 的行）+ 把文件移回原位置（或记为 fail）；
  - 跨盘 rename（copy+remove）同理：copy 成功但 INSERT 失败时，删除已 copy 的目标文件；
  - 新增单测：模拟 INSERT 失败 → 验证原文件还在原位、box 目录无孤儿。

### 3. 代码签名技术评估文档（B 中遗留）

- **现状**：PROJECT_STATE 「已知问题」B 中「task-033 installer 未做代码签名，未购 EV/OV 证书」——用户首次运行 .exe/.msi 会有 SmartScreen 警告。
- **修法**：写 `docs/research/code-signing-evaluation.md`，覆盖：
  - **方案 A：EV 证书**（~$300-500/年，SmartScreen 立即信任，硬件 token）；
  - **方案 B：OV 证书**（~$100-200/年，需积累 SmartScreen reputation 1-3 个月）；
  - **方案 C：自签名**（免费，SmartScreen 必然警告，不推荐分发给用户）；
  - **方案 D：Microsoft Store / winget 分发**（规避 SmartScreen，但需上架审核）；
  - **结论**：推荐方案 + 预估年成本 + 是否值得做（当前用户量小，SmartScreen 警告可接受）。

### 4. v0.1.7 CHANGELOG 草稿

- **现状**：task-057 已合并 main，但版本号仍是 v0.1.6；
- **修法**：
  - `CHANGELOG.md` 追加 v0.1.7 条目（草稿，待发版时定稿）；
  - 覆盖：RmRestart 自动重启（默认关闭 + 设置开关）、prefs 用户偏好、restored_at 审计列、跨盘 rename 测试、G6 豁免口径；
  - 版本号**不立即升**（留到发版任务再升），只写 CHANGELOG 草稿。

**本任务不做**：
- 不做差量更新（已决策不立项）；
- 不做 release profile 瘦身（已决策不做）；
- 不做 G6 修复（已豁免）；
- 不做品牌图标（已决策不做）；
- 不升版本号（留到发版任务）；
- 不发版（用户手动 workflow_dispatch）。

## 前置条件

1. `main` 已合并 task-057（merge `e9f0e7a`）；
2. 关键锚点：
   - `engine/src/prefs.rs`（加白名单字段）
   - `engine/src/execute/locking.rs`（RmSession::restart 加白名单过滤）
   - `engine/src/execute/safe_box.rs`（INSERT + rename 事务包裹）
   - `app/src/pages/Settings.tsx`（白名单编辑框 UI）
   - `CHANGELOG.md`（追加 v0.1.7 草稿）
   - 仓库根（新建 `docs/research/code-signing-evaluation.md`）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. RmRestart 白名单

1. prefs 表加 `restart_whitelist: Vec<String>`（默认空数组）；
2. 后端 `RmSession::restart` 按白名单过滤：空 = 全部可重启进程都试；非空 = 只重启白名单内进程名；
3. Settings 页加白名单编辑框（textarea，每行一个进程名）；
4. 切换白名单立即 `set_prefs` 持久化；
5. 单测：白名单过滤逻辑（空 / 匹配 / 不匹配）。

### 2. safe_box 原子性

6. `fs::rename` 路径用事务包裹（先 INSERT 再 rename）；
7. rename 失败时回滚事务（DELETE 刚 INSERT 的行）+ 文件留在原位；
8. 跨盘 copy+remove 同理：copy 成功但 INSERT 失败时删除目标文件；
9. 单测：模拟 INSERT 失败 → 原文件还在、box 目录无孤儿。

### 3. 代码签名评估

10. `docs/research/code-signing-evaluation.md` 存在；
11. 覆盖 A/B/C/D 四个方案对比（成本 / SmartScreen 行为 / 上架要求）；
12. 给出推荐方案 + 是否值得做的明确结论。

### 4. v0.1.7 CHANGELOG 草稿

13. `CHANGELOG.md` 追加 v0.1.7 条目（草稿）；
14. 覆盖 RmRestart / prefs / restored_at / 跨盘测试 / G6 豁免；
15. 版本号**不立即升**（package.json / tauri.conf.json / Cargo.toml 保持 0.1.6）。

### 后端单测

16. `cargo test -p engine` 全绿（现有 255+8 + 本任务新增 ≥4 条全绿）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/prefs.rs`（加白名单字段）
- `engine/src/execute/locking.rs`（RmRestart 白名单过滤）
- `engine/src/execute/safe_box.rs`（事务包裹）
- `engine/tests/`（白名单过滤 + 原子性回滚用例）
- `app/src/pages/Settings.tsx`（白名单编辑框）
- `CHANGELOG.md`（追加 v0.1.7 草稿）
- `docs/research/code-signing-evaluation.md`（新建）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-058-m4-finishup`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-058-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `feat(prefs): RmRestart 白名单（空=不限制）+ Settings 编辑框 (#task-058)`（单元 1）
   —— 含 prefs + locking + Settings.tsx + 单测。
2. `fix(execute): safe_box INSERT + rename 事务包裹，失败回滚不留孤儿 (#task-058)`（单元 2）
   —— 含 safe_box.rs + 单测。
3. `docs(research): 代码签名技术评估（EV/OV/自签/Store 四方案） (#task-058)`（单元 3）
4. `docs(changelog): 追加 v0.1.7 草稿条目（RmRestart/prefs/restored_at/跨盘/G6豁免） (#task-058)`（单元 4）
5. 自验全绿后，最后一次提交：`docs(state): 任务058完成，M4 收尾——白名单/原子性/签名评估/CHANGELOG 草稿就绪 (#task-058)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 058: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「已知问题」段把 task-016 原子性标记已解决；
   - 「下一步计划」改为「v0.1.7 发版（待用户手动 workflow_dispatch）；后续按需迭代（代码签名购买决策 / 品牌图标）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-29  task-058  M4 收尾（RmRestart 白名单 / safe_box 原子性 / 签名评估 / CHANGELOG 草稿）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：白名单过滤逻辑、safe_box 事务回滚、prefs 新字段默认值。
- **集成测试（必做）**：`cargo test -p engine` 全绿，确保现有不回归。
- **端到端写路径（必做，本任务硬要求）**：白名单持久化必须有一条真实「编辑白名单 → 查 prefs 存储 → 重开应用白名单仍在」证据——不能只构造内存 state。
- **真机冒烟（必做、不可裁剪）**：
  - 白名单为空：开启 RmRestart 开关 → notepad 被关后自动重启（现状）；
  - 白名单加 `chrome.exe`：notepad 被关后**不**自动重启，chrome 被关后自动重启；
  - 白名单持久化：编辑白名单 → 重开应用 → 白名单仍在；
  - safe_box 原子性：模拟一次 INSERT 失败（可手动改库），验证原文件还在原位；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 4 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（Settings 白名单编辑框 + RmRestart 行为变化，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 白名单过滤逻辑 | 单测输出（空/匹配/不匹配三态） |
| safe_box 事务回滚 | 单测输出（INSERT 失败 → 原文件在原位） |
| 白名单真机行为 | 真机截图：白名单为空 vs 非空时 notepad 重启行为不同 |
| 白名单持久化 | 真机证据：编辑白名单 → 重开应用 → 白名单仍在（DevTools 查 prefs） |
| 代码签名评估 | 文件存在 + 结论段落（推荐方案 + 成本） |
| CHANGELOG v0.1.7 草稿 | 文件内容预览 |
| 不回归 | `cargo test -p engine` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 4 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码/文档提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-058-m4-finishup`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 白名单数据结构（Vec<String> 还是 HashSet）；
   - safe_box 事务最终代码结构；
   - 代码签名评估最终结论；
   - CHANGELOG v0.1.7 条目摘要；
   - `cargo test -p engine` 通过输出（含新增 ≥4 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（4 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（v0.1.7 发版待用户手动；代码签名购买决策待用户拍板；品牌图标仍不做）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务058-M4 收尾（RmRestart 白名单 + safe_box 原子性 + 签名评估 + CHANGELOG） (#task-058)`，
   该提交只含 `tasks/058-m4-finishup.md` 一个文件，落在 `feature/task-058-m4-finishup` 分支最前端。
