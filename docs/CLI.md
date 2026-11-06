# 命令行（CLI）与便携版

> 适用版本：v0.4.x 起。GUI 用法见 [USER_GUIDE.md](./USER_GUIDE.md)。

C-Disk Cleaner 有两个入口，共用同一套 `engine`：

| 入口 | 形态 | 适合 |
|---|---|---|
| **GUI** | 安装版（NSIS / MSI） | 日常使用，看得见列表、能逐项确认 |
| **CLI** | `c-clean-cli.exe` | 脚本 / 定时任务 / 服务器 / 不想开窗口 |

两者**共用同一份数据库**（默认 `%APPDATA%\com.and0114.cclean`），所以 CLI 扫出来的
条目，GUI 打开就能看到并继续处理；反过来也一样。便携版则是两者都把数据放在 exe 旁边。

---

## 一、拿到 CLI

### 方式 A：从 Release 下载独立 CLI

每个 Release 都带 `c-clean-cli.exe`（单文件，无安装、无依赖）。放到任意目录即可：

```bat
c-clean-cli.exe --help
```

### 方式 B：用便携版 zip（GUI + CLI 一体）

Release 里另有 `C-DiskCleaner_<版本>_portable_x64.zip`。解压后结构是：

```text
C-DiskCleaner_0.4.0_portable/
├── cdisk-cleaner.exe          ← GUI，双击即用
├── c-clean-cli.exe            ← CLI
├── cclean-elevation.exe       ← 提权助手（系统还原点 / DISM 用）
├── rules/                     ← 出厂规则库
├── scripts/                   ← ps1 回退脚本（正常用不到）
└── portable_data/             ← 数据目录（数据库 / 保险箱 / 日志）
    └── README.txt
```

**解压即用，不需要安装，不写 `%APPDATA%`。** 数据全在 `portable_data/`，想搬走就连
整个文件夹一起拷。

> 便携版**不要放在只读位置**（如 `Program Files`）。它的设计前提就是「exe 旁边的
> 目录可写」；放在只读目录里，数据库会创建失败。

---

## 二、子命令

```text
c-clean-cli <子命令> [选项]

  scan          扫描并列出可清理项（摘要 + 前 N 条）
  dry-run       扫描并列出全部条目（不删除）
  clean         扫描 + 清理；默认只预览，加 --yes 才真的移入保险箱
  list-rules    列出内置规则与用户自定义规则
  help          显示帮助
```

### `scan` —— 看一眼能清多少

```bat
c-clean-cli scan
```

```text
共 5870 项，合计 334.9 MB

大小     风险  模块            路径
-------  ----  --------------  ------------------------------------------
13.0 MB  L2    windows_update  C:\WINDOWS\SoftwareDistribution\...
12.9 MB  L1    browser_cache   C:\Users\...\cache2\entries\9AE3ADAB...
...
（只显示前 50 条，加 --limit 5870 或改用 dry-run 看全部）
```

### `dry-run` —— 看全部明细

```bat
c-clean-cli dry-run
```

与 `scan` 同一次扫描，只是**不受 `--limit` 限制**、把所有条目都打出来。
适合「我到底要删什么」这种复核场景。

### `clean` —— 清理（默认只预览）

```bat
c-clean-cli clean                 :: 只预览，不删任何文件
c-clean-cli clean --yes           :: 真的清（只清 L1 安全项）
c-clean-cli clean --yes --include-l2
```

三条安全设计，与 GUI 完全一致：

1. **默认是预览**。不加 `--yes` 只打印计划，一行都不会删；
2. **默认只清 L1**（安全缓存：临时文件、浏览器缓存、包管理器缓存…）。L2 是「需逐项
   确认」的等级，命令行里没有逐项确认的界面，所以要额外显式 `--include-l2`；
3. **删除 = 移入保险箱**，不是直接抹掉。文件进 `<数据目录>\safe_box\`，在 GUI 的
   「保险箱」页可以逐条恢复。

> ⚠️ **`--include-l2` 不创建系统还原点。** 还原点是 GUI 对 L2 批次的交互流程
> （要提权、要弹 UAC 确认），命令行里没法弹窗。所以这条路径下**没有整体回滚**，
> 只有保险箱的逐条恢复。真要清理 L2，建议用 GUI。

### `list-rules` —— 规则从哪来

```bat
c-clean-cli list-rules
c-clean-cli list-rules --format json
```

```text
内置规则库 v1.0.0（1 条）
  [A] npm_cache  (L1)

用户自定义规则（2 条）
  #1   我的游戏缓存          D:\Games\Cache\*                 L1 启用
  #2   旧日志               D:\Logs\**\*.log                 L2 停用
