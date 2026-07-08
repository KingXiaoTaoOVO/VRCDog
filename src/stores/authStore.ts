import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { VrcApi, DbApi } from '../api';
import { initWebsocket, closeWebSocket } from '../api/websocket';
import { initGamelogWatcher, stopGamelogWatcher } from '../api/gamelogWatcher';
import { isTauri } from '@tauri-apps/api/core';
import type { VrcUser } from '../types/vrc';
import { setAppLocale, translate } from '../i18n';
import { useUiStore } from './uiStore';
import { mergeCookiesAndSave } from '../api/cookies';

export const useAuthStore = defineStore('auth', () => {
  const uiStore = useUiStore();
  const t = translate;

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
      // 检查是否有已保存的账号，如果有则不调用 VRChat logout API
      // 因为 logout 会让服务器端 cookie 失效，导致保存的账号无法一键登录
      let hasSavedAccounts = false;
      try {
        const raw = await DbApi.getSetting({ key: 'savedAccounts' });
        if (raw) {
          const accounts = JSON.parse(raw);
          hasSavedAccounts = Array.isArray(accounts) && accounts.length > 0;
        }
      } catch {}

      if (!hasSavedAccounts) {
        // 没有保存的账号，正常调用 logout 使 cookie 失效
        try { await VrcApi.logout(); } catch {}
      }
      // 只清除本地 auth 存储，不影响 savedAccounts 中保存的 cookie
      try { await DbApi.clearAuth(); } catch {}
    }
    if (heartbeatTimer) { clearInterval(heartbeatTimer); heartbeatTimer = null; }
    serverConnected.value = true; // reset
    consecutiveFailures = 0;
    currentUser.value = null;
    isLoggedIn.value = false;
    // ⚠️ 关键修复：只有用户主动完全退出（keepVrcAuth=false）才回到角色选择页
    // auth 过期/被踢/被封（keepVrcAuth=true）只回到登录页，保留模式选择
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
        if (normalTick < 15) return; // 每 15 秒发送一次心跳，减少请求频率
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
      // 合并在线+离线好友为一次请求，减少 API 调用
      const allFriends = await VrcApi.getFriends({ n: 100, offset: 0, offline: true });
      
      if (allFriends.length > 0 && isTauri()) {
        const onlineFriends = allFriends.filter((f: VrcUser) => f.location && f.location !== 'offline');
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
          setAppLocale(String(allSettings.language));
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
        await uiStore.fetchServerFeatures(getBaseUrl(), autoLoginUser);
        
        if (res.auth_cookie) {
          await mergeCookiesAndSave(res.auth_cookie);
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
