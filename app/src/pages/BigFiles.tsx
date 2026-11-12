// task-066：大文件地图（E 模块）——**只读页面，没有删除按钮**。
//
// 设计上的三个决定：
//
// 1. **进来就自动扫一次**。这与 Dashboard 的 WinSxS 卡片（必须手动点）相反，
//    因为那个会弹 UAC；本页纯只读、不弹任何系统弹窗，自动扫描不会打断用户。
//    后端是 async command，扫描跑在 Tauri 异步执行器上，UI 不会卡。
//
// 2. **不提供任何删除入口**。hiberfil.sys / Windows.old / 回收站 / 用户大文件
//    这四类的正规处理方式完全不同（powercfg、磁盘清理、清空回收站、用户自判），
//    统一给一个「删除」按钮必然出错。所以只展示 + 给一条能照着做的建议。
//
// 3. **颜色全走 CSS 变量**（`var(--text)` / `var(--border)` / `var(--bg)`），
//    深色主题下自动跟随——这是 App.tsx 主题机制的约定，硬编码颜色会在深色下瞎。

import { useCallback, useEffect, useState } from "react";
import { formatBytes } from "../utils/formatBytes";
import { api } from "../utils/invoke";
import { layoutTreemap, type TreemapDatum } from "../utils/treemap";

/** 与 engine BigFileItem（camelCase）对齐。 */
interface BigFileItem {
  path: string;
  sizeBytes: number;
  category: string;
  recommendation: string;
  /** task-084 B3：所属盘符（大写单字母，如 "C"）。跨盘扫描后按此分组。
   * 兼容旧后端：字段缺失时按空串处理，归入「其他」。 */
  driveLetter: string;
}

/** 与 engine BigFilesReport（camelCase）对齐。 */
interface BigFilesReport {
  categories: BigFileItem[];
  topFiles: BigFileItem[];
  targetSpaces: BigFileItem[];
  totalBytes: number;
  scannedEntries: number;
  truncated: boolean;
  elapsedMs: number;
}

/** task-084 A2：与 engine DupGroup（camelCase）对齐。 */
interface DupGroup {
  groupId: string;
  sizeBytes: number;
  files: {
    path: string;
    sizeBytes: number;
    modifiedMs: number;
    isKeep: boolean;
    /** task-084 B3：所属盘符。 */
    driveLetter: string;
  }[];
}

/** task-084 A3：与 engine FolderStat（camelCase）对齐。 */
interface FolderStat {
  path: string;
  sizeBytes: number;
  fileCount: number;
  category: string;
  driveLetter: string;
}

/** task-084：BigFiles 页的四个 tab。 */
type TabKey = "base" | "target" | "duplicates" | "folders";

const TABS: { key: TabKey; label: string }[] = [
  { key: "base", label: "基础" },
  { key: "target", label: "目标空间" },
  { key: "duplicates", label: "重复文件" },
  { key: "folders", label: "文件夹分布" },
];

/**
 * 分类展示元数据。key 必须与 engine 的 `CATEGORY_*` 常量一致
 * （`hiberfil` / `windows_old` / `recycle_bin` / `top_files`）。
 */
const CATEGORY_META: Record<string, { title: string; hint: string; color: string }> = {
  hiberfil: { title: "休眠文件", hint: "hiberfil.sys", color: "#6366f1" },
  windows_old: { title: "旧系统备份", hint: "Windows.old", color: "#0891b2" },
  recycle_bin: { title: "回收站", hint: "已删除待清空", color: "#d97706" },
  top_files: { title: "大文件 Top100", hint: "用户目录里的大家伙", color: "#059669" },
  // task-084 A1：目标空间四类（Downloads/Documents/Desktop/Temp）
  target_downloads: { title: "下载目录", hint: "Downloads", color: "#0d9488" },
  target_documents: { title: "文档目录", hint: "Documents >100MB", color: "#7c3aed" },
  target_desktop: { title: "桌面", hint: "Desktop >100MB", color: "#db2777" },
  target_temp: { title: "临时目录", hint: "Temp >50MB", color: "#ea580c" },
};

