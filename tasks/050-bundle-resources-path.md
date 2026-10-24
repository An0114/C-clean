# 任务编号：050 打包资源路径错位修复——`bundle.resources` 映射式落位 + `AppPaths` 资源定位加固

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevEngineRust（后端路径 + 打包配置）+ DevFrontendReact **不参与**；一轮对话可完成
> 来源：`docs/PROJECT_STATE.md`「已知问题」`[B 中] 打包态资源路径错位（2026-10-24 测试轮发现，非 task-049 引入）`
> 上游依赖：`debt-cleanup-003`（`16b30e2`，引入 `bundle.resources` 相对路径 + `AppPaths`）、task-049/049b 测试轮实测证据
> 里程碑：M1-D 收尾第二项（M2 已关闭）

**编号说明（决策留痕）**：本任务占用 **050**，原定给「safe_box 增强（时间范围筛选 / 多选 session / audit_log 联表）」的编号顺延为 **051**。
理由：`PROJECT_STATE.md` 下一步计划第 2 条已写明「优先级建议高于 ①」，且该缺陷导致**全新安装用户首次启动即不可用**（发版阻塞级），而 051 是功能增强（可延后）。`tasks/049b-*.md` 里「050 预留给 safe_box 增强」的旧注记以本文件为准。

## 状态字段

pending

## 任务目标

修掉「打包后规则库与 ps1 脚本没落到应用能找的位置」这个缺陷，让**干净机器首次安装 → 首次启动 → 扫描 / 清理可用**这条链路成立，并加一道防回归守卫。

具体三件事：

1. **根因修复（打包侧）**：`app/src-tauri/tauri.conf.json` 的 `bundle.resources` 目前是**数组式**且源路径带 `../../`，Tauri 按「保留原目录结构」规则把每个 `..` 换成 `_up_`，于是文件落到 `$INSTDIR\_up_\_up_\rules\...` / `$INSTDIR\_up_\_up_\scripts\...`；而 `AppPaths` 只找 `resource_dir()/rules` 与 `resource_dir()/scripts` → 全新用户首次启动找不到 bundled 规则与脚本。改为**映射式**（源 → 安装包内目标路径），让资源稳定落到 `<resource_dir>/rules/` 与 `<resource_dir>/scripts/`。
2. **定位逻辑加固（应用侧）**：`app/src-tauri/src/paths.rs` 的 `locate_resource_dir()` 在 release 下**不再依赖 `std::env::current_dir()` 向上搜索**（当前实现会让「从任意含 `rules/`+`scripts/` 的目录启动安装版」劫持资源），删掉静默的 `PathBuf::from(".")` 兜底，找不到资源时返回带绝对路径的明确错误；抽出纯函数便于单测。
3. **防回归**：新增零依赖的契约测试，钉住「资源必须是映射式 + 目标必须是 `rules/`、`scripts/` 前缀 + 7 个文件一个不少」，接入 CI。

**本任务不做**（划清边界）：

- 不改 `commands.rs` 的任何业务逻辑（含 `load_external_deny` 的错误码），不改 `lib.rs` 命令注册表；
- 不改 `engine/**`（含 `ScanOrchestrator` 的脚本调用方式）、不改前端 `app/src/**` 任何页面；
- 不改 `rules/cache_rules.json` / `rules_meta.json` 内容与 `scripts/*.ps1`；
- 不改 `release.yml`、`capabilities/default.json`、`app/package.json`、`engine/Cargo.toml`；
- **不做**「把 scripts 复制到 data_root」这类架构改造（脚本随安装包版本走，只读即可）；
- **不做** safe_box 增强（顺延 051）、不做 `RmRestart`、不做图标。

## 前置条件

