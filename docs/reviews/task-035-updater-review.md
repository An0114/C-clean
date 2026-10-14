# 测试评审报告：task-035 tauri updater 自动更新

## 本次任务名称
task-035：tauri updater 自动更新——签名密钥对 + update endpoint + 前端「检查应用更新」卡片

## Git 提交记录核对结果

✅ 9 个提交均符合原子提交规范，全部关联 `#task-035`：

| commit | type | 内容 |
|--------|------|------|
| `6ead477` | docs(task) | 新增任务035文档 |
| `72e2571` | chore(tauri) | 接入 tauri-plugin-updater Rust 插件 |
| `7a2a4bf` | chore(tauri) | tauri.conf.json 加 updater 配置（pubkey + endpoint） |
| `1023a02` | feat(frontend) | Settings 加「应用更新」卡片 |
| `28c1f1a` | ci | release.yml 加签名 key + latest.json |
| `4f7eb0a` | fix(tauri) | updater 配置移到 plugins.updater |
| `1857401` | docs(state) | 任务035完成 |
| `61b8a2e` | docs(task) | 回填实现备注 |
| `9051145` | docs(state) | 评审前清障完成 |

无私钥泄漏（`git log -p` grep `PRIVATE KEY` 无结果）。✅

## 分步验证清单（每条必须有运行时证据）

| # | 验收标准 | 代码走查 | 运行时现象 | 结论 |
|---|---------|---------|-----------|------|
| 1 | `tauri.conf.json` 配 `plugins.updater`（active/endpoints/pubkey） | ✅ | `plugins.updater.active=true`，endpoint 指向 `raw.githubusercontent.com/.../main/latest.json`，pubkey 为公钥（base64） | 通过 |
| 2 | Rust 侧接入 `tauri-plugin-updater` | ✅ | `Cargo.toml` 加 `tauri-plugin-updater = "2"`，`lib.rs` 加 `.plugin(tauri_plugin_updater::Builder::new().build())` | 通过 |
| 3 | 前端 `pnpm add @tauri-apps/plugin-updater + plugin-process` | ✅ | `app/package.json` 有依赖，`Settings.tsx` import 了 check/relaunch | 通过 |
| 4 | Settings 页加「应用更新」卡片 | ✅ | 真机渲染：规则库卡片下方有「应用更新」标题 + 更新状态行 + 「检查应用更新」按钮 | 通过 |
| 5 | 点「检查应用更新」能调 check() | ❌ | 点按钮后「更新状态」红字：**`Command plugin:updater\|check not allowed by ACL`**。updater 命令被 Tauri ACL 拒绝，功能完全不可用 | **不通过** |
| 6 | 私钥不入库 | ✅ | `git log -p` grep `PRIVATE KEY` 无结果；私钥在 `~/.tauri/cclean.key` | 通过 |
| 7 | release.yml 加签名 key + latest.json 生成 | ✅ | Setup signing key 步骤写私钥到 `~/.tauri/cclean.key`；Generate latest.json 步骤读 `.msi.sig` 生成 JSON；release action 上传 latest.json | 通过 |
| 8 | NSIS 打包成功 | ✅ | `pnpm tauri build` 产出 NSIS 3.97 MiB + MSI 5.73 MiB，无编译错误 | 通过 |
| 9 | 安装后 Dashboard/清理/保险箱/历史 不白屏不崩 | ✅ | 应用启动正常，5 tab 可切换，无白屏 | 通过 |
| 10 | Console 零红色 error | ✅ | CDP 收集：零 exception、零 console.error（ACL 拒绝是业务返回值，不是 JS exception） | 通过 |

## 代码评审发现问题

### Bug-1（P0）：Tauri ACL 未放行 updater 插件命令

`app/src-tauri/capabilities/default.json` 的 permissions 只有 `core:default` + `core:event:default`，没有加 `updater:default`（或 `updater:allow-check` + `updater:allow-download-and-install`）。Tauri 2 的权限系统在运行时直接拒绝 `plugin:updater|check` 命令，前端 catch 后显示「Command plugin:updater|check not allowed by ACL」。

## 安全风险检查结果

- **OWASP A01**：updater 用 minisign 公钥验签，安装包签名验证正确。✅
- **私钥管理**：私钥不入库，通过 GitHub Secret 注入 CI。✅
- **endpoint**：HTTPS raw URL，无中间人风险（公钥验签兜底）。✅
- 无硬编码密钥、无 SQL 注入。✅

## 评审结论：**不通过**

updater 核心功能（点「检查应用更新」）被 Tauri ACL 直接拒绝，用户完全无法使用自动更新。这是 P0 阻塞性 bug。

## Bug 清单 + 修复建议

### Bug-1（P0）：updater ACL 权限缺失

- **现象**：Settings 页点「检查应用更新」，「更新状态」显示 `Command plugin:updater|check not allowed by ACL`。
- **复现步骤**：
  1. 安装 NSIS 包并启动
  2. 切到「设置」tab
  3. 滚到「应用更新」卡片，点「检查应用更新」
  4. 状态行红字显示 ACL 拒绝错误
- **期望行为**：`check()` 正常发起 HTTP 请求拉 latest.json；网络不通时报网络错；无更新时显示「已是最新版本」。
- **修复建议**：在 `app/src-tauri/capabilities/default.json` 的 `permissions` 数组加 `"updater:default"`（或精确加 `"updater:allow-check"` + `"updater:allow-download-and-install"` + `"process:allow-relaunch"`）。`@tauri-apps/plugin-process` 的 relaunch 也需要对应权限。
- **严重度**：P0（功能完全不可用）

## Git 操作记录

- **未合并**，`feature/task-035-tauri-updater` 分支保留待修复。
- Revert 记录：无。

## 后续动作建议

1. 新建 bugfix/task-035-updater-acl：在 capabilities/default.json 补 updater + process 权限。
2. 修复后重跑 NSIS 真机冒烟：点「检查应用更新」应能拉 latest.json（当前 raw URL 是占位，会报 404 或校验错，但不应是 ACL 拒绝）。
3. 首次 v0.1.1 发版后再端到端测一次「装旧版 → 检查更新 → 自动装新版」。
