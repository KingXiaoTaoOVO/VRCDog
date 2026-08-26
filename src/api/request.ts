import { invoke, isTauri } from "@tauri-apps/api/core";
import { mergeCookiesAndSave, normalizeAuthCookieJson } from './cookies';
import { isDebugLogEnabled } from './debugConfig';
import { ServerApi } from './serverClient';

const VRCHAT_API_ORIGIN = 'https://api.vrchat.cloud';
const VRCHAT_API_BASE = `${VRCHAT_API_ORIGIN}/api/1`;
const REPLAY_SAFE_METHODS = new Set(['GET', 'HEAD', 'OPTIONS', 'PUT', 'DELETE']);

export function buildVrchatApiUrl(url: string): string {
  const value = url.trim();
  if (!value) throw new Error('API URL cannot be empty');

  if (/^https?:\/\//i.test(value)) {
    const parsed = new URL(value);
    if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
      throw new Error(`Unsupported API URL protocol: ${parsed.protocol}`);
    }
    return parsed.toString();
  }
  if (/^[a-z][a-z\d+.-]*:/i.test(value) || value.startsWith('//')) {
    throw new Error(`Unsupported API URL: ${value}`);
  }

  const endpoint = value.replace(/^\/+/, '').replace(/^api\/1\/?/i, '');
  return `${VRCHAT_API_BASE}/${endpoint}`;
}

export type VrcRequestErrorCode =
  | 'VRCHAT_AUTH_EXPIRED'
  | 'VRCHAT_PERMISSION_DENIED'
  | 'VRCHAT_RATE_LIMITED'
  | 'VRCHAT_NETWORK_ERROR'
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
  try {
    return new URL(url).hostname.toLowerCase() === 'api.vrchat.cloud';
  } catch {
    return false;
  }
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

function isTransientHttpStatus(status: number): boolean {
  return status === 408
    || status === 425
    || status === 429
    || status === 500
    || status === 502
    || status === 503
    || status === 504
    || (status >= 520 && status <= 524);
}

function isNetworkError(error: unknown): boolean {
  const msg = error instanceof Error ? error.message : String(error);
  return /network|timed?\s*out|timeout|fetch|connection|connect|dns|error sending request|request body|response body|ECONNREFUSED|ENOTFOUND|EAI_AGAIN|EPIPE|socket|tcp|tls|certificate/i.test(msg);
}

function getRetryAfterMs(error: any): number | null {
  const retryAfter = error?.headers?.['retry-after'] ?? error?.response?.headers?.['retry-after'];
  if (retryAfter) {
    const seconds = Number(retryAfter);
    if (!isNaN(seconds) && seconds > 0 && seconds < 300) return seconds * 1000;
  }
  return null;
}

// ==================== Rate Limit Tracking ====================

const rateLimitState = {
  retryAfterMs: 0,
  lastRateLimitTime: 0,
};

const pendingGetRequests = new Map<string, Promise<unknown>>();

function recordRateLimit(retryAfterMs: number) {
  rateLimitState.retryAfterMs = retryAfterMs;
  rateLimitState.lastRateLimitTime = Date.now();
}

function getRateLimitDelay(): number {
  if (!rateLimitState.retryAfterMs) return 0;
  const elapsed = Date.now() - rateLimitState.lastRateLimitTime;
  const remaining = rateLimitState.retryAfterMs - elapsed;
  return remaining > 0 ? remaining : 0;
}

// ==================== Safe Invoke ====================

