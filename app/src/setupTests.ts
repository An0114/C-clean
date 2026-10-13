// task-032：Vitest setup——引入 @testing-library/jest-dom 匹配器。
// 注意：必须从 vitest 显式导入 expect，否则 setup 文件在测试框架注册前执行会报 expect is not defined。
import "@testing-library/jest-dom/vitest";
