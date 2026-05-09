<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { Settings, Save, Trash2, Globe, Monitor, Shield, HardDrive, Bell, Gamepad2, Check, DownloadCloud } from 'lucide-vue-next';
import { SysApi, DbApi } from '../api';
import { useI18n } from 'vue-i18n';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { check } from '@tauri-apps/plugin-updater';
import { getVersion } from '@tauri-apps/api/app';

const { t, locale } = useI18n();

const activeTab = ref('general');
const isSaving = ref(false);
const saved = ref(false);

const appVersion = ref('');
const checkUpdateStatus = ref('');
const isCheckingUpdate = ref(false);

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
  notifyTtsVolume: 50,
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
  openLocalFilesWithVrcx: false,
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
  transLlmPrompt: '你是一个翻译专家，综合所有的 OCR 乱入文本，给出现在最好的目标语言。',
  customApiUrl: '',
  ocrLanguage: 'ja',
  ocrSpeedMode: 'balanced',
  statusColor: '#FFFFFF',
  overlayFontSize: 24,
  transPanelMaxWidth: 800,
  gripPressureThreshold: 0.5,
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
    console.warn('加载设置失败:', err);
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

    // 如果开启了 Discord RPC，立即更新
    if (config.value.discordRpcEnabled) {
      try {
        await SysApi.setDiscordRpc({
          details: config.value.discordRpcDetails,
          state: config.value.discordRpcState,
        });
      } catch { /* ignore */ }
    }

    // 触发全局设置更新事件
    window.dispatchEvent(new CustomEvent('settings-updated', { detail: config.value }));

    saved.value = true;
    setTimeout(() => { saved.value = false; }, 2000);
  } catch (err) {
    console.warn('保存设置失败:', err);
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
    alert('成功向 SteamVR 注册自动启动!');
  } catch (err: any) {
    alert('注册 SteamVR 自动启动失败: ' + err);
  }
};

const openBindings = async () => {
  try {
    await invoke('sys_open_steamvr_bindings');
  } catch (err: any) {
    alert('打开 SteamVR 键位面板失败: ' + err);
  }
};

