// task-027/029/030：App 加 Dashboard / Items / SafeBox / History tab 切换（不引 Router，useState）。
import { useState } from "react";
import Dashboard from "./pages/Dashboard";
import History from "./pages/History";
import Items from "./pages/Items";
import SafeBox from "./pages/SafeBox";

type Tab = "dashboard" | "items" | "safebox" | "history";

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
      </div>
      {tab === "dashboard" && <Dashboard />}
      {tab === "items" && <Items />}
      {tab === "safebox" && <SafeBox />}
      {tab === "history" && <History />}
    </div>
  );
}
