import { invoke, isTauri } from "@tauri-apps/api/core";

// [VRCX 对齐] Cookie 合并工具 — VRCX 的 CookieContainer 自动合并同名 cookie
// 我们需要手动合并，确保 auth + twoFactorAuth 永远共存
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

    // 以 cookie name 为 key 合并（新的覆盖旧的同名 cookie）
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

async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    console.warn(`[Browser Mock] API Command: ${cmd}`, args);
    if (cmd === 'vrc_execute') return Promise.resolve({ status: 200, data: '{}' }) as any;
    if (cmd === 'vrc_get_server_status') return Promise.resolve({ status: { description: 'All Systems Operational' } }) as any;
    if (cmd === 'db_get_auth') return Promise.resolve('mock_auth_cookie_abc123') as any;
    if (cmd === 'check_system_status') return Promise.resolve({ hub_installed: true, unity_installed: true, tool_installed: true, vcc_installed: true, alcom_installed: false }) as any;
    if (cmd === 'vrc_fetch_config' || cmd === 'db_getAllSettings' || cmd === 'db_get_all_settings') return Promise.resolve({}) as any;
    if (cmd === 'vrc_get_friends') {
      return Promise.resolve([]) as any;
    }
    if (cmd === 'vrc_search_users' || cmd === 'vrc_search_worlds' || cmd === 'vrc_get_notifications' || cmd === 'vrc_get_avatars' || cmd === 'db_get_status_presets' || cmd === 'vrc_get_latest_gamelogs' || cmd === 'db_get_all_notes') {
      return Promise.resolve([]) as any;
    }
    return Promise.resolve({}) as any;
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
    console.error(`[Tauri API Error] ${cmd}:`, error);
    window.dispatchEvent(new CustomEvent('app-debug-log', {
      detail: { type: 'error', cmd, args: sanitizeArgs(args), duration: duration.toFixed(1), error: errorMsg, timestamp: new Date().toLocaleTimeString() }
    }));
    throw new Error(errorMsg);
  }
}

