-- task-086 5.5：增量扫描缓存。
--
-- 语义：以「模块 + 扫描根目录 + 根 mtime」为签名缓存上次的条目清单。
-- 下次扫描时签名未变 → 直接复用，不再遍历该目录；变了 → 重新扫描并覆盖。
-- 条目以 JSON 存储（ScanItem 的序列化形态），避免为缓存再建关联表。
CREATE TABLE IF NOT EXISTS scan_cache (
    module       TEXT    PRIMARY KEY,   -- 模块代码（temp / prefetch / npm_cache ...）
    signature    TEXT    NOT NULL,      -- 根目录路径 + mtime 的签名
    items_json   TEXT    NOT NULL,      -- 上次扫描的 ScanItem 列表
    updated_at   INTEGER NOT NULL       -- 写入时间（unix 秒）
);

-- task-086 5.8：通知中心历史。
--
-- Windows 通知中心只保留系统 toast；用户在系统里清掉后应用内就查不到了。
-- 这张表让「清理完成 / 自动扫描」的提醒在应用内可回看。
CREATE TABLE IF NOT EXISTS notification_history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    ts          INTEGER NOT NULL,          -- 写入时间（unix 秒）
    title       TEXT    NOT NULL,
    body        TEXT    NOT NULL,
    read        INTEGER NOT NULL DEFAULT 0 -- 0 未读 / 1 已读
);
CREATE INDEX IF NOT EXISTS idx_notification_history_ts
    ON notification_history(ts DESC);
