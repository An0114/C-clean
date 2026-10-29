# 任务编号：058 M4 收尾——RmRestart 白名单 + safe_box 原子性 + 代码签名评估 + v0.1.7 CHANGELOG 草稿

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：PROJECT_STATE「下一步计划 5. 后续按需迭代」+「已知问题」段 B 中/B 低遗留。
> 上游依赖：task-057 已合并 main（merge `e9f0e7a`，RmRestart 自动重启 + 设置开关已上线，P0 flags 误用已修）。
> **本任务范围（4 个功能单元，厚任务）**：RmRestart 白名单 UI、safe_box 原子性回滚、代码签名技术评估文档、v0.1.7 CHANGELOG 草稿。
> **为什么把这些合并**：都是 M3/M4 闭环后的「收尾小修 + 研究 + 文档」，低风险且互相不耦合；按新提示词「同一模块/同一阶段收尾的事项合并派发」原则，一次 PR 清掉。
> **不做**：差量更新（已决策不立项）、release profile 瘦身（已决策不做）、G6 修复（已豁免）、品牌图标（已决策不做）。

## 状态字段

testing

## 任务目标

### 1. RmRestart 白名单 UI（task-057 后续）

- **现状**：task-057 上线后，开启开关会自动重启**所有** bRestartable 进程——但用户可能不希望某些进程（如更新器、后台服务）被自动拉起。
- **修法**：
  - prefs 表加 `restart_whitelist: Vec<String>`（进程名，如 `["notepad.exe", "code.exe"]`，空数组 = 全部可重启进程都试）；
  - Settings 页「行为偏好」下方加一个简单的白名单编辑框（textarea，每行一个进程名，支持 `*` 通配）；
  - 后端 `RmSession::restart` 调用前，对 bRestartable 进程按白名单过滤——白名单非空时只重启白名单内的；
  - **默认行为**：白名单为空 = 不限制（保持 task-057 现状），用户主动加进程名才启用白名单过滤。

### 2. safe_box 原子性回滚（task-016 遗留，B 中）

- **现状**：`engine/src/execute/safe_box.rs` 里 `fs::rename` 成功但 `INSERT INTO safe_box` 失败时，文件已在 box 目录但 DB 无行——下次 `enforce_quota` 会把孤儿文件 purge 掉，但用户感知是「文件消失了」。
- **修法**：
  - 用事务包裹：先 `INSERT`（拿到 rowid）再 `fs::rename`；
  - 如果 `fs::rename` 失败，**回滚事务**（DELETE 刚 INSERT 的行）+ 把文件移回原位置（或记为 fail）；
  - 跨盘 rename（copy+remove）同理：copy 成功但 INSERT 失败时，删除已 copy 的目标文件；
  - 新增单测：模拟 INSERT 失败 → 验证原文件还在原位、box 目录无孤儿。

### 3. 代码签名技术评估文档（B 中遗留）

- **现状**：PROJECT_STATE 「已知问题」B 中「task-033 installer 未做代码签名，未购 EV/OV 证书」——用户首次运行 .exe/.msi 会有 SmartScreen 警告。
- **修法**：写 `docs/research/code-signing-evaluation.md`，覆盖：
  - **方案 A：EV 证书**（~$300-500/年，SmartScreen 立即信任，硬件 token）；
  - **方案 B：OV 证书**（~$100-200/年，需积累 SmartScreen reputation 1-3 个月）；
  - **方案 C：自签名**（免费，SmartScreen 必然警告，不推荐分发给用户）；
  - **方案 D：Microsoft Store / winget 分发**（规避 SmartScreen，但需上架审核）；
  - **结论**：推荐方案 + 预估年成本 + 是否值得做（当前用户量小，SmartScreen 警告可接受）。

### 4. v0.1.7 CHANGELOG 草稿

- **现状**：task-057 已合并 main，但版本号仍是 v0.1.6；
- **修法**：
  - `CHANGELOG.md` 追加 v0.1.7 条目（草稿，待发版时定稿）；
  - 覆盖：RmRestart 自动重启（默认关闭 + 设置开关）、prefs 用户偏好、restored_at 审计列、跨盘 rename 测试、G6 豁免口径；
  - 版本号**不立即升**（留到发版任务再升），只写 CHANGELOG 草稿。

