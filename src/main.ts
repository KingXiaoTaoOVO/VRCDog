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
    const [{ default: TrayMenuView }, { default: i18n }] = await withStartupRetry(
      () => Promise.all([
        import("./components/TrayMenuView.vue"),
        import("./i18n"),
      ]),
      'tray menu',
    );
    const trayApp = createApp(TrayMenuView);
    trayApp.use(i18n);
    trayApp.mount("#app");
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
  console.error('[Startup] VrcDog failed to boot:', error);
  if (isRecoverableLoadError(error)) {
    document.body.dataset.startupFailed = 'network';
    const root = document.getElementById('app');
    if (root) {
      root.innerHTML = `
        <div style="height:100vh;display:flex;align-items:center;justify-content:center;background:#fffaf0;color:#9a6a38;font-family:system-ui,Segoe UI,sans-serif;">
          <div style="text-align:center;font-weight:700;">
            <div style="margin-bottom:12px;">VrcDog 正在恢复启动连接...</div>
            <button id="vrcdog-reload" style="border:0;border-radius:10px;padding:10px 16px;background:#e7a94d;color:white;font-weight:700;cursor:pointer;">重新加载</button>
          </div>
        </div>`;
      document.getElementById('vrcdog-reload')?.addEventListener('click', () => window.location.reload());
    }
    recoverStartupOnce();
  }
});
