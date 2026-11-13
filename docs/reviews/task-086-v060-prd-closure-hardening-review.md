# 评审报告：task-086 v0.6.0 PRD 全闭环 + 安全加固 + 体验深化（29 项打包）

> 评审角色：Review Lead
> 评审时间：2026-11-13
> 分支：`feature/task-086-v060-prd-closure-hardening`
> HEAD：`6e0fb04`
> PR：#51（https://github.com/An0114/C-clean/pull/51）
> 范围：组 1 PRD 闭环（3）/ 组 2 安全加固（3）/ 组 3 体验深化（5）/ 组 4 工程化清理（4）/ 组 5 P2 长尾（8）/ 组 6 文档合规（6）

## 1. 输入核验

| 输入项 | 状态 | 事实核对 |
|--------|------|---------|
| 测试报告 | ❌ **缺失** | `docs/test-reports/086-test-report.md` 不存在；`git log main..HEAD -- docs/test-reports` 无任何 086 报告提交；目录清单止于 `085-test-report.md`。任务文档「实现备注」内嵌了 DevAgent 自验证据，但**不能替代独立测试 Agent（提示词 9）的测试报告**。 |
| CI 运行号与结论 | ✅ | PR #51，run **43431889382**，job 131907157654，conclusion **success**（windows-latest：fmt / clippy -D warnings / cargo test / pnpm lint+test+build / tauri build + 产物校验 / 规则库 checksum 全过）。 |
| feature 分支最新 commit | ✅ | `6e0fb04`（与 PR head sha 一致）。 |

**输入核验结论：不通过。** 三样输入缺测试报告，按一票否决规则直接打回测试 Agent 补做；不以「CI 绿、代码没问题」通融。

## 2. Git 提交记录核对结果

16 个提交（`067dfd0` … `6e0fb04`），全部关联 `#task-086`，type/scope 规范（feat / fix / security / chore / docs），原子性按 6 组拆分；无混杂业务/文档混提；diff 未检出密钥 / 证书 / `.env` / `.sig` / 凭证文件。

变更范围：107 文件，+5088 / -2020。与任务书 29 项大合并规模匹配；白名单外必要偏差已在任务文档「实现备注 · 二、白名单对照与偏差」逐条登记（prefs KV 复用未新建 v8 迁移、新增独立模块 `cache.rs` / `notification_history.rs` / `context_menu.rs` 与 v9 迁移、`@vitest/coverage-v8` devDependency 等）。

工作区另有未跟踪 memory 文件与 `tasks/084-*.md` 本地改动，**不属于本分支提交内容**，评审不入账。

## 3. 验收覆盖性核验

测试报告缺失，无法按「报告证据 ↔ 验收标准」逐条核对；以下基于 CI 绿 + 代码走查 + 任务文档自验备注做覆盖性比对，**抽查结论不等同于独立测试轮复核**。

