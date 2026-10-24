# 测试报告：task-050 打包资源路径错位修复

> 测试 Agent（提示词 9），独立于开发轮执行：**不复用**开发轮自验数字，L1–L5 全部门禁在本机从头复跑。
> 被测分支：`bugfix/task-050-bundle-resources-path` @ `e7171c4`（本地 = `origin`，`git rev-list --left-right --count` → `0 0`）。
> 上游：`docs/PROJECT_STATE.md`「已知问题 · 打包态资源路径错位」→ `tasks/050-bundle-resources-path.md`（实现备注）。
> 测试日期：2026-10-24 19:48–20:30（Windows 11 / 用户 `An`，非管理员会话，Session 9 Console）。
> **分支形态说明**：本任务按任务书「hotfix 语义」直接从 `main`（`b547988`）拉 `bugfix/task-050-*`，**不存在 feature 分支**（先例 `bugfix/task-035-release-signing`），故下文「feature 分支」一律指该 bugfix 分支。
> **同源测试声明**：开发轮由 DevEngineRust 执行、测试轮为独立轮次，开发轮的「六道门禁 / 安装布局 / CI run」数字全部丢弃并重跑；两轮由同一模型家族执行、同一台机器，仍存在「同一盲区被两轮共享」的风险，评审时请按此折扣看待「零缺陷」结论。

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
|---|---|---|---|
| dev（本地启动） | `pnpm tauri dev`（vite 5173 + `target/debug/cdisk-cleaner.exe`）；WebView2 运行时 155.0.4283.62 | 是 | **不通过（环境阻塞，第三档）**：`[paths]` 日志与窗口句柄正常，但 WebView2 渲染进程在启动后 3–6 秒内消失，窗口留白、CDP 9222 不监听、Crashpad 每次启动新增 `.dmp`；**未取得 Console 证据，不伪造** |
| 打包安装（干净目录） | `pnpm --filter app tauri build` → NSIS 4,199,554 B（4.01 MiB）+ MSI 6,094,848 B（5.81 MiB）；NSIS `/S /D=%TEMP%\cclean-test-050`；MSI `msiexec /a` 抽检 | 是 | **通过（文件系统层）/ GUI 渲染同 dev 阻塞** |
| CI | workflow `ci`；run **40090971932**（head `e7171c4`） | 是 | **绿**（单 job `build`，含「Tauri 资源映射契约」步骤 success） |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据（日志/截图/报告路径） |
|---|---|---|---|---|
| L1 单元 | Rust 全量（workspace，非只跑改动文件）+ 前端全量 + 契约测试 | `cargo test --workspace`；`pnpm --filter app test`；`node --test scripts/tauri-resources.test.mjs` | **全绿**：cargo **197 passed / 0 failed / 8 ignored**（39 个测试二进制）；vitest **34 passed（6 files）**；契约 **6 pass / 0 fail** | `/tmp/cargo-test-full.log`；条数汇总 `passed=197 failed=0 ignored=8`；engine 基线 188+8 不回退，app crate `paths::` **9 passed** |
| L2 集成/契约 | `bundle.resources` ↔ `AppPaths` 落位契约；`commands.rs` 消费方；守卫有效性（变异测试）；资源缺失负向用例 | `node --test scripts/tauri-resources.test.mjs`（缺陷态/修复态各一次）；`cargo test -p cdisk-cleaner paths::`；删除安装目录 `scripts/` 后启动安装版 | **通过**：缺陷态（数组式）**6 条红 5 条**、修复态 6/6 绿；`paths::` 9 条全绿；缺失 `scripts/` 时启动即报 `找不到同时含 rules/ 与 scripts/ 的资源目录：\\?\C:\...\cclean-test-050-broken 下缺少 scripts/；安装包可能不完整，请重新安装` 并退出（不再静默回退 `.`） | 变异测试输出见 §3 C12；负向 stderr 原文见 §3 B7 |
| L3 端到端/UI（dev） | 启动 → `[paths]` 日志 → 窗口/Console | `pnpm tauri dev` | **阻塞（第三档）**：`[paths]` 为绝对路径（E17 通过）；窗口标题 `C-Disk Cleaner`、hwnd 存在；但 WebView2 子进程 3s 时 4 个 → 6s 时 0 个，CDP 9222 未监听，Crashpad 于 19:57 / 20:01 / 20:06 / 20:09 各落一个 6.7–6.9 MB `.dmp`；追加 `--disable-gpu --disable-software-rasterizer` 与 `--remote-debugging-port=9222` 均无效 | 见 §3 E20；**未取得截图与 Console，无编造** |
| L4 真机/打包安装 | 干净目录安装 → 资源布局 → 首次启动重建 rules → 安装目录零写入 → 诱饵目录防劫持 → MSI 抽检 → 脚本可执行 | `tauri build`；`setup.exe /S /D=...`；`msiexec /a`；启动安装版（cwd 指向诱饵目录）；`Get-FileHash` 前后比对 | **通过**：安装 exit 0，`<INSTDIR>` 恰好 **9 文件**（2 rules + 5 scripts + exe + uninstall），`_up_` 不存在；MSI 解包 **8 文件**（exe + 7 payload）同布局、无 `_up_`；`[paths] scripts_dir=\\?\...\cclean-test-050\scripts`（安装目录，非诱饵 cwd）；`%APPDATA%\...\rules` 被重建（614 + 261 B）且与安装包 SHA-256 逐字节一致；安装目录 9 文件 **NO_DIFF**；`data\cdiskcleaner.db` 438,272 B 落在 APPDATA | 见 §3 A3 / E17–E19 |
| L5 CI 门禁 | 本分支最新 run | GitHub Actions API：`/actions/runs/40090971932/jobs` | **绿**：`conclusion=success`，第 8 步「Tauri 资源映射契约」success，第 17 步「Tauri build」success | run 40090971932；分支上三次 run（40090245117 / 40090552969 / 40090971932）全 success |

