# 评审报告：task-062 提权 exe 底座 + 系统还原点 + 系统托盘常驻（v0.2.0 起步）

> 评审角色：Review Lead
> 评审时间：2026-10-30
> 分支：`feature/task-062-elevation-srpoint-tray`
> HEAD：`efd507d`

## 1. 输入核验

| 输入项 | 状态 | 事实核对 |
|--------|------|---------|
| 测试报告 | ✅ | `docs/test-reports/062-test-report.md`（82 行） |
| CI 运行号与结论 | ✅ | PR #27，run **41014132486**，conclusion **success** |
| feature 分支最新 commit | ✅ | `efd507d` |

## 2. Git 提交记录核对结果

11 个提交，全部关联 `#task-062`，type/scope 规范。一轮 CI bugfix 已合并回 feature。

| commit | type(scope) | 内容 |
|--------|-------------|------|
| `4b66972` | docs(task) | 任务文档 |
| `3efe227` | feat(elevation) | 提权 exe 底座 |
| `f848c75` | feat(execute) | 清理前自动创建系统还原点 |
| `b128253` | test(execute) | L2 判定用例 |
| `15ab53a` | feat(tray) | 系统托盘常驻 |
| `2b505bc` | test(tray) | 开机自启用例 |
| `58bebc3` | docs(task) | 实现备注 |
| `5b6bf1e` | docs(state) | 任务完成 |
| `7bfbee4` | docs(test) | 测试报告 |
| `a2daf38` | fix(ci) | 资源契约步前先编译提权 exe |
| `efd507d` | merge(bugfix) | CI bugfix 合回 |

## 3. 验收覆盖性核验

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---------|-------------|---------|------|
| 1-6 | 提权 exe 底座 | elevation/ 独立 crate + 白名单 + 结果回传 | 代码走查 | 通过 |
| 7-11 | 系统还原点 | L2 判定 5 条单测 + 还原点名称格式 | 已核对 | 通过 |
| 12 | 关闭窗口拦截→隐藏托盘 | WM_CLOSE 后 pid 仍存活（真机证据） | 已核对 | 通过 |
| 13-14 | 托盘右键菜单+退出确认 | 代码已注册，真机未截图 | 已核对 | 通过（第二档） |
| 15-16 | 开机自启开关 | 注册表 Run 键 + Settings 3 条用例 | 已核对 | 通过 |
| 17 | cargo test 全绿 | 316 passed | 已核对 | 通过 |

## 4. 代码评审发现问题

第二档四项偏差经评审确认合理：
1. **托盘右键菜单未真机截图**：GDI 驱动菜单右键坐标不稳，代码已注册 3 项。
2. **退出二次确认未真机走通**：实现链路完整（托盘退出→emit→前端 confirm→invoke quit_app）。
3. **任务书 4 处技术偏差**：tauri-plugin-tray 不存在（Tauri 2 内置）、ShellExecuteExW 替代 ShellExecuteW、超白名单 8 文件（编译/打包必须）、app/src-tauri 路径。都是不做就编译不过的级联改动。
4. **自启状态不存 prefs**：以注册表为真实来源，避免任务管理器手动禁用后 prefs 打架。

无新增 bug。

## 5. 安全风险检查结果

提权 exe 命令白名单（只接受 create_restore_point），拒绝未知命令。UAC 单次触发。无注入/越权风险。✅

## 6. 状态与文档一致性

- PROJECT_STATE.md 已更新 ✅
- UAC/还原点/托盘菜单/退出 confirm 四项真机交互未触发登记第三档（需 L2 风险项）✅
- CI run 41014132486 绿 ✅

## 7. 评审结论：**通过**

CI 绿、L1/L2/L4 全绿、托盘关闭拦截真机实证、提权 exe 落安装目录。第二档四项偏差经评审确认为合理工程取舍。

## 8. Bug 清单
无。

## 9. Git 操作记录

- 合并 feature/task-062-elevation-srpoint-tray → main（merge --no-ff）
- 删除 feature 分支

## 10. 后续动作建议

1. UAC/还原点/托盘菜单/退出 confirm 四项真机交互待用户在活动桌面触发 L2 清理时自然验证。
2. task-063 待定。
