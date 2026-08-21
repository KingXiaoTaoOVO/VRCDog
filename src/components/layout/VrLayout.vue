<script setup lang="ts">
import { ref, computed, defineAsyncComponent, onMounted, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useUiStore } from '../../stores/uiStore';
import { useAuthStore } from '../../stores/authStore';
import { storeToRefs } from 'pinia';
import { currentTheme, setTheme, type ThemeId } from '../../theme';
import VrcAvatar from '../VrcAvatar.vue';
import {
  Glasses,
  Users,
  Settings,
  LogOut,
  Monitor,
  Radio,
  Languages,
  Paintbrush,
} from 'lucide-vue-next';
import DebugConsole from '../DebugConsole.vue';
import DirectOpenModal from '../DirectOpenModal.vue';
import { getVersion } from '@tauri-apps/api/app';

const { t } = useI18n();
const uiStore = useUiStore();
const authStore = useAuthStore();

const { activeTab, filteredThemes } = storeToRefs(uiStore);
const { currentUser, serverConnected, clientServerUrl, isLoggedIn } = storeToRefs(authStore);

const sleep = (ms: number) => new Promise((resolve: (value: unknown) => void) => window.setTimeout(resolve, ms));

const isRecoverableViewLoadError = (error: unknown) => {
  const message = String((error as any)?.message || error || '');
  return /ERR_NETWORK_CHANGED|Failed to fetch dynamically imported module|Importing a module script failed|Load failed/i.test(
    message,
  );
};

async function retryViewImport<T>(loader: () => Promise<T>, name: string): Promise<T> {
  let lastError: unknown;
  for (let attempt = 1; attempt <= 5; attempt += 1) {
    try {
      return await loader();
    } catch (error) {
      lastError = error;
      if (!isRecoverableViewLoadError(error) || attempt === 5) break;
      console.warn(`[VrViewLoader] ${name} failed, retrying (${attempt}/5):`, error);
      await sleep(250 * attempt);
    }
  }
  throw lastError;
}