| # | 验收标准 | 代码 / CI 证据 | 抽查结果 | 结论 |
|---|---------|---------------|---------|------|
| 1 | 报告 HTML/CSV 导出到指定路径 | `commands.rs::report_export`（含路径解析/HTML/CSV/文件名清洗单测）+ `Items.tsx` 导出按钮；lib.rs 已注册 | 走查接线到位 | 待测试轮复核真机导出 |
| 2 | 计划任务频率 + 阈值（默认 5GB） | `auto_scan.rs` + `Settings.tsx`；复用 prefs KV | 走查到位 | 待测试轮复核按新配置跑 |
| 3 | 目标推荐组合 + 差额提示 | `recommend.rs` + `PlanTemplates.test.tsx` 17 条；lib.rs 注册 `recommend_plan` | 走查 + 单测存在 | 待测试轮复核 20GB 场景 |
| 4 | 进度条随模块推进 | `runner.rs` `interpolate_percent_*`（task-060 既有） | 备注称既有实现复核通过 | 待测试轮真机复核非 0% |
| 5 | 符号链接/junction 拦截（code 2009） | `executor.rs:228` 仅判最终组件本身；`dry_run.rs:106` 预检同拦；audit 记 `SymlinkRefused`；错误码 2009 已登记 | 走查实现保守、不连坐中间层 junction | 待测试轮真机负向用例 |
| 6 | 8.3 短名解析 | `resolve.rs` `GetLongPathNameW` 展开 + 往返单测；展开失败不拒绝合法清理（增强项） | 走查 + 单测存在 | 待测试轮复核 `SYSTEM~1` |
| 7 | History 折线图 7/30/90 天 | `Dashboard.tsx` `TrendChart`（task-073 既有） | 备注称既有 | 待测试轮复核 |
| 8 | BigFiles 矩形树图 | `treemap.ts` 纯函数 + 6 条单测 + `BigFiles.test.tsx` | 走查 + 单测存在 | 待测试轮复核渲染 |
| 9 | 影响说明规则库热更新 | `impact_note` / `rebuild_note` 入库；`temp_scan_test.rs` 校验来自规则库 | 走查到位 | 待测试轮复核改 JSON 后文案变化 |
| 10 | 高分屏 150%/200% | CSS clamp 响应式；备注称 150% 全页、200% 仅仪表盘 1 张（`10-dashboard-200pct.png`） | **200% 覆盖不全** | 见缺口 #5 |
| 11 | 更新下载进度反馈 | `App.tsx` chunk 累计/百分比/速度 EMA（task-055 既有） | 备注称既有 | 待测试轮复核 |
| 12 | release.yml target_commitish | task-054 既有；本分支 release.yml 改动为 ARM runner 名 + 去 `scripts/` 分发 | 走查到位 | 通过（既有） |
| 13 | session_id 会话筛选 | v8 `audit_log.session_id` 迁移 + History 下拉 + 后端入参 | 走查到位 | 待测试轮复核筛选 |
| 14 | ps1 删除 + 回退清理 | `scripts/` 下已无 `scan_*.ps1`；`CCLEAN_RUST_*` 回退链路删除，源码仅剩注释提及 | 实查目录 + grep 确认干净 | 通过 |
| 15 | DenialReason 来源拆分 | `BuiltinRedline` / `ExternalRedline` / `UserRule`；`classify_with_external` 外部红线归并 | 走查到位 | 待测试轮复核前端文案 |
| 16 | 跨盘恢复 copy+remove fallback | `safe_box.rs` `move_path`（rename → copy+remove，拒 reparse 穿透）+ 单测 | 走查到位 | 待测试轮真机 C→D |
| 17 | RmShutdown 等待进度事件 | task-049b/060 既有 1s ticker（优于任务书 5s） | 备注称既有 | 待测试轮复核 |
| 18 | 兄弟进程连带风险提示 | confirm 弹窗文案「同一控制台内的相关进程可能一并关闭」+ `Items.test.tsx` 断言 | 走查到位 | 待测试轮复核 |
| 19 | UNC 自定义规则 L2 | normalize/resolve/whitelist/user_rules 链路 + 测试；用户规则强制 L2 | 走查到位 | 待测试轮复核 |
| 20 | 增量扫描 | v9 `scan_cache`，模块+根目录 mtime 签名；`browser_cache`/`software_cache` 刻意不缓存（正确性优先） | 取舍已登记 | 待测试轮复核二次扫描 |
| 21 | 多语言英文 | `i18n/` zh-CN/en-US/index + 5 条单测 + 英文界面截图 | 走查 + 单测存在 | 待测试轮复核切换后核心文案 |
| 22 | 右键菜单集成 | `context_menu.rs` 4 条单测 + `--scan-path` 解析 2 条 + Settings 开关；**Explorer 真机右键点击未取证** | **真机缺口** | 见缺口 #4 |
| 23 | 通知中心历史 | `notification_history` 5 条单测；`auto_scan::notify` 先落库再 toast；**本机未跑真实自动清理，历史为空** | **真机缺口** | 见缺口 #3 |
| 24 | SECURITY.md | 杀软误报说明 + 安全披露流程，文件存在 | 走查存在 | 通过（文档层） |
| 25 | CONTRIBUTING.md | 环境/风格/安全红线/PR 流程，文件存在 | 走查存在 | 通过（文档层） |
| 26 | 测试覆盖率 Rust ≥80% / 前端 ≥70% | 备注：engine 行覆盖 84.4%（workspace 71.9%）；前端 statements/lines 86.49%、branches 82.74%、functions 74.21% | 口径已如实记录（commands.rs 薄封装未覆盖） | 待测试轮独立复跑覆盖率 |
| 27 | 性能基准 ≤180s / RSS | 备注：GUI 全扫 5s/峰值 51.1MB、空闲 47.8MB；CLI 11.6s/16.1MB；WebView2 子进程树空闲约 435MB（Chromium 固有开销） | Rust 进程口径达标；**整组口径超 300MB 目标，需发版说明明确口径** | 观察项 |
| 28 | 8 页 GUI 截图 | `docs/screenshots/` 12 张（8 页 + 语言/右键通知/200% 补充） | 目录实查存在 | 待测试轮复核截图真实性 |
| 29 | 不回归：cargo ≥688 / pnpm ≥224 | CI 绿；备注自报 cargo **769 passed / 0 failed / 15 ignored**、pnpm **233 passed（17 files）** | CI 门禁通过 | 待测试轮独立复跑计数 |

