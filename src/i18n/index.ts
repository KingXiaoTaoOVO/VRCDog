import { createI18n } from 'vue-i18n';
import zhCN from './locales/zh-CN.json';
import cs from './locales/cs.json';
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
import {
  FALLBACK_LOCALE,
  getPreferredLocale,
  normalizeLocale,
  type AppLocale
} from './languages';

export {
  DEFAULT_LOCALE,
  FALLBACK_LOCALE,
  SUPPORTED_LOCALES,
  getLocaleLabel,
  getNextLocale,
  getPreferredLocale,
  localeLabels,
  localeOptions,
  normalizeLocale,
  type AppLocale
} from './languages';

type LocaleMessages = Record<string, any>;

const isRecord = (value: unknown): value is LocaleMessages => (
  value !== null && typeof value === 'object' && !Array.isArray(value)
);

function mergeMessages(...sources: LocaleMessages[]): LocaleMessages {
  const result: LocaleMessages = {};

  for (const source of sources) {
    for (const [key, value] of Object.entries(source)) {
      if (isRecord(value) && isRecord(result[key])) {
        result[key] = mergeMessages(result[key], value);
      } else if (isRecord(value)) {
        result[key] = mergeMessages(value);
      } else if (isRecord(result[key])) {
        continue;
      } else {
        result[key] = value;
      }
    }
  }

  return result;
}

const withEnglishFallback = (messages: LocaleMessages = {}) => mergeMessages(zhCN, enUS, messages);
const withChineseFallback = (messages: LocaleMessages = {}) => mergeMessages(zhCN, messages);

const savedLocale = getPreferredLocale();

const messages: Record<AppLocale, LocaleMessages> = {
  'cs': withEnglishFallback(cs),
  'zh-CN': zhCN as LocaleMessages,
  'en-US': withChineseFallback(enUS),
  'ja-JP': withEnglishFallback(jaJP),
  'es': withEnglishFallback(es),
  'fr': withEnglishFallback(fr),
  'hu': withEnglishFallback(hu),
  'ko': withEnglishFallback(ko),
  'pl': withEnglishFallback(pl),
  'pt': withEnglishFallback(pt),
  'ru': withEnglishFallback(ru),
  'th': withEnglishFallback(th),
  'vi': withEnglishFallback(vi),
  'zh-TW': withChineseFallback(zhTW)
};

const i18n = createI18n({
  legacy: false,
  globalInjection: true,
  locale: savedLocale,
  fallbackLocale: FALLBACK_LOCALE,
  missingWarn: false,
  warnHtmlMessage: false,
  fallbackWarn: false,
  messages
});

interface SetAppLocaleOptions {
  persist?: boolean;
  notify?: boolean;
  preferred?: string | null;
}

export function setAppLocale(rawLocale: string | null | undefined, options: SetAppLocaleOptions = {}): AppLocale {
  const nextLocale = normalizeLocale(rawLocale);

  if (i18n.global) {
    (i18n.global.locale as any).value = nextLocale;
  }

  if (options.persist !== false && typeof window !== 'undefined') {
    const preferredLocale = normalizeLocale(options.preferred ?? rawLocale ?? nextLocale);
    localStorage.setItem('vrcdog-locale', nextLocale);
    localStorage.setItem('vrcdog-locale-pref', preferredLocale);
  }

  if (options.notify && typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent('settings-updated', { detail: { language: nextLocale } }));
  }

  return nextLocale;
}

export function translate(key: string, params?: Record<string, unknown>): string {
  return (i18n.global as any).t(key, params);
}

export default i18n;
