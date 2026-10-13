# 任务编号：034 GitHub Release workflow——workflow_dispatch 手动发 v0.1.0 安装包

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevRustRelease
> 来源：`docs/PROJECT_STATE.md` 下一步计划（task-033 合并后写入）——v0.1.0 installer 已能在 CI 产出，下一步把它发到 GitHub Release。
> 上游依赖：task-033（`tauri build` + upload-artifact 已在 CI 跑通）、task-006（GitHub Actions 工作流基础）。
> **克制范围**：只接 workflow_dispatch 手动发版 + push tag 自动发版，**不做** tauri updater 自动更新（需要签名密钥对，留 task-035）。

## 任务目标

目前 CI 每次 push main 都出 installer artifact，但 artifact 7 天后过期、也不便于用户下载。本任务补正式发版通道：

1. 新建 `.github/workflows/release.yml`：
   - 触发器：
     - `workflow_dispatch`：手动触发，输入 version（如 `0.1.0`）；
     - `push.tags`: `v*`：打 tag 自动发版；
   - 步骤：
     - checkout；
     - setup node 22 + pnpm 11；
     - setup rust stable；
     - 装依赖 `pnpm install --frozen-lockfile`；
     - `pnpm --filter app tauri build`；
     - 用 `softprops/action-gh-release@v2` 把 `app/src-tauri/target/release/bundle/nsis/*.exe` + `bundle/msi/*.msi` 上传到 GitHub Release；
     - Release 名称：`v${{ inputs.version || github.ref_name }}`；
2. 权限：`contents: write`（让 action 能创建 Release）；
3. 首次手动触发 v0.1.0 发版。

**本任务不做**：
- 不做 tauri updater 自动更新（需要签名密钥，留 task-035）；
- 不做自动 bump version（手动传 version）；
- 不做 changelog 自动生成（先手写 release notes）；
- 不做 GitHub Release pages / 分发网站；
- 不做代码签名。

## 前置条件

1. `main` 分支已含 task-001~033 全部产出；
2. 关键锚点：
   - `.github/workflows/ci.yml`（task-033 已有 tauri build 步骤可参考）；
   - `app/src-tauri/tauri.conf.json`（task-033 已配置 bundle targets）；
3. 仓库是 public（`An0114/C-clean`），GITHUB_TOKEN 已有 contents:write 默认权限。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. release.yml

1. 新建 `.github/workflows/release.yml`：
   ```yaml
   name: Release
   on:
     workflow_dispatch:
       inputs:
         version:
           description: "Version (e.g. 0.1.0)"
           required: true
           type: string
     push:
       tags: ["v*"]
   permissions:
     contents: write
   jobs:
     release:
       runs-on: windows-latest
       steps:
         - uses: actions/checkout@v4
         - uses: actions/setup-node@v4
           with: { node-version: 22, cache: pnpm }
         - uses: pnpm/action-setup@v4
           with: { version: 11 }
         - uses: dtolnay/rust-toolchain@stable
         - run: pnpm install --frozen-lockfile
         - run: pnpm --filter app tauri build
         - uses: softprops/action-gh-release@v2
           with:
             name: v${{ github.event.inputs.version || github.ref_name }}
             files: |
               app/src-tauri/target/release/bundle/nsis/*.exe
               app/src-tauri/target/release/bundle/msi/*.msi
             generate_release_notes: true
   ```
   （具体字段以 YAML 校验为准，DevAgent 在备注里贴最终版本）

### B. 首次手动发版 v0.1.0

2. DevAgent 在 GitHub 上手动触发 workflow_dispatch，version 输入 `0.1.0`；
3. 等 CI 跑完（约 5-10 分钟）；
4. 到 GitHub Releases 页确认：
   - Release 名 `v0.1.0`；
   - 有 .msi 和 .exe 两个附件；
   - 能下载。

### C. 卫生

5. 不改业务代码；
6. 不改 CI.yml（保留 main 分支跑 lint/test/build）；
7. 不接 updater。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `.github/workflows/release.yml`（新建）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-034-github-release`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `ci: 新增 release.yml——workflow_dispatch 手动发版 + tag 自动发版 (#task-034)`
   —— 含 `.github/workflows/release.yml`。
2. 自验全绿后，最后一次提交：`docs(state): 任务034完成，GitHub Release 通道就绪 (#task-034)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 034: ...`；
   - 「最近一次变更」追加一行；
   - 「下一步计划」改为「v0.1.0 已发版；下一轮进入 tauri updater 自动更新配置（签名密钥对 + update endpoint JSON）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-13  task-034  GitHub Release workflow_dispatch + tag 发版通道，v0.1.0 首次发版`。

**提交前自验（必须全绿）：**
- YAML 语法通过（CI 跑通）；
- 手动触发 workflow_dispatch v0.1.0 成功；
- GitHub Releases 页能看到 v0.1.0 + 两个 installer 附件。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 1 个 ci 提交 + 1 个 `docs(state)` 提交，共 2 个 commit，全部 push 到 `feature/task-034-github-release`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - release.yml 最终内容；
   - 首次 v0.1.0 发版的 Release URL；
   - 附件大小；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（updater 自动更新、签名、changelog 自动生成留 task-035+）；
3. `docs/PROJECT_STATE.md` 已按第 2 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务034-GitHub Release 发版通道 (#task-034)`，
   该提交只含 `tasks/034-github-release.md` 一个文件，落在 `feature/task-034-github-release` 分支最前端。
