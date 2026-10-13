// task-030：保险箱 UI——safe_box_list 表格 + 单条「恢复」按钮。
//
// 与任务书偏差（按 engine 真实 shape 对齐）：
// - list 响应是 { entries, totalItems, totalBytes }（不是裸数组）；
// - SafeBoxEntry = { id, originalPath, sizeBytes, quarantinedAt }
//   （不是任务书假设的 { path, size_bytes, moved_at, original_path }）；
// - restore 入参是 safeBoxId（不是 itemId）。

import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { formatBytes } from "../utils/formatBytes";
import { formatTs } from "../utils/formatTimestamp";

interface SafeBoxEntry {
  id: number;
  originalPath: string;
  sizeBytes: number;
  quarantinedAt: number;
}

interface SafeBoxListResp {
  entries: SafeBoxEntry[];
  totalItems: number;
  totalBytes: number;
}

type Status = "loading" | "ok" | "error";

const thStyle: React.CSSProperties = {
  textAlign: "left",
  padding: "8px 10px",
  borderBottom: "1px solid #e5e7eb",
  fontSize: 12,
  color: "#6b7280",
  fontWeight: 600,
};

const tdStyle: React.CSSProperties = {
  padding: "8px 10px",
  borderBottom: "1px solid #f3f4f6",
  fontSize: 13,
};

export default function SafeBox() {
  const [status, setStatus] = useState<Status>("loading");
  const [entries, setEntries] = useState<SafeBoxEntry[]>([]);
  const [totalItems, setTotalItems] = useState(0);
  const [totalBytes, setTotalBytes] = useState(0);
  const [msg, setMsg] = useState<string>("");
  const [restoringId, setRestoringId] = useState<number | null>(null);
  const [result, setResult] = useState<{ ok: boolean; text: string } | null>(null);

  const load = useCallback(async () => {
    setStatus("loading");
    setMsg("");
    try {
      const r = await invoke<SafeBoxListResp>("safe_box_list", {});
      setEntries(r.entries || []);
      setTotalItems(r.totalItems || 0);
      setTotalBytes(r.totalBytes || 0);
      setStatus("ok");
    } catch (e) {
      setEntries([]);
      setTotalItems(0);
      setTotalBytes(0);
      setMsg(String(e));
      setStatus("error");
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const restore = useCallback(
    async (item: SafeBoxEntry) => {
      setRestoringId(item.id);
      setResult(null);
      try {
        await invoke("safe_box_restore", { safeBoxId: item.id });
        setResult({ ok: true, text: `已恢复：${item.originalPath}` });
        await load();
      } catch (e) {
        setResult({ ok: false, text: `恢复失败：${String(e)}` });
      } finally {
        setRestoringId(null);
      }
    },
    [load]
  );

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 12 }}>
        <button onClick={load} disabled={restoringId !== null}>
          刷新
        </button>
        <span style={{ fontSize: 13, color: "#374151" }}>
          共 {totalItems} 项 / 总大小 {formatBytes(totalBytes)}
        </span>
      </div>

      {result && (
        <div
          style={{
            marginBottom: 12,
            padding: 10,
            borderRadius: 6,
            fontSize: 13,
            background: result.ok ? "#dcfce7" : "#fee2e2",
            color: result.ok ? "#166534" : "#b91c1c",
            border: `1px solid ${result.ok ? "#86efac" : "#fca5a5"}`,
          }}
        >
          {result.text}
        </div>
      )}

      {status === "loading" && (
        <div style={{ color: "#6b7280", fontSize: 13 }}>加载中...</div>
      )}

      {status === "error" && (
        <div style={{ color: "#b91c1c", fontSize: 13 }}>加载失败：{msg}</div>
      )}

      {status === "ok" && entries.length === 0 && (
        <div
          style={{ color: "#9ca3af", fontSize: 13, textAlign: "center", padding: "32px 0" }}
        >
          保险箱为空
        </div>
      )}

      {status === "ok" && entries.length > 0 && (
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr>
              <th style={thStyle}>原路径</th>
              <th style={{ ...thStyle, width: 100 }}>大小</th>
              <th style={{ ...thStyle, width: 160 }}>移入时间</th>
              <th style={{ ...thStyle, width: 90 }}>操作</th>
            </tr>
          </thead>
          <tbody>
            {entries.map((r) => (
              <tr key={r.id}>
                <td style={{ ...tdStyle, fontFamily: "monospace", fontSize: 12 }}>
                  {r.originalPath}
                </td>
                <td style={tdStyle}>{formatBytes(r.sizeBytes)}</td>
                <td style={tdStyle}>{formatTs(r.quarantinedAt)}</td>
                <td style={tdStyle}>
                  <button
                    onClick={() => restore(r)}
                    disabled={restoringId !== null}
                    style={{
                      padding: "4px 10px",
                      fontSize: 12,
                      borderRadius: 4,
                      border: "1px solid #2563eb",
                      background: restoringId === r.id ? "#bfdbfe" : "#2563eb",
                      color: "#fff",
                      cursor: restoringId !== null ? "not-allowed" : "pointer",
                    }}
                  >
                    {restoringId === r.id ? "恢复中…" : "恢复"}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
