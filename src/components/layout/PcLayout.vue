<script setup lang="ts">
import { ref, onMounted, computed, markRaw, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useUiStore } from '../../stores/uiStore';
import { useAuthStore } from '../../stores/authStore';
import { storeToRefs } from 'pinia';
import { getVersion } from '@tauri-apps/api/app';
import { wsState } from '../../api/websocket';
import { currentTheme, setTheme, type ThemeId } from '../../theme';
import VrcAvatar from '../VrcAvatar.vue';
import { LogOut, Monitor, Activity, Globe, MessageSquare, ChevronRight, ChevronDown, Users, ScrollText, List, ShieldAlert, Check } from 'lucide-vue-next';
import CustomNavModal from '../CustomNavModal.vue';

const { t } = useI18n();
const uiStore = useUiStore();
const authStore = useAuthStore();

const { 
  activeTab, 
  activeSidebarTabs, 
  filteredThemes, 
  showVrcDogMenu, 
  showCustomNavModal,
  editableNavConfig,
  vrcServerStatus 
} = storeToRefs(uiStore);

const { currentUser, serverConnected, clientServerUrl } = storeToRefs(authStore);

const appVersion = ref('');
onMounted(async () => {
  try { appVersion.value = await getVersion(); } catch(e) {}
});
const logoLoadFailed = ref(false);
watch(() => currentTheme.value.id, () => {
  logoLoadFailed.value = false;
});

type DensityMode = 'compact' | 'normal' | 'comfortable';

const menuOpenSection = ref<'theme' | 'density' | null>(null);
const densityMode = ref<DensityMode>('normal');
const densityOptions: { key: DensityMode; label: string; navClass: string; childClass: string; menuClass: string }[] = [
  { key: 'compact', label: '紧凑', navClass: 'py-1.5', childClass: 'py-1', menuClass: 'py-1.5' },
  { key: 'normal', label: '标准', navClass: 'py-2', childClass: 'py-1.5', menuClass: 'py-2' },
  { key: 'comfortable', label: '宽松', navClass: 'py-2.5', childClass: 'py-2', menuClass: 'py-2.5' },
];

const currentDensity = computed(() => densityOptions.find(option => option.key === densityMode.value) || densityOptions[1]);
const menuSurfaceColor = computed(() => currentTheme.value.colors.bgMain);
const menuSectionBg = computed(() => {
  switch (currentTheme.value.id) {
    case 'cat': return '#f5fff8';
    case 'helmet': return '#fff6f7';
    case 'mono': return '#ffffff';
    default: return '#fffaf0';
  }
});

watch(showVrcDogMenu, (visible) => {
  if (!visible) {
    menuOpenSection.value = null;
  }
});

const handleSaveCustomNavConfig = async (newConfig: any[] | null) => {
  await uiStore.saveCustomNavConfig(newConfig);
};

const toggleMenuSection = (section: 'theme' | 'density') => {
  menuOpenSection.value = menuOpenSection.value === section ? null : section;
};

const selectThemeFromMenu = (themeId: ThemeId) => {
  setTheme(themeId);
};

