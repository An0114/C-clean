# 任务编号：053 进度条尾部冲刷 + 保险箱历史来源渲染——G7 收尾 100% + G8 NULL 来源文案

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact
> 来源：`docs/test-reports/gui-walkthrough-2026-10-25.md` §6.3（G7）、§6.2（G8）；PROJECT_STATE「下一步计划 ⑤」。
> 上游依赖：task-052（G1 进度条脚本级换算已合并 `d92db1b`，但真机复验发现收尾不满）。
> **本任务范围（5 个功能单元）**：G7（P1，唯一发版阻塞）+ G8（P2，随 bugfix 一起带）。
> **流程教训（走查报告 §6.4）**：① 测试替身不能替代生产链路——`VecEmitter` 断言 100 与 `ThrottledEmitter` 生产行为是两件事，凡经过节流/队列/去抖的通道必须有走真实包装层的用例；② 「数据为空」不等于「功能失效」，判定类缺陷前先确认观察对象是不是本次变更产生的数据。

## 状态字段

testing

## 任务目标

### G7【P1，唯一发版阻塞】扫描结束时进度条不满

- **现象**：扫描结束（按钮已变「再次扫描」）时进度条仍停在一段距离内，未到 100%。
- **根因（三重叠加）**：
  1. `ThrottledEmitter` 节流窗口内最后一条 `percent=100` 被丢弃；
  2. `ScanOrchestrator` 收尾事件没走强制发送；
  3. 前端 `Dashboard.tsx` 在 `scan_start` resolve 后立刻退订，最后一条事件来不及到。
- **测试为什么没抓住**：task-052 的 `scan_progress_percent` 集成测试用 `VecEmitter`（不经过 `ThrottledEmitter`），收尾 100 必然被记录 → 断言通过。**测试替身绕过了生产链路上的节流器**。
- **修法（三处都要动，缺一不可）**：
  - **后端 A**：`ThrottledEmitter` 加 trailing 补偿（被丢的最后事件暂存为 pending，窗口到点补发）或提供 `emit_force()` 供编排层收尾使用；
  - **后端 B**：`ScanOrchestrator` 所有脚本跑完后，调用 `emit_force()` 强制发送 `percent=100`；
  - **前端**：`scan_start` resolve 后主动 `setProgress(p => p && {...p, percent: 100})` 作为 UI 兜底；`unlisten` 延后 ~200ms 或改为「收到 100 或超时后再退订」；
  - **测试**：新增一条**经过 `ThrottledEmitter`** 的收尾事件用例（缩短 `MIN_INTERVAL` 或注入时钟），断言「最后到达的 percent === 100」。

### G8【P2】保险箱历史行 NULL 来源渲染为「—」造成误判

- **现象**：task-052 修复后，老数据（task-052 上线前入箱的 78 条）依然显示来源列 `—`、Session 下拉筛不到，用户以为「修了还是全 `—`」。
- **根因**：`source_task_id` 一旦写入 NULL 就补不回来（信息在当时就丢了）。
- **修法**：
  - 前端 `SafeBox.tsx` 来源列：`sourceTaskId == null` 时渲染「历史（无来源）」（淡灰色，区别于真 `—`）；
  - 筛选框旁加一句说明文案：「早于 v0.1.6 的条目无来源信息」；
  - **不改 DB**（历史数据无法追溯，强行回填反而引入风险）。

**本任务不做**：
- 不做 G6（设置页 IPC 红错，Tauri 2.x 已知项，发版不阻塞）；
- 不做 RmShutdown/RmRestart（立项决策不做）；
- 不做 banner 下载进度反馈 / 差量更新（M3+）。

## 前置条件

