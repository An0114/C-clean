# 测试报告：task-051 保险箱时间区间筛选 / 多 session / 审计历史联表

> 测试角色：测试 Agent（提示词 9，独立于开发轮）｜ 测试时间：2026-10-25 13:14–14:30 ｜ 分支：`feature/task-051-safebox-filter-history`
> 测试起点 `ad404df`（开发轮交付态）→ 缺陷修复子分支 `bugfix/task-051-refresh-click-arg`（`ef054aa` + `05d5475`）→ `--no-ff` 合回 feature（merge `c1a1af6`）
> 任务书：`tasks/051-safebox-filter-history.md`（含开发轮「实现备注」）

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| dev（本地启动） | `pnpm tauri dev`；Windows 11；Node 24.14 / pnpm 11.9.0 / rust stable（`rust-toolchain.toml`）；vite 5.4.11 @ `localhost:5173` | 是 | **部分通过**：进程/`[paths]`/前端产物正常；**窗口渲染被本机 WebView2 崩溃阻塞**（第 4 轮复现，见 §6 第三档） |
| 浏览器引擎复核（补偿） | Edge `--headless=new`（Chromium）+ Vite dev server，`window.__TAURI_INTERNALS__.invoke` 打桩，加载真实 `SafeBox.tsx` | 是 | 通过（真浏览器渲染 + 真实交互，Console 0 错误） |
| 打包安装（干净目录） | `pnpm --filter app tauri build`（临时密钥 `%TEMP%\cclean-smoke-051.key`）→ NSIS `/S /D=%TEMP%\cclean-smoke-051` | 是 | 通过（9 文件布局、安装目录零写入、数据落用户目录） |
| CI | workflow `ci`：run **40168680216**（HEAD `7bfc130`，测试轮全部提交）+ run **40168310323**（`c1a1af6`，合并态纯代码） | 是 | 均 `success`（21 步全绿） |

> 说明：任务书 F26 写的是 NSIS 安装。开发轮因 `is_admin=False` 被 UAC 拦住改用 MSI per-user；**本轮 NSIS `/S /D=` 直接成功**（exit 0，无 `consent.exe` 残留），故按任务书原路径完成安装级验证。

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据（日志/截图/报告路径） |
| --- | --- | --- | --- | --- |
| L1 单元 | 全量 Rust（engine 188+8、app crate 9、elevation 等） | `cargo test --workspace` | **220 passed / 0 failed / 8 ignored**，exit 0（基线 197+8 → +23 与开发轮一致，无回退） | 命令输出（`test result: ok` × 30 个测试二进制汇总） |
| L1 单元 | 全量前端 | `pnpm --filter app test` → 默认 forks 池在**本会话沙箱**内写 `%TEMP%` 报 `EPERM`（环境问题）；改 `vitest run --pool=threads` | **46 passed（7 files）**，exit 0（基线 34 + 开发轮 11 + 本轮 1） | `SafeBox.test.tsx (12 tests)`、`Items.test.tsx (6)`、utils 28 |
| L1 单元 | 资源映射契约（task-050 防回归） | `node --test scripts/tauri-resources.test.mjs` | **6/6 pass** | 6 条用例名全绿 |
| L2 集成/契约 | 引擎集成：时间区间 6 + 多 session 6 + 历史 6 | `cargo test -p engine --test safe_box_time_range --test safe_box_multi_session --test safe_box_item_history` | **18 passed / 0 failed** | 用例名见 §3 A7/B10/C15 |
| L2 集成/契约 | 既有 pagination 5 + session_filter 4 | `cargo test -p engine --test safe_box_pagination --test safe_box_session_filter` | **9 passed**（仅改调用签名，断言未放宽，见 §4 说明） | `git diff main..HEAD -- engine/tests/safe_box_*.rs` |
| L2 契约核对 | 命令签名 ↔ 前端实参 / serde ↔ TS interface / Envelope 解包 | 见 §2.1 契约核对表 | 通过（0 处不一致） | 代码位置逐条列出 |
| L2 守卫有效性 | 变异测试 ×2（见 §2.2） | 临时改实现 → 跑用例 → `git checkout` 还原 | 变异 1 → **3 条红**；变异 2 → **5 条红**；还原后 18/18 绿 | 变异前后 `test result` 对比 |
| L3 端到端/UI | dev 启动 + 保险箱页四个新交互 | `pnpm tauri dev`；Edge headless 真浏览器 | 窗口壳正常、`[paths]` 正常；**渲染进程 3s 内起 4 个子进程、6s 归零** → GUI 点击不可用（第三档）；**真 Chromium 下四个交互全部走通** | 截图 `%TEMP%\cclean-test-evidence\051-safebox-probe.png`（24 小时筛选 + 历史展开）、`051-safebox-probe2.png`（全选 session）、DOM `051-safebox-dom.html` |
| L4 真机/打包 | 打包 → 干净目录安装 → 启动 → 数据落位 | 见 §2.3 | 通过（布局/零写入/数据目录/防劫持 4 项全过；GUI 交互仍受渲染进程崩溃限制） | `%TEMP%\cclean-test-evidence\051-inst-hash-before.txt`、`051-installed-stderr.txt`、`051-cdp3-stderr.txt` |
| L5 CI 门禁 | 本分支 CI 全绿 | GitHub Actions | 见 §7 | run id + 21 步明细 |

