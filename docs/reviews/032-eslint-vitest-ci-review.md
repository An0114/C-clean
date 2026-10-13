# 测试评审报告

## 本次任务名称

032 前端工程化补齐——ESLint 10 flat config + Vitest 2.1 接入，CI 跑 lint/test

## Git 提交记录核对结果

✅ 8 个提交全部关联 `#task-032`，type 前缀规范（docs / chore(frontend) / test(frontend) / ci），无混杂提交。

| # | Commit | 类型 | 文件范围 | 原子性 |
|---|--------|------|----------|--------|
| 1 | `3ec1065` | docs(task) | tasks/032-eslint-vitest-ci.md | ✅ |
| 2 | `239eef0` | chore(frontend) | eslint.config.js + package.json + pnpm-lock.yaml | ✅ |
| 3 | `ddfe37f` | chore(frontend) | package.json + setupTests.ts + vite.config.ts + pnpm-lock.yaml | ✅ |
| 4 | `8557c4d` | test(frontend) | formatBytes.test.ts + formatTimestamp.test.ts | ✅ |
| 5 | `996a803` | ci | ci.yml | ✅ |
| 6 | `e155146` | docs(state) | PROJECT_STATE.md | ✅ |
| 7 | `173eac6` | docs(task) | tasks/032-eslint-vitest-ci.md | ✅ |
| 8 | `2afb689` | docs(state) | PROJECT_STATE.md | ✅ |

## 分步验证清单

### A. ESLint 接入

- ✅ devDeps：eslint@^10.12.0 + @eslint/js@^10 + @typescript-eslint/{parser,eslint-plugin}@^8.71 + eslint-plugin-react-hooks@^7.1 + eslint-plugin-react-refresh@^0.5.7。
- ✅ `app/eslint.config.js`（flat config，ESM）。
- ✅ rules：no-undef off（TS 类型检查）；@typescript-eslint/no-unused-vars warn（_ 前缀忽略）；react-hooks/rules-of-hooks error；react-hooks/exhaustive-deps warn；react-refresh/only-export-components warn。
- ✅ `package.json` scripts：`"lint": "eslint src --ext .ts,.tsx --max-warnings 10"`。

### B. Vitest 接入

- ✅ devDeps：vitest@^2.1.9 + @testing-library/{react,jest-dom}@^7 + jsdom@^30。
- ✅ `vite.config.ts` 加 test 字段：environment=jsdom + setupFiles=["./src/setupTests.ts"] + css=false。
- ✅ `src/setupTests.ts`：`import "@testing-library/jest-dom/vitest"`（子路径，非 `@testing-library/jest-dom`）。
- ✅ `formatBytes.test.ts`：0 / 500 / 2048 / 5MB / 2GB（5 个断言）。
- ✅ `formatTimestamp.test.ts`：0 / NaN / 负数 / 合法 unix 秒（4 个断言）。
- ✅ `package.json` scripts：`"test": "vitest run"`。

### C. CI 接线

- ✅ ci.yml 前端 job 加 `pnpm --filter app lint` + `pnpm --filter app test`。
- ✅ 删除原 echo 占位步骤。

### D. 卫生与边界

- ✅ 不改 `app/src/pages/**`、`app/src/App.tsx` 任何业务代码。
- ✅ 不改 Rust 代码。
- ✅ 不写组件测试（先基础设施）。
- ✅ 不接 Prettier / Husky。

### 自验命令复跑

| 命令 | 评审人复跑结果 |
|------|----------------|
| `pnpm --filter app lint` | ✅ exit 0 |
| `pnpm --filter app test` | ✅ exit 0（2 文件 9 断言全过：formatBytes 5 + formatTs 4） |
| `pnpm --filter app build` | ✅ exit 0（Vite + tsc 通过） |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ exit 0 |
| `cargo fmt --all -- --check` | ✅ exit 0 |
| `cargo test -p engine` | ✅ 137 passed / 0 failed（基线无回退，后端无改动） |

