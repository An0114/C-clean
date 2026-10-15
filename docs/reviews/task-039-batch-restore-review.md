# 测试评审报告：task-039 safe_box 批量恢复

> 评审角色：企业级测试评审 Agent（只读 + 评审报告；无业务代码提交权限）
> 评审对象：`feature/task-039-safebox-batch-restore` @ `daf7adf`（11 commits）
> 评审基线：`main` @ `2068fcd`（task-038 合并点）
> 评审时间：2026-10-15
> 评审方法：git 事实核对 + 代码走查 + **本机真机运行时冒烟**（`pnpm tauri dev` + WebView2 CDP 挂载 9222 端口，逐个 tab 点击 + 造真数据走完整批量恢复链路）+ **GitHub Actions 实查**

## 本次任务名称

task-039：`safe_box.restore_many` 批量恢复——复用 task-038 checkbox UI + 后端 `restore_many` + Tauri 命令 + 前端绿色「批量恢复已选(N)」按钮。

## Git 提交记录核对结果（原子性、规范符合性）

`git log main..feature/task-039-safebox-batch-restore` 实际 11 个提交（与任务书要求的 5 个 + 评审前清障 6 个）：

| # | commit | type | 文件 | 判定 |
|---|--------|------|------|------|
| 1 | `89c530d` | docs(task) | `tasks/039-safebox-batch-restore.md` | ✅ 单文件 |
| 2 | `4e3e0c6` | feat(execute) | `engine/src/execute/safe_box.rs` | ✅ 单文件 |
| 3 | `577b87f` | feat(tauri) | `commands.rs` + `lib.rs` | ✅ 单职责 |
| 4 | `1e35347` | test(execute) | `engine/tests/safe_box_restore_many.rs` | ✅ 单文件 |
| 5 | `2e5eff3` | feat(frontend) | `app/src/pages/SafeBox.tsx` | ✅ 单文件 |
| 6 | `40001ef` | docs(state) | `docs/PROJECT_STATE.md` | ✅ 单文件 |
| 7 | `adcba07` | fix(execute) | `safe_box.rs` + `safe_box_purge_many.rs` | ✅ 纯 fmt（见下） |
| 8 | `54dd237` | test(execute) | `engine/tests/safe_box_restore_many.rs` | ✅ 单文件 |
| 9 | `de107af` | fix(frontend) | `app/src/pages/SafeBox.tsx` | ✅ 单文件 |
| 10 | `afa6112` | docs(task) | `tasks/039-safebox-batch-restore.md` | ✅ 单文件 |
| 11 | `daf7adf` | docs(state) | `docs/PROJECT_STATE.md` | ✅ 单文件 |

结论：

- ✅ 11 个提交全部形如 `type(scope): 说明 (#task-039)`，**全部关联 `#task-039`**，无一个提交混杂业务代码与文档（`docs(state)` 均为单文件提交）。
- ✅ 前 5 个提交与任务书 §Git 提交要求逐条对齐（含 `tasks/039-*.md` 单独提交、落分支最前端）。
- ✅ 白名单核对：业务改动只落在 `engine/src/execute/safe_box.rs`、`engine/tests/safe_box_restore_many.rs`、`app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`、`app/src/pages/SafeBox.tsx`、`tasks/039-*.md`、`docs/PROJECT_STATE.md`，**无越界文件**。
- ⚠️ **登记偏差 1**：`adcba07` 触及 `engine/tests/safe_box_purge_many.rs`（不在任务书白名单）。已用 `git show -w adcba07` 核对：除签名折行与 `BatchOpResult` doc 注释外**无语义变化**（纯 `cargo fmt` 结果），该修复是让 main 红灯 CI 转绿的必要项，判定可接受。
- ⚠️ **登记偏差 2**：任务书要求 `docs(state)` 为最后一次提交，实际 `40001ef`（任务039完成）之后仍有 3 个代码/测试提交 + 2 个文档提交。属评审前清障追加，末条 `daf7adf` 仍是 `docs(state)` 单文件提交，且 PROJECT_STATE 已如实登记清障过程，判定可接受。

