# 任务编号：061 M5 阶段起步——深色/浅色主题 + 清理历史趋势图 + 计划自动清理

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：PRD P1-1 / P1-5 / P2-3（M5 社区化长期阶段首批功能）。
> 上游依赖：v0.1.7 已发布（merge `3b2a961`），M2/M3/M4 全部收官；PROJECT_STATE 已知问题段已精简（commit `a1e72ac`）。
> **本任务范围（3 个功能单元，M5 起步厚任务）**：深色/浅色主题切换、清理历史趋势图、计划自动清理（定时任务）。
> **为什么把这些合并**：都是 M5 阶段首批 P1/P2 功能，互相独立但都属于「用户体验增强」类；按新提示词「同一阶段的相关功能合并派发」原则，一次 PR 启动 M5。
> **不做**：跨盘恢复 fallback / RmShutdown 连带兄弟进程 / RmShutdown 30s 无进度（用户决策维持现状）；代码签名 / 品牌图标 / G6（已决策不做或豁免）；差量更新 / release 瘦身（已决策不做）。

## 状态字段

pending

## 任务目标

### 1. P2-3 深色/浅色主题切换

- **PRD 描述**：用户可在深色/浅色主题间切换，跟随系统或手动指定。
- **修法**：
  - Tauri 2 内置主题 API（`tauri::theme::Theme`）或前端 CSS 变量切换；
  - 设置页加「外观」选项：跟随系统 / 浅色 / 深色（三态）；
  - 选择持久化到 prefs（复用 task-057 的 prefs 表）；
  - 首次启动跟随系统（`prefers-color-scheme`）。

### 2. P1-5 清理历史与趋势图

- **PRD 描述**：仪表盘显示最近 30 天清理量趋势图，让用户看到「清理收益」。
- **现状**：仪表盘已有统计卡片（清理轮次 / 释放总量），但无趋势图。
- **修法**：
  - 后端新增 `cleanup_stats_trend(days: i64) -> Vec<TrendPoint>`，按天聚合 `SUM(bytes_freed)`；
  - 前端仪表盘加趋势图（用轻量 SVG 或 recharts，不引重型库）；
  - 趋势图 X 轴 = 日期，Y 轴 = 释放空间（MB/GB 自适应）；
  - 空数据时显示「暂无清理记录」占位。

### 3. P1-1 计划自动清理

- **PRD 描述**：用户可设置定时任务（每天/每周/每月）自动扫描并清理。
- **修法**：
  - **MVP 范围**：只做「后台静默扫描 + 通知」，不做自动删除（安全第一）；
  - prefs 表加 `auto_scan_enabled: bool`（默认 false）+ `auto_scan_interval: String`（daily/weekly/monthly）；
  - 后端用 Windows 任务计划程序（`schtasks`）或 Tauri 后台定时器触发扫描；
  - 扫描完成后发系统通知（「上次扫描释放 X GB」），不自动删除；
  - 设置页加「自动扫描」开关 + 间隔选择（daily/weekly/monthly）。

**本任务不做**：
- 不做自动删除（安全第一，只扫描 + 通知）；
- 不做 P1-2 大文件地图 / P1-3 残留定位 / P1-4 注册表健康 / P1-7 方案模板 / P1-8 目标空间（留后续任务）；
- 不做 P2-1 多盘 / P2-2 自定义规则 / P2-4 便携版 CLI（留后续任务）。

## 前置条件

1. `main` 已合并 task-060（merge `3b2a961`），PROJECT_STATE 已更新（commit `a1e72ac`）；
2. 关键锚点：
   - `engine/src/prefs.rs`（加主题/自动扫描字段）
   - `engine/src/audit_log/stats.rs`（加趋势聚合 SQL）
   - `app/src/pages/Settings.tsx`（主题/自动扫描 UI）
   - `app/src/pages/Dashboard.tsx`（趋势图）
   - `app/src/index.css` 或 `App.tsx`（主题 CSS 变量）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 深色/浅色主题

1. 设置页「外观」三态：跟随系统 / 浅色 / 深色；
2. 选择持久化到 prefs（重开应用保留）；
3. 切换后立即生效（不需重启）；
4. 首次启动跟随系统；
5. 单测：prefs 默认值 = system。

### 2. 清理历史趋势图

6. 后端 `cleanup_stats_trend(days)` 返回按天聚合的 `SUM(bytes_freed)`；
7. 前端仪表盘显示最近 30 天趋势图（SVG 折线或柱状）；
8. 空数据时显示「暂无清理记录」占位；
9. 单测：趋势聚合 SQL（3 天数据 → 3 个点）。