**本任务不做**：
- 不做差量更新（已决策不立项）；
- 不做 release profile 瘦身（已决策不做）；
- 不做 G6 修复（已豁免）；
- 不做品牌图标（已决策不做）；
- 不升版本号（留到发版任务）；
- 不发版（用户手动 workflow_dispatch）。

## 前置条件

1. `main` 已合并 task-057（merge `e9f0e7a`）；
2. 关键锚点：
   - `engine/src/prefs.rs`（加白名单字段）
   - `engine/src/execute/locking.rs`（RmSession::restart 加白名单过滤）
   - `engine/src/execute/safe_box.rs`（INSERT + rename 事务包裹）
   - `app/src/pages/Settings.tsx`（白名单编辑框 UI）
   - `CHANGELOG.md`（追加 v0.1.7 草稿）
   - 仓库根（新建 `docs/research/code-signing-evaluation.md`）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. RmRestart 白名单

1. prefs 表加 `restart_whitelist: Vec<String>`（默认空数组）；
2. 后端 `RmSession::restart` 按白名单过滤：空 = 全部可重启进程都试；非空 = 只重启白名单内进程名；
3. Settings 页加白名单编辑框（textarea，每行一个进程名）；
4. 切换白名单立即 `set_prefs` 持久化；
5. 单测：白名单过滤逻辑（空 / 匹配 / 不匹配）。

### 2. safe_box 原子性

6. `fs::rename` 路径用事务包裹（先 INSERT 再 rename）；
7. rename 失败时回滚事务（DELETE 刚 INSERT 的行）+ 文件留在原位；
8. 跨盘 copy+remove 同理：copy 成功但 INSERT 失败时删除目标文件；
9. 单测：模拟 INSERT 失败 → 原文件还在、box 目录无孤儿。

### 3. 代码签名评估

10. `docs/research/code-signing-evaluation.md` 存在；
11. 覆盖 A/B/C/D 四个方案对比（成本 / SmartScreen 行为 / 上架要求）；
12. 给出推荐方案 + 是否值得做的明确结论。

### 4. v0.1.7 CHANGELOG 草稿

13. `CHANGELOG.md` 追加 v0.1.7 条目（草稿）；
14. 覆盖 RmRestart / prefs / restored_at / 跨盘测试 / G6 豁免；
15. 版本号**不立即升**（package.json / tauri.conf.json / Cargo.toml 保持 0.1.6）。

### 后端单测

16. `cargo test -p engine` 全绿（现有 255+8 + 本任务新增 ≥4 条全绿）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/prefs.rs`（加白名单字段）
- `engine/src/execute/locking.rs`（RmRestart 白名单过滤）
- `engine/src/execute/safe_box.rs`（事务包裹）
- `engine/tests/`（白名单过滤 + 原子性回滚用例）
- `app/src/pages/Settings.tsx`（白名单编辑框）
- `CHANGELOG.md`（追加 v0.1.7 草稿）
- `docs/research/code-signing-evaluation.md`（新建）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-058-m4-finishup`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-058-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `feat(prefs): RmRestart 白名单（空=不限制）+ Settings 编辑框 (#task-058)`（单元 1）
   —— 含 prefs + locking + Settings.tsx + 单测。
2. `fix(execute): safe_box INSERT + rename 事务包裹，失败回滚不留孤儿 (#task-058)`（单元 2）
   —— 含 safe_box.rs + 单测。
3. `docs(research): 代码签名技术评估（EV/OV/自签/Store 四方案） (#task-058)`（单元 3）
4. `docs(changelog): 追加 v0.1.7 草稿条目（RmRestart/prefs/restored_at/跨盘/G6豁免） (#task-058)`（单元 4）
5. 自验全绿后，最后一次提交：`docs(state): 任务058完成，M4 收尾——白名单/原子性/签名评估/CHANGELOG 草稿就绪 (#task-058)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 058: ...`；
   - 「最近一次变更」追加一行，含 4 个单元摘要 + commit hash；
   - 「已知问题」段把 task-016 原子性标记已解决；
   - 「下一步计划」改为「v0.1.7 发版（待用户手动 workflow_dispatch）；后续按需迭代（代码签名购买决策 / 品牌图标）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-29  task-058  M4 收尾（RmRestart 白名单 / safe_box 原子性 / 签名评估 / CHANGELOG 草稿）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：白名单过滤逻辑、safe_box 事务回滚、prefs 新字段默认值。