### 2.1 契约核对表（逐字段）

| 契约点 | 后端 | 前端 | 结论 |
| --- | --- | --- | --- |
| 列表命令名 | `safe_box_list`（`commands.rs`） | `api<SafeBoxPage>("safe_box_list", args)` | 一致 |
| 列表入参 | `page: u32, page_size: u32, session_ids: Option<Vec<i64>>, from_ts: Option<i64>, to_ts: Option<i64>` | `{ page, pageSize, sessionIds, fromTs }`（Tauri 2 camelCase↔snake_case 自动映射；`toTs` 前端不传 → `None`） | 一致（真浏览器实测实参：`{"page":1,"pageSize":50,"fromTs":1792819931,"sessionIds":[9001,9002]}`） |
| 列表出参 | `SafeBoxPage{items,total,page,pageSize,totalBytes}`（`#[serde(rename_all="camelCase")]`） | `interface SafeBoxPage{items,total,page,pageSize,totalBytes}` | 一致 |
| 条目字段 | `SafeBoxEntry{id,original_path,size_bytes,quarantined_at,source_task_id}` → `id/originalPath/sizeBytes/quarantinedAt/sourceTaskId` | 同名 5 字段，`sourceTaskId: number \| null` | 一致（`Option<i64>` → `null`） |
| 历史命令名 | `safe_box_item_history`（`lib.rs` 已注册） | `api<AuditLogEntry[]>("safe_box_item_history", { safeBoxId })` | 一致 |
| 历史入参 | `safe_box_id: i64` | `safeBoxId: item.id` | 一致 |
| 历史出参 | `Vec<AuditLogEntry>`，`eventType/path/detail/createdAt/result` | `interface AuditLogEntry{id,eventType,path,detail,createdAt,result}` | 一致 |
| 信封解包 | 所有命令 `Result<Envelope<T>, String>` | `api()` 在 `code!==0` 抛错、成功返回 `data` | 一致（页面错误分支显示 `加载失败：<msg>`） |
| 事件 payload | 本任务无新增事件 | — | N/A |

### 2.2 变异测试（证明新增用例不是"陪跑"）

| # | 变异内容 | 预期 | 实测 |
| --- | --- | --- | --- |
| 1 | `quarantined_at >= ?` 改成 `> ?`（闭区间下界变开区间） | 闭区间/下界用例红 | **3 failed**：`closed_interval_includes_both_bounds`、`only_from_returns_items_after_bound_and_total_matches`、`time_range_with_pagination_keeps_total_as_range_count`（`total 是全区间总数，不是全表 45`） |
| 2 | `COUNT(*)` 丢掉 `where_clause`（模拟「total 与筛选不一致」这个本任务最容易犯的错） | total 断言红 | **5 failed**：`two_sessions_return_only_those_items`、`nonexistent_session_returns_empty_page`、`single_existing_session_still_works`、`duplicate_session_ids_are_deduped`、`multi_session_with_time_range_and_pagination` |
| — | 还原源码 | 全绿 | `safe_box_time_range` 6 / `safe_box_multi_session` 6 / `safe_box_item_history` 6 = **18 passed** |