const lazyView = (name: string, loader: () => Promise<any>) =>
  defineAsyncComponent({
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

const OvrTranslatorView = lazyView('OvrTranslatorView', () => import('../OvrTranslatorView.vue'));
const TranslatorView = lazyView('TranslatorView', () => import('../TranslatorView.vue'));
const FriendsListView = lazyView('FriendsListView', () => import('../FriendsListView.vue'));
const SettingsView = lazyView('SettingsView', () => import('../SettingsView.vue'));
const RemoteAssistView = lazyView('RemoteAssistView', () => import('../RemoteAssistView.vue'));
const DanmakuView = lazyView('DanmakuView', () => import('../DanmakuView.vue'));
const DrawingView = lazyView('DrawingView', () => import('../DrawingView.vue'));

const appVersion = ref('');
onMounted(async () => {
  try {
    appVersion.value = await getVersion();
  } catch (e) {
    // ignore
  }
});

const logoLoadFailed = ref(false);
watch(
  () => currentTheme.value.id,
  () => {
    logoLoadFailed.value = false;
  },
);

const themeStyles = computed(() => {
  const colors = currentTheme.value.colors;
  return {
    '--theme-bg-main': colors.bgMain,
    '--theme-surface': colors.surface,
    '--theme-surface-hover': colors.surfaceHover,
    '--theme-blob1': colors.blob1,
    '--theme-blob2': colors.blob2,
    '--theme-border-soft': colors.borderSoft,
    '--theme-border-strong': colors.borderStrong,
    '--theme-text': colors.text,
    '--theme-text-strong': colors.textStrong,
    '--theme-text-soft': colors.textSoft,
    '--theme-text-muted': colors.textMuted,
    '--theme-primary': colors.primaryBtnBg,
    '--theme-primary-hover': colors.primaryBtnHover,
    '--theme-active-bg': colors.activeBg,
    '--theme-glass-effect': colors.glassEffect,
    '--theme-terminal-bg': colors.terminalBg,
  };
});

const getStatusColor = (status: string) => {
  switch (status.toLowerCase()) {
    case 'active':
    case 'join me':
      return 'bg-primary';
    case 'ask me':
      return 'bg-orange-500';
    case 'busy':
    case 'do not disturb':
      return 'bg-red-500';
    default:
      return 'bg-surface';
  }
};

const vrTabs = computed(() => [
  { key: 'ovr', icon: Glasses, label: t('layout.ovr_settings') },
  { key: 'translator', icon: Languages, label: t('layout.desktop_translator') },
  { key: 'danmaku', icon: Radio, label: t('sidebar.danmaku') },
  { key: 'drawing', icon: Paintbrush, label: t('sidebar.drawing') },
  { key: 'remote', icon: Monitor, label: t('remote_assist.title') },
  { key: 'social', icon: Users, label: t('layout.social_lobby') },
  { key: 'settings', icon: Settings, label: t('layout.settings') },
]);
</script>

<template>
  <div
    class="flex h-screen overflow-hidden relative"
    :style="[themeStyles, { background: 'var(--theme-bg-main)' }]"
  >
    <!-- 主题感知装饰光斑（与 PC 端一致的 blob 实现） -->
    <div class="blob blob-1"></div>
    <div class="blob blob-2"></div>

    <!-- 侧边栏（主题感知，匹配 PC 端配色） -->
    <aside
      class="w-56 bg-surface border-r border-border-soft shadow-2xl flex flex-col z-10 p-4 relative flex-shrink-0"
    >
      <div class="flex items-center gap-2.5 mb-4">
        <div
          class="w-10 h-10 rounded-full overflow-hidden border-2 bg-surface flex-shrink-0"
          :style="{ borderColor: 'var(--theme-border-strong)' }"
        >
          <img
            v-if="!logoLoadFailed"
            :src="currentTheme.logo"
            class="w-full h-full object-cover"
            alt=""
            @error="logoLoadFailed = true"
          />
          <Glasses v-else class="w-6 h-6" :style="{ color: 'var(--theme-text-muted)' }" />
        </div>
        <div>
          <h2 class="font-bold text-sm leading-tight" :style="{ color: 'var(--theme-text-strong)' }">
            {{ currentTheme.appTitle }} VR
          </h2>
          <p class="text-[10px] font-medium" :style="{ color: 'var(--theme-text-soft)' }">
            {{ t('layout.ovr_settings') || 'OVR Overlay' }}
          </p>
        </div>
      </div>

      <!-- 主题切换（与 PC 端一致） -->
      <div
        class="flex justify-between items-center bg-surface rounded-xl p-1 mb-4"
        :style="{ border: '1px solid var(--theme-border-soft)' }"
      >
        <button
          v-for="tTheme in Object.values(filteredThemes)"
          :key="tTheme.id"
          class="flex-1 py-1 text-xs font-bold rounded-lg transition-colors flex items-center justify-center gap-1"
          :style="
            currentTheme.id === tTheme.id
              ? { backgroundColor: tTheme.colors.activeBg, color: tTheme.colors.textStrong }
              : { color: tTheme.colors.textSoft, opacity: 0.7 }
          "
          :title="t(tTheme.name)"
          @click="setTheme(tTheme.id as ThemeId)"
        >
          {{ t(tTheme.name).slice(0, 2) }}
        </button>
      </div>

      <!-- VR Status -->
      <div class="mb-4 p-3 bg-surface rounded-2xl border space-y-2" :style="{ borderColor: 'var(--theme-border-soft)' }">
        <h3
          class="text-[10px] font-bold uppercase tracking-wider mb-1"
          :style="{ color: 'var(--theme-text-muted)' }"
        >
          {{ t('layout.vr_status') || 'VR Status' }}
        </h3>
        <div class="flex items-center gap-2 text-[11px] font-medium" :style="{ color: 'var(--theme-text-soft)' }">
          <div class="w-2 h-2 rounded-full bg-green-400 animate-pulse" />
          <span>{{ $t('auto_656cc53f') }}</span>
        </div>
        <div class="flex items-center gap-2 text-[11px] font-medium" :style="{ color: 'var(--theme-text-muted)' }">
          <div
            class="w-2 h-2 rounded-full"
            :class="currentUser ? 'bg-green-400' : 'bg-surface-hover'"
          />
          <span>{{ currentUser?.displayName || t('layout.not_logged_in') }}</span>
        </div>
      </div>

      <!-- VR 导航（主题感知） -->
      <div class="flex-1 space-y-1 overflow-y-auto custom-scrollbar">
        <button
          v-for="tab in vrTabs"
          :key="tab.key"
          class="w-full flex items-center gap-2.5 px-3 py-2.5 rounded-xl border font-bold transition-all text-left text-sm hover:bg-[var(--theme-surface-hover)] hover:text-[var(--theme-text-strong)]"
          :class="activeTab === tab.key ? 'border-primary/30' : 'border-transparent'"
          :style="
            activeTab === tab.key
              ? { backgroundColor: 'var(--theme-active-bg)', color: 'var(--theme-text-strong)' }
              : { color: 'var(--theme-text-soft)' }
          "
          @click="uiStore.activeTab = tab.key as any"
        >
          <component :is="tab.icon" :size="18" />
          {{ tab.label }}
        </button>
      </div>

      <!-- 用户信息 + 退出（主题感知） -->
      <div class="mt-auto pt-3 border-t space-y-2" :style="{ borderColor: 'var(--theme-border-soft)' }">
        <div class="flex items-center gap-2.5">
          <VrcAvatar
            :user="currentUser"
            custom-class="w-9 h-9 rounded-xl object-cover flex-shrink-0"
            :style="{ backgroundColor: 'var(--theme-surface-hover)' }"
          />
          <div class="flex-1 overflow-hidden">
            <p class="text-xs font-bold truncate" :style="{ color: 'var(--theme-text-strong)' }">
              {{ currentUser?.displayName }}
            </p>
            <p
              class="text-[10px] font-bold flex items-center gap-1"
              :class="{
                'text-primary': currentUser?.status === 'active' || currentUser?.status === 'join me',
                'text-orange-400': currentUser?.status === 'ask me',
                'text-red-400': currentUser?.status === 'busy',
                'text-border-strong': !currentUser?.status,
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

        <div
          v-if="clientServerUrl"
          class="mt-1 px-2 py-1.5 rounded-lg border text-[10px] font-bold flex items-center gap-1"
          :class="
            serverConnected
              ? 'bg-emerald-500/10 border-emerald-500/20 text-emerald-400'
              : 'bg-red-500/10 border-red-500/20 text-red-400 animate-pulse'
          "
        >
          <div
            class="w-1.5 h-1.5 rounded-full"
            :class="serverConnected ? 'bg-emerald-400' : 'bg-red-400'"
          />
          <span>{{ serverConnected ? t('layout.server_connected') : t('layout.server_disconnected') }}</span>
        </div>

        <div class="flex gap-2 mt-1">
          <button
            class="flex-1 flex items-center justify-center gap-1.5 px-2 py-2 rounded-xl hover:bg-[var(--theme-surface-hover)] font-bold text-xs transition-colors border border-transparent hover:border-primary/30"
            :style="{ color: 'var(--theme-text-muted)' }"
            @click="uiStore.appMode = null"
          >
            <Monitor :size="14" /> {{ t('layout.reselect_mode') || 'Reselect Mode' }}
          </button>
          <button
            class="flex-1 flex items-center justify-center gap-1.5 px-2 py-2 rounded-xl text-red-400 hover:bg-red-500/10 font-bold text-xs transition-colors border border-transparent hover:border-red-400/20"
            @click="() => authStore.handleLogout(false)"
          >
            <LogOut :size="14" /> {{ $t('app.logout') }}
          </button>
        </div>

        <div class="text-center pt-2 mt-2 border-t" :style="{ borderColor: 'var(--theme-border-soft)' }">
          <span class="text-[10px] font-mono font-bold tracking-wider" :style="{ color: 'var(--theme-text-muted)' }"
            >v{{ appVersion }}</span
          >
        </div>
      </div>
    </aside>

    <!-- VR 主内容区 -->
    <main class="flex-1 relative z-10 overflow-y-auto">
      <div v-show="activeTab === 'translator'" class="p-6 h-full overflow-hidden">
        <TranslatorView />
      </div>
      <div v-if="activeTab === 'ovr'" class="p-6 h-full overflow-y-auto">
        <OvrTranslatorView />
      </div>
      <div v-else-if="activeTab === 'translator'" />
      <div v-else-if="activeTab === 'danmaku'" class="p-6 h-full overflow-hidden">
        <DanmakuView />
      </div>
      <div v-else-if="activeTab === 'drawing'" class="h-full overflow-hidden">
        <DrawingView />
      </div>
      <div v-else-if="activeTab === 'social'" class="p-6 h-full overflow-hidden">
        <FriendsListView />
      </div>
      <div v-else-if="activeTab === 'remote'" class="p-6 h-full overflow-hidden">
        <RemoteAssistView />
      </div>
      <div v-else-if="activeTab === 'settings'" class="p-6 h-full overflow-hidden">
        <SettingsView />
      </div>
      <div v-else class="p-6 h-full overflow-y-auto">
        <OvrTranslatorView />
      </div>
    </main>

    <!-- 全局调试面板 -->
    <DebugConsole />
    <DirectOpenModal />
  </div>
</template>
