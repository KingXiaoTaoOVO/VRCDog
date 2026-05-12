import { createI18n } from 'vue-i18n';
import zhCN from './locales/zh-CN.json';
import enUS from './locales/en-US.json';
import jaJP from './locales/ja-JP.json';
import es from './locales/es.json';
import fr from './locales/fr.json';
import hu from './locales/hu.json';
import ko from './locales/ko.json';
import pl from './locales/pl.json';
import pt from './locales/pt.json';
import ru from './locales/ru.json';
import th from './locales/th.json';
import vi from './locales/vi.json';
import zhTW from './locales/zh-TW.json';

const savedLocale = localStorage.getItem('vrcdog-locale') || 'zh-CN';

const i18n = createI18n({
  legacy: false,
  globalInjection: true,
  locale: savedLocale,
  fallbackLocale: 'zh-CN',
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS,
    'ja-JP': jaJP,
    'es': es,
    'fr': fr,
    'hu': hu,
    'ko': ko,
    'pl': pl,
    'pt': pt,
    'ru': ru,
    'th': th,
    'vi': vi,
    'zh-TW': zhTW
  }
});

export default i18n;
