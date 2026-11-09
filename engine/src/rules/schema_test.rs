//! task-083：matches schema / L3 反向断言 / 路径去重的单元测试。
//!
//! 全部是**纯函数测试**，不读注册表、不碰真实规则库、不需要管理员权限——
//! 这样在 CI 的 windows-latest runner 上也能稳定跑（runner 没有用户的
//! 真实 `%LOCALAPPDATA%` / 软件安装记录）。

use crate::rules::models::{CacheRules, RuleEntry};
use crate::rules::schema::{
    assert_no_l3_conflict_or_panic, compile_specs, dedup_hits, file_name, find_l3_conflicts,
    matches_file, MatchError, MatchSpec, RuleHit,
};
use std::collections::HashMap;
use std::path::Path;

/// 从仓库真实规则库加载。这条同时钉住三件事：
/// ① `rules_meta.json` 的 SHA-256 与 `cache_rules.json` 同步（忘改就启动失败）；
/// ② `matches` 三种类型能被 serde 读出来；
/// ③ 真实规则库通过 L3 反向断言。
#[test]
fn repository_rules_load_with_matches() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../rules");
    let loaded = crate::rules::loader::load(&dir)
        .expect("真实规则库必须能加载（检查 rules_meta.json 的 checksum 是否同步）");
    let entries = &loaded.rules.modules["A"];
    let demo = entries
        .iter()
        .find(|e| e.id == "download_transient")
        .expect("示例规则应存在");
    assert_eq!(demo.matches.len(), 3, "glob / regex / suffix 各一条");
    assert!(
        find_l3_conflicts(&loaded.rules).is_empty(),
        "真实规则库不该有 L3 冲突"
    );
    // 旧规则不带 matches 字段，加载后应为空数组而非报错
    let old = entries.iter().find(|e| e.id == "npm_cache").unwrap();
    assert!(old.matches.is_empty());
}

fn entry(id: &str, paths: &[&str]) -> RuleEntry {
    RuleEntry {
        id: id.to_string(),
        category: id.to_string(),
        display_name: id.to_string(),
        paths: paths.iter().map(|s| s.to_string()).collect(),
        risk_level: "L1".to_string(),
        description: String::new(),
        impact_note: String::new(),
        rebuild_note: String::new(),
        elevated: false,
        matches: Vec::new(),
    }
}

fn rules(entries: Vec<RuleEntry>, deny: &[&str]) -> CacheRules {
    let mut modules = HashMap::new();
    modules.insert("A".to_string(), entries);
    CacheRules {
        version: "1.0.0".to_string(),
        modules,
        whitelist_deny: deny.iter().map(|s| s.to_string()).collect(),
    }
}

fn hit(path: &str, id: &str, rule_path: &str, level: &str) -> RuleHit {
    RuleHit {
        path: path.to_string(),
        rule_id: id.to_string(),
        rule_path: rule_path.to_string(),
        risk_level: level.to_string(),
    }
}

// ===================== matches：glob =====================

#[test]
fn glob_matches_basename_case_insensitively() {
    let specs = [MatchSpec::Glob {
        pattern: "*.tmp".to_string(),
    }];
    let compiled = compile_specs("r", &specs).unwrap();
    assert!(matches_file(&compiled, r"C:\Temp\a.tmp"));
    assert!(
        matches_file(&compiled, r"C:\Temp\A.TMP"),
        "Windows 大小写不敏感"
    );
    assert!(
        matches_file(&compiled, "D:/data/nested/b.tmp"),
        "只看 basename"
    );
    assert!(!matches_file(&compiled, r"C:\Temp\a.log"));
    assert!(!matches_file(&compiled, r"C:\Temp\atmp"));
}

#[test]
fn glob_question_mark_and_literal_dot() {
    let specs = [MatchSpec::Glob {
        pattern: "cache_?.dat".to_string(),
    }];
    let compiled = compile_specs("r", &specs).unwrap();
    assert!(matches_file(&compiled, r"C:\x\cache_1.dat"));
    assert!(
        !matches_file(&compiled, r"C:\x\cache_12.dat"),
        "? 只匹配一个字符"
    );
    // `.` 必须被转义，否则 `cache_1Xdat` 这种也会被当成命中
    assert!(!matches_file(&compiled, r"C:\x\cache_1Xdat"));
}