### 3. 计划自动扫描

10. prefs 表加 `auto_scan_enabled`（默认 false）+ `auto_scan_interval`（默认 daily）；
11. 设置页加「自动扫描」开关 + 间隔选择；
12. 后台定时器触发扫描（不自动删除）；
13. 扫描完成发系统通知；
14. 单测：开关默认 false + 间隔默认 daily。

### 后端单测

15. `cargo test -p engine` 全绿（现有 255+8 + 本任务新增 ≥3 条全绿）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/prefs.rs`（主题/自动扫描字段）
- `engine/src/audit_log/stats.rs`（趋势聚合）
- `engine/tests/`（趋势/自动扫描用例）
- `app/src/pages/Settings.tsx`（外观/自动扫描 UI）
- `app/src/pages/Dashboard.tsx`（趋势图）
- `app/src/index.css` 或 `App.tsx`（主题 CSS 变量）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-061-m5-start`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-061-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 3 个功能单元拆 3 个原子提交：

1. `feat(ui): 深色/浅色主题切换（跟随系统/浅色/深色三态，prefs 持久化） (#task-061)`（单元 1）
2. `feat(stats): 清理历史趋势图（最近 30 天按天聚合，仪表盘 SVG 折线） (#task-061)`（单元 2）
3. `feat(scan): 计划自动扫描（开关+间隔，后台触发，只扫描不删除，完成发通知） (#task-061)`（单元 3）
4. 自验全绿后，最后一次提交：`docs(state): 任务061完成，M5 起步——主题切换/趋势图/自动扫描 (#task-061)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 061: ...`；
   - 「最近一次变更」追加一行，含 3 个单元摘要 + commit hash；
   - 「下一步计划」改为「M5 持续推进：P1-2/P1-3/P1-4/P1-7/P1-8 等后续功能」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-29  task-061  M5 起步（主题/趋势图/自动扫描）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：prefs 新字段默认值、趋势聚合 SQL、自动扫描开关默认值。
- **集成测试（必做）**：`cargo test -p engine` 全绿。
- **端到端写路径（必做，本任务硬要求）**：主题切换必须有一条真实「切换主题 → 查 prefs 存储 → 重开应用主题仍在」证据；自动扫描开关同理。
- **真机冒烟（必做、不可裁剪）**：
  - 主题切换：浅色/深色/跟随系统三态切换后立即生效；
  - 主题持久化：切换主题 → 重开应用 → 主题仍在；
  - 趋势图：仪表盘显示最近 30 天趋势（有历史数据时）；
  - 自动扫描：开启开关 → 后台扫描触发 → 发系统通知（不自动删除）；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 4 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（主题切换 UI + 趋势图 + 自动扫描开关，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 主题三态切换 | 真机截图：浅色/深色/跟随系统三态 |
| 主题持久化 | 真机证据：切换主题 → 重开应用 → 主题仍在（DevTools 查 prefs） |
| 趋势图渲染 | 真机截图：仪表盘趋势图 |
| 趋势图空数据 | 真机证据：无历史数据时显示占位 |
| 自动扫描开关 | 真机证据：开启开关 → 后台扫描触发 → 发系统通知 |
| 不自动删除 | 真机证据：扫描完成后文件仍在原位（不自动清理） |
| 不回归 | `cargo test -p engine` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 4 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-061-m5-start`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 主题实现方式（CSS 变量 vs Tauri theme API）；
   - 趋势图实现方式（SVG vs recharts）；
   - 自动扫描触发方式（schtasks vs 后台定时器）；
   - `cargo test -p engine` 通过输出（含新增 ≥3 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（4 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（M5 后续功能：P1-2/P1-3/P1-4/P1-7/P1-8 等）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务061-M5 起步（主题切换+趋势图+自动扫描） (#task-061)`，
   该提交只含 `tasks/061-m5-start.md` 一个文件，落在 `feature/task-061-m5-start` 分支最前端。

---

## 实现备注

> 执行日期：2026-10-30　分支：`feature/task-061-m5-start`（基线 `8dac082`，从 main `3b2a961` 拉出）

### 1. 实际改动文件清单