export const VrcApi = {
  setProxy: (params: { proxyUrl: string | null, authCookie?: string | null }) => safeInvoke<void>('vrc_set_proxy', params),
  getImageBytes: (params: any) => safeInvoke<string>('vrc_get_image_bytes', params),

  request: async (url: string, method: string = 'GET', data?: any, authCookie?: string, customHeaders?: any) => {
    let reqUrl = url.startsWith('http') ? url : `https://api.vrchat.cloud/api/1${url}`;
    let bodyStr = null;
    const headers: any = { ...customHeaders };

    if (data && method !== 'GET') {
      headers['Content-Type'] = 'application/json;charset=utf-8';
      bodyStr = JSON.stringify(data);
    } else if (data && method === 'GET') {
      const params = new URLSearchParams();
      for (const key in data) {
        if (data[key] !== undefined && data[key] !== null) {
          params.append(key, data[key].toString());
        }
      }
      const qs = params.toString();
      if (qs) {
        reqUrl += (reqUrl.includes('?') ? '&' : '?') + qs;
      }
    }

    // [VRCX 对齐] 始终自动注入存储的 auth cookie，就像 VRCX 的全局 CookieContainer
    // VRCX 的 C# HttpClient 通过 CookieContainer 自动为每个请求附带认证 Cookie
    // 我们通过在每次请求时从 DB 读取并注入来达到同样效果
    let effectiveAuthCookie = authCookie;
    if (!effectiveAuthCookie && reqUrl.includes('api.vrchat.cloud')) {
      try {
        const storedCookie = await invoke<string | null>('db_get_auth');
        if (storedCookie) {
          effectiveAuthCookie = storedCookie;
        }
      } catch { /* DB not ready yet, ignore */ }
    }

    const executeRequest = async (cookie?: string) => {
      const res: any = await safeInvoke('vrc_execute', {
        options: { url: reqUrl, method, headers, body: bodyStr, auth_cookie: cookie }
      });

      // [VRCX 对齐] 每次响应中如果有新的 Set-Cookie，自动合并保存到 DB
      // VRCX 通过 CookieContainer 自动合并，我们手动 merge 保持 auth + twoFactorAuth 共存
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

      // [VRCX 对齐] 处理 "Missing Credentials" — VRCX 在 request.js 中检测到此错误时
      // 会触发 authStore.handleAutoLogin() 重新认证
      // 注意：只重试一次，避免无限循环！如果重试仍失败，派发事件通知 UI 强制重新登录
      if (res.status === 401 && reqUrl.includes('api.vrchat.cloud') && !reqUrl.includes('/config')) {
        const errMsg = parsed?.error?.message || '';
        console.warn(`[VrcApi] 401 on ${reqUrl} — ${errMsg}`);
        // 仅对 Missing Credentials 做一次自动重试
        if (errMsg.includes('Missing Credentials') || errMsg.includes('missing credentials')) {
          try {
            const savedCookie = await invoke<string | null>('db_get_auth');
            if (savedCookie) {
              await invoke('vrc_set_proxy', { proxyUrl: null, authCookie: savedCookie });
              const retryRes: any = await safeInvoke('vrc_execute', {
                options: { url: reqUrl, method, headers, body: bodyStr, auth_cookie: savedCookie }
              });
              if (retryRes.status >= 200 && retryRes.status < 300) {
                // 重试成功，用重试结果
                res = retryRes;
                if (res.data) {
                  try { parsed = JSON.parse(res.data); } catch { parsed = res.data; }
                }
              } else {
                // 重试仍失败，Cookie 确实已过期，通知 UI 重新登录
                console.error('[VrcApi] Auth cookie expired, dispatching re-login event');
                window.dispatchEvent(new CustomEvent('vrc-auth-expired'));
              }
            } else {
              window.dispatchEvent(new CustomEvent('vrc-auth-expired'));
            }
          } catch (retryErr) {
            console.warn('[VrcApi] Re-auth retry failed:', retryErr);
          }
        }
      }

      if (res.status >= 200 && res.status < 300) {
        if (parsed && typeof parsed === 'object') {
          parsed._auth_cookie = res.auth_cookie;
        }
        return parsed;
      } else {
        if (parsed && parsed.error && parsed.error.message) {
          throw new Error(parsed.error.message);
        }
        throw new Error(`HTTP ${res.status}: ${res.data}`);
      }
    } catch (err: any) {
      throw new Error(err.message || err);
    }
  },

  getServerStatus: () => VrcApi.request('https://status.vrchat.com/api/v2/status.json', 'GET', null, undefined, { Referer: 'https://vrcx.app' }),
  fetchConfig: () => VrcApi.request('/config'),

  clearCookies: () => safeInvoke('vrc_clear_cookies'),

  login: async (params: any) => {
    const headers: any = {};
    if (params.username && params.password) {
      const b64 = btoa(`${encodeURIComponent(params.username)}:${encodeURIComponent(params.password)}`);
      headers['Authorization'] = `Basic ${b64}`;
    } else if (!params.authCookie) {
      return { error: '必须提供账号密码或Auth Cookie' };
    }
    try {
      const res: any = await safeInvoke('vrc_execute', {
        options: { url: 'https://api.vrchat.cloud/api/1/auth/user', method: 'GET', headers, auth_cookie: params.authCookie }
      });
      const parsed = res.data ? JSON.parse(res.data) : null;
      
      // [VRCX 对齐] 合并保存 Set-Cookie 到 DB（保留已有的 twoFactorAuth 等 Cookie）
      if (res.auth_cookie) {
        await mergeCookiesAndSave(res.auth_cookie);
      }
      
      if (res.status >= 200 && res.status < 300) {
        if (parsed && parsed.requiresTwoFactorAuth) {
          return { requires_two_factor_auth: parsed.requiresTwoFactorAuth, auth_cookie: res.auth_cookie };
        }
        return { current_user: parsed, auth_cookie: res.auth_cookie };
      } else if (res.status === 401) {
        return { error: parsed?.error?.message || (params.authCookie && !params.username ? 'Auth Cookie 无效或已过期，请重新登录' : '账号或密码错误') };
      } else if (res.status === 403) {
        return { error: 'HTTP 403 — VRChat 拒绝连接。可能需要开 VPN 或检查网络。' };
      } else {
        return { error: `HTTP ${res.status}` };
      }
    } catch (err: any) {
      return { error: err.message || err };
    }
  },

  verify2fa: async (params: any) => {
    const endpoint = params.method.toLowerCase() === 'emailotp' ? 'emailotp/verify' : params.method.toLowerCase() === 'otp' ? 'otp/verify' : 'totp/verify';
    // [VRCX 对齐] 2FA 验证时，必须携带 auth cookie 且确保 twoFactorAuth 响应与之合并
    // 从 DB 读取完整的已存 cookie（可能包含 login 阶段的 auth cookie）
    let fullCookie = params.authCookie;
    try {
      const stored = await invoke<string | null>('db_get_auth');
      if (stored) fullCookie = stored;
    } catch {}
    const result = await VrcApi.request(`/auth/twofactorauth/${endpoint}`, 'POST', { code: params.code }, fullCookie);
    return result;
  },

  logout: async () => {
    return safeInvoke('vrc_set_proxy', { proxyUrl: null, authCookie: null });
  },

  getCurrentUser: () => VrcApi.request('/auth/user'),
  getUser: (params: any) => VrcApi.request(`/users/${params.userId || params}`),
  getMutualCounts: (params: any) => VrcApi.request(`/users/${params.userId || params}/mutuals`),
  getMutualFriends: (params: any) => VrcApi.request(`/users/${params.userId || params}/mutuals/friends`, 'GET', { n: params.n || 100, offset: params.offset || 0 }),

  getFriends: (params: any) => {
    const q: any = { n: params.n || 60, offset: params.offset || 0 };
    if (params.offline) q.offline = true;
    return VrcApi.request('/auth/user/friends', 'GET', q);
  },

  searchUsers: (params: any) => VrcApi.request('/users', 'GET', { search: params.query || params.search, n: params.n || 20, offset: params.offset || 0 }),
  searchWorlds: (params: any) => VrcApi.request('/worlds', 'GET', { search: params.query || params.search, n: params.n || 20, offset: params.offset || 0 }),
  searchGroups: (params: any) => VrcApi.request('/groups', 'GET', { search: params.query || params.search, n: params.n || 20, offset: params.offset || 0 }),
  getWorld: (params: any) => VrcApi.request(`/worlds/${params.worldId || params}`),

  updateStatus: (params: any) => VrcApi.request(`/users/${params.userId || params.user_id}`, 'PUT', { status: params.status, statusDescription: params.statusDescription }),

  getNotifications: (params: any) => {
    const q: any = { n: params.n || 60, offset: params.offset || 0 };
    if (params.type) q.type = params.type;
    if (params.hidden !== undefined) q.hidden = params.hidden;
    if (params.after) q.after = params.after;
    return VrcApi.request('/auth/user/notifications', 'GET', q);
  },

  acceptNotification: (params: any) => VrcApi.request(`/auth/user/notifications/${params.notificationId || params}/accept`, 'PUT'),
  hideNotification: (params: any) => VrcApi.request(`/auth/user/notifications/${params.notificationId || params}/hide`, 'PUT'),

  getAvatars: (params: any) => {
    const q: any = { n: params.n || 60, offset: params.offset || 0 };
    if (params.user) { q.user = params.user; q.releaseStatus = 'all'; }
    if (params.search || params.query) { q.search = params.search || params.query; }
    if (params.releaseStatus) { q.releaseStatus = params.releaseStatus; }
    return VrcApi.request('/avatars', 'GET', q);
  },
  getAvatar: (params: any) => VrcApi.request(`/avatars/${params.avatarId || params}`),
  selectAvatar: (params: any) => VrcApi.request(`/avatars/${params.avatarId || params}/select`, 'PUT'),

  getFavorites: (params: any) => {
    const q: any = { n: params.n || 60, offset: params.offset || 0 };
    if (params.type_) q.type = params.type_;
    if (params.type) q.type = params.type;
    return VrcApi.request('/favorites', 'GET', q);
  },
  getFavoriteWorlds: (params: any) => VrcApi.request('/worlds/favorites', 'GET', { n: params?.n || 60, offset: params?.offset || 0 }),
  getFavoriteAvatars: (params: any) => VrcApi.request('/avatars/favorites', 'GET', { n: params?.n || 60, offset: params?.offset || 0 }),

  getGroups: () => VrcApi.request('/users/me/groups'),
  getGroup: (params: any) => VrcApi.request(`/groups/${params.groupId || params}`),
  getModerations: () => VrcApi.request('/auth/user/playermoderations'),

  friendRequest: (params: any) => VrcApi.request(`/user/${params.userId || params}/friendRequest`, 'POST'),
  unfriend: (params: any) => VrcApi.request(`/auth/user/friends/${params.userId || params}`, 'DELETE'),
  inviteMyself: (params: any) => VrcApi.request(`/invite/myself/to/${params.worldId}:${params.instanceId}`, 'POST'),
};

