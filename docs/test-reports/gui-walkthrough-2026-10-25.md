# 真机 GUI 走查报告（2026-10-25）

> 对应 `PROJECT_STATE.md`「下一步计划 ② 发版前硬门禁：真机 GUI 走查」。
> 执行方式：**用户（AnFer）在有活动桌面的机器上手动走查**，本文件由 DocsKeeper 根据走查记录 + 截图 + 代码走查回填。
> 与前四轮「WebView2 渲染进程崩溃、GUI 证据取不到」不同：**本轮首次取得真实 GUI 点击与 Console 证据**，因此所有条目从「第三档（环境阻塞）」转为**实测结论**。

## 一、执行信息

| 项 | 内容 |
|---|---|
| 时间 | 2026-10-25（晚间） |
| 形态 | `tauri dev` 窗口（DevTools 打开）+ 安装态 |
| 代码基线 | `main` @ `801af22`（含 task-050 `cadb286`、task-051 `e6ab561`、③ 关闭体验文案 `801af22`） |
| 覆盖范围 | ① task-050 E20「扫描 → 清理」不返 `INTERNAL`；② task-051 保险箱四交互；③ task-049 ④ 占用关闭；④ Console 零红 |
| 结论 | **门禁不通过**：P1 三项（G1/G2/G3）+ P2 两项（G4/G5）+ P3 一项（G6，已知问题复现）。修复并复验后方可发版 |

## 二、逐项结果

### ① task-050 E20：扫描 → 清理

| 检查点 | 结果 | 说明 |
|---|---|---|
| 扫描进度条 | **异常** | 进度条本体不动，但下方路径文字持续变化（如 `C:\Users\An\AppData\Local\pip\cache\http-v2\1\8\4\7\9\…`），计数显示「— 1 项」 |
| 清理列表有数据 | 是 | 「已选 100 项，共 2.8 GB」，行内 `#9007/#9008/#9009/#9003/#3/#7/#8…` |
| 清理成功 | 是 | 结果条「已清理 1 项，释放 430 B」，**未返回 `INTERNAL`** → E20 的原始放行条件成立 |
| 历史有记录 | 是 | `permanent_delete` 逐条落库，详情含 `bytes_freed / err / item_id` |
| Console 红错 | 无 | 清理链路 Console 干净 |

### ② task-051 保险箱四交互

| 检查点 | 结果 | 说明 |
|---|---|---|
| 时间预设（全部/24h/3d/7d） | 正常 | 可切换、高亮正确 |
| 多 session 筛选 | **异常** | Session 下拉框**完全空白**，没有任何 `session #N (M 条)` 选项 |
| 来源列 | **异常** | 73 条全显示 `—` |
| 行内「历史」懒加载 | 正常 | 可展开 |
| 汇总 | 正常 | 「共 73 项 / 总大小 921.9 MB」 |

### ③ task-049 ④ 占用关闭

| 检查点 | 结果 | 说明 |
|---|---|---|
| 被锁文件出现在列表 | **异常** | 在 `%TEMP%` 下新建文件并上锁 → 重新扫描 → 清理列表中找不到该文件 |
| 红条 + 占用进程名 | 是 | 清理**其他**扫描到的被占用文件时，红条正常列出明细：`blocked by L3: L3BrowserProfile`、`权限不足`，并带「关闭占用进程并重试」按钮 |
| confirm 弹窗文案 | 正常 | 含「同一控制台内的相关进程可能一并关闭」 |
| Console 红错 | 无 | |

### ④ Console 全局

| 场景 | 结果 |
|---|---|
| 仪表盘 / 清理 / 保险箱 / 历史 | 0 红错 |
| 设置页（`rules_check_update` 首调） | **2 条红错 + 2 条 warning**：`POST http://ipc.localhost/rules_check_update net::ERR_CONNECTION_REFUSED` + `IPC custom protocol failed, Tauri will now use the postMessage interface instead`（`invoke.ts:27` ← `Settings.tsx:87/104`） |

> 该报错**不是回归**，与已知问题「task-039 评审运行时冒烟发现：设置页首调 IPC 走 `ipc.localhost` 失败」完全一致，本轮首次在真机复现确认。

