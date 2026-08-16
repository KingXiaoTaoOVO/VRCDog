<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref, watch, watchEffect } from 'vue';
import { useI18n } from 'vue-i18n';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import i18n from './i18n';

// Stores
import { useAuthStore } from './stores/authStore';
import { useUiStore } from './stores/uiStore';
import { useEnvStore } from './stores/envStore';
import { useSystemContextStore } from './stores/systemContext';
import { storeToRefs } from 'pinia';
import { currentTheme } from './theme';
import { DbApi, OvrApi, SysApi, VrcApi } from './api';
import { initGamelogWatcher } from './api/gamelogWatcher';

// Layouts and components
import ToastContainer from './components/ToastContainer.vue';
import ServerDashboardView from './components/ServerDashboardView.vue';
import SurveyCenter from './components/SurveyCenter.vue';
import RoleSelectView from './components/RoleSelectView.vue';
import LoginView from './components/LoginView.vue';
import OverlayView from './components/OverlayView.vue';
import VrpianoOverlayView from './components/VrpianoOverlayView.vue';
import ModeSelect from './components/layout/ModeSelect.vue';
import VrLayout from './components/layout/VrLayout.vue';
import PcLayout from './components/layout/PcLayout.vue';
import DirectOpenModal from './components/DirectOpenModal.vue';
import UserProfileModal from './components/UserProfileModal.vue';
import EntityDetailModals from './components/EntityDetailModals.vue';
import GlobalSearchModal from './components/GlobalSearchModal.vue';
import DebugConsole from './components/DebugConsole.vue';

// Assets
import dogImg from './assets/dog.jpg';
import { Link2, Loader2, RefreshCcw } from 'lucide-vue-next';
import { setAppLocale } from './i18n';

const sleep = (ms: number) => new Promise((resolve) => window.setTimeout(resolve, ms));

const isRecoverableViewLoadError = (error: unknown) => {
  const message = String((error as any)?.message || error || '');
  return /ERR_NETWORK_CHANGED|Failed to fetch dynamically imported module|Importing a module script failed|Load failed/i.test(message);
};

async function retryViewImport<T>(loader: () => Promise<T>, name: string): Promise<T> {
  let lastError: unknown;
  for (let attempt = 1; attempt <= 5; attempt += 1) {
    try {
      return await loader();
    } catch (error) {
      lastError = error;
      if (!isRecoverableViewLoadError(error) || attempt === 5) break;
      console.warn(`[ViewLoader] ${name} failed, retrying (${attempt}/5):`, error);
      await sleep(250 * attempt);
    }
  }
  throw lastError;
}

const lazyView = (name: string, loader: () => Promise<any>) => defineAsyncComponent({
  loader: () => retryViewImport(loader, name),
  delay: 120,
  timeout: 30000,
  onError(error, retry, fail, attempts) {
    if (isRecoverableViewLoadError(error) && attempts <= 5) {
      window.setTimeout(retry, Math.min(250 * attempts, 1500));
      return;
    }
    fail();
  },
});

