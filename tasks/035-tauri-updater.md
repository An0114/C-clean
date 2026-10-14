# 任务编号：035 tauri updater 自动更新——签名密钥对 + update endpoint + 前端「检查应用更新」

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevRustReleaseFrontend
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-034 + debt-cleanup-003/004 合并后写入）——v0.1.0 已发版，下一步让用户不用重新下载安装包就能升级。
> 上游依赖：task-033（tauri.conf bundle）、task-034（release.yml 发版）、task-031（Settings 页面已有 rules 更新卡片）。
> **克制范围**：只接 tauri updater 最小闭环——签名 + endpoint + 前端按钮；**不做**后台自动检查弹窗（先手动触发）。

## 任务目标

目前用户升级必须手动下载新 .msi 重装。本任务接 tauri 官方 updater：

1. **密钥对**：
   - DevAgent 跑 `pnpm --filter app tauri signer generate -w`（不带密码，CI 友好），得到私钥 `~/.tauri/cclean.key` 和公钥；
   - 公钥写到 `app/src-tauri/tauri.conf.json` 的 `updater.pubkey`；
   - 私钥**不能入仓**——DevAgent 在备注里写「请用户到 GitHub Repo → Settings → Secrets → Actions 新增 `TAURI_SIGNING_PRIVATE_KEY` = 私钥内容」（DevAgent 不代做 GitHub Secrets）；
2. **tauri.conf.json updater 配置**：
   ```json
   "updater": {
     "active": true,
     "endpoints": [
       "https://raw.githubusercontent.com/An0114/C-clean/main/latest.json"
     ],
     "pubkey": "<公钥内容>"
   }
   ```
   （endpoint 用 static JSON，不依赖 GitHub Releases API——简单稳定）
3. **release.yml 加签名步骤**：
   - build 前从 secret 读 `TAURI_SIGNING_PRIVATE_KEY` 写到临时文件；
   - `tauri build` 自动用私钥签 .msi / .exe；
   - build 后用 `tauri-dynamic-resource` 或手写脚本生成 `latest.json`（包含 version / notes / pub_date / platforms.windows-x86_64.url / signature），随 Release 上传；
4. **前端 Settings 加「检查应用更新」按钮**：
   - 在现有「规则库」卡片下方加「应用更新」卡片；
   - 点击 → `@tauri-apps/plugin-updater` 的 `check()`；
   - 有更新：显示新版本号 + 「立即更新」按钮 → `downloadAndInstall()`；
   - 无更新：绿字「已是最新」；
   - 失败：红条。

**本任务不做**：
- 不做后台自动检查（启动时静默 poll + 弹窗，留 task-036）；
- 不做增量更新；
- 不做 Windows 以外平台（项目明确不做）；
- 不代用户配 GitHub Secrets（DevAgent 在备注里给步骤）。

## 前置条件

1. `main` 分支已含 task-001~034 + debt-cleanup-003/004 全部产出；
2. 关键锚点：
   - `app/src-tauri/tauri.conf.json`（task-033 已配置 bundle）；
   - `.github/workflows/release.yml`（task-034 已有发版步骤）；
   - `app/src/pages/Settings.tsx`（task-031 已有规则库卡片）；
3. 依赖：`@tauri-apps/plugin-updater`（需 `pnpm add`）+ Rust 侧 `tauri-plugin-updater`。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. 密钥对 + tauri.conf updater 配置

1. 跑 `pnpm --filter app tauri signer generate -w ~/.tauri/cclean.key`；
2. 把输出的公钥（一行 base64）写到 `tauri.conf.json` 的 `updater.pubkey`；
3. `tauri.conf.json` 加 `updater.active: true` + `endpoints` 指向 raw latest.json；
4. 私钥不入库，`.gitignore` 加 `~/.tauri/`（全局，不影响项目）。

### B. Rust 侧插件接入

5. `app/src-tauri/Cargo.toml` 加 `tauri-plugin-updater = "2"`；
6. `app/src-tauri/src/lib.rs` 在 `tauri::Builder::default()` 链上 `.plugin(tauri_plugin_updater::Builder::new().build())`。

### C. 前端依赖 + 按钮