## CI 门禁核对结果（强制项，基于 GitHub Actions API 实查，非印象）

| 对象 | run | sha | 结论 |
|------|-----|-----|------|
| 待评审分支（PR #2 触发） | [38671216149](https://github.com/An0114/C-clean/actions/runs/38671216149) | `daf7adf` | **success**（17/17 步全绿：Rustfmt → Clippy → Rust 测试 → pnpm install --frozen-lockfile → Lint → Test → Build → Tauri build → Upload installer artifact → rules 双守卫） |
| 合并前 main 基线 | [38571515030](https://github.com/An0114/C-clean/actions/runs/38571515030) | `2068fcd` | **failure**（`Rustfmt 检查` 步红：`Diff in engine/src/execute/safe_box.rs:441`、`engine/tests/safe_box_purge_many.rs:12/53` —— 即 task-038 引入的 fmt 问题，后续步骤全部 skipped） |
| 合并后 main | [38673720322](https://github.com/An0114/C-clean/actions/runs/38673720322) | `cbc5aaa` | **success**（本分支 `adcba07` 修的正是上面两处，合并后 main CI 由红转绿） |

说明（事实核对）：该仓库 `ci.yml` 只在 `push: main` 与 `pull_request` 上触发，**推 feature 分支本身不会产生 CI 运行**（GitHub Actions API 全量 run 列表可证：历史 run 全部来自 `push: main` 或 Release `workflow_dispatch`，**无一条来自 feature 分支 push**）。因此“该分支的 CI 状态”在本次评审前根本不存在；为满足「合并前必须看 CI」的强制要求，评审侧创建 PR #2 取得分支真实 CI 结论 = **绿**，再执行合并。不存在“CI 是环境问题”的口头通融：唯一的 CI 红是真实代码格式问题，且已在本分支修复并实测转绿。

## 分步验证清单（每条必须有运行时证据）

运行时环境（真实执行，非推断）：

- 启动：`app/` 下 `pnpm tauri dev`（`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`），产物 `target/debug/cdisk-cleaner.exe` 正常拉起窗口，`[paths]` 打印 `data_root=C:\Users\An\AppData\Roaming\com.and0114.cclean`、`box_dir=...\safe_box`。
- DevTools：CDP 挂载 WebView2 `Edg/155.0.4283.45` 的 page target（`http://localhost:5173/`），开启 `Runtime/Log/Page` 域**持续采集 console 与异常**。
- 造真数据：向 `safe_box` 表 + `box_dir` 注入 3 条真实条目（2 条原父目录存在=可恢复，1 条原父目录缺失=预期失败），全部指向 `%TEMP%\cclean-review039\`，避免触碰用户既有数据。

| # | 验收标准 | 代码走查 | 运行时现象（截图/日志/描述） | 结论 |
|---|---------|---------|---------------------------|------|
| 1 | A1 `restore_many(conn, box_dir, item_ids)`（与任务书 §A.1 的偏差已在任务文档备注说明） | ✅ | 真机点「批量恢复已选(3)」返回 `succeeded=2, failed=1` | 通过 |
| 2 | A2 循环调 `restore_from_safe_box`，单个失败不中断、收集 errors | ✅ | 真机批次中 `id=4`（父目录缺失）失败后整批仍正常返回，红条显示 `成功 2 条，失败 1 条：id=4: 原父目录不存在，请先手动创建: …\missing_dir`；「失败夹中间不阻断后续 id」由新增用例 `restore_many_failure_in_middle_does_not_stop_rest` 覆盖（pass） | 通过 |
| 3 | A3 Tauri 命令 `safe_box_restore_many`（入参 `itemIds`）+ 注册 `lib.rs` | ✅ | 前端 `invoke` 成功返回（CDP 采集零 exception），信封 `code=0`，数据字段与前端 interface 对齐 | 通过 |
| 4 | A4 `BatchOpResult` 新建（未复用 `PurgeManyResult`，任务文档已备注理由） | ✅ | 真机读到的 `succeeded/failed/errors` 数值正确、camelCase 对齐 | 通过 |
| 5 | B5 三条集成测试（全成功 / 部分失败 / 空 Vec） | ✅ | `cargo test -p engine`：`safe_box_restore_many` **4 passed / 0 failed**（含清障补的中间失败用例） | 通过 |
| 6 | C6 绿色「批量恢复已选(N)」在「批量删除已选」**左边**，N=0 禁用 | ✅ | 截图 `11-safebox-selected-all.png`：未选态两按钮均 `disabled`、文案 `批量恢复已选(0)`；点表头全选后变 `批量恢复已选(3)`（绿色可用，位于红色 `批量删除已选(3)` 左侧），DOM 断言 `checkbox states=[true,true,true,true]` | 通过 |
| 7 | C6 点击 → `window.confirm` → invoke → 绿/红条 + 清空 selected + 重拉列表 | ✅ | CDP 捕获对话框事件 `{type:"confirm", message:"确定恢复 3 个文件到原路径？"}`；确认后页面回到 `共 1 项 / 总大小 8 B`、`批量恢复已选(0)`（disabled）、勾选清空（截图 `12-safebox-after-batch-restore.png`） | 通过 |
| 8 | D7 不改单条 `restore_item` / `purge_item` 逻辑 | ✅ | `git show -w adcba07` 仅签名折行 + doc 注释；单条恢复/删除代码零改动 | 通过 |
| 9 | D8 不改 checkbox 列 / 全选逻辑 | ⚠️ 登记偏差 | 全选与单选逻辑未改；`de107af` 给 checkbox 与行内按钮的 `disabled` 条件追加 `batchPurging/batchRestoring`（批量进行中禁止单条操作，消除竞态），位于白名单文件内 | 通过（偏差已登记） |
| 10 | 提交前自验 7 条命令全绿 | ✅ | 评审独立复跑（合并后 `cbc5aaa` 工作树）：`cargo fmt --all -- --check`=0；`cargo clippy --workspace --all-targets -- -D warnings`=0；`cargo test --workspace` **147 passed / 0 failed**；`pnpm --filter app lint`=0；`pnpm --filter app test` **12 passed**；`pnpm --filter app build`=0（`168.51 kB / gzip 53.43 kB`） | 通过 |
| 11 | 端到端副作用正确（文件回原位、失败项保留、审计留痕） | ✅ | 磁盘：`…\cclean-review039\target\restored_a.txt`=4B(`AAAA`)、`restored_b.txt`=6B(`BBBBBB`) 回原路径；失败项 `rev039_c.tmp` 仍在 `box_dir`、DB 行 `status='in_box'` 保留；`audit_log` 新增 2 条 `action='restore' result='success'` | 通过 |
| 12 | Console 零红色 error（本任务页面） | ✅ | 保险箱全流程（刷新→全选→批量恢复→批量删除）CDP 采集：**0 条 console.error、0 条 exception**；五 tab 复扫同样 0 红错（设置页例外，见「已知问题 1」，非本任务页面） | 通过 |
| 13 | 回归：批量删除（task-038 功能）未被本次改动破坏 | ✅ | 真机「批量删除已选(1)」→ confirm `确定永久删除 1 个文件？此操作不可恢复。` → `成功 1 条` → `共 0 项 / 保险箱为空`，box 文件同步删除，零 console 错误 | 通过 |
| 14 | 其他四个 tab 可正常点开、无白屏/无红错 | ✅ | 仪表盘（总释放空间 6.7 MB / 成功 1 / 失败 1 / 清理轮次 0）、清理（表格渲染 `C:\Windows\SoftwareDistribution\… 13.0 MB windows_update`）、历史（purge 事件与路径详情）、设置（见已知问题）均正常渲染，零 exception | 通过 |

> 运行时原始证据（截图 `01/02/03/10-13/20-*.png` 与本 CDP 采集日志）随评审会话生成，存于仓库外评审工作目录（未入库，避免向仓库引入二进制产物）。

## 代码评审发现问题

- 未发现 P0/P1 缺陷；未发现逻辑错误、越权、竞态残留。
- 设计核对：`restore_many` 语义与批量语义一致（单个失败不抛硬错、失败收集进 `errors`、空 Vec 直接返默认值），`BatchOpResult` 与 `PurgeManyResult` 解耦（`succeeded` vs `deleted`）理由充分。
- 竞态核对：批量进行中 刷新按钮 / 两个批量按钮 / 表头全选 / 行内 checkbox / 行内「恢复」「永久删除」全部 `disabled`，无并发入口。
- 边界核对：失败项（父目录缺失）在 UI 与 DB/磁盘三方状态一致（行保留 `in_box`、箱内文件保留），用户可手动建目录后重试，无静默丢数据。
- 技术债（非本次新增，随评审发现，建议单独立项）：见下「已知问题」两条（设置页 IPC 首调红错、规则检查无超时收敛）。

## 安全风险检查结果

按 OWASP Top 10（2021）逐项核对本次改动面（前端 1 个按钮 + 后端 1 个循环函数 + 1 条 IPC 命令）：

| 类别 | 结论 |
|------|------|
| A01 权限控制失效 | ✅ 无新增外部入口；命令仅经 Tauri WebView IPC 可达，未开 HTTP/自定义协议暴露面 |
| A03 注入 | ✅ `restore_many` 全部走既有参数化查询（`WHERE id=?1`），无 SQL 拼接；无 shell/命令拼装 |
| A04 不安全设计 | ✅ 破坏性操作有 `window.confirm` 二次确认且文案含数量；恢复目标路径**只能来自 DB 内既有记录**，非前端任意路径输入，无法借批量接口写任意文件 |
| A08 数据完整性 | ✅ `fs::rename` 失败即返 Err、不删 DB 行，不会出现“文件没了行还在/行没了文件还在”；`audit_log` 落 `restore/success` 留痕 |
| A09 日志与监控 | ⚠️ 低：错误串含完整绝对路径并回传前端（本地单机应用，可接受）；审计表可被本地用户直接改写（既有设计，DB 无完整性校验） |
| A10 SSRF | ✅ 本次未新增任何网络调用（`rules_check_update` 的既有外呼在设置页，与本次无关） |
| 密钥/供应链 | ✅ 无硬编码密钥；依赖未变动，`pnpm-lock.yaml`/`Cargo.lock` 未被本任务改动 |

## 评审结论：【通过】

依据：① 11 个提交原子、规范、全部关联任务号、无混杂；② 任务书 8 条验收标准（A1–A4 / B5 / C6 / D7–D8）逐条走查 + **真机运行证据**全部满足；③ 批量恢复全链路真机跑通（含失败分支与回归批量删除），文件/DB/审计三方一致；④ 本任务页面 DevTools Console 零红色 error；⑤ 全量门禁本地复跑全绿（147 后端测试 + 12 前端断言）；⑥ **分支 CI 实测绿色**（run 38671216149）。偏差 2 条（fmt 触及 purge 测试文件、cleanup 提交次序）已逐条登记，无阻断项。

## 如果不通过：Bug 清单 + 修复建议

本次结论为「通过」，无阻断性 Bug。以下 2 条为评审冒烟**额外发现**的非阻塞已知问题（**非本任务引入**，属 task-022/031 设置页范围，已同步登记 `docs/PROJECT_STATE.md`【已知问题】）：

### 已知问题 1（P2，中）设置页首次调用 `rules_check_update` 产生红色 console error

- 现象：进入「设置」tab 时，DevTools Console 出现 2 条红色 `error`（source=`network`）：`Failed to load resource: net::ERR_CONNECTION_REFUSED`，url=`http://ipc.localhost/rules_check_update`；伴随 2 条 warning「IPC custom protocol failed, Tauri will now use the postMessage interface instead」。功能经 postMessage 回退后仍可用（不白屏、不卡死）。
- 复现步骤：关掉应用 → `pnpm tauri dev` → 启动完成后**首次**点击「设置」（或 `Page.reload` 后首次点「设置」）→ 观察 Console 与 Network；同一文档内再次进入设置页**不再复现**。
- 影响面（实测对照）：同条件下点「历史/清理/仪表盘」的 IPC 调用 **0 红错**，说明不是通用的“文档加载后首次 invoke”竞态，而是该命令特有路径。
- 期望行为：Console 无红色 error（或一次性回退不改用会报红的传输通道）；Tauri 侧对 ipc.localhost 首调失败做静默重试而非留红错。
- 归属核对：`app/src/pages/Settings.tsx` 在本分支与 `main` **完全一致**（`git diff main...HEAD` 为空），`rules_check_update` 命令本分支仅新增无关命令 → **非 task-039 引入，不阻断本次合并**。
- 建议：新建 `bugfix/task-039-settings-ipc-redconsole`（或并入 task-040 顺带修）排查 Tauri 2.x ipc.localhost 首调失败与 `useCustomProtocol` 回退策略。

### 已知问题 2（P2，中）设置页规则检查长时间停留「检查中…」

- 现象：设置页「远端版本 / 状态」长期显示 `检查中…`（实测 ≥8s 未收敛，无超时失败态）；本机仍能访问 GitHub API，但 `https://raw.githubusercontent.com/An0114/C-clean/main/latest.json` 对**私有仓库**返回 404（实测 404），Rust 侧无短超时/失败态收敛，前端 `phase` 停在 `checking`。
- 期望行为：外呼失败/超时应在数秒内落到 `error` 态并给出可读原因（区分网络/校验/后端），而不是无限「检查中…」。
- 归属核对：`rules_check_update` 为 task-022 产出、task-031 接入前端，与本任务无关。
- 建议：为规则检查加显式超时（如 5s）+ 失败态文案；同时评估私有仓 `latest.json` 端点是否需要改走 Releases 资产（与 task-034/035 的 updater 端点同类问题）。

## Git 操作记录（merge commit hash / revert 记录）

| 动作 | 结果 |
|------|------|
| 合并前基线 | `main` = `2068fcd34bff44dc1afcfc5d942ee999820a92dc`（与 `origin/main` 一致，已核对 `git ls-remote`） |
| 合并 | `git checkout main` → `git merge --no-ff feature/task-039-safebox-batch-restore -m "merge: task-039 safe_box 批量恢复 (#task-039)"` |
| **merge commit hash** | **`cbc5aaac1d1b033162c607a58955c1db1dbdb0ab`**（`--no-ff`，双亲：`2068fcd` + `daf7adf`） |
| 合并内容核对 | `git diff --stat daf7adf cbc5aaa` **为空**（合并结果树 = 已通过 CI 的分支树，无合并冲突/无夹带改动） |
| 推送 | `git push origin main` → `2068fcd..cbc5aaa  main -> main`（`origin/main` 已核验为 `cbc5aaa`） |
| 合并后 CI | run 38673720322（sha `cbc5aaa`）= **success**，main 由红转绿 |
| feature 分支 | `feature/task-039-safebox-batch-restore`（local + remote）在 main 转绿后删除 |
| revert | **无**（无严重问题，未触发回滚；未使用 `reset`，主干历史未被改写） |
| PR | #2（评审侧为取得分支 CI 状态创建）随合并自动关闭 |
| 评审报告 | 本次单独提交 `docs(review): 任务039评审报告 (#task-039)`，仅含 `docs/reviews/task-039-batch-restore-review.md` |

## 后续动作建议

- 派发 **task-040**：dry-run 预览 UI（前端「预览可清理项」按钮 → `cleanup.dryRun` → 大小汇总），PROJECT_STATE 下一步计划已就绪。
- 新建 **`bugfix/task-039-settings-ipc-redconsole`**：修已知问题 1 + 2（设置页红错 + 检查更新无超时收敛），建议并入下次发版前。
- CI 治理建议：`ci.yml` 目前只在 `push: main` 与 PR 上跑，**feature 分支推送无 CI**，导致“合并前看分支 CI”只能靠临时开 PR；建议加 `on: push: branches: ["feature/**"]` 或对 main 开分支保护（要求 PR + CI 必过），否则同类问题会反复出现（task-038 的 fmt 红灯正是漏网样例）。
- 打包冒烟建议：批量恢复的**真机端到端**本次已由评审侧造数据跑通（见清单 #6/#7/#11），建议后续把该造数据脚本固化进回归手册；安装包级（干净目录安装）冒烟仍按约束 8 在发版任务执行。
