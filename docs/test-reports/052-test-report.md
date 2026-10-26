# 测试报告：task-052 GUI 走查 P1+P2 修复（G1-G5）

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
|---|---|---|---|
| dev（CDP 真机） | release 二进制 + WebView2 CDP:9222 | 是 | 通过 |
| 打包安装（干净目录） | `target/release/cdisk-cleaner.exe` 运行验证 | 是 | 通过 |
| CI | 待创建 draft PR | 待确认 | 待触发 |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
|---|---|---|---|---|
| L1 单元 | cargo + pnpm | `cargo test -p engine` / `pnpm test` | 全绿 | cargo exit 0（全部通过，含 scan_progress_percent / already_gone / audit_log_stats / execute_writes_source_task_id / list_pending_items 等新增测试）；pnpm 53 passed（7 files，含 Items.test.tsx）；clippy 0；fmt 0；lint 0；build 180.85KB / gzip 57.76KB |
| L2 集成/契约 | 21 文件 diff | `git diff --name-only main HEAD` | 通过 | ① engine/src/scan/orchestrator.rs + runner.rs（G1 脚本级百分比）；② engine/src/execute/list_pending.rs（G2 分页+稳定排序）；③ engine/src/execute/executor.rs + safe_box.rs + delete.rs + fs_deleter.rs（G3 source_task_id 接线 + G5 already_gone）；④ engine/src/audit_log/stats.rs（G4 COUNT(*)）；⑤ app/src-tauri/src/commands.rs（命令层透传）；⑥ app/src/pages/Items.tsx + Items.test.tsx（G2 前端分页栏） |
| L3 端到端/UI | Dashboard + Items | CDP :9222 | 通过 | ① Dashboard：清理轮次=17（G4 修复验证，修复前恒 0）；成功 146 / 失败 73 / 释放 1.3 GB；② Items 页：分页栏存在（hasPagination=true），表格 Session 列显示 #9010/#9009/#9008（G3 source_task_id 真机写入验证）；③ Console 0 red error |
| L4 真机/打包 | release 二进制 + 数据目录隔离 | `target/release/cdisk-cleaner.exe` | 通过 | NSIS 4.01MB + MSI 5.83MB 产出；AppData 73 文件 1.3GB；安装目录零用户数据写入；签名步骤因本地无 TAURI_SIGNING_PRIVATE_KEY 跳过（非代码缺陷） |
| L5 CI 门禁 | 待创建 draft PR | — | 待触发 | 分支尚未 push 到远端 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
|---|---|---|---|
| G1 | ScanOrchestrator 脚本级百分比，完成时 100% | scan_progress_percent 集成测试通过；cargo test exit 0 | 通过 |
| G2 | list_pending 分页 + 稳定排序 + 前端分页栏 | CDP 真机：Items 页 hasPagination=true，hasPageInfo=true；list_pending_items 测试通过 | 通过 |
| G3 | source_task_id 真实写入 + UI Session 列显示 | CDP 真机：表格 Session 列 #9010/#9009/#9008；execute_writes_source_task_id 测试通过 | 通过 |
| G4 | stats COUNT(*)，清理轮次不再恒 0 | CDP 真机：Dashboard 清理轮次=17（修复前恒 0）；audit_log_stats 测试通过 | 通过 |
| G5 | already_gone 语义，结果条单列 | already_gone 测试通过；deleter 返回 Ok(false) 表示 NotFound 幂等 | 通过 |

## 4. 缺陷清单
无。

## 5. 测试期修复记录（bugfix 子分支）
无。

## 6. 遗留问题（三档分流）

### 第一档（已修）
无。

### 第二档（需用户拍板，状态 blocked）
无。

### 第三档（已登记为已知限制）
- **CI 未触发**：分支尚未 push 到远端，draft PR 待创建。
- **G1 进度条真机渲染未验证**：扫描进度条百分比由 scan_progress_percent 集成测试覆盖，真机扫描过程中的进度条动画未 CDP 捕获（需触发完整扫描流程）。
- **G5 already_gone 真机文案未验证**：需触发「文件已被系统删除后清理」场景，由 already_gone 集成测试覆盖。
- **签名步骤跳过**：本地无 TAURI_SIGNING_PRIVATE_KEY，NSIS/MSI 产出但 .sig 未生成（CI 环境配置密钥后自动生成）。

## 7. 测试结论

- 结论：**通过**（L1-L4 全绿；G3/G4 真机验证通过；G1/G2/G5 由集成测试覆盖；Console 0 红错）
- feature 分支最新 commit：`fe22b2c`
- CI：待创建 draft PR 后确认
- 是否可进入评审：**是**（待 push + CI 绿后最终确认）
- 移交说明：测试报告 `docs/test-reports/052-test-report.md`；CDP 真机证据：Dashboard 清理轮次=17、Items 页分页栏+Session 列 #9010/#9009/#9008、Console 0 红错。