```

内置规则随版本下发（受 SHA-256 保护）；用户规则存在本地数据库里，在 GUI 的
「设置 → 自定义规则」里增删改，**不会被规则更新覆盖**。

---

## 三、选项

| 选项 | 说明 |
|---|---|
| `--format <table\|json>` | 输出格式，默认 `table` |
| `--drive <C,D>` | 只扫指定盘（逗号分隔）。默认扫全部**固定盘**（U 盘 / 网络盘不会进列表） |
| `--data-dir <PATH>` | 数据目录，见下节 |
| `--limit <N>` | `scan` / `clean` 预览的打印条数上限，默认 50 |
| `--yes` | `clean` 时真的执行 |
| `--include-l2` | `clean` 时连 L2 一起清 |
| `-h` / `--help` | 帮助 |
| `-V` / `--version` | 版本 |

盘符写 `C` 或 `C:` 都行，大小写不敏感。`--drive C` 时，根目录不在 C 盘的模块会被
整块跳过（比如 Prefetch 在系统盘，只选 D 盘就不会扫它）。

---

## 四、数据目录怎么定

优先级从高到低：

1. `--data-dir <PATH>`
2. 环境变量 `CCLEAN_DATA_DIR`
3. **`<exe 所在目录>\portable_data`** —— 存在就用（便携版就是这个）
4. `%APPDATA%\com.and0114.cclean` —— 与 GUI 安装版同一个库

想跑一次「不影响正式数据」的试扫，可以指定临时目录：

```bat
c-clean-cli scan --data-dir D:\tmp\cclean-trial
```

---

## 五、JSON 输出（给脚本用）

加 `--format json`，输出单个 JSON 对象到 stdout。**日志与警告一律走 stderr**，
所以 `> out.json` 拿到的永远是干净 JSON。

### `scan` / `dry-run`

```json
{
  "totalItems": 5870,
  "totalBytes": 351211583,
  "shown": 5870,
  "items": [
    {
      "path": "C:\\Users\\...\\cache2\\entries\\9AE3ADAB...",
      "sizeBytes": 13590379,
      "module": "browser_cache",
      "riskLevel": "L1"
    }
  ]
}
```

`totalItems` 是**本次会话的全部条目数**，`shown` 是实际打印的条数（受 `--limit` 影响）。

### `clean`

预览（无 `--yes`）：

```json
{
  "dryRun": true,
  "sessionId": 12,
  "wouldCleanItems": 5828,
  "wouldCleanBytes": 311812096,
  "skippedL2": 41,
  "items": [ { "path": "...", "sizeBytes": 1024, "riskLevel": "L1" } ]
}
```

真清（`--yes`）：

```json
{
  "dryRun": false,
  "sessionId": 13,
  "deletedCount": 5828,
  "failedCount": 0,
  "alreadyGoneCount": 3,
  "bytesFreedTotal": 311812096,
  "safeBoxDir": "C:\\Users\\...\\com.and0114.cclean\\safe_box",
  "failures": []
}
```

`alreadyGoneCount` 是「扫描后、清理前已被系统或别的程序删掉」的条目数——既不算成功
也不算失败，单独计数（否则会出现「清了 3 项，释放 0 B」这种看着像 bug 的数字）。

### `list-rules`

```json
{
  "rulesVersion": "1.0.0",
  "builtin": [ { "module": "A", "id": "npm_cache", "riskLevel": "L1" } ],
  "user": [ { "id": 1, "name": "我的缓存", "pattern": "D:\\MyCache\\*",
              "riskLevel": "L1", "enabled": true } ]
}
```

---

## 六、退出码

| 码 | 含义 | 脚本里怎么用 |
|---|---|---|
| `0` | 成功 | 注意：`clean` 有失败条目**也**返回 0，失败明细在 `failures` 里 |
| `1` | 运行期错误（DB 打不开、扫描失败…） | 当错误处理 |
| `2` | 用法错误（未知子命令 / 开关 / 缺参数值） | 说明脚本自己写错了，不是环境问题 |

用法错误单独给 `2`，是为了让脚本能区分「我参数写错了」和「跑起来但失败了」。

---

## 七、典型脚本

每天凌晨扫一次，条目数超阈值才提醒：

```bat
@echo off
c-clean-cli scan --format json > "%TEMP%\cclean.json"
for /f %%i in ('powershell -NoProfile -Command "(Get-Content '%TEMP%\cclean.json' -Raw | ConvertFrom-Json).totalBytes"') do set BYTES=%%i
if %BYTES% GTR 10737418240 echo 可清理超过 10 GB，建议清理
```

只清 C 盘的 L1 安全项：

```bat
c-clean-cli clean --yes --drive C
```

---

## 八、已知限制

- **CLI 不创建系统还原点**（见上文 `--include-l2` 的警告）；
- **CLI 没有「关闭占用进程并重试」**（那是 GUI 的 `cleanup_retry_locked`）；被占用的
  文件会出现在 `failures` 里，关掉占用程序后重跑即可；
- **`clean` 一次最多处理 50 万条**（内部翻页上限），超出会打警告并截断；
- **非系统盘默认没有可清理目标**：5 个内置模块的根目录都在用户目录 / 系统目录所在盘，
  所以 `--drive D` 很可能是 0 项。要让某盘有产出，用「设置 → 自定义规则」加一条指向
  该盘的规则；
- **`prefetch` 模块在普通权限下恒为 0 项**：`C:\Windows\Prefetch` 的 ACL 只给
  管理员读。这是设计如此，不是故障。
