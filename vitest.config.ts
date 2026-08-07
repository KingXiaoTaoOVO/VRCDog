import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  test: {
    environment: 'jsdom',
    environmentOptions: {
      jsdom: {
        // A real origin is required for jsdom to provide a working `localStorage`.
        // Without it, specs that don't manually mock storage (e.g. LoginView) crash.
        url: 'http://localhost/',
      },
    },
    globals: true,
    include: ['src/**/*.{test,spec}.{ts,tsx,js,jsx}', '*.{test,spec}.{ts,tsx,js,jsx}'],
    exclude: [
      '**/node_modules/**',
      '**/dist/**',
      '**/src-tauri/**',
      '**/src-python/**',
      '**/源码模板/**',
    ],
  },
})
