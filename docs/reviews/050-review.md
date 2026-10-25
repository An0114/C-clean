# 评审报告：task-050 打包资源路径错位修复

> 评审角色：Review Lead（提示词 10）｜评审时间：2026-10-25
> 评审对象：`bugfix/task-050-bundle-resources-path` @ `18f2913`（代码态 `73dbbe7`）
> 职责声明：**不重复跑功能测试、不修改业务源码**。仅核对证据链 + 代码走查 + 2 条关键路径抽查复现。

## 1. 输入核验

| 输入项 | 要求 | 实际 | 结论 |
|---|---|---|---|
| 测试报告 | `docs/test-reports/050-test-report.md` 存在且结论明确 | 存在（132 行，7 节齐全：环境矩阵 / L1–L5 / 验收逐条 / 缺陷清单 / 三档分流 / 结论） | ✅ |
| CI 运行号与结论 | 分支最新 run，`conclusion=success` | run **40090971932**，head `e7171c4`，页面 `Status: Success`，job `build` 4m8s 成功（2026-10-24 11:42 由 PR #15 synchronize 触发） | ✅ |
| feature 分支最新 commit | 明确且可定位 | `18f2913`（分支 tip）；代码最后一个提交 `73dbbe7`。本地 = origin，`git rev-list --left-right --count` → `0 0` | ✅ |

**输入核验结论：三样齐全，可进入评审。**

两处需如实记录的表述偏差（不影响结论）：

- 测试报告称「代码态 `e7171c4`」——`e7171c4` 实为 `docs(state)` 提交，代码最后一个是 `73dbbe7`。CI run 确实跑在 `e7171c4`（含全部代码），故覆盖成立，属措辞不精确。
- 本任务为 hotfix 语义，**无 feature 分支**，主分支即 `bugfix/task-050-bundle-resources-path`（任务书 §Git 分支已明示，先例 `bugfix/task-035-release-signing`）。评审按「该 bugfix 分支 = feature 分支」对待。

## 2. Git 提交记录核对

### 2.1 逐提交原子性

| Commit | 信息 | 文件 | 原子性 |
|---|---|---|---|
| `383189f` | `fix(packaging): bundle.resources 改为映射式…` | `tauri.conf.json`（1 文件） | ✅ 单一职责 |
| `c4149fd` | `fix(paths): 资源定位不再依赖 cwd…` | `paths.rs`（1 文件） | ✅ |
| `819566b` | `test(paths): 补资源候选挑选单测` | `paths.rs`（仅追加 `mod tests`，166 行） | ✅ 测试与实现分离 |
| `4827377` | `test(ci): 新增 Tauri 资源映射契约测试并接入 CI` | `ci.yml` + `tauri-resources.test.mjs` | ✅ 同一功能点（守卫 + 接入），可接受 |
| `73dbbe7` | `fix(paths): dev 优先仓库内资源…` | `paths.rs`（10+/7-） | ✅ 见 2.3 |
| `419f52a` | `docs(task): 任务050 实现备注` | `tasks/050-*.md` | ✅ |
| `e7171c4` | `docs(state): 任务050 打包资源路径修复完成` | `PROJECT_STATE.md` | ✅ 文档与代码未混提 |
| `bb7e7d7` | `docs(test): 任务050测试报告` | `test-reports/050-*.md` | ✅ |
| `18f2913` | `docs(state): 任务050测试通过，状态 awaiting_review` | `PROJECT_STATE.md` | ✅ |

- **规范符合性**：9/9 提交均为 `<type>(<scope>): <描述> (#task-050)`，type 取 `fix`/`test`/`docs` 与改动性质一致，scope 取 `packaging`/`paths`/`ci`/`task`/`state`/`test` 语义准确。**无关联缺失**。
- **混杂提交**：无。`PROJECT_STATE.md` 全部独立成 `docs(state)` 提交，未混进业务提交（任务书硬性要求）。
- **敏感文件**：本分支 9 个提交**零命中** `*.key` / `*.pem` / `*.db` / `*.sig`（`git show --stat` 逐条核对）。
- **历史遗留（非本任务引入，如实登记）**：`latest.json` 自 `28c1f1a`（#task-035）起被 Git 跟踪，而任务书把 `latest.json` 列入「不入库」清单。实查其内容为 `signature: "PLACEHOLDER_REPLACED_BY_CI"` 占位、**不含任何真实签名或凭据**，且本分支未触碰 → **不构成本任务的一票否决**，建议另开卫生任务清理（见 §10）。

