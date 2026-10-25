# MEMORY.md — C-clean 项目长期约定

_项目级、跨会话生效的稳定约定。用户明确交代的规则写这里，不靠猜。_

---

## CI / 分支流程（2026-10-25 AnFer 明确交代）

**核心：CI 只在测试阶段跑，开发阶段不跑。**

1. **开发阶段（开发分支下）**：**不触发 CI**。写完代码、本地门禁过了就提交、推送，不建 PR、不等 CI。
2. **测试阶段（测试分支下）**：由测试环节**拉 PR + 触发 CI**。
3. **测试期 CI 不通过 = 必须修**：不允许"记一笔遗留"就往下走，红了自己改到绿。
4. **评审阶段**：评审 Agent 只做一件事——**查看测试环节最后那次 CI 是否全绿**。
   - 全绿 → 合并 main
   - 不全绿 → 打回修复，不合并

> 对上一版流程的影响：原「push 后必须等 CI 全绿才能报开发完成」不再适用于开发轮，改为「开发轮报完成只需本地门禁全绿」；CI 门禁整体后移到测试轮，且测试轮的 CI 结果成为评审的唯一放行依据。

## 环境坑（本机）

- `git push` 会因全局 `http.sslbackend=openssl` 报 `TLS connect error: error:0A000126 ... unexpected eof`，**push 时带 `git -c http.sslBackend=schannel`**（未改全局配置）。
- GitHub API 必须经代理 `http://127.0.0.1:8607` 且带 `User-Agent`；`--noproxy '*'` 对 `api.github.com` 不通。
- NSIS `perMachine` 在非管理员 shell 无法静默安装（弹 UAC / 残留 `consent.exe`），安装级布局验证改用 MSI per-user：`msiexec /i x.msi /qn ALLUSERS=2 MSIINSTALLPERUSER=1 INSTALLDIR=<dir>`。
- 本机 WebView2 渲染进程创建即崩溃（Crashpad 新增 `.dmp`、CDP 9222 不通），**GUI 点击证据取不到**，一律按第三档登记 + jsdom/引擎集成用例补偿，不伪造。

## 并发与取证（2026-10-25 task-051 测试轮沉淀）

- **本仓库可能同时被多个 Agent 会话操作**（实测遇到 Codex 在 `bugfix/task-049-close-experience-copy` 上 `reset --hard origin/main`，把别人的 HEAD 顶掉）。开工先 `git branch --show-current` 核对；**每个改动立刻 commit**，别攒批；发现 HEAD 被顶走先用 `git symbolic-ref HEAD refs/heads/<原分支>` 还回去（不动工作区），再决定是否换 worktree。
- **不要用 git worktree 跑前端门禁**：worktree 不在会话工作区内，vitest 默认 forks 池写 `%TEMP%` 报 `EPERM`；`pnpm install --offline` 在该目录产出的 `node_modules` 全是空目录（Windows 建符号链接失败）。要隔离只能复制仓库+hoisted 安装，代价大于收益。
- **GUI 取证替代方案（已验证可用）**：Tauri 窗口 WebView2 渲染进程崩时，用 `msedge --headless=new --screenshot=... --dump-dom` 加载 Vite dev server 上的一页 harness（`window.__TAURI_INTERNALS__ = { invoke }` 打桩），跑真实 React 组件 + 自动点击，能拿到真浏览器渲染、IPC 实参序列、Console 错误数。
- NSIS `perMachine` 静默安装**偶发可成功**（2026-10-25 task-051 测试轮 `/S /D=` exit 0、无 `consent.exe`），不能当作必然；失败时仍走 MSI per-user（`ALLUSERS=2 MSIINSTALLPERUSER=1`）。
- `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` 能让 CDP 端口**瞬时**起来（1.5s 内 200），但渲染进程随即无响应，`Runtime.evaluate` 超时，别指望靠它截图。