**契约核对表**：`recommend_plan` / `report_export` / `context_menu_status` / `context_menu_set_enabled` / `context_menu_scan` / `notification_history_*` 均在 `commands.rs` 实现并在 `lib.rs` 注册，返回 `Envelope<T>`；serde/命令签名走查未见脱钩。

## 4. 代码评审发现问题

**通过项（走查确认）**：
1. **安全拦截分层正确**：`executor.rs` 只对最终组件本身判 reparse point，不连坐 `AppData\Local` 这类中间层 junction；`safe_box` 复制前同样跳过 reparse point，避免穿透到箱外。
2. **8.3 解析为增强而非硬失败**：`GetLongPathNameW` 展开失败时不拒绝合法清理，保守性与可用性平衡合理。
3. **ps1 回退链路清理彻底**：5 个 `scan_*.ps1` 删除，`CCLEAN_RUST_*` 环境开关无残留逻辑（仅注释）。
4. **增量扫描取舍透明**：缓存仅覆盖 temp/prefetch/dev_cache/windows_update；安装根 mtime 不随缓存文件变化的 browser_cache、software_cache 不缓存，避免长期返回旧结果。
5. **错误码与审计留痕**：symlink 拒绝记 audit `SymlinkRefused` + code 2009；ResolveFailed 分类写入 detail。

**观察项（非阻塞，供测试轮 / 后续跟踪）**：
- `engine` 覆盖率 84.4% 达标，但全 workspace 71.9%，主要欠账在 `commands.rs` 的 `#[tauri::command]` 薄封装（1318 行未覆盖）；建议后续补命令层契约测试。
- 整组内存口径：WebView2 子进程树空闲约 435MB、扫描峰值约 508MB，超出 PRD 300MB 目标；备注已说明属 Chromium 固有开销，需在 v0.6.0 发版说明中明确 RSS 口径（Rust 进程 vs 整组）。
- 增量缓存未让二次扫描显著变快（命中模块占比小），备注已如实记录，非缺陷。

## 5. 安全风险检查结果（OWASP Top10）

| 风险类 | 检查结论 |
|--------|---------|
| 注入（路径/SQL/命令） | 路径经 normalize → resolve_to_real → classify 真实路径判定；SQL 走参数化（既有）；未见拼接执行外部命令。✅ |
| 不安全反序列化 | 规则库 JSON 解析 + checksum 守卫（CI 规则库 SHA-256 校验步）；未见反序列化不可信二进制。✅ |
| 敏感信息泄露 | diff 无密钥/凭证/证书；临时签名密钥仅 CI runner 临时生成，不入库。✅ |
| 越权 / 路径穿越 | PathTraversal 分类拦截；UNC 仅允许用户自定义规则 L2，内置规则不自动扫网络盘。✅ |
| 符号链接跟随 | 删除/入箱两条链路均判 reparse point，拒绝穿透。✅ |
| 新增依赖 | 仅 `@vitest/coverage-v8`（devDependency，不进产物）；无运行时新增大依赖。✅ |

未发现需阻断合并的安全缺陷。

## 6. 状态与文档一致性核对

- `PROJECT_STATE.md` 顶部登记状态为 **`testing`**（非 `awaiting_review`），下一阶段写明「测试轮验证 + 合并 main + 打 tag」——与「独立测试轮尚未交付」的事实一致，流程位置正确。
- 任务文档「实现备注 · 七、打包安装冒烟与 CI」自陈：NSIS 干净目录安装冒烟未执行、写备注时 CI 未跑；现 CI 已由 PR #51 触发并绿，但**安装冒烟与真机缺口仍未在独立测试报告中复核/降级登记**。
- 已知问题段已登记：NSIS 安装冒烟待有桌面机器补做、真实通知历史为空、Explorer 右键未人工点击、200% 仅 1 页、整组内存口径、品牌图标/代码签名/G6 不修。
- 版本号实查 5 处文件均 `0.6.0`（tauri.conf.json / src-tauri Cargo.toml / app package.json / 根 package.json / cli Cargo.toml），`Cargo.lock` 由构建同步；未见残留 `0.5.0` 版本号。

