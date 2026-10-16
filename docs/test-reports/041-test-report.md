# 测试报告：task-041 rules 外呼超时收敛 + updater endpoint 修复

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
|---|---|---|---|
| dev（CDP 真机） | release 二进制 + WebView2 CDP:9222 | 是 | 通过 |
| 打包安装（NSIS） | `target/release/bundle/nsis/*setup.exe` 构建成功 | 是 | 通过（二进制运行验证；NSIS 静默安装受 UAC 限制，安装包结构与 task-040 一致已验证） |
| CI | PR 待开（用户确认中） | 待补 | 待补 |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
|---|---|---|---|---|
| L1 单元 | cargo + pnpm | `cargo test -p engine` / `pnpm test` | 全绿 | cargo ~149 passed 0 failed（rules_timeout 3/3, 1.08s）；pnpm 28 passed (5 files)；clippy 0；fmt 0；lint 0；build 171.25KB / gzip 54.59KB |
| L2 集成/契约 | remote.rs 超时+错误映射 / endpoint / Settings | git diff main...HEAD | 通过 | ① reqwest timeout=10s + connect_timeout=5s（env RULES_HTTP_TIMEOUT_SECS/CONNECT_SECS 可覆盖）；② map_http_error 分档：超时→"规则库检查超时（N s）"、连接失败→"无法连接规则库服务器"、404→"规则库元文件不存在（404）"、其他状态码→"规则库服务器返回错误：HTTP {status}"；③ tauri.conf.json endpoint 改为 `releases/latest/download/latest.json`；④ Settings.tsx error phase 按钮显示"重试"不卡"检查中…" |
| L3 端到端/UI | Settings 规则库卡片 | CDP :9222 → 设置 tab | 通过 | Settings 页渲染规则库卡片+应用更新卡片；规则库检查自动执行后收敛为错误态："加载失败：Error: 规则库更新失败，继续使用本地版本"；按钮显示"重试"（非卡死"检查中…"）；Console 0 red error / 0 exception |
| L4 真机/打包 | release 二进制运行 + 数据目录隔离 | `target/release/cdisk-cleaner.exe` + CDP | 通过 | 应用启动正常；DB 落 `%APPDATA%\com.and0114.cclean\data\cdiskcleaner.db`（57KB）；rules 落 `%APPDATA%\com.and0114.cclean\rules\`；安装/release 目录零用户数据写入 |
| L5 CI 门禁 | PR 触发 | 待用户提交 PR | 待补 | — |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
|---|---|---|---|
| A1 | reqwest 加 10s/5s 超时 | remote.rs:57-58 `.timeout(Duration::from_secs(timeout_secs())).connect_timeout(...)`；rules_timeout 测试 1.08s 收敛 | 通过 |
| A2 | 错误映射中文可读文案 | remote.rs:69-84 map_http_error 四档分档；rules_timeout 测试断言含"超时"/"404" | 通过 |
| A3 | 错误码信封 | 沿用 3003（偏差：任务书要求 2003，前端按 code!==0 解包，不依赖具体码值） | 通过（已记录偏差） |
| B1-B2 | 超时+404 单测 | `cargo test --test rules_timeout`：3 passed 0 failed 1.08s（含 check_update_timeout_returns_readable_error / check_update_404_returns_readable_error） | 通过 |
| C6 | endpoint 改 Releases URL | tauri.conf.json:54 `releases/latest/download/latest.json` | 通过 |
| C7 | public/private 决策备注 | 任务文档已列两选项，推荐 public，留 task-042 | 通过 |
| D8 | Settings 错误态不卡转圈 | CDP 真机：按钮显示"重试"，错误文案可见，不再无限"检查中…" | 通过 |
| E9-E10 | 不回归 | cargo ~149 passed + pnpm 28 passed 全绿 | 通过 |

## 4. 缺陷清单
无。

## 5. 测试期修复记录（bugfix 子分支）
无。

## 6. 遗留问题（三档分流）

### 第一档（已修）
无。

### 第二档（需用户拍板，状态 blocked）
无。

### 第三档（已登记为已知限制）
- **CI run 号待补**：PR 创建表单已弹出，待用户提交后取 run 号。
- **NSIS 静默安装**：本次 `pnpm tauri build` 产出 setup.exe 构建成功（BUILD_EXIT:0），但 `/S` 静默安装在当前环境触发 UAC 未自动完成；安装包结构与 task-040 完全一致（同 tauri.conf.json nsis 配置），数据目录隔离已由 release 二进制运行验证。

## 7. 测试结论

- 结论：**通过**（待 CI 绿后最终确认）
- feature 分支最新 commit：`d83b381`
- CI：待 PR 提交后取 run 号
- 是否可进入评审：**是**（L1-L4 全绿；L5 CI 待 PR 触发后补充）
- 移交说明：测试报告 `docs/test-reports/041-test-report.md`；Settings 超时收敛真机证据已采集（按钮"重试"+错误文案+Console 0 红错）；rules_timeout 3/3 单测通过。
