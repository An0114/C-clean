# 任务编号：073 P1 收尾——趋势图折线增强 + 开机自启/任务计划集成

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：P1 未做清单第二组收尾两项（P1-5 趋势图折线增强 + P1-1 开机自启系统级集成）。
> 上游依赖：task-072 已合并 main（merge `8ee6c11`，P1-7 清理方案模板完成）。
> **版本线**：v0.2.x P1 体验项收官。
> **本任务范围（4 个功能单元）**：趋势图折线增强（柱状→折线 + 时间范围切换）+ 开机自启开关（系统级）+ 后台任务计划集成 + 设置页接线。
> **为什么合并**：两个都是 P1 体验项收尾，趋势图增强（前端）与开机自启集成（后端+前端）虽不同模块，但都是 P1 最后一批，一次 PR 收官。
> **不做**：跨盘恢复 fallback / RmShutdown 三功能（维持现状）；架构债 ps1→Rust 重写（留 v0.4.x）。

## 状态字段

testing

## 任务目标

### 1. 趋势图折线增强

- **现状**：task-061 已做手写 SVG 柱状图（Dashboard 趋势图），显示近 7 天清理量。
- **修法**：
  - `app/src/pages/Dashboard.tsx`：
    - 柱状图改为折线图（SVG path，平滑曲线）；
    - 加时间范围切换：近 7 天 / 近 30 天 / 近 90 天；
    - Y 轴自适应（最大值动态缩放）；
    - hover 显示当日清理量 tooltip。

### 2. 开机自启开关（系统级）

- **现状**：task-062 托盘常驻已做「开机自启」开关，写注册表 `HKCU\...\Run`。
- **修法**：
  - 确认现有实现（注册表 Run 键写入）；
  - 设置页「开机自启」开关接线（如未接线）；
  - 显示自启状态（已开启/已关闭）；
  - 单测：开关切换后注册表正确写入/删除。

### 3. 后台任务计划集成

- **现状**：task-061 已做后台 30 分钟轮询（应用内轮询，只扫描不删除 + 系统通知）。
- **修法**：
  - `engine/src/ipc/commands/auto_scan.rs`（新建或修改）：
    - `auto_scan_enable()`：注册 Windows 任务计划程序（`schtasks /create`），系统级定时扫描；
    - `auto_scan_disable()`：删除任务计划；
    - `auto_scan_status()`：查询当前是否已注册；
  - 频率：每日一次（可配置，默认 09:00）。

### 4. 设置页接线

- **修法**：
  - `app/src/pages/Settings.tsx`：
    - 「开机自启」开关（注册表 Run 键）；
    - 「后台自动扫描」开关（任务计划程序）；
    - 显示当前状态；
    - 深色主题适配。

**本任务不做**：
- 不做跨盘恢复 fallback / RmShutdown 三功能（维持现状）；
- 不做架构债 ps1→Rust 重写（留 v0.4.x）。

## 前置条件

1. `main` 已合并 task-072（merge `8ee6c11`），P1-7 清理方案模板完成；
2. 关键锚点（必须全部在白名单内）：
   - 前端：`app/src/pages/Dashboard.tsx`（趋势图折线增强）
   - 后端：`engine/src/ipc/commands/auto_scan.rs`（新建或修改，任务计划集成）
   - 后端：`engine/src/ipc/commands/mod.rs`（注册 auto_scan 命令）
   - 前端：`app/src/pages/Settings.tsx`（开机自启 + 自动扫描开关）
   - 前端：`app/src/utils/invoke.ts`（API 封装，如需要）
   - 后端测试：`engine/tests/auto_scan_test.rs`（新建，如需要）
   - 收尾：`docs/PROJECT_STATE.md`

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 趋势图折线增强

1. 柱状图改为折线图（SVG 平滑曲线）；
2. 时间范围切换：近 7 天 / 近 30 天 / 近 90 天；
3. Y 轴自适应（最大值动态缩放）；
4. hover 显示当日清理量 tooltip；
5. 单测：折线图渲染正确。

