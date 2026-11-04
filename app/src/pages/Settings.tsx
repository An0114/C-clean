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
  // task-061：自动扫描（只扫描不删除）。默认关闭 + daily。
  autoScanEnabled: boolean;
  autoScanInterval: ScanInterval;
}

/** task-061：自动扫描间隔，与 `engine::prefs::{SCAN_DAILY, SCAN_WEEKLY, SCAN_MONTHLY}` 对齐。 */
type ScanInterval = "daily" | "weekly" | "monthly";

/**
 * task-073：系统级定时扫描的任务状态。
 *
 * 与 `engine::ipc::commands::auto_scan::AutoScanTaskStatus`（camelCase）对齐。
 * 字段名对不上会全 undefined，界面就会一直显示「未注册」——比报错更糟。
 */
interface AutoScanTaskStatus {
  registered: boolean;
  taskName: string;
  time: string;
}

const SCAN_OPTIONS: { value: ScanInterval; label: string }[] = [
  { value: "daily", label: "每天" },
  { value: "weekly", label: "每周" },
  { value: "monthly", label: "每月" },
];

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
    autoScanEnabled: false,
    autoScanInterval: "daily",
  });
  const [prefsSaving, setPrefsSaving] = useState<boolean>(false);
  // task-058：白名单编辑框的原始文本（每行一个进程名），提交时才解析成数组。
  const [whitelistText, setWhitelistText] = useState<string>("");
  const [whitelistSaving, setWhitelistSaving] = useState<boolean>(false);

  // task-062：开机自启。**不存 prefs**——真实来源是系统注册表（autostart 插件），
  // 用户在「任务管理器 → 启动」里手动禁用也能被正确反映。
  const [autostartEnabled, setAutostartEnabled] = useState<boolean>(false);
  const [autostartSaving, setAutostartSaving] = useState<boolean>(false);

  useEffect(() => {
    (async () => {
      try {
        setAutostartEnabled(Boolean(await api<boolean>("get_autostart_enabled", {})));
      } catch {
        // 读失败保持 false（不谎报「已开启」）
      }
    })();
  }, []);

  const onToggleAutostart = useCallback(async (next: boolean) => {
    setAutostartEnabled(next);
    setAutostartSaving(true);
    try {
      // 用后端回读的真实状态覆盖（避免 UI 与注册表不一致）
      const actual = await api<boolean>("set_autostart_enabled", { enabled: next });
      setAutostartEnabled(Boolean(actual));
    } catch (e) {
      setAutostartEnabled(!next);
      setToast({ ok: false, text: `设置开机自启失败：${String(e)}` });
    } finally {
      setAutostartSaving(false);
    }
  }, []);

  // task-073：系统级定时扫描（Windows 任务计划）。
  // 状态来源是「任务是否真的注册在系统里」，不是本地 prefs——所以每次进页面
  // 都要问一次后端，不能拿 prefs 猜。
  const [taskStatus, setTaskStatus] = useState<AutoScanTaskStatus | null>(null);
  const [taskSaving, setTaskSaving] = useState<boolean>(false);
  const [taskError, setTaskError] = useState<string>("");

  useEffect(() => {
    (async () => {
      try {
        setTaskStatus(await api<AutoScanTaskStatus>("auto_scan_status", {}));
        setTaskError("");
      } catch (e) {
        // **不谎报「未注册」**：查不到可能是 schtasks 被组策略/安全软件拦了，
        // 用户需要知道真实原因，否则会以为开关一直是关的。
        setTaskStatus(null);
        setTaskError(String(e));
      }
    })();
  }, []);

  const onToggleTask = useCallback(
    async (next: boolean) => {
      setTaskSaving(true);
      setTaskError("");
      try {
        // 时间固定 09:00：接口支持任意 HH:MM，但界面不暴露时间选择——
        // 一个「每天几点」的输入框换不来多少价值，却多一个可能填错的东西。
        const st = next
          ? await api<AutoScanTaskStatus>("auto_scan_enable", { time: "09:00" })
          : await api<AutoScanTaskStatus>("auto_scan_disable", {});
        // 用后端回读的状态覆盖（schtasks 可能返回 0 但任务被立刻禁掉）
        setTaskStatus(st);
        setToast({
          ok: true,
          text: next ? "已注册系统级定时扫描（每天 09:00）" : "已取消系统级定时扫描",
        });
      } catch (e) {
        setTaskError(String(e));
        setToast({ ok: false, text: `系统级定时设置失败：${String(e)}` });
      } finally {
        setTaskSaving(false);
      }
    },
    []
  );

  useEffect(() => {
    (async () => {
      try {
        const p = await api<Prefs>("get_prefs", {});
        // p 可能为 null（IPC 异常 / 旧后端），统一按默认值兜住。
        const wl = p?.restartWhitelist ?? [];
        const th = p?.theme;
        const si = p?.autoScanInterval;
        setPrefs({
          autoRestartOnClose: Boolean(p?.autoRestartOnClose),
          restartWhitelist: wl,
          theme: th === "light" || th === "dark" || th === "system" ? th : "system",
          autoScanEnabled: Boolean(p?.autoScanEnabled),
          autoScanInterval:
            si === "weekly" || si === "monthly" || si === "daily" ? si : "daily",
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

  // task-061：自动扫描开关。开启后后台会按间隔静默扫描（只扫描、不删除）。
  const onToggleAutoScan = useCallback(
    async (next: boolean) => {
      setPrefs((p) => ({ ...p, autoScanEnabled: next }));
      setPrefsSaving(true);
      try {
        await savePrefs({ autoScanEnabled: next });
      } catch (e) {
        setPrefs((p) => ({ ...p, autoScanEnabled: !next }));
        setToast({ ok: false, text: `保存设置失败：${String(e)}` });
      } finally {
        setPrefsSaving(false);
      }
    },
    [savePrefs]
  );

  const onChangeScanInterval = useCallback(
    async (next: ScanInterval) => {
      const prev = prefs.autoScanInterval;
      if (prev === next) return;
      setPrefs((p) => ({ ...p, autoScanInterval: next }));
      setPrefsSaving(true);
      try {
        await savePrefs({ autoScanInterval: next });
      } catch (e) {
        setPrefs((p) => ({ ...p, autoScanInterval: prev }));
        setToast({ ok: false, text: `保存设置失败：${String(e)}` });
      } finally {
        setPrefsSaving(false);
      }
    },
    [prefs.autoScanInterval, savePrefs]
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

      {/* task-078：用户自定义规则（紧跟在「规则库」后面——两者是同一件事的两层：
          厂商下发的那层在上，用户自己加的那层在下） */}
      <UserRulesSection />

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

      {/* task-061：自动扫描。MVP 只扫描 + 通知，**绝不自动删除**——安全第一。 */}
      <h2 style={{ margin: "24px 0 12px", fontSize: 16 }}>自动扫描</h2>
      <div style={cardStyle}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div>
            <div style={{ fontSize: 13, fontWeight: 600 }}>定期在后台扫描</div>
            <div style={{ fontSize: 12, color: "var(--text-muted)", marginTop: 4, lineHeight: 1.5 }}>
              按设定间隔在后台扫描一遍，完成后发系统通知告诉你「发现多少可清理项」。
              <b>只扫描，不会自动删除任何文件</b>——要清理仍需你手动确认。默认关闭。
            </div>
          </div>
          <label
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              cursor: prefsSaving ? "wait" : "pointer",
              whiteSpace: "nowrap",
            }}
          >
            <input
              type="checkbox"
              checked={prefs.autoScanEnabled}
              disabled={prefsSaving}
              onChange={(e) => onToggleAutoScan(e.target.checked)}
            />
            <span style={{ fontSize: 12, color: "var(--text)" }}>
              {prefs.autoScanEnabled ? "已开启" : "已关闭"}
            </span>
          </label>
        </div>

        <div
          style={{
            marginTop: 14,
            borderTop: "1px solid var(--border)",
            paddingTop: 14,
            display: "flex",
            alignItems: "center",
            gap: 10,
            flexWrap: "wrap",
          }}
        >
          <span style={{ fontSize: 12, color: "var(--text)" }}>扫描间隔：</span>
          {SCAN_OPTIONS.map((opt) => (
            <label
              key={opt.value}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 5,
                fontSize: 12,
                color: prefs.autoScanEnabled ? "var(--text)" : "var(--text-muted)",
                cursor: prefsSaving || !prefs.autoScanEnabled ? "not-allowed" : "pointer",
              }}
            >
              <input
                type="radio"
                name="scan-interval"
                checked={prefs.autoScanInterval === opt.value}
                disabled={prefsSaving || !prefs.autoScanEnabled}
                onChange={() => onChangeScanInterval(opt.value)}
              />
              {opt.label}
            </label>
          ))}
        </div>
        {!prefs.autoScanEnabled && (
          <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 8 }}>
            先开启上面的开关，间隔选择才会生效。
          </div>
        )}

        {/* task-073：系统级定时（Windows 任务计划）。
            与上面的「应用内轮询」是两层：那个只在应用运行时生效，这个能在
            应用完全没跑的时候把程序拉起来扫一遍再退出。 */}
        <div style={{ marginTop: 16, borderTop: "1px solid var(--border)", paddingTop: 14 }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <div>
              <div style={{ fontSize: 13, fontWeight: 600, color: "var(--text)" }}>
                系统级定时（应用未运行时也扫描）
              </div>
              <div
                style={{ fontSize: 12, color: "var(--text-muted)", marginTop: 4, lineHeight: 1.5 }}
              >
                在 Windows「任务计划程序」里注册一个每天{" "}
                <b>{taskStatus?.time ?? "09:00"}</b>{" "}
                运行的任务，到点静默启动本程序扫一遍、发通知、然后自动退出
                （<b>只扫描，不删除</b>）。关掉主开关后即使任务还在也会跳过。
              </div>
            </div>
            <label
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                whiteSpace: "nowrap",
                cursor: taskSaving || !prefs.autoScanEnabled ? "not-allowed" : "pointer",
              }}
            >
              <input
                type="checkbox"
                checked={Boolean(taskStatus?.registered)}
                disabled={taskSaving || !prefs.autoScanEnabled}
                onChange={(e) => onToggleTask(e.target.checked)}
                aria-label="系统级定时扫描"
              />
              <span style={{ fontSize: 12, color: "var(--text)" }}>
                {taskStatus === null
                  ? "查询中…"
                  : taskStatus.registered
                    ? "已注册"
                    : "未注册"}
              </span>
            </label>
          </div>

          {taskStatus && (
            <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 8, lineHeight: 1.6 }}>
              任务名 <code style={{ fontSize: 11 }}>{taskStatus.taskName}</code>
              ，可在「任务计划程序」里核对。
            </div>
          )}

          {!prefs.autoScanEnabled && (
            <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 8 }}>
              先开启上面的「定期在后台扫描」，系统级定时才会生效。
            </div>
          )}

          {/* 查询/注册失败要如实显示原因。绝不能显示成「未注册」——
              那会让用户以为开关是关的，实际可能是被组策略拦了。 */}
          {taskError && (
            <div
              style={{
                marginTop: 8,
                background: "#fef2f2",
                border: "1px solid #fecaca",
                color: "#b91c1c",
                padding: "6px 9px",
                borderRadius: 4,
                fontSize: 11,
                lineHeight: 1.6,
                wordBreak: "break-all",
              }}
            >
              系统级定时不可用：{taskError}
            </div>
          )}
        </div>
      </div>

      {/* task-062：开机自启。状态真实来源是系统注册表（autostart 插件），不存 prefs。 */}
      <h2 style={{ margin: "24px 0 12px", fontSize: 16 }}>启动</h2>
      <div style={cardStyle}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div>
            <div style={{ fontSize: 13, fontWeight: 600 }}>开机自启</div>
            <div style={{ fontSize: 12, color: "var(--text-muted)", marginTop: 4, lineHeight: 1.5 }}>
              登录 Windows 后自动启动 C-Disk Cleaner，并在托盘常驻。
              <b>默认关闭</b>；关闭主窗口不会退出程序，需要从托盘菜单「退出」才真正结束。
            </div>
            {/* task-073：把「状态存在哪」写出来。用户想确认有没有真生效时，
                自己能去任务管理器核对，比反复开关一遍更可靠。 */}
            <div style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 6, lineHeight: 1.6 }}>
              状态取自系统注册表{" "}
              <code style={{ fontSize: 11 }}>
                HKCU\Software\Microsoft\Windows\CurrentVersion\Run
              </code>
              ，也可在「任务管理器 → 启动」里核对。在那边手动禁用后，这里会如实显示为已关闭。
            </div>
          </div>
          <label
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              cursor: autostartSaving ? "wait" : "pointer",
              whiteSpace: "nowrap",
            }}
          >
            <input
              type="checkbox"
              checked={autostartEnabled}
              disabled={autostartSaving}
              onChange={(e) => onToggleAutostart(e.target.checked)}
            />
            <span style={{ fontSize: 12, color: "var(--text)" }}>
              {autostartEnabled ? "已开启" : "已关闭"}
            </span>
          </label>
        </div>
      </div>
    </div>
  );
}

