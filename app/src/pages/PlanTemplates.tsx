// task-072：清理方案模板（P1-7）——把常清的那批条目存成命名方案，下次一键勾选。
//
// 四个关键设计决定：
//
// 1. **方案存的是路径三元组，不是条目 id**。`cleanup_plan.item_refs` 存
//    `{moduleCode, category, path}`，应用时由后端按三元组回查当前库拿 id。
//    所以「重新扫描后方案仍可用」是设计使然，不是巧合——界面上要说清这一点，
//    否则用户会担心「重扫之后方案是不是就废了」。
//
// 2. **「本次匹配」与「方案条目数」分开显示**。方案存了 12 条、这次只匹配到 3 条
//    时，必须写「12 条（本次匹配 3 条）」而不是只显示 3——只显示 3 会让人以为
//    方案坏了、数据丢了。
//
// 3. **保存用的是「清理」页当前勾选**。方案页自己不产生选择，所以按钮上直接写
//    清「（N 项）」；N 为 0 时禁用并提示先去清理页勾选，而不是点了没反应。
//
// 4. **导出到固定目录、不弹「另存为」**。本项目没引文件选择框插件，导出后把
//    绝对路径显示出来让用户复制；导入是把路径粘回来。这是能力边界，界面上要
//    写明白，别让用户去找一个不存在的「浏览…」按钮。
//
// 颜色全走 CSS 变量，深色主题自动跟随。

import { useCallback, useEffect, useState } from "react";
import { formatBytes } from "../utils/formatBytes";
import { formatTs } from "../utils/formatTimestamp";
import { api, apiRaw } from "../utils/invoke";
import { t } from "../i18n";

/** 与 `engine::ipc::commands::plan::PlanSummary`（camelCase）对齐。 */
export interface PlanSummary {
  id: number;
  name: string;
  note: string | null;
  /** 方案里存了多少条引用（稳定值）。 */
  itemCount: number;
  /** 本次在库里匹配到多少条（可能小于 itemCount）。 */
  matchedCount: number;
  sizeBytes: number;
  createdAt: number;
  updatedAt: number;
}

/** 与 `PlanApplied`（camelCase）对齐。 */
export interface PlanApplied {
  id: number;
  name: string;
  itemIds: number[];
  unmatched: string[];
  sizeBytes: number;
}

/** 应用方案后交给 App 的载荷：要让「清理」页恢复勾选并说明发生了什么。 */
export interface PlanApplyPayload {
  itemIds: number[];
  /** 给用户看的一句话（含「有 N 条本次没找到」这类信息）。 */
  notice: string;
}

const cardStyle: React.CSSProperties = {
  background: "var(--card)",
  border: "1px solid var(--border)",
  borderRadius: 8,
  padding: 16,
};

const inputStyle: React.CSSProperties = {
  padding: "7px 10px",
  fontSize: 13,
  borderRadius: 6,
  border: "1px solid var(--border)",
  background: "var(--bg)",
  color: "var(--text)",
};

const btnStyle = (disabled: boolean): React.CSSProperties => ({
  padding: "7px 14px",
  fontSize: 13,
  borderRadius: 6,
  border: `1px solid ${disabled ? "var(--border)" : "#2563eb"}`,
  background: disabled ? "var(--bg)" : "#2563eb",
  color: disabled ? "var(--text-muted)" : "#fff",
  cursor: disabled ? "not-allowed" : "pointer",
});

const ghostBtnStyle = (disabled = false): React.CSSProperties => ({
  padding: "4px 10px",
  fontSize: 12,
  borderRadius: 5,
  border: "1px solid var(--border)",
  background: "var(--bg)",
  color: disabled ? "var(--text-muted)" : "var(--text)",
  cursor: disabled ? "not-allowed" : "pointer",
});

const dangerBtnStyle: React.CSSProperties = {
  padding: "4px 10px",
  fontSize: 12,
  borderRadius: 5,
  border: "1px solid #fca5a5",
  background: "#fee2e2",
  color: "#b91c1c",
  cursor: "pointer",
};

interface Props {
  /** 「清理」页当前勾选的条目 id（由 App 从 Items 页收集）。 */
  currentSelection: number[];
  /** 应用方案：交给 App 去切页并让 Items 恢复勾选。 */
  onApply: (payload: PlanApplyPayload) => void;
}

