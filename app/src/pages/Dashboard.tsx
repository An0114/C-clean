// task-026/028：仪表盘数字卡片页 + 开始扫描按钮 + 实时进度条。
// - stats 卡片：invoke('audit_log_stats') 拿 4 个数字。
// - 开始扫描：invoke('scan_start') + listen('scan://progress') 实时进度条。
// 状态：stats loading/ok/error；scan idle/scanning/done/error。

import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { formatBytes } from "../utils/formatBytes";
import { api } from "../utils/invoke";

// engine CleanupStats / ScanProgress 均 #[serde(rename_all = "camelCase")]，
// 前端 interface 必须用 camelCase（debt-cleanup-004 Bug-1+2：之前 snake_case
// 导致 successCount/failedCount/sessionCount/currentPath/scannedItems 全 undefined）。
interface CleanupStats {
  totalBytesFreed: number;
  successCount: number;
  failedCount: number;
  sessionCount: number;
}

// task-028：与 engine::ipc::progress::ScanProgress 真实 shape 对齐
// （任务书假设 {module, done, total}，实际是 {sessionId, percent, currentPath, scannedItems}）。
interface ScanProgress {
  sessionId: number;
  percent: number;
  currentPath: string;
  scannedItems: number;
}

type Status = "loading" | "ok" | "error";
type ScanStatus = "idle" | "scanning" | "done" | "error";

/**
 * task-053 G7.3：`scan_start` resolve 后延迟退订的时长。
 *
 * 后端在命令返回**之前**已经 emit 了 `percent = 100`，但 Tauri 事件是异步
 * 投递到 webview 的，Promise resolve 与事件到达的先后顺序没有保证。原实现
 * 在 resolve 后立刻 `unlisten()`，最后一条事件就被自己退订掉了 —— 这是 G7
 * 「进度条停在 88%/96%」的第三个成因。留 200ms 给投递队列收尾。
 */
const TAIL_UNLISTEN_DELAY_MS = 200;

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

  // task-053 G7.3：延迟退订的兜底定时器句柄（卸载时清掉，避免泄漏）。
  const tailTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (tailTimerRef.current !== null) clearTimeout(tailTimerRef.current);
    },
    []
  );

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

  // task-028：开始扫描——订阅进度事件 → invoke scan_start → 延迟退订 + 刷新 stats。
  //
  // task-053 G7.3：退订时机改为「收到 percent=100 立刻退订 / 否则 200ms 兜底」。
  // 原实现 finally 里同步 unlisten，把后端收尾的那条 100 事件一起退掉了。
  const startScan = useCallback(async () => {
    setScanStatus("scanning");
    setScanMsg("");
    setProgress(null);
    // 上一次扫描可能还挂着兜底定时器，先清掉，免得它退订本次的订阅。
    if (tailTimerRef.current !== null) {
      clearTimeout(tailTimerRef.current);
      tailTimerRef.current = null;
    }

    // 用可变句柄承载 unlisten：listen 回调可能早于 await 返回就触发 stop，
    // 那时 `handle.unlisten` 还是 null，必须靠 stopped 标记在赋值后补退订。
    const handle: { unlisten: UnlistenFn | null } = { unlisten: null };
    let stopped = false;

    const stopListening = () => {
      if (stopped) return;
      stopped = true;
      if (tailTimerRef.current !== null) {
        clearTimeout(tailTimerRef.current);
        tailTimerRef.current = null;
      }
      if (handle.unlisten) {
        handle.unlisten();
        handle.unlisten = null;
      }
    };

    try {
      handle.unlisten = await listen<ScanProgress>("scan://progress", (e) => {
        setProgress(e.payload);
        // 收尾事件已到，不必再等 200ms 兜底。
        if ((e.payload.percent ?? 0) >= 100) stopListening();
      });
      // 注册期间若已判定收尾（极端时序），这里立刻补退订。
      if (stopped && handle.unlisten) {
        handle.unlisten();
        handle.unlisten = null;
      }

      // debt-cleanup-003 Bug-1：scan_start 签名 scope: String 必填、target_drive: Option<String>。
      // Tauri 2 自动把 camelCase JS 参转 snake_case Rust 参：targetDrive → target_drive。
      await api("scan_start", { scope: "full", targetDrive: null });

      // task-053 G7.3 兜底：后端已 force 发出 100，但事件可能还在投递队列里。
      // 先把 UI 置满，杜绝「按钮已变『再次扫描』但进度条停在 88%」。
      // progress 为 null（一条事件都没收到）时保持 null，不伪造 sessionId。
      setProgress((p) => (p ? { ...p, percent: 100 } : p));
      setScanStatus("done");
      await load();
    } catch (e) {
      setScanStatus("error");
      setScanMsg(String(e));
      // Bug-5：失败时清掉残留进度条，否则「重试扫描」按钮旁边还挂着上一次的进度。
      setProgress(null);
    } finally {
      // 延迟退订：给最后一条 percent=100 留出投递时间（已收尾则不再排）。
      if (!stopped) {
        tailTimerRef.current = setTimeout(stopListening, TAIL_UNLISTEN_DELAY_MS);
      }
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
              {progress.currentPath || "准备中..."} — {progress.scannedItems} 项
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
            <div style={valueStyle("#111827")}>{formatBytes(data.totalBytesFreed)}</div>
          </div>
          <div style={cardStyle}>
            <div style={labelStyle}>成功次数</div>
            <div style={valueStyle("#111827")}>{data.successCount}</div>
          </div>
          <div style={cardStyle}>
            <div style={labelStyle}>失败次数</div>
            <div style={valueStyle(data.failedCount > 0 ? "#dc2626" : "#111827")}>
              {data.failedCount}
            </div>
          </div>
          <div style={cardStyle}>
            <div style={labelStyle}>清理轮次</div>
            <div style={valueStyle("#111827")}>{data.sessionCount}</div>
          </div>
        </div>
      )}
    </main>
  );
}
