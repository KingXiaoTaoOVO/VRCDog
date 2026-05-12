<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useUiStore } from '../../stores/uiStore';
import { useAuthStore } from '../../stores/authStore';
import { storeToRefs } from 'pinia';
import { getVersion } from '@tauri-apps/api/app';
import { wsState } from '../../api/websocket';
import { currentTheme, setTheme, type ThemeId } from '../../theme';
import VrcAvatar from '../VrcAvatar.vue';
import { LogOut, Monitor, Activity, Globe, MessageSquare, ChevronRight } from 'lucide-vue-next';
import CustomNavModal from '../CustomNavModal.vue';

const { t } = useI18n();
const uiStore = useUiStore();
const authStore = useAuthStore();

const { 
  activeTab, 
  activeSidebarTabs, 
  filteredThemes, 
  showVrcxMenu, 
  showCustomNavModal,
  editableNavConfig,
  vrcServerStatus 
} = storeToRefs(uiStore);

const { currentUser, serverConnected, clientServerUrl } = storeToRefs(authStore);

const appVersion = ref('');
onMounted(async () => {
  try { appVersion.value = await getVersion(); } catch(e) {}
});

const handleSaveCustomNavConfig = async (newConfig: any[] | null) => {
  await uiStore.saveCustomNavConfig(newConfig);
};

const getStatusColor = (status: string) => {
  switch (status.toLowerCase()) {
    case 'active': return 'bg-green-500';
    case 'join me': return 'bg-blue-500';
    case 'ask me': return 'bg-orange-500';
    case 'busy': case 'do not disturb': return 'bg-red-500';
    default: return 'bg-slate-400';
  }
}

// 动态注入 CSS 变量
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
</script>

