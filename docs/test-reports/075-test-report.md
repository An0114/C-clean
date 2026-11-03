# 测试报告：task-075 R5 持续——scan_prefetch.ps1 + scan_browser_cache.ps1 → Rust 重写

- 分支：`feature/task-075-r5-prefetch-browser-rust`
- 测试 HEAD：`974e55e`
- 测试时间：2026-11-03（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）
- **R5 第二个 ps1 批次闭环：prefetch + browser_cache Rust 模块（含 Firefox 新增）+ 三模块统一接线 + 总开关 `CCLEAN_RUST_ALL` + 21 条新单测**

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| 构建验证 | `cargo check/clippy/test` + `pnpm build`（无前端改动） | 是 | 通过 |
| 打包 | NSIS `C-DiskCleaner_0.2.1_x64-setup.exe`（4.50 MiB / 4718377 字节） | 是 | 通过（出包成功） |
| 干净目录安装 | `/S /D=...`（`-Verb RunAs`） | 未尝试 | **归第三档**（UAC 非交互受限；本任务无前端改动，按用户 2026-11-03 新规仍可归第三档） |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区（含 21 条新增 + 1 条 `#[ignore]` 真机） | `cargo test --workspace` | 通过 | `cargo-test.log`：62 batches / **530 passed / 0 failed**（基线 509 + 新增 21，与开发声称一致；含 1 ignored） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（1 条 elevation incremental lock warning + 2 条 elevation 既有 warning）；`pnpm.log`：build=0（243.01 kB / gzip 75.55 kB，与 074 完全一致，无前端改动） |
| L3 真机 | 扫描结果含 prefetch/browser_cache 项 / 环境变量回退 | 安装态截图 + 环境变量切换 | **未完成**（归第三档） | 由 21 条单测 + 1 条 `#[ignore]` 真机冒烟（dev 已跑：browser_cache 4576 项 / 278.5 MB / edge 7 / chrome 0 / firefox 4569；prefetch 0 项因 ACL）覆盖 |
| L4 打包 | NSIS 出包 | `tauri build` | 通过 | `tauri-build.log`：NSIS 4.50 MiB + MSI 6.65 MiB；FileVersion=0.2.1；签名 exit=1 为已知预期 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `engine/src/scan/modules/prefetch.rs` 实现 Prefetch 目录扫描 | 5 条内联单测全绿（递归收集 4 文件含非 .pf / 模块类别风险三字段 / 目录缺失返空 / prefetch_root 基于 SystemRoot / scan() 不可读不 panic） | 通过 |
| 2 | 返回 `Vec<ScanItem>`，格式与 ps1 输出一致 | 与 074 同一 `ScanItem` 结构 + 同一 `insert_batch` 入库 | 通过（语义等价） |
| 3 | 单测：扫描 Prefetch 目录返回正确结果 | 5 条内联 + 3 条权限现实集成全绿 | 通过 |
| 4 | `browser_cache.rs` 实现浏览器缓存扫描（Chrome/Edge/Firefox） | 7 条内联单测全绿（Edge 2 / Firefox 1 / Chrome 缺失不报错 / 模块风险字段 / 空 LOCALAPPDATA 返空 / fixed_roots 与 ps1 逐字一致 / firefox_cache_dirs 只挑有 cache2 的 profile / **绝不扫 Login Data/Bookmarks/History/Cookies**） | 通过 |
| 5 | 正确处理环境变量和通配符路径 | `firefox_cache_dirs()` 通配 profile 目录 + 单测钉住 | 通过 |
| 6 | 单测：扫描浏览器缓存目录返回正确结果 | 7 条内联 + 2 条 Rust vs ps1 真实对比 + 1 条 Firefox 行为契约全绿 | 通过 |
| 7 | `orchestrator.rs` prefetch 模块改为直接调 Rust | `rust_module_items` match 分支 + 单测钉住 PREFETCH_SCRIPT 在 SCRIPTS 里 | 通过 |
| 8 | `orchestrator.rs` browser_cache 模块改为直接调 Rust | 同上（BROWSER_CACHE_SCRIPT 分支） | 通过 |
| 9 | 环境变量可回退 ps1 | `flag_enabled_from` 纯函数 + 总开关 `CCLEAN_RUST_ALL=0`（任务书外新增） | 通过 |
| 10 | 单测：Rust 优先 / ps1 回退两种模式 | 6 条集成测试含 2 条真实 Rust vs ps1 对比 | 通过 |
| 11 | `cargo test --workspace` 全绿（新增 ≥4 条） | 530 passed（新增 21 条，远超 ≥4） | 通过 |
| 12 | Rust 实现与 ps1 输出抽样对比（格式一致） | 2 条真实对比单测（Edge/Chrome）+ dev 真机冒烟 | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮未发现需代码修复的缺陷 | L1-L2 全绿 | — | — | — |

## 5. 测试期修复记录（bugfix）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮无需修复 | — | — | — | — |

## 6. 遗留问题（三档分流）