export const DbApi = {
  getAuth: () => safeInvoke<string | null>('db_get_auth'),
  saveAuth: (params: any) => safeInvoke<void>('db_save_auth', params),
  clearAuth: () => safeInvoke<void>('db_clear_auth'),
  getHeatmap: () => safeInvoke<any[]>('db_get_heatmap'),
  getHeatmapDetails: (params: { day: number, hour: number }) => safeInvoke<any[]>('db_get_heatmap_details', params),
  getNotes: () => safeInvoke<any[]>('db_get_all_notes'),
  getNote: (params: any) => safeInvoke<any>('db_get_note', params),
  saveNote: (params: any) => safeInvoke<void>('db_save_note', params),
  getPresets: () => safeInvoke<any[]>('db_get_status_presets'),
  savePreset: (params: any) => safeInvoke<void>('db_save_status_preset', params),
  deletePreset: (params: any) => safeInvoke<void>('db_delete_status_preset', params),
  exportAll: () => safeInvoke<any>('db_export_all'),
  recordActivity: (params: any) => safeInvoke<void>('db_record_activity', params),
  batchRecordFriends: (params: any) => safeInvoke<number>('db_batch_record_friends', params),
  addFriendLog: (params: any) => safeInvoke<void>('db_add_friend_log', params),
  getFriendLogs: (params: any) => safeInvoke<any[]>('db_get_friend_logs', params),
  saveSetting: (params: { key: string; value: string }) => safeInvoke<void>('db_save_setting', params),
  getSetting: (params: { key: string }) => safeInvoke<string | null>('db_get_setting', params),
  getAllSettings: () => safeInvoke<any>('db_get_all_settings'),
  saveFriend: (params: any) => safeInvoke<void>('db_save_friend', params),
  batchSaveFriends: (params: { friendsJson: string }) => safeInvoke<number>('db_batch_save_friends', params),
  getCachedFriends: () => safeInvoke<any[]>('db_get_friends'),
  removeFriend: (params: { userId: string }) => safeInvoke<void>('db_remove_friend', params),
  saveGameLogs: (params: { logsJson: string }) => safeInvoke<number>('db_save_game_logs', params),
  getGameLogs: (params: { limit?: number; offset?: number }) => safeInvoke<any[]>('db_get_game_logs', params),
  saveNotification: (params: { notificationJson: string }) => safeInvoke<void>('db_save_notification', params),
  batchSaveNotifications: (params: { notificationsJson: string }) => safeInvoke<number>('db_batch_save_notifications', params),
  getNotifications: (params: { limit?: number; offset?: number }) => safeInvoke<any[]>('db_get_notifications', params),
  deleteNotification: (params: { id: string }) => safeInvoke<void>('db_delete_notification', params),
  
  // Favorite Worlds endpoints
  addFavoriteWorld: (params: { worldId: string; name: string; imageUrl?: string | null }) => safeInvoke<void>('db_add_favorite_world', params),
  getFavoriteWorlds: () => safeInvoke<any[]>('db_get_favorite_worlds'),
  removeFavoriteWorld: (params: { worldId: string }) => safeInvoke<void>('db_remove_favorite_world', params),

  // Favorite Avatars endpoints
  addFavoriteAvatar: (params: { avatarId: string; name: string; imageUrl?: string | null; authorId?: string | null; authorName?: string | null }) => safeInvoke<void>('db_add_favorite_avatar', params),
  getFavoriteAvatars: () => safeInvoke<any[]>('db_get_favorite_avatars'),
  removeFavoriteAvatar: (params: { avatarId: string }) => safeInvoke<void>('db_remove_favorite_avatar', params),
};

