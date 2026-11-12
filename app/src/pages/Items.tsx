// task-029：清理闭环 Items 页面。
// - 启动 invoke('cleanup_list_pending') 拉待清理表格（size 倒序）；
// - checkbox 多选 + 底部「已选 N 项，共 X MB」+「清理已选」按钮；
// - 清理中按钮禁用；完成后自动重拉列表 + 成功/失败提示。
//
// task-040：加「预览可清理项」按钮 + dryRun 只读预览面板。
// - 走 apiRaw（不是 api）：cleanup.dryRun 的 2002 用 err_with_data，
//   失败信封的 data 非 null 且带 blockedByL3 明细，api() 会把它抛掉；
// - 只预览不执行：面板内不新增第二个执行入口，确认后仍走既有「清理已选」。
//
// 与任务书偏差：cleanup_item 表无 status/scanned_at 列；
// PendingItem 仅 id/path/sizeBytes/moduleCode/sessionId，无时间列展示。
//
// task-049：清理失败从「只报告」升级为「可处置」。
// - cleanup_execute 的返回值不再丢弃，失败项展示 路径 + 原因 + 占用进程；
// - 有占用进程时出现「关闭占用进程并重试」，window.confirm 二次确认后调
//   cleanup_retry_locked（后端 RmShutdown + 护栏过滤 + 复用 execute_items 重删）；
// - 字段全部 camelCase（后端 #[serde(rename_all = "camelCase")]），历史已多次踩
//   snake_case 坑，这里显式对齐。
//
// task-052 G2：列表分页。原实现硬编码 `limit: 100`，第 101 条之后的条目（例如
// %TEMP% 里被锁住的小文件）在 UI 上根本找不到 —— 走查 G2 缺陷。现在改为
// `{ page, pageSize }` 分页入参 + 底部翻页栏，翻页时清空勾选（避免跨页误删）。

import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { formatBytes } from "../utils/formatBytes";
import { api, apiRaw } from "../utils/invoke";
import {
  describeDenial,
  formatBreakdownLine,
  formatDryRunHeadline,
  sortBreakdown,
  type BlockedItem,
  type DryRunResp,
} from "../utils/dryRun";

interface PendingItem {
  id: number;
  path: string;
  sizeBytes: number;
  moduleCode: string;
  sessionId: number;
  /** task-064：L0/L1/L2/L3，用于风险徽标与「全选 L1」。 */
  riskLevel: string;
  /** task-083：规则类别键；软件缓存条目形如 `software_cache_<slug>`。 */
  category: string;
  /** task-083：来自规则库的「删了会怎样 / 怎么重建」，可能为空。 */
  impactNote: string | null;
  rebuildNote: string | null;
}

/** task-083：软件缓存模块的代码，与 `engine::scan::modules::software_cache::MODULE_CODE` 一致。 */
const SOFTWARE_CACHE_MODULE = "software_cache";

/**
 * 从 category 推导 UI 上的软件名。
 *
 * **为什么是推导而不是后端直接给 `softwareName`**：`cleanup_item.category` 早就
 * 存在且已经透传到前端，加一个字段要动 DB 迁移 / runner / IPC 三层；而软件
 * 归属本来就是「这条规则属于哪个软件」，用 category 表达最自然。
 *
 * slug 是 `visual_studio_code` 这种 ASCII 形式（`software_cache.rs::category_slug`
 * 为了清理方案的稳定性只保留字母数字），这里反推成可读的标题形式。
 */
