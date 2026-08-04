import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

const projectRoot = fileURLToPath(new URL(".", import.meta.url));
const host = process.env.TAURI_DEV_HOST || "127.0.0.1";
const normalizedPath = (value: string) => value.replace(/\\/g, "/").toLowerCase();
const shouldIgnoreWatchPath = (value: string) => {
  const path = normalizedPath(value);
  return /\/(?:\.cargo-target|target)(?:\/|$)/.test(path) || /\/src-tauri(?:\/|$)/.test(path) || /\/vrcdog-server\/target(?:\/|$)/.test(path);
};

// https://vite.dev/config/
export default defineConfig(async () => ({
  root: projectRoot,
  plugins: [vue(), tailwindcss()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host,
    hmr: {
      protocol: "ws",
      host,
      clientPort: 1420,
    },
    watch: {
      // 3. Keep Rust/Tauri outputs out of chokidar. On Windows the linker and
      // antivirus can hold generated .exe files open, causing fs.watch EBUSY.
      ignored: shouldIgnoreWatchPath,
    },
  },
  optimizeDeps: {
    entries: ['index.html'],
  },
  build: {
    chunkSizeWarningLimit: 1500,
    rollupOptions: {
      output: {
        manualChunks(id: string) {
          if (id.includes('node_modules')) {
            if (id.includes('echarts')) {
              return 'vendor-echarts';
            }
            if (id.includes('vue')) {
              return 'vendor-vue';
            }
            if (id.includes('lucide')) {
              return 'vendor-lucide';
            }
            if (id.includes('@tauri-apps')) {
              return 'vendor-tauri';
            }
            return 'vendor';
          }
        }
      }
    }
  }
}));