export const SysApi = {
  checkSystemStatus: () => safeInvoke<any>('check_system_status'),
  checkSteamVR: () => safeInvoke<boolean>('sys_check_steamvr'),
  installSoftware: (params: any) => safeInvoke<void>('install_software', params),
  uninstallSoftware: (params: any) => safeInvoke<void>('uninstall_software', params),
  launchSoftware: (params: any) => safeInvoke<void>('launch_software', params),
  clearVrcCache: () => safeInvoke<number>('sys_clear_vrchat_cache'),
  isVrcRunning: () => safeInvoke<boolean>('sys_is_vrchat_running'),
  launchVrc: (params?: { launchArgs?: string }) => safeInvoke<void>('sys_launch_vrchat', params),
  sendOscParam: (params: { address: string; value: number }) => safeInvoke<void>('sys_send_osc_param', params),
  sendOscChatbox: (params: { text: string; complete: boolean }) => safeInvoke<void>('sys_send_osc_chatbox', params),
  setDiscordRpc: (params: { details: string; state: string }) => safeInvoke<void>('sys_set_discord_rpc', params),
  saveTextFile: (params: { path: string; content: string }) => safeInvoke<void>('sys_save_text_file', params),
  startAudioCapture: (params: { sourceLang: string, engine: string }) => safeInvoke<void>('sys_start_audio_capture', params),
  stopAudioCapture: () => safeInvoke<void>('sys_stop_audio_capture'),
  startOscAutomation: () => safeInvoke<void>('sys_start_osc_automation'),
  stopOscAutomation: () => safeInvoke<void>('sys_stop_osc_automation'),
  showInExplorer: (params: { path: string }) => safeInvoke<void>('sys_show_in_explorer', params),
  verifyServerPassword: (params: { password: string }) => safeInvoke<void>('sys_verify_server_password', params),
  startServer: (params: { host: string, port: number }) => safeInvoke<void>('sys_start_server', params),
  stopServer: () => safeInvoke<void>('sys_stop_server'),
  isServerRunning: () => safeInvoke<boolean>('sys_is_server_running'),
  pingServer: (params: { url: string }) => safeInvoke<string>('sys_ping_server', params),
  openNewClient: () => safeInvoke<void>('sys_open_new_client'),
  setAutostart: (params: { enable: boolean }) => safeInvoke<void>('sys_set_autostart', params),
  openDir: (params: { target: string }) => safeInvoke<void>('sys_open_dir', params),
};

