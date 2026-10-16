// task-031：设置页——规则库卡片 + 「检查更新」+「应用更新」按钮。
//
// 与任务书偏差（按 engine 真实 serde shape 对齐）：
// - CheckUpdateResp / ApplyResult 均 #[serde(rename_all = "camelCase")]，
//   前端字段用 camelCase（hasUpdate/currentVersion/...），不是任务书假设的 snake_case。
// - Tauri 命令返回 Envelope<T>，但前端按既有 Dashboard/History/SafeBox 惯例
//   直接消费 data 字段（信封解包由既有运行时约定处理）。

import { useCallback, useEffect, useState } from "react";
import { api } from "../utils/invoke";
import { check as checkApp, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

// task-042：后端返回 CheckUpdateResult（三态：upToDate / newVersion / offline）。
type CheckStatus = "upToDate" | "newVersion" | "offline";

interface CheckUpdateResult {
  status: CheckStatus;
  localVersion: string;
  localSha256: string;
  remoteVersion: string | null;
  remoteSha256: string | null;
  message: string;
}

interface ApplyResult {
  newVersion: string;
  newSha256: string;
}

// task-044：回滚结果。
interface RollbackResult {
  previousVersion: string;
  restoredSha256: string;
  message: string;
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
  const [info, setInfo] = useState<CheckUpdateResult | null>(null);
  const [msg, setMsg] = useState<string>("");
  const [toast, setToast] = useState<{ ok: boolean; text: string } | null>(null);
  // task-044：是否存在可回滚的 .bak。
  const [hasBak, setHasBak] = useState<boolean>(false);
  const [rollingBack, setRollingBack] = useState<boolean>(false);

  // task-035：应用自更新状态
  const [appUpdate, setAppUpdate] = useState<Update | null>(null);
  const [appPhase, setAppPhase] = useState<"idle" | "checking" | "downloading" | "error">("idle");
  const [appMsg, setAppMsg] = useState<string>("");

  // 启动时拉一次：拿当前版本 + 是否有更新。
  const check = useCallback(async (withToast = false) => {
    setPhase("checking");
    setMsg("");
    setToast(null);
    try {
      const r = await api<CheckUpdateResult>("rules_check_update", {});
      setInfo(r);
      setPhase("idle");
      if (withToast) {
        setToast({
          ok: r.status !== "offline",
          text: r.message,
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

  // task-044：刷新「是否有 .bak」状态（启动时 + 应用/回滚后）。
  const refreshHasBak = useCallback(async () => {
    try {
      const v = await api<boolean>("rules_has_backup", {});
      setHasBak(Boolean(v));
    } catch {
      setHasBak(false);
    }
  }, []);

  useEffect(() => {
    refreshHasBak();
  }, [refreshHasBak]);

  // task-044：回滚到上一版。
  const rollback = useCallback(async () => {
    if (!window.confirm("确定回滚到上一版规则库？")) return;
    setRollingBack(true);
    setToast(null);
    try {
      const r = await api<RollbackResult>("rules_rollback", {});
      setToast({ ok: true, text: r.message });
      await refreshHasBak();
      await check();
    } catch (e) {
      setToast({ ok: false, text: `回滚失败：${String(e)}` });
    } finally {
      setRollingBack(false);
    }
  }, [check, refreshHasBak]);

  const apply = useCallback(async () => {
    setPhase("applying");
    setToast(null);
    try {
      const r = await api<ApplyResult>("rules_apply_update", {});
      setToast({ ok: true, text: `已更新到版本 ${r.newVersion}` });
      // 应用后重拉，刷新当前版本号 + hasUpdate=false。
      await check();
    } catch (e) {
      setToast({ ok: false, text: `应用更新失败：${String(e)}` });
      setPhase("idle");
    }
  }, [check]);

  // task-035：检查应用自更新（tauri updater）
  const checkApplication = useCallback(async () => {
    setAppPhase("checking");
    setAppMsg("");
    try {
      const update = await checkApp();
      if (!update) {
        setAppUpdate(null);
        setAppMsg("已是最新版本");
        setAppPhase("idle");
        return;
      }
      setAppUpdate(update);
      setAppMsg(`发现新版本 ${update.version}`);
      setAppPhase("idle");
    } catch (e) {
      setAppUpdate(null);
      setAppMsg(String(e));
      setAppPhase("error");
    }
  }, []);

  // task-035：下载并安装应用更新
  const downloadAndInstall = useCallback(async () => {
    if (!appUpdate) return;
    setAppPhase("downloading");
    try {
      await appUpdate.downloadAndInstall((event) => {
        // 进度事件：下载中（event 类型 Progress 含 downloaded/total）
        void event;
      });
      await relaunch();
    } catch (e) {
      setAppMsg(String(e));
      setAppPhase("error");
    }
  }, [appUpdate]);

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

      {/* task-042：离线黄条 / 新版本蓝条 / 最新绿条 */}
      {phase === "idle" && info?.status === "offline" && (
        <div
          style={{
            color: "#92400e",
            background: "#fef3c7",
            border: "1px solid #fcd34d",
            padding: "8px 12px",
            borderRadius: 6,
            fontSize: 13,
            marginBottom: 12,
          }}
        >
          离线模式：使用本地规则库 v{info.localVersion}（{info.message}）
        </div>
      )}
      {phase === "idle" && info?.status === "newVersion" && (
        <div
          style={{
            color: "#1e40af",
            background: "#dbeafe",
            border: "1px solid #93c5fd",
            padding: "8px 12px",
            borderRadius: 6,
            fontSize: 13,
            marginBottom: 12,
          }}
        >
          发现新版本 v{info.remoteVersion}，点「应用更新」下载
        </div>
      )}
      {phase === "idle" && info?.status === "upToDate" && (
        <div
          style={{
            color: "#166534",
            background: "#dcfce7",
            border: "1px solid #86efac",
            padding: "8px 12px",
            borderRadius: 6,
            fontSize: 13,
            marginBottom: 12,
          }}
        >
          规则库已是最新（v{info.localVersion}）
        </div>
      )}

      <div style={cardStyle}>
        <div style={rowStyle}>
          <span>当前版本</span>
          <span style={monoStyle}>{info?.localVersion ?? "-"}</span>
        </div>
        <div style={rowStyle}>
          <span>远端版本</span>
          <span style={monoStyle}>
            {phase === "checking" ? "检查中…" : info?.remoteVersion ?? "（离线）"}
          </span>
        </div>
        <div style={rowStyle}>
          <span>当前 SHA-256</span>
          <span style={monoStyle}>
            {info?.localSha256?.slice(0, 16) ?? "-"}…
          </span>
        </div>
        <div style={rowStyle}>
          <span>状态</span>
          <span style={{ fontWeight: 600 }}>
            {info == null
              ? "-"
              : info.status === "newVersion"
                ? `有新版本 v${info.remoteVersion ?? ""} 可用`
                : info.status === "offline"
                  ? "离线模式（本地规则库可用）"
                  : "已是最新"}
          </span>
        </div>

        <div style={{ marginTop: 16, display: "flex", gap: 8 }}>
          <button
            onClick={() => check(true)}
            disabled={phase === "checking" || phase === "applying"}
          >
            {phase === "checking"
              ? "检查中…"
              : phase === "error"
                ? "重试"
                : "检查更新"}
          </button>

          {info?.status === "newVersion" && (
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

          {/* task-044：灰色「回滚到上一版」按钮，仅在存在 .bak 时显示 */}
          {hasBak && (
            <button
              onClick={rollback}
              disabled={phase === "checking" || phase === "applying" || rollingBack}
              style={{
                padding: "6px 14px",
                borderRadius: 6,
                border: "1px solid #9ca3af",
                background: "#f3f4f6",
                color: "#374151",
                cursor:
                  phase === "checking" || phase === "applying" || rollingBack
                    ? "not-allowed"
                    : "pointer",
              }}
            >
              {rollingBack ? "回滚中…" : "回滚到上一版"}
            </button>
          )}
        </div>
      </div>

      {/* task-035：应用自更新卡片（tauri updater） */}
      <h2 style={{ margin: "24px 0 12px", fontSize: 16 }}>应用更新</h2>
      <div style={cardStyle}>
        <div style={rowStyle}>
          <span>更新状态</span>
          <span
            style={{
              color:
                appPhase === "error"
                  ? "#b91c1c"
                  : appUpdate
                    ? "#b45309"
                    : "#166534",
            }}
          >
            {appPhase === "checking"
              ? "检查中…"
              : appPhase === "downloading"
                ? "下载安装中…"
                : appMsg || "尚未检查"}
          </span>
        </div>
        {appUpdate && (
          <div style={rowStyle}>
            <span>新版本</span>
            <span style={monoStyle}>{appUpdate.version}</span>
          </div>
        )}
        <div style={{ marginTop: 16, display: "flex", gap: 8 }}>
          <button
            onClick={checkApplication}
            disabled={appPhase === "checking" || appPhase === "downloading"}
          >
            {appPhase === "checking" ? "检查中…" : "检查应用更新"}
          </button>
          {appUpdate && (
            <button
              onClick={downloadAndInstall}
              disabled={appPhase === "checking" || appPhase === "downloading"}
              style={{
                padding: "6px 14px",
                borderRadius: 6,
                border: "1px solid #16a34a",
                background: appPhase === "downloading" ? "#bbf7d0" : "#16a34a",
                color: "#fff",
                cursor:
                  appPhase === "checking" || appPhase === "downloading"
                    ? "not-allowed"
                    : "pointer",
              }}
            >
              {appPhase === "downloading" ? "安装中…" : "立即更新并重启"}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
