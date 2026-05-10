<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue';
import { Settings, Save, Trash2, Globe, Monitor, Shield, HardDrive, Bell, Gamepad2, Check, DownloadCloud, Play, Rocket, Loader2 } from 'lucide-vue-next';
import { SysApi, DbApi } from '../api';
import { useI18n } from 'vue-i18n';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { check } from '@tauri-apps/plugin-updater';
import { getVersion } from '@tauri-apps/api/app';
import { open } from '@tauri-apps/plugin-dialog';

const { t, locale } = useI18n();

const activeTab = ref('general');
const isSaving = ref(false);
const saved = ref(false);

const appVersion = ref('');
const checkUpdateStatus = ref('');
const isCheckingUpdate = ref(false);

const vrcConfigText = ref('');
const vrcConfigError = ref('');
const vrcConfigSaving = ref(false);

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
  theme: 'light',
  proxyEnabled: false,
  proxyUrl: 'http://127.0.0.1:7890',
  notifyFriendsOnline: true,
  notifyInvite: true,
  notifyStatusChange: false,
  notifyTts: false,
  notifyTtsCondition: 'always',
  notifyDesktopCondition: 'never',
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
  openLocalFilesWithVrcx: false,
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
  vrchatLaunchArgs: ''
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
          } else {
            target[key] = val;
          }
        }
      }
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

    // 设置窗口置顶
    try {
      await getCurrentWindow().setAlwaysOnTop(config.value.topWindow);
    } catch (e) { console.warn("Failed to set top window", e); }

    // 更新多语言引擎并持久化
    if (config.value.language) {
      localStorage.setItem('vrcdog-locale', config.value.language);
      locale.value = config.value.language;
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
    alert(t('settings.steamvr_register_success'));
  } catch (err: any) {
    alert(t('settings.steamvr_register_fail').replace('{error}', err));
  }
};

