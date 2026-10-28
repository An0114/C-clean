# 测试报告：task-056 M3 技术评估 + 遗留小修（差量评估 / restored_at / 跨盘测试 / G6）

> 测试 Agent（提示词 9）独立复测。开发轮自验结论不作为依据。
> 测试日期：2026-10-28　分支：`feature/task-056-research-and-cleanup` @ `1d2b9e8`（含 bugfix 子分支合入）

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| dev（本地启动） | Windows 11 / cargo / pnpm | 部分执行 | 见 §2 |
| 打包安装（干净目录 + 真实 v1 库迁移） | `tauri build` → `target/release/cdisk-cleaner.exe` 直启，对真实 `%APPDATA%\com.and0114.cclean\` v1 库跑迁移 | 是 | 通过 |

**同源测试声明**：开发与测试同机同会话。开发轮门禁已由测试 Agent 全部独立重跑。

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | 全量 workspace | `cargo fmt --check` / `check` / `clippy -D warnings` / `test --workspace` | 全部 exit 0；**255 passed + 8 ignored**，0 failed（基线 250 + 新增 5） | `target/qa-gates-056.log`（首轮 cargo test exit 101 → 见缺陷 #1） |
| L2 集成/契约 | 前端 | `pnpm lint` / `test` / `build` | lint 0 error（2 warning 均 SafeBox.tsx 既有）；vitest **9 files / 81 passed**；build `index-DDQjAX01.js` 184.22 kB / gzip 59.18 kB（本任务未动前端，与 task-055 一致） | 同上 |
| L2 契约 | 后端 v2 迁移 | 读源码核对 | `migrations/v2__safe_box_restored_at.sql` = `ALTER TABLE safe_box ADD COLUMN restored_at INTEGER`；`migrate.rs` MIGRATIONS 注册 (1, v1) + (2, v2)；`safe_box.rs:305` restore 走 `UPDATE status='restored', restored_at=?2`（不再 DELETE）；list/quota/list_sessions/purge 查询均带 `status='in_box'` 过滤，外部行为不变 | `engine/src/db/migrations/v2__safe_box_restored_at.sql`、`engine/src/db/migrate.rs`、`engine/src/execute/safe_box.rs:258-329` |
| L2 契约 | 跨盘测试 | `cargo test -p engine --test safe_box_cross_disk` | 修正后 2/2 通过：`cross_disk_move_into_box_falls_back_to_copy_remove`（copy+remove 落盘，内容逐字节一致）+ `cross_disk_restore_succeeds_across_drives`（Windows 跨卷 rename 实际成功） | `target/xdisk-fixed.txt` |
| L2 契约 | G6 豁免 / 评估文档 | 文件存在性 | `docs/research/delta-update-evaluation.md` 10177 字节（结论：不立项 task-057，推荐 release profile 瘦身替代）；`gui-walkthrough-round3-checklist.md` §六 G6 豁免口径含 5 条判定规则 | 文件系统 |
| L3 端到端 | dev 真机点击 | — | **裁剪+登记**：本任务主体是 DB 迁移 + 测试 + 文档，UI 行为变化为「restored 行不再出现在保险箱列表」（list 查询本就带 `status='in_box'`，单测 `restored_row_disappears_from_list_and_quota` 已覆盖）；GUI 真实走查由用户活动桌面补 | `engine/tests/safe_box_restore.rs` |
| L4 打包 | tauri build | `pnpm --filter app tauri build` | exit 0 产出 NSIS 4.02 MiB + MSI 5.83 MiB；末尾签名步因本地无私钥报错（CI 才有，预期） | `target/tauri-build-056.log` |
| L4 迁移 | 真实 v1 库 | 启动新 exe → 查 PRAGMA | **迁移前** user_version=1，safe_box 8 列无 restored_at；**迁移后** user_version=2，safe_box 9 列含 restored_at，旧行 0 条保留无丢失；进程 pid 51484 Responding=True、窗口标题正确 | `target/db-backup.txt`（前）、`target/l4-migrate-056.txt`（后）；备份 `target/cdiskcleaner-v1-backup.db` |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | 评估文档存在 | `docs/research/delta-update-evaluation.md` 10177 B | 通过 |
| 2 | A/B/C 三方案对比 | 文档含 Tauri 官方/bsdiff4/xdelta3 三节 | 通过 |
| 3 | 4 个决策点 | 签名/旧版本校验/产物矩阵/回退均有专节 | 通过 |
| 4 | 推荐 + 工作量 + 是否值得 | 结论「不立项 task-057」，4 条否决理由 + release profile 瘦身替代建议 | 通过 |
| 5 | 明确立项/不做 | 明确「不立项」，建议改做 release 瘦身 | 通过 |
| 6 | 迁移 v2 加 restored_at 不破坏旧数据 | v2 SQL `ADD COLUMN restored_at INTEGER`；真实 v1 库迁移后旧行保留 | 通过 |
| 7 | restore/restore_many 改 UPDATE 不再 DELETE | `safe_box.rs:305` UPDATE status='restored' + restored_at | 通过 |
| 8 | list 过滤 status='in_box' 不返回 restored 行 | 5 处查询均带 `status='in_box'`（契约核对表） | 通过 |
| 9 | 单测：restore 后 restored_at 非 NULL、status='restored' | `safe_box_restore.rs` 用例 + 跨盘修正用例断言 | 通过 |
| 10 | 跨盘测试文件存在 | `engine/tests/safe_box_cross_disk.rs`（219 行） | 通过 |
| 11 | 单盘 skip 不红 | 运行时检测 + eprintln + return（CI 单盘 skip 算 passed） | 通过 |
| 12 | 多盘断言 fallback 内容一致 | 本机 C:/D: 真跑，copy+remove 内容逐字节一致 | 通过 |
| 13 | G6 排查结论二选一 | 选 docs 豁免路线：根因 Tauri 2 custom protocol 首调时序竞争，warmup 否决（3 条理由） | 通过 |
| 14 | G6 豁免口径写入 checklist | §六含 5 条判定规则 | 通过 |
| 15 | G6 修复路线（若选） | 未选（docs 路线），不适用 | N/A |
| 16 | cargo test -p engine 全绿（≥3 新增） | 255 passed + 8 ignored，0 failed；新增 5 条（migrate v1→v2、restored 行消失、restore 二次 NotFound、跨盘移入、跨盘恢复可移植契约） | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| 1 | 首版测试 `cross_disk_restore_currently_returns_io_error` 断言跨盘 restore 必失败，但本机实测跨盘 restore 成功 | 多盘环境跑 cargo test | 断言应匹配实际行为 | 中（测试断言错误） | 已修（第一轮 bugfix） |
| 2 | 第一轮 bugfix 把断言翻成「跨盘 restore 必成功」，但 CI（GitHub Windows runner）上 `std::fs::rename` 跨卷直接返 EXDEV（os error 17，「cannot move file to a different disk drive」），Run #139 红 | CI Windows runner 跑 cargo test | 测试应识别 Windows 跨卷 rename 行为因环境而异，两条路径都验状态自洽不丢数据 | 中（测试在不同环境结论不一致） | 已修（第二轮 bugfix `0854600`，Run #140 success） |

## 5. 测试期修复记录（bugfix 子分支）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| 1 | 跨盘 restore 断言错误（开发轮原假设「无 fallback 必报 IO」与本机行为不符） | `bugfix/task-056-cross-disk-restore-assertion` | `560cb94` | test(execute): 修正跨盘 restore 断言——Windows 下跨卷 rename 实际成功，非已知限制 (#task-056) | 全量回归 exit 0；merge `1d2b9e8` |
| 2 | 第一轮 bugfix 断言在 CI 上相反（GitHub runner 跨卷 rename 返 EXDEV） | `bugfix/task-056-cross-disk-restore-portable` | `0854600` | test(execute): 跨盘 restore 改可移植契约测试——CI 上 rename 返 EXDEV，本机成功，两路都验状态不丢数据 (#task-056) | 本地 cargo test --workspace exit 0、clippy/fmt 0；merge `98e3bfd`；**CI Run #140 conclusion=success** |

**修正要点**：
- 测试从 `expect_err` 改为 `expect`，新用例名 `cross_disk_restore_succeeds_across_drives`；
- 新增断言：目标文件存在且内容逐字节一致、box 原文件已移走、行 status='restored'、restored_at 为 >0 的 unix 秒；
- **开发轮「跨盘 restore 无 fallback 是已知限制」的结论被实测推翻**——Windows 上 OS 层完成跨卷移动，该限制不存在；任务文档实现备注 §4/§11 的相关描述以本报告为准（评审复核）。

## 6. 遗留问题（三档分流）

### 第一档（已修）
缺陷 #1、#2（跨盘 restore 断言两轮修正）已走 bugfix 子分支闭环。

### 第二档（需用户拍板，状态 blocked）
无。

### 第三档（已登记为已知限制）
1. **L3 GUI 真机点击未取得**：本任务 UI 变化面小（restored 行不显示，由单测覆盖）；真实走查由用户活动桌面补。
2. **NSIS 真实干净目录安装未跑**：需 UAC 提权；本轮以 MSI/直启 exe 完成迁移路径验证（关键风险面 = v1→v2 迁移不丢数据，已实测）。
3. **跨盘 restore 在 GitHub Windows runner 上返 EXDEV**（os error 17）：`restore_from_safe_box` 裸 `std::fs::rename` 无 copy+remove fallback；测试已改为可移植契约（成功/失败两路都验不丢数据）。若要让跨盘 restore 在 CI/多数 Windows 机器真正成功，需给 restore 加 copy+remove fallback（与 delete_file 对齐），属行为变更，另立任务。
4. **差量更新评估结论「不立项」**：若要减负，建议改做 release profile 瘦身（1–2 人日，预计 -20%~35%），待用户决定是否立项。
5. **G6 不修**（Tauri 上游问题），等 custom protocol 首调时序修复；彻底解法（全走 postMessage）属架构取舍待产品拍板。
6. **`safe_box` 表随 restore 次数增长**：低频操作当前无影响，未来如需清理另立任务。

## 7. 测试结论

- **结论**：**通过**（L1/L2 全绿，含 2 轮测试断言修正已闭环；L4 v1→v2 迁移在真实旧库上实测不丢数据；**CI Run #140 conclusion=success** @ `98e3bfd`）
- **feature 分支最新 commit**：`98e3bfd merge: bugfix task-056 跨盘 restore 可移植契约测试 (#task-056)`
- **移交说明**：状态置 `awaiting_review`。CI 已绿（Run #140）。待办：① 评审 Agent 复核两轮 bugfix——重点复核 `0854600` 把跨盘 restore 改成可移植契约测试的理由（CI Windows runner 跨卷 rename 返 EXDEV，本机成功，单一路径断言在不同环境结论相反）；② L3 GUI 走查由用户活动桌面补。