1. `main` 已合并 task-052（merge `d92db1b`，G1 脚本级换算、G2 分页、G3 source_task_id 接线、G4 轮次口径、G5 already_gone 已就位）；
2. 关键锚点：
   - `engine/src/scan/runner.rs`（ThrottledEmitter 封装位置，需看实现）
   - `engine/src/scan/orchestrator.rs`（收尾点，脚本循环结束后）
   - `app/src/pages/Dashboard.tsx:116-119`（进度条渲染 + scan_start resolve / unlisten）
   - `app/src/pages/SafeBox.tsx`（来源列 `sourceTaskId` 渲染 + Session 筛选框）
   - `engine/tests/scan_progress_percent.rs`（现有 VecEmitter 测试，**被绕过**——需新增 ThrottledEmitter 版本）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### G7.1 后端 ThrottledEmitter 尾部冲刷

1. 在 `ThrottledEmitter`（具体实现文件由 DevAgent 定位）加 trailing 补偿：被节流丢弃的最后事件暂存为 `pending`，下一个窗口起点补发；或暴露 `emit_force(event)` 方法；
2. 选择「trailing 补偿」或「emit_force()」任一方案均可，但必须保证**生产链路最后一条 percent=100 不会被吞**；
3. 现有 `VecEmitter` 测试保持通过（不破坏已有测试替身用法）。

### G7.2 ScanOrchestrator 收尾强制发送

4. 所有脚本跑完后，Orchestrator 显式发送一次 `percent=100` 事件（走 `emit_force()` 或 trailing 路径）；
5. 该事件**不能**被节流吞掉（单元测试断言）。

### G7.3 前端 Dashboard UI 兜底 + 延迟退订

6. `scan_start` Promise resolve 后：
   - 主动 `setProgress(p => p && {...p, percent: 100})`；
   - `unlisten` 延后 ~200ms 执行（或改为「收到 percent=100 事件后立刻退订 + 200ms 超时兜底退订」）；
7. 不破坏现有 `scannedItems` / `foundSize` 渲染。

### G7.4 经过 ThrottledEmitter 的尾部用例（关键回归）

8. 新增测试文件或用例：
   - 构造真实 `ThrottledEmitter`（注入缩短的 `MIN_INTERVAL` 或 fake clock）；
   - 模拟「Orchestrator 连续 emit 多个 percent 事件，最后一条是 100」；
   - 断言：**最后到达 subscriber 的事件 percent === 100**（不是 96/88）；
   - 同时断言事件总数 ≤ 节流前事件数（节流仍生效，不是全放行）。

### G8.1 保险箱 NULL 来源文案区分

9. `SafeBox.tsx` 来源列：
   - `sourceTaskId != null` → 显示 `#N`（现状）；
   - `sourceTaskId == null` → 显示「历史（无来源）」（淡灰色，如 `text-gray-400`）；
10. Session 筛选框旁加说明文案：「早于 v0.1.6 的条目无来源信息」（小字、灰色）。

### 后端单测（新增）

11. `cargo test -p engine` 全绿：现有 245+8 条不回归 + 本任务新增 ≥2 条（G7.4 关键回归 + 任意后端改动配套）。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `engine/src/scan/runner.rs`（ThrottledEmitter trailing / emit_force）
- `engine/src/scan/orchestrator.rs`（收尾强制发送）
- `engine/tests/` 下新增或修改 scan_progress_percent 测试（G7.4）
- `app/src/pages/Dashboard.tsx`（G7.3 UI 兜底 + 延迟退订）
- `app/src/pages/SafeBox.tsx`（G8.1 NULL 来源文案）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`bugfix/task-053-progress-tail-flush`（**从最新 `main` 拉出**，不是从 task-052 分支拉——task-052 已合并 main）；
  - 命名沿用 PROJECT_STATE「下一步计划 ⑤」的 bugfix 命名，但本质上是从 main 拉的新任务分支，走完整 PR 流程；
- 缺陷修复子分支：若评审打回，从当前 bugfix 分支拉 `bugfix/task-053-<描述>`，**只允许一层**；
- 禁止直接 push `main`。

## Git 提交要求

按 4 个功能单元拆 4 个原子提交：

1. `fix(scan): ThrottledEmitter 加 trailing 补偿（或 emit_force），最后事件不被节流吞 (#task-053)`（G7.1 后端）
   —— 含 `engine/src/scan/runner.rs` + 配套单测。
