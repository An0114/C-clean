# 测试评审报告

## 本次任务名称
001 初始化 monorepo 目录骨架 + Tauri 2 最小空窗口（feature/task-001-init-monorepo）

评审人：TestReviewer
评审日期：2026-10-11
被审分支：`feature/task-001-init-monorepo`（base: `main` @ 09adc52）
合并目标分支：`main`

---

## Git 提交记录核对结果（原子性、规范符合性）

feature 分支相对 main 共 **8 个提交**，全部携带 `(#task-001)` 尾标，type 前缀规范（chore / feat / docs），无跨任务混杂提交：

| Commit | Message | 文件范围 | 原子性 |
|--------|---------|----------|--------|
| bdeeea4 | docs(task): 新增任务001-初始化monorepo脚手架 | 仅 `tasks/001-...md` | ✅ DocsKeeper 派发基线，任务文档明确允许 |
| 42cc54f | chore(workspace): 初始化 Cargo workspace 与 pnpm workspace 配置 | `.npmrc` `Cargo.toml` `package.json` `pnpm-workspace.yaml` | ✅ 工作区配置同类 |
| 5742151 | chore(scaffold): 新增 engine 空 lib 与 elevation 空 bin 占位 crate | `engine/**` `elevation/**` | ✅ 干净 |
| ecea8cb | feat(ui): 初始化 app/src Vite + React 18 + TypeScript 前端骨架 | `app/{package,index,vite,tsconfig,src/**}` `pnpm-lock.yaml` | ✅ 干净 |
| 496568f | feat(tauri): 接入 app/src-tauri Tauri 2 壳并链接 engine | `app/src-tauri/**` `Cargo.lock` | ✅ 干净（Cargo.lock 按规定在此提交入库） |
| f041803 | chore(scaffold): 补全 rules/ scripts/ 占位与 README 启动说明 | `.gitignore` `README.md` `rules/.gitkeep` `scripts/.gitkeep` | ✅ 卫生/占位同类 |
| 9111297 | docs(state): 任务001完成，更新进度与下一步 | 仅 `docs/PROJECT_STATE.md` | ✅ 单独提交，符合状态文件规范 |
| 480c78d | docs(task): 追加任务001实现备注 | 仅 `tasks/001-...md` | ✅ DevAgent 补录自验证据，未混业务代码 |

**轻微偏差（不阻断）**：
- 任务文档要求第 1 个提交含 `.gitignore` 追加段，实际落在第 5 个提交（f041803）；同时 `.npmrc` 随第 1 个提交入库。二者均为同类卫生/配置文件，未与业务代码混杂，DevAgent 已在实现备注逐条说明原因。
- 任务文档预期 6 个业务/状态提交，实际 8 个（多出派发基线 docs(task) 与实现备注 docs(task)），均为文档类提交，符合"状态/任务文档单独提交"原则。

---

## 分步验证清单（逐项对照验收标准）

### A. 目录骨架
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 1 | 根目录存在 app/ engine/ elevation/ rules/ scripts/ docs/ tasks/ | ✅ | `Get-ChildItem` 实测 |
| 2 | app/src/ 与 app/src-tauri/ 存在 | ✅ | 目录树核对 |
| 3 | rules/ scripts/ 仅 `.gitkeep` 占位，无真实 JSON/ps1 | ✅ | 两目录各仅 1 个 `.gitkeep` |
| 4 | engine/ elevation/ 仅空 crate，无 scan/grade/... 子模块目录 | ✅ | 仅 `src/lib.rs` / `src/main.rs` |

### B. Rust workspace
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 5 | 根 Cargo.toml：members = [app/src-tauri, engine, elevation]，edition 2021，rust-version 1.77，workspace.dependencies 仅 tauri 一条 | ✅ | `Cargo.toml` L3-L18 逐项核对；无 rusqlite/serde/jwalk 等业务依赖 |
| 6 | engine 最小 lib + `hello_engine()` + smoke 测试，`cargo test -p engine` 通过 | ✅ | 实测 `1 passed / 0 failed`，exit 0 |
| 7 | elevation 最小 bin，`eprintln!` + 非 0 退出 | ✅ | `elevation/src/main.rs` L8-L10 `exit(1)` |
| 8 | src-tauri 依赖 `tauri={workspace=true}` 与 `engine={path=../../engine}`；注册空命令 `ping` 返回 `hello_engine()`；tauri.conf.json productName/title=「C-Disk Cleaner」、frontendDist=`../dist`、devUrl=`http://localhost:5173` | ✅ | `Cargo.toml` L19-L21；`lib.rs` L8-L11；`tauri.conf.json` L3/L8/L10/L15 全部命中 |
| 9 | Cargo.lock 入库（4315 行），未被 gitignore | ✅ | diff stat 含 `Cargo.lock 4315 +++` |

