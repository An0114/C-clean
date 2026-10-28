//! task-056 单元 3：跨盘 rename fallback 集成测试（task-016 遗留 B 低）。
//!
//! 背景：`SafeBoxDeleter::delete_file` 的 `fs::rename` 在跨盘时返回
//! `ERROR_NOT_SAME_DEVICE`(17)，代码走 `copy + remove_file` fallback。CI 的
//! windows-latest 只有 C 盘，这条路径**从未被实测过**——真机上从 D 盘删文件
//! 进保险箱（保险箱在 C 盘 `%APPDATA%`）恰好走的就是它。
//!
//! 设计要点：
//! - **单盘环境必须 skip 而不是失败**：CI 只有一个盘，硬断言会让 CI 永久红；
//! - skip 用「打印一行 `[skip]` + 直接 return」，`cargo test` 显示为 passed
//!   （不是 `#[ignore]`，因为本地多盘时要真跑，不能靠人记得加 `--ignored`）；
//! - 本机有 C:/D: 两个盘，所以本地跑会真的验证 fallback。

#![cfg(windows)]

use std::path::{Path, PathBuf};

use engine::db::conn::open;
use engine::execute::delete::Deleter;
use engine::execute::normalize::normalize;
use engine::execute::safe_box::SafeBoxDeleter;

/// 取路径的盘符（大写）。非 `X:\` 形式返回 None。
fn drive_of(p: &Path) -> Option<char> {
    let s = p.to_string_lossy();
    let mut it = s.chars();
    let c = it.next()?.to_ascii_uppercase();
    if c.is_ascii_alphabetic() && it.next() == Some(':') {
        Some(c)
    } else {
        None
    }
}

/// 收集「可写」的候选目录，按盘符去重。
///
/// 优先 `std::env::temp_dir()`（一定可写），再逐个盘根试探
/// （`tempdir_in` 失败即跳过，普通用户对 `C:\` 根通常无写权限）。
fn candidate_dirs() -> Vec<(char, PathBuf)> {
    let mut out: Vec<(char, PathBuf)> = Vec::new();

    let mut push = |p: PathBuf| {
        if let Some(d) = drive_of(&p) {
            if !out.iter().any(|(c, _)| *c == d) {
                out.push((d, p));
            }
        }
    };

    push(std::env::temp_dir());
    for c in 'C'..='Z' {
        let root = PathBuf::from(format!("{c}:\\"));
        if root.exists() {
            push(root);
        }
    }
    out
}

/// 在两个**不同盘**上各建一个临时目录；找不到就返回 None（单盘环境）。
fn two_tempdirs_on_different_drives() -> Option<(tempfile::TempDir, tempfile::TempDir)> {
    let dirs = candidate_dirs();

    for (drive_a, base_a) in &dirs {
        let Ok(td_a) = tempfile::Builder::new()
            .prefix("cclean-xdisk-a-")
            .tempdir_in(base_a)
            .or_else(|_| {
                // 盘根不可写时退到该盘的 Public / Users 目录再试一次
                tempfile::Builder::new()
                    .prefix("cclean-xdisk-a-")
                    .tempdir_in(base_a.join("Users").join("Public"))
            })
        else {
            continue;
        };

        for (drive_b, base_b) in &dirs {
            if drive_b == drive_a {
                continue;
            }
            if let Ok(td_b) = tempfile::Builder::new()
                .prefix("cclean-xdisk-b-")
                .tempdir_in(base_b)
                .or_else(|_| {
                    tempfile::Builder::new()
                        .prefix("cclean-xdisk-b-")
                        .tempdir_in(base_b.join("Users").join("Public"))
                })
            {
                return Some((td_a, td_b));
            }
        }
    }
    None
}

