//! task-078：用户自定义规则的集成测试。
//!
//! # 为什么必须有一个「真扫描」的用例
//!
//! 任务书的验收标准第 18/19 条是**端到端**的：新增规则 → 扫描 → Items 出现；
//! 关闭规则 → 重新扫描 → 不再出现。
//!
//! 这条不能只靠 `user_rules.rs` 的 CRUD 单测证明——CRUD 通过只说明「规则存进去了」，
//! 不说明「扫描器真的会去扫它」。而本任务最容易被忽略的事实是：
//! **`rules/loader.rs` 在扫描链路上原本没有任何消费者**（它只服务 L3 外部红线与
//! 规则库更新比对）。所以如果只改 loader，规则存得再好也不会出现在清理列表里。
//!
//! `end_to_end_rule_produces_items_then_disabled_removes_them` 走完整链路：
//! 真 DB → 真规则行 → 真 `ScanOrchestrator::run_all` → 真 `cleanup_item` 行。
//!
//! # 为什么用临时目录当「缓存目录」
//!
//! 用户规则指向的是任意路径。测试要能断言「恰好扫到这几个文件」，就只能自己造目录。
//! `tempfile::tempdir()` 落在 `%TEMP%` 下（本机在 C 盘），盘符与真实场景一致。
//!
//! **注意**：内置规则里 `%TEMP%` 是内置模块的扫描目标，所以这个临时目录下的文件
//! 有可能被内置 `temp` 模块也扫到。为此断言时**必须按 `module_code = 'user_rules'`
//! 过滤**，否则会把内置模块的条目算进来，测试变成「内置模块有没有工作」。

use engine::db::conn::open;
use engine::ipc::commands::user_rules as ur;
use engine::ipc::progress::NoopEmitter;
use engine::scan::modules::user_rules::{scan_rules, UserRuleSpec};
use engine::scan::ScanOrchestrator;

// ===================== 辅助 =====================

/// 造一个临时库（真实文件，走完整迁移），返回 (tempdir, conn)。
fn db() -> (tempfile::TempDir, rusqlite::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open(dir.path()).unwrap();
    (dir, conn)
}