7. `cd app && pnpm add @tauri-apps/plugin-updater @tauri-apps/plugin-process`；
8. `app/src/pages/Settings.tsx` 在规则库卡片下方加「应用更新」卡片：
   ```ts
   import { check } from "@tauri-apps/plugin-updater";
   import { relaunch } from "@tauri-apps/plugin-process";
   async function checkAppUpdate() {
     const update = await check();
     if (!update) { /* 已是最新 */ return; }
     // 显示新版本 + 「立即更新」按钮
     // 点击：await update.downloadAndInstall(); await relaunch();
   }
   ```

### D. release.yml 签名 + latest.json

9. `release.yml` 加：
   ```yaml
   - name: Setup signing key
     env:
       TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
     run: |
       New-Item -Path "$env:USERPROFILE\.tauri" -Force | Out-Null
       Set-Content -Path "$env:USERPROFILE\.tauri\cclean.key" -Value $env:TAURI_SIGNING_PRIVATE_KEY
   ```
10. build 后手写 PowerShell 脚本生成 `latest.json`：
    ```json
    {
      "version": "0.1.1",
      "notes": "本次更新内容",
      "pub_date": "2026-10-14T00:00:00Z",
      "platforms": {
        "windows-x86_64": {
          "signature": "<.msi 对应的 .sig 内容>",
          "url": "https://github.com/An0114/C-clean/releases/download/v0.1.1/C-Disk-Cleaner_0.1.1_x64_en-US.msi"
        }
      }
    }
    ```
11. `latest.json` 随 Release 上传，同时推一份到 `main` 分支 `latest.json`（raw URL 可达）。

### E. 卫生