### ⑤ 统计口径（额外发现）

| 现象 | 结果 |
|---|---|
| 仪表盘「清理轮次」 | **恒为 0**（成功 78 / 失败 32 / 释放 921.9 MB 都有值） |
| 单次清理释放量 | **部分批次显示「已清理 N 项，释放 0 B」**，历史对应行 `bytes_freed: 0` 且 `err: null` |

## 三、问题清单

### G1【P1】扫描进度条恒为 0%，只有路径在动

- **现象**：扫描全程进度条宽度 0，下方路径与计数正常刷新；扫描结束后进度条仍是空的。
- **根因（已定位）**：`engine/src/scan/runner.rs:118-124` 每条 item 的 `emit` 里 **`percent: 0` 硬编码**，注释写着「单脚本内 percent 未知，统一给 0；后续编排脚本跑完再换算百分比」——但 `engine/src/scan/orchestrator.rs` **从未做过这个换算**，全文件无任何 `emit`/`percent` 写入；前端 `Dashboard.tsx:116-119` 直接取 `progress.percent` 渲染宽度。链路两头都在，中间那段换算没写。
- **影响**：用户无法判断扫描进展，长扫描看起来「卡住了」。这是唯一一条「UI 明确说谎」的缺陷。
- **建议修法**：在 `ScanOrchestrator` 里按「脚本序号 / 脚本总数」给出基线百分比（例如 5 个脚本各占 20%，脚本内按 `scanned_items` 做亚线性插值或直接取脚本完成度），或在 runner 内按该脚本已知 `total_items`（`done` 事件带 `total_items`，但只有结束时才知道）改为**先扫描后回填**不可行 → 采用编排层脚本级进度最稳。验收点：5 个脚本跑完时进度条单调递增至 100%，且任意时刻 `percent ∈ [0,100]`。

### G2【P1】清理列表固定只取 100 条，小文件永远看不见（「被锁文件没出现在列表」的真因）

- **现象**：`%TEMP%` 下新建并上锁的文件，重新扫描后在清理列表找不到。
- **根因（已定位，**不是扫描漏扫**）**：
  - 前端 `app/src/pages/Items.tsx:176` 硬编码 `api("cleanup_list_pending", { limit: 100 })`，**无 offset、无分页、无总数**；
  - 后端 `engine/src/execute/list_pending.rs:39-42` 排序为 `ORDER BY size_bytes DESC LIMIT ?2 OFFSET ?3`，默认 `limit` 上限 500；
  - 走查现场「已选 100 项，共 2.8 GB」正是撞满 limit 的表现：100 条全是大文件（431.9 MB ×3、412 MB、83.7 MB…），新上锁的临时文件通常只有几十 KB～几 MB，**必然排在 100 名之后被截断**。
- **影响**：小文件（恰恰是 temp 目录里数量最多、也最容易被占用的那批）在 UI 上不可见、不可选、不可清理；用户会误判为「扫描不到被锁文件」，task-049 的占用处置能力对这批文件完全不可达。
- **建议修法**：Items 页补分页（复用 `History`/`SafeBox` 的分页栏模式），或后端补 `total` 让前端显示「共 N 项，已加载 100」；**排序建议改 `size_bytes DESC, id DESC` 保证稳定**（同 size 时顺序未定义，翻页会跳行）。验收点：造 >100 条（含 1 个小文件）的 session，小文件能在第 2 页被找到并勾选清理。

### G3【P1】保险箱 `source_task_id` 恒为 NULL → Session 多选空白 + 来源列全 `—`

- **现象**：Session 多选框没有任何 `session #N (M 条)` 选项；「来源」列 73 条全 `—`。
- **根因（已定位，两处同一病根）**：`app/src-tauri/src/commands.rs:149`（`cleanup_execute`）与 `:227`（`cleanup_retry_locked`）构造保险箱删除器时 **第三参写死 `None`**：
  ```rust
  Box::new(SafeBoxDeleter::new(&paths.box_dir, &conn, None))
  ```
  `SafeBoxDeleter::insert_row` 把该值原样写进 `safe_box.source_task_id`（`safe_box.rs:198-200`），于是**所有保险箱行 `source_task_id` 都是 NULL**。而：
  - `list_sessions`（`safe_box.rs:573-577`）的 SQL 是 `WHERE status='in_box' AND source_task_id IS NOT NULL GROUP BY source_task_id` → **恒返回空集** → 前端多选框空白；
  - 前端「来源」列取 `entry.sourceTaskId`，NULL → `—`。
