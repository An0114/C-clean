# 评审报告：task-080 发版 v0.4.0（R5 收官 + P2 全部完成）

> 评审角色：Review Lead
> 评审时间：2026-11-07
> 分支：`feature/task-080-release-v0.4.0`
> HEAD：`9192873`
> **v0.4.0 = R5 架构债（5 个 ps1 Rust 重写）+ P2（多盘扫描 / 用户自定义规则 / 便携版 CLI）**

## 1. 输入核验

| 输入项 | 状态 | 事实核对 |
|--------|------|---------|
| 测试报告 | ✅ | `docs/test-reports/080-test-report.md`（96 行） |
| CI 运行号与结论 | ✅ | PR #45，run **42339760436**，conclusion **success** |
| feature 分支最新 commit | ✅ | `9192873` |

## 2. Git 提交记录核对结果

7 个提交，全部关联 `#task-080`，type/scope 规范，无混杂提交，无敏感文件。

## 3. 验收覆盖性核验

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---------|-------------|---------|------|
| 1-4 | 版本号 6 处全部 0.4.0 | CLI --version = 0.4.0 + 6 处文件确认 | 已核对 | 通过 |
| 5-7 | CHANGELOG v0.4.0 条目 | 四段（新增/架构优化/测试/已知问题） | 已核对 | 通过 |
| 8-10 | README + USER_GUIDE 同步 | 功能列表加多盘/自定义规则/CLI/便携版 | 已核对 | 通过 |
| 11-12 | cargo build / pnpm build 输出版本号 | clippy 输出 cdisk-cleaner v0.4.0 + NSIS 文件名 0.4.0 | 已核对 | 通过 |
| 13 | PR 合并后可打 tag v0.4.0 | 待 PR 合并后执行 | 待执行 | 待执行 |

## 4. 代码评审发现问题

第二档十项偏差经评审确认合理：
1. **当前版本不是 v0.1.7，是 v0.2.1**：任务书「现状」段写错。本次从 0.2.1 升到 0.4.0。
2. **src-tauri/ 实际在 app/src-tauri/**：任务书全文的路径都少了 app/。
3. **根 Cargo.toml 没有 version**：纯 workspace，不需要改。
4. **白名单漏了 3 个必改文件**：根 package.json / cli/Cargo.toml / Cargo.lock。
5. **版本号实际改了 5 个文件（6 处，含 Cargo.lock）**。
6. **CHANGELOG 日期写 2026-11-07**（实际准备日）。
7. **USER_GUIDE 顺带修正了 Dashboard 颜色阈值的原文档错误**：原文写「超过 85% 转橙，超过 95% 转红」，代码是 >=85% 红 / >=70% 橙。
8. **章节顺延**：USER_GUIDE 新增 12.2 + 第 15 章，交叉引用已同步。
9. **latest.json 未改**：CI 产物，手改无意义且会被覆盖。
10. **tag 未打**：按任务书由 DocsKeeper 在 PR 合并 main 后手动打。

关键证据正确：dev 已做 MSI per-user 静默安装实测（FileVersion=0.4.0 / 卸载成功）+ 便携版实测（portable_data 字符串出现 1 次 vs 安装态 0 次）。

## 5. 安全风险检查结果

纯版本号 + 文档改动，无代码风险。✅

## 6. 状态与文档一致性

- PROJECT_STATE.md 已更新 ✅
- GUI dev 真机未起（无前端代码改动）登记第三档 ✅
- CI run 42339760436 绿 ✅

## 7. 评审结论：**通过**

CI 绿、L1/L2 全绿、632 passed / 197 vitest。第二档十项偏差经评审确认为合理工程取舍。**v0.4.0 发版准备就绪。**

## 8. Bug 清单
无。

## 9. Git 操作记录

- 合并 feature/task-080-release-v0.4.0 → main（merge --no-ff）
- 删除 feature 分支

## 10. 后续动作建议

1. **PR 合并后打 tag v0.4.0** 触发 release.yml 出四个产物（NSIS / MSI / 便携版 zip / CLI exe）。
2. 下一阶段：UI 深色适配补完（Items / SafeBox / History，task-081）/ 规则导入导出。
3. 规则库 cache_rules.json 的 modules 段双份真值源问题建议 task-081 前拍板。