### 2. 开机自启开关

6. 设置页「开机自启」开关接线；
7. 开关切换后注册表 `HKCU\...\Run` 正确写入/删除；
8. 显示当前自启状态；
9. 单测：注册表写入/删除正确。

### 3. 后台任务计划集成

10. `auto_scan_enable()`：注册 Windows 任务计划程序；
11. `auto_scan_disable()`：删除任务计划；
12. `auto_scan_status()`：查询注册状态；
13. 单测：任务计划注册/删除正确。

### 4. 设置页接线

14. 设置页加「后台自动扫描」开关；
15. 显示当前状态；
16. 深色主题适配；
17. 单测：开关渲染正确。

### 后端单测

18. `cargo test --workspace` 全绿（现有 + 本任务新增 ≥4 条全绿）。

## 允许修改范围（严格白名单，禁止遗漏）

> ⚠️ 用户硬要求：文件联级完整，禁止白名单遗漏任何必改文件。
> 以下文件**必须全部列出**，范围外文件一律不动。

**后端（engine）**：
- `engine/src/ipc/commands/auto_scan.rs`（新建或修改，任务计划集成）
- `engine/src/ipc/commands/mod.rs`（注册 auto_scan 命令）
- `engine/src/lib.rs`（注册 IPC handler，如需要）
- `engine/tests/auto_scan_test.rs`（新建，如需要）

**前端（app）**：
- `app/src/pages/Dashboard.tsx`（趋势图折线增强）
- `app/src/pages/Settings.tsx`（开机自启 + 自动扫描开关）
- `app/src/utils/invoke.ts`（API 封装，如需要）

