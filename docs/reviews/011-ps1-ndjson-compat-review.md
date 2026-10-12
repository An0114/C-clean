# 测试评审报告

## 本次任务名称

011 ps1 兼容层从零编写——5 个清理脚本 + NDJSON 输出协议 + 冒烟测试（M1 前置清理收官）

## Git 提交记录核对结果

✅ 8 个提交全部关联 `#task-011`，type 前缀规范（docs / feat / test），文件范围与任务文档 §Git 提交要求逐条对齐，无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `0ed9aa9` | docs(task) | tasks/011-ps1-ndjson-compat.md | ✅ |
| 2 | `5c0b137` | docs(scripts) | scripts/PROTOCOL.md | ✅ |
| 3 | `ff83b3c` | feat(scripts) | scan_temp.ps1 + scan_windows_update.ps1 | ✅ |
| 4 | `22d1e37` | feat(scripts) | scan_prefetch.ps1 + scan_browser_cache.ps1 + scan_dev_cache.ps1 | ✅ |
| 5 | `e7c06e4` | test(scripts) | scripts/smoke_test.ps1 | ✅ |
| 6 | `12a4616` | docs(state) | docs/PROJECT_STATE.md | ✅ |
| 7 | `7c47dac` | docs(task) | tasks/011-ps1-ndjson-compat.md（回填实现备注） | ✅ |
| 8 | `90f28b7` | docs(state) | docs/PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. NDJSON 协议文档 `scripts/PROTOCOL.md`

- ✅ item 事件 schema 完整：type/module_code/category/path/size_bytes/risk_level。
- ✅ done 事件 schema 完整：type/module_code/total_items/total_bytes。
- ✅ 明确 stdout 只输出 NDJSON，日志走 stderr。
- ✅ 明确 UTF-8 无 BOM（`[System.Text.UTF8Encoding]::new($false)`）。
- ✅ 明确不输出 risk_level=3，L3 由 Rust 侧 task-003 白名单二次过滤。
- ✅ 明确消费侧契约（item 入 cleanup_item、done 对账、非零退出码标记失败）。

### B. 5 个扫描脚本

- ✅ scan_temp.ps1（module_code=temp）：扫 `$env:TEMP` + `C:\Windows\Temp`。
- ✅ scan_windows_update.ps1（module_code=windows_update）：扫 `C:\Windows\SoftwareDistribution\Download`。
- ✅ scan_prefetch.ps1（module_code=prefetch）：扫 `C:\Windows\Prefetch`。
- ✅ scan_browser_cache.ps1（module_code=browser_cache）：扫 Edge/Chrome `...\User Data\Default\Cache`，注释明确不碰 Login Data/Bookmarks。
- ✅ scan_dev_cache.ps1（module_code=dev_cache）：扫 npm-cache / pip\cache / pnpm-store。

### C. 脚本通用要求

- ✅ 每个 ps1 头部 `#requires -Version 5.1`。
- ✅ 每个 ps1 头部注释块：用途 / MODULE_CODE / schema 简述。
- ✅ 无 `Write-Host` / `Write-Output "Processing..."` 人类日志。
- ✅ 无 `Remove-Item` / `Clear-RecycleBin` 等删除命令。
- ✅ `[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)` 强制 UTF-8 无 BOM。
- ✅ JSON 行用 `[Console]::Out.WriteLine($obj | ConvertTo-Json -Compress)` 直接写 stdout。
- ✅ 权限不足 `ErrorAction SilentlyContinue` + try/catch 写 stderr。
- ✅ 跳过 `$_.LinkType` 非空的 symlink/junction 文件。

### D. 冒烟测试（评审人复跑）

```
[OK] scan_temp.ps1:           items=165, total_bytes=16587796
[OK] scan_windows_update.ps1: items=6,   total_bytes=16535741
[OK] scan_prefetch.ps1:       items=0,   total_bytes=0
[OK] scan_browser_cache.ps1:  items=13,  total_bytes=32303428
[OK] scan_dev_cache.ps1:      items=33,  total_bytes=15121106
```

✅ 5/5 脚本 stdout 每行 `ConvertFrom-Json` 成功；最后一行 type=done；前面行 type=item；字段齐全；module_code 与脚本对应。

### E. 卫生与边界

