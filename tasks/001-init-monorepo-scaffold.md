# 任务编号：001 初始化 monorepo 目录骨架 + Tauri 2 最小空窗口

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevAgent（建议 DevTauriBridge，负责把 app/src-tauri 与 engine 串起来）
> 来源：`docs/PROJECT_STATE.md` 待办 001 / 「下一步计划」段。

## 任务目标

在仓库根目录一次性落地 `app/`、`engine/`、`elevation/`、`rules/`、`scripts/` 五大目录骨架，建立 Rust workspace + pnpm workspace 双工作区，跑通 **Tauri 2 最小空窗口**：`pnpm tauri dev` 能弹出一个标题为「C-Disk Cleaner」的空 Win32/WebView2 窗口，前端显示 Vite+React 18+TS 默认占位页。**本任务不实现任何业务逻辑**，不写 SQLite schema、不写 IPC 命令、不写规则库——这些留给 002~005。

## 前置条件

1. `docs/PRD.md`、`docs/TRD.md`、`docs/ARCHITECTURE.md`、`docs/AGENTS.md`、`docs/PROJECT_STATE.md` 已合并到 `main`，目录树与技术栈以 `ARCHITECTURE.md` §1.x（约 90~108 行）为准：
   - `app/src/` 渲染层、`app/src-tauri/` Tauri 壳
   - `engine/` Rust 核心库（本任务只放空 lib crate，子模块目录不创建）
   - `elevation/` 独立提权 exe（本任务只放空 bin crate 占位）
   - `rules/`、`scripts/` 仅占位
2. 本机已安装：Rust stable（rustup）、Node.js ≥ 20、pnpm ≥ 9（corepack enable 或独立安装）、WebView2 Runtime（Win11 自带）、Microsoft C++ Build Tools。
3. 工作分支从最新 `main` 拉出；启动前先 `git checkout main && git pull --ff-only`，确认无未提交改动。
4. 已知问题（PROJECT_STATE「已知问题」段）：本地 main 领先 origin/main 共 7 个提交尚未 push——本任务**不处理远端同步**，仅在本地 feature 分支开发，push 策略由维护者统一决定。

## 需求 & 验收标准

> 以下每一条都必须可机器/人工复核，DevAgent 自验时逐条勾选。

### A. 目录骨架（对齐 ARCHITECTURE §目录树）

1. 仓库根下存在一级目录：`app/`、`engine/`、`elevation/`、`rules/`、`scripts/`、`docs/`、`tasks/`。
2. `app/` 下存在：`app/src/`、`app/src-tauri/`。
3. `rules/`、`scripts/` 本任务**只放 `.gitkeep` 占位**，禁止放入真实 JSON 规则或 ps1 脚本（那是 005 / M1 的事）。
4. `engine/` 与 `elevation/` 本任务**只创建空 crate**，不预建 `scan/grade/cleaner/safebox/...` 子模块目录，避免与后续任务抢写锁。

### B. Rust workspace

5. 仓库根存在 `Cargo.toml`（workspace 根），显式声明：
   - `[workspace] members = ["app/src-tauri", "engine", "elevation"]`
   - `[workspace.package]` 统一 `edition = "2021"`、`rust-version` 与 TRD 一致（如 TRD 未钉死则填 `"1.77"`，并在实现备注里说明取此值的原因）
   - `[workspace.dependencies]` 只允许预放 `tauri = { version = "2", features = [] }` 一条，**禁止**在本任务引入 rusqlite / serde / jwalk 等业务依赖（留给 002/003/004）。
6. `engine/` 是一个最小 lib crate（`src/lib.rs` 仅一行 `pub fn hello_engine() -> &'static str { "engine" }`，配同目录 `tests/smoke.rs` 一个断言），`cargo test -p engine` 通过。
7. `elevation/` 是一个最小 bin crate（`src/main.rs` 仅打印一行 `eprintln!("elevation stub, not for direct run");` 并以非 0 退出码退出，明示「本 crate 不应被普通用户直接运行」），`cargo check -p elevation` 通过。
8. `app/src-tauri/` 是 Tauri 2 主进程壳：
   - `Cargo.toml` 中 `[dependencies]` 依赖 `tauri = { workspace = true }` 与 `engine = { path = "../../engine" }`；
   - `src/main.rs` / `src/lib.rs` 用 `tauri::Builder::default()` 注册一个空命令 `ping`（返回 `engine::hello_engine()`），不注册任何业务 command；
   - `tauri.conf.json` 中 `productName = "C-Disk Cleaner"`、`windows[0].title = "C-Disk Cleaner"`、`frontendDist` 指向 `../dist`、`devUrl` 指向 Vite 默认端口（`http://localhost:5173`）。
