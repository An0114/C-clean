// task-031：设置页——规则库卡片 + 「检查更新」+「应用更新」按钮。
//
// 与任务书偏差（按 engine 真实 serde shape 对齐）：
// - CheckUpdateResp / ApplyResult 均 #[serde(rename_all = "camelCase")]，
//   前端字段用 camelCase（hasUpdate/currentVersion/...），不是任务书假设的 snake_case。
// - Tauri 命令返回 Envelope<T>，但前端按既有 Dashboard/History/SafeBox 惯例
//   直接消费 data 字段（信封解包由既有运行时约定处理）。

import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface CheckUpdateResp {
  hasUpdate: boolean;
  currentVersion: string;
  remoteVersion: string;
  currentSha256: string;
  remoteSha256: string;
}

interface ApplyResult {
  newVersion: string;
  newSha256: string;
}

type Phase =
  | "loading"
  | "idle"
  | "checking"
  | "applying"
  | "error";

const cardStyle: React.CSSProperties = {
  border: "1px solid #e5e7eb",
  borderRadius: 8,
  padding: "20px 24px",
  background: "#fff",
  maxWidth: 560,
};

const rowStyle: React.CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  padding: "6px 0",
  fontSize: 13,
  color: "#374151",
};

const monoStyle: React.CSSProperties = {
  fontFamily: "monospace",
  fontSize: 12,
  color: "#6b7280",
};

export default function Settings() {
  const [phase, setPhase] = useState<Phase>("loading");
  const [info, setInfo] = useState<CheckUpdateResp | null>(null);
  const [msg, setMsg] = useState<string>("");
  const [toast, setToast] = useState<{ ok: boolean; text: string } | null>(null);

  // 启动时拉一次：拿当前版本 + 是否有更新。
  const check = useCallback(async (withToast = false) => {
    setPhase("checking");
    setMsg("");
    setToast(null);
    try {
      const r = await invoke<CheckUpdateResp>("rules_check_update", {});
      setInfo(r);
      setPhase("idle");
      if (withToast) {
        setToast({
          ok: true,
          text: r.hasUpdate ? `发现新版本 ${r.remoteVersion}` : "已是最新版本",
        });
      }
    } catch (e) {
      setInfo(null);
      setMsg(String(e));
      setPhase("error");
    }
  }, []);

  useEffect(() => {
    check();
  }, [check]);

  const apply = useCallback(async () => {
    setPhase("applying");
    setToast(null);
    try {
      const r = await invoke<ApplyResult>("rules_apply_update", {});
      setToast({ ok: true, text: `已更新到版本 ${r.newVersion}` });
      // 应用后重拉，刷新当前版本号 + hasUpdate=false。
      await check();
    } catch (e) {
      setToast({ ok: false, text: `应用更新失败：${String(e)}` });
      setPhase("idle");
    }
  }, [check]);

  return (
    <div>
      <h2 style={{ margin: "0 0 12px", fontSize: 16 }}>规则库</h2>

      {toast && (
        <div
          style={{
            marginBottom: 12,
            padding: 10,
            borderRadius: 6,
            fontSize: 13,
            background: toast.ok ? "#dcfce7" : "#fee2e2",
            color: toast.ok ? "#166534" : "#b91c1c",
            border: `1px solid ${toast.ok ? "#86efac" : "#fca5a5"}`,
          }}
        >
          {toast.text}
        </div>
      )}

      {phase === "loading" && (
        <div style={{ color: "#6b7280", fontSize: 13 }}>加载中...</div>
      )}

      {phase === "error" && (
        <div style={{ color: "#b91c1c", fontSize: 13, marginBottom: 12 }}>
          加载失败：{msg}
        </div>
      )}

      <div style={cardStyle}>
        <div style={rowStyle}>
          <span>当前版本</span>
          <span style={monoStyle}>{info?.currentVersion ?? "-"}</span>
        </div>
        <div style={rowStyle}>
          <span>远端版本</span>
          <span style={monoStyle}>
            {phase === "checking" ? "检查中…" : info?.remoteVersion ?? "-"}
          </span>
        </div>
        <div style={rowStyle}>
          <span>当前 SHA-256</span>
          <span style={monoStyle}>
            {info?.currentSha256?.slice(0, 16) ?? "-"}…
          </span>
        </div>
        <div style={rowStyle}>
          <span>状态</span>
          <span style={{ fontWeight: 600, color: info?.hasUpdate ? "#b45309" : "#166534" }}>
            {info == null
              ? "-"
              : info.hasUpdate
                ? `有新版本 ${info.remoteVersion} 可用`
                : "已是最新"}
          </span>
        </div>

        <div style={{ marginTop: 16, display: "flex", gap: 8 }}>
          <button
            onClick={() => check(true)}
            disabled={phase === "checking" || phase === "applying"}
          >
            {phase === "checking" ? "检查中…" : "检查更新"}
          </button>

          {info?.hasUpdate && (
            <button
              onClick={apply}
              disabled={phase === "checking" || phase === "applying"}
              style={{
                padding: "6px 14px",
                borderRadius: 6,
                border: "1px solid #16a34a",
                background: phase === "applying" ? "#bbf7d0" : "#16a34a",
                color: "#fff",
                cursor:
                  phase === "checking" || phase === "applying"
                    ? "not-allowed"
                    : "pointer",
              }}
            >
              {phase === "applying" ? "应用中…" : "应用更新"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