### 2.3 L4 打包安装逐项

| 检查项 | 实测 |
| --- | --- |
| `tauri build` | exit **0**；NSIS `C-DiskCleaner_0.1.0_x64-setup.exe` **4,202,727 B (4.01 MiB)** + `.sig`；MSI `6,107,136 B (5.82 MiB)` + `.sig` |
| NSIS 静默安装 | `/S /D=%TEMP%\cclean-smoke-051` → exit **0**，`INSTALLED=True`，`CONSENT_PROCS=0` |
| 安装目录布局 | **9 项**：`cdisk-cleaner.exe`(16,311,808) / `uninstall.exe`(79,461) / `rules\cache_rules.json`(614) / `rules\rules_meta.json`(261) / `scripts\scan_{temp,prefetch,browser_cache,dev_cache,windows_update}.ps1`（5 个） |
| task-050 回归防护 | `_up_` 目录 **不存在**；`RULES_OK=True`；`PS1_COUNT=5` |
| 安装目录零写入 | 启动前后 9 文件 SHA-256 全等 → **`INSTALL_DIR_NO_DIFF=1`** |
| 数据落用户目录 | `[paths] data_root=%APPDATA%\com.and0114.cclean rules_dir=…\rules scripts_dir=\\?\…\cclean-smoke-051\scripts`；启动前删除 `%APPDATA%\…\rules` → 启动后**重建 2 个文件**，与安装包内置**逐字节一致**（`same=True`） |
| 防 cwd 劫持 | 从诱饵目录 `%TEMP%\cclean-decoy-051`（内含假 `rules\cache_rules.json={"decoy":true}`）启动，仍定位安装目录 `scripts`，诱饵未被读取（内容原样） |
| 启动探针 | 进程存活 12s，`title='C-Disk Cleaner'`、`hwnd=592778`；WebView2 子进程 `0 → 4(t=6s) → 0(t=9s)`；Crashpad **无新增 dmp**；CDP 9222 仅瞬时 200（见 §6 第三档） |
| 卸载 | `uninstall.exe /S` → exit 0，安装目录已删除 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| A1 | `SafeBoxFilter{session_ids,from_ts,to_ts}` + `list_paged(..., &filter)` | `engine/src/execute/safe_box.rs:339-369`（结构 + `normalized()` / `is_empty_range()`）、`:450` 新签名；调用方 `commands.rs` 构造 filter | 通过 |
| A2 | 动态 WHERE，值一律占位符绑定 | `:473-494`：`clauses.push("source_task_id IN (?, …)")`、`quarantined_at >= ?` / `<= ?`；参数 `Vec<i64>` + `params_from_iter`；SQL 里无字符串插值（仅占位符数量拼接） | 通过 |
| A3 | items / COUNT / SUM 同一 WHERE 与参数 | `:496-528`：三条语句共用 `where_clause` 与 `filter_params`；变异 2 证明断言能抓住不一致（5 条红） | 通过 |
| A4 | 闭区间 `>= from` `<= to`，单边也生效 | `closed_interval_includes_both_bounds`（`ts==[400,300,200]`，两端都含）；`only_from…` / `only_to…` 各 3 条 | 通过 |
| A5 | `from>to` → 空页不 Err；`<=0` 视为未设置 | `from_greater_than_to_returns_empty_page_not_error`（`items=[] total=0 totalBytes=0 page=2 pageSize=20`）；`non_positive_bounds_are_ignored`（5 条全返） | 通过 |
| A6 | 分页 clamp 与 `ORDER BY id DESC` 不变 | `safe_box_pagination` 5 条全绿（`page_size 99999→200`、`id DESC` 断言原样保留） | 通过 |
| A7 | `safe_box_time_range.rs` ≥5 条 | **6 条**：`only_from_returns_items_after_bound_and_total_matches`、`only_to_returns_items_before_bound_and_total_matches`、`closed_interval_includes_both_bounds`、`from_greater_than_to_returns_empty_page_not_error`、`non_positive_bounds_are_ignored`、`time_range_with_pagination_keeps_total_as_range_count` | 通过 |
| B8 | 空=不过滤；非空 `IN (...)`；去重 | `empty_session_ids_equals_no_filter`（9 条全返）、`duplicate_session_ids_are_deduped`（重复 id 仍 3 条）；`normalized()` 保序去重 | 通过 |
| B9 | 与时间区间可叠加 | `multi_session_with_time_range_and_pagination`（首页 3 + 次页 1，两页 `total=4`、`totalBytes=40`） | 通过 |
| B10 | ≥4 条 | **6 条**（见 §2 L2 行） | 通过 |
| B11 | 既有 9 条只改签名 | diff 仅 `None`→`&SafeBoxFilter::default()` / `Some(n)`→`filter`，断言逐字未动；`list_sessions_groups_by_session` 未触碰 | 通过 |
| C12 | `SafeBoxEntry.source_task_id` → `sourceTaskId`，两条路径都填 | `list_safe_box`(`:405,418`) 与 `list_paged`(`:515,537`) 均 SELECT 该列；真浏览器渲染出 `#9001` 与 `—` | 通过 |
| C13 | `list_by_target(conn,target,limit)`，NOCASE，limit clamp [1,100] | `engine/src/audit_log/query.rs:88-110`：`WHERE target = ?1 COLLATE NOCASE ORDER BY ts DESC, id DESC LIMIT ?2`，`limit.clamp(1,100)`；`path_match_is_case_insensitive` 通过 | 通过 |
| C14 | `item_history(conn,safe_box_id)`，不存在 → Err 含 not found | `safe_box.rs:602-612`，先取 `original_path` 再 `list_by_target(…,20)`；`missing_item_returns_not_found_error` 断言 `err.contains("not found")` | 通过 |
| C15 | ≥4 条 | **6 条**：`returns_history_of_same_path_in_ts_desc_order`（`ts==[300,200,100]`）、`path_match_is_case_insensitive`、`unrelated_paths_are_not_returned`、`no_history_returns_empty_vec_not_error`、`missing_item_returns_not_found_error`、`limit_caps_result_to_20` | 通过 |
| D16 | 4 档时间预设，`fromTs=now-N`，切页重置+清空选中 | 真浏览器：点「24 小时」后实参 `{"page":1,"pageSize":50,"fromTs":1792819931}`；jsdom `点「24 小时」带 fromTs（>0）且重置到第 1 页`、`筛选变化后清空已选` | 通过 |
| D17 | 多选 session + 全选/清空 | 截图 2：`已选 2 个 session`，实参 `sessionIds:[9001,9002]`；`「清空」后不再传 sessionIds` 通过 | 通过 |
| D18 | 来源列 `#N` / `—`，宽度 ≤90 | `SafeBox.tsx:534`（`width:90`）、`:555-557`；截图可见 `#9001` 与 `—` | 通过 |
| D19 | 行内历史展开：中文化/加载中/空/失败/单行展开/筛选清空 | 截图 1：展开行渲染「移入保险箱 / 扫描 / 永久删除」+ 时间/结果/详情；jsdom 覆盖空态（`该路径暂无审计记录`）与再次点击收起；`load()` 首行即 `setHistory(EMPTY_HISTORY)` | 通过 |
| D20 | 既有交互不破坏；刷新沿用筛选 | **测试期发现缺陷并已修**：`onClick={load}` 把 click 事件当 `page`；修复后 `刷新时 page 仍是数字，且沿用当前筛选条件` 通过；三态互斥（`restoringId/purgingId/batchPurging/batchRestoring`）代码未改 | 通过（修后） |
| D21 | `SafeBox.test.tsx` ≥6 条 | **12 条**（开发轮 11 + 本轮 1） | 通过 |
| E22 | 不新增依赖、不动锁文件 | `git diff main..HEAD -- Cargo.lock pnpm-lock.yaml app/package.json app/src-tauri/Cargo.toml` → **空** | 通过 |
| E23 | 只改允许文件 | 改动集 = `commands.rs`/`lib.rs`/`SafeBox.tsx`/`SafeBox.test.tsx`/`engine/src/execute/safe_box.rs`/`engine/src/audit_log/query.rs`/5 个 engine 测试/`docs/PROJECT_STATE.md`/`tasks/051-*.md`；`grep` 禁止清单 → **未触碰** | 通过 |
| E24 | 七条门禁 | fmt 0 / clippy 0（42 条 Windows 增量锁清理告警，非 lint）/ cargo test 220+8 / lint 0 error 2 warning / vitest 46 / build `179.94 kB │ gzip 57.32 kB`（基线 176.39，+3.55 kB 在 ±5 内）/ 契约 6/6 | 通过 |
| F25 | dev 真机交互 | **未取得**（渲染进程崩溃）；补偿：真 Chromium 全交互 + jsdom 12 条（§6 第三档） | 第三档（环境） |
| F26 | 安装包冒烟 | `tauri build` exit 0 + NSIS 干净目录安装 9 项布局 + 启动探针（§2.3） | 通过 |
| F27 | 造数据 | 本轮未向用户库写入任何测试行（复用 `%APPDATA%` 真实库，仅读取）；开发轮的构造数据已自行清理 | N/A |
| F28 | GUI 缺口登记 | §6 第三档，含窗口存活时长 / WV2 进程曲线 / dmp / CDP 探测结果 | 已登记 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| 1 | 保险箱页点「刷新」→ 传 `page` 变成 React 事件对象：真实环境下 `invoke` 序列化失败 → 页面显示「加载失败」；jsdom 下 `setPage(event)` 直接让 React 渲染对象、整树崩（白屏） | 打开保险箱页 → 点右上「刷新」 | 按当前筛选条件重拉，`page` 仍是数字 | 中（主页刷新不可用；可切 tab 绕过） | **已修**（`bugfix/task-051-refresh-click-arg`） |

