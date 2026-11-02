# 评审报告：task-073 P1 收尾——趋势图折线增强 + 开机自启/任务计划集成

> 评审角色：Review Lead
> 评审时间：2026-11-02
> 分支：`feature/task-073-p1-finish-linechart-autoscan`
> HEAD：`d9df652`
> **P1 体验项收官**

## 1. 输入核验

| 输入项 | 状态 | 事实核对 |
|--------|------|---------|
| 测试报告 | ✅ | `docs/test-reports/073-test-report.md`（97 行） |
| CI 运行号与结论 | ✅ | PR #38，run **41456282527**，conclusion **success** |
| feature 分支最新 commit | ✅ | `d9df652` |

## 2. Git 提交记录核对结果

9 个提交，全部关联 `#task-073`，type/scope 规范，含 1 条测试轮沙箱 flake 修复（`ab7b9cb`，未改产品代码），无混杂提交，无敏感文件。

| commit | type(scope) | 内容 |
|--------|-------------|------|
| `91b7515` | docs(task) | 任务文档 |
| `8fe502b` | feat(dashboard) | 趋势图折线增强 |
| `509a64c` | feat(settings) | 开机自启状态显示完善 |
| `f292e32` | feat(auto_scan) | 任务计划集成 |
| `b6dee9a` | feat(settings) | 后台自动扫描开关接线 |
| `299d40c` | docs(state) | 任务完成 |
| `0363ad7` | docs(task) | 实现备注 |
| `ab7b9cb` | test(auto_scan) | 沙箱 flake 修复 |
| `d9df652` | docs(test) | 测试报告 |

## 3. 验收覆盖性核验

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---------|-------------|---------|------|
| 1-5 | 趋势图折线 | Dashboard.test +6 条 | 已核对 | 通过 |
| 6-9 | 开机自启 | Settings.test +1 条回读优先 | 已核对 | 通过 |
| 10-13 | 任务计划集成 | 23 条后端单测（含跨 crate 静态检查） | 已核对 | 通过 |
| 14-17 | 自动扫描开关 | Settings.test +8 条 | 已核对 | 通过 |
| 18 | cargo test 全绿 | 491 passed | 已核对 | 通过 |

## 4. 代码评审发现问题

第二档八项偏差经评审确认合理：
1. **白名单 6 处偏差**：commands/lib.rs、auto_scan.rs、tauri.conf.json、Dashboard.test.tsx、Settings.test.tsx、App.test.tsx。都是不做就编译不过的级联改动。
2. **任务书「现状 7 天」写错**：task-061 实际是 30 天，保留 30 天为默认并加 7/90 选项。
3. **开机自启未重写**：task-062 已完整实现，只补状态说明与一条回读优先用例。
4. **`--auto-scan` 静默模式任务书没提但必须做**：否则 schtasks 每天拉起一个完整 GUI 窗口。
5. **频率可配置只到接口层**：固定 09:00，没做时间选择器。
6. **计划任务只支持 daily**：没有 weekly/monthly 触发器。
7. **Rust 侧注册表写入/删除无单测**：由 tauri_plugin_autostart 在运行时内完成。
8. **`visible:false` 全局启动路径改动**：正常启动窗口由 setup 显式 show()，理论上可能晚几毫秒出现。

无新增 bug。

## 5. 安全风险检查结果

schtasks 参数拼装防注入（数字+冒号）、退出码判断不解析本地化输出、注册后回读。无注入/越权风险。✅

## 6. 状态与文档一致性

- PROJECT_STATE.md 已更新 ✅
- 10 条真机冒烟未在测试轮完成登记第三档 ✅
- CI run 41456282527 绿 ✅

## 7. 评审结论：**通过**

CI 绿、L1/L2 全绿、491 passed / 180 pnpm。第二档八项偏差经评审确认为合理工程取舍。**P1 体验项全部收官。**

## 8. Bug 清单
无（测试轮 1 条沙箱 flake 已当场修复，未改产品代码）。

## 9. Git 操作记录

- 合并 feature/task-073-p1-finish-linechart-autoscan → main（merge --no-ff）
- 删除 feature 分支

## 10. 后续动作建议

1. 10 条真机冒烟待用户装 0.2.1 后手动点验（尤其「窗口能显示」和「schtasks /run 不弹窗」）。
2. 下一阶段：架构债（5 个 .ps1 → Rust）/ P2（多盘扫描 / 自定义规则 / 便携版 CLI）。
3. Items/SafeBox/History 三页深色适配待补。
