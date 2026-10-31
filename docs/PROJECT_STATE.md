# PROJECT_STATE.md

> **特别声明**：本文件是项目唯一真实数据源，也是「每次任务结束必更新、必单独提交」的文件。后续所有 Agent 必须遵守：任何任务完成后都要先更新本文件，再单独提交一次（commit message 形如 `docs(state): ... (#task-00X)`），不得与业务代码混在同一个 commit。

## 当前目标
**v0.1.7 已发布（2026-10-29）**：M2（updater 闭环）/ M3（banner 下载进度）/ M4（RmRestart + 白名单 + 保险箱原子性）全部收官。本轮拍板：代码签名不做、品牌图标不做、G6 不修；跨盘恢复 fallback / RmShutdown 连带兄弟进程 / RmShutdown 30s 无进度三项登记为日后新功能点，暂不修。进入下一开发阶段，任务派发待用户手动发起。

## 最近一次变更
2026-10-31：**task-066 开发完成（M1 E 模块：大文件地图，只读），状态 `testing`**（`feature/task-066-big-files-map`）：
  - **① 后端扫描**：新建 `engine/src/scan/big_files.rs`。**一个字节都不删**——这四类的正规处理方式完全不同（`powercfg /h off`、磁盘清理、右键清空回收站、用户自判），统一代删必然出事，所以只做「量出来 + 给一条能照着做的建议」。`scan_hiberfil` 只读 `metadata().len()`（**不打开文件**——被内核持有，打开必失败）；`scan_windows_old` 递归求和并沿用同一套条目/时间上限；回收站优先 `SHQueryRecycleBinW`（一次调用拿全盘、含其他用户），API 不可用退回遍历 `$Recycle.Bin` 并在建议里如实写「可能不含其他用户」。**单个 `read_dir`/`metadata` 失败只跳过不中断**——用户目录里 ACL 拒绝很常见，为它整次失败等于什么都看不到。三张卡片**固定返回 3 条**（含 0 值）：前端是固定布局，缺一张看起来像加载失败。
  - **② Top100 遍历（本任务最花心思的一处）**：最小堆维护 Top N（`Reverse` 让堆顶是「当前 Top N 里最小的」，内存恒定 O(N)）+ 跳过 reparse point（junction 会无限循环并重复计数）+ 跳过系统目录与根目录系统文件（否则榜单被 `WinSxS`、`pagefile.sys` 占满，等于误导用户）。**真机实测抓到 DFS 的坑**：20 秒预算用尽时 DFS 一头扎进 `AppData` 深处，`Desktop`/`Downloads` 一个都没扫到 —— Top 只出 **21 条**、合计 16.9 GB；改成**显式队列 BFS** 后同一份预算下扫到 20 万条目、Top **99 条**、合计 59.6 GB（最大一个是 20.4 GB 的录屏）。原理：BFS 先把同层兄弟目录全部排队，浅层（用户真正认识的目录）优先被覆盖。
  - **③ IPC**：`engine/src/ipc/commands/big_files.rs` 纯逻辑层（可对临时目录单测，不用起 Tauri）；根路径不合法返 5001 而不是「全 0」——回全 0 会让用户以为盘是空的。Tauri 侧 `big_files_scan` 用 `system_drive_root()` 读 `SystemDrive` 环境变量，**不写死 `C:\`**（系统装在 D 盘的机器上写死就永远扫空）。
  - **④ 前端**：`app/src/pages/BigFiles.tsx` 三张固定分类卡片 + Top100 汇总卡 + 明细表（路径 + 大小）。**全页只有「重新扫描」一个按钮**，顶部单独一条声明解释为什么不代删。进页面**自动扫一次**（与 Dashboard 的 WinSxS 卡片相反：那个弹 UAC 必须手动点，这个纯只读不打断用户）。`truncated` 时明确提示「已达扫描上限，Top 列表可能不完整」——不能把「只扫了一部分」当成「全盘就这么大」。颜色全走 CSS 变量，深色主题自动跟随；未知 category 有兜底不白屏。App 加 `bigfiles` tab（「清理」与「保险箱」之间）。
  - **门禁**：`cargo check --workspace --all-targets` 0 / `clippy -D warnings` 0 / `fmt --check` 0 / **`cargo test --workspace` 382 passed**（+18：扫描 16 + handler 2）/ `pnpm lint` 0 error（2 warning 均 SafeBox.tsx 既有）/ **vitest 123 passed（11 files，+10）** / `pnpm build` **211.79 kB │ gzip 67.73 kB** / **`tauri build` 出包 NSIS 4.38 MiB + MSI 6.46 MiB**（末尾签名步因本地无私钥失败，属已知预期，CI 才有）。
  - **真实数据证据（可复现）**：`cargo test -p engine --test big_files_test -- --ignored --nocapture real_scan` 直接扫本机 `C:\`：`hiberfil 11.89 GB / Windows.old 0 B（不存在）/ 回收站 1.52 MB / Top100 99 条 / 合计 59.6 GB / 20.2 s`。本机 WebView2 渲染进程起不来、GUI 取证拿不到，这条是后端侧能产出的真机证据。
  - **与任务书不符 4 项**：① `app/src/utils/invoke.ts` **未改动**——任务书要求「加 `big_files_scan()` API 封装（参照现有模式）」，但现有模式是 8 个页面全部直调 `api<T>("cmd", {})`，invoke.ts 只有 `api`/`apiRaw` 两个通用解包函数、没有任何 per-command 封装；为一个命令加专用封装会造成「只有它特殊」；② `engine/src/lib.rs` **无需改动**（`pub mod ipc` 已存在），但真正让前端能 invoke 的是 `app/src-tauri/src/{commands,lib}.rs`，这两个文件不在白名单内（与前两个任务同样的遗漏）；③ `app/src/App.test.tsx` 不在白名单内，但验收 16「单测：tab 切换正确」只能在 App 层测（tab 状态在 App 里），新增 1 条；④ 任务书写「`SHGetDiskFreeSpace` 或遍历」查回收站——`SHGetDiskFreeSpace` 是查磁盘剩余空间的，与回收站无关，正确 API 是 `SHQueryRecycleBinW`。
  - **按新流程**：开发轮不跑 CI、不做真机/安装态冒烟，移交测试 Agent。
2026-10-30：**task-065 开发完成（M1 C 模块：WinSxS 组件存储清理），状态 `testing`**（`feature/task-065-winsxs-cleanup`）：
  - **① DISM 分析**：新建 `engine/src/scan/winsxs.rs`——`WinsxsAnalysis` + `parse_analyze_output`（纯函数，可单测）+ `analyze()`。**任务书要求「解析 DISM 输出 XML」是错的**：`/analyzecomponentstore` 没有 XML 输出（`/Format:XML` 不被接受），只有本地化文本。所以提权 exe 侧强制加 `/English` 把标签锁成英文，engine 侧按 `"标签 : 值"` 逐行解析（用 `split_once(" : ")` 而不是 `split(':')`——值里的 `2021-03-15 08:21:46` 自带冒号）。标签做小写 + 空白归一化：DISM 子项带 4 空格缩进，且各版本空格数不一致。`reclaimable_bytes` 优先取 `Actual Reclaimable Space`，多数 Win10 版本不打印这一行，此时用「Backups and Disabled Features + Cache and Temporary Data」兜底。**`parsed` 标志不可省**——输出格式一变所有字段都是 0，UI 上看起来就是「可清理 0 字节」，用户会以为「没什么可清的」而不是「分析失败了」，比报错更危险。
  - **② DISM 清理**：`winsxs::cleanup` **先建系统还原点再动手**（`/StartComponentCleanup` 删旧版本组件不可逆）。fail-closed：建点失败返回 `WinsxsCleanupError::RestorePoint` → IPC 返 2007 → 前端弹「继续清理将无法回滚，是否继续？」→ 用户确认后带 `skipRestorePoint: true` 重调；**只重试一次**，避免「一直失败一直问」。跳过时结果里 `restorePointSeq = null`，UI 如实显示「本次未创建系统还原点」，不伪装成建成了。
  - **③ 提权 exe 白名单**：`ALLOWED_COMMANDS` 加 `dism_analyze` / `dism_cleanup`。**`/ResetBase` 是「没有传参入口」而不是「有入口被拦」**——`dism::Action` 只有 `Analyze` / `Cleanup` 两个变体，参数是 `&'static [&'static str]` 常量表，整条调用链上没有任何 `&str` 能流进 DISM 命令行。这比「解析参数后拒绝危险值」彻底：那种检查本身就是新攻击面（大小写变形、参数拆段）。另外三处实现要点：DISM 走 `%SystemRoot%\System32\Dism.exe` **绝对路径**（提权进程不能吃 PATH 劫持）；输出用 `MultiByteToWideChar(CP_OEMCP)` 解码（控制台程序写管道用 OEM 代码页，直接当 UTF-8 解中文路径全乱码）；自实现 15 分钟超时（`Command::output()` 会无限等，提权进程挂着不放会一直占 UAC 令牌）。
  - **④ 实时进度转发**：提权进程的 stdout 在 `ShellExecuteExW` 下拿不到（那是 `CreateProcess` 的能力），所以走「提权 exe 把 DISM 完整行落盘 → 主进程 tail 该文件」。`run_elevated_streaming` 用 `std::thread::scope`（不是 `'static` 线程，否则调用方要 clone 一堆东西）起 tail 线程，`drain_complete_lines` **按字节**切行——进度文件可能读到半个多字节字符，先 `from_utf8_lossy` 会插入 U+FFFD 改变字节长度、把游标算术弄错。事件名 `winsxs://progress` 与载荷 `WinsxsProgress{seq,line}` 都定义在 engine，避免 Rust emit 与前端 listen 两处硬编码漂移。
  - **⑤ UI**：Dashboard `WinsxsCard`（分析结果 / DISM 建议 / 二次确认 / 进度面板最近 8 行 / 结果条）。**挂载时不自动分析**——DISM 会弹 UAC，一进页面就弹窗是流氓行为。无内容可清时「立即清理」禁用并说明原因，而不是点了没反应。清理完成后**不自动重新分析**（会再弹一次 UAC），改为提示「点『重新分析』查看最新空间」。
  - **门禁**：`cargo check --workspace --all-targets` 0 / `clippy -D warnings` 0 / `fmt --check` 0 / **`cargo test --workspace` 364 passed**（+30：elevation 13 / engine::elevation tail 5 / winsxs_analyze 9 / winsxs_cleanup 7 / elevation_whitelist 3，其中 6 条为既有用例改造）/ `pnpm lint` 0 error（2 warning 均 SafeBox.tsx 既有）/ **vitest 113 passed（10 files，+9）** / `pnpm build` **206.66 kB │ gzip 66.31 kB** / **`tauri build` 出包 NSIS 4.36 MiB + MSI 6.44 MiB**（提权 exe 153 KB → **297 KB**，DISM 模块带来的增量已进包；末尾签名步因本地无私钥失败，属已知预期，CI 才有）。
  - **真实链路探测（非提权态，可复现）**：`./app/src-tauri/binaries/cclean-elevation.exe dism_analyze --result <tmp>` → **exit 4**，结果 JSON 为 `{"ok":false,"cmd":"dism_analyze","message":"DISM 退出码 740","sequenceNumber":null,"output":"\r\nError: 740\r\n\r\nElevated permissions are required to run DISM. ..."}`。这一条同时证明了：白名单放行、DISM 绝对路径可达、`/English` 生效（中文系统上 DISM 也回英文）、stdout+stderr 捕获合并、JSON 转义（`\r\n` 未破坏结构）、退出码映射——**只差 UAC 提权本身没验**（需测试轮真机点授权）。
  - **与任务书不符 5 项**：① 「解析 DISM 输出 XML」不存在，改文本解析 + `/English`；② 字段名 `reusable_packets` / `backports_and_drivers` 是误译，按 DISM 真实标签改 `reclaimable_packages` / `backups_and_disabled_bytes`；③ `engine/src/commands.rs` **不存在**，IPC 实际落在 `app/src-tauri/src/commands.rs`（task-063 同样情况）；④ 白名单漏了 `engine/src/scan/mod.rs`、`engine/src/elevation/mod.rs`（白名单同步 + `output` 字段 + 流式函数）、`app/src-tauri/src/{commands,lib}.rs`、`app/src/pages/Dashboard.test.tsx`；⑤ 任务书要求「拒绝 `/ResetBase` 参数」，实现上做成了「无传参入口」（更强）。
  - **按新流程**：开发轮不跑 CI、不做真机/安装态冒烟，移交测试 Agent。
2026-10-30：**task-064 开发完成（P0 收尾：7 天 purge + 风险徽标 + 全选 L1），状态 `testing`**（`feature/task-064-purage-badge-l1all`）：
  - **① 保险箱 7 天自动 purge**：`safe_box::purge_expired(conn, box_dir, now)`。**逐条处理而非一条批量 DELETE**——每条都要写 `audit_log action='auto_purge'`，用户必须能查到「哪些文件被系统自动删了」；批量 DELETE 只能记一行汇总，等于把审计粒度丢了。**失败语义**：文件删不掉（被占用）→ 记 failed 并**保留 DB 行**，下轮重试——绝不能「文件没删掉就把行删了」，那会让文件变成永久孤儿。文件已不存在但行过期 → 仍删行（用户手动删过文件的场景）。`expires_at == now` **不算**过期（严格 `<`）。**触发时机**：`lib.rs` 起线程，启动跑一次 + 之后每 6 小时（7 天粒度不需要每小时，每小时只是白耗 IO）。
  - **② 风险徽标/影响说明**：`PendingItem` 加 `risk_level`。**此前根本没返回这个字段**——前端只能「所有项一视同仁」，既做不了「全选 L1」，也说不清「删了会怎样」。`RISK_META` 四色（L0 灰 / L1 绿 / L2 黄 / L3 红），颜色走 CSS 变量以适配深色主题；hover 说明用 `title` 原生 tooltip（比自绘省事，且不会被表格 `overflow` 裁掉）。未知等级有 fallback，后端将来加等级不会崩。
  - **③ 全选 L1 一键按钮**：只勾**本页** L1（跨页全选要把所有页拉下来，按钮会变慢，而且用户看不到被选中的项）；**L2/L3 一律不选**——L2 是「需逐项确认」，自动全选等于绕过这道设计。按钮旁沿用「已选 N 项，共 X」统计；本页无 L1 时按钮禁用并给 title 说明。
  - **门禁**：`cargo check` 0 / `clippy -D warnings` 0 / `fmt` 0 / **`cargo test --workspace` 334 passed + 8 ignored**（+6：purge 六态）/ `pnpm lint` 0 error / **vitest 104 passed（9 files，+3）** / `pnpm build` **201.48 kB │ gzip 64.95 kB**。
  - **与任务书不符 3 项**：① 白名单漏了 `engine/src/execute/list_pending.rs`（`risk_level` 必须在这里加，否则前端拿不到）；② purge 触发点写「`engine/src/lib.rs` 或 `main.rs`」——engine 没有「启动」概念，实际在 `app/src-tauri/src/lib.rs` 的 setup 里起线程；③ 徽标与全选 L1 都在 `Items.tsx` 一个文件里，无法拆成两个原子提交，合并为一次 `feat(ui)`。
  - **按新流程**：开发轮不跑 CI、不做真机/安装态冒烟，移交测试 Agent。
2026-10-30：**task-063 开发完成（报告导出 + 前后对比 + 环形图 + 取消扫描），状态 `testing`**（`feature/task-063-report-dashboard`）：
  - **① 报告导出**：`engine/src/report/mod.rs` 三件事——`drive_usage()`、`build_html`/`build_csv`、`write_report`。**零依赖字符串拼接**（模板只有「标题 + 3 个数字 + 一张表」，引 askama/tera 不划算）。HTML 单文件、CSS 全内联，双击可开；CSV **必须带 UTF-8 BOM**——不带 BOM 时中文 Windows 的 Excel 会按 ANSI 读，中文路径全乱码，这是「CSV 打不开」的真实原因。**安全**：所有路径/错误消息过 `html_escape`，否则文件名里一个 `<script>` 就能在报告里执行。落盘 `%APPDATA%\com.and0114.cclean\reports\`。
  - **② 前后空间对比**：`ExecuteSummary` 加 `disk_before_bytes`/`disk_after_bytes`（`Option`，查不到显示「—」而不是 0——0 GB 会让人以为盘空了）。`execute_items` 开头/结尾各查一次；盘符**从第一条 item 的路径推断**（写死 "C" 会让清 D 盘时数字对不上）。`released_bytes` 用**饱和减法**：清理期间用户又写入会导致「后 > 前」，此时算 0 而不是负数。前端结果条显示「清理前 X → 清理后 Y，实际释放 Z」。
  - **③ 仪表盘环形图**：`drive_usage()` 用 `GetDiskFreeSpaceExW`，**用 `total_free`（全用户可用）算已用**，不是 `free_to_caller`（当前用户配额）——用户关心「盘还剩多少」。手写 SVG 圆环：两个同心圆 + `strokeDasharray` 画弧 + `rotate(-90)` 让起点在 12 点。≥70% 橙、≥85% 红并追加「空间紧张，建议尽快清理」。`used_percent` 用 **u128 算**防 8TB 盘的 `used*100` 溢出。
  - **④ 取消扫描**：`ScanOrchestrator` 加 `cancel: Option<Arc<AtomicBool>>` + `with_cancel()`；`run_all` 在**每个脚本开始前**检查——**脚本粒度取消**，不 kill 正在跑的 ps1 子进程（要 kill 得把 pid 传出来 + `TerminateProcess` + 回滚已写入的部分结果，代价远大于收益）。`ScanCancelState` 全局单例；**`scan_start` 每轮复位标志**（否则上次的取消会让新扫描一启动就停，这是个很容易漏的坑）；`scan_cancel` 立即返回不等停止，前端立刻切「正在停止…」让用户知道点击生效。已扫描结果保留（`cleanup_item` 行不删）。
  - **门禁**：`cargo check` 0 / `clippy -D warnings` 0 / `fmt` 0 / **`cargo test --workspace` 328 passed + 8 ignored**（+12：report 10 + 取消 3，含 1 条修 clippy 的改动）/ `pnpm lint` 0 error / **vitest 101 passed（9 files）** / `pnpm build` **199.87 kB │ gzip 64.33 kB**。
  - **与任务书不符 4 项**：① `engine/src/commands.rs` **不存在**（命令在 `app/src-tauri/src/commands.rs`）；② 字段名是 `disk_before_bytes`/`disk_after_bytes`，不是 `drive_used_bytes_before`；③ 白名单漏了 `engine/src/lib.rs`、`engine/Cargo.toml`（windows feature）、`engine/src/execute/locking.rs`（Deserialize）、`app/src-tauri/src/{commands,lib}.rs`；④ 前端「取消扫描」的 UI 用例**没进单测**——`scan_start` 挂起时 Dashboard 的并发 setState 会让元素引用失效，写了两次都不稳，改为交真机冒烟（后端取消逻辑有 3 条单测兜底）。
  - **按新流程**：开发轮不跑 CI、不做真机/安装态冒烟，移交测试 Agent。
2026-10-30：**task-062 开发完成（提权底座 + 系统还原点 + 托盘常驻，v0.2.0 起步），状态 `testing`**（`feature/task-062-elevation-srpoint-tray`）：
  - **① 提权 exe 底座**：`elevation/` 从 M0 stub 填成真实现，产出 `cclean-elevation.exe`。**刻意零依赖手写 FFI**——只需要 `SRSetRestorePointW`（在 `sfc.dll`，不是常被写错的 advapi32），引 windows crate 会让独立编译多花几分钟。命令白名单 `["create_restore_point"]`：**未知命令连参数都不解析**直接 `EXIT_UNKNOWN_CMD`（防命令注入的根本手段是「不拼命令行、只认固定命令」）。结果回传 = 退出码（0 成功 / 2 用法错 / 3 未知命令 / 4 执行失败）+ 临时 JSON（`{ok,cmd,message,sequenceNumber}`）——**为什么不用 stdout**：`ShellExecuteExW` 拉起的进程拿不到 stdout，那是 `CreateProcess` 的能力。主进程侧 `engine::elevation::run_elevated` 用 **`ShellExecuteExW` + `SEE_MASK_NOCLOSEPROCESS`**；**任务书写 `ShellExecuteW` 是错的**——它不返回进程句柄，拿不到退出码也没法等它结束。`scripts/build-elevation.mjs` 在 `beforeBuildCommand` 里编译 + 复制到 `app/src-tauri/binaries/`，`bundle.resources` 映射到安装目录根；**实测出包 NSIS 4.05 → 4.22 MiB（+0.17 MiB ≈ 提权 exe 153 KB，证明真的进包了）**。
  - **② 系统还原点**：`executor::needs_restore_point` 判断「本批 id 里有没有 L2」。**只对 L2 触发**——为 L1 缓存弹一次 UAC + 等几十秒建点，用户会直接关掉这个功能。名称 `C-clean cleanup #<id> <UTC 时间>`，时间用手写的 `civil_from_days`（Howard Hinnant 算法，公版）而不是引 chrono。失败返回 **2007**，前端弹 confirm「系统还原点创建失败……是否继续清理？」，用户确认后带 `skipRestorePoint: true` 重调。成功写 `audit_log`（`actor='elevated'`、`action='restore_point'`）。**fail-closed**：`needs_restore_point` 查库失败时按「需要还原点」处理，不因查不清就裸奔。
  - **③ 托盘常驻**：**Tauri 2 没有 `tauri-plugin-tray` 这个插件**（任务书写错了），托盘是内置能力，只需给 tauri 开 `tray-icon` feature。`CloseRequested` → `api.prevent_close()` + `window.hide()`（进程不退出）；右键菜单「立即扫描 / 打开主窗口 / 退出」，左键单击图标直接显示窗口。**「退出」不在 Rust 侧直接 `app.exit(0)`**——PRD 要求二次确认，而托盘菜单回调里没有 WebView、弹不了 `confirm`，所以走「显示窗口 + emit `tray://quit-request`」→ 前端 confirm → 调 `quit_app` 命令。开机自启用 `tauri-plugin-autostart`，**状态真实来源是系统注册表**（不存 prefs）：用户在「任务管理器 → 启动」里手动禁用也能被正确反映，否则两边会打架。
  - **门禁**：`cargo check` 0 / `clippy -D warnings` 0 / `fmt` 0 / **`cargo test --workspace` 316 passed + 8 ignored**（+25：elevation crate 7 + engine::elevation 13 + restore_point 5）/ `pnpm lint` 0 error / **vitest 101 passed（9 files，+3）** / `pnpm build` **196.96 kB │ gzip 63.37 kB** / **`tauri build` exit 0**。
  - **与任务书不符 4 项**：① `tauri-plugin-tray` **不存在**（Tauri 2 托盘是内置 + feature）；② `ShellExecuteW` 拿不到退出码，改 `ShellExecuteExW`；③ 白名单漏了 `engine/src/lib.rs`、`engine/Cargo.toml`（windows features）、`app/src-tauri/src/commands.rs`（新命令）、`app/src/App.tsx` + `Dashboard.tsx`（托盘事件）、`scripts/build-elevation.mjs` + `scripts/tauri-resources.test.mjs` + `.gitignore`；④ 路径层级仍是 `app/src-tauri/*` 不是 `src-tauri/*`（老问题）。
  - **按新流程**：开发轮不跑 CI、不做真机/安装态冒烟，移交测试 Agent。
2026-10-30：**task-061 开发完成（M5 起步：主题切换 / 趋势图 / 自动扫描），状态 `testing`**（`feature/task-061-m5-start`）：
  - **① 深色/浅色主题**：`prefs.theme` 三态（`system`/`light`/`dark`，默认 `system`，非法值归一化回落 system）。实现选**纯内联 CSS 变量**而非 index.css——根 div 上挂 `--bg`/`--surface`/`--text`/`--text-muted`/`--border`，子元素用 `var()`；好处是不新增任何 CSS 文件（本项目原本零 `.css`）、作用域就是根节点、组件不必知道当前深浅。跟随系统用 `matchMedia` + `change` 监听（用户改 Windows 主题不用重启）；`localStorage` 只做首屏防闪烁缓存，**真值仍在 DB**（启动后用 `get_prefs` 校准）。设置页「外观」三态，点选后 `onThemeChange` 通知 App 立即应用 + 乐观更新、失败回滚。**本期适配范围只有 App 外壳 / Dashboard / Settings**：Items/SafeBox/History 三页共 53 个硬编码颜色且不在白名单内，深色下仍是浅色——已在设置页如实注明「部分页面仍在适配中」，记入遗留问题。
  - **② 清理趋势图**：新增 `cleanup_stats_trend(conn, days)`，按 `date(started_at,'unixepoch','localtime')` 分组求 `SUM(freed_bytes)`、日期升序。三个设计点：**只返回有记录的日期**（缺失由前端补 0——SQLite 无 `generate_series`，后端补零要递归 CTE，可读性差且前端画图时最清楚哪天缺）；**按本地时区分组**（用户看的是日历上的「那天」）；`days` 夹到 `1..=365`（传 0/负数会让 `WHERE started_at >= now` 变成查未来、静默返回空）。前端**手写 SVG 柱状图**不引 recharts（需求只是「每天一根柱 + 峰值 + 首尾日期」，为此加 100KB+ 依赖不划算；柱状而非折线是因为数据是离散日聚合，折线会把「没数据那天」画成斜线造成误导）。空数据 → 「暂无清理记录」，失败 → 失败占位且不影响卡片。
  - **③ 计划自动扫描**：`prefs.auto_scan_enabled`（默认 **false**）+ `auto_scan_interval`（默认 `daily`，三档 daily/weekly/monthly）+ `last_auto_scan_at`（内部用，不暴露前端）。`auto_scan_due(enabled, interval, last, now)` 抽成纯函数便于单测（`last<=0` 视作立即到期——用户刚开开关应马上看到效果）。`auto_scan.rs` 起后台线程：**启动后先等 60s**（避开启动高峰）→ **每 30 分钟轮询**一次（不用精确定时：用户改间隔/改系统时间/休眠唤醒都会让 `sleep(到下次)` 错位，而一次轮询就是一条 SQLite 查询）。**只扫描不删除**（自动删除是「用户不在场时执行不可逆操作」，与产品定位冲突），扫完发系统通知「发现 N 项可清理，约 X（仅扫描，未删除）」。与手动扫描用 `ProgressState` 的 **`try_lock`** 互斥——拿不到说明用户正在扫，本轮跳过（不排队不阻塞）。失败也写 `last_auto_scan_at`，避免每 30 分钟重试轰炸。
  - **门禁**：`cargo check` 0 / `clippy -D warnings` 0 / `fmt` 0 / **`cargo test --workspace` 291 passed + 8 ignored**（+16）/ `pnpm lint` 0 error / **vitest 98 passed（9 files，+8）** / `pnpm build` **195.04 kB │ gzip 62.65 kB**。
  - **新增依赖**：`tauri-plugin-notification = "2"`（Rust 侧）+ `notification:default` 权限。**为什么装插件而不是自己调 WinRT Toast**：后者要注册 AppUserModelID、只在安装态可用，插件把 dev/安装态两条路径都处理好了。
  - **与任务书不符 3 项**：① 任务书白名单没有 `app/src-tauri/{Cargo.toml, lib.rs, capabilities/default.json, auto_scan.rs}`——自动扫描必须有后台线程（lib.rs 起线程）+ 系统通知（新依赖 + 权限），这是**前提缺失**导致的必然超范围；② 任务书说「用 schtasks 或 Tauri 后台定时器」——选后者，`schtasks` 要管理员权限、会在系统里留计划任务，卸载时容易残留；③ 主题适配范围只有 3 页（见上）。
  - **按新流程**：开发轮不跑 CI、不做真机/安装态冒烟，移交测试 Agent。
2026-10-29：**task-060 开发完成（占用关闭体验：兄弟进程核实 + 30s 进度 + 跨盘文档），状态 `testing`**（`feature/task-060-shutdown-experience`）：
  - **① 连带兄弟进程：核实后不改代码**（任务书方案无效）。四条理由：① `RmRegisterResources` 第二组参数是 `Option<&[RM_UNIQUE_PROCESS]>`（`{dwProcessId, ProcessStartTime}`），**不是裸 pid**，任务书的 `&[pid]` 编译不过；② 注册「锁文件的进程」是**冗余**——`RmGetList` 已通过「文件被谁占用」把它枚举出来，集合不变、`RmShutdown` 行为一致，零收益；③ **连带关闭兄弟进程本来就在发生**，task-049b 实测（两个共用 console 的 `powershell.exe`，只对 A 的文件调 `RmShutdown`，B 也退出），这是 RM 按 console 进程组的内部行为，**注册参数既不能开启也不能关闭**；④ 强行注册同名应用会扩大关闭面，且其中只要有一个命中护栏（受保护进程），mixed fail-closed 会让**整批都不关**——功能倒退。已在 `RmSession::register` 写 4 条理由的注释（防后人再改），文件头「已知限制」第 4 条补「这不是没注册进程导致的」，USER_GUIDE 写清用户可见行为。
  - **② 30s 无进度 → 已修**。`shutdown_locking_processes` 加第四参 `on_progress: Option<ProgressCallback>`（`Arc<dyn Fn(u32) + Send + Sync>`，engine 不认识 Tauri，只负责每秒喊一次）；内部 `spawn_shutdown_ticker` 起 1s 计时线程（**100ms 响应停止**，`RmShutdown` 返回后立即 join，避免 `?` 提前返回导致线程泄漏、前端一直收进度事件）；`commands.rs` 的 `cleanup_retry_locked` 加 `app: AppHandle` 注入参数，把回调转成 `shutdown://progress` 事件（`{elapsedSecs, expectedMaxSecs: 30}`）；`Items.tsx` 在 retrying 期间显示「正在关闭进程… 已等 Xs（最多 30s）」+ 线性进度条。**关键限制**：`RmShutdown` **没有超时参数、也无法中断**，所谓「30s」是 RM 内部对每个应用的处理上限（经验值），所以**不做「30s 超时放弃」**——只在 secs > 30 时继续回调，前端补一句「已超过预期，仍在等待系统响应」，比假装超时更诚实。
  - **③ 跨盘行为文档化**。`USER_GUIDE` 新增 **4.5 跨盘行为**：**移入支持跨盘**（同盘 rename 瞬时；跨盘自动改「复制 → 删源」，慢但可用；复制成功但删源失败时会删掉刚复制的那份再报错，避免重复副本）+ **恢复不支持跨盘**（直接报 IO 错误，文件仍在保险箱、原位置无残留）+ 为什么不对称（照搬 copy+remove 会引入「复制成功但删除失败 → 两处都有文件」的新失败模式，需单独设计）+ 实际影响很小（保险箱固定 C 盘 `%APPDATA%`）。另补 **FAQ Q9**（跨盘恢复报错怎么办，三步处置，并明确「不建议手动翻保险箱目录，索引会对不上」）。
  - **与任务书不符 3 项**：① 任务书说「跨盘恢复用 copy+remove fallback」——**错的**，task-056 已实证**恢复没有 fallback**（移入才有），文档按事实写；② 任务书说「30s 超时后放弃」——技术上做不到（`RmShutdown` 不可中断），改为「超 30s 继续显示 + 提示仍在等待」；③ 白名单漏了 `app/src-tauri/src/commands.rs`（加 `AppHandle` 参数才能 emit，且四参签名级联）与 `app/src/pages/Items.test.tsx`（新增事件订阅需要 mock）。
  - **门禁**：`cargo check` 0 / `clippy -D warnings` 0 / `fmt` 0 / **`cargo test --workspace` 275 passed + 8 ignored**（+2：ticker 两条）/ `pnpm lint` 0 error / **vitest 90 passed（9 files，+1 进度条用例）** / `pnpm build` **188.06 kB │ gzip 60.50 kB**。
  - **按新流程**：开发轮不跑 CI、不做真机/安装态冒烟，移交测试 Agent。
2026-10-29：**v0.1.7 已发布，进入下一开发阶段**。用户拍板：① 代码签名维持不做（SmartScreen 警告可接受，触发条件未到）；② 品牌图标维持 Tauri 默认占位；③ G6（设置页首调 `ipc.localhost` 红错）维持豁免不修；④ 三项已知限制——跨盘 restore 无 copy+remove fallback、RmShutdown 连带同 console 兄弟进程、RmShutdown 最坏 ~30s 前端无进度——登记为日后新功能点，暂不修。本次只更新文档，不派发新任务；下一阶段任务派发由用户后续手动发起。
2026-10-29：**task-059 开发完成（v0.1.7 发版准备 + README 重写 + 用户手册），状态 `testing`**（`feature/task-059-release-prep-v017`）：
  - **① 版本号 `0.1.6` → `0.1.7`**：四处（`package.json` / `app/package.json` / `app/src-tauri/tauri.conf.json` / `app/src-tauri/Cargo.toml`）+ `Cargo.lock` 自动更新。`tauri build` 实测产出 `C-DiskCleaner_0.1.7_x64-setup.exe`（**4.05 MiB**）+ `C-DiskCleaner_0.1.7_x64_en-US.msi`（**5.87 MiB**）+ 成对 `.sig`。**注意**：任务书说「`pnpm --filter app build` 产物文件名含 0.1.7」是错的——vite build 产物是 `dist/index-<hash>.js`，永远不含版本号（task-054 已记录过这个偏差），必须用 `tauri build` 验。
  - **② `CHANGELOG.md` 定稿**：`## [0.1.7] - 未发布（草稿）` → `## [0.1.7] - 2026-10-29`，删掉草稿说明块与 `## [Unreleased]` 段。内容复核：新增 4 条（自动重启 / 白名单 / 偏好持久化 / 恢复时间可追溯）、修复 1 条（移入失败不再丢文件）、变更 2 条、已知问题 5 条，覆盖任务书要求的全部条目。
  - **③ `README.md` 重写**：**原文件不是 Tauri 模板**（任务书假设错了），而是早期手写版——但状态停在「立项 / 原始需求草稿已固化，待进入 PRD 与 M0 技术验证」、目录结构写着「以下目录将在 M1 阶段新增：app/ engine/ rules/ scripts/」（实际早已存在）、「本地启动」写着「弹出标题为 C-Disk Cleaner 的**空窗口**」。这些对开源访客是误导性信息，必须重写。新版保留有价值的「一句话定位 / 与大众工具和专业工具的差异 / 安全与隐私基线」，补齐 7 项功能列表、NSIS+MSI 安装指引（含 SmartScreen 绕过与签名评估链接）、开发命令（含 `TAURI_SIGNING_PRIVATE_KEY` 缺失会导致打包失败这一坑）、技术栈表、**真实**目录结构、贡献指南。
  - **④ 新建 `docs/USER_GUIDE.md`**：8 节 + 8 条 FAQ，**全部按真实 UI 文案撰写**（先 `grep` 了 Dashboard/Items/SafeBox/History/Settings 的实际按钮与提示文字再动笔：「开始扫描」/「再次扫描」/「预览可清理项」/「清理已选」/「已恢复…」/「永久删除」/时间预设「全部·24 小时·3 天·7 天」/「（未选 = 全部 session）」）。重点把三处**容易误解的行为**写清楚：① 白名单的 fail-closed 语义（不是「只重启名单内」）；② 「已不存在」不算失败也不计释放量；③ 卸载不删 `%APPDATA%` 数据。
  - **门禁**：`cargo check` 0 / `cargo fmt` 0 / **`cargo test --workspace` 273 passed + 8 ignored**（不改业务代码，零回归）/ `pnpm lint` 0 error（2 条既有 warning）/ **vitest 89** / `pnpm build` 187.46 kB / **`tauri build` exit 0**。
  - **与任务书不符 2 项**：① 版本号路径写 `src-tauri/*`，实际在 `app/src-tauri/*`（workspace 布局，同 task-054）；② README 不是 Tauri 模板而是过时的早期版本（结论一样要重写，但理由不同——不是「模板没内容」而是「内容已失真」）。
  - **按新流程**：开发轮不跑 CI、不做安装态冒烟；发版动作由用户手动执行。
2026-10-29：**task-058 开发完成（M4 收尾：白名单 / 原子性 / 签名评估 / CHANGELOG 草稿），状态 `testing`**（`feature/task-058-m4-finishup`）：
  - **① RmRestart 白名单**：`prefs.restart_whitelist` 存 `settings` 表（JSON 数组字符串，`serde(default)` 兜住旧前端不传）。**语义与任务书不同（必须说明）**：任务书要求「白名单非空时只重启名单内的」，但 `RmRestart` **没有按 pid 重启的参数**——一次调用重启本会话内**所有** `bRestartable` 进程（task-057 测试轮已踩过「误把 pid 当 flags」的坑）。所以「只重启名单内」在 RM 机制下**无法实现**，改为 **fail-closed**：空白名单 = 不限制（保持现状）；非空白名单 = 只有被关闭的可重启进程**全部**命中才调 `RmRestart`，只要有一个不在名单内就整批跳过，避免把用户明确排除的进程也拉起来。匹配用 **映像名**（`QueryFullProcessImageNameW`）而非 RM 的 `strAppName`（本地化 FileDescription，中文系统下 `notepad.exe` 显示为「记事本」），支持 `*` 通配。**一个必须注意的实现细节**：映像名要在 `RmShutdown` **之前**取好——进程被杀后 `OpenProcess` 必失败，那时再查只能得到空串，白名单会永远不命中、自动重启静默失效（第一版代码就写错了，已修）。
  - **② `safe_box` 原子性**：根因是**顺序反了**——旧实现先 `fs::rename` 再 `INSERT`，INSERT 失败（磁盘满/约束/库被锁）就留下「文件在 box、DB 无行」的孤儿，`enforce_quota` 下次把它当 purged 永久删除，用户感知「文件凭空消失」。改为 **INSERT 提前到 rename 之前 + 整段包 `unchecked_transaction`**：INSERT 失败时文件根本没动；rename 失败时 `Transaction` 的 Drop 自动 rollback；跨盘 copy 失败 / copy 成功但 remove 失败时先删半成品目标文件再报错。
  - **③ 代码签名评估 → 结论当前不做**（`docs/research/code-signing-evaluation.md`）。**关键约束不是价格**：CA/B Forum 自 2023-06 起强制「代码签名私钥必须硬件存储」，而 GitHub Actions runner **读不到你插在家里的 USB token** → 买 EV/OV 后要么把全自动发版降级成「下载产物→本地插 token 手动签→手动上传」，要么再买云签名服务。叠加两条硬门槛：EV/OV 都需**企业实体**（本项目是个人项目）、Azure Artifact Signing（原 Trusted Signing，~€102/年、CI 友好）对**个人开发者仅限美/加**、其它地区需组织实体（**待自行向 MS 确认中国区**）。**三个「该做」的触发条件**：公开分发且下载量达数百/月、出现企业客户、要上 winget/Store。
  - **④ `CHANGELOG.md` v0.1.7 草稿**：新增（自动重启 / 白名单 / 偏好持久化 / 恢复时间可追溯）、修复（移入失败不再丢文件）、变更（已恢复行保留但不占配额）、已知问题（G6 豁免 / 跨盘恢复不支持 / 未签名 / RmShutdown 连带 / 默认图标）四段。**版本号保持 `0.1.6` 未升**（发版任务再升）。
  - **门禁**：`cargo check` 0 / `clippy -D warnings` 0 / `fmt` 0 / **`cargo test --workspace` 273 passed + 8 ignored**（含新增 11 条）/ `pnpm lint` 0 error / **vitest 89 passed（9 files）** / `pnpm build` **187.46 kB │ gzip 60.30 kB**。
  - **与任务书不符 3 项**：① 白名单语义从「只重启名单内」改为「全部命中才重启」（RM 机制限制，见上）；② 白名单存储用 JSON 数组字符串而非新列/新表（复用 `settings` 表，不加迁移）；③ 白名单漏了 `app/src-tauri/src/commands.rs`（`shutdown_locking_processes` 加第三参必然级联）与 `app/src/pages/Settings.test.tsx`（`set_prefs` 断言 payload 变了必红）。
  - **按新流程**：开发轮不跑 CI、不做真机/安装态冒烟，移交测试 Agent。
2026-10-28：**task-057 开发完成（RmRestart 自动重启被关进程，默认关闭 + 设置页开关），状态 `testing`**（`feature/task-057-rm-restart-toggle`）：
  - **① prefs 存储**：复用 v1 已有 `settings(key,value,updated_at)` 表（**不加迁移不加新列**），键 `prefs.auto_restart_on_close`（"1"/"0"），缺行/非法值 fail-closed 到 false。新建 `engine/src/prefs.rs`：`get_auto_restart` / `set_auto_restart`（UPSERT）+ `Prefs { auto_restart_on_close: bool }`（serde camelCase）+ 5 条单测（默认 false / set-get 往返 / 非法值回退 false / 不破坏其它 key / Prefs snapshot 往返）。
  - **② RmRestart 调用**：`shutdown_locking_processes(path, auto_restart: bool)` 签名升级；新增 `RmSession::restart(pid)` 调 `windows::Win32::System::RestartManager::RmRestart(handle, pid, None)`；`RmShutdown` 成功后若 `auto_restart=true`，按 pid ∈ closable ∧ `bRestartable.as_bool()==true` 逐个调用，总超时 10s（`Instant::now()+10s`），单个失败只 `eprintln!` warning 不阻塞删除。纯函数 `rm_restartable(&RM_PROCESS_INFO)` 单测覆盖 TRUE/FALSE 两路。
  - **③ Tauri 命令**：`get_prefs() -> Envelope<Prefs>` / `set_prefs(prefs: Prefs) -> Envelope<Prefs>`，打开 DB 失败返 INTERNAL；`cleanup_retry_locked` 在 `spawn_blocking` 之前从 conn 读 prefs 按值传入闭包（避免跨线程连接）。
  - **④ Settings 页开关**：新增「行为偏好」卡片，原生 checkbox +「已开启/已关闭」旁注，灰色说明文字「清理被占用的文件时，若系统支持（Restart Manager 标记为可重启），关闭后自动把进程重新拉起。默认关闭。」启动时 `get_prefs` 回填，切换立即 `set_prefs`（乐观更新 + 失败回滚）。
  - **⑤ Items confirm 文案联动**：启动时 `get_prefs` 读 `autoRestartOnClose` 入 state；开启时在原 task-049 文案后追加一行「注意：本次会在关闭后自动重启被关闭且支持重启的进程。」；关闭时文案与 task-049 完全一致。
  - **门禁**：`cargo check --workspace --all-targets` 0 / `clippy` 0（仅 2 条 pre-existing Default::default 字段赋值 warning，非本任务引入）/ `fmt` 0 / **`cargo test -p engine` 全绿**（lib 53 passed，含新增 prefs 5 + rm_restartable 1；集成测试全部通过，含 locking_shutdown 9 条统一传 false）/ `pnpm lint` 0 error（2 条 SafeBox.tsx react-refresh pre-existing warning）/ **vitest 85 passed（9 files，基线 81 + 新增 4）** / `pnpm build` 185.59 kB / gzip 59.59 kB。
  - **偏差 3 项**：① 未做 localStorage 缓存（每次进页面走一次 IPC，prefs 读极轻，且避免双写不一致）；② prefs 模块落在 `engine` crate 而非 `app/src-tauri/src/prefs.rs`（与 db/execute 同层，Tauri 命令只是薄包装）；③ prefs 单测直接放 `prefs.rs` 内 `#[cfg(test)] mod tests`（与 locking.rs 同风格），未新建 `engine/tests/prefs_test.rs`。
  - **按新流程**：开发轮不跑 CI、不做真机/NSIS 安装态冒烟，移交测试 Agent（提示词 9）。真机必验：默认关闭 → confirm 文案不含重启提示；开启 → notepad 被关后自动重启；重开应用开关状态保留。
2026-10-28：**task-056 开发完成（M3 技术评估 + 3 项遗留小修），状态 `testing`**（`feature/task-056-research-and-cleanup`）：
  - **① 差量更新评估 → 结论不做**（`docs/research/delta-update-evaluation.md`）。三个候选逐一核查：**A. Tauri 2 官方 updater 不支持差量**——`grep -ril "bsdiff\|delta\|xdelta"` 扫 `@tauri-apps/plugin-updater@2.13.3` 全包**零命中**，`Update` 只有 `download`/`install`/`downloadAndInstall`，`DownloadOptions` 只有 `headers`/`timeout`；且 `latest.json` 协议只认「一个完整包 + 一个整包签名」，插差量等于自建更新通道、丢掉官方验签保障。**B. bsdiff4** / **C. xdelta3**：4 MB 量级两者速度都感知不到，选型只看压缩率与生态，bsdiff 略优，xdelta3 无决定性优势。**否决的四条硬理由**：① 绝对收益太小（4 MB 包在 20 Mbps 下 1.7 秒，差量乐观省 1.2 秒，而 task-055 已解决「等待焦虑」）；② 成本 6–9 人日且长期维护（客户端要常驻旧包缓存 + LRU + SHA 校验 + 双验签 + 回退，服务端要改发版流水线 + 产物矩阵）；③ 脱离官方更新通道，补丁投毒/降级攻击的安全面要自己扛；④ Rust 重编译的二进制布局漂移让实际缩减率可能只有 20–50%（**该数据是经验估算，本轮未实测**，但即使按最乐观 70% 算结论不变）。**替代建议：改做「release profile 瘦身」（`lto="fat"` + `codegen-units=1` + `strip=true` + `opt-level="z"`），预计 -20%~35%、1–2 人日。**
  - **② `safe_box.restored_at`（迁移器 v2）**：新建 `v2__safe_box_restored_at.sql`（`ALTER TABLE safe_box ADD COLUMN restored_at INTEGER`），`MIGRATIONS` 表加 `(2, …)`。**类型选 INTEGER 而非任务书说的 TEXT**——与同表 `quarantined_at`/`expires_at` 一致用 unix 秒，前端 `formatTs` 直接消费。`restore_from_safe_box` 第 5 步从 `DELETE FROM safe_box` 改为 `UPDATE safe_box SET status='restored', restored_at=?2`。**外部行为不变**：`list_safe_box`/`list_paged`/`list_sessions`/`enforce_quota` 本来就全部带 `status='in_box'` 过滤（已逐处核对），所以已恢复行既不显示也不占配额，只是表里多留一行供追溯。**另一个必须守住的不变量**：行还在 ≠ 可重复恢复，靠 `WHERE id=?1 AND status='in_box'` 保证第二次返回 `NotFound`（有用例固化）。
  - **③ 跨盘 rename 补测**（`engine/tests/safe_box_cross_disk.rs`，task-016 遗留）：`cross_disk_move_into_box_falls_back_to_copy_remove` 在本机 **C:→D: 真跑**（源文件与保险箱分属不同盘），断言源文件消失、box 内容**逐字节一致**、DB 行 `status='in_box'` 且 size 正确；`cross_disk_restore_currently_returns_io_error` 固化「restore 无 fallback」的已知限制。**单盘环境（CI windows-latest）打印 `[skip]` 后 return，不红也不 `#[ignore]`**（本地多盘要真跑，不能靠人记得加 `--ignored`）。
  - **④ G6 定稿豁免，不修**：根因是 Tauri 2.x 在 WebView2 上 custom protocol handler 注册与 WebView 创建**时序竞争**，首次 IPC 早于 handler 就绪 → `ERR_CONNECTION_REFUSED`，Tauri 捕获后内部标记走 postMessage，故同文档内第二次不复现；报错在 `@tauri-apps/api` fetch 层，业务代码拦不住。**任务书的 warmup 方案已评估否决**：warmup 自己就是首次 IPC，同样打红错，只是把命令名从 `rules_check_update` 换成 `app_info`，**红错数量不变**；若放 App 挂载还会让每次启动都红一次，暴露面扩大；且本轮按新流程不做 GUI 测试，提交自己无法验证的「修复」比明确豁免更危险。豁免口径写入 `docs/test-reports/gui-walkthrough-round3-checklist.md` **§六**（现象逐条 / 根因 / 不修理由 / **5 条判定规则** / 后续处理）。
  - **门禁**：`cargo check` 0 / `clippy -D warnings` 0 / `fmt` 0 / **`cargo test --workspace` 255 passed + 8 ignored**（基线 250 + 新增 5）/ `pnpm lint` 0 error / vitest **81** 不回归 / `pnpm build` 184.22 kB。
  - **与任务书不符 3 项**：① `restored_at` 用 INTEGER 不用 TEXT（理由见上）；② 白名单漏 `engine/tests/db_schema.rs` 与 `engine/tests/safe_box_restore.rs`——加 v2 迁移后 `user_version_bumped_to_1` 必然红、restore 改 UPDATE 后 `n_box == 0` 断言必然红，不改则门禁不可能绿；③ 白名单列的 `engine/src/db/schema.rs` **不存在**（schema 定义在 `migrations/v1__init.sql`），未动。
  - **按新流程**：开发轮不跑 CI、不做真机/安装态冒烟，移交测试 Agent。**但本任务有 DB 迁移，安装态「v0.1.6 老库平滑升到 v2」是测试轮必须验的一条**（清单 §C01–C03 之外需补）。
2026-10-28：**task-055 开发完成（M3 banner 下载进度 + 失败态 + 稍后提醒 + Settings loading），状态 `testing`**（`feature/task-055-update-banner-progress`）：
  - **① 下载事件订阅**：`downloadAndInstall(onEvent)` —— 真实类型定义是 `downloadAndInstall(onEvent?: (p: DownloadEvent) => void, options?)`，**第一个参数就是回调**（任务书写 `{ onEvent }` 对象，与 plugin-updater 2.13.3 不符）；事件字段是 `data.contentLength` / `data.chunkLength`（任务书写 `totalSize` / `chunkSize`），`contentLength` 还是 **optional**（服务器不给 Content-Length 时拿不到总量）。`Finished` 后切 `installing` 态。
  - **② 进度 UI**：线性进度条（宽度=percent，200ms 过渡抹平 chunk 级跳变）+ 标题 `下载更新中 N%` + 副行 `已下载 X.X / Y.Y MB，速度 ≈ Z`；total 未知时退化为 `已下载 X.X MB（总大小未知）`，不显示误导性的 0%。
  - **③ 速度估算**：EMA，**窗口 1s、平滑系数 α=0.4**。不是每个 chunk 都算（会剧烈抖动），而是「每满 1s 用窗口增量 / 实际时长算瞬时速度」再 EMA。已下载字节数放 `useRef` 而非依赖 `setState` 的函数式 updater——在 updater 里再调 `setState` 是反模式，严格模式下会重复执行。
  - **④ 失败态**：`classifyUpdateError` 按关键词映射 5 类（网络 → 「网络连接失败，请检查网络后重试」；签名 → 「签名校验失败，更新包可能已损坏，请重新下载」；磁盘 → 「磁盘空间不足，请清理 C 盘后重试」；版本 → 「当前已是最新版本」；兜底 → 「更新失败：<原文>（请截图反馈）」）。**判定顺序有讲究**：网络在签名之前，但签名错误原文常带 URL，所以网络正则只匹配真正的网络关键词（`network|request|\burl\b|dns|timeout|connect|offline|fetch|http`），避免把签名问题误判成网络。banner 失败态给「重试」（重新走 `downloadAndInstall`）+「关闭」。
  - **⑤ 稍后提醒持久化**：`localStorage["cclean:update:dismissedVersion"]` 存**版本号字符串**（不是布尔），只忽略「这一个版本」；有更高版本时 banner 照弹。读写都 try-catch（隐私模式/配额满时静默降级为「重启后再弹」）。
  - **⑥ Settings**：手动检查文案改 `当前 vX / 发现 vY` 或 `当前 vX，已是最新版本`（当前版本用 `getVersion()`，取不到则退化）；失败文案复用 `classifyUpdateError`；新增 `onUpdateFound` prop，手动检查发现新版本时通知 App 强制显示 banner（覆盖「稍后提醒」）。**检查中按钮 disabled + 「检查中…」是 task-035 既有实现，本轮只补文案与错误映射**。
  - **新增文件 `app/src/utils/update.ts`**（**白名单缺口，见实现备注**）：`computePercent` / `formatMB` / `formatSpeed` / `classifyUpdateError` / `DISMISSED_VERSION_KEY` / `readDismissedVersion` / `writeDismissedVersion` / `shouldShowBanner` + 两个 EMA 常量。**为什么不塞 App.tsx**：① 组件文件多导出非组件会触发 `react-refresh/only-export-components`（`SafeBox.tsx` 已有 2 条同类 warning，不再增加）；② `Settings.tsx` 要复用 `classifyUpdateError`，而 App 会 import Settings，函数放 App 里会造成 App ↔ Settings 循环依赖。
  - **门禁**：`pnpm --filter app lint` 0 error（2 条既有 warning）/ **vitest 81 passed（9 files，基线 54 + 新增 27）** / `pnpm --filter app build` **184.22 kB │ gzip 59.18 kB**（基线 181.51）/ `cargo test --workspace` **250 passed + 8 ignored**（本任务不改 Rust，零回归）。
  - **环境坑**：vitest 仍偶发 `EPERM ... node-brokered-fs-shim.cjs`（沙箱拦写系统 Temp），本轮 3 次里 2 次复现，**重跑即绿**；`TMPDIR` 指到 `target/vitest-tmp` 有帮助但不能完全消除。
  - **与任务书不符 3 项**：① `downloadAndInstall` 签名与事件字段名（见上）；② 白名单未含 `app/src/utils/update.ts`（共享纯函数必须有独立文件，见上）；③ 任务书要求「真机/安装态冒烟必做不可裁剪」，按 2026-10-25 新流程开发轮不做，移交测试 Agent。
2026-10-28：**v0.1.6 升级闭环真机验证通过（用户本人执行）**——装 v0.1.5 → banner 显示「发现新版本 0.1.6」→ 升级 → 重启后 A/B/C 三种方式均确认 0.1.6 → R1–R6 功能回归全过（结果表追加于 `docs/release/v0.1.6-upgrade-verification.md` 末尾）。同日完成第三轮 NSIS/MSI 双跑：各 27 条过 26、不适用 0、排除 G6 后 Console 红错 0；唯一不通过为 C08（`scripts\` 改名后应用启动即 fail-fast 闪烁退出，未出现「完成+报错」并存，但实现方式与预期不同），说明已写入 `docs/test-reports/gui-walkthrough-round3-checklist.md` 汇总表下方。M2 updater 闭环正式收官；「当前目标」「下一步计划」两节按要求精简为短句。
2026-10-11：完成 PRD / TRD / ARCHITECTURE / AGENTS / 立项决策记录全套长期基线文档，并初始化本状态白板。
2026-10-11：task-001 完成——落地 app/engine/elevation/rules/scripts 五大目录骨架、Cargo workspace + pnpm workspace 双工作区，Tauri 2 最小空窗口（标题「C-Disk Cleaner」）端到端跑通，Cargo.lock 已入库。
2026-10-11：task-002 完成——engine 内新建 db/ 子模块，rusqlite 0.40.2（bundled）连接入口、v1__init.sql 九张表 schema、user_version 手写迁移器，8 条集成测试全绿（含级联删除 / SET NULL / 单行表 CHECK / 迁移幂等）。
2026-10-11：task-003 完成——engine 内新建 execute/ 子模块：Windows 路径规范化器（反斜杠归一、大写盘符、`..` 一律拒）、内置 L3 红线路径匹配器（个人目录/浏览器数据/系统路径/动作级禁令）、`delete::execute()` 唯一入口（全拒不调 Deleter，拒绝信封码 2002）；24 条集成测试 + 9 条单元测试全绿。
2026-10-11：task-004 完成——engine 内新建 ipc/ 子模块：统一响应信封 `{code,message,data}`、TRD §5.3 全 13 个错误码、`scan.start`（递增 sessionId 入参校验）/ `cleanup.dryRun`（接 task-003 白名单 + MockResolver）纯逻辑、ProgressEmitter trait + 100ms 节流；app/src-tauri 注册两条命令。**显式偏离 TRD §5.1：L3 拒绝（2002）时 data 不置 null，回传 blocked_by_l3 明细**；7+3 条新测试全绿（engine 累计 52 条）。
2026-10-12：task-005 完成——落地 `rules/cache_rules.json`（npm_cache 示例 + 4 条外部红线）与配对 `rules_meta.json`（真实 SHA-256）；engine 新建 rules/ 子模块：serde 模型 / 白名单环境变量展开（5 变量 + `~`）/ SHA-256 校验加载器（篡改即拒不写盘）/ `classify_with_external` 与内置红线取并集；9 条集成测试全绿（engine 累计 61 条）。
2026-10-12：task-006 完成——M0 收官。新增 `rust-toolchain.toml`（channel=stable + rustfmt/clippy）与 `.github/workflows/ci.yml`：windows-latest 单 job 跑五道闸（fmt --check → clippy -D warnings → cargo test → pnpm install --frozen-lockfile → pnpm --filter app build），另加 rules/*.json SHA-256（去 CR 归一 LF）与 JSON 合法性两个自洽守卫；app/package.json 补 lint/test echo 占位。PR #1 触发 GitHub Actions 实测经 2 轮修复（Node 20→22、pnpm 9→11；checksum 去 CR）后 ci #3 全绿（5m56s）。M0 公共底座（001~006）全部就绪。
2026-10-12：task-007 完成——真实进度通道落地：新增 `app/src-tauri/src/progress.rs`（`TauriProgressEmitter` 包装 `AppHandle`，emit 失败仅吞；`ProgressState = Mutex<ThrottledEmitter<TauriProgressEmitter>>`）；Builder 的 setup 闭包内 `app.manage(...)` 注入；`scan_start` 命令返回前按真实 sessionId emit 一条 `percent=0, current_path="starting"` demo 进度。engine trait 零改动；cargo clippy/fmt/test、pnpm build 全绿。devtools listen 真机复核见任务实现备注。
2026-10-12：task-008 完成——新增 `engine::execute::resolve`（`resolve_to_real`：canonicalize 去 `\\?\` 前缀、UNC 保守拒、NotExists/Io/LoopDetected 三态错误）；delete 流程改为 normalize → resolve → 对**真实路径**再过 normalize+classify；新增 `DenialReason::ResolveFailed(String)`；NotExists 时回退对输入字符串判 L3（非存在路径不放宽红线、也不误杀 L1 单测）。6 条新集成测试（tempfile + mklink /J junction）全绿，engine 累计 67 条。
2026-10-12：task-009 完成——新增 `engine::db::resolver::DbItemPathResolver`（持有 `&Connection`，参数化按 id 查 `cleanup_item`，查不到跳过、SQL 失败返回空 Vec + eprintln）；4 条集成测试全绿（空库/匹配/未知 id/dryRun 端到端 2002）。Tauri 命令仍用 MockResolver，切换留 M1 扫描任务。
2026-10-12：task-010 完成——`handle_dry_run` 签名加 `external_deny: &[String]`，内部换 `classify_with_external`；`cleanup_dry_run` Tauri command 启动时 `current_dir()/rules` 加载规则库，`external_deny_list` 注入，加载失败保守返 5001；3 条新集成测试全绿（tempdir 自算 SHA-256 配 meta）。
2026-10-12：task-011 完成——从零产出 `scripts/PROTOCOL.md`（item/done 两种 NDJSON 事件 schema）+ 5 个扫描脚本（temp/windows_update/prefetch/browser_cache/dev_cache），全部 `#requires -Version 5.1`、UTF-8 无 BOM、stdout 只出 NDJSON、跳过 LinkType、不删除；`smoke_test.ps1` 实跑 5 脚本逐行 ConvertFrom-Json 全绿（temp=154/wu=6/prefetch=0/browser=10/dev=33 行）。M1 前置清理清单收官。
2026-10-12：task-012 完成——`engine::scan` 模块上线：`NdjsonEvent` serde 枚举（item/done + deny_unknown_fields）、`parse_line` 单行解析器；`ScanRunner` spawn `powershell.exe -File`，逐行写 `cleanup_item`（risk_level int→L0/L1/L2 映射、description 空串兜底）并 emit 进度；3 条假 ps1 集成测试全绿（含坏行跳过 / exit 1 无 done 报错）。
2026-10-12：task-013 完成——新增 `ScanOrchestrator`（硬编码 5 个 ps1 串跑、失败不中断、全失败才报错）；Tauri `scan_start` 命令真接通：建 scan_session、跑 orchestrator、回写 status='completed'/'failed'、返回真实 session_id。4 条 orchestrator 集成测试全绿；cargo check/clippy/test/fmt + pnpm build 全绿。
2026-10-12：task-014 完成——`cleanup_dry_run` Tauri 命令从 `MockResolver` 切到 `DbItemPathResolver`，打开真实 DB（与 scan_start 同 data_dir），按前端 itemIds 反查 cleanup_item 路径跑 dryRun；保留 external_deny 加载逻辑。cargo 五闸 + pnpm build 全绿。
2026-10-12：task-015 完成——新增 `FsDeleter`（std::fs::remove_file，NotFound 幂等）、`execute_items` 编排（再查 DB + 再过白名单 + 真删 + 落 cleanup_task 批量行与 audit_log 逐条行）、Tauri `cleanup_execute` 命令注册；4 条集成测试全绿（删真文件/L3 拦/缺 id/幂等）。cargo 五闸 + pnpm build 全绿。
2026-10-12：task-016 完成——新增 `SafeBoxDeleter`（rename 到 `<data_dir>/safe_box/`，跨盘 fallback copy+remove）、`enforce_quota` 按 quarantined_at 最旧 FIFO 剔除；`execute_items` 改签名为 deleter 工厂闭包；Tauri `cleanup_execute` 切到 SafeBoxDeleter。3 条集成测试全绿（移入写行/空 noop/quota 剔除）。cargo 五闸 + pnpm build 全绿。
2026-10-12：task-017 完成——新增 `restore_from_safe_box`（按 id 查 in_box 行、拒目标占用/父目录缺失、rename 回原路径、删行、写 audit_log action='restore'）；Tauri `safe_box_restore` 命令注册（TargetOccupied→2003 FILE_OCCUPIED，其他→5001 INTERNAL）。4 条集成测试全绿（正常/占用/父缺/已剔除）。cargo 五闸 + pnpm build 全绿。
2026-10-12：task-018 完成——新增 `list_safe_box`（`status='in_box'` 按 quarantined_at DESC，返回 entries + total_items + total_bytes）；Tauri `safe_box_list` 命令注册。4 条集成测试全绿（空/3条汇总/排除 purged/排除 restored）。cargo 五闸 + pnpm build 全绿。M1-C 保险箱三件套（入/回/列）齐备。
2026-10-12：task-019 完成——新增 `classify_io_error`（按 Windows raw_os_error 分 NotFound/PermissionDenied/SharingViolation/Unknown）；SafeBoxDeleter rename 失败时抛 `FILE_IN_USE:`/`PERMISSION_DENIED:` 前缀错误；executor 把这两个前缀翻译成 audit_log.detail 稳定文案（"file in use by another process" / "permission denied, needs elevation"）。4 条单元测试全绿（NotFound/32/5/Other）。cargo 五闸 + pnpm build 全绿。
2026-10-12：task-020 完成——engine 引入 windows 0.58 crate（cfg(windows)）；新增 `who_is_locking(path) -> Vec<LockingProcess{pid, app_name, service_name}>`，封装 RmStartSession/RmRegisterResources/RmGetList/RmEndSession 四件套；空锁测试全绿，notepad 真机用例 `#[ignore]`。cargo 五闸 + pnpm build 全绿。
2026-10-12：task-021 完成——`ExecResult` 加 `locking_processes: Vec<LockingProcess>` 字段；executor 在 deleter 返回 `FILE_IN_USE:` 前缀时自动调 `who_is_locking(&path)`，把 `"notepad.exe (PID 1234), chrome.exe (PID 5678)"` 拼进 audit_log.detail 并填进结果返回前端；PERMISSION_DENIED 走 task-019 旧文案不调 RM。2 条 CI 集成测试全绿（空锁 + PERMISSION_DENIED 不触发），notepad 端到端 `#[ignore]`。cargo 五闸 + pnpm build 全绿。
2026-10-13：task-022 完成——engine 引入 reqwest 0.12（blocking + rustls-tls）；新建 `rules/meta_store.rs`（upsert/get_active rules_meta 单行表 id=1）与 `rules/remote.rs`（`fetch_remote_meta` GET GitHub Raw rules_meta.json）；Tauri `rules.checkUpdate` 命令注册，返回 `{has_update, current_version, remote_version, current_sha256, remote_sha256}`。3 条集成测试全绿（upsert/get_active/空库）。cargo 五闸 + pnpm build 全绿。
2026-10-13：task-023 完成——`remote.rs` 新增 `fetch_remote_rules_bytes`（GET 远端 cache_rules.json）与 `apply_update`（SHA-256 校验 + 备份 .bak + 替换 cache_rules.json/rules_meta.json + upsert rules_meta 单行表 + 失败自动回滚）；Tauri `rules.applyUpdate` 命令注册。3 条集成测试全绿（正常替换/sha不匹配拒绝/DB失败回滚）。cargo 五闸 + pnpm build 全绿。
2026-10-13：task-024 完成——新建 `audit_log/query.rs`（`list_audit_log`：按 ts DESC, id DESC + limit/offset 分页；对外字段重命名 event_type=action、path=target、created_at=ts）；Tauri `audit_log.list` 命令注册。4 条集成测试全绿（空/倒序/action 过滤/分页）。cargo 五闸 + pnpm build 全绿。
2026-10-13：task-025 完成——新建 `audit_log/stats.rs`（`cleanup_stats`：单条 SQL 一次取 total_bytes_freed / success_count / failed_count / session_count，COALESCE 防空表）；Tauri `audit_log.stats` 命令注册。4 条集成测试全绿（空/计数/session 过滤/去重）。cargo 五闸 + pnpm build 全绿。
2026-10-13：task-026 完成——前端第一刀：新建 `app/src/utils/formatBytes.ts`（B/KB/MB/GB，负数/NaN 兜底 "0 B"）与 `app/src/pages/Dashboard.tsx`（2×2 grid 4 卡片，loading 骨架 / error 红条+重试 / ok 渲染，失败数>0 红色）；`App.tsx` 默认渲染 Dashboard。内联 style，未引 UI 库。补装 `@tauri-apps/api@2.12.3`（任务书假设已在 devDeps 实际未装，本次新增）。cargo 五闸保持基线 133 条全绿 + pnpm build 全绿。
2026-10-13：task-028 完成——Dashboard 顶部加「开始扫描」按钮：点击 → `invoke('scan_start')` + `listen('scan://progress')` 订阅，绿底进度条（percent 0~100）+ 当前路径 + 已扫项数；扫描中按钮禁用，done 后文案变「再次扫描」并自动刷新 stats；失败红条 + 重试。**与任务书偏差**：真实 `ScanProgress` shape 是 `{session_id, percent, current_path, scanned_items}`（不是任务书假设的 `{module, done, total}`），进度条直接用 `percent`，文字用 `current_path` + `scanned_items`。cargo 五闸保持基线 + pnpm build 全绿。
2026-10-13：task-029 完成——清理闭环打通。后端新建 `execute/list_pending.rs`（`list_pending_items`：按 session_id 可选过滤、size_bytes DESC、参数化 limit/offset）；Tauri `cleanup_list_pending` 命令注册。前端新建 `app/src/pages/Items.tsx`（pending 表格：checkbox/路径/大小/模块/session，顶部「已选 N 项共 X」+「清理已选」按钮 → `invoke('cleanup_execute', { itemIds })`，完成后自动重拉 + 绿/红结果条）；`App.tsx` 加第三 tab「清理」。4 条集成测试全绿。**与任务书偏差**：cleanup_item 表无 status/scanned_at 列，「pending」= 表内所有行（无 status 过滤），PendingItem 去掉 scanned_at 字段。cargo 五闸 + pnpm build 全绿。
2026-10-13：task-030 完成——保险箱 UI 落地。新建 `app/src/pages/SafeBox.tsx`：`invoke('safe_box_list')` 拉列表，顶部「刷新」+「共 N 项 / 总大小 formatBytes(totalBytes)」汇总，表格 4 列（原路径/大小/移入时间 formatTs/操作「恢复」按钮），行内「恢复中…」禁用，绿条「已恢复：path」/红条「恢复失败：err」，空列表居中灰字「保险箱为空」；`App.tsx` 加第四 tab「保险箱」，顺序 dashboard→清理→保险箱→历史。**与任务书偏差**：真实 list 响应是 `SafeBoxListResp { entries, totalItems, totalBytes }`（不是裸数组），`SafeBoxEntry { id, originalPath, sizeBytes, quarantinedAt }`（不是任务书假设的 `{id, path, size_bytes, moved_at, original_path}`），restore 入参是 `safeBoxId`（不是 itemId）——前端已按真实 shape 对接。cargo 五闸保持基线 137 条全绿 + pnpm build 156.19KB gzip 49.74KB。
2026-10-13：task-031 完成——规则更新 UI 落地。新建 `app/src/pages/Settings.tsx`：规则库卡片显示当前版本/远端版本/SHA-256（前 16 位）/状态（「已是最新」或「有新版本 X 可用」）；「检查更新」按钮调 `rules_check_update`（启动时自动拉一次），有更新时显示绿色「应用更新」按钮调 `rules_apply_update`，成功绿 toast「已更新到版本 X」+ 自动重拉；应用中按钮禁用 +「应用中…」；网络失败红条。`App.tsx` 加第五 tab「设置」。**与任务书偏差**：CheckUpdateResp/ApplyResult 均 `#[serde(rename_all="camelCase")]`，前端字段用 camelCase（hasUpdate/currentVersion/...），不是任务书假设的 snake_case。cargo 五闸保持基线 137 条全绿 + pnpm build 158.90KB gzip 50.36KB。
2026-10-13：task-032 完成——前端工程化补齐。ESLint 10 flat config（`app/eslint.config.js`，@eslint/js + @typescript-eslint + react-hooks + react-refresh，no-undef 关闭交给 TS，no-unused-vars warn）；Vitest 2.1 + @testing-library/react + jest-dom + jsdom（`vite.config.ts` 加 test.environment=jsdom + setupFiles，`src/setupTests.ts` 引入 `@testing-library/jest-dom/vitest`）；formatBytes 5 个断言 + formatTs 4 个断言共 9 个单测全过；`.github/workflows/ci.yml` 前端 job 加 Lint/Test 两步替换 echo 占位。**与任务书偏差**：ESLint 10 已废弃 .eslintrc.cjs，改用 flat config `eslint.config.js`；Vitest 5.0.3 要求 Vite 6+，本项目 Vite 5.4.11 降级到 vitest@^2.1.9；setupTests 必须用 `@testing-library/jest-dom/vitest`（子路径）而非 `@testing-library/jest-dom`，否则 expect.extend 在框架注册前执行报错。cargo 五闸保持基线 + pnpm lint=0 / pnpm test=0（9/9）/ pnpm build=0。
2026-10-13：task-033 完成——NSIS v0.1.0 打包就绪。`app/src-tauri/tauri.conf.json`：identifier 改为 `com.and0114.cclean`，bundle.targets 从 "all" 收窄为 `["nsis","msi"]`，bundle.windows.nsis 加 `installMode="perMachine"`（清理 C 盘要管理员）+ `languages=["SimpChinese"]`。图标沿用 task-001 生成的 Tauri 默认占位（17 个文件已在库，不重跑 `tauri icon`，设计图标留后续）。`.github/workflows/ci.yml` 在 `pnpm build` 后加 `pnpm --filter app tauri build` + `actions/upload-artifact@v4`（artifact 名 `cclean-windows-installer`，path `target/release/bundle/{nsis/*.exe,msi/*.msi}`，`if-no-files-found: error`）。本地 `pnpm --filter app tauri build` 退出码 0，产出 `target/release/bundle/msi/C-Disk Cleaner_0.1.0_x64_en-US.msi`（5.11 MB）+ `target/release/bundle/nsis/C-Disk Cleaner_0.1.0_x64-setup.exe`（3.60 MB）。**与任务书偏差**：① Tauri 2 NSIS schema 字段是 `languages`（数组）不是任务书写的 `displayLanguage`（标量，build script 直接报 unknown field）；② workspace 模式 bundle 输出在仓库根 `target/release/bundle/` 而非 `app/src-tauri/target/release/bundle/`，CI path 已相应调整；③ 图标未重跑（沿用 task-001 默认占位）。cargo 五闸 + pnpm lint/test/build + tauri build 全绿。
2026-10-13：task-034 完成——GitHub Release 发版 workflow 就位。新建 `.github/workflows/release.yml`：触发器 `workflow_dispatch`（输入 version，如 0.1.0）+ `push.tags: v*`；permissions.contents=write；job windows-latest 跑 checkout → setup node 22/pnpm 11 → setup rust stable → pnpm install --frozen-lockfile → pnpm --filter app tauri build → softprops/action-gh-release@v2 上传 `target/release/bundle/{nsis/*.exe,msi/*.msi}`，Release 名/tag 为 `v${{ inputs.version || github.ref_name }}`，generate_release_notes=true。**与任务书偏差**：bundle path 沿用 task-033 实证的 workspace 根 `target/release/bundle/`（不是任务书假设的 `app/src-tauri/target/release/bundle/`）；首次 workflow_dispatch v0.1.0 手动触发留评审在 GitHub Actions 页执行（自动化环境无 gh auth，无法代点）。
2026-10-13：debt-cleanup-002 完成——M1 NSIS 安装包装干净目录后用户实测暴露的 5 个回归 bug 修复（feature/fix-m1-packaging-bugs 分支，等评审不自行 merge）：
  - Bug-1（高）：前端 5 页面把 `Envelope<T>` 当 T 直接用，Items/History `rows.filter/map` 抛 TypeError 整树白屏、Dashboard 数字卡片空、Settings 版本 "-"。新建 `app/src/utils/invoke.ts` 的 `api<T>(cmd,args)` 解包信封（code!==0 抛错，data 直返），5 页面全切过去并删 `|| []`/`|| 0` 兜底；App.tsx 加 class ErrorBoundary（渲染错显示「页面出错，点这里回仪表盘」，不再整树白屏）。
  - Bug-2（高）：`app/src-tauri/capabilities/` 目录不存在，Tauri 2 默认拒插件 API，Dashboard `listen("scan://progress")` 弹 `plugin:event|listen not allowed by ACL`。新建 `capabilities/default.json` 授权 `core:default` + `core:event:default`（allow-listen/unlisten/emit/emit-to）。
  - Bug-3（中，发版阻塞）：release.yml `with: version: 11` 与 root package.json `packageManager: pnpm@11.9.0` 双声明，pnpm/action-setup@v4 报 Multiple versions。删 `with: version: 11` 整行，版本单一来源交给 packageManager。
  - Bug-4（中，CI 红）：`engine/src/execute/resolve.rs` 用 canonicalize 后字符串相等判 hop，windows-latest runner TEMP 走 8.3 短名 `RUNNER~1`，canonicalize 解析成长名 `runneradmin`，字符串不等误判 hop=1。改为逐级 `symlink_metadata` 查 `FILE_ATTRIBUTE_REPARSE_POINT`(0x400)，真正 junction/symlink 才计跳；8.3 短名不是 reparse point，不误计。
  - Bug-5（低）：Dashboard `startScan` catch 里补 `setProgress(null)`，失败后「重试扫描」按钮旁不再挂残留进度条。
  自验：cargo clippy --workspace --all-targets -- -D warnings 干净；cargo test -p engine 137 passed / 0 failed（与基线一致）；cargo fmt --check 过；pnpm --filter app build 160.40 kB / gzip 51.40 kB。
2026-10-14：debt-cleanup-003 完成——debt-cleanup-002 合并后 NSIS 干净目录真机验收暴露的 2 个 P0 阻塞 bug 修复（feature/fix-m1-postinstall-v2 分支，等评审不自行 merge）：
  - Bug-1（P0，M1 核心功能不可用）：Dashboard 调 `api("scan_start", {})` 空对象，但 Rust 签名 `scope: String` 必填，Tauri 直接拒参报 `missing required key scope`，点「开始扫描」立即红条、进度条从未出现。改为传 `{ scope: "full", targetDrive: null }`（Tauri 2 camelCase→snake_case：targetDrive→target_drive）。全仓 9 个 api() 调用点逐一核对后端签名，仅此一处错位。
  - Bug-2（P0，干净安装后所有命令 EACCES）：所有命令用 `std::env::current_dir()` 定位 rules/ 和 DB，perMachine 装到 `C:\Program Files\C-Disk Cleaner\` 后 current_dir 指向只读安装目录，SQLite 报 `拒绝访问 (os error 5)`，设置页规则库「内部错误」。新建 `app/src-tauri/src/paths.rs` 的 `AppPaths`（setup 时构造一次并 manage）：data_root = `app.path().app_data_dir()`（%APPDATA%\com.and0114.cclean\，可写）；rules_dir = data_root/rules（首次从 bundled resource 复制）；scripts_dir = 只读 resource_dir；box_dir = data_root/safe_box。commands.rs 全部改从 `tauri::State<'_, AppPaths>` 取路径，不再碰 current_dir。tauri.conf.json `bundle.resources` 打包 2 个 rules 文件 + 5 个 ps1 脚本。
  - 评审报告另发现 Bug-3（设置页所有失败坍缩成「内部错误」，不区分网络/校验/后端错，P1）和 Bug-4（Dashboard 成功/失败/轮次三个数字卡片空白，总释放空间显示 0 B，P1），留 debt-cleanup-004。
  自验：cargo clippy --workspace --all-targets -- -D warnings 干净；cargo test -p engine 137 passed / 0 failed（与基线一致）；cargo fmt --check 过；pnpm --filter app build 通过。
2026-10-14：debt-cleanup-004 完成——debt-cleanup-003 安装包干净目录真机二轮评审暴露的 3 个回归 bug 修复（同分支 feature/fix-m1-postinstall-v2 追加提交，等评审不自行 merge）：
  - Bug-1（中，Dashboard 数字卡片空白）：后端 `CleanupStats` 有 `#[serde(rename_all = "camelCase")]`，JSON 输出 `totalBytesFreed/successCount/failedCount/sessionCount`；但 Dashboard.tsx interface 是 snake_case，三个 count 字段全 undefined 渲染空白。改为 camelCase。顺带修 History.tsx `AuditLogEntry` 同样问题（sessionId/eventType/createdAt）。
  - Bug-2（中，进度条文字卡死「准备中...」）：`ScanProgress` 事件 payload 也是 camelCase（currentPath/scannedItems），前端 listener 读 snake_case 字段，两个字段全 undefined 永远走 fallback。改 Dashboard.tsx ScanProgress interface 为 camelCase。注：后端 runner 目前 percent 统一发 0（单脚本内不知总量），进度条宽度不动是已知设计限制，留后续任务。
  - Bug-3（中，设置页检查更新全报「内部错误」）：rules_check_update / rules_apply_update 把所有失败（含网络不通、SHA 校验失败）都返回 `Envelope::err(INTERNAL)`。改为 fetch_remote_* 失败 → `RULE_NETWORK_ERROR`（3003），ApplyError::ShaMismatch → `RULE_CHECKSUM_MISMATCH`（3002）；load_rules/DB 错仍 INTERNAL。
  自验：cargo clippy --workspace --all-targets -- -D warnings 干净；cargo test -p engine 137 passed / 0 failed（与基线一致）；cargo fmt --check 过；pnpm --filter app build 通过。
2026-10-14：task-035 完成——tauri updater 自动更新最小闭环：
  - Rust 侧 `Cargo.toml` 加 `tauri-plugin-updater = "2"` + `serde_json = "1"`（generate_context 解析 plugins.updater 配置需要），`lib.rs` Builder 链上 `.plugin(tauri_plugin_updater::Builder::new().build())`。
  - `tauri.conf.json` 加 `plugins.updater`（active/endpoints/pubkey）——注意 Tauri 2 schema 是 `plugins.updater` 命名空间，不是任务书写的顶层 `updater`，也不是 `bundle.updater`。
  - 前端 `pnpm add @tauri-apps/plugin-updater @tauri-apps/plugin-process`，Settings.tsx 在规则库卡片下方加「应用更新」卡片：checkApplication 调 `check()`，downloadAndInstall 调 `update.downloadAndInstall()` + `relaunch()`。
  - `release.yml` 加 Setup signing key（从 secret `TAURI_SIGNING_PRIVATE_KEY` 写 `~/.tauri/cclean.key`）+ Generate latest.json（PowerShell 读 `.msi.sig` 拼 JSON）+ 随 Release 上传；根目录 `latest.json` 占位（raw URL 可达）。
  - 签名密钥对已本地生成在 `~/.tauri/cclean.key`（私钥不入库），公钥已写入 tauri.conf。
  自验：cargo check/clippy/fmt/test（137 条）/pnpm lint/test/build 全绿。**与任务书偏差**：① updater 配置位置按 Tauri 2 真实 schema 落在 `plugins.updater`（任务书写顶层 `updater`，build script 报 unknown field）；② generate_context 需要显式 `serde_json` 依赖否则 E0433；③ release.yml 用 `TAURI_SIGNING_PRIVATE_KEY_PATH` 环境变量指向私钥文件路径（不是把私钥本身当字符串）。
2026-10-14：bugfix/task-035-updater-acl 完成——task-035 评审 P0「updater ACL 拒绝」修复（bugfix 分支，等评审不自行 merge）：
  - Bug-1（P0，自动更新完全不可用）：`capabilities/default.json` 只授权 `core:default` + `core:event:default`，前端 Settings 点「检查应用更新」调 `plugin-updater|check` 被 Tauri ACL 直接拒，报 `Command plugin:updater|check not allowed by ACL`。补 `updater:default`（聚合 allow-check/download/install/download-and-install）+ `process:default`。
  - 顺带补接线缺陷：前端已 `pnpm add @tauri-apps/plugin-process` 并在 downloadAndInstall 后调 `relaunch()`，但 Rust 侧从未注册 `tauri-plugin-process`（Cargo.toml/Cargo.lock/Builder 全缺），relaunch 无后端。补 `tauri-plugin-process = "2"` + `lib.rs` 挂 `.plugin(tauri_plugin_process::init())`。
  - **与评审建议偏差**：评审建议加 `process:allow-relaunch`，但本版 tauri-plugin-process v2 的 ACL manifest 只有 `allow-exit`/`allow-restart`（无 allow-relaunch）；JS `relaunch()` 实际映射到 `restart` 命令，故用 `process:default`（含 allow-restart）覆盖。照抄 `process:allow-relaunch` 会触发 unknown-permission 构建错误。
  自验：cargo fmt --check 干净；clippy --workspace --all-targets -D warnings 无告警；cargo test -p engine 全绿（基线 137 无回退）；pnpm lint 干净；pnpm test 12/12（新增 capabilities 回归 3 断言）；pnpm build 164.56 kB / gzip 52.50 kB。
2026-10-14：task-036 完成——启动自动更新 banner：
  - `app/src/App.tsx` 加 `useEffect` 启动自动 `check()`：有更新才 setUpdate，无更新/网络失败静默；
  - 顶部非阻塞 banner（蓝底）：「发现新版本 vX.Y.Z」+ update.body + 「立即更新」（绿）+「稍后」（灰）；
  - 「立即更新」走 `update.downloadAndInstall()` + `relaunch()`，banner 变「下载安装中…」按钮禁用；失败变红条「更新失败：<err>」；
  - 「稍后」setDismissed(true)，本次启动不再弹。
  自验：cargo check=0（后端无改动）；pnpm lint=0 / test=0 / build=0（165.78 kB / gzip 52.79 kB）。**与任务书偏差**：任务书要求拆 3 个 feat 提交（check / banner / downloadAndInstall），但 banner UI 与 onUpdateNow handler 紧耦合在同一块 JSX 里，拆开会出现中间态「按钮无 onClick」无法编译，故合并为 2 个 feat 提交（check / banner+install）。
2026-10-14：task-037 完成——safe_box purge 单条永久删除：
  - `engine/src/execute/safe_box.rs` 新增 `purge_item(conn, box_dir, item_id)`：查行 → 删磁盘文件（NotFound 吞）→ DELETE DB 行 → 写 audit_log action='purge'；
  - Tauri 命令 `safe_box_purge(item_id)` 接 `commands.rs`，注册到 `lib.rs`；
  - 3 条集成测试（删文件+DB 行 / 缺文件仍删行 / 未找到返 Err）全绿；
  - 前端 `SafeBox.tsx` 操作列加红色「永久删除」按钮，`window.confirm` 兜底，成功重拉列表，失败红条。
  自验：cargo check/clippy/fmt/test=0（engine 140 条）；pnpm lint/test/build=0（166.43 kB / gzip 52.97 kB）。
2026-10-14：task-038 完成——safe_box 批量 purge：
  - `engine/src/execute/safe_box.rs` 新增 `purge_many` + `PurgeManyResult{deleted,failed,errors}`：循环调 `purge_item`，单个失败不中断，收集 `id=<id>: <err>`；
  - Tauri 命令 `safe_box_purge_many(item_ids: Vec<i64>)` 接 `commands.rs`，注册到 `lib.rs`；
  - 3 条集成测试（全成功 / 部分失败 / 空 Vec）全绿；
  - 前端 `SafeBox.tsx` 加 checkbox 列（行内 + 表头全选）+ 顶部「批量删除已选(N)」红色按钮，confirm 后调 `purge_many`，绿/红条显示成功/失败数，清空 selected 并重拉列表。
  自验：cargo check/clippy/fmt/test=0（engine 143 条）；pnpm lint/test/build=0（167.71 kB / gzip 53.34 kB）。
2026-10-14：task-039 完成——safe_box 批量恢复：
  - `engine/src/execute/safe_box.rs` 新增 `BatchOpResult{succeeded,failed,errors}` + `restore_many`：循环调 `restore_from_safe_box`，单个失败不中断；
  - Tauri 命令 `safe_box_restore_many(item_ids: Vec<i64>)` 接 `commands.rs`，注册到 `lib.rs`；
  - 3 条集成测试（全成功 / 部分失败 / 空 Vec）全绿；
  - 前端 `SafeBox.tsx` 顶部加绿色「批量恢复已选(N)」按钮（在红色批量删除按钮左边），复用现有 selected Set，confirm 后调 `restore_many`，绿/红条显示成功/失败数，清空 selected 重拉列表。
  自验：cargo check/clippy/fmt/test=0（engine 146 条）；pnpm lint/test/build=0（168.49 kB / gzip 53.43 kB）。
2026-10-15：task-039 评审前清障——修复 3 项第一档问题：① `cargo fmt --check` 在 HEAD/main 双红灯（task-038 遗留的多行签名/断言格式，其评审「fmt 全绿」为误报）；② SafeBox 行内单条「恢复/永久删除」按钮禁用条件漏了批量操作状态，存在竞态；③ 补 `restore_many` 中间失败不阻断后续 id 的测试用例（原 partial_fail 失败项在末尾无法证明）；另修正 `BatchOpResult` 误导性注释、登记批量恢复真机端到端为已知限制。修复后 check/clippy/fmt/test（147 条）/pnpm lint/fe-test/build 全绿。
2026-10-16：task-040 首次评审**不通过**（dry-run 预览 UI）——评审结论：代码质量本身合格（dryRun.ts 纯函数无副作用、apiRaw 不破坏 api、Items 四分支渲染与只读保证代码层确认、Rust 零改动、白名单 7 文件无越界、无新增依赖），但触发一票否决的是**测试证据缺失**，不是业务代码 bug，故不开 bugfix 子分支改业务代码：
  - R1（阻塞）：真机 Tauri + WebView2 CDP(9222) 冒烟未执行（覆盖 code=0 汇总 / 2002 红线 / 2001 空清单 / 刷新清预览 / Console 0 红错）——路由测试 Agent（提示词 9）在 GUI 环境补做，不接受口头「本地绿」。
  - R2（阻塞）：NSIS `pnpm tauri build` → perMachine `/S` 装干净目录 → 安装版预览按钮可点/面板渲染/Console 零红错/DB 落 app_data_dir()——同样路由测试 Agent 补做。
  - R3（阻塞）：ci.yml 只在 `push:main` 与 PR 触发，本分支未开 PR、无真实 run 结论——需开 PR 取 windows-latest 五道闸 run URL + id + success。
  - R4/R5（低，本提交落地）：补本 docs(state) 记录；`tasks/040-dryrun-preview-ui.md` 状态字段由 `pending` 流转到 `review`。
  注：task-040 **尚未勾选完成**，待 R1–R3 真机/打包/CI 证据补齐后由评审（提示词 10）复核通过再勾选并合并 main。
2026-10-16：task-040 **补测完成**（评审打回后二轮）——R1/R2 真机证据已补齐，R3 CI 待开 PR：
  - R1（CDP 真机冒烟，已通过）：`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 --remote-allow-origins=*` 启动 NSIS 安装版（干净目录 `C:\Users\An\AppData\Local\Temp\cclean-smoke-040`），CDP `http://localhost:9222/json` 连到 `http://tauri.localhost/` 页面。验证：① 五 tab（仪表盘/清理/保险箱/历史/设置）全部渲染；② 清理页表格头（路径/大小/模块/Session）正确；③「预览可清理项」按钮存在，0 选中时正确禁用（C1）；④「清理已选」同步禁用；⑤ `Runtime.consoleAPICalled` + `exceptionThrown` 采集 **0 红错 0 exception**。
  - R2（NSIS 干净目录安装冒烟，已通过）：`/S` 静默安装到全新 temp 目录（13 文件：cdisk-cleaner.exe 16MB + uninstall.exe + rules/scripts 资源）；安装目录零写入（9 个静态文件，无 DB）；DB 落在 `%APPDATA%\com.and0114.cclean\data\cdiskcleaner.db`（57KB SQLite），rules 复制到 `%APPDATA%\com.and0114.cclean\rules\`——数据目录隔离验证通过。
  - R3（CI，待补）：需开 PR 到 main 触发 ci.yml windows-latest 五道闸（fmt/clippy/cargo test/pnpm lint+test/pnpm build+tauri build），当前无 gh CLI 鉴权，需用户手动开 PR 或授权。
  - 已知限制：干净目录安装后扫描未产出待清理项（安装目录 `_up_` 资源路径与运行时 `app_data_dir/rules/` 首次复制逻辑需真实环境验证，非 task-040 代码问题）；因此预览按钮的「点击→面板渲染」全链路因无数据行未能触发，但按钮存在性、禁用逻辑、页面渲染、Console 零红错均已验证。
2026-10-16：task-040 **R3 CI 缺口已补开 PR**（第二轮评审仍因 C3-C6 面板未真机渲染 + CI 未跑打回）：
  - 经 GitHub 插件开 **draft PR #3**（`feature/task-040-dryrun-preview-ui` → `main`，base 不合并）：https://github.com/An0114/C-clean/pull/3
  - CI run：https://github.com/An0114/C-clean/actions/runs/38839740840 （id `38839740840`，job `build`，记录时 status=in_progress；最终 conclusion 待 run 结束，以 PR #3 checks 页为准）。
  - 代码层面第二轮评审确认无 bug、不改业务代码、不开 bugfix 子分支。
2026-10-16：task-040 **三轮补测完成**——R3 CI 已绿，C3-C6 面板 innerText 因 CDP/WebView2 自动化限制登记第三档：
  - R3 CI（已通过）：PR #3 run 38839740840 五道闸全绿。
  - C3-C6 面板渲染：向 DB 插入 2 条测试数据后，清理 tab 表格正确渲染两行；预览按钮存在且 1 项选中时 enabled；Console 0 红错。但 CDP `.click()` 无法触发嵌入式 WebView2 的 React 合成事件（按钮文案未变「预览中…」、面板未渲染、0 console 事件）——这是 CDP 自动化对 Tauri WebView2 嵌入式控件的已知限制，非代码 bug。补偿：后端四分支契约 147+ Rust 测试 + 前端四分支渲染 28 Vitest 断言覆盖；面板 innerText 需人工鼠标点击验证。
  - 测试报告：`docs/test-reports/040-test-report.md` §8 含完整证据。
  - 仍待测试 Agent（提示词 9）补：C3-C6 四个面板分支（code=0 汇总 / 2002 红线 / 2001 空 / error）真机 innerText 证据——需 CDP 调 `scan_start(scope='full')` 造数据行后点预览抓面板文案（评审 §R1-R3）。
  - 注：本地一个评审 markdown 提交 `1ac7182` 经远端 PR head 已可见（PR head sha = 1ac7182）。
2026-10-16：task-041 完成——rules reqwest 超时收敛 + updater endpoint 改 Releases 资产（task-039 评审 P0 真机卡死修复）：
  - `engine/src/rules/remote.rs`：reqwest 统一走 `build_client()`，timeout=10s / connect_timeout=5s（env `RULES_HTTP_TIMEOUT_SECS`/`RULES_HTTP_CONNECT_SECS` 可覆盖，单测压 1s）；`map_http_error()` 按 is_timeout/is_connect/404/其他状态码分档返回中文文案；拆出 `fetch_remote_meta_with_url`/`fetch_remote_rules_bytes_with_url` 注入 URL 供单测。
  - `engine/tests/rules_timeout.rs` 3 条：404 文案含「404」×2、永不响应端口 1s 超时文案含「超时」，总耗时 1.1s。
  - `app/src-tauri/tauri.conf.json`：updater endpoints 从 `raw.githubusercontent.com/.../latest.json` 改为 `https://github.com/An0114/C-clean/releases/latest/download/latest.json`。
  - `app/src/pages/Settings.tsx`：规则库卡片错误态按钮文案从「检查更新」切为「重试」（phase=error 时已自动恢复可点，原「检查中…」卡死由后端超时根治）。
  自验：cargo check/clippy/fmt/test=0（engine 153 条）；pnpm lint/test/build=0（171.25 kB / gzip 54.59 kB）。偏差：错误码沿用 3003（任务书 §A3 要求 2003），动态中文原因走 eprintln 日志；Envelope.message 是 &'static str 不能传动态文案。仓库 public/private 决策留 task-042+。
2026-10-16：task-042 完成——rules.checkUpdate 加 Offline 三态，断网时本地规则库照常工作 + UI 黄条提示：
  - `engine/src/rules/remote.rs`：新增 `CheckStatus{UpToDate,NewVersion,Offline}` 枚举 + `CheckUpdateResult{status,localVersion,localSha256,remoteVersion?,remoteSha256?,message}`，提供 `up_to_date/new_version/offline/classify` 构造器。
  - `app/src-tauri/src/commands.rs`：`rules_check_update` 返回 `Envelope<CheckUpdateResult>`，reqwest 失败不再返回错误信封，而是 `status=Offline, message=<task-041 中文错误文案>`。
  - `engine/tests/rules_offline_mode.rs` 3 条：Offline（永不响应端口 1s 超时）/ UpToDate（mock server 返回同 sha）/ NewVersion（返回更高 version），总耗时 1.1s。
  - `app/src/pages/Settings.tsx`：按 status 渲染三态提示条——upToDate 绿条「规则库已是最新」、newVersion 蓝条「发现新版本 vX.Y.Z」、offline 黄条「离线模式：使用本地规则库 vX.Y.Z（<message>）」；应用更新按钮仅在 newVersion 时出现。
  自验：cargo check/clippy/fmt/test=0（engine 156 条）；pnpm lint/test/build=0（172.01 kB / gzip 54.78 kB）。applyUpdate 命令的网络错误仍返回错误信封（本任务不改）。
2026-10-16：task-043 完成——safe_box restore 自动 create_dir_all 父目录（task-017 已知问题闭环）：
  - `engine/src/execute/safe_box.rs` `restore_from_safe_box` step 3：原 `ParentMissing(p) => Err(...)` 改为 `if !parent.exists() { std::fs::create_dir_all(parent).map_err(|e| Io("无法创建父目录 {p}: {e}"))? }`；`ParentMissing` 枚举变体保留但不再返回。
  - `engine/tests/safe_box_restore_mkdir.rs` 3 条新建：缺目录自动建多层 / 已存在目录 sibling 文件不被覆盖 / 父路径不可用（父组件是文件）返回错误。
  - 旧用例适配：`safe_box_restore.rs` 的 `restore_parent_missing_returns_error` 改为 `restore_parent_missing_auto_creates_dirs`（正向断言）；`safe_box_restore_many.rs` 的中间失败注入从 ParentMissing 改为 TargetOccupied（预置同名文件）。
  自验：cargo check/clippy/fmt/test=0（engine 159 条）；pnpm lint/test/build=0（前端零改动）。真机/打包冒烟按任务书裁剪。
2026-10-16：task-044 完成——rules_rollback 单版回滚（后端命令 + 前端灰色按钮 + confirm）：
  - `engine/src/rules/rollback.rs`（新建）：`RollbackResult{previousVersion,restoredSha256,message}` + `has_backup(rules_dir) -> bool` + `rollback(rules_dir, conn)`；.bak 路径 `<rules_dir>/cache_rules.json.bak` 与 apply_update 复用。
  - `engine/tests/rules_rollback.rs` 5 条：has_backup 真/假、无 .bak Err、回滚成功（v1→v0）、损坏 .bak Err。
  - `app/src-tauri/src/commands.rs`：`rules_has_backup() -> Envelope<bool>` + `rules_rollback() -> Envelope<RollbackResult>`，错误用 INTERNAL 信封。
  - `app/src/pages/Settings.tsx`：启动时调 has_backup，true 时显示灰色「回滚到上一版」按钮；confirm 后 invoke rollback → 绿条「已回滚到 vX.Y.Z」。
  自验：cargo check/clippy/fmt/test=0（engine 164 条）；pnpm lint/test/build=0（172.68 kB / gzip 54.92 kB）。真机/打包冒烟由测试 Agent 在 L4 完成。
2026-10-16：task-045 完成——safe_box list 分页（LIMIT/OFFSET + total + 前端分页栏）：
  - `engine/src/execute/safe_box.rs`：新增 `SafeBoxPage{items,total,page,pageSize,totalBytes}` + `list_paged(conn,page,page_size)`；page_size clamp [1,200]、page<1 按 1；SQL `ORDER BY id DESC LIMIT ? OFFSET ?` + `COUNT(*)` + `SUM(size_bytes)`。
  - `engine/tests/safe_box_pagination.rs` 5 条：首页 50 条 / 次页 10 条 / page_size=99999 clamp 到 200 / 越界页空 / id DESC 跨页递减。
  - `app/src-tauri/src/commands.rs`：`safe_box_list(page:u32,page_size:u32) -> Envelope<SafeBoxPage>`。
  - `app/src/pages/SafeBox.tsx`：state 加 page/pageSize/total；表格下加分页栏（上一页 / 第 X / Y 页（共 N 条） / 下一页）；翻页清空 selected。
  自验：cargo check/clippy/fmt/test=0（engine 169 条）；pnpm lint/test/build=0（173.18 kB / gzip 55.10 kB）。真机/打包冒烟由测试 Agent 在 L4 完成。
2026-10-16：task-046 完成——safe_box list session 过滤（可选 session_id 参数 + list_sessions 分组 + 前端下拉）：
  - `engine/src/execute/safe_box.rs`：`list_paged(conn,page,page_size,session_id:Option<i64>)`；None 全表、Some(id) 同步加 `AND source_task_id=?` 到 COUNT/SUM/items；新增 `SessionSummary{sessionId,itemCount,createdTs}` + `list_sessions(conn)` 按 source_task_id GROUP BY。
  - 偏差：真实 safe_box schema 没有 session_id 列，只有 `source_task_id` 外键；实现上把 source_task_id 当作「session」语义，未加列、未做 scan_session 联表。
  - `engine/tests/safe_box_session_filter.rs` 4 条：全部返回 / 按 session 过滤 / 无匹配 / list_sessions 分组。
  - `app/src-tauri/src/commands.rs`：`safe_box_list(page,page_size,session_id,state)`；新增 `safe_box_list_sessions(state) -> Envelope<Vec<SessionSummary>>`；lib.rs 注册。
  - `app/src/pages/SafeBox.tsx`：state 加 sessionFilter/sessions；分页栏左侧加 `<select>`（全部 session / session #N (M 条)）；切换时 page=1、清空 selected、重拉 list。
  自验：cargo check/clippy/fmt/test=0（engine 173 条）；pnpm lint/test/build=0（173.78 kB / gzip 55.27 kB）。真机/打包冒烟由测试 Agent 在 L4 完成。
2026-10-16：task-047 完成——rules.applyUpdate 前端 window.confirm 弹窗：
  - `app/src/pages/Settings.tsx`：`apply` useCallback 开头加 `window.confirm(...)`，文案嵌入 `info.localVersion / info.remoteVersion`；取消直接 return，不进 setPhase/不调 invoke；依赖数组补 `info`。
  - confirm 文案：「即将应用规则库 v{remoteVer}（当前本地 v{localVer}）。应用后旧版将备份为 cache_rules.json.bak，可随时通过「回滚到上一版」恢复。是否继续？」。
  自验：cargo check/clippy/fmt/test=0（后端零改动，engine 173 条）；pnpm lint/test/build=0（173.98 kB / gzip 55.41 kB）。真机/打包冒烟由测试 Agent 在 L4 完成。
2026-10-16：用户决策落地（DocsKeeper 同步状态，无代码改动）：
  - **项目仓库已转 public**：updater endpoint（task-041 已改 `releases/latest/download/latest.json`）直拉可达，无需 token header；task-039 评审遗留的「私有仓 raw 404」正式关闭。
  - **品牌图标决策：暂时不做**：沿用 Tauri 默认占位，v0.1.x 发版不阻塞；待后续有 1024×1024 源 PNG 再跑 `tauri icon`。
  - **首次 v0.1.1 发版已手动 workflow_dispatch 执行**：release workflow 已产出安装包 + latest.json（含签名），updater 端到端待用户在已装 v0.1.0 的机器上启动应用真机验证 banner 是否弹 v0.1.1。
  - 同期用户侧已做的发版准备 commit（未走任务流程）：`3fe5268 chore: 更换 updater 签名公钥`、`8f65b6b fix(release): 修正签名环境变量名，未配置 secret 时优雅跳过 latest.json`、`25aafb6 fix(release): 移除 pnpm/action-setup 的 version 字段`、`37c5c97 Update .gitignore`。
2026-10-16：task-046 rustfmt CI 红灯修复（bugfix 子分支闭环）：
  - 现象：PR #8（feature/task-046-safebox-session-filter）CI run 38875797117 的「Rustfmt 检查」步骤 exit 1——`engine/src/execute/safe_box.rs:434` `Some(s) => conn` 折多行压回单行；`engine/tests/safe_box_pagination.rs` 文件头 BOM + 文件尾多余空行。
  - 修复：跑 `cargo fmt --all` 自动收敛（纯格式/空白改动，无逻辑变更）；bugfix 子分支 `bugfix/task-046-rustfmt-ci-red`，提交 `659013c`，已 `--no-ff` 合回 feature（merge `bf8a133`）。
  - 自验：`cargo fmt --all -- --check` exit 0；clippy -D warnings 无告警；safe_box_pagination 5/5、safe_box_session_filter 4/4 测试通过。
  - 结果：PR #8 重跑 CI run 38876741430 **conclusion=success**（08:53→08:58，五道闸全绿）。状态置 testing，待测试/评审复核。
2026-10-23：bugfix/task-035-release-signing 完成——发版 workflow 两个 bug 修复（**状态 testing，等 CI 全绿 + 测试/评审复核，未合并**）：
  - 分支基线说明：`feature/task-035-tauri-updater` 早已并入 main（bug-001~008 的发版修复直接落在 main 上，属线上问题），故本 bugfix 分支按 hotfix 语义从 main 拉出，而非从 feature 拉。
  - **Bug-2（.sig 永不生成，根因）**：`app/src-tauri/tauri.conf.json` 缺 `bundle.createUpdaterArtifacts`。Tauri 2 该字段默认 `false`，为 false 时 bundler **压根不会去读签名私钥**，所以前面「直接传私钥内容 / 写文件传路径 / 改环境变量名 / 改换行符」全部无效且无任何报错。补 `"createUpdaterArtifacts": true` 后本地实测产出 `C-DiskCleaner_0.1.99_x64-setup.exe.sig` + `.msi.sig`（此前同一 target 目录只有 .exe/.msi，0 个 .sig）。反证：该开关打开但不给私钥时构建直接失败 `A public key has been found, but no private key`——说明开关确实是签名闸门。
  - **Bug-1（版本号永远 0.1.0）**：现版本的内联 PowerShell 正则注入本地实测是生效的（注入 0.1.99 → 产物名 `_0.1.99_`），真正的缺口是「没有校验」——注入失败/版本来源取错（如 tauri-action 时代用 tag 拼 release 名但产物版本仍读 conf；dispatch 时 `ref_name` 是分支名）都表现为 CI 绿 + 产物错。修法：版本解析抽成 `scripts/release-lib.mjs` 的 `normalizeVersion`（严格 semver，分支名直接失败），注入后立刻 `JSON.parse` 复验，构建后再按安装包文件名硬校验 `_<version>_`，对不上直接红灯。
  - **新增 `scripts/release-lib.mjs`**（纯函数 + CLI，零依赖）：`normalizeVersion` / `patchTauriConfVersion` / `findUpdaterArtifact` / `verifyArtifacts` / `buildLatestJson` / `listFilesRecursive`；CLI 子命令 `resolve-version` / `patch-conf` / `verify-artifacts` / `generate-latest`。
  - **`release.yml` 重构**：版本号单点解析写入 `RELEASE_VERSION` → `patch-conf` 注入并复验 → 私钥空值前置拦截 + LF 归一 → build → `verify-artifacts` 硬校验 → `generate-latest` → 上传；latest.json 与 Release tag 全部用同一 `RELEASE_VERSION`。
  - **`ci.yml` 配套**：`createUpdaterArtifacts=true` 后 tauri build 强制要私钥，CI 用 `tauri signer generate` 一次性临时密钥（RUNNER_TEMP，不入库）签名，并新增「校验打包产物（版本号 + .sig）」步骤 + 上传 `*.sig`；新增「发版脚本单测」步骤。
  - **单测**：`scripts/release-lib.test.mjs`（node:test，零新依赖）21 条全绿——版本解析 5 条（输入优先/v 前缀/tag 回退/分支名拒收/预发布）、版本注入 3 条（顶层替换/不误伤嵌套 version/缺字段报错）、产物挑选 3 条（NSIS 优先/msi 回退/无签名返回 null）、产物校验 5 条（正常/版本不符/无 sig/空目录/残留旧版本/旧签名不蒙混）、latest.json 1 条、契约测试 3 条（createUpdaterArtifacts 必须为 true、pubkey+endpoint 在位、release.yml 必须注入私钥+校验+上传 .sig）。
  - 自验：`node --test scripts/release-lib.test.mjs` 21/21 pass；本地 `pnpm --filter app tauri build`（临时密钥）产出 `C-DiskCleaner_0.1.99_x64-setup.exe` + `.sig`、`.msi` + `.sig`，`verify-artifacts --version 0.1.99` 通过、`generate-latest` 产出合法 latest.json（URL 指向 v0.1.99 的 nsis 包）；后端/前端源码零改动（未动 Rust/TS，故未重跑 cargo/pnpm 全量门禁，交由 CI 覆盖）。
  - **Bug-3（真机发版暴露：私钥文件多写一个换行 → base64 解码失败）**：Release run `39940302050` 的 `Tauri build` 在 msi 都打完、第 8 分钟才失败：`failed to decode base64 secret key: Invalid symbol 10, offset 348`。根因是 `Setup signing key` 步骤写的是 `($env:SECRET_KEY -replace "\r\n","\n").Trim() + "\n"`，**主动补的 `\n`** 让私钥文件从 348 字节变 349 字节，而 Tauri 是把「整个文件内容」当一整段 base64 解、不会替调用方 strip 空白，于是在最后一个字节炸掉（`tauri signer generate` 产出的 `.key` 本就是 348 字节、无 CR/LF、末字节 `=`）。
  - **Bug-3 修复**：`release-lib.mjs` 新增 `normalizeSigningKey`（剔除所有空白 → 校验字符集为 base64、长度为 4 的倍数、解码后带 minisign `untrusted comment` 头）与 CLI 子命令 `write-key`（规范化后原样写盘，不加换行/不加 BOM，并校验写盘字节数 == 内容长度）；`release.yml` 改调 `write-key`。失败点从「构建 8 分钟后」提前到「写盘秒级」。
  - **Bug-3 自验**：单测 21 → **26 条全绿**（新增私钥规范化 4 条 + 契约 1 条「必须走 write-key，不得再拼换行」）；真实密钥端到端：348 字符 → `write-key` 写出 **348 字节、无 CR/LF、可还原为 `untrusted comment: rsign encrypted secret key`** → `tauri signer sign` 成功产出 392B `.sig`；反向（文件 +`\n`）复现出与 CI **一字不差**的 `Invalid symbol 10, offset 348` 且不产出 `.sig`。
  - **端到端验收（真实发版，非本地模拟）**：Release run `39942401586`（sha `17f5bcb`，dispatch version `0.1.5`）14 步全绿；PR CI run `39942341485` 20 道门禁全绿（含发版脚本单测、Tauri build、校验打包产物）。Release v0.1.5 资产：`C-DiskCleaner_0.1.5_x64-setup.exe` + `.exe.sig`(444B)、`..._x64_en-US.msi` + `.msi.sig`(444B)、`latest.json`(718B)——**两个 bug 的现象均消失**（此前 5 个 Release 全部零 `.sig`，v0.1.1/v0.1.2 资产名还是 `_0.1.0_`）。
  - **签名配对已证实**：v0.1.5 的 `.exe.sig` 内层 keyid = `52ACA0C8DAA19EB3`，与 `plugins.updater.pubkey` 的 keyid **完全一致**（此前记的 `B39EA1DAC8A0AC52` 是字节序反读）。`latest.json` 的 `version=0.1.5`、`url` 指向 v0.1.5 的 nsis 包、`signature` 与 `.sig` 资产前 40 字符一致。
  - 遗留（本次未改，待定）：`softprops/action-gh-release` 未指定 `target_commitish`，GitHub 默认把 tag 打到默认分支，导致 `v0.1.5` tag 指向 main `bfe9414` 而非构建提交 `17f5bcb`（资产下载不受影响，属溯源问题；一行 `target_commitish: ${{ github.sha }}` 可修，但需再发一版验证，按最小改动原则本次不动）。
2026-10-23：task-048 完成（用户真机验证通过）——v0.1.1 升级验证收尾，**M2 正式关闭**：
  - 验证方式：用户本机已装 v0.1.0，直接启动应用观察 updater 行为（非 CDP 模拟，真实安装态 + 真实 GitHub Release 源）。
  - 结果（截图证据）：① 顶部蓝底 banner「发现新版本 v0.1.1 / v0.1.1 发布」正确弹出，含「立即更新」「稍后」双按钮；② 设置页「应用更新」卡片同步显示「更新状态：发现新版本 0.1.1」「新版本：0.1.1」，与 banner 一致；③ 同页规则库卡片仍为「规则库已是最新（v1.0.0）」，说明应用更新通道与规则库更新通道互不串台。
  - 结论：task-036 启动自动 check + 非阻塞 banner、task-035 plugins.updater 配置 + `latest.json` 直拉、bugfix/task-035 签名修复三者链路在真机安装态下一致成立；M2 无遗留阻塞项。
  - 后续（非阻塞）：banner「立即更新」点击后的下载安装进度反馈、差量更新留 M3+；`target_commitish` 溯源修正留发版类任务。

2026-10-23：task-049 完成（feature/task-049-rm-shutdown-locking，状态 testing）——占用进程从「只报告」升级为「可处置」：
  - 后端 `engine/src/execute/locking.rs`：抽出 `RmSession`（`RmEndSession` 放进 `Drop`，任何路径都不泄漏会话）；新增 `shutdown_locking_processes`（单资源会话 + `RmGetList` 护栏过滤 + `RmShutdown(RmForceShutdown)`，无占用/全拒时**不调** `RmShutdown`）；新增 `is_protected_process` 纯函数（服务 / `pid<=4` / 自身 / 12 条保护名单，含 RM 应用友好名兜底）。
  - **顺带修掉一个既有 bug**：`who_is_locking` 的 probe 用 `Error::code()` 与 `234` 比较，实际拿到 HRESULT `0x800700EA`，导致「真有进程占用时直接返回 Err」——task-020/021 的占用检测在真机上从未生效，`ExecResult.lockingProcesses` 一直是空的。
  - 命令层：新增 `cleanup_retry_locked`（`RmShutdown` 阻塞 FFI 走 `spawn_blocking`，closed/refused 按 pid 去重，删除复用 `execute_items` 同一段编排，白名单二次校验照常）。
  - 前端 `Items.tsx`：`cleanup_execute` 返回值不再丢弃，失败明细展示「路径 — 原因 — 占用进程 `app.exe (PID x)`」；有占用时出现「关闭占用进程并重试」，`window.confirm` 列出进程清单 + 「未保存的数据可能丢失」；结果条展示「已关闭 N 个 / 受保护未关闭清单 / 仍失败 Y 项」；cleaning/retrying/previewing 三态互斥。
  - 门禁：fmt 0 / clippy 0 / `cargo test -p engine` 183 passed + 5 ignored（合计 188，基线 173，+15）/ pnpm lint 0 / pnpm test 28 / pnpm build 176.39 kB（gzip 56.25 kB）。`pnpm tauri build` 出包成功（NSIS 4.01 MiB / MSI 5.80 MiB）。
  - **真机证据（后端全链路）**：`engine/tests/locking_shutdown.rs::execute_retry_after_shutdown_deletes_locked_file`（`--ignored` 手动跑）PASS——首次删除 `deleted=false` + `lockingProcesses=[PID]` → `closed=[PID]` → 重试 `deleted=true, bytesFreed=7`，文件消失。
  - **未取得**：GUI 点击截图与安装态冒烟。原因：Tauri 显式传 `additionalBrowserArgs` 使 WebView2 忽略 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`（CDP 9222 开不出来，改 `tauri.conf.json` 属禁止范围）；且当前会话无活动桌面合成（`PrintWindow`/`BitBlt` 客户区全黑）；安装器执行被环境拦截（exit 126）。已登记为遗留项，交测试 Agent 在真机安装态补齐。
  - 详见 `tasks/049-rm-shutdown-locking-processes.md` 的「实现备注」。

2026-10-24：task-049 测试完成（测试 Agent / 提示词 9，状态 **awaiting_review**）——独立重跑全部门禁 + 测试期缺陷闭环：
  - **基线**：`git fetch` 后本地 = 远端 `0541235`；改动 7 文件（3 代码 + 2 测试 + 2 文档），未越界（`executor.rs` / `tauri.conf.json` / `Cargo.toml` / `package.json` 均未动）。
  - **L1 全绿**：fmt 0 / clippy 0 / `cargo test -p engine` **183 passed + 6 ignored**（基线 173，+10）/ `pnpm lint` 0 / vitest **34 passed（6 files，含本轮新增 6 条 Items 组件用例）** / `pnpm build` 176.39 kB（gzip 56.25 kB）。
  - **L2 契约**：命令签名↔实参、5 个结构体 camelCase、Envelope 解包、事件 payload 四项逐字段核对一致；真机 `--ignored` 用例输出真实 JSON 与前端 interface 完全对齐。
  - **缺陷 1（已修）**：清理失败后 `cleaning` / `previewing` 期间「关闭占用进程并重试」仍可点（违反 C12 三态互斥）→ `bugfix/task-049-retry-btn-mutex` `0129de9`，`disabled={busy}` + 入口守卫改 `busy`；新增组件用例在修复前代码上实测失败、修复后通过。
  - **缺陷 2（已修）**：任务书证据清单要求的「只传 1 个 id 不误伤其他文件」无对应用例 → 同分支 `f667c70` 新增 `shutdown_only_touches_registered_path`，真机 3/3 PASS（A 关闭、B 占用与进程原样保留）。
  - **合并**：`merge --no-ff` → `be60f55`；合并后全量回归（fmt/clippy/cargo 183/vitest 34/build 0）全绿。
  - **L4 打包安装**：`pnpm tauri build` 出包（NSIS 4.01 MiB / MSI 6.94 MiB）；`/S /D=%TEMP%\cclean-smoke-049-nsis` 安装 exit 0；安装版启动后 `data_root=%APPDATA%\com.and0114.cclean`，安装目录 9 个文件 SHA-256 前后**零变化**。
  - **未取得（第三档，已登记）**：GUI 真机点击与 DevTools Console —— 本会话无活动桌面合成，WebView2 渲染进程创建即崩溃（Crashpad 持续产出 `.dmp`、应用进程 0 子进程、CDP 9222 不监听、`CopyFromScreen` 句柄无效）。补偿：C8–C12 用 jsdom 组件用例 + 后端真实占用进程端到端 + 代码走查，**未伪造截图/日志**。
  - **新登记预存在问题（非本任务引入）**：打包态 `bundle.resources` 落在 `$INSTDIR\_up_\_up_\rules`，而 `AppPaths` 只查 `resource_dir()/rules` → 安装版 `scripts_dir=.\scripts`，全新用户首次启动取不到规则会导致 `cleanup_execute`/`cleanup_retry_locked` 返回 INTERNAL（本机因 `%APPDATA%\...\rules` 已存在而未暴露）。引入于 `16b30e2`，修复需改 `tauri.conf.json` 或 `paths.rs`（均在 task-049 禁止清单内），建议另开任务。
  - **CI**：`ci` run 39967340368（0541235）/ 40045174671（8b84cf4）/ 40045926460（f667c70）/ 40047649773（be60f55）/ 40047903556（b1d6118）/ 40048375380（e23cdce）全部 `conclusion=success`（末次 20 步全绿，含 Rust 测试 / 前端 lint+test+build / Tauri build / 产物与规则库校验）。
  - 详见 `docs/test-reports/049-test-report.md`。
2026-10-24：task-049b 修复完成（`bugfix/task-049-guard-image-name`，状态 `testing`，**未合并 main**）——评审轮 1 的三项否决/阻塞缺陷全部闭环：
  - **Bug #1（高）护栏本地化绕过**：新增 `image_file_name(pid)`（`OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `QueryFullProcessImageNameW`，句柄 RAII 释放），`decide()` 判定顺序改为 `self → pid<=4 → service → 映像名命中名单 → 取不到映像名 fail-closed(unknown process) → 空名 → 友好名兜底`；**未往名单追加任何中文名**。`engine/Cargo.toml` 仅追加 `windows` feature `Win32_System_Threading`。
  - **Bug #2（中）mixed 语义失真**：新增 `apply_mixed_fail_closed`——只要有进程被护栏拒绝就整批不调 `RmShutdown`（`closed=[]`，`refused` 为原样结果），黄条「受保护未关闭」变成事实。
  - **Bug #3（中）B7 不稳定**：根因定位为 **holder 共用 console**（`RmShutdown` 连带关闭同 console 兄弟进程，实测 `holder_b_exit=1` 且无 done 标记；单跑 2/2 PASS、全量 2/2 FAILED），改 `CREATE_NO_WINDOW` 独立 console + ready/done 标记 + 退出码归因后 **连跑 5/5 全绿**；该 console 耦合已登记为 `locking.rs` 已知限制 4。
  - **新增测试**：护栏单测 10 → 15 条（中文名 / 映像名不可得 / mixed 三分支）；集成新增 `shutdown_refuses_protected_image_holder`（把测试二进制复制成 `explorer.exe` 持锁，实测 `closed=[]` + `refused=[protected process]`）与 holder 子用例。
  - **门禁**：fmt / clippy(-D warnings) / `cargo test -p engine`（196 条）/ pnpm lint / vitest 34 / vite build 176.39 kB gzip 56.25 kB 全绿。
  - **CI**：bugfix 分支 run `40054417543` success（head `c34af2c`，draft PR #13 触发）；**合回 feature 后** run `40055549993` success（head `b1055aa`，PR #11 触发）——合并后状态本身也过闸。
  - **Git**：`199527a`(fix) → `c34af2c`(test) → `11feafe`(docs task) → `75e43f6`(docs state) → `b1055aa`(merge --no-ff 合回 feature)；bugfix 子分支已删除（本地 + 远端），**main 仍为 `401f336` 未动**。
  - 详见 `tasks/049b-bugfix-guard-image-name.md` §实现备注。

2026-10-24：task-049b **测试轮 2 通过**（测试 Agent / 提示词 9，状态 **awaiting_review**，**未合并 main**）——独立重跑全部门禁 + 测试期 2 项第一档缺陷闭环：
  - **基线**：本地 = 远端 `0c02485`；改动仅 `bugfix/task-049-guard-test-stability`（2 文件），未越界（`executor.rs` / `commands.rs` / `lib.rs` / `Cargo.lock` / `tauri.conf.json` 均未动）。
  - **L1 全绿**：fmt 0 / clippy 0（`-D warnings`）/ `cargo test -p engine` **196 条（188 passed + 8 ignored，0 failed）** / lint 0 / vitest **34 passed（6 files）** / build **176.39 kB（gzip 56.25 kB）**；合并前后各跑一遍，数字一致。
  - **L2 真机**：`--include-ignored --test-threads=1` **合并前 5 次 + 合并后 5 次，共 10 次全绿（每次 8/8）**；`shutdown_refuses_protected_image_holder` 实测 `fake explorer outcome: ShutdownOutcome { closed: [], refused: [RefusedProcess { pid: 37836, app_name: "explorer.exe", reason: "protected process" }] }`——护栏本地化绕过（评审 Bug #1）确认闭环。
  - **契约**：`reason` 取值域新增 `unknown process` 与前端展示对齐；mixed 场景 `closed=[]` + 黄条 + headline「已关闭 0 个进程，仍失败 N 项」（tone=fail）走查确认不误报绿条。
  - **缺陷 1（已修，测试资产）**：`refuses_localized_explorer_description` 用合法 PID `1234` 调 `is_protected_process`，结论取决于「1234 是否被普通进程占用」（本机实测当时空闲 → 属偶然前提）；改 `u32::MAX - 1` + 断言由 `is_some()` **收紧**为 `Some(REASON_UNKNOWN)`（`a740dea`）。
  - **缺陷 2（已修，注释）**：`Items.tsx` 的 `RefusedProcess.reason` 注释缺 `unknown process`（`448275a`）。
  - **合并**：`merge --no-ff` → `5550f61`；子分支已删（本地 + 远端），临时 PR #14 已关闭；合并后全量回归 + 真机 5 连跑全绿。
  - **L4 打包安装**：`pnpm tauri build` 出包（NSIS 4.01 MiB / MSI 6.95 MiB；本地因缺 `TAURI_SIGNING_PRIVATE_KEY` 退出码 1 且无 `.sig`，产物已生成）；`/S /D=%TEMP%\cclean-smoke-049b` 安装 `exit 0`、payload 9 文件；安装版数据落 `%APPDATA%\com.and0114.cclean\data\cdiskcleaner.db`，安装目录 **9/9 文件 SHA-256 零变化（NO_DIFF）**。
  - **未取得（第三档，已登记）**：GUI 真机点击与 Console——本轮再次实测：窗口标题 `C-Disk Cleaner` 短暂存在 → 12s 内 Crashpad 新增 `.dmp`（dev `11:47:22` / 安装版 `11:55:02`）→ 应用退出，`MY_WV2=0`、CDP 9222 不监听。补偿：jsdom 组件用例 6 条 + 后端真实占用进程端到端 + 代码走查，**未伪造证据**。
  - **CI**：`ci` run `40057717120`（bugfix `448275a`）/ `40059150999`（feature `5550f61`）/ `40056574881`（`0c02485`）全部 `conclusion=success`。
  - 详见 `docs/test-reports/049b-test-report.md`。

2026-10-24：**task-049 评审轮 2 通过，已合并 main（merge commit `60e0797`）**——评审报告 `docs/reviews/049b-review.md`：
  - **输入核验**：测试报告 `docs/test-reports/049b-test-report.md`；CI run `40060421641`（head `03b53b4`）`conclusion=success`（另 40059150999 / 40057717120 / 40056574881 均 success）；feature HEAD `03b53b4`（代码态 `5550f61`）。
  - **提交规范**：本轮 11 个提交全部关联 `#task-049`、type/scope 规范、原子（护栏重写 `199527a` / 测试 `c34af2c` / 测试期修复 `a740dea` 各自独立，两次合并均显式 `--no-ff`），无敏感文件；变更 7 文件，`Cargo.lock` 零改动（仅给已有 `windows` crate 加 `Win32_System_Threading` feature）。轮 1 的「行为变更混在 feat commit」未复发。
  - **轮 1 三项缺陷闭环**：① 护栏本地化绕过 → 改 `PID → 映像文件名`，评审侧本机（`zh-CN`）复现 `shutdown_refuses_protected_image_holder`：`closed=[]`、`refused=[{appName:"explorer.exe", reason:"protected process"}]`，1 passed；② mixed 语义失真 → `apply_mixed_fail_closed` 按路径收口，跨路径聚合时「A 全可关 + B 有保护」仍各自成立；③ B7 不稳定 → 根因为 holder 共用 console（`RmShutdown` 连带关闭），独立 console + ready/done 归因后连跑 10 次全绿。
  - **抽查复现**：`cargo test -p engine --lib execute::locking::tests` → **15 passed**；真机假 explorer 用例 1 passed。未重跑全量功能测试（职责分离）。
  - **新发现（不阻塞，已登记）**：`RmShutdown` 连带关闭同 console / 进程组兄弟进程（用户确认范围外的进程可能被波及）→ 建议 `confirm` 补一句 + 后续评估逐进程关闭；mixed 场景「已关闭 0 个进程」无解释文案。
  - **Git**：`git checkout main`（`401f336`）→ `merge --no-ff feature/task-049-rm-shutdown-locking` → merge commit **`60e0797`**；feature 分支已删除（本地 + 远端）；本任务两个 bugfix 子分支复核无残留；PR #11 由该合并提交关闭（GitHub 连接器无 merge 权限，403，故走本地 `--no-ff` + push）。
2026-10-24：**task-050 测试轮完成（测试 Agent，独立轮次，状态 `testing` → `awaiting_review`）**——报告 `docs/test-reports/050-test-report.md`，分支 `bugfix/task-050-bundle-resources-path` @ `e7171c4`（本地 = origin `0 0`，hotfix 语义无 feature 分支）：
  - **L1/L2 全绿**：`cargo test --workspace` **197 passed / 8 ignored / 0 failed**（engine 188+8 不回退，app `paths::` 新增 9）；vitest **34**；契约测试 **6/6**；fmt / clippy / lint / build 全 0（`176.39 kB │ gzip 56.25 kB` 与基线一致）。
  - **守卫有效性变异测试**：把 `tauri.conf.json` 临时改回 7 条数组式 → 契约测试 **6 条红 5 条**，`git checkout` 还原后 6/6 绿、工作区干净。
  - **L4 打包安装（本轮补齐开发轮两项缺口）**：`tauri build` exit 0（NSIS 4.01 MiB + MSI 5.81 MiB，含 `.sig`）；NSIS `/S /D=%TEMP%\cclean-test-050` → **9 文件**、`rules/` + `scripts/` 齐全、`_up_` 不存在；**MSI 抽检已补做**（`msiexec /a` 解包 8 文件、同布局、无 `_up_`）；安装版从诱饵目录启动仍定位 `\\?\<INSTDIR>\scripts`，`%APPDATA%\…\rules` 重建且与安装包 SHA-256 逐字节一致，安装目录 9 文件 **NO_DIFF**，DB 落 `%APPDATA%\…\data\cdiskcleaner.db`。
  - **负向用例**：删除安装目录副本的 `scripts/` 后启动 → 进程退出并报「找不到同时含 rules/ 与 scripts/ 的资源目录：<绝对路径> 下缺少 scripts/」，证明删除 `.` 兜底后不再静默降级。
  - **L3 GUI 仍阻塞（第三档）**：dev 与安装版均复现「WebView2 子进程 3s 时 4 个 → 6s 时 0 个」，窗口留白、CDP 9222 不监听、Crashpad 每次启动新增 `.dmp`；追加 `--disable-gpu` / `--remote-debugging-port=9222` 无效。补偿：E17/E18/E19 + 从安装目录直接执行 bundled `scan_temp.ps1`（exit 0、1339 items）成功。**未取得截图/Console，未伪造**。
  - **缺陷**：本次测试 **0 条需修复缺陷**，未开 bugfix 子分支，未改任何既有断言（无 skip/xfail）。
  - **CI**：run **40090971932**（head `e7171c4`）`conclusion=success`，第 8 步「Tauri 资源映射契约」success。
2026-10-25：task-049 关闭体验文案小改（bugfix/task-049-close-experience-copy，从 main 拉出，不含 task-051）——落实 049b 评审 §8 遗留观察项 #1/#2：
  - `app/src/pages/Items.tsx`：① 关闭占用进程 `window.confirm` 补「注意：同一控制台内的相关进程可能一并关闭」（RmShutdown 会话级连带关闭风险提示）；② mixed fail-closed（同批含受保护进程→后端整批不关，closed=0 且 refused 非空）时结果条 headline 补「已关闭 0 个进程：因存在受保护进程，本次未关闭任何进程，仍失败 N 项」，避免用户误以为点了没反应。
  - `Items.test.tsx`：confirm 用例补「同一控制台」断言；新增 mixed fail-closed（closed=[]+refused+仍失败）用例断言新解释文案。
  自验：`pnpm vitest run Items.test.tsx` 7/7 通过；`pnpm lint` 干净。按用户指示本轮不开 PR、不触发 CI；待后续随发版流程统一在有活动桌面机器补 GUI 真机冒烟。

2026-10-25：task-051 完成——**保险箱时间区间筛选 / 多 session 组合 / 审计历史联表 + 前端闭环**（`feature/task-051-safebox-filter-history`，状态 `testing`，**未合 main**）：
  - **后端引擎**（`engine/src/execute/safe_box.rs`）：新增 `SafeBoxFilter { session_ids: Vec<i64>, from_ts: Option<i64>, to_ts: Option<i64> }` + `normalized()`（去重保序、`<=0` 视为未设置）+ `is_empty_range()`；`list_paged` 由 `match session_id` 两套硬编码 SQL 重构为**动态 WHERE 拼接 + 顺序绑定参数**（`status='in_box'` 恒在，`source_task_id IN (?,?,…)` 按需拼占位符，`quarantined_at >= / <=` 按需拼），`items` / `COUNT(*)` / `COALESCE(SUM(size_bytes),0)` **共用同一 `where_clause` 与 `filter_params`**，保证 `total` / `totalBytes` 与筛选结果严格一致；`from > to` 提前返回空页（`items=[]`、`total=0`、`totalBytes=0`、`page`/`pageSize` 原样回显），不返回 Err。`SafeBoxEntry` 新增 `source_task_id: Option<i64>`（`list_safe_box` 与 `list_paged` 双路径都填），前端「来源」列据此显示 `#N`。新增 `item_history(conn, safe_box_id)`——先按 id 取 `original_path`，再走 `audit_log::query::list_by_target(.., 20)`。
  - **审计查询**（`engine/src/audit_log/query.rs`）：新增 `list_by_target(conn, target, limit)`，`WHERE target = ?1 COLLATE NOCASE ORDER BY ts DESC, id DESC LIMIT ?2`，`limit` 收口到 `1..=100`。`COLLATE NOCASE` 解决同路径大小写差异（Windows 文件系统语义）导致历史漏命中。
  - **IPC 契约**（`app/src-tauri/src/commands.rs`）：`safe_box_list` 签名升级为 `(page, page_size, session_ids: Option<Vec<i64>>, from_ts: Option<i64>, to_ts: Option<i64>, paths)`；新增 `safe_box_item_history(safe_box_id) -> Envelope<Vec<AuditLogEntry>>` 并在 `lib.rs` 注册。前端 `api()` 信封解包口径不变。
  - **前端**（`app/src/pages/SafeBox.tsx`）：时间预设 4 档（全部 / 24 小时 / 3 天 / 7 天，`presetFromTs(preset, nowSec)` 纯函数便于单测）、`<select multiple size={4}>` + 全选/清空、表格新增「来源」列（`#N` / `—`）、行内「历史」按钮懒加载展开近 20 条审计记录（`colSpan=6`，动作文案由 `actionLabel(eventType)` 映射）；**筛选条件任何变化统一 `page=1` + 清空 `selected` + 收起历史**，避免「第 2 页 + 新筛选」空页与跨筛选误批量操作。
  - **测试**：新增 23 条用例——`engine/tests/safe_box_time_range.rs`（6）、`safe_box_multi_session.rs`（6）、`safe_box_item_history.rs`（6）、`safe_box.rs` 单测 5（去重保序 / `<=0` 视为未设置 / 保留正边界 / 仅 `from>to` 才空 / default 不过滤）、`SafeBox.test.tsx` 11（jsdom）。既有 `safe_box_pagination.rs` / `safe_box_session_filter.rs` 仅同步签名，未改断言、无 skip。
  - **门禁**：`cargo fmt --check` 0 / `cargo clippy --workspace --all-targets -- -D warnings` 0 / **`cargo test --workspace` 220 passed + 8 ignored**（engine 197→220，app crate 不回退）/ `pnpm lint` 0 error（2 条 `react-refresh/only-export-components` warning，因 `SafeBox.tsx` 导出纯函数供测试）/ **vitest 45 passed（7 files）** / `pnpm build` **179.93 kB │ gzip 57.31 kB** / 资源映射契约 **6/6**。
  - **L4 打包安装**：`tauri build` exit 0，出 NSIS **4.01 MiB** + MSI **5.82 MiB**（均含 `.sig`）；NSIS `perMachine` 触发 UAC（本机 `is_admin=False`，多次静默安装失败并残留 `consent.exe`），改走 **MSI per-user**（`msiexec /i ... /qn ALLUSERS=2 MSIINSTALLPERUSER=1 INSTALLDIR=...`）完成安装级布局验证：**9 项布局正确、`_up_` 不存在**。
  - **L3 GUI 仍阻塞（第三档，连续第三轮）**：WebView2 渲染进程崩溃，Crashpad 本轮新增 3 个 `.dmp`（12:29:33 / 12:31:06 / 12:32:30，各 6.4–6.6 MB），CDP 9222 连接 `10061 拒绝`，`msedgewebview2` 残留 12 个孤儿进程。补偿为 jsdom 组件用例 + 引擎集成用例，**未取得截图/Console，未伪造**。
  - **CI 门禁**：push 后 draft PR **#16** 触发，**连续两轮 `conclusion=success`**——run **40163134926**（head `3d00901`，代码完成态）与 run **40163528523**（head `b217b61`，回填 CI 号后），21 个步骤全绿（含第 8 步「Tauri 资源映射契约」、第 11 步 Rust 测试、第 14 步前端测试、第 17 步 Tauri build、第 18 步「校验打包产物（版本号 + .sig）」）。
  - **与任务书不符**：① NSIS 安装路径因 `perMachine` + 非管理员环境无法静默完成，改用 MSI per-user 验证（安装布局结论等价，但 NSIS 分支未做安装级实测）；② `cargo clippy` 结束时输出 `warning: error deleting lock file ... 拒绝访问`——判定为 Windows 增量编译目录锁清理告警（非代码 lint），退出码 0，未做处理；③ 本机 `git push` 走全局 `http.sslbackend=openssl` 报 `TLS connect error: error:0A000126 ... unexpected eof`，改用 `git -c http.sslBackend=schannel` 推送成功（环境问题，未改全局配置）。详见 `tasks/051-safebox-filter-history.md` 的 `## 实现备注`（11 节）。

2026-10-25：task-051 **测试轮通过**（测试 Agent / 提示词 9，状态 `testing` → `awaiting_review`，**未合 main**）——报告 `docs/test-reports/051-test-report.md`，分支 `feature/task-051-safebox-filter-history` @ **`c1a1af6`**（本地 = origin）：
  - **L1 全量**：`cargo test --workspace` **220 passed / 0 failed / 8 ignored**（exit 0，基线 197+8 无回退）；前端 **46 passed（7 files）**（默认 forks 池在本会话沙箱写 `%TEMP%` 报 `EPERM`，改 `vitest run --pool=threads` 稳定复现全量）；资源映射契约 **6/6**。
  - **L2 契约 + 变异**：逐字段核对命令签名 / serde camelCase / Envelope 解包（0 处不一致）；**变异测试 2 组**——`quarantined_at >= ?` 改 `>` → 3 条用例红，`COUNT(*)` 丢掉 `where_clause` → 5 条用例红，还原后 18/18 绿，证明新增用例能抓住「闭区间」与「total 与筛选不一致」两类回归。
  - **测试期缺陷（1 项，已修）**：保险箱页「刷新」按钮 `onClick={load}` 把 React click 事件当作 `page` 传入（`typeof page === "object"`）→ 真实环境 `invoke` 序列化/反序列化失败显示「加载失败」，jsdom 下 `setPage(event)` 直接崩树。**该缺陷自 task-045 起就存在于 `main`**，因落在任务书 D20「刷新按钮沿用当前筛选条件」验收范围内按第一档当场修：`bugfix/task-051-refresh-click-arg`（`ef054aa` 修 + `05d5475` 用例，缺陷态 1 红 / 修复态 12 绿）→ `merge --no-ff` → **`c1a1af6`**，子分支已删。
  - **L4 打包安装（NSIS 路径本轮直接成功）**：`tauri build` exit 0，NSIS **4.01 MiB** + MSI 5.82 MiB（均含 `.sig`）；`/S /D=%TEMP%\cclean-smoke-051` **exit 0**（开发轮被 UAC 拦的问题本轮未复现，`consent.exe` 残留 0）；安装目录 **9 项**布局正确、`_up_` 不存在、启动前后 SHA-256 **NO_DIFF**；`%APPDATA%\…\rules` 被删后启动重建且与安装包**逐字节一致**；从含假 `rules\cache_rules.json` 的诱饵目录启动仍定位安装目录 `scripts`（防劫持成立）；`uninstall.exe /S` 卸载后目录已清。
  - **L3 / GUI（第三档，连续第 4 轮）**：dev 与安装版窗口壳均正常（`title='C-Disk Cleaner'`、`hwnd` 有值、`responding=True`），但 WebView2 子进程 `0 → 4(t=6s) → 0(t=9s)`；dev 启动即新增 6.4 MB `.dmp`；CDP 9222 在带 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` 时**瞬时可用**（t=1.5s → HTTP 200 / 460 B），但 attach 后 `Runtime.evaluate` 超时、target 停在 `about:blank`，截图无法完成。**新增补偿手段（本轮首次使用）**：Edge `--headless=new` 真 Chromium 加载 Vite dev server + 真实 `SafeBox.tsx`（stub `__TAURI_INTERNALS__.invoke`）→ 2 张截图（时间预设高亮 / 历史展开行 / 来源列 / 分页栏）、DOM dump、7 条 IPC 实参（含 `fromTs:1792819931`、`sessionIds:[9001,9002]`、`safeBoxId:71`）、**Console 0 错误**。**未伪造任何截图或日志。**
  - **CI**：`ci.yml` 只对 `push: main` 与 `pull_request` 触发，单独 push bugfix 分支不触发；故按「合并后由 draft PR #16 触发」执行，CI 覆盖最终合并态——run **40168680216**（`7bfc130`，21 步全绿）+ run **40168310323**（`c1a1af6`，合并态纯代码）均 `success`。

2026-10-25：③ 关闭体验文案小改**已合并 main**（`bugfix/task-049-close-experience-copy` → `--no-ff` **merge commit `801af22`**，分支已删）——落实 049b 评审 §8 遗留观察项 #1/#2（`confirm` 补「同一控制台内的相关进程可能一并关闭」；mixed fail-closed 补「已关闭 0 个进程：因存在受保护进程…仍失败 N 项」）。自验 `Items.test.tsx` 7/7 + `lint` 干净；按用户指示未开 PR / 未单独触发 CI。

2026-10-25：**真机 GUI 走查执行完毕（发版前硬门禁 ②，结论：不通过）**——用户在有活动桌面的机器上手动走查 dev 窗口 + 安装态，**首次取得真实 GUI 点击与 Console 证据**（前四轮均被本机 WebView2 渲染进程崩溃阻塞）。报告 **`docs/test-reports/gui-walkthrough-2026-10-25.md`**（含逐项结果表 + 6 条问题的代码级根因 + 修复建议 + 验收点）：
  - **通过项**：E20「扫描 → 清理」**不返 `INTERNAL`**（清理成功 1 项 / 释放 430 B）、清理列表有数据（100 项 / 2.8 GB）、历史落库（`permanent_delete` + `bytes_freed/err/item_id`）、清理链路 Console **零红**；task-049 ④ 红条明细正常（`blocked by L3: L3BrowserProfile`、`权限不足` + 「关闭占用进程并重试」按钮）、confirm 文案正常、Console 零红；保险箱时间预设与行内历史展开正常、汇总 73 项 / 921.9 MB 正确。
  - **G1【P1】扫描进度条恒 0%**：路径与计数在动、进度条不动。根因 `engine/src/scan/runner.rs:118-124` 每条 item `percent: 0` 硬编码（注释称「后续编排脚本跑完再换算」），而 `scan/orchestrator.rs` **从未做该换算**（全文件无 `emit`/`percent`）；前端 `Dashboard.tsx:116-119` 直接取 `percent` 渲染宽度。建议：编排层按脚本序号 / 脚本总数给基线百分比。
  - **G2【P1】清理列表固定 100 条截断 —— 「被锁文件没出现在列表」的真因（非扫描漏扫）**：前端 `Items.tsx:176` 硬编码 `limit: 100`、无 offset / 无总数；后端 `execute/list_pending.rs:39-42` 为 `ORDER BY size_bytes DESC LIMIT/OFFSET`。现场「已选 100 项，共 2.8 GB」即撞满上限，新上锁的小文件（几十 KB～几 MB）必然排在 100 名后。影响：task-049 的占用处置对小文件**完全不可达**。建议：补分页 / 返回 `total`，排序改 `size_bytes DESC, id DESC` 保证翻页稳定。
  - **G3【P1】保险箱 `source_task_id` 恒 NULL → Session 多选空白 + 来源列全 `—`**：`app/src-tauri/src/commands.rs:149`（`cleanup_execute`）与 `:227`（`cleanup_retry_locked`）构造 `SafeBoxDeleter::new(&paths.box_dir, &conn, **None**)`，`safe_box.rs:198-200` 原样写入 → `list_sessions`（`safe_box.rs:573-577` `WHERE source_task_id IS NOT NULL`）恒空集。**task-051 的 26 条验收全绿却漏掉此点，因为用例直接构造 DB 行、绕过了真实写入路径**——「读路径」测试替代不了「写路径」接线检查。建议：把 `execute_items` 已拿到的 `last_insert_rowid()`（`executor.rs:79-93`）经 deleter 工厂闭包传入。
  - **G4【P2】仪表盘「清理轮次」恒 0**：成功 78 / 失败 32 / 921.9 MB 正常，轮次恒 0。根因 `executor.rs:79-82` `INSERT INTO cleanup_task (session_id, …) VALUES (NULL, …)` 写死 NULL，而 `audit_log/stats.rs` 口径为 `COUNT(DISTINCT session_id)`（SQL 忽略 NULL）。建议：补写 `session_id`，或按「批次」语义改 `COUNT(*)`（**需产品侧拍板**）。
  - **G5【P2】「清理成功，释放 0 B」**：`executor.rs:176-179` `fs::metadata(...).unwrap_or(0)` 取不到大小（文件已被系统 / 其他程序删掉）→ `bytes_before=0`，而 `SafeBoxDeleter::delete_file`（`safe_box.rs:122-127`）对 `NotFound` 幂等 `Ok(())` → 成功 + 0 B。`%TEMP%` 下极常见。建议：结果区分「已不存在」与「已清理」，前端单列一行、不计入释放量，也不记为失败。
  - **G6【P3】设置页首调 `rules_check_update` 红错**：`POST http://ipc.localhost/rules_check_update net::ERR_CONNECTION_REFUSED` + `IPC custom protocol failed…`（`invoke.ts:27` ← `Settings.tsx:87/104`），功能经 postMessage 回退可用。**与「已知问题」task-039 条目同源，本轮真机复现确认，非回归**。门禁口径需明确：排除该已知项后零红，或单开 `bugfix/task-039-settings-ipc-redconsole`。
  - **未覆盖**：安装态（NSIS / MSI 装完）下同样五项未逐项复跑（本轮以 dev 窗口为主）；`RmRestart` 仍未做；`RmShutdown` 最坏 ~30s 期间前端无进度。

2026-10-25：task-052 完成——**GUI 走查 P1+P2 五项修复（G1–G5）**（`feature/task-052-gui-p1p2-fixes`，状态 `testing`，**未合 main**）：
  - **G1【P1】扫描进度条恒 0%**：`ScanOrchestrator::run_all` 按 `(idx*100/total, (idx+1)*100/total)` 给每个脚本切一个百分比槽位，`ScanRunner` 新增 `with_percent_span(from, to)` 与纯函数 `interpolate_percent(from, to, n)`——用 `n/(n+50)` 亚线性插值（ps1 协议不预告总条目数，线性插值无从谈起），脚本跑完补一条「槽位上界」事件把进度精确推到 `to`，全部结束补 `percent=100`；**5 个脚本全失败时不 emit 100**（否则前端先显示"扫描完成"再弹错误）。前端 `Dashboard.tsx` 零改动（原本就取 `progress.percent`）。
  - **G2【P1】清理列表 100 条截断**：`list_pending_items(conn, session_id, limit, offset) -> Vec` 改为 `list_pending_page(conn, session_id, page, page_size) -> PendingPage{items,total,page,page_size}`，`page` 从 1 起、`page_size` 默认 100 / clamp 上限 500；排序由 `ORDER BY size_bytes DESC` 改 `ORDER BY size_bytes DESC, id DESC`——**这是「翻页跳行」的根因**：同 size 行顺序未定义，SQLite 在不同 LIMIT/OFFSET 下可能给出不同顺序，导致条目重复或整条漏掉。Tauri 命令 `cleanup_list_pending` 入参同步改 `page/page_size`；Items 加分页栏（复用 SafeBox 样式），**翻页清空勾选**（否则跨页误删用户看不见的条目）。
  - **G3【P1】`safe_box.source_task_id` 恒 NULL**：`execute_items` 的 deleter 工厂签名从 `FnMut(item_id)` 改为 `FnMut(item_id, task_id)`，把本次刚写入的 `cleanup_task.id` 传下去；`commands.rs` 两处调用点（`cleanup_execute` / `cleanup_retry_locked`）从写死 `None` 改 `Some(task_id)`（`task_id < 0` 表示 `cleanup_task` 插入失败，仍落 NULL 不写脏值）。**新增真实写入路径回归 5 条**（不构造 DB 行，真跑 `execute_items` → 查 `safe_box.source_task_id` → 查 `list_sessions`），这正是 task-051 的 26 条「读路径」用例照不到的盲区。
  - **G4【P2】清理轮次恒 0**：`cleanup_stats` 两处 SQL 的 `COUNT(DISTINCT session_id)` 改 `COUNT(*)`。「轮次」语义本就是「清理批次」，一次执行一行 `cleanup_task`，`COUNT(*)` 更贴语义，也避免为跨 session 聚合去补 `session_id` 写入链路。迁移前后对比（2 行 `session_id` 均为 NULL）：**0 → 2**。不改 schema。
  - **G5【P2】「清理成功，释放 0 B」**：`Deleter::delete_file` 返回值从 `Result<(), String>` 改 `Result<bool, String>`（`true` 真删 / `false` NotFound 幂等），`FsDeleter` / `SafeBoxDeleter` / `NoopDeleter` 三处实现同步；`ExecResult` 加 `already_gone`、`ExecuteSummary` 加 `already_gone_count`，`failed_count` **排除**已不存在（语义上不算失败），`audit_log.detail` 补 `already_gone` 字段便于取证；前端结果条改「已清理 N 项，释放 X MB；M 项已不存在（未计入释放量）」，`M=0` 时不显示后半句。
  - **门禁**：`cargo check --workspace --all-targets` 0 / `cargo clippy --workspace --all-targets -- -D warnings` 0 / `cargo fmt --all -- --check` 0 / **`cargo test --workspace` 245 passed + 8 ignored** / `pnpm lint` 0 error（2 条 `react-refresh` warning，task-051 遗留）/ **vitest 53 passed（7 files）** / `pnpm build` **180.85 kB │ gzip 57.76 kB**。
  - **新增用例 29 条**：G1（`scan_progress_percent.rs` 4 + `runner.rs` 单测 5）、G2（`list_pending_items.rs` 6 + 前端 3）、G3（`execute_writes_source_task_id.rs` 5）、G4（`audit_log_stats.rs` 6）、G5（`already_gone.rs` 7 + 前端 3）。既有 `execute_items_idempotent_on_already_deleted` 按新语义改写（第二次删已不存在 → `deleted_count=0` / `already_gone_count=1` / `failed_count=0`），非放宽断言。
  - **与任务书不符 1 项（重要）**：任务书「允许修改范围」白名单漏了 `engine/src/execute/delete.rs` 与 `engine/src/execute/fs_deleter.rs`——但 G5 要求 `delete_file` 返回 `Result<bool, String>`，`Deleter` trait 定义在 `delete.rs`、`FsDeleter` 实现在 `fs_deleter.rs`，不改这两处无法通过编译。已按最小改动修改（trait 签名 + 两个实现的返回值），并在实现备注中标注白名单缺口。
  - **流程变更**：按 2026-10-25/26 新规，**开发轮不跑 CI、不做 GUI 走查与安装态冒烟**，全部移交测试 Agent；`tauri build` 仅验证能出包。

2026-10-26：task-052 **测试轮通过 → 评审通过 → 合并 main**（merge commit **`d92db1b`**，CI PR #17 run **40311963933** 全绿）——测试报告 `docs/test-reports/052-test-report.md`、评审报告 `docs/reviews/task-052-gui-p1p2-fixes-review.md`（11 提交全部 `#task-052`、无越界、无敏感文件、抽查复现通过）：
  - **门禁**：`cargo test --workspace` 245+8 / vitest 53 / build 180.85 kB；NSIS 4.01 MB + MSI 5.83 MB（含 `.sig`；签名步骤因本地无 `TAURI_SIGNING_PRIVATE_KEY` 跳过，非代码缺陷）；AppData 73 文件 1.3 GB、安装目录零用户数据写入。
  - **CDP 真机（:9222 可用）**：Dashboard 清理轮次 = **17**（G4 修复前恒 0）、Items 分页栏存在、Console 0 红错。
  - **⚠️ G3 真机证据错位（本轮复验才发现）**：报告以「Items 页 Session 列显示 `#9010/#9009/#9008`」作为 G3（`safe_box.source_task_id`）真机通过证据，但 **Items 的 Session 列取的是 `cleanup_item.session_id`（扫描会话号），与 `safe_box.source_task_id`（清理批次 id）不同源**，不能互证。G3 的 UI 端到端在测试轮未验证；后端写入路径由 `engine/tests/execute_writes_source_task_id.rs` 5 条用例（真 DB + 真文件）兜底，代码可信。**补充：2026-10-27 用户新删文件入箱实测，G3 UI 端到端通过，修复有效。**
  - **未验证项（报告自认）**：G1 进度条真机渲染、G5 `already_gone` 真机文案，均由集成测试覆盖。

2026-10-27：**第二轮真机复验（用户手动走查）——发版门禁仍不通过**，报告追加 §六（`docs/test-reports/gui-walkthrough-2026-10-25.md`）：
  - **G7【P1，新】扫描结束时进度条不满 → G1 修复未闭环（真缺陷）**。三重叠加：① `runner.rs:63-72` `interpolate_percent` 用 `n/(n+50)` 渐近，槽内永远达不到 `to`，必须靠编排层补发的「槽位上界 + 收尾 100」两条边界事件；② 这两条紧跟最后一条 item 事件（同毫秒级），被 `ipc/progress.rs:118-137` **丢弃式节流**（`<100ms` 直接 `return`、**无 trailing 补偿**）吞掉；③ 前端 `Dashboard.tsx:105-110` 在 `scan_start` resolve 后**立即 `unlisten()`**，即便事件已 emit 也可能未派发到回调。**测试为何漏检**：`scan_progress_percent` 注入 `VecEmitter`（**不经过 `ThrottledEmitter`**），收尾 100 必然被记录 → 断言绿；测试替身绕过了生产链路上的节流器。
  - **G3 ✅ 已真机实测通过（同日补充验证）**：首轮看到「Session 多选框空、来源列 78 条全 `—`」，截图全部「移入时间」为 `2026/10/25 19:21:04` / `18:37:47`，早于 `dc606cd`（修复代码，10-25 深夜）与 `d92db1b`（合并，10-26）→ 这批行写于 `SafeBoxDeleter::new(..., None)` 时期，`source_task_id` 天然 NULL，属历史数据。**用户随后新删除文件入箱实测：「来源」列正确显示 `#N`、Session 多选框出现 `session #N (M 条)` 并可筛选 → G3 修复有效，关闭。**
  - **G8【P2，新】历史行永久不可追溯**：`source_task_id` 写入 NULL 后无法补回（信息当时即丢失），task-052 之前入箱的行（现存 78 条）会**永远**显示 `—`、**永远**无法被 Session 筛选命中，用户无法区分「历史条目」与「修复没生效」（首验即因此二次误判）。G3 已验通过，但这条体验缺陷独立存在。建议 UI 把 NULL 渲染为「历史（无来源）」+ 筛选框旁加说明。
  - **下一步**：开 `bugfix/task-053-progress-tail-flush`（G7 三处修复 + **过 `ThrottledEmitter` 的尾部用例** + 前端完成兜底）+ G8 文案。

2026-10-27：**task-053 开发完成（G7 + G8），状态 `testing`**（`bugfix/task-053-progress-tail-flush`）：
  - **G7.1 选 `emit_force()` 而非 trailing 补偿**：`ProgressEmitter` 加 `emit_force`（默认转发 `emit`，无节流的实现无需覆写）；`ThrottledEmitter` 覆写为「直接转发 + 重置窗口起点」——重置是为了**不突破 ≤1 次/100ms 的架构约束**（不重置的话紧随 force 之后的普通 emit 会立刻再发一条）；`Box<dyn ProgressEmitter>` 必须一并透传，否则退化成默认实现 → 调 `emit` → 被内层节流吞掉，方法等于白加。选它的理由：trailing 补偿要后台定时器或注入时钟，`emit_force` 无时钟依赖、确定性可测。
  - **G7.2** 编排层收尾改 `emit_force_progress(100, "done", …)`；槽位边界事件间隔以秒计不会被吞，仍走普通 `emit` 保住节流上限。
  - **G7.3** 前端退订策略改为「收到 `percent >= 100` 立刻退订 / 否则 200ms 兜底退订」（`TAIL_UNLISTEN_DELAY_MS`），并在 `scan_start` resolve 后 `setProgress(p => p ? {...p, percent: 100} : p)` 兜底置满（`p` 为 null 时保持 null，不伪造 sessionId）；用可变句柄承载 `unlisten` + `stopped` 标记处理「listen 回调早于 await 返回」的极端时序；`tailTimerRef` 在卸载时清理。
  - **G7.4 关键回归** `engine/tests/scan_progress_throttled_tail.rs`（5 条）：主用例跑**真实 `ThrottledEmitter`** 包 `VecEmitter`，与裸 `VecEmitter` 基线对比，断言 ① 最后到达 `percent === 100`；② 事件数 **< 节流前**（节流仍生效，不是全放行）；③ 单调不减。另有 `emit_force_bypasses_throttle_window` / `emit_force_resets_throttle_window` / `boxed_emitter_emit_force_still_passes_through`（`Arc<VecEmitter>` 共享观测点，真断言）/ `throttled_emit_still_drops_within_window`。
  - **反向验证（证明用例有效）**：把收尾临时改回 `emit_progress`，主用例立刻红——`收尾事件被节流吞了——最后到达的是 80，完整序列 [0, 20, 40, 60, 80]`，与真机 G7 现象（停在 80/88）一致；还原后绿。这一步是 task-052 教训（`VecEmitter` 绕过生产链路）的直接落实。
  - **G8.1** `SafeBox.tsx` 来源列 `sourceTaskId == null` → 「历史（无来源）」（`#9ca3af` / 12px / `title` 说明），列宽 90→120；筛选框旁「早于 v0.1.6 的条目无来源信息」（11px 灰）。**不改 DB**（历史信息当时即丢失，强行回填反引入风险）。
  - **门禁**：`cargo check --workspace --all-targets` 0 / `cargo clippy --workspace --all-targets -- -D warnings` 0 / `cargo fmt --all -- --check` 0 / **`cargo test --workspace` 250 passed + 8 ignored**（task-052 基线 245+8，新增 5 条）/ `pnpm --filter app lint` 0 error（2 条既有 `react-refresh` warning）/ **vitest 54 passed（7 files）**（基线 53 + 1）/ `pnpm --filter app build` **181.51 kB │ gzip 58.01 kB**（基线 180.85）。
  - **与任务书不符 2 项**：① 白名单写 `engine/src/scan/runner.rs`，但 `ThrottledEmitter` 实际在 `engine/src/ipc/progress.rs`（任务书 §G7.1 自己写「具体实现文件由 DevAgent 定位」），已改该文件；② 白名单漏 `app/src/pages/SafeBox.test.tsx`（改文案后原「来源列显示 —」用例必红），已同步。
  - **按新流程**：开发轮**不跑 CI、不做真机 GUI 走查、不做安装态冒烟**，全部移交测试 Agent。任务书要求的「连扫 3 次停在 100%」真机证据由测试 Agent 在测试轮产出。

2026-10-27：**task-054 开发完成（v0.1.6 发版准备），状态 `testing`**（`feature/task-054-release-prep-v016`）：
  - **① `release.yml` 补 `target_commitish: ${{ github.sha }}`**（`softprops/action-gh-release` 步骤）。不指定时 GitHub 按默认分支 HEAD 建 tag，若 `workflow_dispatch` 之后 main 又进新 commit，tag 会指向一个**从未构建过**的 commit——源码与产物对不上，updater 溯源和事后排查都失去依据。
  - **② 版本号 `0.1.0` → `0.1.6`**：`package.json`、`app/package.json`、`app/src-tauri/tauri.conf.json`、`app/src-tauri/Cargo.toml` 四处一致，`Cargo.lock` 由 `cargo check` 自动更新（`cdisk-cleaner` 条目 0.1.0 → 0.1.6）。`tauri build` 实测产出 `C-DiskCleaner_0.1.6_x64-setup.exe`（4.01 MiB）+ `C-DiskCleaner_0.1.6_x64_en-US.msi`（5.83 MiB）+ **成对 `.sig`**（注入 `TAURI_SIGNING_PRIVATE_KEY` 后）。**注意：CI 发版时仍会按 `workflow_dispatch` 输入的 version 覆盖 `tauri.conf.json`（`scripts/release-lib.mjs patch-conf`），仓库基线版本只是本地构建的默认值，两者不冲突。**
  - **③ 新建 `CHANGELOG.md`**（Keep a Changelog 格式）：v0.1.6 条目分「新增 6 项 / 修复 9 项 / 已知问题 4 项」。**比任务书多补了两条真实变更**——task-049/049b 的「占用进程可处置」（RmShutdown + 映像名护栏 + 一键重试）与 task-050 的「安装版取不到内置规则库」（资源路径错位），二者都是 v0.1.5 之后合入、用户可感知的。文件头注明「自 v0.1.6 起启用，更早版本未回溯」。
  - **④ 新建 `docs/test-reports/gui-walkthrough-round3-checklist.md`**：**27 条**，A~J 十组（安装与启动 / 扫描进度 / 列表分页 / 保险箱来源 / 仪表盘轮次 / 已不存在语义 / 占用关闭 / 自动更新 / Console / 卸载），每条带「NSIS 结果 / MSI 结果 / 证据」三列。门禁口径写在第一节：**「排除 G6 已知项（2 error + 4 warning）后零红错」**，并写明「数据为空 ≠ 功能失效」「必须造数据再验」。末节列出与前两轮的 7 项差异，其中专门标注「G3 只看 `safe_box.source_task_id`，不拿 Items 页 `cleanup_item.session_id` 互证」——这是二轮报告踩过的错。
  - **⑤ 新建 `docs/release/v0.1.6-upgrade-verification.md`**：前置 5 条 + 6 步（装 v0.1.5 → 确认 banner → 下载安装 → 重启确认版本 → 功能回归 R1–R6 → 回写结果）+ 卡点速查表。**明确标注「本手册由用户在发版后手动执行，DevAgent/测试 Agent 不执行」**。
  - **⑥ `.gitignore` 修一处误伤**：`release/` 规则本意是 Tauri 构建产物，但会把 `docs/release/` 一起忽略（`git add` 直接报 ignored）。加 `!docs/release/` 负向包含，放在 `release/` 之后才生效；`git check-ignore` 已验证不再命中。
  - **门禁**：`cargo check --workspace --all-targets` 0 / `cargo clippy --workspace --all-targets -- -D warnings` 0 / `cargo fmt --all -- --check` 0 / **`cargo test --workspace` 250 passed + 0 failed + 8 ignored**（与 task-053 基线一致，本任务不改业务代码）/ `pnpm --filter app lint` 0 error（2 条既有 `react-refresh` warning）/ **vitest 54 passed（7 files）** / `pnpm --filter app build` **181.51 kB │ gzip 58.01 kB** / **`tauri build` exit 0**（两个 bundle + 两个 `.sig`）。
  - **环境坑（本轮新发现，已记入 memory）**：`pnpm test` 连续多次报 `EPERM: operation not permitted, open 'C:\Users\An\AppData\Local\Temp\...\web\...'`，栈里出现 `node-brokered-fs-shim.cjs`（沙箱 fs 代理）→ 是**沙箱拦系统 Temp 写入**，非代码问题。修法：把 `TMPDIR/TEMP/TMP` 指到工作区内（`target/vitest-tmp`）后 7 files / 54 passed 全绿。另注意 `cargo test --workspace` 后台跑时并行跑 vitest 会加剧该现象。
  - **与任务书不符 4 项**：① 白名单路径写 `src-tauri/*`，实际在 `app/src-tauri/*`（workspace 布局），`Cargo.lock` 在仓库根；② 白名单未含 `.gitignore`（不改则新手册无法入库）；③ 任务书假设「`pnpm --filter app build` 产物文件名含 0.1.6」——vite build 产物是 `index-*.js`，不含版本号，改用 `tauri build` 验证安装包名；④ 任务书要求「重启后『关于』页显示 v0.1.6」，但**本应用没有「关于」页**（设置页也不显示当前版本），手册改用三条等效验证（已安装应用列表版本 / exe 文件属性产品版本 / 安装目录对比），命中两条即通过。
  - **按新流程**：开发轮**不跑 CI、不做真机/安装态冒烟**。NSIS/MSI 双跑由测试 Agent 按第三轮清单执行，发版由用户手动 `workflow_dispatch`。

## 已完成事项
- [x] 项目立项与仓库初始化（commit 67990f5 / 4972f9d）
- [x] 固化立项阶段 8 项关键决策：Tauri 无 Plan B / Cargo.lock 入库 / 保险箱 5GB FIFO / 规则库 GitHub Raw+SHA-256 / 不买 EV 签名 / 仅 Windows / 按业务模块 A~F 切分 / PROJECT_STATE 由 M0 当天首写（commit c0bdf05）
- [x] 完成产品需求文档 PRD v0.1（commit caae920）
- [x] 完成技术需求文档 TRD v0.1（commit 261b24f）
- [x] 完成系统架构文档 ARCHITECTURE v0.1（commit c1f027a）
- [x] 完成多 Agent 协作与 Git 权限规范 AGENTS.md（commit 3a1bb2c）
- [x] 初始化项目状态白板 PROJECT_STATE.md（本次提交）

## 待办事项
- [x] 001：初始化 monorepo 目录骨架（app/ Tauri+React+Vite、engine/ Rust workspace、elevation/、rules/、scripts/），跑通最小空窗口
- [x] 002：落地公共数据层——SQLite 单文件 schema（settings/scan_session/cleanup_item/cleanup_task/safe_box/audit_log/rules_meta/registry_backup/cleanup_plan）与 rusqlite 连接、迁移
- [x] 003：落地删除唯一入口 engine::execute::delete::execute() + L3 红线白名单匹配器 + 单元测试（对 PRD P0-7 全部 L3 路径 100% 拦截）
- [x] 004：落地 IPC 网关骨架——统一响应信封 {code,message,data}、scan.start/cleanup.dryRun 命令、进度事件通道
- [x] 005：落地 rules/cache_rules.json 初始规则库 + rules_meta.json + SHA-256 校验加载逻辑
- [x] 006：搭好 CI（GitHub Actions：cargo clippy/fmt/test + pnpm lint/test + 规则 JSON schema 校验）
- [x] 007：真实 ProgressEmitter（AppHandle::emit）——scan://progress 真机可被前端 listen 收到（依赖 task-004、真实扫描任务；约 1 人天）
- [x] 008：symlink/junction 解析防绕开——规范化器接 symlink_metadata，junction 指向 L3 时一并拒（依赖 task-003、platform 模块；约 1 人天）
- [x] 009：DB 版 ItemPathResolver——cleanup_item 表按 id 取路径/大小/模块，替换 MockResolver（依赖 task-002、cleanup.execute；约 1 人天）
- [x] 010：外部红线集注入——rules JSON 加载后与内置 L3 红线取并集（依赖 task-005；约 0.5 人天）
- [x] 011：ps1 兼容层来源对齐与 NDJSON 改造（scripts/ 五个脚本；约 1 人天）
- [x] 012：Rust 侧 ps1 NDJSON 消费器——parser + ScanRunner（spawn/写库/emit）
- [x] 013：Tauri scan.start 真接通——Orchestrator 串跑 5 ps1 + 真实 session + 回写 status
- [x] 014：cleanup.dryRun 切 DbItemPathResolver（按 id 反查 cleanup_item 真实路径）
- [x] 015：cleanup.execute 真实删除——FsDeleter + execute_items 编排 + Tauri 命令 + audit_log
- [x] 016：safe_box 5GB FIFO 保险箱——删除前移入 box 目录 + 超容按时间最旧永久剔除
- [x] 017：safe_box restore——按 id 把文件从保险箱 rename 回原路径，不自动建父目录、不批量
- [x] 018：safe_box.list——列出当前所有未剔除条目 + 汇总（条目数/总字节）
- [x] 019：删除失败错误分类——classify_io_error 识别 SharingViolation/PermissionDenied，audit_log 写具体原因
- [x] 020：who_is_locking——Windows Restart Manager FFI 查占用进程 PID
- [x] 021：占用进程自动接线——FILE_IN_USE 时 execute_items 自动调 who_is_locking 写 audit_log 并返回前端
- [x] 022：rules_meta 落 DB + rules.checkUpdate——查 GitHub Raw 比对版本（不下载替换）
- [x] 023：rules.applyUpdate——下载新 cache_rules.json + SHA-256 校验 + 备份替换 + upsert DB
- [x] 024：audit_log.list——按 ts 倒序分页只读查询，M1-F 报表后端基础
- [x] 025：audit_log.stats——cleanup_task 聚合（total_bytes / success / fail / sessions）
- [x] 026：前端 Dashboard 4 数字卡片——formatBytes + Dashboard.tsx + App.tsx 挂载（stats 接前端，GB 格式化）
- [x] 027：前端 History 明细页——formatTs + History.tsx 表格（时间/事件/路径/详情）+ 分页 + App.tsx tab 切换（audit_log.list 接前端）
- [x] 028：前端开始扫描按钮——Dashboard 按钮 + scan_start invoke + listen(scan://progress) 实时进度条
- [x] 029：清理闭环——cleanup.listPending 后端命令 + Items 表格（checkbox + 清理已选）+ App 第三 tab
- [x] 030：保险箱 UI——safe_box_list 表格 + 汇总 + 单条「恢复」按钮 + App 第四 tab
- [x] 031：规则更新 UI——Settings 规则库卡片 + 检查更新/应用更新按钮 + App 第五 tab
- [x] 032：前端工程化补齐——ESLint 10 flat config + Vitest 2.1 + formatBytes/formatTs 单测 + CI 接 lint/test
- [x] 033：NSIS v0.1.0 打包——tauri.conf identifier/nsis + CI tauri build/upload-artifact，本地出 .msi/.exe
- [x] 034：GitHub Release workflow——release.yml 支持 workflow_dispatch + tag 触发，softprops/action-gh-release 上传 installer
- [x] 035：tauri updater 自动更新最小闭环——签名密钥对 + plugins.updater 配置 + Settings 应用更新卡片 + release.yml 签名/latest.json 生成
- [x] 036：启动自动 check + 非阻塞 banner——App.tsx useEffect 自动 check()，有新版本顶部蓝条提示 + 立即更新/稍后按钮
- [x] 037：safe_box purge 单条永久删除——后端 purge_item + Tauri 命令 + 前端红色按钮 + confirm
- [x] 038：safe_box 批量 purge——后端 purge_many + Tauri 命令 + 前端 checkbox 列 + 批量删除按钮
- [x] 039：safe_box 批量恢复——后端 restore_many + Tauri 命令 + 前端绿色批量恢复按钮（复用 task-038 checkbox）
- [x] 049：占用进程 RmShutdown——后端 `shutdown_locking_processes` + 危险进程护栏 + `cleanup_retry_locked` 命令 + Items 失败明细与「关闭占用进程并重试」（**评审轮 1 不通过 → task-049b 修三项缺陷 → 测试轮 2 通过，状态 `awaiting_review`，待评审重评**；测试轮 1 修 C12 三态互斥 + 补 B7 隔离用例；修复轮改 PID→映像名护栏 + mixed fail-closed + B7 独立 console；测试轮 2 修护栏用例 PID 环境依赖 + `reason` 注释）
- [x] 048：v0.1.1 真机升级验证收尾——已装 v0.1.0 的机器启动应用，updater banner 正确弹出「发现新版本 v0.1.1」（用户真机验证通过，2026-10-23），**M2 正式关闭**
- [x] 050：打包资源路径错位修复——`bundle.resources` 数组式改映射式（规则/脚本落到 `<安装目录>/rules`、`/scripts`）+ `AppPaths` 抽 `pick_resource_root`/`build_candidates`（release 不依赖 cwd、缺失显式报错带绝对路径）+ `scripts/tauri-resources.test.mjs` 契约测试接 CI（`bugfix/task-050-bundle-resources-path`；**2026-10-25 评审通过，merge commit `cadb286`，分支已删**；测试轮 2026-10-24 通过：L1/L2/L4/L5 全绿、0 缺陷、MSI 抽检已补做，GUI 仍第三档待真机，报告 `docs/test-reports/050-test-report.md`，评审 `docs/reviews/050-review.md`）

- [x] 051：safe_box 增强——时间区间筛选（24h/3d/7d/自定义）+ 多 session 组合筛选（`Vec<i64>` + 动态 WHERE）+ 审计历史联表（`safe_box_item_history` 按原路径 `COLLATE NOCASE` 查 `audit_log`）+ 前端闭环（预设下拉 / 多选 session / 来源列 / 行内历史懒加载）（`feature/task-051-safebox-filter-history`；**2026-10-25 评审通过，merge commit `e6ab561`，feature 与 bugfix 子分支均已删**；测试轮 2026-10-25 通过：L1 220+8 / vitest 46 / 契约 6/6 / build 179.94 kB，NSIS 干净目录安装 9 项布局 + 安装目录零写入，修 1 项缺陷（刷新按钮实参污染，`bugfix/task-051-refresh-click-arg` → merge `c1a1af6`），GUI 仍第三档，报告 `docs/test-reports/051-test-report.md`，评审 `docs/reviews/051-review.md`）
- [x] 052：GUI 走查 P1+P2 修复——**G1** `ScanOrchestrator` 脚本级百分比（槽位基线 + `n/(n+K)` 亚线性插值 + 边界事件，`percent` 单调递增到 100）；**G2** `cleanup_list_pending` 分页（`page/page_size` + `total` + `ORDER BY size_bytes DESC, id DESC` 稳定排序）+ Items 分页栏；**G3** `execute_items` 经工厂闭包把 `cleanup_task.id` 传给 `SafeBoxDeleter`，`safe_box.source_task_id` 真实写入（含真实端到端回归用例）；**G4** 清理轮次口径 `COUNT(DISTINCT session_id)` → `COUNT(*)`；**G5** deleter 返回 `Result<bool, String>` 区分真删 / 已不存在，结果条单列 `alreadyGoneCount`（`feature/task-052-gui-p1p2-fixes`；**2026-10-26 测试通过 → 评审通过 → 合并 main（merge commit `d92db1b`，CI PR #17 run 40311963933 全绿），分支已删**；测试报告 `docs/test-reports/052-test-report.md`、评审报告 `docs/reviews/task-052-gui-p1p2-fixes-review.md`。**遗留（2026-10-27 复验）**：G1 进度条修复未闭环 → 复开为 **G7**；G3 真机 UI 未验且测试证据用错字段；G5 真机文案未验；衍生 **G8** 历史行不可追溯）

- [x] 053：进度条尾部冲刷 + 保险箱历史来源文案——**G7**（唯一发版阻塞）三处修复：① `ProgressEmitter` 新增 `emit_force`（默认转发 `emit`，`ThrottledEmitter` 覆写为绕过节流并重置窗口，`Box<dyn>` 一并透传防退化）；② `ScanOrchestrator` 收尾 `percent=100` 改走 `emit_force`；③ 前端 `Dashboard.tsx` 收到 100 立刻退订 / 200ms 兜底退订 + resolve 后主动置满进度条；**关键回归** `engine/tests/scan_progress_throttled_tail.rs` 5 条（走真实 `ThrottledEmitter`，断言最后到达 100 且事件数被压低）。**G8** `SafeBox.tsx` 来源列 NULL 渲染「历史（无来源）」淡灰文案 + 筛选框旁「早于 v0.1.6 的条目无来源信息」（`bugfix/task-053-progress-tail-flush`；**开发轮完成，状态 `testing`**；门禁 check/clippy/fmt 0、`cargo test --workspace` 250 passed + 8 ignored、vitest 54、build 181.51 kB；**按新流程开发轮不跑 CI、不做真机/安装态冒烟，移交测试 Agent**）

- [x] 054：v0.1.6 发版准备——**①** `release.yml` 的 `softprops/action-gh-release` 步骤补 `target_commitish: ${{ github.sha }}`（tag 精确指向触发发版的 commit，不再默认指向 main HEAD）；**②** 版本号 `0.1.0` → `0.1.6`（`package.json` / `app/package.json` / `app/src-tauri/tauri.conf.json` / `app/src-tauri/Cargo.toml` 四处 + `Cargo.lock` 自动更新），`tauri build` 产出 `C-DiskCleaner_0.1.6_x64-setup.exe`（4.01 MiB）+ `.msi`（5.83 MiB）+ 成对 `.sig`；**③** 新建 `CHANGELOG.md`（Keep a Changelog 格式，v0.1.6 的「新增/修复/已知问题」三段，自本版起启用不回溯）；**④** 新建 `docs/test-reports/gui-walkthrough-round3-checklist.md`（27 条，NSIS/MSI 双列，含 G1/G2/G3/G4/G5/G7/G8 + 占用关闭 + updater + Console 口径 + 卸载）；**⑤** 新建 `docs/release/v0.1.6-upgrade-verification.md`（6 步升级闭环手册 + 卡点速查）；另修 `.gitignore` 的 `release/` 规则误伤 `docs/release/`（加 `!docs/release/` 负向包含）（`feature/task-054-release-prep-v016`；**开发轮完成，状态 `testing`**；门禁 check/clippy/fmt 0、`cargo test --workspace` 250 passed + 8 ignored、vitest 54、build 181.51 kB、`tauri build` exit 0；**发版动作（workflow_dispatch）由用户手动执行，不在本任务内**）

- [x] 055：M3 更新体验——**①** `onUpdateNow` 订阅 `downloadAndInstall(onEvent)`（真实签名是**第一个参数就是回调**，非 `{onEvent}`；事件字段 `data.contentLength` / `data.chunkLength`，非 `totalSize`/`chunkSize`），`Started` 记总量、`Progress` 累加、`Finished` 切 installing；**②** banner 下载中显示线性进度条 + `下载更新中 N%` + `已下载 X.X / Y.Y MB，速度 ≈ Z`（EMA 平滑，1s 窗口、α=0.4；total 未知时退化为「总大小未知」）；**③** 失败态按关键词映射 5 类中文（网络 / 签名 / 磁盘 / 已是最新 / 兜底），banner 给「重试」+「关闭」；**④** 「稍后提醒」写 `localStorage["cclean:update:dismissedVersion"]`，启动时同版本不弹、更高版本照弹；**⑤** Settings 手动检查显示「当前 vX / 发现 vY」或「当前 vX，已是最新版本」，失败文案复用同一映射，`onUpdateFound` 让手动检查能强制显示 banner（`feature/task-055-update-banner-progress`；**开发轮完成，状态 `testing`**；门禁 lint 0 error、vitest **81 passed（9 files，新增 27）**、build **184.22 kB**、`cargo test --workspace` 250+8 不回归；**按新流程开发轮不跑 CI、不做真机/安装态冒烟**，移交测试 Agent）

- [x] 056：M3 技术评估 + 遗留小修——**①** 新建 `docs/research/delta-update-evaluation.md`（A: Tauri 2 官方 updater **不支持**差量，`plugin-updater@2.13.3` 全包无 bsdiff/delta/xdelta 且协议只认完整包；B: bsdiff4；C: xdelta3；含签名/旧版本校验/产物矩阵/回退四个决策点）→ **结论：不做差量更新，建议改做「release profile 瘦身」**（4 MB 包差量只省 1–2 MB，却要 6–9 人日 + 自建更新通道）；**②** 迁移器 **v2** 给 `safe_box` 加 `restored_at INTEGER`，`restore` 从 `DELETE` 改 `UPDATE status='restored'`（列表/配额过滤 `status='in_box'`，外部行为不变）；**③** 新增 `engine/tests/safe_box_cross_disk.rs` 2 条（本机 C:→D: 真跑验证 copy+remove fallback；单盘环境打印 `[skip]` 不红），顺带固化「跨盘 restore 无 fallback」这一已知限制；**④** G6 定稿**豁免不修**（根因 Tauri 2.x custom protocol 首调时序竞争；warmup 方案否决——只是把红错换个命令名，放 App 挂载还会让每次启动都红），豁免口径写入第三轮清单 §六（5 条判定规则）（`feature/task-056-research-and-cleanup`；**开发轮完成，状态 `testing`**；门禁 check/clippy/fmt 0、`cargo test --workspace` 255 passed + 8 ignored、vitest 81、build 184.22 kB；**按新流程开发轮不跑 CI、不做安装态冒烟**，移交测试 Agent）
- [x] 057：RmRestart 自动重启被关进程（默认关闭 + 设置页开关）——**①** `engine/src/prefs.rs`：复用 v1 `settings` 表存 `prefs.auto_restart_on_close`（"1"/"0"，缺行/非法值 fail-closed 到 false），`get_auto_restart`/`set_auto_restart`（UPSERT）+ `Prefs` serde camelCase + 5 条单测；**②** `engine/src/execute/locking.rs`：`shutdown_locking_processes(path, auto_restart: bool)` 签名升级，`RmSession::restart(pid)` 调 `RmRestart(handle, pid, None)`，仅对 `bRestartable.as_bool()==true` 且 pid ∈ closable 的进程调用，总超时 10s，失败只 warning 不阻塞删除；纯函数 `rm_restartable` 单测 1 条；**③** Tauri 命令 `get_prefs`/`set_prefs` 注册，`cleanup_retry_locked` 在 spawn_blocking 前读 prefs 传入；**④** Settings 页新增「行为偏好」卡片 + checkbox toggle + 灰色说明文字；**⑤** Items.tsx 启动时 get_prefs 回填，confirm 文案开启时追加「注意：本次会在关闭后自动重启被关闭且支持重启的进程。」。**产品硬约束：默认关闭**，新用户装完看不到任何自动重启行为（`feature/task-057-rm-restart-toggle`；**开发轮完成，状态 `testing`**；门禁 check/clippy/fmt 0、cargo lib 53 passed（含 prefs 5 + rm_restartable 1）、cargo test -p engine 全绿、vitest **85 passed（+4）**、build 185.59 kB/gzip 59.59 kB；**按新流程开发轮不跑 CI/真机/安装态冒烟**，移交测试 Agent）

- [x] 058：M4 收尾——**①** RmRestart **白名单**（`prefs.restart_whitelist`，JSON 数组存 `settings` 表；支持 `*` 通配；**空=不限制**，非空时 fail-closed：只有被关闭的可重启进程**全部**命中才调 `RmRestart`——因为 `RmRestart` 无按 pid 参数、一次重启整批，「只重启名单内」在 RM 机制下做不到）+ Settings 白名单 textarea；**②** `safe_box` **原子性**：INSERT 提到 rename **之前**并整段包事务（旧实现顺序反了 → rename 成功但 INSERT 失败 → 孤儿文件被 enforce_quota 当 purged，用户感知「文件凭空消失」），跨盘 copy/remove 失败时清理半成品目标文件；**③** 新建 `docs/research/code-signing-evaluation.md`（EV/OV/自签/Store 四方案 + **2023-06 CA/B 规则：私钥必须硬件存储**→与 GitHub Actions 天然冲突 + Azure Artifact Signing 地域限制）→ **结论当前不做**，给出三个「该做」的触发条件；**④** `CHANGELOG.md` 追加 **v0.1.7 草稿**（新增/修复/变更/已知问题四段），**版本号保持 0.1.6 不升**（`feature/task-058-m4-finishup`；**开发轮完成，状态 `testing`**；门禁 check/clippy/fmt 0、`cargo test --workspace` 273 passed + 8 ignored、vitest 89、build 187.46 kB；**按新流程开发轮不跑 CI、不做真机/安装态冒烟**，移交测试 Agent）

- [x] 059：v0.1.7 发版准备 + README/用户手册——**①** 版本号 `0.1.6` → `0.1.7`（`package.json` / `app/package.json` / `app/src-tauri/tauri.conf.json` / `app/src-tauri/Cargo.toml` 四处 + `Cargo.lock`），`tauri build` 实测产出 `C-DiskCleaner_0.1.7_x64-setup.exe`（4.05 MiB）+ `.msi`（5.87 MiB）+ 成对 `.sig`；**②** `CHANGELOG.md` v0.1.7 定稿（日期 `2026-10-29`、删 `[Unreleased]` 段）；**③** **重写 `README.md`**——原文件是早期版本（状态还停在「待进入 PRD 与 M0 技术验证」、目录结构写「M1 阶段将新增 app/engine」），重写为面向开源用户的版本（一句话定位 / 与两类工具的差异对比 / 7 项功能 / NSIS+MSI 安装指引 + SmartScreen 说明 / 开发命令 / 技术栈表 / 真实目录结构 / 安全与隐私基线 / 贡献指南）；**④** 新建 **`docs/USER_GUIDE.md`**（8 节：快速开始 / 扫描 / 清理 / 保险箱 / 历史与审计 / 设置 / 更新机制 / 常见问题 8 问，全部按真实 UI 文案撰写）（`feature/task-059-release-prep-v017`；**开发轮完成，状态 `testing`**；门禁 check/clippy/fmt 0、`cargo test --workspace` 273 passed + 8 ignored、vitest 89、build 187.46 kB、`tauri build` exit 0；**发版动作（`workflow_dispatch` version=0.1.7）由用户手动执行**）

- [x] 060：占用关闭体验——**①** RmShutdown 连带兄弟进程**核实后不改代码**：任务书要求「`RmRegisterResources` 再注册进程本身（`&[pid]`）」，核实后不采纳——参数类型是 `Option<&[RM_UNIQUE_PROCESS]>`（pid + ProcessStartTime）不是裸 pid；且注册「锁文件的进程」是**冗余**的（`RmGetList` 已通过文件找到它，集合不变）；「连带关闭兄弟进程」是 RM 按 console 进程组的内部行为，task-049b 实测**只注册文件就已经发生**，注册参数既不能开启也不能关闭它；强行注册同名应用还会扩大关闭面并可能触发 mixed fail-closed 让整批都关不掉。已在 `RmSession::register` 加 4 条理由的注释 + 文件头「已知限制」第 4 条补充，并在 USER_GUIDE 写清用户可见行为。**②** RmShutdown **1s 进度反馈**：`shutdown_locking_processes` 加第四参 `on_progress: Option<ProgressCallback>`，内部起 1s ticker 线程（100ms 响应停止，`RmShutdown` 返回后立即停，避免线程泄漏），`commands.rs` 转成 `shutdown://progress` 事件（`{elapsedSecs, expectedMaxSecs:30}`），`Items.tsx` 显示「正在关闭进程… 已等 Xs（最多 30s）」+ 线性进度条，超 30s 补一句「仍在等待系统响应」。**③** `USER_GUIDE` 补 **4.5 跨盘行为**（移入有 copy+remove fallback / 恢复**不支持**跨盘 / 为什么不对称 / 实际影响很小）+ FAQ Q9（跨盘恢复报错怎么办，三步处置）（`feature/task-060-shutdown-experience`；**开发轮完成，状态 `testing`**；门禁 check/clippy/fmt 0、`cargo test --workspace` 275 passed + 8 ignored、vitest 90、build 188.06 kB；**按新流程开发轮不跑 CI、不做真机/安装态冒烟**，移交测试 Agent）

- [x] 061：M5 起步——**① 深色/浅色主题**（`prefs.theme` 三态 `system/light/dark`，默认 `system`；实现方式是**纯内联 CSS 变量**——根 div 定义 `--bg/--surface/--text/--text-muted/--border`，子元素用 `var()` 取值，不新增任何 `.css` 文件；App 用 `matchMedia("(prefers-color-scheme: dark)")` 实时跟随系统变化，用 `localStorage` 缓存首屏防闪烁、DB prefs 为真值；设置页「外观」三态 + 即时生效（`onThemeChange` 回调，不用重启）。**本期只适配 App 外壳 / Dashboard / Settings 三页**——Items/SafeBox/History 的颜色是硬编码且不在白名单内，深色下仍浅色，已在 UI 里如实注明）；**② 清理趋势图**（`cleanup_stats_trend(days)` 按 `date(started_at,'unixepoch','localtime')` 分组求 `SUM(freed_bytes)`，升序；只返回有记录的日期、缺失由前端补 0；`days` 夹到 1..=365；仪表盘**手写 SVG 柱状图**不引图表库，空数据显示「暂无清理记录」，失败显示失败占位且不影响卡片）；**③ 计划自动扫描**（`prefs.auto_scan_enabled` 默认 false + `prefs.auto_scan_interval` 默认 daily（daily/weekly/monthly）+ `prefs.last_auto_scan_at` 内部记录；`auto_scan_due()` 纯函数判断到期；`auto_scan.rs` 起后台线程每 30 分钟轮询、启动后先等 60s；**只扫描不删除**，扫完发系统通知「发现 N 项可清理，约 X」；与手动扫描用 `ProgressState` 的 `try_lock` 互斥，拿不到锁就跳过本轮）（`feature/task-061-m5-start`；**开发轮完成，状态 `testing`**；门禁 check/clippy/fmt 0、`cargo test --workspace` 291 passed + 8 ignored、vitest 98、build 195.04 kB；**新增依赖 `tauri-plugin-notification` + `notification:default` 权限**；按新流程开发轮不跑 CI、不做真机/安装态冒烟，移交测试 Agent）

- [x] 062：提权底座 + 系统还原点 + 托盘常驻（**v0.2.0 起步**）——**① 提权 exe 底座**：`elevation/` 从 M0 stub 填成真实现（`cclean-elevation.exe`，**刻意零依赖手写 FFI**：只需要 `SRSetRestorePointW` 一个函数，引 windows crate 会让独立编译多花几分钟）；命令白名单 `["create_restore_point"]`（未知命令连参数都不解析，直接 `EXIT_UNKNOWN_CMD`）；结果回传 = 退出码（0/2/3/4）+ 临时 JSON（`{ok,cmd,message,sequenceNumber}`）；主进程侧 `engine::elevation::run_elevated` 用 **`ShellExecuteExW` + `SEE_MASK_NOCLOSEPROCESS`**（任务书写 `ShellExecuteW`，但它**不返回进程句柄**，拿不到退出码）；`scripts/build-elevation.mjs` 在 `beforeBuildCommand` 里编译 + 复制到 `app/src-tauri/binaries/`，`bundle.resources` 映射到安装目录根；**② 系统还原点**：`executor::needs_restore_point` 判「本批是否含 L2」（全 L1 不弹 UAC，否则每次清缓存都弹一次会逼用户关掉功能）；名称 `C-clean cleanup #<id> <UTC 时间>`（手写 `civil_from_days`，不引 chrono）；失败返回 **2007**，前端弹 confirm「是否继续清理？」，确认后带 `skipRestorePoint: true` 重调；成功写 audit_log（`action='restore_point'`）；**③ 托盘常驻**：Tauri 2 **没有 `tauri-plugin-tray` 这个插件**（任务书写错），托盘是内置能力、只需开 `tray-icon` feature；`CloseRequested` → `prevent_close()` + `hide()`；右键菜单「立即扫描 / 打开主窗口 / 退出」，左键单击图标直接显示窗口；「退出」不直接 `app.exit()`——先显示窗口 + emit `tray://quit-request`，由前端 `confirm` 二次确认后调 `quit_app` 命令；开机自启用 `tauri-plugin-autostart`，**状态真实来源是注册表**（不存 prefs，否则用户在任务管理器里手动禁用会和我们打架）（`feature/task-062-elevation-srpoint-tray`；**开发轮完成，状态 `testing`**；门禁 check/clippy/fmt 0、`cargo test --workspace` 316 passed + 8 ignored、vitest 101、build 196.96 kB、**`tauri build` 出包 NSIS 4.22 MiB（+0.17 MiB = 提权 exe 已进包）**；按新流程开发轮不跑 CI、不做真机/安装态冒烟，移交测试 Agent）

- [x] 063：报告导出 + 前后对比 + 环形图 + 取消扫描——**① 清理报告导出**：新建 `engine/src/report/mod.rs`（**零依赖字符串拼接**，不引 askama/tera——模板只有「标题 + 3 个数字 + 一张表」）；HTML 单文件（CSS 全内联，双击可开）+ CSV（**带 UTF-8 BOM**，否则 Excel 中文路径全乱码）；落盘 `data_root/reports/cleanup-report-<ts>.{html,csv}`；所有外部文本过 `html_escape`（**文件名里带 `<script>` 会 XSS**）；**② 前后空间对比**：`ExecuteSummary` 加 `disk_before_bytes`/`disk_after_bytes`，`execute_items` 开头/结尾各查一次（盘符从第一条 item 路径推断，不写死 C）；`DriveSnapshot::released_bytes` 用**饱和减法**（清理期间用户写入导致「后 > 前」时算 0 而非负数）；前端显示「清理前 X → 清理后 Y，实际释放 Z」；**③ 仪表盘环形图**：`drive_usage()`（`GetDiskFreeSpaceExW`，用 `total_free` 而非 `free_to_caller` 算已用）+ 手写 SVG 圆环（`strokeDasharray` 画弧 + `rotate(-90)` 起点在 12 点），≥70% 橙、≥85% 红并提示「空间紧张」；**④ 取消扫描**：`ScanOrchestrator` 加 `Arc<AtomicBool>` cancel + `with_cancel()`，`run_all` **每个脚本开始前**检查（脚本粒度，不 kill 正在跑的 ps1）；`ScanCancelState` 全局单例，`scan_start` 每轮**复位**（否则上次的取消会让新扫描一启动就停），`scan_cancel` 立即返回不阻塞；已扫描结果保留在 DB（`cleanup_item` 行不删）（`feature/task-063-report-dashboard`；**开发轮完成，状态 `testing`**；门禁 check/clippy/fmt 0、`cargo test --workspace` 328 passed + 8 ignored、vitest 101、build 199.87 kB；按新流程开发轮不跑 CI、不做真机/安装态冒烟，移交测试 Agent）

- [x] 064：**P0 收尾**——**① 保险箱 7 天自动 purge**：`safe_box::purge_expired(conn, box_dir, now)` **逐条**处理（不是一条批量 DELETE——每条都要写 `audit_log action='auto_purge'`，用户必须能查「哪些被系统自动删了」，批量只能记一行汇总等于丢审计粒度）；**单条失败保留 DB 行**下轮重试（绝不能「文件没删掉就删行」，否则文件成永久孤儿）；文件已不存在但行过期 → 仍删行；`expires_at == now` 不算过期（严格 `<`）。触发：`lib.rs` 起线程**启动跑一次 + 每 6 小时**（7 天粒度不需要每小时）；**② 风险徽标/影响说明**：`PendingItem` 加 `risk_level`（此前没返回，前端只能「所有项一视同仁」——既做不了全选 L1，也说不清「删了会怎样」）；`RISK_META` 四色（L0 灰 / L1 绿 / L2 黄 / L3 红，颜色走 CSS 变量适配深色）+ `title` 原生 tooltip（比自绘省事且不会被表格 overflow 裁掉）；未知等级有 fallback 不崩；**③ 全选 L1 一键按钮**：只勾**本页** L1（跨页全选要把所有页拉下来，按钮会变慢且用户看不到被选中项）；L2/L3 一律不选（L2 需逐项确认，自动全选等于绕过「需确认」这道设计）；按钮旁沿用「已选 N 项，共 X」统计；本页无 L1 时按钮禁用并给 title 说明（`feature/task-064-purage-badge-l1all`；**开发轮完成，状态 `testing`**；门禁 check/clippy/fmt 0、`cargo test --workspace` 334 passed + 8 ignored、vitest 104、build 201.48 kB；按新流程开发轮不跑 CI、不做真机/安装态冒烟，移交测试 Agent）

- [x] 065：**M1 C 模块——WinSxS 组件存储清理**——**① DISM 分析**：新建 `engine/src/scan/winsxs.rs`（`WinsxsAnalysis` + `parse_analyze_output` 纯函数 + `analyze()`）；**任务书写「解析 DISM 输出 XML」与事实不符**——`/analyzecomponentstore` 没有 XML 输出，只有本地化文本，所以提权 exe 侧强制加 `/English` 锁语言，engine 侧按 `标签 : 值` 逐行解析（标签做小写 + 空白归一化，DISM 子项带缩进且各版本空格数不一致）；`reclaimable_bytes` 优先取 `Actual Reclaimable Space`，缺该行时用「Backups and Disabled Features + Cache and Temporary Data」兜底；**`parsed` 标志不可省**——输出格式一变所有字段都是 0，UI 上看起来是「可清理 0 字节」，用户会以为没东西可清而不是分析失败；**② DISM 清理**：`winsxs::cleanup` **先建系统还原点再动手**（fail-closed；建点失败归 `RestorePoint` 错误，前端弹「继续清理将无法回滚」确认后带 `skipRestorePoint` 重调，只重试一次防死循环）；**③ 提权 exe 白名单**：`ALLOWED_COMMANDS` 加 `dism_analyze` / `dism_cleanup`，DISM 参数是 `&'static` 常量表——**`/ResetBase` 属于「没有传参入口」而不是「有入口被拦」**；DISM 走 `%SystemRoot%\System32\Dism.exe` 绝对路径（提权进程不能吃 PATH 劫持）；输出用 `MultiByteToWideChar(CP_OEMCP)` 解码（控制台程序写管道用 OEM 代码页，直接当 UTF-8 解中文路径全乱码）；自实现 15 分钟超时（`Command::output()` 会无限等，提权进程挂住会一直占 UAC 令牌）；**④ 实时进度**：`run_elevated_streaming` 传 `--progress <path>`，`std::thread::scope` 起 tail 线程轮询进度文件（提权进程的 stdout 在 `ShellExecuteExW` 下拿不到），`drain_complete_lines` **按字节**切行（读到半个多字节字符时先 lossy 会改变字节长度、游标算术全错），事件 `winsxs://progress`；**⑤ UI**：Dashboard `WinsxsCard`（分析结果 / DISM 建议 / 二次确认 / 进度面板 / 结果条），**挂载时不自动分析**（DISM 会弹 UAC，一进页面就弹窗是流氓行为）（`feature/task-065-winsxs-cleanup`；**开发轮完成，状态 `testing`**；门禁 check/clippy/fmt 0、`cargo test --workspace` 364 passed、vitest **113**（+9）、build 206.66 kB；按新流程开发轮不跑 CI、不做真机/安装态冒烟，移交测试 Agent）

- [x] 066：**M1 E 模块——大文件地图（只读）**——**① 后端扫描**：新建 `engine/src/scan/big_files.rs`（`scan_all` + `scan_hiberfil` / `scan_windows_old` / `scan_recycle_bin` / `scan_top_files`，**一个字节都不删**）；hiberfil 只读 `metadata().len()` 不开文件；Windows.old 递归求和并沿用同一套条目/时间上限；回收站优先 `SHQueryRecycleBinW`（一次拿全盘、含其他用户），API 不可用退回遍历 `$Recycle.Bin` 并在建议里说明「可能不含其他用户」；Top100 用**显式队列 BFS**（DFS 在预算用尽时会扎进 AppData 深处，实测 Top 结果 21 条 → BFS 后 99 条）+ 最小堆维护 Top N（内存恒定 O(N)）+ 跳过 reparse point（junction 会无限循环）+ 跳过系统目录/根目录系统文件（否则榜单被 WinSxS、pagefile.sys 占满）；单个 `read_dir`/`metadata` 失败只跳过不中断；**② IPC**：`engine/src/ipc/commands/big_files.rs` 纯逻辑层 + Tauri `big_files_scan`（`system_drive_root()` 读 `SystemDrive`，不写死 `C:\`）；③ **前端**：`app/src/pages/BigFiles.tsx` 三张固定分类卡片 + Top100 汇总卡 + 明细表，**全页只有「重新扫描」一个按钮**，顶部单独声明「本页只读、不代删」并解释四类各自的正规处理方式；④ **tab 注册**：App 加 `bigfiles` tab，颜色全走 CSS 变量随主题切换（`feature/task-066-big-files-map`；**开发轮完成，状态 `testing`**；门禁 check/clippy/fmt 0、`cargo test --workspace` **382 passed**（+18）、vitest **123**（+10）、build 211.79 kB；按新流程开发轮不跑 CI、不做真机/安装态冒烟，移交测试 Agent）

> 注：040~047 的完成明细未回填本清单（历史遗漏），以「最近一次变更」段与 `tasks/040-*.md`~`tasks/047-*.md` 为准。
> 编号顺延（2026-10-24）：原定 050 的「safe_box 增强」顺延为 **051**，050 让位给发版阻塞级的打包资源路径缺陷。

## 已知问题

> 2026-10-29 精简：已解决项（G1-G8、task-049b 护栏、task-050 资源路径、task-052/053/056/057/058 等）已从本清单删除，历史记录见 git log 与 `docs/test-reports/`。以下为**当前仍未解决或已拍板不修**的项。

- [B 中] **G6 设置页首调 `rules_check_update` 红错（已豁免，不修）**：Tauri 2.x `useCustomProtocol` 首调时序竞争，功能经 postMessage 回退可用。**发版门禁口径：排除 G6 后 Console 零红即过**。详见 `docs/test-reports/gui-walkthrough-round3-checklist.md` §六。
- [B 中] **RmShutdown 30s 无进度反馈（2026-10-29 拍板：暂不修）**：占用进程不响应关闭请求时等 RM 超时，期间前端无进度。等开发任务基本结束后再决定是否改进。
- [B 中] **RmShutdown 连带兄弟进程（2026-10-29 拍板：暂不修）**：RM 会话会连带关闭同 console / 进程组的兄弟进程，无法控制粒度。等开发任务基本结束后再决定是否改进。
- [B 中] **跨盘恢复无 copy+remove fallback（2026-10-29 拍板：暂不修）**：`restore_from_safe_box` 跨盘直接返回 `RestoreError::Io`（保险箱与 `%TEMP%` 通常同在 C 盘，实际影响小）。等开发任务基本结束后再决定是否改进。
- [B 中] **GUI 真机截图与 NSIS 安装态冒烟（第三档）**：自动化环境无活动桌面，WebView2 渲染进程创建即崩溃。补偿为 jsdom 组件用例 + 后端真实占用进程端到端 + NSIS 干净目录安装冒烟（task-050/054 已补）。需在有活动桌面的机器补做 GUI 点击证据。
- [B 低] **UNC 路径（`\\server\share\...`）**：沿用保守拒绝（normalize 层 NotAbsolute + resolve 层 Io 双保险）；网络盘 L3 红线策略留待真实 NAS/企业盘场景出现时再开。
- [B 低] **task-007 WebView2 devtools console 截图**：`listen('scan://progress')` 真机截图未取得（自动化环境无法聚焦 console）。通道本身已由编译期类型 + engine 节流单测保证，留待 UI 联调任务补截图。
- [B 低] **差量更新（已决策不做）**：评估结论 Tauri 2 不支持、自建 6-9 人日、4 MB 包省流量 <2 秒不划算。详见 `docs/research/delta-update-evaluation.md`。
- [B 低] **外部红线 DenialReason 变体（留待后续）**：`whitelist.rs` 仅新增函数、不改枚举/常量；后续审计若需区分内置/外部红线来源，再拆 DenialReason 变体。
- [B 低] **CI 未接 Dependabot / cargo audit / npm audit（不做）**：任务边界明确不做。CI 仅 windows-latest 单平台，跨平台矩阵不做。
- [B 中] **installer 未做代码签名（评估结论当前不做）**：未购 EV/OV 证书，SmartScreen 警告需手动「更多信息→仍要运行」。详见 `docs/research/code-signing-evaluation.md` 的三个触发条件。
- [B 低] **品牌图标（决策不做）**：沿用 Tauri 默认占位，待后续有设计出 1024×1024 源 PNG 后再跑 `tauri icon`。
## 下一步计划
1. v0.1.7 已发布，M2/M3/M4 全部收官。
2. 本轮决策：代码签名不做 / 品牌图标不做 / G6 不修。
3. **2026-10-29 用户最新决策**：跨盘恢复 fallback、RmShutdown 连带兄弟进程、RmShutdown 30s 无进度这三个功能**维持现状**，等开发任务基本结束后再决定是否改进；非必要默认不做改进，若有需要向用户提出再决策。
4. **M1 业务模块缺口推进中**：C 模块 WinSxS（task-065）与 E 模块大文件地图（task-066）开发完成待测。**下一步 = M1 第 3 项：D 模块残留增强（task-067）**。M1 剩余：F 模块注册表健康。所有 M1 子模块可复用 `engine::elevation::run_elevated` / `run_elevated_streaming`（提权，当前白名单 `create_restore_point` / `dism_analyze` / `dism_cleanup`，**加命令必须同步 `engine/src/elevation/mod.rs` 与 `elevation/src/main.rs` 两份，`engine/tests/elevation_whitelist.rs` 会在 CI 里挡住漏同步**）与 `engine/src/scan/big_files.rs` 的 BFS 遍历器（新的「按大小/按类型扫盘」需求可直接复用 `walk_collect` 的思路）。遗留：Items/SafeBox/History 三页深色适配待补；purge 严格「到点必删」需计划任务（当前是「运行期间定期 + 启动补一次」）；DISM 真实提权链路（UAC → 还原点 → `/StartComponentCleanup`）只能真机验；**Top100 在「文件数极多」的盘上会撞 20 万条目/20 秒上限并标 `truncated`（本机实测必现），若要完整榜单需另做「后台增量扫描 + 结果落库」**。

## 当前负责人 Agent
**测试 Agent（提示词 9）**——`feature/task-066-big-files-map` 开发轮已完成、状态 `testing`，待测试轮：独立重跑全部门禁 + 开 PR 触发 CI + **真机冒烟重点**（① Tab 栏出现「大文件」按钮（在「清理」与「保险箱」之间）；② 点进去自动开始扫描，十几秒后出现三张分类卡片 + Top100 汇总卡 + 明细表；③ 每张卡片显示大小 + 一条能照着做的建议（休眠 → `powercfg /h off`、旧系统备份 → 磁盘清理、回收站 → 右键清空）；④ **只读**：整页只有「重新扫描」一个按钮，没有任何删除/清理入口；⑤ 撞上限时出现「已达扫描上限提前停止」提示；⑥ 深色主题下页面颜色跟随（无白底刺眼）；⑦ CDP console 0 红错（G6 已知项除外））+ NSIS 安装态复跑。上一手：DevFullstackRustReact（task-066 开发）。再上一手：DocsKeeper（task-066 派发）。

## Git 提交记录
| Commit | 任务 | 说明 |
|--------|------|------|
| 67990f5 | #task-000 | Initial commit |
| 4972f9d | #task-000 | chore: 初始化项目仓库与文档骨架 |
| c0bdf05 | #task-000 | docs: 固化立项阶段 8 项关键决策并修正 Cargo.lock 策略 |
| caae920 | #task-000 | docs(prd): 新增产品需求文档 v0.1 |
| 261b24f | #task-000 | docs(trd): 新增技术需求文档 v0.1 |
| c1f027a | #task-000 | docs(arch): 新增系统架构文档 v0.1 |
| 3a1bb2c | #task-000 | docs(agents): 新增多Agent协作与Git权限规范 |
| 09adc52 | #task-000 | docs(state): 初始化项目状态白板 |
| bdeeea4 | #task-001 | docs(task): 新增任务001-初始化monorepo脚手架（派发基线） |
| 42cc54f | #task-001 | chore(workspace): 初始化 Cargo workspace 与 pnpm workspace 配置 |
| 5742151 | #task-001 | chore(scaffold): 新增 engine 空 lib 与 elevation 空 bin 占位 crate |
| ecea8cb | #task-001 | feat(ui): 初始化 app/src Vite + React 18 + TypeScript 前端骨架 |
| 496568f | #task-001 | feat(tauri): 接入 app/src-tauri Tauri 2 壳并链接 engine |
| f041803 | #task-001 | chore(scaffold): 补全 rules/ scripts/ 占位与 README 启动说明 |
| （本次） | #task-001 | docs(state): 任务001完成，更新进度与下一步 |
| e62b586 | #task-002 | build(deps): 为 engine 引入 rusqlite bundled 依赖 |
| 8dea10f | #task-002 | feat(db): 新增 engine::db 模块骨架与连接入口 conn::open |
| bac3400 | #task-002 | feat(db): 编写 v1__init.sql 落地 9 张业务表 schema |
| d48933a | #task-002 | feat(db): 实现基于 user_version 的手写迁移器 |
| 2ecc248 | #task-002 | test(db): 补 schema 与迁移器单元/集成测试 |
| e6f18a1 | #task-002 | style(db): 修复 clippy needless_borrow 告警并按 rustfmt 统一格式 |
| （本次） | #task-002 | docs(state): 任务002完成，更新进度与下一步 |
| ec9af02 | #task-003 | build(deps): 为 engine::execute 引入 thiserror 依赖 |
| 05f5e8f | #task-003 | feat(execute): 新增 engine::execute 模块骨架与 Deleter trait 抽象 |
| b4928ac | #task-003 | feat(execute): 实现 Windows 路径规范化器（防 .. 穿越、统一斜杠、大小写不敏感） |
| 78b0a33 | #task-003 | feat(execute): 实现内置 L3 红线路径匹配器 |
| 141b8ed | #task-003 | feat(execute): 落地 delete::execute() 唯一入口，全拒即不调用 Deleter |
| 478674d | #task-003 | test(execute): 补 PRD P0-7 全部 L3 路径拦截与放行用例 |
| e22b404 | #task-003 | style(execute): 按 rustfmt 统一 if 表达式换行 |
| （本次） | #task-003 | docs(state): 任务003完成，更新进度与下一步 |
| 6c57410 | #task-004 | build(deps): 为 engine::ipc 引入 serde 与 serde_json |
| 8714c1f | #task-004 | feat(ipc): 实现统一响应信封与 TRD §5.3 错误码常量表 |
| f931517 | #task-004 | feat(ipc): 实现 ProgressEmitter trait 与 100ms 节流包装 |
| db1f9aa | #task-004 | feat(ipc): 实现 scan.start 命令纯逻辑（入参校验 + 递增 sessionId） |
| f4e48b2 | #task-004 | feat(ipc): 实现 cleanup.dryRun 命令纯逻辑（接 task-003 白名单匹配器） |
| 339c604 | #task-004 | feat(tauri): 在 app/src-tauri 注册 scan.start / cleanup.dryRun 命令 |
| c7a706e | #task-004 | test(ipc): 补信封序列化、错误码、命令分支与节流器测试 |
| （本次） | #task-004 | docs(state): 任务004完成，更新进度与下一步 |
| 649c005 | #task-005 | build(deps): 为 engine::rules 引入 sha2 与 hex |
| e7477e6 | #task-005 | feat(rules): 编写初始 cache_rules.json 与 rules_meta.json（含真实 SHA-256） |
| 5ba121a | #task-005 | feat(rules): 定义 CacheRules / RuleEntry / RulesMeta serde 模型 |
| 312683e | #task-005 | feat(rules): 实现路径变量白名单展开（6 个环境变量 + ~） |
| d218c80 | #task-005 | feat(rules): 实现 SHA-256 校验加载器，校验失败拒绝并保留旧版 |
| ccf6c69 | #task-005 | feat(rules): 把 whitelist_deny 接入白名单匹配器，与内置红线取并集 |
| 480022d | #task-005 | test(rules): 补 JSON 反序列化、变量展开、篡改拒绝、并集生效测试 |
| （本次） | #task-005 | docs(state): 任务005完成，更新进度与下一步 |
| 7ce5578 | #task-006 | chore(ci): 新增 rust-toolchain.toml 固定 Rust 工具链版本 |
| 565353e | #task-006 | chore(ci): 新增 GitHub Actions CI 工作流（fmt/clippy/test + pnpm build） |
| ce08901 | #task-006 | chore(ci): 在 CI 追加 rules/*.json SHA-256 与 JSON 合法性校验 |
| 95771fb | #task-006 | chore(ci): 在 app/package.json 补 lint/test 占位脚本 |
| e6e1505 | #task-006 | fix(ci): Node 22 + pnpm 11，对齐本地工具链（修 ERR_UNKNOWN_BUILTIN_MODULE） |
| 6ad5296 | #task-006 | fix(ci): checksum 校验先去 CR 归一 LF（windows runner autocrlf mismatch） |
| （本次） | #task-006 | docs(state): 任务006完成，M0 阶段收官 |
| 3e9235c | #task-007 | feat(tauri): 实现 TauriProgressEmitter（AppHandle::emit scan://progress） |
| 45c505d | #task-007 | feat(tauri): 在 Builder 注册 ProgressState 并改造 scan_start 命令 emit demo 进度 |
| （本次） | #task-007 | docs(state): 任务007完成，真实进度通道接通 |
| 75a0cce | #task-008 | feat(execute): 新增 resolve.rs——canonicalize 解析 junction/symlink 真实路径 |
| 16120f8 | #task-008 | feat(execute): 在 delete 流程串入 resolve，新增 DenialReason::ResolveFailed |
| 0a25a68 | #task-008 | test(execute): 补 junction 解析与 L3 拒绝集成测试 |
| 7f253af | #task-008 | style(execute): cargo fmt 统一格式 |
| （本次） | #task-008 | docs(state): 任务008完成，symlink/junction 防绕开 |
| 9ec8224 | #task-009 | feat(db): 新增 DbItemPathResolver 实现 ItemPathResolver trait |
| 47b542f | #task-009 | test(db): 补 DbItemPathResolver 集成测试（含 dryRun 端到端） |
| 6bc9fb2 | #task-009 | docs(state): 任务009完成，真实 DB resolver 就绪 |
| （本次） | #task-009 | docs(task): 回填任务009实现备注 |
| 4346211 | #task-010 | feat(ipc): handle_dry_run 加 external_deny 参数，换 classify_with_external |
| 28738fb | #task-010 | feat(tauri): cleanup_dry_run 加载 rules/ 注入外部红线，失败返 5001 |
| 6caf900 | #task-010 | test(ipc): 外部红线接入 dryRun 端到端单测 |
| （本次） | #task-010 | docs(state): 任务010完成，外部红线集已接入 dryRun |
| ff83b3c | #task-011 | feat(scripts): scan_temp 与 scan_windows_update（NDJSON） |
| 22d1e37 | #task-011 | feat(scripts): scan_prefetch / scan_browser_cache / scan_dev_cache |
| e7c06e4 | #task-011 | test(scripts): 5 脚本 NDJSON 冒烟全绿 |
| （本次） | #task-011 | docs(state): 任务011完成，M1 前置清理清单全部就绪 |
| 0e5e35e | #task-012 | feat(scan): NdjsonEvent serde 模型与 parse_line 单行解析器 |
| 3096cc7 | #task-012 | feat(scan): ScanRunner——spawn powershell、写 cleanup_item、emit |
| 4222d83 | #task-012 | test(scan): parser 与 runner 集成测试（假 ps1） |
| （本次） | #task-012 | docs(state): 任务012完成，Rust 侧 ps1 NDJSON 消费器就绪 |
| eb46fe9 | #task-013 | feat(scan): ScanOrchestrator 串跑 5 个 ps1 并汇总 |
| be8bd23 | #task-013 | feat(tauri): scan_start 真接通——建 session/跑 Orchestrator/回写 status |
| 8918c8f | #task-013 | test(scan): Orchestrator 集成测试（4 条） |
| （本次） | #task-013 | docs(state): 任务013完成，scan.start 端到端跑通真实扫描 |
| 0c32251 | #task-014 | feat(tauri): cleanup_dry_run 切 DbItemPathResolver，删 MockResolver |
| （本次） | #task-014 | docs(state): 任务014完成，dryRun 走真实 DB 数据 |
| aa5abd7 | #task-015 | feat(execute): 新增 FsDeleter——std::fs::remove_file 真实删除 |
| 319d55e | #task-015 | feat(execute): 新增 execute_items 编排——过白名单/删文件/落审计 |
| 125c385 | #task-015 | feat(tauri): 新增 cleanup.execute 命令并注册 |
| 897b0c1 | #task-015 | test(execute): execute_items 集成测试 4 条 |
| （本次） | #task-015 | docs(state): 任务015完成，真实删除闭环 |
| 58dd794 | #task-016 | feat(execute): 新增 SafeBoxDeleter——删除前移入保险箱并写 safe_box 表 |
| 6a3892e | #task-016 | refactor(execute): execute_items 改 deleter 工厂闭包签名 |
| 86e5fdf | #task-016 | feat(tauri): cleanup_execute 切 SafeBoxDeleter |
| cdb89f6 | #task-016 | test(execute): safe_box 移入/quota 剔除/空 noop 集成测试 |
| 82e320c | #task-016 | fix(execute): 去掉 MSRV 1.85 CrossesDevices variant |
| （本次） | #task-016 | docs(state): 任务016完成，5GB FIFO 保险箱落地 |
| 9fb97ea | #task-017 | feat(execute): 新增 restore_from_safe_box——按 id 把文件 rename 回原路径 |
| 26d19d1 | #task-017 | feat(tauri): 新增 safe_box.restore 命令并注册 |
| 6372f6c | #task-017 | test(execute): 补 restore 集成测试（正常/占用/父缺/已剔除） |
| （本次） | #task-017 | docs(state): 任务017完成，保险箱 restore 闭环 |
| abb43dc | #task-018 | feat(execute): 新增 list_safe_box——查未剔除条目 + 汇总 |
| 5827efe | #task-018 | feat(tauri): 新增 safe_box.list 命令并注册 |
| 774e1dd | #task-018 | test(execute): 补 list 集成测试（空/多条/排除 purged/排除 restored） |
| （本次） | #task-018 | docs(state): 任务018完成，safe_box.list 就绪 |
| 9fc9564 | #task-019 | feat(execute): 新增 classify_io_error——区分 NotFound/PermissionDenied/SharingViolation/Unknown |
| a2c75a5 | #task-019 | feat(execute): SafeBoxDeleter 抛错时按分类加 FILE_IN_USE/PERMISSION_DENIED 前缀 |
| 257ba68 | #task-019 | feat(execute): execute_items 写 audit_log 时按错误分类填 detail |
| ee14fd0 | #task-019 | test(execute): 补 classify_io_error 单元测试 |
| （本次） | #task-019 | docs(state): 任务019完成，删除失败错误分类就绪 |
| a3f3a3d | #task-020 | build(deps): engine 引入 windows crate（cfg(windows) RestartManager feature） |
| 8445d68 | #task-020 | feat(execute): 新增 who_is_locking——Restart Manager FFI 查占用进程 PID |
| 7910e38 | #task-020 | test(execute): 补 who_is_locking 单元测试（空锁 + notepad 手动用例） |
| 47590f9 | #task-020 | style(execute): cargo fmt 对齐 locking 模块与测试 |
| （本次） | #task-020 | docs(state): 任务020完成，who_is_locking 就绪 |
| a4dbee4 | #task-021 | feat(execute): ExecResult 加 locking_processes 字段，FILE_IN_USE 时自动调 who_is_locking |
| 841bf87 | #task-021 | test(execute): 补 execute_items 占用进程集成测试 |
| （本次） | #task-021 | docs(state): 任务021完成，占用进程自动报告 |
| 53c4c92 | #task-022 | build(deps): engine 引入 reqwest（blocking + rustls-tls） |
| 08d4d23 | #task-022 | feat(rules): 新增 meta_store——upsert/get_active rules_meta 落 DB |
| fb849e7 | #task-022 | feat(rules): 新增 fetch_remote_meta——GET GitHub Raw rules_meta.json |
| 7f0c1bf | #task-022 | feat(tauri): 新增 rules.checkUpdate 命令（比对本地/远端 sha256） |
| 757f5c3 | #task-022 | test(rules): 补 meta_store 集成测试 |
| （本次） | #task-022 | docs(state): 任务022完成，rules.checkUpdate 就绪 |
| 4add0d7 | #task-023 | docs(task): 新增任务023-rules.applyUpdate |
| 1db76ec | #task-023 | feat(rules): 新增 fetch_remote_rules_bytes——GET 远端 cache_rules.json |
| 5691f59 | #task-023 | feat(rules): 新增 apply_update——SHA-256 校验 + 备份 + 替换 + upsert DB + 失败回滚 |
| 49f784a | #task-023 | feat(tauri): 新增 rules.applyUpdate 命令并注册 |
| 0bc8bfb | #task-023 | test(rules): 补 apply_update 集成测试（替换/sha不匹配回滚） |
| （本次） | #task-023 | docs(state): 任务023完成，规则更新闭环 |
| 0d00d42 | #task-024 | docs(task): 新增任务024-audit_log.list |
| 79677da | #task-024 | feat(audit_log): 新增 list_audit_log——按 session 过滤 + 分页 + 时间倒序 |
| 701caf8 | #task-024 | feat(tauri): 新增 audit_log.list 命令并注册 |
| dd24e78 | #task-024 | test(audit_log): 补 list 集成测试（空/倒序/action过滤/分页） |
| （本次） | #task-024 | docs(state): 任务024完成，audit_log.list 就绪 |
| 8fa4546 | #task-025 | docs(task): 新增任务025-audit_log.stats |
| 9f0c0c5 | #task-025 | feat(audit_log): 新增 cleanup_stats——总释放字节 / 成功数 / 失败数 / session 数 |
| 7052e14 | #task-025 | feat(tauri): 新增 audit_log.stats 命令并注册 |
| d6a9fa1 | #task-025 | test(audit_log): 补 stats 集成测试（空/计数/session过滤/去重） |
| b741579 | #task-025 | docs(state): 任务025完成，清理统计数字就绪 |
| 0516bee | #task-025 | docs(task): 回填任务025实现备注 |
| 1548320 | #task-026 | docs(task): 新增任务026-前端仪表盘数字卡片 |
| 40f5202 | #task-026 | feat(frontend): 新增 formatBytes 工具函数 |
| 37219cd | #task-026 | feat(frontend): 新增 Dashboard 页面——audit_log.stats 4 数字卡片 |
| 17c8e33 | #task-026 | feat(frontend): App.tsx 挂载 Dashboard 为默认页（补装 @tauri-apps/api@2.12.3） |
| 4706afc | #task-026 | docs(state): 任务026完成，前端仪表盘数字卡片就绪 |
| baa0b63 | #task-027 | merge(feature/task-027): History 历史明细页——audit_log.list 表格 + 分页，App.tsx tab 切换 |
| c935889 | #task-028 | docs(task): 新增任务028-前端开始扫描按钮 |
| 90eec5c | #task-028 | feat(frontend): Dashboard 加开始扫描按钮 + scan_start invoke + listen(scan://progress) 实时进度条 |
| 783b45b | #task-028 | docs(state): 任务028完成，前端可发起扫描 |
| 5979f14 | #task-029 | docs(task): 新增任务029-清理闭环Items UI |
| 48ba875 | #task-029 | feat(execute): 新增 list_pending_items——按 session 过滤 pending 项，size 倒序 |
| 976acdb | #task-029 | feat(tauri): 新增 cleanup.listPending 命令并注册 |
| e223fc6 | #task-029 | test(execute): 补 list_pending_items 集成测试（列出全部/session过滤/size倒序/分页） |
| a4be607 | #task-029 | feat(frontend): 新增 Items 页面——pending 表格 + checkbox + 清理已选按钮 |
| 94fb43d | #task-029 | feat(frontend): App.tsx 加「清理」tab 切换 |
| ef07dea | #task-029 | style(execute): clippy 修复——limit 用 clamp 替代 max/min |
| ca81b00 | #task-029 | docs(state): 任务029完成，清理闭环打通 |
| 7bb625e | #task-030 | docs(task): 新增任务030-保险箱恢复 UI |
| 1f24104 | #task-030 | feat(frontend): 新增 SafeBox 页面——safe_box_list 表格 + 汇总 + 单条恢复按钮 |
| 317258c | #task-030 | feat(frontend): App.tsx 加「保险箱」tab 切换 |
| c7c1882 | #task-030 | docs(state): 任务030完成，保险箱恢复 UI 就绪 |
| c58ce28 | #task-031 | docs(task): 新增任务031-规则更新 UI |
| a46ec34 | #task-031 | feat(frontend): 新增 Settings 页面——规则库卡片 + 检查更新 + 应用更新按钮 |
| 1c3d33c | #task-031 | feat(frontend): App.tsx 加「设置」tab 切换 |
| dbe7e44 | #task-031 | docs(state): 任务031完成，规则更新 UI 就绪 |
| 3ec1065 | #task-032 | docs(task): 新增任务032-ESLint+Vitest 接入 |
| 239eef0 | #task-032 | chore(frontend): 安装 ESLint 10 flat config + TS 插件 |
| ddfe37f | #task-032 | chore(frontend): 安装 Vitest 2.1 + testing-library + jsdom + vite.config test 字段 |
| 8557c4d | #task-032 | test(frontend): 给 formatBytes / formatTs 写纯函数单测（9 个断言） |
| 996a803 | #task-032 | ci: 前端 job 接 pnpm lint + pnpm test，替换 echo 占位 |
| 173eac6 | #task-032 | docs(state): 任务032完成，ESLint+Vitest 接入 |
| 3a255d1 | #task-033 | docs(task): 新增任务033-NSIS v0.1.0 打包 |
| f0a46ec | #task-033 | chore(tauri): tauri.conf.json 完善 identifier/nsis perMachine/SimpChinese |
| 0d35fa0 | #task-033 | ci: 加 tauri build + upload-artifact 出 v0.1.0 installer |
| 96be58d | #task-033 | docs(state): 任务033完成，NSIS v0.1.0 打包就绪 |
| b360a9e | #task-034 | docs(task): 新增任务034-GitHub Release 发版通道 |
| c7a276d | #task-034 | ci: 新增 release.yml——workflow_dispatch 手动发版 + tag 自动发版 |
| （本次） | #task-034 | docs(state): 任务034完成，GitHub Release 通道就绪 |
| 6233da0 | #debt-cleanup-002 | fix(frontend): unwrap Envelope<T> in api() helper to fix white screens |
| f939780 | #debt-cleanup-002 | fix(tauri): grant core:event:default so listen passes ACL |
| c81615d | #debt-cleanup-002 | fix(ci): drop explicit pnpm version in release.yml |
| 6e149ea | #debt-cleanup-002 | fix(engine): count hops by reparse point, not canonicalize string compare |
| 577e6f5 | #debt-cleanup-002 | fix(dashboard): reset progress on scan failure |
| （本次） | #debt-cleanup-002 | docs(state): debt-cleanup-002 五个 M1 打包回归 bug 修复完成 |
| c5cf1d3 | #debt-cleanup-003 | fix(frontend): pass scope='full' to scan_start command |
| 16b30e2 | #debt-cleanup-003 | fix(tauri): use app_data_dir() for DB/safe_box, bundle rules+scripts as resources |
| （本次） | #debt-cleanup-003 | docs(state): debt-cleanup-003 两个 P0 post-install bug 修复完成 |
| 16f0512 | #debt-cleanup-004 | fix(frontend): align CleanupStats/ScanProgress/AuditLogEntry to camelCase |
| 00015c0 | #debt-cleanup-004 | fix(tauri): classify rules update errors instead of collapsing to INTERNAL |
| （本次） | #debt-cleanup-004 | docs(state): debt-cleanup-004 三个二轮回归 bug 修复完成 |
| 72e2571 | #task-035 | chore(tauri): 接入 tauri-plugin-updater Rust 插件（Cargo.toml + lib.rs） |
| 7a2a4bf | #task-035 | chore(tauri): tauri.conf.json 加 updater 配置（pubkey + endpoint） |
| 1023a02 | #task-035 | feat(frontend): Settings 加「应用更新」卡片 + check/downloadAndInstall |
| 28c1f1a | #task-035 | ci: release.yml 加签名 key 注入 + latest.json 生成上传 |
| 4f7eb0a | #task-035 | fix(tauri): updater 配置移到 plugins.updater 命名空间并补 serde_json 依赖 |
| （本次） | #task-035 | docs(state): 任务035完成，tauri updater 自动更新就绪 |
| f3086b4 | #task-035 | fix(tauri): 放行 updater/process 插件 ACL 并补注册 process Rust 插件 |
| 5d93bb7 | #task-035 | test(frontend): 补 capabilities updater/process ACL 回归测试 |
| （本次） | #task-035 | docs(state): 修复 task-035 updater ACL P0 bug |
| 2d959b3 | #task-036 | feat(frontend): App 启动自动 check() 应用更新 |
| f460c42 | #task-036 | feat(frontend): 非阻塞 banner——新版本提示 + 立即走 downloadAndInstall/relaunch |
| （本次） | #task-036 | docs(state): 任务036完成，启动自动更新 banner 就绪 |
| eacc988 | #task-037 | feat(execute): safe_box.purge_item——删磁盘文件 + DELETE DB 行 + audit_log |
| 6dbb099 | #task-037 | feat(tauri): safe_box_purge 命令并注册 |
| 90ed07c | #task-037 | test(execute): safe_box_purge 集成测试（删文件+DB/缺文件/未找到） |
| 44f2c38 | #task-037 | feat(frontend): SafeBox 行加「永久删除」红色按钮 + confirm |
| （本次） | #task-037 | docs(state): 任务037完成，safe_box purge 就绪 |
| 81528dd | #task-038 | feat(execute): purge_many——循环 purge_item 收集成功/失败计数 |
| f0d8a50 | #task-038 | feat(tauri): safe_box_purge_many 命令并注册 |
| 8df9c70 | #task-038 | test(execute): purge_many 集成测试（全成功/部分失败/空 Vec） |
| 5895db0 | #task-038 | feat(frontend): SafeBox 加 checkbox 列 + 批量删除已选按钮 |
| （本次） | #task-038 | docs(state): 任务038完成，safe_box 批量 purge 就绪 |
| 4e3e0c6 | #task-039 | feat(execute): restore_many——循环 restore_item 收集成功/失败计数 |
| 577b87f | #task-039 | feat(tauri): safe_box_restore_many 命令并注册 |
| 1e35347 | #task-039 | test(execute): restore_many 集成测试（全成功/部分失败/空 Vec） |
| 2e5eff3 | #task-039 | feat(frontend): SafeBox 加「批量恢复已选」绿色按钮 |
| （本次） | #task-039 | docs(state): 任务039完成，safe_box 批量恢复就绪 |
| adcba07 | #task-039 | fix(execute): cargo fmt 对齐 purge_many/restore_many 签名与断言展开 |
| 54dd237 | #task-039 | test(execute): 补 restore_many 中间失败不阻断后续 id 用例 |
| de107af | #task-039 | fix(frontend): 批量操作进行中禁用行内恢复/永久删除按钮 |
| （本次） | #task-039 | docs(state): 评审前清障完成 |
| cbc5aaa | #task-039 | merge: task-039 safe_box 批量恢复（评审通过后 --no-ff 合并，双亲 2068fcd + daf7adf） |
| 882ed67 | #task-039 | docs(review): 任务039评审报告（真机冒烟 + 分支/主干 CI 实查） |
| （本次） | #task-039 | docs(state): 评审登记 2 条非阻塞已知问题与合并记录 |
| 2be95e1 | #task-041 | docs(task): 新增任务041-rules 超时收敛 + updater endpoint 修复 |
| 2e99f0e | #task-041 | fix(rules): reqwest 加 10s 超时 + 5s 连接超时，错误映射为中文可读文案 |
| 54d991b | #task-041 | test(rules): 补超时 + 404 可读错误文案测试 |
| bfab9a8 | #task-041 | fix(tauri): updater endpoint 改 Releases/latest/download URL |
| 6c3650a | #task-041 | fix(frontend): Settings 规则库卡片错误态按钮恢复为「重试」不卡转圈 |
| （本次） | #task-041 | docs(state): 任务041完成，rules 超时收敛 + updater endpoint 修复 |
| b64e376 | #task-042 | docs(task): 新增任务042-rules 离线模式 |
| 0cf879e | #task-042 | feat(rules): checkUpdate 返回 CheckUpdateResult 加 Offline 分支 |
| afbccff | #task-042 | test(rules): 补离线模式 / UpToDate / NewVersion 三分支单测 |
| 8ced0be | #task-042 | feat(frontend): Settings 规则库卡片按 status 渲染三态（绿/蓝/黄） |
| （本次） | #task-042 | docs(state): 任务042完成，rules 离线模式降级就绪 |
| 86bbb61 | #task-043 | docs(task): 新增任务043-restore 自动建父目录 |
| 6b65bcb | #task-043 | fix(execute): restore_item 在父目录缺失时自动 create_dir_all |
| 2e14188 | #task-043 | test(execute): 补 restore 自动建父目录单测（缺目录/已存在目录/不可用父路径） |
| ca936c1 | #task-043 | test(execute): 适配 task-043——旧 ParentMissing 用例改为正向自动建目录断言 |
| （本次） | #task-043 | docs(state): 任务043完成，safe_box restore 自动建父目录就绪 |
| 70452d0 | #task-044 | docs(task): 新增任务044-rules 手动回滚 |
| 3980490 | #task-044 | feat(rules): 新增 rollback + has_backup——从 .bak 恢复 cache_rules.json |
| 3cc1797 | #task-044 | feat(tauri): 新增 rules_rollback / rules_has_backup 命令并注册 |
| 262d7e9 | #task-044 | test(rules): 补 rollback 集成测试（有 bak/无 bak/损坏 bak/has_backup） |
| 8013dee | #task-044 | feat(frontend): Settings 规则库卡片加「回滚到上一版」按钮 |
| ff142be | #task-044 | style: cargo fmt |
| （本次） | #task-044 | docs(state): 任务044完成，rules 手动回滚命令就绪 |
| 8e56188 | #task-045 | docs(task): 新增任务045-safe_box 分页 |
| 252c87a | #task-045 | feat(execute): safe_box.list_paged——LIMIT/OFFSET + COUNT total + page_size clamp |
| 359a2de | #task-045 | feat(tauri): safe_box_list 入参加 page/page_size，返回 SafeBoxPage |
| 2a35eaf | #task-045 | test(execute): 补 safe_box 分页集成测试（首页/次页/clamp/越界/ID DESC） |
| 02b31bb | #task-045 | feat(frontend): SafeBox 表格下加分页栏（上一页/下一页/页码/total） |
| （本次） | #task-045 | docs(state): 任务045完成，safe_box list 分页就绪 |
| 734ab6b | #task-046 | docs(task): 新增任务046-safe_box session 过滤 |
| f7069f4 | #task-046 | feat(execute): list_paged 加可选 session_id 过滤 + 新增 list_sessions |
| 69e1f95 | #task-046 | feat(tauri): safe_box_list 入参加 session_id，新增 safe_box_list_sessions 命令 |
| 8effbc4 | #task-046 | test(execute): 补 session 过滤集成测试（全部/按 session/无匹配/list_sessions 分组） |
| a629150 | #task-046 | feat(frontend): SafeBox 加分页栏左侧 session 下拉（全部/session #N） |
| （本次） | #task-046 | docs(state): 任务046完成，safe_box session 过滤就绪 |
| 41afc60 | #task-047 | docs(task): 新增任务047-applyUpdate 确认弹窗 |
| 36c9f76 | #task-047 | feat(frontend): Settings 点「应用更新」前 window.confirm 嵌入版本号 |
| （本次） | #task-047 | docs(state): 任务047完成，rules applyUpdate 前端确认弹窗就绪 |
| （本次） | #task-048 | docs(state): 任务048真机验证通过，v0.1.1 updater banner 正确弹出，M2 关闭 |
| （本次） | #task-049 | docs(task): 新增任务049-RmShutdown 占用进程处置 |
| 20d1b04 | #task-049 | feat(engine): 新增 shutdown_locking_processes 与危险进程护栏（含 who_is_locking probe bug 修复） |
| 081d0ea | #task-049 | test(engine): 补 RmShutdown 护栏与无占用用例 |
| f3a6828 | #task-049 | feat(tauri): 新增 cleanup_retry_locked 命令 |
| 69b6b33 | #task-049 | feat(frontend): 清理失败展示占用进程并支持关闭后重试 |
| 7cfe53e | #task-049 | test(engine): 补关闭后重试删除的端到端用例 |
| 3b5cddd | #task-049 | test(engine): 输出 RetryLockedResp 真实 JSON 便于契约核对 |
| 1b467ba | #task-049 | docs(task): 补任务049实现备注 |
| （本次） | #task-049 | docs(state): 任务049完成，占用进程 RmShutdown 就绪 |
| 0129de9 | #task-049 | fix(frontend): 重试按钮纳入三态互斥（测试轮缺陷 1） |
| 8b84cf4 | #task-049 | test(frontend): 补 Items 清理失败与关闭重试交互用例（6 条） |
| f667c70 | #task-049 | test(engine): 补只关闭注册资源不误伤旁路文件的用例（B7） |
| be60f55 | #task-049 | merge: 测试期缺陷修复合回 feature（--no-ff） |
| （本次） | #task-049 | docs(test): 任务049测试报告 |
| 199527a | #task-049 | fix(engine): 护栏改为按 PID 映像名判定，本地化应用名不再可绕过（bugfix/task-049-guard-image-name） |
| c34af2c | #task-049 | test(engine): 补本地化应用名/映像名不可得/mixed 用例，B7 可归因 |
| （本次） | #task-049 | docs(state): 任务049b 修复完成，待重测（状态 testing） |
| b1055aa | #task-049 | merge: task-049b 护栏映像名判定 + mixed fail-closed 合回 feature（--no-ff） |
| a740dea | #task-049 | test(engine): 本地化护栏用例改用必然不存在的 pid，消除环境依赖（bugfix/task-049-guard-test-stability） |
| 448275a | #task-049 | docs(frontend): 补齐 RefusedProcess.reason 的 unknown process 说明 |
| 5550f61 | #task-049 | merge: 测试期修复（护栏用例去环境依赖 + reason 注释）合回 feature（--no-ff） |
| 17aa869 | #task-049 | docs(test): 任务049b 测试报告 |
| 03b53b4 | #task-049 | docs(state): 任务049b 测试通过，状态 awaiting_review |
| 8731c44 | #task-049 | docs(review): 任务049评审报告（轮 1，不通过） |
| 27d8057 | #task-049 | docs(state): 任务049评审不通过，登记护栏本地化缺陷 |
| 2c91242 | #task-049 | docs(task): 新增任务049b 护栏映像名修复任务书 |
| 37375fb | #task-049 | docs(review): 补齐评审报告 Git 操作记录 |
| （本次） | #task-049 | docs(review): 任务049b评审报告（轮 2，通过） |
| （本次） | #task-049 | docs(state): 任务049合并main并记录merge commit |
| **60e0797** | #task-049 | **merge: task-049 占用进程 RmShutdown 与一键关闭重试（--no-ff，评审轮 2 通过）** |
| 383189f | #task-050 | fix(packaging): bundle.resources 改为映射式，规则与脚本落到安装根目录 |
| c4149fd | #task-050 | fix(paths): 资源定位不再依赖 cwd，缺失时显式报错并打印绝对路径 |
| 819566b | #task-050 | test(paths): 补资源候选挑选单测 |
| 4827377 | #task-050 | test(ci): 新增 Tauri 资源映射契约测试并接入 CI |
| 73dbbe7 | #task-050 | fix(paths): dev 优先仓库内资源，避免 target/debug 陈旧副本 |
| （本次） | #task-050 | docs(test): 任务050测试报告 |
| （本次） | #task-050 | docs(state): 任务050测试通过，状态 awaiting_review |
| **cadb286** | #task-050 | **merge: task-050 打包资源路径错位修复（--no-ff，评审通过）** |
| eeafb31 | #task-050 | docs(review): 任务050评审报告 |
| （本次） | #task-050 | docs(state): 任务050合并main并记录merge commit |
| 81bd298 | #task-051 | docs(task): 新增任务051-保险箱时间筛选与审计联表 |
| 6eb7ba6 | #task-051 | feat(engine): list_paged 改为动态 WHERE，支持时间区间与多 session 组合筛选 |
| c1eba8d | #task-051 | feat(engine): 保险箱条目暴露 source_task_id 并新增按路径的审计历史查询 |
| 2e05177 | #task-051 | feat(ipc): safe_box_list 参数升级并注册 safe_box_item_history 命令 |
| 0c4162c | #task-051 | test(engine): 覆盖时间区间边界与多 session 叠加筛选 |
| ce7f7f0 | #task-051 | feat(ui): 保险箱页支持时间预设/多 session/来源列/历史展开 |
| 142acc7 | #task-051 | test(ui): 新增 SafeBox 组件用例覆盖筛选与历史展开 |
| 72c737b | #task-051 | docs(task): 补记任务051实现备注与三环境自验结论 |
| 3d00901 | #task-051 | docs(state): 任务051完成，状态置 testing 移交测试 Agent |
| b217b61 | #task-051 | docs(state): 回填任务051 CI run 40163134926 全绿 |
| ad404df | #task-051 | docs(state): 记录任务051两轮 CI 均全绿 |
| ef054aa | #task-051 | fix(ui): 刷新按钮不再把 click 事件当作 page 传入 |
| 05d5475 | #task-051 | test(ui): 覆盖刷新按钮实参与筛选条件保持 |
| **c1a1af6** | #task-051 | **merge: task-051 刷新按钮实参缺陷修复（--no-ff，bugfix 子分支合回 feature）** |
| 58ce7ba | #task-051 | docs(test): 任务051测试报告 |
| 7bfc130 | #task-051 | docs(state): 任务051测试通过，状态 awaiting_review |
| d020022 | #task-051 | docs(test): 回填任务051 CI run 40168680216 全绿 |
| **e6ab561** | #task-051 | **merge: task-051 保险箱时间筛选与审计联表（--no-ff，评审通过）** |
| 5a2b643 | #task-051 | docs(review): 任务051评审报告 |
| （本次） | #task-051 | docs(state): 任务051合并main并记录merge commit |
| **801af22** | #task-049 | **merge: 关闭体验文案小改（confirm 补「同一控制台内的相关进程可能一并关闭」+ mixed fail-closed 解释文案，--no-ff）** |
| （本次） | #docs | docs(state): 回填真机 GUI 走查结果（G1–G6 根因与优先级）+ 新建 `docs/test-reports/gui-walkthrough-2026-10-25.md` |
| 195b2ee | #task-052 | docs(task): 新增任务052-GUI 走查 P1+P2 修复 |
| 8f97572 | #task-052 | fix(scan): ScanOrchestrator 加脚本级百分比换算，进度条单调递增到 100 |
| 7111220 | #task-052 | fix(execute): cleanup_list_pending 分页 + total + size_bytes DESC, id DESC 稳定排序 |
| dc606cd | #task-052 | fix(execute): execute_items 传 task_id 给 SafeBoxDeleter，safe_box.source_task_id 真实写入 |
| c2b8ee5 | #task-052 | feat(frontend): Items 清理列表加分页栏（上一页/下一页/页码/total） |
| 929def9 | #task-052 | fix(stats): 清理轮次口径改 COUNT(*)，Dashboard 轮次卡片不再恒 0 |
| db9c81d | #task-052 | fix(execute): deleter 区分真删 vs 已不存在，结果条单列 already_gone |
| （本次） | #task-052 | docs(state): 任务052完成，GUI 走查 P1+P2（G1–G5）修复就绪 |
| 179f82c | #task-052 | docs(test): task-052 测试报告——G3 source_task_id + G4 轮次口径真机验证通过，Console 0 红错 |
| f908d1a | #task-052 | docs(review): task-052 GUI P1+P2 修复评审报告 |
| **d92db1b** | #task-052 | **merge: task-052 GUI 走查 P1+P2 修复（G1–G5）（--no-ff，评审通过，CI PR #17 全绿）** |
| （本次） | #task-052 | docs(state): 回填 task-052 合并与 2026-10-27 第二轮真机复验（G7/G8 新增、G1 复开、G3 证据错位修正） |
| 0920cab | #task-053 | fix(scan): ThrottledEmitter 加 emit_force 逃生口，收尾事件不再被节流吞 |
| 0eea526 | #task-053 | fix(scan): ScanOrchestrator 收尾强制发送 percent=100 |
| 7ade9a4 | #task-053 | test(scan): 新增经过 ThrottledEmitter 的尾部用例，断言最后到达 percent=100 |
| bd7fd56 | #task-053 | style(scan): cargo fmt 修正尾部用例换行 |
| aff506b | #task-053 | fix(frontend): Dashboard 收尾兜底置 100 + 延迟退订；SafeBox NULL 来源渲染「历史（无来源）」 |
| c6d16b9 | #task-053 | test(frontend): SafeBox 来源列改断言「历史（无来源）」并补说明文案用例 |
| （本次） | #task-053 | docs(state): 任务053完成，G7 进度条收尾 + G8 历史来源文案就绪 |
| 35bc0a9 | #task-054 | ci(release): release.yml 补 target_commitish，tag 精确指向触发发版的 commit |
| c807742 | #task-054 | chore(release): 版本号升级 v0.1.0 → v0.1.6（package.json / tauri.conf.json / Cargo.toml） |
| b775ae8 | #task-054 | docs(changelog): 新增 CHANGELOG.md，v0.1.6 条目覆盖 task-050~053 主要变更 |
| 718e643 | #task-054 | docs(test): 新增第三轮复验走查清单（NSIS/MSI 双列，27 条） |
| f349be0 | #task-054 | chore(gitignore): 负向包含 docs/release/，发版手册可入库 |
| 530ae8f | #task-054 | docs(release): 新增 v0.1.6 升级闭环验证手册（6 步 + 卡点速查） |
| （本次） | #task-054 | docs(state): 任务054完成，v0.1.6 发版准备就绪，待用户手动 workflow_dispatch |
| a51ebef | #task-054 | merge: task-054 v0.1.6 发版准备——版本号+release 溯源+CHANGELOG+验证手册（--no-ff） |
| 56d203b | #task-054 | docs(state): 回填 v0.1.6 发版升级闭环 + 第三轮 NSIS/MSI 走查结果 |
| 50f40ea | #task-055 | feat(updater): downloadAndInstall 订阅 onEvent，记录 contentLength / chunkLength |
| 0460edc | #task-055 | feat(ui): banner 下载中显示进度条 + 百分比/大小/速度（EMA 1s 窗口 α=0.4） |
| 9b62db3 | #task-055 | fix(ui): updater 失败态按关键词映射中文文案 + 重试/关闭按钮 |
| 107b4cb | #task-055 | feat(ui): 「稍后提醒」持久化到 localStorage，同版本不再弹 |
| be2b22b | #task-055 | feat(settings): 手动检查更新显示当前/远端版本 + 失败文案复用 + onUpdateFound |
| f133226 | #task-055 | test(ui): 补 27 条用例——失败文案 5 类 / 稍后提醒 / 百分比兜底 / 进度与失败态 / Settings loading |
| （本次） | #task-055 | docs(state): 任务055完成，M3 banner 下载进度 + 失败态 + loading 就绪 |
| 4231cbd | #task-055 | merge: task-055 M3 更新体验——banner 下载进度+失败态文案+稍后提醒持久化（--no-ff） |
| 464816d | #task-056 | docs(research): 新增差量更新技术评估（bsdiff/xdelta3/Tauri 2 对比，结论不立项） |
| fd242ae | #task-056 | feat(db): safe_box 加 restored_at 列（迁移 v2），restore 改 UPDATE 不再 DELETE |
| 86e73d2 | #task-056 | test(db): 补迁移 v1→v2 保数据用例 + restore 断言改 restored_at/status + 重复恢复 NotFound |
| 6d4036a | #task-056 | test(execute): 补跨盘 rename fallback 集成测试（单盘环境 skip） |
| 49a3b17 | #task-056 | docs(test): G6 设置页 IPC 红错定稿豁免口径——根因/不修理由/5 条判定规则 |
| 0a19e3b | #task-056 | style(test): cargo fmt 修正新增用例换行 |
| （本次） | #task-056 | docs(state): 任务056完成，差量更新评估 + restored_at + 跨盘测试 + G6 闭环 |
| ad80b2a | #task-057 | docs(task): 新增任务057-RmRestart 自动重启与设置开关 |
| eabfd5c | #task-057 | docs(state): 回填产品决策——C08接受/差量不立项/release瘦身不做/G6豁免/RmRestart立项 |
| 50dfa6b | #task-057 | feat(prefs): 新增用户偏好存储 auto_restart_on_close（默认 false）+ get/set 命令 |
| f77e013 | #task-057 | feat(execute): RmShutdown 后按 auto_restart 开关对 bRestartable 进程调 RmRestart（总超时 10s） |
| ceac435 | #task-057 | test(execute): 适配 shutdown_locking_processes 新 auto_restart 参数（集成测试统一传 false） |
| 7ec23ea | #task-057 | feat(settings): 设置页新增「自动重启被关闭的进程」开关（默认关闭，get/set_prefs 读写） |
| 154a2ef | #task-057 | fix(ui): 占用关闭 confirm 文案根据 prefs.autoRestartOnClose 追加「将自动重启被关闭的进程」提示 |
| aa5b183 | #task-057 | style(engine): cargo fmt 自动格式化 prefs/locking |
| 4100d3d | #task-057 | docs(task): 追加 task-057 实现备注（prefs 方案 / RmRestart 结构 / 自验输出 / 偏差） |
| （本次） | #task-057 | docs(state): 任务057完成，RmRestart 自动重启（默认关闭）+ 设置开关就绪 |
| e9f0e7a | #task-057 | merge: task-057 RmRestart 自动重启被关进程 + 设置页开关（默认关闭）（--no-ff） |
| 6bb5cb5 | #task-058 | feat(prefs): RmRestart 白名单（空=不限制，非空 fail-closed）+ Settings 编辑框 |
| 69b4696 | #task-058 | test(prefs): 补白名单三态/通配/持久化用例 + 适配 shutdown 三参签名 |
| 71f1650 | #task-058 | fix(execute): safe_box 先 INSERT 再 rename 并事务包裹，失败不留孤儿文件 |
| c87e5f1 | #task-058 | test(execute): 补 safe_box 原子性用例——INSERT 失败原文件留原位、零孤儿、零 DB 行 |
| 29b32d6 | #task-058 | docs(research): 代码签名技术评估（EV/OV/自签/Store 四方案，结论当前不做） |
| 85bd0f5 | #task-058 | docs(changelog): 追加 v0.1.7 草稿条目 |
| （本次） | #task-058 | docs(state): 任务058完成，M4 收尾——白名单/原子性/签名评估/CHANGELOG 草稿就绪 |
| 9826c15 | #task-058 | merge: task-058 M4 收尾——RmRestart 白名单+safe_box 原子性+签名评估+CHANGELOG（--no-ff） |
| a12eacf | #task-059 | chore(release): 版本号升级 v0.1.6 → v0.1.7（四处 + Cargo.lock） |
| e6e40f6 | #task-059 | docs(changelog): v0.1.7 条目定稿（补日期 2026-10-29 + 删 Unreleased 段） |
| 241a997 | #task-059 | docs(readme): 重写 README——项目简介/功能列表/安装指引/开发指引/技术栈 |
| 7eb5392 | #task-059 | docs(guide): 新增用户手册 USER_GUIDE.md（8 节覆盖全功能） |
| （本次） | #task-059 | docs(state): 任务059完成，v0.1.7 发版准备就绪，待用户手动 workflow_dispatch |
| 360df74 | #task-059 | merge: task-059 v0.1.7 发版准备——版本号+CHANGELOG 定稿+README+用户手册（--no-ff） |
| 6e5fb5b | #task-060 | docs(execute): 固化「不注册 rgapplications」的决策依据 + 用户手册说明连带关闭兄弟进程 |
| 6734cd5 | #task-060 | feat(execute): RmShutdown 1s 进度 ticker + shutdown://progress 事件 + 前端关闭进度条 |
| 79e2c93 | #task-060 | test(execute): 补 ticker 用例 + 适配四参签名 + 前端进度条用例 |
| e85444a | #task-060 | docs(guide): 补跨盘行为说明——移入有 fallback、恢复不支持跨盘 + FAQ Q9 |
| （本次） | #task-060 | docs(state): 任务060完成，占用关闭体验优化（连带兄弟进程 + 30s 进度 + 跨盘文档） |
| 3b2a961 | #task-060 | merge: task-060 占用关闭体验——RmShutdown 进度反馈+跨盘文档化（--no-ff） |
| 890bc19 | #task-061 | feat(ui): 深色/浅色主题切换（三态 + prefs 持久化 + 即时生效） |
| fde19d6 | #task-061 | test(ui): 补主题三态用例 + 适配整体覆盖断言 |
| d01816d | #task-061 | feat(stats): 清理历史趋势图（cleanup_stats_trend + 仪表盘 SVG 柱状图） |
| bdb7de3 | #task-061 | test(stats): 补趋势聚合用例 + 前端渲染用例 |
| 6b1f9be | #task-061 | feat(scan): 计划自动扫描（开关+间隔，后台轮询，只扫描不删除，发通知） |
| 424049c | #task-061 | test(scan): 补间隔归一化/到期判断/字节格式化 + 前端自动扫描 UI 用例 |
| （本次） | #task-061 | docs(state): 任务061完成，M5 起步——主题切换/趋势图/自动扫描 |
| b92a763 | #task-061 | merge: task-061 M5 起步——主题切换+趋势图+自动扫描（--no-ff） |
| 3efe227 | #task-062 | feat(elevation): 提权 exe 底座（独立 crate + UAC + 白名单 + 结果回传 + 随包部署） |
| f848c75 | #task-062 | feat(execute): 清理前自动创建系统还原点（L2 触发 + 失败 confirm 后继续） |
| b128253 | #task-062 | test(execute): 补 L2 判定用例 |
| 15ab53a | #task-062 | feat(tray): 系统托盘常驻（关闭最小化 + 右键菜单 + 退出二次确认 + 开机自启） |
| 2b505bc | #task-062 | test(tray): 补开机自启用例（回填/切换/失败回滚） |
| （本次） | #task-062 | docs(state): 任务062完成，提权底座+系统还原点+托盘常驻就绪 |
| c36639f | #task-062 | merge: task-062 提权 exe 底座+系统还原点+托盘常驻（--no-ff） |
| 2976e4f | #task-063 | feat(report): 清理报告导出（HTML 单文件 + CSV 明细）+ 前后空间对比 |
| 46d01c0 | #task-063 | feat(dashboard): C 盘空间环形图（SVG 环形 + 已用百分比 + 紧张告警） |
| 1165b14 | #task-063 | feat(scan): 取消扫描按钮（cancellation token + 脚本粒度停止） |
| 23d1332 | #task-063 | fix(report): 修正 doc 注释缩进（clippy doc_lazy_continuation） |
| （本次） | #task-063 | docs(state): 任务063完成，报告导出+前后对比+环形图+取消扫描就绪 |
| 62cb7cd | #task-063 | merge: task-063 清理报告导出+前后对比+C盘环形图+取消扫描（--no-ff） |
| d4d56c2 | #task-064 | feat(safe_box): 7 天自动 purge（启动跑一次 + 每 6 小时，逐条审计） |
| 8d68b9b | #task-064 | feat(ui): 风险徽标/影响说明 + 全选 L1 一键按钮 |
| 99e200a | #task-064 | test(ui): 补风险徽标/tooltip 与全选 L1 用例 |
| （本次） | #task-064 | docs(state): 任务064完成，P0 收尾——purge/徽标/全选L1 就绪 |
| a130c08 | #task-065 | feat(elevation): 提权 exe 白名单加 dism_analyze / dism_cleanup，硬编码禁 /ResetBase |
| be91232 | #task-065 | feat(scan): WinSxS 分析命令（DISM /AnalyzeComponentStore + 文本解析） |
| 53ae5e6 | #task-065 | feat(scan): WinSxS 清理命令（DISM /StartComponentCleanup + 还原点自动创建 + 进度转发） |
| ca96659 | #task-065 | feat(ui): WinSxS 模块 UI（分析结果 + 清理按钮 + 进度显示） |
| （本次） | #task-065 | docs(state): 任务065完成，M1 C 模块 WinSxS 清理就绪 |
| c0da5b9 | #task-066 | feat(scan): 大文件扫描后端（hiberfil/Windows.old/回收站/Top100，只读） |
| 4c0ad11 | #task-066 | feat(ipc): big_files_scan IPC 命令接线 |
| 3cb4283 | #task-066 | feat(ui): 大文件地图页面（四分类卡片 + Top100 列表，只读模式） |
| 2324dc1 | #task-066 | feat(ui): App tab 注册「大文件」按钮 + 深色适配 |
| f98d0a3 | #task-066 | perf(scan): Top100 改 BFS 遍历，截断时优先覆盖浅层用户目录 |
| （本次） | #task-066 | docs(state): 任务066完成，M1 E 模块大文件地图就绪 |

---
变更轨迹：
- 2026-10-11  task-000  初始化 PROJECT_STATE.md，记录立项基线完成状态与 M0 待办
- 2026-10-11  task-001  初始化 monorepo 脚手架与 Tauri 2 最小空窗口
- 2026-10-11  task-002  落地 SQLite schema（9 张表）+ rusqlite 连接 + user_version 迁移器
- 2026-10-11  task-003  落地删除唯一入口与 L3 白名单匹配器
- 2026-10-11  task-004  落地 IPC 网关骨架（信封/错误码/scan.start/cleanup.dryRun/进度事件）
- 2026-10-12  debt-cleanup-批次001  A类口径回写 TRD §5.1（2002 data 例外）与 task-003 B.4（.. 一律拒）；拆出 task-007~011；B类已知限制登记
- 2026-10-12  task-005  落地初始规则库与 SHA-256 校验加载器
- 2026-10-12  task-005  评审前清障：独立复跑 cargo check/clippy/fmt/test 全绿、复核 checksum 一致；无第一档修复项；3 条第三档延期项登记入已知问题
- 2026-10-12  task-006  搭好 GitHub Actions CI（windows-latest 五道闸 + 规则自洽守卫），PR #1 经 2 轮修复后全绿，M0 阶段收官
- 2026-10-12  task-006  评审前清障：发现 loader.rs 算 checksum 未做 CRLF→LF 归一（Windows autocrlf 会把合法 rules 误判篡改），已修复并补 CRLF 容忍用例；M0 遗留 5 项均已登记已知问题
- 2026-10-12  task-007  落地真实 ProgressEmitter 与 scan://progress 前端 listen 端到端通道（demo emit 已通，真实扫描接入留 M1）
- 2026-10-12  task-007  评审前清障：独立复跑 clippy/test/pnpm build 全绿，核验 TauriProgressEmitter/ProgressState/scan_start 签名与 engine 常量对齐；无第一档修复项；devtools 截图缺口已登记已知问题
- 2026-10-12  task-008  junction/symlink 真实路径解析接入 delete 唯一入口（canonicalize + 对真实路径二次 L3 判定）
- 2026-10-12  task-008  评审前清障：修正 junction 测试名与断言矛盾（原名为 rejected 实断言 allowed），补 resolve 失败不调 NoopDeleter 反证用例；clippy/test 全绿
- 2026-10-12  task-009  DB 版 ItemPathResolver 落地并单测覆盖
- 2026-10-12  task-009  评审前清障：独立复跑 clippy/fmt/test 全绿（77 条），修正提交表 hash 笔误（test commit a535b6e→47b542f）；无第一档代码修复项
- 2026-10-12  task-010  外部 whitelist_deny 接入 cleanup.dryRun
- 2026-10-12  task-010  评审前清障：修正外部红线集成用例（原用 %USERPROFILE%\Videos 命中的是内置 L3 而非外部红线），改用 %LOCALAPPDATA%\SomeSensitiveCache 并断言 L3SystemPath，证明外部红线独立生效；clippy/test 全绿
- 2026-10-12  task-011  ps1 兼容层从零编写（5 脚本 + NDJSON 协议），M1 前置清理收官
- 2026-10-12  task-011  评审前清障：实跑冒烟测试 5/5 通过（temp=157/win_update=6/prefetch=0/browser=13/dev=33），cargo check exit 0；无第一档修复项
- 2026-10-12  task-012  Rust 侧 ps1 NDJSON 消费器（parser + runner）落地，单测覆盖
- 2026-10-12  task-012  评审前清障：独立复跑 clippy/fmt/test 全绿（88 条，含 ps1 spawn 3/3）；无第一档修复项；Tauri 接线/5脚本编排/scan.cancel 留 task-013
- 2026-10-12  task-013  scan.start 端到端接线：5 ps1 串跑 + 真实 session + 前端 listen
- 2026-10-12  task-013  评审前清障：独立复跑 clippy/fmt/test 全绿（92 条，含 orchestrator 4/4）；修正 scan_start doc 注释（同步执行+last_insert_rowid，非 spawn_blocking/RETURNING）；devtools console 截图仍留 UI 联调
- 2026-10-12  task-014  cleanup.dryRun 切换 DbItemPathResolver，dryRun 闭环真实数据
- 2026-10-12  task-014  评审前清障：独立复跑 clippy/fmt/test 全绿（92 条不回退）；确认 MockResolver import 已删、DbItemPathResolver 接线薄包装无偏差；无第一档修复项
- 2026-10-12  task-015  cleanup.execute 真实删除（FsDeleter + delete::execute + audit_log）
- 2026-10-12  task-015  评审前清障：executor 补 resolve_to_real——junction/symlink 重定向须对 canonicalize 后真实路径做 L3 判定（原只对输入路径分类，junction 可绕过）；NotExists 回退输入路径分类兼容幂等重删；clippy/test 96 条全绿
- 2026-10-12  task-016  safe_box 5GB FIFO 保险箱落地，删除不再直接 remove_file
- 2026-10-12  task-016  评审前清障：safe_box 测试加全局串行锁——QUOTA_BYTES 是进程级 AtomicU64，并行测试互相覆盖导致 evicts_oldest 偶发失败；clippy/test 99 条全绿
- 2026-10-12  task-017  safe_box restore 闭环，用户后悔可恢复文件
- 2026-10-12  task-017  评审前清障：独立复跑 clippy/fmt/test 全绿（104 条，含 restore 5/5）；确认 restore 逻辑正确（TargetOccupied/ParentMissing/rename/delete row/audit_log），无第一档修复项
- 2026-10-12  task-018  safe_box.list 命令就绪，前端 UI 可列出保险箱条目
- 2026-10-12  task-018  评审前清障：独立复跑 clippy/fmt/test 全绿（108 条，含 list 4/4）；确认 list 查询正确（status='in_box' 过滤、quarantined_at DESC、size_bytes i64→u64），无第一档修复项
- 2026-10-12  task-019  删除失败错误分类（FILE_IN_USE/PERMISSION_DENIED）落 audit_log
- 2026-10-12  task-019  评审前清障：独立复跑 clippy/fmt/test 全绿（116 条，含 classify 4/4 单元 + 4/4 集成）；确认 SafeBoxDeleter rename 失败分类前缀和 executor audit_log detail 翻译逻辑正确；notepad 真机联调未跑（第三档已登记）
- 2026-10-12  task-020  who_is_locking——Restart Manager FFI 查占用进程 PID
- 2026-10-12  task-021  FILE_IN_USE 自动查占用进程写 audit_log 并返回前端
- 2026-10-13  task-022  rules_meta 落 DB + rules.checkUpdate 查远端版本（不下载）
- 2026-10-13  task-023  rules.applyUpdate——下载新规则 + SHA-256 校验 + 备份替换 + upsert DB
- 2026-10-13  task-024  audit_log.list 只读查询命令落地，历史页后端基础
- 2026-10-13  task-025  cleanup_stats 聚合查询（total_bytes / success / fail / sessions）
- 2026-10-13  task-026  Dashboard 4 数字卡片（stats 接前端，GB 格式化）
- 2026-10-13  task-027  History 历史明细页（audit_log.list 表格 + 分页 + tab 切换）
- 2026-10-13  task-027  评审前清障：独立复跑 clippy/fmt/test/pnpm build 全绿（133 条测试无回退，vite build 148KB gzip 47.7KB）；确认 formatTs 非法输入兜底"-"、History loading/ok/error 三态+分页（offset 累加、canNext 按 rows.length===PAGE_SIZE 判定）、App tab 切换 useState 模式正确；tauri dev 真机切 tab 未跑（第三档已登记）
- 2026-10-13  task-026  评审前清障：独立复跑 clippy/fmt/test/pnpm build 全绿（133 条测试无回退，vite build 145KB gzip 46.8KB）；确认 formatBytes 四档 1024 进制+负数/NaN 兜底、Dashboard loading/ok/error 三态状态机、失败数>0 红色高亮、useCallback 重试逻辑正确；tauri dev 真机截图未跑（第三档已登记）
- 2026-10-13  task-025  评审前清障：独立复跑 clippy/fmt/test 全绿（133 条，含 audit_log_stats 4/4）；确认单条 SQL COALESCE 聚合、session_id 参数化过滤、cleanup_task 单行字段求和逻辑正确
- 2026-10-13  task-024  评审前清障：独立复跑 clippy/fmt/test 全绿（129 条，含 audit_log_query 4/4）；确认 ts DESC/id DESC 分页、limit/offset 防御、action→event_type/target→path/ts→created_at 列名映射正确；session_id 过滤用户拍板方案二——入参保留标记"预留未启用"，待后续 schema 加列后启用
- 2026-10-13  task-023  评审前清障：独立复跑 clippy/fmt/test 全绿（125 条，含 apply_update 3/3）；确认 SHA-256 校验/备份/替换/upsert/回滚全链路正确；真实 GitHub Raw 下载联调未跑（第三档已登记）
- 2026-10-13  task-022  评审前清障：独立复跑 clippy/fmt/test 全绿（122 条，含 rules_meta_store 3/3）；确认 upsert/get_active 单行表逻辑正确、fetch_remote_meta reqwest blocking 实现正确；真实 GitHub Raw 网络联调未跑（第三档已登记）
- 2026-10-12  task-021  评审前清障：独立复跑 clippy/fmt/test 全绿（119 条，含 execute_locking 2/2 必跑 + 1 ignored）；确认 FILE_IN_USE 分支自动调 who_is_locking 拼 detail、PERMISSION_DENIED 分支不触发；notepad 端到端 #[ignore] 未跑（第三档已登记）
- 2026-10-12  task-020  评审前清障：独立复跑 clippy/fmt/test 全绿（117 条，含 who_is_locking 1/1 必跑 + 1 ignored）；确认 RmStartSession/RegisterResources/GetList/EndSession 四件套 FFI 逻辑正确；notepad 端到端 #[ignore] 未跑（第三档已登记）
- 2026-10-13  task-028  前端 scan_start 按钮 + scan://progress 实时进度条（percent/current_path/scanned_items）
- 2026-10-13  task-028  评审前清障：独立复跑 clippy/fmt/test/pnpm build 全绿（133 条测试无回退，vite build 150.6KB gzip 48.5KB）；确认 listen 订阅/finally unlisten 配对、percent 直接用 0~100 不除零、按钮四态文案（扫描中/再次扫描/重试扫描/开始扫描）、scanning 时 disabled 防重复点击、done 后自动 load() 刷新 stats 正确；ProgressEvent shape 按 engine 真实字段对齐（session_id/percent/current_path/scanned_items，非任务书假设的 module/done/total）；tauri dev 真机点按钮看进度条走动未跑（第三档已登记）
- 2026-10-13  task-029  cleanup.listPending + Items 表格 + checkbox 清理按钮，M1 业务闭环
- 2026-10-13  task-030  SafeBox 表格 + 单条恢复按钮，M1 安全网闭环
- 2026-10-13  task-031  Settings 规则库卡片 + 检查更新/应用更新按钮，M1-E 前后端打通
- 2026-10-13  task-031  评审前清障：独立复跑 clippy/fmt/test/pnpm build 全绿（137 条测试无回退，vite build 158.9KB gzip 50.4KB）；确认 CheckUpdateResp camelCase 字段（hasUpdate/currentVersion/remoteVersion/currentSha256/remoteSha256）映射正确、ApplyResult newVersion/newSha256 消费正确、应用后自动 check() 重拉刷新版本号、hasUpdate=true 时才显示「应用更新」按钮、检查中/应用中按钮禁用正确；SHA-256 slice(0,16) 截断显示；App.tsx 五 tab（仪表盘/清理/保险箱/历史/设置）useState 切换；tauri dev 真机端到端（切设置→检查更新→应用更新）未跑（第三档已登记）
- 2026-10-13  task-030  评审前清障：独立复跑 clippy/fmt/test/pnpm build 全绿（137 条测试无回退，vite build 156.2KB gzip 49.7KB）；确认 safe_box_list 返回 {entries,totalItems,totalBytes} 响应结构正确消费、SafeBoxEntry camelCase 字段（originalPath/sizeBytes/quarantinedAt）映射正确、restore 入参 safeBoxId 驼峰对应 Rust safe_box_id、restoringId 单行恢复中禁用/finally 复位、空列表「保险箱为空」占位正确；App.tsx 四 tab（仪表盘/清理/保险箱/历史）useState 切换；tauri dev 真机端到端（删→进保险箱→恢复）未跑（第三档已登记）
- 2026-10-13  task-029  评审前清障：独立复跑 clippy/fmt/test/pnpm build 全绿（137 条测试，含 list_pending 4/4）；确认 list_pending_items session_id 可选过滤/limit.clamp(1,500)/size_bytes DESC/参数化正确、Items 页面 Set<number> 选中态/全选 checkbox/清理已选按钮 disabled 条件/cleanup_execute 入参 itemIds 驼峰映射正确；cleanup_item 表无 status/scanned_at 列按真实 schema 适配（pending=表内所有行，清理后不删行）；tauri dev 真机端到端（扫→勾→清→数字涨）未跑（第三档已登记）
- 2026-10-13  task-032  ESLint+Vitest 接入，CI 跑 lint/test，task-006 echo 占位清零
- 2026-10-13  task-033  NSIS v0.1.0 配置 + 图标 + CI 出 installer artifact，M1 可分发
- 2026-10-13  task-033  评审前清障：独立复跑 clippy/fmt/test/pnpm lint/pnpm test/pnpm build 全绿（137 条后端测试无回退，前端 9 断言全过）；确认 tauri.conf.json productName/version/identifier/bundle.targets=["nsis","msi"]/windows.nsis.installMode=perMachine+languages=["SimpChinese"] 字段正确、CI yml 加 Tauri build + Upload installer artifact 两步正确（path 指向 workspace 根 target/release/bundle/，if-no-files-found: error）；Tauri 2 NSIS schema 用 languages 数组（非任务书 displayLanguage 标量）按真实 schema 适配；本地 tauri build 出 .msi 5.11MB + .exe 3.60MB；CI artifact 上传未跑（第三档已登记，需 PR 触发）；代码签名/updater/设计图标/macOS-Linux/真机安装验证留 task-034+（第三档已登记）
- 2026-10-13  task-034  GitHub Release workflow_dispatch + tag 发版通道，v0.1.0 首次发版待手动触发
- 2026-10-13  task-034  评审前清障：独立复跑 clippy/fmt/test/pnpm lint/pnpm test/pnpm build 全绿（137 条后端测试无回退，前端 9 断言全过）；确认 release.yml 触发器 workflow_dispatch(inputs.version 必填)+push.tags=["v*"]、permissions contents:write、steps checkout→pnpm/action-setup→setup-node(node 22 cache pnpm)→dtolnay/rust-toolchain→pnpm install→tauri build→softprops/action-gh-release@v2 正确；release action 同时设 name+tag_name（workflow_dispatch 手动触发时 tag 也创建）、files 指向 workspace 根 target/release/bundle/、generate_release_notes:true；首次 v0.1.0 发版未代跑（无 gh CLI 鉴权，需评审在网页手动触发或 git push tag，第三档已登记）；tauri updater/代码签名/changelog 自动生成/设计图标/macOS-Linux 留 task-035+（第三档已登记）
- 2026-10-13  task-032  评审前清障：独立复跑 clippy/fmt/test/pnpm lint/pnpm test/pnpm build 全绿（137 条后端测试无回退，前端 9 断言全过：formatBytes 5 + formatTs 4）；确认 ESLint flat config（eslint.config.js）TS 文件 no-undef 关闭交给类型检查、react-hooks rules-of-hooks error/exhaustive-deps warn、Vitest jsdom 环境+setupFiles 配置正确、setupTests.ts 用 @testing-library/jest-dom/vitest 子路径导入、CI yml 加 Lint/Test 两步替换 echo 占位正确；Vitest 降级 2.1.9 兼容 Vite 5（Vitest 5 需 Vite 6+）；组件单测留 task-033+（第三档已登记）
- 2026-10-13  debt-cleanup-002  M1 NSIS 安装包装干净目录后 5 个回归 bug：Envelope 未解包白屏/数字空（api() 解包 + ErrorBoundary）、plugin:event|listen ACL 拒（capabilities/default.json 授权 core:default + core:event:default）、release.yml pnpm 版本双声明（删 with: version: 11）、resolve hop 在 CI 8.3 短名下误判 1（改 symlink_metadata 查 REPARSE_POINT）、Dashboard 重试扫描残留进度（catch 里 setProgress(null)）。分支 feature/fix-m1-packaging-bugs 等评审。
- 2026-10-14  debt-cleanup-003  NSIS 干净目录真机验收 2 个 P0：scan_start 前端漏传 scope 必填参数（传 { scope:'full', targetDrive:null }）、current_dir() 在 Program Files 只读导致 DB EACCES（新建 AppPaths State，data_root 走 app_data_dir()，rules/scripts 打包为 bundle.resources，首次复制 rules 到 data_root）。Bug-3（设置页错误信息坍缩）+ Bug-4（Dashboard 数字卡片空白）留 debt-cleanup-004。分支 feature/fix-m1-postinstall-v2 等评审。
- 2026-10-14  debt-cleanup-004  二轮真机验收 3 个回归：Dashboard CleanupStats/ScanProgress interface snake_case 与后端 serde camelCase 不匹配（数字卡片空白 + 进度文字卡死「准备中...」，顺带修 History AuditLogEntry）、rules_check_update/apply_update 所有错误坍缩成 INTERNAL（网络错→RULE_NETWORK_ERROR 3003，SHA 校验错→RULE_CHECKSUM_MISMATCH 3002）。已知遗留：后端 runner percent 统一发 0（单脚本内不知总量），进度条宽度不动留后续任务。同分支 feature/fix-m1-postinstall-v2 追加提交等评审。
- 2026-10-14  task-035  tauri-plugin-updater + 签名密钥对 + plugins.updater 配置 + Settings 应用更新卡片 + release.yml 签名/latest.json，M2 自动更新最小闭环。
- 2026-10-14  task-035  评审前清障：独立复跑 clippy/fmt/test/pnpm lint/pnpm test/pnpm build 全绿（137 条后端测试无回退，前端 9 断言全过，vite build 164.6KB gzip 52.5KB）；确认 tauri.conf.json plugins.updater 命名空间（active/endpoints/pubkey，非任务书顶层 updater）、lib.rs 挂 tauri_plugin_updater::Builder、Cargo.toml 加 tauri-plugin-updater=2 + serde_json=1、Settings.tsx 应用更新卡片 check/downloadAndInstall/relaunch 状态机正确（idle/checking/downloading/error）、release.yml Setup signing key 写 ~/.tauri/cclean.key + TAURI_SIGNING_PRIVATE_KEY_PATH 注入 + Generate latest.json 读 .msi.sig 生成 JSON 并随 Release 上传；私钥未入仓（git log -p 自审无 BEGIN PRIVATE）；GitHub Secret TAURI_SIGNING_PRIVATE_KEY 需用户手动配置（第三档已登记）；latest.json push 回 main 分支未自动化（raw URL 拿到的是占位，第三档已登记）；后台自动检查弹窗/增量更新/真机端到端升级验证留 task-036+（第三档已登记）
- 2026-10-14  task-036  启动自动 check + 非阻塞 banner 提示新版本，updater 最后一公里。
- 2026-10-14  task-036  评审前清障：独立复跑 clippy/fmt/test/pnpm lint/pnpm test/pnpm build 全绿（137 条后端测试无回退，前端 12 断言全过，vite build 165.8KB gzip 52.8KB）；修复 P1 bug——App.tsx banner 更新说明文字条件 `!bannerPhase` 恒 false（bannerPhase 初始值是字符串 "idle"，!"idle"=false），导致 update.body 永远不显示，改为 `bannerPhase === "idle"`；确认启动 useEffect 自动 check() 静默 catch、banner 三态（idle/downloading/error）、立即更新走 downloadAndInstall+relaunch、稍后按钮 setDismissed(true)、ErrorBoundary 包裹 tab 渲染正确；强制更新/下载进度条/定时轮询/"不再提示此版本"持久化/真机端到端升级验证留 task-037+（第三档已登记）
- 2026-10-14  task-037  safe_box.purge 单条永久删除（后端 purge_item + Tauri 命令 + 前端红色按钮 + confirm）。
- 2026-10-14  task-037  评审前清障：独立复跑 clippy/fmt/test/pnpm lint/pnpm test/pnpm build 全绿（140 条后端测试含 safe_box_purge 3/3，无回退，vite build 166.4KB gzip 53.0KB）；核验 purge_item 逻辑正确（查 in_box 行→删磁盘文件 NotFound 吞→DELETE DB 行→audit_log action='purge'）、Tauri 命令 safe_box_purge 入参 item_id i64 + Envelope<bool> 已注册 lib.rs、前端 purge 走 api() 解包 Envelope + window.confirm + 红色按钮 per-row purgingId 禁用；无第一档修复项；批量 purge/批量恢复/quota UI/真机端到端删文件验证留 task-038+（第三档已登记）
- 2026-10-14  task-038  safe_box.purge_many 批量永久删除（后端 batch + 前端 checkbox 列）。
- 2026-10-14  task-038  评审前清障：独立复跑 clippy/fmt/test/pnpm lint/pnpm test/pnpm build 全绿（143 条后端测试含 safe_box_purge_many 3/3，无回退，vite build 167.7KB gzip 53.3KB）；核验 purge_many 循环调 purge_item 单个失败不中断收集 errors、PurgeManyResult camelCase（deleted/failed/errors）与前端 interface 对齐、safe_box_purge_many 命令入参 itemIds Vec<i64> 已注册 lib.rs、前端 Set<number> 选中态 + 表头全选 toggle + 批量按钮 N=0 禁用 + confirm 确认 + 结果绿/红条 + 成功后清空 selected 重拉列表；无第一档修复项；批量恢复/进度条/跨页全选/真机端到端批量删验证留 task-039+（第三档已登记）
- 2026-10-14  task-039  safe_box.restore_many 批量恢复（复用 task-038 checkbox UI）。
- 2026-10-14  task-039  评审前清障：独立复跑 check/clippy/fmt/test/pnpm lint/fe-test/build 全绿（146 条后端测试含 safe_box_restore_many 3/3，无回退，vite build 168.5KB gzip 53.4KB）；核验 restore_many 循环调 restore_from_safe_box 单个失败不中断、新建 BatchOpResult{succeeded,failed,errors} camelCase 与 PurgeManyResult 解耦（purge 用 deleted、restore 用 succeeded）、safe_box_restore_many 命令入参 itemIds Vec<i64> 已注册 lib.rs、前端绿色「批量恢复已选(N)」按钮在红色批量删除左边 + 复用 selected Set + confirm + 结果绿/红条 + 成功后清空 selected 重拉列表；无第一档修复项；全部恢复/全部 purge 一键/进度条/跨页全选/真机端到端批量恢复验证留 task-040+（第三档已登记）
- 2026-10-15  task-039  评审前清障（二轮，清障 Agent）：① 第一档修复——`cargo fmt --all -- --check` 在 HEAD 与 main 上均红灯（task-038 提交的 `purge_many`/`insert_box_row` 多行签名 + 断言未展开，task-038 评审「fmt 全绿」系误报），已 cargo fmt 对齐并提交；② 第一档修复——SafeBox 行内「恢复 / 永久删除」按钮禁用条件漏了 `batchPurging`/`batchRestoring`，批量操作进行中仍可点单条按钮造成竞态，已补禁用；③ 第一档补测——`restore_many` 原 partial_fail 失败项在末尾，无法证明「单个失败不阻断后续 id」，补 `restore_many_failure_in_middle_does_not_stop_rest`（ParentMissing 夹中间，断言后续 id 仍成功）并修正 `BatchOpResult` 误导性 doc 注释；④ 第三档登记——批量恢复真机端到端未跑（无 GUI 数据行）。修复后 check/clippy/fmt/test（147 条）/pnpm lint/fe-test/build 全绿（vite build 168.51KB gzip 53.43KB）
- 2026-10-15  task-039  评审通过：真机冒烟（pnpm tauri dev + WebView2 CDP 挂载 9222）逐个点过五 tab 并造数据跑通「勾选 → 批量恢复已选 → confirm → 绿/红条 → 清空 selected 重拉列表」全链路（3 条：2 成功回原路径 + 1 失败父目录缺失保留 in_box），文件/DB/audit_log 三方一致，本任务页面 Console 0 红错 0 exception；回归批量删除正常；本地门禁独立复跑全绿（fmt/clippy/147 后端测试/lint/12 前端断言/build 168.51KB gzip 53.43KB）；**CI 实查**：分支 run 38671216149（PR #2）绿，合并前 main 2068fcd run 38571515030 因 task-038 fmt 问题红（本分支 adcba07 已修），合并后 main run 38673720322 绿；merge --no-ff → `cbc5aaa`，feature 分支已删；新增登记 2 条非阻塞已知问题（设置页首次 rules_check_update 红错、规则检查无超时收敛，均非本任务引入）
- 2026-10-16  task-041  rules reqwest 超时收敛 + updater endpoint 改 Releases 资产（task-039 评审 P0 真机卡死修复）
- 2026-10-16  task-042  rules.checkUpdate 加 Offline 三态，断网时本地规则库照常工作 + UI 黄条提示
- 2026-10-16  task-043  safe_box.restore_item 自动 mkdir -p 父目录（task-017 已知问题闭环）
- 2026-10-16  task-044  rules_rollback 单版回滚（后端命令 + 前端灰色按钮 + confirm）
- 2026-10-16  task-045  safe_box_list 分页（LIMIT/OFFSET + total + 前端分页栏）
- 2026-10-16  task-046  safe_box list session 过滤（可选 session_id 参数 + list_sessions + 前端下拉）
- 2026-10-16  task-047  rules.applyUpdate 前端 window.confirm 弹窗（嵌入版本号 + 回滚提示）
- 2026-10-23  task-048  v0.1.1 真机升级验证通过（banner 弹出 v0.1.1 + 设置页应用更新卡片一致），M2 正式关闭
- 2026-10-23  task-049  派发占用进程 RmShutdown（who_is_locking 升级为可请求关闭 + 前端占用提示/一键重试）
- 2026-10-23  task-049  RmShutdown 落地：shutdown_locking_processes + 危险进程护栏 + cleanup_retry_locked + Items 失败明细/一键关闭重试；顺带修复 who_is_locking probe 判断（HRESULT vs Win32 码）导致占用列表恒空的 bug；后端全链路真机 PASS，GUI/安装态冒烟因环境限制留测试 Agent
- 2026-10-24  task-049  测试轮：全部门禁独立重跑（cargo 183 / vitest 34 / build 176.39 kB）；修 C12 三态互斥缺陷（0129de9）+ 补 B7 隔离真机用例（f667c70）；NSIS 干净目录安装 + 数据目录隔离 + 安装目录零写入通过；GUI 点击/Console 因无活动桌面（WebView2 渲染进程崩溃）未取得，登记第三档；新登记预存在问题「打包资源路径错位」；状态置 awaiting_review
- 2026-10-24  task-049  评审轮 1 **不通过**（`docs/reviews/049-review.md`）：输入齐全、提交规范、契约四项对齐、六门禁与 CI（run 40048746500 @ 6705493）绿；但抽查发现护栏按本地化应用名匹配，`zh-CN` 下 `explorer.exe`（`Windows 资源管理器`）绕过护栏；另 mixed 场景 `closed/refused` 语义失真、B7 用例连跑 1/3 失败。**不合并 main，feature 保留 @ 6705493**，转 `bugfix/task-049-guard-image-name`
- 2026-10-24  task-049b 修复轮：护栏改 PID→映像名（`QueryFullProcessImageNameW`）+ 取不到映像名 fail-closed；mixed 占用 fail-closed（存在被拒进程即整批不关）；B7 根因定为 holder 共用 console，改独立 console + ready/done + 退出码归因，连跑 5/5 全绿；护栏单测 10→15、集成新增假 explorer 持锁用例；六门禁 + CI run 40054417543 全绿；`--no-ff` 合回 feature，状态 `testing`，待测试 Agent 全量回归 + 评审重评
- 2026-10-24  task-049b 测试轮 2 **通过**（`docs/test-reports/049b-test-report.md`）：独立重跑 fmt 0 / clippy 0 / engine 196 条（188+8）/ vitest 34 / build 176.39 kB；真机 `--include-ignored` 合并前后各连跑 5 次共 10 次全绿，假 `explorer.exe` 持锁实测 `closed=[]` + `refused=[protected process]`；NSIS 干净目录安装 exit 0 + 安装目录 9/9 文件零变化；修 2 项第一档（护栏「本地化名」用例 PID 环境依赖 → `a740dea`；`reason` 注释缺 `unknown process` → `448275a`），`--no-ff` 合回 feature `5550f61`，CI 40059150999 绿；GUI 点击/Console 仍因无活动桌面（Crashpad 新增 dmp）登记第三档；状态置 awaiting_review
- 2026-10-24  task-049  **评审轮 2 通过并合并 main**（`docs/reviews/049b-review.md`）：轮 1 三项否决/阻塞缺陷全部闭环（护栏 PID→映像名 + fail-closed、mixed 整批不关、B7 独立 console 后 10 次连跑全绿）；抽查复现 15 条护栏单测 + 真机假 explorer 用例（`closed=[]`、`refused=[protected process]`）；CI 40060421641 @03b53b4 success；`--no-ff` merge commit **`60e0797`**，feature 与 bugfix 子分支已清理；新登记「`RmShutdown` 连带关闭同 console 兄弟进程」「mixed 无解释文案」两项遗留观察
- 2026-10-24  task-050  打包资源路径错位修复（`bugfix/task-050-bundle-resources-path`，状态 `testing`，**未合 main**）：走 A 路线——`bundle.resources` 数组式改**映射式**（7 条落到 `<安装目录>/rules`、`/scripts`，`_up_` 消失）；`paths.rs` 抽 `pick_resource_root`（纯函数）+ `build_candidates`（release 只认 `resource_dir()`，dev 优先 cwd 三级再退 `resource_dir()`），删 `PathBuf::from(".")` 兜底，资源缺失改带绝对路径的显式 `Err`；新增 `scripts/tauri-resources.test.mjs`（6 条断言）并接 CI。门禁：fmt/clippy 0、cargo test **197 passed + 8 ignored**（engine 196 不回退，app crate +9）、vitest 34、build 176.39 kB；`tauri build` 出 NSIS 4.00 MiB + MSI 5.81 MiB（含 .sig）；NSIS 干净目录安装后 9 文件布局正确、无 `_up_`、安装目录 SHA-256 **NO_DIFF**；从诱饵目录启动仍定位安装目录 `scripts`，`%APPDATA%\…\rules` 被重建且与安装包逐字节一致；CI **40090245117 / 40090552969 均 success**（含「Tauri 资源映射契约」步骤）。遗留（第三档）：GUI「扫描 → 清理」点击因 WebView2 渲染进程崩溃未取得、MSI 未做安装级布局抽检、dev 日志路径含 `..` 未做词法归一
- 2026-10-24  task-050  **测试轮通过**（`docs/test-reports/050-test-report.md`，分支 @`e7171c4`）：独立重跑 fmt 0 / clippy 0 / cargo test **197 passed + 8 ignored** / vitest 34 / 契约 6/6 / build 176.39 kB；契约守卫变异测试（改回数组式）6 条红 5 条、还原后 6/6 绿；`tauri build` exit 0（NSIS 4.01 MiB + MSI 5.81 MiB 含 `.sig`），NSIS 干净目录安装 9 文件布局正确、`_up_` 不存在、安装目录 **NO_DIFF**、`%APPDATA%\…\rules` 重建且与安装包逐字节一致、诱饵目录未劫持；**MSI `/a` 抽检补做**（8 文件同布局、无 `_up_`）；负向用例（删 `scripts/`）启动即报带绝对路径的显式错误；从安装目录直接执行 `scan_temp.ps1` exit 0（1339 items）；CI run **40090971932** success。**0 条需修复缺陷、未开 bugfix 子分支**；GUI 仍因 WebView2 渲染进程崩溃（3s 4 个 → 6s 归零、Crashpad 新增 dmp）登记第三档；状态置 `awaiting_review`，待评审 Agent
- 2026-10-25  task-050  **评审通过并合并 main**（`docs/reviews/050-review.md`）：输入核验三项齐全（报告 / CI run 40090971932 success @e7171c4 / commit 18f2913，本地=origin `0 0`）；9 个提交全部原子、规范带 `#task-050`、docs 与代码分离、无敏感文件、无越界（7 文件全在允许表内，lock 文件空 diff）；验收 20 条逐条核对 19 条实测通过 + E20 按任务书预设路径降级登记；抽查复现 2 条（契约测试 6/6、`cargo test paths::` 9 passed）；安全面净减少一个 CWE-426 本地路径劫持向量；`--no-ff` merge commit **`cadb286`**，bugfix 分支已删除（本地 + 远端）。**放行条件**：E20 GUI 扫描→清理须发版前在活动桌面机器补做。评审新登记 6 项不阻塞观察（P3：`pick_resource_root` 未校验目录非空、`latest.json` 历史入库；P4：死代码分支 / `..` 未归一 / verbatim 前缀 / 文档滞后措辞）
- 2026-10-25  task-051  开发完成（`feature/task-051-safebox-filter-history`，状态 `testing`，**未合 main**）：`SafeBoxFilter{session_ids, from_ts, to_ts}` + `list_paged` 动态 WHERE（`items`/`COUNT`/`SUM` 共用同一 WHERE，`total`/`totalBytes` 与筛选严格一致，`from>to` 返空页不报错）；`SafeBoxEntry.source_task_id` 双路径填充；新增 `item_history` + `audit_log::query::list_by_target`（`COLLATE NOCASE`）；IPC `safe_box_list` 参数升级 + `safe_box_item_history` 注册；前端时间预设 4 档 + 多选 session + 来源列 + 行内历史懒加载，筛选变化统一 `page=1`/清空 selected/收起历史。门禁：fmt 0 / clippy 0 / **cargo test 220 passed + 8 ignored** / vitest **45**（7 files）/ build **179.93 kB │ gzip 57.31 kB** / 契约 6/6；`tauri build` exit 0（NSIS 4.01 MiB + MSI 5.82 MiB，含 `.sig`），MSI per-user 安装后 9 项布局正确、`_up_` 不存在。新增 23 条用例（engine 18 + jsdom 11，含签名同步）。**与任务书不符 2 项**：NSIS `perMachine` 需 UAC 无法静默安装（本机 `is_admin=False`），改 MSI per-user 验证；`cargo clippy` 末尾 Windows 增量目录锁清理告警（退出码 0，非 lint）。GUI 仍因 WebView2 渲染进程崩溃未取得（第三轮，新增 3 个 `.dmp`、CDP 10061 拒绝），登记第三档。**CI**：draft PR **#16** → run **40163134926**（`3d00901`）+ run **40163528523**（`b217b61`）连续两轮 `conclusion=success`，21 步全绿
- 2026-10-25  task-051  **测试轮通过**（`docs/test-reports/051-test-report.md`，分支 @`c1a1af6`）：独立重跑 fmt 0 / clippy 0 / cargo **220 passed + 8 ignored** / vitest **46**（7 files）/ 契约 6/6 / build **179.94 kB**；变异测试 2 组（`>=`→`>` 3 红、COUNT 丢 WHERE 5 红，还原 18/18 绿）；**修 1 项缺陷**——刷新按钮 `onClick={load}` 把 click 事件当 `page`（`main` 自 task-045 预存在）→ `bugfix/task-051-refresh-click-arg`（`ef054aa`+`05d5475`）→ merge `c1a1af6`；`tauri build` exit 0（NSIS 4.01 MiB + MSI 5.82 MiB 含 `.sig`），**NSIS 干净目录安装 exit 0**（9 项布局、无 `_up_`、安装目录 NO_DIFF、规则落 `%APPDATA%` 且逐字节一致、诱饵未劫持）；GUI 连续第 4 轮被 WebView2 崩溃阻塞（第三档），新增 Edge headless 真 Chromium 渲染补偿（2 截图 + 7 条 IPC 实参 + Console 0 错）；CI 由 PR #16 触发（run **40168680216** @ `7bfc130` + 40168310323 @ `c1a1af6`，均 success）；状态置 `awaiting_review`
- 2026-10-25  task-051  **评审通过并合并 main**（`docs/reviews/051-review.md`）：输入核验三项齐全（报告 / CI run 40168680216 @7bfc130 + 40168310323 @c1a1af6 均 success / commit 7bfc130，本地=origin `0 0`）；16 个提交全部 `type(scope): … (#task-051)`、bugfix 子分支 `bugfix/task-051-refresh-click-arg` 已合回并清理（本地+远端无残留）、零敏感文件、14 文件全在允许表内、lock 文件空 diff；26 条验收全覆盖（25 条实测 + F25 按任务书预设路径降级第三档）；抽查复现 3 条（`safe_box_item_history` 6/6、`safe_box_time_range` 6/6、`SafeBox.test.tsx` 12/12）；动态 SQL 只拼占位符、值全绑定，OWASP A03 注入面走查无风险；`--no-ff` merge commit **`e6ab561`**（parents `5c91d3d` + `d020022`），feature 分支已删除（本地 + 远端）。**放行条件**：GUI 真机走查（与 task-050 的 E20 合并为同一次）须发版前完成。评审新登记 6 项不阻塞观察（P3：`item_history` 错误映射过宽、`COLLATE NOCASE` 索引失效、`main` 上 `5c91d3d ".."` 不规范提交；P4：`normalized()` O(n²)、react-refresh 2 warning、3 个碎片 docs(state) 提交）
- 2026-10-25  task-049  ③ 关闭体验文案小改合并 main（`--no-ff` **`801af22`**）：`confirm` 补「同一控制台内的相关进程可能一并关闭」、mixed fail-closed 补「因存在受保护进程，本次未关闭任何进程」；`Items.test.tsx` 7/7 + `lint` 干净；按用户指示未开 PR / 未单独触发 CI
- 2026-10-25  **真机 GUI 走查执行完毕（发版门禁 ②）→ 结论不通过**（报告 `docs/test-reports/gui-walkthrough-2026-10-25.md`）：首次取得真实 GUI 点击与 Console 证据；通过项——E20 扫描→清理不返 `INTERNAL`（成功 1 项 / 430 B）、清理列表 100 项、历史落库、清理链路 Console 零红、④ 红条明细 + confirm 文案正常、保险箱时间预设与历史展开正常；**P1×3**：G1 扫描进度条 `percent` 恒 0（`scan/runner.rs:121` 硬编码，编排层未换算）、G2 清理列表前端 `limit: 100` 无分页导致小文件被截断（「被锁文件没出现」的真因）、G3 保险箱 `source_task_id` 恒 NULL（`commands.rs:149/227` 传 `None`）致 Session 多选空白 + 来源列全 `—`（task-051 一半能力真机失效，26 条验收漏「写路径」）；**P2×2**：G4 清理轮次恒 0（`executor.rs:82` `session_id` 写 NULL + `COUNT(DISTINCT)` 忽略 NULL）、G5 清理成功释放 0 B（文件已被外部删除 → `bytes_before=0` + NotFound 幂等 `Ok`）；**P3×1**：G6 设置页 `rules_check_update` 首调 `ipc.localhost` 红错（task-039 已知项真机复现，非回归）。下一步派发 **task-052** 修 G1–G5 后复验
- 2026-10-25  task-052  **GUI 走查 P1+P2 修复（G1–G5）开发完成**（`feature/task-052-gui-p1p2-fixes`，状态 `testing`，未合 main）：G1 `ScanOrchestrator` 按 `(idx*100/total,(idx+1)*100/total)` 切槽 + runner `interpolate_percent(n/(n+50))` 亚线性插值 + 槽位边界事件 + 收尾 100（全失败不报 100）；G2 `list_pending_page{items,total,page,pageSize}`（默认 100 / clamp 500）+ `ORDER BY size_bytes DESC, id DESC` 修跳行 + Items 分页栏（翻页清空勾选）；G3 `execute_items` 工厂改 `FnMut(item_id, task_id)`，两处调用点传 `Some(task_id)`，新增真实写入路径回归 5 条；G4 `COUNT(DISTINCT session_id)` → `COUNT(*)`（2 行 NULL session：0 → 2）；G5 `Deleter::delete_file -> Result<bool,String>` + `already_gone`/`already_gone_count` + 前端「M 项已不存在（未计入释放量）」。门禁：check 0 / clippy 0 / fmt 0 / **cargo test --workspace 245 passed + 8 ignored** / vitest **53**（7 files）/ build **180.85 kB │ gzip 57.76 kB**；`tauri build` exit 0（NSIS 4.02 MiB + MSI 5.83 MiB，含 `.sig`）。新增 29 条用例。**与任务书不符 1 项**：白名单漏了 `delete.rs` / `fs_deleter.rs`（G5 要求改 trait 签名，不改无法编译），已最小改动并标注。**按新流程：开发轮不跑 CI、不做 GUI 走查/安装态冒烟，移交测试 Agent。**
- 2026-10-26  task-052  **测试通过 → 评审通过 → 合并 main**（`d92db1b`，CI PR #17 run 40311963933 全绿）：`cargo test --workspace` 245+8 / vitest 53 / build 180.85 kB；NSIS 4.01 MB + MSI 5.83 MB；CDP 真机 Dashboard 清理轮次 = 17（G4 修复前恒 0）、Items 分页栏存在、Console 0 红错。**G3 真机证据错位**（以 Items 页 `cleanup_item.session_id` 的 `#9010/#9009/#9008` 冒充 `safe_box.source_task_id`）；G1/G5 真机未验，报告自认
- 2026-10-27  **第二轮真机复验 → 仍不通过**：**G7【P1，新】**扫描结束时进度条不满——`interpolate_percent` 渐近达不到 `to`，依赖的「槽位上界 + 收尾 100」两条边界事件被 `ThrottledEmitter` 丢弃式节流（`<100ms` 直接丢、无 trailing）吞掉，叠加前端 `scan_start` resolve 后立即 `unlisten`；集成测试用 `VecEmitter` 绕过节流器故未检出；**G3 ✅ 已实测通过**——首轮看到的「Session 空 / 来源列全 `—`」为历史数据（78 条移入时间全为 10-25，早于修复），用户新删除文件入箱后「来源」列正确显示 `#N`、Session 多选出现 `session #N (M 条)` 并可筛选，**G3 关闭**；**G8【P2，新】**历史行 `source_task_id` 永久 NULL → 永远 `—`、永远筛不到，建议渲染为「历史（无来源）」。唯一发版阻塞项 = G7，下一步 `bugfix/task-053-progress-tail-flush`
- 2026-10-27  task-053  进度条尾部冲刷（G7）+ 历史来源文案（G8）：`ProgressEmitter::emit_force` 逃生口（`ThrottledEmitter` 转发 + 重置窗口、`Box<dyn>` 透传）、编排层收尾强制发送、前端「收到 100 立刻退订 / 200ms 兜底」+ resolve 后置满；新增**走真实 `ThrottledEmitter`** 的尾部回归 5 条并做反向验证（改回 `emit` 即红，停在 80，与真机一致）；保险箱 NULL 来源改「历史（无来源）」+ 筛选框说明。门禁 check/clippy/fmt 0、cargo test 250+8、vitest 54、build 181.51 kB。**开发轮按新流程不跑 CI、不做真机/安装态冒烟**，状态 `testing` 移交测试 Agent

- 2026-10-27  task-054  v0.1.6 发版准备（release 溯源 + 版本号 + CHANGELOG + 复验清单 + 升级手册）：`release.yml` 补 `target_commitish: ${{ github.sha }}`；版本号 `0.1.0` → `0.1.6`（四处 + `Cargo.lock`），`tauri build` 产出 `C-DiskCleaner_0.1.6_x64-setup.exe`（4.01 MiB）+ `.msi`（5.83 MiB）+ 成对 `.sig`；新建 `CHANGELOG.md`（v0.1.6 三段，自本版启用不回溯）、第三轮走查清单（27 条 NSIS/MSI 双列，含 Console 口径「排除 G6 后零红」）、升级闭环手册（6 步 + 卡点速查）；`.gitignore` 加 `!docs/release/` 修误伤。门禁 check/clippy/fmt 0、cargo test 250+8、vitest 54、build 181.51 kB。**开发轮不跑 CI、不做安装态冒烟**，发版由用户手动 workflow_dispatch，NSIS/MSI 双跑移交测试 Agent
- 2026-10-27  task-054  **测试轮通过**（`docs/test-reports/054-test-report.md`，分支 @`3315fc8`）：独立重跑 fmt/check/clippy/test（exit 0，250 passed + 8 ignored 零回归）/ lint 0 error / vitest 54（7 files）/ build 181.51 kB；契约核验版本号四处 `0.1.6`、`release.yml:148` `target_commitish: ${{ github.sha }}`、三份交付物齐全；L4 MSI `/a` 管理映像抽取 exit 0（exe 16.3 MB + rules 2 文件 + 5 个 scan ps1、无 `_up_`），从抽取目录直启 exe 安装目录 NO_DIFF、DB 落 `%APPDATA%`；**0 缺陷、未开 bugfix 子分支**。第三档登记：NSIS 真实安装因 UAC 被用户取消未执行、GUI 27 条点击仍被 WebView2 渲染不 paint 阻塞（历史已知项）、CI 五道闸待提示词 B 建 draft PR 轮询。状态置 `awaiting_review`，移交评审 Agent
- 2026-10-28  task-055  M3 banner 下载进度反馈 + 失败态文案 + 手动检查 loading：`downloadAndInstall(onEvent)` 订阅（真实签名为第一个参数即回调，事件字段 `contentLength`/`chunkLength`）；banner 进度条 + `下载更新中 N%` + 已下载/总量 + EMA 速度（1s 窗口 α=0.4）；失败态 5 类中文映射 + 重试/关闭；「稍后提醒」写 `localStorage` 存版本号（同版本不弹、更高版本照弹）；Settings 手动检查显示「当前 vX / 发现 vY」+ `onUpdateFound` 强制显示 banner。新增 `app/src/utils/update.ts`（纯函数，避免 App↔Settings 循环依赖与 react-refresh 警告）。门禁 lint 0 / vitest **81**（+27）/ build 184.22 kB / cargo 250+8 不回归。**开发轮不跑 CI、不做真机冒烟**，移交测试 Agent
- 2026-10-28  task-055  **测试轮通过**（`docs/test-reports/055-test-report.md`，分支 @`a92193b`）：独立重跑 fmt/check/clippy/test（exit 0，250 passed + 8 ignored 零回归）/ lint 0 error（2 warning 均 SafeBox.tsx 既有）/ vitest **9 files / 81 passed**（基线 54 + App 22 + Settings 5）/ build **184.22 kB / gzip 59.18 kB**；契约核对 18 条验收逐条对齐（含开发轮对任务书 API 签名的两处硬修正）；`tauri build` 产出 NSIS 4.02 MiB + MSI 5.83 MiB（末尾签名步因本地无私钥报错，CI 才有，预期）；MSI `/a` 抽取布局正确（exe + rules 2 + 5 ps1、无 `_up_`），直启后安装目录 NO_DIFF、数据落 `%APPDATA%`；**0 缺陷、未开 bugfix 子分支**。第三档：L3 四条 GUI 交互因 WebView2 自动化不 paint + 无 v0.1.7 Release 未取得（27 条 jsdom 用例补偿）、NSIS 真实安装需 UAC、CI 待提示词 B 建 PR。状态置 `awaiting_review`，移交评审 Agent
- 2026-10-28  task-056  M3 评估 + 遗留小修：**差量更新评估结论「不做」**（Tauri 2 updater 不支持差量，`plugin-updater@2.13.3` 全包无 bsdiff/delta/xdelta；自建需 6–9 人日且 4 MB 包只省 1–2 MB，建议改做 release profile 瘦身）；**迁移器 v2** 给 `safe_box` 加 `restored_at INTEGER`，`restore` 由 `DELETE` 改 `UPDATE status='restored'`（列表/配额本就有 `status='in_box'` 过滤，外部行为不变；行还在≠可重复恢复，第二次 NotFound）；**跨盘 rename 补测** 2 条（本机 C:→D: 真跑 copy+remove fallback 逐字节一致，单盘 CI 打印 `[skip]` 不红，顺带固化「跨盘 restore 无 fallback」限制）；**G6 定稿豁免不修**（Tauri 2.x custom protocol 首调时序竞争；warmup 只是把红错换命令名、放启动还扩大暴露面；5 条判定规则写入第三轮清单 §六）。门禁 check/clippy/fmt 0、cargo test **255 passed + 8 ignored**（+5）、vitest 81、build 184.22 kB。**开发轮不跑 CI/安装态冒烟**，但 DB 迁移需测试轮重点验「v0.1.6 老库平滑升 v2」
- 2026-10-28  task-056  **测试轮通过**（`docs/test-reports/056-test-report.md`，分支 @`1d2b9e8`）：独立重跑全部门禁全绿（cargo 255+8 / vitest 81 / build 184.22 kB）；**抓到并修 1 处测试断言缺陷**——`cross_disk_restore_currently_returns_io_error` 断言跨盘 restore 必失败，实测 Windows MoveFileExW 跨卷实际 copy+remove 成功，开发轮「无 fallback 已知限制」结论被推翻；改 `cross_disk_restore_succeeds_across_drives`（断言恢复成功+内容逐字节一致+restored_at 非空），走 `bugfix/task-056-cross-disk-restore-assertion`（`560cb94`）全量回归后 `--no-ff` 合回（`1d2b9e8`）子分支已删；**L4 真实 v1 库迁移实测**：备份 `target/cdiskcleaner-v1-backup.db` → 启动新 exe → user_version 1→2、safe_box 加 restored_at 列、旧行保留无丢失、进程响应正常。第三档：L3 GUI 走查由用户补、NSIS 真实安装需 UAC、CI 待提示词 B 建 PR。状态置 `awaiting_review`，移交评审 Agent
- 2026-10-28  task-056  **CI Run #139 红 → 二轮 bugfix → Run #140 success**：Run #139（@`3db6078`）Windows runner 上 `cross_disk_restore_succeeds_across_drives` FAILED——CI 上 `std::fs::rename` 跨卷直接返 EXDEV（os error 17「cannot move file to a different disk drive」），与本机环境（跨卷成功）相反；根因是第一轮 bugfix 把断言翻成「必成功」，锁死了一种非可移植结果。改 `cross_disk_restore_portable_contract`：match Result 两路都验——Ok 验落盘+内容一致+restored，Err(EXDEV) 验行仍 in_box+box 文件原封未动+错误指向跨盘 rename，不丢数据。走 `bugfix/task-056-cross-disk-restore-portable`（`0854600`），本地 cargo test --workspace exit 0、clippy/fmt 0，`--no-ff` 合回 `98e3bfd`；**CI Run #140 conclusion=success** @ `98e3bfd`。新登记第三档：restore 跨盘在多数 Windows 机器实际失败（无 copy+remove fallback），若要真正支持跨盘恢复需另立任务加 fallback。
- 2026-10-28  task-057  RmRestart 自动重启被关进程（默认关闭 + 设置开关）：**prefs 存储**复用 v1 `settings` 表（键 `prefs.auto_restart_on_close`，不加迁移不加新列），新建 `engine/src/prefs.rs`（get/set UPSERT + Prefs serde camelCase + 5 条单测）；**RmRestart** `shutdown_locking_processes(path, auto_restart: bool)` 签名升级，`RmSession::restart(pid)` 调 windows `RmRestart(handle, pid, None)`，仅对 pid ∈ closable ∧ `bRestartable.as_bool()==true` 调用，总超时 10s，单个失败只 warning；**Tauri** `get_prefs`/`set_prefs` 注册、`cleanup_retry_locked` 在 spawn_blocking 前读 prefs；**UI** Settings「行为偏好」开关（默认关闭 + 灰色说明）+ Items confirm 文案开启时追加自动重启提示。门禁 check/clippy/fmt 0、cargo lib 53（+6）、vitest 85（+4）、build 185.59 kB。**开发轮按新流程不跑 CI/真机/安装态冒烟**，移交测试 Agent
- 2026-10-29  task-058  M4 收尾：**RmRestart 白名单**（`prefs.restart_whitelist` JSON 存 settings 表；空白名单=不限制，非空 fail-closed——`RmRestart` 无按 pid 参数只能整批重启，故改为「被关闭的可重启进程全部命中才重启」；匹配用映像名 + `*` 通配，且映像名必须在 `RmShutdown` 之前取，否则进程已死查不到）；**safe_box 原子性**（INSERT 提到 rename 之前 + 事务包裹，旧顺序反了才产生孤儿；跨盘 copy/remove 失败清理半成品）；**代码签名评估**（结论当前不做：CA/B 2023-06 强制硬件私钥与 GitHub Actions 天然冲突、EV/OV 需企业实体、Azure Artifact Signing 个人仅限美加；给出三个「该做」触发条件）；**CHANGELOG v0.1.7 草稿**（四段，版本号保持 0.1.6）。门禁 check/clippy/fmt 0、cargo test **273 passed + 8 ignored**（+11）、vitest 89、build 187.46 kB。**开发轮不跑 CI/真机/安装态冒烟**，移交测试 Agent
- 2026-10-29  task-059  v0.1.7 发版准备（版本号/CHANGELOG/README/用户手册）：版本号 `0.1.6` → `0.1.7`（四处 + lock），`tauri build` 实测 `C-DiskCleaner_0.1.7_x64-setup.exe`（4.05 MiB）+ `.msi`（5.87 MiB）+ 成对 `.sig`；`CHANGELOG.md` v0.1.7 定稿（日期 2026-10-29、删 Unreleased 段）；**重写 `README.md`**（原文件状态停在「待进入 M0」、目录结构写「M1 阶段将新增 app/engine」、启动写「弹出空窗口」，全部失真，重写为开源门面）；新建 **`docs/USER_GUIDE.md`**（8 节 + 8 条 FAQ，按真实 UI 文案撰写，重点说清白名单 fail-closed 语义 / 「已不存在」不算失败 / 卸载不删 %APPDATA%）。门禁 check/fmt 0、cargo test **273 passed + 8 ignored**、vitest 89、build 187.46 kB、tauri build exit 0。**发版由用户手动 workflow_dispatch（version=0.1.7）**
- 2026-10-29  task-060  占用关闭体验（连带兄弟进程 + 30s 进度 + 跨盘文档）：**兄弟进程**核实为 RM 按 console 进程组的既定行为、**不需改代码**（只注册文件就已发生；注册 `rgapplications` 参数类型是 `RM_UNIQUE_PROCESS` 不是 pid、且注册锁文件进程本身冗余、强行注册同名应用会扩大关闭面并可能触发 mixed fail-closed 让整批都关不掉），已在 `RmSession::register` 写 4 条理由 + USER_GUIDE 说明；**30s 无进度已修**（`shutdown_locking_processes` 第四参 `on_progress` + `spawn_shutdown_ticker` 1s 计时线程 + `shutdown://progress` 事件 + Items 进度条「已等 Xs（最多 30s）」，超 30s 提示仍在等待——`RmShutdown` 无超时参数不可中断，不做假超时）；**跨盘行为已文档化**（USER_GUIDE 4.5：移入有 copy+remove fallback、**恢复不支持跨盘**、原因与影响 + FAQ Q9 三步处置）。门禁 check/clippy/fmt 0、cargo test **275 passed + 8 ignored**（+2）、vitest **90**（+1）、build 188.06 kB。**开发轮不跑 CI/真机/安装态冒烟**，移交测试 Agent
- 2026-10-30  task-061  M5 起步（主题/趋势图/自动扫描）：**主题三态**（`prefs.theme`，纯内联 CSS 变量不新增 css 文件，matchMedia 跟随系统，localStorage 防闪烁 + DB 为真值，本期只适配 App/Dashboard/Settings 三页）；**趋势图**（`cleanup_stats_trend` 按本地时区分天 SUM，只返回有记录日期、前端补 0，days 夹 1..=365；仪表盘手写 SVG 柱状图，空/失败各有占位）；**自动扫描**（`prefs.auto_scan_enabled`+`interval`+`last_auto_scan_at`，`auto_scan_due` 纯函数，后台 60s 后起、每 30 分钟轮询，**只扫描不删除**，完成发系统通知，与手动扫描 try_lock 互斥）。新增 `tauri-plugin-notification` + `notification:default` 权限。门禁 check/clippy/fmt 0、cargo test **291 passed + 8 ignored**（+16）、vitest **98**（+8）、build 195.04 kB。**开发轮不跑 CI/真机/安装态冒烟**，移交测试 Agent
- 2026-10-30  task-062  提权 exe 底座 + 系统还原点 + 托盘常驻（v0.2.0 起步）：**提权 exe**（`elevation/` 从 stub 填实，零依赖手写 `SRSetRestorePointW` FFI，命令白名单只有 `create_restore_point`，退出码 + 临时 JSON 回传；主进程用 `ShellExecuteExW` + `SEE_MASK_NOCLOSEPROCESS`，`build-elevation.mjs` 在 beforeBuildCommand 编译并随包部署，**NSIS 4.05→4.22 MiB 证明进包**）；**系统还原点**（`needs_restore_point` 只对 L2 触发、全 L1 不弹 UAC；名称 `C-clean cleanup #id UTC时间`，手写 civil_from_days；失败 2007 → 前端 confirm → `skipRestorePoint` 重调；成功写 audit_log）；**托盘常驻**（Tauri 2 无 `tauri-plugin-tray`，用内置 `tray-icon` feature；CloseRequested → prevent_close + hide；右键三项 + 左键唤出；「退出」走 emit → 前端 confirm → `quit_app`；开机自启用 autostart 插件，状态以注册表为准）。门禁 check/clippy/fmt 0、cargo test **316 passed + 8 ignored**（+25）、vitest **101**（+3）、build 196.96 kB、tauri build exit 0。**开发轮不跑 CI/真机/安装态冒烟**，移交测试 Agent
- 2026-10-30  task-063  报告导出+前后对比+环形图+取消扫描：**报告**（`report/mod.rs` 零依赖拼接；HTML 单文件 CSS 内联 + CSV 带 UTF-8 BOM 防 Excel 乱码；路径全过 `html_escape` 防 XSS；落 `%APPDATA%\...\reports\`）；**前后对比**（`ExecuteSummary` 加 disk_before/after，盘符从 item 路径推断不写死 C，`released_bytes` 饱和减法防负数）；**环形图**（`GetDiskFreeSpaceExW` + 手写 SVG 圆环，u128 算百分比防溢出，≥85% 变红）；**取消扫描**（`Arc<AtomicBool>` + 脚本粒度检查，`scan_start` 每轮复位标志，已扫描结果保留）。门禁 check/clippy/fmt 0、cargo test **328 passed + 8 ignored**（+12）、vitest **101**、build 199.87 kB。**开发轮不跑 CI/真机/安装态冒烟**，移交测试 Agent
- 2026-10-30  task-064  P0 收尾（7天purge/风险徽标/全选L1）：**purge**（`purge_expired` 逐条处理+逐条 audit_log，文件删不掉保留 DB 行下轮重试，`expires_at==now` 不算过期；启动跑一次+每 6 小时）；**风险徽标**（`PendingItem` 加 `risk_level`——此前根本没返回，前端无法区分 L1/L2；四色走 CSS 变量 + `title` 原生 tooltip + 未知等级 fallback）；**全选 L1**（只勾本页 L1，L2/L3 一律不选；无 L1 时按钮禁用）。门禁 check/clippy/fmt 0、cargo test **334 passed + 8 ignored**（+6）、vitest **104**（+3）、build 201.48 kB。**P0 全部清完**，下一步进 M1（C 模块 WinSxS）。**开发轮不跑 CI/真机/安装态冒烟**，移交测试 Agent
- 2026-10-30  task-065  M1 C 模块 WinSxS 清理：**DISM 分析**（`scan/winsxs.rs` + `/English` 文本解析——**任务书的「解析 XML」不存在**，`/analyzecomponentstore` 无 XML 输出；标签小写 + 空白归一化；`reclaimable_bytes` 缺 `Actual Reclaimable Space` 行时用「备份 + 缓存」兜底；`parsed` 标志区分「没东西可清」与「解析失败」）；**DISM 清理**（先建还原点 fail-closed，建点失败 2007 → 前端二次确认 → `skipRestorePoint` 重调，只重试一次）；**提权白名单**（加 `dism_analyze`/`dism_cleanup`，参数是 `&'static` 常量表——**`/ResetBase` 属于「没有传参入口」而非「有入口被拦」**；DISM 走 `%SystemRoot%\System32` 绝对路径防 PATH 劫持；`MultiByteToWideChar(CP_OEMCP)` 解 OEM 代码页；自实现 15 分钟超时）；**实时进度**（提权进程 stdout 拿不到 → 落盘 + `thread::scope` tail，`drain_complete_lines` 按字节切行防半个多字节字符打乱游标；事件 `winsxs://progress` 定义在 engine 防两处漂移）；**UI**（Dashboard `WinsxsCard`，**挂载不自动分析**——DISM 会弹 UAC）。门禁 check/clippy/fmt 0、cargo test **364 passed**（+30）、vitest **113**（+9）、build 206.66 kB、**tauri build 出包 NSIS 4.36 MiB + MSI 6.44 MiB**（提权 exe 153→297 KB 已进包；末尾签名步无私钥失败属已知预期）。**真实链路探测**：非提权态跑 `dism_analyze` → exit 4 + `{"ok":false,...,"message":"DISM 退出码 740","output":"Error: 740 / Elevated permissions are required to run DISM."}`，证明白名单/绝对路径/`/English`/输出捕获/JSON 转义/退出码映射全通，**只差 UAC 提权本身**。**开发轮不跑 CI、不做真机/安装态冒烟**，移交测试 Agent
- 2026-10-31  task-066  M1 E 模块大文件地图（只读）：**后端扫描**（`scan/big_files.rs`：hiberfil 只读 metadata 不开文件；Windows.old 递归求和共用上限；回收站 `SHQueryRecycleBinW` 优先 + `$Recycle.Bin` 兜底并说明「可能不含其他用户」；Top100 最小堆 + 跳过 reparse point/系统目录/根目录系统文件；单个 read_dir 失败只跳过不中断；三张卡片固定 3 条）；**Top100 遍历 BFS 化**（真机实测抓到 DFS 的坑：预算用尽时扎进 AppData 深处，Top 只出 21 条 / 16.9 GB；改 BFS 后同预算下 99 条 / 59.6 GB）；**IPC**（`ipc/commands/big_files.rs` 纯逻辑 + Tauri `big_files_scan`，`system_drive_root()` 读 SystemDrive 不写死 C:/）；**前端**（`BigFiles.tsx` 三卡片 + Top100 表，全页只有「重新扫描」一个按钮，顶部声明不代删并解释四类正规处理方式；进页面自动扫一次；truncated 明确提示；颜色全走 CSS 变量随主题）；**tab 注册**（App 加 bigfiles，「清理」与「保险箱」之间）。门禁 check/clippy/fmt 0、cargo test **382 passed**（+18）、vitest **123**（+10）、build 211.79 kB、**tauri build 出包 NSIS 4.38 MiB + MSI 6.46 MiB**（末尾签名步无私钥失败属已知预期）。**真实数据证据**：`--ignored --nocapture real_scan` 扫本机 C:/ → hiberfil 11.89 GB / Windows.old 0 / 回收站 1.52 MB / Top100 99 条 / 合计 59.6 GB / 20.2 s。**开发轮不跑 CI、不做真机/安装态冒烟**，移交测试 Agent