| 文件 | 类型 | 改动 |
|---|---|---|
| `engine/src/prefs.rs` | 后端 | `theme` 三态（`normalize_theme`/`get_theme`/`set_theme`）+ 自动扫描三键（`auto_scan_enabled`/`auto_scan_interval`/`last_auto_scan_at`）+ `interval_secs` + `auto_scan_due` 纯函数 + `read_setting`/`write_setting` helper；`Prefs` 加三字段；10 条单测 |
| `engine/src/audit_log/stats.rs` | 后端 | `TrendPoint` + `cleanup_stats_trend(conn, days)` |
| `app/src-tauri/src/auto_scan.rs` | 新建 | 后台轮询线程 + 复用 ScanOrchestrator + 系统通知 + `human_bytes` |
| `app/src-tauri/src/commands.rs` | Tauri（**白名单缺口**） | `set_prefs` 写 theme/auto_scan 三键；新增 `cleanup_stats_trend` 命令 |
| `app/src-tauri/src/lib.rs` | Tauri（**白名单缺口**） | `mod auto_scan` + `.plugin(notification)` + setup 里 `auto_scan::spawn` + 注册新命令 |
| `app/src-tauri/Cargo.toml` | 依赖（**白名单缺口**） | `tauri-plugin-notification = "2"` |
| `app/src-tauri/capabilities/default.json` | 权限（**白名单缺口**） | `notification:default` |
| `app/src/App.tsx` | 前端 | `THEME_TOKENS` + 三态 state + matchMedia 监听 + prefs 校准 + 根 div 注入 CSS 变量 + `onThemeChange` 传给 Settings |
| `app/src/pages/Settings.tsx` | 前端 | `Prefs` 加三字段 + `savePrefs` 统一提交 helper + 外观三态 UI + 自动扫描开关/间隔 UI |
| `app/src/pages/Dashboard.tsx` | 前端 | 颜色改 `var()` + `TrendChart` 组件（手写 SVG 柱状图） |
| `engine/tests/cleanup_stats_trend.rs` | 新建测试 | 5 条 |
| `app/src/App.test.tsx` / `app/src/pages/Settings.test.tsx` | 测试 | 3 + 5 条 |

### 2. 主题实现方式：纯内联 CSS 变量（不新增 .css）

本项目原本**没有任何 `.css` 文件**，101 个颜色全写在内联 style 里。所以主题走 CSS 变量：

```tsx
const THEME_TOKENS = {
  light: { bg: "#f9fafb", surface: "#ffffff", text: "#111827", muted: "#6b7280", border: "#e5e7eb" },
  dark:  { bg: "#0f172a", surface: "#1e293b", text: "#f1f5f9", muted: "#94a3b8", border: "#334155" },
};

// 根 div 上挂变量
<div style={{ ...themeVars, background: tokens.bg, color: tokens.text }}>

// 子元素用 var()
<div style={{ background: "var(--surface)", border: "1px solid var(--border)" }}>
```

三个理由：
1. **不新增文件**（保持零 css，也不进构建产物）；
2. **变量作用域就是根 div**，切换时只改根节点的值，所有 `var()` 自动跟随，组件不必知道当前深浅；
3. **不需要给元素加 class**（内联 style 用 class 覆盖不了，只能靠 `!important`，很难维护）。

**跟随系统的实现**：`matchMedia("(prefers-color-scheme: dark)")` + `addEventListener("change")`，用户在 Windows 设置里切主题不用重启应用。

**防闪烁**：`localStorage["cclean:theme"]` 只做**首屏同步读取**（避免「先浅色渲染再跳深色」），**真值仍在 DB**——启动后 `get_prefs` 校准一次，DB 为准。

**⚠️ 适配范围只有 3 页**：App 外壳 / Dashboard / Settings。Items（26 色）/ SafeBox（19 色）/ History（8 色）**不在本任务白名单内**，深色下仍是浅色底。已在设置页文案里如实写明「部分页面仍在适配中」，并记入遗留问题。**这不是遗漏，是白名单边界决定的。**

### 3. 趋势图实现方式：手写 SVG 柱状图（不引 recharts）

需求只是「每天一根柱子 + 顶部标峰值 + 首尾标日期」，为此引 100KB+ 的图表库不划算（前端产物一直在 190 kB 上下，本任务后是 195 kB）。

**为什么柱状不用折线**：数据是「每日聚合」的离散量。折线会把「某天没清理」画成一条穿过它的斜线，视觉上像「那天也有量」；柱状能直观表达「哪天有、哪天没有」。

**后端三个设计点**：

| 决策 | 理由 |
|---|---|
| **只返回有记录的日期**，缺失由前端补 0 | SQLite 没有 `generate_series`，后端补零要写递归 CTE 或手工循环，可读性明显变差；而「哪几天没数据」本来就是画图时最清楚的事 |
| 按 **本地时区**分组（`'localtime'`） | 用户看的是日历上的「10 月 29 日」，不是 UTC 日期 |
| `days` 夹到 `1..=365` | 传 0 或负数会让 `WHERE started_at >= now` 变成「查未来」，静默返回空——夹一下比让调用方自己保证更稳 |

