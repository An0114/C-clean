// task-026/028：仪表盘数字卡片页 + 开始扫描按钮 + 实时进度条。
// - stats 卡片：invoke('audit_log_stats') 拿 4 个数字。
// - 开始扫描：invoke('scan_start') + listen('scan://progress') 实时进度条。
// 状态：stats loading/ok/error；scan idle/scanning/done/error。

import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { formatBytes } from "../utils/formatBytes";
import { api, apiRaw } from "../utils/invoke";

// engine CleanupStats / ScanProgress 均 #[serde(rename_all = "camelCase")]，
// 前端 interface 必须用 camelCase（debt-cleanup-004 Bug-1+2：之前 snake_case
// 导致 successCount/failedCount/sessionCount/currentPath/scannedItems 全 undefined）。
interface CleanupStats {
  totalBytesFreed: number;
  successCount: number;
  failedCount: number;
  sessionCount: number;
}

// task-028：与 engine::ipc::progress::ScanProgress 真实 shape 对齐
// （任务书假设 {module, done, total}，实际是 {sessionId, percent, currentPath, scannedItems}）。
interface ScanProgress {
  sessionId: number;
  percent: number;
  currentPath: string;
  scannedItems: number;
}

type Status = "loading" | "ok" | "error";
type ScanStatus = "idle" | "scanning" | "done" | "error";

/**
 * task-053 G7.3：`scan_start` resolve 后延迟退订的时长。
 *
 * 后端在命令返回**之前**已经 emit 了 `percent = 100`，但 Tauri 事件是异步
 * 投递到 webview 的，Promise resolve 与事件到达的先后顺序没有保证。原实现
 * 在 resolve 后立刻 `unlisten()`，最后一条事件就被自己退订掉了 —— 这是 G7
 * 「进度条停在 88%/96%」的第三个成因。留 200ms 给投递队列收尾。
 */
const TAIL_UNLISTEN_DELAY_MS = 200;

const cardStyle: React.CSSProperties = {
  border: "1px solid var(--border)",
  borderRadius: 8,
  padding: "16px 20px",
  // task-082：原来写死 `#fff`——深色下是一块白底（task-081 漏了这一处）。
  background: "var(--surface)",
  minHeight: 100,
  // 浅色下是柔和投影；深色下 6% 黑叠在 #1e293b 上几乎不可见，
  // 正好符合「深色不靠阴影分层」的惯例——层次由 border 承担。
  // 不判主题就能同时满足两档，理由同 task-081 的语义色处理。
  boxShadow: "0 1px 3px rgba(0, 0, 0, 0.06)",
};

const labelStyle: React.CSSProperties = {
  fontSize: 12,
  color: "var(--text-muted)",
  marginBottom: 10,
};

const valueStyle = (color: string): React.CSSProperties => ({
  // task-082：28 → 32。仪表盘是「一眼看数」的页面，主数字够大才好扫。
  fontSize: 32,
  fontWeight: 600,
  color,
  // 32px 的行高默认偏挤，给一点呼吸空间
  lineHeight: 1.2,
});

const btnStyle = (disabled: boolean): React.CSSProperties => ({
  padding: "8px 16px",
  fontSize: 14,
  borderRadius: 6,
  border: "1px solid #10b981",
  background: disabled ? "#a7f3d0" : "#10b981",
  color: "#fff",
  cursor: disabled ? "not-allowed" : "pointer",
});

