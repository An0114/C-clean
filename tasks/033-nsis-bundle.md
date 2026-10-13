# 任务编号：033 NSIS 打包——Tauri bundle 配置 + 应用图标 + CI 出 v0.1.0 .msi 产物

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevRustRelease
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-032 合并后写入）——M1 业务 + 工程化都齐了，准备发 v0.1.0 试用包。
> 上游依赖：task-001（Tauri 2 脚手架）、task-032（CI 已跑 lint/test/build）、task-006（GitHub Actions CI 已有 windows-latest）。
> **克制范围**：只接 Tauri bundle 配置 + 一个图标 + CI 出 .msi artifact，**不做**数字签名、自动更新、多平台矩阵（项目明确不做 macOS/Linux）。

## 任务目标

目前 C-clean 只能 `tauri dev` 跑，普通用户装不了。本任务补最小可分发包：

1. **`tauri.conf.json` 完善**：
   - `productName: "C-Disk Cleaner"`；
   - `version: "0.1.0"`；
   - `identifier: "com.and0114.cclean"`；
   - `bundle.windows.icon` 指向 .ico；
   - `bundle.windows.nsis.installMode: "perMachine"`（清理 C 盘需要管理员权限）；
   - `bundle.windows.nsis.displayLanguage: "SimpChinese"`；
2. **应用图标**：
   - 用 `tauri icon` 命令从一张 1024×1024 PNG 生成全套图标（.ico / .icns / various png）；
   - 生成到 `app/src-tauri/icons/`；
3. **CI 加 build 步骤**：
   - 现有 CI job 最后加 `pnpm --filter app tauri build`；
   - 用 `actions/upload-artifact@v4` 把 `app/src-tauri/target/release/bundle/nsis/*.exe` 和 `bundle/msi/*.msi` 上传为 artifact；
4. **本地真机验证**：
   - `pnpm --filter app tauri build` 本地能产出 .msi；
   - 双击安装 → 启动应用 → 看到 5 个 tab。

**本任务不做**：
- 不做数字签名（没证书，artifact 会有 SmartScreen 警告，备注说明）；
- 不做自动更新（tauri updater 配置留 task-034）；
- 不做 macOS/Linux（项目明确不做跨平台矩阵）；
- 不改业务代码；
- 不写 NSIS 自定义模板（用默认）。

## 前置条件

1. `main` 分支已含 task-001~032 全部产出；
2. 关键锚点：
   - `app/src-tauri/tauri.conf.json`（task-001 脚手架版本，目前字段不全）；
   - `app/src-tauri/Cargo.toml`；
   - `.github/workflows/ci.yml`（task-032 已有 lint/test/build）；
3. DevAgent 本地有 1024×1024 的源 PNG 图标（没有就用 Tauri 默认占位，备注里写「待设计」）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. tauri.conf.json 完善

1. 改 `app/src-tauri/tauri.conf.json`：
   ```json
   {
     "productName": "C-Disk Cleaner",
     "version": "0.1.0",
     "identifier": "com.and0114.cclean",
     "bundle": {
       "active": true,
       "targets": ["nsis", "msi"],
       "windows": {
         "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.ico"],
         "nsis": {
           "installMode": "perMachine",
           "displayLanguage": "SimpChinese"
         }
       }
     }
   }
   ```
   （具体字段以 Tauri 2 schema 为准，DevAgent 在备注里贴最终 JSON）

### B. 应用图标

2. 跑 `pnpm --filter app tauri icon path/to/source.png`：
   - 生成全套图标到 `app/src-tauri/icons/`；
   - 若 DevAgent 没有源 PNG，临时用 Tauri 默认图标（备注「设计图标留后续」）。

### C. CI 接 build + upload-artifact

3. 改 `.github/workflows/ci.yml`：
   - 前端 job 在 `pnpm build` 后加：
     ```yaml
     - name: Tauri build
       run: pnpm --filter app tauri build
     - name: Upload installer artifact
       uses: actions/upload-artifact@v4
       with:
         name: cclean-windows-installer
         path: |
           app/src-tauri/target/release/bundle/nsis/*.exe
           app/src-tauri/target/release/bundle/msi/*.msi
         if-no-files-found: error
     ```
4. CI job 需要装 WebView2（windows-latest 默认已预装，备注确认）。

### D. 本地真机验证

5. DevAgent 本地跑 `pnpm --filter app tauri build`：
   - 退出码 0；
   - `app/src-tauri/target/release/bundle/nsis/` 下有 .exe；
   - `app/src-tauri/target/release/bundle/msi/` 下有 .msi；
   - 贴文件大小截图或文字。

### E. 卫生

6. 不改 `app/src/**` 业务代码；
7. 不改 `engine/**` Rust 代码；
8. 不做代码签名。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `app/src-tauri/tauri.conf.json`
- `app/src-tauri/icons/`（tauri icon 生成，整目录）
- `.github/workflows/ci.yml`
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-033-nsis-bundle`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `chore(tauri): tauri.conf.json 完善 productName/version/identifier/nsis (#task-033)`
   —— 含 `app/src-tauri/tauri.conf.json`。
2. `chore(tauri): tauri icon 生成全套应用图标 (#task-033)`
   —— 含 `app/src-tauri/icons/`。
3. `ci: 加 tauri build + upload-artifact 出 v0.1.0 installer (#task-033)`
   —— 含 `.github/workflows/ci.yml`。
4. 自验全绿后，最后一次提交：`docs(state): 任务033完成，NSIS v0.1.0 打包就绪 (#task-033)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 033: ...`；
   - 「最近一次变更」追加一行；
   - 「已知问题」段更新 task-006 那条（NSIS 出包矩阵已配 Windows 单平台）；
   - 「下一步计划」改为「M1 v0.1.0 打包完成；下一轮进入：GitHub Release 发版（手动 / workflow_dispatch）+ tauri updater 自动更新配置」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-13  task-033  NSIS v0.1.0 配置 + 图标 + CI 出 installer artifact，M1 可分发`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 137 条全绿）；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0；
- `pnpm --filter app tauri build` 退出码 0，target/release/bundle 下有 .msi 和 .exe。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 3 个代码提交 + 1 个 `docs(state)` 提交，共 4 个 commit，全部 push 到 `feature/task-033-nsis-bundle`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 最终 tauri.conf.json 关键 JSON 片段；
   - `tauri build` 输出（.msi / .exe 文件大小）；
   - CI 跑通截图（artifact 上传成功）；
   - SmartScreen / 未签名警告说明；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（数字签名、updater、设计图标、macOS/Linux 留 task-034+）；
3. `docs/PROJECT_STATE.md` 已按第 4 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务033-NSIS v0.1.0 打包 (#task-033)`，
   该提交只含 `tasks/033-nsis-bundle.md` 一个文件，落在 `feature/task-033-nsis-bundle` 分支最前端。
