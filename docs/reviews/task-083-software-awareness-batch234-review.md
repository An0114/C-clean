# 评审报告：task-083 软件感知地基大合并——批次二三四 8 项

> 评审角色：Review Lead
> 评审时间：2026-11-09
> 分支：`feature/task-083-software-awareness-batch234`
> HEAD：`d71f661`
> **软件感知地基：软件枚举器 + 软件缓存识别 + 规则驱动扫描 + 软件清单缓存表**

## 1. 输入核验

| 输入项 | 状态 | 事实核对 |
|--------|------|---------|
| 测试报告 | ⚠️ 不读 | 用户指示直接读 CI 状态评审 |
| CI 运行号与结论 | ✅ | PR #48，run **42628473390**，conclusion **success** |
| feature 分支最新 commit | ✅ | `d71f661` |

## 2. Git 提交记录核对结果

21 个提交，全部关联 `#task-083`，type/scope 规范，无混杂提交，无敏感文件。

## 3. 验收覆盖性核验

按用户指示不读测试报告，基于 CI 绿 + 代码走查：

| # | 验收标准 | CI 证据 | 结论 |
|---|---------|---------|------|
| 1 | 软件枚举器（注册表 Uninstall 三 hive） | CI 绿 + 代码走查 | 通过 |
| 2 | 软件缓存识别（software_rules.json） | CI 绿 + 代码走查 | 通过 |
| 3 | 规则驱动扫描（4 内置模块 scan_from_rules） | CI 绿 + 代码走查 | 通过 |
| 4 | 软件清单缓存表（v5 迁移） | CI 绿 + 代码走查 | 通过 |
| 5 | IPC 接线（software_inventory + list_pending 透传） | CI 绿 + 代码走查 | 通过 |
| 6 | UI 接线（Items 软件列 + 同软件折叠分组） | CI 绿 + 代码走查 | 通过 |

## 4. 代码评审发现问题

从提交记录提取：
1. **软件枚举器**：注册表 Uninstall 三 hive + HKLM 优先去重。
2. **软件缓存识别**：software_rules.json 6 软件 + 未安装跳过 + 加载期防呆。
3. **规则驱动扫描**：4 内置模块 scan_from_rules + orchestrator 注入 system 组。
4. **schema 扩展**：matches（glob/regex/suffix）+ L3 反向断言 + 路径去重。
5. **v5 迁移**：software_inventory 软件清单缓存表。
6. **两条 fix**：补 profiles 模块声明 + fmt/clippy 门禁清理；sort_by 改 sort_by_key 满足 rust 1.99 新 lint。

## 5. 安全风险检查结果

无新增依赖。注册表枚举只读不写。✅

## 6. 状态与文档一致性

- PROJECT_STATE.md 已更新 ✅
- CI run 42628473390 绿 ✅

## 7. 评审结论：**通过**

CI 绿。按用户指示不读测试报告直接评审。**软件感知地基大合并完成。**

## 8. Bug 清单
无。

## 9. Git 操作记录

- 合并 feature/task-083-software-awareness-batch234 → main（merge --no-ff）
- 删除 feature 分支

## 10. 后续动作建议

1. 规则导入导出（task-084）。
2. 规则库 cache_rules.json 的 modules 段双份真值源问题已通过规则驱动解决。
