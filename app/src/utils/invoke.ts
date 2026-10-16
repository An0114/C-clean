//! 统一 IPC 调用封装（debt-cleanup-002 / Bug-1）。
//!
//! 后端所有 `#[tauri::command]` 都返回 `Result<Envelope<T>, String>`，
//! 前端裸 `invoke` 拿到的是 `{code, message, data}` 信封，不是 T 本身。
//! 之前 5 个页面全部把信封当 T 直接用，导致：
//!   - Items/History 渲染 `rows.filter/rows.map` 时 rows 是信封对象，
//!     抛 TypeError → React 无 Error Boundary 整树卸载 → 白屏；
//!   - Dashboard/SafeBox/Settings 数字字段全 undefined → 卡片空 / 版本 "-"。
//!
//! 本模块把信封解包收敛到一处：`code !== 0` 抛错，`data` 就是真数据。

import { invoke } from "@tauri-apps/api/core";

/// 与 engine::ipc::envelope::Envelope 对齐的 JSON shape。
export interface Envelope<T> {
  code: number;
  message: string;
  data: T | null;
}

/// 解包后的 IPC 调用。失败（code !== 0）抛 Error(message)，
/// 成功直接返回 data（调用方无需再做 `|| []` / `|| 0` 兜底）。
export async function api<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const env = await invoke<Envelope<T>>(cmd, args);
  if (env.code !== 0) {
    throw new Error(env.message);
  }
  // 成功路径 data 一定是 Some(T)；失败路径已抛错。
  return env.data as T;
}

/// 原样返回信封的 IPC 调用（task-040）。
///
/// `api()` 在 `code !== 0` 时抛错，会丢掉失败信封里的 `data`；但 cleanup.dryRun 的
/// 2002（L3 红线）**显式偏离 TRD §5.1**，用的是 `err_with_data` —— 失败时 `data`
/// 非 null 且带 `blockedByL3` 明细。预览要展示这份明细，就必须读原始信封。
///
/// 本函数对任何 `code` 都不抛错，由调用方按 code 自行分支。
export async function apiRaw<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<Envelope<T>> {
  return invoke<Envelope<T>>(cmd, args);
}
