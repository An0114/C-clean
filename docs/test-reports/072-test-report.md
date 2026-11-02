# 测试报告：task-072 P1-7 清理方案模板（cleanup_plan save/apply/import/export + 前端方案页 + tab 注册）

- 分支：`feature/task-072-plan-templates`
- 测试 HEAD：见 `target/qa-072/head.txt`
- 测试时间：2026-11-02（UTC+8）
- 测试 Agent：QA（同源测试，见 §7）
- **P1-7 清理方案模板（cleanup_plan 表接线 + 前端方案页 + 「方案」tab）**

## 1. 测试环境矩阵

| 环境 | 版本 / 路径 | 是否执行 | 结论 |
| --- | --- | --- | --- |
| 构建验证 | `cargo check/clippy/test` + `pnpm lint/test/build` | 是 | 通过 |
| 打包 | NSIS `C-DiskCleaner_0.2.1_x64-setup.exe`（4.48 MiB / 4699050 字节） | 是 | 通过（出包成功） |
| 干净目录安装 | `/S /D=D:\qa-072-install`（`-Verb RunAs`） | 尝试 2 次 | **未完成**（UAC 非交互被自动取消，归第三档） |

## 2. 分层测试结果

| 层 | 范围 | 命令 | 结果 | 证据 |
| --- | --- | --- | --- | --- |
| L1 单元 | Rust 全工作区（含 24 条新增）+ 前端（含 18 条新增） | `cargo test --workspace` / `pnpm --filter app test` | 通过 | `cargo-test.log`：59 batches / **468 passed / 0 failed**（基线 444 + 新增 24，与开发声称一致）；`pnpm.log`：14 files / **166 passed**（基线 148 + 新增 18，与开发声称一致） |
| L2 集成/契约 | 门禁 | `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `pnpm lint` / `pnpm build` | 通过 | `gate.log`：fmt=0、clippy=0（1 条 incremental lock warning + 2 条 elevation 既有 warning）；`pnpm.log`：lint=0（2 条 SafeBox 既有 warning）、build=0（238.23 kB / gzip 73.90 kB，比 071 的 230.32/71.78 涨 8/+2 kB，符合新页面增量） |
| L3 真机 | tab 栏「方案」按钮可见性 | 安装态截图 | **未完成**（装包未完成） | 归第三档（UAC 非交互受限） |
| L4 打包 | NSIS 出包 | `tauri build` | 通过 | `tauri-build.log`：NSIS 4.48 MiB + MSI 6.62 MiB；FileVersion=0.2.1；签名 exit=1 为已知预期 |

## 3. 验收标准逐条核对

| # | 验收标准 | 运行时现象 / 证据 | 结论 |
| --- | --- | --- | --- |
| 1 | `plan_save(name, selections)` | 单测覆盖：`save_*` 系列（含 `save_dedupes_same_path` / `save_skips_items_that_are_gone` / `save_rejects_empty_selection` / `save_rejects_blank_name`） | 通过 |
| 2 | `plan_list()` 列出所有方案 | 单测覆盖（PlanTemplates 前端 17 条含列表渲染） | 通过 |
| 3 | `plan_delete(id)` 删除 | 单测覆盖（前端删除 confirm 两态） | 通过 |
| 4 | 单测：save/list/delete 逻辑正确 | 24 条后端用例全绿 | 通过 |
| 5 | `plan_apply(id)` 返回方案数据 | 单测覆盖：`apply_still_works_after_rescan_with_new_ids` / `apply_picks_newest_row_when_path_duplicated` / `apply_reports_unmatched_paths` | 通过 |
| 6 | `plan_export(id)` 导出 JSON | 单测覆盖：`export_writes_valid_plan_file`（含 camelCase 反面断言）/ `export_sanitizes_plan_name_in_filename` | 通过 |
| 7 | `plan_import(file_path)` 导入 | 单测覆盖：4 条导入校验（format/version/itemRefs 非空） | 通过 |
| 8 | 单测：apply/export/import 逻辑正确 | 全绿 | 通过 |
| 9 | 方案列表显示名称/时间/条目数/大小 | 前端 `PlanTemplates.test.tsx` 17 条含「挂载时拉取方案列表，并渲染名称/时间/条目数/大小」 | 通过 |
| 10 | 「保存当前选择为方案」按钮 | 前端单测覆盖（保存禁用两态 / 实参 trim / 2001 文案） | 通过 |
| 11 | 「应用」按钮恢复选择到 Items 页 | 单测覆盖（应用三态 / 「—」而非 0 B / 匹配数不一致）；端到端用例 `end_to_end_save_then_apply_through_handlers` 真实 SQLite 零 mock | 通过 |
| 12 | 「导出/导入」JSON 文件操作 | 单测覆盖（导出路径 / 导入空路径不发 IPC / 导入 trim） | 通过 |
| 13 | 「删除」confirm 后删除 | 前端单测覆盖（删除 confirm 两态） | 通过 |
| 14 | 深色主题适配 | 开发声称 CSS 变量适配（未真机验，归第三档） | 单测通过，真机归第三档 |
| 15 | 单测：页面渲染正确 | 17 条全绿 | 通过 |
| 16 | Tab 栏加「方案」按钮 | 前端 `App.test.tsx` 29 条（原 28 + 新增 1 条 tab 切换） | 通过（单测），真机归第三档 |
| 17 | 切换渲染 `<PlanTemplates />` | 单测覆盖 | 通过（单测），真机归第三档 |
| 18 | 单测：tab 切换正确 | 新增 1 条全绿 | 通过 |
| 19 | `cargo test --workspace` 全绿（新增 ≥4 条） | 468 passed（新增 24 条，远超 ≥4） | 通过 |

## 4. 缺陷清单

| # | 现象 | 复现步骤 | 期望行为 | 严重度 | 处理 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮未发现需代码修复的缺陷 | L1-L2 全绿 | — | — | — |

## 5. 测试期修复记录（bugfix 子分支）

| # | 缺陷 | 修复分支 | commit hash | 提交信息 | 回归结果 |
| --- | --- | --- | --- | --- | --- |
| — | 测试轮无需修复 | — | — | — | — |

## 6. 遗留问题（三档分流）

### 第一档（已修）
- cleanup_plan 表接线 6 条 IPC（save/list/delete/apply/export/import）+ 前端 PlanTemplates 页 + tab 注册，24 条后端 + 18 条前端单测全绿。

### 第二档（需评审/用户拍板）
1. **白名单 4 处偏差**：①`engine/src/lib.rs` 无需改动，但真正让前端能 invoke 的 `app/src-tauri/src/{commands,lib}.rs` 漏在白名单外（与 065~068 同样遗漏）；②`app/src/pages/Items.tsx` 必须改（验收 11 绕不过，加 3 个可选 prop 零回归）；③`app/src/App.test.tsx` 加 1 条（验收 18 只能在 App 层测）；④`app/src/utils/invoke.ts` 未改（现有模式无 per-command 封装）。
2. **提交拆法**：任务书要求 4 个代码提交（按功能单元），但单元 1 与 2 同在新建文件 `plan.rs` 里无法干净拆，实际拆成 5 个代码/测试提交。
3. **导出/导入无系统文件选择框**：本项目没引 `tauri-plugin-dialog`，导出写固定目录返回路径、导入靠粘贴路径。任务书写「下载 JSON 文件/选择导入」在无对话框插件前提下做不到——能力边界，不是取巧。若要真·文件选择框需引插件（涨包体积 + capability 配置），建议单开任务。
4. **`is_template` / `note` 字段只写不读**：界面上没做这两个概念，字段留着不动（不编假 UI）。
5. **方案无数量上限**：列表全量渲染无分页/搜索，几十个以内无感，多了要补。
6. **跨页通信用 prop 而非全局 store**：照抄 Dashboard 的 `scanSignal` 先例（App 持 state + prop 下发），三个回调 `useCallback` 固定引用避免 effect 无限触发。

### 第三档（已登记为已知限制）
- **真机 9 条冒烟未在测试轮完成**（UAC 非交互 + 装包被自动取消）：①tab 栏「方案」按钮；②点切换到 PlanTemplates 页；③列表显示名称/时间/条目数；④「保存当前选择」按钮可用；⑤「应用」恢复勾选到 Items 页；⑥「导出/导入」JSON 文件操作；⑦重新扫描后再应用同一方案（三元组而非 id 的关键验证）；⑧深色主题适配；⑨CDP console 0 红错。由 24+18 条新单测 + 开发端到端用例（`end_to_end_save_then_apply_through_handlers` 真实 SQLite 零 mock）覆盖。**建议用户装 0.2.1 后手动点一遍这 9 处。**
- **安装态未完成**：NSIS 出包成功（4.48 MiB / FileVersion=0.2.1），但两次 `-Verb RunAs` 静默安装均被 UAC 非交互自动取消（与既往经验一致），装包/启动/截图未做。
- **CI 五道闸未跑**：开发轮不建 PR，PR 由 CI 门禁观察员创建后跑。
- **签名**：本地未签名（exit=1 已知预期）。
- **`tasks/060-shutdown-experience.md` 「工作区已删除、HEAD 里存在」连续 10 个任务未决**。
- **下一步**：P1 体验项收尾（趋势图折线增强 / 开机自启任务计划集成）或架构债（5 个 ps1→Rust 重写）+ 深色补齐 Items/SafeBox/History 三页。

## 7. 测试结论

- 结论：**通过（468 passed / 0 failed / 166 pnpm / NSIS 出包 4.48 MiB）**
- feature 分支最新 commit：见 `target/qa-072/head.txt`
- 同源测试声明：本测试 Agent 与开发 Agent 同会话（同源）。测试轮独立重跑全部门禁（fmt/clippy/test/lint/build）+ NSIS 出包，未复用开发自验结论。
- 移交说明：feature 分支待 push 后由「提示词 B（CI 门禁观察员）」创建 draft PR 并轮询 CI。测试 Agent 不下合并结论，交评审 Agent（提示词 10）判定。**请评审重点复核：①白名单 4 处偏差（tauri commands/lib.rs、Items.tsx、App.test.tsx、invoke.ts 未改）；②提交拆法 4→5；③导出/导入无系统文件选择框（无 tauri-plugin-dialog）是否接受；④is_template/note 只写不读、方案无数量上限是否认可。9 条真机冒烟归第三档，建议用户装 0.2.1 后手动点验。**