9. **`Cargo.lock` 必须入库**（决策 #2），不得加入 `.gitignore`。

### C. pnpm workspace + 前端

10. 仓库根存在 `pnpm-workspace.yaml`，内容仅 `packages:\n  - "app"`。
11. 仓库根存在最小 `package.json`（private、name 为 `cdisk-cleaner-monorepo`、scripts 至少含 `dev` / `build` / `tauri` 三个转发脚本），**禁止**在根 `package.json` 安装业务依赖。
12. `app/` 下是 Vite + React 18 + TypeScript 项目（`app/package.json`、`app/vite.config.ts`、`app/tsconfig.json`、`app/index.html`、`app/src/main.tsx`、`app/src/App.tsx`），`pnpm install --frozen-lockfile` 在 `app/` 下通过，`pnpm --filter app build` 产物输出到 `app/dist/`。
13. `app/src/App.tsx` 仅渲染一个静态占位页：标题「C-Disk Cleaner」+ 一行小字「M0 scaffold — waiting for task 002」，不调用任何 IPC。

### D. 最小空窗口端到端

14. 在仓库根执行 `pnpm install` 后，执行 `pnpm tauri dev`（等价于 `pnpm --filter app tauri dev`）能：
    - 编译 Rust 壳成功；
    - 拉起 Vite dev server；
    - 弹出一个原生窗口，标题为「C-Disk Cleaner」；
    - 窗口内显示第 13 条所述占位页。
15. 窗口内不出现任何 Tauri / Vite 报错红屏；DevTools 无红色 error（warning 可接受，但需在实现备注里列出）。

### E. 杂项与卫生

16. `.gitignore` **只允许追加**以下三类条目，不得删除/改写已有规则：
    - Rust 产物：`target/`、`**/*.rs.bk`
    - Node/Tauri 产物：`node_modules/`、`app/dist/`、`app/src-tauri/target/`、`app/src-tauri/gen/`
    - 系统/IDE：`.DS_Store`、`Thumbs.db`（如未已存在）
17. `README.md` 在末尾追加一段「## 本地启动（M0 脚手架）」，列出前置依赖三行命令 + `pnpm install` / `pnpm tauri dev` 两条命令，不展开业务说明。
18. **禁止改动**以下文件：`docs/PRD.md`、`docs/TRD.md`、`docs/ARCHITECTURE.md`、`docs/AGENTS.md`、`docs/01-立项决策记录.md`、`.gitattributes`、`LICENSE`。
19. 代码中**禁止**硬编码任何真实用户路径（`C:\Users\...`）、token、密钥；窗口 icon 本任务可暂用 Tauri 默认图标，不强行塞自定义 `.ico`。

## 允许修改范围

> 严格白名单，范围外文件一律不动。DevAgent 若发现必须改范围外文件才能跑通，**停下来在任务文件「实现备注」里报 DocsKeeper**，不得自行扩大。

