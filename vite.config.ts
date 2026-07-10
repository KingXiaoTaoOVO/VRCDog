import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST || "127.0.0.1";

// https://vite.dev/config/
export default defineConfig(async () => ({
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
      // 3. tell Vite to ignore watching generated/backend-heavy folders.
      ignored: ["**/src-tauri/**"],
    },
  },
  optimizeDeps: {
    entries: ['index.html'],
  },
  build: {
    chunkSizeWarningLimit: 1500,
    rollupOptions: {
      output: {
        manualChunks(id) {
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