export async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    console.warn(`[Browser Mode] API Command: ${cmd}`, args);

    if (cmd === 'vrc_execute') {
      const requestUrl = String((args?.options as any)?.url || '');
      const method = String((args?.options as any)?.method || 'GET').toUpperCase();
      const headers = (args?.options as any)?.headers || {};
      const body = (args?.options as any)?.body;
      const authCookie = (args?.options as any)?.auth_cookie;

      const vrchatBase = 'https://api.vrchat.cloud/api/1';
      let apiPath = requestUrl;
      if (apiPath.startsWith(vrchatBase)) {
        apiPath = apiPath.slice(vrchatBase.length);
      }
      apiPath = apiPath.replace(/^\/+/, '');

      if (!apiPath || apiPath === '') {
        return Promise.resolve({ status: 200, data: '{}' } as T);
      }

      try {
        const proxyHeaders: Record<string, string> = {};
        if (typeof headers === 'object') {
          for (const [key, value] of Object.entries(headers)) {
            if (value !== undefined && value !== null) {
              proxyHeaders[key] = String(value);
            }
          }
        }
        if (authCookie && !proxyHeaders['Authorization']) {
          proxyHeaders['Cookie'] = String(authCookie);
        }

        const data = await ServerApi.proxyVrchatApi(apiPath, method, proxyHeaders, body);
        return { status: 200, data: JSON.stringify(data) } as T;
      } catch (err: any) {
        const message = err?.message || 'Proxy request failed';
        return Promise.resolve({
          status: err?.status || 500,
          data: JSON.stringify({ error: { message } }),
        }) as T;
      }
    }

    if (cmd === 'db_get_auth') return Promise.resolve(null) as T;
    if (cmd === 'db_get_setting') return Promise.resolve(null) as T;
    if (cmd === 'db_get_all_settings') return Promise.resolve({}) as T;
    if (cmd === 'db_get_auth') return Promise.resolve(null) as T;

    if (cmd.startsWith('sys_')) {
      return Promise.resolve({} as T);
    }

    if (cmd.startsWith('vrpiano_')) {
      if (cmd === 'vrpiano_midishow_login') {
        const account = String((args as any)?.request?.account || 'preview');
        return Promise.resolve({ state: 'signed_in', message: 'Browser mode', username: account }) as T;
      }
      if (cmd === 'vrpiano_midishow_login_status') {
        return Promise.resolve({ state: 'signed_in', message: 'Browser mode', username: 'preview' }) as T;
      }
      return Promise.resolve({} as T);
    }

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
    suppressAuthExpired?: boolean;
    /** Maximum number of retries for transient errors (default: 2 for network, 1 for 401) */
    maxRetries?: number;
    /** Request timeout in milliseconds (default: 30000) */
    timeoutMs?: number;
    /** Set false for a GET that must bypass the single-flight request cache. */
    dedupe?: boolean;
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

function getGetRequestKey(url: string, options: RequestOptions): string {
  const requestUrl = new URL(buildVrchatApiUrl(url));
  const params = options.params as Record<string, unknown> | undefined;
  for (const key of Object.keys(params || {}).sort()) {
    const value = params?.[key];
    if (value === undefined || value === null) continue;
    if (Array.isArray(value)) {
      for (const item of value) requestUrl.searchParams.append(key, String(item));
    } else if (typeof value === 'object') {
      requestUrl.searchParams.set(key, JSON.stringify(value));
    } else {
      requestUrl.searchParams.set(key, String(value));
    }
  }
  return [
    requestUrl.toString(),
    `timeout=${options.timeoutMs ?? 30000}`,
    `retries=${options.maxRetries ?? 2}`,
    `suppressAuthExpired=${Boolean(options.suppressAuthExpired)}`,
  ].join('|');
}

/**
 * Base request function with retry, timeout, rate limiting, and error classification.
 * Equivalent concurrent GETs share one network operation, matching VRCX's
 * request coalescing and preventing bursty dashboard refreshes.
 */
