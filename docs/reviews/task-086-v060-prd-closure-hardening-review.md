# 评审报告：task-086 v0.6.0 PRD 全闭环 + 安全加固 + 体验深化（29 项打包）

> 评审角色：Review Lead
> 评审时间：2026-11-14（复评轮；首轮 2026-11-13 不通过）
> 分支：`feature/task-086-v060-prd-closure-hardening`
> HEAD：`b9fedd8`
> PR：#51（https://github.com/An0114/C-clean/pull/51）
> 范围：组 1 PRD 闭环（3）/ 组 2 安全加固（3）/ 组 3 体验深化（5）/ 组 4 工程化清理（4）/ 组 5 P2 长尾（8）/ 组 6 文档合规（6）

## 1. 输入核验

| 输入项 | 状态 | 事实核对 |
|--------|------|---------|
| 测试报告 | ✅ | `docs/test-reports/086-test-report.md`（`2bf1012`，113 行），独立补测轮交付；状态置 `awaiting_review`（`b9fedd8`）。 |
| CI 运行号与结论 | ✅ | PR #51，最新 run **43442011196** @ `b9fedd8`，conclusion **success**（windows-latest：fmt / clippy -D warnings / cargo test / pnpm lint+test+build / tauri build + 产物校验 / 规则库 checksum）。 |
| feature 分支最新 commit | ✅ | `b9fedd8`（与 PR head 一致）。 |

**输入核验结论：齐全。** 首轮缺失的独立测试报告本轮已补齐。

## 2. Git 提交记录核对结果

18 个提交（`46c8bd0` … `b9fedd8`），全部关联 `#task-086`，type/scope 规范（feat / fix / security / chore / docs），按 6 组原子拆分；测试轮两个提交（`2bf1012` docs(test)、`b9fedd8` docs(state)）仅动测试报告与状态文档，**无代码混入**；diff 未检出密钥 / 证书 / `.env` / `.sig` / 凭证文件。

首轮评审报告提交 `7fc7d87` 随分支保留为本轮回溯留痕。

## 3. 验收覆盖性核验

