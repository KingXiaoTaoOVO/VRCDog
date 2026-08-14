<script setup lang="ts">
import { useToast } from "../composables/useToast";

const toast = useToast();
import { ref, onMounted, computed, watch, onErrorCaptured } from 'vue';
import { Settings, Save, Trash2, Globe, Monitor, Shield, HardDrive, Bell, Gamepad2, Check, DownloadCloud, Play, Rocket, Loader2, Zap, Radio, FileJson, FolderOpen, AlertTriangle, Camera, AlertCircle, Eye, EyeOff, Lock, UserCheck, History, Smartphone, Laptop, Fingerprint, Activity, Layers, Sliders, Languages, Cpu, Info, ChevronRight, Glasses, Search, ClipboardList } from 'lucide-vue-next';
import { SysApi, DbApi } from '../api';
import { useI18n } from 'vue-i18n';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { check } from '@tauri-apps/plugin-updater';
import { getVersion } from '@tauri-apps/api/app';
import { open } from '@tauri-apps/plugin-dialog';
import CustomSelect from './CustomSelect.vue';
import { localeOptions, normalizeLocale, setAppLocale } from '../i18n';
import { setDebugLogEnabled } from '../api/debugConfig';
import { setTheme, themes, currentThemeId, type ThemeId } from '../theme';
import { useAuthStore } from '../stores/authStore';
import { useUiStore } from '../stores/uiStore';

const { t, locale } = useI18n();
const authStore = useAuthStore();
const uiStore = useUiStore();
const clientConfigPath = ref('');
const openSurveyCenter = () => window.dispatchEvent(new CustomEvent('open-survey-center'));

const tabs = [
  { id: 'general', label: 'settings.nav_general', icon: Settings },
  { id: 'language', label: 'settings.nav_language', icon: Languages },
  { id: 'interface', label: 'settings.nav_theme', icon: Monitor },
  { id: 'notifications', label: 'settings.nav_notifications', icon: Bell },
  { id: 'network', label: 'settings.nav_network', icon: Globe },
  { id: 'storage', label: 'settings.nav_storage', icon: HardDrive },
  { id: 'discord', label: 'settings.nav_integration', icon: Radio },
  { id: 'auto_launch', label: 'settings.nav_auto_start', icon: Rocket },
  { id: 'advanced', label: 'settings.nav_advanced', icon: Sliders },
  { id: 'security', label: 'settings.nav_security', icon: Shield },
  { id: 'vr', label: 'settings.nav_vr', icon: Gamepad2 },
  { id: 'ovr_ocr', label: 'settings.nav_ocr', icon: Camera },
  { id: 'ovr_trans', label: 'settings.nav_translation', icon: Languages }
];
const vrOnlyTabs = new Set(['vr', 'ovr_ocr', 'ovr_trans']);
const visibleTabs = computed(() => (
  uiStore.appMode === 'pc'
    ? tabs.filter((tab) => !vrOnlyTabs.has(tab.id))
    : tabs
));


const activeTab = ref('general');
const isSaving = ref(false);
const saved = ref(false);

const appVersion = ref('');
const checkUpdateStatus = ref('');
const isCheckingUpdate = ref(false);

watch(() => uiStore.appMode, (mode) => {
  if (mode === 'pc' && vrOnlyTabs.has(activeTab.value)) {
    activeTab.value = 'general';
  }
});

const vrcConfigText = ref('');
const vrcConfigError = ref('');
const vrcConfigSuccess = ref(false);
const vrcConfigSaving = ref(false);

// 组件级别错误兜底，避免单个错误导致整个组件崩溃
onErrorCaptured((err) => {
  console.error('[SettingsView] Caught error:', err);
  // 不阻止错误继续传播（阻止会隐藏深层子组件的问题），只做日志和兜底UI通知
  return false;
});

const loadVrcConfig = async () => {
  try {
    const raw = await SysApi.getVrcConfig();
    vrcConfigText.value = raw;
  } catch (err) {
    console.error('Failed to load config.json', err);
  }
};

const saveVrcConfig = async () => {
  vrcConfigError.value = '';
  vrcConfigSuccess.value = false;
  try {
    JSON.parse(vrcConfigText.value); // Validate JSON format first
  } catch (err: any) {
    vrcConfigError.value = t('settings.vrc_config_invalid');
    return;
  }
  
  try {
    vrcConfigSaving.value = true;
    await SysApi.saveVrcConfig({ content: vrcConfigText.value });
    vrcConfigError.value = t('settings.vrc_config_saved');
    vrcConfigSuccess.value = true;
    setTimeout(() => {
      if (vrcConfigError.value === t('settings.vrc_config_saved')) vrcConfigError.value = '';
    }, 3000);
  } catch (err: any) {
    console.error('Failed to save config.json', err);
    vrcConfigError.value = t('settings.vrc_config_failed');
  } finally {
    vrcConfigSaving.value = false;
  }
};

watch(() => activeTab.value, (newTab) => {
  if (newTab === 'advanced' && !vrcConfigText.value) {
    loadVrcConfig();
  }
});

const pickFolderForConfig = async (key: string) => {
  try {
    const selectedPath = await open({
      directory: true,
      multiple: false,
      title: t('settings.select_dir_title')
    });
    if (selectedPath && typeof selectedPath === 'string') {
      let currentConfig = {} as any;
      try {
        if (vrcConfigText.value) currentConfig = JSON.parse(vrcConfigText.value);
      } catch { /* ignore */ }
      
      currentConfig[key] = selectedPath;
      vrcConfigText.value = JSON.stringify(currentConfig, null, 2);
      
      await saveVrcConfig();
    }
  } catch (err) {
    console.error(err);
  }
};

const config = ref({
  autoCheckUpdate: true,
  autoStart: false,
  minimizeToTray: true,
  topWindow: false,
  language: 'zh-CN',
  theme: 'dog',
  proxyEnabled: false,
  proxyUrl: 'http://127.0.0.1:7890',
  pipelineUrl: '',
  clientServerUrl: 'http://127.0.0.1:11451',
  notifyFriendsOnline: true,
  notifyInvite: true,
  notifyStatusChange: false,
  notifySystem: true,
  notifySound: true,
  notifyTts: false,
  notifyTtsCondition: 'always',
  notifyDesktopCondition: 'always',
  notifyShowWhenAfk: false,
  notifyTtsVolume: 50,
  notifyTtsVoice: '',
  pollInterval: 30,
  cacheLimit: 5,
  discordRpcEnabled: false,
  discordRpcDetails: t('settings.discord_details_default') || '',
  discordRpcState: t('settings.discord_state_default') || '',
  discordRpcEnableWorldIntegration: false,
  discordRpcShowRoomTypeAndCount: true,
  discordRpcShowPlatform: true,
  discordRpcShowRoomInfoInPrivate: false,
  discordRpcShowJoinButton: false,
  discordRpcShowWorldThumbnail: true,
  discordRpcShowWorldName: true,
  translationApiEnabled: false,
  translationApiKey: '',
  youtubeApiEnabled: false,
  youtubeApiKey: '',
  remoteAvatarDbEnabled: false,
  webApiTimeout: 10,
  requestLimit: 10,
  hardwareAcceleration: true,
  customUrlScheme: true,
  oscAutomation: false,
  openLocalFilesWithVrcDog: false,
  clearCacheOnExit: false,
  enableDebugConsole: false,
  vrOverlayEnabled: true,
  vrOverlayOpacity: 80,
  vrHandTracking: false,
  autoStartSteamVR: false,
  wristMode: false,
  ocrContrast: 1.0,
  ocrSharpen: false,
  ocrDenoise: false,
  ocrMergeToleranceX: 0.2,
  ocrMergeToleranceY: 0.3,
  transService: 'tencent',
  transSourceLang: 'auto',
  transTargetLang: 'zh',
  transApiKey: '',
  transAppId: '',
  transLlmModel: '',
  transLlmPrompt: t('ovr.trans_llm_prompt_default'),
  customApiUrl: '',
  ocrLanguage: 'ja',
  ocrSpeedMode: 'balanced',
  statusColor: '#FFFFFF',
  overlayFontSize: 24,
  transPanelMaxWidth: 800,
  gripPressureThreshold: 0.5,
  killAppsOnExit: false,
  autoLaunchApps: [] as string[],
  vrchatLaunchArgs: '',
  // --- 界面设置 (Interface tab) ---
  density: 'normal',
  fontFamily: 'Inter / Noto Sans CJK',
  zoomLevel: 100,
  showVrcPlusIcon: true,
  showRoomId: false,
  showLocalFriendNotes: true,
  showAgeRestrictedRooms: false,
  zebraTableMode: false,
  accessibleStatusIndicator: false,
  useInGameStatusColors: true,
  showNewDashboardButton: true,
  favoriteSortOrder: 'name',
  roomPlayerSortOrder: 'time',
  timeFormat: '24h',
  forceIsoTimeFormat: false,
  firstDayOfWeek: 'monday',
  showOnlineNotes: true,
  showLocalNotes: true,
  hideFriendDeleteEvents: false,
  randomFriendNameColors: true,
  friendColorGuest: '#CCCCCC',
  friendColorNewUser: '#1778FF',
  friendColorUser: '#2BCF5C',
  friendColorKnown: '#FF7B42',
  friendColorTrusted: '#B18FFF',
  friendColorVeteran: '#FF2826',
  friendColorLegend: '#7B2F2F'
});

// 主题切换立即生效
watch(() => config.value.theme, (newTheme) => {
  setTheme(newTheme as ThemeId);
});

const parsedAutoLaunchApps = computed({
  get() {
    try {
      let arr = config.value.autoLaunchApps;
      if (typeof arr === 'string') {
        arr = JSON.parse(arr);
      }
      return Array.isArray(arr) ? arr.join('\n') : '';
    } catch { return ''; }
  },
  set(val) {
    const lines = val.split('\n').map(l => l.trim()).filter(l => l.length > 0);
    (config.value as any).autoLaunchApps = lines;
  }
});

// 从 DB 加载设置
const loadSettings = async () => {
  try {
    const all = await DbApi.getAllSettings();
    if (all && typeof all === 'object') {
      for (const [key, val] of Object.entries(all)) {
        if (key in config.value) {
          const target = config.value as any;
          if (typeof target[key] === 'boolean') {
            target[key] = val === true || val === 'true';
          } else if (typeof target[key] === 'number') {
            target[key] = Number(val) || target[key];
          } else if (key === 'language') {
            target[key] = normalizeLocale(String(val));
          } else {
            target[key] = val;
          }
        }
      }
    }
    // 应用已保存的主题
    if (config.value.theme && themes[config.value.theme as ThemeId]) {
      setTheme(config.value.theme as ThemeId);
    }
    try {
      const clientConfig = await SysApi.getClientServerConfig();
      if (clientConfig?.server_url) config.value.clientServerUrl = clientConfig.server_url;
      clientConfigPath.value = clientConfig?.config_path || '';
    } catch {
      config.value.clientServerUrl = authStore.clientServerUrl || config.value.clientServerUrl;
    }
  } catch (err) {
    console.warn('Failed to load settings:', err);
  }
};

