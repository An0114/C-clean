// task-029：清理闭环 Items 页面。
// - 启动 invoke('cleanup_list_pending') 拉待清理表格（size 倒序）；
// - checkbox 多选 + 底部「已选 N 项，共 X MB」+「清理已选」按钮；
// - 清理中按钮禁用；完成后自动重拉列表 + 成功/失败提示。
//
// task-040：加「预览可清理项」按钮 + dryRun 只读预览面板。
// - 走 apiRaw（不是 api）：cleanup.dryRun 的 2002 用 err_with_data，
//   失败信封的 data 非 null 且带 blockedByL3 明细，api() 会把它抛掉；
// - 只预览不执行：面板内不新增第二个执行入口，确认后仍走既有「清理已选」。
//
// 与任务书偏差：cleanup_item 表无 status/scanned_at 列；
// PendingItem 仅 id/path/sizeBytes/moduleCode/sessionId，无时间列展示。

import { useCallback, useEffect, useState } from "react";
import { formatBytes } from "../utils/formatBytes";
import { api, apiRaw } from "../utils/invoke";
import {
  describeDenial,
  formatBreakdownLine,
  formatDryRunHeadline,
  sortBreakdown,
  type BlockedItem,
  type DryRunResp,
} from "../utils/dryRun";

interface PendingItem {
  id: number;
  path: string;
  sizeBytes: number;
  moduleCode: string;
  sessionId: number;
}

type Status = "loading" | "ok" | "error";

/** task-040：预览面板状态（判别联合，便于按 kind 分支渲染与整体清空）。 */
type Preview =
  | { kind: "ok"; resp: DryRunResp }
  | { kind: "empty" }
  | { kind: "blocked"; blocked: BlockedItem[] }
  | { kind: "error"; message: string };

/** 预览面板统一底色/文字/边框（与既有 result 提示条同风格）。 */
const panelStyle = (bg: string, fg: string, border: string): React.CSSProperties => ({
  marginBottom: 12,
  padding: 10,
  borderRadius: 6,
  fontSize: 13,
  background: bg,
  color: fg,
  border: `1px solid ${border}`,
});

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
  // task-040 预览态。
  const [previewing, setPreviewing] = useState(false);
  const [preview, setPreview] = useState<Preview | null>(null);

  const load = useCallback(async () => {
    setStatus("loading");
    setMsg("");
    // C8：列表重拉后旧预览可能已过期，先清掉。
    setPreview(null);
    try {
      const list = await api<PendingItem[]>("cleanup_list_pending", { limit: 100 });
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
    // C8：勾选集合一变，旧预览就失效。
    setPreview(null);
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const totalSelectedBytes = rows
    .filter((r) => selected.has(r.id))
    .reduce((sum, r) => sum + r.sizeBytes, 0);

  const cleanup = useCallback(async () => {
    if (selected.size === 0 || cleaning) return;
    setCleaning(true);
    setResult(null);
    // C8：清理成功后清单会变，预览必然过期。
    setPreview(null);
    try {
      const ids = Array.from(selected);
      await api("cleanup_execute", { itemIds: ids });
      setResult({ ok: true, text: `已清理 ${ids.length} 项` });
      setSelected(new Set());
      await load();
    } catch (e) {
      setResult({ ok: false, text: String(e) });
    } finally {
      setCleaning(false);
    }
  }, [selected, cleaning, load]);

  // task-040 C2：只读预览。走 apiRaw 以拿回 2002 信封里的 blockedByL3。
  const previewSelection = useCallback(async () => {
    if (selected.size === 0 || cleaning || previewing) return;
    setPreviewing(true);
    setPreview(null);
    try {
      const env = await apiRaw<DryRunResp>("cleanup_dry_run", {
        itemIds: Array.from(selected),
      });
      if (env.code === 0 && env.data) {
        // C3
        setPreview({ kind: "ok", resp: env.data });
      } else if (env.code === 2002) {
        // C4：L3 命中明细就在失败信封的 data 上。
        setPreview({ kind: "blocked", blocked: env.data?.blockedByL3 ?? [] });
      } else if (env.code === 2001) {
        // C5
        setPreview({ kind: "empty" });
      } else {
        // C6
        setPreview({ kind: "error", message: env.message });
      }
    } catch (e) {
      // C6：invoke 本身 reject（命令未注册 / 序列化失败等）。
      setPreview({ kind: "error", message: String(e) });
    } finally {
      setPreviewing(false);
    }
  }, [selected, cleaning, previewing]);

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
          onClick={previewSelection}
          disabled={selected.size === 0 || cleaning || previewing}
          style={{
            padding: "6px 14px",
            borderRadius: 6,
            border: "1px solid #2563eb",
            background: selected.size === 0 || cleaning || previewing ? "#bfdbfe" : "#2563eb",
            color: "#fff",
            cursor:
              selected.size === 0 || cleaning || previewing ? "not-allowed" : "pointer",
          }}
        >
          {previewing ? "预览中…" : "预览可清理项"}
        </button>
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

      {/* task-040 预览面板：只读展示，面板内不提供执行入口。 */}
      {preview?.kind === "ok" && (
        <div style={panelStyle("#eff6ff", "#1e3a8a", "#93c5fd")}>
          <div style={{ fontWeight: 600, marginBottom: 6 }}>
            {formatDryRunHeadline(preview.resp)}
          </div>
          {sortBreakdown(preview.resp.breakdown).map((row) => (
            <div key={`${row.moduleCode}/${row.category}`} style={{ marginBottom: 2 }}>
              {formatBreakdownLine(row)}
            </div>
          ))}
          <div style={{ marginTop: 8, color: "#1d4ed8" }}>
            确认无误后点上方「清理已选」执行
          </div>
        </div>
      )}

      {preview?.kind === "blocked" && (
        <div style={panelStyle("#fee2e2", "#b91c1c", "#fca5a5")}>
          <div style={{ fontWeight: 600, marginBottom: 6 }}>
            {preview.blocked.length} 项命中红线，已阻止清理
          </div>
          {preview.blocked.map((b) => (
            <div key={b.itemId} style={{ marginBottom: 2 }}>
              <span style={{ fontFamily: "monospace", fontSize: 12 }}>#{b.itemId}</span>{" "}
              <span style={{ fontFamily: "monospace", fontSize: 12 }}>{b.path}</span>
              {" · "}
              {describeDenial(b.denialReason)}
            </div>
          ))}
        </div>
      )}

      {preview?.kind === "empty" && (
        <div style={panelStyle("#fef9c3", "#854d0e", "#fde047")}>
          已选条目已不在待清理列表（可能已被清理或扫描会话已过期），请点「刷新」
        </div>
      )}

      {preview?.kind === "error" && (
        <div style={panelStyle("#fee2e2", "#b91c1c", "#fca5a5")}>
          预览失败：{preview.message}
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
                    // C8：全选/全不选同样让旧预览失效。
                    setPreview(null);
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
