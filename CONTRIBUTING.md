# 贡献指南

感谢参与 C-Disk Cleaner。本项目会真实删除用户文件，**安全约束优先于功能与开发便利**；提交前请先阅读[安全策略](docs/SECURITY.md)。

## 1. 开发环境

| 依赖 | 要求 |
|---|---|
| 操作系统 | Windows 10 1809+ / Windows 11（仅 Windows） |
| Rust | stable，版本由 [`rust-toolchain.toml`](rust-toolchain.toml) 固定 |
| Node.js | ≥ 20 |
| pnpm | ≥ 9（`corepack enable`） |
| MSVC | Microsoft C++ Build Tools（含 Windows SDK） |
| WebView2 | Runtime（Win11 自带，Win10 通常随 Edge 安装） |

```powershell
pnpm install                        # 安装前端 workspace 依赖
pnpm tauri dev                      # 开发模式：Rust 壳 + Vite dev server
cargo test --workspace              # Rust 全量测试
pnpm --filter app test              # 前端单测（vitest）
```

打包需要 updater 签名私钥，缺失会失败；本地仅验证构建时可先跑 `pnpm --filter app build`。

## 2. 仓库结构

```
app/          Tauri 应用：React 前端 + src-tauri 主进程壳
engine/       Rust 核心：db / scan / execute / ipc（GUI 与 CLI 共用）
cli/          命令行入口 c-clean-cli（不依赖 Tauri）
elevation/    独立提权 exe（按需 UAC，完成即退出）
rules/        JSON 规则库
docs/         架构、用户手册、测试报告、项目状态
tasks/        任务文档（按编号）
```

## 3. 代码风格

### Rust

- 必须通过 `cargo fmt --all` 与 `cargo clippy --workspace --all-targets -- -D warnings`；
- 错误路径显式处理，禁止在生产代码中用 `unwrap()` / `expect()` 兜底外部输入；
- IPC 返回值统一用 `Envelope<T>`，跨层字段使用 `camelCase`；
- 数据库迁移只允许**追加**：新增 `engine/src/db/migrations/vN__*.sql`，同步更新 `migrate.rs` 的 `MIGRATIONS`、版本断言与 `engine/tests/db_schema.rs`；不得修改已发布的迁移文件；
- 注释与文档使用中文，写「为什么」而不是复述代码；
- 新增逻辑在同文件 `#[cfg(test)]` 或 `engine/tests/` 补测试，涉及真实文件操作的测试必须使用 `tempfile`，不得触碰真实用户目录。

### 前端

- 必须通过 `pnpm --filter app lint`（`--max-warnings 10`）与 `pnpm --filter app test`；
- TypeScript 严格模式，不引入 `any` 绕过类型（确实需要时在同行注释说明原因）；
- 组件测试用 Vitest + Testing Library，mock `@tauri-apps/api/core` 的 `invoke`；
- 文案优先走 `app/src/i18n/`（`zh-CN.json` / `en-US.json`），新增 key 两种语言都要补；
- 不新增状态管理库或 UI 框架；样式沿用现有 CSS 变量与组件结构。

### 提交前门禁

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm --filter app lint
pnpm --filter app test
pnpm --filter app build
node --test scripts/tauri-resources.test.mjs
```

七项全绿再提交；CI 会在 `windows-latest` 上复跑。

## 4. 安全红线（必须遵守）

1. **不得绕过 L3**：内置红线路径只能由 L3 拦截，自定义规则只允许 L1 / L2；
2. **删除必须经过保险箱**：除保险箱自身的过期清理外，不得直接 `remove_file` / `remove_dir_all`；
3. **执行前必须可预演**：新增清理入口要支持 dry-run，并在 UI 展示将被删除的清单；
4. **提权命令白名单**：新增提权操作必须登记到 `elevation` 的固定命令表，参数硬编码，不接受外部拼接；
5. **无遥测**：不得引入统计、崩溃上报、云同步等外联行为；
6. **不硬编码真实用户路径、凭据、token**；测试数据一律用 `tempfile` 或占位符；
7. **不修改 `.gitignore`、CI 配置、版本号**，除非任务明确要求。

## 5. 提交与 PR 流程

1. 从最新 `main` 拉分支：`feature/task-<编号>-<slug>`；
2. 一个 PR 只做一个任务，保持原子提交，提交信息格式：

   ```
   <type>(<scope>): <描述> (#task-<编号>)
   ```

   `type` 取 `feat` / `fix` / `docs` / `test` / `refactor` / `chore` / `security`；
3. 显式 `git add <file>`，禁止 `git add .` / `git commit -a`；
4. PR 描述写清：改了什么、为什么改、如何验证、风险与回滚方式；UI 改动附截图；
5. 评审通过且 CI 全绿后再合并；禁止直接 push `main`。

### PR 检查清单

- [ ] 改动范围与任务文档一致，没有顺手重构无关模块；
- [ ] 新增/修改逻辑有对应测试，且全量门禁通过；
- [ ] 涉及清理规则时，已说明 L1/L2/L3 分级与 dry-run 行为；
- [ ] 涉及 UI 时，附浅色/深色与 150% 缩放截图（至少一张）；
- [ ] 涉及用户可见行为时，同步更新 `docs/USER_GUIDE.md`；涉及 CLI 时同步 `docs/CLI.md`；
- [ ] 不包含真实用户路径、密钥、调试残留或大文件。

## 6. 文档同步

| 改动 | 需要同步 |
|---|---|
| 用户可见功能 / UI | `docs/USER_GUIDE.md`、`README.md` 功能列表 |
| CLI 参数 / JSON 字段 | `docs/CLI.md` |
| 模块边界 / 安全约束 | `docs/ARCHITECTURE.md` |
| 进度 / 已知问题 | `docs/PROJECT_STATE.md`（由维护者统一更新） |

## 7. 报告安全问题

请勿在公开 Issue 中披露可利用细节，流程见 [docs/SECURITY.md](docs/SECURITY.md)。