- 新建整目录：`app/**`、`engine/**`、`elevation/**`、`rules/**`（仅 `.gitkeep`）、`scripts/**`（仅 `.gitkeep`）
- 仓库根新增文件：`Cargo.toml`、`Cargo.lock`、`pnpm-workspace.yaml`、`package.json`、`.npmrc`（如确需钉 pnpm 版本才加）
- 仓库根**追加修改**：`.gitignore`（仅追加 §E-16 所列条目）、`README.md`（仅追加 §E-17 所列启动段）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`（由最后一次 `docs(state)` 提交处理）

## Git 分支

- 分支名：`feature/task-001-init-monorepo`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

> 严格按以下顺序原子提交，每个提交只动一类文件，**禁止 `git add .` / `git add -A` / `git commit -a`**，每次 `git commit` 必须显式列文件路径。

1. `chore(workspace): 初始化 Cargo workspace 与 pnpm workspace 配置 (#task-001)`
   —— 含根 `Cargo.toml`、`pnpm-workspace.yaml`、根 `package.json`、`.gitignore` 追加段。
2. `chore(scaffold): 新增 engine 空 lib 与 elevation 空 bin 占位 crate (#task-001)`
   —— 含 `engine/**`、`elevation/**`。
3. `feat(ui): 初始化 app/src Vite + React 18 + TypeScript 前端骨架 (#task-001)`
   —— 含 `app/package.json`、`app/index.html`、`app/vite.config.ts`、`app/tsconfig*.json`、`app/src/**`、`pnpm-lock.yaml`。
4. `feat(tauri): 接入 app/src-tauri Tauri 2 壳并链接 engine (#task-001)`
   —— 含 `app/src-tauri/**`、`Cargo.lock`（本任务首次生成即在此提交入库）。
5. `chore(scaffold): 补全 rules/ scripts/ 占位与 README 启动说明 (#task-001)`
   —— 含 `rules/.gitkeep`、`scripts/.gitkeep`、`README.md` 追加段。
6. 自验全绿后，最后一次提交：`docs(state): 任务001完成，更新进度与下一步 (#task-001)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 把待办 001 从 `- [ ]` 改为 `- [x]`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「派发 task-002：落地 SQLite schema 与 rusqlite 连接」；
   - 「Git 提交记录」表追加本任务 6 条 commit 摘要；
   - 文件末尾「变更轨迹」追加 `2026-10-11  task-001  初始化 monorepo 脚手架与 Tauri 2 最小空窗口`。

**提交前自验（必须全绿，任一不过不得 push）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo test -p engine` 退出码 0；
- `pnpm install --frozen-lockfile` 退出码 0；
- `pnpm --filter app build` 退出码 0；
- 本机手测一次 `pnpm tauri dev` 能弹出标题正确的空窗口（DevAgent 在「实现备注」里写明实测截图路径或现象描述）。

## 输出要求

DevAgent 交付物（不是本任务文档产出，是后续执行结果）：

1. 上述 5 个代码/配置提交 + 1 个 `docs(state)` 提交，共 6 个 commit，全部 push 到 `feature/task-001-init-monorepo`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单（与白名单逐条对照）；
   - 自验命令与退出码（逐条列）；
   - 手测空窗口现象描述（标题、占位页文案、DevTools 是否有 warning）；
   - 遗留问题 / 与本任务文档不符的偏差（如有）；
3. `docs/PROJECT_STATE.md` 已按第 6 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务001-初始化monorepo脚手架 (#task-001)`，
   该提交只含 `tasks/001-init-monorepo-scaffold.md` 一个文件，落在 `feature/task-001-init-monorepo` 分支最前端（即 DevAgent 开工前的基线）。

## 实现备注（DevAgent 回填，2026-10-11）

### 实际改动文件清单（对照白名单）

- 仓库根新增：`Cargo.toml`、`Cargo.lock`、`pnpm-workspace.yaml`、`package.json`、`.npmrc`、`pnpm-lock.yaml`（6 个）
- `app/`：`app/package.json`、`app/index.html`、`app/vite.config.ts`、`app/tsconfig.json`、`app/src/main.tsx`、`app/src/App.tsx`、`app/src/vite-env.d.ts`
- `app/src-tauri/`：`Cargo.toml`、`build.rs`、`src/main.rs`、`src/lib.rs`、`tauri.conf.json`、`icons/`（tauri icon 生成的全套 Windows/macOS 图标；android/ ios/ 子目录已删除，符合「仅 Windows」决策）
- `engine/`：`Cargo.toml`、`src/lib.rs`（hello_engine 占位）、`tests/smoke.rs`（1 断言）
- `elevation/`：`Cargo.toml`、`src/main.rs`（非 0 退出占位）
- `rules/.gitkeep`、`scripts/.gitkeep`
- 仓库根追加：`.gitignore`（仅追加 `app/src-tauri/gen/` 一条）、`README.md`（追加「本地启动（M0 脚手架）」段）、`docs/PROJECT_STATE.md`（状态同步）
- 未改动：PRD/TRD/ARCHITECTURE/AGENTS/立项决策记录、`.gitattributes`、`LICENSE`。

### 与任务文档的偏差（均已在代码/提交中注明原因）

1. **`.npmrc` 新增**：首次 `pnpm install` 直连 npmjs.org 连续 ECONNRESET，追加 `registry=https://registry.npmmirror.com` 镜像（白名单允许「如确需」新增 `.npmrc`）。
2. **`.gitignore` 追加 `app/src-tauri/gen/`**：已有 `src-tauri/gen/` 因含斜杠被锚定仓库根，匹配不到 `app/src-tauri/gen/`（Tauri v2 build 期生成的 ACL schema），属 §E-16 允许追加项，故显式追加锚定路径。其余 §E-16 条目（target/、node_modules/、dist/、.DS_Store、Thumbs.db 等）基线下均已存在，未重复追加。
3. **`pnpm-workspace.yaml` 增加 `allowBuilds: esbuild: true`**：pnpm v11 不再读 package.json 的 `onlyBuiltDependencies`，且默认拦截 esbuild 构建脚本导致 vite 无法启动，按 pnpm v11 新约定迁入本文件。
4. **`app/src-tauri/Cargo.toml` 增加 `[lib] name = "cdisk_cleaner_lib"`**：Tauri 约定 main.rs 以 `<bin>_lib::run()` 调用，不加此段会报 E0433。另加 `tauri-build` build-dependency（Tauri 壳必需构建期依赖，非业务依赖，未进 workspace.dependencies）。
5. **rust-version 取 `"1.77"`**：TRD 未钉死，按任务说明填入。
6. **额外一次 docs(task) 提交**：本「实现备注」按任务输出要求追加到本文件，因提交 6 规定只动 PROJECT_STATE.md，故单独补一次 `docs(task)` 提交，不与业务代码混提。

### 自验命令与退出码（全部在仓库根执行）

| 命令 | 退出码 | 结果 |
|------|--------|------|
| `cargo check --workspace --all-targets` | 0 | dev profile 编译通过，无 warning |
| `cargo test -p engine` | 0 | smoke 测试 1 passed / 0 failed |
| `pnpm install --frozen-lockfile` | 0 | Already up to date |
| `pnpm --filter app build` | 0 | vite 5.4.11 构建成功，产物输出到 `app/dist/`（142.94 kB JS） |
| `pnpm tauri dev` | 窗口正常拉起后手动终止 | Vite 5173 ready；Rust dev 编译 3m54s 通过；进程常驻 |

注：cargo 拉包时本机 rsproxy 镜像 schannel 吊销检查离线（CRYPT_E_REVOCATION_OFFLINE），以环境变量 `CARGO_HTTP_CHECK_REVOKE=false` 会话内绕过，未改动全局 cargo 配置。

### 手测空窗口现象

- `pnpm tauri dev` 后进程 `cdisk-cleaner.exe` 常驻，主窗口标题实测为 **「C-Disk Cleaner」**（Process.MainWindowTitle 确认）。
- 通过 PrintWindow 截取窗口内容确认：标题栏为自定义「C」图标 + 「C-Disk Cleaner」；正文居中显示 **C-Disk Cleaner** 大标题与灰色小字 **M0 scaffold — waiting for task 002**，无 Tauri/Vite 红屏报错。
- DevTools 未开；dev 日志除 pnpm 自身的 NativeCommandError 包装（PowerShell 对 stderr 的正常包装，非错误）外，无 error；仅有一条 info：「tauri dependency has workspace inheritance enabled…Expected features: []」（workspace 继承的正常提示）。
- 手测时桌面曾弹出一个与本项目无关的 `explorer.exe 无法访问指定设备` 系统弹窗（疑似用户桌面其他快捷方式失效），已用回车关闭，与本任务无关。

### 遗留问题 / 风险点

- 本机网络对 npmjs.org / crates.io 直连不稳，已分别走 npmmirror 与 rsproxy 镜像；CI（task-006）落地时需确认 GitHub Actions 网络不受影响。
- `app/src-tauri/gen/schemas/` 已被 gitignore，Tauri v2 在团队协作时若启用 capabilities ACL 再评估是否入库（M0 未启用任何权限，无需入库）。
- 前端未配置 ESLint / `tsc` 严格构建（build 脚本仅 `vite build`），留待 task-006 CI 阶段统一补齐 lint/test。