#[test]
fn glob_double_star_matches_anything() {
    let specs = [MatchSpec::Glob {
        pattern: "**".to_string(),
    }];
    let compiled = compile_specs("r", &specs).unwrap();
    assert!(matches_file(&compiled, r"C:\x\anything.bin"));
    assert!(matches_file(&compiled, "README"));
}

// ===================== matches：suffix / regex / OR / 兼容 =====================

#[test]
fn suffix_is_case_insensitive() {
    let specs = [MatchSpec::Suffix {
        pattern: ".LOG".to_string(),
    }];
    let compiled = compile_specs("r", &specs).unwrap();
    assert!(matches_file(&compiled, r"C:\x\app.log"));
    assert!(matches_file(&compiled, r"C:\x\APP.LOG"));
    assert!(!matches_file(&compiled, r"C:\x\app.log.bak"));
}

#[test]
fn regex_matches_digit_suffixes() {
    let specs = [MatchSpec::Regex {
        pattern: r"^cache_[0-9]+\.bin$".to_string(),
    }];
    let compiled = compile_specs("r", &specs).unwrap();
    assert!(matches_file(&compiled, r"C:\x\cache_123.bin"));
    assert!(
        matches_file(&compiled, r"C:\x\CACHE_7.BIN"),
        "regex 也统一忽略大小写"
    );
    assert!(!matches_file(&compiled, r"C:\x\cache_abc.bin"));
    assert!(!matches_file(&compiled, r"C:\x\xcache_1.bin"));
}

#[test]
fn multiple_specs_are_or_not_and() {
    let specs = [
        MatchSpec::Glob {
            pattern: "*.tmp".to_string(),
        },
        MatchSpec::Suffix {
            pattern: ".log".to_string(),
        },
    ];
    let compiled = compile_specs("r", &specs).unwrap();
    assert!(matches_file(&compiled, r"C:\x\a.tmp"));
    assert!(matches_file(&compiled, r"C:\x\b.log"));
    assert!(!matches_file(&compiled, r"C:\x\c.bin"));
}

#[test]
fn empty_specs_keep_old_rules_working() {
    // 旧规则（无 matches 字段）不该被过滤掉任何东西
    assert!(matches_file(&[], r"C:\x\a.tmp"));
    assert!(matches_file(&[], r"C:\x\anything"));
}

#[test]
fn bad_regex_is_reported_with_rule_id_and_index() {
    let specs = [
        MatchSpec::Glob {
            pattern: "*.tmp".to_string(),
        },
        MatchSpec::Regex {
            pattern: "([".to_string(),
        },
    ];
    let err = compile_specs("my_rule", &specs).unwrap_err();
    match err {
        MatchError::BadRegex { rule_id, index, .. } => {
            assert_eq!(rule_id, "my_rule");
            assert_eq!(index, 1);
        }
    }
}

#[test]
fn deserialize_old_rule_without_matches_field() {
    // 这是 A1 最要紧的一条：存量规则 / 用户自定义规则 / 远程旧版规则库
    // 都没有 matches 字段，加字段后必须仍能反序列化。
    let json = r#"{
        "id":"old","category":"old","display_name":"旧规则",
        "paths":["C:\\Temp"],"risk_level":"L1","description":"",
        "impact_note":"","rebuild_note":"","elevated":false
    }"#;
    let entry: RuleEntry = serde_json::from_str(json).unwrap();
    assert!(entry.matches.is_empty(), "缺省应为空数组，不能报错");
}

#[test]
fn deserialize_matches_three_types() {
    let json = r#"[
        {"type":"glob","pattern":"*.tmp"},
        {"type":"regex","pattern":"^a[0-9]+$"},
        {"type":"suffix","pattern":".log"}
    ]"#;
    let specs: Vec<MatchSpec> = serde_json::from_str(json).unwrap();
    assert_eq!(specs.len(), 3);
    assert_eq!(
        specs[0],
        MatchSpec::Glob {
            pattern: "*.tmp".into()
        }
    );
    assert!(matches!(specs[1], MatchSpec::Regex { .. }));
    assert!(matches!(specs[2], MatchSpec::Suffix { .. }));
}

#[test]
fn file_name_handles_both_separators() {
    assert_eq!(file_name(r"C:\a\b\c.tmp"), "c.tmp");
    assert_eq!(file_name("D:/a/b/c.tmp"), "c.tmp");
    assert_eq!(file_name("just-a-name"), "just-a-name");
}

