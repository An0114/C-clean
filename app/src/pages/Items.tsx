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
//
// task-049：清理失败从「只报告」升级为「可处置」。
// - cleanup_execute 的返回值不再丢弃，失败项展示 路径 + 原因 + 占用进程；
// - 有占用进程时出现「关闭占用进程并重试」，window.confirm 二次确认后调
//   cleanup_retry_locked（后端 RmShutdown + 护栏过滤 + 复用 execute_items 重删）；
// - 字段全部 camelCase（后端 #[serde(rename_all = "camelCase")]），历史已多次踩
//   snake_case 坑，这里显式对齐。

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

/** task-049：engine::execute::locking::LockingProcess 的 JSON shape。 */
interface LockingProcess {
  pid: number;
  appName: string;
  serviceName: string | null;
}

/** task-049：engine::execute::locking::RefusedProcess（护栏拒绝，带原因）。 */
interface RefusedProcess {
  pid: number;
  appName: string;
  /**
   * "windows service" / "system process" / "self" / "protected process" /
   * "unknown process"（task-049b：取不到进程映像名时的 fail-closed 原因）
   */
  reason: string;
}

/** task-049：engine::execute::executor::ExecResult。 */
interface ExecResult {
  itemId: number;
  path: string;
  deleted: boolean;
  bytesFreed: number;
  error: string | null;
  lockingProcesses: LockingProcess[];
}

/** task-049：engine::execute::executor::ExecuteSummary。 */
interface ExecuteSummary {
  results: ExecResult[];
  deletedCount: number;
  failedCount: number;
  bytesFreedTotal: number;
}

/** task-049：cleanup_retry_locked 响应体。 */
interface RetryLockedResp {
  shutdown: { closed: LockingProcess[]; refused: RefusedProcess[] };
  summary: ExecuteSummary;
}

/** 清理/重试结果条状态（tone 决定绿/红，failures 决定明细与重试按钮）。 */
interface ResultState {
  tone: "ok" | "fail";
  headline: string;
  failures: ExecResult[];
  /** 被护栏拒绝的进程（仅重试路径会有）。 */
  refused: RefusedProcess[];
}

/** `chrome.exe (PID 1234)` —— 与后端 audit_log 的拼法保持一致。 */
const formatProc = (p: { appName: string; pid: number }) => `${p.appName} (PID ${p.pid})`;

/** 把后端稳定错误前缀翻成人话，其余原样展示（不吞信息）。 */
function describeExecError(err: string | null): string {
  if (!err) return "未知原因";
  const inUse = "FILE_IN_USE:";
  const denied = "PERMISSION_DENIED:";
  if (err.startsWith(inUse)) {
    const detail = err.slice(inUse.length).trim();
    return detail ? `文件被占用（${detail}）` : "文件被占用";
  }
  if (err.startsWith(denied)) {
    const detail = err.slice(denied.length).trim();
    return detail ? `权限不足（${detail}）` : "权限不足，可能需要管理员权限";
  }
  return err;
}