const saveSettings = async () => {
  isSaving.value = true;
  try {
    const entries = Object.entries(config.value);
    for (const [key, val] of entries) {
      await DbApi.saveSetting({ key, value: JSON.stringify(val) });
    }
    const clientConfig = await SysApi.saveClientServerConfig({
      serverUrl: config.value.clientServerUrl,
    }).catch(() => null);
    if (clientConfig?.config_path) clientConfigPath.value = clientConfig.config_path;
    await authStore.updateClientServerUrl(config.value.clientServerUrl, true);

    // 设置窗口置顶
    try {
      await getCurrentWindow().setAlwaysOnTop(config.value.topWindow);
    } catch (e) { console.warn("Failed to set top window", e); }

    // This setting controls VRCDog startup with Windows. It must not be reused
    // as a VRChat crash-restart switch.
    try {
      await SysApi.setAutostart({ enable: config.value.autoStart });
    } catch (e) { console.warn('Failed to update Windows auto-start', e); }

    // 更新多语言引擎并持久化
    if (config.value.language) {
      const nextLocale = setAppLocale(config.value.language, { notify: true });
      config.value.language = nextLocale;
      locale.value = nextLocale;
    }

    // 更新 URL Scheme
    try {
      await SysApi.registerUrlScheme({ enable: config.value.customUrlScheme });
    } catch (e) { console.warn("Failed to register URL scheme", e); }

    // 如果{{ t('settings.state_on') }}启了 Discord RPC，立即更新
    if (config.value.discordRpcEnabled) {
      try {
        await SysApi.setDiscordRpc({
          details: config.value.discordRpcDetails,
          state: config.value.discordRpcState,
        });
      } catch { /* ignore */ }
    } else {
      try { await SysApi.setDiscordRpc({ details: "", state: "" }); } catch { /* ignore */ }
    }

    // 更新 OSC 硬件自动化
    try {
      if (config.value.oscAutomation) {
        await SysApi.startOscAutomation();
      } else {
        await SysApi.stopOscAutomation();
      }
    } catch { /* ignore */ }

    // 触发全局设置更新事件
    window.dispatchEvent(new CustomEvent('settings-updated', { detail: config.value }));

    saved.value = true;
    setTimeout(() => { saved.value = false; }, 2000);
  } catch (err) {
    console.warn('Failed to save settings:', err);
  } finally {
    isSaving.value = false;
  }
};

// 调试控制台开关：立即同步全局标志，无需等待保存
const toggleDebugConsole = () => {
  config.value.enableDebugConsole = !config.value.enableDebugConsole;
  setDebugLogEnabled(config.value.enableDebugConsole);
};

const isClearing = ref(false);
const actionMessage = ref('');
const actionError = ref('');

const clearCache = async () => {
  isClearing.value = true;
  actionMessage.value = '';
  actionError.value = '';
  try {
    const bytesDeleted = await SysApi.clearVrcCache();
    const mbDeleted = (bytesDeleted / 1024 / 1024).toFixed(2);
    actionMessage.value = t('settings.cache_clear_success', { mb: mbDeleted });
    setTimeout(() => { actionMessage.value = ''; }, 3000);
  } catch (err: any) {
    actionError.value = t('settings.cache_clear_fail', { err: err.message || err });
    setTimeout(() => { actionError.value = ''; }, 3000);
  } finally {
    isClearing.value = false;
  }
};

const isClearingAuth = ref(false);
const clearAuth = async () => {
  if (confirm(t('settings.auth_clear_confirm'))) {
    isClearingAuth.value = true;
    actionMessage.value = '';
    try {
      await DbApi.clearAuth();
      actionMessage.value = t('settings.auth_clear_success');
      setTimeout(() => { actionMessage.value = ''; }, 3000);
    } catch (err) {
      console.warn(err);
    } finally {
      isClearingAuth.value = false;
    }
  }
};

const registerSteamVR = async () => {
  try {
    await invoke('sys_register_steamvr_autostart');
    toast.info(t('settings.steamvr_register_success'));
  } catch (err: any) {
    toast.error(t('settings.steamvr_register_fail').replace('{error}', err));
  }
};

const openBindings = async () => {
  try {
    await invoke('sys_open_steamvr_bindings');
  } catch (err: any) {
    toast.error(t('settings.steamvr_bindings_fail').replace('{error}', err));
  }
};

const checkForUpdates = async (silent = false) => {
  if (isCheckingUpdate.value) return;
  isCheckingUpdate.value = true;
  checkUpdateStatus.value = silent ? '' : t('settings.update_checking');
  try {
    const update = await check();
    if (update) {
      if (confirm(t('settings.update_found').replace('{version}', update.version).replace('{body}', update.body || ''))) {
        checkUpdateStatus.value = t('settings.update_downloading');
        await update.downloadAndInstall();
        await invoke('process::restart');
      } else {
        checkUpdateStatus.value = t('settings.update_cancelled');
      }
    } else {
      checkUpdateStatus.value = t('settings.update_latest');
      if (!silent) setTimeout(() => { checkUpdateStatus.value = ''; }, 3000);
    }
  } catch (err) {
    console.error('Update check failed:', err);
    checkUpdateStatus.value = t('settings.update_failed').replace('{error}', String(err));
  } finally {
    isCheckingUpdate.value = false;
  }
};

// 重置列表/表格布局（后续可扩展为实际的布局重置逻辑）
const resetListLayout = () => {
  config.value.density = 'normal';
  config.value.favoriteSortOrder = 'name';
  config.value.roomPlayerSortOrder = 'time';
  toast.info('List layout reset to defaults');
};

const resetTableLayout = () => {
  toast.info('Table layout reset to defaults');
};

onMounted(async () => {
  await loadSettings();
  try {
    appVersion.value = await getVersion();
    if (config.value.autoCheckUpdate) {
      checkForUpdates(true);
    }
  } catch (e) {
    console.error(e);
  }
});

import { useNotificationEngine } from '../stores/notificationEngine';
const { playTts, notify } = useNotificationEngine();

const testTTS = () => {
  playTts(t('settings.test_tts_msg'), config.value.notifyTtsVoice, config.value.notifyTtsVolume);
};

const testNotification = async () => {
  const result = await notify(t('settings.test_notify_title'), t('settings.test_notify_msg'), 'test');
  if (result.desktop === 'sent') {
    toast.success(t('settings.test_notify_msg'));
  } else if (result.desktop === 'denied') {
    toast.error('Windows notification permission was denied');
  } else if (result.desktop === 'unavailable') {
    toast.error('System notifications are unavailable in this build');
  } else {
    toast.info('System notifications are disabled by the current condition');
  }
};