const DashboardView = lazyView('DashboardView', () => import('./components/DashboardView.vue'));
const FeedView = lazyView('FeedView', () => import('./components/FeedView.vue'));
const FriendLocationsView = lazyView('FriendLocationsView', () => import('./components/FriendLocationsView.vue'));
const ChartsView = lazyView('ChartsView', () => import('./components/ChartsView.vue'));
const PlayerListView = lazyView('PlayerListView', () => import('./components/PlayerListView.vue'));
const GalleryView = lazyView('GalleryView', () => import('./components/GalleryView.vue'));
const ModerationView = lazyView('ModerationView', () => import('./components/ModerationView.vue'));
const SettingsView = lazyView('SettingsView', () => import('./components/SettingsView.vue'));
const FriendsListView = lazyView('FriendsListView', () => import('./components/FriendsListView.vue'));
const SearchView = lazyView('SearchView', () => import('./components/SearchView.vue'));
const NotificationsView = lazyView('NotificationsView', () => import('./components/NotificationsView.vue'));
const MyAvatarsView = lazyView('MyAvatarsView', () => import('./components/MyAvatarsView.vue'));
const GroupsView = lazyView('GroupsView', () => import('./components/GroupsView.vue'));
const FavoritesView = lazyView('FavoritesView', () => import('./components/FavoritesView.vue'));
const HeatmapView = lazyView('HeatmapView', () => import('./components/HeatmapView.vue'));
const NotesView = lazyView('NotesView', () => import('./components/NotesView.vue'));
const StatusPresetsView = lazyView('StatusPresetsView', () => import('./components/StatusPresetsView.vue'));
const BilidownView = lazyView('BilidownView', () => import('./components/BilidownView.vue'));
const DanmakuView = lazyView('DanmakuView', () => import('./components/DanmakuView.vue'));
const VrpianoView = lazyView('VrpianoView', () => import('./components/VrpianoView.vue'));
const ToolsView = lazyView('ToolsView', () => import('./components/ToolsView.vue'));
const TranslatorView = lazyView('TranslatorView', () => import('./components/TranslatorView.vue'));
const OvrTranslatorView = lazyView('OvrTranslatorView', () => import('./components/OvrTranslatorView.vue'));
const ExportView = lazyView('ExportView', () => import('./components/ExportView.vue'));
const EnvView = lazyView('EnvView', () => import('./components/EnvView.vue'));
const RemoteAssistView = lazyView('RemoteAssistView', () => import('./components/RemoteAssistView.vue'));

const { t, locale } = useI18n({ useScope: 'global' });

const authStore = useAuthStore();
const uiStore = useUiStore();
const envStore = useEnvStore();

const { appRole, isLoggedIn, autoLoginLoading, banMessage, serverConnected, reconnectCountdown, clientServerUrl, currentUser, pendingSurveyCount, surveyRequired } = storeToRefs(authStore);
const { appMode, activeTab } = storeToRefs(uiStore);

const overlayMode = new URLSearchParams(window.location.search).get('mode');
const isTranslationOverlayMode = overlayMode === 'overlay';
const isVrpianoOverlayMode = overlayMode === 'vrpiano-overlay';
const isOverlayMode = isTranslationOverlayMode || isVrpianoOverlayMode;
let ovrAutoInitTimer: number | null = null;
let ovrAutoInitInFlight = false;
let ovrWaitingLogged = false;
let allowMainWindowClose = false;

const stopOvrAutoInit = () => {
  if (ovrAutoInitTimer !== null) {
    window.clearInterval(ovrAutoInitTimer);
    ovrAutoInitTimer = null;
  }
};

const ensureOvrInitialized = async () => {
  if (ovrAutoInitInFlight) return;
  ovrAutoInitInFlight = true;
  try {
    const status = await OvrApi.init();
    if (status?.initialized) {
      stopOvrAutoInit();
      ovrWaitingLogged = false;
    } else if (!ovrWaitingLogged) {
      console.info('OpenVR auto-initialization is waiting for SteamVR.');
      ovrWaitingLogged = true;
    }
  } catch (error) {
    if (!ovrWaitingLogged) {
      console.warn('OpenVR auto-initialization is waiting for SteamVR:', error);
      ovrWaitingLogged = true;
    }
  } finally {
    ovrAutoInitInFlight = false;
  }
};

watch(appMode, (mode) => {
  stopOvrAutoInit();
  ovrWaitingLogged = false;
  if (mode !== 'vr' || isOverlayMode || !isTauri()) return;
  void ensureOvrInitialized();
  ovrAutoInitTimer = window.setInterval(() => void ensureOvrInitialized(), 5000);
}, { immediate: true });
onUnmounted(stopOvrAutoInit);
document.documentElement.classList.toggle('translation-overlay-mode', isTranslationOverlayMode);
document.body.classList.toggle('translation-overlay-mode', isTranslationOverlayMode);
document.documentElement.classList.toggle('vrpiano-overlay-mode', isVrpianoOverlayMode);
document.body.classList.toggle('vrpiano-overlay-mode', isVrpianoOverlayMode);
const serverDashboardTarget = ref<{
  mode: 'local' | 'remote';
  url: string;
  password: string;
}>({
  mode: 'local',
  url: '',
  password: '',
});
const disconnectedServerUrl = ref('');
const reconnectingServer = ref(false);
const reconnectServerError = ref('');
const minimizeToTrayEnabled = ref(true);
const surveyCenterOpen = ref(false);
const surveyCenterInitialTab = ref<'pending' | 'history'>('pending');
const currentSurveyUserId = computed(() => currentUser.value?.id || currentUser.value?.displayName || '');