2. `fix(scan): ScanOrchestrator 收尾强制发送 percent=100 (#task-053)`（G7.2 后端）
   —— 含 `engine/src/scan/orchestrator.rs` + 配套单测。
3. `test(scan): 新增经过 ThrottledEmitter 的尾部用例，断言最后到达 percent=100 (#task-053)`（G7.4 关键回归）
   —— 含 `engine/tests/` 下测试文件。
4. `fix(frontend): Dashboard scan_start resolve 后兜底置 100 + 延迟退订；SafeBox NULL 来源渲染「历史（无来源）」 (#task-053)`（G7.3 + G8.1 前端）
   —— 含 `app/src/pages/Dashboard.tsx`、`app/src/pages/SafeBox.tsx`。
5. 自验全绿后，最后一次提交：`docs(state): 任务053完成，G7 进度条收尾 + G8 历史来源文案就绪 (#task-053)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 053: ...`；
   - 「最近一次变更」追加一行，含 G7/G8 修复摘要 + commit hash；
   - 「已知问题」段把走查报告 G7/G8 两条标记已解决；
   - 「下一步计划」改为「派发 task-054：第三轮复验走查（NSIS/MSI 各一遍）+ release.yml target_commitish + v0.1.6 发版」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-27  task-053  进度条尾部冲刷（G7）+ 历史来源文案（G8）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test -p engine` 退出码 0（245+8 现有 + 本任务新增 ≥2 条全绿）；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：G7.1 trailing/force、G7.2 收尾强制发送、G7.4 经过 ThrottledEmitter 的尾部用例（关键回归）。
- **集成测试（必做）**：`cargo test -p engine` 全绿，确保现有不回归。
- **端到端写路径（必做）**：本任务不涉及 DB 字段派生 UI（G8 只是 NULL 文案区分），但 G7.4 的「经过 ThrottledEmitter」用例本质上就是生产链路端到端，**不可用 VecEmitter 替代**。
- **真机冒烟（必做、不可裁剪）**：
  - G7：连续扫描 3 次，每次结束进度条都停在 100%（不是 96/88）；
  - G7：扫描结束后 `scannedItems` 计数与清理列表总数一致；
  - G8：保险箱老行（source_task_id=NULL）来源列显示「历史（无来源）」，新行仍显示 `#N`；筛选框旁有「早于 v0.1.6」说明文案；
  - CDP 抓 console 0 红错（G6 已知项除外）。
