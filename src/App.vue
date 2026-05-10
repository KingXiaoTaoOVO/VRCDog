<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { VrcApi, DbApi, SysApi } from "./api/index";
import { initWebsocket, closeWebSocket, wsState } from "./api/websocket";
import { initGamelogWatcher, stopGamelogWatcher } from "./api/gamelogWatcher";
import VrcAvatar from "./components/VrcAvatar.vue";
import StatusCard, { type ComponentStatus } from "./components/StatusCard.vue";
import InstallDialog from "./components/InstallDialog.vue";
import { Settings, RefreshCcw, Bone, X, Heart, Users, Wrench, Flame, StickyNote, Sparkles, Download, LogOut, Loader2, Search, Globe, Bell, UserCircle, ScrollText, UsersRound, LayoutDashboard, Rss, Image, ShieldAlert, Activity } from "lucide-vue-next";
// ignore VrcAvatar implicit any
import dogImg from './assets/dog.jpg';
import LoginView from './components/LoginView.vue';
import FriendsListView from './components/FriendsListView.vue';
import HeatmapView from './components/HeatmapView.vue';
import NotesView from './components/NotesView.vue';
import ExportView from './components/ExportView.vue';
import SearchView from './components/SearchView.vue';
import NotificationsView from './components/NotificationsView.vue';
import MyAvatarsView from './components/MyAvatarsView.vue';
import FavoritesView from './components/FavoritesView.vue';

import GroupsView from './components/GroupsView.vue';
import DashboardView from './components/DashboardView.vue';
import FeedView from './components/FeedView.vue';
import GalleryView from './components/GalleryView.vue';
import ModerationView from './components/ModerationView.vue';
import SettingsView from './components/SettingsView.vue';
import PlayerListView from './components/PlayerListView.vue';

import FriendLocationsView from './components/FriendLocationsView.vue';
import ChartsView from './components/ChartsView.vue';
import StatusPresetsView from './components/StatusPresetsView.vue';
import DebugConsole from './components/DebugConsole.vue';
import EnvView from './components/EnvView.vue';
import ToolsView from './components/ToolsView.vue';
import TranslatorView from './components/TranslatorView.vue';
import OvrTranslatorView from './components/OvrTranslatorView.vue';
import OverlayView from './components/OverlayView.vue';
import RoleSelectView from './components/RoleSelectView.vue';
import ServerDashboardView from './components/ServerDashboardView.vue';
import BilidownView from './components/BilidownView.vue';
import DirectOpenModal from './components/DirectOpenModal.vue';
import UserProfileModal from './components/UserProfileModal.vue';
import EntityDetailModals from './components/EntityDetailModals.vue';
import GlobalSearchModal from './components/GlobalSearchModal.vue';
import CustomNavModal from './components/CustomNavModal.vue';
import { Menu, ChevronUp, ChevronRight, X as XIcon, MessageSquare } from 'lucide-vue-next';
import { Network, BarChart3, History, MapPinned, Languages, ScanEye, Monitor, Glasses, Palette } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import type { VrcUser } from './types/vrc';
import { markRaw } from 'vue';
import { currentTheme, setTheme, themes, type ThemeId } from './theme';

const isOverlayMode = window.location.search.includes('mode=overlay');

const sidebarTabs = [
  { key: 'dashboard', label: 'sidebar.dashboard', icon: markRaw(LayoutDashboard) },
  { key: 'feed', label: 'sidebar.feed', icon: markRaw(Rss) },
  { key: 'locations', label: 'sidebar.locations', icon: markRaw(MapPinned) },
  { key: 'charts', label: 'sidebar.charts', icon: markRaw(BarChart3) },
  { key: 'playerlist', label: 'sidebar.playerlist', icon: markRaw(Network) },
  { key: 'gallery', label: 'sidebar.gallery', icon: markRaw(Image) },
  { key: 'social', label: 'sidebar.social', icon: markRaw(Users) },
  { key: 'search', label: 'sidebar.search', icon: markRaw(Search) },
  { key: 'notifications', label: 'sidebar.notifications', icon: markRaw(Bell) },
  { key: 'groups', label: 'sidebar.groups', icon: markRaw(UsersRound) },
  { key: 'avatars', label: 'sidebar.avatars', icon: markRaw(UserCircle) },
  { key: 'favorites', label: 'sidebar.favorites', icon: markRaw(Heart) },
  { key: 'moderation', label: 'sidebar.moderation', icon: markRaw(ShieldAlert) },
  { key: 'heatmap', label: 'sidebar.heatmap', icon: markRaw(Flame) },
  { key: 'notes', label: 'sidebar.notes', icon: markRaw(StickyNote) },
  { key: 'presets', label: 'sidebar.presets', icon: markRaw(Sparkles) },
  { key: 'tools', label: 'sidebar.tools', icon: markRaw(Wrench) },
  { key: 'bilidown', label: 'sidebar.bilidown', icon: markRaw(Download) },
  { key: 'translator', label: 'sidebar.translator', icon: markRaw(Languages) },
  { key: 'ovr', label: 'sidebar.ovr', icon: markRaw(ScanEye) },
  { key: 'env', label: 'sidebar.env', icon: markRaw(Wrench) },
  { key: 'export', label: 'sidebar.export', icon: markRaw(Download) },
  { key: 'settings', label: 'sidebar.settings', icon: markRaw(Settings) },
];

// Server-managed feature permissions
const serverMenuPerms = ref<Record<string, boolean>>({});
const serverThemePerms = ref<Record<string, boolean>>({});
const serverModePerms = ref<Record<string, boolean>>({});

const showCustomNavModal = ref(false);
const showVrcxMenu = ref(false);
const customNavConfig = ref<any[]>([]);

const loadCustomNavConfig = async () => {
  try {
     const str = await DbApi.getSetting({ key: 'custom_nav_config' });
     if (str) {
        customNavConfig.value = JSON.parse(str);
     }
  } catch(e) {}
};

const saveCustomNavConfig = async (newConfig: any[] | null) => {
   showCustomNavModal.value = false;
   if (newConfig === null) {
       customNavConfig.value = [];
       await DbApi.saveSetting({ key: 'custom_nav_config', value: '[]' });
       return;
   }
   customNavConfig.value = newConfig.map((n: any) => ({ key: n.key, visible: n.visible }));
   await DbApi.saveSetting({ key: 'custom_nav_config', value: JSON.stringify(customNavConfig.value) });
};

const activeSidebarTabs = computed(() => {
  let baseTabs = sidebarTabs.filter(tab => {
     if (clientServerUrl.value && Object.keys(serverMenuPerms.value).length > 0) {
        if (serverMenuPerms.value[tab.key] === false) return false;
     }
     return true;
  });
  if (customNavConfig.value.length === 0) return baseTabs;
  
  const result: any[] = [];
  customNavConfig.value.forEach((cfg: any) => {
      if (cfg.visible !== false) {
         const t = baseTabs.find(b => b.key === cfg.key);
         if (t) result.push(t);
      }
  });
  baseTabs.forEach(t => {
     if (!customNavConfig.value.find((c: any) => c.key === t.key)) result.push(t);
  });
  return result;
});

