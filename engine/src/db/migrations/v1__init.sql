-- v1__init.sql：C-Disk Cleaner 初始 schema（task-002）
-- 字段与类型严格对照 TRD §4 数据字典 / ARCHITECTURE §4 ER 图：
--   - 时间字段一律 INTEGER（Unix 毫秒），不使用 DATETIME / TIMESTAMP；
--   - 文本一律 TEXT，不硬编码长度；
--   - 本文件由迁移器整文件执行，禁止对表名/路径做字符串插值（ARCHITECTURE §A03）。

-- 4.1 settings 应用设置表
CREATE TABLE IF NOT EXISTS settings (
    key        TEXT PRIMARY KEY,
    value      TEXT    NOT NULL,
    updated_at INTEGER NOT NULL
);

-- 4.2 scan_session 扫描会话表
CREATE TABLE IF NOT EXISTS scan_session (
    id                        INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at                INTEGER NOT NULL,
    finished_at               INTEGER,            -- 取消/失败时也记录结束时间，可空
    status                    TEXT    NOT NULL,   -- running / finished / cancelled / failed
    scope                     TEXT    NOT NULL,   -- full / custom / other_drive_readonly
    drive_used_bytes_before   INTEGER,            -- 扫描起点 C 盘已用空间（趋势用），可空
    error_summary             TEXT                -- 被跳过/失败目录汇总，可空
);

-- 4.3 cleanup_item 清理项表
-- L3 红线项永不写入本表（仅存在于白名单匹配器内部，命中即拒绝）。
CREATE TABLE IF NOT EXISTS cleanup_item (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id   INTEGER NOT NULL REFERENCES scan_session(id) ON DELETE CASCADE,
    module_code  TEXT    NOT NULL,                 -- A / B / C / D / E / F
    category     TEXT    NOT NULL,                 -- 规则库类别键，如 npm_cache
    path         TEXT    NOT NULL,                 -- 目标文件/目录绝对路径
    size_bytes   INTEGER NOT NULL DEFAULT 0,
    risk_level   TEXT    NOT NULL,                 -- L0 / L1 / L2；L3 禁止写入本表
    occupied_by  TEXT,                             -- 占用进程名列表（JSON 数组），可空
    description  TEXT    NOT NULL,                 -- 来自规则库
    impact_note  TEXT,                             -- “删了会怎样”，可空
    rebuild_note TEXT,                             -- 重建方式，可空
    extra        TEXT                              -- JSON：模块特有字段，可空
);

-- 4.6 cleanup_task 清理执行记录表
CREATE TABLE IF NOT EXISTS cleanup_task (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id          INTEGER REFERENCES scan_session(id),
    started_at          INTEGER NOT NULL,
    finished_at         INTEGER NOT NULL,
    disk_before_bytes   INTEGER NOT NULL,             -- C 盘剩余空间前后值
    disk_after_bytes    INTEGER NOT NULL,
    freed_bytes         INTEGER NOT NULL DEFAULT 0,
    success_count       INTEGER NOT NULL DEFAULT 0,
    fail_count          INTEGER NOT NULL DEFAULT 0,
    skip_count          INTEGER NOT NULL DEFAULT 0,
    dry_run_snapshot    TEXT    NOT NULL,             -- 执行前“将删除清单”快照 JSON
    restore_point_name  TEXT,                         -- 未创建还原点则为 NULL
    safebox_enabled     INTEGER NOT NULL              -- 本次是否走保险箱（0/1）
);

-- 4.5 safe_box 保险箱索引表（决策 #3）
CREATE TABLE IF NOT EXISTS safe_box (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    original_path   TEXT NOT NULL,                     -- 删除前绝对路径
    safe_rel_path   TEXT NOT NULL,                     -- 相对 <数据根>\SafeBox\ 的存放路径
    size_bytes      INTEGER NOT NULL,
    quarantined_at  INTEGER NOT NULL,                  -- FIFO 淘汰排序键
    expires_at      INTEGER NOT NULL,                  -- quarantined_at + 7*86400*1000
    status          TEXT NOT NULL,                     -- in_box / restored / purged / evicted
    source_task_id  INTEGER REFERENCES cleanup_task(id)
);

-- 4.7 audit_log 审计日志表：自由日志，不建强外键。
CREATE TABLE IF NOT EXISTS audit_log (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    ts             INTEGER NOT NULL,
    actor          TEXT    NOT NULL,                 -- user / system / elevated
    action         TEXT    NOT NULL,                 -- scan/quarantine/permanent_delete/...
    target         TEXT    NOT NULL,                 -- 路径/键路径/URL
    detail         TEXT,                             -- JSON 补充信息，可空
    result         TEXT    NOT NULL,                 -- success / fail / denied
    denial_reason   TEXT                              -- L3 拒绝时填写拦截规则名，可空
);

-- 4.8 registry_backup 注册表备份表
-- 任务记录删除后备份仍可被历史页引用，故 task_id 用 ON DELETE SET NULL。
CREATE TABLE IF NOT EXISTS registry_backup (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id          INTEGER REFERENCES cleanup_task(id) ON DELETE SET NULL,
    reg_path         TEXT NOT NULL,                   -- 被删除的键/值路径
    export_file_path TEXT NOT NULL,                   -- .reg 备份文件绝对路径
    created_at       INTEGER NOT NULL
);

-- 4.9 rules_meta 规则库元数据表：单行表，CHECK(id = 1)。
CREATE TABLE IF NOT EXISTS rules_meta (
    id               INTEGER PRIMARY KEY CHECK (id = 1),
    version          TEXT    NOT NULL,               -- 语义化版本
    checksum_sha256  TEXT    NOT NULL,               -- cache_rules.json 内容的 SHA-256
    source_url        TEXT    NOT NULL,              -- 实际拉取地址
    downloaded_at    INTEGER NOT NULL,
    custom_rules     TEXT    NOT NULL DEFAULT '[]'  -- 用户自定义目录规则 JSON
);

-- 4.4 cleanup_plan 清理方案/模板表
-- 不绑定 session_id（item_refs 存三元组 JSON），保证模板跨扫描复用。
CREATE TABLE IF NOT EXISTS cleanup_plan (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    note        TEXT,                               -- 可空
    is_template INTEGER NOT NULL DEFAULT 0,        -- 0=草稿/方案，1=可复用模板
    item_refs   TEXT    NOT NULL,                   -- JSON 数组：{module_code, category, path}
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);