export function request<T = any>(url: string, options: RequestOptions = {}): Promise<T> {
  const method = (options.method || 'GET').toUpperCase();
  const hasRequestHeaders = Boolean(options.headers && Object.keys(options.headers).length > 0);
  if (method !== 'GET' || options.dedupe === false || options.authCookie || hasRequestHeaders) {
    return requestInternal<T>(url, options);
  }

  let key: string;
  try {
    key = getGetRequestKey(url, options);
  } catch (error) {
    return Promise.reject(error);
  }
  const pending = pendingGetRequests.get(key) as Promise<T> | undefined;
  if (pending) return pending;

  // Bound the in-flight map so a flood of distinct URLs cannot grow it without
  // limit. Each entry is removed once settled, but this guards against
  // pathological cases where a promise never resolves (e.g. a stuck backend).
  const MAX_PENDING_GET_REQUESTS = 300;
  if (pendingGetRequests.size >= MAX_PENDING_GET_REQUESTS) {
    const oldest = pendingGetRequests.keys().next().value;
    if (oldest !== undefined) pendingGetRequests.delete(oldest);
  }

  const operation = requestInternal<T>(url, options);
  pendingGetRequests.set(key, operation);
  const cleanup = () => {
    if (pendingGetRequests.get(key) === operation) pendingGetRequests.delete(key);
  };
  // Using finally() here creates a second rejected promise when the request
  // fails. Register both handlers on the original promise instead.
  void operation.then(cleanup, cleanup);
  return operation;
}