// ===================== task-078：用户自定义规则 =====================
//
// # 为什么单独一个组件
//
// Settings 已经 1000 行了（规则库 / 应用更新 / 外观 / 行为偏好 / 自动扫描 / 启动）。
// 把「列表 + 表单 + 增删改 + 开关」再塞进主组件，改动任何一块都要在千行里找位置。
//
// # 与内置规则库的分工（界面上要说清楚）
//
// 上面那块「规则库」是**厂商下发的**（受 SHA-256 保护、随版本更新整份替换）；
// 这块是**用户自己的**（存本地 DB、永不被规则库更新覆盖）。用户最容易误解的就是
// 「我加的规则会不会被更新冲掉」——所以标题下面直接写清。

interface UserRule {
  id: number;
  name: string;
  pattern: string;
  /** "L1" / "L2"（L3 是红线等级，后端会拒）。 */
  riskLevel: string;
  enabled: boolean;
  createdAt: number;
  updatedAt: number;
}

/** 表单草稿：`id === null` 表示「新增」。 */
interface RuleDraft {
  id: number | null;
  name: string;
  pattern: string;
  riskLevel: string;
}

const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: "6px 8px",
  borderRadius: 6,
  border: "1px solid var(--border)",
  background: "var(--surface)",
  color: "var(--text)",
  fontSize: 13,
  boxSizing: "border-box",
};

