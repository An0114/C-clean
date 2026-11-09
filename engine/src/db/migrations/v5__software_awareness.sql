-- task-083：软件清单缓存表（B1 软件枚举器的落地 + C2 UI 软件列的数据底座）。
--
-- ============================ 为什么需要一张缓存表 ============================
-- B1 枚举器 `scan::software_enum` 每次是「现场读注册表 Uninstall 键」——快、
-- 实时（软件卸载后下次调用立刻反映），但它有代价：
--   1. 注册表枚举是系统调用，Items 页每次拉列表都做一遍很浪费；
--   2. 软件清单不是「清理项」，没有会话归属，不该被 scan_session 的
--      生命周期管辖；它需要一张独立于扫描会话的表来长期缓存。
--
-- 这张表就是那一层「缓存」。写方：B1 枚举完成后落一次（HKLM 优先去重后的
-- 结果）；读方：UI 软件列 / 折叠分组直接查表，不需要每次碰注册表。
-- `software_inventory` 命令当前按 task-083 设计以「只读现场枚举」交付（不写
-- DB、不依赖 AppPaths），本表作为后续缓存的地基先落 schema——如果哪天要缓存，
-- 写方只需要一条 INSERT OR REPLACE，读方接口不变。
--
-- ============================ 字段说明 ============================
-- 与 `scan::software_enum::SoftwareEntry` 字段一一对应，保证 Rust 侧直接
-- INSERT 时无需字段改名：
--   - `name`：DisplayName（软件名，去重时用作展示主键）；
--   - `version`：DisplayVersion；
--   - `publisher`：Publisher；
--   - `install_location`：InstallLocation（可为空，很多软件不写）；
--   - `registry_key`：来源注册表键路径（HKLM/HKCU + Uninstall\ 下的子键），
--     是「同一软件的原始出处」的唯一锚点。
--
-- `registry_key` 做唯一索引：HKLM 优先去重后，同一个 Uninstall 子键只允许一条
-- 记录；同一软件 HKLM + HKCU 都在时，刷新写方应保留 HKLM 那条并覆盖 HKCU 那条。
--
-- 时间列为 INTEGER unix 秒，与库内其余时间列口径一致（v4 注释已说明）。
CREATE TABLE IF NOT EXISTS software_inventory (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    name             TEXT    NOT NULL,                 -- DisplayName
    version          TEXT,                             -- DisplayVersion，可空
    publisher        TEXT,                             -- Publisher，可空
    install_location TEXT,                             -- InstallLocation，可空
    registry_key     TEXT,                             -- 来源 Uninstall 子键路径，可空
    detected_at      INTEGER NOT NULL                  -- 上次枚举时间（unix 秒）
);

-- 软件清单是「按名称给 UI 分组折叠」的（C2），索引建在 name 上，
-- 分组查询 `SELECT ... ORDER BY name` 走这里；registry_key 唯一锁住去重口径。
CREATE INDEX IF NOT EXISTS idx_software_inventory_name ON software_inventory (name);
CREATE UNIQUE INDEX IF NOT EXISTS idx_software_inventory_registry_key
    ON software_inventory (registry_key);