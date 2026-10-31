// task-068：注册表健康中心（F 模块）——Run 键 / 卸载残留 / 计划任务三分类体检。
//
// 三条安全边界（和 task-067 一脉相承）：
//
// 1. **只有 Run 启动项可删**，且只删「值」不删键。卸载残留删了会影响
//    「应用和功能」的卸载/修复流程；计划任务删除风险更高——两者都只展示，
//    引导用户走各自的正规入口。
// 2. **L3 项没有勾选框**（系统组件 / 安全软件）。后端还会独立复核一次，
//    前端这道只是「别让用户白点」。
// 3. **删前必须备份**，且备份路径要显示给用户——不告诉用户备份在哪，
//    等于没备份。备份失败时后端不会删（fail-closed），UI 会看到失败原因。
//
// 颜色全走 CSS 变量，深色主题自动跟随。

import { useCallback, useEffect, useState } from "react";
import { api, apiRaw } from "../utils/invoke";

/** 与 engine RegistryHealthItem（camelCase）对齐。 */
interface RegistryHealthItem {
  name: string;
  path: string;
  category: string;
  riskLevel: string;
  recommendation: string;
  registryKey: string;
  valueName: string;
  exists: boolean;
  deletable: boolean;
}

/** 与 engine RegistryHealthReport（camelCase）对齐。 */
interface RegistryHealthReport {
  runItems: RegistryHealthItem[];
  uninstallItems: RegistryHealthItem[];
  taskItems: RegistryHealthItem[];
  taskScanAvailable: boolean;
  elapsedMs: number;
}

interface RunTarget {
  root: string;
  subkey: string;
  valueName: string;
}

interface RegistryDeleteFailure {
  name: string;
  reason: string;
}

interface RegistryDeleteResult {
  deleted: string[];
  failed: RegistryDeleteFailure[];
  backups: string[];
}

const RISK_L3 = "L3";

/** 安全红线拒绝（engine::ipc::error_codes::WHITELIST_DENIED）。 */
const CODE_WHITELIST_DENIED = 2002;

type InnerTab = "run" | "uninstall" | "task";

const cardStyle: React.CSSProperties = {
  border: "1px solid var(--border)",
  borderRadius: 8,
  padding: "14px 16px",
  background: "var(--surface, var(--bg))",
};

const thStyle: React.CSSProperties = {
  padding: "8px 12px",
  fontWeight: 500,
  color: "var(--text-muted)",
  textAlign: "left",
  fontSize: 12,
};

const tdStyle: React.CSSProperties = {
  padding: "8px 12px",
  verticalAlign: "top",
};

/** 风险徽标：L3 红（禁删）/ L2 橙（需确认）。 */
function RiskBadge({ level }: { level: string }) {
  const isL3 = level === RISK_L3;
  return (
    <span
      style={{
        display: "inline-block",
        padding: "1px 6px",
        borderRadius: 4,
        fontSize: 11,
        background: isL3 ? "#fee2e2" : "#fef3c7",
        border: `1px solid ${isL3 ? "#fca5a5" : "#fcd34d"}`,
        color: isL3 ? "#b91c1c" : "#92400e",
        whiteSpace: "nowrap",
      }}
      title={isL3 ? "L3：本工具不代删（系统组件或安全软件）" : "L2：删除前需逐项确认"}
    >
      {isL3 ? "L3 禁删" : "L2 需确认"}
    </span>
  );
}

/** 「目标文件已不存在」提示——这是最值得清的一类，要显眼。 */
function MissingBadge() {
  return (
    <span
      style={{
        display: "inline-block",
        padding: "1px 6px",
        borderRadius: 4,
        fontSize: 11,
        background: "#fef2f2",
        border: "1px solid #fecaca",
        color: "#991b1b",
        whiteSpace: "nowrap",
      }}
      title="启动项指向的程序文件已不存在，大概率是卸载残留"
    >
      文件已不存在
    </span>
  );
}

/** 从 `HKCU\Software\...\Run` 拆出 (root, subkey)。 */
function toTarget(item: RegistryHealthItem): RunTarget | null {
  const key = item.registryKey;
  const i = key.indexOf("\\");
  if (i <= 0) return null;
  return {
    root: key.slice(0, i),
    subkey: key.slice(i + 1),
    valueName: item.valueName,
  };
}

/** 勾选键：不同 root/subkey 下可能有同名值，必须带上键路径。 */
function selectKey(item: RegistryHealthItem): string {
  return `${item.registryKey}|${item.valueName}`;
}

