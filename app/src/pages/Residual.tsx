// task-067：残留增强（D 模块）——注册表卸载列表 ↔ 文件目录交叉比对。
//
// 三个关键设计决定：
//
// 1. **两类残留分两个 tab，删除能力只给一类**。类型 A（注册表残留）只展示 +
//    引导手动处理：本工具不代删注册表键（改错一个键可能让系统起不来）。
//    类型 B（文件残留）可以移入保险箱——但见下一条。
//
// 2. **类型 B 默认不勾选、不提供「全选」**。它的判定依据是「注册表里找不到
//    对应卸载项」，而绿色软件、便携版、商店应用、无 InstallLocation 的程序
//    全都会落进来（本机实测 11 条里就有几个是正常安装）。全选按钮等于鼓励
//    用户批量误删，所以只给逐项勾选。
//
// 3. **移入保险箱而不是直接删**。目录整体 rename 进保险箱，结构完整保留，
//    7 天内可在「保险箱」页恢复；删除前还会过引擎的唯一入口红线校验。
//
// 颜色全走 CSS 变量，深色主题自动跟随。

import { useCallback, useEffect, useState } from "react";
import { formatBytes } from "../utils/formatBytes";
import { api, apiRaw } from "../utils/invoke";

/** 与 engine ResidualItem（camelCase）对齐。 */
interface ResidualItem {
  name: string;
  path: string;
  residualType: string;
  sizeBytes: number;
  registryKey: string | null;
  confidence: string;
  riskLevel: string;
  note: string;
}

/** 与 engine ResidualReport（camelCase）对齐。 */
interface ResidualReport {
  registryItems: ResidualItem[];
  fileItems: ResidualItem[];
  totalBytes: number;
  registryEntries: number;
  dirsScanned: number;
  sizeTruncated: boolean;
  elapsedMs: number;
}

interface ResidualDeleteFailure {
  path: string;
  reason: string;
}

interface ResidualDeleteResult {
  moved: string[];
  failed: ResidualDeleteFailure[];
  bytesMoved: number;
}

/** 与 engine::scan::residual 的常量对齐。 */
const TYPE_REGISTRY = "registry";
const TYPE_FILES = "files";

/** 还原点创建失败同款的「安全红线拒绝」错误码（engine::ipc::error_codes）。 */
const CODE_WHITELIST_DENIED = 2002;

/** 残留定位失败（task-084 A4）：目标路径已不存在。 */
const CODE_LOCATE_NOT_FOUND = 2008;

type InnerTab = "files" | "registry";

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

/** 残留类型 → 中文标签。取值与 engine 的 `RESIDUAL_REGISTRY` / `RESIDUAL_FILES` 对齐。 */
function typeLabel(residualType: string): string {
  if (residualType === TYPE_REGISTRY) return "注册表残留";
  if (residualType === TYPE_FILES) return "文件残留";
  // 后端将来加类型不会白屏，原样显示
  return residualType || "—";
}

/** 置信度徽标：高 = 绿，低 = 橙（低置信必须视觉上就「不安心」）。 */
function ConfidenceBadge({ value }: { value: string }) {
  const high = value === "high";
  // task-082：徽标只有「高/低」两个字，光看颜色说明不了什么。hover 给判定依据——
  // 这两句话直接对应 `engine::scan::residual` 的两个分支：
  //   high = 注册表记了安装目录、但目录已不存在（记录可信）；
  //   low  = 目录在、注册表找不到归属（绿色软件/便携版/自建目录都会落到这里）。
  const hint = high
    ? "高置信：注册表里记录了安装目录，但该目录已不存在——确实已卸载或半卸载。本工具不代删注册表键，需清理请到「应用和功能」修复或卸载"
    : "低置信：目录还在，但注册表里找不到归属。绿色软件、便携版、商店应用、用户自建目录都会落到这里，可能误判——删除前请先确认目录内容（会移入保险箱保留 7 天）";
  return (
    <span
      title={hint}
      style={{
        display: "inline-block",
        padding: "1px 6px",
        borderRadius: 4,
        fontSize: 11,
        background: high ? "#ecfdf5" : "#fffbeb",
        border: `1px solid ${high ? "#6ee7b7" : "#fcd34d"}`,
        color: high ? "#065f46" : "#92400e",
        whiteSpace: "nowrap",
        cursor: "help",
      }}
    >
      {high ? "高置信" : "低置信"}
    </span>
  );
}

