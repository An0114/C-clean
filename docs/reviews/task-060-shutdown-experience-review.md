# 评审报告：task-060 占用关闭体验（RmShutdown 进度反馈 + 跨盘文档化）

> 评审角色：Review Lead
> 评审时间：2026-10-29
> 分支：`feature/task-060-shutdown-experience`
> HEAD：`c6c3bee`

## 1. 输入核验

| 输入项 | 状态 | 事实核对 |
|--------|------|---------|
| 测试报告 | ✅ | `docs/test-reports/060-test-report.md`（72 行） |
| CI 运行号与结论 | ✅ | PR #25，run **40881935064**，conclusion **success** |
| feature 分支最新 commit | ✅ | `c6c3bee` |

## 2. Git 提交记录核对结果

5 个提交，全部关联 `#task-060`，type/scope 规范，无混杂提交，无敏感文件。

| commit | type(scope) | 内容 |
|--------|-------------|------|
| `79e2c93` | test(execute) | ticker 用例 + 前端进度条用例 |
| `e85444a` | docs(guide) | 跨盘行为说明 + FAQ Q9 |
| `43018a1` | docs(task) | 实现备注 |
| `0836019` | docs(state) | 任务完成 |
| `c6c3bee` | docs(test) | 测试报告 |

## 3. 验收覆盖性核验

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---------|-------------|---------|------|
| 1 | 注册进程本身 | 开发核实 RM API 类型不支持，注释固化 | 已核对 | 通过（见第二档） |
| 2 | 父子进程都退出 | 既有 locking_shutdown.rs 单测 | 已核对 | 通过 |
| 3 | confirm 文案不变 | 未改 Items.tsx confirm | 已核对 | 通过 |
| 4 | RmShutdown 异步 ticker | spawn_shutdown_ticker + 2 条单测 | 已核对 | 通过 |
| 5 | 前端进度条「已等 Xs / 最多 30s」 | Items.test.tsx 用例覆盖 7s/31s | 已核对 | 通过 |
| 6 | 30s 超时放弃 | RmShutdown 无超时参数，改为继续等待+提示 | 已核对 | 通过（见第二档） |
| 7-8 | USER_GUIDE 跨盘行为说明 | 4.5 节 + FAQ Q9 | 已核对 | 通过 |
| 9 | cargo test 全绿 | engine 64 passed | 已核对 | 通过 |

## 4. 代码评审发现问题

第二档三项偏差经评审确认合理：
1. **注册进程本身**：RM API `rgapplications` 是 `RM_UNIQUE_PROCESS` 不是 pid，连带关闭兄弟进程本就是 RM 按 console 进程组的既定行为（task-049b 已实测）。强行注册反而扩大关闭面。
2. **30s 超时放弃**：`RmShutdown` 是同步阻塞 FFI，无超时参数、无法中断。改为超 30s 继续回调并提示「仍在等待」，ticker 只是观察者不阻塞删除流程。
3. **跨盘恢复无 fallback**：恢复直接返回 IO 错误（已有单测固化），文档按实际写，fallback 另立任务。

无新增 bug。

## 5. 安全风险检查结果

纯前端进度条 UI + 后端 ticker 观察者线程，无注入/越权风险。✅

## 6. 状态与文档一致性

- PROJECT_STATE.md 已更新 ✅
- 进度条真机触发未拍到登记第三档（需持锁+注册 RM 重启的进程）✅
- CI run 40881935064 绿 ✅

## 7. 评审结论：**通过**

CI 绿、L1/L2/L4 全绿。第二档三项偏差经评审确认为 Windows RM API 固有限制下的合理取舍。

## 8. Bug 清单
无。

## 9. Git 操作记录

- 合并 feature/task-060-shutdown-experience → main（merge --no-ff）
- 删除 feature 分支

## 10. 后续动作建议

1. 跨盘恢复 copy+remove fallback 另立任务。
2. `shutdown://progress` 并发化时需加 path/itemId 字段。
3. task-061 待定。
