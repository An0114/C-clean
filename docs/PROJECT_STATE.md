# PROJECT_STATE.md

> **特别声明**：本文件是项目唯一真实数据源，也是「每次任务结束必更新、必单独提交」的文件。后续所有 Agent 必须遵守：任何任务完成后都要先更新本文件，再单独提交一次（commit message 形如 `docs(state): ... (#task-00X)`），不得与业务代码混在同一个 commit。

## 当前目标
完成 M0 阶段：初始化 Tauri 2 + Rust + React monorepo 脚手架，跑通最小可运行窗口与公共底座（SQLite schema、白名单删除入口、IPC 网关、规则库加载），为 M1 六大业务模块（A~F）并行开发打好地基。

## 最近一次变更
2026-10-11：完成 PRD / TRD / ARCHITECTURE / AGENTS / 立项决策记录全套长期基线文档，并初始化本状态白板。
2026-10-11：task-001 完成——落地 app/engine/elevation/rules/scripts 五大目录骨架、Cargo workspace + pnpm workspace 双工作区，Tauri 2 最小空窗口（标题「C-Disk Cleaner」）端到端跑通，Cargo.lock 已入库。
2026-10-11：task-002 完成——engine 内新建 db/ 子模块，rusqlite 0.40.2（bundled）连接入口、v1__init.sql 九张表 schema、user_version 手写迁移器，8 条集成测试全绿（含级联删除 / SET NULL / 单行表 CHECK / 迁移幂等）。

## 已完成事项
- [x] 项目立项与仓库初始化（commit 67990f5 / 4972f9d）
- [x] 固化立项阶段 8 项关键决策：Tauri 无 Plan B / Cargo.lock 入库 / 保险箱 5GB FIFO / 规则库 GitHub Raw+SHA-256 / 不买 EV 签名 / 仅 Windows / 按业务模块 A~F 切分 / PROJECT_STATE 由 M0 当天首写（commit c0bdf05）
- [x] 完成产品需求文档 PRD v0.1（commit caae920）
- [x] 完成技术需求文档 TRD v0.1（commit 261b24f）
- [x] 完成系统架构文档 ARCHITECTURE v0.1（commit c1f027a）
- [x] 完成多 Agent 协作与 Git 权限规范 AGENTS.md（commit 3a1bb2c）
- [x] 初始化项目状态白板 PROJECT_STATE.md（本次提交）

## 待办事项
- [x] 001：初始化 monorepo 目录骨架（app/ Tauri+React+Vite、engine/ Rust workspace、elevation/、rules/、scripts/），跑通最小空窗口
- [x] 002：落地公共数据层——SQLite 单文件 schema（settings/scan_session/cleanup_item/cleanup_task/safe_box/audit_log/rules_meta/registry_backup/cleanup_plan）与 rusqlite 连接、迁移
- [ ] 003：落地删除唯一入口 engine::delete::execute() + L3 红线白名单匹配器 + 单元测试（对 PRD P0-7 全部 L3 路径 100% 拦截）
- [ ] 004：落地 IPC 网关骨架——统一响应信封 {code,message,data}、scan.start/cleanup.dryRun 命令、进度事件通道
- [ ] 005：落地 rules/cache_rules.json 初始规则库 + rules_meta.json + SHA-256 校验加载逻辑
- [ ] 006：搭好 CI（GitHub Actions：cargo clippy/fmt/test + pnpm lint/test + 规则 JSON schema 校验）

## 已知问题
- 本地 main 领先 origin/main 共 7 个提交（自 4972f9d 起的文档基线均未 push），M0 启动前需确认远端同步策略。
- 决策 #1 提到"现有 5 个 PowerShell 脚本"作为 M1 兼容层，但仓库内尚无 scripts/ 实际脚本，其来源与 NDJSON 改造方案待对齐。

## 下一步计划
派发 task-003：落地删除唯一入口 `engine::delete::execute()` + L3 红线白名单匹配器 + 单元测试（对 PRD P0-7 全部 L3 路径 100% 拦截）。

## 当前负责人 Agent
DocsKeeper（文档与状态管家）

## Git 提交记录
| Commit | 任务 | 说明 |
|--------|------|------|
| 67990f5 | #task-000 | Initial commit |
| 4972f9d | #task-000 | chore: 初始化项目仓库与文档骨架 |
| c0bdf05 | #task-000 | docs: 固化立项阶段 8 项关键决策并修正 Cargo.lock 策略 |
| caae920 | #task-000 | docs(prd): 新增产品需求文档 v0.1 |
| 261b24f | #task-000 | docs(trd): 新增技术需求文档 v0.1 |
| c1f027a | #task-000 | docs(arch): 新增系统架构文档 v0.1 |
| 3a1bb2c | #task-000 | docs(agents): 新增多Agent协作与Git权限规范 |
| 09adc52 | #task-000 | docs(state): 初始化项目状态白板 |
| bdeeea4 | #task-001 | docs(task): 新增任务001-初始化monorepo脚手架（派发基线） |
| 42cc54f | #task-001 | chore(workspace): 初始化 Cargo workspace 与 pnpm workspace 配置 |
| 5742151 | #task-001 | chore(scaffold): 新增 engine 空 lib 与 elevation 空 bin 占位 crate |
| ecea8cb | #task-001 | feat(ui): 初始化 app/src Vite + React 18 + TypeScript 前端骨架 |
| 496568f | #task-001 | feat(tauri): 接入 app/src-tauri Tauri 2 壳并链接 engine |
| f041803 | #task-001 | chore(scaffold): 补全 rules/ scripts/ 占位与 README 启动说明 |
| （本次） | #task-001 | docs(state): 任务001完成，更新进度与下一步 |
| e62b586 | #task-002 | build(deps): 为 engine 引入 rusqlite bundled 依赖 |
| 8dea10f | #task-002 | feat(db): 新增 engine::db 模块骨架与连接入口 conn::open |
| bac3400 | #task-002 | feat(db): 编写 v1__init.sql 落地 9 张业务表 schema |
| d48933a | #task-002 | feat(db): 实现基于 user_version 的手写迁移器 |
| 2ecc248 | #task-002 | test(db): 补 schema 与迁移器单元/集成测试 |
| e6f18a1 | #task-002 | style(db): 修复 clippy needless_borrow 告警并按 rustfmt 统一格式 |
| （本次） | #task-002 | docs(state): 任务002完成，更新进度与下一步 |

---
变更轨迹：
- 2026-10-11  task-000  初始化 PROJECT_STATE.md，记录立项基线完成状态与 M0 待办
- 2026-10-11  task-001  初始化 monorepo 脚手架与 Tauri 2 最小空窗口
- 2026-10-11  task-002  落地 SQLite schema（9 张表）+ rusqlite 连接 + user_version 迁移器
