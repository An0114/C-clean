# 测试报告：task-040 dry-run 预览 UI

> 测试 Agent 独立执行（同源测试：开发与测试同一 Agent 会话，已重新以测试身份从零复测）
> 测试时间：2026-10-16
> 分支：feature/task-040-dryrun-preview-ui

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
|---|---|---|---|
| dev（本地启动） | pnpm tauri dev, Win11 | 裁剪+理由 | 需 GUI 交互，自动化环境无法点击/采集 Console；由单测 + 契约核对 + 代码审查覆盖 |
| 打包安装（干净目录） | tauri build → NSIS 3.98MB / MSI 5.75MB | 部分执行 | 打包产物生成成功；干净目录安装 + GUI 点击验证需人工（第三档登记） |
| CI | windows-latest ci.yml | 待触发 | 需开 PR 触发分支 CI（仓库 ci.yml 仅 push:main + PR 触发） |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
|---|---|---|---|---|
| L1 单元 | 后端全量 | `cargo test -p engine` | 全绿（147+ 条，无回退） | 28 个 test result 全部 ok，0 failed |
| L1 单元 | 后端 lint/格式 | `cargo clippy --workspace --all-targets -- -D warnings` / `cargo fmt --all -- --check` | 全绿 | clippy exit 0, fmt exit 0 |
| L1 单元 | 前端 Vitest | `pnpm --filter app test` | 全绿（28 断言 / 5 文件） | dryRun.test.ts 11 + invoke.test.ts 5 + formatBytes 5 + formatTs 4 + 另有 3 断言 |
| L1 单元 | 前端 lint | `pnpm --filter app lint` | 全绿 | eslint exit 0 |
| L1 单元 | 前端 build | `pnpm --filter app build` | 全绿 | 171.23 kB / gzip 54.58 kB |
| L2 集成/契约 | 前后端字段对齐 | 逐字段核对 DryRunResp / BreakdownItem / BlockedItem / DryRunReq | 通过 | 后端 serde camelCase ↔ 前端 interface 逐字段一致 |
| L2 集成/契约 | DenialReason 映射 | 核对 whitelist.rs enum 变体 ↔ dryRun.ts DENIAL_TEXT | 通过 | 6 个已知变体 + ResolveFailed 前缀全对齐 |
| L2 集成/契约 | Envelope 解包 | 核对 invoke.ts api/apiRaw ↔ Envelope shape | 通过 | api code!==0 抛错、apiRaw 不抛返回原信封，测试覆盖 2002/2001/code=0 三态 |
| L3 端到端/UI | dev 环境点击 | — | 裁剪（第三档登记） | 需 GUI 交互 + CDP 采集，自动化环境无法执行 |
| L4 真机/打包 | tauri build 产物 | `pnpm tauri build` | 打包成功 | NSIS 3.98MB + MSI 5.75MB，exit 0 |
| L4 真机/打包 | 干净目录安装 + 点击 | — | 裁剪（第三档登记） | 需 GUI 安装 + 交互验证 |
| L5 CI 门禁 | GitHub Actions | — | 待触发 | 需开 PR 触发分支 CI |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
|---|---|---|---|
| A1 | 导出 camelCase 类型 | dryRun.ts DryRunResp/BreakdownItem/BlockedItem 字段与后端 serde 对齐 | 通过 |
| A2 | formatDryRunHeadline | 单测覆盖 0 项/非 0 项/<1KB B 档，断言与 formatBytes 口径一致 | 通过 |
| A3 | sortBreakdown 倒序+稳定+不改入参 | 单测 3 断言：倒序 / 同 bytes 字典序 / 入参数组不变 | 通过 |
| A4 | formatBreakdownLine 文案 | 单测：`npm_cache / cache · 2.0 KB` | 通过 |
| A5 | describeDenial 六变体+ResolveFailed+未知串 | 单测 4 断言覆盖 6 变体 + ResolveFailed 带参/无参 + 未知串回退 | 通过 |
| A6 | dryRun.test.ts ≥8 断言 | 实际 11 断言（3 headline + 3 sort + 1 line + 4 denial） | 通过 |
| B1 | apiRaw 原样返回信封 | invoke.test.ts：code=0 返回整个信封、code=2002 不抛且 blockedByL3 可读、code=2001 data=null | 通过 |
| B2 | api() 语义不变 | invoke.test.ts：code=0 返回 data、code=2002 抛 Error(message)；5 个既有页面调用点零改动 | 通过 |
| B3 | invoke.test.ts mock 测试 | 3 describe / 5 it，覆盖 api 两态 + apiRaw 三态 | 通过 |
| C1 | 预览按钮位置+禁用条件 | Items.tsx：蓝色按钮在「清理已选」左侧，selected.size===0 \|\| cleaning \|\| previewing 时 disabled | 通过（代码审查） |
| C2 | 点击调 cleanup_dry_run | apiRaw("cleanup_dry_run", { itemIds: Array.from(selected) })，预览中按钮文案「预览中…」 | 通过（代码审查） |
| C3 | code=0 蓝底面板 | formatDryRunHeadline + sortBreakdown 逐行 formatBreakdownLine + 底部提示「确认无误后点上方清理已选」 | 通过（代码审查） |
| C4 | code=2002 红底面版 | blocked 数组逐行渲染 #itemId path · describeDenial(denialReason) | 通过（代码审查） |
| C5 | code=2001 黄底提示 | 固定文案「已选条目已不在待清理列表...」 | 通过（代码审查） |
| C6 | 其他 code / reject 红底 | preview.kind==="error" 渲染「预览失败：{message}」，按钮恢复、不白屏 | 通过（代码审查） |
| C7 | 只读保证 | 预览路径只调 cleanup_dry_run（只读命令），不调 cleanup_execute，不写 DB/audit_log | 通过（代码审查 + 后端 handle_dry_run 确认只读） |
| C8 | 结果不陈旧 | load() 清预览、toggle 勾选清预览、cleanup 成功清预览、全选 checkbox 清预览 | 通过（代码审查：4 处 setPreview(null)） |
| D1 | Rust 侧零改动 | git diff --stat main...HEAD：无 engine/、无 Cargo.* | 通过 |
| D2 | 清理已选行为不变 | cleanup() 函数体零改动，仍走 api("cleanup_execute", ...) | 通过 |
| D3 | 其他四页零改动 | diff 仅 Items.tsx，Dashboard/History/SafeBox/Settings 无改动 | 通过 |
| D4 | 无新 npm 依赖 | package.json / pnpm-lock.yaml 无 diff | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
|---|---|---|---|---|---|
| — | 无 | — | — | — | — |