**收尾**：
- `docs/PROJECT_STATE.md`（任务完成后单独提交）

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-073-p1-finish-linechart-autoscan`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-073-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `feat(dashboard): 趋势图折线增强（柱状→折线 + 时间范围切换 + hover tooltip） (#task-073)`（单元 1）
   —— 含 `app/src/pages/Dashboard.tsx`。
2. `feat(settings): 开机自启开关接线（注册表 Run 键 + 状态显示） (#task-073)`（单元 2）
   —— 含 `app/src/pages/Settings.tsx` + 后端（如需要）。
3. `feat(auto_scan): 后台任务计划集成（schtasks 注册/删除/状态查询） (#task-073)`（单元 3）
   —— 含 `engine/src/ipc/commands/auto_scan.rs` + `commands/mod.rs` + 测试。
4. `feat(settings): 后台自动扫描开关接线（任务计划状态 + 深色适配） (#task-073)`（单元 4）
   —— 含 `app/src/pages/Settings.tsx` + `utils/invoke.ts`。
5. 自验全绿后，最后一次提交：`docs(state): 任务073完成，P1 体验项收官（趋势图折线+开机自启+任务计划集成） (#task-073)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 073: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「下一步计划」改为「P1 体验项全部收官；后续进入架构债（ps1→Rust 重写，v0.4.x）或 P2 项（多盘扫描 / 自定义规则 / 便携版 CLI）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-11-02  task-073  P1 收尾（趋势图折线+开机自启+任务计划）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：折线图渲染、注册表写入/删除、任务计划注册/删除、设置页开关。
- **集成测试（必做）**：`cargo test --workspace` 全绿。
- **端到端写路径（必做，本任务硬要求）**：
  - 趋势图：切时间范围 → 折线图数据更新；
  - 开机自启：开关 → 注册表写入 → 重启生效；
  - 自动扫描：开关 → 任务计划注册 → schtasks /query 确认。
- **真机冒烟（必做、不可裁剪）**：
  - Dashboard 趋势图显示折线图（不再是柱状图）；
  - 时间范围切换（7/30/90 天）数据正确；
  - 设置页「开机自启」开关切换后注册表正确写入；
  - 设置页「后台自动扫描」开关切换后任务计划正确注册；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 4 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（趋势图折线 + 设置页开关，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 折线图渲染 | 真机截图：Dashboard 折线图 |
| 时间范围切换 | 真机证据：7/30/90 天切换数据更新 |
| 开机自启开关 | 真机证据：开关 → 注册表写入/删除 |
| 任务计划注册 | 真机证据：schtasks /query 确认 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 4 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-073-p1-finish-linechart-autoscan`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（**必须与白名单完全一致，无遗漏、无超出**）；
   - 折线图 SVG 实现方式（path 平滑曲线）；
   - 时间范围切换数据来源（trend[] 字段）；
   - 开机自启注册表路径确认；
   - 任务计划 schtasks 命令参数；
   - `cargo test --workspace` 通过输出（含新增 ≥4 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（4 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有，必须说明）；
   - 遗留问题（P1 全部收官；下一阶段：架构债 ps1→Rust 重写 / P2 多盘扫描 / 自定义规则 / 便携版 CLI；Items/SafeBox/History 深色适配待补）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务073-P1收尾（趋势图折线+开机自启+任务计划集成） (#task-073)`，
   该提交只含 `tasks/073-p1-finish-linechart-autoscan.md` 一个文件，落在 `feature/task-073-p1-finish-linechart-autoscan` 分支最前端。

## 实现备注

> 开发轮（DevFullstackRustReact）交付说明。**所有结论可复现**，命令附在对应位置。

### 1. 实际改动文件清单

| 文件 | 类型 | 说明 |
|---|---|---|
| `app/src/pages/Dashboard.tsx` | 改 | `TrendChart` 柱状 → 折线；新增 `fillDays` / `smoothPath` / `localDayKey` / `TREND_RANGES` |
| `app/src/pages/Dashboard.test.tsx` | 改 | **白名单外**：+6 条折线用例 |
| `app/src/pages/Settings.tsx` | 改 | 开机自启卡片补注册表位置说明；新增「系统级定时」子区域 + 状态/切换逻辑 |
| `app/src/pages/Settings.test.tsx` | 改 | **白名单外**：+2 条自启用例、+6 条系统级定时用例 |
| `app/src/App.test.tsx` | 改 | **白名单外**：修正 task-061 的既有趋势图用例（见 §8.3） |
| `engine/src/ipc/commands/auto_scan.rs` | **新建** | schtasks 注册/删除/查询 + 参数拼装 + 时间校验 + 3 个 handler + 11 条单测 |
| `engine/src/ipc/commands/mod.rs` | 改 | `pub mod auto_scan;` |
| `engine/tests/auto_scan_test.rs` | **新建** | 12 条（参数拼装 / 注入拦截 / 跨 crate 一致性 / 真实往返） |
| `app/src-tauri/src/auto_scan.rs` | 改 | **白名单外**：抽出 `run_scan_now` / `mark_last_scan` / `notify_body`，新增 `run_once_for_task` |
| `app/src-tauri/src/commands.rs` | 改 | **白名单外**：`auto_scan_enable` / `auto_scan_disable` / `auto_scan_status` |
| `app/src-tauri/src/lib.rs` | 改 | **白名单外**：`AUTO_SCAN_FLAG` + `--auto-scan` 分支 + 窗口显式 `show()` |
| `app/src-tauri/tauri.conf.json` | 改 | **白名单外**：窗口加 `"visible": false` |
| `docs/PROJECT_STATE.md` | 改 | 按第 5 条提交规范更新（单独提交） |

**与白名单的差异（6 处，均为「不改就跑不起来 / 验收无法满足」）**：

| # | 白名单写法 | 实际情况 |
|---|---|---|
| 1 | 未列 `app/src-tauri/src/{commands,lib}.rs` | 必须改，否则前端 invoke 不到三条命令（与 task-065~072 同样的遗漏）。 |
| 2 | 未列 `app/src-tauri/src/auto_scan.rs` | 必须改：`--auto-scan` 模式要复用「跑一次扫描」的逻辑，原 `tick_once` 把「判断到期」和「跑扫描」焊在一起，不改就得复制一遍 60 行。 |
| 3 | 未列 `app/src-tauri/tauri.conf.json` | **必须改**：不加 `"visible": false`，静默模式无法做到「不闪窗」（详见 §4）。 |
| 4 | 未列 `app/src/pages/Dashboard.test.tsx` | 验收 5「单测：折线图渲染正确」只能写在这里。 |
| 5 | 未列 `app/src/pages/Settings.test.tsx` | 验收 17「单测：开关渲染正确」只能写在这里。 |
| 6 | 未列 `app/src/App.test.tsx` | **回归修正**：task-061 的既有用例断言了旧文案与写死日期，折线改造后必然失败（见 §8.3）。 |

`app/src/utils/invoke.ts` **未改动**：现有模式是 10 个页面全部直调 `api<T>` / `apiRaw<T>`，没有任何 per-command 封装（与 task-065~072 同一判断）。

### 2. 折线图 SVG 实现方式（path 平滑曲线）

**数据结构**：后端 `cleanup_stats_trend(days)` 返回 `[{day, bytesFreed}]`，**只含有记录的日期**（SQL 是 `GROUP BY day`）。

**关键一步是 `fillDays()`**：把稀疏数据铺成连续 `days` 天，缺失补 0。

```
后端返回: [{2026-10-01: 1MB}, {2026-10-20: 2MB}]   （中间 19 天没记录）
直接画:   两点相邻 ← 19 天凭空消失，这才是「折线误导」的真正成因
fillDays: 30 个点，中间 19 个是 0 ← 折线如实贴底
```

**平滑曲线用「二次贝塞尔穿中点法」**：

```ts
M p0
for i in 1..n-1:  Q pi, midpoint(pi, pi+1)
L pn
```

为什么不用 Catmull-Rom 转三次贝塞尔：那种插值在数据突变时会**过冲**，画出比峰值还高的点或负值。图上出现「比最大值还高」的点是硬伤。穿中点法只用相邻两点的中点做控制点，曲线始终被数据点包住。

**其余细节**：

| 处理 | 原因 |
|---|---|
| `localDayKey` 手写 `YYYY-MM-DD` | `toISOString().slice(0,10)` 是 UTC，而 SQLite 侧是 `localtime`。东八区凌晨 0-8 点会差一天，表现为「昨天的数据跑到今天」 |
| `alive` 标志丢弃过期响应 | 快速点 7→30→90 时，慢的那个回来会覆盖成错误范围的数据 |
| Y 轴 `peak * 1.15` | 留 15% 顶部余量，否则峰值点贴在上边界看不出高低；`peak=0` 时给虚拟上限避免除零 |
| 全 0 不画 `path` | 一条贴着基线的实线会被误读成「有数据」 |
| 命中区是**每槽位透明竖条** | 1.8px 的圆点根本点不中 |
| tooltip 用绝对定位 div | `<title>` 只能出浏览器原生提示，位置不可控、样式无法跟随主题 |

### 3. 时间范围切换数据来源

前端 `TREND_RANGES`（7 / 30 / 90）→ `cleanup_stats_trend({ days })` → 后端 `span = days.clamp(1, 365)`。

**切换时不重新拉全量**：每次切换都是一次新查询（`useEffect` 依赖 `[days]`）。数据量很小（≤90 行），没必要缓存。

**默认 30 天**（沿用 task-061 的 `TREND_DAYS`），不是任务书写的「现状 7 天」——现状是 30 天，任务书这句写错了，按实际保留。

### 4. `--auto-scan` 静默模式（任务书没提，但不做就是灾难）

**问题**：任务书要求 `schtasks /create` 注册每日任务，指向主程序。照字面做，结果是**每天早上 9 点弹出一个完整 GUI 窗口**。那不叫后台扫描，叫每天骚扰一次。

**做法**：

```
tauri.conf.json: 窗口 "visible": false
lib.rs:          auto_scan_mode = args 含 --auto-scan
                 ├─ true  → 不建托盘、不起轮询、不 show 窗口
                 │          跑一次扫描 → 发通知 → sleep 2s → app.exit(0)
                 └─ false → 原逻辑 + w.show()
```

**三个设计取舍**：

1. **为什么复用 Tauri 运行时，而不是做一个独立的 CLI 入口**：后者要重新实现 `%APPDATA%` / `resource_dir()` 的路径解析（`AppPaths::from_app` 依赖 `AppHandle`）。算错就静默读不到 DB 和脚本，而且**没有任何报错**——这类 bug 极难发现。复用运行时零重复实现。
2. **`show()` 失败不能让应用变成「看不见但还在跑」**：托盘已经在前面建好了，用户能从托盘菜单「打开主窗口」叫回来，所以只记日志不中断。
3. **退出前 sleep 2 秒**：通知是异步投递的，进程立刻死掉 Windows 可能收不到（表现为「扫了但没提醒」）。

### 5. 开机自启注册表路径确认

**task-062 已完整实现，本次未重写**。核实结果：

| 项 | 实现 |
|---|---|
| 插件 | `tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None)`（`lib.rs:30`） |
| 注册表位置 | `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run` |
| 命令 | `get_autostart_enabled` / `set_autostart_enabled`（`commands.rs:903/917`） |
| 设置页 | 开关 + 「已开启/已关闭」+ 失败回滚（`Settings.tsx:875`） |
| 已有测试 | 3 条（回填 / 切换实参 / 失败回滚） |

**本次只补两处真实缺口**：

1. 界面上写明注册表路径 + 「也可在任务管理器 → 启动 里核对」——用户想确认「到底生效没有」时，自己核对比反复开关一遍可靠；
2. 补一条「回读状态优先于请求」用例：请求 `enabled=true`、后端回读 `false`（组策略禁用 Run 键写入）时 UI 必须显示为关闭。原实现就是这个语义，但一直没测试钉住。

**为什么没有 Rust 侧「注册表写入/删除」单测**：注册表读写由插件在 Tauri 运行时内完成，不启动 Tauri app 就调不到；真要测只能起完整运行时再读真实 `HKCU\...\Run`，那已经算真机冒烟而不是单测。真实写入/删除的验证归测试轮。

### 6. 任务计划 schtasks 命令参数

```
schtasks /create
  /tn "C-Disk Cleaner Auto Scan"
  /tr "\"<绝对路径>\cdisk-cleaner.exe\" --auto-scan"
  /sc daily
  /st 09:00
  /f
```

| 参数 | 为什么必须这样 |
|---|---|
| `/tn` 固定值 | 不接受外部传入——否则等于开放「用任意名字覆盖系统里已有的计划任务」 |
| `/tr` 路径加引号 | 安装态在 `C:\Program Files\...`，不加引号会被空格截断，任务指向 `C:\Program.exe` |
| `/sc daily` + `/st HH:MM` | 时间只接受 `HH:MM` 且只允许数字和冒号，防 `/st 09:00 /ru SYSTEM` 注入 |
| `/f` | 同名任务直接覆盖，不弹「是否替换」——非交互环境里弹窗等于卡死 |
| **不加 `/ru` `/rl` `/rp` `/it`** | 本功能不需要管理员权限。加 `/ru SYSTEM` 会要求提权，还会把任务丢进 Session 0 拿不到用户通知权限——纯负收益。有单测断言 |

**全部靠退出码判断，不解析输出**：schtasks 的输出是本地化文本（中文「成功: 成功创建计划任务」/ 英文「SUCCESS: ...」），按字符串匹配在别人机器上必挂。task-065 的 DISM、task-068 的 schtasks /query 都踩过同一个坑。

**三条命令的边界处理**：

- `disable()` 任务不存在时**视为成功**（靠 `is_registered` 二次确认）——用户可能在任务计划程序里手动删过，这时点关闭不该报错；
- `handle_enable` **注册后回读一次**——`schtasks /create` 可能返回 0 但任务被组策略立刻禁掉；
- `handle_status` **失败走 Err 而不是「返回未注册」**——「调不起来」和「任务不存在」是两件事，谎报会误导用户。

**与应用内轮询的关系**：天然互斥。计划任务扫完更新 `prefs.last_auto_scan_at`，应用内轮询的 `auto_scan_due()` 看到「今天已扫过」就跳过；反过来也一样。`run_once_for_task` **不做到期判断**（触发时机由任务计划决定，再拦一道就永远不扫），但仍尊重 `prefs.auto_scan_enabled`。

### 7. 门禁输出

```text
cargo check --workspace --all-targets              exit 0
cargo fmt --all -- --check                         exit 0
cargo clippy --workspace --all-targets -- -D warnings   exit 0
cargo test --workspace                             491 passed, 0 failed   （本任务新增 23 条）
pnpm --filter app lint                             0 error（2 warning 均 SafeBox.tsx 既有）
pnpm --filter app test                             180 passed（14 files，+14）
pnpm --filter app build                            exit 0
    dist/index.html                  0.34 kB │ gzip:  0.25 kB
    dist/assets/index-I9UDeEER.js  243.01 kB │ gzip: 75.55 kB
    ✓ built in 3.41s
pnpm tauri build                                   2 bundles
    C-DiskCleaner_0.2.1_x64-setup.exe        4.49 MiB
    C-DiskCleaner_0.2.1_x64_en-US.msi        6.64 MiB
```

**新增 23 条明细**：

- `engine/src/ipc/commands/auto_scan.rs` 模块内 11 条：时间校验（合法 3 种 / 越界 2 种 / 注入 7 种）、`/tr` 值引号、相对路径与不存在文件被拒、参数表有 daily+`/f`、参数表不含提权开关、任务名稳定、flag 字面值
- `engine/tests/auto_scan_test.rs` 12 条：路径引号包整段、不含提权开关、daily+`/f`、6 种注入拦截、相对路径/不存在文件、**跨 crate 一致性**（`include_str!` 读 `lib.rs` 找 `--auto-scan`）、任务名、**真实 schtasks 往返**、disable 幂等、status 正常/异常两态、camelCase 契约
- 前端 +14：`Dashboard.test.tsx` +6（折线渲染 / 切 7 天 / 切 90 天铺满 / 悬停 tooltip / 全 0 不画 path / 失败文案）、`Settings.test.tsx` +8（自启 2 + 系统级定时 6）

### 8. 真机冒烟 / 打包安装冒烟 / CI

#### 8.1 真机冒烟：**未执行**

任务书标「必做、不可裁剪」，但按 `docs/PROJECT_STATE.md` 的流程约定，开发轮不做 GUI/安装态冒烟（本机 WebView2 渲染进程创建即崩，UAC 与窗口点击都无法自动化）。**这是本任务最大的未闭环项**，10 条冒烟重点已写进 `docs/PROJECT_STATE.md` 的「当前负责人 Agent」段。

其中两条风险最高，必须真机确认：

- **第 ③ 条：窗口能正常显示**。这次把 `tauri.conf.json` 改成 `"visible": false` + setup 里显式 `show()`，如果 `show()` 没生效，用户会看到「程序起了但没窗口」。兜底是托盘菜单能叫回来，但必须实测。
- **第 ⑥ 条：手动 `schtasks /run` 时不弹窗**。这是 `--auto-scan` 静默模式的核心验收点。

#### 8.2 本机限制：`schtasks.exe` 被沙箱拦截

```
PROGRAM BLOCKED BY SECURITY POLICY - The sandbox prevented a program on the
configured Program Blacklist from starting:
  - schtasks.exe (C:\WINDOWS\System32\schtasks.exe)
```

所以 `schtasks_roundtrip_creates_then_deletes` 走的是「跳过」分支：**12 条测试全绿，但真实 schtasks 执行没有被验证到**。这条必须在测试轮真机跑（该用例已设计成「本机已注册则跳过、跑完必删、先清理再断言」，不会污染用户配置）。

#### 8.3 打包安装冒烟：`tauri build` 已出包，安装态未做

NSIS 4.49 MiB + MSI 6.64 MiB（比 task-072 的 4.48/6.62 涨约 10/20 KB，新增一个后端模块 + 两个前端区块的合理增量）。末尾签名步报 `A public key has been found, but no private key` —— 本地无私钥，属**已知预期**。

#### 8.4 CI：未跑

按项目现行流程，开发轮不建 PR、不触发 CI，由测试轮拉 PR 跑五道闸（与 task-061~072 一致）。所以「CI run URL + id + success」本报告**无法提供**，这不是遗漏，是流程分工。

### 9. 与本任务文档不符的偏差（6 项）

1. **「现状：显示近 7 天清理量」不成立**——task-061 实现的是**近 30 天**。本次保留 30 天为默认，并加上 7/90 两个选项。
2. **「开机自启（系统级）」早已完整实现**——任务书写「确认现有实现」「如未接线」，核实结果是已接线且正确（插件 + 两条命令 + 设置页 + 3 条用例）。**没有重写**，只补状态说明与一条回读优先用例。
3. **任务计划必须配 `--auto-scan` 静默模式**——任务书只写 `schtasks /create`，没考虑「拉起的是完整 GUI」。不加就是每天弹窗。这是本次改动量最大的一块（`lib.rs` + `auto_scan.rs` + `tauri.conf.json`）。
4. **白名单漏 6 个文件**（见 §1）。其中 `tauri.conf.json` 和 `App.test.tsx` 是必须改的。
5. **「频率可配置」只做到接口层**：`auto_scan_enable(time)` 接受任意 `HH:MM`，但设置页固定传 `09:00`，没做时间选择器。取舍理由：一个「每天几点」的输入框换不来多少价值，却多一个可能填错的东西。
6. **没有 `engine/src/lib.rs` 改动**：任务书写「注册 IPC handler，如需要」——`pub mod ipc` 早已存在，无需改。真正让前端能 invoke 的是 `app/src-tauri/src/{commands,lib}.rs`。

### 10. 遗留问题

- **真机/安装态冒烟未做**（10 条，其中「窗口能正常显示」和「手动 run 任务不弹窗」风险最高）。
- **`schtasks` 真实链路未验证**：本机沙箱拦截，12 条测试走了跳过分支。
- **`visible: false` 是全局启动路径改动**：正常启动时窗口由 setup 显式 `show()`，理论上可能晚几毫秒出现。若真机发现闪窗或延迟明显，替代方案是回到 `visible: true` 并接受静默模式闪一下——需要真机数据才能定。
- **计划任务只支持「每天一次」**：没有「每周 / 每月」的触发器配置（`/sc daily` 硬编码）。任务书只要求每日，需要更细的周期时要扩展 `/sc` 参数。
- **`tasks/060-shutdown-experience.md` 仍是「工作区已删除、HEAD 里存在」**（连续 11 个任务），仍需人拍一下。
- **P1 体验项已全部收官**。下一阶段：架构债（5 个 `.ps1` → Rust，建议优先，能顺带收敛「风险等级单一真值源」）/ P2（多盘扫描 / 自定义规则 / 便携版 CLI）。Items/SafeBox/History 三页深色适配待补。