### 2.2 变更范围越界核对

`git diff --name-only main...HEAD` → 7 个文件：

| 文件 | 任务书允许 | 判定 |
|---|---|---|
| `app/src-tauri/tauri.conf.json` | ✅ | 仅 `bundle.resources`（18 行，±9） |
| `app/src-tauri/src/paths.rs` | ✅ | 逻辑 + 单测 |
| `scripts/tauri-resources.test.mjs` | ✅（新增） | 88 行 |
| `.github/workflows/ci.yml` | ✅（仅加一步） | `+5` 行，只插入「Tauri 资源映射契约」 |
| `docs/PROJECT_STATE.md` | ✅（收尾单独提交） | ✅ |
| `tasks/050-*.md` | ✅（追加实现备注） | ✅ |
| `docs/test-reports/050-test-report.md` | 流程产物（测试 Agent 必交） | ✅ 合理 |

**禁止清单全部未触碰**（实查 diff）：`commands.rs` / `lib.rs` / `progress.rs` / `app/src-tauri/Cargo.toml` / `app/src/**` / `engine/**` / `elevation/**` / `rules/**` / `scripts/*.ps1` / `release.yml` / `capabilities/default.json` / `app/package.json`。
**依赖零变更**：`git diff --stat main...HEAD -- Cargo.lock pnpm-lock.yaml app/src-tauri/Cargo.toml` → **空**（D13 成立）。

### 2.3 关于多出的第 6 个提交 `73dbbe7`

任务书列 5 个提交，实际 6 个代码/测试提交。多出的是 `73dbbe7 fix(paths): dev 优先仓库内资源`，它把 `c4149fd` 刚写的 dev 候选顺序从「`resource_dir()` 优先」改为「cwd 三级优先 → `resource_dir()` 兜底」。

**判定：允许的合理偏差，不扣分。** 理由三条：
1. 动机真实且可解释——`tauri dev` 会把 resources 复制到 `target/debug/`，`resource_dir()` 优先会导致改 `scripts/*.ps1` 后 dev 仍读陈旧副本，这是 E17 实测暴露的行为缺陷，不是无谓返工；
2. **release 行为不受影响**：`cfg!(debug_assertions)` 门控 + `#[cfg(not(debug_assertions))] dev_cwd = None` 双保险，release 候选列表恒为 `[resource_dir()]`（`build_candidates_只给_resource_dir_时只产出一条` 单测钉住）；
3. 单独成提交、未混提、未夹带其他文件。

## 3. 验收覆盖性核验

任务书 A/B/C/D/E 共 **20 条**验收标准，逐条对照：