/** 去重（同一进程可能占用多个文件），保留首次出现顺序。 */
function dedupeProcs(procs: LockingProcess[]): LockingProcess[] {
  const seen = new Set<number>();
  const out: LockingProcess[] = [];
  for (const p of procs) {
    if (seen.has(p.pid)) continue;
    seen.add(p.pid);
    out.push(p);
  }
  return out;
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
  // task-049：重试态（关闭占用进程 + 重删）。与 cleaning/previewing 互斥。
  const [retrying, setRetrying] = useState(false);
  const [result, setResult] = useState<ResultState | null>(null);
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

  // task-049：三态互斥（清理 / 重试 / 预览 任一为真时，其余操作按钮禁用）。
  const busy = cleaning || retrying || previewing;

  const cleanup = useCallback(async () => {
    if (selected.size === 0 || busy) return;
    setCleaning(true);
    setResult(null);
    // C8：清理成功后清单会变，预览必然过期。
    setPreview(null);
    try {
      const ids = Array.from(selected);
      // task-049 C8：接住返回值，失败项要展示原因与占用进程（此前直接丢弃）。
      const summary = await api<ExecuteSummary>("cleanup_execute", { itemIds: ids });
      const failures = summary.results.filter((r) => !r.deleted);
      setResult({
        tone: failures.length === 0 ? "ok" : "fail",
        headline:
          failures.length === 0
            ? `已清理 ${summary.deletedCount} 项，释放 ${formatBytes(summary.bytesFreedTotal)}`
            : `清理完成：成功 ${summary.deletedCount} 项 / 失败 ${summary.failedCount} 项`,
        failures,
        refused: [],
      });
      setSelected(new Set());
      await load();
    } catch (e) {
      setResult({
        tone: "fail",
        headline: `清理失败：${String(e)}`,
        failures: [],
        refused: [],
      });
    } finally {
      setCleaning(false);
    }
  }, [selected, busy, load]);

  // task-049 C10/C11：确认 → 关闭占用进程 → 自动重试删除。
  // C12：与 cleanup / preview 共用 busy，重试按钮同样纳入三态互斥。
  const retryLocked = useCallback(async () => {
    if (!result || busy) return;
    const locked = result.failures.filter((f) => f.lockingProcesses.length > 0);
    if (locked.length === 0) return;

    const procs = dedupeProcs(locked.flatMap((f) => f.lockingProcesses));
    const list = procs.map(formatProc).join("\n");
    const confirmed = window.confirm(
      `即将请求关闭以下进程：\n${list}\n\n未保存的数据可能丢失，关闭后会自动重试删除。\n注意：同一控制台内的相关进程可能一并关闭。是否继续？`,
    );
    // 取消：直接返回，不调命令，按钮保持可点。
    if (!confirmed) return;

    setRetrying(true);
    try {
      const resp = await api<RetryLockedResp>("cleanup_retry_locked", {
        itemIds: locked.map((f) => f.itemId),
      });
      const failures = resp.summary.results.filter((r) => !r.deleted);
      const closedCount = resp.shutdown.closed.length;
      // close-experience-copy：mixed fail-closed（同批含受保护进程 → 后端整批不关）时
      // closed=0 且 refused 非空，需在结果条里解释「一个都没关」，否则用户只看到
      // 「已关闭 0 个进程，仍失败 N 项」会误以为点了没反应。
      const protectedBlocking = closedCount === 0 && resp.shutdown.refused.length > 0;
      setResult({
        tone: failures.length === 0 ? "ok" : "fail",
        headline:
          failures.length === 0
            ? `已关闭 ${closedCount} 个进程，重试成功 ${resp.summary.deletedCount} 项`
            : protectedBlocking
              ? `已关闭 0 个进程：因存在受保护进程，本次未关闭任何进程，仍失败 ${failures.length} 项`
              : `已关闭 ${closedCount} 个进程，仍失败 ${failures.length} 项`,
        failures,
        refused: resp.shutdown.refused,
      });
      // 无论成败都重拉清单、清空选中与预览。
      setSelected(new Set());
      setPreview(null);
      await load();
    } catch (e) {
      // 失败时保留原失败明细，用户还能再点一次重试。
      setResult((prev) =>
        prev ? { ...prev, tone: "fail", headline: `关闭并重试失败：${String(e)}` } : prev,
      );
    } finally {
      setRetrying(false);
    }
  }, [result, busy, load]);

  // task-040 C2：只读预览。走 apiRaw 以拿回 2002 信封里的 blockedByL3。
  const previewSelection = useCallback(async () => {
    if (selected.size === 0 || busy) return;
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
  }, [selected, busy]);

  return (
    <div>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 12 }}>
        <button onClick={load} disabled={busy}>
          刷新
        </button>
        <span style={{ fontSize: 13, color: "#374151" }}>
          已选 {selected.size} 项，共 {formatBytes(totalSelectedBytes)}
        </span>
        <button
          onClick={previewSelection}
          disabled={selected.size === 0 || busy}
          style={{
            padding: "6px 14px",
            borderRadius: 6,
            border: "1px solid #2563eb",
            background: selected.size === 0 || busy ? "#bfdbfe" : "#2563eb",
            color: "#fff",
            cursor: selected.size === 0 || busy ? "not-allowed" : "pointer",
          }}
        >
          {previewing ? "预览中…" : "预览可清理项"}
        </button>
        <button
          onClick={cleanup}
          disabled={selected.size === 0 || busy}
          style={{
            padding: "6px 14px",
            borderRadius: 6,
            border: "1px solid #dc2626",
            background: selected.size === 0 || busy ? "#fecaca" : "#dc2626",
            color: "#fff",
            cursor: selected.size === 0 || busy ? "not-allowed" : "pointer",
          }}
        >
          {cleaning ? "清理中…" : "清理已选"}
        </button>
      </div>

      {/* task-049：结果条 = 汇总 + 失败明细（原因 + 占用进程）+ 关闭并重试。 */}
      {result && (
        <div
          style={{
            marginBottom: 12,
            padding: 10,
            borderRadius: 6,
            fontSize: 13,
            background: result.tone === "ok" ? "#dcfce7" : "#fee2e2",
            color: result.tone === "ok" ? "#166534" : "#b91c1c",
            border: `1px solid ${result.tone === "ok" ? "#86efac" : "#fca5a5"}`,
          }}
        >
          <div style={{ fontWeight: 600 }}>{result.headline}</div>

          {result.failures.length > 0 && (
            <div style={{ marginTop: 8 }}>
              {result.failures.map((f) => (
                <div key={f.itemId} style={{ marginBottom: 4, lineHeight: 1.5 }}>
                  <span style={{ fontFamily: "monospace", fontSize: 12 }}>
                    {f.path || `#${f.itemId}`}
                  </span>
                  {" — "}
                  {describeExecError(f.error)}
                  {f.lockingProcesses.length > 0 && (
                    <>
                      {" — 占用："}
                      <span style={{ fontFamily: "monospace", fontSize: 12 }}>
                        {f.lockingProcesses.map(formatProc).join(", ")}
                      </span>
                    </>
                  )}
                </div>
              ))}
            </div>
          )}

          {/* 受保护进程必须让用户看见，不能静默跳过。 */}
          {result.refused.length > 0 && (
            <div
              style={{
                marginTop: 8,
                padding: "6px 8px",
                borderRadius: 4,
                background: "#fef9c3",
                color: "#854d0e",
                border: "1px solid #fde047",
              }}
            >
              以下进程受保护未关闭：
              {result.refused.map((r) => `${r.appName}（${r.reason}）`).join("、")}
            </div>
          )}

          {/* 只有存在占用进程时才给重试入口；没有占用就别让用户空点。
              C12：cleaning / retrying / previewing 任一为真时禁用。 */}
          {result.failures.some((f) => f.lockingProcesses.length > 0) && (
            <button
              onClick={retryLocked}
              disabled={busy}
              style={{
                marginTop: 10,
                padding: "6px 14px",
                borderRadius: 6,
                border: "1px solid #b45309",
                background: busy ? "#fde68a" : "#d97706",
                color: "#fff",
                cursor: busy ? "not-allowed" : "pointer",
              }}
            >
              {retrying ? "关闭并重试中…" : "关闭占用进程并重试"}
            </button>
          )}
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