const selectInterfaceLanguage = (nextLocale: string) => {
  const normalized = setAppLocale(nextLocale, { notify: true });
  config.value.language = normalized;
  locale.value = normalized;
  DbApi.saveSetting({ key: 'language', value: JSON.stringify(normalized) }).catch(() => {});
};
</script>
<template>
  <div class="settings-view h-full flex bg-[var(--theme-bg-main)] backdrop-blur-md relative overflow-hidden text-[var(--theme-text)]">
    <!-- Sidebar Menu -->
    <div class="w-64 shrink-0 flex flex-col gap-2 p-4 bg-[var(--theme-surface)] backdrop-blur-3xl overflow-y-auto custom-scrollbar z-20">
      <div class="text-[10px] font-black text-[var(--theme-text-soft)] mb-4 px-2 uppercase tracking-widest mt-2">{{ t('settings.title') || 'Settings' }}</div>
      
      <button 
        v-for="tab in visibleTabs"
        :key="tab.id"
        class="w-full text-left px-4 py-3 rounded-xl transition-all duration-300 flex items-center gap-3 relative group overflow-hidden"
        :class="activeTab === tab.id 
          ? 'bg-[var(--theme-primary)]/18 text-[var(--theme-primary)] shadow-sm ring-1 ring-[var(--theme-primary)]/25'
          : 'text-[var(--theme-text-soft)] hover:bg-[var(--theme-surface-hover)] hover:text-[var(--theme-primary)]'"
        @click="activeTab = tab.id"
      >
        <component :is="tab.icon" class="w-5 h-5 shrink-0 transition-transform group-hover:scale-110" />
        <span class="font-medium text-sm">{{ t(tab.label) }}</span>
      </button>

      <div class="mt-auto pt-6 px-1">
         <button
          class="w-full px-4 py-3 rounded-2xl bg-[var(--theme-primary)] hover:bg-[var(--theme-primary-hover)] text-white font-bold transition-all shadow-xl shadow-[var(--theme-primary)]/30 flex items-center justify-center gap-3 active:scale-95 group"
          @click="saveSettings"
         >
          <Check v-if="saved" class="w-5 h-5" />
          <Loader2 v-else-if="isSaving" class="w-5 h-5 animate-spin" />
          <Save v-else class="w-5 h-5 transition-transform group-hover:rotate-12" />
          {{ saved ? t('settings.saved') : t('settings.save') }}
         </button>
          <div class="mt-4 px-2 py-3 bg-[var(--theme-bg-main)]/5 rounded-xl border border-[var(--theme-border-soft)] flex flex-col gap-1">
           <div class="text-[10px] uppercase tracking-tighter text-[var(--theme-text-muted)]">Core Engine</div>
           <div class="text-xs font-mono font-bold text-[var(--theme-text-soft)] flex items-center justify-between">
             <span>VrcDog v1.2.0</span>
             <span class="w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
           </div>
         </div>
      </div>
    </div>

    <div class="flex-1 p-10 overflow-y-auto custom-scrollbar relative z-10 bg-[var(--theme-bg-main)]/5">
      <div class="max-w-4xl mx-auto mb-8 animate-in fade-in slide-in-from-top-4 duration-500">
        <h1 class="text-4xl font-black text-[var(--theme-text-strong)] mb-2 tracking-tight">{{ t(visibleTabs.find((tab) => tab.id === activeTab)?.label || '') }}</h1>
        <p class="text-[var(--theme-text-soft)] text-sm font-medium">{{ t('settings.subtitle') || 'Configure your personal experience' }}</p>
      </div>

      <Transition
        name="fade"
        mode="out-in"
      >
        <div v-if="activeTab === 'language'" class="language-settings pb-20 animate-in fade-in zoom-in-95 duration-300">
          <section class="language-panel">
            <div class="language-panel-copy">
              <span class="language-panel-icon"><Globe :size="24" /></span>
              <div>
                <h2>{{ t('settings.language_intro') }}</h2>
                <p>{{ t('settings.language_hint') }}</p>
              </div>
            </div>
            <div class="language-grid" role="listbox" :aria-label="t('settings.nav_language')">
              <button
                v-for="option in localeOptions"
                :key="option.value"
                type="button"
                role="option"
                class="language-choice"
                :class="{ selected: config.language === option.value }"
                :aria-selected="config.language === option.value"
                @click="selectInterfaceLanguage(option.value)"
              >
                <span>{{ option.label }}</span>
                <Check v-if="config.language === option.value" :size="16" />
              </button>
            </div>
          </section>
        </div>

        <!-- 界面 (Interface) -->
        <div v-else-if="activeTab === 'interface'" class="space-y-8 pb-20 animate-in fade-in zoom-in-95 duration-300">
          <!-- 外观 -->
          <div>
            <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ $t('auto_afcde261') }}</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-xl transition-all glass-panel-hover">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('settings.theme') }}</div>
                <CustomSelect v-model="config.theme" :options="[
                  { label: 'Dog', value: 'dog' },
                  { label: 'Cat', value: 'cat' },
                  { label: 'Helmet', value: 'helmet' },
                  { label: 'Mono', value: 'mono' }
                ]" />
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_8456bc40') }}</div>
                <CustomSelect v-model="config.fontFamily" :options="[
                  { label: 'Inter / Noto Sans CJK', value: 'Inter / Noto Sans CJK' },
                  { label: 'Segoe UI', value: 'Segoe UI' },
                  { label: 'System Default', value: 'system-ui' }
                ]" />
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_b560fcfe') }}</div>
                <div class="flex items-center gap-3">
                  <button class="w-7 h-7 rounded bg-[var(--theme-surface)]/60 hover:bg-[var(--theme-surface-hover)] border border-[var(--theme-border-soft)] flex items-center justify-center transition-colors" @click="config.zoomLevel = Math.max(50, config.zoomLevel - 10)">-</button>
                  <span class="text-[13px] w-8 text-center font-bold">{{ config.zoomLevel }}%</span>
                  <button class="w-7 h-7 rounded bg-[var(--theme-surface)]/60 hover:bg-[var(--theme-surface-hover)] border border-[var(--theme-border-soft)] flex items-center justify-center transition-colors" @click="config.zoomLevel = Math.min(200, config.zoomLevel + 10)">+</button>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.showVrcPlusIcon = !config.showVrcPlusIcon">
                <div>
                  <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_028b9138') }}</div>
                  <div class="text-[11px] text-[var(--theme-text-muted)] mt-0.5">{{ $t('auto_2617405c') }}</div>
                </div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.showVrcPlusIcon ? 'bg-primary' : 'bg-[var(--theme-surface)]/60'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.showVrcPlusIcon ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- 显示设置 -->
          <div>
            <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ $t('auto_91836294') }}</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.showRoomId = !config.showRoomId">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_31c09df9') }}</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.showRoomId ? 'bg-primary' : 'bg-[var(--theme-surface)]/60'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.showRoomId ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.showLocalFriendNotes = !config.showLocalFriendNotes">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_56f57fac') }}</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.showLocalFriendNotes ? 'bg-primary' : 'bg-[var(--theme-surface)]/60'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.showLocalFriendNotes ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.showAgeRestrictedRooms = !config.showAgeRestrictedRooms">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_ff97bc7a') }}</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.showAgeRestrictedRooms ? 'bg-primary' : 'bg-[var(--theme-surface)]/60'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.showAgeRestrictedRooms ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.zebraTableMode = !config.zebraTableMode">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_2f0ac118') }}</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.zebraTableMode ? 'bg-primary' : 'bg-[var(--theme-surface)]/60'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.zebraTableMode ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.accessibleStatusIndicator = !config.accessibleStatusIndicator">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_7a5da35b') }}</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.accessibleStatusIndicator ? 'bg-primary' : 'bg-[var(--theme-surface)]/60'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.accessibleStatusIndicator ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.useInGameStatusColors = !config.useInGameStatusColors">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_e2eadb04') }}</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.useInGameStatusColors ? 'bg-primary' : 'bg-[var(--theme-surface)]/60'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.useInGameStatusColors ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- 导航 -->
          <div>
            <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ $t('auto_056f2d7d') }}</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.showNewDashboardButton = !config.showNewDashboardButton">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_9fcce8ae') }}</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.showNewDashboardButton ? 'bg-primary' : 'bg-[var(--theme-surface)]/60'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.showNewDashboardButton ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- 列表与表格 -->
          <div>
            <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ $t('auto_c79dc3c2') }}</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_5a9170fe') }}</div>
                <div class="flex bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded p-1 gap-1">
                   <button class="px-3 py-1 text-[12px] rounded shadow-sm transition-colors" :class="config.favoriteSortOrder === 'name' ? 'bg-primary text-white' : 'text-border-strong hover:text-[var(--theme-text-muted)]'" @click="config.favoriteSortOrder = 'name'">{{ $t('auto_d7ec2d3f') }}</button>
                   <button class="px-3 py-1 text-[12px] rounded shadow-sm transition-colors" :class="config.favoriteSortOrder === 'time' ? 'bg-primary text-white' : 'text-border-strong hover:text-[var(--theme-text-muted)]'" @click="config.favoriteSortOrder = 'time'">{{ $t('auto_19fcb9eb') }}</button>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_162a0560') }}</div>
                <div class="flex bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded p-1 gap-1">
                   <button class="px-3 py-1 text-[12px] rounded shadow-sm transition-colors" :class="config.roomPlayerSortOrder === 'time' ? 'bg-primary text-white' : 'text-border-strong hover:text-[var(--theme-text-muted)]'" @click="config.roomPlayerSortOrder = 'time'">{{ $t('auto_19fcb9eb') }}</button>
                   <button class="px-3 py-1 text-[12px] rounded shadow-sm transition-colors" :class="config.roomPlayerSortOrder === 'alphabetical' ? 'bg-primary text-white' : 'text-border-strong hover:text-[var(--theme-text-muted)]'" @click="config.roomPlayerSortOrder = 'alphabetical'">{{ $t('auto_078e09ab') }}</button>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('app.line_density') }}</div>
                <div class="flex bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded p-1 gap-1">
                   <button class="px-3 py-1 text-[12px] rounded shadow-sm transition-colors" :class="config.density === 'compact' ? 'bg-primary text-white' : 'text-border-strong hover:text-[var(--theme-text-muted)]'" @click="config.density = 'compact'">{{ $t('settings.compact') || 'Compact' }}</button>
                   <button class="px-3 py-1 text-[12px] rounded shadow-sm transition-colors" :class="config.density === 'normal' ? 'bg-primary text-white' : 'text-border-strong hover:text-[var(--theme-text-muted)]'" @click="config.density = 'normal'">{{ $t('settings.normal') || 'Normal' }}</button>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_14d026fc') }}</div>
                <button class="px-4 py-1.5 bg-[var(--theme-surface)]/60 hover:bg-[var(--theme-surface-hover)] border border-[var(--theme-border-soft)] rounded text-[13px] transition-colors" @click="resetListLayout()">{{ $t('auto_224e2ccd') }}</button>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_890497ec') }}</div>
                <button class="px-4 py-1.5 bg-[var(--theme-surface)]/60 hover:bg-[var(--theme-surface-hover)] border border-[var(--theme-border-soft)] rounded text-[13px] transition-colors" @click="resetTableLayout()">{{ $t('auto_224e2ccd') }}</button>
              </div>
            </div>
          </div>

          <!-- 时间 / 日期 -->
          <div>
            <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ $t('auto_f7ba9585') }}</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_2ca9949e') }}</div>
                <div class="flex bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded p-1 gap-1">
                   <button class="px-3 py-1 text-[12px] rounded shadow-sm transition-colors" :class="config.timeFormat === '12h' ? 'bg-primary text-white' : 'text-border-strong hover:text-[var(--theme-text-muted)]'" @click="config.timeFormat = '12h'">{{ $t('auto_eafbd6a2') }}</button>
                   <button class="px-3 py-1 text-[12px] rounded shadow-sm transition-colors" :class="config.timeFormat === '24h' ? 'bg-primary text-white' : 'text-border-strong hover:text-[var(--theme-text-muted)]'" @click="config.timeFormat = '24h'">{{ $t('auto_1ba133f7') }}</button>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.forceIsoTimeFormat = !config.forceIsoTimeFormat">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_dc6b2bf6') }}</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.forceIsoTimeFormat ? 'bg-primary' : 'bg-[var(--theme-surface)]/60'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.forceIsoTimeFormat ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_f6e303f2') }}</div>
                <CustomSelect v-model="config.firstDayOfWeek" :options="[
                  { label: $t('auto_5ce43821'), value: 'monday' }, { label: $t('auto_67b19578'), value: 'sunday' }
                ]" />
              </div>
            </div>
          </div>

          <!-- 玩家信息 -->
          <div>
            <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ $t('auto_08d8cee2') }}</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.showOnlineNotes = !config.showOnlineNotes">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_e72dfb69') }}</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.showOnlineNotes ? 'bg-primary' : 'bg-[var(--theme-surface)]/60'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.showOnlineNotes ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.showLocalNotes = !config.showLocalNotes">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_f450eeea') }}</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.showLocalNotes ? 'bg-primary' : 'bg-[var(--theme-surface)]/60'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.showLocalNotes ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- 好友日志 -->
          <div>
            <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ $t('auto_dc67c65a') }}</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.hideFriendDeleteEvents = !config.hideFriendDeleteEvents">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_5a972d68') }}</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.hideFriendDeleteEvents ? 'bg-primary' : 'bg-[var(--theme-surface)]/60'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.hideFriendDeleteEvents ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- 好友名称显示颜色 -->
          <div>
            <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ $t('auto_7e0ec2cc') }}</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.randomFriendNameColors = !config.randomFriendNameColors">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_2c115035') }}</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.randomFriendNameColors ? 'bg-primary' : 'bg-[var(--theme-surface)]/60'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.randomFriendNameColors ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
              
              <template v-if="!config.randomFriendNameColors">
              <!-- Color Grid -->
              <div class="grid grid-cols-2 gap-x-10 gap-y-4 mt-4 px-3">
                <div class="flex flex-col gap-1.5">
                  <span class="text-[12px] text-border-strong font-bold">{{ $t('auto_21b0ef59') }}</span>
                  <div class="flex items-center gap-2">
                    <input type="color" v-model="config.friendColorGuest" class="w-8 h-8 rounded bg-transparent border-0 cursor-pointer p-0">
                    <input v-model="config.friendColorGuest" type="text" class="bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded px-3 py-1.5 text-[13px] w-28 outline-none font-mono text-[var(--theme-text-muted)]">
                  </div>
                </div>
                <div class="flex flex-col gap-1.5">
                  <span class="text-[12px] text-border-strong font-bold">{{ $t('auto_98eb6857') }}</span>
                  <div class="flex items-center gap-2">
                    <input type="color" v-model="config.friendColorNewUser" class="w-8 h-8 rounded bg-transparent border-0 cursor-pointer p-0">
                    <input v-model="config.friendColorNewUser" type="text" class="bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded px-3 py-1.5 text-[13px] w-28 outline-none font-mono text-[var(--theme-text-muted)]">
                  </div>
                </div>
                <div class="flex flex-col gap-1.5">
                  <span class="text-[12px] text-border-strong font-bold">{{ $t('auto_069a4b89') }}</span>
                  <div class="flex items-center gap-2">
                    <input type="color" v-model="config.friendColorUser" class="w-8 h-8 rounded bg-transparent border-0 cursor-pointer p-0">
                    <input v-model="config.friendColorUser" type="text" class="bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded px-3 py-1.5 text-[13px] w-28 outline-none font-mono text-[var(--theme-text-muted)]">
                  </div>
                </div>
                <div class="flex flex-col gap-1.5">
                  <span class="text-[12px] text-border-strong font-bold">{{ $t('auto_ea381c63') }}</span>
                  <div class="flex items-center gap-2">
                    <input type="color" v-model="config.friendColorKnown" class="w-8 h-8 rounded bg-transparent border-0 cursor-pointer p-0">
                    <input v-model="config.friendColorKnown" type="text" class="bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded px-3 py-1.5 text-[13px] w-28 outline-none font-mono text-[var(--theme-text-muted)]">
                  </div>
                </div>
                <div class="flex flex-col gap-1.5">
                  <span class="text-[12px] text-border-strong font-bold">{{ $t('auto_a1f33b77') }}</span>
                  <div class="flex items-center gap-2">
                    <input type="color" v-model="config.friendColorTrusted" class="w-8 h-8 rounded bg-transparent border-0 cursor-pointer p-0">
                    <input v-model="config.friendColorTrusted" type="text" class="bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded px-3 py-1.5 text-[13px] w-28 outline-none font-mono text-[var(--theme-text-muted)]">
                  </div>
                </div>
                <div class="flex flex-col gap-1.5">
                  <span class="text-[12px] text-border-strong font-bold">{{ $t('auto_62d39b03') }}</span>
                  <div class="flex items-center gap-2">
                    <input type="color" v-model="config.friendColorVeteran" class="w-8 h-8 rounded bg-transparent border-0 cursor-pointer p-0">
                    <input v-model="config.friendColorVeteran" type="text" class="bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded px-3 py-1.5 text-[13px] w-28 outline-none font-mono text-[var(--theme-text-muted)]">
                  </div>
                </div>
                <div class="flex flex-col gap-1.5">
                  <span class="text-[12px] text-border-strong font-bold">{{ $t('auto_0b045b6c') }}</span>
                  <div class="flex items-center gap-2">
                    <input type="color" v-model="config.friendColorLegend" class="w-8 h-8 rounded bg-transparent border-0 cursor-pointer p-0">
                    <input v-model="config.friendColorLegend" type="text" class="bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded px-3 py-1.5 text-[13px] w-28 outline-none font-mono text-[var(--theme-text-muted)]">
                  </div>
                </div>
              </div>
              </template>
            </div>
          </div>
        </div>

        <!-- 常规 -->
        <div
          v-else-if="activeTab === 'general'"
          class="space-y-5 max-w-4xl mx-auto mt-4 pb-10"
        >
          <!-- 更新设置 -->
          <div>
            <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ $t('auto_95b9b22c') }}</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors">
                <div class="flex items-center gap-2">
                  <DownloadCloud class="w-4 h-4 text-border-strong" />
                  <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('settings.software_update') }} ({{ $t('settings.current_version') }} v{{ appVersion }})</div>
                </div>
                <button
                  :disabled="isCheckingUpdate"
                  class="px-4 py-1.5 bg-[var(--theme-surface)]-hover/60 backdrop-blur-md hover:bg-[var(--theme-surface)]-active/60 backdrop-blur-md border-border-soft rounded text-[13px] text-text-strong transition-colors disabled:opacity-50 flex items-center gap-2"
                  @click="checkForUpdates(false)"
                >
                  <span v-if="isCheckingUpdate" class="w-3.5 h-3.5 border-border-soft border-t-transparent rounded-full animate-spin" />
                  {{ isCheckingUpdate ? $t('settings.checking_update') : $t('settings.check_update') }}
                </button>
              </div>
              <div v-if="checkUpdateStatus" class="px-3 py-2 text-[12px] text-primary ml-6">
                {{ checkUpdateStatus }}
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.autoCheckUpdate = !config.autoCheckUpdate">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_32f4a3ee') }}</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.autoCheckUpdate ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.autoCheckUpdate ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- 首选项 -->
          <div>
            <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ $t('auto_ccfd50f4') }}</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.autoStart = !config.autoStart">
                <div>
                  <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_c534d49a') }}</div>
                  <div class="text-[11px] text-[var(--theme-text-muted)] mt-0.5">{{ $t('auto_5fb85ff0') }}</div>
                </div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.autoStart ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.autoStart ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.minimizeToTray = !config.minimizeToTray">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_52f59745') }}</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.minimizeToTray ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.minimizeToTray ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.topWindow = !config.topWindow">
                <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_54cfb890') }}</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.topWindow ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.topWindow ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="toggleDebugConsole">
                <div class="flex items-center gap-2">
                  <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_18580f7f') }}</div>
                  <span class="px-1.5 py-0.5 bg-blue-500/20 text-blue-400 text-[10px] rounded font-bold">Dev</span>
                </div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.enableDebugConsole ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.enableDebugConsole ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- 游戏设置 -->
          <div>
            <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ $t('auto_492c27d7') }}</h2>
            <div class="space-y-4 px-3">
              <div>
                <div class="text-[13px] text-[var(--theme-text-muted)] mb-2">{{ $t('auto_915237ea') }}</div>
                <input
                  v-model="config.vrchatLaunchArgs"
                  type="text"
                  :placeholder="$t('auto_360d6522')"
                  class="w-full bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded px-3 py-1.5 text-[13px] text-[var(--theme-text-muted)] outline-none  transition-colors"
                >
                <div class="text-[11px] text-[var(--theme-text-muted)] mt-1.5">{{ $t('auto_a56b0514') }}</div>
              </div>
            </div>
          </div>

          <!-- API 设置 -->
          <div>
            <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ $t('auto_bfb40c81') }}</h2>
            <div class="space-y-1">
              <div class="flex flex-col p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors">
                <div class="flex items-center justify-between mb-2">
                  <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_d54de199') }}</div>
                  <div class="text-[13px] font-bold text-primary">{{ config.pollInterval }}s</div>
                </div>
                <input
                  v-model="config.pollInterval"
                  type="range"
                  min="10"
                  max="120"
                  step="5"
                  class="w-full h-1 bg-[var(--theme-surface)]-active rounded-lg appearance-none cursor-pointer accent-primary"
                >
              </div>
            </div>
          </div>
        </div>

          <!-- 消息通知 (Notifications) -->
          <div v-else-if="activeTab === 'notifications'" class="max-w-4xl mx-auto space-y-8 pb-10 mt-4">
            <div>
              <h2 class="text-[15px] font-bold text-text-strong mb-4 flex items-center gap-2">{{ $t('auto_0090bd38') }}</h2>
              <div class="space-y-1">
                <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.notifySystem = !config.notifySystem">
                  <div>
                    <div class="text-[13px] text-[var(--theme-text-muted)]">系统通知</div>
                    <div class="text-[11px] text-[var(--theme-text-muted)] mt-0.5">收到邀请、好友请求和重要提醒时显示桌面通知</div>
                  </div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.notifySystem ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.notifySystem ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>
                <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors">
                  <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_d5e4f300') }}</div>
                  <CustomSelect v-model="config.notifyDesktopCondition" :options="[
                  { label: $t('auto_487496f2'), value: 'never' }, { label: $t('auto_4ef2caee'), value: 'desktop' }, { label: $t('auto_b3ae5000'), value: 'vr' }, { label: $t('auto_38070670'), value: 'not_vr' }, { label: $t('auto_cc89c3ca'), value: 'vrc_running' }, { label: $t('auto_ab8ca152'), value: 'vrc_not_running' }, { label: $t('auto_986a5f50'), value: 'always' }
                ]" />
                </div>
                <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.notifyShowWhenAfk = !config.notifyShowWhenAfk">
                  <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_f4edc0a7') }}</div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.notifyShowWhenAfk ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.notifyShowWhenAfk ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>
                <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.notifySound = !config.notifySound">
                  <div>
                    <div class="text-[13px] text-[var(--theme-text-muted)]">通知音效</div>
                    <div class="text-[11px] text-[var(--theme-text-muted)] mt-0.5">系统通知触发时播放一声轻提示音</div>
                  </div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.notifySound ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.notifySound ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>
              </div>
            </div>

            <div>
              <h2 class="text-[15px] font-bold text-text-strong mb-4 flex items-center gap-2">{{ $t('auto_d60c11a6') }}</h2>
              <div class="space-y-1">
                <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors">
                  <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_fda097ba') }}</div>
                  <CustomSelect v-model="config.notifyTtsCondition" :options="[
                  { label: $t('auto_487496f2'), value: 'never' }, { label: $t('auto_4ef2caee'), value: 'desktop' }, { label: $t('auto_b3ae5000'), value: 'vr' }, { label: $t('auto_38070670'), value: 'not_vr' }, { label: $t('auto_cc89c3ca'), value: 'vrc_running' }, { label: $t('auto_ab8ca152'), value: 'vrc_not_running' }, { label: $t('auto_986a5f50'), value: 'always' }
                ]" />
                </div>
                
                <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.notifyTts = !config.notifyTts">
                  <div>
                    <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_825e9dca') }}</div>
                    <div class="text-[11px] text-[var(--theme-text-muted)] mt-0.5">{{ $t('auto_79b1d0e6') }}</div>
                  </div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.notifyTts ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.notifyTts ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>

                <div v-if="config.notifyTts" class="flex flex-col p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors ml-6 bg-[var(--theme-primary)]/5 ring-1 ring-[var(--theme-primary)]/15">
                  <div class="flex items-center justify-between mb-2">
                    <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_aebf393e') }}</div>
                    <div class="text-[13px] font-bold text-primary">{{ config.notifyTtsVolume }}%</div>
                  </div>
                  <input
                    v-model="config.notifyTtsVolume"
                    type="range"
                    min="0"
                    max="100"
                    step="1"
                    class="w-full h-1 bg-[var(--theme-surface)]-active rounded-lg appearance-none cursor-pointer accent-primary"
                  >
                </div>

                <div class="flex gap-4 p-3 mt-2">
                  <button
                    class="px-4 py-1.5 bg-[var(--theme-surface)]-hover/60 backdrop-blur-md hover:bg-[var(--theme-surface)]-active/60 backdrop-blur-md border-border-soft rounded text-[13px] text-[var(--theme-text-muted)] hover:text-text-strong transition-colors flex items-center gap-2"
                    @click="testTTS"
                  >
                    <Play class="w-3.5 h-3.5" /> {{ t('settings.play_test') || 'Play Test' }}
                  </button>
                  <button
                    class="px-4 py-1.5 bg-[var(--theme-surface)]-hover/60 backdrop-blur-md hover:bg-[var(--theme-surface)]-active/60 backdrop-blur-md border-border-soft rounded text-[13px] text-[var(--theme-text-muted)] hover:text-text-strong transition-colors flex items-center gap-2"
                    @click="testNotification"
                  >
                    <Bell class="w-3.5 h-3.5" /> {{ t('settings.send_notification') || 'Test Notify' }}
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- 网络 (Network) -->
          <div v-else-if="activeTab === 'network'" class="max-w-4xl mx-auto space-y-8 pb-10 mt-4">
            <div>
              <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ t('settings.client_server_title') }}</h2>
              <div class="p-4 bg-[var(--theme-surface)]/60 border border-[var(--theme-border-soft)] rounded-lg">
                <label class="block">
                  <span class="block text-[13px] font-bold text-[var(--theme-text-muted)] mb-2">{{ t('role.server_address') }}</span>
                  <div class="flex items-center gap-2 px-3 py-2.5 bg-[var(--theme-bg-main)]/40 border border-[var(--theme-border-soft)] rounded-lg focus-within:border-[var(--theme-primary)]">
                    <Globe class="w-4 h-4 text-[var(--theme-text-muted)] shrink-0" />
                    <input
                      v-model="config.clientServerUrl"
                      type="url"
                      :placeholder="t('role.server_address_ph')"
                      class="min-w-0 flex-1 bg-transparent text-[13px] text-[var(--theme-text)] outline-none font-mono"
                    >
                  </div>
                </label>
                <div class="mt-3 flex items-start gap-2 text-[11px] text-[var(--theme-text-muted)]">
                  <FileJson class="w-4 h-4 shrink-0" />
                  <span>
                    {{ t('settings.client_server_file_desc') }}
                    <code v-if="clientConfigPath" class="block mt-1 break-all text-[var(--theme-primary)]">{{ clientConfigPath }}</code>
                  </span>
                </div>
                <div class="mt-3 flex flex-wrap gap-2">
                  <button
                    type="button"
                    class="px-3 py-2 border border-[var(--theme-border-soft)] rounded-lg text-[12px] font-bold text-[var(--theme-text-muted)] hover:bg-[var(--theme-surface-hover)] flex items-center gap-2"
                    @click="SysApi.openDir({ target: 'client_config' })"
                  >
                    <FolderOpen class="w-4 h-4" />
                    {{ t('settings.open_client_config_dir') }}
                  </button>
                  <button
                    type="button"
                    class="px-3 py-2 border border-[var(--theme-border-soft)] rounded-lg text-[12px] font-bold text-[var(--theme-text-muted)] hover:bg-[var(--theme-surface-hover)] flex items-center gap-2"
                    @click="openSurveyCenter"
                  >
                    <ClipboardList class="w-4 h-4" />
                    我的问卷记录
                  </button>
                </div>
              </div>
            </div>
            <div>
              <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ t('settings.pipeline_url_title') }}</h2>
              <div class="p-4 bg-[var(--theme-surface)]/60 border border-[var(--theme-border-soft)] rounded-lg">
                <label class="block">
                  <div class="flex items-center gap-2 px-3 py-2.5 bg-[var(--theme-bg-main)]/40 border border-[var(--theme-border-soft)] rounded-lg focus-within:border-[var(--theme-primary)]">
                    <Radio class="w-4 h-4 text-[var(--theme-text-muted)] shrink-0" />
                    <input
                      v-model="config.pipelineUrl"
                      type="text"
                      :placeholder="t('settings.pipeline_url_placeholder')"
                      class="min-w-0 flex-1 bg-transparent text-[13px] text-[var(--theme-text)] outline-none font-mono"
                    >
                  </div>
                </label>
                <div class="mt-3 flex items-start gap-2 text-[11px] text-[var(--theme-text-muted)]">
                  <Info class="w-4 h-4 shrink-0 mt-0.5" />
                  <span>{{ t('settings.pipeline_url_desc') }}</span>
                </div>
              </div>
            </div>
            <div>
              <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ $t('auto_898db399') }}</h2>
              <div class="space-y-1">
                <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.proxyEnabled = !config.proxyEnabled">
                  <div class="flex items-center gap-2">
                    <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_36d4ed47') }}</div>
                    <span class="px-1.5 py-0.5 bg-blue-500/20 text-blue-400 text-[10px] rounded font-bold">{{ $t('auto_a5327004') }}</span>
                  </div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.proxyEnabled ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.proxyEnabled ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>

                <div v-if="config.proxyEnabled" class="p-3 ml-6 rounded-xl bg-[var(--theme-primary)]/5 ring-1 ring-[var(--theme-primary)]/15">
                  <div class="text-[13px] text-[var(--theme-text-muted)] mb-2">{{ $t('auto_4a669973') }}</div>
                  <input
                    v-model="config.proxyUrl"
                    type="text"
                    :placeholder="$t('auto_d3352c52')"
                    class="w-full max-w-sm bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded px-3 py-1.5 text-[13px] text-[var(--theme-text-muted)] outline-none  font-mono transition-colors"
                  >
                </div>
              </div>
            </div>
          </div>

          <!-- 缓存 -->
          <div
            v-else-if="activeTab === 'storage'"
            class="max-w-4xl mx-auto space-y-8 pb-10 mt-4"
          >
            <div>
              <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ $t('auto_ec84c3ae') }}</h2>
              <div class="space-y-4">
                <div class="p-4 bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded-lg">
                  <div class="flex items-center justify-between mb-4">
                    <div>
                      <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_be0e8c4a') }}</div>
                      <div class="text-[11px] text-[var(--theme-text-muted)] mt-0.5">{{ $t('auto_52235201') }}</div>
                    </div>
                    <button
                      :disabled="isClearing"
                      class="px-4 py-1.5 bg-red-500/10 hover:bg-red-500/20 border-red-500/20 rounded text-[13px] text-red-400 transition-colors flex items-center gap-2 disabled:opacity-50"
                      @click="clearCache"
                    >
                      <Trash2 class="w-3.5 h-3.5" :class="{'animate-spin': isClearing}" />
                      {{ isClearing ? t('settings.clearing') : t('settings.clear_all') }}
                    </button>
                  </div>
                  <div v-if="actionMessage && activeTab === 'storage'" class="px-3 py-2 text-[12px] text-green-400 bg-green-400/10 rounded mb-2">{{ actionMessage }}</div>
                  <div v-if="actionError && activeTab === 'storage'" class="px-3 py-2 text-[12px] text-red-400 bg-red-400/10 rounded">{{ actionError }}</div>
                </div>

                <div class="p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors">
                  <div class="flex items-center justify-between mb-2">
                    <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_fd9439f9') }}</div>
                    <div class="text-[13px] font-bold text-primary">{{ config.cacheLimit }} GB</div>
                  </div>
                  <div class="text-[11px] text-[var(--theme-text-muted)] mb-3">{{ $t('auto_7b0fed42') }}</div>
                  <input
                    v-model="config.cacheLimit"
                    type="range"
                    min="1"
                    max="20"
                    class="w-full h-1 bg-[var(--theme-surface)]-active rounded-lg appearance-none cursor-pointer accent-primary"
                  >
                </div>
              </div>
            </div>
          </div>

          <!-- 集成 (Discord / APIs) -->
          <div v-else-if="activeTab === 'discord'" class="max-w-4xl mx-auto space-y-8 pb-10 mt-4">
            <div>
              <h2 class="text-[15px] font-bold text-text-strong mb-4 flex items-center gap-2">
                Discord RPC
                <span class="px-1.5 py-0.5 bg-primary/20 text-primary text-[10px] rounded font-bold">{{ $t('auto_cb1700cc') }}</span>
              </h2>
              <div class="text-[12px] text-amber-500/80 mb-4 px-1">{{ $t('auto_326b53f6') }}</div>

              <div class="space-y-1">
                <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.discordRpcEnabled = !config.discordRpcEnabled">
                  <div class="text-[13px] text-[var(--theme-text-muted)] font-bold">{{ $t('auto_3c9a1ecb') }}</div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.discordRpcEnabled ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.discordRpcEnabled ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>

                <template v-if="config.discordRpcEnabled">
                  <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.discordRpcEnableWorldIntegration = !config.discordRpcEnableWorldIntegration">
                    <div>
                      <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_6eb92b82') }}</div>
                      <div class="text-[11px] text-[var(--theme-text-muted)] mt-0.5">{{ $t('auto_eb117849') }}</div>
                    </div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.discordRpcEnableWorldIntegration ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.discordRpcEnableWorldIntegration ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>

                  <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.discordRpcShowRoomTypeAndCount = !config.discordRpcShowRoomTypeAndCount">
                    <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_91b7dcbf') }}</div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.discordRpcShowRoomTypeAndCount ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.discordRpcShowRoomTypeAndCount ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>

                  <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.discordRpcShowPlatform = !config.discordRpcShowPlatform">
                    <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_086cf470') }}</div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.discordRpcShowPlatform ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.discordRpcShowPlatform ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>

                  <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.discordRpcShowRoomInfoInPrivate = !config.discordRpcShowRoomInfoInPrivate">
                    <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_aa0494ff') }}</div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.discordRpcShowRoomInfoInPrivate ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.discordRpcShowRoomInfoInPrivate ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>

                  <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.discordRpcShowJoinButton = !config.discordRpcShowJoinButton">
                    <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_73fbf843') }}</div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.discordRpcShowJoinButton ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.discordRpcShowJoinButton ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>

                  <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.discordRpcShowWorldThumbnail = !config.discordRpcShowWorldThumbnail">
                    <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_f5dce70b') }}</div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.discordRpcShowWorldThumbnail ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.discordRpcShowWorldThumbnail ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>

                  <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.discordRpcShowWorldName = !config.discordRpcShowWorldName">
                    <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_7c5900a9') }}</div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.discordRpcShowWorldName ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.discordRpcShowWorldName ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>

                  <div class="p-3 border-border-soft mt-2 pt-4">
                    <div class="flex items-center gap-4">
                      <div class="flex-1">
                        <div class="text-[13px] text-[var(--theme-text-muted)] mb-1">{{ $t('auto_544f4d90') }}</div>
                        <input
                          v-model="config.discordRpcDetails"
                          type="text"
                          class="w-full bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded px-3 py-1.5 text-[13px] text-[var(--theme-text-muted)] outline-none  transition-colors"
                        >
                      </div>
                      <div class="flex-1">
                        <div class="text-[13px] text-[var(--theme-text-muted)] mb-1">{{ $t('auto_c181c420') }}</div>
                        <input
                          v-model="config.discordRpcState"
                          type="text"
                          class="w-full bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded px-3 py-1.5 text-[13px] text-[var(--theme-text-muted)] outline-none  transition-colors"
                        >
                      </div>
                    </div>
                  </div>
                </template>
              </div>
            </div>

            <!-- Translation API -->
            <div class="pt-4 border-border-soft">
              <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ $t('auto_b48b3914') }}</h2>
              <div class="space-y-1">
                <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.translationApiEnabled = !config.translationApiEnabled">
                  <div>
                    <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_da56e03d') }}</div>
                    <div class="text-[11px] text-[var(--theme-text-muted)] mt-0.5">{{ $t('auto_0c780a55') }}</div>
                  </div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.translationApiEnabled ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.translationApiEnabled ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>
                <div v-if="config.translationApiEnabled" class="p-3 ml-6 rounded-xl bg-[var(--theme-primary)]/5 ring-1 ring-[var(--theme-primary)]/15">
                  <div class="text-[13px] text-[var(--theme-text-muted)] mb-2">{{ $t('auto_f49cefbb') }}</div>
                  <input
                    v-model="config.translationApiKey"
                    type="password"
                    :placeholder="$t('auto_850b7af5')"
                    class="w-full max-w-sm bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded px-3 py-1.5 text-[13px] text-[var(--theme-text-muted)] outline-none  font-mono transition-colors"
                  >
                </div>
              </div>
            </div>

            <!-- YouTube API -->
            <div class="pt-4 border-border-soft">
              <h2 class="text-[15px] font-bold text-text-strong mb-4">YouTube API</h2>
              <div class="space-y-1">
                <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.youtubeApiEnabled = !config.youtubeApiEnabled">
                  <div>
                    <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_35f77773') }}</div>
                    <div class="text-[11px] text-[var(--theme-text-muted)] mt-0.5">{{ $t('auto_562dae68') }}</div>
                  </div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.youtubeApiEnabled ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.youtubeApiEnabled ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>
                <div v-if="config.youtubeApiEnabled" class="p-3 ml-6 rounded-xl bg-[var(--theme-primary)]/5 ring-1 ring-[var(--theme-primary)]/15">
                  <div class="text-[13px] text-[var(--theme-text-muted)] mb-2">{{ $t('auto_f49cefbb') }}</div>
                  <input
                    v-model="config.youtubeApiKey"
                    type="password"
                    :placeholder="$t('auto_850b7af5')"
                    class="w-full max-w-sm bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded px-3 py-1.5 text-[13px] text-[var(--theme-text-muted)] outline-none  font-mono transition-colors"
                  >
                </div>
              </div>
            </div>

            <!-- Remote Avatar Database -->
            <div class="pt-4 border-border-soft">
              <h2 class="text-[15px] font-bold text-text-strong mb-4">{{ $t('auto_5dd326af') }}</h2>
              <div class="space-y-1">
                <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.remoteAvatarDbEnabled = !config.remoteAvatarDbEnabled">
                  <div>
                    <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_e5c256af') }}</div>
                    <div class="text-[11px] text-[var(--theme-text-muted)] mt-0.5">{{ $t('auto_a1104dd4') }}</div>
                  </div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.remoteAvatarDbEnabled ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.remoteAvatarDbEnabled ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>
                <div v-if="config.remoteAvatarDbEnabled" class="flex justify-end pt-2 px-3">
                  <button class="px-4 py-1.5 bg-[var(--theme-surface)]-hover/60 backdrop-blur-md hover:bg-[var(--theme-surface)]-active/60 backdrop-blur-md border-border-soft rounded text-[13px] text-[var(--theme-text-muted)] hover:text-text-strong transition-colors">
                    {{ t('settings.contribute_avatar') || 'Contribute Avatar Data' }}
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- 第三方程序自动化 (Auto Launch) -->
          <div v-else-if="activeTab === 'auto_launch'" class="max-w-4xl mx-auto space-y-8 pb-10 mt-4">
            <div>
              <div class="flex items-center gap-2 mb-4 text-white">
                <Rocket :size="18" class="text-primary" />
                <h2 class="text-[15px] font-bold">{{ $t('auto_2a9254e8') }}</h2>
              </div>
              <div class="text-[12px] text-border-strong mb-6">{{ $t('auto_fe5c7aa5') }}</div>

              <div class="space-y-4">
                <div class="space-y-1">
                  <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.killAppsOnExit = !config.killAppsOnExit">
                    <div>
                      <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_b4a6c98c') }}</div>
                      <div class="text-[11px] text-[var(--theme-text-muted)] mt-0.5">{{ $t('auto_167f8433') }}</div>
                    </div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.killAppsOnExit ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.killAppsOnExit ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>

                  <div class="flex items-center justify-between p-3 hover:bg-[var(--theme-surface)] rounded-lg transition-colors cursor-pointer" @click="config.clearCacheOnExit = !config.clearCacheOnExit">
                    <div>
                      <div class="text-[13px] text-[var(--theme-text-muted)]">{{ $t('auto_5161a1f8') }}</div>
                      <div class="text-[11px] text-[var(--theme-text-muted)] mt-0.5">{{ $t('auto_d3e8fc8e') }}</div>
                    </div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.clearCacheOnExit ? 'bg-primary' : 'bg-[var(--theme-surface)]-active'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-[var(--theme-surface)] shadow-sm transition-all" :class="config.clearCacheOnExit ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>
                </div>

                <div class="p-4 bg-[var(--theme-surface)]/60 backdrop-blur-md border-border-soft rounded-lg">
                  <div class="text-[13px] text-[var(--theme-text-muted)] mb-2">{{ $t('auto_e100edbb') }}</div>
                  <div class="text-[11px] text-[var(--theme-text-muted)] mb-3">{{ $t('auto_92742c96') }}</div>
                  <textarea
                    v-model="parsedAutoLaunchApps"
                    rows="6"
                    class="w-full bg-background/80 backdrop-blur-md border-border-soft rounded-lg px-3 py-2 text-[13px] text-[var(--theme-text-muted)] outline-none  font-mono resize-y"
                    placeholder="C:\Program Files\Example\app.exe&#10;D:\Tools\OSC\tracker.exe"
                  />
                </div>
              </div>
            </div>
          </div>

        <!-- 高级设置 (Advanced) -->
        <div v-else-if="activeTab === 'advanced'" class="space-y-8 pb-20 animate-in fade-in zoom-in-95 duration-300">
          <section class="glass-panel p-6 space-y-6">
            <div class="flex items-center gap-3 mb-2">
              <div class="p-2 bg-primary/20 rounded-lg text-primary">
                <Settings :size="20" />
              </div>
              <div>
                <h2 class="text-xl font-bold text-text">{{ t('settings.section_advanced') }}</h2>
                <p class="text-xs text-[var(--theme-text-muted)]">{{ t('settings.advanced_desc') || 'System-level configurations and tweaks' }}</p>
              </div>
            </div>

            <div class="space-y-2">
              <!-- Network Settings -->
              <div class="flex items-center justify-between p-4 bg-[var(--theme-surface)]/40 hover:bg-[var(--theme-surface)]-hover/50 rounded-2xl border border-white/5 transition-all">
                <div class="flex items-center gap-4">
                  <div class="p-2 bg-blue-500/10 rounded-lg text-blue-400">
                    <Globe :size="18" />
                  </div>
                  <div>
                    <div class="text-sm font-bold text-text">{{ t('settings.web_api_timeout') || 'API Timeout' }}</div>
                    <div class="text-[11px] text-[var(--theme-text-muted)]">Seconds before request fails</div>
                  </div>
                </div>
                <div class="flex items-center gap-3">
                  <input
                    v-model.number="config.webApiTimeout"
                    type="number"
                    min="1"
                    max="60"
                    class="w-20 bg-background/50 border border-white/10 rounded-lg px-3 py-1.5 text-sm font-mono text-center focus:border-primary/50 outline-none transition-all"
                  >
                  <span class="text-xs text-[var(--theme-text-muted)]">s</span>
                </div>
              </div>

              <!-- Hardware Acceleration -->
              <div 
                class="flex items-center justify-between p-4 bg-[var(--theme-surface)]/40 hover:bg-[var(--theme-surface)]-hover/50 rounded-2xl border border-white/5 transition-all cursor-pointer group"
                @click="config.hardwareAcceleration = !config.hardwareAcceleration"
              >
                <div class="flex items-center gap-4">
                  <div class="p-2 bg-orange-500/10 rounded-lg text-orange-400">
                    <Zap :size="18" />
                  </div>
                  <div>
                    <div class="text-sm font-bold text-text">{{ t('settings.hardware_accel') || 'Hardware Acceleration' }}</div>
                    <div class="text-[11px] text-red-400/70">{{ t('settings.restart_notice') || 'Requires restart to take effect' }}</div>
                  </div>
                </div>
                <div 
                  class="w-12 h-6 rounded-full relative transition-all duration-300"
                   :class="config.hardwareAcceleration ? 'bg-primary' : 'bg-[var(--theme-bg-main)]/10 dark:bg-[var(--theme-text)]/10'"
                >
                  <div 
                    class="absolute top-1 w-4 h-4 rounded-full bg-white shadow-lg transition-all duration-300"
                    :class="config.hardwareAcceleration ? 'left-7' : 'left-1'"
                  />
                </div>
              </div>

              <!-- OSC Automation -->
              <div 
                class="flex items-center justify-between p-4 bg-[var(--theme-surface)]/40 hover:bg-[var(--theme-surface)]-hover/50 rounded-2xl border border-white/5 transition-all cursor-pointer group"
                @click="config.oscAutomation = !config.oscAutomation"
              >
                <div class="flex items-center gap-4">
                  <div class="p-2 bg-purple-500/10 rounded-lg text-purple-400">
                    <Radio :size="18" />
                  </div>
                  <div>
                    <div class="text-sm font-bold text-text">{{ t('settings.osc_auto') || 'OSC Automation' }}</div>
                    <div class="text-[11px] text-[var(--theme-text-muted)]">{{ t('settings.osc_auto_desc') || 'Automatic OSC parameters handling' }}</div>
                  </div>
                </div>
                <div 
                  class="w-12 h-6 rounded-full relative transition-all duration-300"
                   :class="config.oscAutomation ? 'bg-primary' : 'bg-[var(--theme-bg-main)]/10 dark:bg-[var(--theme-text)]/10'"
                >
                  <div 
                    class="absolute top-1 w-4 h-4 rounded-full bg-white shadow-lg transition-all duration-300"
                    :class="config.oscAutomation ? 'left-7' : 'left-1'"
                  />
                </div>
              </div>
            </div>
          </section>

          <!-- VRChat Config Editor -->
          <section class="glass-panel p-6 space-y-6">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-3">
                <div class="p-2 bg-emerald-500/20 rounded-lg text-emerald-400">
                  <FileJson :size="20" />
                </div>
                <div>
                  <h2 class="text-xl font-bold text-text">{{ t('settings.vrc_config_editor') || 'VRChat Config' }}</h2>
                  <p class="text-xs text-[var(--theme-text-muted)]">Edit config.json directly</p>
                </div>
              </div>
              <button 
                :disabled="vrcConfigSaving" 
                class="px-6 py-2 bg-primary hover:bg-primary/90 text-white font-bold text-sm rounded-xl transition-all shadow-lg shadow-primary/20 flex items-center gap-2 disabled:opacity-50 active:scale-95"
                @click="saveVrcConfig"
              >
                <Save v-if="!vrcConfigSaving" :size="16" />
                <Loader2 v-else :size="16" class="animate-spin" />
                {{ vrcConfigSaving ? 'Saving...' : 'Save Changes' }}
              </button>
            </div>

            <div class="space-y-4">
              <div class="flex gap-3">
                <button
                  class="flex-1 px-4 py-3 bg-[var(--theme-surface)]/40 hover:bg-[var(--theme-surface)]-hover/50 border border-white/5 rounded-2xl text-xs text-[var(--theme-text-muted)] hover:text-text transition-all flex items-center justify-center gap-2 group"
                  @click="pickFolderForConfig('cache_directory')"
                >
                  <HardDrive :size="16" class="group-hover:text-primary" /> Set Cache Dir
                </button>
                <button
                  class="flex-1 px-4 py-3 bg-[var(--theme-surface)]/40 hover:bg-[var(--theme-surface)]-hover/50 border border-white/5 rounded-2xl text-xs text-[var(--theme-text-muted)] hover:text-text transition-all flex items-center justify-center gap-2 group"
                  @click="pickFolderForConfig('camera_res_dir')"
                >
                  <Camera :size="16" class="group-hover:text-primary" /> Set Photo Dir
                </button>
              </div>

              <div class="relative group">
                <div class="absolute -inset-0.5 bg-gradient-to-r from-emerald-500/20 to-primary/20 rounded-2xl blur opacity-30 group-hover:opacity-50 transition duration-500"></div>
                <textarea
                  v-model="vrcConfigText"
                  spellcheck="false"
                   class="relative w-full h-80 p-6 bg-[var(--theme-bg-main)]/40 text-emerald-400 font-mono text-[13px] rounded-2xl border border-white/10 outline-none focus:border-emerald-500/50 custom-scrollbar resize-none transition-all"
                  placeholder="{}"
                />
              </div>

              <div v-if="vrcConfigError" class="p-4 rounded-xl flex items-center gap-3 animate-in fade-in zoom-in-95 duration-300" :class="vrcConfigSuccess ? 'bg-green-500/10 text-green-400 border border-green-500/20' : 'bg-red-500/10 text-red-400 border border-red-500/20'">
                <AlertCircle :size="16" />
                <span class="text-xs font-bold">{{ vrcConfigError }}</span>
              </div>
            </div>
          </section>

          <!-- Danger Zone -->
          <section class="glass-panel p-6 border-red-500/20 bg-red-500/5">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-3">
                <div class="p-2 bg-red-500/20 rounded-lg text-red-400">
                  <AlertTriangle :size="20" />
                </div>
                <div>
                  <h2 class="text-xl font-bold text-red-400">Danger Zone</h2>
                  <p class="text-xs text-red-400/60">Destructive actions cannot be undone</p>
                </div>
              </div>
              <button class="px-6 py-2 bg-red-500/10 hover:bg-red-500 text-red-400 hover:text-white rounded-xl text-sm font-bold transition-all border border-red-500/20 active:scale-95">
                Reset All Settings
              </button>
            </div>
          </section>
        </div>

        <!-- 隐私安全 (Security) -->
        <div v-else-if="activeTab === 'security'" class="space-y-8 pb-20 animate-in fade-in zoom-in-95 duration-300">
          <section class="glass-panel p-6 space-y-6">
            <div class="flex items-center gap-3 mb-2">
              <div class="p-2 bg-red-500/20 rounded-lg text-red-400">
                <Shield :size="20" />
              </div>
              <div>
                <h2 class="text-xl font-bold text-text">{{ t('settings.section_security') }}</h2>
                <p class="text-xs text-[var(--theme-text-muted)]">{{ t('settings.auth_security') }}</p>
              </div>
            </div>

            <div class="space-y-4">
              <!-- Force Logout Card -->
              <div class="p-6 bg-red-500/5 border border-red-500/20 rounded-2xl space-y-4">
                <div class="flex items-start gap-4">
                  <div class="p-3 bg-red-500/10 rounded-xl text-red-400">
                    <Lock :size="24" />
                  </div>
                  <div class="flex-1">
                    <div class="text-base font-bold text-red-400">{{ t('settings.force_logout') }}</div>
                    <p class="text-[11px] text-red-400/60 leading-relaxed mt-1">
                      {{ t('settings.auth_security_desc') || 'Logs out all sessions and clears locally stored cookies. You will need to login again.' }}
                    </p>
                  </div>
                </div>

                <button
                  :disabled="isClearingAuth"
                  class="w-full py-3 bg-red-500/10 hover:bg-red-500 text-red-400 hover:text-white rounded-xl text-sm font-bold transition-all border border-red-500/20 flex items-center justify-center gap-2 active:scale-95 disabled:opacity-50"
                  @click="clearAuth"
                >
                  <Trash2 v-if="!isClearingAuth" :size="18" />
                  <Loader2 v-else :size="18" class="animate-spin" />
                  {{ isClearingAuth ? t('settings.clearing_auth') : t('settings.force_logout') }}
                </button>
              </div>
            </div>
          </section>

          <p v-if="actionMessage && activeTab === 'security'" class="p-4 bg-green-500/10 text-green-400 border border-green-500/20 rounded-xl text-xs font-bold animate-in slide-in-from-bottom-2">
            {{ actionMessage }}
          </p>
        </div>

        <!-- VR Settings -->
        <div v-else-if="activeTab === 'vr'" class="space-y-8 pb-20 animate-in fade-in zoom-in-95 duration-300">
          <section class="glass-panel p-6 space-y-6">
            <div class="flex items-center gap-3 mb-2">
              <div class="p-2 bg-indigo-500/20 rounded-lg text-indigo-400">
                <Monitor :size="20" />
              </div>
              <div>
                <h2 class="text-xl font-bold text-text">{{ t('settings.section_vr') }}</h2>
                <p class="text-xs text-[var(--theme-text-muted)]">In-game overlay and wrist features</p>
              </div>
            </div>

            <div class="space-y-3">
              <!-- Enable Overlay -->
              <div 
                class="flex items-center justify-between p-4 bg-[var(--theme-surface)]/40 hover:bg-[var(--theme-surface)]-hover/50 rounded-2xl border border-white/5 transition-all cursor-pointer group"
                @click="config.vrOverlayEnabled = !config.vrOverlayEnabled"
              >
                <div class="flex items-center gap-4">
                  <div class="p-2 bg-indigo-500/10 rounded-lg text-indigo-400">
                    <Layers :size="18" />
                  </div>
                  <div>
                    <div class="text-sm font-bold text-text">{{ t('settings.vr_overlay_enable') }}</div>
                    <div class="text-[11px] text-[var(--theme-text-muted)]">{{ t('settings.vr_overlay_enable_desc') }}</div>
                  </div>
                </div>
                <div 
                  class="w-12 h-6 rounded-full relative transition-all duration-300"
                   :class="config.vrOverlayEnabled ? 'bg-primary' : 'bg-[var(--theme-bg-main)]/10 dark:bg-[var(--theme-text)]/10'"
                >
                  <div 
                    class="absolute top-1 w-4 h-4 rounded-full bg-white shadow-lg transition-all duration-300"
                    :class="config.vrOverlayEnabled ? 'left-7' : 'left-1'"
                  />
                </div>
              </div>

              <!-- Opacity Slider -->
              <div class="p-4 bg-[var(--theme-surface)]/40 rounded-2xl border border-white/5 space-y-3">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-3 text-sm font-bold text-text">
                    <Eye :size="16" class="text-[var(--theme-text-muted)]" />
                    {{ t('settings.vr_overlay_opacity') }}
                  </div>
                  <div class="text-xs font-mono text-primary">{{ config.vrOverlayOpacity }}%</div>
                </div>
                <input
                  v-model="config.vrOverlayOpacity"
                  type="range"
                  min="10"
                  max="100"
                  step="5"
                  class="w-full h-1.5 bg-background/50 rounded-lg appearance-none cursor-pointer accent-primary"
                >
              </div>

              <!-- Wrist Mode -->
              <div 
                class="flex items-center justify-between p-4 bg-[var(--theme-surface)]/40 hover:bg-[var(--theme-surface)]-hover/50 rounded-2xl border border-white/5 transition-all cursor-pointer group"
                @click="config.wristMode = !config.wristMode"
              >
                <div class="flex items-center gap-4">
                  <div class="p-2 bg-purple-500/10 rounded-lg text-purple-400">
                    <Activity :size="18" />
                  </div>
                  <div>
                    <div class="text-sm font-bold text-text">{{ t('settings.vr_hand_tracking') }}</div>
                    <div class="text-[11px] text-[var(--theme-text-muted)]">{{ t('settings.vr_hand_tracking_desc') }}</div>
                  </div>
                </div>
                <div 
                  class="w-12 h-6 rounded-full relative transition-all duration-300"
                   :class="config.wristMode ? 'bg-primary' : 'bg-[var(--theme-bg-main)]/10 dark:bg-[var(--theme-text)]/10'"
                >
                  <div 
                    class="absolute top-1 w-4 h-4 rounded-full bg-white shadow-lg transition-all duration-300"
                    :class="config.wristMode ? 'left-7' : 'left-1'"
                  />
                </div>
              </div>
            </div>
          </section>

          <!-- SteamVR Action Card -->
          <section class="glass-panel p-6 space-y-4">
            <div class="flex items-center gap-3">
               <div class="p-2 bg-blue-500/20 rounded-lg text-blue-400">
                 <Rocket :size="20" />
               </div>
               <div>
                 <h2 class="text-xl font-bold text-text">{{ t('settings.steamvr_integration') }}</h2>
                 <p class="text-xs text-[var(--theme-text-muted)]">{{ t('settings.steamvr_integration_desc') }}</p>
               </div>
            </div>
            
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
               <button 
                class="p-4 bg-primary/10 hover:bg-primary border border-primary/20 rounded-2xl text-left transition-all group active:scale-95"
                @click="registerSteamVR"
               >
                 <div class="text-primary group-hover:text-white font-bold text-sm mb-1">{{ t('settings.steamvr_register') }}</div>
                 <div class="text-primary/60 group-hover:text-white/60 text-[10px]">{{ t('settings.steamvr_register_desc') }}</div>
               </button>
               <button 
                 class="p-4 bg-[var(--theme-bg-main)]/5 dark:bg-[var(--theme-text)]/5 hover:bg-[var(--theme-bg-main)]/10 dark:hover:bg-[var(--theme-text)]/10 border border-[var(--theme-border-soft)] dark:border-[var(--theme-text)]/10 rounded-2xl text-left transition-all group active:scale-95"
                @click="openBindings"
               >
                 <div class="text-text group-hover:text-primary font-bold text-sm mb-1">{{ t('settings.steamvr_bindings') }}</div>
                 <div class="text-[var(--theme-text-muted)] text-[10px]">{{ t('settings.steamvr_bindings_desc') }}</div>
               </button>
            </div>
          </section>
        </div>

        <!-- OVR OCR Settings -->
        <div v-else-if="activeTab === 'ovr_ocr'" class="space-y-8 pb-20 animate-in fade-in zoom-in-95 duration-300">
          <section class="glass-panel p-6 space-y-6">
            <div class="flex items-center gap-3 mb-2">
              <div class="p-2 bg-blue-500/20 rounded-lg text-blue-400">
                <Search :size="20" />
              </div>
              <div>
                <h2 class="text-xl font-bold text-text">{{ t('settings.ocr_title') }}</h2>
                <p class="text-xs text-[var(--theme-text-muted)]">{{ t('settings.ocr_desc') }}</p>
              </div>
            </div>

            <div class="space-y-4">
              <!-- Lang Selection -->
              <div class="p-4 bg-[var(--theme-surface)]/40 rounded-2xl border border-white/5 space-y-3">
                <div class="text-sm font-bold text-text">{{ t('settings.ocr_capture_language') }}</div>
                <CustomSelect v-model="config.ocrLanguage" :options="[
                  { label: t('settings.language_japanese'), value: 'ja' },
                  { label: t('settings.language_english'), value: 'en-US' },
                  { label: t('settings.language_chinese_simplified'), value: 'zh-Hans-CN' },
                  { label: t('settings.language_chinese_traditional'), value: 'zh-Hant-TW' },
                  { label: t('settings.language_korean'), value: 'ko' }
                ]" />
              </div>

              <!-- Speed/Accuracy -->
              <div class="p-4 bg-[var(--theme-surface)]/40 rounded-2xl border border-white/5 space-y-3">
                <div class="text-sm font-bold text-text">{{ t('settings.ocr_processing_strategy') }}</div>
                <CustomSelect v-model="config.ocrSpeedMode" :options="[
                  { label: t('settings.ocr_fast'), value: 'fast' },
                  { label: t('settings.ocr_balanced'), value: 'balanced' },
                  { label: t('settings.ocr_accurate'), value: 'accurate' }
                ]" />
              </div>

              <!-- Advanced Image Tweak -->
              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div class="p-4 bg-[var(--theme-surface)]/30 rounded-2xl border border-white/5 space-y-3">
                   <div class="text-xs font-bold text-[var(--theme-text-muted)]">{{ t('settings.ocr_merge_x') }}</div>
                   <input v-model.number="config.ocrMergeToleranceX" type="range" min="0.0" max="1.0" step="0.05" class="w-full h-1 bg-background/50 rounded-lg appearance-none cursor-pointer accent-primary">
                </div>
                <div class="p-4 bg-[var(--theme-surface)]/30 rounded-2xl border border-white/5 space-y-3">
                   <div class="text-xs font-bold text-[var(--theme-text-muted)]">{{ t('settings.ocr_merge_y') }}</div>
                   <input v-model.number="config.ocrMergeToleranceY" type="range" min="0.0" max="1.0" step="0.05" class="w-full h-1 bg-background/50 rounded-lg appearance-none cursor-pointer accent-primary">
                </div>
              </div>
            </div>
          </section>
        </div>

        <!-- OVR Translation Settings -->
        <div v-else-if="activeTab === 'ovr_trans'" class="space-y-8 pb-20 animate-in fade-in zoom-in-95 duration-300">
          <section class="glass-panel p-6 space-y-6">
            <div class="flex items-center gap-3 mb-2">
              <div class="p-2 bg-emerald-500/20 rounded-lg text-emerald-400">
                <Languages :size="20" />
              </div>
              <div>
                <h2 class="text-xl font-bold text-text">{{ t('settings.translation_engine') }}</h2>
                <p class="text-xs text-[var(--theme-text-muted)]">{{ t('settings.translation_engine_desc') }}</p>
              </div>
            </div>

            <div class="space-y-4">
              <!-- Service Provider -->
              <div class="p-4 bg-[var(--theme-surface)]/40 rounded-2xl border border-white/5 space-y-3">
                <div class="text-sm font-bold text-text">{{ t('settings.translation_provider') }}</div>
                <CustomSelect v-model="config.transService" :options="[
                  { label: t('settings.translation_builtin'), value: 'builtin' },
                  { label: 'DeepSeek', value: 'deepseek' },
                  { label: 'OpenAI (GPT)', value: 'openai' },
                  { label: 'Tencent', value: 'tencent' },
                  { label: 'Google', value: 'google' }
                ]" />
              </div>
            </div>
          </section>
        </div>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
