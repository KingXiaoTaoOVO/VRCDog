export const DEFAULT_LOCALE = 'zh-CN';
export const FALLBACK_LOCALE = 'en-US';

export const SUPPORTED_LOCALES = [
  'cs',
  'en-US',
  'es',
  'fr',
  'hu',
  'ja-JP',
  'ko',
  'pl',
  'pt',
  'ru',
  'th',
  'vi',
  'zh-CN',
  'zh-TW'
] as const;

export type AppLocale = typeof SUPPORTED_LOCALES[number];

export const localeLabels: Record<AppLocale, string> = {
  cs: 'Čeština',
  'en-US': 'English',
  es: 'Español',
  fr: 'Français',
  hu: 'Magyar',
  'ja-JP': '日本語',
  ko: '한국어',
  pl: 'Polski',
  pt: 'Português',
  ru: 'Русский',
  th: 'ไทย',
  vi: 'Tiếng Việt',
  'zh-CN': '简体中文',
  'zh-TW': '繁體中文'
};

export const localeOptions = SUPPORTED_LOCALES.map((code) => ({
  label: localeLabels[code],
  value: code
}));

const localeAliasMap: Record<string, AppLocale> = {
  cs: 'cs',
  czech: 'cs',
  en: 'en-US',
  'en-us': 'en-US',
  english: 'en-US',
  es: 'es',
  'es-es': 'es',
  fr: 'fr',
  'fr-fr': 'fr',
  hu: 'hu',
  'hu-hu': 'hu',
  ja: 'ja-JP',
  jp: 'ja-JP',
  'ja-jp': 'ja-JP',
  japanese: 'ja-JP',
  ko: 'ko',
  kr: 'ko',
  'ko-kr': 'ko',
  pl: 'pl',
  'pl-pl': 'pl',
  pt: 'pt',
  'pt-br': 'pt',
  'pt-pt': 'pt',
  ru: 'ru',
  'ru-ru': 'ru',
  th: 'th',
  'th-th': 'th',
  vi: 'vi',
  'vi-vn': 'vi',
  zh: 'zh-CN',
  'zh-cn': 'zh-CN',
  'zh-hans': 'zh-CN',
  'zh-hans-cn': 'zh-CN',
  'zh-sg': 'zh-CN',
  'zh-tw': 'zh-TW',
  'zh-hant': 'zh-TW',
  'zh-hant-tw': 'zh-TW',
  'zh-hk': 'zh-TW'
};

function unwrapLocale(rawLocale: string | null | undefined): string {
  if (!rawLocale) return '';
  const trimmed = String(rawLocale).trim();
  if (!trimmed) return '';

  if (
    (trimmed.startsWith('"') && trimmed.endsWith('"')) ||
    (trimmed.startsWith("'") && trimmed.endsWith("'"))
  ) {
    try {
      const parsed = JSON.parse(trimmed);
      if (typeof parsed === 'string') return parsed.trim();
    } catch {
      return trimmed.slice(1, -1).trim();
    }
  }

  return trimmed;
}

export function normalizeLocale(rawLocale: string | null | undefined): AppLocale {
  const normalized = unwrapLocale(rawLocale).replace(/_/g, '-');
  const lower = normalized.toLowerCase();

  if (lower in localeAliasMap) {
    return localeAliasMap[lower];
  }

  const exact = SUPPORTED_LOCALES.find((locale) => locale.toLowerCase() === lower);
  if (exact) return exact;

  const language = lower.split('-')[0];
  if (language in localeAliasMap) {
    return localeAliasMap[language];
  }

  return DEFAULT_LOCALE;
}

export function getPreferredLocale(): AppLocale {
  if (typeof window === 'undefined') return DEFAULT_LOCALE;

  const storedPreference = localStorage.getItem('vrcdog-locale-pref')
    || localStorage.getItem('livehime-locale-pref');
  if (storedPreference) return normalizeLocale(storedPreference);

  const storedLocale = localStorage.getItem('vrcdog-locale')
    || localStorage.getItem('livehime-locale');
  if (storedLocale) return normalizeLocale(storedLocale);

  const browserLocales = navigator.languages?.length ? navigator.languages : [navigator.language];
  for (const browserLocale of browserLocales) {
    const normalized = normalizeLocale(browserLocale);
    if (normalized !== DEFAULT_LOCALE || browserLocale?.toLowerCase().startsWith('zh')) {
      return normalized;
    }
  }

  return DEFAULT_LOCALE;
}

export function getLocaleLabel(rawLocale: string | null | undefined): string {
  return localeLabels[normalizeLocale(rawLocale)];
}

export function getNextLocale(rawLocale: string | null | undefined): AppLocale {
  const current = normalizeLocale(rawLocale);
  const index = SUPPORTED_LOCALES.indexOf(current);
  return SUPPORTED_LOCALES[(index + 1) % SUPPORTED_LOCALES.length];
}
