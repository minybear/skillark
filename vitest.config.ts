import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Vitest config for G10 frontend interaction E2E. Kept separate from
// vite.config.ts so the Tauri-specific dev-server options don't interfere.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    css: false,
  },
});
