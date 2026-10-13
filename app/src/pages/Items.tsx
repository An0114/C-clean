// task-029：清理闭环 Items 页面。
// - 启动 invoke('cleanup_list_pending') 拉待清理表格（size 倒序）；
// - checkbox 多选 + 底部「已选 N 项，共 X MB」+「清理已选」按钮；
// - 清理中按钮禁用；完成后自动重拉列表 + 成功/失败提示。
//
// 与任务书偏差：cleanup_item 表无 status/scanned_at 列；
// PendingItem 仅 id/path/sizeBytes/moduleCode/sessionId，无时间列展示。

import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { formatBytes } from "../utils/formatBytes";

interface PendingItem {
  id: number;
  path: string;
  sizeBytes: number;
  moduleCode: string;
  sessionId: number;
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

export default function Items() {
  const [status, setStatus] = useState<Status>("loading");
  const [rows, setRows] = useState<PendingItem[]>([]);
  const [msg, setMsg] = useState<string>("");
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [cleaning, setCleaning] = useState(false);
  const [result, setResult] = useState<{ ok: boolean; text: string } | null>(null);

  const load = useCallback(async () => {
    setStatus("loading");
    setMsg("");
    try {
      const list = await invoke<PendingItem[]>("cleanup_list_pending", { limit: 100 });
      setRows(list);
      setStatus("ok");
    } catch (e) {
      setRows([]);
      setMsg(String(e));
      setStatus("error");
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const toggle = (id: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const totalSelectedBytes = rows
    .filter((r) => selected.has(r.id))
    .reduce((sum, r) => sum + (r.sizeBytes || 0), 0);

  const cleanup = useCallback(async () => {
    if (selected.size === 0 || cleaning) return;
    setCleaning(true);
    setResult(null);
    try {
      const ids = Array.from(selected);
      await invoke("cleanup_execute", { itemIds: ids });
      setResult({ ok: true, text: `已清理 ${ids.length} 项` });
      setSelected(new Set());
      await load();
    } catch (e) {
      setResult({ ok: false, text: String(e) });
    } finally {
      setCleaning(false);
    }
  }, [selected, cleaning, load]);

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 12 }}>
        <button onClick={load} disabled={cleaning}>
          刷新
        </button>
        <span style={{ fontSize: 13, color: "#374151" }}>
          已选 {selected.size} 项，共 {formatBytes(totalSelectedBytes)}
        </span>
        <button
          onClick={cleanup}
          disabled={selected.size === 0 || cleaning}
          style={{
            padding: "6px 14px",
            borderRadius: 6,
            border: "1px solid #dc2626",
            background: selected.size === 0 || cleaning ? "#fecaca" : "#dc2626",
            color: "#fff",
            cursor: selected.size === 0 || cleaning ? "not-allowed" : "pointer",
          }}
        >
          {cleaning ? "清理中…" : "清理已选"}
        </button>
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

      {status === "loading" && <div style={{ color: "#6b7280", fontSize: 13 }}>加载中...</div>}

      {status === "error" && (
        <div style={{ color: "#b91c1c", fontSize: 13 }}>
          加载失败：{msg}
        </div>
      )}

      {status === "ok" && (
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr>
              <th style={{ ...thStyle, width: 32 }}>
                <input
                  type="checkbox"
                  checked={rows.length > 0 && selected.size === rows.length}
                  onChange={(e) => {
                    if (e.target.checked) setSelected(new Set(rows.map((r) => r.id)));
                    else setSelected(new Set());
                  }}
                />
              </th>
              <th style={thStyle}>路径</th>
              <th style={{ ...thStyle, width: 100 }}>大小</th>
              <th style={{ ...thStyle, width: 80 }}>模块</th>
              <th style={{ ...thStyle, width: 80 }}>Session</th>
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 && (
              <tr>
                <td colSpan={5} style={{ ...tdStyle, color: "#9ca3af", textAlign: "center" }}>
                  暂无待清理项
                </td>
              </tr>
            )}
            {rows.map((r) => (
              <tr key={r.id}>
                <td style={tdStyle}>
                  <input
                    type="checkbox"
                    checked={selected.has(r.id)}
                    onChange={() => toggle(r.id)}
                  />
                </td>
                <td style={{ ...tdStyle, fontFamily: "monospace", fontSize: 12 }}>{r.path}</td>
                <td style={tdStyle}>{formatBytes(r.sizeBytes)}</td>
                <td style={tdStyle}>{r.moduleCode}</td>
                <td style={tdStyle}>#{r.sessionId}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