.settings-view :deep(input[type="range"]) {
  width: 100%;
  height: 18px;
  appearance: none;
  -webkit-appearance: none;
  background: transparent;
  cursor: pointer;
  accent-color: var(--theme-primary);
}

.settings-view :deep(input[type="range"]::-webkit-slider-runnable-track) {
  height: 8px;
  border-radius: 999px;
  border: 1px solid color-mix(in srgb, var(--theme-border-strong) 76%, transparent);
  background:
    linear-gradient(180deg, rgba(0, 0, 0, 0.14), rgba(255, 255, 255, 0.2)),
    color-mix(in srgb, var(--theme-primary) 18%, var(--theme-bg-main));
  box-shadow: inset 0 2px 5px rgba(69, 26, 3, 0.18), 0 1px 0 rgba(255, 255, 255, 0.58);
}

.settings-view :deep(input[type="range"]::-webkit-slider-thumb) {
  width: 18px;
  height: 18px;
  margin-top: -6px;
  appearance: none;
  -webkit-appearance: none;
  border-radius: 999px;
  border: 3px solid #fff;
  background: var(--theme-primary);
  box-shadow: 0 4px 12px color-mix(in srgb, var(--theme-primary) 34%, transparent), 0 0 0 1px color-mix(in srgb, var(--theme-primary) 52%, #000);
}

.settings-view :deep(input[type="range"]:hover::-webkit-slider-runnable-track) {
  background:
    linear-gradient(180deg, rgba(0, 0, 0, 0.12), rgba(255, 255, 255, 0.26)),
    color-mix(in srgb, var(--theme-primary) 28%, var(--theme-bg-main));
}

.settings-view :deep(input[type="range"]:hover::-webkit-slider-thumb) {
  background: var(--theme-primary-hover);
  transform: scale(1.08);
}

.settings-view :deep(input[type="range"]:focus-visible) {
  outline: 2px solid color-mix(in srgb, var(--theme-primary) 56%, transparent);
  outline-offset: 4px;
  border-radius: 999px;
}

.settings-view :deep(.relative.inline-block.w-8.h-4.rounded-full),
.settings-view :deep(.w-12.h-6.rounded-full.relative) {
  box-sizing: border-box;
  overflow: visible;
  border: 1px solid color-mix(in srgb, var(--theme-border-strong) 80%, transparent) !important;
  background:
    linear-gradient(180deg, rgba(0, 0, 0, 0.18), rgba(255, 255, 255, 0.12)),
    color-mix(in srgb, var(--theme-bg-main) 74%, var(--theme-text-muted)) !important;
  box-shadow: inset 0 2px 6px rgba(69, 26, 3, 0.2), 0 1px 0 rgba(255, 255, 255, 0.58);
}

.settings-view :deep(.relative.inline-block.w-8.h-4.rounded-full) {
  width: 38px !important;
  height: 22px !important;
}

.settings-view :deep(.w-12.h-6.rounded-full.relative) {
  width: 50px !important;
  height: 26px !important;
}

.settings-view :deep(.relative.inline-block.w-8.h-4.rounded-full.bg-primary),
.settings-view :deep(.w-12.h-6.rounded-full.relative.bg-primary) {
  border-color: color-mix(in srgb, var(--theme-primary) 75%, #000) !important;
  background:
    linear-gradient(180deg, var(--theme-primary), var(--theme-primary-hover)) !important;
  box-shadow: inset 0 1px 2px rgba(255, 255, 255, 0.2), 0 5px 14px color-mix(in srgb, var(--theme-primary) 24%, transparent);
}

.settings-view :deep(.relative.inline-block.w-8.h-4.rounded-full > .absolute.rounded-full),
.settings-view :deep(.w-12.h-6.rounded-full.relative > .absolute.rounded-full) {
  background: #fff !important;
  border: 1px solid color-mix(in srgb, var(--theme-border-strong) 70%, #fff);
  box-shadow: 0 2px 7px rgba(69, 26, 3, 0.24), inset 0 1px 0 rgba(255, 255, 255, 0.92);
}

.settings-view :deep(.relative.inline-block.w-8.h-4.rounded-full > .absolute.rounded-full) {
  top: 2px !important;
  width: 16px !important;
  height: 16px !important;
}

.settings-view :deep(.relative.inline-block.w-8.h-4.rounded-full > .absolute.rounded-full.left-1) {
  left: 3px !important;
  right: auto !important;
}

.settings-view :deep(.relative.inline-block.w-8.h-4.rounded-full > .absolute.rounded-full.right-1) {
  left: auto !important;
  right: 3px !important;
}

.settings-view :deep(.w-12.h-6.rounded-full.relative > .absolute.rounded-full) {
  top: 2px !important;
  width: 20px !important;
  height: 20px !important;
}

.settings-view :deep(.w-12.h-6.rounded-full.relative > .absolute.rounded-full.left-1) {
  left: 3px !important;
}

.settings-view :deep(.w-12.h-6.rounded-full.relative > .absolute.rounded-full.left-7) {
  left: 25px !important;
}

.settings-view :deep(.relative.inline-block.w-8.h-4.rounded-full:not(.bg-primary) > .absolute.rounded-full),
.settings-view :deep(.w-12.h-6.rounded-full.relative:not(.bg-primary) > .absolute.rounded-full) {
  background: color-mix(in srgb, #fff 86%, var(--theme-primary)) !important;
  border-color: color-mix(in srgb, var(--theme-border-strong) 86%, #fff);
}

:deep(.border-white\/5),
:deep(.border-white\/10),
:deep(.border-transparent) {
  border-color: transparent !important;
}

.language-settings {
  max-width: 880px;
  margin: 0 auto;
}

.language-panel {
  padding: 24px;
  border: 1px solid var(--theme-border-strong);
  border-radius: 8px;
  background: color-mix(in srgb, var(--theme-surface) 94%, transparent);
  box-shadow: 0 16px 38px color-mix(in srgb, var(--theme-text-strong) 8%, transparent);
}

.language-panel-copy {
  margin-bottom: 22px;
  display: flex;
  align-items: center;
  gap: 13px;
}

.language-panel-icon {
  width: 46px;
  height: 46px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 auto;
  border: 1px solid color-mix(in srgb, var(--theme-primary) 28%, var(--theme-border-soft));
  border-radius: 8px;
  color: var(--theme-primary);
  background: color-mix(in srgb, var(--theme-primary) 13%, var(--theme-surface));
}

.language-panel h2 {
  margin: 0;
  color: var(--theme-text-strong);
  font-size: 19px;
  line-height: 1.25;
  font-weight: 800;
}

.language-panel p {
  margin: 5px 0 0;
  max-width: 62ch;
  color: var(--theme-text-soft);
  font-size: 13px;
  line-height: 1.6;
  font-weight: 550;
}

.language-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.language-choice {
  min-width: 0;
  min-height: 46px;
  padding: 0 13px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  border: 1px solid var(--theme-border-soft);
  border-radius: 8px;
  color: var(--theme-text-strong);
  background: var(--theme-surface-hover);
  font-size: 13px;
  font-weight: 700;
  text-align: left;
  cursor: pointer;
  transition: transform 160ms ease, border-color 160ms ease, background 160ms ease;
}

.language-choice:hover {
  transform: translateY(-1px);
  border-color: color-mix(in srgb, var(--theme-primary) 55%, var(--theme-border-soft));
}

.language-choice:focus-visible {
  outline: 3px solid color-mix(in srgb, var(--theme-primary) 24%, transparent);
  outline-offset: 2px;
}

@media (max-width: 720px) {
  .language-grid { grid-template-columns: 1fr; }
  .language-panel { padding: 18px; }
}



.fade-enter-active, .fade-leave-active { transition: opacity 0.2s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
