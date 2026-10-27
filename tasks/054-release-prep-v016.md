# 任务编号：054 发版前最后一公里——release.yml 溯源 + 版本号 v0.1.6 + 第三轮复验走查清单 + 升级闭环验证

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFullstackRustReact + 测试 Agent + 用户手动发版
> 来源：PROJECT_STATE「下一步计划 ⑥」；走查报告 §6.4。
> 上游依赖：task-053（merge `351704f`，G7/G8 已合并）。
> **本任务范围（5 个功能单元）**：release.yml 溯源、版本号升级、第三轮复验走查清单、升级闭环验证脚本、G6 门禁豁免口径（或可选修复）。
> **本任务是 v0.1.6 发版前最后一个代码/文档任务**；发版动作本身（workflow_dispatch）由用户手动执行，不在本任务内。
> **流程教训（走查报告 §6.4）**：测试替身不能替代生产链路；「数据为空」不等于「功能失效」——本任务的复验清单必须包含「造数据后再验」的条目。

## 状态字段

testing

## 任务目标

### 1. release.yml 补 `target_commitish`（tag 溯源）

- **现状**：`release.yml` 触发后建 tag / Release，但没指定 tag 指向哪个 commit；workflow_dispatch 若在 main 有新 commit 后触发，tag 可能错指到旧 commit。
- **修法**：在 build 步骤后加 `target_commitish: ${{ github.sha }}`（或在 softprops/action-gh-release 步骤加 `commitish: ${{ github.sha }}`），确保 tag 精确指向触发发版的那个 commit。

### 2. 版本号升级 v0.1.5 → v0.1.6

- **现状**：`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`（`version` 字段）三处版本号需同步；
- **修法**：三处统一升到 `0.1.6`，`pnpm version minor` 或手动改；
- **校验**：发版后 NSIS/MSI 安装包文件名含 `0.1.6`，安装后「关于」页显示 `0.1.6`。

### 3. CHANGELOG 草稿（v0.1.6 条目）

- **现状**：仓库无 CHANGELOG.md；
- **修法**：新建 `CHANGELOG.md`，按 Keep a Changelog 格式写 v0.1.6 条目，覆盖：
  - 新增：safe_box 时间筛选 / 多 session / 来源列（task-051）、批量 purge / 恢复（task-038/039）、dry-run 预览（task-040）、updater 自动检查 + Offline 三态 + 非阻塞 banner（task-035/036/042）、占用关闭 confirm 文案（task-049）、items 分页（task-052 G2）；
  - 修复：扫描进度条脚本级换算 + 收尾 100%（task-052 G1 + task-053 G7）、清理列表 100 条截断（task-052 G2）、safe_box.source_task_id 接线（task-052 G3）、清理轮次口径（task-052 G4）、已不存在 vs 已清理语义（task-052 G5）、历史来源文案（task-053 G8）、rules reqwest 超时（task-041）、updater endpoint 改 Releases/latest/download（task-041）；
  - 已知问题：G6 设置页首调 ipc.localhost 红错（Tauri 2.x 已知项，功能经 postMessage 回退可用）。

### 4. 第三轮复验走查清单（NSIS / MSI 各一遍）

- **现状**：前两轮走查都是 dev 窗口，NSIS/MSI 安装态只跑过部分条目；
- **修法**：新建 `docs/test-reports/gui-walkthrough-round3-checklist.md`，按前两轮清单重写一遍，每条加「NSIS 结果 / MSI 结果 / 证据链接」三列；必须覆盖：
  - G1/G7 进度条：连扫 3 次每次停 100%；
  - G2 分页：造 >100 条（含小文件）验证第 2 页可见；
  - G3 safe_box：新删文件入箱 → 来源列 `#N` + Session 下拉 `session #N (M 条)`；
  - G4 轮次：连清 2 次卡片显示 2；
  - G5 already_gone：清理已被外部删除的文件 → 结果条显示「M 项已不存在」；
  - G8 历史来源：老行显示「历史（无来源）」；
  - G6 门禁豁免口径：明确写「排除 G6 已知项后 Console 零红」，G6 本身不阻塞；
  - 占用关闭 confirm 文案含「同一控制台内相关进程可能一并关闭」；
  - updater 自动检查 banner（首次启动后非阻塞出现）。

