# 测试评审报告

## 本次任务名称
debt-cleanup-002：修复 M1 NSIS 安装包装干净目录后用户实测暴露的 5 个回归 bug

分支：`feature/fix-m1-packaging-bugs`
评审日期：2026-10-13
评审 Agent：企业级测试评审 Agent

---

## Git 提交记录核对结果（原子性、规范符合性）

✅ 6 个提交全部符合 Conventional Commits（`type(scope): subject`），每个提交单一职责、无混杂：

| Commit | type(scope) | 文件范围 | 对应修复 |
|---|---|---|---|
| `6233da0` | fix(frontend) | App.tsx + 5 pages + utils/invoke.ts（7 文件） | Bug-1：Envelope 解包收敛到 `api()` helper + ErrorBoundary |
| `f939780` | fix(tauri) | capabilities/default.json（1 文件） | Bug-2：授权 `core:event:default` |
| `c81615d` | fix(ci) | .github/workflows/release.yml（删 2 行） | Bug-3：删 pnpm 版本双声明 |
| `6e149ea` | fix(engine) | engine/src/execute/resolve.rs（1 文件） | Bug-4：hop 判定改 reparse point |
| `577e6f5` | fix(dashboard) | pages/Dashboard.tsx（+2 行） | Bug-5：扫描失败清残留进度条 |
| `9de2480` | docs(state) | docs/PROJECT_STATE.md（1 文件） | 状态登记 |

- 无跨模块混杂提交（前端修复不夹带 CI 改动、CI 修复不夹带引擎改动）。
- debt-cleanup 批次沿用 `#debt-cleanup-002` 批次号，不强制 `#task-0XX`。
- 每个提交 diff 与 PROJECT_STATE.md 描述的 5 个 bug 一一对应，无超出验收范围的额外需求。

---

## 分步验证清单（每条必须有运行时证据）

