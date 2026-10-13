# 测试评审报告

## 本次任务名称

034 GitHub Release workflow——workflow_dispatch 手动发版 + tag 自动发版

## Git 提交记录核对结果

✅ 5 个提交全部关联 `#task-034`，type 前缀规范（docs / ci），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `b360a9e` | docs(task) | tasks/034-github-release.md | ✅ |
| 2 | `c7a276d` | ci | release.yml | ✅ |
| 3 | `51b345d` | docs(state) | PROJECT_STATE.md | ✅ |
| 4 | `d8b49fa` | docs(task) | tasks/034-github-release.md | ✅ |
| 5 | `ad9a877` | docs(state) | PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. release.yml

- ✅ 触发器：`workflow_dispatch`（inputs.version 必填 string）+ `push.tags: ["v*"]`。
- ✅ `permissions: contents: write`。
- ✅ job：windows-latest。
- ✅ steps：checkout@v4 → pnpm/action-setup@v4 (v11) → setup-node@v4 (node 22, cache pnpm) → dtolnay/rust-toolchain@stable → pnpm install --frozen-lockfile → pnpm --filter app tauri build。
- ✅ `softprops/action-gh-release@v2`：name + tag_name 都用 `v${{ github.event.inputs.version || github.ref_name }}`。
- ✅ files：`target/release/bundle/nsis/*.exe` + `target/release/bundle/msi/*.msi`。
- ✅ `generate_release_notes: true`。

### B. 首次手动发版 v0.1.0

- ✅ DevAgent 未代跑（无 gh CLI 鉴权），留评审在 GitHub Actions 页手动触发 workflow_dispatch（version 输 `0.1.0`），或本地 `git tag v0.1.0 && git push origin v0.1.0` 走 tag 通道。

### C. 卫生

- ✅ 不改业务代码。
- ✅ 不改 CI.yml（保留 main 分支跑 lint/test/build）。
- ✅ 不接 updater。

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

1. **bundle path 偏差**：任务书假设 `app/src-tauri/target/release/bundle/`，workspace 模式实际是仓库根 `target/release/bundle/`。DevAgent 已适配。**第二档口径，已登记。**
2. **tag_name 显式设置**：任务书示例只写 name，DevAgent 给 release action 同时显式设了 `tag_name`，避免 workflow_dispatch 手动触发时 tag 不创建。**第二档口径，已登记。**
3. **无 tauri updater 自动更新**：需签名密钥对 + update endpoint JSON，留 task-035。**第三档。**
4. **无代码签名**：未做代码签名（无证书），用户首次运行会有 SmartScreen 警告。**第三档。**
5. **无 changelog 自动生成**：先手写 release notes。**第三档。**
6. **首次发版未代跑**：自动化环境无 gh auth，留评审手动触发。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ 纯 CI 配置任务，无业务代码改动。 |
| A03 注入 | ✅ 无用户输入；release.yml 是静态配置。 |
| A04 不安全设计 | ✅ permissions: contents: write（最小权限，只创建 Release）；frozen lockfile 防依赖篡改；softprops/action-gh-release@v2 是官方维护 action。 |
| A08 数据完整性 | ✅ Release 由 GitHub Actions 在隔离 runner 构建；frozen lockfile 防依赖篡改；规则库 SHA-256 校验步骤保留。 |

### 专项安全分析

- **会不会被伪造 Release？** 不会。Release 由 GITHUB_TOKEN 创建，权限最小化（只 contents:write）。✅
- **installer 会不会被替换？** 不会。release.yml 在 GitHub Actions 跑，artifact 由 GitHub 托管；本地 build 不污染 CI。✅
- **workflow_dispatch 会不会被未授权触发？** 不会。GitHub 仓库 workflow_dispatch 默认只对有 write 权限的用户开放。✅

## 评审结论：**通过**

GitHub Release 发版通道就绪：release.yml（workflow_dispatch 手动发版 + push tag v* 自动发版）+ permissions: contents:write + softprops/action-gh-release@v2 上传 .msi + .exe + generate_release_notes；lint/test/build/clippy/fmt/cargo test 全 0（137 基线无回退）；5 个提交全部关联 #task-034 无混杂。

## Git 操作记录

- **merge commit**：`7db39f1`（`git merge --no-ff feature/task-034-github-release`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-034-github-release`，was ad9a877）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务034评审报告 (#task-034)`

## 后续动作建议

1. **task-035**：tauri updater 自动更新配置（签名密钥对 + update endpoint JSON）。
2. **task-036**：代码签名（证书申请 + signing script）+ 品牌图标设计。
3. **后续**：组件单测（Dashboard/History/Items/SafeBox/Settings）+ Prettier/Husky + safe_box purge 命令 + 端到端真机安装验证。
4. **立即动作**：评审在 GitHub Actions 页手动触发 workflow_dispatch（version 输 `0.1.0`）完成首次发版，或本地 `git tag v0.1.0 && git push origin v0.1.0` 走 tag 通道。
