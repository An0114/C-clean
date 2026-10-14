// task-027/029/030/031：App 加 Dashboard / Items / SafeBox / History / Settings tab 切换（不引 Router，useState）。
// debt-cleanup-002 / Bug-1：外层加最小 Error Boundary——任何页面渲染期抛错
// 不再整树白屏，而是在该 tab 区域显示「页面出错，点这里回仪表盘」。
import { Component, useEffect, useState, type ReactNode } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import Dashboard from "./pages/Dashboard";
import History from "./pages/History";
import Items from "./pages/Items";
import SafeBox from "./pages/SafeBox";
import Settings from "./pages/Settings";

type Tab = "dashboard" | "items" | "safebox" | "history" | "settings";

const tabBtnStyle = (active: boolean): React.CSSProperties => ({
  padding: "6px 14px",
  marginRight: 8,
  borderRadius: 6,
  border: "1px solid #e5e7eb",
  background: active ? "#111827" : "#fff",
  color: active ? "#fff" : "#111827",
  cursor: "pointer",
  fontSize: 13,
});

/// 最小 Error Boundary：class component（React 18 无 useErrorBoundary hook）。
/// 仅捕获子树渲染期错误，不吞事件处理器 / 异步错误。
/// 捕获后显示错误提示 + 「回仪表盘」按钮（重置 state 即回退）。
class ErrorBoundary extends Component<
  { children: ReactNode; onReset: () => void },
  { hasError: boolean; message: string }
> {
  state: { hasError: boolean; message: string } = { hasError: false, message: "" };

  static getDerivedStateFromError(e: unknown) {
    return { hasError: true, message: e instanceof Error ? e.message : String(e) };
  }

  componentDidCatch(e: unknown, info: { componentStack?: string }) {
    // 打到 WebView2 控制台，便于现场取证。
    console.error("[ErrorBoundary]", e, info.componentStack ?? "");
  }

  render() {
    if (this.state.hasError) {
      return (
        <div
          style={{
            marginTop: 16,
            padding: 16,
            borderRadius: 8,
            background: "#fee2e2",
            border: "1px solid #fca5a5",
            color: "#b91c1c",
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: 8 }}>页面出错</div>
          <div style={{ fontSize: 12, marginBottom: 12, opacity: 0.8 }}>
            {this.state.message}
          </div>
          <button
            onClick={() => {
              this.setState({ hasError: false, message: "" });
              this.props.onReset();
            }}
          >
            点这里回仪表盘
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

export default function App() {
  const [tab, setTab] = useState<Tab>("dashboard");
  // task-036：启动时自动 check 一次应用更新（不弹窗、不阻塞）。
  // 无更新 / 网络失败都静默；有更新才在顶部 banner 提示。
  const [update, setUpdate] = useState<Update | null>(null);
  const [dismissed, setDismissed] = useState(false);
  const [bannerPhase, setBannerPhase] = useState<"idle" | "downloading" | "error">("idle");
  const [bannerError, setBannerError] = useState("");
  useEffect(() => {
    (async () => {
      try {
        const u = await check();
        if (u) setUpdate(u);
      } catch {
        // 静默：网络错 / endpoint 未就绪都不打扰用户
      }
    })();
  }, []);

  // task-036：banner「立即更新」——下载安装后自动重启
  const onUpdateNow = async () => {
    if (!update) return;
    setBannerPhase("downloading");
    setBannerError("");
    try {
      await update.downloadAndInstall();
      await relaunch();
    } catch (e) {
      setBannerPhase("error");
      setBannerError(String(e));
    }
  };
  return (
    <div
      style={{
        padding: 24,
        fontFamily: "system-ui, sans-serif",
        background: "#f9fafb",
        minHeight: "100vh",
        boxSizing: "border-box",
      }}
    >
      <h1 style={{ margin: "0 0 16px" }}>C-Disk Cleaner</h1>

      {/* task-036：非阻塞更新 banner——有新版本才显示，不挡操作 */}
      {update && !dismissed && (
        <div
          style={{
            background: bannerPhase === "error" ? "#fee2e2" : "#dbeafe",
            border: `1px solid ${bannerPhase === "error" ? "#fca5a5" : "#93c5fd"}`,
            color: bannerPhase === "error" ? "#b91c1c" : "#1e3a8a",
            padding: "10px 14px",
            borderRadius: 8,
            marginBottom: 12,
            display: "flex",
            gap: 12,
            alignItems: "center",
          }}
        >
          <div style={{ flex: 1 }}>
            <div style={{ fontWeight: 600 }}>
              {bannerPhase === "error"
                ? `更新失败：${bannerError}`
                : bannerPhase === "downloading"
                  ? "下载安装中…（应用即将重启）"
                  : `发现新版本 v${update.version}`}
            </div>
            {bannerPhase === "idle" && update.body && (
              <div style={{ fontSize: 12, color: "#374151", marginTop: 4 }}>
                {update.body}
              </div>
            )}
          </div>
          {bannerPhase !== "error" && (
            <button
              onClick={onUpdateNow}
              disabled={bannerPhase === "downloading"}
              style={{
                background: "#10b981",
                color: "#fff",
                border: 0,
                padding: "6px 12px",
                borderRadius: 6,
                cursor: bannerPhase === "downloading" ? "not-allowed" : "pointer",
              }}
            >
              {bannerPhase === "downloading" ? "安装中…" : "立即更新"}
            </button>
          )}
          <button
            onClick={() => setDismissed(true)}
            style={{
              background: "#fff",
              border: "1px solid #d1d5db",
              padding: "6px 12px",
              borderRadius: 6,
              cursor: "pointer",
            }}
          >
            稍后
          </button>
        </div>
      )}

      <div style={{ marginBottom: 16 }}>
        <button style={tabBtnStyle(tab === "dashboard")} onClick={() => setTab("dashboard")}>
          仪表盘
        </button>
        <button style={tabBtnStyle(tab === "items")} onClick={() => setTab("items")}>
          清理
        </button>
        <button style={tabBtnStyle(tab === "safebox")} onClick={() => setTab("safebox")}>
          保险箱
        </button>
        <button style={tabBtnStyle(tab === "history")} onClick={() => setTab("history")}>
          历史
        </button>
        <button style={tabBtnStyle(tab === "settings")} onClick={() => setTab("settings")}>
          设置
        </button>
      </div>
      <ErrorBoundary onReset={() => setTab("dashboard")}>
        {tab === "dashboard" && <Dashboard />}
        {tab === "items" && <Items />}
        {tab === "safebox" && <SafeBox />}
        {tab === "history" && <History />}
        {tab === "settings" && <Settings />}
      </ErrorBoundary>
    </div>
  );
}