export function softwareLabel(item: Pick<PendingItem, "moduleCode" | "category">): string {
  if (item.moduleCode !== SOFTWARE_CACHE_MODULE) return "—";
  const slug = (item.category || "").replace(/^software_cache_/, "");
  if (!slug || slug === "unknown") return "其他软件";
  return slug
    .split("_")
    .filter(Boolean)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

/** task-083：hover 说明优先用规则库的逐项文案，没有才退回通用等级文案。 */
export function impactHint(item: Pick<PendingItem, "impactNote" | "rebuildNote">, fallback: string): string {
  const parts = [item.impactNote, item.rebuildNote].filter((x): x is string => !!x && x.trim() !== "");
  return parts.length > 0 ? parts.join("；") : fallback;
}

/** task-064：风险等级的展示元数据（颜色走 CSS 变量，深色主题自动适配）。 */
const RISK_META: Record<string, { label: string; fg: string; bg: string; hint: string }> = {
  L0: {
    label: "L0",
    fg: "var(--text-muted)",
    bg: "var(--bg)",
    hint: "L0 仅提示，不参与清理（无风险）",
  },
  L1: {
    label: "L1",
    fg: "#16a34a",
    bg: "rgba(22, 163, 74, 0.16)",
    // task-082：hover 除了说「风险多高」，还要说「影响什么」——
    // 用户真正想知道的是「删了会发生什么」，而不是一个等级符号。
    hint: "L1 安全缓存：删除不影响系统，相关程序下次使用时会自动重建（如浏览器缓存、包管理器缓存、预读取文件）",
  },
  L2: {
    label: "L2",
    fg: "#d97706",
    bg: "rgba(217, 119, 6, 0.18)",
    hint: "L2 需确认：可能是正在运行的程序正在使用的文件（如临时安装包、更新下载缓存），删除可能让该程序报错；清理前会创建系统还原点",
  },
  L3: {
    label: "L3",
    fg: "#ef4444",
    bg: "rgba(220, 38, 38, 0.16)",
    hint: "L3 受保护，不允许删除，也禁止自动清理：系统关键路径或用户个人文件（如 System32、桌面、文档），误删可能导致系统或程序无法启动。这里只展示，不给勾选框",
  },
};

/** 未知等级（后端将来新增）也要能显示，不能崩。 */
function riskMeta(level: string) {
  return (
    RISK_META[level] ?? {
      label: level || "?",
      fg: "var(--text-muted)",
      bg: "var(--bg)",
      hint: `未知风险等级 ${level}`,
    }
  );
}

/** task-052 G2：engine::execute::list_pending::PendingPage 的 JSON shape。 */
interface PendingPage {
  items: PendingItem[];
  total: number;
  page: number;
  pageSize: number;
}

/** 每页条数（与后端 DEFAULT_PAGE_SIZE 对齐）。 */
const PAGE_SIZE = 100;

/** task-049：engine::execute::locking::LockingProcess 的 JSON shape。 */
interface LockingProcess {
  pid: number;
  appName: string;
  serviceName: string | null;
}

/** task-049：engine::execute::locking::RefusedProcess（护栏拒绝，带原因）。 */
interface RefusedProcess {
  pid: number;
  appName: string;
  /**
   * "windows service" / "system process" / "self" / "protected process" /
   * "unknown process"（task-049b：取不到进程映像名时的 fail-closed 原因）
   */
  reason: string;
}

/** task-049：engine::execute::executor::ExecResult。 */
interface ExecResult {
  itemId: number;
  path: string;
  deleted: boolean;
  /** task-052 G5：删除前就已不存在（既不算成功释放、也不算失败）。 */
  alreadyGone: boolean;
  bytesFreed: number;
  error: string | null;
  lockingProcesses: LockingProcess[];
}

/** task-049：engine::execute::executor::ExecuteSummary。 */
interface ExecuteSummary {
  results: ExecResult[];
  deletedCount: number;
  failedCount: number;
  /** task-052 G5：本次「已不存在」的条目数（不计入 bytesFreedTotal）。 */
  alreadyGoneCount: number;
  bytesFreedTotal: number;
  /** task-063：清理前目标盘已用字节（查不到为 null，UI 显示「—」）。 */
  diskBeforeBytes?: number | null;
  /** task-063：清理后目标盘已用字节。 */
  diskAfterBytes?: number | null;
}

/** task-049：cleanup_retry_locked 响应体。 */
interface RetryLockedResp {
  shutdown: { closed: LockingProcess[]; refused: RefusedProcess[] };
  summary: ExecuteSummary;
}

/** 清理/重试结果条状态（tone 决定绿/红，failures 决定明细与重试按钮）。 */
interface ResultState {
  tone: "ok" | "fail";
  headline: string;
  failures: ExecResult[];
  /** 被护栏拒绝的进程（仅重试路径会有）。 */
  refused: RefusedProcess[];
  /** task-063：最后一次清理的完整结果，供「导出报告」使用。 */
  summary?: ExecuteSummary;
}

/** `chrome.exe (PID 1234)` —— 与后端 audit_log 的拼法保持一致。 */
const formatProc = (p: { appName: string; pid: number }) => `${p.appName} (PID ${p.pid})`;

/** task-063：导出按钮样式（结果条里的小按钮）。 */
const exportBtnStyle: React.CSSProperties = {
  padding: "4px 10px",
  borderRadius: 5,
  border: "1px solid var(--border)",
  background: "var(--surface)",
  color: "var(--text)",
  fontSize: 12,
  cursor: "pointer",
};

/** 把后端稳定错误前缀翻成人话，其余原样展示（不吞信息）。 */
function describeExecError(err: string | null): string {
  if (!err) return "未知原因";
  const inUse = "FILE_IN_USE:";
  const denied = "PERMISSION_DENIED:";
  if (err.startsWith(inUse)) {
    const detail = err.slice(inUse.length).trim();
    return detail ? `文件被占用（${detail}）` : "文件被占用";
  }
  if (err.startsWith(denied)) {
    const detail = err.slice(denied.length).trim();
    return detail ? `权限不足（${detail}）` : "权限不足，可能需要管理员权限";
  }
  return err;
}

/** 去重（同一进程可能占用多个文件），保留首次出现顺序。 */
function dedupeProcs(procs: LockingProcess[]): LockingProcess[] {
  const seen = new Set<number>();
  const out: LockingProcess[] = [];
  for (const p of procs) {
    if (seen.has(p.pid)) continue;
    seen.add(p.pid);
    out.push(p);
  }
  return out;
}

type Status = "loading" | "ok" | "error";

/** task-040：预览面板状态（判别联合，便于按 kind 分支渲染与整体清空）。 */
type Preview =
  | { kind: "ok"; resp: DryRunResp }
  | { kind: "empty" }
  | { kind: "blocked"; blocked: BlockedItem[] }
  | { kind: "error"; message: string };

/** 预览面板统一底色/文字/边框（与既有 result 提示条同风格）。 */
const panelStyle = (bg: string, fg: string, border: string): React.CSSProperties => ({
  marginBottom: 12,
  padding: 10,
  borderRadius: 6,
  fontSize: 13,
  background: bg,
  color: fg,
  border: `1px solid ${border}`,
});

const thStyle: React.CSSProperties = {
  textAlign: "left",
  padding: "8px 10px",
  borderBottom: "1px solid var(--border)",
  fontSize: 12,
  color: "var(--text-muted)",
  fontWeight: 600,
};

const tdStyle: React.CSSProperties = {
  padding: "8px 10px",
  borderBottom: "1px solid var(--border)",
  fontSize: 13,
};

/** task-072：「方案」页应用方案后交给本页的载荷。 */
export interface PlanApplyPayload {
  /** 要恢复勾选的清理项 id（后端按方案三元组回查当前库得到）。 */
  itemIds: number[];
  /** 给用户看的一句话（含「有 N 条本次没找到」这类信息）。 */
  notice: string;
}

interface ItemsProps {
  /**
   * task-072：待应用的方案（一次性）。App 在「方案」页点「应用」后传入。
   *
   * 为什么是 prop 而不是全局 store：本项目没有状态管理库，跨页通信用的是
   * App 里的一份 state + prop 下发（`Dashboard` 的 `scanSignal` 就是同一套）。
   */
  planApply?: PlanApplyPayload | null;
  /** 应用完通知外层清空载荷（避免重进本页时重复套用）。 */
  onPlanConsumed?: () => void;
  /** 把当前勾选报告给外层，「方案」页要用它显示「能保存多少项」。 */
  onSelectionChange?: (ids: number[]) => void;
}

export default function Items({
  planApply,
  onPlanConsumed,
  onSelectionChange,
}: ItemsProps = {}) {
  const [status, setStatus] = useState<Status>("loading");
  const [rows, setRows] = useState<PendingItem[]>([]);
  const [msg, setMsg] = useState<string>("");
  /** task-072：应用方案后的提示（载荷被清空后仍要让用户看到）。 */
  const [planNotice, setPlanNotice] = useState<string>("");
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [cleaning, setCleaning] = useState(false);
  // task-052 G2：分页态。
  const [page, setPage] = useState(1);
  const [pageSize] = useState(PAGE_SIZE);
  const [total, setTotal] = useState(0);
  // task-049：重试态（关闭占用进程 + 重删）。与 cleaning/previewing 互斥。
  const [retrying, setRetrying] = useState(false);
  // task-060：关闭进程期间「已等待秒数」。null = 没在关（不显示进度条）。
  // 来源是后端 `shutdown://progress` 事件（engine 每秒回调一次）。
  const [shutdownSecs, setShutdownSecs] = useState<number | null>(null);
  // task-063：报告导出状态与提示。
  const [exporting, setExporting] = useState(false);
  const [exportMsg, setExportMsg] = useState("");
  // task-086 1.1：用户自定义导出目录（PRD P0-6「用户自选保存位置」）。
  // 空串 = 走后端默认 `data_root/reports`，与旧行为完全一致。
  const [exportDir, setExportDir] = useState("");
  const [result, setResult] = useState<ResultState | null>(null);
  // task-040 预览态。
  const [previewing, setPreviewing] = useState(false);
  const [preview, setPreview] = useState<Preview | null>(null);

  // task-081：表格行 hover 高亮。
  //
  // 为什么用 state 而不是 CSS `:hover`：本项目全部是 inline style，没有样式表，
  // 写不了伪类。
  //
  // 高亮色用 `var(--surface)`（卡片色）而不是 `var(--bg)`（页面色）：表格本身是透明的，
  // 行背景就等于页面背景，用 `--bg` 做 hover 在两种主题下都**毫无变化**（深色下
  // 两者同为 #0f172a，肉眼完全看不出）。`--surface` 比页面背景亮一档：
  // 深色 #0f172a → #1e293b（明显）、浅色 #f9fafb → #ffffff（轻微但可辨）。
  const [hoverRowId, setHoverRowId] = useState<number | null>(null);

  // task-083：同软件多项可折叠。
  // 折的是「显示」，不是数据——隐藏的行仍在 selected / 分页里，用户勾选过的
  // 不会因为折叠而丢掉，展开后原样回来。
  const [collapsedSoftware, setCollapsedSoftware] = useState<Set<string>>(new Set());

  const toggleSoftware = (label: string) => {
    setCollapsedSoftware((prev) => {
      const next = new Set(prev);
      if (next.has(label)) next.delete(label);
      else next.add(label);
      return next;
    });
  };

  // task-077：盘筛选。空串 = 全部盘。
  //
  // **筛选在服务端做**（`cleanup_list_pending` 的 `drive` 入参）：前端只过滤当前页
  // 的话，`total` 和分页条还是全量数字，用户会看到「共 4576 条」但每页只有几条
  // 匹配，翻十页也翻不完——那比没有筛选更让人困惑。
  const [driveFilter, setDriveFilter] = useState<string>("");
  const [drives, setDrives] = useState<string[]>([]);

  // 盘列表来自后端枚举（只含固定盘）。查不到就不显示下拉框——
  // 给一个空下拉框会让用户以为「没有盘」，而不显示只是少一个筛选器。
  useEffect(() => {
    (async () => {
      try {
        const list = await api<Array<{ letter: string }> | null>("drive_list", {});
        setDrives(Array.isArray(list) ? list.map((d) => d.letter) : []);
      } catch {
        setDrives([]);
      }
    })();
  }, []);

  // task-057：启动时拉一次偏好，confirm 文案据此追加「将自动重启被关闭的进程」。
  // 读失败保持 false（fail-closed，不打扰用户）。
  const [autoRestartOnClose, setAutoRestartOnClose] = useState(false);
  useEffect(() => {
    (async () => {
      try {
        const p = await api<{ autoRestartOnClose: boolean }>("get_prefs", {});
        setAutoRestartOnClose(Boolean(p.autoRestartOnClose));
      } catch {
        setAutoRestartOnClose(false);
      }
    })();
  }, []);

  /**
   * 拉取指定页。
   *
   * `p`/`ps` 显式传入而不是读闭包里的 state：翻页按钮刚 `setPage` 时
   * state 还没更新，直接读会拿到旧页码（翻页不动的经典坑）。
   */
  const load = useCallback(
    async (p: number = page, ps: number = pageSize, drive: string = driveFilter) => {
      setStatus("loading");
      setMsg("");
      // C8：列表重拉后旧预览可能已过期，先清掉。
      setPreview(null);
      try {
        const resp = await api<PendingPage>("cleanup_list_pending", {
          page: p,
          pageSize: ps,
          // task-077：**只在有筛选时才带 `drive`**。
          //
          // 不传 = 全部盘（后端 `Option<String>` 收 None）。显式传 `drive: null`
          // 也能工作，但会让「不筛选」这个最常见路径的实参多一个恒为 null 的字段，
          // 既有测试里 `{page, pageSize}` 的精确断言全要跟着改——契约越窄越好。
          ...(drive === "" ? {} : { drive }),
        });
        setRows(resp.items);
        setTotal(resp.total);
        setPage(resp.page);
        setStatus("ok");
      } catch (e) {
        setRows([]);
        setTotal(0);
        setMsg(String(e));
        setStatus("error");
      }
    },
    [page, pageSize, driveFilter],
  );

  /**
   * task-077：切换盘筛选。
   *
   * 三件事一起做，缺一个都会让用户困惑：
   * 1. **清空勾选**——与翻页同一判断。用户勾了几条 C 盘项、再筛到 D 盘，
   *    「清理已选」仍然会删那几条 C 盘项，而列表里根本看不到它们；
   * 2. 回到第 1 页——筛完只剩 2 页却停在第 5 页会直接空白；
   * 3. 用新盘符重新拉（`load` 的第三个参数显式传，不等 state 更新）。
   */
  const changeDriveFilter = (drive: string) => {
    setDriveFilter(drive);
    setSelected(new Set());
    setPage(1);
    void load(1, pageSize, drive);
  };

  useEffect(() => {
    void load(1, pageSize);
    // 首次挂载只拉第一页；后续翻页走 gotoPage。
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // task-072：应用方案 → 恢复勾选。
  //
  // **一次性消费**：套完立刻通知外层清空载荷，否则翻页回来、或从别的页再进
  // 「清理」页时会把同一份方案再套一遍，把用户后来的手动勾选覆盖掉。
  // 提示文案留在本地 state：载荷被清掉之后用户还得看得到「勾选了几项、几条没找到」。
  useEffect(() => {
    if (!planApply || planApply.itemIds.length === 0) return;
    planApplied.current = true;
    setSelected(new Set(planApply.itemIds));
    setPlanNotice(planApply.notice);
    setPreview(null); // 勾选变了，旧预览失效（与 C8 的处理一致）
    onPlanConsumed?.();
  }, [planApply, onPlanConsumed]);

  // task-072：把当前勾选报告给外层——「方案」页的「保存」按钮要显示能存多少项。
  useEffect(() => {
    onSelectionChange?.(Array.from(selected));
  }, [selected, onSelectionChange]);

  // task-084 C2：草稿暂存。三件事在一个开关（draftHydrated）下串起来：
  //   1. 挂载首拉完成后读回上次勾选（draft_load），按路径回当前页 rows 恢复勾选；
  //   2. 恢复完成之后，任何 selected 变更都即时覆写草稿（draft_save）；
  //   3. 清理成功后清空草稿（draft_clear，在 runCleanup 里）。
  // 为什么必须这套开关：首拉完成前 selected 是空的，若不 gate，那一次「空勾选」变更
  // 会在恢复拖着回写之前先把草稿覆写成空——用户重开的勾选就丢了。
  //
  // 恢复范围说明（受分页约束的取舍）：草稿存的是**路径**，而本页只拉第一页，
  // 恢复只匹配当前 rows（第一页）里存在、且路径在草稿中的项。第一页之外的历史勾选
  // 页面上本来就看不到（用户需翻页手动复核），不强行跨页恢复，避免「勾了却看不见」
  // 的隐性状态。
  const draftHydrated = useRef(false);
  // task-084 C2：本会话是否已应用过「方案」。应用方案的勾选是用户**当下**的选择，
  // 优先级高于旧草稿——若直接按旧草稿恢复，会把刚套好的方案勾选覆盖掉。
  const planApplied = useRef(false);

  useEffect(() => {
    if (draftHydrated.current) return;
    // 等首拉真正落地（status 变 ok 且 rows 非空）再做恢复。
    if (status !== "ok" || rows.length === 0) return;
    // 已应用方案的会话不再恢复旧草稿（方案勾选优先），但标记已化，后续变更照常写草稿。
    if (planApplied.current) {
      draftHydrated.current = true;
      return;
    }
    (async () => {
      try {
        const list = await api<Array<{ itemPath: string }> | null>("draft_load", {});
        if (!Array.isArray(list)) return;
        const draftPaths = new Set(list.map((d) => d.itemPath));
        const restored = new Set(
          rows.filter((r) => draftPaths.has(r.path)).map((r) => r.id),
        );
        if (restored.size > 0) {
          setSelected(restored);
          setMsg(`已恢复上次会话的 ${restored.size} 项勾选`);
        }
      } catch {
        // 读失败 fail-closed：不恢复、也不打扰用户（草稿是低价值状态）。
      } finally {
        draftHydrated.current = true;
      }
    })();
  }, [status, rows]);

  // task-084 C2：恢复完成后，勾选一变就即时覆写整张草稿。
  useEffect(() => {
    if (!draftHydrated.current) return;
    const paths = rows.filter((r) => selected.has(r.id)).map((r) => r.path);
    // 后端是「清空 + 重写」，空 paths = 清草稿（用户手动取消全部勾选）。
    void api<null>("draft_save", { itemPaths: paths }).catch((e) => {
      // 写失败 fail-closed：丢一次草稿不阻塞页面，下次勾选仍会重写。
      console.error("draft_save failed", e);
    });
  }, [rows, selected]);

  /**
   * 翻页：越界直接忽略；翻页时清空勾选。
   *
   * 不清空的话，用户在 A 页勾了几项、翻到 B 页又勾几项，点「清理已选」会一次性
   * 删掉两个页面的条目 —— 用户看不到 A 页的勾选状态，属于隐性误操作。
   */
  const gotoPage = useCallback(
    (next: number) => {
      const maxPage = Math.max(1, Math.ceil(total / pageSize));
      const target = Math.min(Math.max(1, next), maxPage);
      if (target === page) return;
      setSelected(new Set());
      setPreview(null);
      setResult(null);
      void load(target, pageSize);
    },
    [page, pageSize, total, load],
  );

  const toggle = (id: number) => {
    // C8：勾选集合一变，旧预览就失效。
    setPreview(null);
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const totalSelectedBytes = rows
    .filter((r) => selected.has(r.id))
    .reduce((sum, r) => sum + r.sizeBytes, 0);

  // task-064：本页的 L1 安全项（「全选 L1」按钮用）。
  // 只统计**当前页**——跨页全选要先把所有页拉下来，而列表是分页的，
  // 那会让按钮点击变慢且行为难以解释（用户看不到被选中的项）。
  const l1Rows = rows.filter((r) => r.riskLevel === "L1");
  const l1Count = l1Rows.length;
  const l1Bytes = l1Rows.reduce((sum, r) => sum + r.sizeBytes, 0);

  const selectAllL1 = useCallback(() => {
    setSelected(new Set(l1Rows.map((r) => r.id)));
  }, [l1Rows]);

  // task-049：三态互斥（清理 / 重试 / 预览 任一为真时，其余操作按钮禁用）。
  const busy = cleaning || retrying || previewing;

  // task-064：本页的 L0 提示项（task-084 B2「一键清理 L0」用）。
  // L0 = 完全无风险临时文件（风险徽标底色最淡）。同样**只统计当前页**。
  const l0Rows = rows.filter((r) => r.riskLevel === "L0");
  const l0Count = l0Rows.length;
  const l0Bytes = l0Rows.reduce((sum, r) => sum + r.sizeBytes, 0);

  /**
   * 清理核心（供「清理已选」与 task-084 B2「一键清理 L0」共用）。
   *
   * 抽出而不是各自内联：两处都要走「接住 2007 还原点失败 → 让用户决定继续」这条
   * 路径，还要渲染统一的结果条 + 重拉列表。唯一区别是 ids 来源不同。
   */
  const runCleanup = useCallback(
    async (ids: number[]) => {
      if (ids.length === 0 || busy) return;
      setCleaning(true);
      setResult(null);
      // C8：清理成功后清单会变，预览必然过期。
      setPreview(null);
      try {
        // task-062：含 L2 风险项时后端会先建系统还原点（弹一次 UAC）。
        // 用 apiRaw 而不是 api——需要接住 2007（还原点失败）这个业务码，
        // api() 会把它当异常抛掉，拿不到码就没法区分「还原点失败」和「清理失败」。
        let env = await apiRaw<ExecuteSummary>("cleanup_execute", { itemIds: ids });
        if (env.code === 2007) {
          // 还原点创建失败（最常见原因：系统还原被关闭 / UAC 被拒）。
          // 让用户自己决定要不要继续——不替他做决定，但也不让流程卡死。
          const go = window.confirm(
            "系统还原点创建失败。\n\n" +
              "常见原因：系统还原功能未开启、UAC 提权被取消、或还原点磁盘空间不足。\n\n" +
              "继续清理的话，文件仍会先移入保险箱（7 天内可恢复），但**没有系统级回滚点**。\n\n" +
              "是否继续清理？",
          );
          if (!go) {
            setResult({
              tone: "fail",
              headline: "已取消清理：系统还原点创建失败",
              failures: [],
              refused: [],
            });
            return;
          }
          // 用户确认继续 → 带 skipRestorePoint 重调。
          env = await apiRaw<ExecuteSummary>("cleanup_execute", {
            itemIds: ids,
            skipRestorePoint: true,
          });
        }
        if (env.code !== 0 || !env.data) {
          throw new Error(env.message || `清理失败（code=${env.code}）`);
        }
        const summary = env.data;
        // task-052 G5：已不存在（alreadyGone）既不算成功释放、也不算失败，
        // 必须从失败明细里排除，否则会渲染出一堆「— 未知原因」的假失败行。
        const failures = summary.results.filter((r) => !r.deleted && !r.alreadyGone);
        const gone = summary.alreadyGoneCount ?? 0;
        // 走查 G5 的原始症状是「已清理 1 项，释放 0 B」：真删了 0 字节却报成功。
        // 现在把「已不存在」单独说出来，用户能看懂为什么释放量比预期小。
        const okHeadline =
          `已清理 ${summary.deletedCount} 项，释放 ${formatBytes(summary.bytesFreedTotal)}` +
          (gone > 0 ? `；${gone} 项已不存在（未计入释放量）` : "");
        setResult({
          tone: failures.length === 0 ? "ok" : "fail",
          headline:
            failures.length === 0
              ? okHeadline
              : `清理完成：成功 ${summary.deletedCount} 项 / 失败 ${summary.failedCount} 项` +
                (gone > 0 ? ` / 已不存在 ${gone} 项` : ""),
          failures,
          refused: [],
          // task-063：留着完整结果，供结果条上的「导出报告」使用。
          summary,
        });
        setSelected(new Set());
        // task-084 C2：清理落地后草稿作废，清掉（避免下次重开又恢复出已删路径）。
        void api<null>("draft_clear", {}).catch(() => {});
        await load();
      } catch (e) {
        setResult({
          tone: "fail",
          headline: `清理失败：${String(e)}`,
          failures: [],
          refused: [],
        });
      } finally {
        setCleaning(false);
      }
    },
    [busy, load]
  );

  const cleanup = useCallback(
    () => runCleanup(Array.from(selected)),
    [runCleanup, selected]
  );

  // task-084 B2：一键清理 L0。直接清本页所有 L0 项，不要求先手动勾选。
  const cleanL0 = useCallback(() => {
    if (l0Count === 0 || busy) return;
    const roomHint = formatBytes(l0Bytes);
    const go = window.confirm(
      `将清理本页 ${l0Count} 项 L0 无风险临时文件（约 ${roomHint}）。\n\n` +
      "L0 为完全无风险项，删除不影响系统；清理前仍会创建系统还原点。是否继续？",
    );
    if (!go) return;
    runCleanup(l0Rows.map((r) => r.id));
  }, [l0Count, l0Bytes, l0Rows, busy, runCleanup]);

  // task-063：导出报告。把本次清理结果原样传回后端渲染（结果只在前端内存里）。
  const exportReport = useCallback(
    async (fmt: "html" | "csv") => {
      if (!result?.summary) return;
      setExporting(true);
      setExportMsg("");
      try {
        const path = await api<string>("report_export", {
          format: fmt,
          summary: result.summary,
          // task-086 1.1：用户填了目录就导出到那里；没填不传（后端回落默认目录）。
          ...(exportDir.trim() !== "" ? { path: exportDir.trim() } : {}),
        });
        setExportMsg(`已保存：${path}`);
      } catch (e) {
        setExportMsg(`导出失败：${String(e)}`);
      } finally {
        setExporting(false);
      }
    },
    [result, exportDir],
  );

  // task-049 C10/C11：确认 → 关闭占用进程 → 自动重试删除。
  // C12：与 cleanup / preview 共用 busy，重试按钮同样纳入三态互斥。
  const retryLocked = useCallback(async () => {
    if (!result || busy) return;
    const locked = result.failures.filter((f) => f.lockingProcesses.length > 0);
    if (locked.length === 0) return;

    const procs = dedupeProcs(locked.flatMap((f) => f.lockingProcesses));
    const list = procs.map(formatProc).join("\n");
    // task-057：开关开启时追加一行提示，让用户知道关闭后进程会被自动拉起。
    const restartHint = autoRestartOnClose
      ? "\n注意：本次会在关闭后自动重启被关闭且支持重启的进程。"
      : "";
    const confirmed = window.confirm(
      `即将请求关闭以下进程：\n${list}\n\n未保存的数据可能丢失，关闭后会自动重试删除。\n注意：同一控制台内的相关进程可能一并关闭。${restartHint}\n是否继续？`,
    );
    // 取消：直接返回，不调命令，按钮保持可点。
    if (!confirmed) return;

    setRetrying(true);
    setShutdownSecs(0);
    // task-060：订阅「关闭进度」。后端在 RmShutdown 阻塞期间每秒推一次
    // `{ elapsedSecs, expectedMaxSecs }`，这里只取已等秒数。
    // 订阅失败（非 Tauri 环境 / 事件通道异常）不影响主流程，进度条只是不出现。
    let unlisten: (() => void) | null = null;
    try {
      unlisten = await listen<{ elapsedSecs: number }>("shutdown://progress", (e) => {
        setShutdownSecs(e.payload.elapsedSecs);
      });
    } catch {
      // 订阅失败（非 Tauri 环境 / 事件通道异常）不影响主流程，
      // 只是进度条不出现——用户仍能看到按钮变成「关闭并重试中…」。
    }
    try {
      const resp = await api<RetryLockedResp>("cleanup_retry_locked", {
        itemIds: locked.map((f) => f.itemId),
      });
      // task-052 G5：重试路径同样排除「已不存在」，否则会显示假失败。
      const failures = resp.summary.results.filter((r) => !r.deleted && !r.alreadyGone);
      const closedCount = resp.shutdown.closed.length;
      // close-experience-copy：mixed fail-closed（同批含受保护进程 → 后端整批不关）时
      // closed=0 且 refused 非空，需在结果条里解释「一个都没关」，否则用户只看到
      // 「已关闭 0 个进程，仍失败 N 项」会误以为点了没反应。
      const protectedBlocking = closedCount === 0 && resp.shutdown.refused.length > 0;
      setResult({
        tone: failures.length === 0 ? "ok" : "fail",
        headline:
          failures.length === 0
            ? `已关闭 ${closedCount} 个进程，重试成功 ${resp.summary.deletedCount} 项`
            : protectedBlocking
              ? `已关闭 0 个进程：因存在受保护进程，本次未关闭任何进程，仍失败 ${failures.length} 项`
              : `已关闭 ${closedCount} 个进程，仍失败 ${failures.length} 项`,
        failures,
        refused: resp.shutdown.refused,
      });
      // 无论成败都重拉清单、清空选中与预览。
      setSelected(new Set());
      setPreview(null);
      await load();
    } catch (e) {
      // 失败时保留原失败明细，用户还能再点一次重试。
      setResult((prev) =>
        prev ? { ...prev, tone: "fail", headline: `关闭并重试失败：${String(e)}` } : prev,
      );
    } finally {
      // task-060：先退订再清状态，避免退订前的最后一条事件把进度条又点亮。
      unlisten?.();
      setShutdownSecs(null);
      setRetrying(false);
    }
  }, [result, busy, load, autoRestartOnClose]);

  // task-040 C2：只读预览。走 apiRaw 以拿回 2002 信封里的 blockedByL3。
  const previewSelection = useCallback(async () => {
    if (selected.size === 0 || busy) return;
    setPreviewing(true);
    setPreview(null);
    try {
      const env = await apiRaw<DryRunResp>("cleanup_dry_run", {
        itemIds: Array.from(selected),
      });
      if (env.code === 0 && env.data) {
        // C3
        setPreview({ kind: "ok", resp: env.data });
      } else if (env.code === 2002) {
        // C4：L3 命中明细就在失败信封的 data 上。
        setPreview({ kind: "blocked", blocked: env.data?.blockedByL3 ?? [] });
      } else if (env.code === 2001) {
        // C5
        setPreview({ kind: "empty" });
      } else {
        // C6
        setPreview({ kind: "error", message: env.message });
      }
    } catch (e) {
      // C6：invoke 本身 reject（命令未注册 / 序列化失败等）。
      setPreview({ kind: "error", message: String(e) });
    } finally {
      setPreviewing(false);
    }
  }, [selected, busy]);

  // task-083：把一行渲染成 <tr>。抽到函数里是因为分组渲染时每组要复用同一套
  // checkbox / hover / 风险徽标逻辑，避免重复。
  const renderRow = (r: PendingItem) => (
    <tr
      key={r.id}
      onMouseEnter={() => setHoverRowId(r.id)}
      onMouseLeave={() => setHoverRowId(null)}
      style={{ background: hoverRowId === r.id ? "var(--surface)" : "transparent" }}
    >
      <td style={tdStyle}>
        <input type="checkbox" checked={selected.has(r.id)} onChange={() => toggle(r.id)} />
      </td>
      <td style={tdStyle}>
        {/* task-083：hover 文案优先用规则库逐项说明（impact/rebuild note），
            没有才退回通用等级说明——数据驱动的 hover。 */}
        <span
          title={impactHint(r, riskMeta(r.riskLevel).hint)}
          style={{
            display: "inline-block",
            padding: "1px 6px",
            borderRadius: 4,
            fontSize: 11,
            fontWeight: 600,
            color: riskMeta(r.riskLevel).fg,
            background: riskMeta(r.riskLevel).bg,
            border: `1px solid ${riskMeta(r.riskLevel).fg}`,
            cursor: "help",
            whiteSpace: "nowrap",
          }}
        >
          {riskMeta(r.riskLevel).label}
        </span>
      </td>
      {/* task-083：软件列。软件缓存条目显示软件名；其余显示「—」。 */}
      <td style={tdStyle}>{softwareLabel(r)}</td>
      <td style={tdStyle} title={r.path}>
        <div
          style={{
            maxWidth: 400,
            overflowX: "auto",
            whiteSpace: "nowrap",
            fontFamily: "monospace",
            fontSize: 12,
            // 滚动条跟随主题（标准属性，可写进 inline style）
            scrollbarWidth: "thin",
            scrollbarColor: "var(--border) transparent",
          } as React.CSSProperties}
        >
          {r.path}
        </div>
      </td>
      <td style={tdStyle}>{formatBytes(r.sizeBytes)}</td>
      <td style={tdStyle}>{r.moduleCode}</td>
      <td style={tdStyle}>#{r.sessionId}</td>
    </tr>
  );

  // task-083：按软件分组渲染。同一软件的多个待清理项合成一组，组头带折叠按钮；
  // 折叠时只显示首行，其余隐藏（数据仍在 selected / 分页里，不丢勾选）。
  // 非软件条目（label === "—"）逐行渲染，不参与分组。
  const renderRows = () => {
    const groups: { label: string; items: PendingItem[] }[] = [];
    for (const r of rows) {
      const label = softwareLabel(r);
      const last = groups[groups.length - 1];
      if (last && last.label === label) last.items.push(r);
      else groups.push({ label, items: [r] });
    }
    const out: React.ReactNode[] = [];
    groups.forEach((g) => {
      const isSoftware = g.label !== "—";
      const collapsed = isSoftware && collapsedSoftware.has(g.label);
      if (isSoftware && g.items.length > 1) {
        out.push(
          <tr key={`grp-${g.label}`} style={{ background: "var(--bg)" }}>
            <td colSpan={7} style={{ ...tdStyle, paddingLeft: 8 }}>
              <button
                type="button"
                onClick={() => toggleSoftware(g.label)}
                style={{
                  background: "transparent",
                  border: "none",
                  color: "var(--text)",
                  cursor: "pointer",
                  fontSize: 13,
                  fontWeight: 600,
                  padding: "2px 4px",
                }}
              >
                {collapsed ? "▶" : "▼"} {g.label}（{g.items.length} 项）
              </button>
            </td>
          </tr>,
        );
        const visible = collapsed ? g.items.slice(0, 1) : g.items;
        visible.forEach((r) => out.push(renderRow(r)));
      } else {
        g.items.forEach((r) => out.push(renderRow(r)));
      }
    });
    return out;
  };

  return (
    <div>
      {/* task-072：应用方案后的提示条。可关掉——它只是「刚才发生了什么」，
          不该一直占着版面。 */}
      {planNotice && (
        <div
          data-testid="plan-notice"
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "flex-start",
            gap: 12,
            background: "rgba(37, 99, 235, 0.12)",
            border: "1px solid rgba(37, 99, 235, 0.4)",
            color: "#3b82f6",
            padding: "8px 10px",
            borderRadius: 6,
            fontSize: 12,
            marginBottom: 12,
          }}
        >
          <span>{planNotice}</span>
          <button
            onClick={() => setPlanNotice("")}
            style={{
              border: "none",
              background: "transparent",
              color: "#3b82f6",
              cursor: "pointer",
              fontSize: 12,
              padding: 0,
            }}
            aria-label="关闭方案提示"
          >
            关闭
          </button>
        </div>
      )}
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 12 }}>
        <button onClick={() => void load(page, pageSize)} disabled={busy}>
          刷新
        </button>
        {/* task-077：盘筛选。只在枚举到盘时出现（查不到就不显示，别给空下拉框）。 */}
        {drives.length > 0 && (
          <label style={{ fontSize: 13, color: "var(--text)", display: "flex", alignItems: "center", gap: 6 }}>
            磁盘
            <select
              aria-label="按磁盘筛选"
              value={driveFilter}
              onChange={(e) => changeDriveFilter(e.target.value)}
              disabled={busy}
              style={{
                padding: "4px 8px",
                borderRadius: 6,
                border: "1px solid var(--border)",
                // 原生 select 不跟随主题：不给 background/color 的话，
                // 深色下会保留系统的白底黑字，在暗页面上像个洞。
                background: "var(--surface)",
                color: "var(--text)",
              }}
            >
              <option value="">全部</option>
              {drives.map((d) => (
                <option key={d} value={d}>
                  {d}:
                </option>
              ))}
            </select>
          </label>
        )}
        <span style={{ fontSize: 13, color: "var(--text)" }}>
          已选 {selected.size} 项，共 {formatBytes(totalSelectedBytes)}
        </span>
        {/* task-064：一键全选 L1（安全项）。L2/L3 一律不选——L2 需要逐项确认，
            自动全选 L2 等于绕过「需确认」这道设计。 */}
        <button
          onClick={selectAllL1}
          disabled={busy || l1Count === 0}
          title={
            l1Count === 0
              ? "本页没有 L1 安全项"
              : `勾选本页 ${l1Count} 项 L1 安全缓存（约 ${formatBytes(l1Bytes)}）`
          }
          style={{
            padding: "6px 14px",
            borderRadius: 6,
            border: "1px solid #16a34a",
            background: l1Count === 0 ? "rgba(22, 163, 74, 0.3)" : "#16a34a",
            // task-081：disabled 态底色是半透明，白字在上面几乎看不见（浅色下更明显）。
            // 用 muted 文字——它在两种主题下都与半透明底有足够对比度。
            color: l1Count === 0 ? "var(--text-muted)" : "#fff",
            fontSize: 13,
            cursor: l1Count === 0 ? "not-allowed" : "pointer",
          }}
        >
          全选 L1（安全）
        </button>
        {/* task-084 B2：一键清理 L0。L0 = 完全无风险临时文件，直接清，不用先勾选。
            底色用 muted 灰——刻意比 L1 绿色更「弱」，体现「无风险」不是「亮点」。 */}
        <button
          onClick={cleanL0}
          disabled={busy || l0Count === 0}
          title={
            l0Count === 0
              ? "本页没有 L0 无风险项"
              : `一键清理本页 ${l0Count} 项 L0 临时文件（约 ${formatBytes(l0Bytes)}）`
          }
          style={{
            padding: "6px 14px",
            borderRadius: 6,
            border: "1px solid var(--border)",
            background: l0Count === 0 ? "var(--surface)" : "var(--bg)",
            color: l0Count === 0 ? "var(--text-muted)" : "var(--text)",
            fontSize: 13,
            cursor: l0Count === 0 ? "not-allowed" : "pointer",
          }}
        >
          {cleaning ? "清理中…" : "一键清理 L0"}
        </button>
        <button
          onClick={previewSelection}
          disabled={selected.size === 0 || busy}
          style={{
            padding: "6px 14px",
            borderRadius: 6,
            border: "1px solid #2563eb",
            background: selected.size === 0 || busy ? "rgba(37, 99, 235, 0.3)" : "#2563eb",
            color: selected.size === 0 || busy ? "var(--text-muted)" : "#fff",
            cursor: selected.size === 0 || busy ? "not-allowed" : "pointer",
          }}
        >
          {previewing ? "预览中…" : "预览可清理项"}
        </button>
        <button
          onClick={cleanup}
          disabled={selected.size === 0 || busy}
          style={{
            padding: "6px 14px",
            borderRadius: 6,
            border: "1px solid #b91c1c",
            background: selected.size === 0 || busy ? "rgba(185, 28, 28, 0.3)" : "#b91c1c",
            color: selected.size === 0 || busy ? "var(--text-muted)" : "#fff",
            cursor: selected.size === 0 || busy ? "not-allowed" : "pointer",
          }}
        >
          {cleaning ? "清理中…" : "清理已选"}
        </button>
      </div>

      {/* task-049：结果条 = 汇总 + 失败明细（原因 + 占用进程）+ 关闭并重试。 */}
      {result && (
        <div
          style={{
            marginBottom: 12,
            padding: 10,
            borderRadius: 6,
            fontSize: 13,
            background:
              result.tone === "ok" ? "rgba(22, 163, 74, 0.16)" : "rgba(220, 38, 38, 0.16)",
            color: result.tone === "ok" ? "#16a34a" : "#ef4444",
            border: `1px solid ${
              result.tone === "ok" ? "rgba(22, 163, 74, 0.45)" : "rgba(220, 38, 38, 0.45)"
            }`,
          }}
        >
          <div style={{ fontWeight: 600 }}>{result.headline}</div>

          {/* task-063：前后空间对比。两个值都拿到才显示——只有一侧时显示「—」没意义。 */}
          {result.summary?.diskBeforeBytes != null &&
            result.summary?.diskAfterBytes != null && (
              <div style={{ fontSize: 12, marginTop: 6, lineHeight: 1.6 }}>
                磁盘已用：清理前 {formatBytes(result.summary.diskBeforeBytes)} → 清理后{" "}
                {formatBytes(result.summary.diskAfterBytes)}，实际释放{" "}
                {formatBytes(
                  Math.max(0, result.summary.diskBeforeBytes - result.summary.diskAfterBytes),
                )}
              </div>
            )}

          {/* task-063：导出报告（HTML 单文件 / CSV 明细）。 */}
          {result.summary && (
            <div style={{ marginTop: 8, display: "flex", gap: 8, alignItems: "center" }}>
              <button
                onClick={() => exportReport("html")}
                disabled={exporting}
                style={exportBtnStyle}
              >
                导出 HTML 报告
              </button>
              <button
                onClick={() => exportReport("csv")}
                disabled={exporting}
                style={exportBtnStyle}
              >
                导出 CSV
              </button>
              {/* task-086 1.1：PC/脚本没有原生右键「另存为」的系统对话框，
                  这里用可编辑目录输入框让用户自选保存位置（空 = 默认目录）。 */}
              <input
                type="text"
                value={exportDir}
                onChange={(e) => setExportDir(e.target.value)}
                placeholder="导出目录（留空 = 默认 reports 目录）"
                disabled={exporting}
                style={{
                  flex: 1,
                  minWidth: 160,
                  padding: "4px 8px",
                  borderRadius: 5,
                  border: "1px solid var(--border)",
                  background: "var(--bg)",
                  color: "var(--text)",
                  fontSize: 12,
                }}
              />
              {exportMsg && (
                <span style={{ fontSize: 11, color: "var(--text-muted)", wordBreak: "break-all" }}>
                  {exportMsg}
                </span>
              )}
            </div>
          )}

          {result.failures.length > 0 && (
            <div style={{ marginTop: 8 }}>
              {result.failures.map((f) => (
                <div key={f.itemId} style={{ marginBottom: 4, lineHeight: 1.5 }}>
                  <span style={{ fontFamily: "monospace", fontSize: 12 }}>
                    {f.path || `#${f.itemId}`}
                  </span>
                  {" — "}
                  {describeExecError(f.error)}
                  {f.lockingProcesses.length > 0 && (
                    <>
                      {" — 占用："}
                      <span style={{ fontFamily: "monospace", fontSize: 12 }}>
                        {f.lockingProcesses.map(formatProc).join(", ")}
                      </span>
                    </>
                  )}
                </div>
              ))}
            </div>
          )}

          {/* 受保护进程必须让用户看见，不能静默跳过。 */}
          {result.refused.length > 0 && (
            <div
              style={{
                marginTop: 8,
                padding: "6px 8px",
                borderRadius: 4,
                background: "rgba(234, 179, 8, 0.18)",
                color: "#ca8a04",
                border: "1px solid rgba(234, 179, 8, 0.45)",
              }}
            >
              以下进程受保护未关闭：
              {result.refused.map((r) => `${r.appName}（${r.reason}）`).join("、")}
            </div>
          )}

          {/* 只有存在占用进程时才给重试入口；没有占用就别让用户空点。
              C12：cleaning / retrying / previewing 任一为真时禁用。 */}
          {result.failures.some((f) => f.lockingProcesses.length > 0) && (
            <button
              onClick={retryLocked}
              disabled={busy}
              style={{
                marginTop: 10,
                padding: "6px 14px",
                borderRadius: 6,
                border: "1px solid #b45309",
                background: busy ? "#fde68a" : "#d97706",
                color: "#fff",
                cursor: busy ? "not-allowed" : "pointer",
              }}
            >
              {retrying ? "关闭并重试中…" : "关闭占用进程并重试"}
            </button>
          )}

          {/* task-060：关闭进程期间的进度反馈。
              RmShutdown 是阻塞调用、最坏等 30s，此前用户只看到按钮文字变化，
              会以为卡死。这里按后端每秒推送的「已等秒数」画线性进度条；
              超过 30s 不报错、继续显示，只是补一句「仍在等待系统响应」——
              因为 RmShutdown 没有超时参数，我们也中断不了它。 */}
          {retrying && shutdownSecs !== null && (
            <div style={{ marginTop: 8, maxWidth: 380 }}>
              <div style={{ fontSize: 12, color: "#d97706", marginBottom: 4 }}>
                正在关闭进程… 已等 {shutdownSecs}s（最多 30s）
                {shutdownSecs > 30 ? "，已超过预期，仍在等待系统响应" : ""}
              </div>
              <div
                style={{
                  height: 6,
                  background: "#fde68a",
                  borderRadius: 3,
                  overflow: "hidden",
                }}
              >
                <div
                  style={{
                    height: "100%",
                    width: `${Math.min(100, (shutdownSecs / 30) * 100)}%`,
                    background: "#d97706",
                    transition: "width 300ms linear",
                  }}
                />
              </div>
            </div>
          )}
        </div>
      )}

      {/* task-040 预览面板：只读展示，面板内不提供执行入口。 */}
      {preview?.kind === "ok" && (
        <div style={panelStyle("rgba(37, 99, 235, 0.12)", "#3b82f6", "rgba(37, 99, 235, 0.4)")}>
          <div style={{ fontWeight: 600, marginBottom: 6 }}>
            {formatDryRunHeadline(preview.resp)}
          </div>
          {sortBreakdown(preview.resp.breakdown).map((row) => (
            <div key={`${row.moduleCode}/${row.category}`} style={{ marginBottom: 2 }}>
              {formatBreakdownLine(row)}
            </div>
          ))}
          <div style={{ marginTop: 8, color: "#3b82f6" }}>
            确认无误后点上方「清理已选」执行
          </div>
        </div>
      )}

      {preview?.kind === "blocked" && (
        <div style={panelStyle("rgba(220, 38, 38, 0.14)", "#ef4444", "rgba(220, 38, 38, 0.4)")}>
          <div style={{ fontWeight: 600, marginBottom: 6 }}>
            {preview.blocked.length} 项命中红线，已阻止清理
          </div>
          {preview.blocked.map((b) => (
            <div key={b.itemId} style={{ marginBottom: 2 }}>
              <span style={{ fontFamily: "monospace", fontSize: 12 }}>#{b.itemId}</span>{" "}
              <span style={{ fontFamily: "monospace", fontSize: 12 }}>{b.path}</span>
              {" · "}
              {describeDenial(b.denialReason)}
            </div>
          ))}
        </div>
      )}

      {preview?.kind === "empty" && (
        <div style={panelStyle("rgba(234, 179, 8, 0.16)", "#ca8a04", "rgba(234, 179, 8, 0.4)")}>
          已选条目已不在待清理列表（可能已被清理或扫描会话已过期），请点「刷新」
        </div>
      )}

      {preview?.kind === "error" && (
        <div style={panelStyle("rgba(220, 38, 38, 0.14)", "#ef4444", "rgba(220, 38, 38, 0.4)")}>
          预览失败：{preview.message}
        </div>
      )}

      {status === "loading" && <div style={{ color: "var(--text-muted)", fontSize: 13 }}>加载中...</div>}

      {status === "error" && (
        <div style={{ color: "#ef4444", fontSize: 13 }}>
          加载失败：{msg}
        </div>
      )}

      {status === "ok" && (
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr>
              <th style={{ ...thStyle, width: 32 }}>
                <input
                  type="checkbox"
                  checked={rows.length > 0 && selected.size === rows.length}
                  onChange={(e) => {
                    // C8：全选/全不选同样让旧预览失效。
                    setPreview(null);
                    if (e.target.checked) setSelected(new Set(rows.map((r) => r.id)));
                    else setSelected(new Set());
                  }}
                />
              </th>
              <th style={{ ...thStyle, width: 56 }}>风险</th>
              {/* task-083：软件列。放在路径之前——用户先确认「这是哪个软件的缓存」，
                  再看具体路径，比反过来更符合阅读顺序。 */}
              <th style={{ ...thStyle, width: 150 }}>软件</th>
              <th style={thStyle}>路径</th>
              <th style={{ ...thStyle, width: 100 }}>大小</th>
              <th style={{ ...thStyle, width: 80 }}>模块</th>
              <th style={{ ...thStyle, width: 80 }}>Session</th>
            </tr>
          </thead>
          <tbody>
            {rows.length === 0 && (
              <tr>
                <td colSpan={7} style={{ ...tdStyle, color: "var(--text-muted)", textAlign: "center" }}>
                  暂无待清理项
                </td>
              </tr>
            )}
            {rows.length > 0 && renderRows()}
          </tbody>
        </table>
      )}

      {/* task-052 G2：分页栏（样式对齐 SafeBox 的翻页栏）。 */}
      {status === "ok" && (
        <div
          style={{
            marginTop: 16,
            display: "flex",
            justifyContent: "center",
            alignItems: "center",
            gap: 12,
            fontSize: 13,
            color: "var(--text)",
          }}
        >
          <button onClick={() => gotoPage(page - 1)} disabled={page <= 1 || status === "loading"}>
            上一页
          </button>
          <span>
            第 {page} / {Math.max(1, Math.ceil(total / pageSize))} 页（共 {total} 条）
          </span>
          <button
            onClick={() => gotoPage(page + 1)}
            disabled={page * pageSize >= total || status === "loading"}
          >
            下一页
          </button>
        </div>
      )}
    </div>
  );
}