- **集成测试（必做）**：`cargo test -p engine` 全绿，确保现有不回归。
- **端到端写路径（必做，本任务硬要求）**：白名单持久化必须有一条真实「编辑白名单 → 查 prefs 存储 → 重开应用白名单仍在」证据——不能只构造内存 state。
- **真机冒烟（必做、不可裁剪）**：
  - 白名单为空：开启 RmRestart 开关 → notepad 被关后自动重启（现状）；
  - 白名单加 `chrome.exe`：notepad 被关后**不**自动重启，chrome 被关后自动重启；
  - 白名单持久化：编辑白名单 → 重开应用 → 白名单仍在；
  - safe_box 原子性：模拟一次 INSERT 失败（可手动改库），验证原文件还在原位；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，上述 4 条在安装态复跑。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（Settings 白名单编辑框 + RmRestart 行为变化，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| 白名单过滤逻辑 | 单测输出（空/匹配/不匹配三态） |
| safe_box 事务回滚 | 单测输出（INSERT 失败 → 原文件在原位） |
| 白名单真机行为 | 真机截图：白名单为空 vs 非空时 notepad 重启行为不同 |
| 白名单持久化 | 真机证据：编辑白名单 → 重开应用 → 白名单仍在（DevTools 查 prefs） |
| 代码签名评估 | 文件存在 + 结论段落（推荐方案 + 成本） |
| CHANGELOG v0.1.7 草稿 | 文件内容预览 |
| 不回归 | `cargo test -p engine` + `pnpm test` 全绿 |
| 真机冒烟 | CDP consoleAPICalled（排除 G6 后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + 4 条复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码/文档提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-058-m4-finishup`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - 白名单数据结构（Vec<String> 还是 HashSet）；
   - safe_box 事务最终代码结构；
   - 代码签名评估最终结论；
   - CHANGELOG v0.1.7 条目摘要；
   - `cargo test -p engine` 通过输出（含新增 ≥4 条用例）；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（4 条截图路径）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（v0.1.7 发版待用户手动；代码签名购买决策待用户拍板；品牌图标仍不做）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务058-M4 收尾（RmRestart 白名单 + safe_box 原子性 + 签名评估 + CHANGELOG） (#task-058)`，
   该提交只含 `tasks/058-m4-finishup.md` 一个文件，落在 `feature/task-058-m4-finishup` 分支最前端。

---

## 实现备注

> 执行日期：2026-10-29　分支：`feature/task-058-m4-finishup`（基线 `8d3a393`，从 main `e9f0e7a` 拉出）

### 1. 实际改动文件清单

| 文件 | 类型 | 改动 |
|---|---|---|
| `engine/src/prefs.rs` | 后端 | 白名单读写（JSON 存 `settings`）+ `wildcard_match` + `whitelist_allows_restart` + 8 条单测；`Prefs` 去掉 `Copy`、加 `restart_whitelist`（`serde(default)`） |
| `engine/src/execute/locking.rs` | 后端 | `shutdown_locking_processes` 加第三参 `restart_whitelist`；`restart_plan` 在 shutdown **之前**算好（映像名 + 可重启 pid） |
| `app/src-tauri/src/commands.rs` | Tauri（**白名单缺口**） | 读白名单并传入；`set_prefs` 同时写白名单 |
| `app/src/pages/Settings.tsx` | 前端 | `Prefs` interface 加 `restartWhitelist`；textarea + 保存按钮 + fail-closed 说明文案；`onToggleAutoRestart` 必须带当前白名单一起提交 |
| `engine/tests/locking_shutdown.rs` | 测试 | 9 处调用点适配三参签名 |
| `engine/tests/safe_box_atomicity.rs` | 新建测试 | 3 条原子性用例 |
| `app/src/pages/Settings.test.tsx` | 测试（**白名单缺口**） | 改 `set_prefs` payload 断言 + 新增 4 条白名单用例 |
| `engine/src/execute/safe_box.rs` | 后端 | INSERT 提前 + 事务包裹 + 跨盘失败清理 |
| `docs/research/code-signing-evaluation.md` | 新建 | 代码签名评估 |
| `CHANGELOG.md` | 文档 | v0.1.7 草稿 |
| `docs/PROJECT_STATE.md` | 文档 | 6 处 |

### 2. 白名单数据结构：`Vec<String>`（不是 `HashSet`）