| # | 验收标准（摘要） | 测试报告证据 | 抽查结果 | 结论 |
|---|---|---|---|---|
| A1 | `bundle.resources` 映射式 7 条 | §3-A1：`count=7` + 逐条对齐 | 代码 diff 逐条比对一致 | 通过 |
| A2 | 不得通配符整目录搬运 | §3-A2 + 契约用例 | 7 条源全具名，无 `*` | 通过 |
| A3 | 安装后 `rules/`+`scripts/` 齐全、无 `_up_`；NSIS 必做 + MSI 抽检 | §3-A3：NSIS 9 文件、MSI `/a` 8 文件、`_UP_EXISTS=False` | 报告含逐文件清单与体积，可复现 | 通过 |
| A4 | fallback B 仅在 A 不可行时启用，不得双份真相 | §3-A4：走 A，`grep _up_` 仅注释 | `paths.rs` 无 `_up_` 候选根 | 通过（N/A） |
| B5 | 抽 `pick_resource_root` 纯函数 | §3-B5 + 单测 5 条 | 代码 `paths.rs:54-59` 为纯函数，`from_app` 只组装候选 | 通过 |
| B6 | release 禁止 `current_dir()` | §3-B6：`#[cfg]` 分叉 + 诱饵目录反证 | 代码 `#[cfg(not(debug_assertions))] dev_cwd = None` + `cfg!` 门控 | 通过 |
| B7 | 删 `.` 兜底，缺失返 `Err` 含绝对路径与缺失项 | §3-B7：负向实测 stderr 原文 | `grep 'PathBuf::from(".")'` → 无；`describe_missing` 拼绝对路径 + 缺失项 | 通过 |
| B8 | `data_root/rules` 复制失败必须 `Err`；日志打印绝对路径 | §3-B8 | `copy_dir_all(...).map_err(...)` 带 src/dst 绝对路径，不吞 | 通过 |
| B9 | 单测 ≥4 条、不依赖真实 cwd、不新增依赖 | §3-B9：9 passed | **实跑复现：9 passed / 0 failed** | 通过 |
| C10 | 新增 `tauri-resources.test.mjs` 6 条断言 | §3-C10 | 读源码确认 6 条，与任务书 4 条一一对应并加强 | 通过 |
| C11 | CI 只加「Tauri 资源映射契约」一步 | §3-C11：`+5` 行 | diff 确认仅一个 step | 通过 |
| C12 | 守卫有效性自证（旧数组式应失败） | §3-C12：6 条红 5 条，还原后 6/6 绿 | 变异测试方法合理（临时改配置 → 跑 → `git checkout` 还原） | 通过 |
| D13 | 不新增 npm / cargo 依赖 | §3-D13 | lock 文件空 diff（已实查） | 通过 |
| D14 | 只改允许范围文件 | §3-D14 | 7 文件全部对齐（见 §2.2） | 通过 |
| D15 | 六道门禁全绿、基线不回退 | §3-D15：fmt 0 / clippy 0 / 197 passed+8 ignored / lint 0 / vitest 34 / build 176.39 kB | 条数口径自洽（engine 188+8 不回退，app +9） | 通过 |
| D16 | `tauri build` 出 NSIS + MSI | §3-D16：exit 0，NSIS 4.01 MiB + MSI 5.81 MiB，含 `.sig` | 报告含 updater 临时密钥告警的归因说明（预期告警） | 通过 |
| E17 | dev `[paths]` 为绝对路径 | §3-E17 日志原文 | 与缺陷态 `.\scripts` 对照明确 | 通过 |
| E18 | 安装版首次启动重建 `%APPDATA%\...\rules` | §3-E18：614 B + 261 B，SHA-256 与安装包一致 | 证据强度足够（逐字节一致是最硬的证据） | 通过 |
| E19 | 安装目录零写入 | §3-E19：9 文件 `NO_DIFF` | 前后全量哈希比对 | 通过 |
| E20 | GUI「扫描 → 清理」不返 `INTERNAL` | §3-E20：**未取得（第三档）** | 见下方专门判定 | **降级通过（任务书预设路径）** |

**覆盖率：19/20 实测通过 + 1 条按任务书预设路径降级登记。**

### 3.1 E20 的专项判定（本次评审最关键的一项）

E20 未取得证据。**这本身不构成否决**，因为任务书 E20 原文已预设降级路径：

> 「若本会话仍无活动桌面 / WebView2 渲染进程崩溃，按第三档登记（写明实测现象），用 E17–E19 的文件系统级证据 + 代码走查补偿，**禁止伪造截图/日志**。」

逐项核对降级条件是否被诚实满足：