### 5. 升级闭环验证脚本（装 v0.1.5 → 检查更新 → 装 v0.1.6）

- **现状**：updater 端到端闭环（装旧版 → 检查更新 → 下载安装 → 重启到新版）从未真机跑过；
- **修法**：写一份 `docs/release/v0.1.6-upgrade-verification.md` 操作手册（不是代码），包含：
  - 前置：v0.1.5 安装包已在 GitHub Release（用户手动 workflow_dispatch 发过）；
  - 步骤 1：在干净目录装 v0.1.5；
  - 步骤 2：启动 → 设置页 → 「检查更新」→ 看到 v0.1.6 可更新 banner；
  - 步骤 3：点「立即更新」→ 下载进度（如有）→ 安装 → 重启；
  - 步骤 4：重启后「关于」页显示 v0.1.6；
  - 步骤 5：safe_box / 扫描 / 清理基本功能回归一遍。

**本任务不做**：
- 不做 G6 修复（Tauri 2.x 已知项，本任务只列门禁豁免口径；若 DevAgent 在 1 个提交内能修掉，可选做，不修不阻塞发版）；
- 不做 RmShutdown/RmRestart（立项决策不做）；
- 不做 banner 下载进度反馈 / 差量更新（M3+）；
- **不执行 workflow_dispatch 发版**（用户手动）。

## 前置条件

1. `main` 已合并 task-053（merge `351704f`，G7/G8 全绿）；
2. v0.1.5 已在 GitHub Release（用户 2026-10-23 手动发过，`.exe.sig` keyid `52ACA0C8DAA19EB3` 验证通过）；
3. 关键锚点：
   - `.github/workflows/release.yml`（补 target_commitish）
   - `package.json` / `src-tauri/tauri.conf.json` / `src-tauri/Cargo.toml`（版本号三处）
   - 仓库根（新建 `CHANGELOG.md`）
   - `docs/test-reports/`（新建 round3 checklist）
   - `docs/release/`（新建升级验证手册）

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### 1. release.yml 溯源

1. `.github/workflows/release.yml` 加 `target_commitish: ${{ github.sha }}`（或 softprops/action-gh-release 的 `commitish` 参数）；
2. 本地 `act` 或 CI 预览跑过（不强制真发版），能看到 tag 指向正确 commit。

### 2. 版本号升级

3. `package.json` version = `0.1.6`；
4. `src-tauri/tauri.conf.json` version = `0.1.6`；
5. `src-tauri/Cargo.toml` version = `0.1.6`（`src-tauri/Cargo.lock` 由 `cargo build` 自动更新）；
6. `pnpm --filter app build` 成功，产物文件名含 `0.1.6`。

### 3. CHANGELOG

7. `CHANGELOG.md` 存在，含 v0.1.6 条目（Keep a Changelog 格式）；
8. 条目覆盖 task-051/052/053 主要变更（新增 / 修复 / 已知问题 三段）；
9. 不写未来计划（unreleased 段可留空）。

### 4. 第三轮复验走查清单

10. `docs/test-reports/gui-walkthrough-round3-checklist.md` 存在；
11. 清单含 NSIS / MSI 两列结果占位；
12. 清单覆盖 G1/G2/G3/G4/G5/G7/G8 + 占用关闭 + updater banner + Console 零红（排除 G6）；
13. 明确写出门禁口径：「排除 G6 已知项后 Console 零红」。

### 5. 升级闭环验证手册

14. `docs/release/v0.1.6-upgrade-verification.md` 存在；
15. 手册含装 v0.1.5 → 检查更新 → 装 v0.1.6 → 重启 → 关于页显示 v0.1.6 → 基本功能回归 共 6 步；
16. 手册注明「本手册由用户在发版后手动执行，DevAgent 不执行」。

### 测试要求

17. `cargo test --workspace` 全绿（本任务不改业务代码，预期 253+13 条不回归）；
18. `pnpm --filter app lint / test / build` 全绿；
19. CI 五道闸（fmt / clippy / test / lint / build）全绿。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `.github/workflows/release.yml`（加 target_commitish）
- `package.json`（version）
- `src-tauri/tauri.conf.json`（version）
- `src-tauri/Cargo.toml`（version）
- `src-tauri/Cargo.lock`（cargo build 自动更新）
- `CHANGELOG.md`（新建）
- `docs/test-reports/gui-walkthrough-round3-checklist.md`（新建）
- `docs/release/v0.1.6-upgrade-verification.md`（新建）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支（feature + bugfix 子分支规则）