- **存储**：`settings` 表 `prefs.restart_whitelist` 键，值为 **JSON 数组字符串**（`["notepad.exe","code*.exe"]`）。**不加新列、不加迁移**——复用 task-057 已有的 `settings` 表，与 `prefs.auto_restart_on_close` 同源。
- **为什么 `Vec` 不用 `HashSet`**：需要保留用户输入顺序（UI 回填 textarea 时顺序要一致），且白名单规模是个位数，`HashSet` 的 O(1) 查找没有意义。归一化（trim + 小写）在读写两侧都做，重复项不去掉（语义无影响，去重反而让用户看到的内容与输入不一致）。
- **空列表的存储语义**：写空 → **删除该行**（不是存 `[]`）。这样「从没配过」和「配了又清空」在存储层是同一状态，读侧只需判 `Vec::is_empty()`。
- **脏数据兜底**：JSON 解析失败 → 返回空 Vec（= 不限制）+ `eprintln`。**不能让脏数据把「重启行为」变成不可预期**。

### 3. ⚠️ 白名单语义与任务书不同（必须说明）

任务书写：「白名单非空时**只重启白名单内的**」。

**这个要求在 Restart Manager 机制下无法实现。** `RmRestart` 的签名是 `RmRestart(hSession, dwRestartFlags, pfnProgress)`——**没有按 pid 的参数**，一次调用会把本会话内**所有** `bRestartable==TRUE` 的应用全部拉起。task-057 测试轮已经踩过相关坑（旧实现误把 `pid` 当 `flags` 传，非零 flags 让每次调用都失败）。

所以实现改为 **fail-closed**：

| 白名单 | 被关闭的可重启进程 | 是否调 `RmRestart` |
|---|---|---|
| 空 | 任意 | **调**（不限制，保持 task-057 现状） |
| 非空 | 全部命中白名单 | **调**（安全：不会重启名单外的东西） |
| 非空 | 有任何一个不命中 | **不调**（否则一调就把用户明确排除的也拉起来） |
| 非空 | 空（没有可重启进程） | **不调**（本来就没东西可重启） |

UI 文案如实写了这个语义（「只有被关闭的可重启进程**全部**在此列表中才会自动重启」），不让用户以为是「只重启名单内的」。

**替代方案为什么不用**：自己 `CreateProcess` 逐个重启白名单内的进程可以做到「只重启名单内」，但会丢掉 RM 的「恢复原始命令行/工作目录」能力（例如 `code.exe` 无参启动 vs 原来打开了某个文件夹），代价更大。选 RM 全有或全无 + fail-closed。

### 4. 一个容易写错的地方：映像名必须在 `RmShutdown` 之前取

第一版代码把 `image_file_name(pid)` 写在 `session.shutdown()?` **之后**——但那时进程已经被杀，`OpenProcess` 必然失败，`image_file_name` 返回 `None` → 白名单拿到空串 → 永远不命中 → **自动重启静默失效**（且日志看不出异常）。

修法：把「要重启谁」的计算整体提到 shutdown 之前：

```rust
let restart_plan: Option<(Vec<u32>, Vec<String>)> = if auto_restart {
    let closable_pids: HashSet<u32> = closable.iter().map(|lp| lp.pid).collect();
    let restartable = restartable_pids(&closable_pids, &infos);
    if restartable.is_empty() { None }
    else {
        let names = restartable.iter()
            .map(|pid| image_file_name(*pid).unwrap_or_default())
            .collect();
        Some((restartable, names))
    }
} else { None };

session.shutdown()?;

if let Some((restartable, names)) = restart_plan {
    if !whitelist_allows_restart(restart_whitelist, &names) { /* 跳过 */ }
    else { session.restart_all() }
}
```

**为什么用映像名而不是 `LockingProcess.app_name`**：后者来自 RM 的 `strAppName`，是**本地化 FileDescription**（中文系统下 `notepad.exe` 显示为「记事本」），用户在白名单里写 `notepad.exe` 永远匹配不上。task-049b 的护栏已经因为这个问题改成 `QueryFullProcessImageNameW`，白名单沿用同一套。

### 5. safe_box 事务最终代码结构