const openBindings = async () => {
  try {
    await invoke('sys_open_steamvr_bindings');
  } catch (err: any) {
    alert(t('settings.steamvr_bindings_fail').replace('{error}', err));
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

const testNotification = () => {
  notify(t('settings.test_notify_title'), t('settings.test_notify_msg'), 'test');
};
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-slate-50/50 rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-indigo-500/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <header class="mb-8 flex justify-between items-end shrink-0 z-10">
      <div>
        <h1 class="text-3xl font-extrabold text-slate-900 tracking-tight flex items-center gap-3">
          <span class="inline-flex items-center justify-center p-2 bg-indigo-100 rounded-2xl shadow-sm border border-indigo-200/50">
            <Settings class="w-6 h-6 text-indigo-600" />
          </span>
          {{ t('settings.title') }}
        </h1>
        <p class="text-slate-500 font-bold mt-2 text-sm ml-1">
          {{ t('settings.subtitle') }}
        </p>
      </div>
      <button
        class="px-6 py-2.5 rounded-xl font-extrabold shadow-sm transition-all flex items-center gap-2 active:scale-95"
        :class="saved ? 'bg-emerald-500 text-white shadow-emerald-500/30' : 'bg-slate-900 hover:bg-black text-white shadow-slate-900/20'"
        @click="saveSettings"
      >
        <Check
          v-if="saved"
          class="w-4 h-4"
        />
        <Save
          v-else
          class="w-4 h-4"
          :class="{'animate-spin': isSaving}"
        />
        {{ saved ? t('settings.saved') : t('settings.save') }}
      </button>
    </header>

    <div class="flex-1 bg-white/80 backdrop-blur-md border border-slate-200 rounded-3xl shadow-sm flex overflow-hidden z-10">
      <!-- 左侧边栏导航 -->
      <div class="w-56 bg-slate-50/50 border-r border-slate-100 p-4 space-y-1.5 overflow-y-auto custom-scrollbar">
        <button
          :class="activeTab === 'general' ? 'bg-indigo-100 text-indigo-900 font-extrabold shadow-sm' : 'text-slate-600 hover:bg-slate-100/50 hover:text-slate-900'"
          class="w-full text-left px-4 py-3 rounded-xl transition-all flex items-center gap-3 text-sm font-bold"
          @click="activeTab = 'general'"
        >
          <Monitor :size="16" /> {{ t('settings.nav_general') }}
        </button>
        <button
          :class="activeTab === 'notifications' ? 'bg-indigo-100 text-indigo-900 font-extrabold shadow-sm' : 'text-slate-600 hover:bg-slate-100/50 hover:text-slate-900'"
          class="w-full text-left px-4 py-3 rounded-xl transition-all flex items-center gap-3 text-sm font-bold"
          @click="activeTab = 'notifications'"
        >
          <Bell :size="16" /> {{ t('settings.nav_notifications') }}
        </button>
        <button
          :class="activeTab === 'network' ? 'bg-indigo-100 text-indigo-900 font-extrabold shadow-sm' : 'text-slate-600 hover:bg-slate-100/50 hover:text-slate-900'"
          class="w-full text-left px-4 py-3 rounded-xl transition-all flex items-center gap-3 text-sm font-bold"
          @click="activeTab = 'network'"
        >
          <Globe :size="16" /> {{ t('settings.nav_network') }}
        </button>
        <button
          :class="activeTab === 'storage' ? 'bg-indigo-100 text-indigo-900 font-extrabold shadow-sm' : 'text-slate-600 hover:bg-slate-100/50 hover:text-slate-900'"
          class="w-full text-left px-4 py-3 rounded-xl transition-all flex items-center gap-3 text-sm font-bold"
          @click="activeTab = 'storage'"
        >
          <HardDrive :size="16" /> {{ t('settings.nav_storage') }}
        </button>
        <button
          :class="activeTab === 'discord' ? 'bg-indigo-100 text-indigo-900 font-extrabold shadow-sm' : 'text-slate-600 hover:bg-slate-100/50 hover:text-slate-900'"
          class="w-full text-left px-4 py-3 rounded-xl transition-all flex items-center gap-3 text-sm font-bold"
          @click="activeTab = 'discord'"
        >
          <Gamepad2 :size="16" /> {{ t('settings.nav_integration') }}
        </button>
        <button
          :class="activeTab === 'auto_launch' ? 'bg-indigo-100 text-indigo-900 font-extrabold shadow-sm' : 'text-slate-600 hover:bg-slate-100/50 hover:text-slate-900'"
          class="w-full text-left px-4 py-3 rounded-xl transition-all flex items-center gap-3 text-sm font-bold"
          @click="activeTab = 'auto_launch'"
        >
          <Rocket :size="16" /> {{ t('settings.nav_auto_start') }}
        </button>
        <button
          :class="activeTab === 'advanced' ? 'bg-indigo-100 text-indigo-900 font-extrabold shadow-sm' : 'text-slate-600 hover:bg-slate-100/50 hover:text-slate-900'"
          class="w-full text-left px-4 py-3 rounded-xl transition-all flex items-center gap-3 text-sm font-bold"
          @click="activeTab = 'advanced'"
        >
          <Settings :size="16" /> {{ t('settings.nav_advanced') }}
        </button>
        <button
          :class="activeTab === 'security' ? 'bg-indigo-100 text-indigo-900 font-extrabold shadow-sm' : 'text-slate-600 hover:bg-slate-100/50 hover:text-slate-900'"
          class="w-full text-left px-4 py-3 rounded-xl transition-all flex items-center gap-3 text-sm font-bold"
          @click="activeTab = 'security'"
        >
          <Shield :size="16" /> {{ t('settings.nav_security') }}
        </button>
        <button
          :class="activeTab === 'vr' ? 'bg-indigo-100 text-indigo-900 font-extrabold shadow-sm' : 'text-slate-600 hover:bg-slate-100/50 hover:text-slate-900'"
          class="w-full text-left px-4 py-3 rounded-xl transition-all flex items-center gap-3 text-sm font-bold"
          @click="activeTab = 'vr'"
        >
          <Settings :size="16" /> {{ t('settings.nav_vr') }}
        </button>
        <button
          :class="activeTab === 'ovr_ocr' ? 'bg-indigo-100 text-indigo-900 font-extrabold shadow-sm' : 'text-slate-600 hover:bg-slate-100/50 hover:text-slate-900'"
          class="w-full text-left px-4 py-3 rounded-xl transition-all flex items-center gap-3 text-sm font-bold"
          @click="activeTab = 'ovr_ocr'"
        >
          <Settings :size="16" /> {{ t('settings.nav_ocr') }}
        </button>
        <button
          :class="activeTab === 'ovr_trans' ? 'bg-indigo-100 text-indigo-900 font-extrabold shadow-sm' : 'text-slate-600 hover:bg-slate-100/50 hover:text-slate-900'"
          class="w-full text-left px-4 py-3 rounded-xl transition-all flex items-center gap-3 text-sm font-bold"
          @click="activeTab = 'ovr_trans'"
        >
          <Globe :size="16" /> {{ t('settings.nav_translation') }}
        </button>
      </div>

      <!-- 右侧内容区 -->
      <div class="flex-1 p-8 overflow-y-auto custom-scrollbar">
        <Transition
          name="fade"
          mode="out-in"
        >
          <!-- 常规 -->
          <div
            v-if="activeTab === 'general'"
            class="space-y-5"
          >
            <h2 class="text-xl font-extrabold text-slate-900 mb-4 border-b border-slate-200 pb-2">
              {{ t('settings.section_general') }}
            </h2>

            <div class="p-4 bg-gradient-to-r from-slate-50 to-indigo-50/30 rounded-2xl border border-slate-100">
              <div class="flex items-center justify-between mb-4">
                <div>
                  <h3 class="font-bold text-slate-900 flex items-center gap-2">
                    <DownloadCloud class="w-5 h-5 text-indigo-600" />
                    {{ t('settings.software_update') }}
                    <span class="px-2 py-0.5 bg-indigo-200 text-slate-800 text-[10px] rounded-md font-bold font-mono">v{{ appVersion }}</span>
                  </h3>
                  <p class="text-xs text-slate-400 mt-1">
                    {{ t('settings.update_info_desc') }}
                  </p>
                </div>
                <div class="flex flex-col items-end gap-2">
                  <button
                    :disabled="isCheckingUpdate"
                    class="px-4 py-1.5 bg-indigo-500 hover:bg-indigo-600 text-white rounded-xl text-sm font-bold shadow shadow-indigo-500/20 transition-all disabled:opacity-50 flex items-center gap-2"
                    @click="checkForUpdates(false)"
                  >
                    <DownloadCloud
                      v-if="!isCheckingUpdate"
                      class="w-4 h-4"
                    />
                    <span
                      v-else
                      class="w-4 h-4 border border-slate-200/30 border-t-white rounded-full animate-spin"
                    />
                    {{ isCheckingUpdate ? t('settings.checking_update') : t('settings.check_update') }}
                  </button>
                </div>
              </div>
              
              <div class="flex items-center justify-between pt-3 border-t border-slate-100">
                <div class="text-sm font-bold text-slate-800">
                  {{ t('settings.auto_check_update') }}
                </div>
                <label class="relative inline-flex items-center cursor-pointer">
                  <input
                    v-model="config.autoCheckUpdate"
                    type="checkbox"
                    class="sr-only peer"
                  >
                  <div class="w-11 h-6 bg-indigo-200/50 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-indigo-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-indigo-500" />
                </label>
              </div>
              
              <div
                v-if="checkUpdateStatus"
                class="mt-3 text-xs font-bold text-indigo-600 bg-slate-100 px-3 py-2 rounded-lg border border-slate-200"
              >
                {{ checkUpdateStatus }}
              </div>
            </div>

            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-slate-100">
              <div>
                <h3 class="font-bold text-slate-900">
                  {{ t('settings.auto_start') }}
                </h3>
                <p class="text-xs text-slate-400 mt-0.5">
                  {{ t('settings.auto_start_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.autoStart"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-indigo-500" />
              </label>
            </div>
            <div class="p-4 bg-white rounded-2xl border border-slate-100 mt-4">
              <div>
                <h3 class="font-bold text-slate-900">
                  {{ t('settings.vrc_launch_args') }}
                </h3>
                <p class="text-xs text-slate-400 mt-0.5">
                  {{ t('settings.vrc_launch_args_desc_2') }}
                </p>
              </div>
              <input
                v-model="config.vrchatLaunchArgs"
                type="text"
                placeholder="--no-vr --profile=2"
                class="mt-3 w-full px-4 py-2 bg-slate-50 border border-slate-200 rounded-xl text-sm font-bold text-slate-700 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
              >
            </div>

            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-slate-100 mt-4">
              <div>
                <h3 class="font-bold text-slate-900 flex items-center gap-1.5">
                  {{ t('settings.debug_console') }} <span class="bg-blue-100 text-blue-700 px-1.5 py-0.5 rounded text-[10px]">Dev</span>
                </h3>
                <p class="text-xs text-slate-400 mt-0.5">
                  {{ t('settings.debug_console_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.enableDebugConsole"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-indigo-500" />
              </label>
            </div>

            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-slate-100">
              <div>
                <h3 class="font-bold text-slate-900">
                  {{ t('settings.minimize_tray') }}
                </h3>
                <p class="text-xs text-slate-400 mt-0.5">
                  {{ t('settings.minimize_tray_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.minimizeToTray"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-indigo-500" />
              </label>
            </div>

            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-slate-100">
              <div>
                <h3 class="font-bold text-slate-900">
                  {{ t('settings.top_window') }}
                </h3>
                <p class="text-xs text-slate-400 mt-0.5">
                  {{ t('settings.top_window_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.topWindow"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-indigo-500" />
              </label>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-2">
                {{ t('settings.display_language') }}
              </h3>
              <select
                v-model="config.language"
                class="w-full max-w-xs px-4 py-2 rounded-xl border border-slate-200 outline-none focus:border-indigo-400 bg-slate-50/50"
              >
                <option value="zh-CN">
                  {{ t('settings.auto_d688a3a4') }}
                </option>
                <option value="en-US">
                  English
                </option>
                <option value="ja-JP">
                  {{ t('settings.auto_00110af8') }}
                </option>
              </select>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-2">
                {{ t('settings.theme') }}
              </h3>
              <select
                v-model="config.theme"
                class="w-full max-w-xs px-4 py-2 rounded-xl border border-slate-200 outline-none focus:border-indigo-400 bg-slate-50/50"
              >
                <option value="light">
                  {{ t('settings.theme_light') }}
                </option>
                <option value="dark">
                  {{ t('settings.theme_dark') }}
                </option>
                <option value="system">
                  {{ t('settings.theme_system') }}
                </option>
              </select>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-2">
                {{ t('settings.poll_interval') }}
              </h3>
              <p class="text-xs text-slate-400 mb-3">
                {{ t('settings.poll_interval_desc') }}
              </p>
              <input
                v-model="config.pollInterval"
                type="range"
                min="10"
                max="120"
                step="5"
                class="w-full max-w-sm h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-indigo-500"
              >
              <div class="text-slate-900 font-bold mt-2">
                {{ config.pollInterval }} {{ t('settings.seconds') }}
              </div>
            </div>
          </div>

          <!-- 消息通知 -->
          <div
            v-else-if="activeTab === 'notifications'"
            class="space-y-5"
          >
            <h2 class="text-xl font-extrabold text-slate-900 mb-4 border-b border-slate-200 pb-2 flex items-center gap-2">
              <Bell
                class="text-indigo-500"
                :size="20"
              /> {{ t('settings.desktop_notification_title') }}
            </h2>
            
            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-3">
                {{ t('settings.when_to_show_desktop_notification') }}
              </h3>
              <select
                v-model="config.notifyDesktopCondition"
                class="w-full max-w-xs px-4 py-2 rounded-xl border border-slate-200 outline-none focus:border-indigo-400 bg-slate-50/50 text-sm"
              >
                <option value="never">
                  {{ t('settings.never') }}
                </option>
                <option value="desktop">
                  {{ t('settings.in_desktop') }}
                </option>
                <option value="vr">
                  {{ t('settings.in_vr') }}
                </option>
                <option value="not_vr">
                  {{ t('settings.not_in_vr') }}
                </option>
                <option value="vrc_running">
                  {{ t('settings.vrc_running') }}
                </option>
                <option value="vrc_not_running">
                  {{ t('settings.vrc_not_running') }}
                </option>
                <option value="always">
                  {{ t('settings.always') }}
                </option>
              </select>
            </div>

            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-slate-100">
              <div>
                <h3 class="font-bold text-slate-900">
                  {{ t('settings.show_desktop_notify_when_afk') }}
                </h3>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.notifyShowWhenAfk"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:bg-indigo-500 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all" />
              </label>
            </div>

            <h2 class="text-xl font-extrabold text-slate-900 mb-4 border-b border-slate-200 pb-2 mt-8 flex items-center gap-2">
              <Play
                class="text-indigo-500"
                :size="20"
              /> {{ t('settings.tts_options_title') }}
            </h2>

            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-3">
                {{ t('settings.when_to_use_tts') }}
              </h3>
              <select
                v-model="config.notifyTtsCondition"
                class="w-full max-w-xs px-4 py-2 rounded-xl border border-slate-200 outline-none focus:border-indigo-400 bg-slate-50/50 text-sm"
              >
                <option value="never">
                  {{ t('settings.never') }}
                </option>
                <option value="desktop">
                  {{ t('settings.in_desktop') }}
                </option>
                <option value="vr">
                  {{ t('settings.in_vr') }}
                </option>
                <option value="not_vr">
                  {{ t('settings.not_in_vr') }}
                </option>
                <option value="vrc_running">
                  {{ t('settings.vrc_running') }}
                </option>
                <option value="vrc_not_running">
                  {{ t('settings.vrc_not_running') }}
                </option>
                <option value="always">
                  {{ t('settings.always') }}
                </option>
              </select>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-2">
                {{ t('settings.tts_vol') }}
              </h3>
              <div class="flex items-center gap-4">
                <input
                  v-model="config.notifyTtsVolume"
                  type="range"
                  min="0"
                  max="100"
                  class="w-full max-w-sm h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-indigo-500"
                >
                <span class="text-slate-900 font-bold w-8">{{ config.notifyTtsVolume }}%</span>
              </div>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-slate-100 flex gap-4">
              <button
                class="px-4 py-2 bg-indigo-50 text-slate-600 hover:bg-slate-200 rounded-lg font-bold text-sm transition-colors"
                @click="testTTS"
              >
                {{ t('settings.play_test_tts') }}
              </button>
              <button
                class="px-4 py-2 bg-blue-100 text-blue-700 hover:bg-blue-200 rounded-lg font-bold text-sm transition-colors"
                @click="testNotification"
              >
                {{ t('settings.send_test_notify') }}
              </button>
            </div>

            <h2 class="text-xl font-extrabold text-slate-900 mb-4 border-b border-slate-200 pb-2 mt-8 flex items-center gap-2">
              <Gamepad2
                class="text-indigo-500"
                :size="20"
              /> 事件{{ t('settings.state_on') }}{{ t('settings.state_off') }}
            </h2>

            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-slate-100">
              <div class="flex items-center justify-between mb-4">
                <div>
                  <h3 class="font-bold text-slate-900">
                    {{ t('settings.notify_tts') }}
                  </h3>
                  <p class="text-xs text-slate-400 mt-0.5">
                    {{ t('settings.notify_tts_desc') }}
                  </p>
                </div>
                <label class="relative inline-flex items-center cursor-pointer">
                  <input
                    v-model="config.notifyTts"
                    type="checkbox"
                    class="sr-only peer"
                  >
                  <div class="w-11 h-6 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:bg-indigo-500 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all" />
                </label>
              </div>
              <div v-if="config.notifyTts">
                <h3 class="font-bold text-slate-900 mb-2">
                  {{ t('settings.tts_volume') }}
                </h3>
                <input
                  v-model="config.notifyTtsVolume"
                  type="range"
                  min="0"
                  max="100"
                  step="1"
                  class="w-full max-w-sm h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-indigo-500"
                >
                <div class="text-slate-900 font-bold mt-2">
                  {{ config.notifyTtsVolume }}%
                </div>
              </div>
            </div>
          </div>

          <!-- 网络 -->
          <div
            v-else-if="activeTab === 'network'"
            class="space-y-5"
          >
            <h2 class="text-xl font-extrabold text-slate-900 mb-4 border-b border-slate-200 pb-2">
              {{ t('settings.section_network') }}
            </h2>
            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <div class="flex items-center justify-between mb-4">
                <div>
                  <h3 class="font-bold text-slate-900 flex items-center gap-2">
                    {{ t('settings.enable_proxy') }} <span class="px-2 py-0.5 bg-blue-100 text-blue-700 text-[10px] rounded-md font-bold">{{ t('settings.accelerate_api') }}</span>
                  </h3>
                  <p class="text-xs text-slate-400 mt-0.5">
                    {{ t('settings.proxy_desc') }}
                  </p>
                </div>
                <label class="relative inline-flex items-center cursor-pointer">
                  <input
                    v-model="config.proxyEnabled"
                    type="checkbox"
                    class="sr-only peer"
                  >
                  <div class="w-11 h-6 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500" />
                </label>
              </div>
              <div v-if="config.proxyEnabled">
                <label class="block text-xs font-bold text-slate-900 mb-1">{{ t('settings.proxy_url') }}</label>
                <input
                  v-model="config.proxyUrl"
                  type="text"
                  placeholder="http://127.0.0.1:7890"
                  class="w-full max-w-sm px-4 py-2 border border-slate-200 rounded-xl outline-none focus:border-blue-400 font-mono text-sm bg-blue-50/30"
                >
              </div>
            </div>
          </div>

          <!-- 缓存 -->
          <div
            v-else-if="activeTab === 'storage'"
            class="space-y-5"
          >
            <h2 class="text-xl font-extrabold text-slate-900 mb-4 border-b border-slate-200 pb-2">
              {{ t('settings.section_storage') }}
            </h2>
            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-2">
                {{ t('settings.vrc_cache') }}
              </h3>
              <p class="text-xs text-slate-400 mb-4">
                {{ t('settings.vrc_cache_desc') }}
              </p>
              <button
                :disabled="isClearing"
                class="px-5 py-2 bg-red-50 hover:bg-red-500 text-red-600 hover:text-white rounded-xl font-bold transition-colors flex items-center gap-2 text-sm shadow-sm disabled:opacity-50"
                @click="clearCache"
              >
                <Trash2
                  class="w-4 h-4"
                  :class="{'animate-spin': isClearing}"
                /> {{ isClearing ? t('settings.clearing') : t('settings.clear_vrc_cache') }}
              </button>
              <p
                v-if="actionMessage && activeTab === 'storage'"
                class="mt-3 text-xs font-bold text-green-600 bg-green-50 px-3 py-2 rounded border border-green-200"
              >
                {{ actionMessage }}
              </p>
              <p
                v-if="actionError && activeTab === 'storage'"
                class="mt-3 text-xs font-bold text-red-600 bg-red-50 px-3 py-2 rounded border border-red-200"
              >
                {{ actionError }}
              </p>
            </div>
            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-2">
                {{ t('settings.data_limit') }}
              </h3>
              <p class="text-xs text-slate-400 mb-3">
                {{ t('settings.data_limit_desc') }}
              </p>
              <input
                v-model="config.cacheLimit"
                type="range"
                min="1"
                max="20"
                class="w-full max-w-sm h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-indigo-500"
              >
              <div class="text-slate-900 font-bold mt-2">
                {{ config.cacheLimit }} GB
              </div>
            </div>
          </div>

          <!-- 集成 (Discord / APIs) -->
          <div
            v-else-if="activeTab === 'discord'"
            class="space-y-5"
          >
            <h2 class="text-xl font-extrabold text-slate-900 mb-4 border-b border-slate-200 pb-2">
              {{ t('settings.section_integration') }}
            </h2>
            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-4 flex items-center gap-2">
                {{ t('settings.discord_rpc_title') }}
                <span class="px-2 py-0.5 bg-indigo-100 text-indigo-700 text-[10px] rounded-md font-bold">{{ t('settings.discord_rpc_only_vrc') }}</span>
              </h3>
              <p class="text-xs text-slate-400 mb-4">
                建议在 VRChat 的 “config.json” 中停用原生的 {{ t('settings.discord_rpc_title') }}来防止冲突
              </p>
              
              <div class="space-y-3">
                <div class="flex items-center justify-between py-2 border-b border-slate-50">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.enable') }}
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.discordRpcEnabled"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                
                <div
                  v-if="config.discordRpcEnabled"
                  class="flex items-center justify-between py-2 border-b border-slate-50"
                >
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.discord_rpc_world_integration') }}
                    <p class="text-xs text-slate-400 font-normal mt-0.5">
                      {{ t('settings.discord_rpc_world_integration_desc') }}
                    </p>
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.discordRpcEnableWorldIntegration"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                
                <div
                  v-if="config.discordRpcEnabled"
                  class="flex items-center justify-between py-2 border-b border-slate-50"
                >
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.discord_rpc_show_room_type') }}
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.discordRpcShowRoomTypeAndCount"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                
                <div
                  v-if="config.discordRpcEnabled"
                  class="flex items-center justify-between py-2 border-b border-slate-50"
                >
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.discord_rpc_show_platform') }}
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.discordRpcShowPlatform"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                
                <div
                  v-if="config.discordRpcEnabled"
                  class="flex items-center justify-between py-2 border-b border-slate-50"
                >
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.discord_rpc_show_private_info') }}
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.discordRpcShowRoomInfoInPrivate"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                
                <div
                  v-if="config.discordRpcEnabled"
                  class="flex items-center justify-between py-2 border-b border-slate-50"
                >
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.discord_rpc_show_join_btn') }}
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.discordRpcShowJoinButton"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                
                <div
                  v-if="config.discordRpcEnabled"
                  class="flex items-center justify-between py-2 border-b border-slate-50"
                >
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.discord_rpc_show_thumbnail') }}
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.discordRpcShowWorldThumbnail"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                
                <div
                  v-if="config.discordRpcEnabled"
                  class="flex items-center justify-between py-2 border-b border-slate-50"
                >
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.discord_rpc_show_world_name') }}
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.discordRpcShowWorldName"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                <div
                  v-if="config.discordRpcEnabled"
                  class="space-y-3"
                >
                  <div>
                    <label class="block text-xs font-bold text-slate-900 mb-1">{{ t('settings.discord_details') }}</label>
                    <input
                      v-model="config.discordRpcDetails"
                      type="text"
                      class="w-full px-4 py-2 border border-slate-200 rounded-xl outline-none focus:border-indigo-400 text-sm bg-indigo-50/30"
                    >
                  </div>
                  <div>
                    <label class="block text-xs font-bold text-slate-900 mb-1">{{ t('settings.discord_state') }}</label>
                    <input
                      v-model="config.discordRpcState"
                      type="text"
                      class="w-full px-4 py-2 border border-slate-200 rounded-xl outline-none focus:border-indigo-400 text-sm bg-indigo-50/30"
                    >
                  </div>
                </div>
              </div>
            </div>

            <!-- Translation API -->
            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-4">
                {{ t('settings.translation_api_title') }}
              </h3>
              <div class="space-y-3">
                <div class="flex items-center justify-between py-2 border-b border-slate-50">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_7854b52a') }}
                    <p class="text-xs text-slate-400 font-normal mt-0.5">
                      {{ t('settings.auto_02716bdb') }}
                    </p>
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.translationApiEnabled"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                <div
                  v-if="config.translationApiEnabled"
                  class="flex items-center justify-between py-2 border-b border-slate-50"
                >
                  <div class="text-sm font-bold text-slate-800 w-1/3">
                    输入 {{ t('settings.api_key') }}
                  </div>
                  <input
                    v-model="config.translationApiKey"
                    type="password"
                    :placeholder="t('settings.auto_31cba103')"
                    class="w-2/3 px-3 py-1.5 border border-slate-200 rounded-lg outline-none focus:border-indigo-400 text-sm bg-indigo-50/30"
                  >
                </div>
              </div>
            </div>

            <!-- YouTube API -->
            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-4">
                YouTube API
              </h3>
              <div class="space-y-3">
                <div class="flex items-center justify-between py-2 border-b border-slate-50">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_7854b52a') }}
                    <p class="text-xs text-slate-400 font-normal mt-0.5">
                      {{ t('settings.auto_8e8e5c39') }}
                    </p>
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.youtubeApiEnabled"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                <div
                  v-if="config.youtubeApiEnabled"
                  class="flex items-center justify-between py-2 border-b border-slate-50"
                >
                  <div class="text-sm font-bold text-slate-800 w-1/3">
                    输入 {{ t('settings.api_key') }}
                  </div>
                  <input
                    v-model="config.youtubeApiKey"
                    type="password"
                    :placeholder="t('settings.auto_31cba103')"
                    class="w-2/3 px-3 py-1.5 border border-slate-200 rounded-lg outline-none focus:border-indigo-400 text-sm bg-indigo-50/30"
                  >
                </div>
              </div>
            </div>

            <!-- Remote Avatar Database -->
            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-4">
                {{ t('settings.auto_f94d2445') }}
              </h3>
              <div class="space-y-3">
                <div class="flex items-center justify-between py-2 border-b border-slate-50">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_7854b52a') }}
                    <p class="text-xs text-slate-400 font-normal mt-0.5">
                      {{ t('settings.auto_488d74ab') }}
                    </p>
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.remoteAvatarDbEnabled"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                <div
                  v-if="config.remoteAvatarDbEnabled"
                  class="flex justify-end pt-2"
                >
                  <button class="px-4 py-2 border border-slate-200 text-slate-800 rounded-lg text-xs font-bold hover:bg-slate-50">
                    {{ t('settings.auto_1326f274') }}
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- 第三方程序自动化 (Auto Launch) -->
          <div
            v-else-if="activeTab === 'auto_launch'"
            class="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300"
          >
            <div class="bg-indigo-50/50 rounded-2xl p-6 border border-indigo-100/50 relative overflow-hidden">
              <div class="absolute -right-10 -bottom-10 opacity-5 pointer-events-none">
                <Rocket :size="150" />
              </div>
              <h3 class="text-xl font-bold text-slate-900 mb-2 flex items-center gap-2 relative z-10">
                <Rocket class="text-indigo-600" />
                {{ t('settings.auto_launch_title') }}
              </h3>
              <p class="text-sm text-slate-600 mb-6 relative z-10 font-medium">
                {{ t('settings.auto_launch_desc') }}
              </p>

              <div class="space-y-5 relative z-10">
                <label class="flex items-center gap-3 cursor-pointer group bg-white/60 p-3 rounded-xl border border-slate-200 hover:border-indigo-300 transition-colors">
                  <div class="relative">
                    <input
                      v-model="config.killAppsOnExit"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-11 h-6 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-indigo-600 shadow-inner" />
                  </div>
                  <span class="text-slate-700 font-bold group-hover:text-slate-900">
                    {{ t('settings.kill_apps_on_exit') }}
                  </span>
                </label>

                <label class="flex items-center gap-3 cursor-pointer group bg-white/60 p-3 rounded-xl border border-slate-200 hover:border-indigo-300 transition-colors">
                  <div class="relative">
                    <input
                      v-model="config.clearCacheOnExit"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-11 h-6 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-indigo-600 shadow-inner" />
                  </div>
                  <span class="text-slate-700 font-bold group-hover:text-slate-900">
                    {{ t('settings.auto_705b353f') }}
                  </span>
                </label>

                <div>
                  <label class="block text-sm font-bold text-slate-700 mb-2">
                    {{ t('settings.auto_launch_list') }}
                  </label>
                  <textarea
                    v-model="parsedAutoLaunchApps"
                    rows="6"
                    class="w-full bg-white/80 border border-slate-200 rounded-xl px-4 py-3 text-sm focus:ring-2 focus:ring-indigo-500/50 focus:border-indigo-500 outline-none transition-all resize-y font-mono shadow-inner text-slate-600"
                    placeholder="C:\Program Files\Example\app.exe&#10;D:\Tools\OSC\tracker.exe"
                  />
                </div>
              </div>
            </div>
          </div>

          <div
            v-else-if="activeTab === 'advanced'"
            class="space-y-5"
          >
            <h2 class="text-xl font-extrabold text-slate-900 mb-4 border-b border-slate-200 pb-2">
              {{ t('settings.auto_dfac151d') }}
            </h2>
            
            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-4">
                {{ t('settings.auto_fc40d46d') }}
              </h3>
              <div class="space-y-4">
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_62e1577d') }}
                  </div>
                  <input
                    v-model.number="config.webApiTimeout"
                    type="number"
                    min="1"
                    max="60"
                    class="w-24 px-3 py-1.5 border border-slate-200 rounded-lg outline-none focus:border-indigo-400 text-sm text-center"
                  >
                </div>
                
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_2e79854b') }}
                  </div>
                  <input
                    v-model.number="config.requestLimit"
                    type="number"
                    min="1"
                    max="50"
                    class="w-24 px-3 py-1.5 border border-slate-200 rounded-lg outline-none focus:border-indigo-400 text-sm text-center"
                  >
                </div>

                <div class="flex items-center justify-between py-2 border-t border-slate-50">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_1a025b2f') }}
                    <p class="text-xs text-red-500 font-normal mt-0.5">
                      更改后必须重启生效。如果您遇到显示问题，请尝试关闭此选项。
                    </p>
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.hardwareAcceleration"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>

                <div class="flex items-center justify-between py-2 border-t border-slate-50">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_2d5e67c5') }}
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.customUrlScheme"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>

                <div class="flex items-center justify-between py-2 border-t border-slate-50">
                  <div class="text-sm font-bold text-slate-800">
                    使用本应用打{{ t('settings.state_on') }}本地文件
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.openLocalFilesWithVrcx"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>

                <div class="flex items-center justify-between py-2 border-t border-slate-50">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_0a6453d2') }}
                    <p class="text-xs text-slate-400 font-normal mt-0.5">
                      {{ t('settings.auto_bd677786') }}
                    </p>
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.oscAutomation"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
              </div>

              <!-- VRChat Config.json Editor -->
              <div class="p-4 bg-white rounded-2xl border border-slate-100">
                <div class="flex items-center justify-between mb-4">
                  <div>
                    <h3 class="font-bold text-slate-900">
                      {{ t('settings.auto_a257183e') }}
                    </h3>
                    <p class="text-xs text-slate-400 mt-0.5">
                      {{ t('settings.auto_a64a7d1d') }}
                    </p>
                  </div>
                  <button 
                    :disabled="vrcConfigSaving" 
                    class="px-4 py-2 bg-indigo-50 hover:bg-indigo-100 text-indigo-600 font-bold text-sm rounded-lg transition-colors flex items-center gap-2"
                    @click="saveVrcConfig"
                  >
                    <Save
                      v-if="!vrcConfigSaving"
                      :size="16"
                    />
                    <Loader2
                      v-else
                      :size="16"
                      class="animate-spin"
                    />
                    {{ vrcConfigSaving ? '保存中...' : '保存配置' }}
                  </button>
                </div>
                  
                <div class="flex items-center gap-3 mb-4">
                  <button
                    class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 text-slate-700 font-bold text-xs rounded-lg transition-colors border border-slate-200 flex items-center gap-2"
                    @click="pickFolderForConfig('cache_directory')"
                  >
                    <HardDrive :size="14" /> {{ t('settings.auto_2baedf4e') }}
                  </button>
                  <button
                    class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 text-slate-700 font-bold text-xs rounded-lg transition-colors border border-slate-200 flex items-center gap-2"
                    @click="pickFolderForConfig('camera_res_dir')"
                  >
                    <Image :size="14" /> {{ t('settings.auto_12a95080') }}
                  </button>
                </div>

                <div
                  v-if="vrcConfigError"
                  class="mb-3 text-xs font-bold px-3 py-2 rounded-lg"
                  :class="vrcConfigError.includes('成功') ? 'bg-emerald-100 text-emerald-600' : 'bg-red-100 text-red-600'"
                >
                  {{ vrcConfigError }}
                </div>
                
                <textarea
                  v-model="vrcConfigText"
                  spellcheck="false"
                  class="w-full h-64 p-4 bg-slate-900 text-emerald-400 font-mono text-sm rounded-xl border border-slate-800 outline-none focus:ring-2 focus:ring-indigo-500 custom-scrollbar resize-y"
                  placeholder="{}"
                />
              </div>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-red-50 mt-4">
              <div class="flex justify-between items-center">
                <div class="text-sm font-bold text-red-900">
                  {{ t('settings.auto_dd318f41') }}
                </div>
                <button class="px-4 py-2 bg-red-100 text-red-700 hover:bg-red-500 hover:text-white rounded-lg text-xs font-bold transition-colors">
                  {{ t('settings.auto_ae30fb5a') }}
                </button>
              </div>
            </div>
          </div>

          <!-- 隐私安全 -->
          <div
            v-else-if="activeTab === 'security'"
            class="space-y-5"
          >
            <h2 class="text-xl font-extrabold text-slate-900 mb-4 border-b border-slate-200 pb-2">
              {{ t('settings.section_security') }}
            </h2>
            <div class="p-4 bg-white rounded-2xl border border-red-100 bg-red-50/30">
              <h3 class="font-bold text-red-900 mb-2 flex items-center gap-2">
                <Shield class="w-4 h-4" /> {{ t('settings.auth_security') }}
              </h3>
              <p class="text-xs text-red-700/80 mb-4 leading-relaxed">
                {{ t('settings.auth_security_desc') }}
              </p>
              <button
                :disabled="isClearingAuth"
                class="px-5 py-2 bg-red-500 hover:bg-red-600 text-white rounded-xl font-bold transition-colors flex items-center gap-2 text-sm shadow-md disabled:opacity-50"
                @click="clearAuth"
              >
                <Trash2 class="w-4 h-4" /> {{ isClearingAuth ? t('settings.clearing_auth') : t('settings.force_logout') }}
              </button>
              <p
                v-if="actionMessage && activeTab === 'security'"
                class="mt-3 text-xs font-bold text-green-600 bg-green-50 px-3 py-2 rounded border border-green-200"
              >
                {{ actionMessage }}
              </p>
            </div>
          </div>

          <!-- VR -->
          <div
            v-else-if="activeTab === 'vr'"
            class="space-y-5"
          >
            <h2 class="text-xl font-extrabold text-slate-900 mb-4 border-b border-slate-200 pb-2">
              {{ t('settings.section_vr') }}
            </h2>
            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-slate-100">
              <div>
                <h3 class="font-bold text-slate-900">
                  {{ t('settings.vr_overlay_enable') }}
                </h3>
                <p class="text-xs text-slate-400 mt-0.5">
                  {{ t('settings.vr_overlay_enable_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.vrOverlayEnabled"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:bg-indigo-500 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all" />
              </label>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-2">
                {{ t('settings.vr_overlay_opacity') }}
              </h3>
              <input
                v-model="config.vrOverlayOpacity"
                type="range"
                min="10"
                max="100"
                step="5"
                class="w-full max-w-sm h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-indigo-500"
              >
              <div class="text-slate-900 font-bold mt-2">
                {{ config.vrOverlayOpacity }}%
              </div>
            </div>

            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-slate-100">
              <div>
                <h3 class="font-bold text-slate-900">
                  {{ t('settings.vr_hand_tracking') }}
                </h3>
                <p class="text-xs text-slate-400 mt-0.5">
                  {{ t('settings.vr_hand_tracking_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.wristMode"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:bg-indigo-500 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all" />
              </label>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-4">
                {{ t('settings.auto_61453b9e') }}
              </h3>
              <div class="space-y-4">
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_c1b1a54b') }}
                  </div>
                  <input
                    v-model.number="config.transPanelMaxWidth"
                    type="range"
                    min="300"
                    max="1200"
                    step="10"
                    class="w-1/2 h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-indigo-500"
                  >
                  <span class="text-xs font-mono font-bold w-12 text-right">{{ config.transPanelMaxWidth }}px</span>
                </div>
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_21da7a06') }}
                  </div>
                  <input
                    v-model.number="config.overlayFontSize"
                    type="range"
                    min="12"
                    max="72"
                    step="1"
                    class="w-1/2 h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-indigo-500"
                  >
                  <span class="text-xs font-mono font-bold w-12 text-right">{{ config.overlayFontSize }}pt</span>
                </div>
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_f6db9e99') }}
                  </div>
                  <input
                    v-model="config.statusColor"
                    type="color"
                    class="w-10 h-10 p-0 border-0 rounded cursor-pointer"
                  >
                </div>
              </div>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-4">
                {{ t('settings.auto_4d7096cf') }}
              </h3>
              <div class="space-y-4">
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_800eddf3') }}
                  </div>
                  <input
                    v-model.number="config.gripPressureThreshold"
                    type="range"
                    min="0.1"
                    max="1.0"
                    step="0.05"
                    class="w-1/2 h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-indigo-500"
                  >
                  <span class="text-xs font-mono font-bold w-12 text-right">{{ config.gripPressureThreshold }}</span>
                </div>
              </div>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-4">
                {{ t('settings.auto_e96cab7c') }}
              </h3>
              <div class="space-y-4">
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_47099174') }}
                  </div>
                  <input
                    v-model.number="config.ocrContrast"
                    type="range"
                    min="0.5"
                    max="2.0"
                    step="0.1"
                    class="w-1/2 h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-indigo-500"
                  >
                  <span class="text-xs font-mono font-bold w-8 text-right">{{ config.ocrContrast }}</span>
                </div>
                
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_a9476266') }}
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.ocrSharpen"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:bg-indigo-500 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all" />
                  </label>
                </div>

                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_db1dcc8d') }}
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input
                      v-model="config.ocrDenoise"
                      type="checkbox"
                      class="sr-only peer"
                    >
                    <div class="w-9 h-5 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:bg-indigo-500 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all" />
                  </label>
                </div>

                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_4a57a937') }}
                  </div>
                  <input
                    v-model.number="config.ocrMergeToleranceX"
                    type="range"
                    min="0.0"
                    max="1.0"
                    step="0.05"
                    class="w-1/2 h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-indigo-500"
                  >
                  <span class="text-xs font-mono font-bold w-8 text-right">{{ config.ocrMergeToleranceX }}</span>
                </div>

                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-slate-800">
                    {{ t('settings.auto_1c993b5a') }}
                  </div>
                  <input
                    v-model.number="config.ocrMergeToleranceY"
                    type="range"
                    min="0.0"
                    max="1.0"
                    step="0.05"
                    class="w-1/2 h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-indigo-500"
                  >
                  <span class="text-xs font-mono font-bold w-8 text-right">{{ config.ocrMergeToleranceY }}</span>
                </div>
              </div>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-blue-50">
              <h3 class="font-bold text-blue-900 mb-2 flex items-center gap-2">
                {{ t('settings.auto_d0cd9337') }}
              </h3>
              <p class="text-xs text-blue-700/80 mb-4 leading-relaxed">
                {{ t('settings.auto_533646d8') }}
              </p>
              <div class="flex flex-wrap gap-3">
                <button
                  class="px-4 py-2 bg-blue-100 hover:bg-blue-500 hover:text-white text-blue-700 rounded-xl text-sm font-bold transition-colors"
                  @click="registerSteamVR"
                >
                  {{ t('settings.auto_8d481df8') }}
                </button>
                <button
                  class="px-4 py-2 bg-indigo-100 hover:bg-indigo-500 hover:text-white text-indigo-700 rounded-xl text-sm font-bold transition-colors"
                  @click="openBindings"
                >
                  打开按键绑定面板 (浏览器)
                </button>
              </div>
            </div>
          </div>

          <!-- OVR OCR -->
          <div
            v-else-if="activeTab === 'ovr_ocr'"
            class="space-y-5"
          >
            <h2 class="text-xl font-extrabold text-slate-900 mb-4 border-b border-slate-200 pb-2">
              {{ t('settings.auto_2a5ef7a4') }}
            </h2>
            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-2">
                {{ t('settings.auto_0ae96994') }}
              </h3>
              <p class="text-xs text-slate-400 mb-4">
                {{ t('settings.auto_e9c37226') }}
              </p>
              <select
                v-model="config.ocrLanguage"
                class="w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
              >
                <option value="ja">
                  {{ t('settings.auto_ee36e359') }}
                </option>
                <option value="en-US">
                  {{ t('settings.auto_24d56afe') }}
                </option>
                <option value="zh-Hans-CN">
                  {{ t('settings.auto_48aad8ae') }}
                </option>
                <option value="zh-Hant-TW">
                  {{ t('settings.auto_dd16f5f4') }}
                </option>
                <option value="ko">
                  {{ t('settings.auto_80f8fa5a') }}
                </option>
              </select>
            </div>
            
            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-2">
                {{ t('settings.auto_7b6bd792') }}
              </h3>
              <select
                v-model="config.ocrSpeedMode"
                class="w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
              >
                <option value="fast">
                  {{ t('settings.auto_b085bc76') }}
                </option>
                <option value="balanced">
                  {{ t('settings.auto_9f909f44') }}
                </option>
                <option value="accurate">
                  {{ t('settings.auto_3252bb5f') }}
                </option>
              </select>
            </div>
            <div class="p-4 bg-white rounded-2xl border border-blue-50">
              <h3 class="font-bold text-blue-900 mb-2 flex items-center gap-2">
                {{ t('settings.auto_5d750047') }}
              </h3>
              <p class="text-xs text-blue-700/80 mb-4 leading-relaxed">
                {{ t('settings.auto_5e315524') }}
              </p>
            </div>
          </div>

          <!-- OVR Translation -->
          <div
            v-else-if="activeTab === 'ovr_trans'"
            class="space-y-5"
          >
            <h2 class="text-xl font-extrabold text-slate-900 mb-4 border-b border-slate-200 pb-2">
              {{ t('settings.auto_fb7ea758') }}
            </h2>
            
            <div class="p-4 bg-white rounded-2xl border border-slate-100">
              <h3 class="font-bold text-slate-900 mb-4">
                {{ t('settings.auto_f82d8f40') }}
              </h3>
              <select
                v-model="config.transService"
                class="w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500 mb-4"
              >
                <option value="tencent">
                  {{ t('settings.auto_0c26793d') }}
                </option>
                <option value="baidu">
                  {{ t('settings.auto_07edd78b') }}
                </option>
                <option value="microsoft">
                  {{ t('settings.auto_3aa75e4a') }}
                </option>
                <option value="google">
                  Google Translate
                </option>
                <option value="deepl">
                  DeepL
                </option>
                <option value="openai">
                  OpenAI (LLM)
                </option>
                <option value="deepseek">
                  DeepSeek (LLM)
                </option>
                <option value="ollama">
                  {{ t('settings.auto_cb420daa') }}
                </option>
              </select>
              
              <div
                v-if="['openai', 'deepseek', 'ollama'].includes(config.transService)"
                class="space-y-4"
              >
                <div>
                  <label class="block text-xs font-bold text-slate-700 mb-1">API Key</label>
                  <input
                    v-model="config.transApiKey"
                    type="password"
                    placeholder="sk-..."
                    class="w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                  >
                </div>
                <div>
                  <label class="block text-xs font-bold text-slate-700 mb-1">{{ t('settings.auto_f291aabf') }}</label>
                  <input
                    v-model="config.transLlmModel"
                    type="text"
                    :placeholder="t('settings.auto_d205ac69')"
                    class="w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                  >
                </div>
                <div>
                  <label class="block text-xs font-bold text-slate-700 mb-1">{{ t('settings.auto_be42d249') }}</label>
                  <input
                    v-model="config.customApiUrl"
                    type="text"
                    placeholder="http://127.0.0.1:11434/v1/chat/completions"
                    class="w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                  >
                </div>
                <div>
                  <label class="block text-xs font-bold text-slate-700 mb-1">{{ t('settings.auto_14735200') }}</label>
                  <textarea
                    v-model="config.transLlmPrompt"
                    rows="3"
                    class="w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                  />
                </div>
              </div>
              
              <div
                v-else
                class="space-y-4"
              >
                <div v-if="['tencent', 'baidu'].includes(config.transService)">
                  <label class="block text-xs font-bold text-slate-700 mb-1">{{ t('settings.auto_f70d6b64') }}</label>
                  <input
                    v-model="config.transAppId"
                    type="text"
                    class="w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                  >
                </div>
                <div>
                  <label class="block text-xs font-bold text-slate-700 mb-1">API Key / Secret</label>
                  <input
                    v-model="config.transApiKey"
                    type="password"
                    class="w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                  >
                </div>
              </div>
            </div>
            
            <div class="grid grid-cols-2 gap-4">
              <div class="p-4 bg-white rounded-2xl border border-slate-100">
                <h3 class="font-bold text-slate-900 mb-2">
                  {{ t('settings.auto_04d605c2') }}
                </h3>
                <select
                  v-model="config.transSourceLang"
                  class="w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                >
                  <option value="auto">
                    {{ t('settings.auto_ac653a57') }}
                  </option>
                  <option value="en">
                    英语
                  </option>
                  <option value="ja">
                    日语
                  </option>
                  <option value="ko">
                    韩语
                  </option>
                  <option value="zh">
                    中文
                  </option>
                </select>
              </div>
              <div class="p-4 bg-white rounded-2xl border border-slate-100">
                <h3 class="font-bold text-slate-900 mb-2">
                  {{ t('settings.auto_129d0594') }}
                </h3>
                <select
                  v-model="config.transTargetLang"
                  class="w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
                >
                  <option value="zh">
                    {{ t('settings.auto_d688a3a4') }}
                  </option>
                  <option value="zh-TW">
                    {{ t('settings.auto_dd16f5f4') }}
                  </option>
                  <option value="en">
                    英语
                  </option>
                </select>
              </div>
            </div>
          </div>
        </Transition>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(245, 158, 11, 0.2); border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: rgba(245, 158, 11, 0.4); }
.fade-enter-active, .fade-leave-active { transition: opacity 0.2s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
