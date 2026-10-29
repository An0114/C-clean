# 评审报告：task-058 M4 收尾——RmRestart 白名单 + safe_box 原子性 + 签名评估 + CHANGELOG

> 评审角色：Review Lead
> 评审时间：2026-10-29
> 分支：`feature/task-058-m4-finishup`
> HEAD：`5125150`

## 1. 输入核验

| 输入项 | 状态 | 事实核对 |
|--------|------|---------|
| 测试报告 | ✅ | `docs/test-reports/058-test-report.md`（79 行） |
| CI 运行号与结论 | ✅ | PR #23，run **40828429998**，conclusion **success** |
| feature 分支最新 commit | ✅ | `5125150` |

## 2. Git 提交记录核对结果

10 个提交，全部关联 `#task-058`，type/scope 规范，无混杂提交，无敏感文件。

| commit | type(scope) | 内容 |
|--------|-------------|------|
| `8d3a393` | docs(task) | 任务文档 |
| `6bb5cb5` | feat(prefs) | RmRestart 白名单 + Settings 编辑框 |
| `69b4696` | test(prefs) | 白名单三态/通配/持久化用例 |
| `71f1650` | fix(execute) | safe_box 先 INSERT 再 rename 事务包裹 |
| `c87e5f1` | test(execute) | safe_box 原子性用例 |
| `29b32d6` | docs(research) | 代码签名技术评估 |
| `85bd0f5` | docs(changelog) | v0.1.7 草稿条目 |
| `f1e82ea` | docs(task) | 实现备注 |
| `5564a30` | docs(state) | 任务完成 |
| `5125150` | docs(test) | 测试报告 |

## 3. 验收覆盖性核验

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---------|-------------|---------|------|
| 1-5 | RmRestart 白名单 | 8 条 Rust 单测 + 真机 DB 落库 | 代码走查 prefs.rs | 通过 |
| 6-9 | safe_box 原子性 | 3 条原子性单测（真实外键约束） | 已核对 | 通过 |
| 10-12 | 代码签名评估 | code-signing-evaluation.md | 已核对 | 通过 |
| 13-15 | CHANGELOG v0.1.7 草稿 | CHANGELOG.md | 已核对 | 通过 |
| 16 | cargo test 全绿 | cargo --workspace exit 0 | 已核对 | 通过 |

## 4. 代码评审发现问题

- RmRestart 白名单：空数组=不限制，非空=fail-closed（全部命中才调 RmRestart）
- safe_box 原子性：INSERT→rename→commit 事务包裹，失败不留孤儿文件
- 代码签名评估：结论当前不做，三条触发条件
- CHANGELOG v0.1.7 草稿就绪

**第二档偏差（已评审确认可接受）**：任务书原写「非空=只重启白名单内进程名」，但 `RmRestart(hSession, dwFlags, callback)` API 不支持按 pid 过滤，一次调用会拉起本会话所有 bRestartable 应用。开发实现为 fail-closed（名单非空且被关进程全部命中才调 RmRestart），UI 文案已如实说明。这是 Windows RM API 的固有限制，fail-closed 是合理的工程取舍。

## 5. 安全风险检查结果

白名单 fail-closed 语义安全（默认不重启，非空时全部命中才重启）。safe_box 原子性防止孤儿文件。无注入/越权风险。✅

## 6. 状态与文档一致性

- PROJECT_STATE.md 已更新 ✅
- 真机 notepad/chrome 端到端白名单行为未跑登记第三档 ✅
- CI run 40828429998 绿 ✅

## 7. 评审结论：**通过**

CI 绿、L1-L4 全绿、无缺陷。第二档白名单语义偏差经评审确认为 Windows RM API 固有限制下的合理取舍，UI 文案已如实说明。

## 8. Bug 清单
无。

## 9. Git 操作记录

- 合并 feature/task-058-m4-finishup → main（merge --no-ff）
- 删除 feature 分支

## 10. 后续动作建议

1. 白名单 fail-closed 语义已在 UI 文案中说明，如产品认为不可接受需另立任务改为 CreateProcess 逐个重启。
2. task-059 待定。