1. `main` 已含 task-001~049 全部产出（当前 `b547988`，task-049 合并提交 `60e0797`），工作区干净；
2. 从 `main` 拉出 **一层** bugfix 分支 `bugfix/task-050-bundle-resources-path`（hotfix 语义，先例：`bugfix/task-035-release-signing` 直接从 main 拉；本任务无 feature 分支）；
3. 关键锚点（**先读真实代码，以代码为准，不以本文档伪代码为准**）：
   - `app/src-tauri/tauri.conf.json` → `bundle.resources`（7 条，全部 `../../` 前缀）、`bundle.createUpdaterArtifacts: true`；
   - `app/src-tauri/src/paths.rs` → `locate_resource_dir()`（48-59 行）、`from_app()`（63-111 行）、启动日志 `eprintln!("[paths] ...")`；
   - `app/src-tauri/src/commands.rs:78` → `orch.run_all(&paths.scripts_dir)`（脚本目录的唯一消费者）；
   - `app/src-tauri/src/commands.rs:37` → `load_external_deny(&paths.rules_dir)`；
   - `.github/workflows/ci.yml` → 「发版脚本单测」步骤（`node --test scripts/release-lib.test.mjs`）的写法与位置，新守卫照抄这套形态；
   - `scripts/release-lib.test.mjs` → `node:test` 零依赖单测的既有风格。
4. 本地打包需**临时签名密钥**：`createUpdaterArtifacts: true` 后 `tauri build` 强制要求私钥，否则退出码 1（产物仍生成但无 `.sig`）。打包冒烟前执行：
   ```bash
   pnpm --filter app exec tauri signer generate -w "$TEMP/cclean-smoke-050.key" -p "" --ci --force
   export TAURI_SIGNING_PRIVATE_KEY="$TEMP/cclean-smoke-050.key"
   ```
5. 事实依据（Tauri 2 资源映射语法）：<https://v2.tauri.app/develop/resources/> —— 数组式「保留原目录结构」（`../` → `_up_`），映射式 `{"<源>": "<目标>"}` 可精确控制落位。

## 需求 & 验收标准

> 执行 Agent 自验时逐条勾选并贴证据。A=打包侧，B=应用侧，C=防回归，D=卫生，E=端到端/冒烟。

### A. 资源落位（根因修复）

1. `bundle.resources` 由数组改为**映射对象**，7 条一个不少，目标路径固定为：
   ```jsonc
   {
     "../../rules/cache_rules.json":        "rules/cache_rules.json",
     "../../rules/rules_meta.json":         "rules/rules_meta.json",
     "../../scripts/scan_temp.ps1":         "scripts/scan_temp.ps1",
     "../../scripts/scan_windows_update.ps1":"scripts/scan_windows_update.ps1",
     "../../scripts/scan_prefetch.ps1":     "scripts/scan_prefetch.ps1",
     "../../scripts/scan_browser_cache.ps1":"scripts/scan_browser_cache.ps1",
     "../../scripts/scan_dev_cache.ps1":    "scripts/scan_dev_cache.ps1"
   }
   ```
2. **不得**改用通配符整目录搬运（`../../scripts/` / `../../scripts/**/*`）——会把 `PROTOCOL.md`、`smoke_test.ps1`、`release-lib.mjs` 等构建期资产塞进安装包。
3. 产物断言（NSIS 必做、MSI 抽检）：安装到干净目录后，`<INSTDIR>/rules/cache_rules.json`、`<INSTDIR>/rules/rules_meta.json`、`<INSTDIR>/scripts/scan_{temp,windows_update,prefetch,browser_cache,dev_cache}.ps1` **全部存在**；且 `<INSTDIR>/_up_` **不存在**。
4. **允许的 fallback（B 路线）**：若映射式在当前 Tauri 2 版本不可用（构建报错，或仍落 `_up_`），可保留数组式，改为在 `paths.rs` 把 `resource_dir/_up_/_up_` 纳入候选根。走 fallback 时**必须**在实现备注贴出构建输出/落盘清单，证明 A 路线不可行；**两条路线不得同时保留**（禁止双份真相）。

### B. `AppPaths` 资源定位加固

5. 抽出纯函数并单测（签名可微调，语义固定）：
   ```rust
   /// 返回第一个同时含 rules/ 与 scripts/ 的候选目录。
   fn pick_resource_root(candidates: &[PathBuf]) -> Option<PathBuf>
   ```
   `from_app()` 只负责产出候选列表，不再内联判定。
