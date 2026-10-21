import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { VrcApi, DbApi, SysApi } from '../api';
import { initWebsocket, closeWebSocket } from '../api/websocket';
import { initGamelogWatcher, stopGamelogWatcher } from '../api/gamelogWatcher';
import { isTauri, invoke } from '@tauri-apps/api/core';
import type { VrcUser } from '../types/vrc';
import { setAppLocale, translate } from '../i18n';
import { normalizeNotificationForDb } from '../api/notificationNormalization';
import { setVrcdogClientToken } from '../api/clientToken';
import { useUiStore } from './uiStore';
import { useFriendsStore } from './friendsStore';
import { getCookieValue, mergeCookiesAndSave, normalizeAuthCookieJson } from '../api/cookies';
import { markDataHealthy, setDataRefreshIntervalSec } from './dataHealth';

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
  const loginGracePeriodUntil = ref(0);

  const setLoginGracePeriod = (durationMs = 120_000) => {
    loginGracePeriodUntil.value = Date.now() + durationMs;
  };

  const isInLoginGracePeriod = () => {
    return Date.now() < loginGracePeriodUntil.value;
  };

  let heartbeatTimer: ReturnType<typeof setInterval> | null = null;
  let vrcKeepaliveTimer: ReturnType<typeof setInterval> | null = null;
  let consecutiveFailures = 0;
  let isFetchingHeartbeat = false;
  let serverEventsRegistered = false;
  let serverRegisterInFlight: Promise<boolean | undefined> | null = null;
  let serverRegisterRetryAt = 0;
  let lastServerRegisterError = '';
  let reconnectDisplayTimer: ReturnType<typeof setInterval> | null = null;
  let reconnectAt = 0;

  /** 断线后到下一次重连尝试之间的等待时间（毫秒）。界面文案承诺的是「秒」，所以必须以真实时间戳为准。 */
  const RECONNECT_WAIT_MS = 30_000;

  /** 依据截止时间刷新倒计时显示；返回「是否已经到了该重试的时候」。 */
  const syncReconnectCountdown = (): boolean => {
    const remain = reconnectAt - Date.now();
    reconnectCountdown.value = remain > 0 ? Math.ceil(remain / 1000) : 0;
    return remain <= 0;
  };

  const stopReconnectTicker = () => {
    if (reconnectDisplayTimer) {
      clearInterval(reconnectDisplayTimer);
      reconnectDisplayTimer = null;
    }
    reconnectAt = 0;
  };

  /**
   * 进入断线等待窗口：按真实时间戳设定重试时刻，并起一个 1s 定时器。
   * 定时器只做两件事——把「秒」 smoothly 走完（心跳是 15s 一次，不然数字会长时间卡住），
   * 以及到点时立刻发起一次心跳/重连尝试，而不是干等下一个心跳 tick。
   */
  const armReconnectWindow = () => {
    reconnectAt = Date.now() + RECONNECT_WAIT_MS;
    syncReconnectCountdown();
    if (reconnectDisplayTimer) return;
    reconnectDisplayTimer = setInterval(() => {
      if (!clientServerUrl.value || !currentUser.value) return;
      if (serverConnected.value) {
        stopReconnectTicker();
        return;
      }
      if (syncReconnectCountdown() && !isFetchingHeartbeat) void runHeartbeatTick();
    }, 1000);
  };

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
    if (vrcKeepaliveTimer) { clearInterval(vrcKeepaliveTimer); vrcKeepaliveTimer = null; }
    stopReconnectTicker();
    reconnectCountdown.value = 0;
    serverConnected.value = true; // reset
    consecutiveFailures = 0;
    currentUser.value = null;
    isLoggedIn.value = false;
    // 服务端下发的身份必须一起清掉：
    // clientToken 是「上一个账号」的凭证，服务端用它反查 user_id 并与心跳上报的 user_id 比对，
    // 带着旧 token 注册新账号会被判 auth_failed，进而触发全屏断线遮罩且无法自愈。
    // 同理 uiStore 里的菜单/主题/模式权限来自上个账号，登录失败时不会被刷新，
    // 换账号后侧边栏会继续按上个账号的权限过滤功能。
    clientToken.value = '';
    setVrcdogClientToken('', '');
    uiStore.resetServerPerms();
    resolveSurveyPrompt();
    // Key fix: only return to role selection on full user logout (keepVrcAuth=false)
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
    stopDataRefresh();
    closeWebSocket();
    stopGamelogWatcher();
  };

  const registerWithServer = (user: any, force = false): Promise<boolean | undefined> => {
    if (!clientServerUrl.value) return Promise.resolve(undefined);
    if (serverRegisterInFlight) return serverRegisterInFlight;
    if (!force && Date.now() < serverRegisterRetryAt) return Promise.resolve(undefined);

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
        timeoutMs: 15000,
        maxRetries: 1,
        allowExternalHost: true,
      });

      if (data?.status === 'auth_failed') {
        serverRegisterRetryAt = Date.now() + 15_000;
        serverConnected.value = false;
        armReconnectWindow();
        console.warn('VRCDog server auth_failed:', data.reason);
        return false;
      }

      serverConnected.value = true;
      serverRegisterRetryAt = 0;
      lastServerRegisterError = '';
      consecutiveFailures = 0;
      stopReconnectTicker();
      reconnectCountdown.value = 0;
      // 以服务端本次下发的 token 为准；缺失时清空，绝不沿用上一个账号的 token。
      clientToken.value = typeof data.client_token === 'string' ? data.client_token : '';
      // 同步给 API 层：服务端的 `/api/client/*` 全部要求带 `x-vrcdog-client-token`，
      // 由 request 层按 origin 统一附加，问卷 / 功能权限等接口才不会静默 401。
      setVrcdogClientToken(clientToken.value, clientServerUrl.value);
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
      return true;
      } catch (err) {
      serverRegisterRetryAt = Date.now() + 15_000;
      serverConnected.value = false;
      armReconnectWindow();
      const message = err instanceof Error ? err.message : String(err);
      if (message !== lastServerRegisterError) {
        console.warn(t('auto_149c8616'), err);
        lastServerRegisterError = message;
      }
      // The optional VRCDog server must not prevent a successful VRChat login.
      return false;
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
    // 换了服务器 = 换了 token 的归属 origin：先把旧的 client token 作废并同步给 API 层，
    // 否则 request 层会继续把上一个服务器的 token 往新地址上带（服务端必然 401）。
    clientToken.value = '';
    setVrcdogClientToken('', '');
    if (!reconnect || !currentUser.value) return true;

    serverConnected.value = false;
    serverRegisterRetryAt = 0;
    const registered = await registerWithServer(currentUser.value, true);
    if (registered) {
      serverConnected.value = true;
      await uiStore.fetchServerFeatures(getBaseUrl(), currentUser.value);
      startHeartbeat();
    }
    return registered;
  };

  const startVrcKeepalive = () => {
    if (vrcKeepaliveTimer) clearInterval(vrcKeepaliveTimer);
    // VRChat API keepalive: ping /auth/user periodically (every 2 minutes)
    // This keeps the VRChat session alive and rotates tokens continuously.
    vrcKeepaliveTimer = setInterval(async () => {
      if (!currentUser.value || !isLoggedIn.value) return;
      try {
        const pingRes: any = await VrcApi.request('/auth/user', {
          method: 'GET',
          suppressAuthExpired: true,
          timeoutMs: 15000,
        });
        if (pingRes?.auth_cookie) {
          await mergeCookiesAndSave(pingRes.auth_cookie, currentUser.value?.id);
        }
      } catch (e) {
        console.warn('[VrcKeepalive] Session keepalive ping failed (will retry next interval):', e);
      }
    }, 2 * 60 * 1000);
  };

  const runHeartbeatTick = async () => {
    if (!clientServerUrl.value || !currentUser.value) return;
    if (isFetchingHeartbeat) return;

    // 断线状态下必须按「真实时间」判断是否该重试。
    // 旧实现把 reconnectCountdown 当成秒数（初值 30，界面文案写着 seconds until auto reconnect），
    // 但递减发生在 15s 一次的心跳 tick 里 —— 实际要等 30 × 15s ≈ 7.5 分钟，
    // 而这段时间整个界面被 z-[9998] 的断线遮罩全屏挡住。
    if (!serverConnected.value && !syncReconnectCountdown()) return;

    isFetchingHeartbeat = true;
    try {
      const data: any = await VrcApi.request(`${getBaseUrl()}/api/client/heartbeat`, {
        method: 'POST',
        params: {
          user_id: currentUser.value.id || currentUser.value.displayName
        },
        headers: getClientTokenHeader(),
        timeoutMs: 15000,
        maxRetries: 1,
        allowExternalHost: true,
      });

      if (!serverConnected.value) {
        await registerWithServer(currentUser.value, true);
      }
      serverConnected.value = true;
      consecutiveFailures = 0;
      stopReconnectTicker();
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
        await registerWithServer(currentUser.value, true);
      }
      if (data.status === 'survey_required' || data.status === 'survey_available' || data.status === 'ok') {
        applySurveyStatus(data);
      }
    } catch (err) {
      console.warn(t('auto_a46150ae'), err);

      // 服务端重启、或电脑睡眠超过 45s 时，`cleanup_stale_clients` 已经把这台设备
      // 注册的 client token 从内存里删掉了，心跳于是拿到 401。
      // 旧逻辑只把失败计入重连倒计时，于是「遮罩 → 30 秒 → 再 401」无限循环，
      // 只能靠用户手动点「重新连接」。401 的语义就是「服务端不认识这个 token」，
      // 正确反应是立刻重新注册，而不是继续等下一个周期。
      const status = (err as any)?.status;
      if (status === 401 && currentUser.value) {
        consecutiveFailures = 0;
        serverRegisterRetryAt = 0;
        stopReconnectTicker();
        reconnectCountdown.value = 0;
        const reRegistered = await registerWithServer(currentUser.value, true);
        if (reRegistered) {
          serverConnected.value = true;
          return;
        }
      }

      consecutiveFailures++;
      if (consecutiveFailures >= 5) {
        serverConnected.value = false;
        armReconnectWindow();
      }
    } finally {
      isFetchingHeartbeat = false;
    }
  };

  const startHeartbeat = () => {
    startVrcKeepalive();
    if (heartbeatTimer) clearInterval(heartbeatTimer);
    if (!clientServerUrl.value) return;

    // 心跳以 15s 为间隔运行；断线后的重试由 1s 的倒计时定时器在到点时立即发起。
    heartbeatTimer = setInterval(runHeartbeatTick, 15000);
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
      const legacyResult = await VrcApi.getNotifications({ n: 100, offset: 0 }).catch(() => []);
      const notifs = Array.isArray(legacyResult) ? legacyResult : [];
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

  /**
   * 后台数据刷新循环。
   *
   * 设置页里那条「后台数据刷新频率」滑块（10–120 秒，默认 30 秒）此前**没有任何消费者**
   * ——源码和发布产物里 `pollInterval` 都只出现在 SettingsView 自己内部。
   * 于是好友/状态数据只在登录和进入少数页面时才刷新一次，
   * 而侧边栏的健康判定是「超过 N 分钟没刷新就算过期」，
   * 结果就是正常使用十几分钟后一直挂着 amber 的「数据已过期」，
   * 用户无论怎么调那个滑块都不会变。
   *
   * 这里把它接通：按设置的间隔刷新好友列表（含在线状态与本地缓存），
   * 刷新成功即由 friendsStore 标记数据健康。
   * 注意这里**不**写 friend_activity：那是热力图的数据源，按 30 秒一次记录会把
   * 在线好友反复插入成几十万行，快照频率需要单独设计。
   */
  let dataRefreshTimer: ReturnType<typeof setInterval> | null = null;
  let dataRefreshInFlight = false;

  const runDataRefreshTick = async () => {
    if (!isLoggedIn.value || !currentUser.value) return;
    // 上一轮还没回来就跳过：网络慢时避免请求堆积。
    if (dataRefreshInFlight) return;
    dataRefreshInFlight = true;
    try {
      await useFriendsStore().backgroundRefresh();
    } finally {
      dataRefreshInFlight = false;
    }
  };

  const stopDataRefresh = () => {
    if (dataRefreshTimer) {
      clearInterval(dataRefreshTimer);
      dataRefreshTimer = null;
    }
  };

  /** 读取持久化的间隔并按需重启定时器；返回实际生效的秒数。 */
  const configureDataRefresh = async (explicitSec?: number) => {
    let sec = explicitSec;
    if (typeof sec !== 'number') {
      try {
        const raw = await DbApi.getSetting({ key: 'pollInterval' });
        // 设置是按 `JSON.stringify(value)` 存的，所以这里要 parse；
        // 但只有真的解析出数字才采用，坏值（空串 / true / 对象）一律走默认值。
        if (typeof raw === 'number') {
          sec = raw;
        } else if (typeof raw === 'string' && raw.trim() !== '') {
          const parsed = JSON.parse(raw);
          if (typeof parsed === 'number') sec = parsed;
        }
      } catch {
        sec = undefined;
      }
    }
    const applied = setDataRefreshIntervalSec(sec);
    stopDataRefresh();
    if (!isLoggedIn.value || !currentUser.value) return applied;
    dataRefreshTimer = setInterval(() => void runDataRefreshTick(), applied * 1000);
    return applied;
  };

  const handleLoginSuccess = async (user: any) => {
    setLoginGracePeriod(120_000);
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
        await mergeCookiesAndSave(cookie);
      }
    } catch { /* ignore */ }

    const allowed = await registerWithServer(user);
    if (allowed === false) {
      console.warn('VRCDog server registration was not approved; running in standalone mode');
    }

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
    // 首次同步完成后再起后台刷新，避免和登录时的同步叠成两个请求。
    void configureDataRefresh();
  };

  const tryAutoLogin = async () => {
    setLoginGracePeriod(120_000);
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

      try {
        await VrcApi.fetchConfig();
      } catch {
        /* best-effort config fetch; ignore network hiccups */
      }

      let res: any = null;
      let lastLoginError: any = null;
      for (let attempt = 0; attempt < 3; attempt += 1) {
        try {
          res = await VrcApi.login({
            username: null,
            password: null,
            authCookie: savedCookie
          });
          lastLoginError = null;
          break;
        } catch (loginErr: any) {
          lastLoginError = loginErr;
          const errMsg = String(loginErr?.message || loginErr || '').toLowerCase();
          const isNet = /network|timeout|timed out|fetch|connect|abort/i.test(errMsg);
          if (isNet && attempt < 2) {
            await new Promise((resolve) => setTimeout(resolve, 1500 * (attempt + 1)));
            continue;
          }
          // Do not rethrow session expiry/401 here: we must proceed to the credentials fallback below
          break;
        }
      }

      // Normalize: VRChat API may return user as current_user, currentUser, or directly as res (with res.id)
      let autoLoginUser = res?.current_user || res?.currentUser || (res?.id ? res : null);

      // If session cookie expired/failed, attempt seamless re-auth with saved credentials
      if (!autoLoginUser) {
        try {
          const rawAccounts = await DbApi.getSetting({ key: 'savedAccounts' });
          if (rawAccounts) {
            let accounts: any[] = [];
            try {
              const parsed = JSON.parse(rawAccounts);
              accounts = typeof parsed === 'string' ? JSON.parse(parsed) : parsed;
            } catch {}
            if (Array.isArray(accounts) && accounts.length > 0) {
              const cachedUserStr = await DbApi.getSetting({ key: 'cached_vrc_user' }).catch(() => null);
              let cachedUserId: string | null = null;
              if (cachedUserStr) {
                try {
                  const cu = typeof cachedUserStr === 'string' ? JSON.parse(cachedUserStr) : cachedUserStr;
                  cachedUserId = cu?.user?.id || cu?.id || null;
                } catch {}
              }
              const matched = (cachedUserId ? accounts.find((a: any) => a && a.userId === cachedUserId && a.username && a.savedPassword) : null)
                || accounts.find((a: any) => a && a.username && a.savedPassword)
                || accounts.find((a: any) => a && a.savedPassword);
              if (matched && matched.username && matched.savedPassword) {
                const twoFa = getCookieValue(savedCookie, 'twoFactorAuth') || getCookieValue(matched.authCookie, 'twoFactorAuth');
                const cookieToSend = twoFa ? `twoFactorAuth=${twoFa}` : (savedCookie || matched.authCookie);
                const reAuthRes: any = await VrcApi.login({
                  username: matched.username,
                  password: matched.savedPassword,
                  authCookie: cookieToSend,
                });
                const reUser = reAuthRes?.current_user || reAuthRes?.currentUser || (reAuthRes?.id ? reAuthRes : null);
                if (reUser) {
                  autoLoginUser = reUser;
                  res = reAuthRes;
                  if (reAuthRes.auth_cookie) {
                    await DbApi.saveAuth({ cookie: normalizeAuthCookieJson(reAuthRes.auth_cookie) });
                    await mergeCookiesAndSave(reAuthRes.auth_cookie);
                  }
                }
              }
            }
          }
        } catch (reAuthErr) {
          console.warn('[AutoLogin] Re-auth fallback attempt failed:', reAuthErr);
        }
      }

      if (autoLoginUser) {
        currentUser.value = autoLoginUser;

        DbApi.saveSetting({
          key: 'cached_vrc_user',
          value: JSON.stringify({ user: autoLoginUser, expiresAt: Date.now() + 7 * 24 * 60 * 60 * 1000 })
        }).catch(() => {});

        const allowed = await registerWithServer(autoLoginUser);
        if (allowed === false) {
          console.warn('VRCDog server registration was not approved during auto-login; running in standalone mode');
        }
        isLoggedIn.value = true;
        startHeartbeat();
        await ensureServerEventListeners();
        await uiStore.fetchServerFeatures(getBaseUrl(), autoLoginUser);

        if (res?.auth_cookie) {
          await mergeCookiesAndSave(res.auth_cookie);
        }
        initGamelogWatcher();

        // Start friends sync BEFORE awaiting it
        const friendsSyncPromise = startFriendsSync();
        void syncInitialNotifications();
        await friendsSyncPromise;
        await initWebsocket();
        void configureDataRefresh();
      } else if (res?.error) {
        const errMsg = typeof res.error === 'string'
          ? res.error
          : String(res.error?.message || res.error?.details || '');
        if (/missing credentials|invalid credentials|session expired|cookie expired/i.test(errMsg)) {
          // Check if saved accounts exist before clearing
          let hasSaved = false;
          try {
            const raw = await DbApi.getSetting({ key: 'savedAccounts' });
            if (raw) {
              const accs = JSON.parse(raw);
              hasSaved = Array.isArray(accs) && accs.some((a: any) => a?.username && a?.savedPassword);
            }
          } catch {}
          if (!hasSaved) {
            await DbApi.clearAuth();
          }
        }
      }
    } catch (err: any) {
      const status = err?.status;
      const respMsg = String(
        err?.response?.error?.message ||
        err?.response?.message ||
        err?.response?.details ||
        err?.message ||
        '',
      ).toLowerCase();
      const isNetworkOrTimeout = /network|timeout|timed out|fetch|connect|abort/i.test(respMsg) || !status || status >= 500;
      const looksExpired =
        !isNetworkOrTimeout &&
        status === 401 &&
        !err?.response?.requiresTwoFactorAuth &&
        !err?.response?.requires_two_factor_auth &&
        /missing credentials|invalid credentials|cookie expired|session expired/i.test(respMsg);
      if (looksExpired) {
        let hasSaved = false;
        try {
          const raw = await DbApi.getSetting({ key: 'savedAccounts' });
          if (raw) {
            const accs = JSON.parse(raw);
            hasSaved = Array.isArray(accs) && accs.some((a: any) => a?.username && a?.savedPassword);
          }
        } catch {}
        if (!hasSaved) {
          try { await DbApi.clearAuth(); } catch { /* ignore */ }
        }
      }
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
    configureDataRefresh,
    resolveSurveyPrompt,
    loginGracePeriodUntil,
    setLoginGracePeriod,
    isInLoginGracePeriod
  };
});