export default function Dashboard({ scanSignal = 0 }: { scanSignal?: number }) {
  const [status, setStatus] = useState<Status>("loading");
  const [data, setData] = useState<CleanupStats | null>(null);
  const [msg, setMsg] = useState<string>("");

  // task-028：扫描状态。
  const [scanStatus, setScanStatus] = useState<ScanStatus>("idle");
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [scanMsg, setScanMsg] = useState<string>("");
  // task-063：取消请求已发出（按钮切「正在停止…」，等扫描真正结束再复位）。
  const [cancelling, setCancelling] = useState(false);

  // task-053 G7.3：延迟退订的兜底定时器句柄（卸载时清掉，避免泄漏）。
  const tailTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (tailTimerRef.current !== null) clearTimeout(tailTimerRef.current);
    },
    []
  );

  const load = useCallback(async () => {
    setStatus("loading");
    setMsg("");
    try {
      const s = await api<CleanupStats>("audit_log_stats", {});
      setData(s);
      setStatus("ok");
    } catch (e) {
      setData(null);
      setMsg(String(e));
      setStatus("error");
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  // task-028：开始扫描——订阅进度事件 → invoke scan_start → 延迟退订 + 刷新 stats。
  //
  // task-053 G7.3：退订时机改为「收到 percent=100 立刻退订 / 否则 200ms 兜底」。
  // 原实现 finally 里同步 unlisten，把后端收尾的那条 100 事件一起退掉了。
  const startScan = useCallback(async () => {
    setScanStatus("scanning");
    // task-063：新一轮扫描开始时清掉上一轮的「正在停止」状态。
    setCancelling(false);
    setScanMsg("");
    setProgress(null);
    // 上一次扫描可能还挂着兜底定时器，先清掉，免得它退订本次的订阅。
    if (tailTimerRef.current !== null) {
      clearTimeout(tailTimerRef.current);
      tailTimerRef.current = null;
    }

    // 用可变句柄承载 unlisten：listen 回调可能早于 await 返回就触发 stop，
    // 那时 `handle.unlisten` 还是 null，必须靠 stopped 标记在赋值后补退订。
    const handle: { unlisten: UnlistenFn | null } = { unlisten: null };
    let stopped = false;

    const stopListening = () => {
      if (stopped) return;
      stopped = true;
      if (tailTimerRef.current !== null) {
        clearTimeout(tailTimerRef.current);
        tailTimerRef.current = null;
      }
      if (handle.unlisten) {
        handle.unlisten();
        handle.unlisten = null;
      }
    };

    try {
      handle.unlisten = await listen<ScanProgress>("scan://progress", (e) => {
        setProgress(e.payload);
        // 收尾事件已到，不必再等 200ms 兜底。
        if ((e.payload.percent ?? 0) >= 100) stopListening();
      });
      // 注册期间若已判定收尾（极端时序），这里立刻补退订。
      if (stopped && handle.unlisten) {
        handle.unlisten();
        handle.unlisten = null;
      }

      // debt-cleanup-003 Bug-1：scan_start 签名 scope: String 必填、target_drive: Option<String>。
      // Tauri 2 自动把 camelCase JS 参转 snake_case Rust 参：targetDrive → target_drive。
      await api("scan_start", { scope: "full", targetDrive: null });

      // task-053 G7.3 兜底：后端已 force 发出 100，但事件可能还在投递队列里。
      // 先把 UI 置满，杜绝「按钮已变『再次扫描』但进度条停在 88%」。
      // progress 为 null（一条事件都没收到）时保持 null，不伪造 sessionId。
      setProgress((p) => (p ? { ...p, percent: 100 } : p));
      setScanStatus("done");
      await load();
    } catch (e) {
      setScanStatus("error");
      setScanMsg(String(e));
      // Bug-5：失败时清掉残留进度条，否则「重试扫描」按钮旁边还挂着上一次的进度。
      setProgress(null);
    } finally {
      // 延迟退订：给最后一条 percent=100 留出投递时间（已收尾则不再排）。
      if (!stopped) {
        tailTimerRef.current = setTimeout(stopListening, TAIL_UNLISTEN_DELAY_MS);
      }
    }
  }, [load]);

  // task-062：托盘「立即扫描」→ App 递增 scanSignal → 这里触发一次扫描。
  // 用 ref 记住上次的值，避免「首次挂载就自动扫」（用户切到仪表盘不该自己跑起来）。
  const lastScanSignalRef = useRef(scanSignal);
  useEffect(() => {
    if (scanSignal > lastScanSignalRef.current) {
      lastScanSignalRef.current = scanSignal;
      void startScan();
    }
  }, [scanSignal, startScan]);

  const scanning = scanStatus === "scanning";

  // task-063：取消扫描。只发一个标志位，不 await 扫描真的停下——
  // 后端在下一个脚本开始前才检查，所以这里立刻把按钮切成「正在停止…」，
  // 让用户知道点击生效了（否则会以为没反应而反复点）。
  const cancelScan = useCallback(async () => {
    setCancelling(true);
    setScanMsg("正在停止…（当前脚本跑完后停止）");
    try {
      await api<boolean>("scan_cancel", {});
    } catch (e) {
      setScanMsg(`取消失败：${String(e)}`);
      setCancelling(false);
    }
  }, []);
  // engine ScanProgress.percent 已是 0~100，直接用；缺失时置 0 不除零。
  const percent = progress ? progress.percent ?? 0 : 0;
  const barPct = Math.max(0, Math.min(100, percent));

  return (
    <main
      style={{
        padding: 24,
        fontFamily: "system-ui, sans-serif",
        background: "var(--bg)",
        minHeight: "100vh",
        boxSizing: "border-box",
      }}
    >
      <h1 style={{ margin: "0 0 16px" }}>C-Disk Cleaner</h1>
      <p style={{ color: "var(--text-muted)", marginTop: 0 }}>清理统计</p>

      {/* task-028：开始扫描按钮 + 进度条 */}
      <div style={{ marginBottom: 16 }}>
        <button style={btnStyle(scanning)} disabled={scanning} onClick={startScan}>
          {scanning
            ? "扫描中..."
            : scanStatus === "done"
            ? "再次扫描"
            : scanStatus === "error"
            ? "重试扫描"
            : "开始扫描"}
        </button>

        {/* task-063：取消扫描。只在扫描中显示，且不等待——点了立刻给反馈，
            真正的停止发生在下一个脚本开始前（后端是脚本粒度检查取消标志）。 */}
        {scanning && (
          <button
            onClick={cancelScan}
            disabled={cancelling}
            style={{
              marginLeft: 8,
              padding: "8px 16px",
              borderRadius: 6,
              border: "1px solid #b45309",
              background: cancelling ? "#fde68a" : "#fff",
              color: "#92400e",
              cursor: cancelling ? "not-allowed" : "pointer",
              fontSize: 13,
            }}
          >
            {cancelling ? "正在停止…" : "取消扫描"}
          </button>
        )}

        {(scanning || scanStatus === "done") && progress && (
          <div style={{ marginTop: 12 }}>
            <div
              style={{
                height: 8,
                background: "var(--border)",
                borderRadius: 4,
                overflow: "hidden",
              }}
            >
              <div
                style={{
                  height: "100%",
                  width: `${barPct}%`,
                  background: "#10b981",
                  transition: "width 100ms linear",
                }}
              />
            </div>
            <div style={{ marginTop: 6, fontSize: 12, color: "var(--text)" }}>
              {progress.currentPath || "准备中..."} — {progress.scannedItems} 项
            </div>
          </div>
        )}

        {scanStatus === "error" && (
          <div
            style={{
              marginTop: 12,
              background: "#fee2e2",
              border: "1px solid #fca5a5",
              color: "#b91c1c",
              padding: 12,
              borderRadius: 8,
            }}
          >
            <div style={{ fontWeight: 600, marginBottom: 4 }}>扫描失败</div>
            <div style={{ fontSize: 13 }}>{scanMsg}</div>
          </div>
        )}
      </div>

      {status === "loading" && (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "1fr 1fr",
            gap: 12,
            opacity: 0.5,
          }}
        >
          {[0, 1, 2, 3].map((i) => (
            <div key={i} style={cardStyle}>
              <div style={{ ...labelStyle, background: "var(--border)", height: 12, width: 80 }} />
              <div style={{ background: "var(--border)", height: 28, width: 120, marginTop: 8 }} />
            </div>
          ))}
        </div>
      )}

      {status === "error" && (
        <div
          style={{
            background: "rgba(220, 38, 38, 0.16)",
            border: "1px solid rgba(220, 38, 38, 0.45)",
            color: "#ef4444",
            padding: 12,
            borderRadius: 8,
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: 4 }}>加载统计失败</div>
          <div style={{ fontSize: 13, marginBottom: 8 }}>{msg}</div>
          <button onClick={load}>重试</button>
        </div>
      )}

      {/* task-063：C 盘空间环形图 → task-077：多盘并列
          task-082（backlog 82.1）：磁盘空间卡**上移到统计卡之前**、通栏横排。
          理由：用户打开应用第一眼看的是「盘还剩多少」，这个信息该在最上面。 */}
      <DriveRings />

      {/* task-082（backlog 82.2/82.3）：统计卡左置、趋势图右置。
          用 flex + `flex: 11 / 9` 表达 55% : 45%：
          - 宽屏：按 11:9 分配，统计卡 2×2 网格在左、趋势图在右，顶部对齐（默认 align-items: stretch + 两张卡同高）；
          - 窄屏（< ~700px）：`flex-wrap` 自动换行，两块各自占满整行，不挤压。

          为什么不用 `grid-template-columns: "11fr 9fr"`：inline style 写不了媒体查询，
          grid 的 fr 不会换行，窄窗口下会被压成两条细缝。flex 的 `1 1 340px` 才是自适应的。 */}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 12, marginTop: 12 }}>
        {status === "ok" && data && (
          <div
            style={{
              // `11 1 0` + `minWidth` 而不是 `11 1 340px`：
              // flex-basis 若是 340px，两个 basis 会吃掉大部分宽度，剩余再按 11:9 分，
              // 实测变成 50.5 : 49.5（比例被稀释）。basis 归零才能拿到真正的 55:45；
              // 换行仍由 `minWidth` 兜底——窄于两个 340px 时自动堆叠。
              flex: "11 1 0",
              minWidth: 340,
              display: "grid",
              gridTemplateColumns: "1fr 1fr",
              gap: 12,
              // 让统计卡在窄屏换行后不会因为 min-height 撑得过高
              alignContent: "start",
            }}
          >
            <div style={cardStyle}>
              <div style={labelStyle}>总释放空间</div>
              <div style={valueStyle("var(--text)")}>{formatBytes(data.totalBytesFreed)}</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>成功次数</div>
              <div style={valueStyle("var(--text)")}>{data.successCount}</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>失败次数</div>
              <div style={valueStyle(data.failedCount > 0 ? "#dc2626" : "var(--text)")}>
                {data.failedCount}
              </div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>清理轮次</div>
              <div style={valueStyle("var(--text)")}>{data.sessionCount}</div>
            </div>
          </div>
        )}

        {/* 趋势图右置（45%）。统计卡还在 loading / error 时它照常渲染——
            趋势数据是另一个接口，不该被统计的失败拖住。 */}
        <div style={{ flex: "9 1 0", minWidth: 340, display: "flex" }}>
          <TrendChart />
        </div>
      </div>

      {/* task-065：WinSxS 组件存储（C 模块）
          task-082：单独一行。backlog 82 只要求「统计卡左 / 趋势图右」，
          没给 WinSxS 位置；它内容最多（分析按钮 + 进度 + DISM 输出），
          挤进任何一栏都会把那一栏撑变形。 */}
      <WinsxsCard />
    </main>
  );
}

