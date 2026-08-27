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

  const serverConnected = ref(true);
  const reconnectCountdown = ref(0);
  let heartbeatTimer: ReturnType<typeof setInterval> | null = null;
  let consecutiveFailures = 0;
  let isFetchingHeartbeat = false;
  let serverEventsRegistered = false;

  const getBaseUrl = () => clientServerUrl.value.replace(/\/+$/, '');

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
    const currentId = currentUser.value?.id || currentUser.value?.displayName || '';
    return appRole.value === 'client' && Boolean(userId) && Boolean(currentId) && (userId === currentId || userId === currentUser.value?.displayName);
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
        }
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
    }
    uiStore.activeTab = 'social';
    closeWebSocket();
    stopGamelogWatcher();
  };

  const registerWithServer = async (user: any) => {
    if (!clientServerUrl.value) return;
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
      });

      serverConnected.value = true;
      consecutiveFailures = 0;
      reconnectCountdown.value = 0;
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
      console.warn(t('auto_149c8616'), err);
      serverConnected.value = false;
      return false;
    }
    return true;
  };

  const updateClientServerUrl = async (url: string, reconnect = true) => {
    let normalized = url.trim();
    if (!normalized) throw new Error(t('role.error_require_url'));
    if (!/^https?:\/\//i.test(normalized)) normalized = `http://${normalized}`;
    normalized = normalized.replace('0.0.0.0', '127.0.0.1').replace(/\/+$/, '');

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
    let normalTick = 0;
    let vrcKeepaliveTick = 0;
    heartbeatTimer = setInterval(async () => {
      if (!clientServerUrl.value || !currentUser.value) return;
      if (isFetchingHeartbeat) return;

      if (!serverConnected.value) {
        if (reconnectCountdown.value > 1) {
          reconnectCountdown.value--;
          return;
        }
        reconnectCountdown.value = 0;
      } else {
        normalTick++;
        if (normalTick < 15) return; // every 15s send heartbeat, reduce request frequency
        normalTick = 0;
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
          timeoutMs: 3000,
          maxRetries: 0,
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
    }, 1000);
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
        const errMsg = res.error || '';

        // Attempt to use cache first if cookie is invalid
        const cachedUserStr = await DbApi.getSetting({ key: 'cached_vrc_user' });
        if (cachedUserStr) {
          try {
            const cachedData = JSON.parse(cachedUserStr);
            if (Date.now() < cachedData.expiresAt) {
              currentUser.value = cachedData.user;
              isLoggedIn.value = true;
              registerWithServer(cachedData.user);
              startHeartbeat();
              await ensureServerEventListeners();
              await uiStore.fetchServerFeatures(getBaseUrl(), cachedData.user);
              initGamelogWatcher();
              await startFriendsSync();
              void syncInitialNotifications();
              await initWebsocket();

              if (errMsg.includes('Missing Credentials') || errMsg.includes(t('auto_1abbb174')) || errMsg.includes(t('auto_584cd195')) || errMsg.includes('expired')) {
                // Keep the cached login, but notify? Optional.
              }
              return;
            } else {
              if (errMsg.includes('Missing Credentials') || errMsg.includes(t('auto_1abbb174')) || errMsg.includes(t('auto_584cd195')) || errMsg.includes('expired')) {
                await DbApi.clearAuth();
              }
            }
          } catch {}
        } else if (errMsg.includes('Missing Credentials') || errMsg.includes(t('auto_1abbb174')) || errMsg.includes(t('auto_584cd195')) || errMsg.includes('expired')) {
          await DbApi.clearAuth();
        }
      }
    } catch (err) {
      try {
        const cachedUserStr = await DbApi.getSetting({ key: 'cached_vrc_user' });
        if (cachedUserStr) {
          const cachedData = JSON.parse(cachedUserStr);
          if (Date.now() < cachedData.expiresAt) {
            currentUser.value = cachedData.user;
            isLoggedIn.value = true;
            registerWithServer(cachedData.user);
            startHeartbeat();
            await ensureServerEventListeners();
            initGamelogWatcher();
            await startFriendsSync();
            void syncInitialNotifications();
            await initWebsocket();
          }
        }
      } catch {}
    } finally {
      autoLoginLoading.value = false;
    }
  };

  return {
    appRole,
    isLoggedIn,
    currentUser,
    autoLoginLoading,
    clientServerUrl,
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
    startHeartbeat,
    startFriendsSync,
    resolveSurveyPrompt
  };
});