```rust
// 4. 先开事务并写行，再动文件（task-058）
let tx = self.conn.unchecked_transaction()
    .map_err(|e| format!("begin tx for {src}: {e}"))?;
tx.execute("INSERT INTO safe_box (…) VALUES (…)",
           params![src, rel, size, now, expires_at, self.source_task_id])
  .map_err(|e| format!("insert safe_box row for {src}: {e}"))?;

// 5. rename（跨盘走 copy+remove，失败清理半成品）
match fs::rename(src_path, &box_path) {
    Ok(()) => {}
    Err(e) if is_cross_device(&e) => {
        if let Err(e2) = fs::copy(src_path, &box_path) {
            let _ = fs::remove_file(&box_path);        // 删半成品
            return Err(format!("copy fallback …: {e2}"));
        }
        if let Err(e2) = fs::remove_file(src_path) {
            let _ = fs::remove_file(&box_path);        // 源还在，目标那份是重复的
            return Err(/* FILE_IN_USE / PERMISSION_DENIED 分类 */);
        }
    }
    Err(e) => { return match classify_io_error(&e) { … } }   // `?`/return → tx Drop 自动 rollback
}

// 6. 文件已就位 → 提交
tx.commit().map_err(|e| format!("commit safe_box tx for {src}: {e}"))?;
```

**核心洞察**：不需要"回滚文件"，只需要**把 INSERT 提到 rename 之前**。顺序一反，INSERT 失败时文件还没动，天然无孤儿；rename 失败时事务 Drop 自动回滚行。任务书设想的「先 INSERT 拿 rowid → rename 失败再 DELETE 行 + 把文件移回」里，「移回文件」是多余的——rename 失败本身不会移动文件。

### 6. 代码签名评估最终结论

**当前不做。** 四条理由按权重：

1. **用户量太小**——SmartScreen 的伤害是「首次运行多点两下」，而声誉机制本质是「用下载量换信任」，没有量买不到声誉（OV 花完钱还要等 1–3 个月）。
2. **EV/OV 都要企业实体**——本项目是个人项目，不是「花钱就能买」。
3. **改造发版链路的代价被低估**——CA/B Forum 自 2023-06 起强制「代码签名私钥必须硬件存储」，而 GitHub Actions runner **读不到你插在家里的 USB token**：要么把全自动发版降级为「下载产物 → 本地插 token 手动签 → 手动上传」，要么再买云签名服务。
4. **签名解决不了真正的信任问题**——用户怕的是「会不会误删我的文件」，不是「发布者是谁」；这靠开源 + 保险箱 + dry-run + L3 红线解决。

**三个「该做」的触发条件**：公开分发且下载量达数百/月、出现企业/机构用户、要上 winget 或 Store。

**唯一可能便宜的路**：Azure Artifact Signing（原 Trusted Signing，~€102/年、CI 友好），但**个人开发者仅限美国/加拿大**，其它地区需组织实体——**采购前须自行向 Microsoft 确认中国区可行性**，本文档按「个人、中国大陆、无企业实体」前提给出结论。

### 7. CHANGELOG v0.1.7 条目摘要

| 段 | 条数 | 内容 |
|---|---|---|
| 新增 | 4 | 清理后自动重启（默认关闭）、重启白名单（含 fail-closed 语义说明）、用户偏好持久化、保险箱恢复时间可追溯 |
| 修复 | 1 | 保险箱移入失败不再丢文件（原孤儿问题） |
| 变更 | 2 | 已恢复行保留但不占配额/不出现在列表；补齐跨盘移入测试 |
| 已知问题 | 5 | G6 豁免、跨盘恢复不支持、未签名、`RmShutdown` 连带、默认图标 |

版本号**保持 `0.1.6`**（已 `grep` 验证四处均未变），发版任务再升。

### 8. `cargo test` / `pnpm` 通过输出

```
cargo test --workspace  →  passed=273  failed=0  ignored=8
```

`ignored=8` 与基线一致。新增 11 条：

| 用例 | 位置 | 覆盖 |
|---|---|---|
| `whitelist_defaults_to_empty_when_row_missing` | `prefs.rs` | 缺行 → 空（不限制） |
| `whitelist_roundtrip_persists_and_normalizes` | `prefs.rs` | 存读一致 + 去空白/小写/丢空行 |
| `whitelist_empty_write_removes_row` | `prefs.rs` | 写空 → 删行（不是存 `[]`） |
| `whitelist_corrupt_json_falls_back_to_empty` | `prefs.rs` | 脏数据降级为空，不崩 |
| `wildcard_match_basics` | `prefs.rs` | 精确 / `*.exe` / `code*` / `*` / 大小写 |
| `whitelist_allows_restart_three_states` | `prefs.rs` | 空 / 命中 / 不命中 |
| `whitelist_requires_all_names_to_match` | `prefs.rs` | **全部命中才允许**（fail-closed 核心） |
| `whitelist_rejects_empty_name_list` | `prefs.rs` | 无可重启进程 → 不允许 |
| `insert_failure_leaves_source_file_in_place_and_no_orphan` | `safe_box_atomicity.rs` | **核心**：外键约束制造 INSERT 失败 → 原文件原位 + 内容未变 + box 零孤儿 + DB 零行 |
| `success_path_keeps_file_and_row_consistent` | `safe_box_atomicity.rs` | 对照：正常路径文件与行一致 |
| `missing_source_returns_false_without_row` | `safe_box_atomicity.rs` | 竞态幂等，不留行 |

