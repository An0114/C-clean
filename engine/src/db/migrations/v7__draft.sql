-- task-084 C2：草稿暂存表。
--
-- 用户勾选待清理项后，切换 tab / 关闭应用会丢选择（task 书 §C2 现状）。
-- 本表把「用户勾选了哪些清理项」持久化下来：
--
--   - 勾选时**即时写入**（前端每变更一次就 upsert）；
--   - 重新打开 app 时**自动恢复**上次勾选；
--   - 清理完成 / 手动取消后**清空**。
--
-- 为什么按路径而不是按 id 存：
--
-- 1. **id 会变**：cleanup_item.id 每次扫描都是自增，重扫/重开 app 后同一文件
--    的 id 完全不一样，按 id 存恢复不了；
-- 2. **路径是稳定的**：同一个文件在下次扫描里路径不变，重开后拿路径回查当前
--    库就能重新定位。
--
-- 为什么存「当前行的完整勾选快照」而不是增量日志：草稿的语义就是「用户此刻
-- 选了什么」的瞬态，每次勾选变更都重写整个集合（few-hundred 行级别，写放性能
-- 完全够），比记录 add/remove 增量简单、也永远不会出现「增量没对上」的漂移。
--
-- item_path 加唯一约束：同一路径只存在一行（upsert），避免重复勾选产生脏行。
CREATE TABLE IF NOT EXISTS draft_selections (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    -- 勾选发生时的扫描会话 id（可为空：勾选本身不依赖某个 session）。
    scan_id     INTEGER,
    -- 被勾选清理项的 absolute path（跨扫描稳定，恢复时回查当前库的依据）。
    item_path   TEXT NOT NULL,
    -- 勾选/更新时间（unix 毫秒），排序用。
    selected_at INTEGER NOT NULL
);
-- 同一路径只保留一行（upsert 语义）。
CREATE UNIQUE INDEX IF NOT EXISTS idx_draft_item_path ON draft_selections(item_path);
-- 按勾选时间排序（恢复时保持用户勾选顺序，前端能按序展示）。
CREATE INDEX IF NOT EXISTS idx_draft_selected_at ON draft_selections(selected_at);