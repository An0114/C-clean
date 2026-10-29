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
import { getVersion } from "@tauri-apps/api/app";
import { classifyUpdateError } from "../utils/update";

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

// task-057：用户偏好（与 engine::prefs::Prefs serde camelCase 对齐）。
interface Prefs {
  autoRestartOnClose: boolean;
  // task-058：重启白名单（进程映像名，支持 `*` 通配）。空数组 = 不限制。
  restartWhitelist: string[];
  // task-061：主题三态。默认 "system"（跟随系统）。
  theme: ThemeMode;
}

/** task-061：主题三态。与 `engine::prefs::{THEME_SYSTEM, THEME_LIGHT, THEME_DARK}` 对齐。 */
type ThemeMode = "system" | "light" | "dark";

const THEME_OPTIONS: { value: ThemeMode; label: string; hint: string }[] = [
  { value: "system", label: "跟随系统", hint: "随 Windows 的深浅色设置自动切换" },
  { value: "light", label: "浅色", hint: "始终使用浅色界面" },
  { value: "dark", label: "深色", hint: "始终使用深色界面" },
];

type Phase =
  | "loading"
  | "idle"
  | "checking"
  | "applying"
  | "error";

const cardStyle: React.CSSProperties = {
  border: "1px solid var(--border)",
  borderRadius: 8,
  padding: "20px 24px",
  background: "var(--surface)",
  maxWidth: 560,
};

const rowStyle: React.CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  padding: "6px 0",
  fontSize: 13,
  color: "var(--text)",
};

const monoStyle: React.CSSProperties = {
  fontFamily: "monospace",
  fontSize: 12,
  color: "var(--text-muted)",
};

/**
 * task-055 单元 5：设置页可通知外层「手动检查发现了新版本」。
 *
 * 用途：banner 有「稍后提醒」持久化（忽略某版本），但用户**手动**点「检查应用更新」
 * 时应该强制把 banner 显示出来——手动动作的意图就是「我想看到结果」。
 */
interface SettingsProps {
  onUpdateFound?: (version: string) => void;
  /** task-061：主题切换后通知外层立刻应用（不必重读 prefs / 重启）。 */
  onThemeChange?: (theme: ThemeMode) => void;
}