function UserRulesSection() {
  const [rules, setRules] = useState<UserRule[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState("");
  const [notice, setNotice] = useState("");
  const [draft, setDraft] = useState<RuleDraft | null>(null);
  const [saving, setSaving] = useState(false);
  const [busyId, setBusyId] = useState<number | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await api<UserRule[]>("user_rules_list", {});
      setRules(Array.isArray(list) ? list : []);
      setErr("");
    } catch (e) {
      // 读不出来时保留空列表 + 显示真实原因，**不显示「暂无规则」**——
      // 那会让用户以为自己的规则被删了。
      setErr(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const save = useCallback(async () => {
    if (!draft) return;
    if (!draft.name.trim() || !draft.pattern.trim()) {
      setErr("规则名与路径模式都不能为空");
      return;
    }
    setSaving(true);
    setErr("");
    try {
      if (draft.id === null) {
        await api("user_rules_add", {
          name: draft.name.trim(),
          pattern: draft.pattern.trim(),
          riskLevel: draft.riskLevel,
        });
        setNotice("已新增规则，下次扫描生效");
      } else {
        // 更新时把 enabled 原样带回，避免「改名字顺手把开关重置成开」
        const old = rules.find((r) => r.id === draft.id);
        await api("user_rules_update", {
          id: draft.id,
          name: draft.name.trim(),
          pattern: draft.pattern.trim(),
          riskLevel: draft.riskLevel,
          enabled: old?.enabled ?? true,
        });
        setNotice("已保存修改，下次扫描生效");
      }
      setDraft(null);
      await load();
    } catch (e) {
      // 后端的校验原因（模式太宽泛 / 指向受保护位置 / 模式重复）直接显示
      setErr(String(e));
    } finally {
      setSaving(false);
    }
  }, [draft, rules, load]);

  const toggle = useCallback(
    async (rule: UserRule) => {
      setBusyId(rule.id);
      setErr("");
      try {
        await api("user_rules_set_enabled", { id: rule.id, enabled: !rule.enabled });
        setNotice(rule.enabled ? "已停用，下次扫描不再包含" : "已启用，下次扫描生效");
        await load();
      } catch (e) {
        setErr(String(e));
      } finally {
        setBusyId(null);
      }
    },
    [load],
  );

  const remove = useCallback(
    async (rule: UserRule) => {
      if (!confirm(`删除规则「${rule.name}」？\n\n已扫描出的条目不受影响，但下次扫描不会再包含这些路径。`)) {
        return;
      }
      setBusyId(rule.id);
      setErr("");
      try {
        await api("user_rules_delete", { id: rule.id });
        setNotice("已删除");
        await load();
      } catch (e) {
        setErr(String(e));
      } finally {
        setBusyId(null);
      }
    },
    [load],
  );

  return (
    <>
      <h2 style={{ margin: "24px 0 12px", fontSize: 16 }}>自定义规则</h2>
      <div style={{ ...cardStyle, maxWidth: 720 }}>
        <div style={{ fontSize: 12, color: "var(--text-muted)", marginBottom: 10, lineHeight: 1.7 }}>
          这些规则只存在本机，<strong>不会被规则库更新覆盖</strong>。扫描时会额外扫描匹配的路径；
          与内置规则重叠的部分以<strong>内置规则为准</strong>（不会重复列出）。
          <br />
          路径模式支持 <code>*</code>（本层任意文件名）、<code>**</code>（跨多层目录）、
          <code>?</code>（单个字符），可写 <code>%LOCALAPPDATA%</code> 这类环境变量。
          例：<code>D:\MyCache\*</code>、<code>%LOCALAPPDATA%\Foo\**\*.tmp</code>。
        </div>

        {notice && (
          <div style={{ fontSize: 12, color: "#166534", marginBottom: 8 }}>{notice}</div>
        )}
        {err && (
          <div style={{ fontSize: 12, color: "#b91c1c", marginBottom: 8, whiteSpace: "pre-wrap" }}>
            {err}
          </div>
        )}

        {loading ? (
          <div style={{ color: "var(--text-muted)", fontSize: 13 }}>加载中...</div>
        ) : rules.length === 0 ? (
          <div style={{ color: "var(--text-muted)", fontSize: 13, marginBottom: 12 }}>
            还没有自定义规则。内置规则覆盖不到的缓存目录可以在这里加。
          </div>
        ) : (
          <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 13, marginBottom: 12 }}>
            <thead>
              <tr style={{ color: "var(--text-muted)", textAlign: "left" }}>
                <th style={{ padding: "6px 8px", fontWeight: 500 }}>名称</th>
                <th style={{ padding: "6px 8px", fontWeight: 500 }}>路径模式</th>
                <th style={{ padding: "6px 8px", fontWeight: 500 }}>风险</th>
                <th style={{ padding: "6px 8px", fontWeight: 500 }}>启用</th>
                <th style={{ padding: "6px 8px", fontWeight: 500 }}>操作</th>
              </tr>
            </thead>
            <tbody>
              {rules.map((r) => (
                <tr key={r.id} style={{ borderTop: "1px solid var(--border)" }}>
                  <td style={{ padding: "6px 8px", color: "var(--text)" }}>{r.name}</td>
                  <td style={{ padding: "6px 8px", ...monoStyle }}>{r.pattern}</td>
                  <td style={{ padding: "6px 8px" }}>
                    <span
                      title={
                        r.riskLevel === "L2"
                          ? "L2：需逐项确认（不会被「全选 L1」选中）"
                          : "L1：安全缓存，可一键全选"
                      }
                      style={{
                        padding: "1px 6px",
                        borderRadius: 4,
                        fontSize: 11,
                        background: r.riskLevel === "L2" ? "#fef3c7" : "#dcfce7",
                        color: r.riskLevel === "L2" ? "#92400e" : "#166534",
                      }}
                    >
                      {r.riskLevel}
                    </span>
                  </td>
                  <td style={{ padding: "6px 8px" }}>
                    <input
                      type="checkbox"
                      aria-label={`启用规则 ${r.name}`}
                      checked={r.enabled}
                      disabled={busyId === r.id}
                      onChange={() => void toggle(r)}
                    />
                  </td>
                  <td style={{ padding: "6px 8px" }}>
                    <button
                      onClick={() =>
                        setDraft({
                          id: r.id,
                          name: r.name,
                          pattern: r.pattern,
                          riskLevel: r.riskLevel,
                        })
                      }
                      disabled={busyId === r.id}
                      style={{ marginRight: 8, cursor: "pointer" }}
                    >
                      编辑
                    </button>
                    <button
                      onClick={() => void remove(r)}
                      disabled={busyId === r.id}
                      style={{ color: "#b91c1c", cursor: "pointer" }}
                    >
                      删除
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        <button
          onClick={() => {
            setErr("");
            setDraft({ id: null, name: "", pattern: "", riskLevel: "L1" });
          }}
          style={{ padding: "6px 14px", borderRadius: 6, cursor: "pointer" }}
        >
          新增规则
        </button>
      </div>

      {draft && (
        <div
          role="dialog"
          aria-modal="true"
          aria-label={draft.id === null ? "新增自定义规则" : "编辑自定义规则"}
          style={{
            position: "fixed",
            inset: 0,
            background: "rgba(0,0,0,0.35)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 100,
          }}
        >
          <div
            style={{
              background: "var(--surface)",
              border: "1px solid var(--border)",
              borderRadius: 10,
              padding: "20px 24px",
              width: 460,
              maxWidth: "92vw",
            }}
          >
            <h3 style={{ margin: "0 0 14px", fontSize: 15, color: "var(--text)" }}>
              {draft.id === null ? "新增自定义规则" : "编辑自定义规则"}
            </h3>

            <label style={{ display: "block", fontSize: 12, color: "var(--text-muted)", marginBottom: 4 }}>
              名称
            </label>
            <input
              aria-label="规则名称"
              value={draft.name}
              onChange={(e) => setDraft({ ...draft, name: e.target.value })}
              placeholder="例如：我的游戏缓存"
              style={{ ...inputStyle, marginBottom: 12 }}
            />

            <label style={{ display: "block", fontSize: 12, color: "var(--text-muted)", marginBottom: 4 }}>
              路径模式
            </label>
            <input
              aria-label="路径模式"
              value={draft.pattern}
              onChange={(e) => setDraft({ ...draft, pattern: e.target.value })}
              placeholder="D:\MyCache\*"
              style={{ ...inputStyle, marginBottom: 4, fontFamily: "monospace" }}
            />
            <div style={{ fontSize: 11, color: "var(--text-muted)", marginBottom: 12 }}>
              必须写到具体目录（<code>D:\**</code> 这种会扫整盘，会被拒绝）
            </div>

            <label style={{ display: "block", fontSize: 12, color: "var(--text-muted)", marginBottom: 4 }}>
              风险等级
            </label>
            <select
              aria-label="风险等级"
              value={draft.riskLevel}
              onChange={(e) => setDraft({ ...draft, riskLevel: e.target.value })}
              style={{ ...inputStyle, marginBottom: 16 }}
            >
              <option value="L1">L1 — 安全缓存，可一键全选</option>
              <option value="L2">L2 — 需逐项确认</option>
            </select>

            <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
              <button onClick={() => setDraft(null)} disabled={saving} style={{ cursor: "pointer" }}>
                取消
              </button>
              <button
                onClick={() => void save()}
                disabled={saving}
                style={{
                  padding: "6px 14px",
                  borderRadius: 6,
                  border: "1px solid #2563eb",
                  background: saving ? "#bfdbfe" : "#2563eb",
                  color: "#fff",
                  cursor: saving ? "not-allowed" : "pointer",
                }}
              >
                {saving ? "保存中…" : "保存"}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
