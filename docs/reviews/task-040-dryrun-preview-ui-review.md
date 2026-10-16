# 评审报告：task-040 dry-run 预览 UI

> 评审角色：Review Lead（代码评审 + 验收覆盖性核验，不重复功能测试）
> 评审时间：2026-10-16
> 分支：`feature/task-040-dryrun-preview-ui`
> HEAD：`4af2af2 docs(test): 任务040测试报告 (#task-040)`

## 1. 输入核验（测试报告 / CI / commit hash）

| 输入项 | 状态 | 事实核对 |
|--------|------|---------|
| 测试报告路径 | ✅ 存在 | `docs/test-reports/040-test-report.md`（93 行，已读全文） |
| 测试报告结论 | ⚠️ 自报"通过（自动化层全绿，GUI/打包安装/CI 为第三档登记）" | 报告 §6 明确把 L3 dev GUI、L4 干净目录安装、CI 三项登记为第三档已知限制 |
| CI 运行号与结论 | ❌ **缺失** | 报告 §1 写"待触发"，§7 写"CI 运行号与结论：待 PR 触发"；仓库无本分支的 GitHub Actions run 记录 |
| feature 分支最新 commit | ✅ `4af2af2` | 与 `git log main..HEAD -1` 一致 |

**输入核验结论：不通过。** CI 运行号与结论缺失（任务文档 §测试要求明确"合并前必须有该分支的真实 run 结论，不接受口头'本地绿'"）。

## 2. Git 提交记录核对结果（原子性、规范符合性、变更范围）

6 个提交，全部关联 `#task-040`：

| commit | type(scope) | 内容 | 规范 |
|--------|-------------|------|------|
| `b114e44` | docs(task) | 任务文档 | ✅ |
| `d4da12b` | feat(frontend) | dryRun.ts 纯函数 + 类型 | ✅ |
| `21518f9` | test(frontend) | dryRun.test.ts | ✅ |
| `0cbb830` | feat(frontend) | invoke.ts apiRaw + invoke.test.ts | ✅ |
| `23ee272` | feat(frontend) | Items.tsx 预览按钮 + 面板 | ✅ |
| `4af2af2` | docs(test) | 测试报告 | ✅ |

**问题：缺少任务文档 §Git 提交要求第 5 条规定的 `docs(state): 任务040完成` 提交。**
- 任务文档要求："自验全绿后，最后一次提交：`docs(state): 任务040完成，dry-run 预览 UI 就绪 (#task-040)`，单独一次提交，只动 `docs/PROJECT_STATE.md`"
- 实际：`git diff main...HEAD -- docs/PROJECT_STATE.md` 为空——PROJECT_STATE.md 零改动
- 任务文档 `tasks/040-dryrun-preview-ui.md` 第 11 行状态字段仍是 `` `pending` ``，未流转到 `review`

**变更范围核对**：diff 仅含 7 个文件（dryRun.ts / dryRun.test.ts / invoke.ts / invoke.test.ts / Items.tsx / 测试报告 / 任务文档），Rust 侧零改动，无 package.json/pnpm-lock.yaml 改动。✅ 符合白名单。

