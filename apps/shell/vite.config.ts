import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath } from "node:url";

const surface = (name: string) =>
  fileURLToPath(new URL(`./${name}.html`, import.meta.url));

// Each shell surface is its own document: on Windows they become separate
// webview windows, and in the browser the dev harness embeds them as frames.
export default defineConfig({
  root: fileURLToPath(new URL(".", import.meta.url)),
  clearScreen: false,
  plugins: [react()],
  resolve: {
    alias: {
      "@bw/core": fileURLToPath(
        new URL("../../packages/core/src/index.ts", import.meta.url),
      ),
      "@bw/tokens": fileURLToPath(
        new URL("../../packages/tokens/src/index.ts", import.meta.url),
      ),
    },
  },
  server: { port: 5173, strictPort: true },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "chrome110",
    rollupOptions: {
      input: {
        index: surface("index"),
        background: surface("background"),
        bar: surface("bar"),
        osd: surface("osd"),
        notifications: surface("notifications"),
        dock: surface("dock"),
        overview: surface("overview"),
        regionSelect: surface("regionSelect"),
        sidebarLeft: surface("sidebarLeft"),
        sidebarRight: surface("sidebarRight"),
        wallpaperSelector: surface("wallpaperSelector"),
      },
    },
  },
  test: {
    globals: true,
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