6. **release 下禁止用 `current_dir()`**：候选列表在 `cfg!(debug_assertions) == false` 时只来自 `app.path().resource_dir()`（fallback 时再加 `_up_/_up_`）；dev 保留向上搜索（`src-tauri/` → `app/` → workspace 根），确保 `pnpm tauri dev` 照常工作。用 `cfg!(debug_assertions)` 或 `#[cfg(debug_assertions)]` 均可，但必须能从代码上一眼看出 release 分支不碰 cwd。
7. 删除 `PathBuf::from(".")` 兜底。找不到资源时返回 `Err`，消息必须含**绝对路径**与实际缺失项，例如：`resource_dir=C:\Program Files\C-DiskCleaner 下缺少 rules/ 与 scripts/，安装包可能不完整，请重新安装`。
8. `data_root/rules` 首次复制失败必须是 `Err`（当前已如此，回归守卫）；启动 `eprintln!` 保留并**打印绝对路径**（`scripts_dir` 不允许再出现 `.\scripts`）。
9. 单测 ≥ 4 条（`paths.rs` 内 `#[cfg(test)] mod tests`，用 `tempfile` 或 `std::env::temp_dir()` 手造结构，**不得依赖真实 cwd / 真实安装目录**）：只有 `rules/` → `None`；只有 `scripts/` → `None`；两者都有 → 命中且返回该候选；候选全空 → `None`。若 `tempfile` 未在 app crate 依赖中，**改用 `std::env::temp_dir().join(唯一名)` + 手动清理，禁止新增依赖**。

### C. 防回归守卫

10. 新增 `scripts/tauri-resources.test.mjs`（`node:test`，**零新依赖**），断言：
    - `bundle.resources` 是**对象**（数组即失败）；
    - 目标值集合恰好等于 A1 的 7 条，且每条以 `rules/` 或 `scripts/` 开头；
    - 任一目标值包含 `..` 或 `_up_` → 失败；
    - 每条**源**路径在仓库磁盘上真实存在（相对 `app/src-tauri/` 解析）。
11. `.github/workflows/ci.yml` 在「发版脚本单测」之后新增一步（命名建议「Tauri 资源映射契约」），命令 `node --test scripts/tauri-resources.test.mjs`。**只加这一步**，不改其他步骤。
12. 守卫有效性自证：把修复前的数组式配置作为反例贴进实现备注，并说明该用例在旧配置上会失败（可用 `node -e` 临时构造断言，或直接说明断言分支）。

### D. 卫生

13. 不新增 npm / cargo 依赖（`app/src-tauri/Cargo.toml` 不改）。
14. 允许改动的文件见下表，其余一律不动。
15. 门禁全绿：`cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo test --workspace`（engine 基线 **196 条 = 188 passed + 8 ignored** 不回退，app crate 新增单测计入总数）/ `pnpm --filter app lint` / `pnpm --filter app test`（基线 34 条）/ `pnpm --filter app build`（基线 176.39 kB，gzip 56.25 kB）。
16. `pnpm --filter app tauri build` 出包成功（带前置条件 4 的临时密钥），NSIS + MSI 均有产物。

### E. 端到端 / 冒烟

17. **dev 验证**：`pnpm tauri dev` 启动，终端 `[paths]` 行 `scripts_dir` 为仓库内 `scripts` 的**绝对路径**（对照缺陷态 `.\scripts`）。
18. **安装版验证**：干净目录安装 → 备份并移走 `%APPDATA%\com.and0114.cclean\rules` → 启动安装版 → 该目录被**重新创建**且含 `cache_rules.json` + `rules_meta.json`（证明 bundled 规则被正确定位并复制）。
19. 安装目录写入隔离：安装后 9 个 payload 文件 SHA-256 前后**零变化**（应用不写安装目录）。
20. GUI 链路：扫描 → 清理不再返回 `INTERNAL`（资源缺失的旧症状）。**若本会话仍无活动桌面 / WebView2 渲染进程崩溃**，按第三档登记（写明实测现象），用 E17–E19 的文件系统级证据 + 代码走查补偿，**禁止伪造截图/日志**。

