# 测试报告：task-055 M3 更新体验——banner 下载进度 + 失败态文案 + 手动检查 loading

> 测试 Agent（提示词 9）独立复测。开发轮自验结论不作为依据，所有门禁命令均由测试 Agent 重跑。
> 测试日期：2026-10-28　分支：`feature/task-055-update-banner-progress` @ `a92193b`（与 origin 一致）

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| dev（本地启动） | Windows 11 / cargo / pnpm；`pnpm tauri dev` 未做真实点击（见 §6 第三档） | 部分执行 | 见 §2 |
| 打包安装（干净目录） | `tauri build` 出包 → MSI `/a` 管理映像 `D:\qa-smoke\msi055\` 直启 exe | 是 | 通过 |

**同源测试声明**：开发与测试在同一台机器（ANFER-PC）由同一模型会话完成。开发轮声称的门禁结果已由测试 Agent 全部独立重跑，未复用开发轮输出。

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | 全量 workspace（不按改动文件裁剪） | `cargo fmt --all -- --check` | exit 0 | `target/qa-gates-055.log` |
| L1 | 同上 | `cargo check --workspace --all-targets` | exit 0 | 同上 |
| L1 | 同上 | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 | 同上 |
| L1 | 同上 | `cargo test --workspace` | exit 0，0 failed；250 passed + 8 ignored（与 task-053/054 基线一致，本任务零 Rust 改动，零回归） | 同上（47 个 test result 行全 ok） |
| L2 集成/契约 | 前端 | `pnpm --filter app lint` | exit 0，0 error（2 warning 均为 `SafeBox.tsx` 既有 `react-refresh/only-export-components`，非本任务引入） | 同上 L947-954 |
| L2 | 前端 | `pnpm --filter app test`（TMP 指工作区规避沙箱 EPERM） | exit 0，**9 files / 81 passed**（基线 54 + App 22 + Settings 5） | 同上 L965-981 |
| L2 | 前端构建 | `pnpm --filter app build` | exit 0，`index-DDQjAX01.js` **184.22 kB / gzip 59.18 kB**（基线 181.51/58.01，+2.71 kB 来自进度条 UI 与映射函数） | 同上 L987-996 |
| L2 | 契约核对 | 读源码核对 | `downloadAndInstall((event)=>…)` 第一参数即回调（与 plugin-updater@2.13.3 真实签名一致，开发轮修正了任务书的 `{onEvent}` 误写）；Started→`contentLength ?? 0`、Progress→`chunkLength` 累加、Finished→`installing` 状态；`bannerPhase` 四态 idle/downloading/installing/error；dismiss 写 `localStorage("cclean:update:dismissedVersion")`，启动 `shouldShowBanner(version, dismissed)`；Settings `onUpdateFound` 回传 App 强制弹 banner | `app/src/utils/update.ts`、`app/src/App.tsx:93-192,216-324`、`app/src/pages/Settings.tsx:190-230` |
| L3 端到端/UI | dev 真机点击 | `pnpm tauri dev` | **裁剪+登记**：见 §6 第三档。补偿：81 条 jsdom 用例（含新增 27 条）把「事件→state→文案」整条链固定（total=0 退化、Finished 后按钮 disabled、5 类错误映射、同版本不弹/新版本弹） | `App.test.tsx` 22 条、`Settings.test.tsx` 5 条 |
| L4 打包 | tauri build | `pnpm --filter app tauri build` | exit 0 产出 NSIS `C-DiskCleaner_0.1.6_x64-setup.exe`（4.02 MiB）+ MSI `..._en-US.msi`（5.83 MiB）；末尾签名步因本地无 `TAURI_SIGNING_PRIVATE_KEY` 报错（CI 才有密钥，预期行为，不影响安装包） | `target/tauri-build-055.log` |
| L4 打包 | MSI 负载 | `msiexec /a ... /qn TARGETDIR=D:\qa-smoke\msi055` | exit 0：`cdisk-cleaner.exe` + `rules/`(2 文件) + `scripts/`(5 ps1)，**无 `_up_`**（task-050 资源映射修复保持） | `target/l4-055.txt` |
| L4 数据隔离 | 从 MSI 抽取目录直启 exe | `Start-Process cdisk-cleaner.exe` | PID 25416 存活、Responding=True、MainWindowTitle=`C-Disk Cleaner`；WebView2 子进程 5 个拉起；**安装目录 NO_DIFF**（启动前后文件清单逐字节一致）；DB/规则落 `%APPDATA%\com.and0114.cclean\`（data/cdiskcleaner.db、rules/、safe_box/），未写安装目录 | `target/l4-launch-055.txt` |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `onUpdateNow` 调 `downloadAndInstall({onEvent})` | `App.tsx:148` `update.downloadAndInstall((event) => {…})`（按真实 API 第一参数回调） | 通过 |
| 2 | Started 记录 totalSize | `App.tsx:150` `event.data.contentLength ?? 0`，写 `downloaded.total` | 通过 |
| 3 | Progress 累加 chunkSize 算 downloaded | `App.tsx:153-156` `downloadedRef.current += chunk` 后写 state | 通过 |
| 4 | Finished 切「安装中」 | `App.tsx:177` `setBannerPhase("installing")`，随后 `relaunch()` | 通过 |
| 5 | downloading 显示线性进度条（宽=percent） | `App.tsx:256-264` `width: ${downloadPercent}%` + `transition: width 200ms linear` | 通过 |
| 6 | 文案含百分比 + 已下载/总 MB + 估算速度 | `App.tsx:237` `下载更新中 N%`；`:267-269` `已下载 X.X / Y.Y MB，速度 ≈ Z`；total=0 退化文案「总大小未知」 | 通过 |
| 7 | Finished 后「安装中，即将重启…」且按钮 disabled | `App.tsx:234-235` 文案；`:293` `disabled={downloading\|installing}` | 通过 |
| 8 | 进度条不抖动（EMA 平滑 1s 窗口） | `update.ts:9-12` `SPEED_WINDOW_MS=1000`、`SPEED_EMA_ALPHA=0.4`；ref 真值+setState 渲染分离（避免严格模式 updater 副作用双算） | 通过 |
| 9 | 5 类中文错误文案映射 | `update.ts:90-107` 网络/签名/磁盘/已是最新/兜底，顺序有意设计（网络先于签名，避免签名错误里的 URL 误判）；`App.test.tsx` 用例覆盖 5 类 | 通过 |
| 10 | 失败态显示「重试」+「关闭」 | `App.tsx:274-322` error 态红「重试」+白「关闭」 | 通过 |
| 11 | 「重试」重新调 downloadAndInstall | `App.tsx:278` `onClick={onUpdateNow}` | 通过 |
| 12 | dismiss 写 localStorage 版本号 | `App.tsx:192` `writeDismissedVersion(update.version)`；key=`cclean:update:dismissedVersion` | 通过 |
| 13 | 同版本不弹 banner | `App.tsx:121` `setDismissed(!shouldShowBanner(u.version, readDismissedVersion()))`；`update.ts:75-80` 仅当 dismissedVersion===updateVersion 才静默 | 通过 |
| 14 | 手动 Settings 检查强制显示 | `Settings.tsx:208` `onUpdateFound?.(update.version)` → App `handleManualUpdateFound` 清 dismissed | 通过 |
| 15 | 检查中按钮 disabled + 「检查中…」 | `Settings.tsx:412-413,428,430` appPhase=checking 时 disabled + 文案 | 通过 |
| 16 | 结果显示「当前 vX / 发现 vY」或「已是最新」 | `Settings.tsx:194,199,204` `当前 v{local} / 发现 v{remote}`、`已是最新版本` | 通过 |
| 17 | 失败文案复用映射函数 | `Settings.tsx:211,230` `classifyUpdateError(String(e))` | 通过 |
| 18 | 新增单测 ≥5 条（5 类映射/dismissed/百分比） | vitest 新增 27 条（App 22 + Settings 5），覆盖全部要求面 | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| — | 本轮未发现业务/配置缺陷 | — | — | — | — |

## 5. 测试期修复记录（bugfix 子分支）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| — | 无（无需 bugfix 子分支） | — | — | — | — |

## 6. 遗留问题（三档分流）

### 第一档（已修）
无。

### 第二档（需用户拍板，状态 blocked）
无。开发轮登记的 3 项与任务书偏差（`downloadAndInstall` API 签名修正、白名单漏 `utils/update.ts`、真机冒烟移交测试 Agent）均为执行层适配，不改变接口语义或方案取舍，不构成第二档。

### 第三档（已登记为已知限制）
1. **L3 dev 真机点击未取得**：WebView2 在本自动化会话中进程拉起、主窗口标题正确、Responding=True，但不渲染可见 UI（自 task-050 起持续登记的历史已知项）；且 updater banner 自然出现需要 GitHub 上存在比当前更新的 Release（v0.1.7 未发布）。四条交互（进度条 0→100%、断网中文文案、dismiss 重开不弹、Settings loading）的行为逻辑由 27 条新增 jsdom 用例补偿固定。真实 GUI 走查证据由用户在活动桌面手动补。
2. **L4 NSIS 真实安装未跑**：NSIS perMachine 安装需 UAC 提权（本机 IsAdmin=False）；本轮以 MSI `/a` 管理映像完成负载/布局/数据隔离/零写入验证（等效覆盖安装态风险面）。NSIS 包已产出（4.02 MiB）。
3. **`tauri build` 末尾签名步失败**：本地无 `TAURI_SIGNING_PRIVATE_KEY`，不影响安装包产出；updater `.sig` 与 `latest.json` 由 CI 在真发版时生成。
4. **CI 五道闸未轮询**：测试 Agent 不开 PR，待提示词 B 建 draft PR 后轮询。
5. **G6（设置页首调 `ipc.localhost` 红错）**：既有已知项，Console 零红判定口径为「排除 G6 后零红」。

## 7. 测试结论

- **结论**：**通过**（L1/L2 全绿零回归，81 条前端用例含新增 27 条，L4 MSI 负载/数据隔离/安装目录零写入验证通过；0 缺陷）
- **feature 分支最新 commit**：`a92193b docs(state): 任务055完成，M3 banner 下载进度 + 失败态 + loading 就绪 (#task-055)`
- **移交说明**：状态置 `awaiting_review`。下一步由提示词 B（CI 门禁观察员）基于本分支建 draft PR 并轮询 Windows CI 五道闸；评审 Agent（提示词 10）复审判定。L3 真实 GUI 四条交互（进度条/断网文案/dismiss 持久化/Settings loading）由用户在活动桌面补走查。