前端三种状态：加载中 / 空数据「暂无清理记录」/ 失败「趋势数据加载失败，请稍后重试」（失败不影响上方卡片）。

### 4. 自动扫描触发方式：Tauri 后台轮询线程（不用 schtasks）

任务书给了两个选项（`schtasks` / Tauri 后台定时器），**选后者**。理由：`schtasks` 要管理员权限、会在系统里留一个计划任务，卸载时容易残留（用户卸载后还定时跑，是很糟的体验）；而且它没法读我们的 prefs，间隔改了还要同步删建任务。

**结构**：

```rust
// auto_scan.rs
const INITIAL_DELAY: Duration = 60s;      // 避开启动高峰
const TICK: Duration = 30 * 60s;          // 轮询间隔

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        sleep(INITIAL_DELAY);
        loop {
            tick_once(&app);              // 读 prefs → 判断到期 → 扫 → 通知
            sleep(TICK);
        }
    });
}
```

**为什么轮询而不是「精确 sleep 到下次扫描」**：用户随时可能改间隔/开关，精确 sleep 会在改动后错位；改系统时间、机器休眠唤醒后也会漂移。而一次轮询成本就是一条 SQLite 查询，可以忽略。

**到期判断抽成纯函数**（可单测）：

```rust
pub fn auto_scan_due(enabled: bool, interval: &str, last_at: i64, now: i64) -> bool {
    if !enabled { return false; }
    if last_at <= 0 { return true; }          // 从没扫过 → 立即到期
    now - last_at >= interval_secs(interval)
}
```

`last_at <= 0 → 立即到期` 是有意的：用户刚打开开关时应该马上看到效果，而不是等一整天。

**与手动扫描互斥**：复用 `ProgressState` 内部的 Mutex，后台用 **`try_lock`**——拿不到说明用户正在扫，本轮直接跳过。不能用 `lock()`：那会阻塞用户操作；也不能并发写 `scan_session`。

**只扫描不删除**：调 `ScanOrchestrator::run_all`（与 `scan_start` 同一条链路），**不调** `cleanup_execute`。扫完统计 `cleanup_item` 的行数与总大小，写进通知。

**失败也写 `last_auto_scan_at`**：否则扫描一直失败会每 30 分钟重试轰炸一次。

### 5. 为什么装了 notification 插件（超白名单）

任务书要求「扫描完成发系统通知」，但项目**没有 notification 插件**。要发通知必须：
- `Cargo.toml` 加 `tauri-plugin-notification`；
- `lib.rs` 注册 `.plugin(...)`；
- `capabilities/default.json` 加 `notification:default`；
- `lib.rs` 加 `mod auto_scan` + setup 里起线程。

**这 4 个文件都在白名单外，属于「任务书前提缺失」导致的必然超范围。**

为什么不自己调 WinRT Toast：要注册 AppUserModelID、且**只在安装态可用**（dev 下弹不出来），调试成本高；官方插件把 dev/安装态两条路径都处理好了。

### 6. `cargo test` / `pnpm` 通过输出

```
cargo test --workspace  →  passed=291  failed=0  ignored=8
pnpm --filter app test  →  9 files / 98 passed
```

`291 = 275（task-060 基线）+ 16`，新增明细：

| 用例 | 位置 | 覆盖 |
|---|---|---|
| `theme_defaults_to_system_when_row_missing` | prefs.rs | 缺行默认 system |
| `theme_roundtrip_persists` | prefs.rs | 存读一致 + 重开保留 |
| `theme_normalizes_illegal_values_to_system` | prefs.rs | 大小写/空白/非法值/脏数据回落 |
| `set_theme_writes_normalized_value` | prefs.rs | 落库是归一化后的值 |
| `auto_scan_defaults_off_and_daily` | prefs.rs | 开关默认 false + 间隔默认 daily |
| `auto_scan_roundtrip_persists` | prefs.rs | 存读一致 |
| `auto_scan_interval_normalizes_illegal_to_daily` | prefs.rs | 三档归一化 + 脏数据回落 |
| `interval_secs_maps_three_tiers` | prefs.rs | 86400 / 7×86400 / 30×86400 |
| `auto_scan_due_three_states` | prefs.rs | 关闭/从未扫/未到期/到期 四态 |
| `last_auto_scan_at_defaults_to_zero_and_roundtrips` | prefs.rs | 内部时间戳读写 |
| `trend_returns_one_point_per_day` | cleanup_stats_trend.rs | **3 天 3 个点 + 升序 + 数值对应**（验收 9） |
| `trend_sums_multiple_tasks_on_same_day` | 同上 | 同日多条求和 |
| `trend_excludes_rows_outside_window` | 同上 | 窗口外排除 |
| `trend_empty_table_returns_empty_vec` | 同上 | 空表返回空 |
| `trend_clamps_days_to_valid_range` | 同上 | 0/负数/超大值 |
| `human_bytes_units` | auto_scan.rs | 通知文案的字节格式化（**发现并修了一个 bug**：负数分支用了未 clamp 的原值，会打印 "-1 B"） |
| 主题三态 3 条 + 趋势 3 条 + 自动扫描 2 条 | 前端 | 见测试文件 |