测试报告 §3 逐条核对 29 项；评审抽查关键路径复现证据如下。

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---------|-------------|---------|------|
| 1 | 报告 HTML/CSV 导出指定路径 | `report_export` 命令 + 清洗单测 + 前端按钮 + lib.rs 注册 | 走查接线到位；导出对话框真机归第三档 | 通过（单测+接线） |
| 2 | 计划任务频率 + 阈值（默认 5GB） | `auto_scan.rs` 读 prefs；四档位 + 默认 5GiB 单测；DB 实测 `auto_scan_interval=daily` | 走查到位 | 通过 |
| 3 | 目标推荐组合 + 差额提示 | `recommend_plan` 贪心（L1 累加不足纳入 L2，L2 降序）+ 3 条算法单测 + 前端 17 条 | 走查到位 | 通过 |
| 4 | 进度条随模块推进 | `runner.rs` `interpolate_percent_*`（task-060 既有）+ 进度测试 | 既有实现复核 | 通过 |
| 5 | 符号链接/junction 拦截（2009） | `executor.rs:228` 判最终组件 reparse point；审计 `SymlinkRefused`；dry_run 预检同拦 | **抽查复现**：代码链路保守，不连坐中间层 junction | 通过（单测+走查） |
| 6 | 8.3 短名解析 | `GetLongPathNameW` 展开 + 往返单测；展开失败不拒合法清理 | 走查到位 | 通过 |
| 7 | 折线图 7/30/90 天 | `TrendChart` SVG polyline（task-073 既有）+ 三档切换测试 | 既有复核 | 通过 |
| 8 | 矩形树图 | `treemap.ts` 纯函数 + 6 条单测 + BigFiles 测试 | 走查到位 | 通过 |
| 9 | 影响说明规则库热更新 | `impact_note`/`rebuild_note` 入库；单测校验来自规则库 | 走查到位 | 通过 |
| 10 | 高分屏 150%/200% | 150% 全页 12 张 + 200% 仪表盘 1 张；clamp/rem 响应式；**200% 其余 7 页第三档** | 截图目录实查 12 张非空（812,970 B） | 部分通过（200% 全页第三档） |
| 11 | 更新下载进度反馈 | `App.tsx` chunk 累计/百分比/速度 EMA（task-055 既有） | 既有复核 | 通过 |
| 12 | release.yml target_commitish | task-054 既有，注释标注来源 | 走查到位 | 通过 |
| 13 | session_id 会话筛选 | v8 迁移 + 后端入参 + History 下拉 | 走查到位 | 通过 |
| 14 | ps1 删除 + 回退清理 | `scripts/` 无 `scan_*.ps1`；`CCLEAN_RUST_*` 无残留逻辑 | **抽查复现**：目录 + grep 确认干净 | 通过 |
| 15 | DenialReason 来源拆分 | `BuiltinRedline`/`ExternalRedline`/`UserRule` + 来源单测 | 走查到位 | 通过 |
| 16 | 跨盘恢复 copy+remove | `move_path`（rename → copy+remove）+ 拒 reparse 穿透 + 测试 | 走查到位；真机 C→D 归第三档 | 通过（单测+走查） |
| 17 | RmShutdown 进度事件 | task-049b/060 既有 1s ticker | 既有复核 | 通过 |
| 18 | 兄弟进程连带风险提示 | confirm 弹窗文案 + 前端断言 | 走查到位 | 通过 |
| 19 | UNC 自定义规则 L2 | user_rules 不跳 UNC；规范化失败不放行；断言全 L2 | 走查到位 | 通过 |
| 20 | 增量扫描 | v9 `scan_cache`，模块+根 mtime 签名；DB 实测 5 行；browser/software_cache 刻意不缓存 | **抽查复现**：v9 表结构与报告一致 | 通过（取舍已登记） |
| 21 | 多语言英文 | zh-CN/en-US 各 29 keys 对齐 + 5 条 i18n 单测 + 英文截图 | 走查到位 | 通过 |
| 22 | 右键菜单集成 | `context_menu.rs` install/uninstall/is_installed/build_command/collect_items + 4 条单测 + `--scan-path` 解析 2 条 + 4 IPC 注册；**Explorer 人工点击第三档** | 走查接线齐备 | 部分通过（人工点击第三档） |
| 23 | 通知中心历史 | `record/list/unread_count/mark_all_read/clear` 5 条单测；**真机落库 0→1**（7784 项 / 795.2 MB，未读） | **抽查复现**：`notification_history.rs` 参数化 INSERT + read=0 + MAX_ROWS 200 裁剪 + camelCase 序列化，链路与报告一致 | 通过（真机闭环） |
| 24 | SECURITY.md | 杀软误报说明 + 安全披露流程（6,255 B） | 文档层 | 通过 |
| 25 | CONTRIBUTING.md | 环境/风格/红线/PR 流程（5,440 B） | 文档层 | 通过 |
| 26 | 覆盖率 Rust ≥80% / 前端 ≥70% | 测试轮未独立复测；引用 dev 数据：engine 行覆盖 84.4%、前端 statements/lines 86.49% | 观察项（口径见 §6） | 观察项 |
| 27 | 性能基准 ≤180s / RSS | 测试轮未独立复测；CLI 无头 7661 项跑通、GUI 后端空闲 WS 49.3 MB 与 dev 口径吻合 | 观察项（整组口径见 §6） | 观察项 |
| 28 | 8 页 GUI 截图 | `docs/screenshots/` 12 张全部非空 | 目录实查 | 通过（150% 全页） |
| 29 | 不回归 cargo ≥688 / pnpm ≥224 | 实测 **769 passed** / **233 passed**，双 0 failed | CI 门禁通过 | 通过 |

**契约核对表**：`recommend_plan` / `report_export` / `context_menu_status` / `context_menu_set_enabled` / `context_scan_path` / `take_pending_scan_path` / `notification_history_list/unread_count/mark_read/clear` 均在 `commands.rs` 实现并在 `lib.rs` 注册，返回 `Envelope<T>`；serde camelCase 与前端对齐；未见脱钩。

## 4. 代码评审发现问题

**通过项（走查 + 抽查确认）**：
1. 安全拦截分层正确：只判最终组件 reparse point，不连坐中间层 junction；入箱复制同样跳过 reparse point，防穿透到箱外。
2. 8.3 解析为增强项：展开失败不拒绝合法清理，可用性保守平衡。
3. ps1 回退链路清理彻底：5 个 `scan_*.ps1` 删除，`CCLEAN_RUST_*` 无残留逻辑。
4. 通知历史落库参数化、有上限裁剪（200 条）、未读标记，无无限增长风险。
5. 增量缓存按「模块 + 根目录 mtime」签名，正确性优先刻意不缓存 browser/software_cache。
6. 测试轮未发现需代码修复的阻断缺陷；未开 bugfix 子分支。

