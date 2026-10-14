# 测试评审报告：debt-cleanup-003/004 NSIS post-install bug 修复合并评审

## 本次任务名称
debt-cleanup-003/004：NSIS 打包后干净目录真机冒烟暴露的 post-install bug 修复（扫描 scope 参数 / DB 路径 / 前端 camelCase 对齐 / 设置页错误分类）

## Git 提交记录核对结果

✅ 8 个提交均符合原子提交规范：

| commit | type | 关联任务 | 内容 |
|--------|------|---------|------|
| `c5cf1d3` | fix(frontend) | #debt-cleanup-003 | pass scope='full' to scan_start |
| `16b30e2` | fix(tauri) | #debt-cleanup-003 | use app_data_dir() for DB/rules/safe_box, bundle resources |
| `2e52c5d` | docs(state) | #debt-cleanup-003 | 修复完成记录 |
| `4f12866` | docs(review) | #debt-cleanup-003 | 第二轮 NSIS 真机验收报告 |
| `16f0512` | fix(frontend) | #debt-cleanup-004 | align CleanupStats/ScanProgress/AuditLogEntry to camelCase |
| `00015c0` | fix(tauri) | #debt-cleanup-004 | classify rules update errors instead of collapsing to INTERNAL |
| `9335aaf` | docs(state) | #debt-cleanup-004 | 修复完成记录 |
| `47edd10` | docs(review) | #debt-cleanup-004 | 第三轮 NSIS 真机验收报告 |

每个提交单一 type、关联任务号、无混杂业务/文档/代码提交。✅

## 分步验证清单（每条必须有运行时证据）

| # | 验收标准 | 代码走查 | 运行时现象 | 结论 |
|---|---------|---------|-----------|------|
| 1 | NSIS 安装包产出并静默安装 | ✅ | `pnpm tauri build` 产出 3.62 MiB exe，`/S` 静默安装到 `C:\Program Files\C-Disk Cleaner\` | 通过 |
| 2 | Dashboard 打开不白屏 | ✅ | 标题 + 5 tab + 绿色扫描按钮 + 4 数字卡片渲染 | 通过 |
| 3 | 点开始扫描不红条 | ✅ | 按钮变「扫描中...」，14 秒后变「再次扫描」，无红条 | 通过 |
| 4 | 进度条实时走动 | ✅ | t=4s `C:\Windows\Temp\...—69 项`，t=6s `C:\Windows\SoftwareDistribution\...—4 项`，实时更新 | 通过 |
| 5 | 数字卡片有值 | ✅ | 总释放空间 `0 B`、成功 `0`、失败 `0`、轮次 `0`（全显示，不空白） | 通过 |
| 6 | 清理 tab 列出待清理项 | ✅ | 100+ 条，windows_update/temp/dev_cache 三模块，每条带路径/大小/Session #1 | 通过 |
| 7 | 保险箱空状态 | ✅ | 「共 0 项 / 0 B」「保险箱为空」 | 通过 |
| 8 | 历史空状态 | ✅ | 表头渲染，「暂无记录」 | 通过 |
| 9 | 设置页检查更新错误分类 | ✅ | 报「规则库更新失败，继续使用本地版本」，不再是「内部错误（附日志 ID）」 | 通过 |
| 10 | DB 可写（Bug-2） | ✅ | DB 在 `%APPDATA%\com.and0114.cclean\data\cdiskcleaner.db`（90KB），rules/safe_box 可写 | 通过 |
| 11 | Console 零红色 error | ✅ | CDP 收集 idle + 扫描 + 点检查更新后：零 error、零 exception | 通过 |

## 代码评审发现问题

无新增 bug。三轮 NSIS 真机验收共修复 10 个 bug（debt-cleanup-002 的 5 个 + debt-cleanup-003 的 2 个 P0 + debt-cleanup-004 的 3 个回归）。

**已知技术债务（第三档，不阻塞本次合并）**：
- 进度条 percent 字段后端发 0（单脚本无总数），进度条宽度不动，只有路径/项数文字更新——commit `9335aaf` 已显式登记为 deferred。
- 项目无 GitHub Actions CI（`.github/workflows/` 不存在），开发自验证为 clippy -D warnings / cargo test -p engine 137/0 / pnpm build 全绿。
- 清理→保险箱→恢复→历史的端到端有数据流程未在本轮实测（空状态正常）。

## 安全风险检查结果

- **OWASP A01 Broken Access Control**：perMachine 安装后普通用户对 `C:\Program Files\` 只读，DB 已移到 `%APPDATA%`（用户可写），无权限越界。✅
- **OWASP A03 Injection**：扫描路径来自 ps1 脚本输出 + rules 白名单，删除操作前有 resolve.rs 红线检查 + who-is-locking 检查。✅
- **OWASP A05 Security Misconfiguration**：rules 通过 SHA-256 校验后才使用；错误分类不泄露内部堆栈。✅
- 无硬编码密钥、无 SQL 拼接（rusqlite 参数化查询）、无 eval。✅

## 评审结论：**通过**

## Bug 清单
无。

## Git 操作记录
- merge commit：`0d569cc`（`merge: debt-cleanup-003/004 NSIS post-install bug fixes`）
- feature 分支：`feature/fix-m1-postinstall-v2` 已删除（本地 + 远程）
- 远程 main 已 push 到 `0d569cc`
- Revert 记录：无

## 后续动作建议
1. M1 核心链路完整跑通，可触发 task-034 release workflow 出 v0.1.1 正式安装包。
2. 补 GitHub Actions CI（clippy + cargo test + pnpm build），消除「无 CI」技术债务。
3. 下一轮 debt-cleanup 处理：清理→保险箱→恢复端到端实测、组件单测、进度条 percent 真实计算。
