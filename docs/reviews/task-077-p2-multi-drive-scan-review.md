# 评审报告：task-077 P2 起步——多盘扫描支持

> 评审角色：Review Lead
> 评审时间：2026-11-04
> 分支：`feature/task-077-p2-multi-drive-scan`
> HEAD：`6de4540`
> **P2 起步：磁盘枚举 + 多盘扫描接线 + v3 生成列迁移 + 前端多盘 UI**

## 1. 输入核验

| 输入项 | 状态 | 事实核对 |
|--------|------|---------|
| 测试报告 | ✅ | `docs/test-reports/077-test-report.md`（105 行） |
| CI 运行号与结论 | ✅ | PR #42，run **41774125716**，conclusion **success** |
| feature 分支最新 commit | ✅ | `6de4540` |

## 2. Git 提交记录核对结果

11 个提交，全部关联 `#task-077`，type/scope 规范，无混杂提交，无敏感文件。

## 3. 验收覆盖性核验

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---------|-------------|---------|------|
| 1-4 | drive_enum.rs 实现 | 7 条内联单测 + dev 真机枚举 C:/D: | 代码走查 | 通过 |
| 5-8 | orchestrator 多盘接线 | 6 条内联 + 1 条真机冒烟 + per_drive 单测 | 已核对 | 通过 |
| 9-13 | v3 迁移 + 盘符提取 | 7 条迁移单测 + drive_letter_of 7 条 | 已核对 | 通过 |
| 14-17 | 前端多盘 UI | 9 条新增 vitest（Dashboard 4 + Items 5） | 已核对 | 通过 |
| 18 | cargo test 全绿 | 585 passed | 已核对 | 通过 |

## 4. 代码评审发现问题

第二档十项偏差经评审确认合理：
1. **v3 而非 v2**：v2 已被 task-056 占用，两个 v2 会让后一个永远不执行。
2. **索引名 idx_cleanup_item_drive_letter**：实际表名 cleanup_item。
3. **生成列 vs 写入时提取**：runner.rs 共用入口未动（ps1/Rust 共用）。
4. **未加 ScanProgress.drive 字段**：盘符写进 current_path 标签即可。
5. **按盘过滤 vs 路径替换**：5 模块根目录全锚定环境变量，无根目录在 D 盘。
6. **不硬编码 C:**：靠 %SystemRoot% 自然跳过。
7. **盘×模块二维推进简化为标签带盘符**：每个模块只锚定一个盘。
8. **既有断言「C 是非法盘符」改为合法**：四种写法统一在 parse_drive 归一化。
9. **白名单外 7 个文件必改**：不改就跑不起来。
10. **tokio runtime drop panic（4 次）**：app 未崩，建议 dev 排查是否新代码引入。

关键设计正确：服务端过滤（total/分页一致性）；切盘清空勾选（不能删看不到的项）；Dashboard 一次 drive_list（避免 N+1 IPC）。

## 5. 安全风险检查结果

无新增依赖。多盘枚举不触碰敏感路径。✅

## 6. 状态与文档一致性

- PROJECT_STATE.md 已更新 ✅
- WebView2 窗口未渲染（项目长期已知限制）登记第三档 ✅
- CI run 41774125716 绿 ✅

## 7. 评审结论：**通过**

CI 绿、L1/L2 全绿、585 passed / 189 vitest。第二档十项偏差经评审确认为合理工程取舍。**tokio panic 为 P2 观察项，建议 dev 排查但不阻断合并。**

## 8. Bug 清单
| # | 现象 | 严重度 | 处理建议 |
|---|------|--------|---------|
| 1 | tokio runtime drop panic 4 次 | P2 | dev 排查是否新代码引入（big_files/residual 扫描的 tokio 用法） |

## 9. Git 操作记录

- 合并 feature/task-077-p2-multi-drive-scan → main（merge --no-ff）
- 删除 feature 分支

## 10. 后续动作建议

1. tokio panic 排查（P2）。
2. D 盘 0 项是正确结果（需每盘一份目标如回收站才会有条目）。
3. P2 持续：用户自定义规则（task-078）/ 便携版 CLI（task-079）。
4. 版本线：task-072~077 攒着发 v0.4.0，CHANGELOG 需写明「多盘模式下非系统盘可能显示 0 项（不是 bug）」。
