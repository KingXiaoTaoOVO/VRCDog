import "./styles.css";
import { createApp } from "vue";
import { createPinia } from 'pinia';

const isTrayMenuMode = window.location.search.includes('mode=tray-menu');
const STARTUP_RECOVERY_KEY = 'vrcdog-startup-network-recovered';

const sleep = (ms: number) => new Promise((resolve) => window.setTimeout(resolve, ms));

const isRecoverableLoadError = (error: unknown) => {
  const message = String((error as any)?.message || error || '');
  return /ERR_NETWORK_CHANGED|Failed to fetch dynamically imported module|Importing a module script failed|Load failed/i.test(message);
};

const recoverStartupOnce = () => {
  if (!import.meta.env.DEV || sessionStorage.getItem(STARTUP_RECOVERY_KEY) === '1') return false;
  sessionStorage.setItem(STARTUP_RECOVERY_KEY, '1');
  window.setTimeout(() => window.location.reload(), 350);
  return true;
};

async function withStartupRetry<T>(loader: () => Promise<T>, label: string): Promise<T> {
  let lastError: unknown;
  for (let attempt = 1; attempt <= 5; attempt += 1) {
    try {
      const result = await loader();
      sessionStorage.removeItem(STARTUP_RECOVERY_KEY);
      return result;
    } catch (error) {
      lastError = error;
      if (!isRecoverableLoadError(error) || attempt === 5) break;
      console.warn(`[Startup] ${label} load failed, retrying (${attempt}/5):`, error);
      await sleep(250 * attempt);
    }
  }
  if (isRecoverableLoadError(lastError) && recoverStartupOnce()) {
    return new Promise<T>(() => {});
  }
  throw lastError;
}

window.addEventListener('online', () => {
  if (document.body?.dataset.startupFailed === 'network') {
    recoverStartupOnce();
  }
});

// 生产环境禁止打开 DevTools (F12 / Ctrl+Shift+I / 右键菜单)
if (import.meta.env.PROD) {
  document.addEventListener('keydown', (e) => {
    // F12
    if (e.key === 'F12') {
      e.preventDefault();
      return false;
    }
    // Ctrl+Shift+I / Ctrl+Shift+J / Ctrl+Shift+C
    if (e.ctrlKey && e.shiftKey && ['I', 'J', 'C'].includes(e.key.toUpperCase())) {
      e.preventDefault();
      return false;
    }
    // Ctrl+U (查看源代码)
    if (e.ctrlKey && e.key.toUpperCase() === 'U') {
      e.preventDefault();
      return false;
    }
  });
  // 禁止右键菜单
  document.addEventListener('contextmenu', (e) => {
    e.preventDefault();
  });
}

const bootstrap = async () => {
  if (isTrayMenuMode) {
    try {
      const [{ default: TrayMenuView }, { default: i18n }] = await Promise.all([
        import("./components/TrayMenuView.vue"),
        import("./i18n"),
      ]);
      const trayApp = createApp(TrayMenuView);
      trayApp.use(i18n);
      trayApp.mount("#app");
    } catch (error) {
      console.error('[Startup] Failed to mount tray menu:', error);
      const root = document.getElementById('app');
      if (root) {
        root.innerHTML = `
          <div style="box-sizing:border-box;padding:14px;height:100%;display:flex;flex-direction:column;justify-content:center;background:rgba(255,255,255,0.92);border-radius:14px;box-shadow:0 8px 30px rgba(0,0,0,0.18);font-family:system-ui,sans-serif;user-select:none;">
            <div style="font-weight:700;font-size:14px;margin-bottom:10px;color:#1c1917;text-align:center;">VrcDog</div>
            <button onclick="window.__TAURI_INTERNALS__ ? window.__TAURI_INTERNALS__.invoke('tray_show_main_window') : window.location.reload()" style="width:100%;height:36px;margin-bottom:8px;border-radius:8px;border:none;background:#d97706;color:white;font-weight:700;font-size:13px;cursor:pointer;">显示主界面</button>
            <button onclick="window.__TAURI_INTERNALS__ ? window.__TAURI_INTERNALS__.invoke('tray_quit_app') : window.close()" style="width:100%;height:34px;border-radius:8px;border:1px solid rgba(220,38,38,0.25);background:rgba(254,242,242,0.9);color:#dc2626;font-weight:600;font-size:12px;cursor:pointer;">退出</button>
          </div>`;
      }
    }
    return;
  }

  const [{ default: App }, { default: i18n }] = await withStartupRetry(
    () => Promise.all([
      import("./App.vue"),
      import("./i18n"),
    ]),
    'main app',
  );

  const app = createApp(App);
  app.use(createPinia());
  app.use(i18n);
  app.mount("#app");
};

bootstrap().catch((error) => {
  if (isTrayMenuMode) return;
  console.error('[Startup] VrcDog failed to boot:', error);
  const root = document.getElementById('app');
  if (root) {
    const message = String((error as any)?.message || error || 'Unknown error');
    root.innerHTML = `
      <div style="height:100vh;display:flex;align-items:center;justify-content:center;background:#fffaf0;color:#9a6a38;font-family:system-ui,Segoe UI,sans-serif;">
        <div style="text-align:center;font-weight:700;max-width:480px;padding:0 16px;">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#d97706" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="margin:0 auto 16px;display:block;"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
          <div style="font-size:18px;margin-bottom:12px;">VrcDog 启动失败</div>
          <div style="font-size:13px;color:#92400e;margin-bottom:16px;white-space:pre-wrap;">${message.replace(/</g, '&lt;')}</div>
          <button onclick="window.location.reload()" style="border:0;border-radius:10px;padding:10px 16px;background:#d97706;color:white;font-weight:700;cursor:pointer;">重新加载</button>
        </div>
      </div>`;
  }
  if (isRecoverableLoadError(error)) {
    document.body.dataset.startupFailed = 'network';
    recoverStartupOnce();
  }
});