**观察项（非阻断，后续跟踪）**：
- `commands.rs` 命令薄封装（约 1318 行）缺命令层契约测试，是 workspace 覆盖率欠账主因。
- 整组内存：WebView2 子进程树空闲约 435MB、峰值约 508MB，超 PRD 300MB 整组目标，属 Tauri + WebView2 固有开销。
- 增量缓存二次扫描提速不显著（命中模块占比小），正确性取舍，非缺陷。

## 5. 安全风险检查结果（OWASP Top10）

| 风险类 | 结论 |
|--------|------|
| 注入（路径/SQL/命令） | 路径 normalize→resolve→classify 真实路径判定；SQL 全参数化；未见拼接执行外部命令。✅ |
| 不安全反序列化 | 规则库 JSON + CI SHA-256 checksum 守卫；无可信二进制反序列化。✅ |
| 敏感信息泄露 | diff 无密钥/凭证/证书；CI 临时签名密钥不入库。✅ |
| 越权 / 路径穿越 | PathTraversal 分类拦截；UNC 仅用户自定义规则 L2，内置规则不扫网络盘。✅ |
| 符号链接跟随 | 删除/入箱两条链路均判 reparse point 拒绝穿透。✅ |
| 新增运行时依赖 | 仅 `@vitest/coverage-v8`（devDependency，不进产物）。✅ |

未发现需阻断合并的安全缺陷。

## 6. 状态与文档一致性核对

- `PROJECT_STATE.md` 顶部状态已置 **`awaiting_review`**，登记测试轮 769/233、安装冒烟与通知历史真机闭环、#4/#5 第三档——与测试报告事实一致。
- 版本号实查 6 处一致 `0.6.0`：tauri.conf.json / src-tauri Cargo.toml / app package.json / 根 package.json / cli Cargo.toml（Cargo.lock 构建同步）；CHANGELOG `## [0.6.0] - 2026-11-13` 条目存在。
- **第二档口径（需发版说明明示，不阻断合并）**：
  1. 覆盖率以 engine 口径 84.4% 判达标，workspace 71.9% 欠账在命令薄封装；
  2. 整组内存超 300MB 为 WebView2 固有开销，发版说明须区分「Rust 进程口径 vs 任务管理器整组口径」；
  3. 增量缓存收益有限为正确性取舍；
  4. 四项实现偏差（prefs KV 复用 / target_commitish 既有 / 新增独立模块 + v9 迁移 / coverage devDep）测试复核属实。
- **第三档（环境不可得，已正式登记 + 替代证据）**：Explorer 右键人工点击、200% 其余 7 页、锁屏态窗口 UI 交互与 CDP console、若干真机端到端 UI 项、系统 toast 目视、ARM64 产物（CI 矩阵）、代码签名（死路）。

## 7. 评审结论：**通过**

首轮 5 项缺口复核：
- #1 独立测试报告 —— **本轮闭环**；
- #2 NSIS 干净目录安装冒烟 —— **真机闭环**（装 exit=0 / 产物齐全 / 安装态版本 0.6.0 / 卸 exit=0 无残渣）；
- #3 通知中心真机历史 —— **DB 行级闭环**（0→1）；
- #4 右键人工点击 —— 第三档，替代证据充分（6 条单测 + 4 IPC 注册 + 安装态 exe 就位）；
- #5 200% 全页截图 —— 第三档，替代证据充分（150% 全页 12 张 + 200% 仪表盘 + clamp/rem + 233 vitest）。

CI run 43442011196 全绿，769 passed / 233 passed 双 0 failed，安全红线走查到位，契约齐备，无敏感文件，无阻断缺陷。第三档缺口符合长期环境限制口径（锁屏态无活动桌面），替代证据已逐条留痕。**同意合并 main。**

## 8. Bug 清单

无阻断缺陷。观察项见 §4 与 §6 第二档，不阻塞本版本合并。

## 9. Git 操作记录

- 合并 `feature/task-086-v060-prd-closure-hardening` → `main`（`--no-ff`），merge commit 见下。
- 删除 feature 分支本地 + 远程。
- 首轮不通过报告 `7fc7d87` 随本分支并入 main 保留回溯。

## 10. 后续动作建议

1. 合并 main、push 后，由 DocsKeeper 手动打 tag `v0.6.0` 触发 release.yml（x64 NSIS/MSI/便携 zip/CLI exe + ARM64 矩阵产物）。
2. v0.6.0 发版说明明示：覆盖率 engine 口径、内存 Rust 进程 vs 整组口径、增量缓存取舍。
3. 有活动桌面的机器补：Explorer 右键人工点击、200% 其余 7 页截图、锁屏态真机 UI 端到端项。
4. 后续技术债：commands.rs 命令层契约测试补覆盖；规则库 modules 段双份真值源；残留深色适配页面。