- **影响**：task-051 的三项能力（时间筛选 / 多 session / 来源列）里，**多 session 与来源列在真机上等于没上线**；而 task-051 的 26 条验收全部通过（因为用例是直接构造 `source_task_id` 的 DB 行，绕过了真实写入路径）。**这是本轮最有价值的一条：单测与集成测试覆盖了「读」，没人覆盖「写」。**
- **建议修法**：`execute_items` 内 `cleanup_task` 已 `INSERT` 并拿到 `last_insert_rowid()`（`executor.rs:79-93`），把该 id 通过 deleter 工厂闭包传给 `SafeBoxDeleter::new(..., Some(task_id))`；`cleanup_retry_locked` 同理（复用同一次执行的 task 行）。**顺带**：`executor.rs:82` 的 `session_id` 也写死 `NULL`，见 G4。验收点：真实点「清理已选」入保险箱后，「来源」列出现 `#N`，Session 下拉出现 `session #N (M 条)`，选中后列表被正确过滤。

### G4【P2】仪表盘「清理轮次」恒为 0

- **现象**：成功 78 / 失败 32 / 释放 921.9 MB 均正常，唯独「清理轮次」永远 0。
- **根因（已定位）**：`engine/src/execute/executor.rs:79-82` 插入 `cleanup_task` 时 **`session_id` 写死 `NULL`**：
  ```sql
  INSERT INTO cleanup_task (session_id, started_at, …) VALUES (NULL, ?1, ?1, …)
  ```
  而 `engine/src/audit_log/stats.rs` 的口径是 `COUNT(DISTINCT session_id)`——**SQL 的 `COUNT(DISTINCT x)` 忽略 NULL**，故恒为 0。前端 `Dashboard.tsx` 卡片「清理轮次」直接绑 `sessionCount`。
- **影响**：仪表盘四个数字里一个永远骗人；同时 `cleanup_stats(session_id=Some(_))` 的过滤分支也永远查不到数据。
- **建议修法**：`execute_items` 增参接收 `session_id`（由 `cleanup_execute` 从前端/当前扫描 session 带入，或按「本次清理涉及的 item 所属 session」聚合），写入 `cleanup_task.session_id`；若产品上「一次清理可能跨 session」则口径改为 `COUNT(*)`（清理批次计数）更贴「轮次」语义——**两者需产品侧拍板，建议先按 `COUNT(*)` 修，因为「轮次」的语义本就是批次**。验收点：连续两次清理后卡片显示 2。

### G5【P2】「清理成功，释放 0 B」

- **现象**：部分批次「已清理 1 项，释放 0 B」，历史行 `bytes_freed: 0`、`err: null`。
- **根因（已定位）**：`engine/src/execute/executor.rs:176-179` 先取 `fs::metadata(...).unwrap_or(0)` 作为 `bytes_before`；`SafeBoxDeleter::delete_file`（`safe_box.rs:122-127`）对 `NotFound` **幂等返回 `Ok(())`**。于是「扫描后、清理前文件已被系统/其他程序删掉」时：删除成功 + 释放 0 B。这在 `%TEMP%` 下极其常见（浏览器/更新器/安装器随时清缓存）。
- **影响**：不丢数据，但「释放 0 B」在 UI 上像是功能坏了；与 G4 叠加会让人怀疑统计整体不准。
- **建议修法**：`ExecResult` 增 `alreadyGone: bool`（或复用 `error: None` + `bytes_freed=0` 判定），前端把这类条目从「已清理 N 项」里单列一行「M 项已不存在，未计入释放量」。**注意**：不要把它算成 `fail`，语义上不算失败。验收点：清理一个已被外部删除的文件 → 提示「1 项已不存在」，不出现「释放 0 B」。

### G6【P3】设置页首调 `rules_check_update` 红错（已知问题复现，非回归）

