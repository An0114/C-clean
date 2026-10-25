// task-030：保险箱 UI——safe_box_list 表格 + 单条「恢复」按钮。
// task-045/046：分页 + 单 session 过滤。
// task-051：时间预设筛选 / 多选 session / 「来源」列 / 行内审计历史展开。
//
// 与任务书偏差（按 engine 真实 shape 对齐）：
// - list 响应是 { items, total, page, pageSize, totalBytes }（task-045 起）；
// - SafeBoxEntry = { id, originalPath, sizeBytes, quarantinedAt, sourceTaskId }；
// - restore 入参是 safeBoxId（不是 itemId）。

import { Fragment, useCallback, useEffect, useState } from "react";
import { formatBytes } from "../utils/formatBytes";
import { formatTs } from "../utils/formatTimestamp";
import { api } from "../utils/invoke";

interface SafeBoxEntry {
  id: number;
  originalPath: string;
  sizeBytes: number;
  quarantinedAt: number;
  /** task-051：来源清理批次（safe_box.source_task_id），null = 历史遗留/未关联。 */
  sourceTaskId: number | null;
}

// task-045：分页响应。
interface SafeBoxPage {
  items: SafeBoxEntry[];
  total: number;
  page: number;
  pageSize: number;
  totalBytes: number;
}

// task-046：session 分组摘要（按 source_task_id）。
interface SessionSummary {
  sessionId: number;
  itemCount: number;
  createdTs: number;
}

// task-051：审计历史条目（engine::audit_log::AuditLogEntry，camelCase）。
interface AuditLogEntry {
  id: number;
  eventType: string;
  path: string;
  detail: string | null;
  createdAt: number;
  result: string;
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

/** task-051：时间预设（只有「距现在 N」的相对窗口，天然无时区问题）。 */
type TimePreset = "all" | "24h" | "3d" | "7d";

const PRESETS: { key: TimePreset; label: string; seconds: number }[] = [
  { key: "all", label: "全部", seconds: 0 },
  { key: "24h", label: "24 小时", seconds: 24 * 3600 },
  { key: "3d", label: "3 天", seconds: 3 * 86400 },
  { key: "7d", label: "7 天", seconds: 7 * 86400 },
];

/**
 * task-051：预设 → `fromTs`（Unix **秒**）。`all` 返回 undefined（不传该参数）。
 *
 * 抽成纯函数便于单测：`nowSec` 由调用方传入，测试不依赖真实时钟。
 */
export function presetFromTs(preset: TimePreset, nowSec: number): number | undefined {
  const hit = PRESETS.find((p) => p.key === preset);
  if (!hit || hit.seconds === 0) return undefined;
  return nowSec - hit.seconds;
}

/** task-051：审计动作中文化；未知动作原样显示（不吞信息）。 */
export function actionLabel(eventType: string): string {
  switch (eventType) {
    case "permanent_delete":
      return "永久删除";
    case "restore":
      return "恢复";
    case "purge":
      return "清除";
    case "quarantine":
      return "移入保险箱";
    case "scan":
      return "扫描";
    default:
      return eventType;
  }
}

interface HistoryState {
  /** 当前展开的条目 id；null = 全部收起。 */
  id: number | null;
  loading: boolean;
  rows: AuditLogEntry[];
  error: string;
}

const EMPTY_HISTORY: HistoryState = { id: null, loading: false, rows: [], error: "" };

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
  const [total, setTotal] = useState(0);
  const [totalBytes, setTotalBytes] = useState(0);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(50);
  // task-051：空数组 = 全部 session（不再用单选）。
  const [sessionIds, setSessionIds] = useState<number[]>([]);
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  // task-051：时间预设。
  const [timePreset, setTimePreset] = useState<TimePreset>("all");
  const [history, setHistory] = useState<HistoryState>(EMPTY_HISTORY);
  const [msg, setMsg] = useState<string>("");
  const [restoringId, setRestoringId] = useState<number | null>(null);
  const [purgingId, setPurgingId] = useState<number | null>(null);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [batchPurging, setBatchPurging] = useState(false);
  const [batchRestoring, setBatchRestoring] = useState(false);
  const [result, setResult] = useState<{ ok: boolean; text: string } | null>(null);