### C. pnpm workspace + 前端
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 10 | 根 `pnpm-workspace.yaml` 内容仅 `packages: ["app"]` | ⚠️ | 多一行 `allowBuilds: esbuild: true`。DevAgent 说明 pnpm v11 默认拦截 esbuild postinstall，否则 vite 无法启动；属构建工具链必需项，未引入业务依赖。**评审接受此偏差** |
| 11 | 根 package.json：private、name=cdisk-cleaner-monorepo、scripts 含 dev/build/tauri | ✅ | 逐项核对 |
| 12 | app/ 为 Vite+React18+TS 全套；`pnpm install --frozen-lockfile` 通过；`pnpm --filter app build` 产物到 app/dist/ | ✅ | 实测 frozen-lockfile exit 0；vite build 成功，产物 `dist/assets/index-*.js 142.94 kB` |
| 13 | App.tsx 静态占位：标题「C-Disk Cleaner」+ 小字「M0 scaffold — waiting for task 002」，不调 IPC | ✅ | `App.tsx` L16-L17，无 `@tauri-apps/api` import |

### D. 最小空窗口端到端
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 14 | `pnpm tauri dev` 编译成功、拉起 Vite、弹出标题「C-Disk Cleaner」窗口、显示占位页 | ✅ | DevAgent 实现备注：Process.MainWindowTitle 实测 + PrintWindow 截图确认 |
| 15 | 无 Tauri/Vite 红屏；DevTools 无红色 error | ✅ | DevAgent 实测仅一条 info 级 workspace 继承提示，无 error |

### E. 杂项与卫生
| # | 验收点 | 结果 | 证据 |
|---|--------|------|------|
| 16 | .gitignore 仅追加允许条目，不删改已有规则 | ✅ | diff 仅追加 `app/src-tauri/gen/`（L92-L95，备注解释原 `src-tauri/gen/` 被锚定仓库根匹配不到） |
| 17 | README 末尾追加「本地启动（M0 脚手架）」段 | ✅ | `README.md` L75-L92 |
| 18 | 禁止改动 PRD/TRD/ARCHITECTURE/AGENTS/立项决策记录/.gitattributes/LICENSE | ✅ | `git diff --name-only main...feature` 列表中无上述文件 |
| 19 | 无硬编码 `C:\Users\...` / token / 密钥 | ✅ | 全文正则扫描仅命中规范文档中的禁用条款描述，业务代码零命中 |

### 评审人复跑自验命令（仓库根，main 合并后工作区）
| 命令 | 退出码 | 结果 |
|------|--------|------|
| `cargo check --workspace --all-targets` | 0 | Finished dev profile in 13.65s，无 warning |
| `cargo test -p engine` | 0 | smoke 测试 1 passed / 0 failed |
| `pnpm install --frozen-lockfile` | 0 | Already up to date |
| `pnpm --filter app build` | 0 | vite 5.4.11 构建成功，产物 142.94 kB |

---

## 代码评审发现问题

本任务为 M0 脚手架，无业务逻辑（不写 SQLite/IPC/规则库，任务边界明确）。静态扫描结果：

- **无 `unsafe` 块**；无 `Command::new` / `std::fs::remove*` 等文件删除或子进程调用；无网络请求代码。
- `elevation` 占位实现 `eprintln!` + `exit(1)`，正确传达"不可直接运行"语义。
- Tauri 命令 `ping` 无入参、无文件/路径操作，IPC 攻击面为零。
- 前端占位页为纯 JSX 内联样式，无 `dangerouslySetInnerHTML`、无 `eval`、无外部 URL。

