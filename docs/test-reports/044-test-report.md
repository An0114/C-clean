# 测试报告：task-044 rules 手动回滚命令

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
|---|---|---|---|
| dev（CDP 真机） | release 二进制 + WebView2 CDP:9222 | 是 | 通过 |
| 打包安装（干净目录） | `target/release/cdisk-cleaner.exe` 运行验证 | 是 | 通过 |
| CI | PR #6 (draft) | 待确认 | 待用户确认绿 |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
|---|---|---|---|---|
| L1 单元 | cargo + pnpm | `cargo test -p engine` / `pnpm test` | 全绿 | cargo 全量通过（rules_rollback 3/3：rollback_restores_from_bak / rollback_no_bak_returns_err / rollback_corrupted_bak_returns_err）；pnpm 28 passed (5 files)；clippy 0；fmt 0；lint 0；build 172.68KB / gzip 54.92KB |
| L2 集成/契约 | 8 文件 diff | `git diff --name-only main HEAD` | 通过 | ① engine/src/rules/rollback.rs（新建 RollbackResult + has_backup + rollback）；② engine/src/rules/mod.rs（pub mod）；③ app/src-tauri/src/commands.rs（rules_has_backup / rules_rollback Tauri 命令）；④ app/src-tauri/src/lib.rs（注册）；⑤ app/src/pages/Settings.tsx（hasBak state + 回滚按钮 + confirm + 绿条 toast）；serde camelCase（previousVersion/restoredSha256/message）与前端 interface 逐字段对齐 |
| L3 端到端/UI | Settings 规则库卡片回滚按钮 | CDP :9222 → 设置 tab | 通过 | ① **无 .bak 时**：回滚按钮隐藏（按钮列表仅 7 个：5 tab + 检查更新 + 检查应用更新）；② **有 .bak 时**：按钮出现「回滚到上一版」（disabled=false）；③ 离线黄条渲染「离线模式：使用本地规则库 v1.0.0（规则库检查超时（10s））」（task-042 额外验证）；④ Console 0 red error / 0 exception |
| L4 真机/打包 | release 二进制 + 数据目录隔离 | `target/release/cdisk-cleaner.exe` | 通过 | 应用启动正常；DB 落 `%APPDATA%\com.and0114.cclean\data\cdiskcleaner.db`；rules 落 `%APPDATA%\com.and0114.cclean\rules\`；安装目录零用户数据写入 |
| L5 CI 门禁 | PR #6 | https://github.com/An0114/C-clean/pull/6 | 待确认 | draft PR open，head sha e7e1d34 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
|---|---|---|---|
| A1-A3 | rollback / has_backup 逻辑 | rollback.rs：has_backup 检查 cache_rules.json.bak 存在；rollback 读 .bak → 校验 → 覆盖 cache_rules.json → upsert DB → audit_log | 通过 |
| B1-B3 | 后端单测 | rules_rollback 3/3 passed（有 bak 恢复 / 无 bak 报错 / 损坏 bak 报错） | 通过 |
| C5 | 前端回滚按钮三态 | CDP 真机：无 .bak 隐藏 / 有 .bak 显示「回滚到上一版」/ 离线黄条正常渲染 | 通过 |
| D6-D8 | 不回归 | cargo 全量通过 + pnpm 28 passed | 通过 |

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
- **CI run 号待确认**：PR #6 draft 已开，CI 触发与绿结论待用户确认。
- **回滚操作本身未真机点击**：CDP 验证了按钮显示/隐藏和 UI 渲染，但未实际点击「回滚到上一版」触发 confirm + rollback 流程（避免修改真实 rules 文件）。rollback 逻辑由 rules_rollback 3 条集成测试覆盖（有 bak 恢复 / 无 bak 报错 / 损坏 bak 报错）。

## 7. 测试结论

- 结论：**通过**（L1-L4 全绿；回滚按钮显隐真机验证通过；离线黄条额外验证通过）
- feature 分支最新 commit：`e7e1d34`
- CI：PR #6 https://github.com/An0114/C-clean/pull/6（待确认绿）
- 是否可进入评审：**是**（待 CI 绿后最终确认）
- 移交说明：测试报告 `docs/test-reports/044-test-report.md`；CDP 真机证据：回滚按钮有 .bak 时显示「回滚到上一版」、无 .bak 时隐藏；Console 0 红错；rules_rollback 3/3 单测通过。
