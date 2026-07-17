import { invoke, isTauri } from "@tauri-apps/api/core";
import { mergeCookiesAndSave, normalizeAuthCookieJson } from './cookies';
import { isDebugLogEnabled } from './debugConfig';

function buildVrchatApiUrl(url: string): string {
  if (url.startsWith('http')) return url;
  const endpoint = url.replace(/^\/+/, '').replace(/^api\/1\/?/i, '');
  return `https://api.vrchat.cloud/api/1/${endpoint}`;
}

export type VrcRequestErrorCode =
  | 'VRCHAT_AUTH_EXPIRED'
  | 'VRCHAT_PERMISSION_DENIED'
  | 'VRCHAT_HTTP_ERROR';

export class VrcRequestError extends Error {
  code: VrcRequestErrorCode;
  status?: number;
  url: string;
  response?: unknown;

  constructor(
    message: string,
    details: { code: VrcRequestErrorCode; status?: number; url: string; response?: unknown }
  ) {
    super(message);
    this.name = 'VrcRequestError';
    this.code = details.code;
    this.status = details.status;
    this.url = details.url;
    this.response = details.response;
  }
}

export const isVrcRequestError = (err: unknown, code?: VrcRequestErrorCode): err is VrcRequestError => {
  const candidate = err as Partial<VrcRequestError> | undefined;
  return Boolean(candidate?.name === 'VrcRequestError' && (!code || candidate.code === code));
};

export function parseResponseData(data: unknown) {
  if (!data) return null;
  if (typeof data !== 'string') return data;
  try {
    return JSON.parse(data);
  } catch {
    return data;
  }
}

export function parseExecuteResponse<T = any>(res: any, fallbackUrl = 'vrc_execute'): T {
  const parsed = parseResponseData(res?.data ?? res);
  const status = typeof res?.status === 'number' ? res.status : 200;
  if (status < 200 || status >= 300) {
    const message = extractVrchatErrorMessage(parsed, res?.data, status);
    throw new VrcRequestError(message, {
      code: 'VRCHAT_HTTP_ERROR',
      status,
      url: res?.url || fallbackUrl,
      response: parsed,
    });
  }
  return parsed as T;
}

function extractVrchatErrorMessage(parsed: any, rawData: unknown, status?: number): string {
  const candidates = [
    parsed?.error?.message,
    parsed?.error?.details,
    parsed?.message,
    parsed?.details,
    typeof parsed?.error === 'string' ? parsed.error : undefined,
    typeof parsed === 'string' ? parsed : undefined,
    typeof rawData === 'string' ? rawData : undefined,
  ];
  const message = candidates
    .map((value) => (typeof value === 'string' ? value.trim() : ''))
    .find(Boolean);
  return message || `HTTP ${status || 0}`;
}

function isVrchatUrl(url: string): boolean {
  return url.includes('api.vrchat.cloud');
}

function isVrchatPermissionError(status: number, url: string, message: string): boolean {
  if (!isVrchatUrl(url)) return false;
  const lower = message.toLowerCase();

  if (status === 403) return true;
  if (status !== 401) return false;

  if (url.includes('/avatars') && url.includes('userId=') && /own avatars|browse your own avatars|can only browse/i.test(message)) {
    return true;
  }

  const permissionPattern = /(you can only|not allowed|forbidden|permission|private|must be friends|cannot access|not authorized to access)/i;
  const authPattern = /(missing credentials|invalid credentials|^unauthorized$|unauthorized user|expired|login required|not logged in)/i;
  return permissionPattern.test(lower) && !authPattern.test(lower);
}

function isVrchatAuthExpired(status: number, url: string, message: string): boolean {
  if (status !== 401 || !isVrchatUrl(url)) return false;
  if (isVrchatPermissionError(status, url, message)) return false;

  try {
    const parsedUrl = new URL(url);
    if (parsedUrl.pathname.endsWith('/auth/user')) return true;
  } catch {
    if (url.includes('/auth/user')) return true;
  }

  const lower = message.toLowerCase();
  if (!lower || lower === 'http 401') return true;
  return /(missing credentials|invalid credentials|^unauthorized$|unauthorized user|expired|login required|not logged in)/i.test(lower);
}