说明：该缺陷**不是本任务引入**（`onClick={load}` 自 task-045 起就存在于 `main`），但落在任务书 D20「刷新按钮沿用当前筛选条件」的验收范围内，且修复为单行、无接口语义变更，按第一档当场修掉。`Items.tsx` / `Dashboard.tsx` 同名写法已核查：两处 `load()` 无参，不受影响。

## 5. 测试期修复记录（bugfix 子分支）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| 1 | 刷新按钮把 click 事件当 `page` 传入 | `bugfix/task-051-refresh-click-arg`（从 `ad404df` 拉出） | `ef054aa` | `fix(ui): 刷新按钮不再把 click 事件当作 page 传入 (#task-051)` | 缺陷态跑新用例 → **1 failed（`expected 'object' to be 'number'`）**；修复态 → **12/12 通过**；全量回归见下 |
| 1 | 补回归用例 | 同上 | `05d5475` | `test(ui): 覆盖刷新按钮实参与筛选条件保持 (#task-051)` | 同上 |

- 合回：`git merge --no-ff` → merge commit **`c1a1af6`**；`git diff 05d5475 HEAD` **为空**（合并态与 bugfix tip 完全一致）；子分支已删（本地 + 远端）。
- 全量回归（合并后，feature 分支）：cargo test **220 passed / 8 ignored / 0 failed**；vitest **46 passed（7 files）**；lint 0 error；build 179.94 kB；契约 6/6；引擎 18 条新增用例全绿。
- CI 触发说明：`.github/workflows/ci.yml` 只在 `push: main` 与 `pull_request` 触发，**单独 push bugfix 分支不会触发 CI**（无对应 PR）。因此按「合并后由 draft PR #16 触发」执行，CI 覆盖的正是最终合并态（run 40168310323 @ `c1a1af6`），门禁语义不变。

