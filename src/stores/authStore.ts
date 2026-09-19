import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { VrcApi, DbApi, SysApi } from '../api';
import { initWebsocket, closeWebSocket } from '../api/websocket';
import { initGamelogWatcher, stopGamelogWatcher } from '../api/gamelogWatcher';
import { isTauri, invoke } from '@tauri-apps/api/core';
import type { VrcUser } from '../types/vrc';
import { setAppLocale, translate } from '../i18n';
import { normalizeNotificationForDb } from '../api/notificationNormalization';
import { useUiStore } from './uiStore';
import { useFriendsStore } from './friendsStore';
import { mergeCookiesAndSave } from '../api/cookies';
import { markDataHealthy } from './dataHealth';

export const useAuthStore = defineStore('auth', () => {
  const uiStore = useUiStore();
  const t = translate;

  const appRole = ref<'client' | 'server' | null>(null);
  const isLoggedIn = ref(false);
  const currentUser = ref<VrcUser | null>(null);
  const autoLoginLoading = ref(false);
  const clientServerUrl = ref<string>('');
  const banMessage = ref<string>('');
  const pendingSurveyCount = ref(0);
  const surveyRequired = ref(false);
  /// L2: vrcdog-server 下发的 client token，用于后续请求鉴权
  const clientToken = ref<string>('');

  const serverConnected = ref(true);
  const reconnectCountdown = ref(0);
  let heartbeatTimer: ReturnType<typeof setInterval> | null = null;
  let consecutiveFailures = 0;
  let isFetchingHeartbeat = false;
  let serverEventsRegistered = false;
  let serverRegisterInFlight: Promise<boolean | undefined> | null = null;
  let serverRegisterRetryAt = 0;
  let lastServerRegisterError = '';

  const getBaseUrl = () => clientServerUrl.value.replace(/\/+$/, '');
  const getClientTokenHeader = () => {
    const token = clientToken.value;
    return token ? { 'x-vrcdog-client-token': token } : {};
  };

  const pushSurveyGateToRust = () => {
    if (!isTauri()) return;
    const status = surveyRequired.value ? 'survey_required' : (pendingSurveyCount.value > 0 ? 'survey_available' : 'ok');
    invoke('ovr_set_survey_gate', { status, pending: pendingSurveyCount.value }).catch(() => {});
  };

  const applySurveyStatus = (data: any) => {
    pendingSurveyCount.value = Number(data?.pending_survey_count || 0);
    surveyRequired.value = Boolean(data?.survey_required || data?.status === 'survey_required');
    pushSurveyGateToRust();
  };

  const resolveSurveyPrompt = (pendingCount = 0, required = false) => {
    pendingSurveyCount.value = Math.max(0, pendingCount);
    surveyRequired.value = required;
    pushSurveyGateToRust();
  };

  const normalizeServerEventUserId = (payload: any): string => {
    if (typeof payload === 'string') return payload;
    return payload?.user_id || payload?.userId || '';
  };

  const isCurrentClientEvent = (payload: any): boolean => {
    const userId = normalizeServerEventUserId(payload);
    // 安全：以权威 user.id 判定当前客户端，避免用可伪造的 displayName 误匹配/被冒名触发登出
    const currentId = currentUser.value?.id || '';
    return appRole.value === 'client' && Boolean(userId) && Boolean(currentId) && userId === currentId;
  };

  const ensureServerEventListeners = async () => {
    if (!isTauri() || serverEventsRegistered) return;
    serverEventsRegistered = true;
    const { listen } = await import('@tauri-apps/api/event');

    await listen('client_kicked', (e: any) => {
      if (isCurrentClientEvent(e.payload)) {
        banMessage.value = t('auto_e1b5d9e2');
        handleLogout(true);
      }
    });
    await listen('client_frozen', (e: any) => {
      if (isCurrentClientEvent(e.payload)) {
        banMessage.value = `Account Frozen! Reason: ${e.payload?.reason || t('auto_1622dc9b')}`;
        handleLogout(true);
      }
    });
    await listen('client_banned', (e: any) => {
      if (isCurrentClientEvent(e.payload)) {
        banMessage.value = `Account Banned! Reason: ${e.payload?.reason || t('auto_1622dc9b')}`;
        handleLogout(true);
      }
    });
  };

  const disconnectFromServer = async () => {
    if (!clientServerUrl.value || !currentUser.value) return;
    try {
      await VrcApi.request(`${getBaseUrl()}/api/client/disconnect`, {
        method: 'POST',
        params: {
          user_id: currentUser.value.id || currentUser.value.displayName
        },
        headers: getClientTokenHeader(),
        allowExternalHost: true,
      });
    } catch { /* ignore */ }
  };

  const handleLogout = async (keepVrcAuth: boolean = false) => {
    await disconnectFromServer();
    if (!keepVrcAuth) {
      // Check if there are saved accounts; if so, don't call VRChat logout API
      // because logout would invalidate the server-side cookie, breaking one-click login for saved accounts
      let hasSavedAccounts = false;
      try {
        const raw = await DbApi.getSetting({ key: 'savedAccounts' });
        if (raw) {
          const accounts = JSON.parse(raw);
          hasSavedAccounts = Array.isArray(accounts) && accounts.length > 0;
        }
      } catch {}

      if (!hasSavedAccounts) {
        // No saved accounts, normal logout to invalidate cookie
        try { await VrcApi.logout(); } catch {}
      }
      // Only clear local auth storage; savedAccounts with cookies remain intact
      try { await DbApi.clearAuth(); } catch {}
    }
    if (heartbeatTimer) { clearInterval(heartbeatTimer); heartbeatTimer = null; }
    serverConnected.value = true; // reset
    consecutiveFailures = 0;
    currentUser.value = null;
    isLoggedIn.value = false;
    resolveSurveyPrompt();
    // ⚠️ Key fix: only return to role selection on full user logout (keepVrcAuth=false)
    // Auth expiry/kick/ban (keepVrcAuth=true) only returns to login page, preserving role choice
    if (!keepVrcAuth) {
      appRole.value = null;
      clientServerUrl.value = '';
      // 完全登出：清除持久化角色，下次启动回到角色选择界面（而非自动登录）
      try {
        void DbApi.saveSetting({ key: 'appRole', value: 'null' });
      } catch {}
    }
    uiStore.activeTab = 'social';
    closeWebSocket();
    stopGamelogWatcher();
  };

  const registerWithServer = (user: any): Promise<boolean | undefined> => {
    if (!clientServerUrl.value) return Promise.resolve(undefined);
    if (serverRegisterInFlight) return serverRegisterInFlight;
    if (Date.now() < serverRegisterRetryAt) return Promise.resolve(undefined);

    serverRegisterInFlight = (async () => {
      try {
      const payload: any = {
        user_id: user.id || user.displayName,
        display_name: user.displayName || '',
        avatar_url: user.currentAvatarThumbnailImageUrl || ''
      };
      const savedCookie = await DbApi.getAuth();
      if (savedCookie) {
        payload.auth_cookie = savedCookie;
      }
      const data = await VrcApi.request(`${getBaseUrl()}/api/client/register`, {
        method: 'POST',
        params: payload,
        timeoutMs: 5000,
        maxRetries: 1,
        allowExternalHost: true,
      });

      serverConnected.value = true;
      serverRegisterRetryAt = 0;
      lastServerRegisterError = '';
      consecutiveFailures = 0;
      reconnectCountdown.value = 0;
      if (data.client_token) {
        clientToken.value = data.client_token;
      }
      applySurveyStatus(data);
      if (data.status === 'banned') {
        banMessage.value = `Account Banned! Reason: ${data.reason}${data.duration_hours ? t('auto_edf6fe7c') + data.duration_hours + t('auto_2de0d491') : t('auto_6280ae83')}`;
        handleLogout(true);
        return false;
      } else if (data.status === 'frozen') {
        banMessage.value = `Account Frozen! Reason: ${data.reason}`;
        handleLogout(true);
        return false;
      } else if (data.status === 'kicked') {
        banMessage.value = t('auto_e1b5d9e2');
        handleLogout(true);
        return false;
      }
      } catch (err) {
      serverRegisterRetryAt = Date.now() + 30_000;
      serverConnected.value = false;
      const message = err instanceof Error ? err.message : String(err);
      if (message !== lastServerRegisterError) {
        console.warn(t('auto_149c8616'), err);
        lastServerRegisterError = message;
      }
      // The optional VRCDog server must not prevent a successful VRChat login.
      return undefined;
      }
    })();

    void serverRegisterInFlight.then(() => {
      serverRegisterInFlight = null;
    }, () => {
      serverRegisterInFlight = null;
    });
    return serverRegisterInFlight;
  };

  const updateClientServerUrl = async (url: string, reconnect = true) => {
    let normalized = url.trim();
    if (!normalized) throw new Error(t('role.error_require_url'));
    if (!/^https?:\/\//i.test(normalized)) normalized = `http://${normalized}`;
    normalized = normalized.replace('0.0.0.0', '127.0.0.1').replace(/\/+$/, '');

    // S6: 向用户自配服务端发送 VRChat 会话 Cookie 属敏感凭据外发。
    // 非本机地址强制要求 HTTPS，禁止以明文 http 暴露会话令牌（可被同网段嗅探/中间人窃取）。
    try {
      const parsed = new URL(normalized);
      const host = parsed.hostname.toLowerCase();
      const isLocal =
        host === 'localhost' ||
        host === '127.0.0.1' ||
        host === '[::1]' ||
        host === '::1';
      if (parsed.protocol === 'http:' && !isLocal) {
        throw new Error(
          '远程 VRCDog 服务端必须使用 HTTPS：否则你的 VRChat 会话 Cookie 会以明文发送，存在被窃取风险。本地开发可使用 http://localhost / http://127.0.0.1。'
        );
      }
    } catch (e) {
      // 仅拦截我们主动抛出的 HTTPS 安全错误；其余（非法 URL 解析等）交给后续 pingServer 统一处理
      if (e instanceof Error && e.message.includes('HTTPS')) throw e;
    }

    await SysApi.pingServer({ url: normalized });

    clientServerUrl.value = normalized;
    await Promise.allSettled([
      SysApi.saveClientServerConfig({ serverUrl: normalized }),
      DbApi.saveSetting({ key: 'clientServerUrl', value: JSON.stringify(normalized) }),
    ]);

    consecutiveFailures = 0;
    reconnectCountdown.value = 0;
    if (!reconnect || !currentUser.value) return true;

    serverConnected.value = false;
    const registered = await registerWithServer(currentUser.value);
    if (registered) {
      serverConnected.value = true;
      await uiStore.fetchServerFeatures(getBaseUrl(), currentUser.value);
      startHeartbeat();
    }
    return registered;
  };

  const startHeartbeat = () => {
    if (heartbeatTimer) clearInterval(heartbeatTimer);
    let vrcKeepaliveTick = 0;
    // 心跳以 15s 为间隔直接运行，去掉原先 1s 空转 + 计数跳过的做法，减少无谓定时器唤醒
    heartbeatTimer = setInterval(async () => {
      if (!clientServerUrl.value || !currentUser.value) return;
      if (isFetchingHeartbeat) return;

      if (!serverConnected.value) {
        if (reconnectCountdown.value > 1) {
          reconnectCountdown.value--;
          return;
        }
        reconnectCountdown.value = 0;
      }

      // VRChat API keepalive: call /auth/user every 5 min to prevent session expiry
      vrcKeepaliveTick++;
      if (vrcKeepaliveTick >= 20) { // 20 * 15s = 300s = 5min
        vrcKeepaliveTick = 0;
        try {
          await VrcApi.request('/auth/user', { method: 'GET', suppressAuthExpired: true, timeoutMs: 10000 });
        } catch { /* ignore keepalive errors */ }
      }

      isFetchingHeartbeat = true;
      try {
        const data: any = await VrcApi.request(`${getBaseUrl()}/api/client/heartbeat`, {
          method: 'POST',
          params: {
            user_id: currentUser.value.id || currentUser.value.displayName
          },
          headers: getClientTokenHeader(),
          timeoutMs: 3000,
          maxRetries: 0,
          allowExternalHost: true,
        });

        if (!serverConnected.value) {
           await registerWithServer(currentUser.value);
        }
        serverConnected.value = true;
        consecutiveFailures = 0;
        reconnectCountdown.value = 0;

        if (data.status === 'banned') {
          banMessage.value = `Account Banned! Reason: ${data.reason}${data.duration_hours ? ' for ' + data.duration_hours + ' hours' : ' permanently'}`;
          handleLogout(true);
        } else if (data.status === 'frozen') {
          banMessage.value = `Account Frozen! Reason: ${data.reason}`;
          handleLogout(true);
        } else if (data.status === 'kicked') {
          banMessage.value = t('auto_e1b5d9e2');
          handleLogout(true);
        } else if (data.status === 'register_required') {
          await registerWithServer(currentUser.value);
        }
        if (data.status === 'survey_required' || data.status === 'survey_available' || data.status === 'ok') {
          applySurveyStatus(data);
        }
      } catch (err) {
        console.warn(t('auto_a46150ae'), err);
        consecutiveFailures++;
        if (consecutiveFailures >= 3) {
          if (serverConnected.value) {
              serverConnected.value = false;
              reconnectCountdown.value = 30;
          } else {
              reconnectCountdown.value = 30;
          }
        }
      } finally {
        isFetchingHeartbeat = false;
      }
    }, 15000);
  };

  const doSyncFriends = async (): Promise<VrcUser[]> => {
    const friendsStore = useFriendsStore();
    try {
      const liveFriends = await VrcApi.getAllFriends({ n: 100, offset: 0 });

      if (liveFriends.length > 0 && isTauri()) {
        const onlineFriends = liveFriends.filter((f: VrcUser) => f.location && f.location !== 'offline');
        await DbApi.batchSaveFriends({ friendsJson: JSON.stringify(liveFriends) });
        if (onlineFriends.length > 0) {
          await DbApi.batchRecordFriends({ friendsJson: JSON.stringify(onlineFriends) });
        }
      }
      friendsStore.setFriends(liveFriends);
      if (liveFriends.length > 0) markDataHealthy();
      window.dispatchEvent(new CustomEvent('vrc-friends-synced'));
      return liveFriends;
    } catch (err) {
      friendsStore.setError(err instanceof Error ? err.message : String(err));
      console.warn(t('auto_1d37aaa9'), err);
      window.dispatchEvent(new CustomEvent('vrc-friends-synced'));
      return [] as VrcUser[];
    }
  };

  const syncInitialNotifications = async () => {
    try {
      const [legacyResult, v2Result] = await Promise.allSettled([
        VrcApi.getNotifications({ n: 100, offset: 0 }),
        VrcApi.getNotificationsV2({ n: 100, offset: 0 }),
      ]);
      const notifs = [
        ...(legacyResult.status === 'fulfilled' && Array.isArray(legacyResult.value) ? legacyResult.value : []),
        ...(v2Result.status === 'fulfilled' && Array.isArray(v2Result.value) ? v2Result.value : []),
      ];
      if (notifs.length > 0 && isTauri()) {
        await DbApi.batchSaveNotifications({ notificationsJson: JSON.stringify(notifs.map(normalizeNotificationForDb)) });
      }
      window.dispatchEvent(new CustomEvent('vrc-notifications-synced'));
    } catch (err) {
      console.warn(t('auto_fd188e97'), err);
    }
  };

  /**
   * Kick off friends sync and register the promise with friendsStore
   * BEFORE setting isLoggedIn — this prevents the race condition where
   * DashboardView mounts and fires a redundant API call.
   */
  const startFriendsSync = () => {
    const friendsStore = useFriendsStore();
    const p = doSyncFriends();
    friendsStore.beginSync(p);
    return p;
  };

  const handleLoginSuccess = async (user: any) => {
    currentUser.value = user;

    DbApi.saveSetting({
      key: 'cached_vrc_user',
      value: JSON.stringify({ user: user, expiresAt: Date.now() + 7 * 24 * 60 * 60 * 1000 })
    }).catch(() => {});

    // Load cookies into Rust jar after login for session persistence
    try {
      const cookie = await DbApi.getAuth();
      if (cookie) {
        await VrcApi.loadCookiesOnStartup({ authCookie: cookie });
      }
    } catch { /* ignore */ }

    const allowed = await registerWithServer(user);
    if (allowed === false) { currentUser.value = null; return; }

    // Register friends sync BEFORE isLoggedIn so views see the in-flight promise
    const friendsSyncPromise = startFriendsSync();
    void syncInitialNotifications();

    isLoggedIn.value = true;
    startHeartbeat();
    await ensureServerEventListeners();

    await uiStore.fetchServerFeatures(getBaseUrl(), user);
    initGamelogWatcher();
    await friendsSyncPromise;
    await initWebsocket();
  };

  const tryAutoLogin = async () => {
    autoLoginLoading.value = true;
    try {
      if (!isTauri()) { autoLoginLoading.value = false; return; }

      try {
        const allSettings = await DbApi.getAllSettings() as Record<string, unknown>;
        if (allSettings && typeof allSettings === 'object' && allSettings.language) {
          setAppLocale(String(allSettings.language));
        }
      } catch {}

      const savedCookie = await DbApi.getAuth();
      if (!savedCookie) { autoLoginLoading.value = false; return; }

      // Load saved cookies into the Rust cookie jar on startup
      // This ensures session persistence across app restarts
      try {
        await VrcApi.loadCookiesOnStartup({ authCookie: savedCookie });
      } catch { /* ignore */ }

      await VrcApi.fetchConfig();

      const res = await VrcApi.login({
        username: null,
        password: null,
        authCookie: savedCookie
      });

      // Normalize: VRChat API may return user as current_user, currentUser, or directly as res (with res.id)
      const autoLoginUser = res.current_user || res.currentUser || (res.id ? res : null);
      if (autoLoginUser) {
        currentUser.value = autoLoginUser;

        DbApi.saveSetting({
          key: 'cached_vrc_user',
          value: JSON.stringify({ user: autoLoginUser, expiresAt: Date.now() + 7 * 24 * 60 * 60 * 1000 })
        }).catch(() => {});

        const allowed = await registerWithServer(autoLoginUser);
        if (allowed === false) { currentUser.value = null; autoLoginLoading.value = false; return; }
        isLoggedIn.value = true;
        startHeartbeat();
        await ensureServerEventListeners();
        await uiStore.fetchServerFeatures(getBaseUrl(), autoLoginUser);

        if (res.auth_cookie) {
          await mergeCookiesAndSave(res.auth_cookie);
        }
        initGamelogWatcher();

        // Start friends sync BEFORE awaiting it
        const friendsSyncPromise = startFriendsSync();
        void syncInitialNotifications();
        await friendsSyncPromise;
        await initWebsocket();
      } else if (res.error) {
        // A cached profile is display data only. Never promote it to an
        // authenticated session without a successful /auth/user response.
        // Doing so makes the first menu request fail with 401 and appear as a
        // random logout after startup.
        const errMsg = typeof res.error === 'string'
          ? res.error
          : String(res.error?.message || res.error?.details || '');
        if (/missing credentials|invalid credentials|expired|login required|not logged in/i.test(errMsg)) {
          await DbApi.clearAuth();
        }
      }
    } catch (err: any) {
      // A 401 from /auth/user means the saved session is dead; without
      // clearing it the login screen will keep popping back to the role
      // picker on every launch (and global listeners may treat the stale
      // cookie as live).
      const status = err?.status;
      const respMsg = String(
        err?.response?.error?.message ||
        err?.response?.message ||
        err?.response?.details ||
        '',
      ).toLowerCase();
      const looksExpired =
        status === 401 ||
        /missing credentials|invalid credentials|expired|login required|not logged in/.test(respMsg);
      if (looksExpired) {
        try { await DbApi.clearAuth(); } catch { /* ignore */ }
      }
      // Network failures leave the app on the login screen. A cached user is
      // not sufficient to start authenticated API traffic.
      console.warn('[Auth] automatic login verification failed', err);
    } finally {
      autoLoginLoading.value = false;
    }
  };

  /**
   * 启动即自动登录：读取持久化的角色选择，如果是 client 且存在可用凭据
   * （主 auth cookie 或已保存的任一账号 cookie），则直接进入自动登录流程，
   * 跳过“选择角色”界面，无需用户手动点选/刷新即可进入主界面。
   * 若没有任何凭据，则不做任何事，正常显示角色选择界面。
   */
  const restoreAndAutoLogin = async () => {
    if (!isTauri()) return;
    try {
      const rawRole = await DbApi.getSetting({ key: 'appRole' });
      const role = rawRole ? JSON.parse(rawRole) : null;
      if (role !== 'client' && role !== 'server') return;

      if (role === 'server') {
        appRole.value = 'server';
        return;
      }

      // client：恢复此前保存的服务端地址
      try {
        const saved = await DbApi.getSetting({ key: 'clientServerUrl' });
        if (saved) clientServerUrl.value = JSON.parse(saved);
      } catch {}

      // 选取可用凭据：优先主 auth cookie，其次任一已保存账号的 cookie
      let cookie = '';
      try {
        cookie = (await DbApi.getAuth()) || '';
      } catch {}
      if (!cookie || !cookie.trim()) {
        try {
          const raw = await DbApi.getSetting({ key: 'savedAccounts' });
          if (raw) {
            const accounts = JSON.parse(raw);
            if (Array.isArray(accounts)) {
              for (const a of accounts) {
                if (a && a.authCookie && String(a.authCookie).trim()) {
                  cookie = String(a.authCookie);
                  break;
                }
              }
            }
          }
        } catch {}
      }
      if (!cookie || !cookie.trim()) return; // 没有凭据 → 显示角色选择

      // 将选中的 cookie 设为当前 auth，供 tryAutoLogin 使用
      try {
        await DbApi.saveAuth({ cookie });
      } catch {}

      appRole.value = 'client';
      await tryAutoLogin();
    } catch {
      // 任何异常都退回角色选择界面（appRole 保持 null）
    }
  };

  return {
    appRole,
    isLoggedIn,
    currentUser,
    autoLoginLoading,
    clientServerUrl,
    clientToken,
    banMessage,
    pendingSurveyCount,
    surveyRequired,
    serverConnected,
    reconnectCountdown,
    updateClientServerUrl,
    getBaseUrl,
    disconnectFromServer,
    handleLogout,
    handleLoginSuccess,
    tryAutoLogin,
    restoreAndAutoLogin,
    startHeartbeat,
    startFriendsSync,
    resolveSurveyPrompt
  };
});