// ===================== task-063 环形图 / task-077 多盘并列 =====================
//
// 手写 SVG，不引图表库：只需要「一个圆环 + 中心百分比」，
// 引 recharts / chart.js 会让产物从 198 kB 涨到 300 kB+。
//
// 圆环的画法：两个同心圆，底层是完整灰环，上层用 `strokeDasharray` 只画
// `pct` 那一段，再 `rotate(-90)` 让起点在 12 点方向。
//
// # task-077：为什么改成「一次 drive_list 拿全部盘」
//
// 原来只画 C 盘，数据来自 `drive_usage`（查单个盘）。多盘时有两个选择：
//   ① 先 `drive_list` 拿盘符，再对每个盘调一次 `drive_usage` —— N+1 次 IPC；
//   ② 直接 `drive_list`，它本来就带 total/free —— 1 次 IPC。
// 选 ②。`drive_usage` 保留不动（清理前后对比要用它再查一次单盘）。
//
// # 为什么 0 容量显示「容量未知」而不是画一个 0% 的空环
//
// `GetDiskFreeSpaceExW` 可能失败（卷在修、权限异常），此时后端仍返回这个盘但
// 容量为 0。画成 0% 会被读成「这个盘是空的」，而真相是「查不到」。

interface DriveInfo {
  letter: string;
  totalBytes: number;
  freeBytes: number;
}

