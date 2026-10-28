# 任务编号：055 M3 更新体验——banner 下载进度反馈 + 失败态文案 + 手动检查 loading

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：PROJECT_STATE「下一步计划 2. 进入 M3：banner 下载进度反馈 / 差量更新」。
> 上游依赖：v0.1.6 已发版（2026-10-28 用户手动 workflow_dispatch 完成，升级闭环真机验证通过）；task-036 banner 非阻塞已上线（`App.tsx`）。
> **本任务范围（5 个功能单元，厚任务）**：banner 下载进度事件订阅 + 百分比/大小/速度展示 + 失败态分类文案 + 「稍后提醒」持久化 + Settings 页手动检查 loading。
> **不做**：差量更新（bsdiff/差量包是独立大块，留 task-056+）；RmRestart（产品未拍板）；G6 设置页 IPC 红错（已知项）。
> **C08 拍板**：用户已接受 `scripts\` 缺失时「启动即 fail-fast 闪烁退出」为可接受行为（PROJECT_STATE「下一步计划 1」），本任务不改 C08 相关代码。

## 状态字段

pending

## 任务目标

### 1. updater 下载进度事件订阅

- **现状**：`App.tsx:101` 直接 `await update.downloadAndInstall()`，**不传任何 onEvent 回调**——下载过程中 banner 只显示「下载中...」，用户看不到进度，大文件（NSIS ~4 MB / MSI ~6 MB）在慢网络下像卡死。
- **修法**：`downloadAndInstall` 第二个参数传 `{ onEvent: (e) => ... }`：
  - 收到 `Event::Started { totalSize }` → 记录总大小；
  - 收到 `Event::Progress { chunkSize }` → 累加已下载字节，算百分比；
  - 收到 `Event::Finished` → 切到「安装中，即将重启...」文案；
- Tauri 2 `@tauri-apps/plugin-updater` 的 `downloadAndInstall(onEvent)` 已支持事件回调（无需改 Rust 端）。

### 2. banner 进度 UI

- **现状**：`bannerPhase = "downloading"` 只显示静态文案「下载中...」；
- **修法**：banner 在 downloading 阶段显示：
  - 进度条（线性，宽度 = percent）；
  - 文字：「下载更新中 N%（已下载 X.X / Y.Y MB，速度 ≈ Z KB/s）」；
  - 速度按「每秒 chunk 增量」估算（简单 EMA，窗口 1s）；
  - Finished 事件后切「安装中，即将重启...」并禁用按钮（避免重复点）。

### 3. 失败态分类文案

- **现状**：catch 块只显示 `String(e)` 原始错误，用户看到的是英文错误码（如 `Network error` / `Signature error`）；
- **修法**：按错误信息关键词映射中文文案：
  - 含 `network` / `request` / `url` / `dns` → 「网络连接失败，请检查网络后重试」；
  - 含 `signature` / `key` / `sign` → 「签名校验失败，更新包可能已损坏，请重新下载」；
  - 含 `disk` / `space` / `enospc` → 「磁盘空间不足，请清理 C 盘后重试」；
  - 含 `version` / `already` → 「当前已是最新版本」；
  - 其他 → 兜底「更新失败：<原始错误>（请截图反馈）」。
- banner 失败时提供「重试」按钮（再次 `downloadAndInstall`）+「关闭」按钮。

### 4. 「稍后提醒」持久化

- **现状**：`dismissed` state 只在内存里——关闭应用重开后 banner 又弹；
- **修法**：dismiss 时把「已忽略的版本号」写入 `localStorage`（key: `cclean:update:dismissedVersion`），启动时 checkUpdate 拿到新版本号后，若与 localStorage 中一致则不弹 banner；
- **边界**：用户手动点 Settings 页「检查更新」时强制显示（即使已 dismissed），按钮文案不变。

### 5. Settings 页手动检查 loading

- **现状**：`Settings.tsx:87/104` 调 `rules_check_update` 时按钮无 loading 态，用户可能连点；
- **修法**：
  - 检查期间按钮 disabled + 文案「检查中...」；
  - 网络错误时显示中文文案（复用第 3 条的映射函数）；
  - 检查完成后结果显示「当前 v0.1.6 / 发现 v0.1.7」或「已是最新」。

**本任务不做**：
- 不做差量更新（独立大块，留 task-056）；
- 不做 RmRestart（产品未拍板）；
- 不改 updater Rust 端逻辑（Tauri plugin-updater 已够用）；
- 不做「自动后台下载」（用户必须点「立即更新」才下载）。

## 前置条件

1. `main` 已合并 task-054（merge `a51ebef`），v0.1.6 已发版；
2. 关键锚点：
   - `app/src/App.tsx:78-115`（banner 状态 + `onUpdateNow` + downloadAndInstall 调用）
   - `app/src/pages/Settings.tsx:87, 104`（手动 checkUpdate 调用）
   - `@tauri-apps/plugin-updater` 已在 `package.json`（task-035 引入）
3. v0.1.6 升级闭环已真机验证通过（2026-10-28），updater 链路本身可用。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 下载事件订阅

1. `onUpdateNow` 调用 `update.downloadAndInstall({ onEvent })`；
2. Started 事件记录 `totalSize`（字节）；
3. Progress 事件累加 `chunkSize`，算出 `downloadedBytes`；
4. Finished 事件切「安装中」状态。

### 2. banner 进度 UI

5. `bannerPhase = "downloading"` 时显示线性进度条（宽度 = percent）；
6. 文案含百分比 + 已下载/总大小（MB 保留 1 位）+ 估算速度（KB/s 或 MB/s）；
7. Finished 事件后文案切「安装中，即将重启...」且按钮 disabled；
8. 进度条不抖动（用 EMA 平滑速度估算，窗口 1s）。

### 3. 失败态分类

9. 按关键词映射 5 类中文文案（网络 / 签名 / 磁盘 / 已是最新 / 兜底）；
10. 失败态 banner 显示「重试」+「关闭」两按钮；
11. 点「重试」重新调 `downloadAndInstall`。

### 4. 稍后提醒持久化

12. dismiss 时 `localStorage.setItem("cclean:update:dismissedVersion", update.version)`；
13. 启动 checkUpdate 后，若 `update.version === dismissedVersion` 则不弹 banner；
14. 手动 Settings 页「检查更新」时强制显示（忽略 dismissed）。

### 5. Settings 手动检查 loading

15. 检查期间按钮 disabled + 文案「检查中...」；
16. 结果显示「当前 vX / 发现 vY」或「已是最新」；
17. 失败文案复用第 3 条映射函数。

### 单测

18. 新增或修改 `App.test.tsx` / `Settings.test.tsx`：
    - 失败文案映射函数 5 类用例全覆盖；
    - dismissed 版本号判断逻辑（同版本不弹 / 新版本弹）；
    - 进度百分比计算（含 totalSize = 0 的兜底）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `app/src/App.tsx`（下载事件订阅 + banner 进度 UI + 失败态 + dismissed 持久化）
- `app/src/pages/Settings.tsx`（手动检查 loading + 失败文案复用）
- `app/src/App.test.tsx` / `app/src/pages/Settings.test.tsx`（新增测试）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-055-update-banner-progress`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-055-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 5 个功能单元拆 5 个原子提交：