/** 未知分类的兜底（后端将来加分类不会让页面崩）。 */
const UNKNOWN_META = { title: "其他", hint: "", color: "#6b7280" };

function metaOf(category: string) {
  return CATEGORY_META[category] ?? { ...UNKNOWN_META, title: category || UNKNOWN_META.title };
}

/** 把一组带 `driveLetter` 的条目按盘分组（顺序：盘符升序；空串归「其他」放最后）。 */
function groupByDrive<T extends { driveLetter?: string }>(items: T[]): { drive: string; items: T[] }[] {
  const groups = new Map<string, T[]>();
  for (const it of items) {
    const drive = it.driveLetter || "";
    const list = groups.get(drive) ?? [];
    list.push(it);
    groups.set(drive, list);
  }
  // Map 保持插入序；这里显式排序：字母升序，空串（其他）永远最后。
  const sorted = [...groups.entries()].sort((a, b) => {
    if (a[0] === "") return 1;
    if (b[0] === "") return -1;
    return a[0].localeCompare(b[0]);
  });
  return sorted.map(([drive, list]) => ({ drive, items: list }));
}

/** 盘的展示标签：`C:`；空串显示「其他」。 */
function driveLabel(drive: string) {
  return drive ? `${drive}:` : "其他";
}

/** 单个盘的标题栏（盘符 + 条目数 + 合计字节）。 */
function DriveHeader({
  drive,
  label,
  count,
  sum,
}: {
  drive: string;
  label: string;
  count: number;
  sum: number;
}) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "baseline",
        gap: 8,
        flexWrap: "wrap",
      }}
    >
      <span
        style={{
          padding: "3px 8px",
          borderRadius: 5,
          fontSize: 12,
          fontWeight: 600,
          background: "var(--surface, var(--bg))",
          border: "1px solid var(--border)",
          color: "var(--text)",
        }}
        data-testid={drive ? `drive-header-${drive}` : "drive-header-other"}
      >
        {label}
      </span>
      <span style={{ fontSize: 12, color: "var(--text-muted)" }}>
        {count} 项 · {formatBytes(sum)}
      </span>
    </div>
  );
}

const cardStyle: React.CSSProperties = {
  border: "1px solid var(--border)",
  borderRadius: 8,
  padding: "14px 16px",
  background: "var(--surface, var(--bg))",
};