const editableNavConfig = computed(() => {
  let baseTabs = sidebarTabs.filter(tab => {
     if (clientServerUrl.value && Object.keys(serverMenuPerms.value).length > 0) {
        if (serverMenuPerms.value[tab.key] === false) return false;
     }
     return true;
  });
  if (customNavConfig.value.length === 0) return baseTabs.map(t => ({ ...t, visible: true }));
  
  const result: any[] = [];
  customNavConfig.value.forEach((cfg: any) => {
      const t = baseTabs.find(b => b.key === cfg.key);
      if (t) {
         result.push({ ...t, visible: cfg.visible !== false });
      }
  });
  baseTabs.forEach(t => {
     if (!customNavConfig.value.find((c: any) => c.key === t.key)) {
         result.push({ ...t, visible: true });
     }
  });
  return result;
});

const filteredThemes = computed(() => {
  if (!clientServerUrl.value || Object.keys(serverThemePerms.value).length === 0) return themes;
  const result: typeof themes = {} as typeof themes;
  for (const [key, theme] of Object.entries(themes)) {
    if (serverThemePerms.value[key] !== false) (result as any)[key] = theme;
  }
  return result;
});

const fetchServerFeatures = async () => {
  if (!clientServerUrl.value || !currentUser.value) return;
  const uid = currentUser.value.id || currentUser.value.displayName;
  try {
    const data = await VrcApi.request(`${getBaseUrl()}/api/client/features/${uid}`, 'GET');
    serverMenuPerms.value = data.menus || {};
    serverThemePerms.value = data.themes || {};
    serverModePerms.value = data.modes || {};
    // If current theme is now disabled, switch to first available
    if (serverThemePerms.value[currentTheme.value.id] === false) {
      const available = Object.keys(serverThemePerms.value).find(k => serverThemePerms.value[k] !== false);
      if (available) setTheme(available as ThemeId);
    }
  } catch { /* ignore */ }
};

const { t, locale } = useI18n();
const appRole = ref<'client' | 'server' | null>(null);
const isLoggedIn = ref(false);
const currentUser = ref<VrcUser | null>(null);
const appMode = ref<'pc' | 'vr' | null>(null);
const activeTab = ref<string>('dashboard');
const appVersion = ref('');
const autoLoginLoading = ref(false);
const modeSelectionError = ref('');
const clientServerUrl = ref<string>('');
const banMessage = ref<string>('');
let heartbeatTimer: ReturnType<typeof setInterval> | null = null;

const handleRoleSelected = async (payload: { role: 'client' | 'server', url?: string }) => {
  appRole.value = payload.role;
  if (payload.role === 'client') {
    clientServerUrl.value = payload.url || '';
    tryAutoLogin();
  }
};

// VRC 服务器状态
const vrcServerStatus = ref<string>('');
const vrcServerOk = ref(true);
const fetchServerStatus = async () => {
  try {
    const res = await VrcApi.getServerStatus();
    if (res?.status?.description === 'All Systems Operational') {
      vrcServerOk.value = true;
      vrcServerStatus.value = '';
    } else {
      vrcServerOk.value = false;
      vrcServerStatus.value = res?.status?.description || 'Unknown';
    }
  } catch { vrcServerOk.value = true; }
};

window.addEventListener('settings-updated', (e: any) => {
  if (e.detail?.language) {
    locale.value = e.detail.language;
  }
});

import { useSystemContextStore } from './stores/systemContext';

onMounted(async () => {
  const sysCtx = useSystemContextStore();
  sysCtx.startPolling();
  await loadCustomNavConfig();
  
  try {
    appVersion.value = await getVersion();
  } catch(e) {
    console.error("Failed to get version", e);
  }
});

const serverConnected = ref(true);
const reconnectCountdown = ref(0);
let consecutiveFailures = 0;
const getBaseUrl = () => clientServerUrl.value.replace(/\/+$/, '');

const registerWithServer = async (user: any) => {
  if (!clientServerUrl.value) return;
  try {
    const payload = {
      user_id: user.id || user.displayName,
      display_name: user.displayName || '',
      avatar_url: user.currentAvatarThumbnailImageUrl || ''
    };
    const data = await VrcApi.request(`${getBaseUrl()}/api/client/register`, 'POST', payload);
    
    serverConnected.value = true;
    consecutiveFailures = 0;
    reconnectCountdown.value = 0;
    if (data.status === 'banned') {
      banMessage.value = `账号已被封禁！原因: ${data.reason}${data.duration_hours ? ', 时长: ' + data.duration_hours + '小时' : ', 永久'}`;
      handleLogout(true);
      return false;
    } else if (data.status === 'frozen') {
      banMessage.value = `账号已被冻结！原因: ${data.reason}`;
      handleLogout(true);
      return false;
    }
  } catch (err) {
    console.warn('服务端注册失败，将由心跳检测连接状态', err);
  }
  return true;
};

let isFetchingHeartbeat = false;
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
      if (normalTick < 2) return; // 改为每 2 秒发送一次心跳
      normalTick = 0;
    }

    isFetchingHeartbeat = true;
    try {
      // Add a 3-second timeout to prevent the heartbeat from hanging forever
      const timeoutPromise = new Promise((_, reject) => setTimeout(() => reject(new Error('Heartbeat timeout')), 3000));
      const heartbeatPromise = VrcApi.request(`${getBaseUrl()}/api/client/heartbeat`, 'POST', {
        user_id: currentUser.value.id || currentUser.value.displayName
      });
      const data: any = await Promise.race([heartbeatPromise, timeoutPromise]);
      
      if (!serverConnected.value) {
         // 我们刚刚从断线中恢复，触发一次注册
         await registerWithServer(currentUser.value);
      }
      serverConnected.value = true;
      consecutiveFailures = 0;
      reconnectCountdown.value = 0;
      
      if (data.status === 'banned' || data.status === 'frozen') {
        banMessage.value = data.status === 'banned'
          ? `账号已被封禁！原因: ${data.reason}${data.duration_hours ? ', 时长: ' + data.duration_hours + '小时' : ', 永久'}`
          : `账号已被冻结！原因: ${data.reason}`;
        handleLogout(true);
      } else if (data.status === 'kicked') {
        banMessage.value = '您已被管理员踢出服务器！';
        handleLogout(true);
      }
    } catch (err) {
      console.warn("心跳检测失败:", err);
      consecutiveFailures++;
      if (consecutiveFailures >= 3) { // 连续3次失败才断开
        if (serverConnected.value) {
            serverConnected.value = false;
            reconnectCountdown.value = 30;
        } else {
            reconnectCountdown.value = 30; // 重新开始30秒倒计时
        }
      }
    } finally {
      isFetchingHeartbeat = false;
    }
  }, 1000); // 改为1秒触发一次，用于倒计时
};

