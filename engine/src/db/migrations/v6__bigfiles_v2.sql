-- task-084 A1/A2：大文件地图 v2 —— 目标空间 + 重复文件缓存表。
--
-- 大文件地图（task-066）原本只量 hiberfil / Windows.old / 回收站 / Top100，且全部
-- 纯内存经 IPC 返回。task-084 在基础之上新增「目标空间（Downloads/Documents/Desktop/Temp）」
-- 与「重复文件检测」，这两类结果量较大且适合跨会话复用，落一张缓存表给后续查询/UI 恢复用。
--
-- 本表是**只读扫描结果的缓存**，不参与扫描语义；`detected_at` 记录本次扫描时间戳，
-- 重复扫描用「同组 + 同 detected_at」区分快照。
CREATE TABLE IF NOT EXISTS bigfiles_target_space (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- 目标空间类别键（target_downloads / target_documents / target_desktop / target_temp）。
    category TEXT NOT NULL,
    -- 命中大文件的绝对路径。
    path TEXT NOT NULL,
    -- 占用字节数。
    size_bytes INTEGER NOT NULL,
    -- 处理建议（前端按类别展示，可空）。
    recommendation TEXT,
    -- 盘符（task-084 B3 多盘扩展；C 盘为 NULL 表示盘符未录，保持旧语义）。
    drive_letter TEXT,
    detected_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_bigfiles_ts_category ON bigfiles_target_space(category);

-- 重复文件分组：同一 group_id = 同一 SHA-256（内容相同）的一组文件。
CREATE TABLE IF NOT EXISTS duplicate_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- 内容哈希（SHA-256 十六进制，前 16 位做分组号即可足够区分）。
    group_id TEXT NOT NULL,
    -- 文件绝对路径。
    path TEXT NOT NULL,
    -- 文件字节数（同组文件内容相同，大小必然相等）。
    size_bytes INTEGER NOT NULL,
    -- 最后修改时间（ms 时间戳）；每组保留 modified_ms 最大的一份（is_keep=1）。
    modified_ms INTEGER NOT NULL,
    -- 1 = 建议保留（最新修改时间），0 = 建议可清理。保守策略，**不自动删**。
    is_keep INTEGER NOT NULL DEFAULT 0,
    drive_letter TEXT,
    detected_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_duplicate_group ON duplicate_files(group_id);