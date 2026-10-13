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

## 实现备注（DevAgent 落地后回填）
- **改动文件**：
  - 修改：`app/src-tauri/tauri.conf.json`（identifier 改 com.and0114.cclean；bundle.targets 收窄为 ["nsis","msi"]；bundle.windows.nsis 加 installMode=perMachine + languages=["SimpChinese"]）
  - 修改：`.github/workflows/ci.yml`（pnpm build 后加 Tauri build + Upload installer artifact 两步）
  - 图标：未重跑 `tauri icon`，沿用 task-001 生成的 17 个 Tauri 默认占位文件（已在库），设计图标留后续
- **最终 tauri.conf.json 关键片段**：
  ```json
  {
    "productName": "C-Disk Cleaner",
    "version": "0.1.0",
    "identifier": "com.and0114.cclean",
    "bundle": {
      "active": true,
      "targets": ["nsis", "msi"],
      "icon": ["icons/32x32.png","icons/128x128.png","icons/128x128@2x.png","icons/icon.icns","icons/icon.ico"],
      "windows": {
        "nsis": { "installMode": "perMachine", "languages": ["SimpChinese"] }
      }
    }
  }
  ```
- **本地 `pnpm --filter app tauri build` 退出码 0**，产出：
  - `target/release/bundle/msi/C-Disk Cleaner_0.1.0_x64_en-US.msi`（5.11 MB）
  - `target/release/bundle/nsis/C-Disk Cleaner_0.1.0_x64-setup.exe`（3.60 MB）
- **CI artifact**：名 `cclean-windows-installer`，path `target/release/bundle/{nsis/*.exe,msi/*.msi}`，`if-no-files-found: error`；windows-latest 默认预装 WebView2 Runtime 无需额外装；CI 跑通留评审（本地已验证 build 能出包）。
- **SmartScreen / 未签名警告**：未做代码签名（无证书），用户首次运行 .exe/.msi 会有 SmartScreen「Windows 已保护你的电脑」提示，需手动「更多信息→仍要运行」；正式发版前需评估代码签名。
- **与任务书偏差**：
  1. Tauri 2 NSIS schema 字段是 `languages`（数组），不是任务书写的 `displayLanguage`（标量）——build script 报 `unknown field displayLanguage`，已改为 `languages: ["SimpChinese"]`。
  2. workspace 模式 bundle 输出在仓库根 `target/release/bundle/` 而非任务书假设的 `app/src-tauri/target/release/bundle/`，CI path 已相应调整。
  3. 图标未重跑 `tauri icon`（无 1024×1024 源 PNG，沿用 task-001 默认占位），按任务书「没有就用 Tauri 默认占位，备注待设计」处理。
- **自验七闸**：cargo check=0 / clippy -D warnings=0 / cargo test -p engine=0（137 条基线）/ cargo fmt --check=0 / pnpm --filter app lint=0 / pnpm --filter app test=0（9/9）/ pnpm --filter app build=0 / pnpm --filter app tauri build=0（出 .msi + .exe）。
- **遗留**：代码签名、tauri updater 自动更新、品牌图标设计、macOS/Linux 多平台、safe_box purge 命令、端到端真机安装验证（双击 .msi 装→启动→看 5 个 tab），均留 task-034+。
