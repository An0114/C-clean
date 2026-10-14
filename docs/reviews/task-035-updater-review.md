# 测试评审报告：task-035 tauri updater 自动更新（bugfix ACL 后第二轮）

## 本次任务名称
task-035：tauri updater 自动更新——签名密钥对 + update endpoint + 前端「检查应用更新」卡片（bugfix/task-035-updater-acl 修复后）

## Git 提交记录核对结果

✅ 本轮 bugfix 3 个提交均符合原子提交规范，关联 `#task-035`：

| commit | type | 内容 |
|--------|------|------|
| `f3086b4` | fix(tauri) | 放行 updater/process 插件 ACL + 注册 process Rust 插件 |
| `5d93bb7` | test(frontend) | 补 capabilities updater/process ACL 回归测试 |
| `22b759c` | docs(state) | 修复完成记录 |

加上 task-035 原有的 9 个提交，共 12 个提交全部关联 `#task-035`，无私钥泄漏。

## 分步验证清单（每条必须有运行时证据）

| # | 验收标准 | 代码走查 | 运行时现象 | 结论 |
|---|---------|---------|-----------|------|
| 1 | NSIS 打包成功 | ✅ | `pnpm tauri build` 产出 NSIS 3.97 MiB + MSI 5.73 MiB | 通过 |
| 2 | 安装后 Dashboard 不白屏 | ✅ | 5 tab 渲染正常，无白屏 | 通过 |
| 3 | capabilities 放行 updater/process | ✅ | `default.json` permissions 加了 `updater:default` + `process:default` | 通过 |
| 4 | Rust 侧注册 process 插件 | ✅ | `lib.rs` 加了 `.plugin(tauri_plugin_process::init())` | 通过 |
| 5 | Settings 页「应用更新」卡片渲染 | ✅ | 规则库卡片下方有「应用更新」标题 + 更新状态行 + 「检查应用更新」按钮 | 通过 |
| 6 | 点「检查应用更新」不报 ACL 错误 | ✅ | **不再报 `Command plugin:updater\|check not allowed by ACL`**。点后状态变「检查中...」，约 4 秒后返回 `Could not fetch a valid release JSON from the remote`（远端 latest.json 是占位文件，未真实签名，updater 正确识别为无效 JSON） | 通过 |
| 7 | 错误信息分类明确 | ✅ | updater 插件自己返回「Could not fetch a valid release JSON from the remote」，不是模糊的「内部错误」。首次发版后此错误会消失 | 通过 |
| 8 | 私钥不入库 | ✅ | `git log -p` grep `PRIVATE KEY` 无结果 | 通过 |
| 9 | release.yml 签名 + latest.json | ✅ | Setup signing key + Generate latest.json 步骤完整 | 通过 |
| 10 | Console 零红色 error | ✅ | CDP 收集：零 exception、零 console.error | 通过 |
| 11 | 五 tab 可切换不崩 | ✅ | 仪表盘/清理/保险箱/历史/设置 全部可切换 | 通过 |

## 代码评审发现问题

无新增 bug。上轮 P0（ACL 拒绝）已修复。

**已知限制（非阻塞）**：
- main 分支上的 `latest.json` 是占位文件，首次 v0.1.1 发版后才会有真实签名。当前点「检查应用更新」报 `Could not fetch a valid release JSON` 是预期行为。
- CI 自动把 latest.json push 回 main 留 task-036（实现备注已登记）。
- 端到端 updater 闭环（装 v0.1.0 → 检查更新到 v0.1.1）留首次发版验证。

## 安全风险检查结果

- **OWASP A01**：updater 用 minisign 公钥验签，安装包签名验证正确。✅
- **私钥管理**：私钥不入库，通过 GitHub Secret 注入 CI。✅
- **endpoint**：HTTPS raw URL，公钥验签兜底。✅
- 无硬编码密钥、无 SQL 注入。✅

## 评审结论：**通过**

## Bug 清单
无。

## Git 操作记录
- merge commit：待执行（合并 `bugfix/task-035-updater-acl` 到 main）
- feature/bugfix 分支合并后删除
- Revert 记录：无

## 后续动作建议
1. 合并到 main，触发首次 v0.1.1 发版验证 updater 端到端闭环。
2. task-036：CI 自动 push latest.json 到 main + 后台自动检查弹窗。