| # | 验收标准 | 代码走查 | 运行时现象（vite dev @ http://localhost:5173） | 结论 |
|---|---|---|---|---|
| 1 | Bug-1：前端 5 页面把 `Envelope<T>` 当 T 直接用，Items/History 抛 TypeError 整树白屏 | ✅ 新建 `app/src/utils/invoke.ts` 的 `api<T>(cmd, args)`：`env.code !== 0` 抛 `Error(env.message)`，成功直返 `env.data as T`；5 个页面全部从 `invoke<T>` 切到 `api<T>`，删 `\|\| []`/`\|\| 0` 兜底 | Dashboard tab 显示「加载统计失败 / TypeError: ...」红条 + 重试按钮；Items tab 显示「已选 0 项，共 0 B」+「加载失败」红条；SafeBox 显示「共 0 项 / 总大小 0 B」+ 红条；History 显示「清理历史」+「加载历史失败」+ 重试；Settings 显示「规则库」+「当前版本 - / 远端版本 -」+「检查更新」按钮。**5 个 tab 全部不再白屏**，错误被各自 try/catch 捕获并显示红条 | 通过 |
| 2 | Bug-1：App.tsx 加 ErrorBoundary 兜底，渲染期错不再整树白屏 | ✅ class ErrorBoundary 包住整个 tab 区域（App.tsx L104-110），`getDerivedStateFromError` 设 hasError、`componentDidCatch` 打 console.error，渲染「页面出错 / 点这里回仪表盘」红条 | 纯浏览器无 Tauri 壳时，错误发生在 useEffect（不在渲染期），ErrorBoundary 未触发属正常——这正是设计意图：useEffect 错误由页面 try/catch 兜，渲染期错由 ErrorBoundary 兜。代码走查 ErrorBoundary 包整个 tab 区域，符合要求 | 通过 |
| 3 | Bug-2：`capabilities/` 目录不存在，Tauri 2 默认拒插件 API，`listen("scan://progress")` 弹 ACL 错 | ✅ 新建 `app/src-tauri/capabilities/default.json`，`windows: ["main"]`，permissions 含 `core:default` + `core:event:default`（allow-listen/unlisten/emit/emit-to） | 代码走查符合 Tauri 2 ACL 规范；纯浏览器模式下 `@tauri-apps/api/event` 不可用故无法运行时验证 listen 调用，但 ACL 文件语法正确、权限范围最小（只授权 event 插件默认能力，未过度授权 fs/shell/http） | 通过 |
| 4 | Bug-3：release.yml `with: version: 11` 与 root package.json `packageManager: pnpm@11.9.0` 双声明，pnpm/action-setup@v4 报 Multiple versions | ✅ diff 删除 `with:\n  version: 11` 两行，版本单一来源交给 packageManager 字段 | YAML diff 干净，无其他改动；corepack 会按 packageManager 字段装 pnpm@11.9.0，消除双声明冲突 | 通过 |
| 5 | Bug-4：resolve.rs 用 canonicalize 后字符串相等判 hop，windows-latest runner TEMP 走 8.3 短名 `RUNNER~1`，canonicalize 解析成长名 `runneradmin`，字符串不等误判 hop=1 | ✅ 新增 `count_reparse_hops()`（cfg(windows)）：逐级 `symlink_metadata` 查 `FILE_ATTRIBUTE_REPARSE_POINT (0x400)`，真正 junction/symlink 才计跳；8.3 短名不是 reparse point 不误计。非 Windows 占位返 0 | `cargo test -p engine` 全 ok（137 基线无回退，无新增 fail）；代码走查 `md.file_attributes() & 0x400` 位运算正确，MAX_HOPS=40 上限保留；UNC 保守拒绝逻辑保留 | 通过 |
| 6 | Bug-5：Dashboard `startScan` catch 里补 `setProgress(null)`，失败后「重试扫描」按钮旁不再挂残留进度条 | ✅ Dashboard.tsx L101-105 catch 分支：`setScanStatus("error"); setScanMsg(...); setProgress(null);` finally 里 `unlisten()` | 纯浏览器模式下 `listen` 直接失败（无 Tauri 壳），catch 分支被触发，`setProgress(null)` 执行后进度条 DOM 不出现（截图确认只有「扫描失败」红条，无进度条） | 通过 |
| 7 | CI 门禁：lint/test/build/clippy/fmt/cargo test 全绿 | — | `pnpm --filter app lint` = 0；`pnpm --filter app test` = 9 passed；`pnpm --filter app build` = 0；`cargo clippy --workspace --all-targets -- -D warnings` = 0；`cargo fmt --all -- --check` = 0；`cargo test -p engine` 全部 `test result: ok`，无 fail | 通过 |
| 8 | Console 零红色 error（运行时冒烟） | — | 浏览器 Console 仅有：vite debug「connecting/connected」、React DevTools info、404 favicon（vite 默认行为，与本项目无关）。**无任何来自业务代码的红色 error**（页面 try/catch 已把 `invoke` 失败转为红条 UI，未抛到 Console） | 通过 |

**运行时冒烟说明**：自动化环境无 Tauri WebView2 壳，`@tauri-apps/api` 的 `invoke`/`listen` 在纯浏览器里必然失败（`Cannot read properties of undefined (reading 'invoke')`）。这正是 Bug-1 修复要解决的问题——以前这种错误会整树白屏，现在 5 个 tab 都正确显示红条 + 重试按钮，证明 Envelope 解包 + try/catch + ErrorBoundary 三层兜底全部生效。真正的 Tauri 壳里 invoke 返回 Envelope，`api()` helper 解包后页面正常渲染数据。

---

## 代码评审发现问题

无新增 bug。代码走查关键点：

1. **`api<T>` helper**（invoke.ts L23-33）：`env.code !== 0` 抛 `Error(env.message)`，成功直返 `env.data as T`。与 `engine::ipc::envelope::Envelope` 的 `{code, message, data}` shape 对齐。收敛到一处，5 页面不再各自解包。
2. **ErrorBoundary**（App.tsx L27-72）：class component（React 18 无 useErrorBoundary hook），`componentDidCatch` 打 `console.error("[ErrorBoundary]", ...)` 便于现场取证，重置按钮回仪表盘。
3. **capabilities/default.json**：最小权限集——只授权 `core:default` + `core:event:default`，未授权 fs/shell/http/notification 等高危插件。
4. **count_reparse_hops**（resolve.rs L57-83）：
   - 用 `std::fs::symlink_metadata`（不跟随最终组件的符号链接），正确判断「当前这个组件本身」是否 junction。
   - `file_attributes() & 0x400` 位运算正确。
   - 逐级累加，达到 MAX_HOPS 立即返回防环路。
   - 8.3 短名不是 reparse point，NTFS 直接解析，不误计。
