# TRD.md 技术需求文档

> 版本：v0.1 ｜ 日期：2026-10-11 ｜ 状态：基线版
> 上游文档：`docs/PRD.md`（产品需求，已定稿）、`docs/01-立项决策记录.md`（8 项已固化决策）
> 本文档只定义技术规范（接口、数据结构、依赖、测试标准、部署约束），不重复描述产品业务细节；业务行为以 PRD 为准。

---

## 1. 文档概述

### 1.1 目的

将 PRD 的产品需求翻译为开发可直接依据的技术约束，覆盖：

- 进程/模块划分与边界；
- 本地数据结构（SQLite Schema）与 JSON 规则库格式；
- 前端 ↔ Rust 核心 ↔ 提权子进程之间的接口规范；
- 外部依赖（Windows 系统组件、第三方库、远端规则源）清单；
- 测试验收标准与部署约束。

### 1.2 技术栈基线（来源：README + 立项决策 #1/#2/#4/#6）

| 层 | 选型 | 技术约束 |
|----|------|----------|
| 桌面框架 | Tauri 2 | Rust 核心进程 + WebView2 前端；不引入 Plan B |
| 前端 | React 18 + TypeScript + Vite | 渲染进程不直接访问文件系统/注册表，一切走 IPC |
| 核心引擎 | Rust（`engine/`） | M1 阶段为薄壳，通过白名单 IPC 调用兼容层；M2+ 逐步 Rust 化 |
| 兼容层 | PowerShell 5.1 脚本（`scripts/`） | 仅 M1 存在；脚本必须以 JSON 流输出，不做交互式输入 |
| 特权操作 | 独立提权 exe（`elevation/`） | 按需 UAC，完成即退出，无常驻提权进程 |
| 本地存储 | SQLite（rusqlite，bundled） | 单文件库，随应用数据目录分发，不依赖外部 DB 服务 |
| 规则库 | 纯 JSON | 托管 GitHub Raw，SHA-256 校验，不含可执行代码，不加签名 |
| 目标平台 | Windows 10 1809+ / Windows 11 | x64 + ARM64；**完全不做 macOS / Linux**，不建跨平台抽象层 |
| 构建产物 | NSIS 安装包 + 便携版 zip | `Cargo.lock` 必须入库，发布构建锁定依赖版本 |

### 1.3 进程模型（技术边界）

```
┌─────────────────────────────────────────────┐
│ 主进程（普通权限，常驻）                       │
│  ├─ Tauri WebView2（React 前端）             │
│  ├─ Rust 核心：扫描调度 / 分级 / 保险箱 / DB  │
│  └─ 兼容层宿主：白名单内调用 ps1（子进程）     │
└──────────────┬──────────────────────────────┘
               │ 仅写操作且需系统权限时
               ▼
┌─────────────────────────────────────────────┐
│ 提权辅助进程（独立 exe，UAC 触发，短生命周期）  │
│  DISM 调用 / 注册表写 / 还原点创建 / 系统目录删除 │
└─────────────────────────────────────────────┘
```

- 主进程**永远**以普通用户权限运行；扫描全程零提权。
- 提权辅助进程只接受结构化参数（白名单动作枚举 + 已通过白名单校验的路径列表），不接受任意命令字符串，启动即校验、执行即退出。
- 两个进程之间通过**命名管道 + JSON 消息**通信；提权进程 stdout/stderr 被主进程结构化捕获。

---

## 2. 技术目标与非功能需求细化