export default function RegistryHealth() {
  const [report, setReport] = useState<RegistryHealthReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const [tab, setTab] = useState<InnerTab>("run");
  const [selected, setSelected] = useState<Set<string>>(new Set());

  const [deleting, setDeleting] = useState(false);
  const [delResult, setDelResult] = useState<RegistryDeleteResult | null>(null);
  const [delError, setDelError] = useState("");

  const scan = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const r = await api<RegistryHealthReport>("registry_health_scan", {});
      setReport(r);
      setSelected(new Set());
    } catch (e) {
      setReport(null);
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void scan();
  }, [scan]);

  const toggle = useCallback((key: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }, []);

  const doDelete = useCallback(async () => {
    if (!report) return;
    // 从当前报告里按勾选键反查 item，避免把 UI 状态当数据源
    const picked = report.runItems.filter((i) => selected.has(selectKey(i)));
    if (picked.length === 0) return;

    const targets: RunTarget[] = [];
    for (const i of picked) {
      const t = toTarget(i);
      if (t) targets.push(t);
    }
    if (targets.length === 0) return;

    const list = picked.map((i) => `· ${i.name}`).join("\n");
    const ok = window.confirm(
      `将删除以下 ${targets.length} 个开机启动项：\n\n${list}\n\n` +
        "删除前会自动导出 .reg 备份到「数据目录\\registry_backups\\」，双击备份即可还原。确认继续？"
    );
    if (!ok) return;

    setDeleting(true);
    setDelError("");
    setDelResult(null);
    try {
      const env = await apiRaw<RegistryDeleteResult>("registry_health_delete", { targets });
      if (env.code === CODE_WHITELIST_DENIED) {
        setDelError("全部被安全红线拒绝：勾选项里有系统组件或安全软件启动项（L3）");
        return;
      }
      if (env.code !== 0) {
        setDelError(env.message);
        return;
      }
      setDelResult(env.data);
      await scan();
    } catch (e) {
      setDelError(String(e));
    } finally {
      setDeleting(false);
    }
  }, [report, selected, scan]);

  const runCount = report?.runItems.length ?? 0;
  const uninstallCount = report?.uninstallItems.length ?? 0;
  const taskCount = report?.taskItems.length ?? 0;
  const missingCount = report?.runItems.filter((i) => !i.exists).length ?? 0;

  return (
    <main
      style={{
        padding: 24,
        fontFamily: "system-ui, sans-serif",
        background: "var(--bg)",
        minHeight: "100vh",
        boxSizing: "border-box",
        color: "var(--text)",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "baseline",
          justifyContent: "space-between",
          flexWrap: "wrap",
          gap: 8,
        }}
      >
        <div>
          <h1 style={{ margin: "0 0 4px" }}>注册表健康</h1>
          <p style={{ color: "var(--text-muted)", margin: 0, fontSize: 13 }}>
            开机启动项 / 卸载残留 / 计划任务体检。
            <strong>只有启动项可以代删</strong>，且删前自动导出 .reg 备份。
          </p>
        </div>
        <button
          onClick={scan}
          disabled={loading || deleting}
          style={{
            padding: "8px 16px",
            fontSize: 13,
            borderRadius: 6,
            border: "1px solid var(--border)",
            background: "var(--bg)",
            color: "var(--text)",
            cursor: loading || deleting ? "not-allowed" : "pointer",
          }}
        >
          {loading ? "扫描中…" : "重新扫描"}
        </button>
      </div>

      {loading && (
        <div style={{ marginTop: 16, fontSize: 13, color: "var(--text-muted)" }}>
          正在读取注册表与计划任务…
        </div>
      )}

      {error && (
        <div
          style={{
            marginTop: 16,
            background: "#fee2e2",
            border: "1px solid #fca5a5",
            color: "#b91c1c",
            padding: 12,
            borderRadius: 8,
            fontSize: 13,
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: 4 }}>扫描失败</div>
          <div style={{ wordBreak: "break-all" }}>{error}</div>
        </div>
      )}

      {report && (
        <>
          <div style={{ marginTop: 14, fontSize: 12, color: "var(--text-muted)" }}>
            耗时 {report.elapsedMs} ms
            {missingCount > 0 && (
              <span style={{ color: "#b45309" }}>
                {" "}
                · 其中 {missingCount} 个启动项指向的文件已不存在（疑似卸载残留）
              </span>
            )}
          </div>

          <div style={{ marginTop: 12, display: "flex", gap: 8 }}>
            <TabButton active={tab === "run"} onClick={() => setTab("run")}>
              启动项（{runCount}）
            </TabButton>
            <TabButton active={tab === "uninstall"} onClick={() => setTab("uninstall")}>
              卸载残留（{uninstallCount}）
            </TabButton>
            <TabButton active={tab === "task"} onClick={() => setTab("task")}>
              计划任务（{taskCount}）
            </TabButton>
          </div>

          {tab === "run" && (
            <RunTab items={report.runItems} selected={selected} onToggle={toggle} />
          )}
          {tab === "uninstall" && <ReadOnlyTab items={report.uninstallItems} kind="uninstall" />}
          {tab === "task" && (
            <ReadOnlyTab
              items={report.taskItems}
              kind="task"
              unavailableNote={
                report.taskScanAvailable
                  ? undefined
                  : "读取计划任务需要管理员权限（C:\\Windows\\System32\\Tasks 默认只有管理员可读）。" +
                    "以管理员身份运行本程序才能看到这一栏；本工具不代删计划任务，也可直接用「任务计划程序」查看。"
              }
            />
          )}

          {tab === "run" && selected.size > 0 && (
            <div
              style={{
                ...cardStyle,
                marginTop: 12,
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: 12,
                flexWrap: "wrap",
              }}
            >
              <div style={{ fontSize: 13 }}>
                已选 <strong>{selected.size}</strong> 项
              </div>
              <button
                onClick={doDelete}
                disabled={deleting}
                style={{
                  padding: "8px 16px",
                  fontSize: 13,
                  borderRadius: 6,
                  border: "1px solid #b45309",
                  background: deleting ? "#fde68a" : "#f59e0b",
                  color: deleting ? "#92400e" : "#fff",
                  cursor: deleting ? "not-allowed" : "pointer",
                }}
              >
                {deleting ? "删除中…" : `删除并备份（${selected.size} 项）`}
              </button>
            </div>
          )}

          {delError && (
            <div
              style={{
                marginTop: 12,
                background: "#fee2e2",
                border: "1px solid #fca5a5",
                color: "#b91c1c",
                padding: 12,
                borderRadius: 8,
                fontSize: 13,
              }}
            >
              <div style={{ fontWeight: 600, marginBottom: 4 }}>未能删除</div>
              <div style={{ wordBreak: "break-all" }}>{delError}</div>
            </div>
          )}

          {delResult && (
            <div
              style={{
                marginTop: 12,
                background: delResult.failed.length === 0 ? "#ecfdf5" : "#fffbeb",
                border: `1px solid ${delResult.failed.length === 0 ? "#6ee7b7" : "#fcd34d"}`,
                color: delResult.failed.length === 0 ? "#065f46" : "#92400e",
                padding: 12,
                borderRadius: 8,
                fontSize: 13,
              }}
            >
              <div style={{ fontWeight: 600, marginBottom: 4 }}>
                已删除 {delResult.deleted.length} 个启动项
              </div>
              {/* 备份路径必须显示：不告诉用户备份在哪，等于没备份 */}
              {delResult.backups.length > 0 && (
                <div style={{ marginTop: 4 }}>
                  <div style={{ fontWeight: 600 }}>备份文件（双击即可还原）：</div>
                  {delResult.backups.map((b) => (
                    <div
                      key={b}
                      style={{
                        fontFamily: "ui-monospace, SFMono-Regular, Consolas, monospace",
                        fontSize: 11,
                        wordBreak: "break-all",
                        marginTop: 2,
                      }}
                    >
                      {b}
                    </div>
                  ))}
                </div>
              )}
              {delResult.failed.length > 0 && (
                <div style={{ marginTop: 6 }}>
                  <div style={{ fontWeight: 600 }}>失败 {delResult.failed.length} 项：</div>
                  {delResult.failed.map((f) => (
                    <div key={f.name} style={{ wordBreak: "break-all", marginTop: 2 }}>
                      {f.name} — {f.reason}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </>
      )}
    </main>
  );
}

function TabButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      style={{
        padding: "7px 14px",
        fontSize: 13,
        borderRadius: 6,
        border: `1px solid ${active ? "#2563eb" : "var(--border)"}`,
        background: active ? "#2563eb" : "var(--bg)",
        color: active ? "#fff" : "var(--text)",
        cursor: "pointer",
      }}
    >
      {children}
    </button>
  );
}

/** 启动项：可勾选（L3 除外）→ 删除并备份。 */
function RunTab({
  items,
  selected,
  onToggle,
}: {
  items: RegistryHealthItem[];
  selected: Set<string>;
  onToggle: (key: string) => void;
}) {
  if (items.length === 0) {
    return (
      <div style={{ ...cardStyle, marginTop: 12, fontSize: 13, color: "var(--text-muted)" }}>
        没有开机启动项。
      </div>
    );
  }
  return (
    <Table
      head={
        <>
          <th style={{ ...thStyle, width: 36 }}>选</th>
          <th style={thStyle}>名称</th>
          <th style={thStyle}>命令</th>
          <th style={{ ...thStyle, width: 130 }}>风险</th>
          <th style={thStyle}>建议</th>
        </>
      }
    >
      {items.map((i) => {
        const key = selectKey(i);
        return (
          <tr key={key} style={{ borderTop: "1px solid var(--border)" }}>
            <td style={tdStyle}>
              {/* L3 禁删：不给勾选框，从交互上就断掉 */}
              {i.deletable ? (
                <input
                  type="checkbox"
                  aria-label={`选择 ${i.name}`}
                  checked={selected.has(key)}
                  onChange={() => onToggle(key)}
                />
              ) : (
                <span title="L3 禁删" style={{ color: "var(--text-muted)", fontSize: 11 }}>
                  —
                </span>
              )}
            </td>
            <td style={{ ...tdStyle, fontWeight: 500 }}>
              {i.name}
              {!i.exists && (
                <div style={{ marginTop: 3 }}>
                  <MissingBadge />
                </div>
              )}
            </td>
            <td
              style={{
                ...tdStyle,
                wordBreak: "break-all",
                fontFamily: "ui-monospace, SFMono-Regular, Consolas, monospace",
                fontSize: 11,
              }}
              title={i.registryKey}
            >
              {i.path}
            </td>
            <td style={tdStyle}>
              <RiskBadge level={i.riskLevel} />
            </td>
            <td style={{ ...tdStyle, color: "var(--text-muted)", lineHeight: 1.6 }}>
              {i.recommendation}
            </td>
          </tr>
        );
      })}
    </Table>
  );
}

/** 卸载残留 / 计划任务：只读，没有勾选框也没有删除按钮。 */
function ReadOnlyTab({
  items,
  kind,
  unavailableNote,
}: {
  items: RegistryHealthItem[];
  kind: "uninstall" | "task";
  unavailableNote?: string;
}) {
  if (unavailableNote) {
    return (
      <div
        style={{
          ...cardStyle,
          marginTop: 12,
          background: "#fffbeb",
          border: "1px solid #fcd34d",
          color: "#92400e",
          fontSize: 13,
          lineHeight: 1.7,
        }}
      >
        {unavailableNote}
      </div>
    );
  }
  if (items.length === 0) {
    return (
      <div style={{ ...cardStyle, marginTop: 12, fontSize: 13, color: "var(--text-muted)" }}>
        {kind === "uninstall" ? "没有发现卸载残留。" : "没有发现计划任务。"}
      </div>
    );
  }

  return (
    <div style={{ ...cardStyle, marginTop: 12, padding: 0, overflow: "hidden" }}>
      <div
        style={{
          padding: "12px 16px",
          borderBottom: "1px solid var(--border)",
          background: "#eff6ff",
          color: "#1e40af",
          fontSize: 12,
          lineHeight: 1.7,
        }}
      >
        {kind === "uninstall" ? (
          <>
            注册表里有卸载项，但它记录的安装目录已不存在。
            <strong>本工具不代删注册表键</strong>——删了会影响「应用和功能」的卸载/修复流程。
          </>
        ) : (
          <>
            计划任务列表。
            <strong>本工具不代删计划任务</strong>——任务可能是系统组件，删错影响面比启动项大得多。
          </>
        )}
      </div>
      <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
        <thead>
          <tr>
            <th style={thStyle}>名称</th>
            <th style={thStyle}>路径</th>
            <th style={{ ...thStyle, width: 100 }}>风险</th>
            <th style={thStyle}>建议</th>
          </tr>
        </thead>
        <tbody>
          {items.map((i) => (
            <tr key={`${i.registryKey}|${i.name}`} style={{ borderTop: "1px solid var(--border)" }}>
              <td style={{ ...tdStyle, fontWeight: 500 }}>
                {i.name}
                {!i.exists && kind === "task" && (
                  <div style={{ marginTop: 3 }}>
                    <MissingBadge />
                  </div>
                )}
              </td>
              <td style={{ ...tdStyle, wordBreak: "break-all", fontSize: 11 }} title={i.registryKey}>
                {i.path}
              </td>
              <td style={tdStyle}>
                <RiskBadge level={i.riskLevel} />
              </td>
              <td style={{ ...tdStyle, color: "var(--text-muted)", lineHeight: 1.6 }}>
                {i.recommendation}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Table({ head, children }: { head: React.ReactNode; children: React.ReactNode }) {
  return (
    <div style={{ ...cardStyle, marginTop: 12, padding: 0, overflow: "hidden" }}>
      <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
        <thead>
          <tr>{head}</tr>
        </thead>
        <tbody>{children}</tbody>
      </table>
    </div>
  );
}