export default function BigFiles() {
  const [tab, setTab] = useState<TabKey>("base");
  const [report, setReport] = useState<BigFilesReport | null>(null);
  const [dups, setDups] = useState<DupGroup[] | null>(null);
  const [folders, setFolders] = useState<FolderStat[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [loadingDups, setLoadingDups] = useState(false);
  const [loadingFolders, setLoadingFolders] = useState(false);
  const [error, setError] = useState("");

  const scan = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      setReport(await api<BigFilesReport>("big_files_scan", {}));
    } catch (e) {
      // 失败时清掉旧结果：留着上一次的数字会让人以为「这次也扫成功了」
      setReport(null);
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const scanDups = useCallback(async () => {
    setLoadingDups(true);
    try {
      setDups(await api<DupGroup[]>("big_files_duplicates", {}));
    } catch (e) {
      setDups(null);
      setError(String(e));
    } finally {
      setLoadingDups(false);
    }
  }, []);

  const scanFolders = useCallback(async () => {
    setLoadingFolders(true);
    try {
      setFolders(await api<FolderStat[]>("big_files_folder_stats", {}));
    } catch (e) {
      setFolders(null);
      setError(String(e));
    } finally {
      setLoadingFolders(false);
    }
  }, []);

  // 只读扫描，进来就扫一次（不弹 UAC，不会打断用户）
  useEffect(() => {
    void scan();
  }, [scan]);

  // 第一次切到对应 tab 时懒加载（不进来的 tab 不白扫一遍盘）
  useEffect(() => {
    if (tab === "duplicates" && dups === null) {
      void scanDups();
    }
  }, [tab, dups, scanDups]);
  useEffect(() => {
    if (tab === "folders" && folders === null) {
      void scanFolders();
    }
  }, [tab, folders, scanFolders]);

  const topSum = report ? report.topFiles.reduce((s, f) => s + f.sizeBytes, 0) : 0;
  const targetSum = report ? report.targetSpaces.reduce((s, f) => s + f.sizeBytes, 0) : 0;

  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: "8px 16px",
    fontSize: 13,
    border: "1px solid var(--border)",
    borderBottom: active ? "2px solid var(--accent, #2563eb)" : "1px solid var(--border)",
    borderRadius: "6px 6px 0 0",
    background: active ? "var(--surface, var(--bg))" : "transparent",
    color: active ? "var(--text)" : "var(--text-muted)",
    cursor: "pointer",
    fontWeight: active ? 600 : 400,
  });

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
          <h1 style={{ margin: "0 0 4px" }}>大文件地图</h1>
          <p style={{ color: "var(--text-muted)", margin: 0, fontSize: 13 }}>
            系统盘上最占空间的几类东西。<strong>本页只读，不会删除任何文件。</strong>
          </p>
        </div>
        <button
          onClick={scan}
          disabled={loading}
          style={{
            padding: "8px 16px",
            fontSize: 13,
            borderRadius: 6,
            border: "1px solid var(--border)",
            background: "var(--bg)",
            color: "var(--text)",
            cursor: loading ? "not-allowed" : "pointer",
          }}
        >
          {loading ? "扫描中…" : "重新扫描"}
        </button>
      </div>

      {/* 只读声明放在最显眼的位置：用户看到「大文件」第一反应是「能一键删吗」 */}
      <div
        style={{
          marginTop: 12,
          padding: "10px 12px",
          borderRadius: 6,
          background: "#eff6ff",
          border: "1px solid #bfdbfe",
          color: "#1e40af",
          fontSize: 12,
          lineHeight: 1.7,
        }}
      >
        本页只负责「量出来 + 给建议」，<strong>不提供删除按钮</strong>
        ——这几类的正规处理方式各不相同（休眠用 <code>powercfg /h off</code>、旧系统备份用
        「磁盘清理」、回收站右键清空、用户文件自己判断、重复文件保留最新一份），统一代删会出事。
      </div>

      {/* task-084：四个 tab */}
      <div style={{ marginTop: 16, display: "flex", gap: 4, borderBottom: "1px solid var(--border)" }}>
        {TABS.map((t) => (
          <button key={t.key} role="tab" aria-selected={tab === t.key} onClick={() => setTab(t.key)} style={tabStyle(tab === t.key)}>
            {t.label}
          </button>
        ))}
      </div>

      {loading && (
        <div style={{ marginTop: 16, fontSize: 13, color: "var(--text-muted)" }}>
          正在扫描系统盘…（只读遍历，条目多时可能十几秒）
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

      {tab === "base" && report && (
        <>
          <div style={{ marginTop: 16, fontSize: 12, color: "var(--text-muted)" }}>
            合计 {formatBytes(report.totalBytes)} · 遍历 {report.scannedEntries} 个条目 · 耗时{" "}
            {report.elapsedMs} ms
            {report.truncated && (
              <span style={{ color: "#b45309" }}>
                {" "}
                · 已达扫描上限提前停止，Top 列表可能不是全盘完整结果
              </span>
            )}
          </div>

          {/* 三张固定分类卡片 + Top100 汇总卡 */}
          <div
            style={{
              marginTop: 12,
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))",
              gap: 12,
            }}
          >
            {report.categories.map((c) => (
              <CategoryCard key={c.category} item={c} />
            ))}
            <div style={cardStyle} data-testid="category-top_files">
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span
                  style={{
                    width: 8,
                    height: 8,
                    borderRadius: 4,
                    background: CATEGORY_META.top_files.color,
                    display: "inline-block",
                  }}
                />
                <span style={{ fontSize: 13, fontWeight: 600 }}>{CATEGORY_META.top_files.title}</span>
                <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
                  {report.topFiles.length} 个
                </span>
              </div>
              <div style={{ fontSize: 24, fontWeight: 600, margin: "6px 0 2px" }}>
                {formatBytes(topSum)}
              </div>
              <div style={{ fontSize: 12, color: "var(--text-muted)", lineHeight: 1.7 }}>
                {report.topFiles.length === 0
                  ? "没有超过 100 MB 的用户文件"
                  : "确认不再需要后手动删除；本工具不代删用户文件"}
              </div>
            </div>
          </div>

          <TopFilesTable items={report.topFiles} />
        </>
      )}

      {tab === "target" && (
        <TargetSpacePanel items={report?.targetSpaces ?? []} sum={targetSum} loading={loading} />
      )}

      {tab === "duplicates" && <DuplicatesPanel groups={dups} loading={loadingDups} />}

      {tab === "folders" && <FoldersPanel stats={folders} loading={loadingFolders} />}
    </main>
  );
}

