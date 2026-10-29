# 评审报告：task-057 RmRestart 自动重启被关进程 + 设置页开关（默认关闭）

> 评审角色：Review Lead
> 评审时间：2026-10-29
> 分支：`feature/task-057-rm-restart-toggle`
> HEAD：`b585734`

## 1. 输入核验

| 输入项 | 状态 | 事实核对 |
|--------|------|---------|
| 测试报告 | ✅ | `docs/test-reports/057-test-report.md`（83 行） |
| CI 运行号与结论 | ✅ | PR #22，run **40816959128**，conclusion **success** |
| feature 分支最新 commit | ✅ | `b585734` |

## 2. Git 提交记录核对结果

14 个提交，全部关联 `#task-057`，type/scope 规范。一轮 bugfix 子分支已合并回 feature。

| commit | type(scope) | 内容 |
|--------|-------------|------|
| `eabfd5c` | docs(state) | 产品决策回填 |
| `ad80b2a` | docs(task) | 任务文档 |
| `50dfa6b` | feat(prefs) | 用户偏好存储 auto_restart_on_close |
| `f77e013` | feat(execute) | RmShutdown 后按开关调 RmRestart |
| `ceac435` | test(execute) | 适配 shutdown_locking_processes 新参数 |
| `7ec23ea` | feat(settings) | 设置页新增开关 |
| `154a2ef` | fix(ui) | confirm 文案追加自动重启提示 |
| `aa5b183` | style(engine) | cargo fmt |
| `4100d3d` | docs(task) | 实现备注 |
| `86620d5` | docs(state) | 任务完成 |
| `0da34ad` | fix(execute) | P0：RmRestart flags 误用修复 |
| `befca9e` | docs(state) | 缺陷登记 |
| `e308f9e` | merge(bugfix) | bugfix 合回 feature |
| `b585734` | docs(test) | 测试报告 |

## 3. 验收覆盖性核验

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---------|-------------|---------|------|
| 1-4 | RmRestart 自动重启逻辑 | 真机日志 + bRestartable 过滤 | 代码走查 locking.rs | 通过 |
| 5-7 | prefs 存储 + get/set 命令 | DB 实查 prefs.auto_restart_on_close='1' | 已核对 | 通过 |
| 8-11 | Settings 开关 + 持久化 + 重启回填 | 截图 settings3/installed-max/restart-settings-zoom | 已核对 | 通过 |
| 12-13 | confirm 文案根据开关追加 | 前端单测覆盖 | 已核对 | 通过 |
| 14-17 | 单测全绿 | lib 54 passed + 前端 85 passed | 已核对 | 通过 |

## 4. 代码评审发现问题

P0 缺陷 D1 已修复：
- RmRestart 第二参是保留位必须为 0，原代码把 PID 当 flags 传入导致 ERROR_INVALID_PARAMETER(87)
- 修复为一次 `RmRestart(handle, 0, None)`，由 RM 内核侧原生重启 bRestartable==TRUE 应用

P1 缺陷 D2 已修复：clippy field_reassign_with_default 改用结构体字面量。

无新增 bug。

## 5. 安全风险检查结果

Windows Restart Manager FFI 调用，签名按官方文档核实。无注入/越权风险。✅

## 6. 状态与文档一致性

- PROJECT_STATE.md 已更新 ✅
- 真机 notepad 自动重启未做成端到端点击链登记第三档 ✅
- CI run 40816959128 绿 ✅

## 7. 评审结论：**通过**

CI 绿、L1/L2/L3/L4 全绿、P0 缺陷已修复并闭环。

## 8. Bug 清单
无。

## 9. Git 操作记录

- 合并 feature/task-057-rm-restart-toggle → main（merge --no-ff）
- 删除 feature 分支

## 10. 后续动作建议

1. task-058 待定。