## 3. 验收覆盖性核验（对照测试报告逐条 + 抽查复现）

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---------|-------------|---------|------|
| A1 | camelCase 类型对齐 | dryRun.ts 逐字段对照后端 serde | 代码走查：`moduleCode/category/bytes`、`itemId/path/denialReason`、`totalItems/totalBytes/breakdown/blockedByL3` 与 `engine/src/ipc/commands/dry_run.rs` 对齐 | 通过 |
| A2-A5 | 纯函数文案/排序/红线映射 | dryRun.test.ts 11 断言 | 代码走查 dryRun.ts：sortBreakdown 用 `[...rows].sort` 不改入参；describeDenial 先判 ResolveFailed 前缀再查表，未知串 `?? reason` 回退 | 通过 |
| A6/B3 | 单测断言数 | 28 断言全绿 | 未重跑（评审不重复测试），报告附命令与计数 | 通过 |
| B1/B2 | apiRaw 不抛 2002 / api 语义不变 | invoke.test.ts 5 断言 | 代码走查：api() 函数体零改动 hunk，仅 `interface Envelope` → `export interface Envelope`；apiRaw 直接 `return invoke<Envelope<T>>(cmd, args)` 不做 code 判断 | 通过 |
| C1-C2 | 预览按钮位置/禁用/调 dryRun | 代码审查（无真机截图） | 代码走查：按钮在「清理已选」左侧，蓝色；disabled 条件 `selected.size===0 \|\| cleaning \|\| previewing`；调 `apiRaw("cleanup_dry_run", { itemIds })` | 通过（代码层） |
| C3-C6 | 三码分支渲染 | 代码审查（无真机截图） | 代码走查：判别联合 Preview{ok/blocked/empty/error}，四分支各自渲染面板；2002 读 `env.data?.blockedByL3 ?? []`；reject 走 catch | 通过（代码层） |
| C7 | 只读保证 | 代码审查 + 后端 handle_dry_run 确认 | 代码走查：预览路径只调 cleanup_dry_run，不调 cleanup_execute | 通过（代码层） |
| C8 | 结果不陈旧 | 代码审查：4 处 setPreview(null) | 代码走查：load()/toggle()/cleanup()/全选 checkbox 都清预览 | 通过（代码层） |
| D1-D4 | Rust 零改动/无新依赖 | diff --stat | 已核对 | 通过 |
| **真机 Tauri + CDP 冒烟** | **任务文档 §测试要求："必做、不可裁剪"** | **报告 §6 登记为第三档，未执行** | **未执行** | **❌ 不通过** |
| **NSIS 干净目录安装冒烟** | **同上** | **报告 §6 登记为第三档，未执行** | **未执行** | **❌ 不通过** |
| **GitHub Actions CI 绿** | **任务文档 §测试要求："合并前必须有该分支真实 run 结论"** | **报告 §1/§7："待 PR 触发"** | **未执行** | **❌ 不通过** |

## 4. 代码评审发现问题

代码质量本身良好，无新增 bug：

- ✅ dryRun.ts 纯函数无副作用，sortBreakdown 返回新数组不改入参
- ✅ describeDenial 顺序正确（ResolveFailed 前缀先于查表），未知串回退不吞信息
- ✅ apiRaw 设计合理，api() 零破坏
- ✅ Items.tsx 用判别联合 Preview 类型，四分支渲染清晰
- ✅ C8 在 4 个入口都清预览
- ✅ 无新增 npm 依赖，Rust 侧零改动

**轻微观察（非阻塞）**：
- `describeDenial` 的 ResolveFailed 解析用正则去 Debug 括号/引号，对复杂嵌套字符串可能不完美，但未知串回退兜底，不构成 bug。
- Items.tsx 预览面板 key 用 `` `${row.moduleCode}/${row.category}` ``，理论上有重名风险，但后端 group by 保证唯一。

## 5. 安全风险检查结果（OWASP Top10）

- **注入**：无 SQL/命令注入风险（前端纯展示，后端参数化查询已有）。✅
- **不安全反序列化**：Tauri invoke 走 serde，无 eval/Function。✅
- **敏感信息泄露**：预览路径只读，denialReason 展示后端 Debug 串——这是预期行为（帮助用户理解为什么被拦），不暴露敏感路径（路径本身就是待清理项路径）。✅
- **越权**：无用户态概念。✅
- **只读保证（C7）**：代码层确认预览不调 cleanup_execute。✅

无安全问题。

## 6. 状态与文档一致性核对

| 核对项 | 事实 | 结论 |
|--------|------|------|
| PROJECT_STATE.md 是否更新 | `git diff main...HEAD -- docs/PROJECT_STATE.md` 为空 | ❌ 未更新（任务文档要求单独 docs(state) 提交） |
| 任务文档状态字段 | 第 11 行仍是 `` `pending` `` | ❌ 未流转到 review |
| 【已知问题】登记 | 测试报告 §6 把真机/打包/CI 登记为第三档，但任务文档说这些是"必做不可裁剪"，不应降级为已知限制 | ❌ 测试 Agent 无权把必做项降级 |
| 分支删除/合并状态 | feature 分支未合并，main 顶部 `524541e Update .gitignore` | ✅ 符合预期（评审未通过前不合） |