// ===================== L3 反向断言 =====================

#[test]
fn no_conflict_for_normal_rules() {
    let r = rules(
        vec![entry("npm_cache", &[r"D:\Caches\npm"])],
        &[r"C:\Windows\System32", r"C:\Windows\WinSxS"],
    );
    assert!(find_l3_conflicts(&r).is_empty());
}

#[test]
fn conflict_detected_when_l1_path_under_deny() {
    let r = rules(
        vec![
            entry("bad", &[r"C:\Windows\System32\drivers\etc"]),
            entry("good", &[r"D:\Caches\npm"]),
        ],
        &[r"C:\Windows\System32"],
    );
    let conflicts = find_l3_conflicts(&r);
    assert_eq!(conflicts.len(), 1, "只该报冲突的那一条");
    assert_eq!(conflicts[0].rule_id, "bad");
    assert_eq!(conflicts[0].path, r"C:\Windows\System32\drivers\etc");
}

#[test]
fn conflict_check_is_case_and_separator_insensitive() {
    // 大小写 / 分隔符差异不能成为绕过红线的口子
    let r = rules(
        vec![entry("bad", &["c:/windows/system32/temp"])],
        &[r"C:\Windows\System32"],
    );
    assert_eq!(find_l3_conflicts(&r).len(), 1);
}

#[test]
fn exact_deny_path_itself_is_also_a_conflict() {
    let r = rules(
        vec![entry("bad", &[r"C:\pagefile.sys"])],
        &[r"C:\pagefile.sys"],
    );
    assert_eq!(find_l3_conflicts(&r).len(), 1);
}

#[test]
#[should_panic(expected = "L3 红线反向断言失败")]
fn panic_lists_conflicting_rule_ids() {
    let r = rules(
        vec![entry("dangerous_rule", &[r"C:\Windows\System32\drivers"])],
        &[r"C:\Windows\System32"],
    );
    assert_no_l3_conflict_or_panic(&r);
}

#[test]
#[should_panic(expected = "dangerous_rule")]
fn panic_message_contains_the_rule_id() {
    let r = rules(
        vec![entry("dangerous_rule", &[r"C:\Windows\System32\drivers"])],
        &[r"C:\Windows\System32"],
    );
    assert_no_l3_conflict_or_panic(&r);
}

// ===================== 路径去重 =====================

#[test]
fn dedup_prefers_more_specific_rule_path() {
    // 同一文件被「宽规则」和「精确规则」同时命中，保留精确的那条
    let out = dedup_hits(vec![
        hit(r"C:\Temp\a.tmp", "wide", r"C:\Temp", "L1"),
        hit(r"C:\Temp\a.tmp", "specific", r"C:\Temp\a.tmp", "L2"),
    ]);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].rule_id, "specific");
}

#[test]
fn dedup_prefers_higher_level_when_equally_specific() {
    let out = dedup_hits(vec![
        hit(r"C:\Temp\a.tmp", "low", r"C:\Temp", "L1"),
        hit(r"C:\Temp\a.tmp", "high", r"C:\Temp", "L2"),
    ]);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].rule_id, "high");
}

#[test]
fn dedup_treats_case_and_separator_variants_as_same_file() {
    let out = dedup_hits(vec![
        hit(r"C:\Temp\A.TMP", "a", r"C:\Temp", "L1"),
        hit("c:/temp/a.tmp", "b", r"C:\Temp", "L1"),
    ]);
    assert_eq!(
        out.len(),
        1,
        "Windows 文件系统大小写不敏感，必须视为同一文件"
    );
}

#[test]
fn dedup_keeps_distinct_files_and_output_is_deterministic() {
    let out = dedup_hits(vec![
        hit(r"C:\Temp\z.tmp", "z", r"C:\Temp", "L1"),
        hit(r"C:\Temp\a.tmp", "a", r"C:\Temp", "L1"),
        hit(r"C:\Temp\m.tmp", "m", r"C:\Temp", "L1"),
    ]);
    assert_eq!(out.len(), 3);
    let ids: Vec<&str> = out.iter().map(|h| h.rule_id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["a", "m", "z"],
        "输出按路径排序，不能随 HashMap 顺序漂"
    );
}