const checkForUpdates = async (silent = false) => {
  if (isCheckingUpdate.value) return;
  isCheckingUpdate.value = true;
  checkUpdateStatus.value = silent ? '' : '检查更新中...';
  try {
    const update = await check();
    if (update) {
      if (confirm(`发现新版本 v${update.version}！\n\n${update.body}\n\n是否立即下载并更新？`)) {
        checkUpdateStatus.value = '正在下载更新...';
        await update.downloadAndInstall();
        await invoke('process::restart');
      } else {
        checkUpdateStatus.value = '已取消更新';
      }
    } else {
      checkUpdateStatus.value = '当前已是最新版本';
      if (!silent) setTimeout(() => { checkUpdateStatus.value = ''; }, 3000);
    }
  } catch (err) {
    console.error('Update check failed:', err);
    checkUpdateStatus.value = '检查更新失败: ' + String(err);
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
</script>

<template>
  <div class="h-full flex flex-col">
    <header class="mb-6 flex justify-between items-end">
      <div>
        <h1 class="text-3xl font-extrabold text-[#451a03] tracking-tight flex items-center gap-3">
          {{ t('settings.title') }}
          <span class="inline-flex items-center justify-center p-1.5 bg-gray-200 rounded-xl">
            <Settings class="w-6 h-6 text-gray-700" />
          </span>
        </h1>
        <p class="text-amber-700/80 font-medium mt-1">
          {{ t('settings.subtitle') }}
        </p>
      </div>
      <button
        class="px-6 py-2 rounded-full font-bold shadow-md transition-all flex items-center gap-2"
        :class="saved ? 'bg-green-500 text-white shadow-green-500/30' : 'bg-amber-500 hover:bg-amber-600 text-white shadow-amber-500/30'"
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

    <div class="flex-1 bg-white/60 backdrop-blur-md border-2 border-white rounded-3xl shadow-lg flex overflow-hidden">
      <!-- 左侧边栏导航 -->
      <div class="w-48 bg-white/40 border-r border-amber-50 p-4 space-y-2">
        <button
          :class="activeTab === 'general' ? 'bg-amber-100 text-amber-900 font-bold' : 'text-amber-700 hover:bg-white'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeTab = 'general'"
        >
          <Monitor :size="16" /> {{ t('settings.nav_general') }}
        </button>
        <button
          :class="activeTab === 'notifications' ? 'bg-amber-100 text-amber-900 font-bold' : 'text-amber-700 hover:bg-white'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeTab = 'notifications'"
        >
          <Bell :size="16" /> {{ t('settings.nav_notifications') }}
        </button>
        <button
          :class="activeTab === 'network' ? 'bg-amber-100 text-amber-900 font-bold' : 'text-amber-700 hover:bg-white'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeTab = 'network'"
        >
          <Globe :size="16" /> {{ t('settings.nav_network') }}
        </button>
        <button
          :class="activeTab === 'storage' ? 'bg-amber-100 text-amber-900 font-bold' : 'text-amber-700 hover:bg-white'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeTab = 'storage'"
        >
          <HardDrive :size="16" /> {{ t('settings.nav_storage') }}
        </button>
        <button
          :class="activeTab === 'discord' ? 'bg-amber-100 text-amber-900 font-bold' : 'text-amber-700 hover:bg-white'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeTab = 'discord'"
        >
          <Gamepad2 :size="16" /> {{ t('settings.nav_integration') || '集成' }}
        </button>
        <button
          :class="activeTab === 'advanced' ? 'bg-amber-100 text-amber-900 font-bold' : 'text-amber-700 hover:bg-white'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeTab = 'advanced'"
        >
          <Settings :size="16" /> {{ t('settings.nav_advanced') || '高级' }}
        </button>
        <button
          :class="activeTab === 'security' ? 'bg-amber-100 text-amber-900 font-bold' : 'text-amber-700 hover:bg-white'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeTab = 'security'"
        >
          <Shield :size="16" /> {{ t('settings.nav_security') }}
        </button>
        <button
          :class="activeTab === 'vr' ? 'bg-amber-100 text-amber-900 font-bold' : 'text-amber-700 hover:bg-white'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeTab = 'vr'"
        >
          <Settings :size="16" /> {{ t('settings.nav_vr') || 'VR 叠加层' }}
        </button>
        <button
          :class="activeTab === 'ovr_ocr' ? 'bg-amber-100 text-amber-900 font-bold' : 'text-amber-700 hover:bg-white'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeTab = 'ovr_ocr'"
        >
          <Settings :size="16" /> OCR 图像识别
        </button>
        <button
          :class="activeTab === 'ovr_trans' ? 'bg-amber-100 text-amber-900 font-bold' : 'text-amber-700 hover:bg-white'"
          class="w-full text-left px-4 py-3 rounded-xl transition-colors flex items-center gap-3 text-sm"
          @click="activeTab = 'ovr_trans'"
        >
          <Globe :size="16" /> 翻译服务引擎
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
            <h2 class="text-xl font-extrabold text-amber-950 mb-4 border-b border-amber-100 pb-2">
              {{ t('settings.section_general') }}
            </h2>

            <div class="p-4 bg-gradient-to-r from-amber-50 to-orange-50 rounded-2xl border border-amber-100/50">
              <div class="flex items-center justify-between mb-4">
                <div>
                  <h3 class="font-bold text-amber-900 flex items-center gap-2">
                    <DownloadCloud class="w-5 h-5 text-amber-600" />
                    软件更新
                    <span class="px-2 py-0.5 bg-amber-200 text-amber-800 text-[10px] rounded-md font-bold font-mono">v{{ appVersion }}</span>
                  </h3>
                  <p class="text-xs text-amber-700/60 mt-1">
                    保持您的 VrcDog 客户端处于最新状态，获取最新的功能体验与性能修复
                  </p>
                </div>
                <div class="flex flex-col items-end gap-2">
                  <button
                    @click="checkForUpdates(false)"
                    :disabled="isCheckingUpdate"
                    class="px-4 py-1.5 bg-amber-500 hover:bg-amber-600 text-white rounded-xl text-sm font-bold shadow shadow-amber-500/20 transition-all disabled:opacity-50 flex items-center gap-2"
                  >
                    <DownloadCloud v-if="!isCheckingUpdate" class="w-4 h-4" />
                    <span v-else class="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
                    {{ isCheckingUpdate ? '正在检查...' : '检查更新' }}
                  </button>
                </div>
              </div>
              
              <div class="flex items-center justify-between pt-3 border-t border-amber-200/30">
                <div class="text-sm font-bold text-amber-800">自动检查更新</div>
                <label class="relative inline-flex items-center cursor-pointer">
                  <input
                    v-model="config.autoCheckUpdate"
                    type="checkbox"
                    class="sr-only peer"
                  >
                  <div class="w-11 h-6 bg-amber-200/50 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-amber-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-amber-500" />
                </label>
              </div>
              
              <div v-if="checkUpdateStatus" class="mt-3 text-xs font-bold text-amber-600 bg-amber-100/50 px-3 py-2 rounded-lg border border-amber-200/50">
                {{ checkUpdateStatus }}
              </div>
            </div>

            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-amber-50">
              <div>
                <h3 class="font-bold text-amber-900">
                  {{ t('settings.auto_start') }}
                </h3>
                <p class="text-xs text-amber-700/60 mt-0.5">
                  {{ t('settings.auto_start_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.autoStart"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-amber-500" />
              </label>
            </div>

            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-amber-50">
              <div>
                <h3 class="font-bold text-amber-900 flex items-center gap-1.5">
                  {{ t('settings.debug_console') }} <span class="bg-blue-100 text-blue-700 px-1.5 py-0.5 rounded text-[10px]">Dev</span>
                </h3>
                <p class="text-xs text-amber-700/60 mt-0.5">
                  {{ t('settings.debug_console_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.enableDebugConsole"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-amber-500" />
              </label>
            </div>

            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-amber-50">
              <div>
                <h3 class="font-bold text-amber-900">
                  {{ t('settings.minimize_tray') }}
                </h3>
                <p class="text-xs text-amber-700/60 mt-0.5">
                  {{ t('settings.minimize_tray_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.minimizeToTray"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-amber-500" />
              </label>
            </div>

            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-amber-50">
              <div>
                <h3 class="font-bold text-amber-900">
                  {{ t('settings.top_window') || '窗口置顶' }}
                </h3>
                <p class="text-xs text-amber-700/60 mt-0.5">
                  {{ t('settings.top_window_desc') || '保持应用程序窗口总在最前面' }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.topWindow"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-amber-500" />
              </label>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-2">
                {{ t('settings.display_language') }}
              </h3>
              <select
                v-model="config.language"
                class="w-full max-w-xs px-4 py-2 rounded-xl border border-amber-100 outline-none focus:border-amber-400 bg-amber-50/50"
              >
                <option value="zh-CN">
                  简体中文
                </option>
                <option value="en-US">
                  English
                </option>
                <option value="ja-JP">
                  日本語
                </option>
              </select>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-2">
                {{ t('settings.theme') || '主题 (Theme)' }}
              </h3>
              <select
                v-model="config.theme"
                class="w-full max-w-xs px-4 py-2 rounded-xl border border-amber-100 outline-none focus:border-amber-400 bg-amber-50/50"
              >
                <option value="light">{{ t('settings.theme_light') || '明亮 (Light)' }}</option>
                <option value="dark">{{ t('settings.theme_dark') || '暗黑 (Dark)' }}</option>
                <option value="system">{{ t('settings.theme_system') || '跟随系统' }}</option>
              </select>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-2">
                {{ t('settings.poll_interval') }}
              </h3>
              <p class="text-xs text-amber-700/60 mb-3">
                {{ t('settings.poll_interval_desc') }}
              </p>
              <input
                v-model="config.pollInterval"
                type="range"
                min="10"
                max="120"
                step="5"
                class="w-full max-w-sm h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-amber-500"
              >
              <div class="text-amber-900 font-bold mt-2">
                {{ config.pollInterval }} {{ t('settings.seconds') }}
              </div>
            </div>
          </div>

          <!-- 消息通知 -->
          <div
            v-else-if="activeTab === 'notifications'"
            class="space-y-5"
          >
            <h2 class="text-xl font-extrabold text-amber-950 mb-4 border-b border-amber-100 pb-2">
              {{ t('settings.section_notifications') }}
            </h2>
            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-amber-50">
              <div>
                <h3 class="font-bold text-amber-900">
                  {{ t('settings.notify_online') }}
                </h3>
                <p class="text-xs text-amber-700/60 mt-0.5">
                  {{ t('settings.notify_online_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.notifyFriendsOnline"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:bg-amber-500 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all" />
              </label>
            </div>
            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-amber-50">
              <div>
                <h3 class="font-bold text-amber-900">
                  {{ t('settings.notify_invite') }}
                </h3>
                <p class="text-xs text-amber-700/60 mt-0.5">
                  {{ t('settings.notify_invite_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.notifyInvite"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:bg-amber-500 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all" />
              </label>
            </div>
            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-amber-50">
              <div>
                <h3 class="font-bold text-amber-900">
                  {{ t('settings.notify_status') }}
                </h3>
                <p class="text-xs text-amber-700/60 mt-0.5">
                  {{ t('settings.notify_status_desc') }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.notifyStatusChange"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:bg-amber-500 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all" />
              </label>
            </div>
            
            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <div class="flex items-center justify-between mb-4">
                <div>
                  <h3 class="font-bold text-amber-900">
                    {{ t('settings.notify_tts') || '开启 TTS 语音播报' }}
                  </h3>
                  <p class="text-xs text-amber-700/60 mt-0.5">
                    {{ t('settings.notify_tts_desc') || '当收到重要通知时，自动通过系统语音合成播报' }}
                  </p>
                </div>
                <label class="relative inline-flex items-center cursor-pointer">
                  <input
                    v-model="config.notifyTts"
                    type="checkbox"
                    class="sr-only peer"
                  >
                  <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:bg-amber-500 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all" />
                </label>
              </div>
              <div v-if="config.notifyTts">
                <h3 class="font-bold text-amber-900 mb-2">
                  {{ t('settings.tts_volume') || 'TTS 语音播报音量' }}
                </h3>
                <input
                  v-model="config.notifyTtsVolume"
                  type="range"
                  min="0"
                  max="100"
                  step="1"
                  class="w-full max-w-sm h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-amber-500"
                >
                <div class="text-amber-900 font-bold mt-2">
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
            <h2 class="text-xl font-extrabold text-amber-950 mb-4 border-b border-amber-100 pb-2">
              {{ t('settings.section_network') }}
            </h2>
            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <div class="flex items-center justify-between mb-4">
                <div>
                  <h3 class="font-bold text-amber-900 flex items-center gap-2">
                    {{ t('settings.enable_proxy') }} <span class="px-2 py-0.5 bg-blue-100 text-blue-700 text-[10px] rounded-md font-bold">{{ t('settings.accelerate_api') }}</span>
                  </h3>
                  <p class="text-xs text-amber-700/60 mt-0.5">
                    {{ t('settings.proxy_desc') }}
                  </p>
                </div>
                <label class="relative inline-flex items-center cursor-pointer">
                  <input
                    v-model="config.proxyEnabled"
                    type="checkbox"
                    class="sr-only peer"
                  >
                  <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500" />
                </label>
              </div>
              <div v-if="config.proxyEnabled">
                <label class="block text-xs font-bold text-amber-900 mb-1">{{ t('settings.proxy_url') }}</label>
                <input
                  v-model="config.proxyUrl"
                  type="text"
                  placeholder="http://127.0.0.1:7890"
                  class="w-full max-w-sm px-4 py-2 border border-amber-200 rounded-xl outline-none focus:border-blue-400 font-mono text-sm bg-blue-50/30"
                >
              </div>
            </div>
          </div>

          <!-- 缓存 -->
          <div
            v-else-if="activeTab === 'storage'"
            class="space-y-5"
          >
            <h2 class="text-xl font-extrabold text-amber-950 mb-4 border-b border-amber-100 pb-2">
              {{ t('settings.section_storage') }}
            </h2>
            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-2">
                {{ t('settings.vrc_cache') }}
              </h3>
              <p class="text-xs text-amber-700/60 mb-4">
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
            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-2">
                {{ t('settings.data_limit') }}
              </h3>
              <p class="text-xs text-amber-700/60 mb-3">
                {{ t('settings.data_limit_desc') }}
              </p>
              <input
                v-model="config.cacheLimit"
                type="range"
                min="1"
                max="20"
                class="w-full max-w-sm h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-amber-500"
              >
              <div class="text-amber-900 font-bold mt-2">
                {{ config.cacheLimit }} GB
              </div>
            </div>
          </div>

          <!-- 集成 (Discord / APIs) -->
          <div
            v-else-if="activeTab === 'discord'"
            class="space-y-5"
          >
            <h2 class="text-xl font-extrabold text-amber-950 mb-4 border-b border-amber-100 pb-2">
              {{ t('settings.section_integration') || '集成' }}
            </h2>
            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-4 flex items-center gap-2">
                Discord 状态面板
                <span class="px-2 py-0.5 bg-indigo-100 text-indigo-700 text-[10px] rounded-md font-bold">仅在 VRChat 运行时生效</span>
              </h3>
              <p class="text-xs text-amber-700/60 mb-4">建议在 VRChat 的 “config.json” 中停用原生的 Discord 状态面板来防止冲突</p>
              
              <div class="space-y-3">
                <div class="flex items-center justify-between py-2 border-b border-gray-50">
                  <div class="text-sm font-bold text-gray-800">启用</div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input v-model="config.discordRpcEnabled" type="checkbox" class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                
                <div class="flex items-center justify-between py-2 border-b border-gray-50" v-if="config.discordRpcEnabled">
                  <div class="text-sm font-bold text-gray-800">
                    打开与特定世界的集成
                    <p class="text-xs text-gray-400 font-normal mt-0.5">为 Popcorn Palace、PyPyDance、VRDancing 和 LS Media 显示“正在观看/正在收听”的状态</p>
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input v-model="config.discordRpcEnableWorldIntegration" type="checkbox" class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                
                <div class="flex items-center justify-between py-2 border-b border-gray-50" v-if="config.discordRpcEnabled">
                  <div class="text-sm font-bold text-gray-800">显示房间类型和人数</div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input v-model="config.discordRpcShowRoomTypeAndCount" type="checkbox" class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                
                <div class="flex items-center justify-between py-2 border-b border-gray-50" v-if="config.discordRpcEnabled">
                  <div class="text-sm font-bold text-gray-800">显示当前所在的平台</div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input v-model="config.discordRpcShowPlatform" type="checkbox" class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                
                <div class="flex items-center justify-between py-2 border-b border-gray-50" v-if="config.discordRpcEnabled">
                  <div class="text-sm font-bold text-gray-800">在私人房间时显示房间信息</div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input v-model="config.discordRpcShowRoomInfoInPrivate" type="checkbox" class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                
                <div class="flex items-center justify-between py-2 border-b border-gray-50" v-if="config.discordRpcEnabled">
                  <div class="text-sm font-bold text-gray-800">在面板上显示加入按钮 (仅限公开房间)</div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input v-model="config.discordRpcShowJoinButton" type="checkbox" class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                
                <div class="flex items-center justify-between py-2 border-b border-gray-50" v-if="config.discordRpcEnabled">
                  <div class="text-sm font-bold text-gray-800">显示世界缩略图</div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input v-model="config.discordRpcShowWorldThumbnail" type="checkbox" class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                
                <div class="flex items-center justify-between py-2 border-b border-gray-50" v-if="config.discordRpcEnabled">
                  <div class="text-sm font-bold text-gray-800">在 Discord 状态中显示世界名称</div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input v-model="config.discordRpcShowWorldName" type="checkbox" class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
              <div
                v-if="config.discordRpcEnabled"
                class="space-y-3"
              >
                <div>
                  <label class="block text-xs font-bold text-amber-900 mb-1">{{ t('settings.discord_details') }}</label>
                  <input
                    v-model="config.discordRpcDetails"
                    type="text"
                    class="w-full px-4 py-2 border border-amber-200 rounded-xl outline-none focus:border-indigo-400 text-sm bg-indigo-50/30"
                  >
                </div>
                <div>
                  <label class="block text-xs font-bold text-amber-900 mb-1">{{ t('settings.discord_state') }}</label>
                  <input
                    v-model="config.discordRpcState"
                    type="text"
                    class="w-full px-4 py-2 border border-amber-200 rounded-xl outline-none focus:border-indigo-400 text-sm bg-indigo-50/30"
                  >
                </div>
              </div>
            </div>
            </div>

            <!-- Translation API -->
            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-4">翻译 API</h3>
              <div class="space-y-3">
                <div class="flex items-center justify-between py-2 border-b border-gray-50">
                  <div class="text-sm font-bold text-gray-800">
                    启用
                    <p class="text-xs text-gray-400 font-normal mt-0.5">用于翻译玩家的个人简介</p>
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input v-model="config.translationApiEnabled" type="checkbox" class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                <div class="flex items-center justify-between py-2 border-b border-gray-50" v-if="config.translationApiEnabled">
                  <div class="text-sm font-bold text-gray-800 w-1/3">输入 API 密钥</div>
                  <input v-model="config.translationApiKey" type="password" placeholder="输入 API 密钥" class="w-2/3 px-3 py-1.5 border border-amber-200 rounded-lg outline-none focus:border-indigo-400 text-sm bg-indigo-50/30">
                </div>
              </div>
            </div>

            <!-- YouTube API -->
            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-4">YouTube API</h3>
              <div class="space-y-3">
                <div class="flex items-center justify-between py-2 border-b border-gray-50">
                  <div class="text-sm font-bold text-gray-800">
                    启用
                    <p class="text-xs text-gray-400 font-normal mt-0.5">获取视频标题以供日志记录使用，会一并获取进度条的时间</p>
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input v-model="config.youtubeApiEnabled" type="checkbox" class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                <div class="flex items-center justify-between py-2 border-b border-gray-50" v-if="config.youtubeApiEnabled">
                  <div class="text-sm font-bold text-gray-800 w-1/3">输入 API 密钥</div>
                  <input v-model="config.youtubeApiKey" type="password" placeholder="输入 API 密钥" class="w-2/3 px-3 py-1.5 border border-amber-200 rounded-lg outline-none focus:border-indigo-400 text-sm bg-indigo-50/30">
                </div>
              </div>
            </div>

            <!-- Remote Avatar Database -->
            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-4">远程模型数据库</h3>
              <div class="space-y-3">
                <div class="flex items-center justify-between py-2 border-b border-gray-50">
                  <div class="text-sm font-bold text-gray-800">
                    启用
                    <p class="text-xs text-gray-400 font-normal mt-0.5">启用从远程数据库获取模型数据的功能</p>
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input v-model="config.remoteAvatarDbEnabled" type="checkbox" class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-500" />
                  </label>
                </div>
                <div class="flex justify-end pt-2" v-if="config.remoteAvatarDbEnabled">
                  <button class="px-4 py-2 border border-amber-200 text-amber-800 rounded-lg text-xs font-bold hover:bg-amber-50">
                    数据库提供方设置
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- 高级 (Advanced) -->
          <div
            v-else-if="activeTab === 'advanced'"
            class="space-y-5"
          >
            <h2 class="text-xl font-extrabold text-amber-950 mb-4 border-b border-amber-100 pb-2">
              高级
            </h2>
            
            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-4">外部应用程序</h3>
              <div class="space-y-4">
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-gray-800">Web API 超时 (秒)</div>
                  <input v-model.number="config.webApiTimeout" type="number" min="1" max="60" class="w-24 px-3 py-1.5 border border-amber-200 rounded-lg outline-none focus:border-amber-400 text-sm text-center">
                </div>
                
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-gray-800">并发请求数限制</div>
                  <input v-model.number="config.requestLimit" type="number" min="1" max="50" class="w-24 px-3 py-1.5 border border-amber-200 rounded-lg outline-none focus:border-amber-400 text-sm text-center">
                </div>

                <div class="flex items-center justify-between py-2 border-t border-gray-50">
                  <div class="text-sm font-bold text-gray-800">
                    硬件加速
                    <p class="text-xs text-red-500 font-normal mt-0.5">更改后必须重启生效。如果您遇到显示问题，请尝试关闭此选项。</p>
                  </div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input v-model="config.hardwareAcceleration" type="checkbox" class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-amber-500" />
                  </label>
                </div>

                <div class="flex items-center justify-between py-2 border-t border-gray-50">
                  <div class="text-sm font-bold text-gray-800">自定义 URL Scheme (vrcx://)</div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input v-model="config.customUrlScheme" type="checkbox" class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-amber-500" />
                  </label>
                </div>

                <div class="flex items-center justify-between py-2 border-t border-gray-50">
                  <div class="text-sm font-bold text-gray-800">使用本应用打开本地文件</div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input v-model="config.openLocalFilesWithVrcx" type="checkbox" class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-amber-500" />
                  </label>
                </div>
              </div>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-red-50 mt-4">
               <div class="flex justify-between items-center">
                 <div class="text-sm font-bold text-red-900">重置所有设置</div>
                 <button class="px-4 py-2 bg-red-100 text-red-700 hover:bg-red-500 hover:text-white rounded-lg text-xs font-bold transition-colors">
                   恢复默认值
                 </button>
               </div>
            </div>
          </div>

          <!-- 隐私安全 -->
          <div
            v-else-if="activeTab === 'security'"
            class="space-y-5"
          >
            <h2 class="text-xl font-extrabold text-amber-950 mb-4 border-b border-amber-100 pb-2">
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
            <h2 class="text-xl font-extrabold text-amber-950 mb-4 border-b border-amber-100 pb-2">
              {{ t('settings.section_vr') || 'VR 头显与叠加层' }}
            </h2>
            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-amber-50">
              <div>
                <h3 class="font-bold text-amber-900">
                  {{ t('settings.vr_overlay_enable') || '开启 OVR 叠加层 (Overlay)' }}
                </h3>
                <p class="text-xs text-amber-700/60 mt-0.5">
                  {{ t('settings.vr_overlay_enable_desc') || '在 SteamVR 中显示控制面板' }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.vrOverlayEnabled"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:bg-amber-500 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all" />
              </label>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-2">
                {{ t('settings.vr_overlay_opacity') || '叠加层透明度 (Alpha)' }}
              </h3>
              <input
                v-model="config.vrOverlayOpacity"
                type="range"
                min="10"
                max="100"
                step="5"
                class="w-full max-w-sm h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-amber-500"
              >
              <div class="text-amber-900 font-bold mt-2">
                {{ config.vrOverlayOpacity }}%
              </div>
            </div>

            <div class="flex items-center justify-between p-4 bg-white rounded-2xl border border-amber-50">
              <div>
                <h3 class="font-bold text-amber-900">
                  {{ t('settings.vr_hand_tracking') || '腕部手表模式' }}
                </h3>
                <p class="text-xs text-amber-700/60 mt-0.5">
                  {{ t('settings.vr_hand_tracking_desc') || '将叠加层缩小并吸附在手腕位置，方便随时查看通知' }}
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="config.wristMode"
                  type="checkbox"
                  class="sr-only peer"
                >
                <div class="w-11 h-6 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:bg-amber-500 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all" />
              </label>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-4">文本与样式定制</h3>
              <div class="space-y-4">
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-gray-800">悬浮窗最大宽度 (像素)</div>
                  <input v-model.number="config.transPanelMaxWidth" type="range" min="300" max="1200" step="10" class="w-1/2 h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-amber-500">
                  <span class="text-xs font-mono font-bold w-12 text-right">{{ config.transPanelMaxWidth }}px</span>
                </div>
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-gray-800">文本字体大小</div>
                  <input v-model.number="config.overlayFontSize" type="range" min="12" max="72" step="1" class="w-1/2 h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-amber-500">
                  <span class="text-xs font-mono font-bold w-12 text-right">{{ config.overlayFontSize }}pt</span>
                </div>
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-gray-800">状态指示文本颜色</div>
                  <input v-model="config.statusColor" type="color" class="w-10 h-10 p-0 border-0 rounded cursor-pointer">
                </div>
              </div>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-4">手柄交互阈值设置</h3>
              <div class="space-y-4">
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-gray-800">Index/Touch Grip 抓取力度</div>
                  <input v-model.number="config.gripPressureThreshold" type="range" min="0.1" max="1.0" step="0.05" class="w-1/2 h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-amber-500">
                  <span class="text-xs font-mono font-bold w-12 text-right">{{ config.gripPressureThreshold }}</span>
                </div>
              </div>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-4">OCR 图像增强选项</h3>
              <div class="space-y-4">
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-gray-800">图像对比度</div>
                  <input v-model.number="config.ocrContrast" type="range" min="0.5" max="2.0" step="0.1" class="w-1/2 h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-amber-500">
                  <span class="text-xs font-mono font-bold w-8 text-right">{{ config.ocrContrast }}</span>
                </div>
                
                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-gray-800">图像锐化</div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input v-model="config.ocrSharpen" type="checkbox" class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:bg-amber-500 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all" />
                  </label>
                </div>

                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-gray-800">图像降噪</div>
                  <label class="relative inline-flex items-center cursor-pointer">
                    <input v-model="config.ocrDenoise" type="checkbox" class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-200 peer-focus:outline-none rounded-full peer peer-checked:bg-amber-500 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all" />
                  </label>
                </div>

                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-gray-800">水平排版容差</div>
                  <input v-model.number="config.ocrMergeToleranceX" type="range" min="0.0" max="1.0" step="0.05" class="w-1/2 h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-amber-500">
                  <span class="text-xs font-mono font-bold w-8 text-right">{{ config.ocrMergeToleranceX }}</span>
                </div>

                <div class="flex items-center justify-between">
                  <div class="text-sm font-bold text-gray-800">垂直排版容差</div>
                  <input v-model.number="config.ocrMergeToleranceY" type="range" min="0.0" max="1.0" step="0.05" class="w-1/2 h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-amber-500">
                  <span class="text-xs font-mono font-bold w-8 text-right">{{ config.ocrMergeToleranceY }}</span>
                </div>
              </div>
            </div>

            <div class="p-4 bg-white rounded-2xl border border-blue-50">
              <h3 class="font-bold text-blue-900 mb-2 flex items-center gap-2">SteamVR 增强</h3>
              <p class="text-xs text-blue-700/80 mb-4 leading-relaxed">提供 SteamVR 随启动注册以及键位映射排障功能</p>
              <div class="flex flex-wrap gap-3">
                <button @click="registerSteamVR" class="px-4 py-2 bg-blue-100 hover:bg-blue-500 hover:text-white text-blue-700 rounded-xl text-sm font-bold transition-colors">
                  向 SteamVR 注册自动启动
                </button>
                <button @click="openBindings" class="px-4 py-2 bg-indigo-100 hover:bg-indigo-500 hover:text-white text-indigo-700 rounded-xl text-sm font-bold transition-colors">
                  打开按键绑定面板 (浏览器)
                </button>
              </div>
            </div>
          </div>

          <!-- OVR OCR -->
          <div v-else-if="activeTab === 'ovr_ocr'" class="space-y-5">
            <h2 class="text-xl font-extrabold text-amber-950 mb-4 border-b border-amber-100 pb-2">
              OCR 模型与处理选项
            </h2>
            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-2">识别模型引擎</h3>
              <p class="text-xs text-amber-700/60 mb-4">选择适配当前游戏界面的文字引擎</p>
              <select v-model="config.ocrLanguage" class="w-full bg-gray-50 border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-amber-500">
                <option value="ja">中英日模型 (默认主流)</option>
                <option value="en-US">纯英文模型</option>
                <option value="zh-Hans-CN">纯简体中文</option>
                <option value="zh-Hant-TW">繁体中文</option>
                <option value="ko">韩文模型</option>
              </select>
            </div>
            
            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-2">识别速度策略</h3>
              <select v-model="config.ocrSpeedMode" class="w-full bg-gray-50 border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-amber-500">
                <option value="fast">快速 (牺牲部分精度)</option>
                <option value="balanced">均衡 (默认)</option>
                <option value="accurate">精准 (识别更慢)</option>
              </select>
            </div>
            <div class="p-4 bg-white rounded-2xl border border-blue-50">
              <h3 class="font-bold text-blue-900 mb-2 flex items-center gap-2">请注意</h3>
              <p class="text-xs text-blue-700/80 mb-4 leading-relaxed">图像增强选项(对比度、排版容差等)请前往【VR 叠加层】选项卡底部进行微调。</p>
            </div>
          </div>

          <!-- OVR Translation -->
          <div v-else-if="activeTab === 'ovr_trans'" class="space-y-5">
            <h2 class="text-xl font-extrabold text-amber-950 mb-4 border-b border-amber-100 pb-2">
              翻译引擎配置
            </h2>
            
            <div class="p-4 bg-white rounded-2xl border border-amber-50">
              <h3 class="font-bold text-amber-900 mb-4">服务商选择</h3>
              <select v-model="config.transService" class="w-full bg-gray-50 border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-amber-500 mb-4">
                <option value="tencent">腾讯翻译君</option>
                <option value="baidu">百度翻译</option>
                <option value="microsoft">微软翻译</option>
                <option value="google">Google Translate</option>
                <option value="deepl">DeepL</option>
                <option value="openai">OpenAI (LLM)</option>
                <option value="deepseek">DeepSeek (LLM)</option>
                <option value="ollama">Ollama / 自定义 (LLM)</option>
              </select>
              
              <div v-if="['openai', 'deepseek', 'ollama'].includes(config.transService)" class="space-y-4">
                <div>
                  <label class="block text-xs font-bold text-gray-700 mb-1">API Key</label>
                  <input v-model="config.transApiKey" type="password" placeholder="sk-..." class="w-full bg-gray-50 border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-amber-500">
                </div>
                <div>
                  <label class="block text-xs font-bold text-gray-700 mb-1">LLM 模型名称</label>
                  <input v-model="config.transLlmModel" type="text" placeholder="例如: gpt-4o 或 deepseek-chat" class="w-full bg-gray-50 border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-amber-500">
                </div>
                <div>
                  <label class="block text-xs font-bold text-gray-700 mb-1">自定义接口 URL (可选)</label>
                  <input v-model="config.customApiUrl" type="text" placeholder="http://127.0.0.1:11434/v1/chat/completions" class="w-full bg-gray-50 border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-amber-500">
                </div>
                <div>
                  <label class="block text-xs font-bold text-gray-700 mb-1">系统 Prompt 提示词</label>
                  <textarea v-model="config.transLlmPrompt" rows="3" class="w-full bg-gray-50 border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"></textarea>
                </div>
              </div>
              
              <div v-else class="space-y-4">
                <div v-if="['tencent', 'baidu'].includes(config.transService)">
                  <label class="block text-xs font-bold text-gray-700 mb-1">App ID / 用户名</label>
                  <input v-model="config.transAppId" type="text" class="w-full bg-gray-50 border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-amber-500">
                </div>
                <div>
                  <label class="block text-xs font-bold text-gray-700 mb-1">API Key / Secret</label>
                  <input v-model="config.transApiKey" type="password" class="w-full bg-gray-50 border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-amber-500">
                </div>
              </div>
            </div>
            
            <div class="grid grid-cols-2 gap-4">
              <div class="p-4 bg-white rounded-2xl border border-amber-50">
                <h3 class="font-bold text-amber-900 mb-2">源语言</h3>
                <select v-model="config.transSourceLang" class="w-full bg-gray-50 border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-amber-500">
                  <option value="auto">自动检测</option>
                  <option value="en">英语</option>
                  <option value="ja">日语</option>
                  <option value="ko">韩语</option>
                  <option value="zh">中文</option>
                </select>
              </div>
              <div class="p-4 bg-white rounded-2xl border border-amber-50">
                <h3 class="font-bold text-amber-900 mb-2">目标语言</h3>
                <select v-model="config.transTargetLang" class="w-full bg-gray-50 border border-gray-200 rounded-xl px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-amber-500">
                  <option value="zh">简体中文</option>
                  <option value="zh-TW">繁体中文</option>
                  <option value="en">英语</option>
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
