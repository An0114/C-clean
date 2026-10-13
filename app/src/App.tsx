// task-027/029/030/031：App 加 Dashboard / Items / SafeBox / History / Settings tab 切换（不引 Router，useState）。
// debt-cleanup-002 / Bug-1：外层加最小 Error Boundary——任何页面渲染期抛错
// 不再整树白屏，而是在该 tab 区域显示「页面出错，点这里回仪表盘」。
import { Component, useState, type ReactNode } from "react";
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
