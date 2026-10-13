# 测试评审报告

## 本次任务名称

033 NSIS 打包——tauri.conf.json 完善 + CI 出 v0.1.0 installer artifact

## Git 提交记录核对结果

✅ 6 个提交全部关联 `#task-033`，type 前缀规范（docs / chore(tauri) / ci），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `3a255d1` | docs(task) | tasks/033-nsis-bundle.md | ✅ |
| 2 | `f0a46ec` | chore(tauri) | tauri.conf.json | ✅ |
| 3 | `0d35fa0` | ci | ci.yml | ✅ |
| 4 | `96be58d` | docs(state) | PROJECT_STATE.md | ✅ |
| 5 | `724a336` | docs(task) | tasks/033-nsis-bundle.md | ✅ |
| 6 | `64d2874` | docs(state) | PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. tauri.conf.json 完善

- ✅ `productName: "C-Disk Cleaner"`。
- ✅ `version: "0.1.0"`。
- ✅ `identifier: "com.and0114.cclean"`。
- ✅ `bundle.active: true`。
- ✅ `bundle.targets: ["nsis", "msi"]`。
- ✅ `bundle.windows.icon` 指向 5 个图标文件（32x32/128x128/128x128@2x/icon.icns/icon.ico）。
- ✅ `bundle.windows.nsis.installMode: "perMachine"`（清理 C 盘需要管理员权限）。
- ✅ `bundle.windows.nsis.languages: ["SimpChinese"]`。

### B. 应用图标

- ✅ 沿用 task-001 生成的 17 个 Tauri 默认占位文件（无 1024×1024 源 PNG，设计图标留后续）。

### C. CI 接 build + upload-artifact

- ✅ ci.yml 前端 job 在 `pnpm build` 后加 `pnpm --filter app tauri build`。
- ✅ `actions/upload-artifact@v4` 上传 `target/release/bundle/nsis/*.exe` + `target/release/bundle/msi/*.msi`。
- ✅ `if-no-files-found: error`（缺文件即失败）。
- ✅ windows-latest 默认预装 WebView2 Runtime，无需额外安装。

### D. 本地真机验证

- ✅ DevAgent 本地 `pnpm --filter app tauri build` 退出码 0。
- ✅ 产出：`target/release/bundle/msi/C-Disk Cleaner_0.1.0_x64_en-US.msi`（5.11 MB）。
- ✅ 产出：`target/release/bundle/nsis/C-Disk Cleaner_0.1.0_x64-setup.exe`（3.60 MB）。

### E. 卫生与边界

- ✅ 不改 `app/src/**` 业务代码。
- ✅ 不改 `engine/**` Rust 代码。
- ✅ 不做代码签名。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `pnpm --filter app lint` | ✅ exit 0 |
| `pnpm --filter app test` | ✅ exit 0（2 文件 9 断言全过） |
| `pnpm --filter app build` | ✅ exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `cargo test -p engine` | ✅ 137 passed / 0 failed（基线无回退，后端无改动） |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **NSIS schema 字段偏差**：任务书假设 `displayLanguage`（标量），Tauri 2 实际是 `languages`（数组）。DevAgent 已适配为 `languages: ["SimpChinese"]`。**第二档口径，已登记。**
2. **bundle 输出路径偏差**：workspace 模式输出在仓库根 `target/release/bundle/`，不是任务书假设的 `app/src-tauri/target/release/bundle/`。CI path 已相应调整。**第二档口径，已登记。**
3. **图标未重跑 tauri icon**：无 1024×1024 源 PNG，沿用 task-001 默认占位。设计图标留后续。**第二档口径，已登记。**
4. **无数字签名**：未做代码签名（无证书），用户首次运行会有 SmartScreen 警告。正式发版前需评估。**第三档。**
5. **无 tauri updater 自动更新**：留 task-034。**第三档。**
6. **无 macOS/Linux 多平台**：项目明确不做跨平台矩阵。**第三档。**
7. **真机安装验证未跑**：自动化环境无 GUI。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ 纯打包配置任务，无业务代码改动。 |
| A03 注入 | ✅ 无用户输入；tauri.conf.json 是静态配置。 |
| A04 不安全设计 | ✅ perMachine 安装模式（清理 C 盘需要管理员权限）；NSIS 用默认模板（不写自定义）；if-no-files-found: error 防缺文件出包。 |
| A08 数据完整性 | ✅ CI frozen lockfile 防依赖篡改；规则库 SHA-256 校验步骤保留；artifact 上传后可校验哈希。 |

### 专项安全分析

- **会不会被劫持安装包？** 不会。CI 在 GitHub Actions 跑，artifact 由 GitHub 托管；本地 build 不污染 CI。✅
- **perMachine 会不会装错用户目录？** 不会。perMachine 装到 Program Files，所有用户可用。✅
- **NSIS 自定义模板会不会引入后门？** 不会。用默认模板，不写自定义。✅

## 评审结论：**通过**

M1 可分发：tauri.conf.json 完善（identifier=com.and0114.cclean + targets=[nsis,msi] + nsis.perMachine + languages=[SimpChinese]）+ CI 加 tauri build + upload-artifact（cclean-windows-installer，if-no-files-found: error）；本地 tauri build 出 .msi（5.11 MB）+ .exe（3.60 MB）；lint/test/build/clippy/fmt/cargo test 全 0（137 基线无回退）；6 个提交全部关联 #task-033 无混杂。

## Git 操作记录

- **merge commit**：`3699fd5`（`git merge --no-ff feature/task-033-nsis-bundle`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-033-nsis-bundle`，was 64d2874）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务033评审报告 (#task-033)`

## 后续动作建议

1. **task-034**：GitHub Release 发版（手动 / workflow_dispatch）+ tauri updater 自动更新配置。
2. **task-035**：代码签名（证书申请 + signing script）+ 品牌图标设计。
3. **后续**：组件单测（Dashboard/History/Items/SafeBox/Settings）+ Prettier/Husky + safe_box purge 命令 + 端到端真机安装验证。