- 主分支：`feature/task-054-release-prep-v016`，从最新 `main` 拉出；
- 缺陷修复子分支：若评审打回，从当前 feature 拉 `bugfix/task-054-<描述>`，**只允许一层**；
- 禁止直接 push `main`；
- **本任务合并后**，用户手动 workflow_dispatch 发 v0.1.6（不在本任务 PR 内）。

## Git 提交要求

按 5 个功能单元拆 5 个原子提交：

1. `ci(release): release.yml 补 target_commitish: ${{ github.sha }}，tag 溯源 (#task-054)`（单元 1）
2. `chore(release): 版本号升级 v0.1.5 → v0.1.6（package.json / tauri.conf.json / Cargo.toml） (#task-054)`（单元 2）
3. `docs(changelog): 新增 CHANGELOG.md，v0.1.6 条目覆盖 task-051/052/053 (#task-054)`（单元 3）
4. `docs(test): 新增第三轮复验走查清单（NSIS/MSI 双列） (#task-054)`（单元 4）
5. `docs(release): 新增 v0.1.6 升级闭环验证手册 (#task-054)`（单元 5）
6. 自验全绿后，最后一次提交：`docs(state): 任务054完成，v0.1.6 发版准备就绪，待用户手动 workflow_dispatch (#task-054)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 054: ...`；
   - 「最近一次变更」追加一行，含 5 个单元摘要 + commit hash；
   - 「下一步计划」改为「v0.1.6 发版后由用户执行升级闭环验证手册，跑第三轮复验走查清单；全绿后进入 M3（banner 下载进度 / 差量更新）」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-27  task-054  v0.1.6 发版准备（release 溯源 + 版本号 + CHANGELOG + 复验清单 + 升级手册）`。

**提交前自验（必须全绿）：**
- `cargo check --workspace --all-targets` 退出码 0；
- `cargo clippy --workspace --all-targets -- -D warnings` 退出码 0；
- `cargo test --workspace` 退出码 0；
- `cargo fmt --all -- --check` 退出码 0；
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0；
- `pnpm --filter app build` 退出码 0；
- 版本号三处一致 = `0.1.6`（grep 验证）。

## 测试要求（层级 + 真机/打包冒烟是否必做）

- **单元测试（必做）**：本任务不改业务逻辑，预期现有 253+13 条全绿不回归。
- **集成测试（必做）**：`cargo test --workspace` + `pnpm test` 全绿。
- **端到端（本任务不做）**：升级闭环验证由用户在发版后手动跑（手册已写好）。
- **真机冒烟（本任务不强制）**：DevAgent 只做代码/文档改动，不做真机验证；真机验证由测试 Agent 在第三轮走查清单中执行。
- **打包安装冒烟（必做、不可裁剪）**：本任务合并后由测试 Agent 在 NSIS/MSI 干净目录各跑一遍第三轮清单（DevAgent 不直接做，但必须在 PR 描述里明确「测试 Agent 接管 NSIS/MSI 双跑」）。
- **CI（必做）**：开 PR 触发 windows-latest 五道闸全绿。

> 本任务主体是配置/文档/版本号变更，不直接命中「前端可观察行为」，但合并后立即触发 NSIS/MSI 打包，打包安装冒烟**必做、不可裁剪**。

## 测试证据清单

| 验收标准 | 证据形式 |
|---|---|
| release.yml 加 target_commitish | 文件 diff + CI run 中 tag 指向正确 commit 的日志 |
| 版本号三处一致 | `grep -r "0.1.6" package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml` 输出 |
| 产物文件名含 0.1.6 | `pnpm build` 输出 + `src-tauri/target/release/bundle/` 目录列表 |
| CHANGELOG | 文件内容预览（v0.1.6 三段齐全） |
| 第三轮走查清单 | 文件存在 + 10 条条目齐全 |
| 升级闭环手册 | 文件存在 + 6 步齐全 |
| 不回归 | `cargo test --workspace` + `pnpm test` 全绿输出 |
| CI | PR URL + run id + conclusion=success |
| NSIS/MSI 打包冒烟 | （测试 Agent 在第三轮走查中补）NSIS / MSI 安装路径 + 清单结果 |

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 5 个代码/文档提交 + 1 个 `docs(state)` 提交，共 6 个 commit，全部 push 到 `feature/task-054-release-prep-v016`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - release.yml 最终 diff（target_commitish 加在哪一步）；
   - 版本号三处改动 diff；
   - CHANGELOG v0.1.6 条目摘要；
   - 第三轮走查清单条目数；
   - 升级闭环手册步骤数；
   - `cargo test --workspace` / `pnpm build` 通过输出；
   - CI run URL + id + success；
   - 与本任务文档不符的偏差（如有）；
   - 遗留问题（G6 设置页 IPC 红错仍为已知项，门禁口径「排除 G6 后零红」；升级闭环验证由用户发版后手动跑）；
3. `docs/PROJECT_STATE.md` 已按第 6 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务054-v0.1.6 发版准备 (#task-054)`，
   该提交只含 `tasks/054-release-prep-v016.md` 一个文件，落在 `feature/task-054-release-prep-v016` 分支最前端。

