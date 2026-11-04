//! `scan.start` 命令纯逻辑（本任务不真启动扫描，只发递增 sessionId）。
//!
//! # task-077：从「单盘」到「多盘」
//!
//! 入参原来只有 `targetDrive: Option<String>`（单盘，形如 `"C:"`），且**只校验不使用**
//! ——`commands.rs` 校验完就丢了，扫描永远是全量。task-077 加 `drives: Vec<String>`，
//! 并把它真正传给 `ScanOrchestrator::with_drives`。
//!
//! `targetDrive` 保留不删：前端 `Dashboard.tsx` 一直在传 `targetDrive: null`，
//! 立刻删会连带改前端（本单元不该动 UI）。两者同时出现时 **`drives` 优先**，
//! 语义上 `drives` 是它的多值超集。

use super::super::envelope::Envelope;
use super::super::error_codes::INTERNAL;

/// `scan.start` 入参。前端传 camelCase（targetDrive / drives），Rust 侧见 snake_case。
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStartReq {
    /// "full" | "custom"。
    pub scope: String,
    /// 单盘符选择，形如 "C:"（可选，task-004 起就有；被 `drives` 取代）。
    pub target_drive: Option<String>,
    /// task-077：多盘选择，形如 `["C", "D"]` 或 `["C:", "D:"]`（可选）。
    ///
    /// - 不传（`None`）= 全部固定盘；
    /// - 传空数组 = **非法**（见 [`resolve_drives`]）：用户说「一个盘都不扫」
    ///   与「不指定」是两件事，静默按「全部」处理会让「我只想扫 D 盘」变成
    ///   「扫了全部」，方向正好相反。
    pub drives: Option<Vec<String>>,
}

/// `scan.start` 出参：立即返回会话 id，进度走事件通道。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStartResp {
    /// 递增整数会话 id。
    pub session_id: i64,
}

/// 校验单个盘符字符串，返回大写字母。
///
/// 接受 `"C"` / `"C:"` / `"c"` / `"c:"` 四种写法——前端下拉框的值是 `"C:"`，
/// 而 API 文档里写的是 `"C"`，两种都会真的传进来，只认一种就是等着踩。
fn parse_drive(raw: &str) -> Option<char> {
    let s = raw.trim();
    let letter = s.strip_suffix(':').unwrap_or(s);
    let mut chars = letter.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    if !c.is_ascii_alphabetic() {
        return None;
    }
    Some(c.to_ascii_uppercase())
}

/// 盘符入参非法。
///
/// 用一个专门的错误类型而不是 `Result<_, ()>`：clippy 的 `result_unit_err` 拦的
/// 正是「丢了失败原因」这种写法，而这里确实有值得区分的原因（空数组 vs 格式错），
/// 排查「为什么点了扫描没反应」时一句日志就能定位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadDriveArg {
    /// `drives: []`——用户说「一个盘都不扫」，与「不指定」是两件事。
    EmptyList,
    /// 盘符格式不对（`""` / `"1"` / `"CC"` / `"C:\"` ...）。
    Malformed,
}

impl std::fmt::Display for BadDriveArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyList => write!(f, "drives 不能为空数组（不指定请传 null）"),
            Self::Malformed => write!(f, "盘符格式非法，应形如 \"C\" 或 \"C:\""),
        }
    }
}

/// 把入参解析成「要扫的盘符列表」：`None` = 全部固定盘。
///
/// 返回 `Err` 表示入参非法，调用方应回 `INTERNAL`（与既有 `scope` 校验一致）。
///
/// 规则：
/// 1. `drives` 与 `target_drive` 都为空 → `Ok(None)`（全部）；
/// 2. `drives` 非空 → 逐项解析，任何一项非法整体失败（不做「跳过坏的那项」——
///    用户以为扫了三个盘、实际只扫了两个，比直接报错更糟）；
/// 3. `drives` 为空数组 → `Err(EmptyList)`（见 `ScanStartReq::drives` 的注释）；
/// 4. 只有 `target_drive` → 按单盘解析，等价于 `drives = [那一个]`。
///
/// 去重：`["C", "C:"]` 归一化后都是 `'C'`，重复项只留一个，避免编排层把同一个盘
/// 算两次（`per_drive` 里出现两条 `C:`）。
pub fn resolve_drives(req: &ScanStartReq) -> Result<Option<Vec<char>>, BadDriveArg> {
    // 1) 多盘优先
    if let Some(list) = &req.drives {
        if list.is_empty() {
            return Err(BadDriveArg::EmptyList);
        }
        let mut out = Vec::with_capacity(list.len());
        for raw in list {
            let d = parse_drive(raw).ok_or(BadDriveArg::Malformed)?;
            if !out.contains(&d) {
                out.push(d);
            }
        }
        return Ok(Some(out));
    }

    // 2) 兼容旧的单盘入参
    if let Some(raw) = &req.target_drive {
        let d = parse_drive(raw).ok_or(BadDriveArg::Malformed)?;
        return Ok(Some(vec![d]));
    }

    // 3) 都不传 = 全部固定盘
    Ok(None)
}