## 7. 评审结论：**不通过**

**一票否决项触发**：
1. 任务文档 §测试要求明确"真机冒烟与打包冒烟均为「必做、不可裁剪」，不得以「已有单测」「环境不便」为由裁剪"——测试报告把这两项 + CI 全裁剪，理由"自动化环境无法执行 GUI 点击"不成立（本仓库 task-035/037/038 均已用 CDP 完成真机冒烟）。
2. CI 运行号与结论缺失，任务文档明确"不接受口头'本地绿'"。
3. 缺少任务文档规定的 docs(state) 提交，PROJECT_STATE.md 零改动，任务文档状态字段未流转。

代码本身质量合格，但流程证据不完整，按约束不带病合并。

## 8. Bug 清单 + 修复建议 + bugfix 子分支建议名

本项不是代码 bug，是测试证据缺失 + 状态未流转。打回测试 Agent（提示词 9）补做，不需要 bugfix 子分支改业务代码。

| # | 现象 | 复现步骤 | 期望行为 | 严重度 |
|---|------|---------|---------|--------|
| R1 | 真机 Tauri + CDP 冒烟未执行 | 测试报告 §6 登记"自动化环境无法执行 GUI 点击" | 用 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222 --remote-allow-origins=*"` 启动，CDP Runtime.evaluate 点击「预览可清理项」，抓面板 innerText + Console error；覆盖 code=0 汇总 / code=2002 红线 / code=2001 空清单 / 刷新后清空 | 阻塞（必做项缺失） |
| R2 | NSIS 干净目录安装冒烟未执行 | 测试报告 §6 同上 | `pnpm tauri build` → perMachine `/S` 装到 Program Files → 清 DB 首启 → 切清理 tab → 预览按钮可点 → 面板渲染 → Console 零红错 → DB 落在 app_data_dir() | 阻塞（必做项缺失） |
| R3 | CI 未跑 | 报告"待 PR 触发" | 开 PR 到 main，观察 GitHub Actions（ci.yml）windows-latest job 五道闸全绿，附 run URL + run id + 结论 | 阻塞（CI 门禁） |
| R4 | PROJECT_STATE.md 未更新 | `git diff main...HEAD -- docs/PROJECT_STATE.md` 为空 | 补一个 docs(state) 提交：勾选 040 待办、追加变更轨迹、更新下一步计划 | 低（流程） |
| R5 | 任务文档状态字段仍 pending | tasks/040 第 11 行 | 流转到 `review` | 低（流程） |

**补做建议路径**：在当前 feature 分支上直接补做测试（不需要 bugfix 子分支，因为不改业务代码），补完后在测试报告里追加 §8 真机/打包/CI 证据段，然后重新进入本评审。

## 9. Git 操作记录（merge commit hash / revert 记录）

- **未合并**。feature/task-040-dryrun-preview-ui 保留待补测。
- main 未动。
- 无 revert。

## 10. 后续动作建议

1. **打回测试 Agent（提示词 9）**：在当前 feature 分支补做 R1-R3（真机 CDP 冒烟 + NSIS 干净目录安装 + 开 PR 跑 CI），R4-R5 由 DocsKeeper 同步更新状态。
2. 补做完成后，测试报告追加证据段（CDP innerText 截图/日志、NSIS 安装路径、GitHub Actions run URL），重新提交到 feature 分支。
3. 重新进入本评审（提示词 10），重点核对 R1-R3 证据是否齐备。
4. 评审通过后再 merge --no-ff 到 main。
5. **循环刹车提示**：本次是 task-040 第一次评审不通过，属于正常打回。若补测后再因同一缺失不通过，需上报调度 Agent 做根因决策（CDP 环境是否真不可用 / CI 配置是否需调整）。
