# 评审报告：task-042 rules 离线模式——checkUpdate 失败不影响本地规则库，UI 显示离线态

> 评审角色：Review Agent（提示词 10）
> 评审日期：2026-10-16
> 分支：`feature/task-042-rules-offline-mode` @ `1a4b9ce`
> PR：https://github.com/An0114/C-clean/pull/4（draft）

## 1. 输入核验

| 输入项 | 状态 | 说明 |
|---|---|---|
| 测试报告 `docs/test-reports/042-test-report.md` | **缺失** | 文件不存在，测试 Agent（提示词 9）尚未产出 |
| CI run 号与结论 | **缺失** | PR #4 为 draft，无 CI run 号 / conclusion 记录 |
| feature 分支最新 commit | `1a4b9ce` | docs(state): 任务042完成 |
| 任务文档 | 存在 | `tasks/042-rules-offline-mode.md` |

**输入核验结论：不通过**——测试报告缺失 + CI 证据缺失，触发一票否决。

## 2. Git 提交记录核对

```
b64e376 docs(task): 新增任务042-rules 离线模式 (#task-042)
0cf879e feat(rules): checkUpdate 返回 CheckUpdateResult 加 Offline 分支 (#task-042)
afbccff test(rules): 补离线模式 / UpToDate / NewVersion 三分支单测 (#task-042)
8ced0be feat(frontend): Settings 规则库卡片按 status 渲染三态（绿/蓝/黄） (#task-042)
1a4b9ce docs(state): 任务042完成，rules 离线模式降级就绪 (#task-042)
```

- 原子性：5 个提交，按 task → feat → test → feat → docs(state) 顺序，干净；
- type/scope：`feat(rules)` / `test(rules)` / `feat(frontend)` / `docs(state)` / `docs(task)` 规范；
- 关联：每条均带 `(#task-042)`；
- 变更范围：7 文件（remote.rs / mod.rs / commands.rs / Settings.tsx / rules_offline_mode.rs / PROJECT_STATE.md / 任务文档），全在白名单内，无越界；
- 敏感文件：无密钥 / .env / 用户数据入库。

## 3. 验收覆盖性核验

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---|---|---|---|
| A1 | CheckUpdateResult 三态（UpToDate/NewVersion/Offline） | 无测试报告 | 代码走查：`CheckStatus` enum + 三构造器 + `classify`，结构正确 | 待测试 Agent 补证据 |
| A2 | Tauri 命令返回 shape 同步改 | 无测试报告 | `commands.rs:rules_check_update` 返回 `Envelope<CheckUpdateResult>`，网络失败走 `Envelope::ok(offline(...))` 非错误信封 | 待补 |
| B1 | Offline 分支单测 | 无测试报告 | `rules_offline_mode.rs:check_update_offline_when_request_fails`（1s 超时，断言 status=Offline + message 含"超时"） | 代码存在，待跑 |
| B2 | UpToDate 分支单测 | 无测试报告 | `check_update_uptodate_when_same_version`（同 sha → UpToDate） | 代码存在，待跑 |
| B3 | NewVersion 分支单测 | 无测试报告 | `check_update_new_version_when_higher`（不同 sha → NewVersion + message 含版本号） | 代码存在，待跑 |
| C4 | 前端三态渲染（绿/蓝/黄） | 无测试报告 | Settings.tsx 三个条件块：offline→黄 #fef3c7 / newVersion→蓝 #dbeafe / upToDate→绿 #dcfce7 | 代码存在，待真机验证 |
| D5-D6 | 不改 scan/cleanup/applyUpdate | 无测试报告 | diff 中无 scan/cleanup 改动；applyUpdate 命令未动（偏差已记录） | 通过（代码走查） |
| L3 真机冒烟 | 无测试报告 | 缺失 | 待测试 Agent 补 |
| L4 NSIS 打包冒烟 | 无测试报告 | 缺失 | 待测试 Agent 补 |
| L5 CI | 无 run 号 | 缺失 | PR #4 draft，待 CI 全绿 |

## 4. 代码评审发现

