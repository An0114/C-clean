# 任务编号：056 M3 技术评估 + 遗留小修——差量更新评估 + restored_at 审计列 + 跨盘 rename 测试 + G6 排查

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevBackendRustReact + DevFrontendReact
> 来源：PROJECT_STATE「下一步计划 2（差量更新评估）」+「已知问题」段 B 低/B 中遗留项。
> 上游依赖：task-055 已合并 main（merge `4231cbd`，banner 下载进度闭环）。
> **本任务范围（4 个功能单元，厚但低风险）**：差量更新技术评估文档、`restored_at` 审计列、跨盘 rename 测试用例补全、G6 设置页 IPC 红错排查与门禁豁免口径明确化。
> **为什么不是纯开发任务**：M3 下一项「差量更新」PROJECT_STATE 明确说「先做技术评估再立项」；同时清掉几个 B 低/B 中遗留小修，避免它们长期挂在「已知问题」段。
> **不做**：差量更新实际开发（评估完再立项 task-057）、RmRestart（产品未拍板）、代码签名（未购证书）、品牌图标（决策不做）。

## 状态字段

testing

## 任务目标

### 1. 差量更新技术评估文档（M3 下一项立项前置）

- **现状**：banner 下载进度已落地（task-055），但 NSIS 包 ~4 MB、MSI ~6 MB，每次更新全量下载。PROJECT_STATE「下一步计划 2」明确「差量更新是独立大块，建议先做技术评估再立项」。
- **修法**：写 `docs/research/delta-update-evaluation.md`，覆盖：
  - **候选方案 A：Tauri 2 官方 updater 差量支持**——查 `@tauri-apps/plugin-updater` 是否原生支持 bsdiff/xdelta；若不支持，Tauri 2 是否有官方 plugin 或社区方案；
  - **候选方案 B：bsdiff4（Rust crate `bsdiff` / `bzip2`）**——服务端发版时对相邻版本生成 `.bsdiff` 差量包，客户端下载差量包 + 旧版安装包 → 本地 patch 成新版；
  - **候选方案 C：xdelta3**——与 bsdiff 对比（压缩率 / 速度 / Rust 绑定成熟度）；
  - **关键决策点**：
    - 差量包签名（不能直接复用整包签名，需单独签）；
    - 客户端旧版本校验（patch 前必须确认本地是 v0.1.6，否则不能 patch 到 v0.1.7）；
    - 服务端产物矩阵（每相邻版本对一个差量包，v0.1.5→0.1.6、v0.1.6→0.1.7...）；
    - 回退路径（patch 失败自动下载整包）；
  - **结论**：推荐方案 + 预估工作量（人日）+ 风险 + 是否值得做（对 4 MB 包差量是否真省流量）；
  - **输出**：评估文档 + task-057 立项建议（或明确「不做」）。

### 2. `restored_at` 审计列（B 低，task-017 遗留）

- **现状**：`safe_box` 表有 `purged_at`（purge 时写），但 restore 时只 DELETE 行 + audit_log `action='restore'`，**没有时间列**——用户问「这个文件什么时候恢复的」查不到。
- **修法**：
  - **方案 A（推荐）**：`safe_box` 加 `restored_at TEXT NULL` 列（迁移器 v2 加 `ALTER TABLE safe_box ADD COLUMN restored_at TEXT`）；restore 时 UPDATE 该行 `restored_at = NOW()`（DELETE 改为 UPDATE status='restored' + restored_at）；
  - **方案 B（兜底）**：不动 safe_box，audit_log 已经有 `created_at`，前端从 audit_log 反查——但这是 N+1 查询，不推荐；
  - 选方案 A：迁移器 v2 + 后端 `restore` / `restore_many` 改 UPDATE 而非 DELETE + 单测覆盖。

### 3. 跨盘 rename 测试用例补全（B 低，task-016 遗留）

- **现状**：`engine/src/execute/safe_box.rs` 跨盘 rename 走 copy+remove fallback，但 CI 只跑单盘（C 盘），EXDEV 路径未实测。
- **修法**：
  - 写一个集成测试 `safe_box_cross_disk.rs`：
    - 用 `tempfile::TempDir` 在不同盘建两个目录（如 C 盘 + 如果有 D 盘就用 D 盘，否则 skip）；
    - 断言：跨盘 rename 不返回 EXDEV Err，走 copy+remove fallback，文件内容一致；
  - CI 环境只有 C 盘（windows-latest），测试用 `#[cfg_attr(not(windows), ignore)]` + 运行时检测「只有一个盘则 skip」，避免 CI 红。