1. `feat(updater): downloadAndInstall 订阅 onEvent，记录 totalSize / downloadedBytes (#task-055)`（单元 1）
   —— 含 `app/src/App.tsx` onUpdateNow 改造。
2. `feat(ui): banner 下载中显示进度条 + 百分比/大小/速度 (#task-055)`（单元 2）
   —— 含 `app/src/App.tsx` banner UI。
3. `fix(ui): updater 失败态按关键词映射中文文案 + 重试/关闭按钮 (#task-055)`（单元 3）
   —— 含 `app/src/App.tsx` + 共享 utils。
4. `feat(ui): 「稍后提醒」持久化到 localStorage，同版本不再弹 (#task-055)`（单元 4）
   —— 含 `app/src/App.tsx`。
5. `feat(settings): 手动检查更新按钮 loading + 失败文案复用 (#task-055)`（单元 5）
   —— 含 `app/src/pages/Settings.tsx`。
6. 自验全绿后，最后一次提交：`docs(state): 任务055完成，M3 banner 下载进度 + 失败态 + loading 就绪 (#task-055)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 055: ...`；
   - 「最近一次变更」追加一行，含 5 个单元摘要 + commit hash；
   - 「下一步计划」改为「M3 下一项：差量更新（bsdiff/差量包）评估；RmRestart 待产品拍板」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-28  task-055  M3 banner 下载进度反馈 + 失败态文案 + 手动检查 loading`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0（本任务不改 Rust，预期不回归）；
- `cargo test --workspace` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0（含新增 ≥5 条用例）；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：失败文案映射 5 类、dismissed 判断、进度百分比计算。
- **集成测试（必做）**：`pnpm test` 全绿，现有 53 条不回归。
- **端到端写路径（本任务不涉及 DB）**：无。
- **真机冒烟（必做、不可裁剪）**：
  - 造一个 v0.1.7 假包（或用 dev 模式 mock updater），点「立即更新」→ 看到进度条从 0% 走到 100%，速度估算合理；
  - 断网点「立即更新」→ 看到「网络连接失败，请检查网络后重试」中文文案 + 重试按钮；
  - 点「稍后提醒」→ 关闭应用重开 → banner 不再弹（同版本）；
  - Settings 页点「检查更新」→ 按钮 disabled「检查中...」→ 结果显示；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 4 条真机冒烟在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（banner 进度条 / 失败文案 / Settings 按钮 loading，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 下载事件订阅 | 代码 diff + CDP console 抓 `Started/Progress/Finished` 事件日志 |
| banner 进度 UI | 真机截图：进度条从 0% 走到 100%，含百分比/大小/速度文字 |
| 失败态分类 | 真机截图：断网时显示「网络连接失败...」+ 重试/关闭按钮 |
| 稍后提醒持久化 | 真机截图：dismiss 后重开应用 banner 不弹；localStorage DevTools 显示已存版本号 |
| Settings loading | 真机截图：按钮 disabled「检查中...」→ 结果显示 |
| 单测 | `pnpm test` 输出（5 类文案映射 + dismissed 判断 + 百分比计算全过） |
| 不回归 | `cargo test --workspace` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 4 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 5 个代码提交 + 1 个 `docs(state)` 提交，共 6 个 commit，全部 push 到 `feature/task-055-update-banner-progress`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - onEvent 回调最终代码结构；
   - 速度估算 EMA 参数（窗口/平滑系数）；
   - 5 类失败文案最终中文表述；
   - localStorage key 命名 + 格式；
   - `pnpm test` 通过输出（含新增 ≥5 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（4 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（差量更新留 task-056；G6 已知项不阻塞；RmRestart 待产品拍板）；
3. `docs/PROJECT_STATE.md` 已按第 6 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务055-M3 banner 下载进度反馈 (#task-055)`，
   该提交只含 `tasks/055-update-banner-progress.md` 一个文件，落在 `feature/task-055-update-banner-progress` 分支最前端。
