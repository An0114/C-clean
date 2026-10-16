# 测试报告：task-042 rules 离线模式——checkUpdate 失败不影响本地规则库

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
|---|---|---|---|
| dev（CDP 真机） | release 二进制 + WebView2 CDP:9222 | 是 | 通过 |
| 打包安装（干净目录） | `target/release/cdisk-cleaner.exe` 运行验证（NSIS 同 task-041 结构） | 是 | 通过 |
| CI | PR #4 (draft) | 待确认 | 待用户确认绿 |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
|---|---|---|---|---|
| L1 单元 | cargo + pnpm | `cargo test -p engine` / `pnpm test` | 全绿 | cargo ~153 passed 0 failed（rules_offline_mode 3/3, 1.09s）；pnpm 28 passed (5 files)；clippy 0；fmt 0；lint 0；build 172.01KB / gzip 54.78KB |
| L2 集成/契约 | CheckUpdateResult serde ↔ TS interface | git diff + 代码走查 | 通过 | 后端 `CheckStatus` enum camelCase（upToDate/newVersion/offline）+ `CheckUpdateResult` 六字段（status/localVersion/localSha256/remoteVersion/remoteSha256/message）；前端 TypeScript interface 逐字段对齐；Tauri 命令 `rules_check_update` 返回 `Envelope<CheckUpdateResult>`，网络失败走 `Envelope::ok(offline(...))` 非错误信封 |
| L3 端到端/UI | Settings 规则库卡片三态 | CDP :9222 → 设置 tab | 通过 | ① 五 tab 全部渲染；② Settings 规则库卡片显示绿条「规则库已是最新（v1.0.0）」（bg=#dcfce7 绿色，upToDate 分支）；③ 远端版本 1.0.0、状态"已是最新"；④ 按钮全部 enabled（检查更新/检查应用更新）；⑤ Console 0 red error / 0 exception |
| L4 真机/打包 | release 二进制 + 数据目录隔离 | `target/release/cdisk-cleaner.exe` | 通过 | 应用启动正常；DB 落 `%APPDATA%\com.and0114.cclean\data\cdiskcleaner.db`（57KB）；rules 落 `%APPDATA%\com.and0114.cclean\rules\`；安装/release 目录零用户数据写入 |
| L5 CI 门禁 | PR #4 | https://github.com/An0114/C-clean/pull/4 | 待确认 | draft PR open，head sha 6fbc085，CI run 号待用户确认 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
|---|---|---|---|
| A1 | CheckUpdateResult 三态（UpToDate/NewVersion/Offline） | rules_offline_mode.rs 3 测试全绿；CDP 真机绿条 upToDate 渲染正确 | 通过 |
| A2 | Tauri 命令返回 shape 同步改 | commands.rs: `rules_check_update` 返回 `Envelope<CheckUpdateResult>`，离线走 `Envelope::ok(offline(...))` | 通过 |
| B1 | Offline 分支单测 | `check_update_offline_when_request_fails`：1s 超时，断言 status=Offline + remote_version=None + message 含"超时" | 通过 |
| B2 | UpToDate 分支单测 | `check_update_uptodate_when_same_version`：同 sha → UpToDate + remoteVersion="1.0.0" | 通过 |
| B3 | NewVersion 分支单测 | `check_update_new_version_when_higher`：不同 sha → NewVersion + message 含版本号 | 通过 |
| C4 | 前端三态渲染 | CDP 真机：绿条「规则库已是最新（v1.0.0）」（#dcfce7）；黄条/蓝条由代码条件块渲染（#fef3c7/#dbeafe），Offline/NewVersion 真机渲染需断网/有新版本条件触发，由集成测试覆盖 | 通过（绿条真机验证；黄/蓝条代码+单测覆盖） |
| D5-D6 | 不改 scan/cleanup/applyUpdate | diff 中无 scan/cleanup 改动；applyUpdate 命令未动 | 通过 |

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
- **Offline 黄条真机渲染**：当前网络可达 GitHub，checkUpdate 返回 UpToDate（绿条），无法在不修改 hosts/网络环境的条件下触发 Offline 分支。Offline 分支由 `rules_offline_mode.rs:check_update_offline_when_request_fails` 集成测试覆盖（1s 超时断言），前端黄条渲染逻辑由代码条件块 + lint/build 验证。
- **NewVersion 蓝条真机渲染**：当前远端与本地同版本，无法触发 NewVersion 分支。由 `rules_offline_mode.rs:check_update_new_version_when_higher` 集成测试覆盖。
- **CI run 号**：PR #4 draft 已开，CI 触发与绿结论待用户确认。

## 7. 测试结论

- 结论：**通过**（L1-L4 全绿；绿条 UpToDate 真机验证通过；Offline/NewVersion 分支由集成测试覆盖；L5 CI 待 PR #4 确认绿）
- feature 分支最新 commit：`6fbc085`
- CI：PR #4 https://github.com/An0114/C-clean/pull/4（待确认绿）
- 是否可进入评审：**是**（待 CI 确认绿后最终确认）
- 移交说明：测试报告 `docs/test-reports/042-test-report.md`；CDP 真机验证绿条 upToDate 渲染正确、Console 0 红错；rules_offline_mode 3/3 单测通过；数据目录隔离通过。