export default function PlanTemplates({ currentSelection, onApply }: Props) {
  const [plans, setPlans] = useState<PlanSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  /** 导出后的绝对路径单独存：要用等宽字体展示、方便复制。 */
  const [exportedPath, setExportedPath] = useState("");

  const [nameInput, setNameInput] = useState("");
  const [importPath, setImportPath] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const resp = await api<PlanSummary[]>("plan_list", {});
      setPlans(resp ?? []);
    } catch (e) {
      setPlans([]);
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  // ---- 保存当前选择 ----
  const save = useCallback(async () => {
    const name = nameInput.trim();
    if (!name) {
      setError("请先给方案起个名字");
      return;
    }
    setBusy(true);
    setError("");
    setNotice("");
    setExportedPath("");
    try {
      const env = await apiRaw<number>("plan_save", {
        name,
        itemIds: currentSelection,
      });
      if (env.code !== 0) {
        // 2001 = 待清理清单为空（没勾任何条目）
        setError(env.message);
        return;
      }
      setNotice(`已保存方案「${name}」，共 ${currentSelection.length} 项`);
      setNameInput("");
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, [nameInput, currentSelection, load]);

  // ---- 应用方案 ----
  const apply = useCallback(
    async (p: PlanSummary) => {
      setBusy(true);
      setError("");
      setNotice("");
      setExportedPath("");
      try {
        const env = await apiRaw<PlanApplied>("plan_apply", { id: p.id });
        if (env.code !== 0) {
          setError(env.message);
          return;
        }
        const r = env.data;
        if (!r || r.itemIds.length === 0) {
          // 一条都没匹配上：不要静默切页，否则用户到了清理页发现什么都没勾上。
          setError(
            `方案「${p.name}」里的 ${p.itemCount} 条本次都没找到（可能已被清理，或这次扫描没覆盖到）。先扫描一次再应用。`,
          );
          return;
        }
        const base = `已应用方案「${r.name}」：勾选 ${r.itemIds.length} 项`;
        const notice =
          r.unmatched.length > 0
            ? `${base}；另有 ${r.unmatched.length} 条本次没找到`
            : base;
        onApply({ itemIds: r.itemIds, notice });
      } catch (e) {
        setError(String(e));
      } finally {
        setBusy(false);
      }
    },
    [onApply],
  );

  // ---- 导出 ----
  const exportPlan = useCallback(async (p: PlanSummary) => {
    setBusy(true);
    setError("");
    setNotice("");
    setExportedPath("");
    try {
      const env = await apiRaw<string>("plan_export", { id: p.id });
      if (env.code !== 0) {
        setError(env.message);
        return;
      }
      setExportedPath(env.data);
      setNotice(`已导出方案「${p.name}」`);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, []);

  // ---- 导入 ----
  const importPlan = useCallback(async () => {
    const p = importPath.trim();
    if (!p) {
      setError("请填写要导入的 JSON 文件完整路径");
      return;
    }
    setBusy(true);
    setError("");
    setNotice("");
    setExportedPath("");
    try {
      const env = await apiRaw<number>("plan_import", { filePath: p });
      if (env.code !== 0) {
        setError(env.message);
        return;
      }
      setNotice("已导入为新方案");
      setImportPath("");
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, [importPath, load]);

  // ---- 删除 ----
  const remove = useCallback(
    async (p: PlanSummary) => {
      if (
        !window.confirm(
          `删除方案「${p.name}」？\n\n方案只是保存的勾选组合，删除它不会影响任何已清理或未清理的文件。`,
        )
      ) {
        return;
      }
      setBusy(true);
      setError("");
      setNotice("");
      try {
        const env = await apiRaw<number>("plan_delete", { id: p.id });
        if (env.code !== 0) {
          setError(env.message);
          return;
        }
        setNotice(`已删除方案「${p.name}」`);
        await load();
      } catch (e) {
        setError(String(e));
      } finally {
        setBusy(false);
      }
    },
    [load],
  );

  const canSave = currentSelection.length > 0 && !busy && nameInput.trim().length > 0;

  return (
    <main style={{ padding: 20, maxWidth: 1000 }}>
      <h1 style={{ margin: "0 0 4px" }}>{t("page.plans")}</h1>
      <p style={{ color: "var(--text-muted)", margin: "0 0 16px", fontSize: 13 }}>
        把常清的那批条目存成方案，下次一键勾选。方案存的是**路径**而不是条目编号，
        所以重新扫描之后照样能用。
      </p>

      {/* 保存当前选择 */}
      <div style={cardStyle} data-testid="plan-save-card">
        <div style={{ fontSize: 14, fontWeight: 600, color: "var(--text)", marginBottom: 10 }}>
          保存当前选择为方案
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
          <input
            value={nameInput}
            onChange={(e) => setNameInput(e.target.value)}
            placeholder="方案名，例如「开发机日常」"
            style={{ ...inputStyle, minWidth: 240 }}
            aria-label="方案名"
          />
          <button onClick={save} disabled={!canSave} style={btnStyle(!canSave)}>
            {busy ? "处理中…" : `保存（${currentSelection.length} 项）`}
          </button>
        </div>
        {currentSelection.length === 0 && (
          <div style={{ marginTop: 8, fontSize: 12, color: "var(--text-muted)" }}>
            当前没有勾选任何条目。先去「清理」页勾选，再回来保存。
          </div>
        )}
      </div>

      {/* 导入 */}
      <div style={{ ...cardStyle, marginTop: 12 }}>
        <div style={{ fontSize: 14, fontWeight: 600, color: "var(--text)", marginBottom: 6 }}>
          从 JSON 文件导入
        </div>
        <div style={{ fontSize: 12, color: "var(--text-muted)", marginBottom: 8 }}>
          导出会把文件写到数据目录的 <code>plans\exports\</code> 下，界面上会显示完整路径；
          把那个路径粘到这里即可导入。
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center" }}>
          <input
            value={importPath}
            onChange={(e) => setImportPath(e.target.value)}
            placeholder="C:\Users\...\AppData\Roaming\com.and0114.cclean\plans\exports\xxx.json"
            style={{ ...inputStyle, flex: 1, minWidth: 320 }}
            aria-label="导入文件路径"
          />
          <button onClick={importPlan} disabled={busy} style={ghostBtnStyle(busy)}>
            导入
          </button>
        </div>
      </div>

      {/* 提示区 */}
      {error && (
        <div
          style={{
            marginTop: 12,
            background: "#fee2e2",
            border: "1px solid #fca5a5",
            color: "#b91c1c",
            padding: 10,
            borderRadius: 6,
            fontSize: 12,
            wordBreak: "break-all",
          }}
        >
          {error}
        </div>
      )}
      {notice && (
        <div
          style={{
            marginTop: 12,
            background: "#ecfdf5",
            border: "1px solid #6ee7b7",
            color: "#065f46",
            padding: 10,
            borderRadius: 6,
            fontSize: 12,
          }}
        >
          {notice}
        </div>
      )}
      {exportedPath && (
        <div
          style={{
            marginTop: 8,
            background: "var(--bg)",
            border: "1px solid var(--border)",
            borderRadius: 6,
            padding: 8,
            fontFamily: "ui-monospace, SFMono-Regular, Consolas, monospace",
            fontSize: 11,
            color: "var(--text)",
            wordBreak: "break-all",
          }}
          data-testid="plan-export-path"
        >
          {exportedPath}
        </div>
      )}

      {/* 列表 */}
      <div style={{ ...cardStyle, marginTop: 12 }}>
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "baseline",
            marginBottom: 10,
          }}
        >
          <div style={{ fontSize: 14, fontWeight: 600, color: "var(--text)" }}>
            已保存的方案（{plans.length}）
          </div>
          <button onClick={() => void load()} disabled={loading || busy} style={ghostBtnStyle(loading || busy)}>
            刷新
          </button>
        </div>

        {loading && (
          <div style={{ fontSize: 12, color: "var(--text-muted)" }}>加载中…</div>
        )}

        {!loading && plans.length === 0 && (
          <div style={{ fontSize: 12, color: "var(--text-muted)", lineHeight: 1.8 }}>
            还没有保存任何方案。
            <br />
            在「清理」页勾选条目 → 回到这里起个名字 → 点「保存」。
          </div>
        )}

        {!loading && plans.length > 0 && (
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
            <thead>
              <tr style={{ color: "var(--text-muted)", textAlign: "left" }}>
                <th style={thStyle}>名称</th>
                <th style={thStyle}>创建时间</th>
                <th style={thStyle}>条目数</th>
                <th style={thStyle}>大小</th>
                <th style={{ ...thStyle, textAlign: "right" }}>操作</th>
              </tr>
            </thead>
            <tbody>
              {plans.map((p) => (
                <tr key={p.id} style={{ borderTop: "1px solid var(--border)" }}>
                  <td style={{ ...tdStyle, color: "var(--text)" }}>{p.name}</td>
                  <td style={{ ...tdStyle, color: "var(--text-muted)" }}>
                    {formatTs(p.createdAt)}
                  </td>
                  <td style={{ ...tdStyle, color: "var(--text)" }}>
                    {p.itemCount} 条
                    {p.matchedCount !== p.itemCount && (
                      <span style={{ color: "#b45309", marginLeft: 4 }}>
                        （本次匹配 {p.matchedCount}）
                      </span>
                    )}
                  </td>
                  <td style={{ ...tdStyle, color: "var(--text)" }}>
                    {p.matchedCount === 0 ? "—" : formatBytes(p.sizeBytes)}
                  </td>
                  <td style={{ ...tdStyle, textAlign: "right", whiteSpace: "nowrap" }}>
                    <button
                      onClick={() => void apply(p)}
                      disabled={busy}
                      style={{ ...ghostBtnStyle(busy), marginRight: 6 }}
                    >
                      应用
                    </button>
                    <button
                      onClick={() => void exportPlan(p)}
                      disabled={busy}
                      style={{ ...ghostBtnStyle(busy), marginRight: 6 }}
                    >
                      导出
                    </button>
                    <button onClick={() => void remove(p)} disabled={busy} style={dangerBtnStyle}>
                      删除
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </main>
  );
}

const thStyle: React.CSSProperties = {
  padding: "6px 8px",
  fontWeight: 500,
  fontSize: 11,
};

const tdStyle: React.CSSProperties = {
  padding: "8px 8px",
  verticalAlign: "top",
  wordBreak: "break-all",
};