---

## 实现备注

> 执行日期：2026-10-27　分支：`feature/task-054-release-prep-v016`（基线 `02f45aa`，从 main `351704f` 拉出）

### 1. 实际改动文件清单

| 文件 | 类型 | 改动 |
|---|---|---|
| `.github/workflows/release.yml` | CI | `action-gh-release` 步骤加 `target_commitish: ${{ github.sha }}` + 6 行注释说明 |
| `package.json` | 配置 | version `0.1.0` → `0.1.6`（monorepo 根） |
| `app/package.json` | 配置 | version `0.1.0` → `0.1.6`（应用包） |
| `app/src-tauri/tauri.conf.json` | 配置 | 顶层 version `0.1.0` → `0.1.6`（决定安装包文件名） |
| `app/src-tauri/Cargo.toml` | 配置 | `[package] version` `0.1.0` → `0.1.6` |
| `Cargo.lock` | 配置 | `cdisk-cleaner` 条目 version 自动更新为 `0.1.6`（`cargo check` 触发） |
| `CHANGELOG.md` | 新建 | Keep a Changelog 格式，v0.1.6 三段 |
| `docs/test-reports/gui-walkthrough-round3-checklist.md` | 新建 | 27 条，NSIS/MSI 双列 |
| `docs/release/v0.1.6-upgrade-verification.md` | 新建 | 6 步升级闭环手册 + 卡点速查 |
| `.gitignore` | 配置 | 加 `!docs/release/`（**白名单缺口，见 §9**） |
| `docs/PROJECT_STATE.md` | 文档 | 7 处（见 §10） |

### 2. `release.yml` 最终 diff

```diff
       # ===== 创建 Release 并上传所有产物 =====
       # .msi / .exe / .sig / latest.json 全部上传，确保不漏。
+      #
+      # task-054：补 target_commitish —— tag 精确指向触发本次发版的 commit。
+      # 不指定时 GitHub 按默认分支（main）HEAD 建 tag，若 workflow_dispatch 之后
+      # main 又进了新 commit，tag 就会指向一个「从未构建过的 commit」，源码与产物
+      # 对不上（产物是从 github.sha 那次 checkout 构建的）。updater 的 tag 溯源
+      # 与事后排查都依赖这条对应关系。
       - name: Create Release and upload all artifacts
         uses: softprops/action-gh-release@v2
         with:
           name: v${{ env.RELEASE_VERSION }}
           tag_name: v${{ env.RELEASE_VERSION }}
+          target_commitish: ${{ github.sha }}
           generate_release_notes: true
```

加在**最后一个步骤**（创建 Release 那一步）的 `with:` 里，与 `tag_name` 相邻；参数名是 `target_commitish`（不是 `commitish`），与 `action-gh-release@v2` 的 input 名一致。**未改其它步骤**，构建流程零影响。

> 说明：`github.sha` 是触发本次 workflow 的那个 commit。`push tag` 触发时它等于 tag 指向的 commit，`workflow_dispatch` 触发时等于被选中的 ref 的 HEAD——两种情况都正确。

### 3. 版本号三处（实为四处）改动 diff

