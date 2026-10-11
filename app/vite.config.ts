import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri 2 约定：前端 dev server 固定监听 5173，且端口严格占用，
// 与 tauri.conf.json 中 build.devUrl = http://localhost:5173 对应。
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  // 生产构建产物输出到 app/dist/，对应 tauri.conf.json 的 frontendDist = ../dist
  build: {
    outDir: "dist",
  },
});