## 6. 遗留问题（三档分流）

### 第一档（已修）
- 缺陷 1（刷新按钮实参污染）——已按 §5 闭环。

### 第二档（需用户拍板，状态 blocked）
- 无。本次无 TRD/PRD 口径冲突、无接口语义变更、无方案取舍。

### 第三档（已登记为已知限制）

1. **GUI 点击取证（第 4 轮仍不可得）**
   - 缺失条件：本机 WebView2 渲染进程无法稳定存活，无活动桌面/远程会话。
   - 本轮实测（dev 与安装版均复现）：
     - dev：`[paths] scripts_dir=D:\GitHub\C-clean\app\src-tauri\../..\scripts` 正常；`title='C-Disk Cleaner'`、`hwnd=198844`、`responding=True`；WebView2 子进程 **0**；Crashpad 新增 `13:27:44` 6.4 MB `.dmp`；CDP 9222 `NETSTAT_9222_HITS=0`。
     - 安装版：`title='C-Disk Cleaner'`、`hwnd=592778`；子进程 `0 → 4(t=6s) → 0(t=9s)`；本轮**无新增 dmp**；`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` 时 CDP **瞬时可用**（`t=1.5s` → HTTP 200 / 460 B），但随即 attach 后 `Runtime.evaluate` 超时、target 停在 `about:blank`，`Page.captureScreenshot` 无法完成。
   - 现有补偿（均已实跑，非推演）：
     - **真 Chromium 渲染**：Edge `--headless=new` 加载 Vite dev server + 真实 `SafeBox.tsx`，stub `__TAURI_INTERNALS__.invoke`。截图 2 张（时间预设高亮 / 历史展开行 / 来源列 / 分页栏）、DOM dump 1 份；IPC 调用序列 7 条（含 `fromTs`、`sessionIds:[9001,9002]`、`safeBoxId:71`）；`console.error / onerror: 0`。
     - **jsdom 组件用例 12 条**：四个新交互 + 空态 + 收起 + 刷新实参。
     - **引擎集成 18 条**：筛选语义、`total` 一致性、NOCASE 命中、边界。
     - **安装级文件系统证据**：布局 / 零写入 / 数据目录重建 / 防 cwd 劫持。
   - 影响面：仅「人眼确认视觉细节」与「真实 IPC 往返下的 Console 读数」；筛选/历史/刷新的数据语义已被上述三层覆盖。
   - 建议：发版前在有活动桌面的机器补一次 GUI 走查（含 DevTools Console 零红）。