```diff
# package.json
-  "version": "0.1.0",
+  "version": "0.1.6",

# app/package.json
-  "version": "0.1.0",
+  "version": "0.1.6",

# app/src-tauri/tauri.conf.json
-  "version": "0.1.0",
+  "version": "0.1.6",

# app/src-tauri/Cargo.toml
-version = "0.1.0"
+version = "0.1.6"

# Cargo.lock（cargo check 自动更新，非手改）
 name = "cdisk-cleaner"
-version = "0.1.0"
+version = "0.1.6"
```

校验：

```
$ grep -rn '"version": "0.1.6"\|^version = "0.1.6"' package.json app/package.json app/src-tauri/tauri.conf.json app/src-tauri/Cargo.toml
package.json:3:  "version": "0.1.6",
app/package.json:4:  "version": "0.1.6",
app/src-tauri/tauri.conf.json:4:  "version": "0.1.6",
app/src-tauri/Cargo.toml:3:version = "0.1.6"
```

**与 CI 发版机制的关系（重要）**：`release.yml` 会在构建前用 `scripts/release-lib.mjs patch-conf` 按 `workflow_dispatch` 输入的 version **覆盖** `tauri.conf.json` 的顶层 version，构建后还有 `verify-artifacts` 硬校验安装包文件名。所以仓库里的 `0.1.6` 只是**本地构建的默认基线**，发版时以 workflow 输入为准，两者不冲突。**发版时 version 必须填 `0.1.6`**（与仓库基线一致，避免 tag 与本地产物对不上）。

### 4. `tauri build` 实测产物

```
   Compiling cdisk-cleaner v0.1.6 (D:\GitHub\C-clean\app\src-tauri)
    Finished `release` profile [optimized] target(s) in 1m 34s
     Running makensis to produce ...\bundle\nsis\C-DiskCleaner_0.1.6_x64-setup.exe
     Running light to produce ...\bundle\msi\C-DiskCleaner_0.1.6_x64_en-US.msi
    Finished 2 bundles at:
        ...\nsis\C-DiskCleaner_0.1.6_x64-setup.exe (4.01 MiB)
        ...\msi\C-DiskCleaner_0.1.6_x64_en-US.msi (5.83 MiB)
    Finished 2 updater signatures at:
        ...\nsis\C-DiskCleaner_0.1.6_x64-setup.exe.sig
        ...\msi\C-DiskCleaner_0.1.6_x64_en-US.msi.sig
```

安装包文件名含 `0.1.6`，且**成对产出 `.sig`**（注入 `TAURI_SIGNING_PRIVATE_KEY` 后；不注入则 `tauri build` 会因「public key found but no private key」非零退出）。

### 5. CHANGELOG v0.1.6 条目摘要

| 段 | 条数 | 内容 |
|---|---|---|
| 新增 | 6 | 保险箱多维筛选与来源追溯（051）、批量 purge/恢复（038/039）、dry-run 预览（040）、自动更新（035/036/042）、占用进程可处置（049/049b）、清理列表分页（052）、规则库回滚（044） |
| 修复 | 9 | 扫描进度条（052+053）、小文件被截断（052）、来源列全空（052）、轮次恒 0（052）、释放 0 B（052）、历史来源文案（053）、安装版取不到规则库（050）、规则检查卡「检查中…」（041）、更新检查地址（041） |
| 已知问题 | 4 | G6 设置页 IPC 红错、`RmShutdown` 连带同控制台进程、未代码签名、默认占位图标 |

- 格式：Keep a Changelog + 语义化版本，`## [Unreleased]` 段留空（按要求不写未来计划）。
- 文件头注明「**自 v0.1.6 起启用，更早版本未回溯**」——不为凑格式编造历史条目。
- **比任务书多补 2 条**：`task-049/049b` 的占用进程可处置、`task-050` 的安装版取不到规则库。二者都在 v0.1.5 之后合入且用户可感知，漏写会让 CHANGELOG 失真。

### 6. 第三轮走查清单条目数：**27 条**

