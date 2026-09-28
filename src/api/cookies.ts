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

export function cleanCookieSegment(segment: string): string | null {
  const part = segment.trim().replace(/^(set-cookie|cookie):\s*/i, '');
  if (!part) return null;

  const equals = part.indexOf('=');
  if (equals <= 0) return null;

  const name = part.slice(0, equals).trim();
  // Cookie names must be valid token characters (no brackets, quotes, braces, whitespace)
  if (!/^[a-zA-Z0-9_\-]+$/.test(name)) return null;

  let value = part.slice(equals + 1).trim();
  if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) {
    value = value.slice(1, -1).trim();
  }
  if (!name || !value || COOKIE_ATTR_NAMES.has(name.toLowerCase())) return null;

  const lowerVal = value.toLowerCase();
  if (['deleted', 'null', 'undefined', 'none'].includes(lowerVal)) return null;

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
      // If it looks like a JSON array but failed parsing (e.g. malformed JSON),
      // do not treat the whole array string as a single cookie segment.
      return [];
    }
  }

  const segments = raw.split(';');
  const cookies = segments
    .map(cleanCookieSegment)
    .filter((cookie): cookie is string => Boolean(cookie));

  if (cookies.length > 0) {
    return cookies;
  }

  // If no valid key=value cookies were parsed, check if it's a bare token (no '=')
  // and wrap it as auth=<token>. If it contains '=' but failed parsing, it's corrupt - return empty.
  if (raw.includes('=')) {
    return [];
  }
  const cleanBare = raw.replace(/^["']|["']$/g, '').trim();
  if (!cleanBare || ['deleted', 'null', 'undefined', 'none'].includes(cleanBare.toLowerCase())) {
    return [];
  }
  return [`auth=${cleanBare}`];
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
  if (newCookies.length === 0) {
    return null;
  }

  if (!isTauri()) {
    return JSON.stringify(newCookies);
  }

  let existing: string[] = [];
  try {
    const stored = await invoke<string | null>('db_get_auth');
    existing = parseCookieInput(stored);
  } catch {
    existing = [];
  }

  // Merge: new cookies overwrite existing ones with the same name,
  // BUT do NOT let an invalid/empty/corrupted token overwrite an existing valid auth/twoFactorAuth token!
  const cookieMap = new Map<string, string>();
  for (const cookie of existing) {
    const equalsIdx = cookie.indexOf('=');
    if (equalsIdx > 0) {
      const name = cookie.slice(0, equalsIdx).trim().toLowerCase();
      if (name) cookieMap.set(name, cookie);
    }
  }

  for (const cookie of newCookies) {
    const equalsIdx = cookie.indexOf('=');
    if (equalsIdx > 0) {
      const name = cookie.slice(0, equalsIdx).trim().toLowerCase();
      const val = cookie.slice(equalsIdx + 1).trim().replace(/^["']|["']$/g, '');
      if (name) {
        if (name === 'auth' || name === 'twofactorauth') {
          const lowerVal = val.toLowerCase();
          if (val.length >= 8 && !['deleted', 'null', 'undefined', 'none'].includes(lowerVal)) {
            cookieMap.set(name, `${cookie.slice(0, equalsIdx).trim()}=${val}`);
          }
        } else {
          cookieMap.set(name, cookie);
        }
      }
    }
  }

  const merged = Array.from(cookieMap.values());
  const mergedJson = JSON.stringify(merged);
  await invoke('db_save_auth', { cookie: mergedJson });
  return mergedJson;
}