- **现象**：dev 窗口只有设置页触发 `rule_check_update` 之后 Console 报 2 红 + 2 黄（见第二节 ④）。
- **根因**：Tauri 2.x `ipc.localhost` 自定义协议首调与 `useCustomProtocol` 回退策略问题，功能经 postMessage 回退仍可用。
- **状态**：与「已知问题」中 task-039 登记的条目同源，**本轮为真机复现确认**。发版不阻塞（功能可用），但「Console 零红」这条门禁因此无法判绿 → 需明确门禁口径：**排除该已知项后零红**，或单开 `bugfix/task-039-settings-ipc-redconsole` 修掉。

## 四、结论与后续

1. **发版门禁状态**：**不通过**。E20 的原始条件（扫描→清理不返 `INTERNAL`）已成立，但本轮走查新暴露 P1 三项，其中 G3 使 task-051 一半能力在真机上失效、G2 使 task-049 的占用处置对小文件不可达——**均为功能性缺陷，非体验瑕疵，不可带着发版**。
2. **建议派发 task-052（GUI 走查缺陷修复）**，范围 G1–G5，分两批：
   - 批次 A（P1）：G1 进度条换算、G2 列表分页 + 稳定排序、G3 `source_task_id` 接线；
   - 批次 B（P2）：G4 清理轮次口径（需产品拍板 `COUNT(*)` vs 补 `session_id`）、G5 零字节语义；
   - G6 单独立项或明确门禁豁免口径。
3. **流程教训（写进后续任务书模板）**：
   - **「读路径」测试不能替代「写路径」接线检查**：task-051 26 条验收全绿，但真实写入端传的是 `None`。今后凡新增「从 DB 字段派生 UI」的能力，验收必须包含一条**真实端到端写入**（点按钮 → 查 DB → 查 UI），不能只构造 DB 行。
   - **前端硬编码 `limit` 必须配总数或分页**：否则列表截断会被误判为「后端没扫到」，本轮 G2 就耗费了一次走查归因。
4. **仍未覆盖**：`RmRestart`（关闭后自动重启）未做、`RmShutdown` 最坏 ~30s 无进度、安装态（NSIS/MSI 装完）下的同样五条未逐项复跑（本轮以 dev 窗口为主）。

## 五、证据

走查由用户手动执行，截图为 7 张现场截图（仪表盘进度条 / 清理列表 100 项 / 历史记录 / 红条明细 / 保险箱 Session 空白与来源列 / 设置页 Console / 清理释放 0 B）。截图未入库（含本机路径），关键文本已逐条摘录进本文第二节，可直接作为修复任务的验收输入。

---

## 六、第二轮复验（2026-10-27）

> 背景：task-052（G1–G5 修复）已于 2026-10-26 测试通过、评审通过并合并 main（merge commit **`d92db1b`**，CI PR #17 run 40311963933 全绿）。本轮为用户在新构建上的复验，**只针对 G1 与 G3 两项**。

### 6.1 复验结果

| 检查点 | 结果 | 判断 |
|---|---|---|
| 保险箱 Session 多选框 | **仍为空**（无任何 `session #N (M 条)`） | **不可判定修复失败** —— 见 6.2，现存数据全是修复前入箱的历史行 |
| 保险箱「来源」列 | **仍全为 `—`**（78 项 / 1.3 GB，移入时间 `2026/10/25 19:21:04`、`18:37:47`） | 同上，历史数据 |
| 仪表盘扫描进度条 | **扫描结束时仍未满** | **判定：G1 修复未闭环，真缺陷** —— 见 6.3 |

### 6.2 G3：现象复现，但归因为「历史数据」，修复有效性尚未被真机验证