async function requestInternal<T = any>(url: string, options: RequestOptions = {}): Promise<T> {
  const method = (options.method || 'GET').toUpperCase();
  let reqUrl = buildVrchatApiUrl(url);
  let bodyStr = null;
  const headers: any = { ...options.headers };
  const timeoutMs = Math.min(Math.max(options.timeoutMs ?? 30000, 1000), 120000);

  if (options.params && method !== 'GET') {
    headers['Content-Type'] = 'application/json;charset=utf-8';
    bodyStr = JSON.stringify(options.params);
  } else if (options.params && method === 'GET') {
    const params = new URLSearchParams();
    for (const key in options.params) {
      const val = options.params[key];
      if (val !== undefined && val !== null) {
        if (Array.isArray(val)) {
          for (const item of val) params.append(key, String(item));
        } else if (typeof val === 'object') {
          params.append(key, JSON.stringify(val));
        } else {
          params.append(key, String(val));
        }
      }
    }
    const qs = params.toString();
    if (qs) {
      reqUrl += (reqUrl.includes('?') ? '&' : '?') + qs;
    }
  }

  // Auto-inject auth cookie
  let effectiveAuthCookie = options.authCookie;
  if (!effectiveAuthCookie && isVrchatUrl(reqUrl)) {
    effectiveAuthCookie = await getStoredAuthCookie() || undefined;
  }
  if (effectiveAuthCookie) {
    effectiveAuthCookie = normalizeAuthCookieJson(effectiveAuthCookie);
  }

  const isVrchat = isVrchatUrl(reqUrl);

  const executeRequest = async (cookie?: string) => {
    const res: any = await safeInvoke('vrc_execute', {
      options: {
        url: reqUrl,
        method,
        headers,
        body: bodyStr,
        auth_cookie: cookie,
        timeout_ms: timeoutMs,
      }
    });

    if (!res || typeof res.status !== 'number') {
      throw new Error('Invalid response from native HTTP bridge');
    }
    if (res.auth_cookie && isVrchat) {
      const mergedCookie = await mergeCookiesAndSave(res.auth_cookie);
      if (mergedCookie) {
        res.auth_cookie = mergedCookie;
      }
    }

    return res;
  };

  const canReplay = REPLAY_SAFE_METHODS.has(method) || options.maxRetries !== undefined;
  const maxNetworkRetries = options.maxRetries ?? (REPLAY_SAFE_METHODS.has(method) ? 2 : 0);
  let lastError: any = null;

  // Main request loop with retries for transient errors
  for (let attempt = 0; attempt <= maxNetworkRetries; attempt++) {
    // Wait if rate limited
    const rateLimitWait = isVrchat ? getRateLimitDelay() : 0;
    if (rateLimitWait > 0) {
      await new Promise(resolve => setTimeout(resolve, rateLimitWait));
    }

    try {
      let res = await executeRequest(effectiveAuthCookie);
      let parsed = parseResponseData(res.data);
      let errorMessage = extractVrchatErrorMessage(parsed, res.data, res.status);

      // Handle 429 Rate Limit
      if (res.status === 429 && isVrchat) {
        const retryAfterMs = getRetryAfterMs(res) || 5000;
        recordRateLimit(retryAfterMs);
        if (attempt < maxNetworkRetries) {
          await new Promise(resolve => setTimeout(resolve, retryAfterMs));
          continue;
        }
        throw new VrcRequestError(errorMessage || 'Rate limit exceeded', {
          code: 'VRCHAT_RATE_LIMITED',
          status: 429,
          url: reqUrl,
          response: parsed,
        });
      }

      // Handle transient server errors with retry
      if (isTransientHttpStatus(res.status) && canReplay && attempt < maxNetworkRetries) {
        const delay = Math.min(1000 * Math.pow(2, attempt), 10000);
        await new Promise(resolve => setTimeout(resolve, delay));
        continue;
      }

      // Handle 401 auto-retry
      if (
        res.status === 401 &&
        isVrchat &&
        !reqUrl.includes('/config') &&
        !isVrchatPermissionError(res.status, reqUrl, errorMessage)
      ) {
        if (res.auth_cookie) {
          try { await mergeCookiesAndSave(res.auth_cookie); } catch { /* ignore */ }
        }

        const fireAuthExpired = () => {
          if (!options.suppressAuthExpired) {
            window.dispatchEvent(new CustomEvent('vrc-auth-expired'));
          }
        };

        try {
          const retryCookie = await getStoredAuthCookie();
          if (retryCookie) {
            const retryRes: any = await safeInvoke('vrc_execute', {
              options: {
                url: reqUrl,
                method,
                headers,
                body: bodyStr,
                auth_cookie: retryCookie,
                timeout_ms: timeoutMs,
              }
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

      // Success
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
      }

      // Non-retryable error - classify and throw
      if (isVrchatPermissionError(res.status, reqUrl, errorMessage)) {
        throw new VrcRequestError(errorMessage, {
          code: 'VRCHAT_PERMISSION_DENIED',
          status: res.status,
          url: reqUrl,
          response: parsed,
        });
      }
      if (isVrchatAuthExpired(res.status, reqUrl, errorMessage)) {
        throw new VrcRequestError(errorMessage, {
          code: 'VRCHAT_AUTH_EXPIRED',
          status: res.status,
          url: reqUrl,
          response: parsed,
        });
      }
      throw new VrcRequestError(errorMessage, {
        code: 'VRCHAT_HTTP_ERROR',
        status: res.status,
        url: reqUrl,
        response: parsed,
      });

    } catch (err: any) {
      lastError = err;

      // If it's already a VrcRequestError (non-transient), rethrow immediately
      if (err instanceof VrcRequestError) {
        if (err.code === 'VRCHAT_AUTH_EXPIRED' || err.code === 'VRCHAT_PERMISSION_DENIED' || err.code === 'VRCHAT_RATE_LIMITED') {
          throw err;
        }
        // Transient HTTP errors were already retried above, so throw
        throw err;
      }

      // Network errors - retry with backoff
      if (isNetworkError(err) && canReplay && attempt < maxNetworkRetries) {
        const delay = Math.min(1000 * Math.pow(2, attempt), 10000);
        console.warn(`[Request] Network error, retrying in ${delay}ms (attempt ${attempt + 1}/${maxNetworkRetries})`, err.message);
        await new Promise(resolve => setTimeout(resolve, delay));
        continue;
      }

      // AbortError = timeout
      if (err?.name === 'AbortError' || err?.message?.includes('abort')) {
        throw new VrcRequestError(`Request timed out after ${timeoutMs}ms`, {
          code: 'VRCHAT_NETWORK_ERROR',
          url: reqUrl,
        });
      }

      // Unknown error
      throw new VrcRequestError(err.message || 'Unknown request error', {
        code: 'VRCHAT_NETWORK_ERROR',
        url: reqUrl,
      });
    }
  }

  // Should not reach here, but just in case
  throw lastError || new VrcRequestError('Request failed after retries', {
    code: 'VRCHAT_NETWORK_ERROR',
    url: reqUrl,
  });
}
