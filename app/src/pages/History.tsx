// task-027：历史明细页。
// 启动时 invoke('audit_log_list', { limit, offset })，渲染表格：时间 / 事件类型 / 路径 / 详情。
// 分页：上一页 / 下一页（每页 20 条）；加载骨架 / 错误红条 + 重试。

import { useCallback, useEffect, useState } from "react";
import { formatTs } from "../utils/formatTimestamp";
import { api } from "../utils/invoke";

interface AuditLogEntry {
  id: number;
  session_id: number;
  event_type: string;
  path: string;
  detail: string | null;
  created_at: number;
}

type Status = "loading" | "ok" | "error";

const PAGE_SIZE = 20;

const thStyle: React.CSSProperties = {
  textAlign: "left",
  padding: "8px 12px",
  borderBottom: "1px solid #e5e7eb",
  fontSize: 12,
  color: "#6b7280",
  fontWeight: 600,
  background: "#f9fafb",
};

const tdStyle: React.CSSProperties = {
  padding: "8px 12px",
  borderBottom: "1px solid #f3f4f6",
  fontSize: 13,
  color: "#111827",
  verticalAlign: "top",
};

export default function History() {
  const [status, setStatus] = useState<Status>("loading");
  const [rows, setRows] = useState<AuditLogEntry[]>([]);
  const [offset, setOffset] = useState(0);
  const [msg, setMsg] = useState<string>("");

  const load = useCallback(async (off: number) => {
    setStatus("loading");
    setMsg("");
    try {
      const r = await api<AuditLogEntry[]>("audit_log_list", {
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
  }, []);

  useEffect(() => {
    load(0);
  }, [load]);

  const canPrev = offset > 0;
  const canNext = rows.length === PAGE_SIZE; // 下一页可能还有

  return (
    <div>
      <h2 style={{ margin: "0 0 12px", fontSize: 18 }}>清理历史</h2>

      {status === "loading" && (
        <div style={{ opacity: 0.5, padding: 12 }}>
          {[0, 1, 2, 3].map((i) => (
            <div key={i} style={{ background: "#e5e7eb", height: 16, marginBottom: 8, borderRadius: 4 }} />
          ))}
        </div>
      )}

      {status === "error" && (
        <div
          style={{
            background: "#fee2e2",
            border: "1px solid #fca5a5",
            color: "#b91c1c",
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
          <table style={{ width: "100%", borderCollapse: "collapse", background: "#fff", border: "1px solid #e5e7eb", borderRadius: 8 }}>
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
                  <td colSpan={4} style={{ ...tdStyle, textAlign: "center", color: "#6b7280", padding: 24 }}>
                    暂无记录
                  </td>
                </tr>
              )}
              {rows.map((r) => (
                <tr key={r.id}>
                  <td style={{ ...tdStyle, whiteSpace: "nowrap" }}>{formatTs(r.created_at)}</td>
                  <td style={{ ...tdStyle, whiteSpace: "nowrap" }}>{r.event_type}</td>
                  <td style={{ ...tdStyle, maxWidth: 360, wordBreak: "break-all" }}>{r.path}</td>
                  <td style={{ ...tdStyle, maxWidth: 320, wordBreak: "break-all", color: "#6b7280" }}>
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
            <span style={{ fontSize: 12, color: "#6b7280" }}>
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
