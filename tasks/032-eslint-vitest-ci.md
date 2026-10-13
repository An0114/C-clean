# 任务编号：032 前端工程化补齐——ESLint + Vitest 接入（CI 不再 echo 占位）

> 派发角色：DocsKeeper（文档与状态管家）
> 执行角色：DevFrontendReact
> 来源：`docs/PROJECT_STATE.md` 已知问题（task-006 那条）——「package.json lint/test 为 echo 占位，待前端组件落地后再启用」。task-026~031 前端组件已全部落地，正好补技术债。
> 上游依赖：task-001（Vite + React 18 + TS 脚手架）、task-026~031（5 个页面组件已就位）、task-006（GitHub Actions CI 已有 cargo 部分）。
> **克制范围**：只接 ESLint + Vitest + CI 跑起来，不写新业务代码、不改组件逻辑。

## 任务目标

目前 `package.json` 的 `lint` / `test` 是 `echo` 占位，CI 也不跑——前端写坏了 lint/test 发现不了。本任务补最小工程化：

1. **ESLint**：
   - 安装 `eslint` + `@typescript-eslint/parser` + `@typescript-eslint/eslint-plugin` + `eslint-plugin-react-hooks` + `eslint-plugin-react-refresh`；
   - 新建 `app/.eslintrc.cjs`（或 `eslint.config.js` flat config）；
   - `package.json` 的 `lint` 改为 `eslint src --ext .ts,.tsx`；
2. **Vitest**：
   - 安装 `vitest` + `@testing-library/react` + `@testing-library/jest-dom` + `jsdom`；
   - 新建 `app/vitest.config.ts`（或在 vite.config.ts 里加 test 字段，environment='jsdom'）；
   - 给 `formatBytes.ts` 和 `formatTs.ts` 各写 1 个单测（纯函数，最容易测）；
   - `package.json` 的 `test` 改为 `vitest run`；
3. **CI 接线**：
   - 改 `.github/workflows/ci.yml`：前端 job 加 `pnpm lint` + `pnpm test` 步骤（替换现在的 echo）；
4. **全绿**：
   - `pnpm lint` 退出码 0；
   - `pnpm test` 退出码 0；
   - `pnpm build` 仍退出码 0。

**本任务不做**：
- 不改现有组件业务逻辑（Dashboard/History/Items/SafeBox/Settings 一行不动）；
- 不写组件单测（先跑通基础设施，组件测试留 task-033）；
- 不接 Prettier / Stylelint / Husky；
- 不接 cargo audit / npm audit（task-006 边界明确不做）。

## 前置条件

1. `main` 分支已含 task-001~031 全部产出；
2. 关键锚点：
   - `app/package.json`（现有 scripts：dev / build / preview / lint=echo / test=echo）；
   - `app/vite.config.ts`；
   - `.github/workflows/ci.yml`（现有前端 job 跑 `pnpm install && pnpm build`，lint/test 是 echo）。

## 需求 & 验收标准

> DevAgent 自验时逐条勾选。

### A. ESLint 接入

1. 安装依赖（devDeps）：
   ```
   pnpm --filter app add -D eslint @typescript-eslint/parser @typescript-eslint/eslint-plugin eslint-plugin-react-hooks eslint-plugin-react-refresh
   ```
2. 新建 `app/.eslintrc.cjs`：
   - parser: `@typescript-eslint/parser`；
   - plugins: `@typescript-eslint`, `react-hooks`, `react-refresh`；
   - extends: `eslint:recommended`, `plugin:@typescript-eslint/recommended`；
   - env: `browser`, `es2022`；
   - rules 只开最严的：`no-unused-vars: warn`（先不 error，避免现有代码一大堆报错）；
3. `package.json` scripts：
   ```json
   "lint": "eslint src --ext .ts,.tsx --max-warnings 10"
   ```

### B. Vitest 接入

4. 安装依赖：
   ```
   pnpm --filter app add -D vitest @testing-library/react @testing-library/jest-dom jsdom
   ```
5. 在 `app/vite.config.ts` 加 test 配置：
   ```ts
   /// <reference types="vitest" />
   export default defineConfig({
     // ...现有配置
     test: {
       environment: "jsdom",
       setupFiles: ["./src/setupTests.ts"],
     },
   });
   ```
6. 新建 `app/src/setupTests.ts`：
   ```ts
   import "@testing-library/jest-dom";
   ```
7. 写两个纯函数单测：
   - `app/src/utils/formatBytes.test.ts`：
     - 0 → "0 B"；
     - 500 → "500 B"；
     - 2048 → "2.0 KB"；
     - 5*1024*1024 → "5.0 MB"；
     - 2*1024*1024*1024 → "2.0 GB"；
   - `app/src/utils/formatTimestamp.test.ts`：
     - 0 → "-"；
     - NaN → "-"；
     - 合法 unix 秒 → 非 "-" 的字符串。