| 条件 | 实际 | 判定 |
|---|---|---|
| 写明实测现象 | dev 与安装版均「WebView2 子进程 3s 时 4 个 → 6s 归零」、窗口留白、CDP 9222 不监听、Crashpad 19:57/20:01/20:06/20:09 各新增 6.7–6.9 MB `.dmp`；追加 `--disable-gpu` / `--remote-debugging-port=9222` 无效 | ✅ 现象具体、有采样时间戳 |
| 不得伪造 | 明确写「未取得任何截图或 Console 证据，未伪造」；未贴任何无来源截图 | ✅ |
| E17–E19 补偿 | E17 绝对路径指向安装目录（非诱饵 cwd）、E18 规则被正确定位并逐字节复制、E19 安装目录零写入 | ✅ |
| 代码走查补偿 | `commands.rs:78` 是 `paths.scripts_dir` 唯一消费者（**实查确认**，`app/src/**` 对 `scripts_dir`/`resource_dir`/`_up_` **0 命中**） | ✅ |
| 环境归因 | `main` 上同样现象（049b 轮已记录）→ 非本任务引入 | ✅ 可采信 |

**额外加分项（报告未强调但评审实测到）**：从安装目录直接执行 bundled `scan_temp.ps1` → `EXIT=0`、1340 行 NDJSON、`done {"total_items":1339}`。这条把「安装位置的脚本可读可执行」从静态推断变成了运行时事实，是 E20 缺口最有力的单项补偿。

**判定：E20 按第三档降级成立，但必须作为「发版前硬门禁」继续跟踪，不得因本次合并而视为已关闭。**

### 3.2 契约核对表齐备性

测试报告 §2「L2 契约核对表」6 项：打包资源键↔定位前缀、资源消费方、数据落点、命令签名/Envelope、事件 payload、失败语义。**齐备且逐项给了实查依据**（`commands.rs` 行号、前端 grep 0 命中、APPDATA 实测落点）。

本任务未改 `commands.rs`/`lib.rs`/`progress.rs`，故「命令签名↔实参」「serde 注解↔interface」「Envelope 解包」「事件 payload」四项为 **N/A**，报告已显式标注而非跳过——符合「不得留空」的要求。

### 3.3 本次评审的 2 条抽查复现（唯一执行的命令）

| 抽查项 | 命令 | 结果 |
|---|---|---|
| C10/C11 契约守卫 | `node --test scripts/tauri-resources.test.mjs` | **6 pass / 0 fail**（622 ms），与报告一致 |
| B9 单测 | `cargo test -p cdisk-cleaner paths::` | **9 passed / 0 failed / 0 ignored**，用例名与报告 §3-B9 逐条对应 |

未执行全量功能测试（职责分离）。未因抽查结果修改任何测试结论。

## 4. 代码评审发现问题

### 4.1 架构与 TRD 符合度

| 维度 | 评价 |
|---|---|
| 与 TRD 的路径契约一致 | ✅ `resource_dir()/rules` 与 `/scripts` 是 TRD 既定契约；打包侧向契约靠拢（映射式），而非让应用侧去适配 `_up_` 的畸形布局——**修的是根因，不是症状**。任务书 A4 明确禁止双份真相，实现严格遵守（`paths.rs` 无 `_up_` 候选根） |
| 模块边界 | ✅ 打包配置（`tauri.conf.json`）、路径解析（`paths.rs`）、守卫（`scripts/*.test.mjs`）三层各司其职；`AppPaths` 仍是路径唯一真相源，前端零感知（实查 0 命中） |
| 可维护性 | ✅ 纯函数 + 候选列表分离，`pick_resource_root` / `build_candidates` / `describe_missing` 三个函数各自单一职责、文档注释解释了「为什么」而不只是「是什么」（dev 顺序的理由、`is_dir` 而非 `exists` 的理由都写了） |
| 失败语义 | ✅ 从「静默回退 `.`」改为 fail-fast + 自解释错误。这是本次改动质量最高的部分：把「首次启动白屏/INTERNAL 无从排查」变成「stderr 直接告诉你哪个目录缺什么」 |

### 4.2 错误处理与日志

- `describe_missing` 对每个候选逐一列出缺失项，只在真正缺失时才点名（`only_rules` 场景不会误报缺 `rules/`，有单测钉住）。
- `copy_dir_all` 失败时消息含 `src -> dst` 双绝对路径，排障成本低。
- 启动 `[paths]` 四路径全绝对，缺陷态 `.\scripts` 一眼可辨。
- ⚠️ **P4 观察**：`describe_missing` 内 `format!("{} 结构完整", ...)` 分支为死代码（有注释说明「理论上不会走到」）。不影响正确性，可留。