const handleLoginSuccess = async (user: any) => {
  currentUser.value = user;
  
  // Save offline cache for up to 7 days
  DbApi.saveSetting({
    key: 'cached_vrc_user',
    value: JSON.stringify({ user: user, expiresAt: Date.now() + 7 * 24 * 60 * 60 * 1000 })
  }).catch(() => {});

  const allowed = await registerWithServer(user);
  if (allowed === false) { currentUser.value = null; return; }
  isLoggedIn.value = true;
  startHeartbeat();

  // Listen to admin actions immediately
  if (isTauri()) {
    import('@tauri-apps/api/event').then(({ listen }) => {
      listen('client_kicked', (e: any) => {
        if (e.payload?.user_id === (currentUser.value?.id || currentUser.value?.displayName)) {
          banMessage.value = '您已被管理员踢出服务器！';
          handleLogout(true);
        }
      });
      listen('client_frozen', (e: any) => {
        if (e.payload?.user_id === (currentUser.value?.id || currentUser.value?.displayName)) {
          banMessage.value = `账号已被冻结！原因: ${e.payload.reason || '未知'}`;
          handleLogout(true);
        }
      });
      listen('client_banned', (e: any) => {
        if (e.payload?.user_id === (currentUser.value?.id || currentUser.value?.displayName)) {
          banMessage.value = `账号已被封禁！原因: ${e.payload.reason || '未知'}`;
          handleLogout(true);
        }
      });
    });
  }

  await fetchServerFeatures();
  await initWebsocket();
  initGamelogWatcher();
  syncInitialFriends();
  syncInitialNotifications();
};

const selectAppMode = async (mode: 'pc' | 'vr') => {
  modeSelectionError.value = '';
  if (mode === 'vr') {
    try {
      const isSteamVRRunning = await SysApi.checkSteamVR();
      if (!isSteamVRRunning) {
        modeSelectionError.value = 'VR 运行时未检测到！请先启动 SteamVR、Oculus Link、Pico Connect 或其他 VR 串流软件。';
        return;
      }
    } catch (e: any) {
      modeSelectionError.value = e.message || 'VR 运行时状态检查失败。';
      return;
    }
    activeTab.value = 'ovr'; // VR 模式默认进入 OVR 翻译设置
  } else {
    activeTab.value = 'dashboard'; // PC 模式默认进入仪表盘
  }
  appMode.value = mode;

  window.addEventListener('vrc-open-detail', (e: Event) => {
    // switch to search tab when direct open is used
    if (activeTab.value !== 'search') {
      activeTab.value = 'search';
    }
  });
};