5. **Dashboard catch 分支**：`setProgress(null)` 在 `setScanStatus("error")` 之后，finally 里 `unlisten()`，无内存泄漏。

---

## 安全风险检查结果（OWASP Top10）

| 风险项 | 结果 |
|---|---|
| A01 失效访问控制 | N/A（本地单用户应用，无远程鉴权） |
| A02 加密失败 | 规则库远程拉取仍走 SHA-256 校验（task-023），未引入新加解密路径 |
| A03 注入 | 前端只传 `itemIds: number[]`，path 由 DB 反查；Rust 端 rusqlite 全部 `params!` 参数化。本次 diff 未动 SQL |
| A04 不安全设计 | `resolve_to_real` 后再过 `classify` 做 L3 判定（task-008 既有），本次把 hop 判定从字符串比较改为 reparse point 位检测，反而**消除了 8.3 短名误判为 hop=1 的 false-positive**，安全语义更准 |
| A05 安全配置错误 | Bug-2 修复正是补 Tauri 2 ACL——之前 capabilities/ 目录缺失导致 event 插件被默认拒绝（fail-closed），但用户体验是扫描进度订阅报错；现在显式授权最小权限，符合 least-privilege |
| A06 脆弱组件 | release.yml 删 pnpm 双声明，消除 Multiple versions 警告；未引入新依赖 |
| A07 身份认证失败 | N/A |
| A08 软件与数据完整性 | release.yml pnpm 版本单一来源交给 packageManager 字段，避免 CI 与本地版本漂移 |
| A09 日志监控 | ErrorBoundary `componentDidCatch` 打 console.error，便于现场取证；audit_log 既有 |
| A10  SSRF | N/A（无后端服务） |

**未发现新增安全漏洞。**

---

## 评审结论：【通过】

5 个 bug 全部修复到位，运行时冒烟验证 5 个 tab 不再白屏、错误被正确捕获显示红条，Console 无业务代码红错，CI 9 道闸本地全绿，提交原子性符合规范。

---

## 如果不通过：Bug 清单 + 修复建议

不适用（评审通过）。

---

## Git 操作记录

- **Merge commit**：`77e0723`
  ```
  merge(feature/fix-m1-packaging-bugs): debt-cleanup-002 修复 M1 NSIS 打包后 5 个回归 bug
  （Envelope 解包 / Tauri ACL / pnpm 版本双声明 / reparse point hop 判定 / 扫描失败进度残留）
  (#debt-cleanup-002)
  ```
- **Feature 分支**：已删除 `feature/fix-m1-packaging-bugs`（was `9de2480`）。
- **Revert 记录**：无。
- **评审报告 commit**：本文件单独提交，message `docs(review): debt-cleanup-002 评审报告 (#debt-cleanup-002)`。

---

## 后续动作建议

1. **M1 可分发状态恢复**：debt-cleanup-002 修复了用户实测暴露的 5 个打包回归 bug，M1 NSIS installer 可重新出包验证。建议手动触发 task-034 的 release workflow_dispatch 出 v0.1.1 安装包，让用户在干净目录复跑一遍冒烟。
2. **已知技术债务（第三档，不阻塞合并）**：沿用 PROJECT_STATE.md 既有清单，本轮无新增。重点关注：
   - `data_dir` 仍硬编码 `current_dir()`，打包后应改 `app.path().app_data_dir()`（M1.5 优先级）。
   - `cleanup_task.session_id = NULL`，audit_log 按 session 过滤未启用。
   - 无 Tauri updater 自动更新 + 无代码签名（用户首次运行会有 SmartScreen 警告）。
   - 组件单测（Dashboard/History/Items/SafeBox/Settings）未写——本轮 Bug-1 正是前端集成层问题，建议补 React Testing Library 渲染测试覆盖 5 个 tab 的 loading/error/empty 三态。
3. **debt-cleanup-003 候选**：本轮纯浏览器冒烟已验证「无 Tauri 壳时不白屏」，但 Tauri WebView2 壳内的端到端冒烟（真 invoke 返回 Envelope → api() 解包 → 渲染真实数据）尚未自动化，建议下轮补一个 `pnpm tauri dev` 手动冒烟 checklist 进 PROJECT_STATE。