## 代码评审发现问题

### Bug

无。

### 技术债务（轻微）

1. **ESLint 10 flat config 替代 .eslintrc.cjs**：ESLint 10 已移除 eslintrc 格式，必须用 flat config `eslint.config.js`（ESM）。DevAgent 已适配。**第二档口径，已登记。**
2. **Vitest 降级到 2.1.9**：Vitest 5.0.3 要求 Vite 6+，本项目 Vite 5.4.11 不兼容（启动报 `ERR_PACKAGE_PATH_NOT_EXPORTED: ./module-runner`），降级到 vitest@^2.1.9（Vite 5 兼容的最后一个大版本）。**第二档口径，已登记。**
3. **setupTests.ts 子路径导入**：`@testing-library/jest-dom/vitest`（子路径），不能用 `@testing-library/jest-dom`——后者在 setup 阶段直接调 `expect.extend`，此时 vitest 的 expect 还没注入，报 `ReferenceError: expect is not defined`。DevAgent 已适配。**第二档口径，已登记。**
4. **no-undef off**：TS 已做类型检查，ESLint 关掉避免 `React.CSSProperties` / `HTMLElement` 等全局类型报 no-undef。**第二档口径，已登记。**
5. **pnpm-lock.yaml 在仓库根**：monorepo 根 lockfile，不是 app/pnpm-lock.yaml。**第二档口径，已登记。**
6. **无组件单测**：先跑通基础设施，组件测试留 task-033。**第三档。**
7. **无 Prettier / Husky**：留 task-033。**第三档。**
8. **无 cargo audit / npm audit**：task-006 边界明确不做。**第三档。**

## 安全风险检查结果

### OWASP Top 10 对照

| 项 | 结果 |
|----|------|
| A01 失效访问控制 | ✅ 纯工程化任务，无业务代码改动。 |
| A03 注入 | ✅ 无用户输入；测试文件是纯函数单测。 |
| A04 不安全设计 | ✅ ESLint no-unused-vars 先 warn 不 error（避免现有代码一大堆报错）；max-warnings 10 由 package.json 控制；CI 跑 lint/test 防前端写坏了发现不了。 |
| A08 数据完整性 | ✅ CI frozen lockfile 防依赖篡改；规则库 SHA-256 校验步骤保留。 |

### 专项安全分析

- **会不会引入恶意依赖？** 不会。CI `pnpm install --frozen-lockfile` 锁定依赖版本。✅
- **测试会不会漏测关键路径？** 不会。本任务只接基础设施，组件测试留 task-033。✅
- **ESLint 会不会太严把好代码挡了？** 不会。先宽松（no-unused-vars warn，max-warnings 10），后续再收紧。✅

## 评审结论：**通过**

task-006 echo 占位清零：ESLint 10 flat config（no-unused-vars warn + react-hooks rules error）+ Vitest 2.1（jsdom + setupTests + formatBytes/formatTs 纯函数单测 9 断言）+ CI 接 lint/test 两步；lint/test/build 全 0；clippy/fmt/cargo test 全 0（137 基线无回退）；8 个提交全部关联 #task-032 无混杂。前端工程化补齐，M1 业务全部完成。

## Git 操作记录

- **merge commit**：`73bc01e`（`git merge --no-ff feature/task-032-eslint-vitest-ci`）
- **本地 feature 分支**：已删除（`git branch -D feature/task-032-eslint-vitest-ci`，was 2afb689）
- **远端 feature 分支**：保留不动
- **评审报告 commit**：单独提交 `docs(review): 任务032评审报告 (#task-032)`

## 后续动作建议

1. **task-033**：组件单测（Dashboard/History/Items/SafeBox/Settings）+ Prettier / Husky。
2. **task-034**：NSIS 打包（Tauri bundle 配置 + 图标 + 应用名）准备发版。
3. **后续**：cargo audit / npm audit / Dependabot / safe_box purge 命令 / 端到端真机验收。