### L2 契约核对表（逐字段）

| 核对项 | 后端 | 前端 | 结论 |
|---|---|---|---|
| 打包资源键 ↔ 定位前缀 | `tauri.conf.json bundle.resources` 目标全部以 `rules/`、`scripts/` 开头（7 条） | — | 对齐：`pick_resource_root` 只认 `resource_dir()/rules` + `/scripts`，两者同名前缀一致 |
| 资源消费方 | `commands.rs:78` `orch.run_all(&paths.scripts_dir)`（唯一 scripts 消费者）；`commands.rs:37/114/138/175/377/445/461/478` 消费 `paths.rules_dir` | 前端 `app/src/**` 无任何 `scripts_dir` / `resource_dir` / `_up_` 引用（`grep` 0 命中） | 对齐：路径只在 Rust 侧解析，前端不感知布局 |
| 数据落点 | `data_root = app_data_dir()` → `%APPDATA%\com.and0114.cclean`；`rules_dir`/`box_dir`/DB 全在其下 | — | 对齐：实测 `data\cdiskcleaner.db`、`rules\`、`safe_box\` 均在 APPDATA，安装目录零写入 |
| 命令签名/Envelope | 本任务**未改** `commands.rs` / `lib.rs` / `progress.rs`（`git diff --name-only main..HEAD` 不含） | 无改动 | 无影响（回归由 197 条全量用例覆盖） |
| 事件 payload | 未新增/改动 Tauri 事件 | 无新增监听 | 无影响 |
| 失败语义 | 资源缺失 → `setup` 返回 `Err` → Tauri `Failed to setup app` → 进程退出 | 前端不启动（无白屏无报错，直接无窗口） | 对齐：负向用例实测退出，非静默降级 |

## 3. 验收标准逐条核对（每条必须有运行时证据）

### A. 资源落位（根因修复）

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
|---|---|---|---|
| A1 | `bundle.resources` 为映射对象，7 条目标路径固定 | `node -e` 读配置 → `count=7`；键值逐条与任务书 A1 完全一致（`../../rules/cache_rules.json → rules/cache_rules.json` … `../../scripts/scan_dev_cache.ps1 → scripts/scan_dev_cache.ps1`） | 通过 |
| A2 | 不得用通配符整目录搬运 | 契约测试用例「不得用通配符整目录搬运」绿；7 条源全部是具名文件，`grep "*"` 无命中 | 通过 |
| A3 | 安装后 `<INSTDIR>/rules/*.json` + `<INSTDIR>/scripts/*.ps1` 齐全、`_up_` 不存在；NSIS 必做、MSI 抽检 | NSIS `/S /D=%TEMP%\cclean-test-050` → `INSTALLER_EXIT=0`、`INSTALL_DIR_FILES=9`：`cdisk-cleaner.exe`、`uninstall.exe`、`rules\cache_rules.json`(614)、`rules\rules_meta.json`(261)、`scripts\scan_{browser_cache,dev_cache,prefetch,temp,windows_update}.ps1`；`_UP_EXISTS=False`。MSI `msiexec /a … TARGETDIR` → `MSIEXEC_EXIT=0`、`MSI_FILES=8`，`PFiles\C-DiskCleaner\{rules,scripts}` 同布局，`_up_` 目录数 0 | 通过（**MSI 抽检本轮已补做**，关闭开发轮遗留） |
| A4 | fallback B 仅在 A 不可行时启用，且不得双份真相 | 走 A 路线；`paths.rs` 无 `_up_` 候选根（`grep _up_` 仅出现在注释）；`tauri.conf.json` 无数组式残留 | 通过（N/A） |

### B. `AppPaths` 资源定位加固

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
|---|---|---|---|
| B5 | 抽 `pick_resource_root` 纯函数，`from_app` 只产候选 | `paths.rs:54-59` 纯函数；`from_app` 调 `build_candidates` + `pick_resource_root`；单测 5 条覆盖（只有 rules / 只有 scripts / 两者齐 / 全空 + 同名文件 / 多候选跳过不完整） | 通过 |
| B6 | release 禁止 `current_dir()` | `paths.rs:130-133`：`#[cfg(debug_assertions)] let dev_cwd = current_dir().ok();` / `#[cfg(not(debug_assertions))] let dev_cwd: Option<PathBuf> = None;`；`build_candidates:71` 的 `if cfg!(debug_assertions)` 门控追加 cwd 三级 → release 分支代码上一眼可见不碰 cwd。**运行时反证**：安装版从诱饵目录 `%TEMP%\cclean-test-decoy`（内含假 `rules/cache_rules.json`=`{"decoy":true}` + 假 `scripts/scan_temp.ps1`）启动，`[paths] scripts_dir=\\?\C:\Users\An\AppData\Local\Temp\cclean-test-050\scripts`（安装目录），`%APPDATA%\...\rules\cache_rules.json` 头部为 `"version": "1.0.0"` 而非 `{"decoy":true}` | 通过 |
| B7 | 删 `.` 兜底；缺失返回 `Err` 且消息含绝对路径与缺失项 | `grep 'PathBuf::from(".")'` → NONE。负向实测：把安装目录副本的 `scripts\` 删除后启动 → 进程退出，stderr：`thread 'main' panicked … Failed to setup app: error encountered during setup hook: 找不到同时含 rules/ 与 scripts/ 的资源目录：\\?\C:\Users\An\AppData\Local\Temp\cclean-test-050-broken 下缺少 scripts/；安装包可能不完整，请重新安装` | 通过 |
| B8 | `data_root/rules` 首次复制失败必须 `Err`；启动日志打印绝对路径 | 复制失败路径 `paths.rs:143-149` 为 `map_err` → `Err`（不吞）；首次启动实测 `[paths] data_root=C:\Users\An\AppData\Roaming\com.and0114.cclean rules_dir=…\rules scripts_dir=\\?\…\cclean-test-050\scripts box_dir=…\safe_box`，全部绝对路径（缺陷态此处为 `.\scripts`） | 通过 |
| B9 | 单测 ≥ 4 条、不依赖真实 cwd/安装目录、不新增依赖 | `cargo test -p cdisk-cleaner paths::` → **9 passed / 0 failed**（`pick_resource_root_*` 5 条、`build_candidates_*` 2 条、`describe_missing_*` 2 条）；用 `std::env::temp_dir()` + `Drop` 守卫，未引入 `tempfile`（`app/src-tauri/Cargo.toml` 与 `Cargo.lock` 零 diff） | 通过 |

### C. 防回归守卫

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
|---|---|---|---|
| C10 | 新增 `scripts/tauri-resources.test.mjs`（`node:test`，零依赖）6 条断言 | 文件 88 行，`import test/assert/fs/path/url` 全为 Node 内置；6 条用例名与任务书 C10 四条一一对应并加强（映射条数/通配符/源文件存在） | 通过 |
| C11 | CI 在「发版脚本单测」后新增「Tauri 资源映射契约」一步，只加这一步 | `git diff main..HEAD -- .github/workflows/ci.yml` → `+5` 行，仅插入该 step；CI run 40090971932 第 8 步「Tauri 资源映射契约」success | 通过 |
| C12 | 守卫有效性自证（旧数组式应失败） | 把 `tauri.conf.json` 临时改回 7 条数组式 → `node --test` 结果 **6 条红 5 条**（对象形态 / 条数 / 目标前缀 / 不得含 `..`/`_up_` / 源文件存在全红，仅通配符那条绿），报错含 `bundle.resources 必须是 {源: 目标} 对象；数组式会保留原目录结构并生成 _up_ 前缀`；`git checkout` 还原后 6/6 绿、`git status` 干净 | 通过 |

### D. 卫生

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
|---|---|---|---|
| D13 | 不新增 npm / cargo 依赖 | `git diff main..HEAD --stat -- app/src-tauri/Cargo.toml Cargo.lock pnpm-lock.yaml` → 空 | 通过 |
| D14 | 只改允许范围内文件 | `git diff --name-only main..HEAD` → 6 个：`ci.yml`、`paths.rs`、`tauri.conf.json`、`PROJECT_STATE.md`、`scripts/tauri-resources.test.mjs`、`tasks/050-*.md`，与任务书允许表逐条对齐 | 通过 |
| D15 | 六道门禁全绿、基线不回退 | `cargo fmt --all -- --check` **0**；`cargo clippy --workspace --all-targets -- -D warnings` **0**；`cargo test --workspace` **197 passed / 8 ignored**（engine 188+8 不回退，app +9）；`pnpm --filter app lint` **0**；`pnpm --filter app test` **34 passed**；`pnpm --filter app build` **0** → `index-VkBRTSbR.js 176.39 kB │ gzip 56.25 kB`（基线一致） | 通过 |
| D16 | `tauri build` 出包成功，NSIS + MSI 均有产物 | 设一次性临时密钥（`%TEMP%\cclean-test-050.key`，未入库）后 `pnpm --filter app tauri build` **exit 0**：NSIS 4.01 MiB + `.sig`、MSI 5.81 MiB + `.sig`。构建尾部有 `Warn The updater secret key … does not match the public key`——临时密钥与 `plugins.updater.pubkey` 不配对的预期告警，产物与签名均生成，非代码问题 | 通过 |

### E. 端到端 / 冒烟

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
|---|---|---|---|
| E17 | dev `[paths]` 的 `scripts_dir` 为仓库内 scripts 绝对路径 | `pnpm tauri dev` 终端原文：`[paths] data_root=C:\Users\An\AppData\Roaming\com.and0114.cclean rules_dir=…\rules scripts_dir=D:\GitHub\C-clean\app\src-tauri\../..\scripts box_dir=…\safe_box`（绝对路径，非 `.\scripts`） | 通过 |
| E18 | 安装版首次启动重建 `%APPDATA%\...\rules` 且含 2 个 json | 备份并移走 `rules`（`RULES_EXISTS_NOW=False`）→ 从诱饵目录启动安装版 → `rules` 被重建：`cache_rules.json 614 B` + `rules_meta.json 261 B`；与安装包 payload SHA-256 一致：`cache_rules EA473D9820BE22D…` 相同、`rules_meta 849C3648B31CFD94…` 相同 | 通过 |
| E19 | 安装目录零写入（9 payload SHA-256 前后一致） | 启动前后 `Get-FileHash` 全量比对：`AFTER_COUNT=9`、`INSTALL_DIR_NO_DIFF=1` | 通过 |
| E20 | GUI 走「扫描 → 清理」不返回 `INTERNAL` | **未取得（第三档）**。dev：窗口标题/hwnd 正常，但 WebView2 子进程 `t=3s → 4 个`、`t=6s → 0 个`，CDP 9222 未监听，Crashpad 19:57/20:01/20:06/20:09 各新增 `.dmp`（6.7–6.9 MB）；安装版：`t=3s → 4 个`、`t=6s..18s → 0 个`，窗口留白、无 Console 可取。已尝试 `--disable-gpu --disable-software-rasterizer` 与 `--remote-debugging-port=9222`（临时改 `additionalBrowserArgs`，测试后 `git checkout` 还原），均无效。**补偿证据**：E17 绝对路径 + E18 规则逐字节一致 + E19 零写入 + 从安装目录直接执行 bundled 脚本成功（`powershell -File "\\?\<INSTDIR>\scripts\scan_temp.ps1"` → `EXIT=0`、1340 行 NDJSON、`done {"total_items":1339,"total_bytes":97297149}`，证明安装位置的脚本可读可执行）+ 代码走查（`commands.rs:78` 为 `scripts_dir` 唯一消费者，`INTERNAL` 源自 `load_external_deny` 读 `rules_dir`，而 `rules_dir` 已实测可用） | **阻塞（第三档，需真机补做）** |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
|---|---|---|---|---|---|
| — | **本次测试未发现需修复缺陷（0 条）** | — | — | — | — |

观察项（非缺陷，不阻塞，仅登记）：

| # | 观察 | 说明 | 处理 |
|---|---|---|---|
| O1 | GUI 无法验证 | WebView2 渲染进程在本机每次启动 3–6 秒内消失（dev 与安装版一致），环境问题而非本任务改动引入（`main` 上同样现象，049b 轮已记录） | 已登记第三档，见 §6 |
| O2 | dev `[paths] scripts_dir` 含未折叠的 `..` | `D:\GitHub\C-clean\app\src-tauri\../..\scripts`；`std::path::absolute` 需 Rust 1.79，而 `rust-version = "1.77"` 且 `Cargo.toml` 不在允许改动范围 | 不修，记录；路径可被 shell 正常使用（实测脚本执行成功） |
| O3 | release `scripts_dir` 带 `\\?\` verbatim 前缀 | `resource_dir()` 原样输出；实测用该路径执行 `.ps1` 成功（`EXIT=0`） | 不修，记录 |

## 5. 测试期修复记录（bugfix 子分支）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
|---|---|---|---|---|---|
| — | 无缺陷，未开 bugfix 子分支 | — | — | — | — |

> 说明：本次测试未触发「从 feature 拉 bugfix 子分支」流程，也未修改任何既有测试断言（`git diff main..HEAD -- '*test*'` 仅新增 `scripts/tauri-resources.test.mjs`，既有 34 条 vitest 与 197 条 cargo 用例零改动、无 skip/xfail）。

## 6. 遗留问题（三档分流）

### 第一档（已修）
无。

### 第二档（需用户拍板，状态 blocked）
无。本次未发现 TRD/PRD 口径冲突或接口语义变更需求。

### 第三档（已登记为已知限制）

1. **GUI「扫描 → 清理」点击（E20）本机无法验证**——WebView2 渲染进程崩溃（dev 与安装版均复现；Crashpad `.dmp` 时间戳 19:57 / 20:01 / 20:06 / 20:09，安装版采样 `wv2` 由 4 → 0）。缺失条件：需要一台**有活动桌面且 WebView2 可正常渲染**的机器（发版前必做）。现有补偿：E17 绝对路径定位、E18 bundled 规则被正确定位并逐字节复制、E19 安装目录零写入、安装位置脚本直接执行成功（1339 items / exit 0）、代码走查确认 `scripts_dir`/`rules_dir` 唯一消费方。**未取得任何截图或 Console 证据，未伪造**。
2. **MSI 仅做 `/a` 行政安装解包抽检**，未做真机 `msiexec /i` 安装后启动——本轮已把开发轮「MSI 完全未测」补到「布局已验」，安装后行为仍随 GUI 阻塞一并待真机补做。

## 7. 测试结论

- 结论：**通过**（L1 单元 / L2 集成契约 / L4 打包安装 / L5 CI 全绿；L3 GUI 因本机 WebView2 崩溃阻塞，按第三档登记，不构成本任务缺陷）
- feature 分支最新 commit：**代码态 `e7171c4`**（CI run 40090971932 即此提交）；测试轮新增两个 docs-only 提交（`docs(test)` 本报告 / `docs(state)` 状态流转），不改变可执行代码，其 CI 结果以分支最新 run 为准
- 分支形态：`bugfix/task-050-bundle-resources-path`（本任务无 feature 分支，hotfix 语义；本地与 origin 一致 `0 0`）
- CI 运行号与结论：`ci` run **40090971932**（head `e7171c4`）→ `conclusion=success`，含「Tauri 资源映射契约」步骤 success
- 是否可进入评审：**是**
- 移交说明：
  - 评审对象：`bugfix/task-050-bundle-resources-path` @ `e7171c4`，PR #15（draft，仅用于触发 CI，非合并入口）
  - 测试报告：`docs/test-reports/050-test-report.md`
  - 请评审重点复核两项：① 第三档 GUI 缺口是否接受「发版前真机补做」；② 开发轮多出的 `73dbbe7 fix(paths): dev 优先仓库内资源` 是否属于允许改动范围内的合理偏差（本轮实测 dev 顺序为 cwd 三级 → `resource_dir()`，release 只认 `resource_dir()`，未发现劫持风险）
  - 本测试轮未下「可以合并 main」的结论（评审 Agent 职权）
