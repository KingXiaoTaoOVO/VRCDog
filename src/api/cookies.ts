import { invoke, isTauri } from '@tauri-apps/api/core';

const COOKIE_ATTR_NAMES = new Set([
  'domain',
  'expires',
  'httponly',
  'max-age',
  'path',
  'samesite',
  'secure'
]);

function cleanCookieSegment(segment: string): string | null {
  const part = segment.trim().replace(/^(set-cookie|cookie):\s*/i, '');
  if (!part) return null;

  const equals = part.indexOf('=');
  if (equals <= 0) return null;

  const name = part.slice(0, equals).trim();
  const value = part.slice(equals + 1).trim();
  if (!name || !value || COOKIE_ATTR_NAMES.has(name.toLowerCase())) return null;

  return `${name}=${value}`;
}

export function parseCookieInput(rawCookie: string | null | undefined): string[] {
  if (!rawCookie) return [];

  const raw = rawCookie.trim();
  if (!raw) return [];

  if (raw.startsWith('[')) {
    try {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) {
        return parsed.flatMap((item) => parseCookieInput(String(item)));
      }
    } catch {
      // Fall through and treat it as a raw cookie string.
    }
  }

  const segments = raw.split(';');
  const cookies = segments
    .map(cleanCookieSegment)
    .filter((cookie): cookie is string => Boolean(cookie));

  if (cookies.length > 0) {
    return cookies;
  }

  return raw.includes('=') ? [] : [`auth=${raw}`];
}

export function normalizeAuthCookieJson(rawCookie: string | null | undefined): string {
  const cookies = parseCookieInput(rawCookie);
  return JSON.stringify(cookies);
}

export function getCookieValue(
  rawCookie: string | null | undefined,
  cookieName: string,
): string | null {
  const targetName = cookieName.trim().toLowerCase();
  if (!targetName) return null;

  for (const cookie of parseCookieInput(rawCookie)) {
    const equals = cookie.indexOf('=');
    if (equals <= 0) continue;
    const name = cookie.slice(0, equals).trim().toLowerCase();
    if (name === targetName) {
      return cookie.slice(equals + 1).trim() || null;
    }
  }

  return null;
}

export async function mergeCookiesAndSave(newCookieJson: string | null | undefined): Promise<string | null> {
  const newCookies = parseCookieInput(newCookieJson);
  if (!isTauri() || newCookies.length === 0) {
    return newCookies.length > 0 ? JSON.stringify(newCookies) : null;
  }

  let existing: string[] = [];
  try {
    const stored = await invoke<string | null>('db_get_auth');
    existing = parseCookieInput(stored);
  } catch {
    existing = [];
  }

  const cookieMap = new Map<string, string>();
  for (const cookie of [...existing, ...newCookies]) {
    const name = cookie.split('=')[0]?.trim().toLowerCase();
    if (name) cookieMap.set(name, cookie);
  }

  const merged = Array.from(cookieMap.values());
  const mergedJson = JSON.stringify(merged);
  await invoke('db_save_auth', { cookie: mergedJson });
  return mergedJson;
}
