// M0 脚手架占位页（task-001）：
// 仅静态渲染标题与一行小字，不调用任何 Tauri IPC——
// scan.start / cleanup.dryRun 等命令由 task-004 的 IPC 网关落地。
export default function App() {
  return (
    <main
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        minHeight: "100vh",
        fontFamily: "system-ui, sans-serif",
      }}
    >
      <h1 style={{ marginBottom: 8 }}>C-Disk Cleaner</h1>
      <p style={{ color: "#666" }}>M0 scaffold — waiting for task 002</p>
    </main>
  );
}