2. **前端 `pnpm --filter app test` 默认 forks 池在本会话沙箱内 `EPERM`**
   - 现象：vitest worker 写 `%TEMP%\<rand>\web\<hash>` 报 `EPERM: operation not permitted`，导致部分测试文件未被收集（本轮观察到 3/4/5/6 个文件等不同子集）。
   - 规避：`vitest run --pool=threads` → 稳定 7 files / 46 tests。
   - 判定为**会话沙箱限制**，非代码问题：同一提交在 CI（干净 runner）上「前端测试」步骤 success（run 40163901414 第 14 步），且 `--pool=threads` 与默认池用例集完全一致。

## 7. 测试结论

- 结论：**通过**
- feature 分支最新 commit：`7bfc130`（merge `--no-ff` `c1a1af6` + 测试报告 `58ce7ba` + 状态 `7bfc130`；本地 = origin `0 0`）
- CI 运行号与结论：workflow `ci`，run **40168680216**（HEAD `7bfc130`）`conclusion=success`；run **40168310323**（`c1a1af6`，合并态纯代码）`success`；开发轮代码态 `40163901414`（`ad404df`）`success` 作对照
- 是否可进入评审：**是**
- 移交说明：交评审 Agent（提示词 10）。请重点复核三处：
  1. **本轮唯一代码改动**（`SafeBox.tsx` 单行 + 注释）是否属于第一档合理修复——该缺陷在 `main` 上已存在（task-045 起），本轮因落在 D20 验收范围内而就地修复；
  2. §6 第三档第 1 项（GUI 取证）是否接受「真 Chromium 渲染 + jsdom + 引擎集成 + 安装级文件系统」四层补偿，发版前补真机走查；
  3. 开发轮「与任务书不符」4 项（多 1 个提交 / NSIS 被 UAC 拦改用 MSI / `colSpan=6` / 2 条 react-refresh warning）是否接受。
- 测试产物位置：`%TEMP%\cclean-test-evidence\`（`051-safebox-probe.png`、`051-safebox-probe2.png`、`051-safebox-dom.html`、`051-inst-hash-before.txt`、`051-installed-stderr.txt`、`051-cdp3-stderr.txt`）；测试安装已 `uninstall.exe /S` 卸载，`%APPDATA%\com.and0114.cclean\rules` 已还原为重建态（与安装包逐字节一致）。
