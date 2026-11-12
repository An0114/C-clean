-- task-086 4.2：审计日志关联扫描会话，供 History 按会话筛选。
-- 旧审计行保持 NULL；新删除审计从 cleanup_item.session_id 继承。
ALTER TABLE audit_log ADD COLUMN session_id INTEGER REFERENCES scan_session(id);
CREATE INDEX IF NOT EXISTS idx_audit_log_session ON audit_log(session_id);