### 4. G6 设置页 IPC 红错排查与门禁豁免口径明确化（B 中，已知项）

- **现状**：每次文档加载后首次进「设置」tab，`rules_check_update` 走 `http://ipc.localhost/rules_check_update` 报 2 红 + 2 黄，功能经 postMessage 回退可用。task-039 评审时已发现，至今未修。
- **修法**：
  - **排查**：读 Tauri 2.x `useCustomProtocol` 回退策略源码（或 GitHub issue），定位为什么首调走 ipc.localhost 而非 postMessage；
  - **尝试性修复（可选）**：在 `Settings.tsx` 首调前 warmup 一下 IPC 通道（如调一次无副作用的 `app_info` 命令），触发 postMessage 回退；
  - **如果修不动**：明确门禁豁免口径写进 `docs/test-reports/gui-walkthrough-round3-checklist.md`：「G6 为 Tauri 2.x 已知项，发版门禁口径为排除 G6 后 Console 零红，G6 本身不阻塞发版」；
  - **结论**：要么修掉（提交 fix），要么明确豁免口径（提交 docs），二选一，不要挂着。

**本任务不做**：
- 不做差量更新实际开发（评估完再立项 task-057）；
- 不做 RmRestart（产品未拍板）；
- 不做代码签名（未购 EV/OV 证书）；
- 不做品牌图标（决策不做）；
- 不做 Restart Manager 集成（M1-D 大块，留后续）。

## 前置条件

1. `main` 已合并 task-055（merge `4231cbd`）；
2. 关键锚点：
   - `engine/src/db/migrate.rs`（迁移器，加 v2）
   - `engine/src/execute/safe_box.rs`（restore / restore_many 改 UPDATE）
   - `engine/src/execute/safe_box.rs`（跨盘 rename fallback 位置）
   - `app/src/pages/Settings.tsx:87, 104`（G6 首调点）
   - `docs/test-reports/gui-walkthrough-round3-checklist.md`（G6 豁免口径写入位置）
   - 仓库根（新建 `docs/research/delta-update-evaluation.md`）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. 差量更新评估

1. `docs/research/delta-update-evaluation.md` 存在；
2. 覆盖 A/B/C 三个候选方案对比；
3. 含关键决策点（签名 / 旧版本校验 / 产物矩阵 / 回退）；
4. 给出推荐方案 + 预估工作量 + 是否值得做的明确结论；
5. 结论明确「立项 task-057」或「不做，全量下载够用」。

### 2. restored_at 审计列

6. 迁移器 v2：`ALTER TABLE safe_box ADD COLUMN restored_at TEXT NULL`（不破坏 v1 已有数据）；
7. `restore` / `restore_many` 改 UPDATE（status='restored' + restored_at=NOW()），不再 DELETE；
8. `list` 查询过滤 `status='in_box'` 时不返回 restored 行（行为不变）；
9. 单测覆盖：restore 后该行 `restored_at` 非 NULL、`status='restored'`。

### 3. 跨盘 rename 测试

10. `engine/tests/safe_box_cross_disk.rs` 存在；
11. 运行时检测多盘环境，单盘 skip（CI windows-latest 只一个盘，测试应 skip 不红）；
12. 多盘环境下断言跨盘 rename 走 copy+remove fallback，文件内容一致。

### 4. G6 排查与豁免

13. 排查结论写进任务文档「实现备注」：要么「已修，根因是 X，修法 Y」；要么「修不动，根因是 Tauri 2.x ipc.localhost 首调与 useCustomProtocol 回退策略，属于上游 bug」；
14. `gui-walkthrough-round3-checklist.md` 末尾加 G6 豁免口径说明（如果选择修不动路线）；
15. 如果选择修掉：`Settings.tsx` 加 warmup IPC 调用 + 单测覆盖。

### 后端单测