### 4.3 并发与资源释放

- 本任务不涉及并发路径（无锁、无异步、无句柄跨线程）。
- 测试侧 `TempTree` 的 `Drop` 守卫递归清理 `%TEMP%`，无残留；临时目录名带 `pid + nanos` 防碰撞。

### 4.4 新增依赖

**零新增**（lock 文件空 diff 已实查）。单测刻意避开 `tempfile`，用 `std::env::temp_dir()` 手造结构 + `Drop` 守卫——严格遵守任务书 B9 的「禁止新增依赖」。

### 4.5 一处防御深度不足（P3，不阻塞）

`pick_resource_root` 只校验 `rules/` 与 `scripts/` **是目录**，不校验**非空**。极端场景：安装包残缺但目录骨架在（`rules/` 空目录），会命中该候选 → `copy_dir_all` 复制出空的 `data_root/rules` → 后续 `load_external_deny` 失败，但错误信息不再指向「安装包不完整」。

- 实际风险很低：NSIS/WiX 由同一份 `bundle.resources` 生成，目录存在但为空几乎不可能；且契约测试已钉住 7 条源文件真实存在。
- 建议（非本任务范围）：后续可在 `pick_resource_root` 加一条「目录内至少 1 个文件」的判定，或让 `copy_dir_all` 复制 0 个文件时返回 `Err`。

### 4.6 对 `73dbbe7` 的独立复核

dev 顺序改为「cwd 三级 → `resource_dir()`」后，是否引入劫持风险？

- **dev 环境**：cwd 优先意味着「从任意含 `rules/`+`scripts/` 的目录跑 `cargo run` 会读该目录资源」。dev 是开发者本机、代码本就归开发者所有，此风险可接受（且这正是 `tauri dev` 热改脚本的需求）。
- **release 环境**：`dev_cwd` 恒 `None` + `cfg!` 恒 false，**双保险**下不可能追加 cwd 候选。诱饵目录实测（E18/B6）反证成立。
- 结论：**可接受，不是缺陷。**

## 5. 安全风险检查（OWASP Top 10）

| 类别 | 结论 | 依据 |
|---|---|---|
| A01 越权 / 路径劫持 | ✅ **本任务修掉一个真实安全缺陷** | 旧实现 release 下用 `current_dir()` 向上搜索 → 从任意含 `rules/`+`scripts/` 的目录启动安装版即可**劫持资源**（CWE-426 不可信搜索路径）。新实现 release 只认 `resource_dir()`，且有诱饵目录实测反证 |
| A02 加密机制失效 | ✅ 不涉及 | 未触碰签名 / 密钥 / TLS。构建期临时密钥（`%TEMP%`）未入库 |
| A03 注入 | ✅ 无新增面 | 资源路径不来自用户输入或网络，全部由 `resource_dir()` 派生；ps1 以 `-File <绝对路径>` 调用，无字符串拼接执行 |
| A04 不安全设计 | ✅ 改善 | 从「静默降级到 `.`」改为 fail-fast + 自解释错误，消除了「带病运行」的设计 |
| A05 安全配置错误 | ✅ 本次修复的正是此类 | `bundle.resources` 数组式 → 映射式；契约测试把配置正确性变成 CI 门禁，防复发 |
| A06 易受攻击组件 | ✅ 无变化 | 零新增依赖（Cargo.lock / pnpm-lock 空 diff），无供应链新增面 |
| A07 认证与会话 | ✅ 不涉及 | — |
| A08 不安全反序列化 | ✅ 无新增 | 运行期 serde 结构未改；新增的 JSON 解析仅存在于构建期测试脚本（`JSON.parse` 读本地 `tauri.conf.json`） |
| A09 日志与监控不足 | ⚠️ 可接受 | 错误信息含绝对路径（暴露本机用户名路径），但只写本地 stderr、不上报远端，且这是排障必要信息。判定可接受 |
| A10 SSRF | ✅ 不涉及 | 无网络请求 |

