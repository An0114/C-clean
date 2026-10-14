// task-026/028：仪表盘数字卡片页 + 开始扫描按钮 + 实时进度条。
// - stats 卡片：invoke('audit_log_stats') 拿 4 个数字。
// - 开始扫描：invoke('scan_start') + listen('scan://progress') 实时进度条。
// 状态：stats loading/ok/error；scan idle/scanning/done/error。

import { useCallback, useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { formatBytes } from "../utils/formatBytes";
import { api } from "../utils/invoke";

interface CleanupStats {
  total_bytes_freed: number;
  success_count: number;
  failed_count: number;
  session_count: number;
}

// task-028：与 engine::ipc::progress::ScanProgress 真实 shape 对齐
// （任务书假设 {module, done, total}，实际是 {session_id, percent, current_path, scanned_items}）。
interface ScanProgress {
  session_id: number;
  percent: number;
  current_path: string;
  scanned_items: number;
}

type Status = "loading" | "ok" | "error";
type ScanStatus = "idle" | "scanning" | "done" | "error";

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

const btnStyle = (disabled: boolean): React.CSSProperties => ({
  padding: "8px 16px",
  fontSize: 14,
  borderRadius: 6,
  border: "1px solid #10b981",
  background: disabled ? "#a7f3d0" : "#10b981",
  color: "#fff",
  cursor: disabled ? "not-allowed" : "pointer",
});

export default function Dashboard() {
  const [status, setStatus] = useState<Status>("loading");
  const [data, setData] = useState<CleanupStats | null>(null);
  const [msg, setMsg] = useState<string>("");

  // task-028：扫描状态。
  const [scanStatus, setScanStatus] = useState<ScanStatus>("idle");
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [scanMsg, setScanMsg] = useState<string>("");

  const load = useCallback(async () => {
    setStatus("loading");
    setMsg("");
    try {
      const s = await api<CleanupStats>("audit_log_stats", {});
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

  // task-028：开始扫描——订阅进度事件 → invoke scan_start → finally unlisten + 刷新 stats。
  const startScan = useCallback(async () => {
    setScanStatus("scanning");
    setScanMsg("");
    setProgress(null);
    let unlisten: UnlistenFn | undefined;
    try {
      unlisten = await listen<ScanProgress>("scan://progress", (e) => {
        setProgress(e.payload);
      });
      // debt-cleanup-003 Bug-1：scan_start 签名 scope: String 必填、target_drive: Option<String>。
      // Tauri 2 自动把 camelCase JS 参转 snake_case Rust 参：targetDrive → target_drive。
      await api("scan_start", { scope: "full", targetDrive: null });
      setScanStatus("done");
      await load();
    } catch (e) {
      setScanStatus("error");
      setScanMsg(String(e));
      // Bug-5：失败时清掉残留进度条，否则「重试扫描」按钮旁边还挂着上一次的进度。
      setProgress(null);
    } finally {
      if (unlisten) unlisten();
    }
  }, [load]);

  const scanning = scanStatus === "scanning";
  // engine ScanProgress.percent 已是 0~100，直接用；缺失时置 0 不除零。
  const percent = progress ? progress.percent ?? 0 : 0;
  const barPct = Math.max(0, Math.min(100, percent));

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

      {/* task-028：开始扫描按钮 + 进度条 */}
      <div style={{ marginBottom: 16 }}>
        <button style={btnStyle(scanning)} disabled={scanning} onClick={startScan}>
          {scanning
            ? "扫描中..."
            : scanStatus === "done"
            ? "再次扫描"
            : scanStatus === "error"
            ? "重试扫描"
            : "开始扫描"}
        </button>

        {(scanning || scanStatus === "done") && progress && (
          <div style={{ marginTop: 12 }}>
            <div
              style={{
                height: 8,
                background: "#e5e7eb",
                borderRadius: 4,
                overflow: "hidden",
              }}
            >
              <div
                style={{
                  height: "100%",
                  width: `${barPct}%`,
                  background: "#10b981",
                  transition: "width 100ms linear",
                }}
              />
            </div>
            <div style={{ marginTop: 6, fontSize: 12, color: "#374151" }}>
              {progress.current_path || "准备中..."} — {progress.scanned_items} 项
            </div>
          </div>
        )}

        {scanStatus === "error" && (
          <div
            style={{
              marginTop: 12,
              background: "#fee2e2",
              border: "1px solid #fca5a5",
              color: "#b91c1c",
              padding: 12,
              borderRadius: 8,
            }}
          >
            <div style={{ fontWeight: 600, marginBottom: 4 }}>扫描失败</div>
            <div style={{ fontSize: 13 }}>{scanMsg}</div>
          </div>
        )}
      </div>

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