16. `cargo test -p engine` 全绿（现有 250+8 + 本任务新增 ≥3 条全绿）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `docs/research/delta-update-evaluation.md`（新建）
- `engine/src/db/migrate.rs`（v2 迁移）
- `engine/src/db/schema.rs`（safe_box 结构加 restored_at）
- `engine/src/execute/safe_box.rs`（restore 改 UPDATE）
- `engine/tests/safe_box_cross_disk.rs`（新建）
- `app/src/pages/Settings.tsx`（G6 可选修复）
- `docs/test-reports/gui-walkthrough-round3-checklist.md`（G6 豁免口径）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-056-research-and-cleanup`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-056-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `docs(research): 新增差量更新技术评估（bsdiff/xdelta3/Tauri 2 对比） (#task-056)`（单元 1）
2. `feat(db): safe_box 加 restored_at 列，restore 改 UPDATE 不再 DELETE (#task-056)`（单元 2）
   —— 含迁移器 v2 + 后端 + 单测。
3. `test(execute): 补跨盘 rename 集成测试（单盘环境 skip） (#task-056)`（单元 3）
4. `fix(ui) | docs(test): G6 设置页 IPC 红错排查——二选一（warmup 修复或门禁豁免口径） (#task-056)`（单元 4）
   —— 根据排查结果选 fix 或 docs，commit type 对应。
5. 自验全绿后，最后一次提交：`docs(state): 任务056完成，差量更新评估 + restored_at + 跨盘测试 + G6 闭环 (#task-056)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 056: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「已知问题」段把 task-017（restored_at）/ task-016 跨盘测试 / G6 三条标记已解决或明确豁免；
   - 「下一步计划」改为「根据差量更新评估结论决定是否立项 task-057；RmRestart 待产品拍板」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-28  task-056  M3 评估 + 遗留小修（差量评估 / restored_at / 跨盘测试 / G6）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（现有 250+8 + 本任务新增 ≥3 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0（如果动了 Settings.tsx）；
- `pnpm --filter app test` 退出码 0（如果动了 Settings.tsx）；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：restored_at 列写入 + 跨盘 rename fallback + G6 warmup（如修）。
- **集成测试（必做）**：`cargo test -p engine` 全绿，确保现有不回归；跨盘测试在 CI 单盘环境 skip（不红）。
- **端到端写路径（必做，本任务硬要求）**：restored_at 必须有一条真实「点恢复按钮 → 查 DB restored_at 非 NULL → 查 UI 不再显示该条」证据——不能只构造 DB 行。
- **真机冒烟（可选，非必做）**：本任务主体是 DB 迁移 + 测试 + 文档，UI 行为变化小（restored 条不再出现在保险箱列表），但合并后 NSIS 安装态需验证：
  - 迁移器 v2 在已装 v0.1.6（无 restored_at 列）的数据库上跑不报错；
  - 旧数据（in_box 行）保留。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，验证迁移器 v2 正确执行（不破坏旧数据）。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（restored 行不再出现在保险箱列表——用户可观察），打包安装冒烟**必做、不可裁剪**；真机冒烟可选（DevAgent 自验时跑一遍即可）。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 差量更新评估文档 | 文件存在 + 结论段落（推荐方案 + 工作量 + 是否值得做） |
| 迁移器 v2 | `cargo test -p engine` 输出含 migration 用例 passed |
| restored_at 写入 | 单测输出（restore 后该行 restored_at 非 NULL）+ 真机 SQL 查询 `SELECT restored_at FROM safe_box WHERE status='restored' ORDER BY rowid DESC LIMIT 1` 非 NULL |
| restored 行不显示 | 真机截图：保险箱列表不再显示已恢复行 |
| 跨盘 rename 测试 | `cargo test -p engine` 输出（CI 单盘 skip，本地多盘 passed） |
| G6 排查 | 实现备注含「已修 / 修不动」二选一结论 + commit type 对应 |
| 不回归 | `cargo test -p engine` + `pnpm test` 全绿 |
| 打包安装冒烟 | NSIS 干净目录安装 + 迁移器 v2 执行日志（不破坏旧数据） |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码/文档提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-056-research-and-cleanup`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 差量更新评估最终结论（推荐方案 + 是否立项 task-057）；
   - restored_at 迁移器 v2 diff；
   - 跨盘测试在 CI skip / 本地 passed 的输出；
   - G6 排查最终结论（fix 或 docs 路线）；
   - `cargo test -p engine` 通过输出（含新增 ≥3 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（如有）；
   - 打包安装冒烟证据（NSIS 迁移器 v2 执行日志）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（差量更新是否立项 task-057 / RmRestart 待产品拍板 / 代码签名未购证书）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务056-差量更新评估与遗留小修 (#task-056)`，
   该提交只含 `tasks/056-research-and-cleanup.md` 一个文件，落在 `feature/task-056-research-and-cleanup` 分支最前端。

---

## 实现备注

> 执行日期：2026-10-28　分支：`feature/task-056-research-and-cleanup`（基线 `27d9df6`，从 main `4231cbd` 拉出）

### 1. 实际改动文件清单

| 文件 | 类型 | 改动 |
|---|---|---|
| `docs/research/delta-update-evaluation.md` | 新建 | 差量更新技术评估（A/B/C 三方案 + 4 个决策点 + 结论） |
| `engine/src/db/migrations/v2__safe_box_restored_at.sql` | 新建 | `ALTER TABLE safe_box ADD COLUMN restored_at INTEGER` |
| `engine/src/db/migrate.rs` | 后端 | `MIGRATIONS` 加 `(2, include_str!(…))` + 版本历史注释 |
| `engine/src/execute/safe_box.rs` | 后端 | restore 第 5 步 `DELETE` → `UPDATE status='restored', restored_at` |
| `engine/tests/db_schema.rs` | 测试（**白名单缺口**） | `user_version` 断言 1→2；新增 `migrate_v1_to_v2_preserves_existing_rows` |
| `engine/tests/safe_box_restore.rs` | 测试（**白名单缺口**） | 改「行被删」断言为「restored + restored_at 非 NULL」；新增 2 条 |
| `engine/tests/safe_box_cross_disk.rs` | 新建测试 | 2 条（跨盘 fallback + 跨盘 restore 限制） |
| `docs/test-reports/gui-walkthrough-round3-checklist.md` | 文档 | 末尾加「六、G6 已知项豁免口径」5 小节 |
| `docs/PROJECT_STATE.md` | 文档 | 6 处（见 §10） |

**白名单里列的 `engine/src/db/schema.rs` 不存在**——本项目 schema 定义在 `engine/src/db/migrations/v1__init.sql`，没有单独的 schema.rs。未动。

### 2. 差量更新评估最终结论

**不立项 task-057（差量更新）。**

| 方案 | 结论 |
|---|---|
| A. Tauri 2 官方 updater | **不支持**。`grep -ril "bsdiff\|delta\|xdelta"` 扫 `@tauri-apps/plugin-updater@2.13.3` 全包**零命中**；`Update` 只有 `download`/`install`/`downloadAndInstall`，`DownloadOptions` 只有 `headers`/`timeout`；`latest.json` 协议只认「一个完整包 + 一个整包签名」，插差量 = 自建更新通道、丢掉官方验签 |
| B. bsdiff4 | 压缩率略高、速度慢（4 MB 无感）；Rust crate 维护一般、依赖 bzip2 C 库 |
| C. xdelta3 | 速度快（4 MB 无感）；Rust 侧多为 FFI 包装，成熟度中偏低；**无决定性优势** |

**否决的四条硬理由**：
1. **绝对收益太小**——4 MB 包在 20 Mbps 下 1.7 秒；差量乐观省 1.2 秒，而 task-055 已用进度条+速度显示解决了「等待焦虑」这个真实痛点。
2. **成本 6–9 人日且长期维护**——客户端要常驻旧包缓存（+4 MB 磁盘）+ LRU + SHA 校验 + 双验签 + 回退；服务端要改发版流水线、维护相邻版本产物矩阵、保留历史包。
3. **脱离官方更新通道**——补丁投毒/降级攻击的安全面要自己扛，而这条链路直接关系用户机器安全。
4. **差量效果本身不确定**——Rust 重编译的二进制布局漂移（链接器布局、内联决策、符号顺序）会让实际缩减率远低于理论值，可能只有 20–50%。

**替代建议（更有性价比）**：`release profile` 瘦身——`lto = "fat"` + `codegen-units = 1` + `strip = true` + `opt-level = "z"`，预计 -20%~35%、1–2 人日、不改架构不碰安全链路。建议可选立项。

> **如实标注**：Rust 二进制差量的实际缩减率本轮**未实测**（需要两个相邻版本 release 产物做对照，约 1 人日）。但即使按最乐观 70% 算，结论仍是「不值得」，所以这个数据不影响结论方向。

### 3. restored_at 迁移器 v2 diff

```diff
# engine/src/db/migrate.rs
-static MIGRATIONS: &[(u32, &str)] = &[(1, include_str!("migrations/v1__init.sql"))];
+static MIGRATIONS: &[(u32, &str)] = &[
+    (1, include_str!("migrations/v1__init.sql")),
+    (2, include_str!("migrations/v2__safe_box_restored_at.sql")),
+];
```

```sql
-- v2__safe_box_restored_at.sql（新建）
ALTER TABLE safe_box ADD COLUMN restored_at INTEGER;
```

```diff
# engine/src/execute/safe_box.rs
-    // 5. 删行
-    conn.execute("DELETE FROM safe_box WHERE id=?1", [safe_box_id])
-        .map_err(|e| RestoreError::Db(format!("delete safe_box row: {e}")))?;
+    // 5. 标记为已恢复（task-056：不再 DELETE 行）
+    let now = ...unix 秒...;
+    conn.execute(
+        "UPDATE safe_box SET status='restored', restored_at=?2 WHERE id=?1",
+        params![safe_box_id, now],
+    )
+    .map_err(|e| RestoreError::Db(format!("mark safe_box row restored: {e}")))?;
```

**两个关键判断**：

- **类型选 INTEGER 不选 TEXT**（任务书写 `TEXT NULL`）。同表 `quarantined_at` / `expires_at` 都是 INTEGER unix 秒，前端 `formatTs` 直接消费；用 TEXT 要额外解析 ISO 字符串，且与相邻列不一致。
- **「行还在」不等于「可重复恢复」**。旧实现靠 DELETE 让行消失；新实现行还在，若过滤条件漏了 `status='in_box'`，第二次 restore 就会去 rename 一个已经不存在的 box 文件。`restore_from_safe_box` 的查询本来就带 `AND status='in_box'`，所以第二次返回 `NotFound`——**已用 `restore_twice_second_returns_not_found` 固化**。

**外部行为不变的核对**（逐处查过）：

| 位置 | 过滤条件 | 结论 |
|---|---|---|
| `list_safe_box` | `WHERE status='in_box'` | 已恢复行不显示 ✓ |
| `list_paged`（动态 WHERE） | `clauses = vec!["status='in_box'"]` | 同上 ✓ |
| `list_sessions` | `WHERE status='in_box' AND source_task_id IS NOT NULL` | 同上 ✓ |
| `enforce_quota` 的 SUM / 淘汰查询 | `WHERE status='in_box'` | 已恢复行不占 5GB 配额 ✓ |
| `restore_from_safe_box` / `purge_item` | `WHERE id=?1 AND status='in_box'` | 二次操作返回 NotFound ✓ |

### 4. 跨盘测试：CI skip / 本地 passed

```
# 本机（有 C: 和 D:）——真跑
running 2 tests
test cross_disk_move_into_box_falls_back_to_copy_remove ... ok
test cross_disk_restore_currently_returns_io_error ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.19s

# 单盘环境（CI windows-latest 预期）——打印 skip 后 return，仍算 passed
[skip] 只有单盘环境，跨盘 rename fallback 无法验证（CI 属预期）
```

**为什么用「打印 skip + return」而不是 `#[ignore]`**：`#[ignore]` 需要人记得加 `--ignored` 才会跑，本地有双盘时会被静默跳过——那就等于没测。改成运行时检测 + return，本地自动真跑、CI 自动 skip，两边都不用记命令。

**顺带发现的已知限制**：`restore_from_safe_box` 的 `fs::rename` **没有** copy+remove fallback（只有 `delete_file` 有）。跨盘恢复会直接返回 `RestoreError::Io`。实际影响小（保险箱在 `%APPDATA%`，`%TEMP%` 通常同在 C 盘），但已用 `cross_disk_restore_currently_returns_io_error` 固化当前行为，避免以后误以为支持。**修不修待评估**（若用户把数据目录改到 D 盘，从 C 盘删的文件就恢复不了）。

### 5. G6 排查最终结论：**走 docs 路线（豁免），不修**

**根因**：Tauri 2 的 IPC 有 custom protocol（`http://ipc.localhost/<cmd>`，默认首选）与 postMessage（回退）两条通道。WebView2 上 custom protocol handler 注册与 WebView 创建存在**时序竞争**——文档刚加载完的首次 IPC 可能早于 handler 就绪，于是 `ERR_CONNECTION_REFUSED`；Tauri 捕获失败后**内部标记该 webview 走 postMessage**，所以同文档内第二次不再尝试、也不再报错。报错发生在 `@tauri-apps/api` 的 fetch 层，业务代码拦不住。

**为什么否决任务书的 warmup 方案**（三条）：

1. **warmup 不能消除红错，只是换个位置**。warmup 调用自己就是「首次 IPC」，同样打 `ERR_CONNECTION_REFUSED`；Console 里红错数量不变，只是命令名从 `rules_check_update` 变成 `app_info`——对「Console 零红」这个门禁目标**毫无帮助**。
2. **可能变差**。现在红错只在「用户进设置页」时出现；若在 App 挂载时 warmup，则每次启动都必然红一次，暴露面扩大。
3. **本轮无法验证**。按 2026-10-25/26 流程开发轮不做 GUI 测试，warmup 是否真让第二次顺畅、是否引入启动时序问题，拿不到证据。**提交一个自己无法验证的「修复」，比明确豁免更危险。**

**落地产物**：`gui-walkthrough-round3-checklist.md` §六，含现象逐条（2 error + 4 warning）、根因、不修理由、**5 条判定规则**（只有这 6 条且首次进设置页 → 通过；第二次仍复现 / 设置页功能不可用 / 其它页面同类错 → 不通过）、后续处理（等上游修；彻底解法是显式关 custom protocol 全走 postMessage，属架构级取舍需产品拍板）。

### 6. `cargo test -p engine` / `--workspace` 通过输出

```
cargo test --workspace  →  passed=255  failed=0  ignored=8
```

`255 = 250（基线）+ 5（新增）`，新增明细：

| 用例 | 文件 | 覆盖 |
|---|---|---|
| `migrate_v1_to_v2_preserves_existing_rows` | `db_schema.rs` | v1 库升级到 v2：不报错、`user_version=2`、旧 in_box 行保留、新列 NULL |
| `user_version_bumped_to_latest` / `migration_is_idempotent` | `db_schema.rs` | 断言从 1 改 2（原用例改名，非新增） |
| `restored_row_disappears_from_list_and_quota` | `safe_box_restore.rs` | 恢复后列表 0 条、总大小 0、但行还在 |
| `restore_twice_second_returns_not_found` | `safe_box_restore.rs` | 行还在 ≠ 可重复恢复 |
| `cross_disk_move_into_box_falls_back_to_copy_remove` | `safe_box_cross_disk.rs` | 本机 C:→D: 真跑 copy+remove，内容逐字节一致 |
| `cross_disk_restore_currently_returns_io_error` | `safe_box_cross_disk.rs` | 固化「跨盘 restore 无 fallback」限制 |

`ignored=8` 与基线一致（未新增 `#[ignore]`）。

**`pnpm build`**：`184.22 kB │ gzip 59.18 kB`（与 task-055 一致，本任务未动前端代码）。`pnpm lint` 0 error / `pnpm test` 81 passed。

### 7. 真机冒烟 / 打包安装冒烟 / CI

| 项 | 状态 | 说明 |
|---|---|---|
| 真机点「恢复」→ 查 DB `restored_at` | **移交测试 Agent** | 按新流程开发轮不做 GUI |
| 保险箱列表不再显示已恢复行 | **移交测试 Agent** | 同上 |
| **NSIS 装 v0.1.6 → 覆盖安装含 v2 → 老库平滑升级** | **移交测试 Agent（本任务最关键的一条）** | 本任务有 DB 迁移，安装态升级路径必须实测 |
| CI（PR + 五道闸） | **移交测试 Agent** | 开发分支不跑 CI；**未建 PR** |

开发轮替代验证：`migrate_v1_to_v2_preserves_existing_rows` 已在单元测试层面模拟「v1 老库 + 一条 in_box 行 → 跑 v2 迁移」，覆盖了安装态升级的核心风险（数据丢失 / 迁移报错 / 新列缺失）。

### 8. 门禁输出

```
cargo check --workspace --all-targets                     → 0
cargo clippy --workspace --all-targets -- -D warnings     → 0
cargo fmt --all -- --check                                → 0
cargo test --workspace   → passed=255  failed=0  ignored=8
pnpm --filter app lint   → 0 error（2 条既有 react-refresh warning，SafeBox.tsx）
pnpm --filter app test   → 9 files / 81 passed
pnpm --filter app build  → 184.22 kB │ gzip 59.18 kB
```

### 9. 与任务书不符的偏差（3 项）

1. **`restored_at` 用 `INTEGER` 而非 `TEXT`**。理由见 §3：与同表 `quarantined_at`/`expires_at` 一致，前端 `formatTs` 直接消费。
2. **白名单漏两个测试文件**：`engine/tests/db_schema.rs`（加 v2 后 `user_version_bumped_to_1` 断言 1 必然红）、`engine/tests/safe_box_restore.rs`（restore 改 UPDATE 后 `n_box == 0` 断言必然红）。不改这两个文件，门禁**不可能绿**。已按新语义改写并补用例。
3. **白名单列的 `engine/src/db/schema.rs` 不存在**。本项目 schema 全在 `engine/src/db/migrations/v1__init.sql`，没有单独的 schema.rs。未新建该文件（新建会是空壳，无意义）。

### 10. `docs/PROJECT_STATE.md` 改动（6 处）

1. 「待办事项」追加 `- [x] 056: …`；
2. 「最近一次变更」顶部插入 `2026-10-28：task-056 开发完成` 整段（4 个子项 + 门禁 + 3 项偏差）；
3. 「已知问题」task-017 条目：`restored_at` 遗留标为**已由 task-056 解决**（附 4 条用例名）；
4. 「已知问题」task-016 跨盘条目：标为**已由 task-056 补测**，并新增「跨盘 restore 无 fallback」这一已知限制；
5. 「已知问题」G6 条目：追加 **task-056 定稿豁免**（根因 / warmup 否决理由 / 5 条判定规则 / 彻底解法需产品拍板）；
6. 「下一步计划」第 2 条改为「差量更新评估结论：不立项 task-057；若要减负改做 release profile 瘦身」；「当前负责人」改测试 Agent；「Git 提交记录」加 8 行；「变更轨迹」加 1 行。

### 11. 遗留问题

- **差量更新不立项**（评估结论）；若产品仍想减小更新负担，建议立项「release profile 瘦身」（1–2 人日，预计 -20%~35%）。
- **跨盘 restore 无 fallback**（本轮新固化的已知限制）：`restore_from_safe_box` 的 `fs::rename` 跨盘直接报 IO 错误。若用户把数据目录改到非 C 盘，从 C 盘删的文件恢复不了。修法很直接（照搬 `delete_file` 的 copy+remove fallback），但会引入「恢复中途失败」的新失败模式（copy 成功 remove 失败 → 两边都有文件），需要单独设计，**建议另立任务**。
- **G6 不修**（豁免），等 Tauri 上游。若要「真零红」需关 custom protocol 全走 postMessage，属架构取舍。
- **`RmRestart`** 待产品拍板；**代码签名**未购证书；**品牌图标**决策不做。
- **`safe_box` 表会随 restore 次数增长**（每次留一行）。当前量级（用户手动恢复，低频）无影响；若未来要清理，可加「保留 N 天」的定期清理，不在本任务范围。

### 三句话结论

1. 差量更新评估结论是**不做**——不是做不出来，是 4 MB 的包省下的绝对量 <2 秒、却要 6–9 人日 + 自建更新通道 + 自己扛补丁安全；替代方案「release profile 瘦身」性价比高得多。
2. `restored_at` 走迁移 v2 + restore 改 UPDATE，外部行为不变（所有查询本就带 `status='in_box'`），但多守住一条不变量：**行还在 ≠ 可重复恢复**。
3. G6 明确豁免不修——warmup 只是把红错换个命令名，放启动还会扩大暴露面；豁免口径（含 5 条判定规则）已写进第三轮清单，不再挂着。
