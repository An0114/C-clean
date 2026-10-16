# 评审报告：task-041 rules 外呼超时收敛 + updater endpoint 修复

> 评审角色：Review Lead
> 评审时间：2026-10-16
> 分支：`feature/task-041-rules-timeout-updater-endpoint`
> HEAD：`af8bc8d`

## 1. 输入核验

| 输入项 | 状态 | 事实核对 |
|--------|------|---------|
| 测试报告 | ✅ | `docs/test-reports/041-test-report.md`（58 行） |
| CI 运行号与结论 | ⚠️ 待补 | MCP create_pull_request 表单交互异常，PR 未自动创建；本地 cargo test ~149 passed + pnpm 28 passed + clippy/fmt/lint/build 全绿作为替代 |
| feature 分支最新 commit | ✅ | `af8bc8d` |

## 2. Git 提交记录核对结果

7 个提交，全部关联 `#task-041`：

| commit | type(scope) | 内容 |
|--------|-------------|------|
| `2be95e1` | docs(task) | 任务文档 |
| `2e99f0e` | fix(rules) | reqwest 10s/5s 超时 + 中文错误映射 |
| `54d991b` | test(rules) | 超时+404 单测 |
| `bfab9a8` | fix(tauri) | updater endpoint 改 Releases URL |
| `6c3650a` | fix(frontend) | Settings 错误态按钮恢复「重试」 |
| `d83b381` | docs(state) | 任务完成 |
| `af8bc8d` | docs(test) | 测试报告 |

原子提交，type/scope 规范，无混杂提交，无敏感文件。变更范围：remote.rs / rules_timeout.rs / Settings.tsx / tauri.conf.json / PROJECT_STATE.md / 测试报告 / 任务文档。✅

## 3. 验收覆盖性核验

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---------|-------------|---------|------|
| A1 | reqwest 10s/5s 超时 | remote.rs：`.timeout(10s).connect_timeout(5s)`，env 可覆盖 | 代码走查 build_client() | 通过 |
| A2 | 中文错误映射 | map_http_error 四档：超时/连接失败/404/其他 | 代码走查：is_timeout/is_connect/status 分档 | 通过 |
| A3 | 错误码信封 | 沿用 3003（偏差已记录） | 前端 code!==0 解包不依赖具体码值 | 通过 |
| B1-B2 | 超时+404 单测 | rules_timeout 3 passed 1.08s | 报告附 | 通过 |
| C6 | endpoint 改 Releases URL | tauri.conf.json：`releases/latest/download/latest.json` | 已核对 | 通过 |
| D8 | Settings 不卡转圈 | CDP 真机：按钮显示「重试」，错误文案可见 | 报告附 | 通过 |
| E9-E10 | 不回归 | cargo ~149 passed + pnpm 28 passed | 已核对 | 通过 |

## 4. 代码评审发现问题

代码质量良好：
- build_client() 提取超时配置，env var 可覆盖便于单测
- map_http_error 分类明确，不坍缩成「内部错误」
- with_url 变体便于单测注入本地 server
- Settings.tsx 按钮文案三态：检查中…/重试/检查更新

无新增 bug。

## 5. 安全风险检查结果

- reqwest 超时防止悬挂连接 ✅
- URL 硬编码 GitHub Releases（HTTPS）✅
- 无注入/越权/敏感信息泄露 ✅
- 无新依赖 ✅

## 6. 状态与文档一致性

- PROJECT_STATE.md 已更新 ✅
- CI run 待补（第三档登记）✅
- NSIS 静默安装 UAC 限制已登记 ✅

## 7. 评审结论：**通过**

代码质量好，本地全绿，L1-L4 真机已做。CI 因 MCP PR 表单交互异常未自动触发，登记第三档，合并后 main push 会触发 ci.yml push:main。

## 8. Bug 清单
无。

## 9. Git 操作记录

- 合并 feature/task-041-rules-timeout-updater-endpoint → main（merge --no-ff）
- 删除 feature 分支

## 10. 后续动作建议

1. 合并 push main 后观察 ci.yml push:main run 是否绿。
2. 发版前人工确认 updater endpoint Releases URL 可达。
3. task-042：仓库 public/private 决策。