## 允许修改范围

| 文件 | 允许的改动 |
|---|---|
| `app/src-tauri/tauri.conf.json` | 仅 `bundle.resources` 改为映射式（A 路线）；fallback 时不改此项 |
| `app/src-tauri/src/paths.rs` | `pick_resource_root` 抽取 + 候选列表分 dev/release + 删 `.` 兜底 + 明确错误 + 启动日志 + 单测 |
| `scripts/tauri-resources.test.mjs` | 新增（契约测试） |
| `.github/workflows/ci.yml` | 仅新增「Tauri 资源映射契约」一步 |
| `docs/PROJECT_STATE.md` | 收尾单独提交 |
| `tasks/050-bundle-resources-path.md` | 追加 `## 实现备注` |

**禁止修改**：`app/src-tauri/src/commands.rs`、`app/src-tauri/src/lib.rs`、`app/src-tauri/src/progress.rs`、`app/src-tauri/Cargo.toml`、`app/src/**`（前端全部）、`engine/**`、`elevation/**`、`rules/**`、`scripts/*.ps1`、`scripts/release-lib*.mjs`、`.github/workflows/release.yml`、`capabilities/default.json`、`app/package.json`、`Cargo.lock`、`pnpm-lock.yaml`。

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`bugfix/task-050-bundle-resources-path`，**从 `main`（`b547988`）拉出**。
- 本任务本身就是缺陷修复，**不存在 feature 分支**；hotfix 语义同 `bugfix/task-035-release-signing` 先例。
- 若修复过程中再发现独立缺陷：只允许开**一层**子分支 `bugfix/task-050-<描述>`（从上述 bugfix 分支拉出），**禁止再嵌套第二层**；子分支修完 `--no-ff` 合回本分支并删除。
- 完成后**不合 main**：push 分支 → 由评审（提示词 10）复核通过后再合并；可开 **draft PR** 触发 CI，PR 不作为合并入口。

## Git 提交要求

格式 `<type>(<scope>): <描述> (#task-050)`，**每个功能点完成即提交，禁止攒批**：

1. `fix(packaging): bundle.resources 改为映射式，规则与脚本落到安装根目录 (#task-050)`
2. `fix(paths): 资源定位不再依赖 cwd，缺失时显式报错并打印绝对路径 (#task-050)`
3. `test(paths): 补资源候选挑选单测 (#task-050)`
4. `test(ci): 新增 Tauri 资源映射契约测试并接入 CI (#task-050)`
5. `docs(state): 任务050 打包资源路径修复完成 (#task-050)`（**单独只动 `docs/PROJECT_STATE.md`**，不与代码混提）

补充约束：

- 每个提交前门禁全绿；**不得**把 `PROJECT_STATE.md` 混进业务提交；
- 不重写已推送历史（禁 rebase / force push）；
- 敏感文件（`*.key`、`latest.json`、`data/*.db`）不入库。

## 测试要求（层级 + 真机/打包冒烟是否必做）

| 层级 | 是否必做 | 覆盖内容 |
|---|---|---|
| 单元测试 | **必做** | `paths.rs` 候选挑选 4 条（只有 rules / 只有 scripts / 两者齐 / 全空）+ 契约测试 7 条断言 |
| 集成测试 | **必做** | `cargo test --workspace` 全绿（engine 196 条不回退）；契约测试在真实仓库配置上通过 |
| 端到端 | **必做** | E17 dev `[paths]` 绝对路径 + E18 安装版首次启动重建 `%APPDATA%\...\rules` |
| 真机冒烟 | **必做、不可裁剪** | 安装版启动后 GUI 走「扫描 → 清理」不返回 INTERNAL。**命中打包/安装判定清单，不得以「本地绿」代替**；若环境阻塞 GUI，必须实测记录崩溃现象 + 用 E17–E19 补偿，登记第三档 |
| 打包安装冒烟 | **必做、不可裁剪** | `pnpm --filter app tauri build` → NSIS `/S /D=%TEMP%\cclean-smoke-050` 安装 → 资源布局断言（A3）+ 安装目录零写入（E19）；MSI 抽检资源布局 |
| CI | **必做** | 新 run id + `conclusion=success`，且日志中可见「Tauri 资源映射契约」步骤通过 |