<template>
  <div
    class="flex h-screen overflow-hidden relative"
    :style="themeStyles"
  >
    <!-- 背景装饰色块 -->
    <div class="blob blob-1"></div>
    <div class="blob blob-2"></div>

    <!-- 侧边栏 -->
    <aside
      class="w-[240px] shadow-lg border-r border-white/10 flex flex-col z-10 p-3 relative flex-shrink-0 transition-all duration-300" :style="{ backgroundColor: 'var(--theme-terminal-bg)' }"
    >
      <div class="flex items-center gap-2.5 mb-2">
        <div
          class="w-10 h-10 rounded-full overflow-hidden border-2 bg-surface flex-shrink-0"
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
        class="flex justify-between items-center bg-surface rounded-xl p-1 mb-4"
        :style="{ border: `1px solid ${currentTheme.colors.borderSoft}` }"
      >
        <button
          v-for="tTheme in Object.values(filteredThemes)"
          :key="tTheme.id"
          class="flex-1 py-1 text-xs font-bold rounded-lg transition-colors flex items-center justify-center gap-1"
          :style="currentTheme.id === tTheme.id ? { backgroundColor: tTheme.colors.activeBg, color: tTheme.colors.textStrong } : { color: currentTheme.colors.textSoft, opacity: 0.7 }"
          :title="tTheme.name"
          @click="setTheme(tTheme.id as ThemeId)"
        >
          {{ tTheme.name.slice(0,2) }}
        </button>
      </div>

      <div class="flex-1 space-y-1 overflow-y-auto custom-scrollbar">
        <button
          v-for="tab in activeSidebarTabs"
          :key="tab.key"
          class="w-full flex items-center gap-3 px-3 py-2 rounded-lg font-bold transition-all text-left text-[13px] mb-1"
          :style="activeTab === tab.key ? { backgroundColor: currentTheme.colors.activeBg, color: currentTheme.colors.textStrong, borderColor: currentTheme.colors.borderStrong } : { color: currentTheme.colors.textSoft, borderColor: 'transparent' }"
          @click="uiStore.activeTab = tab.key as any"
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
          class="flex items-center gap-2.5 cursor-pointer hover:bg-background/5 p-1.5 -ml-1.5 rounded-xl transition-colors relative"
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
                'text-primary': currentUser?.status === 'active' || currentUser?.status === 'join me',
                'text-orange-500': currentUser?.status === 'ask me',
                'text-red-500': currentUser?.status === 'busy',
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

        <!-- 实时数据流状态 (WebSocket) -->
        <div
          class="mt-2 px-2 py-1.5 rounded-lg border text-[10px] font-bold flex items-center justify-between"
          :class="wsState.connected ? 'bg-primary/5 border-primary/20 text-primary' : 'bg-orange-50 border-orange-200 text-orange-600'"
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
            class="text-primary"
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
          <span>{{ serverConnected ? $t('app.server_connected') : $t('app.server_disconnected') }}</span>
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
            @click="uiStore.appMode = null"
          >
            <Monitor :size="14" /> {{ $t('app.reselect_mode') }}
          </button>
          <button
            class="flex-1 flex items-center justify-center gap-1.5 px-2 py-2 rounded-xl text-red-500 hover:bg-red-50 font-bold text-xs transition-colors border border-transparent hover:border-red-100"
            @click="() => authStore.handleLogout(false)"
          >
            <LogOut :size="14" /> {{ $t('app.logout') }}
          </button>
        </div>

        <!-- VRCX-like Settings Menu -->
        <div 
          v-if="showVrcxMenu" 
          class="absolute bottom-full left-0 mb-3 w-[220px] glass-panel border border-white/10 shadow-2xl rounded-xl overflow-hidden text-text-muted z-50 animate-fade-in"
        >
          <div class="p-3 flex items-center justify-between border-b border-white/5 bg-surface-hover">
            <div class="flex items-center gap-2">
              <MessageSquare class="w-4 h-4" :style="{ color: currentTheme.colors.textStrong }" />
              <span class="font-bold text-[13px]" :style="{ color: currentTheme.colors.textStrong }">{{ $t('app.vrcx_menu') }}</span>
            </div>
            <span class="text-[11px]" :style="{ color: currentTheme.colors.textSoft }">2026.05.10</span>
          </div>
          <div class="py-1">
            <button class="w-full text-left px-4 py-2 text-[13px] hover:bg-surface transition-colors" :style="{ color: currentTheme.colors.textSoft }" @click="uiStore.activeTab='settings'; showVrcxMenu=false">{{ $t('auto_e366ccf1') }}</button>
            <button class="w-full flex justify-between items-center px-4 py-2 text-[13px] hover:bg-surface transition-colors" :style="{ color: currentTheme.colors.textSoft }">
              {{ $t('app.theme') }} <ChevronRight class="w-4 h-4 opacity-50" />
            </button>
            <button class="w-full flex justify-between items-center px-4 py-2 text-[13px] hover:bg-surface transition-colors" :style="{ color: currentTheme.colors.textSoft }">
              {{ $t('app.line_density') }} <ChevronRight class="w-4 h-4 opacity-50" />
            </button>
            <button class="w-full text-left px-4 py-2 text-[13px] hover:bg-surface transition-colors" :style="{ color: currentTheme.colors.textSoft }" @click="showCustomNavModal = true; showVrcxMenu=false">
              {{ $t('app.customize_navbar') }}
            </button>
          </div>
          <div class="py-1 border-t border-white/5">
            <button 
              class="w-full text-left px-4 py-2 text-[13px] text-red-400 hover:bg-red-500/10 transition-colors"
              @click="authStore.handleLogout(false); showVrcxMenu=false"
            >
              {{ $t('app.logout') }}
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
    <main class="flex-1 relative z-10 overflow-y-auto shadow-2xl border-l border-white/5 bg-background" :style="{ backgroundColor: currentTheme.colors.surface }">
      <div class="p-6 h-full overflow-hidden flex flex-col">
        <slot></slot>
      </div>
    </main>

    <!-- Custom Navigation Modal -->
    <CustomNavModal
      v-if="showCustomNavModal"
      :initial-nav-config="editableNavConfig"
      @close="showCustomNavModal = false"
      @save="handleSaveCustomNavConfig"
    />
  </div>
</template>