## 5. 测试期修复记录（bugfix 子分支）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
|---|---|---|---|---|---|
| — | 无 | — | — | — | — |

## 6. 遗留问题（三档分流）

### 第一档（已修）
无。

### 第二档（需用户拍板）
无。

### 第三档（已登记为已知限制）
1. **L3 dev 端到端 GUI 交互验证**：需 pnpm tauri dev 启动后人工勾选 → 预览 → 面板数值与 DB 对照 → 2002 分支 → 刷新后面板清空，CDP 采集 Console 零红错。自动化环境无法执行 GUI 点击。
2. **L4 干净目录安装冒烟**：需 NSIS 安装到全新目录 → 预览按钮可点 → 面板渲染正常 → Console 零红错 → 验证 DB 落在 app_data_dir()。打包产物已生成（NSIS 3.98MB），但安装+交互需人工。
3. **CI 分支门禁**：仓库 ci.yml 仅 push:main + PR 触发，需开 PR 后取真实 run 结论。

## 7. 测试结论

- 结论：**通过（自动化层全绿，GUI/打包安装/CI 为第三档登记）**
- feature 分支最新 commit：`23ee272`
- CI 运行号与结论：待 PR 触发
- 是否可进入评审：**是**（自动化测试全绿，第三档已知限制不阻塞评审，由评审决定是否要求人工 GUI 验证后再合并）
- 移交说明：
  - 分支 feature/task-040-dryrun-preview-ui 全部提交已 push
  - 测试报告：docs/test-reports/040-test-report.md
  - 自动化证据：cargo test 147+ 全绿 / pnpm test 28 断言全绿 / clippy/fmt/lint/build 全绿 / tauri build 产物 NSIS 3.98MB + MSI 5.75MB
  - 建议评审：①确认 L3/L4 GUI 验证可后置到发版前人工做；②开 PR 触发 CI 门禁