12. 不改业务代码；
13. 私钥绝不出现在 git 历史里（DevAgent 自审 `git log -p`）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `app/src-tauri/tauri.conf.json`
- `app/src-tauri/Cargo.toml`
- `app/src-tauri/src/lib.rs`
- `app/src/pages/Settings.tsx`
- `app/package.json` + `app/pnpm-lock.yaml`
- `.github/workflows/release.yml`
- `latest.json`（项目根，新建）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-035-tauri-updater`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `chore(tauri): 接入 tauri-plugin-updater Rust 插件 (#task-035)`
   —— 含 `app/src-tauri/Cargo.toml`、`app/src-tauri/src/lib.rs`。
2. `chore(tauri): tauri.conf.json 加 updater 配置（pubkey + endpoint） (#task-035)`
   —— 含 `app/src-tauri/tauri.conf.json`。
3. `feat(frontend): Settings 加「应用更新」卡片 + check/downloadAndInstall (#task-035)`
   —— 含 `app/src/pages/Settings.tsx`、`app/package.json`、`app/pnpm-lock.yaml`。
4. `ci: release.yml 加签名 key 注入 + latest.json 生成上传 (#task-035)`
   —— 含 `.github/workflows/release.yml`、`latest.json`。
5. 自验全绿后，最后一次提交：`docs(state): 任务035完成，tauri updater 自动更新就绪 (#task-035)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 035: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「tauri updater 手动检查已就位；下一轮做：①后台自动检查 + 启动弹窗（task-036）②品牌图标设计（用户手动）③首次 v0.1.1 发版验证 updater 闭环」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-14  task-035  tauri-plugin-updater + 签名 + latest.json + Settings 应用更新卡片，M2 自动更新闭环`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（已有 137 条全绿）；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0；
- `git log -p` 自审：**私钥字符串绝不出现在任何 commit**。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-035-tauri-updater`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - **给用户的 GitHub Secrets 配置步骤**（TAURI_SIGNING_PRIVATE_KEY 怎么加）；
   - 公钥内容（贴出来）；
   - 私钥已在本地 ~/.tauri/cclean.key 不入库的确认；
   - `pnpm build` / `cargo test` 输出；
   - 首次 v0.1.1 发版时 latest.json 该怎么更新；
   - 与本任务文档不符的偏差；
   - 遗留问题（后台自动弹窗、增量更新、多平台留 task-036+）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务035-tauri updater 自动更新 (#task-035)`，
   该提交只含 `tasks/035-tauri-updater.md` 一个文件，落在 `feature/task-035-tauri-updater` 分支最前端。

## 实现备注

### 实际改动文件清单
- `app/src-tauri/Cargo.toml`：加 `tauri-plugin-updater = "2"` + `serde_json = "1"`
- `app/src-tauri/src/lib.rs`：Builder 链加 `.plugin(tauri_plugin_updater::Builder::new().build())`
- `app/src-tauri/tauri.conf.json`：加 `plugins.updater`（active/endpoints/pubkey）
- `app/src/pages/Settings.tsx`：规则库卡片下方加「应用更新」卡片（checkApplication / downloadAndInstall）
- `app/package.json` + 根 `pnpm-lock.yaml`：加 `@tauri-apps/plugin-updater` + `@tauri-apps/plugin-process`
- `.github/workflows/release.yml`：加 Setup signing key + Generate latest.json 两步，release action 上传 latest.json
- 根目录新建 `latest.json`（占位，CI 首次发版时会被真实签名覆盖）

### 给用户的 GitHub Secrets 配置步骤
1. 打开 https://github.com/An0114/C-clean/settings/secrets/actions
2. 点 `New repository secret`
3. Name 填 `TAURI_SIGNING_PRIVATE_KEY`
4. Secret 内容粘贴 `C:\Users\An\.tauri\cclean.key` 文件的全部内容（私钥 PEM 文本，含 `-----BEGIN...` 头尾）
5. 点 `Add secret`
6. 之后每次发版（workflow_dispatch 或 push tag v*）CI 会自动写私钥并签名

### 公钥内容
```
dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDE4MTg5ODNBOTMwMkVCNDAKUldSQTZ3S1RPcGdZR0dlNE53L1pZUm4zTkFnOVk1aE03SlpTRHpvR2N4YVJianBOT2Q0d2Q4eUgK
```

### 私钥本地位置
`C:\Users\An\.tauri\cclean.key`（不入库，git 历史中已自审无任何私钥字符串）

### 自验输出
- `cargo check --workspace --all-targets`：0
- `cargo clippy --workspace --all-targets -- -D warnings`：0
- `cargo test -p engine`：0（137 条无回退）
- `cargo fmt --all -- --check`：0
- `pnpm --filter app lint`：0
- `pnpm --filter app test`：0（9 断言）
- `pnpm --filter app build`：0

### 首次 v0.1.1 发版时 latest.json 怎么更新
- 不需要手动改根目录 `latest.json`——CI 里 Generate latest.json 步骤会读 `.msi.sig` 自动生成并随 Release 上传；
- 但 updater endpoint 是 `raw.githubusercontent.com/.../main/latest.json`，CI 上传到 Release 后还需要把这份 latest.json 合并进 main 分支（手动从 Release 下载后 push，或后续在 release.yml 加一步 `git commit`）。**留 task-036 或评审时补 CI 自动 push latest.json 到 main。**

### 与本任务文档不符的偏差
1. **updater 配置位置**：任务书写顶层 `updater`，Tauri 2 build script 报 `unknown field 'updater'`；实测正确位置是 `plugins.updater` 命名空间。
2. **serde_json 依赖**：任务书未提，但 `tauri::generate_context!()` 解析 plugins.updater 配置时需要 `serde_json` 在依赖图里，否则 E0433。
3. **签名私钥环境变量**：任务书写 `TAURI_SIGNING_PRIVATE_KEY` 直接写文件内容；release.yml 里改为 `TAURI_SIGNING_PRIVATE_KEY_PATH` 指向 `~/.tauri/cclean.key`（tauri build 更友好）。
4. **签名生成命令**：任务书写 `tauri signer generate -w`，但交互式 prompt 在非 TTY 下会挂死；实际用 `--ci --password="" -w <path> -f` 一次性生成（密码为空 CI 友好）。

### 遗留问题
- 后台自动检查 + 启动弹窗：留 task-036
- CI 自动把 latest.json push 回 main：需补一步（否则 raw URL 拿到的还是占位）
- 增量更新 / 多平台：明确不做
- 真机端到端（装 v0.1.0 → 触发 v0.1.1 发版 → 点检查更新 → 自动装）：留首次发版验证