export async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    console.warn(`[Browser Mock] API Command: ${cmd}`, args);
    if (cmd === 'vrc_execute') {
      const requestUrl = String((args?.options as any)?.url || '');
      if (requestUrl.includes('/auth/user')) {
        return Promise.resolve({
          status: 200,
          data: JSON.stringify({
            id: 'usr_browser_preview',
            username: 'preview-user',
            displayName: 'VrcDog Preview',
            status: 'active',
            statusDescription: 'Browser UI preview',
            currentAvatarThumbnailImageUrl: '',
            tags: [],
          }),
          auth_cookie: 'auth=browser_preview',
        }) as T;
      }
      if (requestUrl.includes('/config')) {
        return Promise.resolve({
          status: 200,
          data: JSON.stringify({
            apiUrl: 'https://api.vrchat.cloud/api/1',
            websocketUrl: 'wss://pipeline.vrchat.cloud',
          }),
        }) as T;
      }
      if (
        requestUrl.includes('/api/client/register')
        || requestUrl.includes('/api/client/heartbeat')
        || requestUrl.includes('/api/client/check-status/')
      ) {
        return Promise.resolve({
          status: 200,
          data: JSON.stringify({ status: 'ok', allowed: true, features: [] }),
        }) as T;
      }
      if (requestUrl.includes('/api/client/features/')) {
        return Promise.resolve({ status: 200, data: JSON.stringify({ features: [] }) }) as T;
      }
      return Promise.resolve({ status: 200, data: '{}' }) as T;
    }
    if (cmd === 'db_get_auth') return Promise.resolve(null) as T;
    return Promise.resolve({} as T);
  }
  const startTime = performance.now();

  const sanitizeArgs = (originalArgs?: Record<string, unknown>) => {
    if (!originalArgs) return originalArgs;
    const safe: any = JSON.parse(JSON.stringify(originalArgs));
    if (safe.password) safe.password = '***';
    if (safe.authCookie) safe.authCookie = '***';
    if (safe.cookie) safe.cookie = '***';
    if (safe.options?.headers?.Authorization) safe.options.headers.Authorization = '***';
    if (safe.options?.auth_cookie) safe.options.auth_cookie = '***';
    return safe;
  };

  try {
    const res = await invoke<T>(cmd, args);
    const duration = performance.now() - startTime;
    if (isDebugLogEnabled()) {
      window.dispatchEvent(new CustomEvent('app-debug-log', {
        detail: { type: 'success', cmd, args: sanitizeArgs(args), duration: duration.toFixed(1), response: res, timestamp: new Date().toLocaleTimeString() }
      }));
    }
    return res;
  } catch (error: any) {
    const duration = performance.now() - startTime;
    const errorMsg = error.message || (typeof error === 'string' ? error : "Unknown backend error");
    if (isDebugLogEnabled()) {
      window.dispatchEvent(new CustomEvent('app-debug-log', {
        detail: { type: 'error', cmd, args: sanitizeArgs(args), duration: duration.toFixed(1), error: errorMsg, timestamp: new Date().toLocaleTimeString() }
      }));
    }
    throw new Error(errorMsg);
  }
}

export interface RequestOptions {
    method?: string;
    params?: any;
    headers?: any;
    authCookie?: string;
    suppressAuthExpired?: boolean; // 如果为 true，即使 401 也不 dispatch vrc-auth-expired
    [key: string]: any;
}

export async function getStoredAuthCookie(): Promise<string | null> {
  try {
    const storedCookie = await invoke<string | null>('db_get_auth');
    return storedCookie ? normalizeAuthCookieJson(storedCookie) : null;
  } catch {
    return null;
  }
}

/**
 * 基础请求函数，对齐 VrcDog 的 request.js 逻辑
 */