- **打包安装冒烟（必做、不可裁剪）**：NSIS 干净目录安装后，G7/G8 三条在安装态复跑一遍。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务命中「前端任务判定清单」（Dashboard 进度条 + SafeBox 来源列/Session 筛选框，全部真机可观察），真机/打包冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| G7.1 trailing/force 实现 | 单测输出（被丢事件最终被补发）+ 代码 diff |
| G7.2 Orchestrator 收尾 100 强制发送 | 单测输出（脚本循环结束后收到 percent=100） |
| G7.4 经过 ThrottledEmitter 尾部用例 | 单测输出（**关键回归**：最后到达 percent === 100，且事件数 ≤ 节流前） |
| G7.3 前端兜底 + 延迟退订 | 真机截图：扫描结束进度条停在 100%（连续 3 次） |
| G8.1 历史来源文案 | 真机截图：老行显示「历史（无来源）」灰色、新行显示 `#N`、筛选框旁有说明文案 |
| 不回归 | `cargo test -p engine` 全绿 + `pnpm test` 全绿 |
| 真机冒烟 | CDP 抓 page + consoleAPICalled（排除 G6 已知项后）0 红错 |
| 打包安装冒烟 | NSIS 干净目录安装路径 + G7/G8 复跑证据 |
| CI | PR URL + run id + conclusion=success |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `bugfix/task-053-progress-tail-flush`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - G7.1 最终方案选择（trailing 补偿 vs emit_force，二选一 + 理由）；
   - G7.4 关键回归用例代码摘录 + 通过输出；
   - G7.3 延迟退订实现细节（200ms 兜底 vs 收到 100 立刻退订）；
   - G8.1 文案最终样式（颜色、字号、位置）；
   - `cargo test -p engine` 通过输出；
   - `pnpm build` 输出（无 error）；
   - 真机冒烟证据（G7 三次扫描截图 + G8 老行/新行对比截图）；
   - 打包安装冒烟证据（NSIS 干净目录安装路径）；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（G6 设置页 IPC 红错仍为已知项，发版不阻塞）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务053-进度条尾部冲刷与历史来源文案 (#task-053)`，
   该提交只含 `tasks/053-progress-tail-flush.md` 一个文件，落在 `bugfix/task-053-progress-tail-flush` 分支最前端。

---

## 实现备注

> 执行日期：2026-10-27　分支：`bugfix/task-053-progress-tail-flush`（基线 `203e874`，从 main `57abcfc` 拉出）

### 1. 实际改动文件清单

| 文件 | 类型 | 改动 |
|---|---|---|
| `engine/src/ipc/progress.rs` | 后端（**白名单缺口，见 §9**） | trait 加 `emit_force`；`ThrottledEmitter` 覆写；`Box<dyn>` 透传；文件头补节流语义说明 |
| `engine/src/scan/orchestrator.rs` | 后端 | 收尾改 `emit_force_progress`；新增该私有方法 |
| `engine/tests/scan_progress_throttled_tail.rs` | 测试（新增） | 5 条，走真实 `ThrottledEmitter` |
| `app/src/pages/Dashboard.tsx` | 前端 | `TAIL_UNLISTEN_DELAY_MS` + 可变句柄退订 + resolve 后置满 + `tailTimerRef` 卸载清理 |
| `app/src/pages/SafeBox.tsx` | 前端 | 来源列 NULL → 「历史（无来源）」；列宽 90→120；筛选框旁说明文案 |
| `app/src/pages/SafeBox.test.tsx` | 测试（**白名单缺口，见 §9**） | 原「来源列显示 —」用例改断言新文案 + 新增说明文案用例 |
| `docs/PROJECT_STATE.md` | 文档 | 7 处（见 §10） |

`engine/src/scan/runner.rs` 任务书白名单里列了，**实际未改**——`ThrottledEmitter` 不在 runner.rs，见 §9。

### 2. G7.1 最终方案：`emit_force()`（不选 trailing 补偿）

**选它的理由**：trailing 补偿要在节流器里挂后台定时器（或注入 fake clock）才能「窗口到点补发」，引入线程生命周期、`Instant` 与真实时钟耦合、测试要造时钟；`emit_force` 是同步直发，**无时钟依赖、确定性可测**，而且语义更准——收尾事件不是「高频中间进度」，本来就不该进节流窗口。

**关键实现细节（三处容易写错的地方）**：

```rust
// ① trait 默认实现 = 直接转发，本身不节流的实现无需覆写
pub trait ProgressEmitter: Send + Sync + 'static {
    fn emit(&self, payload: &ScanProgress);

    fn emit_force(&self, payload: &ScanProgress) {
        self.emit(payload);
    }
}

// ② ThrottledEmitter：转发 + 重置窗口
fn emit_force(&self, payload: &ScanProgress) {
    // 先落时间戳再发送：`if let` 块结束即释放锁，inner.emit 不持锁。
    if let Ok(mut guard) = self.last.lock() {
        *guard = Some(Instant::now());
    }
    self.inner.emit(payload);
}

// ③ Box 透传——漏了等于白加
impl ProgressEmitter for Box<dyn ProgressEmitter> {
    fn emit(&self, payload: &ScanProgress) { self.as_ref().emit(payload); }
    fn emit_force(&self, payload: &ScanProgress) { self.as_ref().emit_force(payload); }
}
```

- **为什么 force 要重置窗口**：不重置的话，紧随 force 之后的普通 `emit` 会因为「距上次**普通** emit 已超 100ms」而立刻再发一条 → 100ms 窗口内出现两次推送，破坏 ARCHITECTURE §3 的 ≤1 次/100ms 约束。重置后 force 事件本身充当本窗口的唯一次推送。用例 `emit_force_resets_throttle_window` 固化了这条。
- **为什么 Box 必须透传**：`Box<dyn ProgressEmitter>` 若用默认 `emit_force`，会退化成调 `emit` → 被内层 `ThrottledEmitter` 节流吞掉，方法形同不存在。用例 `boxed_emitter_emit_force_still_passes_through` 用 `Arc<VecEmitter>` 共享观测点做**真断言**（不是「不 panic 就算过」）。
- 生产链路实际类型是 `&ThrottledEmitter<TauriProgressEmitter>`，被 coerce 成 `&dyn ProgressEmitter` 传进 `ScanOrchestrator`（`commands.rs:77`）。加在 trait 上才能让编排层通过 `dyn` 调到——这也是**没改 `commands.rs`** 的原因。

### 3. G7.2 编排层收尾

```rust
if summary.scripts_succeeded.is_empty() {
    return Err(OrchestratorError::AllScriptsFailed(total));
}
// 收尾：无论最后一个脚本是否成功，只要整体算成功就补一条 100%。
// 必须 force——普通 emit 会被节流吞（task-053 G7.2）。
self.emit_force_progress(100, "done", summary.total_items as u64);
```

槽位边界事件（20/40/60/80）**保持普通 `emit`**：脚本之间间隔以秒计，远大于 100ms，不会被吞；全走 force 等于让节流形同虚设。

### 4. G7.4 关键回归用例（代码摘录）

```rust
#[test]
fn throttled_orchestrator_tail_event_reaches_100() {
    let (dir, conn, sid) = setup();
    write_all_scripts(dir.path());

    // 基线：裸 VecEmitter（不节流），拿到「节流前应有的事件数」。
    let raw = VecEmitter::default();
    ScanOrchestrator::new(sid, &conn, &raw).run_all(dir.path()).unwrap();
    let raw_count = raw.snapshot().len();

    // 被测：与生产同构的 ThrottledEmitter 包装层。
    let throttled_inner = VecEmitter::default();
    let throttled = ThrottledEmitter::new(throttled_inner);
    ScanOrchestrator::new(sid, &conn, &throttled).run_all(dir.path()).unwrap();

    let events = throttled.inner().snapshot();
    // ① 关键断言：最后到达的是 100，不是 96/88。
    assert_eq!(events.last().unwrap().percent, 100, ...);
    // ② 节流仍生效：事件数必须少于节流前的全量（不是全放行）。
    assert!(events.len() < raw_count, ...);
    // ③ 单调不减、不越界
}
```

**反向验证（这一步才是「用例真的有效」的证据）**：把 `emit_force_progress(100, "done", …)` 临时改回 `emit_progress(…)` 后重跑：

```
assertion `left == right` failed: 收尾事件被节流吞了——最后到达的是 80，完整序列 [0, 20, 40, 60, 80]
  left: 80
 right: 100
test result: FAILED. 0 passed; 1 failed
```

`[0,20,40,60,80]` 与真机 G7 现象（停在 80/88）完全一致；还原后 5/5 绿。task-052 的教训是「`VecEmitter` 绕过生产链路所以断言必绿」，这条反向验证就是防止重演。

### 5. G7.3 前端延迟退订

选了「**收到 `percent >= 100` 立刻退订 / 否则 200ms 兜底退订**」，不是单纯的固定 200ms 延迟：

```tsx
const TAIL_UNLISTEN_DELAY_MS = 200;

const handle: { unlisten: UnlistenFn | null } = { unlisten: null };
let stopped = false;

const stopListening = () => {
  if (stopped) return;
  stopped = true;
  if (tailTimerRef.current !== null) { clearTimeout(tailTimerRef.current); tailTimerRef.current = null; }
  if (handle.unlisten) { handle.unlisten(); handle.unlisten = null; }
};

handle.unlisten = await listen<ScanProgress>("scan://progress", (e) => {
  setProgress(e.payload);
  if ((e.payload.percent ?? 0) >= 100) stopListening();   // 收尾已到，不必等
});
if (stopped && handle.unlisten) { handle.unlisten(); handle.unlisten = null; }  // 注册期竞态补退订

await api("scan_start", { scope: "full", targetDrive: null });
setProgress((p) => (p ? { ...p, percent: 100 } : p));      // UI 兜底置满
setScanStatus("done");
// finally 分支：if (!stopped) tailTimerRef.current = setTimeout(stopListening, 200);
```

三个细节：
- **可变句柄 + `stopped` 标记**：`listen` 回调可能早于 `await` 返回就触发（收尾事件极快），那时 `handle.unlisten` 还是 `null`，光靠 `if (unlisten) unlisten()` 会漏退订 → 用 `stopped` 在赋值后补一次。
- **`setProgress` 的 null 守卫**：`p ? {...p, percent:100} : p`——一条事件都没收到时保持 `null`，不伪造 `sessionId: 0` 的假进度对象（否则进度条会渲染出一个假的会话）。
- **`tailTimerRef` 卸载清理**：`useEffect(() => () => clearTimeout(...), [])`，避免组件卸载后定时器仍去 `unlisten`。

`scannedItems` / `currentPath` 渲染未动（`{...p, percent: 100}` 只覆盖 percent）。

### 6. G8.1 文案最终样式

| 位置 | 内容 | 样式 |
|---|---|---|
| 来源列（`sourceTaskId == null`） | `历史（无来源）` | `color:#9ca3af`、`fontSize:12`、`title="该条目入箱时间早于来源功能上线（v0.1.6），当时未记录来源批次，无法追溯"` |
| 来源列（有值） | `#N` | 不变（继承 `tdStyle` 13px） |
| 来源列表头宽度 | `90` → `120` | 容得下 7 个汉字 |
| 筛选栏（Session 多选右侧） | `早于 v0.1.6 的条目无来源信息` | `color:#9ca3af`、`fontSize:11`，紧跟「（未选 = 全部 session）」之后 |

**不改 DB**——`source_task_id` 当时就是 NULL，信息已丢失，强行回填只能靠猜（按时间窗口匹配 `cleanup_task`），会把「未知」伪装成「已知」，比显示「历史」更糟。

### 7. 门禁输出

```
cargo check --workspace --all-targets                     → 0
cargo clippy --workspace --all-targets -- -D warnings     → 0
cargo fmt --all -- --check                                → 0
cargo test --workspace   → passed=250  failed=0  ignored=8
pnpm --filter app lint   → 0 error（2 条既有 react-refresh warning）
pnpm --filter app test   → 7 files / 54 passed
pnpm --filter app build  → 181.51 kB │ gzip 58.01 kB
```

- `250 = 245（task-052 基线）+ 5（本任务新增）`；`ignored=8` 为既有 `#[ignore]`（含真机 notepad / RM 相关用例），未变。
- vitest `54 = 53 + 1`（新增「早于 v0.1.6」说明文案用例；来源列用例是改写不是新增）。
- build 从 180.85 → 181.51 kB（+0.66 kB，来自注释与文案字符串）。
- **`pnpm test` 首次跑出现一次 `EPERM: operation not permitted, open 'C:\Users\An\AppData\Local\Temp\...'`（vitest 写临时目录，非代码问题），重跑即 7 files / 54 passed 全绿**；与 task-052 记录的同一环境抖动。

### 8. 真机冒烟 / 打包安装冒烟 / CI

| 项 | 状态 | 说明 |
|---|---|---|
| G7 连扫 3 次停在 100% 真机截图 | **移交测试 Agent** | 按 2026-10-26 用户新流程：开发 Agent 不做 GUI/真机 UI 测试 |
| G8 老行/新行对比截图 | **移交测试 Agent** | 同上 |
| CDP Console 0 红错 | **移交测试 Agent** | 同上（口径「排除 G6 已知项」） |
| NSIS 干净目录安装后 G7/G8 复跑 | **移交测试 Agent** | 同上 |
| CI（PR + windows-latest 五道闸） | **移交测试 Agent** | 按 2026-10-25 用户新流程：开发分支不跑 CI，测试轮拉 PR 触发；**未建 PR** |

开发轮已做的替代验证：`cargo test --workspace` 250+8 全绿（含走真实 `ThrottledEmitter` 的 5 条尾部用例）+ `pnpm build` 0 error + 反向验证证明用例能抓住回归。真机/安装态证据按职责划分由测试 Agent 产出。

### 9. 与任务书不符的偏差（2 项）

1. **白名单漏 `engine/src/ipc/progress.rs`**。任务书「允许修改范围」写的是 `engine/src/scan/runner.rs`，但 `ThrottledEmitter` 实际定义在 `engine/src/ipc/progress.rs:69`（§G7.1 自己也写了「具体实现文件由 DevAgent 定位」）。**不改这个文件无法实现 G7.1**，故改了；`runner.rs` 未动。建议后续任务书把 `ipc/progress.rs` 列进白名单。
2. **白名单漏 `app/src/pages/SafeBox.test.tsx`**。G8.1 要求把来源列的 `—` 改成「历史（无来源）」，而既有用例 `表格渲染「来源」列：#N 与 — 两种形态` 断言 `screen.getAllByText("—")`，改文案后必红。同步改断言并补一条说明文案用例。

### 10. `docs/PROJECT_STATE.md` 改动（7 处）

1. 「待办事项」追加 `- [x] 053: …`；
2. 「已知问题」G7 加删除线 + 已解决说明（含反向验证证据）；
3. 「已知问题」G8 加删除线 + 已解决说明；
4. 「最近一次变更」追加 `2026-10-27：task-053 开发完成（G7 + G8），状态 testing` 整段；
5. 「下一步计划」⑤ 划掉标完成、⑥ 改为「派发 task-054：第三轮复验走查（NSIS/MSI 各一遍）+ `release.yml` target_commitish + v0.1.6 发版」；
6. 「当前负责人 Agent」改 **测试 Agent（提示词 9）**；
7. 「Git 提交记录」加 7 行 + 「变更轨迹」加 1 行。

### 11. 遗留问题

- **G6**（设置页 `rules_check_update` 首调 `ipc.localhost` 红错）仍为已知项，Tauri 2.x 行为，发版不阻塞；测试轮 Console 门禁口径为「排除 G6」。
- **`emit_force` 只加在 `ProgressEmitter` 上，未加「保证投递」语义**：它保证的是「不被**本层**节流丢弃」，跨进程/跨 webview 的投递失败仍由 `TauriProgressEmitter` 吞掉（`emit` 失败仅 `eprintln`）。真正要「事件必达」得另做 ack 机制，本任务不做。
- **`TAIL_UNLISTEN_DELAY_MS = 200` 是经验值**：基于「Tauri 事件投递通常在几十 ms 内到达」。若真机上仍偶发「100 未到」，第一调参点就是这个值；更稳的做法是「收到 100 再退订」已经实现了，200ms 只是兜底。
- 前端 `SafeBox.tsx` 的 2 条 `react-refresh/only-export-components` warning 为既有（导出 `presetFromTs`/`actionLabel` 纯函数），非本任务引入。

### 三句话结论

1. G7 的根因是「节流丢弃式 + 编排层无逃生口 + 前端退订时序」三者叠加，本任务**三处都动了**，并用一条走真实 `ThrottledEmitter` 的用例 + 一次反向验证把它钉住（改回普通 emit 立刻红，停在 80，与真机一致）。
2. G8 只是展示层问题（NULL 无法追溯），改文案 + 说明，**不碰 DB**。
3. 按用户新流程，开发轮只交本地门禁全绿（250+8 / 54 / build 0 error），CI、真机、安装态冒烟全部移交测试 Agent。