watch([pendingSurveyCount, isLoggedIn], ([pending, loggedIn]) => {
  if (loggedIn && pending > 0) {
    surveyCenterInitialTab.value = 'pending';
    surveyCenterOpen.value = true;
  }
}, { immediate: true });

const handleSurveyResolved = (pending: number, required: boolean) => {
  authStore.resolveSurveyPrompt(pending, required);
  if (pending === 0 && surveyCenterInitialTab.value === 'pending') surveyCenterOpen.value = false;
};

const openSurveyHistory = () => {
  surveyCenterInitialTab.value = 'history';
  surveyCenterOpen.value = true;
};

window.addEventListener('open-survey-center', openSurveyHistory);

watch(clientServerUrl, (value) => {
  disconnectedServerUrl.value = value;
}, { immediate: true });

// Inject dynamic theme variables into the document
watchEffect(() => {
  const root = document.documentElement;
  const theme = currentTheme.value;
  root.style.setProperty('--theme-bg-main', theme.colors.bgMain);
  root.style.setProperty('--theme-surface', theme.colors.surface);
  root.style.setProperty('--theme-surface-hover', theme.colors.surfaceHover);
  root.style.setProperty('--theme-text', theme.colors.text);
  root.style.setProperty('--theme-text-strong', theme.colors.textStrong);
  root.style.setProperty('--theme-text-soft', theme.colors.textSoft);
  root.style.setProperty('--theme-text-muted', theme.colors.textMuted);
  root.style.setProperty('--theme-border-strong', theme.colors.borderStrong);
  root.style.setProperty('--theme-border-soft', theme.colors.borderSoft);
  root.style.setProperty('--theme-blob1', theme.colors.blob1 || theme.colors.primaryBtnBg);
  root.style.setProperty('--theme-blob2', theme.colors.blob2 || theme.colors.primaryBtnBg);
  root.style.setProperty('--theme-primary', theme.colors.primaryBtnBg);
  root.style.setProperty('--theme-primary-hover', theme.colors.primaryBtnHover);
  root.style.setProperty('--theme-active-bg', theme.colors.activeBg);
  root.style.setProperty('--theme-glass-effect', theme.colors.glassEffect);
  root.style.setProperty('--theme-terminal-bg', theme.colors.terminalBg);
});

const handleRoleSelected = async (payload: {
  role: 'client' | 'server';
  url?: string;
  serverMode?: 'local' | 'remote';
  password?: string;
}) => {
  authStore.appRole = payload.role;
  if (payload.role === 'client') {
    await authStore.updateClientServerUrl(payload.url || '', false);
    // 不调用 tryAutoLogin —— 用户必须手动登录或点击已保存账号
    // 自动登录仅由 LoginView 里的 loginWithSavedAccount 触发
    return;
  }
  serverDashboardTarget.value = {
    mode: payload.serverMode || 'local',
    url: payload.url || '',
    password: payload.password || '',
  };
};

const reconnectWithServerUrl = async () => {
  reconnectingServer.value = true;
  reconnectServerError.value = '';
  try {
    const connected = await authStore.updateClientServerUrl(disconnectedServerUrl.value, true);
    if (!connected) reconnectServerError.value = t('app.server_reconnect_failed');
  } catch (error: any) {
    reconnectServerError.value = error?.message || String(error);
  } finally {
    reconnectingServer.value = false;
  }
};

