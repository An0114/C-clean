// task-026：仪表盘数字卡片页。
// 启动时 invoke('audit_log_stats')，渲染 4 个卡片：总释放空间 / 成功 / 失败 / 清理轮次。
// 状态：loading（骨架）/ ok（4 卡片）/ error（红条 + 重试）。

import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { formatBytes } from "../utils/formatBytes";

interface CleanupStats {
  total_bytes_freed: number;
  success_count: number;
  failed_count: number;
  session_count: number;
}

type Status = "loading" | "ok" | "error";

const cardStyle: React.CSSProperties = {
  border: "1px solid #e5e7eb",
  borderRadius: 8,
  padding: "16px 20px",
  background: "#fff",
  minHeight: 90,
};

const labelStyle: React.CSSProperties = {
  fontSize: 12,
  color: "#6b7280",
  marginBottom: 8,
};

const valueStyle = (color: string): React.CSSProperties => ({
  fontSize: 28,
  fontWeight: 600,
  color,
});

export default function Dashboard() {
  const [status, setStatus] = useState<Status>("loading");
  const [data, setData] = useState<CleanupStats | null>(null);
  const [msg, setMsg] = useState<string>("");

  const load = useCallback(async () => {
    setStatus("loading");
    setMsg("");
    try {
      const s = await invoke<CleanupStats>("audit_log_stats", {});
      setData(s);
      setStatus("ok");
    } catch (e) {
      setData(null);
      setMsg(String(e));
      setStatus("error");
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  return (
    <main
      style={{
        padding: 24,
        fontFamily: "system-ui, sans-serif",
        background: "#f9fafb",
        minHeight: "100vh",
        boxSizing: "border-box",
      }}
    >
      <h1 style={{ margin: "0 0 16px" }}>C-Disk Cleaner</h1>
      <p style={{ color: "#6b7280", marginTop: 0 }}>清理统计</p>

      {status === "loading" && (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "1fr 1fr",
            gap: 12,
            opacity: 0.5,
          }}
        >
          {[0, 1, 2, 3].map((i) => (
            <div key={i} style={cardStyle}>
              <div style={{ ...labelStyle, background: "#e5e7eb", height: 12, width: 80 }} />
              <div style={{ background: "#e5e7eb", height: 28, width: 120, marginTop: 8 }} />
            </div>
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
          <div style={{ fontWeight: 600, marginBottom: 4 }}>加载统计失败</div>
          <div style={{ fontSize: 13, marginBottom: 8 }}>{msg}</div>
          <button onClick={load}>重试</button>
        </div>
      )}

      {status === "ok" && data && (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "1fr 1fr",
            gap: 12,
          }}
        >
          <div style={cardStyle}>
            <div style={labelStyle}>总释放空间</div>
            <div style={valueStyle("#111827")}>{formatBytes(data.total_bytes_freed)}</div>
          </div>
          <div style={cardStyle}>
            <div style={labelStyle}>成功次数</div>
            <div style={valueStyle("#111827")}>{data.success_count}</div>
          </div>
          <div style={cardStyle}>
            <div style={labelStyle}>失败次数</div>
            <div style={valueStyle(data.failed_count > 0 ? "#dc2626" : "#111827")}>
              {data.failed_count}
            </div>
          </div>
          <div style={cardStyle}>
            <div style={labelStyle}>清理轮次</div>
            <div style={valueStyle("#111827")}>{data.session_count}</div>
          </div>
        </div>
      )}
    </main>
  );
}
