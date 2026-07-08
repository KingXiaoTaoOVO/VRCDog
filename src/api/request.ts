import { invoke, isTauri } from "@tauri-apps/api/core";
import { mergeCookiesAndSave, normalizeAuthCookieJson } from './cookies';
import { isDebugLogEnabled } from './debugConfig';

function buildVrchatApiUrl(url: string): string {
  if (url.startsWith('http')) return url;
  const endpoint = url.replace(/^\/+/, '').replace(/^api\/1\/?/i, '');
  return `https://api.vrchat.cloud/api/1/${endpoint}`;
}

export async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    console.warn(`[Browser Mock] API Command: ${cmd}`, args);
    if (cmd === 'vrc_execute') return Promise.resolve({ status: 200, data: '[]' }) as T;
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

/**
 * 基础请求函数，对齐 VRCX 的 request.js 逻辑
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
    try {
      const storedCookie = await invoke<string | null>('db_get_auth');
      if (storedCookie) {
        effectiveAuthCookie = storedCookie;
      }
    } catch { /* DB not ready */ }
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

    let parsed = null;
    if (res.data) {
      try { parsed = JSON.parse(res.data); } catch { parsed = res.data; }
    }

    // 处理 401 自动重试 (对齐 VRCX handleAutoLogin 逻辑)
    if (res.status === 401 && reqUrl.includes('api.vrchat.cloud') && !reqUrl.includes('/config')) {
      // suppressAuthExpired=true 时不 dispatch vrc-auth-expired 事件，
      // 用于 WebSocket 断连重试时的 /auth 调用——/auth 401 不代表用户认证失效
      const fireAuthExpired = () => {
        if (!options.suppressAuthExpired) {
          window.dispatchEvent(new CustomEvent('vrc-auth-expired'));
        } else {
          console.warn('[Request] 401 suppressed (suppressAuthExpired=true) — not dispatching vrc-auth-expired');
        }
      };

      try {
        const savedCookie = await invoke<string | null>('db_get_auth');
        const retryCookie = savedCookie ? normalizeAuthCookieJson(savedCookie) : null;
        if (retryCookie) {
          const retryRes: any = await safeInvoke('vrc_execute', {
            options: { url: reqUrl, method, headers, body: bodyStr, auth_cookie: retryCookie }
          });
          if (retryRes.status >= 200 && retryRes.status < 300) {
            res = retryRes;
            if (retryRes.auth_cookie) {
              await mergeCookiesAndSave(retryRes.auth_cookie);
            }
            if (res.data) {
              try { parsed = JSON.parse(res.data); } catch { parsed = res.data; }
            }
          } else {
            fireAuthExpired();
          }
        } else {
          fireAuthExpired();
        }
      } catch {
        fireAuthExpired();
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
      const err = parsed?.error?.message || `HTTP ${res.status}: ${res.data}`;
      throw new Error(err);
    }
  } catch (err: any) {
    throw err;
  }
}