#[test]
fn dedup_tie_breaks_by_rule_id() {
    // 路径长度、等级都相同时用 id 字典序兜底，保证结果唯一
    let out = dedup_hits(vec![
        hit(r"C:\Temp\a.tmp", "z_rule", r"C:\Temp", "L1"),
        hit(r"C:\Temp\a.tmp", "a_rule", r"C:\Temp", "L1"),
    ]);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].rule_id, "a_rule");
}

// ===================== 4. system 组根目录提取（A2a 快照） =====================

/// 测试内本地归一：分隔符统一 + 小写，仅用于路径比较（schema 里的 `norm_path`
/// 是私有的，测试走 `crate::rules::schema::` 路径访问不到）。
fn normp(s: &str) -> String {
    s.replace('\\', "/").to_lowercase().trim_end_matches('/').to_string()
}

/// 从仓库真实规则库读 `system` 组。这条顺带钉住：
/// ① `rules_meta.json` 的 SHA-256 与 `cache_rules.json` 同步（忘改就启动失败）；
/// ② 新增的 `system` 组能被 serde 读出来、通过 L3 反向断言与变量展开校验。
fn repository_rules() -> CacheRules {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../rules");
    crate::rules::loader::load(&dir)
        .expect("真实规则库必须能加载（检查 rules_meta.json 的 checksum 是否同步）")
        .rules
}

/// system 组必须覆盖 5 个内置模块的全部业务类别（A2a 快照）。
#[test]
fn system_group_covers_all_builtin_categories() {
    let rules = repository_rules();
    let sys = &rules.modules["system"];

    let mut categories: Vec<&str> = sys.iter().map(|e| e.category.as_str()).collect();
    categories.sort_unstable();
    categories.dedup();

    for cat in [
        "user_temp",
        "windows_temp",
        "prefetch",
        "npm_cache",
        "pip_cache",
        "pnpm_store",
        "update_download",
        "edge_cache",
        "chrome_cache",
        "firefox_cache",
    ] {
        assert!(
            categories.contains(&cat),
            "system 组缺少内置模块的 category={cat:?}，实际 {categories:?}"
        );
    }
}

/// system 组里不该有 A~F 组用户可见规则的 category（`npm_cache` 是唯一的合法
/// 跨组重复——A 组是用户可见清理规则，system 组是内置模块路径真值源，
/// 靠 `roots_for_categories` 按展开根去重兜住，不会重复扫）。
#[test]
fn system_group_does_not_overflow_user_categories() {
    let rules = repository_rules();
    let sys = &rules.modules["system"];
    let cats: Vec<&str> = sys.iter().map(|e| e.category.as_str()).collect();
    // 允许集合 = 5 个内置模块的类别
    for cat in cats {
        assert!(
            [
                "user_temp",
                "windows_temp",
                "prefetch",
                "npm_cache",
                "pip_cache",
                "pnpm_store",
                "update_download",
                "edge_cache",
                "chrome_cache",
                "firefox_cache",
            ]
            .contains(&cat),
            "system 组混入了非内置模块类别 {cat:?}"
        );
    }
}

/// windows_temp 永远是 `%SystemRoot%\Temp` + L2，不依赖 TEMP 是否设置（A2a 快照）。
#[test]
fn windows_temp_root_is_system_root_based_and_l2() {
    let rules = repository_rules();
    let roots = crate::rules::schema::roots_for_categories(&rules, &["windows_temp"]);
    assert_eq!(roots.len(), 1, "windows_temp 只有一个根: {roots:#?}");

    let sr =
        std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    let expect = format!(r"{sr}\Temp");
    assert_eq!(normp(&roots[0].root), normp(&expect));
    assert_eq!(roots[0].risk_level, 2, "windows_temp 必须 L2");
}

/// user_temp 解析出 `%TEMP%` 本身，且风险为 L1（A2a 快照）。
#[test]
fn user_temp_resolves_to_env_temp_l1() {
    let Ok(t) = std::env::var("TEMP") else {
        return; // 环境剥掉 TEMP 时跳过（兜底本来就不产生该类别）
    };
    let rules = repository_rules();
    let roots = crate::rules::schema::roots_for_categories(&rules, &["user_temp"]);
    assert!(
        roots.iter().any(|r| normp(&r.root) == normp(&t) && r.risk_level == 1),
        "user_temp 应解析为 TEMP({t}) / L1，实际 {roots:#?}"
    );
}