export default function Settings({ onUpdateFound, onThemeChange }: SettingsProps) {
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
  // task-055 单元 5：当前应用版本（用于「当前 vX / 发现 vY」文案）。
  // 取不到（ACL 未放行 / 非 Tauri 环境）就留空，文案自动退化成不带「当前」。
  const [appLocalVersion, setAppLocalVersion] = useState<string>("");

  // task-057：用户偏好（默认 false，与后端 fail-closed 一致）。
  const [prefs, setPrefs] = useState<Prefs>({
    autoRestartOnClose: false,
    restartWhitelist: [],
    // task-061：默认跟随系统（与后端缺行时的默认值一致）。
    theme: "system",
  });
  const [prefsSaving, setPrefsSaving] = useState<boolean>(false);
  // task-058：白名单编辑框的原始文本（每行一个进程名），提交时才解析成数组。
  const [whitelistText, setWhitelistText] = useState<string>("");
  const [whitelistSaving, setWhitelistSaving] = useState<boolean>(false);

  useEffect(() => {
    (async () => {
      try {
        const p = await api<Prefs>("get_prefs", {});
        // p 可能为 null（IPC 异常 / 旧后端），统一按默认值兜住。
        const wl = p?.restartWhitelist ?? [];
        const th = p?.theme;
        setPrefs({
          autoRestartOnClose: Boolean(p?.autoRestartOnClose),
          restartWhitelist: wl,
          theme: th === "light" || th === "dark" || th === "system" ? th : "system",
        });
        setWhitelistText(wl.join("\n"));
      } catch {
        // 读失败保持默认（fail-closed），不打扰用户。
      }
    })();
  }, []);

  /**
   * task-061：统一提交入口。
   *
   * 后端 `set_prefs` 是**整体覆盖**语义——任何一处保存漏传字段，都会把那个字段
   * 静默清空（task-058 就踩过：只传 autoRestartOnClose 把白名单冲掉了）。
   * 所以所有保存都必须走这里把 patch 合并到当前 prefs 上，禁止手拼对象。
   */
  const savePrefs = useCallback(
    async (patch: Partial<Prefs>): Promise<Prefs | null> => {
      const next: Prefs = { ...prefs, ...patch };
      const saved = await api<Prefs>("set_prefs", { prefs: next });
      return saved ?? null;
    },
    [prefs]
  );

  const onToggleAutoRestart = useCallback(
    async (next: boolean) => {
      // 乐观更新：先切 UI，写库失败再回滚。
      setPrefs((p) => ({ ...p, autoRestartOnClose: next }));
      setPrefsSaving(true);
      try {
        await savePrefs({ autoRestartOnClose: next });
      } catch (e) {
        // 回滚 UI。
        setPrefs((p) => ({ ...p, autoRestartOnClose: !next }));
        setToast({ ok: false, text: `保存设置失败：${String(e)}` });
      } finally {
        setPrefsSaving(false);
      }
    },
    [savePrefs]
  );

  // task-061：主题切换。先本地应用（不卡顿），再落库；失败回滚并提示。
  const onSaveTheme = useCallback(
    async (next: ThemeMode) => {
      const prev = prefs.theme;
      if (prev === next) return;
      setPrefs((p) => ({ ...p, theme: next }));
      onThemeChange?.(next);
      setPrefsSaving(true);
      try {
        await savePrefs({ theme: next });
      } catch (e) {
        setPrefs((p) => ({ ...p, theme: prev }));
        onThemeChange?.(prev);
        setToast({ ok: false, text: `保存主题失败：${String(e)}` });
      } finally {
        setPrefsSaving(false);
      }
    },
    [prefs.theme, savePrefs, onThemeChange]
  );

  // task-058：保存白名单。按行切分 → 去空白 → 丢空行；空结果 = 清空（不限制）。
  const onSaveWhitelist = useCallback(async () => {
    const list = whitelistText
      .split("\n")
      .map((s) => s.trim())
      .filter((s) => s.length > 0);

    setWhitelistSaving(true);
    try {
      const saved = await savePrefs({ restartWhitelist: list });
      const wl = saved?.restartWhitelist ?? list;
      setPrefs((p) => ({ ...p, restartWhitelist: wl }));
      // 用后端归一化后的结果回填（去空白 / 小写），让用户看到真实生效的内容。
      setWhitelistText(wl.join("\n"));
      setToast({
        ok: true,
        text: wl.length === 0 ? "白名单已清空（不限制重启）" : `白名单已保存（${wl.length} 条）`,
      });
    } catch (e) {
      setToast({ ok: false, text: `保存白名单失败：${String(e)}` });
    } finally {
      setWhitelistSaving(false);
    }
  }, [whitelistText, savePrefs]);

  useEffect(() => {
    (async () => {
      try {
        setAppLocalVersion(await getVersion());
      } catch {
        setAppLocalVersion("");
      }
    })();
  }, []);

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
    // task-047：点「应用更新」前先 window.confirm，避免手滑直接覆盖本地 rules。
    const localVer = info?.localVersion ?? "?";
    const remoteVer = info?.remoteVersion ?? "?";
    const ok = window.confirm(
      `即将应用规则库 v${remoteVer}（当前本地 v${localVer}）。\n` +
        `应用后旧版将备份为 cache_rules.json.bak，可随时通过「回滚到上一版」恢复。\n` +
        `是否继续？`
    );
    if (!ok) return;

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
  }, [check, info]);

  // task-035：检查应用自更新（tauri updater）
  //
  // task-055 单元 5：结果文案带「当前版本」，失败走中文映射。
  // 现状（task-035）已有「检查中…」+ disabled，本轮补的是文案与错误映射。
  const checkApplication = useCallback(async () => {
    setAppPhase("checking");
    setAppMsg("");
    const current = appLocalVersion ? `当前 v${appLocalVersion}` : "当前版本";
    try {
      const update = await checkApp();
      if (!update) {
        setAppUpdate(null);
        setAppMsg(`${current}，已是最新版本`);
        setAppPhase("idle");
        return;
      }
      setAppUpdate(update);
      setAppMsg(`${current} / 发现 v${update.version}`);
      setAppPhase("idle");
      // task-055 单元 4+5：手动检查到新版本 → 通知外层强制显示 banner
      // （忽略过该版本的「稍后提醒」不再拦着）。
      onUpdateFound?.(update.version);
    } catch (e) {
      setAppUpdate(null);
      setAppMsg(classifyUpdateError(String(e)));
      setAppPhase("error");
    }
  }, [appLocalVersion, onUpdateFound]);

  // task-035：下载并安装应用更新
  //
  // task-055 单元 5：失败文案复用 classifyUpdateError（与 banner 同一套映射）。
  // 这里的进度事件暂不展示（banner 侧已有完整进度条），故回调保留空实现。
  const downloadAndInstall = useCallback(async () => {
    if (!appUpdate) return;
    setAppPhase("downloading");
    try {
      await appUpdate.downloadAndInstall(() => {
        // 事件回调签名见 plugin-updater：downloadAndInstall(onEvent?, options?)
        // Settings 卡片只显示「下载安装中…」，详细进度在顶部 banner。
      });
      await relaunch();
    } catch (e) {
      setAppMsg(classifyUpdateError(String(e)));
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
        <div style={{ color: "var(--text-muted)", fontSize: 13 }}>加载中...</div>
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
                color: "var(--surface)",
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
                border: "1px solid var(--border)",
                background: "var(--border)",
                color: "var(--text)",
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
                color: "var(--surface)",
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

      {/* task-061：外观（主题三态）。放在行为偏好之前——用户先调外观再调行为更顺手。 */}
      <h2 style={{ margin: "24px 0 12px", fontSize: 16 }}>外观</h2>
      <div style={cardStyle}>
        <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 4 }}>主题</div>
        <div style={{ fontSize: 12, color: "var(--text-muted)", marginBottom: 12, lineHeight: 1.6 }}>
          选择后立即生效，并记住你的选择。默认跟随 Windows 的深浅色设置。
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {THEME_OPTIONS.map((opt) => (
            <label
              key={opt.value}
              style={{
                display: "flex",
                alignItems: "flex-start",
                gap: 8,
                cursor: prefsSaving ? "wait" : "pointer",
                padding: "8px 10px",
                borderRadius: 6,
                border:
                  prefs.theme === opt.value
                    ? "1px solid var(--text)"
                    : "1px solid var(--border)",
                background:
                  prefs.theme === opt.value ? "var(--bg)" : "transparent",
              }}
            >
              <input
                type="radio"
                name="theme"
                checked={prefs.theme === opt.value}
                disabled={prefsSaving}
                onChange={() => onSaveTheme(opt.value)}
                style={{ marginTop: 2 }}
              />
              <span>
                <span style={{ fontSize: 13, color: "var(--text)" }}>{opt.label}</span>
                <span
                  style={{
                    display: "block",
                    fontSize: 11,
                    color: "var(--text-muted)",
                    marginTop: 2,
                  }}
                >
                  {opt.hint}
                </span>
              </span>
            </label>
          ))}
        </div>
        <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 12, lineHeight: 1.6 }}>
          注：当前版本已适配「仪表盘」与「设置」页；「清理 / 保险箱 / 历史」三页仍是浅色，
          深色主题下会出现明暗混排。适配工作排在后续版本。
        </div>
      </div>

      {/* task-057：行为偏好卡片——默认关闭，新用户看不到任何自动重启行为 */}
      <h2 style={{ margin: "24px 0 12px", fontSize: 16 }}>行为偏好</h2>
      <div style={cardStyle}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div>
            <div style={{ fontSize: 13, color: "var(--text)", fontWeight: 600 }}>
              自动重启被关闭的进程
            </div>
            <div style={{ fontSize: 12, color: "var(--text-muted)", marginTop: 4, lineHeight: 1.5 }}>
              清理被占用的文件时，若系统支持（Restart Manager 标记为可重启），
              关闭后自动把进程重新拉起。默认关闭。
            </div>
          </div>
          <label style={{ display: "flex", alignItems: "center", gap: 8, cursor: prefsSaving ? "wait" : "pointer" }}>
            <input
              type="checkbox"
              checked={prefs.autoRestartOnClose}
              disabled={prefsSaving}
              onChange={(e) => onToggleAutoRestart(e.target.checked)}
            />
            <span style={{ fontSize: 12, color: "var(--text)" }}>
              {prefs.autoRestartOnClose ? "已开启" : "已关闭"}
            </span>
          </label>
        </div>

        {/* task-058：重启白名单。语义是 fail-closed（见下方说明），不是「只重启这些」 */}
        <div style={{ marginTop: 16, borderTop: "1px solid var(--border)", paddingTop: 16 }}>
          <div style={{ fontSize: 13, color: "var(--text)", fontWeight: 600 }}>
            重启白名单
          </div>
          <div style={{ fontSize: 12, color: "var(--text-muted)", marginTop: 4, lineHeight: 1.6 }}>
            每行一个进程映像名（如 <code>notepad.exe</code>），支持 <code>*</code> 通配（如 <code>*.exe</code>）。
            <br />
            <b>留空 = 不限制</b>（所有可重启进程都会被拉起）。
            <br />
            填了内容后，只有「被关闭的可重启进程<b>全部</b>在此列表中」才会自动重启——
            系统接口一次只能重启整批、无法逐个挑选，所以只要有一个不在名单内就整批不重启，
            避免把您明确排除的进程也拉起来。
          </div>
          <textarea
            aria-label="重启白名单"
            value={whitelistText}
            onChange={(e) => setWhitelistText(e.target.value)}
            disabled={whitelistSaving}
            rows={4}
            placeholder={"notepad.exe\n*.exe"}
            style={{
              marginTop: 8,
              width: "100%",
              boxSizing: "border-box",
              fontFamily: "monospace",
              fontSize: 12,
              padding: "6px 8px",
              borderRadius: 4,
              border: "1px solid var(--border)",
              resize: "vertical",
            }}
          />
          <div style={{ marginTop: 8, display: "flex", alignItems: "center", gap: 8 }}>
            <button onClick={onSaveWhitelist} disabled={whitelistSaving}>
              {whitelistSaving ? "保存中…" : "保存白名单"}
            </button>
            <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
              {prefs.restartWhitelist.length === 0
                ? "当前：不限制"
                : `当前生效 ${prefs.restartWhitelist.length} 条`}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