/// 跨盘把文件移入保险箱：必须走 copy+remove fallback，且内容逐字节一致。
#[test]
fn cross_disk_move_into_box_falls_back_to_copy_remove() {
    let Some((src_root, box_root)) = two_tempdirs_on_different_drives() else {
        eprintln!("[skip] 只有单盘环境，跨盘 rename fallback 无法验证（CI 属预期）");
        return;
    };

    // 数据库放 src_root（盘 A），保险箱目录放 box_root（盘 B）——与真实场景一致：
    // 用户从 D 盘删文件，保险箱在 C 盘 %APPDATA%。
    let conn = open(src_root.path()).expect("open db");
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (1000, 'running', 'full')",
        [],
    )
    .expect("insert session");

    let box_dir = box_root.path().join("safe_box");
    let src = src_root.path().join("cross_disk_payload.bin");
    let content: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
    std::fs::write(&src, &content).expect("write src");

    let deleter = SafeBoxDeleter::new(&box_dir, &conn, None);
    let np = normalize(src.to_str().expect("utf8 path")).expect("normalize");

    // 前提校验：源与目标是不同盘，否则这个用例没意义
    assert_ne!(
        drive_of(&src).expect("src drive"),
        drive_of(&box_dir).expect("box drive"),
        "源与保险箱必须在不同盘，否则走不到 fallback"
    );

    let moved = deleter
        .delete_file(&np)
        .expect("cross-disk move must succeed");
    assert!(moved, "跨盘移入应返回 true（真删/真移）");

    // 源文件消失
    assert!(!src.exists(), "源文件应从原盘删除");

    // box 里内容逐字节一致（copy 阶段没截断）
    let rel: String = conn
        .query_row(
            "SELECT safe_rel_path FROM safe_box ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .expect("read safe_rel_path");
    let boxed = std::fs::read(box_dir.join(&rel)).expect("read boxed file");
    assert_eq!(boxed.len(), content.len(), "跨盘 copy 后大小必须一致");
    assert_eq!(boxed, content, "跨盘 copy 后内容必须逐字节一致");

    // DB 行也应正确落库
    let (status, size): (String, i64) = conn
        .query_row(
            "SELECT status, size_bytes FROM safe_box ORDER BY id DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("read row");
    assert_eq!(status, "in_box");
    assert_eq!(size, content.len() as i64);
}

/// 跨盘恢复的可移植契约：`restore_from_safe_box` 把 box 文件 rename 回 original_path。
///
/// Windows 上 `std::fs::rename`（MoveFileExW）跨卷行为**因环境而异**：
/// - GitHub Actions Windows runner（多数机器）：跨卷 rename 直接返 EXDEV（os error 17，
///   「The system cannot move the file to a different disk drive」）——这是开发轮原记录的
///   已知限制（restore 无 copy+remove fallback）。
/// - 某些本机配置：跨卷 rename 由 OS 层 copy+remove 兜住，restore 成功。
///
/// 本用例不锁死单一结果，而是验**两条路径都不丢数据、状态自洽**：
/// - Ok：目标文件落盘且内容逐字节一致、box 文件已移走、行 status='restored' + restored_at 非空；
/// - Err（跨卷 EXDEV）：行仍是 in_box、box 文件原封未动（数据不丢）、错误信息指向跨盘 rename。
#[test]
fn cross_disk_restore_portable_contract() {
    let Some((src_root, box_root)) = two_tempdirs_on_different_drives() else {
        eprintln!("[skip] 只有单盘环境，跨盘 restore 无法验证（CI 属预期）");
        return;
    };

    let conn = open(src_root.path()).expect("open db");
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (1000, 'running', 'full')",
        [],
    )
    .expect("insert session");

    let box_dir = box_root.path().join("safe_box");
    std::fs::create_dir_all(&box_dir).expect("create box_dir");

    let rel = "cross_restore.bin";
    let content: Vec<u8> = (0..128u32).map(|i| (i % 251) as u8).collect();
    std::fs::write(box_dir.join(rel), &content).expect("write boxed file");

    // 原路径在另一个盘
    let target = src_root.path().join("restore_target.bin");
    conn.execute(
        "INSERT INTO safe_box (original_path, safe_rel_path, size_bytes, quarantined_at, expires_at, status)
         VALUES (?1, ?2, 128, 1000, 87400, 'in_box')",
        rusqlite::params![target.to_string_lossy().to_string(), rel],
    )
    .expect("insert row");
    let id = conn.last_insert_rowid();

    assert_ne!(
        drive_of(&box_dir).expect("box drive"),
        drive_of(&target).expect("target drive"),
        "保险箱与恢复目标必须在不同盘"
    );

    let res = engine::execute::safe_box::restore_from_safe_box(&conn, &box_dir, id);

    match res {
        Ok(out) => {
            // 路径 A：跨卷 rename 在本环境成功（OS 层 copy+remove 兜住）。
            assert_eq!(out.safe_box_id, id);
            assert!(target.exists(), "恢复后目标文件应存在");
            let restored = std::fs::read(&target).expect("read restored file");
            assert_eq!(restored, content, "跨盘恢复后内容必须逐字节一致");
            assert!(!box_dir.join(rel).exists(), "恢复后 box 文件应已移走");

            let (status, restored_at): (String, Option<i64>) = conn
                .query_row(
                    "SELECT status, restored_at FROM safe_box WHERE id=?1",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .expect("read row");
            assert_eq!(status, "restored", "恢复成功后状态应为 restored");
            assert!(
                restored_at.is_some_and(|t| t > 0),
                "restored_at 应写入 unix 秒"
            );
        }
        Err(e) => {
            // 路径 B：跨卷 rename 返 EXDEV（多数 Windows runner 的已知限制）。
            // 必须保证：行仍 in_box、box 文件原封未动、错误指向跨盘 rename。
            let msg = format!("{e:?}");
            assert!(
                msg.contains("different disk drive")
                    || msg.contains("os error 17")
                    || msg.contains("rename"),
                "跨盘 restore 失败应报 IO/rename 错，实际: {msg}"
            );

            let (status, restored_at): (String, Option<i64>) = conn
                .query_row(
                    "SELECT status, restored_at FROM safe_box WHERE id=?1",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .expect("read row");
            assert_eq!(status, "in_box", "恢复失败不应改状态");
            assert!(restored_at.is_none(), "恢复失败不应写 restored_at");
            assert!(
                box_dir.join(rel).exists(),
                "恢复失败后 box 文件必须还在（数据不丢）"
            );
        }
    }
}