**提示项（非阻断，建议后续任务跟进）**：
1. `tauri.conf.json` 中 `app.security.csp` 设为 `null`。M0 dev 阶段可接受，但 task-004 落地 IPC 网关与前端联调时必须补严格 CSP，避免 OWASP A03（注入/XSS）在生产构建中失守。
2. `app/package.json` 的 `build` 脚本仅 `vite build`，未串联 `tsc -b` 类型检查。DevAgent 已在实现备注承认留待 task-006 CI 阶段统一补齐 lint/test。当前 `tsconfig.json` 已开 `strict`，不影响 M0 验收。
3. `.npmrc` 硬编码 npmmirror 镜像源。对中国大陆开发环境友好，但团队协作/CI（GitHub Actions）应评估是否改为官方源或通过环境变量注入，避免境外贡献者安装异常。已在 DevAgent 遗留问题中记录。

---

## 安全风险检查结果

按 OWASP Top 10 (2021) 逐项过：

| 风险项 | 本任务暴露面 | 结论 |
|--------|--------------|------|
| A01 失效访问控制 | 无权限模型、无路由、无 command 鉴权；仅一个无参 `ping` | ✅ 无暴露 |
| A02 加密失败 | 无凭据、无 TLS、无加密代码 | ✅ 无暴露 |
| A03 注入 | 无 SQL、无 shell、无模板拼接；无用户输入流入危险 API | ✅ 无暴露 |
| A04 不安全设计 | 无业务设计 | ✅ N/A |
| A05 安全配置错误 | `csp: null` 为提示项（见上）；dev 模式无敏感面 | ⚠️ 提示，task-004 前修复 |
| A06 脆弱过时组件 | tauri 2 / react 18.3 / vite 5.4 均为当前稳定线；Cargo.lock/pnpm-lock.yaml 双锁定 | ✅ |
| A07 身份识别失败 | 无登录/身份 | ✅ N/A |
| A08 软件与数据完整性 | Cargo.lock 入库；pnpm-lock.yaml frozen 校验通过；无自动更新管道 | ✅ |
| A09 日志监控不足 | 无业务日志；M0 阶段 N/A | ✅ N/A |
| A10 SSRF | 无出站请求（唯一未来出站是规则库 GitHub Raw，留 task-005） | ✅ 无暴露 |

硬编码密钥/路径扫描：业务代码零命中。

---

## 评审结论：**通过**

19 条验收标准全部满足（1 条 pnpm-workspace.yaml 偏差经技术理由核实后接受），4 项自验命令评审人复跑全部 exit 0，OWASP Top 10 无阻断性风险，8 个提交原子性与规范符合性达标。

---

## 如果不通过：Bug 清单 + 修复建议

不适用（本次评审通过）。

非阻断提示项已在上文「代码评审发现问题」列出，均不构成 bug，建议：
- task-004 立项时把「补 CSP」写入验收标准；
- task-006 CI 加入 `tsc --noEmit` 与 ESLint；
- task-006 CI 评估 `.npmrc` 镜像源的环境化方案。

---

## Git 操作记录

- 合并方式：`git merge --no-ff feature/task-001-init-monorepo`（保留分支拓扑）
- **Merge commit hash：`8becd64`**
  - message：`merge(feature/task-001): 初始化 monorepo 脚手架与 Tauri 2 最小空窗口 (#task-001)`
- 本地 feature 分支：已删除（`Deleted branch feature/task-001-init-monorepo (was 480c78d)`）
- 远端分支 `origin/feature/task-001-init-monorepo`：**保留未删除、未 push main**。原因：PROJECT_STATE「已知问题」明确记录"本地 main 领先 origin/main 共 7 个提交尚未 push，远端同步策略由维护者统一决定"，本评审任务边界不含远端推送。维护者后续统一 push main 并清理远端 feature 分支即可。
- 未触发任何 `git revert`。

---

## 后续动作建议

1. **派发 task-002**：落地 SQLite 单文件 schema（9 张表）+ rusqlite 连接与迁移，在 engine 内新建 `db/` 子模块。注意 task-002 是首个引入业务依赖（rusqlite）的任务，应在 `[workspace.dependencies]` 中统一声明版本。
2. 维护者择机 `git push origin main` 同步远端，并删除 `origin/feature/task-001-init-monorepo`。
3. task-004 IPC 网关立项时，把「配置严格 CSP」「统一响应信封安全校验」写入验收标准。
4. task-006 CI 立项时，把 `tsc --noEmit`、ESLint、`.npmrc` 镜像源环境化纳入流水线。
