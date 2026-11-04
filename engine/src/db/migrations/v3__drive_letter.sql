-- task-077：cleanup_item 增加 drive_letter（盘符）派生列 + 索引。
--
-- ============================ 为什么是 v3 不是 v2 ============================
-- 任务书写的是 `v2__drive_letter.sql`，但 **v2 已经被 task-056 占用了**
-- （`v2__safe_box_restored_at.sql`，给 safe_box 加 restored_at）。
-- `migrate.rs` 的 MIGRATIONS 表按版本号升序执行、并以 `PRAGMA user_version`
-- 记录进度，两个 v2 会导致「后一个永远不执行」（版本号 > current 的判断直接跳过）。
-- 所以本迁移是 **v3**。
--
-- ============================ 为什么是生成列 ============================
-- 任务书说「写入 items 时自动提取盘符」，实现上有两条路：
--
--   A. 在写入路径（`scan/runner.rs::insert_item`）里算好再 INSERT；
--   B. 建一个从 path 派生的生成列，写不写它都自动算。
--
-- 选 B，三个理由：
--   1. **单一真值源**：盘符本来就完全由 path 决定，存一份独立的副本只会漂
--      （改路径忘了改盘符，行就永久错了）；生成列在物理上不可能与 path 不一致。
--   2. **不碰写入路径**：`insert_item` 是 ps1 路径与 Rust 路径**共用**的唯一入库
--      入口（task-074 立的规矩）。改它要同时验证两条路径，而本任务不需要。
--   3. **两条路径自动覆盖**：ps1 回退路径的条目也会被正确标注，不需要 ps1 知道
--      盘符这回事。
--
-- 必须是 **VIRTUAL** 而不是 STORED：SQLite 的 `ALTER TABLE ADD COLUMN` 只允许
-- 加虚拟生成列（STORED 只能在建表时指定）。VIRTUAL 不占存储、只在读取时计算，
-- 对本表这种「一次扫描几千行、查询按 session 过滤」的访问模式完全够用。
--
-- ============================ 表达式为什么这么写 ============================
-- 认三种形态，其余一律 NULL：
--   `C:\Users\...`      → 'C'   （substr(path,2,1)=':'）
--   `c:/users/...`      → 'C'   （upper 归一化，大小写不分叉）
--   `\\?\C:\Windows\...` → 'C'  （canonicalize 的长路径前缀，先跳过 4 个字符）
--   `\\server\share\...` → NULL （UNC 没有盘符，硬套会得到 '\' 这种垃圾）
--   `relative\dir`      → NULL
--
-- **与 Rust 侧 `scan::drive_enum::drive_letter_of` 必须逐条对齐**——两处判定
-- 不一致会出现「Rust 过滤时认为在 D 盘、DB 里记成 C 盘」这种最难查的偏差。
-- 那边的单测 `drive_letter_of_strips_long_path_prefix` / `..._rejects_non_drive_paths`
-- 与本表达式一一对应，改动任一处都要同步另一处。
ALTER TABLE cleanup_item ADD COLUMN drive_letter TEXT
GENERATED ALWAYS AS (
    CASE
        WHEN substr(path, 2, 1) = ':' THEN upper(substr(path, 1, 1))
        WHEN substr(path, 1, 4) = '\\?\' AND substr(path, 6, 1) = ':'
            THEN upper(substr(path, 5, 1))
        ELSE NULL
    END
) VIRTUAL;

-- 索引带上 session_id 在前：本表**所有**业务查询都以 session 为第一过滤条件
-- （`WHERE session_id = ?`），单独按 drive_letter 建索引在这个访问模式下用不上
-- ——SQLite 只能用「最左前缀」，而 session_id 永远在最左边。
--
-- 索引名用 `idx_cleanup_item_drive_letter` 而不是任务书写的 `idx_items_drive_letter`：
-- 本仓库没有 `items` 表，表名是 `cleanup_item`，索引名跟着表名走才不会让后来人
-- 去 `items` 里找。
CREATE INDEX IF NOT EXISTS idx_cleanup_item_drive_letter
    ON cleanup_item (session_id, drive_letter);
