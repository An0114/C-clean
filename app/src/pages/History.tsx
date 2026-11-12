// task-027：历史明细页。
// 启动时 invoke('audit_log_list', { limit, offset })，渲染表格：时间 / 事件类型 / 路径 / 详情。
// 分页：上一页 / 下一页（每页 20 条）；加载骨架 / 错误红条 + 重试。

import { useCallback, useEffect, useState } from "react";
import { formatTs } from "../utils/formatTimestamp";
import { api } from "../utils/invoke";

// engine AuditLogEntry 是 #[serde(rename_all = "camelCase")]，前端用 camelCase。
interface AuditLogEntry {
  id: number;
  sessionId: number | null;
  eventType: string;
  path: string;
  detail: string | null;
  createdAt: number;
}

interface ScanSessionOption {
  id: number;
  startedAt: number;
  status: string;
}

type Status = "loading" | "ok" | "error";

const PAGE_SIZE = 20;

const thStyle: React.CSSProperties = {
  textAlign: "left",
  padding: "8px 12px",
  borderBottom: "1px solid var(--border)",
  fontSize: 12,
  color: "var(--text-muted)",
  fontWeight: 600,
  background: "var(--bg)",
};

const tdStyle: React.CSSProperties = {
  padding: "8px 12px",
  borderBottom: "1px solid var(--border)",
  fontSize: 13,
  color: "var(--text)",
  verticalAlign: "top",
};

export default function History() {
  const [status, setStatus] = useState<Status>("loading");
  const [rows, setRows] = useState<AuditLogEntry[]>([]);
  const [offset, setOffset] = useState(0);
  const [msg, setMsg] = useState<string>("");
  const [sessionId, setSessionId] = useState<number | null>(null);
  const [sessions, setSessions] = useState<ScanSessionOption[]>([]);

  const load = useCallback(async (off: number) => {
    setStatus("loading");
    setMsg("");
    try {
      const r = await api<AuditLogEntry[]>("audit_log_list", {
        sessionId,
        limit: PAGE_SIZE,
        offset: off,
      });
      setRows(r);
      setOffset(off);
      setStatus("ok");
    } catch (e) {
      setRows([]);
      setMsg(String(e));
      setStatus("error");
    }
  }, [sessionId]);

  useEffect(() => {
    load(0);
  }, [load]);

  useEffect(() => {
    api<ScanSessionOption[]>("scan_session_list", {})
      .then(setSessions)
      .catch(() => setSessions([])); // 筛选选项加载失败不阻断历史列表
  }, []);

  const canPrev = offset > 0;
  const canNext = rows.length === PAGE_SIZE; // 下一页可能还有

  return (
    <div>
      <h2 style={{ margin: "0 0 12px", fontSize: 18 }}>清理历史</h2>

      <label style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 12, fontSize: 13 }}>
        按扫描会话筛选
        <select
          data-testid="session-filter"
          value={sessionId ?? ""}
          onChange={(e) => {
            setOffset(0);
            setSessionId(e.target.value === "" ? null : Number(e.target.value));
          }}
          style={{ padding: "6px 8px", background: "var(--surface)", color: "var(--text)", border: "1px solid var(--border)", borderRadius: 6 }}
        >
          <option value="">全部会话</option>
          {sessions.map((s) => (
            <option key={s.id} value={s.id}>
              #{s.id} · {formatTs(s.startedAt)} · {s.status}
            </option>
          ))}
        </select>
      </label>

      {status === "loading" && (
        <div style={{ opacity: 0.5, padding: 12 }}>
          {[0, 1, 2, 3].map((i) => (
            <div key={i} style={{ background: "var(--border)", height: 16, marginBottom: 8, borderRadius: 4 }} />
          ))}
        </div>
      )}

      {status === "error" && (
        <div
          style={{
            background: "rgba(220, 38, 38, 0.16)",
            border: "1px solid rgba(220, 38, 38, 0.45)",
            color: "#ef4444",
            padding: 12,
            borderRadius: 8,
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: 4 }}>加载历史失败</div>
          <div style={{ fontSize: 13, marginBottom: 8 }}>{msg}</div>
          <button onClick={() => load(offset)}>重试</button>
        </div>
      )}

      {status === "ok" && (
        <>
          <table style={{ width: "100%", borderCollapse: "collapse", background: "var(--surface)", border: "1px solid var(--border)", borderRadius: 8 }}>
            <thead>
              <tr>
                <th style={thStyle}>时间</th>
                <th style={thStyle}>事件类型</th>
                <th style={thStyle}>路径</th>
                <th style={thStyle}>详情</th>
              </tr>
            </thead>
            <tbody>
              {rows.length === 0 && (
                <tr>
                  <td colSpan={4} style={{ ...tdStyle, textAlign: "center", color: "var(--text-muted)", padding: 24 }}>
                    暂无记录
                  </td>
                </tr>
              )}
              {rows.map((r) => (
                <tr key={r.id}>
                  <td style={{ ...tdStyle, whiteSpace: "nowrap" }}>{formatTs(r.createdAt)}</td>
                  <td style={{ ...tdStyle, whiteSpace: "nowrap" }}>{r.eventType}</td>
                  <td style={{ ...tdStyle, maxWidth: 360, wordBreak: "break-all" }}>{r.path}</td>
                  <td style={{ ...tdStyle, maxWidth: 320, wordBreak: "break-all", color: "var(--text-muted)" }}>
                    {r.detail ?? "-"}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>

          <div style={{ marginTop: 12, display: "flex", gap: 8, alignItems: "center" }}>
            <button disabled={!canPrev} onClick={() => load(Math.max(0, offset - PAGE_SIZE))}>
              上一页
            </button>
            <span style={{ fontSize: 12, color: "var(--text-muted)" }}>
              第 {offset + 1}–{offset + rows.length} 条
            </span>
            <button disabled={!canNext} onClick={() => load(offset + PAGE_SIZE)}>
              下一页
            </button>
          </div>
        </>
      )}
    </div>
  );
}