| 类别 | PRD 指标 | 技术细化要求 |
|------|----------|--------------|
| 扫描性能 | 全量扫描 ≤ 3 min（C 盘已用 100GB） | 目录遍历必须在工作线程池执行，UI 线程零阻塞；进度事件 ≤ 1 次/100ms 节流上报；AppData 递归采用 BFS + 跳过符号链接（`std::fs::symlink_metadata`），避免环路 |
| UI 响应 | ≤ 200ms | 前端任何 IPC 等待期间不得出现同步阻塞；长任务一律 `invoke` + 事件订阅模式 |
| 内存 | 空闲 ≤ 100MB，扫描峰值 ≤ 300MB | 单条清理项落库前不得全量驻留内存；大目录枚举流式写 SQLite；结果页虚拟滚动 |
| 安装包 | ≤ 20MB | NSIS 压缩；不打包 Chromium（用系统 WebView2）；不打包 PowerShell（用系统自带） |
| 安全 | L3 代码级拦截 | 执行引擎内置路径白名单匹配器，删除动作**唯一入口**为 `engine::delete::execute()`，所有删除（含 ps1 返回结果、CLI 传入路径）必须过同一白名单函数 |
| 隐私 | 无遥测 | 代码层禁止引入 analytics/sentry/广告 SDK；唯一出站 HTTP 请求域名白名单 = `raw.githubusercontent.com`（可设置关闭） |
| 可维护 | 规则库外置 JSON 热更 | 扫描引擎不得硬编码任何具体软件路径；路径/关键词/分级全部来自 `rules/*.json` |
| 可追溯 | 审计日志 | 任何写动作（移入保险箱、删除、恢复、注册表写、还原点创建）必须先写 `audit_log` 再执行，失败也要留痕 |
| 可移植 | 便携版配置随程序目录 | 通过环境变量 `CDISKCLEANER_PORTABLE=1` 切换数据根目录：安装版 = `%LOCALAPPDATA%\CDiskCleaner\`；便携版 = `<exe目录>\data\` |

---

## 3. 外部依赖清单

### 3.1 系统组件（Windows 自带，不随包分发）

| 依赖 | 用途 | 调用方式 | 失败兜底 |
|------|------|----------|----------|
| WebView2 Runtime（Evergreen） | 前端渲染 | Tauri 自动检测；缺失时引导用户安装 | 启动期检测，缺失则弹窗跳转微软下载页 |
| PowerShell 5.1 | M1 兼容层宿主 | 主进程 `CreateProcess` 调用 `powershell.exe -NoProfile -NonInteractive -File <script> -InputJson <...>` | M2+ 引擎 Rust 化后该依赖逐步摘除 |
| DISM（`Dism.exe`） | WinSxS 组件存储分析/清理 | 提权进程调用 `/online /Cleanup-Image /AnalyzeComponentStore` 与 `/StartComponentCleanup`；**禁止 `/ResetBase`**（参数硬编码白名单，未列参数拒绝拼装） | 解析退出码与日志尾部，按 PRD §6 报错 |
| 系统还原 / VSS | 清理前自动创建还原点 | 提权进程调用 `SRSetRestorePoint` / WMI `SystemRestore` | 失败按 PRD §6 询问用户 |
| Restart Manager API | 检测文件占用进程 | `RmStartSession` / `RmGetList` | 检测失败则该项降级为"占用状态未知"，不阻塞扫描 |
| 注册表 API（Advapi32） | 残留检测、Run 键扫描、`.reg` 导出 | Rust `winreg` crate；导出用 `reg export`（提权进程） | 导出失败即阻止删除（PRD §6） |
| 回收站 API | 回收站清理项计量 | `SHEmptyRecycleBin` / IShellFolder 枚举 | 权限不足按错误码处理 |
| WMI / CIM | 已卸载软件列表、休眠文件状态、计划任务 | `Get-CimInstance`（M1 ps1）→ M2+ `windows-rs` 直接调用 | 单源失败不阻断其他维度 |

### 3.2 第三方库（Rust / 前端，随包分发）

| 依赖 | 用途 | 许可证要求 |
|------|------|------------|
| `tauri = 2` | 桌面框架 | MIT/Apache-2.0 |
| `rusqlite`（bundled feature） | 本地数据库 | MIT |
| `serde` / `serde_json` | 全链路序列化 | MIT/Apache-2.0 |
| `sha2` | 规则库 checksum 校验 | MIT/Apache-2.0 |
| `winreg` / `windows`（windows-rs） | 注册表、系统 API 绑定 | MIT/Apache-2.0 |
| `walkdir` / `jwalk` | 目录枚举（并行） | MIT/Unlicense |
| pnpm + React 18 + TypeScript（构建期） | 前端 | MIT |
| NSIS（构建期） | 安装包 | zlib |

**禁止引入**：任何遥测/统计 SDK、广告 SDK、云端同步 SDK、非纯 JSON 的远端代码拉取机制。

### 3.3 远端服务

| 服务 | 用途 | 协议 | 白名单 |
|------|------|------|--------|
| GitHub Raw（`raw.githubusercontent.com/An0114/C-clean/<branch>/rules/`） | 规则库 JSON + `rules_meta.json` 拉取 | HTTPS GET，仅出站 | 设置中可完全关闭；关闭后不发任何请求 |

> 无自建服务端、无账号体系、无登录态。CLI 版与便携版同样遵守该网络白名单。

---

## 4. 数据字典 & 实体定义

本地存储为单个 SQLite 文件 `<数据根>\data\cdiskcleaner.db`，UTF-8，外键约束开启。所有时间字段为 Unix 毫秒（INTEGER）。

### 4.1 `settings` 应用设置表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| key | TEXT | PK | 配置键，如 `safebox.enabled` |
| value | TEXT | NOT NULL | JSON 编码值 |
| updated_at | INTEGER | NOT NULL | 最近修改时间 |

预置键：`safebox.enabled`(默认 true)、`safebox.quota_bytes`(默认 5368709120)、`restore_point.auto_create`(默认 true)、`rules.auto_update`(默认 true)、`theme`(system/light/dark)、`schedule.*`（频率/时间/阈值）。

### 4.2 `scan_session` 扫描会话表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PK AUTOINCREMENT | |
| started_at | INTEGER | NOT NULL | |
| finished_at | INTEGER | NULL | 取消/失败时也记录结束时间 |
| status | TEXT | NOT NULL | `running` / `finished` / `cancelled` / `failed` |
| scope | TEXT | NOT NULL | `full` / `custom` / `other_drive_readonly` |
| drive_used_bytes_before | INTEGER | NULL | 扫描起点 C 盘已用空间（做趋势用） |
| error_summary | TEXT | NULL | 被跳过/失败目录汇总 |

### 4.3 `cleanup_item` 清理项表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PK AUTOINCREMENT | |
| session_id | INTEGER | FK → scan_session.id ON DELETE CASCADE | |
| module_code | TEXT | NOT NULL | `A`/`B`/`C`/`D`/`E`/`F`（对应业务模块） |
| category | TEXT | NOT NULL | 规则库内类别键，如 `npm_cache` |
| path | TEXT | NOT NULL | 目标文件/目录绝对路径 |
| size_bytes | INTEGER | NOT NULL DEFAULT 0 | |
| risk_level | TEXT | NOT NULL | `L0`/`L1`/`L2`；**L3 禁止写入本表**（L3 仅在白名单器内部存在） |
| occupied_by | TEXT | NULL | 占用进程名列表（JSON 数组），无占用为 NULL |
| description | TEXT | NOT NULL | 来自规则库 |
| impact_note | TEXT | NULL | "删了会怎样" |
| rebuild_note | TEXT | NULL | 重建方式 |
| extra | TEXT | NULL | JSON：休眠文件/WinSxS/残留软件名等模块特有字段 |

### 4.4 `cleanup_plan` 清理方案/模板表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PK AUTOINCREMENT | |
| name | TEXT | NOT NULL | |
| note | TEXT | NULL | |
| is_template | INTEGER | NOT NULL DEFAULT 0 | 0=草稿/方案，1=可复用模板 |
| item_refs | TEXT | NOT NULL | JSON 数组：`{module_code, category, path}` 三元组（不绑定 session_id，保证模板跨次复用） |
| created_at / updated_at | INTEGER | NOT NULL | |

### 4.5 `safe_box` 保险箱索引表（决策 #3）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PK AUTOINCREMENT | |
| original_path | TEXT | NOT NULL | 删除前绝对路径 |
| safe_rel_path | TEXT | NOT NULL | 相对 `<数据根>\SafeBox\` 的存放路径（含重名防冲突前缀） |
| size_bytes | INTEGER | NOT NULL | |
| quarantined_at | INTEGER | NOT NULL | **淘汰排序键**（FIFO 按此字段升序） |
| expires_at | INTEGER | NOT NULL | `quarantined_at + 7*86400*1000` |
| status | TEXT | NOT NULL | `in_box` / `restored` / `purged` / `evicted` |
| source_task_id | INTEGER | FK → cleanup_task.id | |

执行前预检：`已用量 + 本次拟移入量 > quota` 时，先按 `quarantined_at` 升序淘汰 `status='in_box'` 的记录直至腾出空间，淘汰动作逐条写 `audit_log`。

### 4.6 `cleanup_task` 清理执行记录表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PK AUTOINCREMENT | |
| session_id | INTEGER | FK → scan_session.id | |
| started_at / finished_at | INTEGER | NOT NULL | |
| disk_before_bytes / disk_after_bytes | INTEGER | NOT NULL | C 盘剩余空间前后值 |
| freed_bytes | INTEGER | NOT NULL DEFAULT 0 | |
| success_count / fail_count / skip_count | INTEGER | NOT NULL DEFAULT 0 | |
| dry_run_snapshot | TEXT | NOT NULL | JSON：执行前用户确认的"将删除清单"快照（路径+大小），审计留证 |
| restore_point_name | TEXT | NULL | 本次创建的还原点名，未创建为 NULL |
| safebox_enabled | INTEGER | NOT NULL | 本次是否走保险箱 |

### 4.7 `audit_log` 审计日志表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PK AUTOINCREMENT | |
| ts | INTEGER | NOT NULL | |
| actor | TEXT | NOT NULL | `user` / `system` / `elevated` |
| action | TEXT | NOT NULL | `scan`/`quarantine`/`permanent_delete`/`restore`/`registry_delete`/`restorepoint_create`/`rules_update`/`whitelist_denied` |
| target | TEXT | NOT NULL | 路径/键路径/URL |
| detail | TEXT | NULL | JSON 补充信息 |
| result | TEXT | NOT NULL | `success` / `fail` / `denied` |
| denial_reason | TEXT | NULL | L3 白名单拒绝时必须填写匹配到的拦截规则名 |

### 4.8 `registry_backup` 注册表备份表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PK AUTOINCREMENT | |
| task_id | INTEGER | FK → cleanup_task.id ON DELETE SET NULL | |
| reg_path | TEXT | NOT NULL | 被删除的键/值路径 |
| export_file_path | TEXT | NOT NULL | `.reg` 备份文件绝对路径 |
| created_at | INTEGER | NOT NULL | |

### 4.9 `rules_meta` 规则库元数据表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | INTEGER | PK CHECK(id=1) | 单行表 |
| version | TEXT | NOT NULL | 语义化版本 |
| checksum_sha256 | TEXT | NOT NULL | `cache_rules.json` 内容的 SHA-256 |
| source_url | TEXT | NOT NULL | 实际拉取地址 |
| downloaded_at | INTEGER | NOT NULL | |
| custom_rules | TEXT | NOT NULL DEFAULT '[]' | JSON 数组：用户自定义目录规则（P2-2），结构同内置规则条目标记 `is_custom=true` |

### 4.10 规则库 JSON 结构（`rules/cache_rules.json`）

```jsonc
{
  "version": "1.0.0",
  "modules": {
    "A": [
      {
        "id": "npm_cache",
        "category": "npm_cache",
        "display_name": "npm 缓存",
        "paths": ["%LOCALAPPDATA%\\npm-cache", "~/.npm/_cacache"],
        "risk_level": "L1",
        "description": "npm 包下载缓存",
        "impact_note": "删除后下次安装依赖重新下载",
        "rebuild_note": "自动重建",
        "elevated": false
      }
    ]
  },
  "whitelist_deny": [
    "%USERPROFILE%\\Desktop",
    "%USERPROFILE%\\Documents",
    "C:\\Windows\\System32",
    "C:\\pagefile.sys"
  ]
}
```

- 路径变量仅允许白名单集合：`%LOCALAPPDATA%` / `%APPDATA%` / `%TEMP%` / `%PROGRAMDATA%` / `%USERPROFILE%` / `~`；规则加载时即展开并校验，不允许任意环境变量。
- `whitelist_deny` 即 L3 红线表，加载后与引擎内置硬编码红线**取并集**生效；规则更新不得移除引擎内置红线。

---

## 5. 接口通用规范（请求/响应格式、错误码）

本应用无 HTTP 后端。"接口"指三类：① Tauri IPC（前端 ↔ Rust 核心）；② 主进程 ↔ 提权进程命名管道消息；③ PowerShell 兼容层 / CLI 的 JSON 输出契约。

### 5.1 统一响应信封

所有接口返回同一信封：

```json
{ "code": 0, "message": "ok", "data": { } }
```

- `code = 0` 成功；非 0 见 §5.3 错误码。
- `data` 结构随命令定义；失败时 `data` 为 `null`，`message` 为可展示给用户的中文短句。
  - **唯一例外（task-004 口径回写，debt-cleanup 批次001）**：`code = 2002`（WHITELIST_DENIED）时 `data` **不置 null**，而是携带 `DryRunResp{totalItems:0,totalBytes:0,breakdown:[],blockedByL3:[...]}`——前端必须拿到「哪几条路径被拦、什么原因」才能展示；其余错误码（2001/5001 等）仍保持 `data=null`。
- 流式进度通过事件推送，不走返回值。

### 5.2 IPC 命令清单（前端 → Rust 核心）

| 命令 | 入参 | 返回 data | 说明 |
|------|------|-----------|------|
| `scan.start` | `{ scope: "full"\|"custom", targetDrive?: "C:" }` | `{ sessionId }` | 立即返回，进度走事件 |
| `scan.cancel` | `{ sessionId }` | `{ stopped: true }` | |
| `scan.listItems` | `{ sessionId, module?, risk?, keyword?, page, pageSize }` | `{ total, items[] }` | 结果分页查询 |
| `cleanup.dryRun` | `{ itemIds: number[] }` | `{ totalItems, totalBytes, breakdown[], blockedByL3: [] }` | 生成将删除清单；任何路径不通过白名单则整体拒绝 |
| `cleanup.execute` | `{ sessionId, itemIds, safebox: bool, createRestorePoint: bool }` | `{ taskId }` | 异步执行，进度走事件 |
| `safebox.list` | `{ page, pageSize }` | `{ total, totalBytes, quotaBytes, items[] }` | |
| `safebox.restore` | `{ ids: number[] }` | `{ restored[], failed[] }` | 恢复到原路径；原路径丢失时返回 `needPicker` |
| `safebox.purge` | `{ ids: number[] \| "all" }` | `{ purgedBytes }` | 永久删除，写审计 |
| `report.export` | `{ taskId, format: "html"\|"csv", savePath }` | `{ filePath }` | |
| `rules.checkUpdate` | `{}` | `{ hasUpdate, localVersion, remoteVersion }` | |
| `rules.applyUpdate` | `{}` | `{ version, checksum }` | 先校验 SHA-256 再落盘 |
| `settings.get` / `settings.set` | 同 §4.1 key/value | 当前值 | |
| `plan.save` / `plan.apply` / `plan.import` / `plan.export` | 同 §4.4 | 模板实体 | |
| `history.list` | `{ from?, to? }` | `{ tasks[], trend[] }` | 趋势 = C 盘剩余时间序列 |

事件（Rust → 前端）：

```
scan://progress   { sessionId, percent, currentPath, module }
scan://finished   { sessionId, cleanableBytes }
cleanup://progress { taskId, finishedCount, totalCount, currentPath }
cleanup://finished { taskId, freedBytes, success, fail, skip }
tray://notify     { title, body }
```

### 5.3 错误码表

| code | 常量 | 用户可见文案 |
|------|------|--------------|
| 0 | OK | —— |
| 4001 | PARAM_INVALID | 请求参数不合法 |
| 1001 | SCAN_BUSY | 已有扫描进行中 |
| 1002 | SCAN_CANCELLED | 扫描已取消 |
| 2001 | DRYRUN_EMPTY | 待清理清单为空 |
| 2002 | WHITELIST_DENIED | 检测到受保护路径，操作已拒绝（L3 红线） |
| 2003 | FILE_OCCUPIED | 文件被进程占用 |
| 2004 | ELEVATION_DENIED | 需要管理员权限，提权被取消 |
| 2005 | SAFEBOX_FULL | 保险箱空间不足且自动淘汰失败 |
| 2006 | REG_BACKUP_FAILED | 注册表备份失败，删除已取消 |
| 2007 | RESTOREPOINT_FAILED | 系统还原点创建失败 |
| 3001 | DISM_ERROR | 组件存储分析失败（附 DISM 原始输出） |
| 3002 | RULE_CHECKSUM_MISMATCH | 规则库校验失败，已保留旧版本 |
| 3003 | RULE_NETWORK_ERROR | 规则库更新失败，继续使用本地版本 |
| 5001 | INTERNAL | 内部错误（附日志 ID） |

### 5.4 提权进程消息契约

请求（命名管道，JSON 单行）：

```json
{ "action": "dism_clean" | "registry_delete" | "create_restorepoint" | "system_path_delete",
  "args": { /* action 对应的结构化参数 */ },
  "token": "<主进程签发的一次性随机串，5 分钟有效>" }
