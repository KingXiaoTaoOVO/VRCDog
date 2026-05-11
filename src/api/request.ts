import { invoke, isTauri } from "@tauri-apps/api/core";

// [VRCX 对齐] Cookie 合并工具 — VRCX 的 CookieContainer 自动合并同名 cookie
async function mergeCookiesAndSave(newCookieJson: string | null | undefined): Promise<void> {
  if (!newCookieJson) return;
  try {
    const newCookies: string[] = JSON.parse(newCookieJson);
    if (!Array.isArray(newCookies) || newCookies.length === 0) return;

    let existing: string[] = [];
    try {
      const stored = await invoke<string | null>('db_get_auth');
      if (stored) {
        const parsed = JSON.parse(stored);
        if (Array.isArray(parsed)) existing = parsed;
      }
    } catch { /* no existing cookies */ }

    const cookieMap = new Map<string, string>();
    for (const c of existing) {
      const name = c.split('=')[0];
      if (name) cookieMap.set(name, c);
    }
    for (const c of newCookies) {
      const name = c.split('=')[0];
      if (name) cookieMap.set(name, c);
    }

    const merged = Array.from(cookieMap.values());
    await invoke('db_save_auth', { cookie: JSON.stringify(merged) });
  } catch { /* ignore merge errors */ }
}

export async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    console.warn(`[Browser Mock] API Command: ${cmd}`, args);
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
    window.dispatchEvent(new CustomEvent('app-debug-log', {
      detail: { type: 'success', cmd, args: sanitizeArgs(args), duration: duration.toFixed(1), response: res, timestamp: new Date().toLocaleTimeString() }
    }));
    return res;
  } catch (error: any) {
    const duration = performance.now() - startTime;
    const errorMsg = error.message || (typeof error === 'string' ? error : "Unknown backend error");
    window.dispatchEvent(new CustomEvent('app-debug-log', {
      detail: { type: 'error', cmd, args: sanitizeArgs(args), duration: duration.toFixed(1), error: errorMsg, timestamp: new Date().toLocaleTimeString() }
    }));
    throw new Error(errorMsg);
  }
}

export interface RequestOptions {
    method?: string;
    params?: any;
    headers?: any;
    authCookie?: string;
    [key: string]: any;
}

/**
 * 基础请求函数，对齐 VRCX 的 request.js 逻辑
 */
export async function request<T = any>(url: string, options: RequestOptions = {}): Promise<T> {
  const method = options.method || 'GET';
  let reqUrl = url.startsWith('http') ? url : `https://api.vrchat.cloud/api/1${url}`;
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

  const executeRequest = async (cookie?: string) => {
    const res: any = await safeInvoke('vrc_execute', {
      options: { url: reqUrl, method, headers, body: bodyStr, auth_cookie: cookie }
    });

    if (res.auth_cookie && reqUrl.includes('api.vrchat.cloud')) {
      await mergeCookiesAndSave(res.auth_cookie);
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
      const errMsg = parsed?.error?.message || '';
      if (errMsg.includes('Missing Credentials') || errMsg.includes('missing credentials')) {
        try {
          const savedCookie = await invoke<string | null>('db_get_auth');
          if (savedCookie) {
            const retryRes: any = await safeInvoke('vrc_execute', {
              options: { url: reqUrl, method, headers, body: bodyStr, auth_cookie: savedCookie }
            });
            if (retryRes.status >= 200 && retryRes.status < 300) {
              res = retryRes;
              if (res.data) {
                try { parsed = JSON.parse(res.data); } catch { parsed = res.data; }
              }
            } else {
              window.dispatchEvent(new CustomEvent('vrc-auth-expired'));
            }
          } else {
            window.dispatchEvent(new CustomEvent('vrc-auth-expired'));
          }
        } catch { /* ignore */ }
      }
    }

    if (res.status >= 200 && res.status < 300) {
      return parsed;
    } else {
      const err = parsed?.error?.message || `HTTP ${res.status}: ${res.data}`;
      throw new Error(err);
    }
  } catch (err: any) {
    throw err;
  }
}