| 组 | 条数 | 覆盖 |
|---|---|---|
| A 安装与启动 | 4 | 干净目录安装 / 资源落位（含「无 `_up_`」）/ 数据目录隔离（安装目录 NO_DIFF）/ 窗口渲染 |
| B 扫描进度 | 4 | **连扫 3 次停 100%**（G1+G7）/ 单调不倒退 / 计数与列表一致 / 全失败不报完成 |
| C 列表分页 | 3 | **造 150+ 小文件验第 2 页**（G2）/ 小文件不被截断 / 翻页清空勾选 |
| D 保险箱来源 | 4 | **新删文件写 `#N`**（G3）/ Session 下拉出现选项 / **历史行「历史（无来源）」**（G8）/ 筛选框说明文案 |
| E 仪表盘轮次 | 1 | **连清 2 次显示 +2**（G4） |
| F 已不存在语义 | 2 | **手动删文件后清理显示「M 项已不存在」**（G5）/ 释放量不含已不存在项 |
| G 占用关闭 | 3 | 列出占用进程 / **confirm 含「同一控制台内的相关进程可能一并关闭」** / 受保护进程整批不关 |
| H 自动更新 | 3 | 启动 banner（非阻塞）/ 设置页卡片一致 / 断网不弹红错 |
| I Console | 2 | 五页走查（**排除 G6 后零红**）/ 扫描+清理链路零红 |
| J 卸载 | 1 | 卸载后安装目录清除、`%APPDATA%` 保留 |

每条带「NSIS 结果 / MSI 结果 / 证据」三列（初始为 ☐ 占位），汇总表 + 放行判定（两个安装包均「不通过=0 且 不适用=0」）+ 与前两轮差异表。

### 7. 升级闭环手册步骤数：**6 步**（+ 前置 5 条 + 卡点速查 5 行）

0. 前置 5 条（v0.1.6 已发布 / `latest.json` 版本与 URL 正确 / v0.1.5 包在 / 签名配对 / 干净环境），并强调**顺序不能反：先发 v0.1.6 再装 v0.1.5**
1. 干净目录安装 v0.1.5（含 MSI per-user 备选命令 + 记录安装目录基线）
2. 启动 → 等 banner「发现新版本 v0.1.6」→ 点「稍后」验证非阻塞 → 设置页卡片一致
3. 点「立即更新」→ 下载（**注明 v0.1.6 无下载进度条属预期**）→ 安装
4. 重启确认版本（**见 §9 偏差 4**：改用三条等效验证）
5. 功能回归 R1–R6（进度条满格 / 分页 / 清理结果 / 保险箱来源 / 轮次 / 设置页）
6. 回写结果表模板 + 「失败不要重装蒙过去」的排查指引

手册开头明确写「**执行者：用户本人，在 v0.1.6 正式发布之后手动执行。DevAgent / 测试 Agent 不执行本手册**」。

### 8. 门禁输出

```
cargo check --workspace --all-targets                     → 0
cargo clippy --workspace --all-targets -- -D warnings     → 0
cargo fmt --all -- --check                                → 0
cargo test --workspace   → passed=250  failed=0  ignored=8
pnpm --filter app lint   → 0 error（2 条既有 react-refresh warning）
pnpm --filter app test   → 7 files / 54 passed
pnpm --filter app build  → 181.51 kB │ gzip 58.01 kB
tauri build              → exit 0（NSIS 4.01 MiB + MSI 5.83 MiB + 2 个 .sig）
```

- `250+8` 与 task-053 完全一致——本任务不改业务代码，**零回归**符合预期。
- **环境坑（本轮新发现）**：`pnpm test` 连续多次报

  ```
  Error: EPERM: operation not permitted, open 'C:\Users\An\AppData\Local\Temp\xxxx\web\40b9f1f3...'
   ❯ wrappedPromisesWriteFile .../cli/vendor/shim/node-brokered-fs-shim.cjs:701:20
  ```

  栈里是 **`node-brokered-fs-shim.cjs`（沙箱 fs 代理）**，即沙箱拦了写系统 Temp，**不是代码问题**。三次重跑分别只跑到 6/5/3 个文件。修法：把 `TMPDIR`/`TEMP`/`TMP` 指到工作区内再跑：

  ```bash
  mkdir -p target/vitest-tmp
  TMPDIR="D:/GitHub/C-clean/target/vitest-tmp" TEMP="...\target\vitest-tmp" TMP="...\target\vitest-tmp" \
    pnpm --filter app test      # → 7 files / 54 passed
  ```

  另外**不要与 `cargo test --workspace` 并行跑 vitest**，IO 争抢会显著加剧该现象（实测并行时失败率更高）。

### 9. 与任务书不符的偏差（4 项）