/// 插一条 scan_session，返回 id。
fn new_session(conn: &rusqlite::Connection) -> i64 {
    conn.execute(
        "INSERT INTO scan_session (started_at, status, scope) VALUES (unixepoch(), 'running', 'full')",
        [],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// 造一个「缓存目录」，返回 (tempdir, 该目录路径)。
fn cache_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("UserCache");
    std::fs::create_dir_all(cache.join("sub")).unwrap();
    std::fs::write(cache.join("a.tmp"), vec![b'x'; 128]).unwrap();
    std::fs::write(cache.join("b.tmp"), vec![b'x'; 256]).unwrap();
    std::fs::write(cache.join("keep.log"), vec![b'x'; 16]).unwrap();
    std::fs::write(cache.join("sub").join("c.tmp"), vec![b'x'; 512]).unwrap();
    (dir, cache)
}

/// 某个 session 下 `module_code='user_rules'` 的条目数。
fn user_rule_items(conn: &rusqlite::Connection, session_id: i64) -> i64 {
    conn.query_row(
        "SELECT count(*) FROM cleanup_item WHERE session_id = ?1 AND module_code = 'user_rules'",
        [session_id],
        |r| r.get(0),
    )
    .unwrap()
}

// ===================== ① v4 迁移 =====================

#[test]
fn v4_creates_user_rules_table_with_expected_columns() {
    let (_d, conn) = db();
    let v: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert!(v >= 4, "迁移应至少到 v4（user_rules）；当前 v{v}");

    let mut stmt = conn.prepare("PRAGMA table_xinfo(user_rules)").unwrap();
    let cols: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    for want in [
        "id",
        "name",
        "pattern",
        "risk_level",
        "enabled",
        "created_at",
        "updated_at",
    ] {
        assert!(cols.iter().any(|c| c == want), "缺列 {want}: {cols:?}");
    }
}

// ===================== ② CRUD（真实文件库） =====================

#[test]
fn crud_on_real_db_file() {
    let (_d, conn) = db();

    let a = ur::add(&conn, "缓存A", r"D:\MyCache\*", "L1").unwrap();
    assert!(a.enabled);
    let b = ur::add(&conn, "日志B", r"D:\Logs\**\*.log", "L2").unwrap();

    let all = ur::list(&conn).unwrap();
    assert_eq!(all.len(), 2, "应按 id 升序返回两条: {all:?}");
    assert_eq!(all[0].id, a.id);
    assert_eq!(all[1].id, b.id);

    // 停用一条 → 扫描用列表里就没有它
    ur::set_enabled(&conn, b.id, false).unwrap();
    let enabled = ur::list_enabled(&conn).unwrap();
    assert_eq!(enabled.len(), 1);
    assert_eq!(enabled[0].id, a.id);

    // 更新
    let u = ur::update(&conn, a.id, "缓存A2", r"D:\MyCache2\*", "L2", true).unwrap();
    assert_eq!(u.name, "缓存A2");
    assert_eq!(u.risk_level, "L2");

    // 删除
    assert!(ur::delete(&conn, a.id).unwrap());
    assert_eq!(ur::list(&conn).unwrap().len(), 1);
}

#[test]
fn l3_rule_is_rejected_before_it_reaches_the_table() {
    let (_d, conn) = db();
    // 风险等级 L3：语义上「永不删除」，与「清理规则」自相矛盾
    assert!(ur::add(&conn, "x", r"D:\a\*", "L3").is_err());
    // 静态根落在红线里：建的时候就拒，不留「扫出来删不掉」的坑
    assert!(ur::add(&conn, "x", r"C:\Windows\System32\*", "L1").is_err());
    // 太宽泛：等于扫整盘
    assert!(ur::add(&conn, "x", r"D:\**", "L1").is_err());

    assert!(ur::list(&conn).unwrap().is_empty(), "被拒的规则不该落库");
}

// ===================== ③ 内置优先（合并规则） =====================

#[test]
fn builtin_rule_shadows_user_rule_on_same_path() {
    let (_g, cache) = cache_dir();
    let pattern = format!(r"{}\*.tmp", cache.to_string_lossy());

    let specs = vec![UserRuleSpec {
        id: 1,
        name: "测试".to_string(),
        pattern: pattern.clone(),
        risk_level: 1,
    }];

    // 无内置前缀 → 正常扫到 2 个 .tmp（a/b，本层）
    let plain = scan_rules(&specs, &[], None);
    assert_eq!(plain.len(), 2, "实际: {plain:#?}");

    // 把该目录登记成「内置已覆盖」→ 用户规则条目被抑制
    let shadowed = scan_rules(&specs, &[cache.to_string_lossy().to_lowercase()], None);
    assert!(shadowed.is_empty(), "内置优先：不该重复产出: {shadowed:#?}");
}

// ===================== ④ 端到端：规则 → 扫描 → 条目 =====================

#[test]
fn end_to_end_rule_produces_items_then_disabled_removes_them() {
    let (_dbdir, conn) = db();
    let (_cachedir, cache) = cache_dir();

    // 规则只匹配本层的 .tmp（a.tmp / b.tmp），不含 sub\c.tmp 与 keep.log
    let pattern = format!(r"{}\*.tmp", cache.to_string_lossy());
    let rule = ur::add(&conn, "端到端缓存", &pattern, "L1").unwrap();

    let empty_scripts = tempfile::tempdir().unwrap();

    // ---- 第一轮：规则启用 ----
    let sid1 = new_session(&conn);
    let s1 = ScanOrchestrator::new(sid1, &conn, &NoopEmitter)
        .run_all(empty_scripts.path())
        .expect("扫描应成功");
    let n1 = user_rule_items(&conn, sid1);
    assert!(n1 >= 2, "应至少扫到规则里的 2 个 .tmp，实际 {n1}");
    assert!(
        s1.scripts_succeeded.iter().any(|x| x == "user_rules"),
        "汇总里应出现 user_rules: {:?}",
        s1.scripts_succeeded
    );

    // 每一条都必须落在规则目录下，且风险等级取自规则
    let mut stmt = conn
        .prepare(
            "SELECT path, risk_level, category FROM cleanup_item
             WHERE session_id = ?1 AND module_code = 'user_rules'",
        )
        .unwrap();
    let rows: Vec<(String, String, String)> = stmt
        .query_map([sid1], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    let cache_prefix = cache.to_string_lossy().to_lowercase();
    for (path, risk, cat) in &rows {
        assert!(
            path.to_lowercase().starts_with(&cache_prefix),
            "条目越界: {path}"
        );
        assert_eq!(risk, "L1", "风险等级应取自规则");
        assert_eq!(cat, &format!("user_rule_{}", rule.id), "category 用规则 id");
    }

    // ---- 第二轮：停用规则 ----
    ur::set_enabled(&conn, rule.id, false).unwrap();
    let sid2 = new_session(&conn);
    ScanOrchestrator::new(sid2, &conn, &NoopEmitter)
        .run_all(empty_scripts.path())
        .expect("扫描应成功");
    assert_eq!(
        user_rule_items(&conn, sid2),
        0,
        "规则停用后不该再产出 user_rules 条目"
    );
}

#[test]
fn disabled_rule_is_not_scanned_even_if_it_would_match() {
    // 与上一条互补：这里不跑编排，直接验证「扫描器拿到的规则集」里没有停用项
    let (_dbdir, conn) = db();
    let (_cachedir, cache) = cache_dir();
    let pattern = format!(r"{}\*.tmp", cache.to_string_lossy());

    let r = ur::add(&conn, "临时", &pattern, "L1").unwrap();
    ur::set_enabled(&conn, r.id, false).unwrap();

    let enabled = ur::list_enabled(&conn).unwrap();
    assert!(enabled.is_empty(), "停用的规则不该出现在扫描列表里");

    // 用空规则集扫 → 0 条
    assert!(scan_rules(&[], &[], None).is_empty());
}
