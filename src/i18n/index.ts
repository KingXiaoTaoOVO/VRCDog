import { createI18n } from 'vue-i18n';
import zhCN from './locales/zh-CN.json';
import enUS from './locales/en-US.json';
import jaJP from './locales/ja-JP.json';

const savedLocale = localStorage.getItem('vrcdog-locale') || 'zh-CN';

const i18n = createI18n({
  legacy: false, // Use Composition API
  locale: savedLocale, // 默认语言
  fallbackLocale: 'zh-CN',
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS,
    'ja-JP': jaJP
  }
});

export default i18n;
