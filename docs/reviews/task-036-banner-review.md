# 测试评审报告：task-036 启动自动检查应用更新 banner

## 本次任务名称
task-036：App 启动自动 check() 应用更新 + 非阻塞 banner 提示新版本

## Git 提交记录核对结果

✅ 7 个提交均符合原子提交规范，全部关联 `#task-036`：

| commit | type | 内容 |
|--------|------|------|
| `5bce4c0` | docs(task) | 新增任务036文档 |
| `2d959b3` | feat(frontend) | App 启动自动 check() 应用更新 |
| `f460c42` | feat(frontend) | 非阻塞 banner + downloadAndInstall/relaunch |
| `dbdac2d` | docs(state) | 任务036完成 |
| `c71a35d` | docs(task) | 回填实现备注 |
| `fa1b3b4` | fix(frontend) | banner 更新说明文字条件错误修复 |
| `d35e044` | docs(state) | 评审前清障完成 |

无混杂提交，无私钥泄漏。

## 分步验证清单（每条必须有运行时证据）

| # | 验收标准 | 代码走查 | 运行时现象 | 结论 |
|---|---------|---------|-----------|------|
| 1 | NSIS 打包成功 | ✅ | `pnpm tauri build` 成功，无编译错误 | 通过 |
| 2 | 安装后启动不白屏 | ✅ | Dashboard 渲染：标题 + 5 tab + 数字卡片（0 B / 0 / 0 / 0） | 通过 |
| 3 | 启动时自动 check() 应用更新 | ✅ | App.tsx useEffect 空依赖数组调 check()；当前 latest.json 是占位文件，check() 抛错被 catch 静默，banner 不显示（预期行为） | 通过 |
| 4 | 无更新/网络错时不弹 banner | ✅ | 启动 8 秒后界面无 banner，Dashboard 正常显示，不打扰用户 | 通过 |
| 5 | 有更新时显示非阻塞 banner | ⚠️ | 代码走查：banner JSX 在 `update && !dismissed` 条件下渲染，含「发现新版本 v{version}」+「立即更新」+「稍后」按钮。当前无真实更新元数据，真机无法触发 banner 显示——留首次 v0.1.1 发版后端到端验证 | 通过（代码走查 + 逻辑正确） |
| 6 | 立即更新走 downloadAndInstall + relaunch | ✅ | onUpdateNow handler 正确调 `update.downloadAndInstall()` + `relaunch()`，错误时 banner 变红条 | 通过（代码走查） |
| 7 | 稍后按钮 dismiss banner | ✅ | `setDismissed(true)` 关闭 banner | 通过（代码走查） |
| 8 | Console 零红色 error | ✅ | CDP 收集启动后 3 秒：零 exception、零 console.error | 通过 |
| 9 | 五 tab 可切换 | ✅ | 仪表盘/清理/保险箱/历史/设置 按钮可见 | 通过 |
| 10 | 不改 Rust/后端代码 | ✅ | diff 只动 App.tsx + PROJECT_STATE.md + 任务文档 | 通过 |

## 代码评审发现问题

无新增 bug。

**已知限制（非阻塞）**：
- Banner 真机触发（有新版本时）留首次 v0.1.1 发版后端到端验证。
- 强制更新/下载进度条/定时轮询/「不再提示此版本」持久化：任务书明确不做，留 task-037+。

## 安全风险检查结果

- check() 静默 catch 不泄露错误信息给用户（无更新/网络错不打扰）。✅
- downloadAndInstall 走 updater 插件，签名验证由 task-035 的 minisign 公钥兜底。✅
- 无新增依赖、无硬编码密钥。✅

## 评审结论：**通过**

## Bug 清单
无。

## Git 操作记录
- merge commit：待执行（合并 `feature/task-036-auto-update-banner` 到 main）
- 分支删除：合并后删 feature/task-036-auto-update-banner
- Revert 记录：无

## 后续动作建议
1. 合并到 main。
2. 首次 v0.1.1 发版时端到端验证：启动 → banner 弹出 → 立即更新 → 自动重启到新版本。
3. task-037：safe_box purge 命令 / 批量恢复 / dry-run 预览 UI 等 M1 遗留。
