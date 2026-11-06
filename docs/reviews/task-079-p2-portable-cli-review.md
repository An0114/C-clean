# 评审报告：task-079 P2 收官——便携版 + CLI 入口

> 评审角色：Review Lead
> 评审时间：2026-11-06
> 分支：`feature/task-079-p2-portable-cli`
> HEAD：`61d4c0f`
> **P2 收官：CLI 独立 binary（零依赖解析器 + 复用 engine）+ 4 子命令 + 便携版打包配置 + CLI 文档**

## 1. 输入核验

| 输入项 | 状态 | 事实核对 |
|--------|------|---------|
| 测试报告 | ✅ | `docs/test-reports/079-test-report.md`（103 行） |
| CI 运行号与结论 | ✅ | PR #44，run **42227589465**，conclusion **success** |
| feature 分支最新 commit | ✅ | `61d4c0f` |

## 2. Git 提交记录核对结果

全部关联 `#task-079`，type/scope 规范，无混杂提交，无敏感文件。

## 3. 验收覆盖性核验

| # | 验收标准 | 测试报告证据 | 抽查结果 | 结论 |
|---|---------|-------------|---------|------|
| 1-3 | CLI 独立编译 | cargo run -p c-clean-cli 直接跑通 + 12 条单测 | 代码走查 | 通过 |
| 4-9 | 4 子命令 | 真机三命令全通（--help / list-rules / scan --format json） | 已核对 | 通过 |
| 10-13 | 便携版配置 | 独立 tauri.portable.conf.json + --config + pick_data_root 四档优先级 | 已核对 | 通过 |
| 14-15 | CLI 文档 | docs/CLI.md 已建 | 已核对 | 通过 |
| 16 | cargo test 全绿 | 632 passed | 已核对 | 通过 |

## 4. 代码评审发现问题

第二档七项偏差经评审确认合理：
1. **零依赖手写解析器而非 clap 4**：本项目一贯零依赖，参数形态只有 4 子命令 + 7 开关，手写 100 行且能直接单测。
2. **Tauri 2 没有 bundle.windows.portable**：bundle target 只有 nsis/msi/app/dmg/deb/rpm/appimage/updater；便携场景的官方机制是 app.appDirectoriesOverride。
3. **不改 src-tauri/tauri.conf.json 加便携配置**：改用独立 tauri.portable.conf.json + --config，避免安装版 Program Files 只读写库失败。
4. **便携版不是单 exe**：还要 rules/（外部 L3 红线）、scripts/（ps1 回退）、cclean-elevation.exe（还原点/DISM）。便携版是一个文件夹。
5. **单元 1（binary）与单元 2（子命令）合成一次提交**：CLI 只有一个 main.rs，「只有 binary 骨架、没有子命令」的中间态是不可运行的死代码。
6. **CI 额外 cargo build --release -p c-clean-cli 并单独上传**：脚本用户不想为拿一个 CLI 去解压整个便携包。
7. **白名单外 2 个文件必改**：tauri.portable.conf.json（新建）/ Cargo.lock（workspace 成员 cli）。

关键设计正确：数据目录四档优先级（--data-dir > CCLEAN_DATA_DIR > portable_data > %APPDATA%）；--yes / --include-l2 只能配 clean（配错直接报用法错误退出码 2）；portable_data/ 放 README.txt 避免 Compress-Archive 跳空目录。

## 5. 安全风险检查结果

无新增依赖。clean 默认 dry-run（最安全默认）；--yes 才真实删除。✅

## 6. 状态与文档一致性

- PROJECT_STATE.md 已更新 ✅
- GUI dev 真机未起（无前端改动）登记第三档 ✅
- CI run 42227589465 绿 ✅

## 7. 评审结论：**通过**

CI 绿、L1/L2 全绿、632 passed / 197 vitest。第二档七项偏差经评审确认为合理工程取舍。**P2 全部完成！**

## 8. Bug 清单
无。

## 9. Git 操作记录

- 合并 feature/task-079-p2-portable-cli → main（merge --no-ff）
- 删除 feature 分支

## 10. 后续动作建议

1. **P2 全部完成**。下一阶段：发版 v0.4.0 / UI 深色适配补完（Items / SafeBox / History）/ 规则导入导出。
2. clean --include-l2 不创建系统还原点（命令行做不了，已在输出里明确警告）。
3. 规则库 cache_rules.json 的 modules 段双份真值源问题（task-078 起遗留）建议发版前拍板。
4. CLI 一次最多处理 50 万条（翻页上限 1000 页 × 500 条）。
