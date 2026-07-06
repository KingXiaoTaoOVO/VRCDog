<script setup lang="ts">
import { onMounted, watchEffect } from 'vue';
import { useI18n } from 'vue-i18n';
import { isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import i18n from './i18n';

// Stores
import { useAuthStore } from './stores/authStore';
import { useUiStore } from './stores/uiStore';
import { useEnvStore } from './stores/envStore';
import { useSystemContextStore } from './stores/systemContext';
import { storeToRefs } from 'pinia';
import { currentTheme } from './theme';
import { DbApi, SysApi, VrcApi } from './api';
import { initGamelogWatcher } from './api/gamelogWatcher';

// Layouts and components
import ToastContainer from './components/ToastContainer.vue';
import ServerDashboardView from './components/ServerDashboardView.vue';
import RoleSelectView from './components/RoleSelectView.vue';
import LoginView from './components/LoginView.vue';
import OverlayView from './components/OverlayView.vue';
import ModeSelect from './components/layout/ModeSelect.vue';
import VrLayout from './components/layout/VrLayout.vue';
import PcLayout from './components/layout/PcLayout.vue';
import DirectOpenModal from './components/DirectOpenModal.vue';
import UserProfileModal from './components/UserProfileModal.vue';
import EntityDetailModals from './components/EntityDetailModals.vue';
import GlobalSearchModal from './components/GlobalSearchModal.vue';
import DebugConsole from './components/DebugConsole.vue';

// PC Views
import DashboardView from './components/DashboardView.vue';
import FeedView from './components/FeedView.vue';
import FriendLocationsView from './components/FriendLocationsView.vue';
import ChartsView from './components/ChartsView.vue';
import PlayerListView from './components/PlayerListView.vue';
import GalleryView from './components/GalleryView.vue';
import ModerationView from './components/ModerationView.vue';
import SettingsView from './components/SettingsView.vue';
import FriendsListView from './components/FriendsListView.vue';
import SearchView from './components/SearchView.vue';
import NotificationsView from './components/NotificationsView.vue';
import MyAvatarsView from './components/MyAvatarsView.vue';
import GroupsView from './components/GroupsView.vue';
import FavoritesView from './components/FavoritesView.vue';
import HeatmapView from './components/HeatmapView.vue';
import NotesView from './components/NotesView.vue';
import StatusPresetsView from './components/StatusPresetsView.vue';
import BilidownView from './components/BilidownView.vue';
import DanmakuView from './components/DanmakuView.vue';
import ToolsView from './components/ToolsView.vue';
import TranslatorView from './components/TranslatorView.vue';
import OvrTranslatorView from './components/OvrTranslatorView.vue';
import ExportView from './components/ExportView.vue';
import EnvView from './components/EnvView.vue';
import RemoteAssistView from './components/RemoteAssistView.vue';

// Assets
import dogImg from './assets/dog.jpg';
import { Loader2 } from 'lucide-vue-next';
import { setAppLocale } from './i18n';

const { t, locale } = useI18n({ useScope: 'global' });

const authStore = useAuthStore();
const uiStore = useUiStore();
const envStore = useEnvStore();

const { appRole, isLoggedIn, autoLoginLoading, banMessage, serverConnected, reconnectCountdown, clientServerUrl, currentUser } = storeToRefs(authStore);
const { appMode, activeTab } = storeToRefs(uiStore);

const isOverlayMode = window.location.search.includes('mode=overlay');

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

const handleRoleSelected = async (payload: { role: 'client' | 'server', url?: string }) => {
  authStore.appRole = payload.role;
  if (payload.role === 'client') {
    authStore.clientServerUrl = payload.url || '';
    // 不调用 tryAutoLogin —— 用户必须手动登录或点击已保存账号
    // 自动登录仅由 LoginView 里的 loginWithSavedAccount 触发
  }
};

window.addEventListener('settings-updated', (e: any) => {
  if (e.detail?.language) {
    locale.value = setAppLocale(e.detail.language, { persist: false });
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
  const sysCtx = useSystemContextStore();
  sysCtx.startPolling();
  await uiStore.loadCustomNavConfig();
  
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

    setTimeout(async () => {
      try {
        const args = await SysApi.getLaunchArgs();
        const urlArg = args.find(a => a.startsWith('vrcx://') || a.startsWith('vrchat://'));
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
      authStore.syncInitialFriends(); 
    }

    await listen('client_kicked', (event: any) => {
      const kickedUserId = event.payload;
      if (appRole.value === 'client' && currentUser.value && (currentUser.value.id === kickedUserId || currentUser.value.displayName === kickedUserId)) {
         authStore.banMessage = t('auto_e1b5d9e2');
         authStore.handleLogout(true);
      }
    });
    await listen('client_banned', (event: any) => {
      const p = event.payload;
      if (appRole.value === 'client' && currentUser.value && (currentUser.value.id === p.user_id || currentUser.value.displayName === p.user_id)) {
         authStore.banMessage = t('app.banned_message', { reason: p.reason, duration: p.duration_hours ? t('auto_edf6fe7c') + p.duration_hours + t('auto_2de0d491') : t('auto_6280ae83') });
         authStore.handleLogout(true);
      }
    });

    await listen('client_frozen', (event: any) => {
      const p = event.payload;
      if (appRole.value === 'client' && currentUser.value && (currentUser.value.id === p.user_id || currentUser.value.displayName === p.user_id)) {
         authStore.banMessage = t('app.ban_message_prefix', { reason: p.reason }) || `Account Frozen: ${p.reason}`;
         authStore.handleLogout(true);
      }
    });
  }
});

// Window close: notify server
if (typeof window !== 'undefined') {
  window.addEventListener('beforeunload', () => {
    if (clientServerUrl.value && currentUser.value) {
      const uid = currentUser.value.id || currentUser.value.displayName;
      fetch(`${authStore.getBaseUrl()}/api/client/disconnect`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ user_id: uid }),
        keepalive: true
      }).catch(() => {});
    }
  });

  window.addEventListener('vrc-auth-expired', async () => {
    console.warn('[App] Auth expired event received, forcing re-login...');
    try { await DbApi.clearAuth(); } catch {}
    authStore.handleLogout(true);
  });
}
</script>

<template>
  <ToastContainer />
  
  <!-- Global Background Elements -->
  <div class="fixed inset-0 overflow-hidden pointer-events-none -z-20 bg-background transition-colors duration-700">
    <div class="blob blob-1"></div>
    <div class="blob blob-2"></div>
  </div>

  <OverlayView v-if="isOverlayMode" />
  
  <ServerDashboardView
    v-else-if="appRole === 'server'"
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
          class="px-6 py-2 bg-surface hover:bg-surface-hover text-white rounded-lg text-sm"
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
      <ToolsView v-else-if="activeTab === 'tools'" />
      <TranslatorView v-else-if="activeTab === 'translator'" />
      <OvrTranslatorView v-else-if="activeTab === 'ovr'" />
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
        <div class="flex gap-2 justify-center">
          <button
            class="px-5 py-2 bg-surface hover:bg-surface-hover text-white rounded-lg text-sm"
            @click="() => authStore.handleLogout(false)"
          >
            {{ t('app.logout') || 'Logout' }}
          </button>
        </div>
      </div>
    </div>
  </template>
</template>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.3s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
.fade-enter-active > div:nth-child(2) { transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1); }
.fade-enter-from > div:nth-child(2) { opacity: 0; transform: translateY(30px) scale(0.9); }
.fade-leave-active > div:nth-child(2) { transition: all 0.2s ease-in; }
.fade-leave-to > div:nth-child(2) { opacity: 0; transform: scale(0.95); }
</style>
