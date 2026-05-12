import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { VrcApi, DbApi } from '../api';
import { initWebsocket, closeWebSocket } from '../api/websocket';
import { initGamelogWatcher, stopGamelogWatcher } from '../api/gamelogWatcher';
import { isTauri } from '@tauri-apps/api/core';
import type { VrcUser } from '../types/vrc';
import i18n from '../i18n';
import { useUiStore } from './uiStore';

export const useAuthStore = defineStore('auth', () => {
  const uiStore = useUiStore();
  const t = i18n.global.t;

  const appRole = ref<'client' | 'server' | null>(null);
  const isLoggedIn = ref(false);
  const currentUser = ref<VrcUser | null>(null);
  const autoLoginLoading = ref(false);
  const clientServerUrl = ref<string>('');
  const banMessage = ref<string>('');
  
  const serverConnected = ref(true);
  const reconnectCountdown = ref(0);
  let heartbeatTimer: ReturnType<typeof setInterval> | null = null;
  let consecutiveFailures = 0;
  let isFetchingHeartbeat = false;

  const getBaseUrl = () => clientServerUrl.value.replace(/\/+$/, '');

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
      try {
        await VrcApi.logout();
        await DbApi.clearAuth();
      } catch {}
    }
    if (heartbeatTimer) { clearInterval(heartbeatTimer); heartbeatTimer = null; }
    serverConnected.value = true; // reset
    consecutiveFailures = 0;
    currentUser.value = null;
    isLoggedIn.value = false;
    uiStore.activeTab = 'social';
    closeWebSocket();
    stopGamelogWatcher();
  };

  const registerWithServer = async (user: any) => {
    if (!clientServerUrl.value) return;
    try {
      const payload = {
        user_id: user.id || user.displayName,
        display_name: user.displayName || '',
        avatar_url: user.currentAvatarThumbnailImageUrl || ''
      };
      const data = await VrcApi.request(`${getBaseUrl()}/api/client/register`, { method: 'POST', params: payload });
      
      serverConnected.value = true;
      consecutiveFailures = 0;
      reconnectCountdown.value = 0;
      if (data.status === 'banned') {
        banMessage.value = `Account Banned! Reason: ${data.reason}${data.duration_hours ? t('auto_edf6fe7c') + data.duration_hours + t('auto_2de0d491') : t('auto_6280ae83')}`;
        handleLogout(true);
        return false;
      } else if (data.status === 'frozen') {
        banMessage.value = `Account Frozen! Reason: ${data.reason}`;
        handleLogout(true);
        return false;
      }
    } catch (err) {
      console.warn(t('auto_149c8616'), err);
    }
    return true;
  };

  const startHeartbeat = () => {
    if (heartbeatTimer) clearInterval(heartbeatTimer);
    let normalTick = 0;
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
        if (normalTick < 2) return; // Change to send heartbeat every 2 seconds
        normalTick = 0;
      }

      isFetchingHeartbeat = true;
      try {
        const timeoutPromise = new Promise((_, reject) => setTimeout(() => reject(new Error('Heartbeat timeout')), 3000));
        const heartbeatPromise = VrcApi.request(`${getBaseUrl()}/api/client/heartbeat`, {
          method: 'POST',
          params: {
            user_id: currentUser.value.id || currentUser.value.displayName
          }
        });
        const data: any = await Promise.race([heartbeatPromise, timeoutPromise]);
        
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

  const syncInitialFriends = async () => {
    if (!isLoggedIn.value) return;
    try {
      const onlineFriends = await VrcApi.getFriends({ n: 100, offset: 0, offline: false });
      const offlineFriends = await VrcApi.getFriends({ n: 100, offset: 0, offline: true });
      const allFriends = [...onlineFriends, ...offlineFriends];
      
      if (allFriends.length > 0 && isTauri()) {
        await DbApi.batchSaveFriends({ friendsJson: JSON.stringify(allFriends) });
        if (onlineFriends.length > 0) {
          await DbApi.batchRecordFriends({ friendsJson: JSON.stringify(onlineFriends) });
        }
      }
      window.dispatchEvent(new CustomEvent('vrc-friends-synced'));
    } catch (err) {
      console.warn(t('auto_1d37aaa9'), err);
    }
  };

  const syncInitialNotifications = async () => {
    if (!isLoggedIn.value) return;
    try {
      const notifs = await VrcApi.getNotifications({ n: 100, offset: 0 });
      if (notifs && notifs.length > 0 && isTauri()) {
        await DbApi.batchSaveNotifications({ notificationsJson: JSON.stringify(notifs) });
      }
      window.dispatchEvent(new CustomEvent('vrc-notifications-synced'));
    } catch (err) {
      console.warn(t('auto_fd188e97'), err);
    }
  };

  const handleLoginSuccess = async (user: any) => {
    currentUser.value = user;
    
    DbApi.saveSetting({
      key: 'cached_vrc_user',
      value: JSON.stringify({ user: user, expiresAt: Date.now() + 7 * 24 * 60 * 60 * 1000 })
    }).catch(() => {});

    const allowed = await registerWithServer(user);
    if (allowed === false) { currentUser.value = null; return; }
    isLoggedIn.value = true;
    startHeartbeat();

    if (isTauri()) {
      import('@tauri-apps/api/event').then(({ listen }) => {
        listen('client_kicked', (e: any) => {
          if (e.payload?.user_id === (currentUser.value?.id || currentUser.value?.displayName)) {
            banMessage.value = t('auto_e1b5d9e2');
            handleLogout(true);
          }
        });
        listen('client_frozen', (e: any) => {
          if (e.payload?.user_id === (currentUser.value?.id || currentUser.value?.displayName)) {
            banMessage.value = `Account Frozen! Reason: ${e.payload.reason || t('auto_1622dc9b')}`;
            handleLogout(true);
          }
        });
        listen('client_banned', (e: any) => {
          if (e.payload?.user_id === (currentUser.value?.id || currentUser.value?.displayName)) {
            banMessage.value = `Account Banned! Reason: ${e.payload.reason || t('auto_1622dc9b')}`;
            handleLogout(true);
          }
        });
      });
    }

    await uiStore.fetchServerFeatures(getBaseUrl(), user);
    await initWebsocket();
    initGamelogWatcher();
    syncInitialFriends();
    syncInitialNotifications();
  };

  const tryAutoLogin = async () => {
    autoLoginLoading.value = true;
    try {
      if (!isTauri()) { autoLoginLoading.value = false; return; }
      
      try {
        const allSettings = await DbApi.getAllSettings() as Record<string, unknown>;
        if (allSettings && typeof allSettings === 'object' && allSettings.language) {
          i18n.global.locale.value = allSettings.language as any;
        }
      } catch {}

      const savedCookie = await DbApi.getAuth();
      if (!savedCookie) { autoLoginLoading.value = false; return; }

      await VrcApi.fetchConfig();

      const res = await VrcApi.login({
        username: null,
        password: null,
        authCookie: savedCookie
      });

      if (res.current_user) {
        currentUser.value = res.current_user;
        
        DbApi.saveSetting({
          key: 'cached_vrc_user',
          value: JSON.stringify({ user: res.current_user, expiresAt: Date.now() + 7 * 24 * 60 * 60 * 1000 })
        }).catch(() => {});

        const allowed = await registerWithServer(res.current_user);
        if (allowed === false) { currentUser.value = null; autoLoginLoading.value = false; return; }
        isLoggedIn.value = true;
        startHeartbeat();
        await uiStore.fetchServerFeatures(getBaseUrl(), res.current_user);
        
        if (res.auth_cookie) {
          try {
            const nc: string[] = JSON.parse(res.auth_cookie);
            let ex: string[] = [];
            try { const s = await DbApi.getAuth(); if (s) { const p = JSON.parse(s); if (Array.isArray(p)) ex = p; } } catch {}
            const m = new Map<string, string>();
            for (const c of ex) { const n = c.split('=')[0]; if (n) m.set(n, c); }
            for (const c of nc) { const n = c.split('=')[0]; if (n) m.set(n, c); }
            await DbApi.saveAuth({ cookie: JSON.stringify(Array.from(m.values())) });
          } catch {
            await DbApi.saveAuth({ cookie: res.auth_cookie });
          }
        }
        await initWebsocket();
        initGamelogWatcher();
        syncInitialFriends();
        syncInitialNotifications();
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
              await uiStore.fetchServerFeatures(getBaseUrl(), cachedData.user);
              await initWebsocket();
              initGamelogWatcher();
              syncInitialFriends();
              syncInitialNotifications();
              
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
            await initWebsocket();
            initGamelogWatcher();
            syncInitialFriends();
            syncInitialNotifications();
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
    serverConnected,
    reconnectCountdown,
    getBaseUrl,
    disconnectFromServer,
    handleLogout,
    handleLoginSuccess,
    tryAutoLogin,
    syncInitialFriends,
    startHeartbeat
  };
});