/** task-084 A1：目标空间 tab。Downloads/Documents/Desktop/Temp 按 category 分组。 */
function TargetSpacePanel({ items, sum, loading }: { items: BigFileItem[]; sum: number; loading: boolean }) {
  if (loading) {
    return <div style={{ marginTop: 16, fontSize: 13, color: "var(--text-muted)" }}>正在扫描目标空间…</div>;
  }
  const groups = new Map<string, BigFileItem[]>();
  for (const it of items) {
    const list = groups.get(it.category) ?? [];
    list.push(it);
    groups.set(it.category, list);
  }
  return (
    <div style={{ marginTop: 16 }}>
      <div style={{ fontSize: 12, color: "var(--text-muted)", marginBottom: 12 }}>
        目标空间大文件（只读）· 合计 {formatBytes(sum)} · 按类别分组
      </div>
      {groups.size === 0 && (
        <div style={{ fontSize: 13, color: "var(--text-muted)" }}>目标空间里没有超过门槛的大文件。</div>
      )}
      {[...groups.entries()].map(([cat, list]) => {
        const meta = metaOf(cat);
        return (
          <div key={cat} style={{ ...cardStyle, marginBottom: 12 }} data-testid={`target-${cat}`}>
            <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
              <span
                style={{
                  width: 8,
                  height: 8,
                  borderRadius: 4,
                  background: meta.color,
                  display: "inline-block",
                }}
              />
              <span style={{ fontSize: 13, fontWeight: 600 }}>{meta.title}</span>
              <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
                {list.length} 个 · {formatBytes(list.reduce((s, f) => s + f.sizeBytes, 0))}
              </span>
            </div>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
              <tbody>
                {list.map((f) => (
                  <tr key={f.path} style={{ borderTop: "1px solid var(--border)" }}>
                    <td style={{ padding: "6px 8px", wordBreak: "break-all" }} title={f.path}>
                      {f.path}
                    </td>
                    <td style={{ padding: "6px 8px", textAlign: "right", whiteSpace: "nowrap" }}>
                      {formatBytes(f.sizeBytes)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        );
      })}
    </div>
  );
}

/** task-084 A2：重复文件 tab。每组保留最新修改时间那份，其余标可清理。task-084 B3 跨盘：文件带盘符标识。 */
function DuplicatesPanel({ groups, loading }: { groups: DupGroup[] | null; loading: boolean }) {
  if (loading) {
    return <div style={{ marginTop: 16, fontSize: 13, color: "var(--text-muted)" }}>正在计算重复文件 SHA-256…（超过 10MB 的文件，稍候）</div>;
  }
  if (!groups) {
    return <div style={{ marginTop: 16, fontSize: 13, color: "var(--text-muted)" }}>暂无数据。</div>;
  }
  if (groups.length === 0) {
    return <div style={{ marginTop: 16, fontSize: 13, color: "var(--text-muted)" }}>扫描范围里没有检测到重复的大文件。</div>;
  }
  return (
    <div style={{ marginTop: 16 }}>
      <div style={{ fontSize: 12, color: "var(--text-muted)", marginBottom: 12 }}>
        对系统盘用户目录及非系统盘顶层目录里超过 10MB 的文件做 SHA-256 去重，按盘标注。每组
        <b>保留最后修改时间最新的一份</b>，其余建议清理；本工具只报告，不代删。共 {groups.length} 组。
      </div>
      {groups.map((g) => {
        const reclaimable = g.sizeBytes * (g.files.length - 1);
        // 组内涉及哪些盘（用于快速识别跨盘重复）
        const drives = [...new Set(g.files.map((f) => f.driveLetter).filter(Boolean))].sort();
        return (
          <div key={g.groupId} style={{ ...cardStyle, marginBottom: 12 }} data-testid="dup-group">
            <div style={{ marginBottom: 8 }}>
              <span style={{ fontSize: 13, fontWeight: 600 }}>重复组 {g.groupId.slice(0, 8)}</span>
              <span style={{ fontSize: 11, color: "var(--text-muted)", marginLeft: 8 }}>
                {g.files.length} 份 × {formatBytes(g.sizeBytes)} · 可释放 {formatBytes(reclaimable)}
              </span>
              {drives.length > 0 && (
                <span style={{ fontSize: 11, color: "var(--text-muted)", marginLeft: 8 }}>
                  涉及盘：{drives.map((d) => `${d}:`).join(" / ")}
                </span>
              )}
            </div>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
              <tbody>
                {g.files.map((f) => (
                  <tr key={f.path} style={{ borderTop: "1px solid var(--border)" }}>
                    <td style={{ padding: "6px 8px", wordBreak: "break-all" }} title={f.path}>
                      {f.isKeep ? "✅ 保留" : "🗑 可清理"}{" "}
                      <span style={{ color: "var(--text-muted)" }}>[{f.driveLetter || "?"}:]</span> {f.path}
                    </td>
                    <td style={{ padding: "6px 8px", color: "var(--text-muted)", whiteSpace: "nowrap" }}>
                      {new Date(f.modifiedMs).toLocaleDateString()}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        );
      })}
    </div>
  );
}

/** task-084 A3：文件夹分布 tab。Top20 按子树大小倒序，task-084 B3 按盘分组。task-086 3.2：加 treemap 可视化切换。 */
function FoldersPanel({ stats, loading }: { stats: FolderStat[] | null; loading: boolean }) {
  const [view, setView] = useState<"table" | "treemap">("table");
  if (loading) {
    return <div style={{ marginTop: 16, fontSize: 13, color: "var(--text-muted)" }}>正在统计文件夹分布…</div>;
  }
  if (!stats) {
    return <div style={{ marginTop: 16, fontSize: 13, color: "var(--text-muted)" }}>暂无数据。</div>;
  }
  if (stats.length === 0) {
    return <div style={{ marginTop: 16, fontSize: 13, color: "var(--text-muted)" }}>没有可统计的文件夹。</div>;
  }
  const groups = groupByDrive(stats);
  const hasData = groups.some((g) => g.items.length > 0);
  const viewStyle = (active: boolean): React.CSSProperties => ({
    padding: "5px 12px",
    fontSize: 12,
    border: "1px solid var(--border)",
    borderRadius: 5,
    background: active ? "var(--accent, #2563eb)" : "var(--surface, var(--bg))",
    color: active ? "#fff" : "var(--text)",
    cursor: "pointer",
    fontWeight: active ? 600 : 400,
  });
  return (
    <div style={{ marginTop: 16 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 12 }}>
        <span style={{ fontSize: 12, color: "var(--text-muted)" }}>
          占用最多的文件夹（子树总大小，倒序）· 按盘分组
        </span>
        <div style={{ display: "flex", gap: 4 }}>
          <button onClick={() => setView("table")} style={viewStyle(view === "table")}>
            列表
          </button>
          <button onClick={() => setView("treemap")} style={viewStyle(view === "treemap")}>
            矩形树图
          </button>
        </div>
      </div>
      {view === "table"
        ? groups.map((g) => (
            <div key={g.drive || "__other"} style={{ ...cardStyle, marginBottom: 12, padding: 0, overflow: "hidden" }}>
              <div
                style={{
                  padding: "10px 16px",
                  borderBottom: "1px solid var(--border)",
                  background: "var(--surface, var(--bg))",
                }}
              >
                <DriveHeader drive={g.drive} label={driveLabel(g.drive)} count={g.items.length} sum={sumOf(g.items)} />
              </div>
              <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
                <thead>
                  <tr style={{ color: "var(--text-muted)", textAlign: "left" }}>
                    <th style={{ padding: "8px 16px", fontWeight: 500 }}>文件夹</th>
                    <th style={{ padding: "8px 16px", fontWeight: 500, width: 90, textAlign: "right" }}>占用</th>
                    <th style={{ padding: "8px 16px", fontWeight: 500, width: 70, textAlign: "right" }}>文件数</th>
                  </tr>
                </thead>
                <tbody>
                  {g.items.map((f) => (
                    <tr key={f.path} style={{ borderTop: "1px solid var(--border)" }}>
                      <td style={{ padding: "8px 16px", wordBreak: "break-all" }} title={f.path}>
                        {f.path}
                      </td>
                      <td style={{ padding: "8px 16px", textAlign: "right", whiteSpace: "nowrap" }}>
                        {formatBytes(f.sizeBytes)}
                      </td>
                      <td style={{ padding: "8px 16px", textAlign: "right", whiteSpace: "nowrap" }}>{f.fileCount}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ))
        : hasData && (
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              {groups.map((g) => (
                <div key={g.drive || "__other"} style={{ ...cardStyle, padding: 0, overflow: "hidden" }}>
                  <div
                    style={{
                      padding: "10px 16px",
                      borderBottom: "1px solid var(--border)",
                      background: "var(--surface, var(--bg))",
                    }}
                  >
                    <DriveHeader drive={g.drive} label={driveLabel(g.drive)} count={g.items.length} sum={sumOf(g.items)} />
                  </div>
                  <div style={{ padding: 12 }}>
                    <FolderTreemap stats={g.items} />
                  </div>
                </div>
              ))}
            </div>
          )}
      {view === "treemap" && !hasData && (
        <div style={{ fontSize: 13, color: "var(--text-muted)" }}>没有可绘制的文件夹数据。</div>
      )}
    </div>
  );
}

function sumOf(items: FolderStat[]): number {
  return items.reduce((s, f) => s + f.sizeBytes, 0);
}

/** task-086 3.2：把一批 FolderStat 渲染成手写 SVG 矩形树图。 */
function FolderTreemap({ stats }: { stats: FolderStat[] }) {
  const W = 760;
  const H = 300;
  // 稳定取色：按路径哈希到一组固定色板（避免深色主题下用错颜色变量）。
  const palette = ["#6366f1", "#0891b2", "#d97706", "#059669", "#db2777", "#7c3aed", "#ea580c", "#0d9488"];
  const hash = (s: string) => {
    let h = 0;
    for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) | 0;
    return Math.abs(h);
  };
  const data: TreemapDatum[] = stats.map((s) => ({
    key: s.path,
    label: s.path.split(/[\\/]/).filter(Boolean).pop() || s.path,
    size: s.sizeBytes,
    color: palette[hash(s.path) % palette.length],
  }));
  const rects = layoutTreemap(data, W, H);

  return (
    <svg
      width="100%"
      viewBox={`0 0 ${W} ${H}`}
      role="img"
      aria-label="文件夹占用矩形树图"
      style={{ display: "block", maxWidth: "100%", height: "auto" }}
      data-testid="folder-treemap"
    >
      {rects.map((r) => (
        <g key={r.datum.key}>
          <rect
            x={r.x}
            y={r.y}
            width={r.w}
            height={r.h}
            fill={r.datum.color}
            fillOpacity={0.85 - Math.min(r.depth, 4) * 0.12}
            stroke="var(--bg)"
            strokeWidth={0.75}
          >
            <title>{`${r.datum.label} · ${formatBytes(r.datum.size)}`}</title>
          </rect>
          {r.w > 48 && r.h > 16 && (
            <text
              x={r.x + 3}
              y={r.y + 11}
              fontSize={r.w > 120 ? 11 : 9}
              fill="#ffffff"
              fillOpacity={0.92}
              style={{ pointerEvents: "none" }}
            >
              {r.datum.label.length > 18 ? r.datum.label.slice(0, 17) + "…" : r.datum.label}
            </text>
          )}
        </g>
      ))}
    </svg>
  );
}

/** 一个分类卡片：标题 + 大小 + 处理建议。 */
function CategoryCard({ item }: { item: BigFileItem }) {
  const meta = metaOf(item.category);
  return (
    <div style={cardStyle} data-testid={`category-${item.category}`}>
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <span
          style={{
            width: 8,
            height: 8,
            borderRadius: 4,
            background: meta.color,
            display: "inline-block",
          }}
        />
        <span style={{ fontSize: 13, fontWeight: 600 }}>{meta.title}</span>
        {meta.hint && <span style={{ fontSize: 11, color: "var(--text-muted)" }}>{meta.hint}</span>}
      </div>
      <div style={{ fontSize: 24, fontWeight: 600, margin: "6px 0 2px" }}>
        {formatBytes(item.sizeBytes)}
      </div>
      <div style={{ fontSize: 12, color: "var(--text-muted)", lineHeight: 1.7 }}>
        {item.recommendation}
      </div>
    </div>
  );
}

/** 各盘 Top 大文件表（task-084 B3 按盘分组）。没有删除按钮——「只读」的实质体现。 */
function TopFilesTable({ items }: { items: BigFileItem[] }) {
  if (items.length === 0) {
    return null;
  }
  const groups = groupByDrive(items);
  return (
    <div style={{ ...cardStyle, marginTop: 12, padding: 0, overflow: "hidden" }}>
      <div style={{ padding: "12px 16px", fontSize: 13, fontWeight: 600, borderBottom: "1px solid var(--border)" }}>
        最大的 {items.length} 个文件（按盘分组）
      </div>
      {groups.map((g) => {
        const sum = g.items.reduce((s, f) => s + f.sizeBytes, 0);
        return (
          <div key={g.drive || "__other"} style={{ padding: "12px 16px", borderBottom: "1px solid var(--border)" }}>
            <DriveHeader drive={g.drive} label={driveLabel(g.drive)} count={g.items.length} sum={sum} />
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12, marginTop: 6 }}>
              <thead>
                <tr style={{ color: "var(--text-muted)", textAlign: "left" }}>
                  <th style={{ padding: "8px 16px", fontWeight: 500 }}>路径</th>
                  <th style={{ padding: "8px 16px", fontWeight: 500, width: 100, textAlign: "right" }}>
                    大小
                  </th>
                </tr>
              </thead>
              <tbody>
                {g.items.map((f) => (
                  <tr key={f.path} style={{ borderTop: "1px solid var(--border)" }}>
                    <td style={{ padding: "8px 16px", wordBreak: "break-all" }} title={f.path}>
                      {f.path}
                    </td>
                    <td style={{ padding: "8px 16px", textAlign: "right", whiteSpace: "nowrap" }}>
                      {formatBytes(f.sizeBytes)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        );
      })}
    </div>
  );
}
