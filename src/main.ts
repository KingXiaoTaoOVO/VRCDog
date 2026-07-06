import "./styles.css";
import { createApp } from "vue";
import App from "./App.vue";
import i18n from "./i18n";

import { createPinia } from 'pinia';

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

const app = createApp(App);
app.use(createPinia());
app.use(i18n);
app.mount("#app");