## 7. 评审结论：**不通过**

**否决依据**：
1. **测试报告缺失（一票否决项）**：无 `docs/test-reports/086-test-report.md`，独立测试 Agent（提示词 9）轮次未交付；当前证据为 DevAgent 自验备注 + CI 门禁，不满足职责分离要求。
2. **任务书标「必做、不可裁剪」的验收项存在未闭环缺口**，且尚未由独立测试轮确认可降级为第三档：NSIS 干净目录安装冒烟未执行；通知中心真机历史为空；Explorer 右键菜单真机点击未取证；200% 缩放仅 1 页。

**已确认无阻断问题**：CI run 43431889382 全绿；安全拦截（symlink/junction、8.3、路径穿越、UNC）走查到位；IPC 命令注册与 Envelope 契约齐备；ps1 回退清理彻底；无敏感文件；版本号 6 处一致。

**处置**：拒绝合并 main，保留 `feature/task-086-v060-prd-closure-hardening`。打回测试 Agent（提示词 9）补独立测试轮，完成后回到本评审重评。

## 8. Bug / 缺口清单 + 修复建议

| # | 现象 | 复现 / 证据 | 期望行为 | 严重度 |
|---|------|------------|---------|--------|
| 1 | 独立测试报告缺失，状态停在 `testing` | `docs/test-reports/` 无 086 报告；`PROJECT_STATE.md` 状态 `testing` | 测试 Agent 产出 `docs/test-reports/086-test-report.md`，独立重跑 fmt/clippy/cargo test/pnpm lint+test+build，逐条复核 29 项，状态置 `awaiting_review` | **阻塞**（流程一票否决） |
| 2 | NSIS 干净目录安装冒烟未执行 | 实现备注 §七：自动化环境安装器被拦截（历史 exit 126），未覆盖安装用户现有版本 | 在有活动桌面的机器干净目录安装后复跑端到端；若环境仍不可执行，须在测试报告中正式登记为第三档限制并说明替代证据 | 高（任务书「不可裁剪」） |
| 3 | 通知中心真机历史为空 | 备注 §六：本机未跑真实自动清理，`notification_history` 表为空；仅 5 条单测覆盖写入链路 | 真机跑一次自动扫描/清理，确认通知落库并在历史可见；不可执行则登记第三档（链路单测覆盖） | 中 |
| 4 | Explorer 右键菜单真机点击未取证 | 备注 §六：注册表安装/卸载由单测与 Settings 开关覆盖，未在 Explorer 人工右键 | 真机右键文件夹→「用 C-clean 清理」打通 `--scan-path` 启动扫描；不可执行则登记第三档 | 中 |
| 5 | 200% 缩放仅仪表盘 1 张 | 备注 §六：切换系统缩放需注销会话，仅 `10-dashboard-200pct.png` | 补其余 7 页 200% 截图，或在测试报告登记第三档 | 中 |
| 6 | 整组内存超 PRD 300MB 目标 | 备注 §五：WebView2 子进程树空闲约 435MB | 非代码缺陷；发版说明明确「Rust 进程口径 vs 整组口径」，后续如需整组达标再评估渲染层 | 观察项 |

**bugfix 子分支建议**：本轮缺口以补测 / 取证为主，**暂不开 bugfix 子分支**。若测试轮发现代码缺陷，从当前 feature 拉 `bugfix/task-086-<描述>`（只允许一层）；修复完成后回测试 Agent 重测，再回本评审重评。

## 9. Git 操作记录

- 本轮**未合并** main；`feature/task-086-v060-prd-closure-hardening` 保留本地与远程。
- 未删除任何分支；无 revert 记录。
- 评审报告随 `docs(review)` 提交到当前 feature 分支留痕。

## 10. 后续动作建议

1. 测试 Agent（提示词 9）：补独立测试轮，优先闭环缺口 #1–#5；CI 已绿可作为门禁基线，不重复承担真机验收职责。
2. 测试报告中对环境不可得项（安装冒烟 / 通知历史 / 右键点击 / 200% 全页）逐条给出「已取证」或「正式第三档登记 + 替代证据」结论，不留模糊项。
3. 重评通过后：`merge --no-ff` → push main → 删 feature 分支 → 由 DocsKeeper 打 tag `v0.6.0` 触发 release.yml（x64 四产物 + ARM64）。
4. 后续跟踪：commands.rs 命令层契约测试补覆盖；整组内存口径写入 v0.6.0 发版说明。