export const GamelogApi = {
  getLatestGamelogs: (params: any) => safeInvoke<any[]>('vrc_get_latest_gamelogs', params),
};

export const GalleryApi = {
  getImages: (params?: { limit?: number; offset?: number }) => safeInvoke<any[]>('gallery_get_images', params || {}),
};

export const OvrApi = {
  init: () => safeInvoke<any>('ovr_init'),
  shutdown: () => safeInvoke<void>('ovr_shutdown'),
  getStatus: () => safeInvoke<any>('ovr_get_status'),
  setConfig: (params: { config: any }) => safeInvoke<void>('ovr_set_config', params),
  toggleTranslation: () => safeInvoke<boolean>('ovr_toggle_translation'),
  captureScreenshot: () => safeInvoke<string>('ovr_capture_screenshot'),
  updateOverlayText: (params: { original: string; translated: string }) => safeInvoke<void>('ovr_update_overlay_text', params),
  setOverlayVisible: (params: { visible: boolean }) => safeInvoke<void>('ovr_set_overlay_visible', params),
  clearTranslation: () => safeInvoke<void>('ovr_clear_translation'),
  translate: (params: {
    req: {
      text: string; source_lang: string; target_lang: string;
      service: string; api_key: string; model: string; prompt: string;
    }
  }) => safeInvoke<any>('ovr_translate', params),
  // Desktop mirror translation mode
  desktopScanOnce: () => safeInvoke<void>('ovr_desktop_scan_once'),
  startAutoScan: () => safeInvoke<void>('ovr_start_auto_scan'),
  stopAutoScan: () => safeInvoke<void>('ovr_stop_auto_scan'),
};
