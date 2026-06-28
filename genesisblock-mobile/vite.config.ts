import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @tauri-apps/cli convention: don't let Vite clear the screen so Rust/Cargo
// compiler output stays visible during `tauri dev`.
export default defineConfig({
  plugins: [react()],
  // Prevent Vite from obscuring Rust errors.
  clearScreen: false,
  server: {
    // Tauri expects a fixed port; fail rather than silently pick another.
    port: 5173,
    strictPort: true,
  },
  // Tauri reads ../dist relative to src-tauri.
  build: {
    outDir: "dist",
    // Tauri uses Chromium on Windows/Android and WebKit on macOS/iOS.
    target: ["es2021", "chrome100", "safari13"],
    sourcemap: false,
  },
});