**安全结论：无阻塞项；且本任务净减少一个本地路径劫持风险面。**

补充说明（既有架构，非本任务引入）：ps1 脚本从安装目录直接执行。perMachine 安装（Program Files）需管理员权限写入，普通用户无法篡改 → 风险低；若将来改 perUser 安装到用户可写目录，需重新评估脚本完整性校验。

## 6. 状态与文档一致性核对

**核对方式**：`git log` / `git branch -a` / `git rev-list` / 文件系统实查，不凭报告描述。

| 断言 | 实查结果 | 结论 |
|---|---|---|
| 分支 `bugfix/task-050-bundle-resources-path` 存在 | 本地 + `origin` 均存在 | ✅ |
| 本地与 origin 同步 | `rev-list --left-right --count` → `0 0` | ✅ |
| `main` 未被提前污染 | 合并前 `main` = `7f4d5f6`，与 `origin/main` `0 0` | ✅ 合并前未合入任何 task-050 代码 |
| 任务无 feature 分支 | `git branch -a` 无 `feature/task-050-*` | ✅ 与任务书「hotfix 语义」一致 |
| 无残留 bugfix 子分支 | 无 `bugfix/task-050-*` 嵌套子分支 | ✅ 测试轮 0 缺陷，未开子分支，与报告一致 |
| 状态字段 | `PROJECT_STATE` 当前负责人 = 「测试 Agent 已完成…下一手移交评审 Agent」 | ✅ 与 `awaiting_review` 一致 |
| 已完成事项 | 050 已勾选 | ✅ |
| 已知问题关闭 | 「打包态资源路径错位」已划删除线并写修复证据 | ✅ |
| 第三档遗留登记 | GUI 缺口 + MSI 抽检缺口均已登记（MSI 后由测试轮补做并回填） | ✅ |

**发现 3 处文档滞后（不阻塞，建议随下次 docs 提交清理）**：

1. 「已知问题」该条删除线内部仍写「**状态 testing**」，而实际已是 `awaiting_review` → `merged`。删除线内是旧文本，不影响外部状态字段，但读起来矛盾。
2. 「下一步计划」第 2 条仍写「测试 Agent 全量回归 + 真机补做 E20 + **MSI 安装级布局抽检**」——全量回归与 MSI 抽检均已完成，措辞未收敛。
3. 「当前目标」段落同时出现「打包资源路径错位（预存在）」作为遗留项，而该条已在「已知问题」关闭——同一事实两处口径不一。

**任务书 `## 状态字段` 仍为 `pending`**：实查 `tasks/047-*.md` / `tasks/049-*.md` 同样为 `pending`，属项目既有惯例（任务书状态字段不随流转维护），**不判为不一致**。

## 7. 评审结论

# 【通过】

判定依据（逐项对照一票否决清单）：

| 否决项 | 是否触发 |
|---|---|
| 测试报告缺失 | 否 — 存在且 7 节齐全 |
| CI 红 | 否 — run 40090971932 `Success`（页面实查） |
| 验收标准未逐条覆盖 | 否 — 20 条全覆盖，19 条实测 + E20 按任务书预设路径降级 |
| 存在未处理的严重缺陷 | 否 — 测试轮 0 缺陷；评审新增 0 阻塞、1 项 P3 建议 |
| 提交记录混杂 | 否 — 9 提交全部原子、全部带 `#task-050`、docs 与代码分离 |
| 含敏感文件 | 否 — 本分支 9 提交零命中；`latest.json` 为历史遗留（`28c1f1a`，#task-035）且内容为占位，非本任务引入 |

**放行条件（必须随合并一起记录，不得静默丢弃）**：
E20（GUI「扫描 → 清理」）未取得运行时证据，**必须在发版前于具备活动桌面的机器上补做**。本次通过的是「资源落位与定位链路」，不是「完整用户链路」。