- ✅ 未修改 engine/**、app/**、elevation/**、rules/** 已有文件。
- ✅ 未新增 Rust 依赖、未改 Cargo.toml。
- ✅ 未写 GitHub Actions workflow。
- ✅ 不硬编码用户路径——用 `$env:TEMP` / `$env:LOCALAPPDATA` / `$env:APPDATA`。
- ✅ 不触碰 L3 红线目录（Desktop/Documents/Downloads/Pictures）；browser_cache 只扫 Cache 子目录，不碰 Login Data/Bookmarks。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `powershell -File smoke_test.ps1` | ✅ 5/5 [OK] |
| `cargo check --workspace` | ✅ Finished in 11.00s，无 warning |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **junction 目录跟随**：`Get-ChildItem -Recurse -File` 对 junction 目录默认跟随递归，`$_.LinkType` 只跳过文件本身是 junction 的情况，不阻止跟随 junction 目录进去。真正的防环由 Rust 侧 task-008 的 canonicalize 二次保证。**第三档，M1-A 消费器实现时注意。**
2. **risk_level 全部硬编码为 1**：任务文档允许（本兼容层不区分 L1/L2，全部按 L1 输出，Rust 侧白名单二次过滤）。**第三档。**
3. **Prefetch 非管理员下 items=0**：预期行为，脚本不报错、只输出 done=0。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ ps1 只扫缓存目录，不碰 L3 个人目录；browser_cache 严格限定 `...\Cache` 子目录，不碰 Login Data/Bookmarks/History。 |
| A03 注入 | ✅ ps1 不执行外部输入；路径来自 `Get-ChildItem` 枚举，不拼接 shell 命令。 |
| A04 不安全设计 | ✅ ps1 不删除任何文件（无 Remove-Item），删除走 Rust 唯一入口。 |
| A08 数据完整性 | ✅ stdout 纯 NDJSON，stderr 分离；Rust 侧可逐行校验 JSON schema。 |

### 专项安全分析

- **ps1 会不会误删？** 不会。grep 全部 5 个脚本，无 `Remove-Item` / `Clear-RecycleBin` / `del` / `rm`。✅
- **ps1 会不会扫到个人文件？** 不会。扫描根目录全部是缓存目录（TEMP / SoftwareDistribution / Prefetch / Browser Cache / dev cache），不含 Desktop/Documents/Downloads/Pictures。✅
- **browser_cache 会不会碰密码/书签？** 不会。严格限定 `...\User Data\Default\Cache`，不碰 `User Data\Default\Login Data` / `Bookmarks` / `History`。✅
- **硬编码用户路径？** 没有。全部用 `$env:TEMP` / `$env:LOCALAPPDATA` / `$env:APPDATA` 展开。✅
- **stdout 会不会被人类日志污染？** 不会。grep 无 `Write-Host` / 裸 `Write-Output`；所有日志走 `[Console]::Error.WriteLine`。✅
- **编码会不会乱码？** 脚本强制 `UTF8Encoding($false)` 无 BOM；冒烟测试 `ConvertFrom-Json` 全过。✅

## 评审结论：**通过**

5 个 ps1 脚本从零编写完成，NDJSON 协议文档完整，冒烟测试 5/5 通过（评审人复跑一致）；无删除命令、无硬编码用户路径、不碰 L3 目录、stdout 纯 NDJSON；8 个提交全部关联 #task-011 无混杂；cargo check 无影响。**M1 前置清理清单（007~011）全部收官。**

## Git 操作记录

- **merge commit**：`d53b9d5`（`git merge --no-ff feature/task-011-ps1-ndjson-compat`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-011-ps1-ndjson-compat`，was 90f28b7）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务011评审报告 (#task-011)`

## 后续动作建议

1. **M1 业务模块 A（扫描器）**：Rust 主进程 spawn ps1 子进程、逐行收 NDJSON、写 cleanup_item 表、emit scan://progress。注意：
   - 按 `\n` split 后 trim `\r`（ps1 输出 `\r\n`）；
   - 逐行 `serde_json` 反序列化，校验 type ∈ {item, done}；
   - item 入表前再过 task-003 白名单 + task-008 真实路径解析；
   - junction 目录跟随风险由 canonicalize 二次兜底。
2. **M1 打包**：rules 路径改 Tauri 资源目录；在 Tauri State 缓存规则。
3. **后续 CI**：在 GitHub Actions Windows runner 上跑 ps1 冒烟测试。