- **事实**：截图中 78 条记录的全部「移入时间」为 `2026/10/25 19:21:04`（多条同秒）与 `2026/10/25 18:37:47` —— 均早于 task-052 的代码落地（`dc606cd`，2026-10-25 深夜）与合并（`d92db1b`，2026-10-26）。这批行是在 `SafeBoxDeleter::new(..., None)` 时期写入的，`source_task_id` 天然为 NULL。
- **因此**：`list_sessions`（`WHERE status='in_box' AND source_task_id IS NOT NULL`）查不到任何值、前端「来源」列对 NULL 显示 `—`，**都是预期结果，不能据此判定 G3 修复无效**。
- **验证前提**：必须**新清理一批文件入箱**（task-052 之后的构建），再看「来源」列是否出现 `#N`、Session 下拉是否出现 `session #N (M 条)`。在此之前 G3 只能记为「代码已修，真机待验」。
- **但暴露一个真实体验缺陷（新增 G8）**：**历史行永久不可追溯**。`source_task_id` 一旦写入 NULL 就再也补不回来（信息在当时就丢了），于是这 78 条会**永远**显示 `—`、**永远**无法被 Session 筛选命中。用户看到的是「修了还是全 `—`」，无法分辨哪些是历史、哪些是修复没生效。建议 UI 侧把 NULL 明确渲染为「历史（无来源）」而非 `—`，并在筛选框旁加一句「早于 vX 的条目无来源信息」，避免二次误判。

### 6.3 G7【P1，新】扫描进度条结束时不满 —— G1 修复未闭环

- **现象**：扫描结束（按钮已变「再次扫描」）时进度条仍停在一段距离内，未到 100%。
- **根因（三重叠加，已逐段定位）**：
  1. **槽内插值是渐近的**：`runner.rs:63-72` `interpolate_percent` 用 `ratio = n/(n+50)`，`n→∞` 才趋近 1，**槽内永远达不到 `to`**。设计上必须依赖编排层补发的「槽位上界」与「收尾 100」两条边界事件才能满格。
  2. **边界事件被丢弃式节流吞掉**：`ipc/progress.rs:118-137` `ThrottledEmitter` 是**纯丢弃**语义 —— 距上次 emit `<100ms` 直接 `return`，**没有 trailing 补偿**。而 `orchestrator.rs:104/111` 的两条边界 emit 恰好紧跟在最后一条 item 事件之后（同一毫秒级），**极大概率被丢弃**。结果：前端最后收到的仍是插值渐近值（如 96%、88%）。
  3. **前端退订时序**：`Dashboard.tsx:105-110` 在 `await api("scan_start")` resolve 后**立即** `unlisten()`，即便 100% 事件已经 emit，也可能还没派发到 JS 回调就被退订。
  4. **测试为什么没抓住**：`scan_progress_percent` 集成测试注入的是 **`VecEmitter`（不经过 `ThrottledEmitter`）**，收尾 100 必然被记录 → 断言通过。**测试替身绕过了生产链路上的节流器**，这正是「集成测试绿、真机不满」的原因（测试报告 §47 也自认「G1 真机渲染未验证」）。
- **修复建议（三处都要动，缺一不可）**：
  - 后端：给 `ThrottledEmitter` 加 trailing 补偿（被丢的事件暂存为 pending，间隔到点补发最后一条），或提供 `emit_force()` 供编排层收尾使用；编排层收尾改用强制发送。
  - 前端：`scan_start` resolve 后主动把进度置 100（`setProgress(p => p && {...p, percent: 100})`），作为 UI 兜底；`unlisten` 延后 ~200ms 或改为「收到 100 或超时后再退订」。
  - 测试：新增一条**经过 `ThrottledEmitter`** 的收尾事件用例（缩短 `MIN_INTERVAL` 或注入时钟），断言「最后一条到达的 percent === 100」。
- **验收点**：真机连续扫描 3 次，每次结束进度条都停在 100%（不是 96/88），且 `scannedItems` 与清理列表总数一致。

### 6.4 本轮结论

- **发版门禁仍不通过**：G7 是 P1 真缺陷（task-052 修复未闭环），G3 需新批次数据才能判绿，G8 是 G3 的衍生体验项。
- **建议**：开 `bugfix/task-053-progress-tail-flush`（G7 三处修复 + 节流尾部用例），同时把「新清理一批验证 G3」列为同一轮复验动作；G8 文案随 bugfix 一起带。
- **流程教训（补充到 §四.3）**：**测试替身不能替代生产链路**。`VecEmitter` 断言 100 与 `ThrottledEmitter` 生产行为是两件事，凡「经过节流 / 队列 / 去抖」的通道，必须有一条走真实包装层的用例。
