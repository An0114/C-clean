// task-030：保险箱 UI——safe_box_list 表格 + 单条「恢复」按钮。
//
// 与任务书偏差（按 engine 真实 shape 对齐）：
// - list 响应是 { entries, totalItems, totalBytes }（不是裸数组）；
// - SafeBoxEntry = { id, originalPath, sizeBytes, quarantinedAt }
//   （不是任务书假设的 { path, size_bytes, moved_at, original_path }）；
// - restore 入参是 safeBoxId（不是 itemId）。

import { useCallback, useEffect, useState } from "react";
import { formatBytes } from "../utils/formatBytes";
import { formatTs } from "../utils/formatTimestamp";
import { api } from "../utils/invoke";

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

interface PurgeManyResult {
  deleted: number;
  failed: number;
  errors: string[];
}

interface BatchOpResult {
  succeeded: number;
  failed: number;
  errors: string[];
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
  const [purgingId, setPurgingId] = useState<number | null>(null);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [batchPurging, setBatchPurging] = useState(false);
  const [batchRestoring, setBatchRestoring] = useState(false);
  const [result, setResult] = useState<{ ok: boolean; text: string } | null>(null);

  const load = useCallback(async () => {
    setStatus("loading");
    setMsg("");
    try {
      const r = await api<SafeBoxListResp>("safe_box_list", {});
      setEntries(r.entries);
      setTotalItems(r.totalItems);
      setTotalBytes(r.totalBytes);
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
        await api("safe_box_restore", { safeBoxId: item.id });
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

  // task-037：永久删除——confirm 后才执行，删磁盘文件 + DB 行 + audit_log。
  const purge = useCallback(
    async (item: SafeBoxEntry) => {
      const ok = window.confirm("确定永久删除该文件？此操作不可恢复。");
      if (!ok) return;
      setPurgingId(item.id);
      setResult(null);
      try {
        await api("safe_box_purge", { itemId: item.id });
        setResult({ ok: true, text: `已永久删除：${item.originalPath}` });
        await load();
      } catch (e) {
        setResult({ ok: false, text: `永久删除失败：${String(e)}` });
      } finally {
        setPurgingId(null);
      }
    },
    [load]
  );

  // task-038：批量勾选切换。
  const toggleOne = useCallback((id: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const allChecked = entries.length > 0 && selected.size === entries.length;

  const toggleAll = useCallback(() => {
    setSelected(() => {
      if (allChecked) return new Set<number>();
      return new Set(entries.map((e) => e.id));
    });
  }, [allChecked, entries]);

  // task-038：批量永久删除——confirm 后调 purge_many。
  const purgeMany = useCallback(async () => {
    const ids = Array.from(selected);
    if (ids.length === 0) return;
    const ok = window.confirm(`确定永久删除 ${ids.length} 个文件？此操作不可恢复。`);
    if (!ok) return;
    setBatchPurging(true);
    setResult(null);
    try {
      const r = await api<PurgeManyResult>("safe_box_purge_many", { itemIds: ids });
      const parts: string[] = [];
      if (r.deleted > 0) parts.push(`成功 ${r.deleted} 条`);
      if (r.failed > 0) parts.push(`失败 ${r.failed} 条：${r.errors.join("；")}`);
      setResult({
        ok: r.failed === 0,
        text: parts.join("，") || "无操作",
      });
      setSelected(new Set());
      await load();
    } catch (e) {
      setResult({ ok: false, text: `批量删除失败：${String(e)}` });
    } finally {
      setBatchPurging(false);
    }
  }, [selected, load]);

  // task-039：批量恢复——复用 selected Set，confirm 后调 restore_many。
  const restoreMany = useCallback(async () => {
    const ids = Array.from(selected);
    if (ids.length === 0) return;
    const ok = window.confirm(`确定恢复 ${ids.length} 个文件到原路径？`);
    if (!ok) return;
    setBatchRestoring(true);
    setResult(null);
    try {
      const r = await api<BatchOpResult>("safe_box_restore_many", { itemIds: ids });
      const parts: string[] = [];
      if (r.succeeded > 0) parts.push(`成功 ${r.succeeded} 条`);
      if (r.failed > 0) parts.push(`失败 ${r.failed} 条：${r.errors.join("；")}`);
      setResult({
        ok: r.failed === 0,
        text: parts.join("，") || "无操作",
      });
      setSelected(new Set());
      await load();
    } catch (e) {
      setResult({ ok: false, text: `批量恢复失败：${String(e)}` });
    } finally {
      setBatchRestoring(false);
    }
  }, [selected, load]);

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 12 }}>
        <button
          onClick={load}
          disabled={
            restoringId !== null || purgingId !== null || batchPurging || batchRestoring
          }
        >
          刷新
        </button>
        <span style={{ fontSize: 13, color: "#374151" }}>
          共 {totalItems} 项 / 总大小 {formatBytes(totalBytes)}
        </span>
        <button
          onClick={restoreMany}
          disabled={
            selected.size === 0 ||
            batchPurging ||
            batchRestoring ||
            restoringId !== null ||
            purgingId !== null
          }
          style={{
            marginLeft: "auto",
            padding: "6px 12px",
            fontSize: 12,
            borderRadius: 4,
            border: "1px solid #16a34a",
            background: selected.size === 0 ? "#bbf7d0" : "#16a34a",
            color: "#fff",
            cursor: selected.size === 0 ? "not-allowed" : "pointer",
          }}
        >
          {batchRestoring ? "恢复中…" : `批量恢复已选(${selected.size})`}
        </button>
        <button
          onClick={purgeMany}
          disabled={
            selected.size === 0 ||
            batchPurging ||
            batchRestoring ||
            restoringId !== null ||
            purgingId !== null
          }
          style={{
            padding: "6px 12px",
            fontSize: 12,
            borderRadius: 4,
            border: "1px solid #dc2626",
            background: selected.size === 0 ? "#fca5a5" : "#dc2626",
            color: "#fff",
            cursor: selected.size === 0 ? "not-allowed" : "pointer",
          }}
        >
          {batchPurging ? "删除中…" : `批量删除已选(${selected.size})`}
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
              <th style={{ ...thStyle, width: 32 }}>
                <input
                  type="checkbox"
                  checked={allChecked}
                  onChange={toggleAll}
                  disabled={
                    batchPurging || batchRestoring || restoringId !== null || purgingId !== null
                  }
                />
              </th>
              <th style={thStyle}>原路径</th>
              <th style={{ ...thStyle, width: 100 }}>大小</th>
              <th style={{ ...thStyle, width: 160 }}>移入时间</th>
              <th style={{ ...thStyle, width: 160 }}>操作</th>
            </tr>
          </thead>
          <tbody>
            {entries.map((r) => (
              <tr key={r.id}>
                <td style={tdStyle}>
                  <input
                    type="checkbox"
                    checked={selected.has(r.id)}
                    onChange={() => toggleOne(r.id)}
                    disabled={
                      batchPurging || batchRestoring || restoringId !== null || purgingId !== null
                    }
                  />
                </td>
                <td style={{ ...tdStyle, fontFamily: "monospace", fontSize: 12 }}>
                  {r.originalPath}
                </td>
                <td style={tdStyle}>{formatBytes(r.sizeBytes)}</td>
                <td style={tdStyle}>{formatTs(r.quarantinedAt)}</td>
                <td style={tdStyle}>
                  <div style={{ display: "flex", gap: 6 }}>
                    <button
                      onClick={() => restore(r)}
                      disabled={
                        restoringId !== null ||
                        purgingId !== null ||
                        batchPurging ||
                        batchRestoring
                      }
                      style={{
                        padding: "4px 10px",
                        fontSize: 12,
                        borderRadius: 4,
                        border: "1px solid #2563eb",
                        background: restoringId === r.id ? "#bfdbfe" : "#2563eb",
                        color: "#fff",
                        cursor:
                          restoringId !== null ||
                          purgingId !== null ||
                          batchPurging ||
                          batchRestoring
                            ? "not-allowed"
                            : "pointer",
                      }}
                    >
                      {restoringId === r.id ? "恢复中…" : "恢复"}
                    </button>
                    <button
                      onClick={() => purge(r)}
                      disabled={
                        restoringId !== null ||
                        purgingId !== null ||
                        batchPurging ||
                        batchRestoring
                      }
                      style={{
                        padding: "4px 10px",
                        fontSize: 12,
                        borderRadius: 4,
                        border: "1px solid #dc2626",
                        background: purgingId === r.id ? "#fecaca" : "#dc2626",
                        color: "#fff",
                        cursor:
                          restoringId !== null ||
                          purgingId !== null ||
                          batchPurging ||
                          batchRestoring
                            ? "not-allowed"
                            : "pointer",
                      }}
                    >
                      {purgingId === r.id ? "删除中…" : "永久删除"}
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