const selectDensity = (mode: DensityMode) => {
  densityMode.value = mode;
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

// ── VrcDog 风格折叠子菜单 ──────────────────────────────────────────
// 定义哪些 tab 有子菜单
const subMenuGroups: Record<string, { key: string; label: string; icon: any }[]> = {
  social: [
    { key: 'feed', label: 'sidebar.feed', icon: markRaw(ScrollText) },
    { key: 'friendslist', label: 'sidebar.friendslist', icon: markRaw(List) },
    { key: 'moderation', label: 'sidebar.moderation', icon: markRaw(ShieldAlert) },
  ],
};

// 展开状态
const expandedGroups = ref<Record<string, boolean>>({ social: false });

// 判断某个 tab 是否是子菜单的父级
const isGroupParent = (key: string) => key in subMenuGroups;

// 判断某个 tab 是否是子菜单项（被某个父级包含）
const isSubItem = (key: string) => {
  return Object.values(subMenuGroups).some(children => children.some(c => c.key === key));
};

// 点击父级 tab：展开/折叠子菜单，同时激活父级
const handleTabClick = (key: string) => {
  if (isGroupParent(key)) {
    expandedGroups.value[key] = !expandedGroups.value[key];
    uiStore.activeTab = key;
  } else {
    uiStore.activeTab = key;
  }
};

// 过滤掉被子菜单组管理的独立 tab（避免重复显示）
const managedSubKeys = computed(() => {
  const keys = new Set<string>();
  Object.values(subMenuGroups).forEach(children => children.forEach(c => keys.add(c.key)));
  return keys;
});

const visibleSidebarTabs = computed(() => {
  return activeSidebarTabs.value.filter(tab => !managedSubKeys.value.has(tab.key));
});

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
      class="w-[240px] shadow-lg flex flex-col z-10 p-3 relative flex-shrink-0 transition-all duration-300" :style="{ backgroundColor: 'var(--theme-surface)', borderRight: '1px solid var(--theme-border-soft)' }"
    >
      <div class="flex items-center gap-2.5 mb-2">
        <div
          class="w-10 h-10 rounded-full overflow-hidden border-2 bg-surface flex-shrink-0"
          :style="{ borderColor: currentTheme.colors.borderStrong }"
        >
          <img
            v-if="!logoLoadFailed"
            :src="currentTheme.logo"
            class="w-full h-full object-cover"
            alt=""
            @error="logoLoadFailed = true"
          >
          <Monitor
            v-else
            :size="20"
            class="m-auto"
            :style="{ color: currentTheme.colors.textSoft }"
          />
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
            :title="t(tTheme.name)"
            @click="setTheme(tTheme.id as ThemeId)"
          >
            {{ t(tTheme.name).slice(0,2) }}
          </button>
        </div>

      <div class="flex-1 space-y-0.5 overflow-y-auto custom-scrollbar">
        <template v-for="tab in visibleSidebarTabs" :key="tab.key">
          <!-- 父级 tab（带子菜单） -->
          <div v-if="isGroupParent(tab.key)">
            <button
              class="w-full flex items-center gap-3 px-3 rounded-lg font-bold transition-all text-left text-[13px]"
              :class="currentDensity.navClass"
              :style="activeTab === tab.key || subMenuGroups[tab.key]?.some(c => c.key === activeTab)
                ? { backgroundColor: currentTheme.colors.activeBg, color: currentTheme.colors.textStrong }
                : { color: currentTheme.colors.textSoft }"
              @click="handleTabClick(tab.key)"
            >
              <component :is="tab.icon" :size="18" />
              <span class="flex-1">{{ $t(tab.label) }}</span>
              <ChevronDown
                :size="14"
                class="transition-transform duration-200 opacity-60"
                :class="expandedGroups[tab.key] ? 'rotate-180' : ''"
              />
            </button>
            <!-- 子菜单 -->
            <transition name="submenu">
              <div v-if="expandedGroups[tab.key]" class="ml-3 mt-0.5 space-y-0.5 pl-3">
                <button
                  v-for="child in subMenuGroups[tab.key]"
                  :key="child.key"
                  class="w-full flex items-center gap-2.5 px-3 rounded-lg font-medium transition-all text-left text-[12px]"
                  :class="currentDensity.childClass"
                  :style="activeTab === child.key
                    ? { backgroundColor: currentTheme.colors.activeBg, color: currentTheme.colors.textStrong }
                    : { color: currentTheme.colors.textSoft }"
                  @click="uiStore.activeTab = child.key"
                >
                  <component :is="child.icon" :size="15" />
                  {{ $t(child.label) }}
                </button>
              </div>
            </transition>
          </div>

          <!-- 普通 tab -->
          <button
            v-else
            class="w-full flex items-center gap-3 px-3 rounded-lg font-bold transition-all text-left text-[13px]"
            :class="currentDensity.navClass"
            :style="activeTab === tab.key
              ? { backgroundColor: currentTheme.colors.activeBg, color: currentTheme.colors.textStrong }
              : { color: currentTheme.colors.textSoft }"
            @click="handleTabClick(tab.key)"
          >
            <component :is="tab.icon" :size="18" />
            {{ $t(tab.label) }}
          </button>
        </template>
      </div>

      <!-- 用户信息 + 退出 -->
      <div
        class="mt-auto pt-3 space-y-2 relative"
        :style="{ boxShadow: 'inset 0 1px 0 ' + currentTheme.colors.borderSoft }"
      >
        <div 
          class="flex items-center gap-2.5 cursor-pointer hover:bg-background/5 p-1.5 -ml-1.5 rounded-xl transition-colors relative"
          @click="showVrcDogMenu = !showVrcDogMenu"
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

        <!-- 实时数据流状态 (WebSocket) — 仅在成功连接时显示 -->
        <div
          v-if="wsState.connected"
          class="mt-2 px-2 py-1.5 rounded-lg border text-[10px] font-bold flex items-center justify-between bg-emerald-50 border-emerald-200 text-emerald-600"
        >
          <div class="flex items-center gap-1">
            <Activity
              :size="10"
              class="animate-pulse"
            />
            <span>{{ $t('status.pipeline_online') }}</span>
          </div>
          <span
            v-if="wsState.messageCount > 0"
            class="text-emerald-600"
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
            class="flex-1 flex items-center justify-center gap-1.5 px-2 py-2 rounded-xl text-orange-500 hover:bg-orange-50 font-bold text-xs transition-colors"
            @click="uiStore.appMode = null"
          >
            <Monitor :size="14" /> {{ $t('app.reselect_mode') }}
          </button>
          <button
            class="flex-1 flex items-center justify-center gap-1.5 px-2 py-2 rounded-xl text-red-500 hover:bg-red-50 font-bold text-xs transition-colors"
            @click="() => authStore.handleLogout(false)"
          >
            <LogOut :size="14" /> {{ $t('app.logout') }}
          </button>
        </div>

        <!-- VrcDog-like Settings Menu -->
        <div 
          v-if="showVrcDogMenu" 
          class="absolute bottom-full left-0 mb-3 w-[232px] shadow-2xl rounded-xl overflow-hidden z-50 animate-fade-in isolate"
          :style="{ backgroundColor: menuSurfaceColor, boxShadow: '0 18px 45px rgba(15, 23, 42, 0.18), 0 0 0 1px ' + currentTheme.colors.borderSoft }"
        >
          <div class="p-3 flex items-center justify-between" :style="{ backgroundColor: menuSectionBg }">
            <div class="flex items-center gap-2">
              <MessageSquare class="w-4 h-4" :style="{ color: currentTheme.colors.textStrong }" />
              <span class="font-bold text-[13px]" :style="{ color: currentTheme.colors.textStrong }">{{ $t('app.vrcdog_menu') }}</span>
            </div>
            <span v-if="appVersion" class="text-[11px]" :style="{ color: currentTheme.colors.textSoft }">v{{ appVersion }}</span>
          </div>
          <div class="py-1">
            <button
              class="w-full min-h-9 flex items-center text-left px-4 text-[13px] leading-none transition-colors hover:brightness-95"
              :class="currentDensity.menuClass"
              :style="{ color: currentTheme.colors.textSoft }"
              @click="uiStore.activeTab='settings'; showVrcDogMenu=false"
            >
              <span class="truncate">{{ $t('sidebar.settings') }}</span>
            </button>

            <button
              class="w-full min-h-9 flex justify-between items-center px-4 text-[13px] leading-none transition-colors hover:brightness-95"
              :class="currentDensity.menuClass"
              :style="{ color: currentTheme.colors.textSoft }"
              @click="toggleMenuSection('theme')"
            >
              <span class="truncate">{{ $t('app.theme') }}</span>
              <ChevronRight class="w-4 h-4 opacity-50 transition-transform" :class="menuOpenSection === 'theme' ? 'rotate-90' : ''" />
            </button>
            <div v-if="menuOpenSection === 'theme'" class="px-2 pb-1 space-y-0.5">
              <button
                v-for="theme in Object.values(filteredThemes)"
                :key="theme.id"
                class="w-full min-h-8 flex items-center justify-between gap-2 rounded-lg px-3 text-[12px] leading-none transition-colors hover:brightness-95"
                :style="currentTheme.id === theme.id
                  ? { backgroundColor: currentTheme.colors.activeBg, color: currentTheme.colors.textStrong }
                  : { color: currentTheme.colors.textSoft }"
                @click="selectThemeFromMenu(theme.id as ThemeId)"
              >
                <span class="truncate">{{ t(theme.name) }}</span>
                <Check v-if="currentTheme.id === theme.id" class="w-3.5 h-3.5 flex-shrink-0" />
              </button>
            </div>

            <button
              class="w-full min-h-9 flex justify-between items-center px-4 text-[13px] leading-none transition-colors hover:brightness-95"
              :class="currentDensity.menuClass"
              :style="{ color: currentTheme.colors.textSoft }"
              @click="toggleMenuSection('density')"
            >
              <span class="truncate">{{ $t('app.line_density') }}</span>
              <div class="flex items-center gap-2 min-w-0">
                <span class="text-[11px] opacity-60 truncate">{{ currentDensity.label }}</span>
                <ChevronRight class="w-4 h-4 opacity-50 transition-transform" :class="menuOpenSection === 'density' ? 'rotate-90' : ''" />
              </div>
            </button>
            <div v-if="menuOpenSection === 'density'" class="px-2 pb-1 space-y-0.5">
              <button
                v-for="option in densityOptions"
                :key="option.key"
                class="w-full min-h-8 flex items-center justify-between gap-2 rounded-lg px-3 text-[12px] leading-none transition-colors hover:brightness-95"
                :style="densityMode === option.key
                  ? { backgroundColor: currentTheme.colors.activeBg, color: currentTheme.colors.textStrong }
                  : { color: currentTheme.colors.textSoft }"
                @click="selectDensity(option.key)"
              >
                <span class="truncate">{{ option.label }}</span>
                <Check v-if="densityMode === option.key" class="w-3.5 h-3.5 flex-shrink-0" />
              </button>
            </div>

            <button
              class="w-full min-h-9 flex items-center text-left px-4 text-[13px] leading-none transition-colors hover:brightness-95"
              :class="currentDensity.menuClass"
              :style="{ color: currentTheme.colors.textSoft }"
              @click="showCustomNavModal = true; showVrcDogMenu=false"
            >
              <span class="truncate">{{ $t('app.customize_navbar') }}</span>
            </button>
          </div>
          <div class="py-1" :style="{ boxShadow: 'inset 0 1px 0 ' + currentTheme.colors.borderSoft }">
            <button 
              class="w-full min-h-9 flex items-center text-left px-4 text-[13px] leading-none text-red-500 hover:bg-red-500/10 transition-colors"
              :class="currentDensity.menuClass"
              @click="authStore.handleLogout(false); showVrcDogMenu=false"
            >
              <span class="truncate">{{ $t('app.logout') }}</span>
            </button>
          </div>
        </div>

        <div
          class="text-center pt-2 mt-2"
          :style="{ boxShadow: 'inset 0 1px 0 ' + currentTheme.colors.borderSoft }"
        >
          <span
            class="text-[10px] font-mono font-bold tracking-wider opacity-40"
            :style="{ color: currentTheme.colors.textSoft }"
          >v{{ appVersion }}</span>
        </div>
      </div>
    </aside>

    <!-- 主内容区 -->
    <main class="flex-1 relative z-10 overflow-y-auto shadow-2xl bg-background" :style="{ backgroundColor: currentTheme.colors.surface }">
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

<style scoped>
.submenu-enter-active,
.submenu-leave-active {
  transition: all 0.2s ease;
  overflow: hidden;
}
.submenu-enter-from,
.submenu-leave-to {
  opacity: 0;
  max-height: 0;
  transform: translateY(-4px);
}
.submenu-enter-to,
.submenu-leave-from {
  opacity: 1;
  max-height: 200px;
}
</style>
