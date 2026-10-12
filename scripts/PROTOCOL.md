# ps1 → Rust NDJSON 行输出协议

> 用途：Rust 主进程（M1-A 扫描器）spawn 各 `scripts/scan_*.ps1` 子进程，**逐行**读取其 stdout，
> 边解析边写入 `cleanup_item` 表，并向前端 emit `scan://progress`。本协议定义每行 JSON 的 schema。
> ps1 只**扫描发现**可清理项，**不删除**（删除走 `engine::execute::delete` 唯一入口）。

## 总规则

1. **stdout 只输出 NDJSON**：一行一个 JSON 对象，禁止任何人类可读日志；日志一律走 stderr。
2. 编码：**UTF-8 无 BOM**（脚本内 `[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)`）。
3. 行分隔：PowerShell 默认 `\r\n` 收尾；Rust 侧按 `\n` split、trim `\r`，两种换行都能被 `ConvertFrom-Json` 接受。
4. **字段缺失禁止**：任何一行都必须包含下文列出的全部字段，不得输出缺字段的行。
5. 错误处理：遇到权限不足的子目录 → 跳过并写 stderr，**不**向 stdout 输出错误行。
6. 不输出 L3 红线事件（`risk_level=3`）：L3 目录（Desktop/Documents/Downloads/Pictures/浏览器 Profile/System32 等）
   由 Rust 侧 task-003 白名单二次过滤；ps1 即使扫到也跳过。

## 事件 1：item（发现一个可清理项，每行一个）

```json
{"type":"item","module_code":"temp","category":"user_temp","path":"C:\\Users\\an\\AppData\\Local\\Temp\\foo.tmp","size_bytes":12345,"risk_level":1}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `type` | string | 恒为 `"item"` |
| `module_code` | string | `temp` / `windows_update` / `prefetch` / `browser_cache` / `dev_cache` 之一 |
| `category` | string | 脚本内部分类键，如 `user_temp` / `windows_temp` / `update_download` / `prefetch` / `edge_cache` / `chrome_cache` / `npm_cache` / `pip_cache` / `pnpm_store` |
| `path` | string | 绝对路径，JSON 内反斜杠双转义（`C:\\...`） |
| `size_bytes` | int | 文件字节数；目录递归统计（本兼容层只输出文件，目录本身不输出） |
| `risk_level` | int | `1`=L1 低风险可删 / `2`=L2 中风险需确认；**本协议不输出 3** |

字段一一对应 `cleanup_item` 表：`module_code / category / path / size_bytes / risk_level`。

## 事件 2：done（脚本最后一行，恰好一行）

```json
{"type":"done","module_code":"temp","total_items":42,"total_bytes":1234567}
```

| 字段 | 类型 | 说明 |
|---|---|---|
| `type` | string | 恒为 `"done"` |
| `module_code` | string | 与本脚本的 module_code 一致 |
| `total_items` | int | 本脚本输出的 item 行数 |
| `total_bytes` | int | 所有 item 的 size_bytes 之和 |

## 消费侧契约（M1-A 实现时遵守）

- 读到 `type=item` → 直接入 `cleanup_item`（Rust 侧再过 task-003 白名单 + task-008 真实路径解析）。
- 读到 `type=done` → 本脚本结束；`total_items/total_bytes` 用于对账（与实际入条数比对）。
- 子进程退出码非 0 → 整脚本标记为失败，已读 item 仍可保留但需在 audit_log 留痕。
