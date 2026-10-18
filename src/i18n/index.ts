import { createI18n } from 'vue-i18n';
import zhCN from './locales/zh-CN.json';
import cs from './locales/cs.json';
import enUS from './locales/en-US.json';
import enUSSupplement from './locales/en-US.supplement';
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
import yue from './locales/yue.json';
import { nativeCoreMessages } from './locales/native-core';
import { drawingMessages } from './drawingMessages';
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

const effectiveEnUS = mergeMessages(enUS, enUSSupplement);
const withEnglishFallback = (locale: AppLocale, messages: LocaleMessages = {}) => (
  mergeMessages(zhCN, effectiveEnUS, messages, nativeCoreMessages[locale] || {}, drawingMessages[locale])
);
const withChineseFallback = (messages: LocaleMessages = {}) => mergeMessages(zhCN, messages);

const savedLocale = getPreferredLocale();

const messages: Record<AppLocale, LocaleMessages> = {
  'cs': withEnglishFallback('cs', cs),
  'zh-CN': mergeMessages(zhCN, drawingMessages['zh-CN']),
  'en-US': mergeMessages(withChineseFallback(effectiveEnUS), drawingMessages['en-US']),
  'ja-JP': withEnglishFallback('ja-JP', jaJP),
  'es': withEnglishFallback('es', es),
  'fr': withEnglishFallback('fr', fr),
  'hu': withEnglishFallback('hu', hu),
  'ko': withEnglishFallback('ko', ko),
  'pl': withEnglishFallback('pl', pl),
  'pt': withEnglishFallback('pt', pt),
  'ru': withEnglishFallback('ru', ru),
  'th': withEnglishFallback('th', th),
  'vi': withEnglishFallback('vi', vi),
  'yue': withEnglishFallback('yue', yue),
  'zh-TW': mergeMessages(zhCN, zhTW, nativeCoreMessages['zh-TW'] || {}, drawingMessages['zh-TW'])
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

/**
 * 取「数组型」文案（星期名、小时坐标这类）。
 *
 * 必须走 `tm()` 而不是 `t()`：vue-i18n 的 `t()` 遇到数组消息**返回 key 本身**（一个字符串），
 * 于是 `Array.isArray(t('charts.days'))` 永远是 false（数组文案形同虚设），
 * 而模板里的 `v-for="(label, idx) in t('charts.hours_short')"` 会**逐字符**遍历那个字符串，
 * 把小时坐标渲染成 "c h a r t s ." 这样的垃圾。
 *
 * 语言包里缺这一项、或当前语种只回退出字符串时，用调用方给的兜底数组。
 */
export function translateList(key: string, fallback: string[]): string[] {
  const value = (i18n.global as any).tm(key);
  if (Array.isArray(value)) {
    const items = value.map((item) => (typeof item === 'string' ? item : String(item ?? '')));
    if (items.length > 0 && items.every((item) => item.trim() !== '')) return items;
  }
  return fallback;
}

export default i18n;