  const load = useCallback(
    async (
      p: number = page,
      ps: number = pageSize,
      preset: TimePreset = timePreset,
      sids: number[] = sessionIds
    ) => {
      setStatus("loading");
      setMsg("");
      // 列表重新加载时收起历史展开（避免展开行指向已换掉的条目）。
      setHistory(EMPTY_HISTORY);
      try {
        const args: Record<string, unknown> = { page: p, pageSize: ps };
        const fromTs = presetFromTs(preset, Math.floor(Date.now() / 1000));
        if (fromTs !== undefined) args.fromTs = fromTs;
        if (sids.length > 0) args.sessionIds = sids;

        const r = await api<SafeBoxPage>("safe_box_list", args);
        setEntries(r.items);
        setTotal(r.total);
        setTotalBytes(r.totalBytes);
        setPage(r.page);
        setPageSize(r.pageSize);
        setStatus("ok");
      } catch (e) {
        setEntries([]);
        setTotal(0);
        setTotalBytes(0);
        setMsg(String(e));
        setStatus("error");
      }
    },
    [page, pageSize, timePreset, sessionIds]
  );

  useEffect(() => {
    // 首次加载：拉一次 sessions 填多选框 + 默认拉全部。
    void (async () => {
      try {
        const ss = await api<SessionSummary[]>("safe_box_list_sessions", {});
        setSessions(ss);
      } catch {
        // 多选框拉失败不阻塞主列表。
      }
      void load(1, 50, "all", []);
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // task-051：切时间预设——page=1、清空选中、收起历史、重拉。
  const onChangePreset = useCallback(
    (next: TimePreset) => {
      setTimePreset(next);
      setSelected(new Set());
      void load(1, pageSize, next, sessionIds);
    },
    [load, pageSize, sessionIds]
  );

  // task-051：多选 session——读 options 的 selected（jsdom 也支持）。
  const onChangeSessions = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      const next = Array.from(e.currentTarget.options)
        .filter((o) => o.selected)
        .map((o) => Number(o.value));
      setSessionIds(next);
      setSelected(new Set());
      void load(1, pageSize, timePreset, next);
    },
    [load, pageSize, timePreset]
  );

  const selectAllSessions = useCallback(() => {
    const all = sessions.map((s) => s.sessionId);
    setSessionIds(all);
    setSelected(new Set());
    void load(1, pageSize, timePreset, all);
  }, [sessions, load, pageSize, timePreset]);

  const clearSessions = useCallback(() => {
    setSessionIds([]);
    setSelected(new Set());
    void load(1, pageSize, timePreset, []);
  }, [load, pageSize, timePreset]);

  // task-045：翻页——清空 selected 避免跨页残留。
  const gotoPage = useCallback(
    (p: number) => {
      setSelected(new Set());
      void load(p, pageSize, timePreset, sessionIds);
    },
    [load, pageSize, timePreset, sessionIds]
  );

  // task-051：历史展开——同一时刻只展开一行，再次点击同一行收起。
  const toggleHistory = useCallback(
    async (item: SafeBoxEntry) => {
      if (history.id === item.id) {
        setHistory(EMPTY_HISTORY);
        return;
      }
      setHistory({ id: item.id, loading: true, rows: [], error: "" });
      try {
        const rows = await api<AuditLogEntry[]>("safe_box_item_history", {
          safeBoxId: item.id,
        });
        // 请求期间用户可能已切到别的行：只回填仍打开的那一行。
        setHistory((prev) =>
          prev.id === item.id ? { id: item.id, loading: false, rows, error: "" } : prev
        );
      } catch (e) {
        setHistory((prev) =>
          prev.id === item.id
            ? { id: item.id, loading: false, rows: [], error: String(e) }
            : prev
        );
      }
    },
    [history.id]
  );

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

  const busy = restoringId !== null || purgingId !== null || batchPurging || batchRestoring;

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 12 }}>
        <button onClick={load} disabled={busy}>
          刷新
        </button>
        <span style={{ fontSize: 13, color: "#374151" }}>
          共 {total} 项 / 总大小 {formatBytes(totalBytes)}
        </span>
        <button
          onClick={restoreMany}
          disabled={selected.size === 0 || busy}
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
          disabled={selected.size === 0 || busy}
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