### 第一档（已修）
- prefetch.ps1 → Rust 等价重写（读 `%SystemRoot%\Prefetch` 而非硬编码 `C:\Windows\Prefetch`）。
- browser_cache.ps1 → Rust 重写 + **Firefox 新增**（ps1 原本不扫 Firefox）。本机实测条目数 7 → 4576（653 倍），Firefox cache2 占 4569 项 / 287 MB。
- 三模块统一接线（`rust_module_items` match + `run_rust_module` 记账），Rust 分支仍在脚本存在检查之前。
- 递归遍历从 `temp.rs` 下沉到 `modules/mod.rs` 共用 `scan_dir`，`temp::scan_dir` 保留为薄包装（既有 7 条单测不用改签名）。
- 总开关 `CCLEAN_RUST_ALL=0` 一次回退所有 Rust 模块（任务书外新增，解决排障 + 测试稳定性）。
- L3 红线模块层落点：`scan_never_touches_profile_files` 单测造 Login Data/Bookmarks/History/Cookies 并断言绝不出现。

### 第二档（需评审/用户拍板）
1. **白名单 3 处偏差**（dev 已在实现备注 §1 列出）：①`modules/temp.rs` 必须改（递归遍历下沉共用，避免三份复制漂移）；②`tests/scan_orchestrator.rs` 回归修正（4 个用例全红）；③`tests/scan_progress_percent.rs` 回归修正。
2. **Firefox 是新增功能不是等价重写**：ps1 原本不扫 Firefox，本次新增后条目数 7 → 4576（653 倍）。功能上合理（能清 287 MB），但列表观感变化大。回退成本极低（删一处 `firefox_cache_dirs` 调用）。**需产品确认是否保留。**
3. **「按大小/时间筛选」未实现**（同 074 判断）。
4. **三处刻意与 ps1 保持一致**：①只扫 Default profile（不扫 Profile 1/2/...）；②只扫 Cache（不扫 Code Cache/GPUCache）；③不按扩展名/大小/时间筛选。
5. **`CCLEAN_RUST_ALL` 总开关任务书外新增**（排障 + 测试稳定性）。
6. **`modules/mod.rs` 改动超出「注册」**：还把递归遍历下沉共用。
7. **prefetch 在普通权限下永远为 0**：ACL 限制（`C:\Windows\Prefetch` 只给管理员），非 bug。要真扫需提权，但扫描器不该为多列几条要求管理员。

### 第三档（已登记为已知限制）
- **真机 GUI / 安装态冒烟未做**（7 条，本任务无前端改动，按用户 2026-11-03 新规归第三档）：①扫描结果含 prefetch 模块项；②扫描结果含 browser_cache 模块项（Edge/Chrome/Firefox 三类分开）；③环境变量回退 ps1 结果一致；④CDP console 0 红错（G6 已知项除外）；⑤Items 页显示两个模块项；⑥NSIS 安装态复跑上述 3 条；⑦环境变量切换后条目数一致。由 21 条单测 + 1 条 `#[ignore]` 真机冒烟（dev 已跑：edge 7 / chrome 0 / firefox 4569 / prefetch 0 ACL）覆盖。**特别提示用户：prefetch 显示 0 项是正常的（ACL）；Firefox 让列表变长是预期结果（653 倍）。**
- **安装态未完成**：NSIS 出包成功（4.50 MiB / FileVersion=0.2.1），UAC 非交互受限。
- **CI 五道闸未跑**：PR 由 CI 门禁观察员创建后跑。
- **签名**：本地未签名（exit=1 已知预期）。
- **浏览器缓存三处未覆盖**（刻意保持与 ps1 一致）：多 profile、Code Cache/GPUCache、其他浏览器（360/QQ）。
- **还剩两个 ps1**（task-076）：`scan_windows_update` / `scan_dev_cache`。全部迁完后删 scripts/ 依赖才是 R5 真正收尾。
- **`tasks/060-shutdown-experience.md` 仍是「工作区已删除、HEAD 里存在」**（连续 13 个任务）。
- **Items/SafeBox/History 三页深色适配待补**。

## 7. 测试结论

- 结论：**通过（530 passed / 0 failed / NSIS 出包 4.50 MiB）**
- feature 分支最新 commit：`974e55e`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/build）+ NSIS 出包，未复用开发自验结论。
- 新规执行说明：本任务**无前端改动**（纯后端 ps1→Rust），按用户 2026-11-03「有前端更改必须真机测试」的指示，L3 真机仍归第三档；后续任务若涉及前端改动，L3 不可跳过。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①白名单 3 处偏差（temp.rs 遍历下沉、两个既有测试回归修正）；②Firefox 新增导致条目数 7→4576（653 倍）是否保留（回退成本极低）；③`CCLEAN_RUST_ALL` 总开关是否接受；④prefetch 普通权限下为 0 的产品说明；⑤三处刻意与 ps1 保持一致（Default profile / 只扫 Cache / 不筛选）是否认可。**