/** 风险等级徽标（L2 = 需逐项确认）。 */
function RiskBadge({ level }: { level: string }) {
  if (!level) return null;
  return (
    <span
      style={{
        display: "inline-block",
        padding: "1px 6px",
        borderRadius: 4,
        fontSize: 11,
        background: "#fef3c7",
        border: "1px solid #fcd34d",
        color: "#92400e",
        whiteSpace: "nowrap",
      }}
      title="L2：删除前需要逐项确认"
    >
      {level} 需确认
    </span>
  );
}

export default function Residual() {
  const [report, setReport] = useState<ResidualReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const [tab, setTab] = useState<InnerTab>("files");
  // 用 Set 存路径：勾选状态按「路径」而不是索引，扫描刷新后不会错位
  const [selected, setSelected] = useState<Set<string>>(new Set());

  const [deleting, setDeleting] = useState(false);
  const [deleteResult, setDeleteResult] = useState<ResidualDeleteResult | null>(null);
  const [deleteError, setDeleteError] = useState("");

  // task-084 A4：一键定位。locatingPath 是「当前正在定位的那条」，
  // 同一时刻只放行一次 explorer 调起，避免连点轰炸。
  const [locatingPath, setLocatingPath] = useState<string | null>(null);
  const [locateError, setLocateError] = useState("");

  const scan = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const r = await api<ResidualReport>("residual_scan", {});
      setReport(r);
      // 扫描后清空勾选：列表变了，旧勾选可能指向已消失的项
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

  const toggle = useCallback((path: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  }, []);

  const doDelete = useCallback(async () => {
    const list = [...selected];
    if (list.length === 0) return;

    const preview = list.slice(0, 5).map((p) => `· ${p}`).join("\n");
    const more = list.length > 5 ? `\n… 等共 ${list.length} 项` : "";
    const ok = window.confirm(
      `将把以下目录移入保险箱（7 天内可在「保险箱」页恢复）：\n\n${preview}${more}\n\n` +
        "判断依据是「注册表里找不到对应卸载项」，可能误判（绿色软件、便携版、商店应用都会落进来）。确认继续？"
    );
    if (!ok) return;

    setDeleting(true);
    setDeleteError("");
    setDeleteResult(null);
    try {
      const env = await apiRaw<ResidualDeleteResult>("residual_delete", { paths: list });
      if (env.code === CODE_WHITELIST_DENIED) {
        setDeleteError("全部被安全红线拒绝：勾选的路径里有受保护目录（桌面 / 文档 / 系统目录等）");
        return;
      }
      if (env.code !== 0) {
        setDeleteError(env.message);
        return;
      }
      setDeleteResult(env.data);
      // 刷新列表：移走的目录应该从表里消失
      await scan();
    } catch (e) {
      setDeleteError(String(e));
    } finally {
      setDeleting(false);
    }
  }, [selected, scan]);

  // task-084 A4：调起资源管理器并选中该路径。
  // 后端（residual_locate）会先校验路径是否存在：
  //   - 存在 → explorer.exe /select 打开并选中；
  //   - 已不存在 → 2008，提示并刷新列表（残留可能已在别处被清掉）。
  // 因为要区分「正常」和「2008」，这里用 apiRaw 而非 api（api 会在非 0 时抛错）。
  const locate = useCallback(
    async (path: string) => {
      setLocatingPath(path);
      setLocateError("");
      try {
        const env = await apiRaw<unknown>("residual_locate", { path });
        if (env.code === CODE_LOCATE_NOT_FOUND) {
          setLocateError(`「${path}」已不存在（可能已被清理），列表已刷新。`);
          // 刷新会让这条从列表里消失，顺便把定位状态复位
          await scan();
        } else if (env.code !== 0) {
          setLocateError(env.message);
        }
      } catch (e) {
        setLocateError(String(e));
      } finally {
        setLocatingPath(null);
      }
    },
    [scan]
  );

  const selectedBytes = report
    ? report.fileItems
        .filter((i) => selected.has(i.path))
        .reduce((s, i) => s + i.sizeBytes, 0)
    : 0;

  const registryCount = report?.registryItems.length ?? 0;
  const fileCount = report?.fileItems.length ?? 0;

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
          <h1 style={{ margin: "0 0 4px" }}>残留扫描</h1>
          <p style={{ color: "var(--text-muted)", margin: 0, fontSize: 13 }}>
            比对「注册表卸载列表」与「安装目录」，找出两类残留。
            <strong>注册表键本工具永不代删</strong>，文件残留只能移入保险箱。
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
          正在读取注册表并扫描安装目录…
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
            读取 {report.registryEntries} 个卸载项 · 扫描 {report.dirsScanned} 个安装目录 · 耗时{" "}
            {report.elapsedMs} ms
            {report.sizeTruncated && (
              <span style={{ color: "#b45309" }}> · 部分目录未统计大小（超出预算）</span>
            )}
          </div>

          {/* 两个内部分类 tab */}
          <div style={{ marginTop: 12, display: "flex", gap: 8 }}>
            <TabButton active={tab === "files"} onClick={() => setTab("files")}>
              文件残留（{fileCount}）
            </TabButton>
            <TabButton active={tab === "registry"} onClick={() => setTab("registry")}>
              注册表残留（{registryCount}）
            </TabButton>
          </div>

          {tab === "files" && (
            <FileTab
              items={report.fileItems}
              selected={selected}
              onToggle={toggle}
              totalBytes={report.totalBytes}
              onLocate={locate}
              locatingPath={locatingPath}
            />
          )}

          {tab === "registry" && <RegistryTab items={report.registryItems} />}

          {locateError && (
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
              <div style={{ fontWeight: 600, marginBottom: 4 }}>未能定位</div>
              <div style={{ wordBreak: "break-all" }}>{locateError}</div>
            </div>
          )}

          {/* 操作条：只有文件残留 tab 且勾了东西才出现 */}
          {tab === "files" && selected.size > 0 && (
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
                已选 <strong>{selected.size}</strong> 项，共 {formatBytes(selectedBytes)}
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
                {deleting ? "移入中…" : `移入保险箱（${selected.size} 项）`}
              </button>
            </div>
          )}

          {deleteError && (
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
              <div style={{ fontWeight: 600, marginBottom: 4 }}>未能移入保险箱</div>
              <div style={{ wordBreak: "break-all" }}>{deleteError}</div>
            </div>
          )}

          {deleteResult && (
            <div
              style={{
                marginTop: 12,
                background: deleteResult.failed.length === 0 ? "#ecfdf5" : "#fffbeb",
                border: `1px solid ${deleteResult.failed.length === 0 ? "#6ee7b7" : "#fcd34d"}`,
                color: deleteResult.failed.length === 0 ? "#065f46" : "#92400e",
                padding: 12,
                borderRadius: 8,
                fontSize: 13,
              }}
            >
              <div style={{ fontWeight: 600, marginBottom: 4 }}>
                已移入保险箱 {deleteResult.moved.length} 项，释放 {formatBytes(deleteResult.bytesMoved)}
              </div>
              <div style={{ color: "var(--text-muted)" }}>
                7 天内可在「保险箱」页恢复，到期自动永久删除。
              </div>
              {deleteResult.failed.length > 0 && (
                <div style={{ marginTop: 6 }}>
                  <div style={{ fontWeight: 600 }}>失败 {deleteResult.failed.length} 项：</div>
                  {deleteResult.failed.map((f) => (
                    <div key={f.path} style={{ wordBreak: "break-all", marginTop: 2 }}>
                      {f.path} — {f.reason}
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

/** 文件残留：可勾选 → 移入保险箱；每条可一键定位到资源管理器。 */
function FileTab({
  items,
  selected,
  onToggle,
  totalBytes,
  onLocate,
  locatingPath,
}: {
  items: ResidualItem[];
  selected: Set<string>;
  onToggle: (path: string) => void;
  totalBytes: number;
  onLocate: (path: string) => void;
  locatingPath: string | null;
}) {
  if (items.length === 0) {
    return (
      <div style={{ ...cardStyle, marginTop: 12, fontSize: 13, color: "var(--text-muted)" }}>
        没有发现文件残留。安装目录里的每个文件夹都能在注册表里找到归属。
      </div>
    );
  }

  return (
    <div style={{ ...cardStyle, marginTop: 12, padding: 0, overflow: "hidden" }}>
      <div
        style={{
          padding: "12px 16px",
          borderBottom: "1px solid var(--border)",
          fontSize: 13,
          display: "flex",
          justifyContent: "space-between",
          gap: 12,
          flexWrap: "wrap",
        }}
      >
        <span style={{ fontWeight: 600 }}>疑似孤儿文件夹</span>
        <span style={{ color: "var(--text-muted)", fontSize: 12 }}>
          合计 {formatBytes(totalBytes)} · 判断依据是「注册表无对应卸载项」，可能误判
        </span>
      </div>
      <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
        <thead>
          <tr>
            <th style={{ ...thStyle, width: 36 }}>选</th>
            <th style={thStyle}>名称</th>
            <th style={thStyle}>路径</th>
            <th style={{ ...thStyle, width: 90, textAlign: "right" }}>大小</th>
            <th style={{ ...thStyle, width: 150 }}>类型 / 置信</th>
            <th style={{ ...thStyle, width: 64 }}>定位</th>
          </tr>
        </thead>
        <tbody>
          {items.map((i) => (
            <tr key={i.path} style={{ borderTop: "1px solid var(--border)" }}>
              <td style={tdStyle}>
                <input
                  type="checkbox"
                  aria-label={`选择 ${i.name}`}
                  checked={selected.has(i.path)}
                  onChange={() => onToggle(i.path)}
                />
              </td>
              <td style={{ ...tdStyle, fontWeight: 500 }}>{i.name}</td>
              <td style={{ ...tdStyle, wordBreak: "break-all" }} title={i.note}>
                {i.path}
              </td>
              <td style={{ ...tdStyle, textAlign: "right", whiteSpace: "nowrap" }}>
                {formatBytes(i.sizeBytes)}
              </td>
              <td style={tdStyle}>
                <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
                    {typeLabel(i.residualType)}
                  </span>
                  <ConfidenceBadge value={i.confidence} />
                  <RiskBadge level={i.riskLevel} />
                </div>
              </td>
              <td style={tdStyle}>
                <button
                  onClick={() => onLocate(i.path)}
                  disabled={locatingPath !== null}
                  aria-label={`定位 ${i.name}`}
                  title="在资源管理器中打开并选中该目录"
                  style={{
                    padding: "3px 10px",
                    fontSize: 12,
                    borderRadius: 5,
                    border: "1px solid var(--border)",
                    background: "var(--bg)",
                    color: "var(--text)",
                    cursor: locatingPath !== null ? "not-allowed" : "pointer",
                    whiteSpace: "nowrap",
                  }}
                >
                  {locatingPath === i.path ? "…" : "定位"}
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** 注册表残留：只展示 + 引导手动处理，**没有勾选框也没有删除按钮**。 */
// 也没有「定位」按钮：类型 A 的失效路径**按定义已不存在**，点了必然 2008。
// 放一个「每次点都报错」的按钮比不放大得多，所以只给文件残留 tab 提供定位。
function RegistryTab({ items }: { items: ResidualItem[] }) {
  if (items.length === 0) {
    return (
      <div style={{ ...cardStyle, marginTop: 12, fontSize: 13, color: "var(--text-muted)" }}>
        没有发现注册表残留。所有记录的安装目录都还在。
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
        注册表里有卸载项，但它记录的安装目录已不存在。
        <strong>本工具不代删注册表键</strong>
        ——改错一个键可能让程序甚至系统起不来。请用「应用和功能」修复/卸载，
        或用注册表编辑器手动处理（列表里给了键路径）。
      </div>
      <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
        <thead>
          <tr>
            <th style={thStyle}>名称</th>
            <th style={thStyle}>失效路径</th>
            <th style={thStyle}>注册表键</th>
            <th style={{ ...thStyle, width: 110 }}>类型 / 置信</th>
          </tr>
        </thead>
        <tbody>
          {items.map((i) => (
            <tr key={i.path} style={{ borderTop: "1px solid var(--border)" }}>
              <td style={{ ...tdStyle, fontWeight: 500 }}>{i.name}</td>
              <td style={{ ...tdStyle, wordBreak: "break-all" }} title={i.note}>
                {i.path}
              </td>
              <td
                style={{
                  ...tdStyle,
                  wordBreak: "break-all",
                  color: "var(--text-muted)",
                  fontSize: 11,
                }}
              >
                {i.registryKey ?? "—"}
              </td>
              <td style={tdStyle}>
                <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                  <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
                    {typeLabel(i.residualType)}
                  </span>
                  <ConfidenceBadge value={i.confidence} />
                  <RiskBadge level={i.riskLevel} />
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