      {/* task-051：筛选栏——时间预设（互斥）+ 多选 session */}
      <div
        style={{
          display: "flex",
          alignItems: "flex-start",
          gap: 8,
          marginBottom: 12,
          flexWrap: "wrap",
        }}
      >
        <span style={{ fontSize: 12, color: "#6b7280", paddingTop: 6 }}>时间</span>
        {PRESETS.map((p) => {
          const active = timePreset === p.key;
          return (
            <button
              key={p.key}
              onClick={() => onChangePreset(p.key)}
              disabled={status === "loading"}
              style={{
                padding: "4px 10px",
                fontSize: 12,
                borderRadius: 4,
                border: `1px solid ${active ? "#2563eb" : "#d1d5db"}`,
                background: active ? "#2563eb" : "#fff",
                color: active ? "#fff" : "#374151",
                cursor: status === "loading" ? "not-allowed" : "pointer",
              }}
            >
              {p.label}
            </button>
          );
        })}

        <span style={{ fontSize: 12, color: "#6b7280", paddingTop: 6, marginLeft: 8 }}>
          Session
        </span>
        <select
          multiple
          size={4}
          aria-label="session 多选"
          value={sessionIds.map(String)}
          onChange={onChangeSessions}
          style={{ padding: "2px 4px", fontSize: 12, minWidth: 180 }}
        >
          {sessions.map((s) => (
            <option key={s.sessionId} value={String(s.sessionId)}>
              session #{s.sessionId} ({s.itemCount} 条)
            </option>
          ))}
        </select>
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <button onClick={selectAllSessions} style={{ fontSize: 11, padding: "2px 8px" }}>
            全选
          </button>
          <button onClick={clearSessions} style={{ fontSize: 11, padding: "2px 8px" }}>
            清空
          </button>
        </div>
        <span style={{ fontSize: 11, color: "#9ca3af", paddingTop: 6 }}>
          {sessionIds.length === 0
            ? "（未选 = 全部 session）"
            : `已选 ${sessionIds.length} 个 session`}
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
              <th style={{ ...thStyle, width: 32 }}>
                <input
                  type="checkbox"
                  checked={allChecked}
                  onChange={toggleAll}
                  disabled={busy}
                />
              </th>
              <th style={thStyle}>原路径</th>
              <th style={{ ...thStyle, width: 100 }}>大小</th>
              <th style={{ ...thStyle, width: 160 }}>移入时间</th>
              <th style={{ ...thStyle, width: 90 }}>来源</th>
              <th style={{ ...thStyle, width: 190 }}>操作</th>
            </tr>
          </thead>
          <tbody>
            {entries.map((r) => (
              <Fragment key={r.id}>
                <tr>
                  <td style={tdStyle}>
                    <input
                      type="checkbox"
                      checked={selected.has(r.id)}
                      onChange={() => toggleOne(r.id)}
                      disabled={busy}
                    />
                  </td>
                  <td style={{ ...tdStyle, fontFamily: "monospace", fontSize: 12 }}>
                    {r.originalPath}
                  </td>
                  <td style={tdStyle}>{formatBytes(r.sizeBytes)}</td>
                  <td style={tdStyle}>{formatTs(r.quarantinedAt)}</td>
                  <td style={tdStyle}>
                    {r.sourceTaskId == null ? "—" : `#${r.sourceTaskId}`}
                  </td>
                  <td style={tdStyle}>
                    <div style={{ display: "flex", gap: 6 }}>
                      <button
                        onClick={() => restore(r)}
                        disabled={busy}
                        style={{
                          padding: "4px 10px",
                          fontSize: 12,
                          borderRadius: 4,
                          border: "1px solid #2563eb",
                          background: restoringId === r.id ? "#bfdbfe" : "#2563eb",
                          color: "#fff",
                          cursor: busy ? "not-allowed" : "pointer",
                        }}
                      >
                        {restoringId === r.id ? "恢复中…" : "恢复"}
                      </button>
                      <button
                        onClick={() => purge(r)}
                        disabled={busy}
                        style={{
                          padding: "4px 10px",
                          fontSize: 12,
                          borderRadius: 4,
                          border: "1px solid #dc2626",
                          background: purgingId === r.id ? "#fecaca" : "#dc2626",
                          color: "#fff",
                          cursor: busy ? "not-allowed" : "pointer",
                        }}
                      >
                        {purgingId === r.id ? "删除中…" : "永久删除"}
                      </button>
                      <button
                        onClick={() => void toggleHistory(r)}
                        style={{
                          padding: "4px 10px",
                          fontSize: 12,
                          borderRadius: 4,
                          border: "1px solid #6b7280",
                          background: history.id === r.id ? "#e5e7eb" : "#fff",
                          color: "#374151",
                          cursor: "pointer",
                        }}
                      >
                        {history.id === r.id ? "收起" : "历史"}
                      </button>
                    </div>
                  </td>
                </tr>

                {/* task-051：审计历史展开行（同一时刻只展开一行） */}
                {history.id === r.id && (
                  <tr>
                    <td colSpan={6} style={{ background: "#f9fafb", padding: "10px 12px" }}>
                      {history.loading && (
                        <div style={{ fontSize: 12, color: "#6b7280" }}>加载中…</div>
                      )}
                      {!history.loading && history.error !== "" && (
                        <div style={{ fontSize: 12, color: "#b91c1c" }}>
                          历史加载失败：{history.error}
                        </div>
                      )}
                      {!history.loading &&
                        history.error === "" &&
                        history.rows.length === 0 && (
                          <div style={{ fontSize: 12, color: "#9ca3af" }}>
                            该路径暂无审计记录
                          </div>
                        )}
                      {!history.loading &&
                        history.error === "" &&
                        history.rows.length > 0 && (
                          <table style={{ width: "100%", borderCollapse: "collapse" }}>
                            <thead>
                              <tr>
                                <th style={{ ...thStyle, width: 160 }}>时间</th>
                                <th style={{ ...thStyle, width: 110 }}>动作</th>
                                <th style={{ ...thStyle, width: 80 }}>结果</th>
                                <th style={thStyle}>详情</th>
                              </tr>
                            </thead>
                            <tbody>
                              {history.rows.map((h) => (
                                <tr key={h.id}>
                                  <td style={tdStyle}>{formatTs(h.createdAt)}</td>
                                  <td style={tdStyle}>{actionLabel(h.eventType)}</td>
                                  <td style={tdStyle}>{h.result}</td>
                                  <td
                                    style={{
                                      ...tdStyle,
                                      fontFamily: "monospace",
                                      fontSize: 11,
                                      color: "#6b7280",
                                    }}
                                  >
                                    {h.detail ?? "—"}
                                  </td>
                                </tr>
                              ))}
                            </tbody>
                          </table>
                        )}
                    </td>
                  </tr>
                )}
              </Fragment>
            ))}
          </tbody>
        </table>
      )}

      {/* task-045：分页栏（session 过滤已上移到筛选栏） */}
      {status === "ok" && (
        <div
          style={{
            marginTop: 16,
            display: "flex",
            justifyContent: "center",
            alignItems: "center",
            gap: 12,
            fontSize: 13,
            color: "#374151",
          }}
        >
          <button
            onClick={() => gotoPage(page - 1)}
            disabled={page <= 1 || status === "loading"}
          >
            上一页
          </button>
          <span>
            第 {page} / {Math.max(1, Math.ceil(total / pageSize))} 页（共 {total} 条）
          </span>
          <button
            onClick={() => gotoPage(page + 1)}
            disabled={page * pageSize >= total || status === "loading"}
          >
            下一页
          </button>
        </div>
      )}
    </div>
  );
}