function DriveRings() {
  const [drives, setDrives] = useState<DriveInfo[]>([]);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const list = await api<DriveInfo[] | null>("drive_list", {});
        setDrives(Array.isArray(list) ? list : []);
      } catch {
        // 查不到就整块不显示——比显示一个 0% 的假环好
        setFailed(true);
      }
    })();
  }, []);

  if (failed || drives.length === 0) return null;

  return (
    <div style={{ ...cardStyle, marginTop: 12 }}>
      <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 12 }}>磁盘空间</div>
      {/* task-082：多盘之间的间距 20 → 24，与卡片间距 12 的倍数关系对齐，
          盘多了也不会挤在一起。 */}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 24 }}>
        {drives.map((d) => (
          <DriveRingItem key={d.letter} drive={d} />
        ))}
      </div>
    </div>
  );
}

function DriveRingItem({ drive }: { drive: DriveInfo }) {
  const total = drive.totalBytes || 0;
  const free = drive.freeBytes || 0;
  const used = Math.max(0, total - free);
  const unknown = total <= 0;
  // 用整数百分比显示；计算时保留小数避免 49.6% 被显示成 50% 后和环对不上
  const pct = unknown ? 0 : Math.min(100, (used / total) * 100);

  const R = 34;
  const CIRC = 2 * Math.PI * R;
  const dash = (pct / 100) * CIRC;

  // 已用超过 85% 变红：这是「该清理了」的信号，与产品定位一致。
  const color = pct >= 85 ? "#dc2626" : pct >= 70 ? "#d97706" : "#2563eb";

  return (
    <div style={{ display: "flex", alignItems: "center", gap: 14 }}>
      <svg
        width={88}
        height={88}
        viewBox="0 0 88 88"
        role="img"
        aria-label={`${drive.letter} 盘空间使用率`}
      >
        <circle cx={44} cy={44} r={R} fill="none" stroke="var(--border)" strokeWidth={9} />
        {!unknown && (
          <circle
            cx={44}
            cy={44}
            r={R}
            fill="none"
            stroke={color}
            strokeWidth={9}
            strokeDasharray={`${dash} ${CIRC - dash}`}
            strokeLinecap="round"
            transform="rotate(-90 44 44)"
          />
        )}
        <text
          x={44}
          y={42}
          textAnchor="middle"
          fontSize={16}
          fontWeight={600}
          fill="var(--text)"
        >
          {unknown ? "—" : `${Math.round(pct)}%`}
        </text>
        <text x={44} y={58} textAnchor="middle" fontSize={10} fill="var(--text-muted)">
          已用
        </text>
      </svg>
      <div style={{ fontSize: 12, lineHeight: 1.8, color: "var(--text)" }}>
        <div style={{ fontWeight: 600, marginBottom: 2 }}>{drive.letter} 盘</div>
        {unknown ? (
          <div style={{ color: "var(--text-muted)" }}>容量未知（查询失败）</div>
        ) : (
          <>
            <div style={{ color: "var(--text-muted)" }}>
              已用 {formatBytes(used)} / 共 {formatBytes(total)}
            </div>
            <div style={{ color: "var(--text-muted)" }}>可用 {formatBytes(free)}</div>
            {pct >= 85 && (
              <div style={{ color: "#b91c1c", marginTop: 2 }}>空间紧张，建议尽快清理</div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

// ===================== task-061 趋势图 / task-073 折线增强 =====================
//
// 实现方式：**手写 SVG 折线**，不引 recharts / chart.js。
// 理由：需求只是「一条折线 + 三个范围按钮 + 悬停提示」，引一个 100KB+ 的图表库
// 不划算（本项目前端产物体积一直在 230 kB 上下）。
//
// # 为什么从柱状改成折线，以及怎么避免折线的老毛病
//
// task-061 的注释写过「折线会把没数据的那天画成一条斜线，误导」。那个担心是对的，
// 但**成因不是折线本身，而是「只画有记录的日子」**——`cleanup_stats_trend` 的
// SQL 是 `GROUP BY day`，没记录的日子根本不返回，于是 10-01 和 10-20 会被画成
// 相邻两点，中间 19 天凭空消失。
//
// 所以本任务改成折线时同步做了 [`fillDays`]：**按真实日期铺满整个范围**，没记录
// 的日子补 0。这样「这天没清理」就是基线上一个 0 值点，折线如实贴底，不会骗人。
// 配合「>0 的日子画实心圆点、=0 不画」，用户一眼能看出哪天真的清过。

interface TrendPoint {
  day: string;
  bytesFreed: number;
}

/** 可选时间范围。与后端 `cleanup_stats_trend(days)` 的 `days` 直接对应。 */
const TREND_RANGES = [
  { days: 7, label: "近 7 天" },
  { days: 30, label: "近 30 天" },
  { days: 90, label: "近 90 天" },
] as const;

const DEFAULT_TREND_DAYS = 30;

/**
 * 本地日期 → `YYYY-MM-DD`。
 *
 * **不能用 `toISOString().slice(0,10)`**：那是 UTC 日期，而 SQLite 侧用的是
 * `date(started_at,'unixepoch','localtime')`。东八区凌晨 0-8 点用 UTC 会差一天，
 * 表现为「昨天的数据跑到今天」。
 */
function localDayKey(d: Date): string {
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

/**
 * 把「只有有记录的日期」补成「连续 `days` 天」。
 *
 * 后端只返回有记录的日期（`GROUP BY day`），直接拿来画折线会把 10-01 和 10-20
 * 画成相邻两点。这里按日历铺满，缺失的补 0。
 */
function fillDays(points: TrendPoint[], days: number, today: Date = new Date()): TrendPoint[] {
  const byDay = new Map(points.map((p) => [p.day, p.bytesFreed]));
  const out: TrendPoint[] = [];
  for (let i = days - 1; i >= 0; i -= 1) {
    const d = new Date(today);
    d.setDate(d.getDate() - i);
    const key = localDayKey(d);
    out.push({ day: key, bytesFreed: byDay.get(key) ?? 0 });
  }
  return out;
}

/**
 * 平滑折线路径（二次贝塞尔穿中点法）。
 *
 * 为什么不用 Catmull-Rom 转三次贝塞尔：那种插值在数据突变时会**过冲**，
 * 画出负值或超过峰值的曲线——图上出现「比最大值还高」的点是硬伤。
 * 穿中点法只用相邻两点的中点做控制点，曲线始终被数据点包住，不会过冲。
 */
function smoothPath(pts: { x: number; y: number }[]): string {
  if (pts.length === 0) return "";
  if (pts.length === 1) return `M ${pts[0].x} ${pts[0].y}`;
  let d = `M ${pts[0].x} ${pts[0].y}`;
  for (let i = 1; i < pts.length - 1; i += 1) {
    const cx = (pts[i].x + pts[i + 1].x) / 2;
    const cy = (pts[i].y + pts[i + 1].y) / 2;
    d += ` Q ${pts[i].x} ${pts[i].y} ${cx} ${cy}`;
  }
  const last = pts[pts.length - 1];
  return `${d} L ${last.x} ${last.y}`;
}

function TrendChart() {
  const [days, setDays] = useState<number>(DEFAULT_TREND_DAYS);
  const [raw, setRaw] = useState<TrendPoint[] | null>(null);
  const [failed, setFailed] = useState(false);
  const [hover, setHover] = useState<number | null>(null);

  useEffect(() => {
    let alive = true;
    (async () => {
      try {
        const r = await api<TrendPoint[]>("cleanup_stats_trend", { days });
        if (!alive) return;
        setRaw(Array.isArray(r) ? r : []);
      } catch {
        // 读失败不打扰用户：图表区显示占位，卡片数据照常
        if (!alive) return;
        setFailed(true);
        setRaw([]);
      }
    })();
    // 切范围时上一次请求可能还没回来，用 alive 丢弃过期响应，
    // 否则快速点 7→30→90 会被慢的那个覆盖成错误范围。
    return () => {
      alive = false;
    };
  }, [days]);

  // 切范围时清掉悬停：旧索引在新数据里指向的是另一天。
  useEffect(() => {
    setHover(null);
  }, [days]);

  // task-082：不再自带 marginTop——它现在和统计卡同处一个 flex 行，
  // 间距由外层 gap 统一给。`flex: 1` 让它在右栏撑满宽度（父级是 flex 容器），
  // 高度由 `align-items: stretch`（默认）拉到与左栏统计卡一致，实现「顶部对齐」。
  const box: React.CSSProperties = { ...cardStyle, flex: 1 };
  const title = (
    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
      <span style={labelStyle}>最近 {days} 天释放趋势</span>
      <span style={{ display: "flex", gap: 4 }} role="group" aria-label="时间范围">
        {TREND_RANGES.map((r) => {
          const on = r.days === days;
          return (
            <button
              key={r.days}
              onClick={() => setDays(r.days)}
              aria-pressed={on}
              style={{
                padding: "2px 8px",
                fontSize: 11,
                borderRadius: 4,
                border: `1px solid ${on ? "#2563eb" : "var(--border)"}`,
                background: on ? "#2563eb" : "var(--bg)",
                color: on ? "#fff" : "var(--text-muted)",
                cursor: "pointer",
              }}
            >
              {r.label}
            </button>
          );
        })}
      </span>
    </div>
  );

  if (raw === null) {
    return (
      <div style={box}>
        {title}
        <div style={{ fontSize: 12, color: "var(--text-muted)", marginTop: 8 }}>加载中…</div>
      </div>
    );
  }

  const points = fillDays(raw, days);
  const total = points.reduce((a, p) => a + p.bytesFreed, 0);

  if (total === 0 && failed) {
    return (
      <div style={box}>
        {title}
        <div style={{ fontSize: 12, color: "var(--text-muted)", marginTop: 8 }}>
          趋势数据加载失败，请稍后重试
        </div>
      </div>
    );
  }

  // SVG 坐标系：固定 viewBox + preserveAspectRatio="none" 让宽度随容器拉伸。
  const W = 600;
  const H = 120;
  const PAD_TOP = 8;
  const PAD_BOTTOM = 4;
  const plotH = H - PAD_TOP - PAD_BOTTOM;

  const peak = Math.max(...points.map((p) => p.bytesFreed), 0);
  // Y 轴自适应：留 15% 顶部余量，否则峰值点会贴着上边界看不出高低。
  // peak=0（全范围没清理）时给一个虚拟上限，避免除零。
  const yMax = peak > 0 ? peak * 1.15 : 1;

  const slot = W / points.length;
  const xy = points.map((p, i) => ({
    x: i * slot + slot / 2,
    y: PAD_TOP + plotH - (p.bytesFreed / yMax) * plotH,
  }));

  const hovered = hover !== null ? points[hover] : null;
  const hoveredXY = hover !== null ? xy[hover] : null;

  return (
    <div style={box}>
      {title}
      <div style={{ display: "flex", justifyContent: "space-between", margin: "6px 0 2px" }}>
        <span style={{ fontSize: 11, color: "var(--text-muted)" }}>{points[0].day}</span>
        <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
          {total > 0 ? `合计 ${formatBytes(total)}` : "暂无清理记录"}
        </span>
        <span style={{ fontSize: 11, color: "var(--text-muted)" }}>
          {points[points.length - 1].day}
        </span>
      </div>

      <div style={{ position: "relative" }}>
        <svg
          viewBox={`0 0 ${W} ${H}`}
          preserveAspectRatio="none"
          role="img"
          aria-label={`最近 ${days} 天释放趋势折线图`}
          data-testid="trend-line"
          style={{ width: "100%", height: 120, display: "block" }}
          onMouseLeave={() => setHover(null)}
        >
          {/* 基线 */}
          <line
            x1={0}
            y1={H - PAD_BOTTOM}
            x2={W}
            y2={H - PAD_BOTTOM}
            stroke="var(--border)"
            strokeWidth={1}
          />
          {/* 折线主体。全 0 时不画线（否则是一条贴着基线的实线，看起来像"有数据"） */}
          {peak > 0 && (
            <path
              d={smoothPath(xy)}
              fill="none"
              stroke="#10b981"
              strokeWidth={1.6}
              strokeLinecap="round"
              vectorEffect="non-scaling-stroke"
            />
          )}
          {/* 有清理的日子画实心点：让「哪天真的清过」一眼可见 */}
          {points.map((p, i) =>
            p.bytesFreed > 0 ? (
              <circle
                key={`dot-${p.day}`}
                cx={xy[i].x}
                cy={xy[i].y}
                r={1.8}
                fill="#10b981"
                vectorEffect="non-scaling-stroke"
              />
            ) : null
          )}
          {/* 悬停高亮 */}
          {hoveredXY && (
            <>
              <line
                x1={hoveredXY.x}
                y1={PAD_TOP}
                x2={hoveredXY.x}
                y2={H - PAD_BOTTOM}
                stroke="var(--border)"
                strokeWidth={1}
                strokeDasharray="2 2"
                vectorEffect="non-scaling-stroke"
              />
              <circle
                cx={hoveredXY.x}
                cy={hoveredXY.y}
                r={3}
                fill="#2563eb"
                vectorEffect="non-scaling-stroke"
              />
            </>
          )}
          {/* 命中区：每个槽位一条透明竖条，比 1.8px 的圆点好点得多 */}
          {points.map((p, i) => (
            <rect
              key={`hit-${p.day}`}
              x={i * slot}
              y={0}
              width={slot}
              height={H}
              fill="transparent"
              onMouseEnter={() => setHover(i)}
              data-testid={`trend-hit-${p.day}`}
            >
              <title>{`${p.day}：${formatBytes(p.bytesFreed)}`}</title>
            </rect>
          ))}
        </svg>

        {hovered && (
          <div
            data-testid="trend-tooltip"
            style={{
              position: "absolute",
              top: 0,
              left: `${((hover! + 0.5) / points.length) * 100}%`,
              transform: "translateX(-50%)",
              background: "var(--bg)",
              border: "1px solid var(--border)",
              borderRadius: 4,
              padding: "3px 7px",
              fontSize: 11,
              color: "var(--text)",
              whiteSpace: "nowrap",
              pointerEvents: "none",
              boxShadow: "0 1px 4px rgba(0,0,0,.12)",
            }}
          >
            {hovered.day} · {formatBytes(hovered.bytesFreed)}
          </div>
        )}
      </div>
    </div>
  );
}

// ===================== task-065：WinSxS 组件存储（C 模块） =====================
//
// 组件存储（WinSxS）清理必须走 DISM + 管理员权限，所以这块 UI 只做三件事：
//   1. 展示分析结果（可清理多少 / DISM 是否建议清）；
//   2. 用户点「立即清理」→ 二次确认 → 调 winsxs_cleanup；
//   3. 把 DISM 的输出行实时铺出来（事件 winsxs://progress）。
//
// **绝不在挂载时自动分析**：DISM 会弹 UAC，页面一打开就弹窗是流氓行为，
// 而且用户只是路过仪表盘也会被打断。

interface WinsxsAnalysis {
  lastCleanup: string | null;
  reclaimablePackages: number;
  actualSizeBytes: number;
  backupsAndDisabledBytes: number;
  cacheAndTempBytes: number;
  reclaimableBytes: number;
  recommendedCleanup: boolean;
  parsed: boolean;
  rawOutput: string;
}

interface WinsxsCleanupResult {
  ok: boolean;
  exitCode: number;
  restorePointSeq: number | null;
  outputTail: string;
  lineCount: number;
}

interface WinsxsProgress {
  seq: number;
  line: string;
}

/**
 * 与 `engine::scan::winsxs::WINSXS_PROGRESS_EVENT` 对齐。
 * engine 侧有单测钉住这个字符串，改名会在 `cargo test` 里红。
 */
const WINSXS_PROGRESS_EVENT = "winsxs://progress";

/** 进度面板最多缓存多少行：DISM 能输出上千行，全渲染会把 WebView 拖卡。 */
const WINSXS_MAX_LINES = 200;

/** 进度面板显示最近几行（看的是「现在在干什么」，不是完整日志）。 */
const WINSXS_TAIL = 8;

/** 还原点创建失败的 IPC 错误码（engine::ipc::error_codes::RESTOREPOINT_FAILED）。 */
const CODE_RESTOREPOINT_FAILED = 2007;

const winsxsBtnStyle = (disabled: boolean): React.CSSProperties => ({
  padding: "7px 14px",
  fontSize: 13,
  borderRadius: 6,
  border: `1px solid ${disabled ? "var(--border)" : "#2563eb"}`,
  background: disabled ? "var(--bg)" : "#2563eb",
  color: disabled ? "var(--text-muted)" : "#fff",
  cursor: disabled ? "not-allowed" : "pointer",
});

function WinsxsCard() {
  const [analysis, setAnalysis] = useState<WinsxsAnalysis | null>(null);
  const [analyzing, setAnalyzing] = useState(false);
  const [analyzeError, setAnalyzeError] = useState("");

  const [cleaning, setCleaning] = useState(false);
  const [cleanLines, setCleanLines] = useState<string[]>([]);
  const [cleanResult, setCleanResult] = useState<WinsxsCleanupResult | null>(null);
  const [cleanError, setCleanError] = useState("");

  const analyze = useCallback(async () => {
    setAnalyzing(true);
    setAnalyzeError("");
    try {
      setAnalysis(await api<WinsxsAnalysis>("winsxs_analyze", {}));
    } catch (e) {
      // 失败时清掉旧结果：留着上一次的数字会让人以为「分析成功了，还是这些」
      setAnalysis(null);
      setAnalyzeError(String(e));
    } finally {
      setAnalyzing(false);
    }
  }, []);

  const cleanup = useCallback(async () => {
    const size = analysis ? formatBytes(analysis.reclaimableBytes) : "若干空间";
    if (
      !window.confirm(
        `将清理组件存储中约 ${size}。过程中会先创建系统还原点，并弹出管理员授权。是否继续？`
      )
    ) {
      return;
    }

    setCleaning(true);
    setCleanError("");
    setCleanResult(null);
    setCleanLines([]);

    let unlisten: UnlistenFn | null = null;
    try {
      // 先订阅再调用：DISM 可能在命令 resolve 之前就把头几行吐出来了。
      unlisten = await listen<WinsxsProgress>(WINSXS_PROGRESS_EVENT, (e) => {
        setCleanLines((prev) => {
          const next = [...prev, e.payload.line];
          return next.length > WINSXS_MAX_LINES
            ? next.slice(next.length - WINSXS_MAX_LINES)
            : next;
        });
      });

      let skipRestorePoint = false;
      let askedContinue = false;
      // 2007 = 还原点建失败。问一次「是否继续」；用户同意就带 skipRestorePoint
      // 重调。**只重试一次**，避免「一直失败一直问」的死循环。
      for (;;) {
        const env = await apiRaw<WinsxsCleanupResult>("winsxs_cleanup", {
          skipRestorePoint,
        });
        if (env.code === CODE_RESTOREPOINT_FAILED && !askedContinue) {
          askedContinue = true;
          const go = window.confirm(
            "系统还原点创建失败（可能系统还原已关闭）。继续清理将无法回滚，是否继续？"
          );
          if (!go) {
            setCleanError("已取消：系统还原点创建失败，未执行清理");
            break;
          }
          skipRestorePoint = true;
          continue;
        }
        if (env.code !== 0) {
          setCleanError(env.message);
          break;
        }
        setCleanResult(env.data);
        break;
      }
    } catch (e) {
      // 提权链路失败（UAC 被取消等）：命令走 Err，原因在字符串里。
      setCleanError(String(e));
    } finally {
      unlisten?.();
      setCleaning(false);
    }
  }, [analysis]);

  const reclaimable = analysis ? formatBytes(analysis.reclaimableBytes) : "";
  const hasReclaimable =
    !!analysis && (analysis.reclaimableBytes > 0 || analysis.reclaimablePackages > 0);
  const canClean = hasReclaimable && !cleaning && !analyzing;

  return (
    <div style={{ ...cardStyle, marginTop: 12 }} data-testid="winsxs-card">
      <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between" }}>
        <div style={{ fontSize: 14, fontWeight: 600, color: "var(--text)" }}>
          WinSxS 组件存储
        </div>
        <div style={{ fontSize: 11, color: "var(--text-muted)" }}>需管理员权限（会弹 UAC）</div>
      </div>

      {!analysis && !analyzing && (
        <div style={{ marginTop: 10 }}>
          <div style={{ fontSize: 12, color: "var(--text-muted)", marginBottom: 8 }}>
            分析组件存储里可回收的旧版本组件。DISM 分析通常十几秒到几分钟，期间请勿关闭窗口。
          </div>
          <button onClick={analyze} style={winsxsBtnStyle(false)}>
            分析组件存储
          </button>
        </div>
      )}

      {analyzing && (
        <div style={{ marginTop: 10, fontSize: 12, color: "var(--text-muted)" }}>
          正在分析…（DISM 运行中，若弹出授权请点「是」）
        </div>
      )}

      {analyzeError && (
        <div
          style={{
            marginTop: 10,
            background: "#fee2e2",
            border: "1px solid #fca5a5",
            color: "#b91c1c",
            padding: 10,
            borderRadius: 6,
            fontSize: 12,
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: 4 }}>分析失败</div>
          <div style={{ wordBreak: "break-all" }}>{analyzeError}</div>
        </div>
      )}

      {analysis && (
        <div style={{ marginTop: 10 }}>
          <div
            style={{
              display: "flex",
              gap: 24,
              flexWrap: "wrap",
              fontSize: 12,
              lineHeight: 1.9,
              color: "var(--text)",
            }}
          >
            <div>
              <div style={{ color: "var(--text-muted)" }}>可清理</div>
              <div style={{ fontSize: 20, fontWeight: 600 }}>{reclaimable}</div>
            </div>
            <div>
              <div style={{ color: "var(--text-muted)" }}>可回收包</div>
              <div style={{ fontSize: 20, fontWeight: 600 }}>{analysis.reclaimablePackages}</div>
            </div>
            <div>
              <div style={{ color: "var(--text-muted)" }}>组件存储实际大小</div>
              <div>{formatBytes(analysis.actualSizeBytes)}</div>
            </div>
            <div>
              <div style={{ color: "var(--text-muted)" }}>上次清理</div>
              <div>{analysis.lastCleanup ?? "从未清理"}</div>
            </div>
            <div>
              <div style={{ color: "var(--text-muted)" }}>DISM 建议</div>
              <div style={{ color: analysis.recommendedCleanup ? "#b45309" : "var(--text)" }}>
                {analysis.recommendedCleanup ? "建议清理" : "暂不建议清理"}
              </div>
            </div>
          </div>

          <div style={{ marginTop: 10, display: "flex", gap: 8 }}>
            <button onClick={cleanup} disabled={!canClean} style={winsxsBtnStyle(!canClean)}>
              {cleaning ? "清理中…" : "立即清理"}
            </button>
            <button
              onClick={analyze}
              disabled={analyzing || cleaning}
              style={{
                padding: "7px 14px",
                fontSize: 13,
                borderRadius: 6,
                border: "1px solid var(--border)",
                background: "var(--bg)",
                color: "var(--text)",
                cursor: analyzing || cleaning ? "not-allowed" : "pointer",
              }}
            >
              重新分析
            </button>
          </div>

          {!cleaning && !hasReclaimable && (
            <div style={{ marginTop: 8, fontSize: 12, color: "var(--text-muted)" }}>
              当前没有可回收的组件，无需清理。
            </div>
          )}
        </div>
      )}

      {cleaning && (
        <div
          style={{
            marginTop: 10,
            background: "var(--bg)",
            border: "1px solid var(--border)",
            borderRadius: 6,
            padding: 8,
            fontFamily: "ui-monospace, SFMono-Regular, Consolas, monospace",
            fontSize: 11,
            lineHeight: 1.6,
            color: "var(--text)",
            maxHeight: 140,
            overflow: "auto",
          }}
        >
          {cleanLines.length === 0 ? (
            <div style={{ color: "var(--text-muted)" }}>
              正在创建系统还原点…（若弹出授权请点「是」）
            </div>
          ) : (
            cleanLines.slice(-WINSXS_TAIL).map((line, i) => <div key={`${i}-${line}`}>{line}</div>)
          )}
        </div>
      )}

      {cleanResult && (
        <div
          style={{
            marginTop: 10,
            background: cleanResult.ok ? "#ecfdf5" : "#fee2e2",
            border: `1px solid ${cleanResult.ok ? "#6ee7b7" : "#fca5a5"}`,
            color: cleanResult.ok ? "#065f46" : "#b91c1c",
            padding: 10,
            borderRadius: 6,
            fontSize: 12,
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: 4 }}>
            {cleanResult.ok
              ? "组件存储清理完成"
              : `清理失败（DISM 退出码 ${cleanResult.exitCode}）`}
          </div>
          <div style={{ color: "var(--text-muted)" }}>
            {cleanResult.restorePointSeq !== null
              ? `清理前已创建系统还原点 #${cleanResult.restorePointSeq}`
              : "本次未创建系统还原点"}
            {" · "}
            共 {cleanResult.lineCount} 行 DISM 输出
          </div>
          {cleanResult.outputTail && (
            <div
              style={{
                marginTop: 6,
                fontFamily: "ui-monospace, SFMono-Regular, Consolas, monospace",
                fontSize: 11,
                wordBreak: "break-all",
              }}
            >
              {cleanResult.outputTail}
            </div>
          )}
          {cleanResult.ok && (
            <div style={{ marginTop: 6, color: "var(--text-muted)" }}>
              点「重新分析」可查看最新可清理空间。
            </div>
          )}
        </div>
      )}

      {cleanError && (
        <div
          style={{
            marginTop: 10,
            background: "#fee2e2",
            border: "1px solid #fca5a5",
            color: "#b91c1c",
            padding: 10,
            borderRadius: 6,
            fontSize: 12,
          }}
        >
          <div style={{ fontWeight: 600, marginBottom: 4 }}>清理未完成</div>
          <div style={{ wordBreak: "break-all" }}>{cleanError}</div>
        </div>
      )}
    </div>
  );
}