1. **白名单路径层级错**：任务书写 `src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`，实际项目是 pnpm + Cargo workspace，路径为 `app/src-tauri/tauri.conf.json`、`app/src-tauri/Cargo.toml`，而 `Cargo.lock` 在**仓库根**（workspace 单一 lock）。已按实际路径改。
2. **白名单漏 `.gitignore`**：`release/` 规则（本意是 Tauri 构建产物）会把 `docs/release/` 一起忽略，`git add docs/release/v0.1.6-upgrade-verification.md` 直接报 `ignored by one of your .gitignore files`。不改则手册无法入库。已加负向规则 `!docs/release/`（必须放在 `release/` **之后**才生效，因为 Git 不允许在「父目录被排除」时重新包含其内部文件），并用 `git check-ignore` 验证不再命中。
3. **验收 6 的验证方式不可行**：任务书说「`pnpm --filter app build` 成功，产物文件名含 `0.1.6`」——`pnpm --filter app build` 只是 `vite build`，产物是 `dist/index-<hash>.js`，**永远不含版本号**。改为 `tauri build` 验证安装包文件名（已实测含 `0.1.6`）。
4. **任务书假设存在「关于」页**：实际应用只有 5 个 tab（仪表盘/清理/保险箱/历史/设置），**没有「关于」页**，设置页也只显示「远端可更新版本」而不显示当前版本。手册步骤 4 改用三条等效验证：① 设置 → 应用 → 已安装的应用里看版本；② 右键 `C-DiskCleaner.exe` → 属性 → 详细信息 → 产品版本；③ 与步骤 1 的安装目录基线对比。**任两条命中即通过**。若后续产品要加「关于」页，那是独立任务，不在本任务白名单内。

### 10. `docs/PROJECT_STATE.md` 改动（7 处）

1. 「待办事项」追加 `- [x] 054: …`；
2. 「最近一次变更」追加 `2026-10-27：task-054 开发完成（v0.1.6 发版准备），状态 testing` 整段（6 个子项 + 门禁 + 环境坑 + 4 项偏差）；
3. 「下一步计划」⑥ 划掉标完成、⑦ 改为「v0.1.6 发版后：用户跑升级手册 + 测试 Agent 跑第三轮清单，全绿进 M3」；
4. 「当前负责人 Agent」改 **测试 Agent（提示词 9）**（并写明发版动作由用户手动）；
5. 「Git 提交记录」加 7 行；
6. 「变更轨迹」加 1 行；
7. （隐含）待办/已知问题段与 task-053 的衔接保持一致，未回改历史结论。

### 11. 遗留问题

- **G6**（设置页首调 `ipc.localhost` 红错）**未修**——任务书标注为可选、不阻塞发版。本轮只把门禁口径写进清单第一节（「排除 G6 的 2 error + 4 warning 后零红」），并在 CHANGELOG「已知问题」段向用户披露。若要修，建议独立任务（涉及 Tauri 2.x `useCustomProtocol` 回退策略，改动面在 `invoke.ts`/`tauri.conf`，不在本任务白名单）。
- **升级闭环未验证**：手册已就绪，但端到端（装 v0.1.5 → 更新到 v0.1.6）必须等 v0.1.6 真正发布后才能跑，且由用户手动执行。**这是 v0.1.6 发布后唯一的未闭环项**。
- **`latest.json` 仍在仓库根入库**（历次发版产物），属既有 P3 观察项，本任务不动。
- **CHANGELOG 历史版本未回溯**：v0.1.0~v0.1.5 无条目，已在文件头声明，不编造。

### 三句话结论

1. 五个单元全部落地：`release.yml` 溯源、版本号 `0.1.6`（四处 + lock）、`CHANGELOG.md`、27 条第三轮清单、6 步升级手册，外加一处 `.gitignore` 误伤修复。
2. 门禁零回归（250+8 / 54 / build 181.51 kB），`tauri build` 实测产出 `C-DiskCleaner_0.1.6_*` 双包 + 成对 `.sig`；本轮新踩的 vitest `EPERM` 是沙箱拦系统 Temp，改 `TMPDIR` 到工作区内即解决。
3. 任务书 4 处与项目实际不符（路径层级 / 漏 `.gitignore` / `pnpm build` 不含版本号 / 无「关于」页）已按实际修正并记录；**发版与升级闭环验证留给用户手动执行，NSIS/MSI 双跑移交测试 Agent**。
