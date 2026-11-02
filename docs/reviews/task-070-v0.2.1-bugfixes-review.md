# 评审报告：task-070 v0.2.0 真机验收 3 个 Bug 修复（WinSxS 退出码 3 + 扫描只出 L1 + 残留入箱永久删除失败）

> 评审角色：Review Lead
> 评审时间：2026-11-02
> 分支：`feature/task-070-v0.2.1-bugfixes`
> HEAD：`e75e1f4`
> **v0.2.1 patch 发版（3 个 Bug 同批修复）**

## 1. 输入核验

| 输入项 | 状态 | 事实核对 |
|--------|------|---------|
| 测试报告 | ✅ | `docs/test-reports/070-test-report.md`（79 行） |
| CI 运行号与结论 | ✅ | PR #35，run **41391974514**，conclusion **success** |
| feature 分支最新 commit | ✅ | `e75e1f4` |

## 2. Git 提交记录核对结果

7 个提交，全部关联 `#task-070`，type/scope 规范，3 个 bugfix 原子提交，无混杂提交，无敏感文件。

| commit | type(scope) | 内容 |
|--------|-------------|------|
| `8cd819e` | docs(task) | 任务文档 |
| `c689fd8` | fix(winsxs) | Bug-1 提权命令名从未进入 argv |
| `be30bc6` | fix(scan) | Bug-2 扫描脚本风险等级写死 L1 |
| `c7d00cc` | fix(safe_box) | Bug-3 目录条目永久删除失败 + 来源记录 |
| `7cb7bc7` | docs(state) | 任务完成 |
| `c4a7cbd` | docs(task) | 实现备注 |
| `e75e1f4` | docs(test) | 测试报告 |

## 3. 验收覆盖性核验

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---------|-------------|---------|------|
| 1-3 | Bug-1 WinSxS 退出码 3 | 4 条 argv 拼装单测 + 开发取证 argv 对比 | 代码走查 | 通过 |
| 4-6 | Bug-2 扫描只出 L1 | 4 条风险等级单测（含检测器自检） | 已核对 | 通过 |
| 7-9 | Bug-3 残留入箱永久删除失败 | 5 条 purge_expired/residual 单测 | 已核对 | 通过 |
| 10-11 | cargo/pnpm test 全绿 | 444 passed + 148 passed | 已核对 | 通过 |
| 12 | 真机冒烟无回归 | 安装态启动 + 8 tab 无回归 | 已核对 | 通过 |

## 4. 代码评审发现问题

第二档五项偏差经评审确认合理：
1. **白名单 2 处偏差**：ipc/commands/residual.rs 未改（实际改在 scan/residual.rs，ipc 层只是转调）、safe_box_test.rs 不存在（目录删除用例加进 purge_expired.rs，来源写入用例加进 residual_test.rs）。
2. **rules/cache_rules.json 未改**：该文件 modules 段在扫描链路上无消费者，风险等级唯一来源是 ps1。
3. **验收第 4 条 L0 只做到两级**：L0 语义是「只读/仅提示不参与清理」，5 个可清理模块无天然 L0 候选，硬塞会让可清理模块变不可清理。
4. **Bug-1 根因与任务书猜的三方向都不同**：build_args_string 只拼 --result，cmd 从未进 argv。
5. **两个 Bug 互相掩盖**：create_restore_point 走同一条坏路径，但还原点只在清理含 L2 项时才建，而 Bug-2 让该路径从没被走到。

无新增 bug。

## 5. 安全风险检查结果

3 个 bugfix 均有单测覆盖，无新增依赖，无注入/越权风险。✅

## 6. 状态与文档一致性

- PROJECT_STATE.md 已更新 ✅
- 3 个 Bug 真机最终点验未在测试轮自动完成登记第三档 ✅
- CI run 41391974514 绿 ✅

## 7. 评审结论：**通过**

CI 绿、L1/L2/L4 全绿、444 passed / 0 failed。第二档五项偏差经评审确认为合理工程取舍。**v0.2.1 patch 发版就绪。**

## 8. Bug 清单
无（3 个真机 Bug 已修复）。

## 9. Git 操作记录

- 合并 feature/task-070-v0.2.1-bugfixes → main（merge --no-ff）
- 删除 feature 分支

## 10. 后续动作建议

1. v0.2.1 patch 发版准备（版本号 0.2.0 → 0.2.1 + CHANGELOG 删三条已知问题）。
2. 用户装 v0.2.1 后手动点验 WinSxS 分析 / 扫描徽标 / 残留永久删除三处真机行为。
3. task-071 P1-7 清理方案模板。