裁剪说明：**无裁剪项**。本任务改的就是打包资源落位，任何一层都不能省；唯一可降级的是「GUI 点击」这一项，降级必须留实测现象 + 补偿证据。

## 测试证据清单

| 验收标准 | 证据形式（命令 + 输出 / 截图 / 日志 / 报告路径） |
|---|---|
| A1/A2 映射式 7 条 | `git diff main -- app/src-tauri/tauri.conf.json` 片段 + `node -e "const c=require('./app/src-tauri/tauri.conf.json');console.log(Object.keys(c.bundle.resources).length)"` → `7` |
| A3 安装布局 | `Get-ChildItem -Recurse "$env:TEMP\cclean-smoke-050" \| Select-Object -Expand FullName` 输出（须含 `rules\*.json`、`scripts\*.ps1`，须无 `_up_`） |
| A4（若走 fallback） | 构建日志中「映射式不可用」的原始报错 + 落盘清单 |
| B5/B6 纯函数与 dev/release 分叉 | `grep -n "pick_resource_root\|current_dir\|debug_assertions" app/src-tauri/src/paths.rs` + 代码摘录 |
| B7/B8 明确报错 | `cargo test -p cdisk-cleaner paths::` 输出（用例名 + 断言消息） |
| B9 单测 | `cargo test --workspace` 输出中 app crate 用例列表 |
| C10/C11 守卫 | `node --test scripts/tauri-resources.test.mjs` 全绿输出 + `git diff` 中 ci.yml 新增步骤片段 |
| C12 守卫有效 | 旧数组式配置下断言失败的输出（或断言分支说明 + 反例 JSON 片段） |
| D15 门禁 | 六条命令退出码 + `cargo test` 条数（engine 196）+ `pnpm build` 体积 |
| D16 出包 | `pnpm tauri build` 退出码 + `target/release/bundle/nsis/*.exe` 与 `msi/*.msi` 体积 |
| E17 dev 日志 | `pnpm tauri dev` 终端 `[paths] data_root=... rules_dir=... scripts_dir=...` 原文（绝对路径） |
| E18 首次启动重建 | `Remove-Item -Recurse %APPDATA%\com.and0114.cclean\rules` → 启动 → `Get-ChildItem %APPDATA%\com.and0114.cclean\rules` 输出 |
| E19 零写入 | 安装目录 9 个文件 `Get-FileHash` 前后对比表（NO_DIFF） |
| E20 GUI / 第三档 | 若取得：截图 + Console 0 红错；若阻塞：崩溃实测现象（窗口标题出现时长 / Crashpad `.dmp` 时间戳 / CDP 9222 状态）+ 补偿说明 |
| CI | run URL + id + `conclusion=success` + 步骤名 |

## 输出要求

1. 按上表提交并 push 到 `bugfix/task-050-bundle-resources-path`（**不合 main**，可开 draft PR 取 CI）；
2. 任务文件末尾追加 `## 实现备注`，至少包含：实际改动文件对账（对照允许范围）、走 A 还是 fallback B 及理由、`pick_resource_root` 的 dev/release 候选差异、单测条数、六道门禁退出码、安装布局清单、`%APPDATA%\...\rules` 重建证据、CI run id、与任务书不符之处（若有）；
3. 明确回答一句：**「干净机器首次安装 → 首次启动 → 扫描/清理可用」是否成立**，并给出支撑证据编号（A3 / E17 / E18 / E20）；
4. 更新 `docs/PROJECT_STATE.md`：勾选待办、关闭「打包态资源路径错位」已知问题、把「下一步计划」中的 050 改为 051（safe_box 增强）并写清编号顺延，单独提交 `docs(state)`。