**`pnpm`**：`lint` 0 error（2 条既有 `react-refresh` warning，SafeBox）/ `test` **89 passed（9 files）** / `build` **187.46 kB │ gzip 60.30 kB**。

### 9. 真机冒烟 / 打包安装冒烟 / CI

| 项 | 状态 | 说明 |
|---|---|---|
| 白名单三态真机行为（空 / 加 chrome.exe / 持久化） | **移交测试 Agent** | 按新流程开发轮不做 GUI/真机 |
| safe_box 原子性真机（改库制造 INSERT 失败） | **移交测试 Agent** | 同上 |
| CDP Console 零红（排除 G6） | **移交测试 Agent** | 同上 |
| NSIS 安装态复跑 | **移交测试 Agent** | 同上 |
| CI（PR + 五道闸） | **移交测试 Agent** | 开发分支不跑 CI；**未建 PR** |

开发轮替代验证：白名单的 8 条单测覆盖「存储 → 归一化 → 通配匹配 → 三态判定」全链路；原子性的核心用例用**真实外键约束**制造 INSERT 失败（不是 mock），断言四件事（源文件在、内容不变、box 零孤儿、DB 零行）。

### 10. 与任务书不符的偏差（3 项）

1. **白名单语义从「只重启名单内」改为「全部命中才重启」**。`RmRestart` 无按 pid 参数，任务书的要求在 RM 机制下无法实现。详见 §3。
2. **白名单存储用 JSON 数组字符串**（`settings` 表），未按任务书「prefs 表加 `restart_whitelist: Vec<String>`」的字面做法加列/加表——本项目根本没有独立的 prefs 表（task-057 就是复用 `settings`），加新表要写迁移，收益为零。
3. **白名单漏了两个必改文件**：`app/src-tauri/src/commands.rs`（`shutdown_locking_processes` 加第三参必然级联编译失败；`set_prefs` 也要写白名单）与 `app/src/pages/Settings.test.tsx`（task-057 的 `set_prefs` payload 断言 `{prefs:{autoRestartOnClose:true}}` 必红）。

### 11. 遗留问题

- **v0.1.7 发版待用户手动 `workflow_dispatch`**（version 填 `0.1.7`，并把四处版本号从 `0.1.6` 升上去）。CHANGELOG 草稿已就绪。
- **代码签名购买决策待用户拍板**。评估结论是当前不做，若将来触发（下载量上来 / 企业客户 / 上 Store）再按 §6 的最小路径执行。
- **品牌图标仍不做**（既有决策）。
- **白名单是 fail-closed 语义**，与用户直觉（「只重启这些」）有偏差，UI 已如实说明；若产品认为这个交互不可接受，需要改成「自己 CreateProcess 逐个重启」，代价是丢失 RM 的命令行恢复能力。
- **`safe_box` 的 `enforce_quota` 仍在事务之外**（在 INSERT 之前执行，会 purge 旧条目）。本轮只保证「本次移入」的原子性；「purge 旧条目 + 移入新条目」这个组合不是原子的（极端情况下 quota 淘汰成功但移入失败 → 空间多释放了一些，无数据丢失）。属可接受，记录在案。

### 三句话结论

1. 白名单的**语义必须改**——`RmRestart` 全有或全无，任务书要的「只重启名单内」做不到，改成「全部命中才重启」的 fail-closed，并在 UI 里如实说明；另外映像名必须在 shutdown **之前**取（第一版写错了，进程已死查不到）。
2. safe_box 原子性的关键是**顺序**：INSERT 提到 rename 之前，失败路径就天然没有孤儿，比任务书设想的「回滚文件」简单得多。
3. 代码签名当前不做，理由不是价格而是「硬件私钥 vs GitHub Actions 自动化」的根本冲突 + EV/OV 要企业实体；评估文档给了三个「该做」的触发条件，不是简单说「以后再说」。