```

- `args.paths` 必须是主进程已通过白名单校验的列表；提权进程**二次校验**后才执行，不通过返回 code=2002。
- 响应同 §5.1 信封；进程执行完即退出，单会话单动作。

### 5.5 PowerShell 兼容层契约（M1）

- 入参：`-InputJson <结构化规则数组>`，禁止脚本内交互读入。
- 出参：stdout 为 NDJSON 流，每行一个对象：
  - `{"type":"item","module":"A","category":"npm_cache","path":"...","size_bytes":...,"risk_level":"L1",...}`
  - `{"type":"progress","percent":42,"current_path":"..."}`
  - `{"type":"done","summary":{"total_items":12,"total_bytes":...}}`
- 退出码：0 = 正常结束；非 0 = stderr 末行作为错误信息返回 code=5001。
- 脚本禁止外联网络、禁止自修改；输出字段缺失时按缺项处理，不得吞异常。

### 5.6 CLI 输出契约（P2-4）

- 子命令：`cdiskcleaner scan [--json] [--full]`、`cdiskcleaner clean --item-ids 1,2,3 [--dry-run] [--yes]`。
- 输出同 §5.1 信封；退出码：0 成功，2 用户拒绝/无操作，3 安全拦截（L3），4 提权被拒，1 内部错误。
- CLI 必须强制走：白名单校验 → DryRun（除非 `--yes` 且非 TTY 由脚本显式确认）→ 保险箱（除非显式 `--no-safebox`）。

---

## 6. 模块技术需求

按业务模块 A~F（立项决策 #7）定义技术要点；公共能力（扫描调度、保险箱、审计、IPC、SQLite schema）由主进程统一提供，业务模块只实现"检测 → 分级 → 执行"三段。

| 模块 | 检测输入 | 分级来源 | 执行路径 | 技术要点 |
|------|----------|----------|----------|----------|
| A 应用缓存 | 规则库路径展开 + `jwalk` 目录大小统计 | 规则条目 `risk_level` | 移入保险箱（普通权限） | 包管理器缓存关键词匹配；软链接跳过 |
| B 系统临时与日志 | 17 类固定系统路径 + WER/Minidump/更新缓存目录 | 固定 L1/L2 映射表 | 系统路径删除 → 提权进程 | 部分路径需提权，主进程先尝试普通权限，拒绝后再走提权 |
| C WinSxS | `Dism.exe /AnalyzeComponentStore` 输出解析 | 固定 L2 | 提权进程 `/StartComponentCleanup` | 参数拼装白名单硬编码，`/ResetBase` 字符串不存在于代码库 |
| D 已卸载残留 | 注册表卸载列表 ↔ 文件系统目录交叉比对 | 规则：空目录= L1；含配置= L2 | 移入保险箱 | 残留项 `extra` 存软件名/卸载时间 |
| E 系统级大文件 | 休眠文件 `hiberfil.sys`、`pagefile` 计量（只读）、`Windows.old`、回收站枚举；Top100 大文件扫描 | 个人目录内=L0 只读；休眠/Windows.old=L2 | 回收站清空、`Windows.old` 删除走提权；**大文件地图不提供删除入口** | 大文件枚举流式写库，前端虚拟滚动 |
| F 注册表健康 | Run 键、卸载残留键、计划任务指向不存在的 exe | 固定 L2 | 提权进程：先 `reg export` → 校验备份文件存在 → 再删键 | 备份失败即终止该条；逐项确认，无批量删除接口 |

公共横切要求：

- **DryRun**：`cleanup.dryRun` 与 `cleanup.execute` 必须基于同一份 `itemIds` 快照执行；execute 内部重新跑一遍白名单，与 dryRun 结果不一致则中止。
- **占用检测**：扫描完成后对全部 L1/L2 目标批量跑 Restart Manager，结果回填 `cleanup_item.occupied_by`；关键系统进程（explorer/svchost 等进程名黑名单）不提供关闭动作。
- **还原点**：仅当本次任务命中 L2/注册表/WinSxS 且开关开启时，在 execute 前置创建；失败按错误码 2007 回传。
- **崩溃恢复**：启动时查询 `cleanup_task.status='running'` 的记录，标记为 `interrupted` 并提示查看日志；保险箱文件不受影响。

---

## 7. 测试验收标准

### 7.1 单元测试（Rust `cargo test` + 前端 Vitest）

| 测试对象 | 通过标准 |
|----------|----------|
| 路径白名单匹配器 | 对 PRD §P0-7 列出的全部 L3 路径（个人目录、pagefile/swapfile、System32、Installer、浏览器 User Data）输入 `execute()`，100% 返回 code=2002，零例外 |
| 风险分级引擎 | 不确定项一律降级为 L2；规则缺失时不崩溃、记 warning |
| 保险箱 FIFO 淘汰 | 构造 quota=100MB、依次移入 5 个文件，断言按 `quarantined_at` 升序淘汰且 `audit_log` 逐条留痕 |
| checksum 校验 | 篡改 `cache_rules.json` 任意字节 → 加载拒绝且旧版本仍可用 |
| 信封/错误码 | 全部 IPC 命令异常路径返回 §5.3 规定码，不抛未捕获 panic 到前端 |

### 7.2 集成测试（干净 Windows VM 快照上执行）

1. 闭环：scan → 勾选 L1 → dryRun → execute → 报告释放量与磁盘实际差值误差 ≤ 5%。
2. 恢复闭环：清理后立即从保险箱恢复被删文件，断言路径/内容/大小一致。
3. 占用场景：预置一个被记事本打开的文件，断言标记 `occupied_by`、提供"关闭/跳过"两条路径。
4. 提权拒绝：UAC 弹窗点"否"，断言相关项落为 code=2004 失败，其他项正常完成。
5. 注册表闭环：删一个无效 Run 键，断言 `.reg` 备份文件存在且 `reg import` 可还原。
6. 保险箱满：quota 压测到满，断言自动淘汰最旧文件且新文件成功入箱。

### 7.3 性能基准（验收硬指标）

| 指标 | 环境 | 阈值 |
|------|------|------|
| 全量扫描耗时 | Win11 x64，C 盘已用 100GB，机械+SSD 混合样本 | ≤ 180s |
| UI 帧率 | 扫描进行中切换页面 | 掉帧 ≤ 1 次/10s，无白屏 |
| 空闲 RSS | 托盘常驻 | ≤ 100MB |
| 扫描峰值 RSS | 同上 | ≤ 300MB |
| 安装包体积 | NSIS 产物 | ≤ 20MB |

### 7.4 兼容性与安全测试

- Win10 1809 / Win11、x64 / ARM64 四组合各至少一次冒烟（扫描+清理 L1）。
- 网络断开时全功能可用；规则更新开关关闭时抓包确认零出站请求。
- 模拟杀软拦截：校验错误文案与放行指引可达。
- 报告导出 HTML/CSV 抽样人工核对与 `cleanup_task` 明细一致。

---

## 8. 部署约束

| 项 | 约束 |
|----|------|
| 数据目录 | 安装版：`%LOCALAPPDATA%\CDiskCleaner\`（db / SafeBox / logs / reg backups）；便携版：exe 同级 `data\` 目录（`CDISKCLEANER_PORTABLE=1`） |
| 安装包 | NSIS，per-user 默认安装，不写 HKLM；仅当用户选择"所有用户"时才要求管理员 |
| 提权模型 | 提权 exe 独立签名内构建；单次触发、执行即退；主进程不长期持有高令牌 |
| 代码签名 | 不购买 EV 证书（决策 #5）；发布说明中说明 SmartScreen 首次警告；发布前必须完成 Defender / 火绒 / 卡巴斯基白名单提交 |
| 构建可复现 | `Cargo.lock` 入库；CI 固定 Rust 版本与 pnpm 版本；产物同时出 x64 / ARM64 两套 |
| 托盘常驻 | 关闭窗口 = 最小化托盘；退出需二次确认；无开机自启默认项（用户可在设置开启） |
| 规则分发 | 规则 PR 合并到 main 后经 GitHub Raw 生效；拉取走 release tag 指向的不可变 URL，不用分支浮动 URL |
| 日志 | 本地滚动日志 `<数据根>\logs\`，保留最近 7 天；不上传 |
| 卸载 | 卸载程序删除程序文件与配置；保险箱数据**默认保留**并在卸载页提示用户可选择一并删除 |

---

## 9. Git 提交记录

| Commit Hash | 提交信息 | 涉及文件 |
|-------------|----------|----------|
| `2bb4e76` | docs(trd): 新增技术需求文档 v0.1 (#task-000) | docs/TRD.md |
