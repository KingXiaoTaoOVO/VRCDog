import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";

const projectRoot = fileURLToPath(new URL(".", import.meta.url));
const host = process.env.TAURI_DEV_HOST || "127.0.0.1";
const normalizedPath = (value: string) => value.replace(/\\/g, "/").toLowerCase();
// Directories that are NOT part of the Vue frontend bundle. They contain pinned
// Rust/C++ deps, prebuilt binaries (.dll/.pdb), Python code, or separate
// reference projects. Watching them with chokidar triggers fs.watch EBUSY on
// Windows (the linker / antivirus keeps generated files like
// vendor/openvr_sys/.../openvr_api64.pdb open), which crashes `tauri dev`.
const NON_FRONTEND_DIRS = ['vendor', 'src-python', '弹幕姬', 'OVR', 'VRCT', 'VrcDog', 'dist', 'node_modules'];
const shouldIgnoreWatchPath = (value: string) => {
  const path = normalizedPath(value);
  if (NON_FRONTEND_DIRS.some((dir) => new RegExp(`/${dir}(?:/|$)`).test(path))) return true;
  return /\/(?:\.cargo-target|target)(?:\/|$)/.test(path) || /\/src-tauri(?:\/|$)/.test(path) || /\/vrcdog-server\/target(?:\/|$)/.test(path);
};

// https://vite.dev/config/
export default defineConfig(async () => ({
  root: projectRoot,
  base: '/web/',
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