/// 处理 scan.start：校验入参 → 递增 sessionId → 返回信封。
/// 不启动后台线程、不写 DB、不 emit 进度（后续任务接入）。
pub fn handle_scan_start(req: ScanStartReq, next_session_id: &mut i64) -> Envelope<ScanStartResp> {
    if req.scope != "full" && req.scope != "custom" {
        return Envelope::err(INTERNAL);
    }
    if resolve_drives(&req).is_err() {
        return Envelope::err(INTERNAL);
    }
    *next_session_id += 1;
    Envelope::ok(ScanStartResp {
        session_id: *next_session_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(scope: &str, target: Option<&str>, drives: Option<Vec<&str>>) -> ScanStartReq {
        ScanStartReq {
            scope: scope.to_string(),
            target_drive: target.map(|s| s.to_string()),
            drives: drives.map(|v| v.into_iter().map(|s| s.to_string()).collect()),
        }
    }

    #[test]
    fn no_drives_means_all() {
        // 都不传 = 全部固定盘（`None` 交给编排层去枚举）
        assert_eq!(resolve_drives(&req("full", None, None)), Ok(None));
    }

    #[test]
    fn parses_both_forms_and_normalizes_case() {
        // "C" / "C:" / "c" / "c:" 四种写法都要认，且统一成大写
        for raw in ["C", "C:", "c", "c:"] {
            assert_eq!(
                resolve_drives(&req("full", None, Some(vec![raw]))),
                Ok(Some(vec!['C'])),
                "{raw:?} 应解析成 'C'"
            );
        }
    }

    #[test]
    fn dedups_after_normalization() {
        // ["C", "C:"] 是同一个盘，不能变成两条（否则 per_drive 里出现两条 C:）
        assert_eq!(
            resolve_drives(&req("full", None, Some(vec!["C", "C:"]))),
            Ok(Some(vec!['C']))
        );
        assert_eq!(
            resolve_drives(&req("full", None, Some(vec!["c", "D", "d:"]))),
            Ok(Some(vec!['C', 'D']))
        );
    }

    #[test]
    fn rejects_empty_drive_list() {
        // 空数组必须报错：静默当成「全部」会让「只想扫 D」变成「扫了全部」
        assert_eq!(
            resolve_drives(&req("full", None, Some(vec![]))),
            Err(BadDriveArg::EmptyList)
        );
    }

    #[test]
    fn rejects_malformed_drives() {
        // 任何一项非法 → 整体失败（不跳过坏项，避免「以为扫了三个盘实际两个」）
        for bad in ["", ":", "CC", "1", "C:\\", "AB", "  "] {
            assert_eq!(
                resolve_drives(&req("full", None, Some(vec![bad]))),
                Err(BadDriveArg::Malformed),
                "{bad:?} 应判非法"
            );
        }
        // 好项 + 坏项混在一起也必须整体失败
        assert_eq!(
            resolve_drives(&req("full", None, Some(vec!["C", "1"]))),
            Err(BadDriveArg::Malformed)
        );
    }

    #[test]
    fn legacy_target_drive_still_works() {
        // task-004 的入参不能被 task-077 改坏：前端 Dashboard 现在还在传它
        assert_eq!(
            resolve_drives(&req("full", Some("D:"), None)),
            Ok(Some(vec!['D']))
        );
        assert_eq!(
            resolve_drives(&req("full", Some("CC"), None)),
            Err(BadDriveArg::Malformed)
        );
        // drives 优先于 target_drive
        assert_eq!(
            resolve_drives(&req("full", Some("D:"), Some(vec!["E"]))),
            Ok(Some(vec!['E']))
        );
    }

    #[test]
    fn handle_validates_scope_and_drives() {
        let mut next = 0;

        // 非法 scope → 错误信封，且**不能**递增 sessionId（否则前端拿到一个
        // 永远不会被填数据的会话 id）
        let bad_scope = handle_scan_start(req("bogus", None, None), &mut next);
        assert_ne!(bad_scope.code, 0, "非法 scope 应返回错误码");
        assert_eq!(next, 0, "非法 scope 不该递增 sessionId");

        let bad_drives = handle_scan_start(req("full", None, Some(vec!["1"])), &mut next);
        assert_ne!(bad_drives.code, 0, "非法 drives 应返回错误码");
        assert_eq!(next, 0, "非法 drives 不该递增 sessionId");

        let ok = handle_scan_start(req("full", None, Some(vec!["C", "D"])), &mut next);
        assert_eq!(ok.code, 0);
        assert_eq!(next, 1);
        assert_eq!(ok.data.unwrap().session_id, 1);
    }
}
