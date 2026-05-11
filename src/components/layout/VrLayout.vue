<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useUiStore } from '../../stores/uiStore';
import { useAuthStore } from '../../stores/authStore';
import { storeToRefs } from 'pinia';
import VrcAvatar from '../VrcAvatar.vue';
import { Glasses, Users, Settings, LogOut, Monitor } from 'lucide-vue-next';
import OvrTranslatorView from '../OvrTranslatorView.vue';
import TranslatorView from '../TranslatorView.vue';
import FriendsListView from '../FriendsListView.vue';
import SettingsView from '../SettingsView.vue';
import DebugConsole from '../DebugConsole.vue';
import DirectOpenModal from '../DirectOpenModal.vue';
import { getVersion } from '@tauri-apps/api/app';
import { ref, onMounted } from 'vue';

const { t } = useI18n();
const uiStore = useUiStore();
const authStore = useAuthStore();

const { activeTab } = storeToRefs(uiStore);
const { currentUser, serverConnected, clientServerUrl, isLoggedIn } = storeToRefs(authStore);

const appVersion = ref('');
onMounted(async () => {
  try { appVersion.value = await getVersion(); } catch(e) {}
});

const getStatusColor = (status: string) => {
  switch (status.toLowerCase()) {
    case 'active': case 'join me': return 'bg-primary'
    case 'ask me': return 'bg-orange-500'
    case 'busy': case 'do not disturb': return 'bg-red-500'
    default: return 'bg-surface'
  }
}


const vrTabs = [
  { key: 'ovr', icon: Glasses, label: 'OVR 翻译设置' },
  { key: 'translator', icon: Languages, label: '桌面翻译器' },
  { key: 'social', icon: Users, label: '社交大厅' },
  { key: 'settings', icon: Settings, label: '基础设置' }
];

import { Languages } from 'lucide-vue-next';
</script>

<template>
  <div
    class="flex h-screen overflow-hidden relative"
    style="background: linear-gradient(135deg, #0f0c29, #302b63, #24243e)"
  >
    <!-- VR 深空背景粒子 -->
    <div class="absolute inset-0 z-0 overflow-hidden pointer-events-none">
      <div class="absolute top-[-15%] right-[-10%] w-[50%] h-[50%] bg-primary-hover rounded-full blur-[120px] animate-pulse" />
      <div
        class="absolute bottom-[-10%] left-[-5%] w-[40%] h-[40%] bg-primary-hover rounded-full blur-[100px] animate-pulse"
        style="animation-delay: 3s"
      />
      <div
        class="absolute top-[30%] left-[40%] w-[30%] h-[30%] bg-cyan-500/10 rounded-full blur-[80px] animate-pulse"
        style="animation-delay: 5s"
      />
    </div>

    <!-- VR 侧边栏 -->
    <aside class="w-56 bg-surface backdrop-blur-xl shadow-2xl border-r border-white/10 flex flex-col z-10 p-4 relative flex-shrink-0">
      <div class="flex items-center gap-2.5 mb-4">
        <div class="w-10 h-10 rounded-full overflow-hidden border-2 border-primary/20 bg-indigo-900/50 flex-shrink-0 flex items-center justify-center">
          <Glasses class="w-6 h-6 text-text-muted" />
        </div>
        <div>
          <h2 class="font-bold text-sm leading-tight text-white">
            VrcDog VR
          </h2>
          <p class="text-[10px] font-medium text-text-muted/70">
            OVR Overlay Translator
          </p>
        </div>
      </div>

      <!-- VR 设备状态面板 -->
      <div class="mb-4 p-3 bg-surface rounded-2xl border border-white/10 space-y-2">
        <h3 class="text-[10px] font-bold text-text-muted/60 uppercase tracking-wider mb-1">
          VR 设备状态
        </h3>
        <div class="flex items-center gap-2 text-[11px] text-white/80 font-medium">
          <div class="w-2 h-2 rounded-full bg-green-400 animate-pulse" />
          <span>{{ $t('auto_656cc53f') }}</span>
        </div>
        <div class="flex items-center gap-2 text-[11px] text-white/60 font-medium">
          <div
            class="w-2 h-2 rounded-full"
            :class="currentUser ? 'bg-green-400' : 'bg-surface-hover'"
          />
          <span>{{ currentUser?.displayName || '未登录' }}</span>
        </div>
      </div>

      <!-- VR 导航 -->
      <div class="flex-1 space-y-1 overflow-y-auto">
        <button
          v-for="tab in vrTabs"
          :key="tab.key"
          class="w-full flex items-center gap-2.5 px-3 py-2.5 rounded-xl border font-bold transition-all text-left text-sm"
          :class="activeTab === tab.key
            ? 'bg-primary/20 text-white border-primary/30'
            : 'text-white/50 border-transparent hover:text-white/80 hover:bg-surface'"
          @click="uiStore.activeTab = tab.key as any"
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
            class="flex-1 flex items-center justify-center gap-1.5 px-2 py-2 rounded-xl text-text-muted hover:bg-surface font-bold text-xs transition-colors border border-transparent hover:border-primary/30"
            @click="uiStore.appMode = null"
          >
            <Monitor :size="14" /> 重选模式
          </button>
          <button
            class="flex-1 flex items-center justify-center gap-1.5 px-2 py-2 rounded-xl text-red-400 hover:bg-red-500/10 font-bold text-xs transition-colors border border-transparent hover:border-red-400/20"
            @click="() => authStore.handleLogout(false)"
          >
            <LogOut :size="14" /> {{ $t('app.logout') }}
          </button>
        </div>

        <div class="text-center pt-2 mt-2 border-t border-white/5">
          <span class="text-[10px] font-mono text-text-muted/50 font-bold tracking-wider">v{{ appVersion }}</span>
        </div>
      </div>
    </aside>

    <!-- VR 主内容区 -->
    <main class="flex-1 relative z-10 overflow-y-auto">
      <div v-if="activeTab === 'ovr'" class="p-6 h-full overflow-y-auto">
        <OvrTranslatorView />
      </div>
      <div v-else-if="activeTab === 'translator'" class="p-6 h-full overflow-hidden">
        <TranslatorView />
      </div>
      <div v-else-if="activeTab === 'social'" class="p-6 h-full overflow-hidden">
        <FriendsListView />
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