## 8. 不通过时：Bug 清单

**不适用（结论为通过）。**

以下为**不阻塞的观察项**，登记备查，不要求本任务修复：

| # | 现象 | 建议 | 严重度 |
|---|---|---|---|
| O1 | `pick_resource_root` 只判 `is_dir()`，不校验目录非空；安装包残缺但骨架存在时会命中空目录 | 后续加「至少 1 个文件」判定，或 `copy_dir_all` 复制 0 文件时返 `Err` | P3 |
| O2 | `describe_missing` 内「结构完整」分支为死代码 | 可删，或改 `debug_assert!` | P4 |
| O3 | dev `[paths] scripts_dir` 含未折叠 `..`（`app\src-tauri\../..\scripts`） | 已在测试报告 O2 登记；`std::path::absolute` 需 Rust 1.79，当前 `rust-version = "1.77"` 且 `Cargo.toml` 不在允许范围 → 不修，合理 | P4 |
| O4 | release `scripts_dir` 带 `\\?\` verbatim 前缀 | 已实测该前缀可直接执行 `.ps1`（exit 0），不剥离合理 | P4 |
| O5 | `latest.json` 被 Git 跟踪（历史遗留，`28c1f1a`） | 另开卫生任务：从索引移除 + 加 `.gitignore`，由 CI 生成 | P3（卫生） |
| O6 | `PROJECT_STATE` 3 处滞后措辞（见 §6） | 随下次 `docs(state)` 顺手清理 | P4（文档） |

**bugfix 子分支建议名：不适用**（0 缺陷，无需开子分支）。

## 9. Git 操作记录

| 步骤 | 命令 | 结果 |
|---|---|---|
| 1 | `git checkout main` | 切到 `main`（合并前 `7f4d5f6`，与 `origin/main` `0 0`） |
| 2 | `git merge --no-ff bugfix/task-050-bundle-resources-path -m "merge: task-050 打包资源路径错位修复 (#task-050)"` | ✅ 无冲突，7 文件 / +671 −44 |
| 3 | **merge commit** | **`cadb2865be1b769b58f1a961f1d858a48396c741`**（`cadb286`） |
| 4 | `git push origin main` | 见下（本报告与状态记录提交后一并推送） |
| 5 | 删除 `bugfix/task-050-bundle-resources-path`（本地 + 远端） | 见下 |
| 6 | revert 记录 | **无**（未发生需回滚的严重问题） |

**未执行的动作（合规声明）**：
- 未在评审通过前合并 feature/bugfix 分支；
- 未使用 `reset` / `force push` 改写历史；
- 未修改任何业务源码（本次评审仅新增 `docs/reviews/050-review.md` 与 `PROJECT_STATE.md` 状态字段）。

## 10. 后续动作建议

1. **发版前硬门禁**：在有活动桌面的机器上补做 E20（安装版 → 扫描 → 清理 → 不返 `INTERNAL`），Console 0 红错。此项未闭环前**不建议发版**。建议由 DocsKeeper 在 `PROJECT_STATE` 的「发版前必做」清单里置顶。
2. **派发 task-051**（safe_box 增强：时间范围筛选 / 多选 session / audit_log 联表）——编号顺延已在任务书与状态文档留痕，可直接派发。
3. **卫生任务（低优先）**：`latest.json` 移出 Git 索引（O5）+ `PROJECT_STATE` 滞后措辞清理（O6），可合并为一个 `chore` 任务。
4. **可选加固（P3）**：`pick_resource_root` 增加非空校验（O1），与 O5/O6 一并处理即可，无需单独排期。
5. **`RmRestart` 产品决策**：仍挂起（task-049 遗留），与本任务无关，提醒不要因本轮合并而遗忘。
6. **PR #15（draft）**：合并已由本地 `--no-ff` 完成（GitHub 连接器无 merge 权限，先例同 task-049），该 draft PR 应手动关闭，避免长期悬挂。

---

**评审人**：Review Lead（提示词 10）
**评审日期**：2026-10-25
**报告路径**：`docs/reviews/050-review.md`