**架构与设计**：
- `CheckStatus` enum 用 `#[serde(rename_all = "camelCase")]`，与前端 TypeScript union `"upToDate" | "newVersion" | "offline"` 逐字对齐 ✓
- `CheckUpdateResult` 字段：status/localVersion/localSha256/remoteVersion/remoteSha256/message，前后端 camelCase 一致 ✓
- 三个构造器 `up_to_date` / `new_version` / `offline` + `classify` 工厂方法，职责清晰 ✓
- 网络失败从 `Envelope::err(RULE_NETWORK_ERROR)` 改为 `Envelope::ok(offline(...))`——这是本任务核心行为变更，符合任务目标（断网不挡用户）✓

**错误处理与日志**：
- 离线分支保留 `eprintln!("[rules_check_update] fetch remote offline: {e}")` 日志 ✓
- `offline()` 构造器把 reqwest 错误文案透传到 `message`，前端直接展示 ✓

**遗留注意（非阻塞）**：
- 旧 `CheckUpdateResp` struct 未删除（任务文档偏差段已记录），仅 re-export 保留，无其他调用点——后续清理可做不阻塞。
- `applyUpdate` 命令的网络错误仍返回 `RULE_NETWORK_ERROR` 错误信封（任务范围外，不改）。

## 5. 安全风险检查（OWASP Top10）

- **注入**：无 SQL/Shell 注入面（不涉及）。
- **越权**：rules 外呼走固定 URL（`REMOTE_RULES_META_URL`），无用户输入拼接。
- **敏感信息泄露**：`eprintln!` 只打印 reqwest error，不含凭证。
- **反序列化**：远端 meta JSON 用 serde 反序列化，无 unmarshal 危险操作。
- **不安全默认**：离线模式降级是显式行为，不会静默吞错（message 透传给前端展示）。

**安全结论：无新增风险。**

## 6. 状态与文档一致性

- PROJECT_STATE.md 由 `1a4b9ce` 更新（docs(state) 提交），与代码同步。
- 任务文档状态字段仍为 `pending`（未翻 `review`/`testing`）——需测试 Agent 流转。
- 第三档遗留：updater token header / 仓库 public/private 留 task-043+，已在任务文档记录。

## 7. 评审结论：**不通过**

**否决原因（一票否决）**：
1. **测试报告缺失**：`docs/test-reports/042-test-report.md` 不存在，测试 Agent（提示词 9）未执行 L1-L5 分层测试。
2. **CI 证据缺失**：PR #4 为 draft，无 CI run 号 / conclusion=success 记录。
3. **真机/打包冒烟未执行**：任务文档明确标注前端任务 L3/L4 必做，当前仅有 DevAgent 自验（cargo/pnpm 编译通过），无 CDP 真机黄条证据、无 NSIS 干净目录安装证据。

**代码本身质量合格**：架构清晰、契约对齐、测试覆盖三分支、无安全风险。打回原因纯粹是**测试与 CI 证据未到位**，不是代码 bug。

## 8. 打回清单（回退至测试 Agent 提示词 9）

测试 Agent 需补做：
1. **L1 全量**：`cargo test -p engine`（含 rules_offline_mode 3/3）+ `pnpm test` + clippy/fmt/lint 全绿；
2. **L2 契约核对**：`CheckUpdateResult` serde camelCase ↔ 前端 interface 逐字段；
3. **L3 真机 CDP**：启动应用 → Settings → 点「检查更新」→ 断网时黄条「离线模式：使用本地规则库 vX.Y.Z」+ Console 0 红错；
4. **L4 NSIS 打包冒烟**：`pnpm tauri build` → 干净目录安装 → 断网 Settings 黄条 → 数据目录隔离；
5. **L5 CI**：PR #4 全绿，记录 run 号 + conclusion=success；
6. 产出 `docs/test-reports/042-test-report.md`，提交 `docs(test): task-042 测试报告 (#task-042)`。

**无 bugfix 子分支需求**——代码无需修改，纯补测试证据。

## 9. Git 操作记录

- 未合并 main。
- feature/task-042-rules-offline-mode 保留。
- main 未动（base = `334da58`，task-041 merge commit）。
- 本评审报告随本提交落盘。

## 10. 后续动作建议

1. 路由测试 Agent（提示词 9）补 L1-L5 全量测试 + 真机/打包冒烟 + CI 证据；
2. 测试报告产出 + CI 全绿后，重新进入本评审（提示词 10）；
3. 评审通过后再 `merge --no-ff` 到 main；
4. 循环刹车：若测试 Agent 补测后仍因同一缺口被打回，置 blocked 上报用户决策。