/// dev_cache 的 npm 新旧两个位置 + pip + pnpm 都能从 `system` 组取到（A2a 快照）。
#[test]
fn dev_cache_roots_match_env_fallback() {
    let rules = repository_rules();
    let roots = crate::rules::schema::roots_for_categories(
        &rules,
        &["npm_cache", "pip_cache", "pnpm_store"],
    );
    assert_eq!(roots.len(), 4, "npm 新+旧、pip、pnpm 共 4 个根: {roots:#?}");

    let lad =
        std::env::var("LOCALAPPDATA").unwrap_or_else(|_| r"C:\Users\test\AppData\Local".into());
    let ad = std::env::var("APPDATA").unwrap_or_else(|_| r"C:\Users\test\AppData\Roaming".into());

    let expected = [
        (format!(r"{lad}\npm-cache"), "npm_cache", 1),
        (format!(r"{ad}\npm-cache"), "npm_cache", 1),
        (format!(r"{lad}\pip\cache"), "pip_cache", 1),
        (format!(r"{lad}\pnpm-store"), "pnpm_store", 1),
    ];
    for (exp_root, exp_cat, exp_risk) in expected {
        assert!(
            roots.iter().any(|r| {
                normp(&r.root) == normp(&exp_root)
                    && r.category == exp_cat
                    && r.risk_level == exp_risk
            }),
            "缺少 {exp_cat}={exp_root} / risk {exp_risk}，实际 {roots:#?}"
        );
    }
}

/// prefetch / windows_update 各自只暴露一个根，且基于 `%SystemRoot%`（A2a 快照）。
#[test]
fn prefetch_and_update_roots_are_system_root_based() {
    let rules = repository_rules();
    let sr =
        std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());

    let prefetch = crate::rules::schema::roots_for_categories(&rules, &["prefetch"]);
    assert_eq!(prefetch.len(), 1, "prefetch 一个根: {prefetch:#?}");
    assert_eq!(
        normp(&prefetch[0].root),
        normp(&format!(r"{sr}\Prefetch"))
    );
    assert_eq!(prefetch[0].risk_level, 1, "prefetch 是 L1（自动重建）");

    let update = crate::rules::schema::roots_for_categories(&rules, &["update_download"]);
    assert_eq!(update.len(), 1, "update_download 一个根: {update:#?}");
    assert_eq!(
        normp(&update[0].root),
        normp(&format!(r"{sr}\SoftwareDistribution\Download"))
    );
    assert_eq!(update[0].risk_level, 2, "更新缓存必须 L2");
}

/// browser_cache 三条类别（edge/chrome/firefox）都基于 `%LOCALAPPDATA%`/`%APPDATA%`
/// 的锚目录，风险全 L1（A2b 快照）。这里的根是 **profile 锚**（`User Data` /
/// `Profiles`），多 profile 枚举由 `profiles.rs` 在模块内部展开，规则库只维护一份锚。
#[test]
fn browser_cache_anchors_resolve_to_local_or_roaming_l1() {
    let rules = repository_rules();
    let lad =
        std::env::var("LOCALAPPDATA").unwrap_or_else(|_| r"C:\Users\test\AppData\Local".into());
    let ad =
        std::env::var("APPDATA").unwrap_or_else(|_| r"C:\Users\test\AppData\Roaming".into());

    let roots = crate::rules::schema::roots_for_categories(
        &rules,
        &["edge_cache", "chrome_cache", "firefox_cache"],
    );
    let expected = [
        (format!(r"{lad}\Microsoft\Edge\User Data"), "edge_cache", 1),
        (format!(r"{lad}\Google\Chrome\User Data"), "chrome_cache", 1),
        (format!(r"{ad}\Mozilla\Firefox"), "firefox_cache", 1),
    ];
    assert_eq!(roots.len(), expected.len(), "三条浏览器锚: {roots:#?}");
    for (exp_root, exp_cat, exp_risk) in expected {
        assert!(
            roots.iter().any(|r| {
                normp(&r.root) == normp(&exp_root)
                    && r.category == exp_cat
                    && r.risk_level == exp_risk
            }),
            "缺少 {exp_cat}={exp_root} / risk {exp_risk}，实际 {roots:#?}"
        );
    }
}