window.addEventListener('settings-updated', (e: any) => {
  if (e.detail?.language) {
    locale.value = setAppLocale(e.detail.language, { persist: false });
  }
  if (typeof e.detail?.minimizeToTray === 'boolean') {
    minimizeToTrayEnabled.value = e.detail.minimizeToTray;
  }
});

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
  if (isOverlayMode) return;
  const sysCtx = useSystemContextStore();
  sysCtx.startPolling();
  await uiStore.loadCustomNavConfig();
  
  if (isTauri()) {
    await listen<{ tab: string }>('ovr_menu_navigate', (event) => {
      const tab = event.payload?.tab;
      if (!tab || !uiStore.sidebarTabs.some((item) => item.key === tab)) return;
      uiStore.appMode = 'vr';
      uiStore.activeTab = tab;
    });
    await listen<Record<string, unknown>>('ovr_config_changed', (event) => {
      void DbApi.saveSetting({
        key: 'ovr_native_runtime_config',
        value: JSON.stringify(event.payload || {}),
      });
    });

    await listen('tray_open_settings', () => {
      uiStore.appMode = 'pc';
      uiStore.activeTab = 'settings';
    });

    try {
      const allSettings = await DbApi.getAllSettings();
      minimizeToTrayEnabled.value = allSettings?.minimizeToTray !== false && allSettings?.minimizeToTray !== 'false';
      await applyProxyFromSettings(allSettings);
    } catch { /* ignore */ }

    const mainWindow = getCurrentWindow();
    await mainWindow.onCloseRequested(async (event) => {
      if (allowMainWindowClose || !minimizeToTrayEnabled.value) return;
      await event.preventDefault();
      try {
        await mainWindow.hide();
      } catch (error) {
        console.error('Failed to hide the main window; closing instead.', error);
        allowMainWindowClose = true;
        await mainWindow.close();
      }
    });

    window.addEventListener('settings-updated', (e: Event) => {
      const customEvent = e as CustomEvent;
      applyProxyFromSettings(customEvent.detail);
    });

    initGamelogWatcher();

    setTimeout(async () => {
      try {
        const args = await SysApi.getLaunchArgs();
        const urlArg = args.find(a =>
          a.startsWith('vrcdog://')
          || a.startsWith('livehime://') // Legacy scheme accepted for existing shortcuts.
          || a.startsWith('vrchat://')
        );
        if (urlArg) {
          if (urlArg.includes('launch/')) {
            const worldId = urlArg.split('launch/')[1];
            if (worldId) {
              const confirmLaunch = confirm(t('app.external_launch', { worldId }));
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

  await envStore.checkEnvironment();
  uiStore.fetchServerStatus();
  setInterval(uiStore.fetchServerStatus, 5 * 60 * 1000);

  if (isTauri()) {
    await listen('install_progress', (event: any) => {
      const p = event.payload;
      if (p.target === 'hub') { envStore.hubProgress = p.progress; envStore.hubProgressMsg = p.status; }
      else if (p.target === 'unity') { envStore.unityProgress = p.progress; envStore.unityProgressMsg = p.status; }
      else if (p.target === 'tool') { envStore.toolProgress = p.progress; envStore.toolProgressMsg = p.status; }
    });
    
    if (isLoggedIn.value) {
      authStore.startFriendsSync();
    }

  }
});

// Window close: notify server
if (typeof window !== 'undefined') {
  window.addEventListener('beforeunload', () => {
    if (!isOverlayMode && clientServerUrl.value && currentUser.value) {
      const uid = currentUser.value.id || currentUser.value.displayName;
      fetch(`${authStore.getBaseUrl()}/api/client/disconnect`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ user_id: uid }),
        keepalive: true
      }).catch(() => {});
    }
  });

  // 防抖：同一时间窗口内多次 auth-expired 事件只处理一次
  let authExpiredTimer: ReturnType<typeof setTimeout> | null = null;
  let authExpiredPending = false;
  window.addEventListener('vrc-auth-expired', async () => {
    if (authExpiredPending) {
      console.warn('[App] Auth expired event already queued, skipping duplicate');
      return;
    }
    authExpiredPending = true;
    console.warn('[App] Auth expired event received, verifying...');

    // 验证：用当前 cookie 再试一次 /auth/user，确认 auth 确实失效
    // 防止 WebSocket 断开等误触发导致用户被强制登出
    try {
      const verifyUser = await VrcApi.request('/auth/user', { method: 'GET', suppressAuthExpired: true });
      if (verifyUser && verifyUser.displayName) {
        // auth 仍然有效，忽略本次事件
        console.log('[App] Auth still valid, ignoring auth-expired event');
        authExpiredPending = false;
        return;
      }
    } catch {
      // verify 也失败了，auth 确实失效
    }

    console.warn('[App] Auth truly expired, clearing login state...');
    try { await DbApi.clearAuth(); } catch {}
    authStore.handleLogout(true);
    // 5 秒内不再重复处理 auth-expired 事件
    if (authExpiredTimer) clearTimeout(authExpiredTimer);
    authExpiredTimer = setTimeout(() => {
      authExpiredPending = false;
      authExpiredTimer = null;
    }, 5000);
  });
}
</script>

<template>
  <ToastContainer v-if="!isOverlayMode" />
  
  <!-- Global Background Elements -->
  <div
    v-if="!isOverlayMode"
    class="fixed inset-0 overflow-hidden pointer-events-none -z-20 bg-background transition-colors duration-700"
  >
    <div class="blob blob-1"></div>
    <div class="blob blob-2"></div>
  </div>

  <OverlayView v-if="isTranslationOverlayMode" />
  <VrpianoOverlayView v-else-if="isVrpianoOverlayMode" />
  
  <ServerDashboardView
    v-else-if="appRole === 'server'"
    :initial-mode="serverDashboardTarget.mode"
    :remote-url="serverDashboardTarget.url"
    :admin-password="serverDashboardTarget.password"
    @exit="authStore.appRole = null"
  />

  <RoleSelectView
    v-else-if="appRole === null"
    @role-selected="handleRoleSelected"
  />

  <div
    v-else-if="autoLoginLoading"
    class="w-full h-screen flex flex-col items-center justify-center bg-background"
  >
    <img
      :src="dogImg"
      class="w-24 h-24 rounded-full border-4 border-border-soft shadow-xl mb-6 animate-pulse"
    >
    <Loader2
      class="animate-spin text-primary mb-3"
      :size="32"
    />
    <p class="text-text-muted font-bold">
      {{ $t('app.loading') }}
    </p>
  </div>

  <div
    v-else-if="!isLoggedIn"
    class="w-full h-full relative"
  >
    <LoginView @login-success="authStore.handleLoginSuccess" />
    <div
      v-if="banMessage"
       class="fixed inset-0 bg-[var(--theme-bg-main)]/70 flex items-center justify-center z-[9999]"
    >
       <div class="bg-[var(--theme-bg-main)]/90 border border-red-500 rounded-xl p-6 max-w-md mx-4 text-center shadow-2xl">
        <div class="text-4xl mb-3">
          🚫
        </div>
        <h2 class="text-xl font-bold text-red-400 mb-3">
          {{ t('app.access_restricted') || 'Access Restricted' }}
        </h2>
        <p class="text-gray-300 text-sm whitespace-pre-line mb-4">
          {{ banMessage }}
        </p>
        <button
          class="px-6 py-2 bg-[var(--theme-primary)] hover:bg-[var(--theme-primary-hover)] text-white rounded-lg text-sm font-bold shadow-md"
          @click="authStore.banMessage = ''"
        >
          {{ t('app.i_know') || 'I Know' }}
        </button>
      </div>
    </div>
  </div>

  <ModeSelect v-else-if="isLoggedIn && !appMode" />

  <VrLayout v-else-if="isLoggedIn && appMode === 'vr'" />

  <PcLayout v-else-if="isLoggedIn && appMode === 'pc'">
    <KeepAlive>
      <DashboardView v-if="activeTab === 'dashboard'" />
      <FeedView v-else-if="activeTab === 'feed'" />
      <FriendLocationsView v-else-if="activeTab === 'locations'" />
      <ChartsView v-else-if="activeTab === 'charts'" />
      <PlayerListView v-else-if="activeTab === 'playerlist'" />
      <GalleryView v-else-if="activeTab === 'gallery'" />
      <ModerationView v-else-if="activeTab === 'moderation'" />
      <SettingsView v-else-if="activeTab === 'settings'" />
      <FriendsListView v-else-if="activeTab === 'social' || activeTab === 'friendslist'" />
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
      <DanmakuView v-else-if="activeTab === 'danmaku'" />
      <VrpianoView v-else-if="activeTab === 'vrpiano'" />
      <ToolsView v-else-if="activeTab === 'tools'" />
      <TranslatorView v-else-if="activeTab === 'translator'" />
      <RemoteAssistView v-else-if="activeTab === 'remote'" />
      <ExportView v-else-if="activeTab === 'export'" />
      <EnvView v-else-if="activeTab === 'env'" />
    </KeepAlive>
  </PcLayout>

  <!-- Global Modals for PC and VR Modes -->
  <template v-if="isLoggedIn && appMode">
    <DebugConsole />
    <DirectOpenModal />
    <UserProfileModal />
    <EntityDetailModals />
    <GlobalSearchModal @navigate="(tab) => uiStore.activeTab = tab" />
    
    <div
      v-if="clientServerUrl && !serverConnected"
       class="fixed inset-0 bg-[var(--theme-bg-main)]/80 backdrop-blur-sm flex items-center justify-center z-[9998]"
    >
      <div class="bg-surface border border-red-500/30 rounded-2xl p-8 max-w-sm mx-4 text-center shadow-2xl">
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
          {{ t('app.server_disconnected') || 'Server Disconnected' }}
        </h2>
        <p class="text-border-strong text-sm mb-4" v-html="$t('app.server_disconnected_desc', { auto_text: $t('auto_7072b137'), countdown: reconnectCountdown })">
        </p>
        <label class="block text-left mb-4">
          <span class="block text-xs font-bold text-text-muted mb-2">{{ t('role.server_address') }}</span>
          <div class="flex items-center gap-2 px-3 py-2.5 bg-surface-hover border border-border-soft rounded-lg focus-within:border-primary">
            <Link2 :size="16" class="text-text-muted shrink-0" />
            <input
              v-model="disconnectedServerUrl"
              type="url"
              class="min-w-0 flex-1 bg-transparent text-sm text-text outline-none"
              :placeholder="t('role.server_address_ph')"
              @keydown.enter="reconnectWithServerUrl"
            >
          </div>
        </label>
        <p v-if="reconnectServerError" class="text-red-400 text-xs mb-3">{{ reconnectServerError }}</p>
        <div class="grid grid-cols-2 gap-2">
          <button
            class="px-5 py-2 bg-surface hover:bg-surface-hover text-white rounded-lg text-sm"
            @click="() => authStore.handleLogout(false)"
          >
            {{ t('app.logout') || 'Logout' }}
          </button>
          <button
            class="px-5 py-2 bg-primary hover:bg-primary-hover text-white rounded-lg text-sm font-bold flex items-center justify-center gap-2 disabled:opacity-60"
            :disabled="reconnectingServer"
            @click="reconnectWithServerUrl"
          >
            <Loader2 v-if="reconnectingServer" :size="15" class="animate-spin" />
            <RefreshCcw v-else :size="15" />
            {{ t('app.save_and_reconnect') }}
          </button>
        </div>
      </div>
    </div>
  </template>

  <SurveyCenter
    v-if="isLoggedIn && surveyCenterOpen && currentSurveyUserId && clientServerUrl"
    :key="`${currentSurveyUserId}:${surveyCenterInitialTab}`"
    :server-url="authStore.getBaseUrl()"
    :user-id="currentSurveyUserId"
    :forced="surveyRequired"
    :initial-tab="surveyCenterInitialTab"
    @resolved="handleSurveyResolved"
    @close="surveyCenterOpen = false"
  />
</template>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.3s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
.fade-enter-active > div:nth-child(2) { transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.fade-enter-from > div:nth-child(2) { opacity: 0; transform: translateY(30px) scale(0.9); }
.fade-leave-active > div:nth-child(2) { transition: all 0.2s ease-in; }
.fade-leave-to > div:nth-child(2) { opacity: 0; transform: scale(0.95); }
</style>
