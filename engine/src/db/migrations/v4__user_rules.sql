-- task-078：用户自定义清理规则表。
--
-- ============================ 为什么另开一张表，而不是写进 rules_meta.custom_rules ============================
-- `rules_meta.json` 里本来就有一个 `custom_rules` 字段（task-005 留的占位，注释写
-- 「本任务不展开」）。看起来直接往里写最省事，但**不能**：
--
--   1. `cache_rules.json` 受 SHA-256 校验保护（`rules_meta.checksum_sha256`），
--      而 `rules_meta.json` 与它是一对。用户增删规则要改文件 → 要么改 checksum
--      （等于把「防篡改」这道闸自己拆了），要么每次校验失败；
--   2. 规则库更新（task-023 `rules.applyUpdate`）会用远端整份覆盖本地文件，
--      用户的规则会被**静默冲掉**；
--   3. 回滚（task-058）同样整份换文件。
--
-- 一句话：`rules/` 是「厂商下发的、只读的、受签名保护的」那一层，
-- 用户规则是「本地可变的」那一层，物理上就该分开。DB 表正是后者。
--
-- ============================ 字段说明 ============================
-- `pattern`：路径模式，支持 `*`（本段内任意字符）/ `**`（跨目录）/ `?`（单个字符），
--   可含 `%ENV%` 与 `~`（与内置规则同一套展开器 `rules::env_expand`）。
--   例：`D:\MyCache\*`、`%LOCALAPPDATA%\Foo\**\*.tmp`。
--
-- `risk_level`：**只允许 L1 / L2**。
--   L3 在本项目里的语义是「红线，永不删除」（见 v1__init.sql 的
--   `cleanup_item` 表注释：「L3 红线项永不写入本表」）。所以「一条 L3 的清理
--   规则」本身就是自相矛盾的：它会扫出永远删不掉的条目，用户看到一堆勾不动
--   的东西。CHECK 约束在库层面直接挡死，比在 IPC 层校验更硬——绕过 IPC 直连
--   DB 也写不进去。
--
-- `created_at` / `updated_at`：**INTEGER unix 秒**，不是任务书写的 TEXT。
--   本库所有时间列都是 INTEGER（`scan_session.started_at`、
--   `safe_box.quarantined_at`、`audit_log.ts`），前端的 `formatTs(unixSec: number)`
--   也只吃数字。用 TEXT 会让这一列无法直接喂给既有的时间格式化函数，
--   而「同一张库里两种时间表示」是最容易出错的组合。
CREATE TABLE IF NOT EXISTS user_rules (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,                 -- 规则名（用户可见，≤ 60 字符由 IPC 校验）
    pattern     TEXT    NOT NULL,                 -- 路径模式（glob，可含 %ENV% / ~）
    risk_level  TEXT    NOT NULL DEFAULT 'L1'
                CHECK (risk_level IN ('L1', 'L2')),  -- L3 = 红线，见上
    enabled     INTEGER NOT NULL DEFAULT 1
                CHECK (enabled IN (0, 1)),
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

-- 扫描时只取 `enabled = 1` 的规则，索引就建在它上面。
-- 表通常只有几十行，这个索引严格来说可有可无——但它把「扫描路径只读启用规则」
-- 这个意图写进了 schema，也让将来规则变多时不用临时加索引。
CREATE INDEX IF NOT EXISTS idx_user_rules_enabled ON user_rules (enabled);

-- 同一个 pattern 不许重复建两条：重复规则会让同一批文件被扫两遍（条目翻倍、
-- 大小重复计算），而用户完全看不出原因。名字允许重复（不同目录的规则可以同名）。
CREATE UNIQUE INDEX IF NOT EXISTS idx_user_rules_pattern ON user_rules (pattern);