// ========== 自动登录 ==========
const tryAutoLogin = async () => {
  autoLoginLoading.value = true;
  try {
    if (!isTauri()) { autoLoginLoading.value = false; return; }
    
    // Load language early
    try {
      const allSettings = await DbApi.getAllSettings();
      if (allSettings && typeof allSettings === 'object' && allSettings.language) {
        locale.value = allSettings.language;
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
      
      // Save offline cache for up to 7 days
      DbApi.saveSetting({
        key: 'cached_vrc_user',
        value: JSON.stringify({ user: res.current_user, expiresAt: Date.now() + 7 * 24 * 60 * 60 * 1000 })
      }).catch(() => {});

      // Auto-login also needs to register with server
      const allowed = await registerWithServer(res.current_user);
      if (allowed === false) { currentUser.value = null; autoLoginLoading.value = false; return; }
      isLoggedIn.value = true;
      startHeartbeat();
      await fetchServerFeatures();
      if (res.auth_cookie) {
        // [VRCX 对齐] 合并保存 cookie，不覆盖已有的 twoFactorAuth
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
      // [VRCX 对齐] 登录失败时，检测 Cookie 是否过期
      // 如果是 "Missing Credentials" 或 "无效"，清除过期 Cookie 强制重新登录
      const errMsg = res.error || '';
      if (errMsg.includes('Missing Credentials') || errMsg.includes('无效') || errMsg.includes('过期') || errMsg.includes('expired')) {
        console.warn('[AutoLogin] Auth cookie expired, clearing and requiring fresh login');
        await DbApi.clearAuth();
        // 不使用离线缓存，强制用户重新登录
        return;
      }
      // 401 or invalid cookie, but let's check if we have a valid offline cache to keep them going
      const cachedUserStr = await DbApi.getSetting({ key: 'cached_vrc_user' });
      if (cachedUserStr) {
        try {
          const cachedData = JSON.parse(cachedUserStr);
          if (Date.now() < cachedData.expiresAt) {
            console.log("[AutoLogin] API failed, using cached user data — will still try to load live data");
            currentUser.value = cachedData.user;
            isLoggedIn.value = true;
            registerWithServer(cachedData.user);
            startHeartbeat();
            await fetchServerFeatures();
            await initWebsocket();
            initGamelogWatcher();
            syncInitialFriends();
            syncInitialNotifications();
          } else {
            await DbApi.clearAuth();
          }
        } catch {}
      }
    }
  } catch (err) {
    console.warn("自动登录遇到网络异常:", err);
    // 网络异常时，尝试使用本地缓存登录，绝不轻易清除 Cookie
    try {
      const cachedUserStr = await DbApi.getSetting({ key: 'cached_vrc_user' });
      if (cachedUserStr) {
        const cachedData = JSON.parse(cachedUserStr);
        if (Date.now() < cachedData.expiresAt) {
          console.log("[AutoLogin] Network error, using offline cache — will still try to load live data");
          currentUser.value = cachedData.user;
          isLoggedIn.value = true;
          registerWithServer(cachedData.user);
          startHeartbeat();
          // [VRCX 对齐] 网络异常缓存模式也尝试加载数据
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

// ========== 退出登录 ==========
const disconnectFromServer = async () => {
  if (!clientServerUrl.value || !currentUser.value) return;
  try {
    await VrcApi.request(`${getBaseUrl()}/api/client/disconnect`, 'POST', {
      user_id: currentUser.value.id || currentUser.value.displayName
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
  activeTab.value = 'social';
  closeWebSocket();
  stopGamelogWatcher();
};

// Window close: notify server
if (typeof window !== 'undefined') {
  window.addEventListener('beforeunload', () => {
    if (clientServerUrl.value && currentUser.value) {
      const uid = currentUser.value.id || currentUser.value.displayName;
      fetch(`${getBaseUrl()}/api/client/disconnect`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ user_id: uid }),
        keepalive: true
      }).catch(() => {});
    }
  });

  // [VRCX 对齐] 运行中 Cookie 过期时，强制重新登录
  // VRCX 通过 authStore.handleAutoLogin() 触发重新登录流程
  window.addEventListener('vrc-auth-expired', async () => {
    console.warn('[App] Auth expired event received, forcing re-login...');
    try { await DbApi.clearAuth(); } catch {}
    currentUser.value = null;
    isLoggedIn.value = false;
    closeWebSocket();
    stopGamelogWatcher();
  });
}

// ========== 环境管家相关 ==========
const hubStatus = ref<ComponentStatus>('checking');
const hubProgress = ref(0);
const hubProgressMsg = ref('');
const unityStatus = ref<ComponentStatus>('checking');
const unityProgress = ref(0);
const unityProgressMsg = ref('');
const toolStatus = ref<ComponentStatus>('checking');
const toolProgress = ref(0);
const toolProgressMsg = ref('');
const vccStatus = ref(false);
const alcomStatus = ref(false);
const showInstallDialog = ref(false);
const showSettings = ref(false);
const dialogConfig = ref({ title: '', target: '', isVccSelection: false });

const checkEnvironment = async () => {
  hubStatus.value = 'checking'; unityStatus.value = 'checking'; toolStatus.value = 'checking';
  try {
    if (!isTauri()) throw new Error("浏览器模式");
    const result = await SysApi.checkSystemStatus();
    hubStatus.value = result.hub_installed ? 'installed' : 'not_installed';
    unityStatus.value = result.unity_installed ? 'installed' : 'not_installed';
    toolStatus.value = result.tool_installed ? 'installed' : 'not_installed';
    vccStatus.value = result.vcc_installed;
    alcomStatus.value = result.alcom_installed;
  } catch {
    setTimeout(() => { hubStatus.value = 'not_installed'; unityStatus.value = 'not_installed'; toolStatus.value = 'not_installed'; }, 1000);
  }
};

const handleInstallClick = (target: string) => {
  if (target === 'hub') dialogConfig.value = { title: t('app.install_hub') || '给汪汪装个 Unity Hub', target: 'hub', isVccSelection: false };
  else if (target === 'unity') dialogConfig.value = { title: t('app.install_unity') || '给汪汪装个 Unity 2022', target: 'unity', isVccSelection: false };
  else if (target === 'tool') dialogConfig.value = { title: t('app.install_tool') || '选个趁手的骨头工具', target: 'tool', isVccSelection: true };
  showInstallDialog.value = true;
};

const handleUninstallSpecific = async (target: string) => {
  if (target === 'hub') hubStatus.value = 'checking';
  if (target === 'unity') unityStatus.value = 'checking';
  if (target === 'tool' || target === 'vcc' || target === 'alcom') toolStatus.value = 'checking';
  try {
    if (!isTauri()) throw new Error("普通浏览器不能执行卸载");
    await SysApi.uninstallSoftware({ target });
    await checkEnvironment();
  } catch (err: any) { alert(err.message || err); await checkEnvironment(); }
};

const handleDialogConfirm = async (config: any) => {
  showInstallDialog.value = false;
  const target = dialogConfig.value.target;
  if (target === 'hub') hubStatus.value = 'installing';
  if (target === 'unity') hubStatus.value = 'installing';
  if (target === 'tool') toolStatus.value = 'installing';
  try {
    if (!('__TAURI_INTERNALS__' in window)) {
      setTimeout(() => {
        if (target === 'hub') { hubStatus.value = 'installed'; }
        if (target === 'unity') { unityStatus.value = 'installed'; }
        if (target === 'tool') { toolStatus.value = 'installed'; }
      }, 2000);
      return;
    }
    await SysApi.installSoftware({ target, path: config.path, tool: config.tool, autoDelete: config.autoDelete });
    await checkEnvironment();
  } catch (error: any) {
    if (target === 'hub') { hubStatus.value = 'error'; hubProgressMsg.value = error.message || error; }
    if (target === 'unity') { unityStatus.value = 'error'; unityProgressMsg.value = error.message || error; }
    if (target === 'tool') { toolStatus.value = 'error'; toolProgressMsg.value = error.message || error; }
  }
};

const getStatusColor = (status: string) => {
  switch (status) {
    case 'active': return 'bg-green-500';
    case 'join me': return 'bg-blue-500';
    case 'ask me': return 'bg-orange-500';
    case 'busy': return 'bg-red-500';
    default: return 'bg-slate-400';
  }
};

// ========== 初始化好友数据并写入 SQLite 离线大缓存 ==========
const syncInitialFriends = async () => {
  if (!isLoggedIn.value) return;
  try {
    // 1. 获取在线好友
    const onlineFriends = await VrcApi.getFriends({ n: 100, offset: 0, offline: false });
    // 2. 获取离线好友
    const offlineFriends = await VrcApi.getFriends({ n: 100, offset: 0, offline: true });
    
    const allFriends = [...onlineFriends, ...offlineFriends];
    
    if (allFriends.length > 0 && isTauri()) {
      // 写入到离线缓存
      const count = await DbApi.batchSaveFriends({ friendsJson: JSON.stringify(allFriends) });
      console.log(`[Cache] 成功同步 ${count} 个好友到本地 SQLite。列表滑动将如丝般顺滑！`);
      
      // 同时也把在线的好友状态记录一下，以便提供给活跃热力图使用
      if (onlineFriends.length > 0) {
        await DbApi.batchRecordFriends({ friendsJson: JSON.stringify(onlineFriends) });
      }
    }

    // 触发全局事件，让各组件知道初始缓存已完毕
    window.dispatchEvent(new CustomEvent('vrc-friends-synced'));
  } catch (err) {
    console.warn('好友同步失败:', err);
  }
};

// ========== 初始化通知数据并写入 SQLite 离线缓存 ==========
const syncInitialNotifications = async () => {
  if (!isLoggedIn.value) return;
  try {
    const notifs = await VrcApi.getNotifications({ n: 100, offset: 0 });
    if (notifs && notifs.length > 0 && isTauri()) {
      const count = await DbApi.batchSaveNotifications({ notificationsJson: JSON.stringify(notifs) });
      console.log(`[Cache] 成功同步 ${count} 条通知到本地 SQLite。`);
    }
    window.dispatchEvent(new CustomEvent('vrc-notifications-synced'));
  } catch (err) {
    console.warn('通知同步失败:', err);
  }
};

const applyProxyFromSettings = async (settings: any) => {
  if (!settings) return;
  const isEnabled = settings.proxyEnabled === 'true' || settings.proxyEnabled === true;
  const url = isEnabled && settings.proxyUrl ? settings.proxyUrl : null;
  try {
    let authCookie = null;
    try {
      authCookie = await DbApi.getAuth();
    } catch { /* ignore */ }
    await VrcApi.setProxy({ proxyUrl: url, authCookie: authCookie });
  } catch (e) {
    console.warn('Failed to set proxy:', e);
  }
};

onMounted(async () => {
  if (isTauri()) {
    try {
      const allSettings = await DbApi.getAllSettings();
      await applyProxyFromSettings(allSettings);
    } catch { /* ignore */ }

    window.addEventListener('settings-updated', (e: Event) => {
      const customEvent = e as CustomEvent;
      applyProxyFromSettings(customEvent.detail);
    });

    initGamelogWatcher();

    // 监听深层链接 (Deep Link) 启动参数
    setTimeout(async () => {
      try {
        const args = await SysApi.getLaunchArgs();
        const urlArg = args.find(a => a.startsWith('vrcx://') || a.startsWith('vrchat://'));
        if (urlArg) {
          // 处理 vrcx:// 链接，比如 vrcx://launch/wrld_1234
          console.log("[VRCX] Received URI Protocol argument:", urlArg);
          
          if (urlArg.includes('launch/')) {
            const worldId = urlArg.split('launch/')[1];
            if (worldId) {
              const confirmLaunch = confirm(`检测到外部启动请求！\n是否要立即加入实例：\n${worldId}`);
              if (confirmLaunch) {
                const parts = worldId.split(':');
                await VrcApi.inviteMyself({ worldId: parts[0], instanceId: parts[1] || '0' });
              }
            }
          }
        }
      } catch { /* ignore */ }
    }, 1000);
  }

  await checkEnvironment();
  fetchServerStatus();
  setInterval(fetchServerStatus, 5 * 60 * 1000); // 每5分钟刷新
  if (isTauri()) {
    await listen('install_progress', (event: any) => {
      const p = event.payload;
      if (p.target === 'hub') { hubProgress.value = p.progress; hubProgressMsg.value = p.status; }
      else if (p.target === 'unity') { unityProgress.value = p.progress; unityProgressMsg.value = p.status; }
      else if (p.target === 'tool') { toolProgress.value = p.progress; toolProgressMsg.value = p.status; }
    });
    // 登录成功后启动一次好友轮询（后续由 WebSocket 接管）
    if (isLoggedIn.value) {
      syncInitialFriends(); 
    }

    // 监听实时服务端踢出/封禁/冻结事件（解决心跳包20秒延迟问题）
    await listen('client_kicked', (event: any) => {
      const kickedUserId = event.payload;
      if (appRole.value === 'client' && currentUser.value && (currentUser.value.id === kickedUserId || currentUser.value.displayName === kickedUserId)) {
         banMessage.value = '您已被管理员踢出服务器！';
         handleLogout(true);
      }
    });

    await listen('client_banned', (event: any) => {
      const p = event.payload;
      if (appRole.value === 'client' && currentUser.value && (currentUser.value.id === p.user_id || currentUser.value.displayName === p.user_id)) {
         banMessage.value = `账号已被封禁！原因: ${p.reason}${p.duration_hours ? ', 时长: ' + p.duration_hours + '小时' : ', 永久'}`;
         handleLogout(true);
      }
    });

    await listen('client_frozen', (event: any) => {
      const p = event.payload;
      if (appRole.value === 'client' && currentUser.value && (currentUser.value.id === p.user_id || currentUser.value.displayName === p.user_id)) {
         banMessage.value = `账号已被冻结！原因: ${p.reason}`;
         handleLogout(true);
      }
    });

  }
});
</script>

<template>
  <OverlayView v-if="isOverlayMode" />
  
  <ServerDashboardView
    v-else-if="appRole === 'server'"
    @exit="appRole = null"
  />

  <RoleSelectView
    v-else-if="appRole === null"
    @role-selected="handleRoleSelected"
  />

  <div
    v-else-if="autoLoginLoading"
    class="w-full h-screen flex flex-col items-center justify-center bg-[#fffbeb]"
  >
    <img
      :src="dogImg"
      class="w-24 h-24 rounded-full border-4 border-slate-200 shadow-xl mb-6 animate-pulse"
    >
    <Loader2
      class="animate-spin text-indigo-500 mb-3"
      :size="32"
    />
    <p class="text-slate-600 font-bold">
      {{ $t('app.loading') }}
    </p>
  </div>

  <!-- 未登录 -->
  <div
    v-else-if="!isLoggedIn"
    class="w-full h-full relative"
  >
    <LoginView @login-success="handleLoginSuccess" />
    <!-- Ban/Freeze Overlay -->
    <div
      v-if="banMessage"
      class="fixed inset-0 bg-black/70 flex items-center justify-center z-[9999]"
    >
      <div class="bg-slate-900 border border-red-500/50 rounded-xl p-6 max-w-md mx-4 text-center shadow-2xl">
        <div class="text-4xl mb-3">
          🚫
        </div>
        <h2 class="text-xl font-bold text-red-400 mb-3">
          访问受限
        </h2>
        <p class="text-slate-300 text-sm whitespace-pre-line mb-4">
          {{ banMessage }}
        </p>
        <button
          class="px-6 py-2 bg-slate-700 hover:bg-slate-600 text-white rounded-lg text-sm"
          @click="banMessage = ''"
        >
          我知道了
        </button>
      </div>
    </div>
  </div>

  <!-- 模式选择 -->
  <div
    v-else-if="isLoggedIn && !appMode"
    class="w-full h-screen flex flex-col items-center justify-center bg-[#fffbeb] relative overflow-hidden"
  >
    <div class="absolute inset-0 z-0 overflow-hidden pointer-events-none">
      <div class="absolute top-[-20%] left-[-10%] w-[60%] h-[60%] bg-pink-200/40 rounded-full blur-[100px] animate-pulse" />
      <div
        class="absolute bottom-[-10%] left-[20%] w-[50%] h-[50%] bg-indigo-200/40 rounded-full blur-[100px] animate-pulse"
        style="animation-delay: 2s"
      />
    </div>
    
    <div class="z-10 bg-white/80 backdrop-blur-xl p-10 rounded-[32px] shadow-2xl border-4 border-white max-w-xl w-full text-center">
      <img
        :src="dogImg"
        class="w-24 h-24 rounded-full border-4 border-slate-200 shadow-lg mx-auto mb-6"
      >
      <h2 class="text-3xl font-extrabold text-slate-900 mb-2">
        {{ $t('app.select_mode_title') || '选择运行模式' }}
      </h2>
      <p class="text-slate-500 mb-8 font-medium">
        {{ $t('app.select_mode_desc') || 'VrcDog 提供桌面管理看板与 SteamVR 沉浸式内置叠加层两种体验。' }}
      </p>
      
      <div class="grid grid-cols-2 gap-4">
        <button
          class="flex flex-col items-center gap-3 p-6 rounded-3xl border-2 transition-all group"
          :class="[
            serverModePerms['pc'] === false 
              ? 'bg-slate-100 border-slate-200 opacity-50 cursor-not-allowed grayscale' 
              : 'bg-slate-50 hover:bg-indigo-50 border-slate-200 hover:scale-105 active:scale-95'
          ]"
          :disabled="serverModePerms['pc'] === false"
          @click="selectAppMode('pc')"
        >
          <Monitor class="w-12 h-12 text-indigo-600 group-hover:text-slate-600" />
          <span class="font-bold text-slate-900 text-lg">PC Desktop</span>
          <span
            v-if="serverModePerms['pc'] !== false"
            class="text-xs text-slate-400"
          >桌面好友管理与分析看板</span>
          <span
            v-else
            class="text-xs text-red-500"
          >无权限访问此模式</span>
        </button>
        <button
          class="flex flex-col items-center gap-3 p-6 rounded-3xl border-2 transition-all group"
          :class="[
            serverModePerms['vr'] === false 
              ? 'bg-slate-100 border-slate-200 opacity-50 cursor-not-allowed grayscale' 
              : 'bg-indigo-50 hover:bg-indigo-100 border-indigo-200 hover:scale-105 active:scale-95'
          ]"
          :disabled="serverModePerms['vr'] === false"
          @click="selectAppMode('vr')"
        >
          <Glasses class="w-12 h-12 text-indigo-600 group-hover:text-indigo-700" />
          <span class="font-bold text-indigo-900 text-lg">VR Overlay</span>
          <span
            v-if="serverModePerms['vr'] !== false"
            class="text-xs text-indigo-700/60"
          >OVR OCR翻译与内嵌面版</span>
          <span
            v-else
            class="text-xs text-red-500"
          >无权限访问此模式</span>
        </button>
      </div>
      
      <div
        v-if="modeSelectionError"
        class="mt-6 p-3 bg-red-50 text-red-600 rounded-xl border border-red-200 text-sm font-bold flex items-center justify-center gap-2"
      >
        <ShieldAlert class="w-4 h-4" /> {{ modeSelectionError }}
      </div>
    </div>
  </div>

  <!-- 已登录主界面 VR模式 -->
  <div
    v-else-if="isLoggedIn && appMode === 'vr'"
    class="flex h-screen overflow-hidden relative"
    style="background: linear-gradient(135deg, #0f0c29, #302b63, #24243e)"
  >
    <!-- VR 深空背景粒子 -->
    <div class="absolute inset-0 z-0 overflow-hidden pointer-events-none">
      <div class="absolute top-[-15%] right-[-10%] w-[50%] h-[50%] bg-indigo-600/15 rounded-full blur-[120px] animate-pulse" />
      <div
        class="absolute bottom-[-10%] left-[-5%] w-[40%] h-[40%] bg-purple-600/15 rounded-full blur-[100px] animate-pulse"
        style="animation-delay: 3s"
      />
      <div
        class="absolute top-[30%] left-[40%] w-[30%] h-[30%] bg-cyan-500/10 rounded-full blur-[80px] animate-pulse"
        style="animation-delay: 5s"
      />
    </div>

    <!-- VR 侧边栏 -->
    <aside class="w-56 bg-white/5 backdrop-blur-xl shadow-2xl border-r border-white/10 flex flex-col z-10 p-4 relative flex-shrink-0">
      <div class="flex items-center gap-2.5 mb-4">
        <div class="w-10 h-10 rounded-full overflow-hidden border-2 border-indigo-400/50 bg-indigo-900/50 flex-shrink-0 flex items-center justify-center">
          <Glasses class="w-6 h-6 text-indigo-300" />
        </div>
        <div>
          <h2 class="font-bold text-sm leading-tight text-white">
            VrcDog VR
          </h2>
          <p class="text-[10px] font-medium text-indigo-300/70">
            OVR Overlay Translator
          </p>
        </div>
      </div>

      <!-- VR 设备状态面板 -->
      <div class="mb-4 p-3 bg-white/5 rounded-2xl border border-white/10 space-y-2">
        <h3 class="text-[10px] font-bold text-indigo-300/60 uppercase tracking-wider mb-1">
          VR 设备状态
        </h3>
        <div class="flex items-center gap-2 text-[11px] text-white/80 font-medium">
          <div class="w-2 h-2 rounded-full bg-green-400 animate-pulse" />
          <span>SteamVR 运行中</span>
        </div>
        <div class="flex items-center gap-2 text-[11px] text-white/60 font-medium">
          <div
            class="w-2 h-2 rounded-full"
            :class="currentUser ? 'bg-green-400' : 'bg-slate-500'"
          />
          <span>{{ currentUser?.displayName || '未登录' }}</span>
        </div>
      </div>

      <!-- VR 导航 -->
      <div class="flex-1 space-y-1 overflow-y-auto">
        <button
          v-for="tab in [
            { key: 'ovr', icon: Glasses, label: 'OVR 翻译设置' },
            { key: 'translator', icon: Languages, label: '桌面翻译器' },
            { key: 'social', icon: Users, label: '社交大厅' },
            { key: 'settings', icon: Settings, label: '基础设置' },
          ]"
          :key="tab.key"
          class="w-full flex items-center gap-2.5 px-3 py-2.5 rounded-xl border font-bold transition-all text-left text-sm"
          :class="activeTab === tab.key
            ? 'bg-indigo-500/20 text-white border-indigo-400/30'
            : 'text-white/50 border-transparent hover:text-white/80 hover:bg-white/5'"
          @click="activeTab = tab.key as any"
        >
          <component
            :is="tab.icon"
            :size="18"
          />
          {{ tab.label }}
        </button>
      </div>

      <!-- 用户信息 + 退出 -->
      <div class="mt-auto pt-3 border-t border-white/10 space-y-2">
        <div class="flex items-center gap-2.5">
          <VrcAvatar
            :user="currentUser"
            custom-class="w-9 h-9 rounded-xl object-cover flex-shrink-0"
            style="background-color: rgba(99,102,241,0.3)"
          />
          <div class="flex-1 overflow-hidden">
            <p class="text-xs font-bold truncate text-white">
              {{ currentUser?.displayName }}
            </p>
            <p
              class="text-[10px] font-bold flex items-center gap-1"
              :class="{
                'text-green-400': currentUser?.status === 'active',
                'text-blue-400': currentUser?.status === 'join me',
                'text-orange-400': currentUser?.status === 'ask me',
                'text-red-400': currentUser?.status === 'busy',
                'text-slate-400': !currentUser?.status,
              }"
            >
              <span
                class="w-1.5 h-1.5 rounded-full inline-block animate-pulse"
                :class="getStatusColor(currentUser?.status || 'offline')"
              />
              {{ $t('status.' + (currentUser?.status?.replace(' ', '_') || 'offline')) }}
            </p>
          </div>
        </div>

        <!-- VrcDog 服务端连接状态 -->
        <div
          v-if="clientServerUrl"
          class="mt-1 px-2 py-1.5 rounded-lg border text-[10px] font-bold flex items-center gap-1"
          :class="serverConnected ? 'bg-emerald-500/10 border-emerald-500/20 text-emerald-400' : 'bg-red-500/10 border-red-500/20 text-red-400 animate-pulse'"
        >
          <div
            class="w-1.5 h-1.5 rounded-full"
            :class="serverConnected ? 'bg-emerald-400' : 'bg-red-400'"
          />
          <span>{{ serverConnected ? '服务端已连接' : '服务端断开' }}</span>
        </div>

        <div class="flex gap-2 mt-1">
          <button
            class="flex-1 flex items-center justify-center gap-1.5 px-2 py-2 rounded-xl text-indigo-300 hover:bg-white/10 font-bold text-xs transition-colors border border-transparent hover:border-indigo-400/20"
            @click="appMode = null"
          >
            <Monitor :size="14" /> 重选模式
          </button>
          <button
            class="flex-1 flex items-center justify-center gap-1.5 px-2 py-2 rounded-xl text-red-400 hover:bg-red-500/10 font-bold text-xs transition-colors border border-transparent hover:border-red-400/20"
            @click="() => handleLogout(false)"
          >
            <LogOut :size="14" /> {{ $t('app.logout') }}
          </button>
        </div>

        <div class="text-center pt-2 mt-2 border-t border-white/5">
          <span class="text-[10px] font-mono text-indigo-300/50 font-bold tracking-wider">v{{ appVersion }}</span>
        </div>
      </div>
    </aside>

    <!-- VR 主内容区 -->
    <main class="flex-1 relative z-10 overflow-y-auto">
      <div
        v-if="activeTab === 'ovr'"
        class="p-6 h-full overflow-y-auto"
      >
        <OvrTranslatorView />
      </div>
      <div
        v-else-if="activeTab === 'translator'"
        class="p-6 h-full overflow-hidden"
      >
        <TranslatorView />
      </div>
      <div
        v-else-if="activeTab === 'social'"
        class="p-6 h-full overflow-hidden"
      >
        <FriendsListView />
      </div>
      <div
        v-else-if="activeTab === 'settings'"
        class="p-6 h-full overflow-hidden"
      >
        <SettingsView />
      </div>
      <!-- 默认回落到 OVR -->
      <div
        v-else
        class="p-6 h-full overflow-y-auto"
      >
        <OvrTranslatorView />
      </div>
    </main>

    <!-- 全局调试面板 -->
    <DebugConsole />
    <DirectOpenModal />

    <!-- 服务端断连全屏遮罩 (VR模式) -->
    <div
      v-if="isLoggedIn && clientServerUrl && !serverConnected"
      class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-[9998]"
    >
      <div class="bg-slate-900 border border-red-500/30 rounded-2xl p-8 max-w-sm mx-4 text-center shadow-2xl">
        <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-red-500/20 flex items-center justify-center">
          <svg
            class="w-8 h-8 text-red-400 animate-pulse"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          ><path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M18.364 5.636a9 9 0 010 12.728M5.636 5.636a9 9 0 000 12.728M12 12h.01"
          /></svg>
        </div>
        <h2 class="text-lg font-bold text-red-400 mb-2">
          服务端连接已断开
        </h2>
        <p class="text-slate-400 text-sm mb-4">
          无法连接到 VrcDog 服务端，软件功能已暂停。<br>系统将在 <span class="text-white font-bold">{{ reconnectCountdown }}</span> 秒后自动尝试重连...
        </p>
        <div class="flex gap-2 justify-center">
          <button
            class="px-5 py-2 bg-slate-700 hover:bg-slate-600 text-white rounded-lg text-sm"
            @click="() => handleLogout(false)"
          >
            退出登录
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- 已登录主界面 PC模式 -->
  <div
    v-else-if="isLoggedIn && appMode === 'pc'"
    class="flex h-screen overflow-hidden relative"
    :style="{ backgroundColor: currentTheme.colors.bgMain }"
  >
    <!-- 萌萌哒背景 -->
    <div class="absolute inset-0 z-0 overflow-hidden pointer-events-none">
      <img
        :src="currentTheme.logo"
        class="absolute -right-20 -bottom-20 w-[500px] opacity-20 mix-blend-multiply blur-sm rounded-full"
        alt=""
      >
      <div
        class="absolute top-[-20%] left-[-10%] w-[60%] h-[60%] rounded-full blur-[100px] animate-pulse"
        :style="{ backgroundColor: currentTheme.colors.blob1 }"
      />
      <div
        class="absolute bottom-[-10%] left-[20%] w-[50%] h-[50%] rounded-full blur-[100px] animate-pulse"
        :style="{ backgroundColor: currentTheme.colors.blob2, animationDelay: '2s' }"
      />
    </div>

    <!-- 侧边栏 -->
    <aside
      class="w-56 bg-white/80 backdrop-blur-md shadow-xl border-r-2 flex flex-col z-10 p-4 relative flex-shrink-0"
      :style="{ borderColor: currentTheme.colors.borderSoft }"
    >
      <div class="flex items-center gap-2.5 mb-2">
        <div
          class="w-10 h-10 rounded-full overflow-hidden border-2 bg-white flex-shrink-0"
          :style="{ borderColor: currentTheme.colors.borderStrong }"
        >
          <img
            :src="currentTheme.logo"
            class="w-full h-full object-cover"
          >
        </div>
        <div>
          <h2
            class="font-bold text-sm leading-tight"
            :style="{ color: currentTheme.colors.textStrong }"
          >
            {{ currentTheme.appTitle }}
          </h2>
          <p
            class="text-[10px] font-medium"
            :style="{ color: currentTheme.colors.textSoft }"
          >
            {{ $t('app.subtitle') }}
          </p>
        </div>
      </div>
      
      <!-- 主题切换 -->
      <div
        class="flex justify-between items-center bg-white/50 rounded-xl p-1 mb-4"
        :style="{ border: `1px solid ${currentTheme.colors.borderSoft}` }"
      >
        <button
          v-for="t in Object.values(filteredThemes)"
          :key="t.id"
          class="flex-1 py-1 text-xs font-bold rounded-lg transition-colors flex items-center justify-center gap-1"
          :style="currentTheme.id === t.id ? { backgroundColor: t.colors.activeBg, color: t.colors.textStrong } : { color: currentTheme.colors.textSoft, opacity: 0.7 }"
          :title="t.name"
          @click="setTheme(t.id as ThemeId)"
        >
          {{ t.name.slice(0,2) }}
        </button>
      </div>

      <div class="flex-1 space-y-1 overflow-y-auto custom-scrollbar">
        <button
          v-for="tab in activeSidebarTabs"
          :key="tab.key"
          class="w-full flex items-center gap-2.5 px-3 py-2.5 rounded-xl border font-bold transition-all text-left text-sm"
          :style="activeTab === tab.key ? { backgroundColor: currentTheme.colors.activeBg, color: currentTheme.colors.textStrong, borderColor: currentTheme.colors.borderStrong } : { color: currentTheme.colors.textSoft, borderColor: 'transparent' }"
          @click="activeTab = tab.key as any"
        >
          <component
            :is="tab.icon"
            :size="18"
          />
          {{ $t(tab.label) }}
        </button>
      </div>

      <!-- 用户信息 + 退出 -->
      <div
        class="mt-auto pt-3 border-t space-y-2 relative"
        :style="{ borderColor: currentTheme.colors.borderSoft }"
      >
        <div 
          class="flex items-center gap-2.5 cursor-pointer hover:bg-black/5 p-1.5 -ml-1.5 rounded-xl transition-colors relative"
          @click="showVrcxMenu = !showVrcxMenu"
        >
          <VrcAvatar
            :user="currentUser"
            custom-class="w-9 h-9 rounded-xl object-cover flex-shrink-0"
            :style="{ backgroundColor: currentTheme.colors.blob2 }"
          />
          <div class="flex-1 overflow-hidden">
            <p
              class="text-xs font-bold truncate"
              :style="{ color: currentTheme.colors.textStrong }"
            >
              {{ currentUser?.displayName }}
            </p>
            <p
              class="text-[10px] font-bold flex items-center gap-1"
              :class="{
                'text-green-500': currentUser?.status === 'active',
                'text-blue-500': currentUser?.status === 'join me',
                'text-orange-500': currentUser?.status === 'ask me',
                'text-red-500': currentUser?.status === 'busy',
                'text-slate-400': !currentUser?.status,
              }"
            >
              <span
                class="w-1.5 h-1.5 rounded-full inline-block animate-pulse"
                :class="getStatusColor(currentUser?.status || 'offline')"
              />
              {{ $t('status.' + (currentUser?.status?.replace(' ', '_') || 'offline')) }}
            </p>
          </div>
        </div>

        <!-- 实时数据流状态 (WebSocket) -->
        <div
          class="mt-2 px-2 py-1.5 rounded-lg border text-[10px] font-bold flex items-center justify-between"
          :class="wsState.connected ? 'bg-green-50 border-green-200 text-green-600' : 'bg-orange-50 border-orange-200 text-orange-600'"
        >
          <div class="flex items-center gap-1">
            <Activity
              :size="10"
              :class="wsState.connected ? 'animate-pulse' : ''"
            />
            <span>{{ wsState.connected ? $t('status.pipeline_online') : $t('status.pipeline_offline') }}</span>
          </div>
          <span
            v-if="wsState.connected && wsState.messageCount > 0"
            class="text-green-500"
          >{{ $t('status.frames', { count: wsState.messageCount }) }}</span>
        </div>

        <!-- VrcDog 服务端连接状态 -->
        <div
          v-if="clientServerUrl"
          class="mt-1 px-2 py-1.5 rounded-lg border text-[10px] font-bold flex items-center gap-1"
          :class="serverConnected ? 'bg-emerald-50 border-emerald-200 text-emerald-600' : 'bg-red-50 border-red-200 text-red-600 animate-pulse'"
        >
          <div
            class="w-1.5 h-1.5 rounded-full"
            :class="serverConnected ? 'bg-emerald-500' : 'bg-red-500'"
          />
          <span>{{ serverConnected ? '服务端已连接' : '服务端断开' }}</span>
        </div>

        <!-- VRC 服务器状态 -->
        <div
          v-if="vrcServerStatus"
          class="mt-1 px-2 py-1.5 rounded-lg bg-red-50 border border-red-200 text-[10px] font-bold text-red-600 flex items-center gap-1"
        >
          <Globe :size="10" /> {{ vrcServerStatus }}
        </div>

        <div class="flex gap-2 mt-1">
          <button
            class="flex-1 flex items-center justify-center gap-1.5 px-2 py-2 rounded-xl text-orange-500 hover:bg-orange-50 font-bold text-xs transition-colors border border-transparent hover:border-orange-100"
            @click="appMode = null"
          >
            <Monitor :size="14" /> 重选模式
          </button>
          <button
            class="flex-1 flex items-center justify-center gap-1.5 px-2 py-2 rounded-xl text-red-500 hover:bg-red-50 font-bold text-xs transition-colors border border-transparent hover:border-red-100"
            @click="() => handleLogout(false)"
          >
            <LogOut :size="14" /> {{ $t('app.logout') }}
          </button>
        </div>

        <!-- VRCX-like Settings Menu -->
        <div 
          v-if="showVrcxMenu" 
          class="absolute bottom-full left-0 mb-3 w-[220px] bg-[#1e1f22] border border-white/10 shadow-2xl rounded-xl overflow-hidden text-slate-200 z-50 animate-fade-in"
        >
          <div class="p-3 flex items-center justify-between border-b border-white/5 bg-[#2b2d31]">
            <div class="flex items-center gap-2">
              <MessageSquare class="w-4 h-4 text-white" />
              <span class="font-bold text-[13px] text-white">VRCX ♥</span>
            </div>
            <span class="text-[11px] text-slate-400">2026.05.10</span>
          </div>
          <div class="py-1">
            <button class="w-full text-left px-4 py-2 text-[13px] hover:bg-white/10 transition-colors" @click="activeTab='settings'; showVrcxMenu=false">设置</button>
            <button class="w-full flex justify-between items-center px-4 py-2 text-[13px] hover:bg-white/10 transition-colors">
              主题 <ChevronRight class="w-4 h-4 text-slate-400" />
            </button>
            <button class="w-full flex justify-between items-center px-4 py-2 text-[13px] hover:bg-white/10 transition-colors">
              行高密度 <ChevronRight class="w-4 h-4 text-slate-400" />
            </button>
            <button class="w-full text-left px-4 py-2 text-[13px] hover:bg-white/10 transition-colors" @click="showCustomNavModal = true; showVrcxMenu=false">
              自定义导航栏
            </button>
          </div>
          <div class="py-1 border-t border-white/5">
            <button 
              class="w-full text-left px-4 py-2 text-[13px] text-red-400 hover:bg-red-500/10 transition-colors"
              @click="handleLogout(false); showVrcxMenu=false"
            >
              退出登录
            </button>
          </div>
        </div>

        <div
          class="text-center pt-2 mt-2 border-t"
          :style="{ borderColor: currentTheme.colors.borderSoft }"
        >
          <span
            class="text-[10px] font-mono font-bold tracking-wider opacity-40"
            :style="{ color: currentTheme.colors.textSoft }"
          >v{{ appVersion }}</span>
        </div>
      </div>
    </aside>

    <!-- 主内容区 -->
    <main class="flex-1 relative z-10 overflow-y-auto">
      <div class="p-6 h-full overflow-hidden flex flex-col">
        <KeepAlive>
          <DashboardView v-if="activeTab === 'dashboard'" />
          <FeedView v-else-if="activeTab === 'feed'" />
          <FriendLocationsView v-else-if="activeTab === 'locations'" />
          <ChartsView v-else-if="activeTab === 'charts'" />
          <PlayerListView v-else-if="activeTab === 'playerlist'" />
          <GalleryView v-else-if="activeTab === 'gallery'" />
          <ModerationView v-else-if="activeTab === 'moderation'" />
          <SettingsView v-else-if="activeTab === 'settings'" />
          <FriendsListView v-else-if="activeTab === 'social'" />
          <SearchView v-else-if="activeTab === 'search'" />
          <NotificationsView v-else-if="activeTab === 'notifications'" />
          <MyAvatarsView v-else-if="activeTab === 'avatars'" />
          <GroupsView v-else-if="activeTab === 'groups'" />
          <FavoritesView v-else-if="activeTab === 'favorites'" />
          <HeatmapView v-else-if="activeTab === 'heatmap'" />
          <NotesView v-else-if="activeTab === 'notes'" />
          <StatusPresetsView
            v-else-if="activeTab === 'presets'"
            :user-id="currentUser?.id"
          />
          <BilidownView v-else-if="activeTab === 'bilidown'" />
          <ToolsView v-else-if="activeTab === 'tools'" />
          <TranslatorView v-else-if="activeTab === 'translator'" />
          <OvrTranslatorView v-else-if="activeTab === 'ovr'" />
          <ExportView v-else-if="activeTab === 'export'" />
          <EnvView v-else-if="activeTab === 'env'" />
        </KeepAlive>
      </div>
    </main>
    
    <!-- 全局接口调试面板 -->
    <DebugConsole />
    <DirectOpenModal />
    <UserProfileModal />
    <EntityDetailModals />

    <!-- 服务端断连全屏遮罩 -->
    <div
      v-if="isLoggedIn && clientServerUrl && !serverConnected"
      class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-[9998]"
    >
      <div class="bg-slate-900 border border-red-500/30 rounded-2xl p-8 max-w-sm mx-4 text-center shadow-2xl">
        <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-red-500/20 flex items-center justify-center">
          <svg
            class="w-8 h-8 text-red-400 animate-pulse"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          ><path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M18.364 5.636a9 9 0 010 12.728M5.636 5.636a9 9 0 000 12.728M12 12h.01"
          /></svg>
        </div>
        <h2 class="text-lg font-bold text-red-400 mb-2">
          服务端连接已断开
        </h2>
        <p class="text-slate-400 text-sm mb-4">
          无法连接到 VrcDog 服务端，软件功能已暂停。<br>系统将在 <span class="text-white font-bold">{{ reconnectCountdown }}</span> 秒后自动尝试重连...
        </p>
        <div class="flex gap-2 justify-center">
          <button
            class="px-5 py-2 bg-slate-700 hover:bg-slate-600 text-white rounded-lg text-sm"
            @click="() => handleLogout(false)"
          >
            退出登录
          </button>
        </div>
      </div>
    </div>

    <!-- Global Search Modal -->
    <GlobalSearchModal @navigate="(tab) => activeTab = tab" />

    <!-- Custom Navigation Modal -->
    <CustomNavModal
      v-if="showCustomNavModal"
      :initial-nav-config="editableNavConfig"
      @close="showCustomNavModal = false"
      @save="saveCustomNavConfig"
    />
  </div>
</template>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.3s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
.fade-enter-active > div:nth-child(2) { transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.fade-enter-from > div:nth-child(2) { opacity: 0; transform: translateY(30px) scale(0.9); }
.fade-leave-active > div:nth-child(2) { transition: all 0.2s ease-in; }
.fade-leave-to > div:nth-child(2) { opacity: 0; transform: scale(0.95); }
</style>