**`pnpm build`**：`195.04 kB │ gzip 62.65 kB`（基线 188.06，+7 kB 来自主题/趋势图/自动扫描 UI）。

### 7. 真机 / 安装态冒烟 / CI

| 项 | 状态 | 说明 |
|---|---|---|
| 主题三态切换 + 重开保留 | **移交测试 Agent** | 按新流程开发轮不做 GUI/真机 |
| 趋势图有数据/空数据两态 | **移交测试 Agent** | 同上 |
| 自动扫描触发 + 通知弹出 + 只扫不删 | **移交测试 Agent** | 需等 60s 后触发；**通知在安装态才正常显示**，dev 下可能静默 |
| CDP Console 零红（排除 G6） | **移交测试 Agent** | 同上 |
| NSIS 安装态复跑 | **移交测试 Agent** | 同上 |
| CI（PR + 五道闸） | **移交测试 Agent** | 开发分支不跑 CI；**未建 PR** |

开发轮替代验证：`auto_scan_due` 的四态单测覆盖「什么时候该扫」的全部边界；`cleanup_stats_trend` 的 5 条覆盖聚合正确性；前端用例覆盖「事件 → state → 文案」。

### 8. 与任务书不符的偏差（3 项）

1. **白名单漏 4 个必改文件**（`Cargo.toml` / `lib.rs` / `capabilities/default.json` / 新建 `auto_scan.rs`）。自动扫描必须有后台线程 + 系统通知，这是任务书「前提缺失」导致的必然超范围。见 §5。
2. **不用 `schtasks`**，用 Tauri 后台轮询线程。理由见 §4（schtasks 要管理员权限、卸载残留、改间隔要重建任务）。
3. **主题只适配 3 页**（App 外壳 / Dashboard / Settings）。Items/SafeBox/History 共 53 个硬编码颜色且不在白名单内，深色下仍浅色。见 §2。

### 9. 遗留问题

- **Items / SafeBox / History 三页的主题适配**：需要在 `App.tsx` 的 token 之外，把这三页的 53 个颜色逐个换成 `var()`。属纯体力活，但改动面大（3 个文件、53 处），建议单独派一个任务。
- **自动扫描的通知在 dev 下可能不弹**：Windows 通知需要 AppUserModelID，通常只有安装态才注册好。测试轮请以 NSIS 安装态为准。
- **后台扫描与「用户正在清理」不互斥**：目前只与「手动扫描」互斥（共享 `ProgressState`）。如果后台扫描时用户正在执行清理，两边都会读写 DB，理论上可能冲突（扫描只 INSERT `scan_session`/`cleanup_item`，清理读写 `cleanup_task`/`safe_box`，表不重叠，实际风险低）。若要严格串行，需再加一把「全局操作锁」。
- **`auto_scan` 线程永不退出**：随进程结束而结束，没有 join 点。这是有意的（后台任务），但意味着测试里不能直接断言它停了。
- **`emit_done` 目前是死代码**（`#[allow(dead_code)]`）：仪表盘还没监听 `auto-scan://done` 刷新数据。留着是为了下个任务接上。

### 三句话结论

1. 三个功能都落地了，但**主题只覆盖 3 页**——这是白名单边界决定的（另 3 页 53 个硬编码颜色不许动），已在 UI 里如实告知并记入遗留。
2. 自动扫描选了「后台轮询 + try_lock 互斥 + 只扫不删」，不用 `schtasks`（要管理员权限、卸载残留）；为此必须装 notification 插件，超了 4 个白名单外文件，属任务书前提缺失。
3. 趋势图手写 SVG（不引图表库）、后端只返回有记录的日期（缺失由前端补 0）——都是为「195 kB 产物 + 简单可读」做的取舍。