8. `package.json` scripts：
   ```json
   "test": "vitest run"
   ```

### C. CI 接线

9. 改 `.github/workflows/ci.yml`：
   - 前端 job 在 `pnpm install` 后、`pnpm build` 前加：
     ```yaml
     - name: Lint
       run: pnpm --filter app lint
     - name: Test
       run: pnpm --filter app test
     ```
   - 删掉原 echo 占位步骤。

### D. 卫生与边界

10. 不改 `app/src/pages/**`、`app/src/App.tsx` 任何业务代码；
11. 不改 Rust 代码；
12. 不写组件测试（先基础设施）；
13. 不接 Prettier / Husky。

## 允许修改范围

> 严格白名单。范围外文件一律不动。

- `app/package.json`（scripts + devDeps）
- `app/pnpm-lock.yaml`（pnpm 自动改）
- `app/.eslintrc.cjs`（新建）
- `app/vite.config.ts`（加 test 字段）
- `app/src/setupTests.ts`（新建）
- `app/src/utils/formatBytes.test.ts`（新建）
- `app/src/utils/formatTimestamp.test.ts`（新建）
- `.github/workflows/ci.yml`（加 lint/test 步骤）
- 任务收尾单独修改：`docs/PROJECT_STATE.md`

## Git 分支

- 分支名：`feature/task-032-eslint-vitest-ci`，从最新 `main` 拉出。
- 禁止直接 push `main`；只 push 上述 feature 分支。

## Git 提交要求

1. `chore(frontend): 安装 ESLint + TS 插件 + 新建 .eslintrc.cjs (#task-032)`
   —— 含 `app/package.json`、`app/pnpm-lock.yaml`、`app/.eslintrc.cjs`。
2. `chore(frontend): 安装 Vitest + testing-library + jsdom + vite.config test 字段 (#task-032)`
   —— 含 `app/package.json`、`app/pnpm-lock.yaml`、`app/vite.config.ts`、`app/src/setupTests.ts`。
3. `test(frontend): 给 formatBytes / formatTs 写纯函数单测 (#task-032)`
   —— 含 `app/src/utils/formatBytes.test.ts`、`app/src/utils/formatTimestamp.test.ts`。
4. `ci: 前端 job 接 pnpm lint + pnpm test，替换 echo 占位 (#task-032)`
   —— 含 `.github/workflows/ci.yml`。
5. 自验全绿后，最后一次提交：`docs(state): 任务032完成，ESLint+Vitest 接入 (#task-032)`
   —— **单独一次提交，只动 `docs/PROJECT_STATE.md`**：
   - 「待办事项」段追加 `- [x] 032: ...`；
   - 「最近一次变更」追加一行；
   - 更新「已知问题」段 task-006 那条（lint/test 已启用）；
   - 「下一步计划」改为「M1 业务全部完成 + 工程化补齐；下一轮进入 NSIS 打包（Tauri bundle 配置 + 图标 + 应用名）准备发版」；
   - 「Git 提交记录」表追加 commit 摘要；
   - 「变更轨迹」追加 `2026-10-13  task-032  ESLint+Vitest 接入，CI 跑 lint/test，task-006 echo 占位清零`。

**提交前自验（必须全绿）：**
- `pnpm --filter app lint` 退出码 0；
- `pnpm --filter app test` 退出码 0（至少 2 个文件 / 8 个断言全过）；
- `pnpm --filter app build` 退出码 0；
- `cargo check --workspace --all-targets` 退出码 0（后端无改动）；
- `cargo test -p engine` 退出码 0（已有 137 条全绿）。

## 输出要求

DevAgent 交付物（后续执行结果）：

1. 上述 4 个代码提交 + 1 个 `docs(state)` 提交，共 5 个 commit，全部 push 到 `feature/task-032-eslint-vitest-ci`；
2. 任务文件末尾追加「## 实现备注」段，包含：
   - 实际改动文件清单；
   - `pnpm lint` / `pnpm test` / `pnpm build` 输出（无 error）；
   - `cargo test -p engine` 通过输出；
   - 与本任务文档不符的偏差（如有，如 ESLint 规则收紧、Vitest 配置细节）；
   - 遗留问题（组件单测、Prettier、Husky、cargo audit 留 task-033+）；
3. `docs/PROJECT_STATE.md` 已按第 5 条提交规范更新；
4. 本任务文档本身**由 DocsKeeper 在派发时**单独提交一次：
   `docs(task): 新增任务032-ESLint+Vitest 接入 (#task-032)`，
   该提交只含 `tasks/032-eslint-vitest-ci.md` 一个文件，落在 `feature/task-032-eslint-vitest-ci` 分支最前端。
