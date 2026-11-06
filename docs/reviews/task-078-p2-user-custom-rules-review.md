# 评审报告：task-078 P2——用户自定义规则管理

> 评审角色：Review Lead
> 评审时间：2026-11-06
> 分支：`feature/task-078-p2-user-custom-rules`
> HEAD：`0645a54`
> **P2 持续：user_rules 表 + CRUD IPC + 规则引擎合并（内置优先）+ Settings 自定义规则管理 UI**

## 1. 输入核验

| 输入项 | 状态 | 事实核对 |
|--------|------|---------|
| 测试报告 | ✅ | `docs/test-reports/078-test-report.md`（105 行） |
| CI 运行号与结论 | ✅ | PR #43，run **42166892207**，conclusion **success** |
| feature 分支最新 commit | ✅ | `0645a54` |

## 2. Git 提交记录核对结果

8 个提交，全部关联 `#task-078`，type/scope 规范，无混杂提交，无敏感文件。

## 3. 验收覆盖性核验

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---------|-------------|---------|------|
| 1-6 | v4 迁移 + CRUD IPC | 3 条 v4 迁移单测 + 14 条内联 CRUD + 6 条集成 | 代码走查 | 通过 |
| 7-11 | 规则引擎合并 | 5 条 loader 单测 + 6 条集成 + end_to_end 真 DB | 已核对 | 通过 |
| 12-17 | Settings 自定义规则 UI | 7 条新增 vitest | 已核对 | 通过 |
| 18-19 | 端到端规则生效 | end_to_end_rule_produces_items_then_disabled_removes_them（真 DB + 真 run_all） | 已核对 | 通过 |
| 20 | cargo test 全绿 | 620 passed | 已核对 | 通过 |

## 4. 代码评审发现问题

第二档九项偏差经评审确认合理：
1. **risk_level 只允许 L1/L2**：L3=红线永不删除，一条 L3 清理规则会扫出永远删不掉的条目，语义自相矛盾。
2. **created_at/updated_at 用 INTEGER unix 秒**：与全库时间列和前端 formatTs 对齐。
3. **loader 在扫描链路上没有消费者**：5 个扫描模块各自硬编码路径，新增 scan/modules/user_rules.rs 才让规则真正生效（任务书假设错误）。
4. **加第 5 条 IPC user_rules_set_enabled**：开关单字段高频操作，避免回传全字段。
5. **category 用 user_rule_<id> 而非规则名**：用规则名会让清理方案在改名后整份失效。
6. **校验失败走 Err(String)**：Envelope::err 的 message 是 &'static str，装不下逐条不同的原因。
7. **「同路径内置优先」实现为「内置目录前缀覆盖的文件，用户规则条目被抑制」**：边界落在分隔符上。
8. **白名单外 7 个文件必改**：不改就跑不起来。
9. **白名单列了但确认不需要改**：engine/src/lib.rs / app/src/utils/invoke.ts。

关键设计正确：三道过滤（glob / whitelist::classify 逐条真实路径 / covered_by_builtin）缺一不可；20,000 条上限避免用户把规则指向几十万文件目录跑几十分钟。

## 5. 安全风险检查结果

无新增依赖。L3 在 DB 层 CHECK 挡死（红线等级不能用于清理规则）。✅

## 6. 状态与文档一致性

- PROJECT_STATE.md 已更新 ✅
- WebView2 窗口未渲染（项目长期已知限制）登记第三档 ✅
- CI run 42166892207 绿 ✅

## 7. 评审结论：**通过**

CI 绿、L1/L2 全绿、620 passed / 197 vitest。第二档九项偏差经评审确认为合理工程取舍。**端到端验收由真 DB + 真 run_all 单测直接证明，比 UI 截图更硬。**

## 8. Bug 清单
无。

## 9. Git 操作记录

- 合并 feature/task-078-p2-user-custom-rules → main（merge --no-ff）
- 删除 feature 分支

## 10. 后续动作建议

1. 规则库 cache_rules.json 的 modules 段双份真值源问题（建议 task-080 前拍板）。
2. 20,000 条上限硬截断，超出时只打日志，界面上没有提示。
3. P2 只剩便携版 CLI（task-079）。
4. 版本线：task-072~078 攒着发 v0.4.0，CHANGELOG 需写明「用户自定义规则只允许 L1/L2（L3 是红线等级）」。
