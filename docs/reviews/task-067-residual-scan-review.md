# 评审报告：task-067 M1 D 模块——残留增强（注册表卸载列表↔文件目录交叉比对）

> 评审角色：Review Lead
> 评审时间：2026-10-31
> 分支：`feature/task-067-residual-scan`
> HEAD：`21c05ee`

## 1. 输入核验

| 输入项 | 状态 | 事实核对 |
|--------|------|---------|
| 测试报告 | ✅ | `docs/test-reports/067-test-report.md`（83 行） |
| CI 运行号与结论 | ✅ | PR #32，run **41176533223**，conclusion **success** |
| feature 分支最新 commit | ✅ | `21c05ee` |

## 2. Git 提交记录核对结果

提交全部关联 `#task-067`，type/scope 规范，无混杂提交，无敏感文件。

| commit | type(scope) | 内容 |
|--------|-------------|------|
| `76f17a2` | feat(ui) | 残留列表页面 |
| `59b56b1` | feat(ui) | App tab 注册「残留」按钮 |
| `8f201ab` | docs(state) | 任务完成 |
| `434c23c` | docs(task) | 实现备注 |
| `21c05ee` | docs(test) | 测试报告 |

## 3. 验收覆盖性核验

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---------|-------------|---------|------|
| 1-4 | 残留扫描后端 | 22 条 rust 单测 + 开发真机 real_scan 数据 | 代码走查 | 通过 |
| 5-7 | IPC handler | commands.rs + lib.rs 注册 | 已核对 | 通过 |
| 8-13 | 页面渲染 | Residual.test 11 条 + App.test 1 条 | 已核对 | 通过 |
| 14-16 | Tab 切换 | dashboard.png 真机截图「残留」按钮 | 已核对 | 通过 |
| 17 | cargo test 全绿 | 405 passed | 已核对 | 通过 |

## 4. 代码评审发现问题

第二档四项偏差经评审确认合理：
1. **白名单 5 处偏差**：safe_box.rs 漏列（delete_dir 从 Err 桩实现为转调 delete_file，delete_file 对目录走 dir_total_bytes 递归求和修配额缺陷）、app commands.rs 漏、engine/src/lib.rs 无需改、invoke.ts 未改、App.test.tsx 漏。都是不做就编译不过的级联改动。
2. **「文件残留可勾选删除」实现为「移入保险箱 7 天可恢复」**（非直接删）：Program Files 不在 L3 红线内，白名单拦不住误判，必须留可恢复退路。
3. **类型 B 误报率无法归零**：初版 182 条误报，修正为只扫 Program Files 系列+13 个系统组件黑名单后压到 11 条。要提高准确率需引入文件指纹/厂商签名/版本资源比对，属新功能点。
4. **注册覆盖面有限**：只覆盖 Uninstall 三处（HKLM 64位/WOW6432Node/HKCU），不含 SystemComponent 隐藏项、不含微软商店应用。

无新增 bug。

## 5. 安全风险检查结果

文件残留移入保险箱 7 天可恢复（非直接删）。注册表残留仅提示无删除按钮。无注入/越权风险。✅

## 6. 状态与文档一致性

- PROJECT_STATE.md 已更新 ✅
- Residual 页面真机点入证据缺失登记第三档 ✅
- CI run 41176533223 绿 ✅

## 7. 评审结论：**通过**

CI 绿、L1/L2/L4 全绿、Tab 栏「残留」按钮真机可见。第二档四项偏差经评审确认为合理工程取舍。M1 D 模块落地。

## 8. Bug 清单
无。

## 9. Git 操作记录

- 合并 feature/task-067-residual-scan → main（merge --no-ff）
- 删除 feature 分支

## 10. 后续动作建议

1. Residual 页面待用户点 tab 自然验证。
2. task-068 F 模块注册表健康中心。
