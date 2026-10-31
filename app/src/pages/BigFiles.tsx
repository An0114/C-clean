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

/** 与 engine BigFileItem（camelCase）对齐。 */
interface BigFileItem {
  path: string;
  sizeBytes: number;
  category: string;
  recommendation: string;
}

/** 与 engine BigFilesReport（camelCase）对齐。 */
interface BigFilesReport {
  categories: BigFileItem[];
  topFiles: BigFileItem[];
  totalBytes: number;
  scannedEntries: number;
  truncated: boolean;
  elapsedMs: number;
}

/**
 * 分类展示元数据。key 必须与 engine 的 `CATEGORY_*` 常量一致
 * （`hiberfil` / `windows_old` / `recycle_bin` / `top_files`）。
 */
const CATEGORY_META: Record<string, { title: string; hint: string; color: string }> = {
  hiberfil: { title: "休眠文件", hint: "hiberfil.sys", color: "#6366f1" },
  windows_old: { title: "旧系统备份", hint: "Windows.old", color: "#0891b2" },
  recycle_bin: { title: "回收站", hint: "已删除待清空", color: "#d97706" },
  top_files: { title: "大文件 Top100", hint: "用户目录里的大家伙", color: "#059669" },
};

/** 未知分类的兜底（后端将来加分类不会让页面崩）。 */
const UNKNOWN_META = { title: "其他", hint: "", color: "#6b7280" };

function metaOf(category: string) {
  return CATEGORY_META[category] ?? { ...UNKNOWN_META, title: category || UNKNOWN_META.title };
}

const cardStyle: React.CSSProperties = {
  border: "1px solid var(--border)",
  borderRadius: 8,
  padding: "14px 16px",
  background: "var(--surface, var(--bg))",
};

export default function BigFiles() {
  const [report, setReport] = useState<BigFilesReport | null>(null);
  const [loading, setLoading] = useState(false);
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

  // 只读扫描，进来就扫一次（不弹 UAC，不会打断用户）
  useEffect(() => {
    void scan();
  }, [scan]);

  const topSum = report ? report.topFiles.reduce((s, f) => s + f.sizeBytes, 0) : 0;

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
        「磁盘清理」、回收站右键清空、用户文件自己判断），统一代删会出事。
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

      {report && (
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
    </main>
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

/** Top100 明细表。没有删除按钮——这是「只读」的实质体现。 */
function TopFilesTable({ items }: { items: BigFileItem[] }) {
  if (items.length === 0) {
    return null;
  }
  return (
    <div style={{ ...cardStyle, marginTop: 12, padding: 0, overflow: "hidden" }}>
      <div style={{ padding: "12px 16px", fontSize: 13, fontWeight: 600, borderBottom: "1px solid var(--border)" }}>
        最大的 {items.length} 个文件
      </div>
      <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 12 }}>
        <thead>
          <tr style={{ color: "var(--text-muted)", textAlign: "left" }}>
            <th style={{ padding: "8px 16px", fontWeight: 500 }}>路径</th>
            <th style={{ padding: "8px 16px", fontWeight: 500, width: 100, textAlign: "right" }}>
              大小
            </th>
          </tr>
        </thead>
        <tbody>
          {items.map((f) => (
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
}