export async function request<T = any>(url: string, options: RequestOptions = {}): Promise<T> {
  const method = options.method || 'GET';
  let reqUrl = buildVrchatApiUrl(url);
  let bodyStr = null;
  const headers: any = { ...options.headers };

  if (options.params && method !== 'GET') {
    headers['Content-Type'] = 'application/json;charset=utf-8';
    bodyStr = JSON.stringify(options.params);
  } else if (options.params && method === 'GET') {
    const params = new URLSearchParams();
    for (const key in options.params) {
      if (options.params[key] !== undefined && options.params[key] !== null) {
        params.append(key, options.params[key].toString());
      }
    }
    const qs = params.toString();
    if (qs) {
      reqUrl += (reqUrl.includes('?') ? '&' : '?') + qs;
    }
  }

  // 自动注入认证 Cookie
  let effectiveAuthCookie = options.authCookie;
  if (!effectiveAuthCookie && reqUrl.includes('api.vrchat.cloud')) {
    effectiveAuthCookie = await getStoredAuthCookie() || undefined;
  }
  if (effectiveAuthCookie) {
    effectiveAuthCookie = normalizeAuthCookieJson(effectiveAuthCookie);
  }

  const executeRequest = async (cookie?: string) => {
    const res: any = await safeInvoke('vrc_execute', {
      options: { url: reqUrl, method, headers, body: bodyStr, auth_cookie: cookie }
    });

    if (res.auth_cookie && reqUrl.includes('api.vrchat.cloud')) {
      const mergedCookie = await mergeCookiesAndSave(res.auth_cookie);
      if (mergedCookie) {
        res.auth_cookie = mergedCookie;
      }
    }

    return res;
  };

  try {
    let res = await executeRequest(effectiveAuthCookie);

    let parsed = parseResponseData(res.data);

    // 处理 401 自动重试 (对齐 VrcDog handleAutoLogin 逻辑)
    let errorMessage = extractVrchatErrorMessage(parsed, res.data, res.status);

    if (
      res.status === 401 &&
      reqUrl.includes('api.vrchat.cloud') &&
      !reqUrl.includes('/config') &&
      !isVrchatPermissionError(res.status, reqUrl, errorMessage)
    ) {
      // 先保存 401 响应中可能的 Set-Cookie（VRChat 有时会在 401 中下发刷新后的 cookie）
      if (res.auth_cookie) {
        try { await mergeCookiesAndSave(res.auth_cookie); } catch { /* ignore */ }
      }

      // suppressAuthExpired=true 时不 dispatch vrc-auth-expired 事件，
      // 用于 WebSocket 断连重试时的 /auth 调用——/auth 401 不代表用户认证失效
      const fireAuthExpired = () => {
        if (!options.suppressAuthExpired) {
          window.dispatchEvent(new CustomEvent('vrc-auth-expired'));
        } else {
          console.warn('[Request] 401 suppressed (suppressAuthExpired=true) - not dispatching vrc-auth-expired');
        }
      };

      try {
        const retryCookie = await getStoredAuthCookie();
        if (retryCookie) {
          const retryRes: any = await safeInvoke('vrc_execute', {
            options: { url: reqUrl, method, headers, body: bodyStr, auth_cookie: retryCookie }
          });
          const retryParsed = parseResponseData(retryRes.data);
          const retryMessage = extractVrchatErrorMessage(retryParsed, retryRes.data, retryRes.status);
          if (retryRes.status >= 200 && retryRes.status < 300) {
            res = retryRes;
            if (retryRes.auth_cookie) {
              await mergeCookiesAndSave(retryRes.auth_cookie);
            }
            parsed = retryParsed;
            errorMessage = retryMessage;
          } else {
            if (isVrchatAuthExpired(retryRes.status, reqUrl, retryMessage)) {
              fireAuthExpired();
            }
            res = retryRes;
            parsed = retryParsed;
            errorMessage = retryMessage;
          }
        } else {
          if (isVrchatAuthExpired(res.status, reqUrl, errorMessage)) {
            fireAuthExpired();
          }
        }
      } catch {
        if (isVrchatAuthExpired(res.status, reqUrl, errorMessage)) {
          fireAuthExpired();
        }
      }
    }

    if (res.status >= 200 && res.status < 300) {
      if (
        res.auth_cookie &&
        parsed &&
        typeof parsed === 'object' &&
        !Array.isArray(parsed)
      ) {
        (parsed as any).auth_cookie = res.auth_cookie;
      }
      return parsed;
    } else {
      const code = isVrchatPermissionError(res.status, reqUrl, errorMessage)
        ? 'VRCHAT_PERMISSION_DENIED'
        : isVrchatAuthExpired(res.status, reqUrl, errorMessage)
          ? 'VRCHAT_AUTH_EXPIRED'
          : 'VRCHAT_HTTP_ERROR';
      throw new VrcRequestError(errorMessage, {
        code,
        status: res.status,
        url: reqUrl,
        response: parsed,
      });
    }
  } catch (err: any) {
    throw err;
  }
}
