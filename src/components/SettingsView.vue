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
  <div class="h-full flex flex-col bg-[#111214] relative overflow-hidden text-slate-200">
    <!-- Top Tabs (VRCX style) -->
    <div class="flex items-center gap-6 px-6 bg-[#1e1f22] border-b border-white/5 shrink-0 h-14 overflow-x-auto custom-scrollbar">
      <button :class="activeTab === 'general' ? 'border-indigo-500 text-white font-bold' : 'border-transparent text-slate-400 hover:text-slate-200'" class="h-full px-2 border-b-2 transition-colors whitespace-nowrap text-[13px]" @click="activeTab = 'general'">系统</button>
      <button :class="activeTab === 'interface' ? 'border-indigo-500 text-white font-bold' : 'border-transparent text-slate-400 hover:text-slate-200'" class="h-full px-2 border-b-2 transition-colors whitespace-nowrap text-[13px]" @click="activeTab = 'interface'">界面</button>
      <button :class="activeTab === 'notifications' ? 'border-indigo-500 text-white font-bold' : 'border-transparent text-slate-400 hover:text-slate-200'" class="h-full px-2 border-b-2 transition-colors whitespace-nowrap text-[13px]" @click="activeTab = 'notifications'">通知</button>
      <button :class="activeTab === 'network' ? 'border-indigo-500 text-white font-bold' : 'border-transparent text-slate-400 hover:text-slate-200'" class="h-full px-2 border-b-2 transition-colors whitespace-nowrap text-[13px]" @click="activeTab = 'network'">网络</button>
      <button :class="activeTab === 'storage' ? 'border-indigo-500 text-white font-bold' : 'border-transparent text-slate-400 hover:text-slate-200'" class="h-full px-2 border-b-2 transition-colors whitespace-nowrap text-[13px]" @click="activeTab = 'storage'">存储</button>
      <button :class="activeTab === 'discord' ? 'border-indigo-500 text-white font-bold' : 'border-transparent text-slate-400 hover:text-slate-200'" class="h-full px-2 border-b-2 transition-colors whitespace-nowrap text-[13px]" @click="activeTab = 'discord'">集成</button>
      <button :class="activeTab === 'auto_launch' ? 'border-indigo-500 text-white font-bold' : 'border-transparent text-slate-400 hover:text-slate-200'" class="h-full px-2 border-b-2 transition-colors whitespace-nowrap text-[13px]" @click="activeTab = 'auto_launch'">启动器</button>
      <button :class="activeTab === 'advanced' ? 'border-indigo-500 text-white font-bold' : 'border-transparent text-slate-400 hover:text-slate-200'" class="h-full px-2 border-b-2 transition-colors whitespace-nowrap text-[13px]" @click="activeTab = 'advanced'">高级</button>
      <button :class="activeTab === 'security' ? 'border-indigo-500 text-white font-bold' : 'border-transparent text-slate-400 hover:text-slate-200'" class="h-full px-2 border-b-2 transition-colors whitespace-nowrap text-[13px]" @click="activeTab = 'security'">安全</button>
      <button :class="activeTab === 'vr' ? 'border-indigo-500 text-white font-bold' : 'border-transparent text-slate-400 hover:text-slate-200'" class="h-full px-2 border-b-2 transition-colors whitespace-nowrap text-[13px]" @click="activeTab = 'vr'">VR</button>
      <button :class="activeTab === 'ovr_ocr' ? 'border-indigo-500 text-white font-bold' : 'border-transparent text-slate-400 hover:text-slate-200'" class="h-full px-2 border-b-2 transition-colors whitespace-nowrap text-[13px]" @click="activeTab = 'ovr_ocr'">OCR翻译</button>
      <button :class="activeTab === 'ovr_trans' ? 'border-indigo-500 text-white font-bold' : 'border-transparent text-slate-400 hover:text-slate-200'" class="h-full px-2 border-b-2 transition-colors whitespace-nowrap text-[13px]" @click="activeTab = 'ovr_trans'">大语言模型</button>
      
      <div class="ml-auto flex items-center gap-2">
         <button
          class="px-4 py-1.5 rounded bg-[#2b2d31] hover:bg-[#35373c] text-[13px] font-bold transition-all flex items-center gap-2 border border-transparent hover:border-white/5"
          @click="saveSettings"
         >
          <Check v-if="saved" class="w-4 h-4 text-green-500" />
          <Save v-else class="w-4 h-4 text-slate-400" :class="{'animate-spin': isSaving}" />
          {{ saved ? '已保存' : '保存设置' }}
         </button>
      </div>
    </div>

    <div class="flex-1 p-6 overflow-y-auto custom-scrollbar">
      <Transition
        name="fade"
        mode="out-in"
      >
        <!-- 界面 (Interface) -->
        <div v-if="activeTab === 'interface'" class="max-w-4xl mx-auto space-y-8 pb-10 mt-4">
          <!-- 外观 -->
          <div>
            <h2 class="text-[15px] font-bold text-white mb-4">外观</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors">
                <div class="text-[13px] text-slate-300">语言</div>
                <select v-model="config.language" class="bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] text-slate-200 outline-none w-48 focus:border-indigo-500">
                  <option value="zh-CN">简体中文 (zh-CN)</option>
                  <option value="en-US">English</option>
                </select>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors">
                <div class="text-[13px] text-slate-300">字体</div>
                <select class="bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] text-slate-200 outline-none w-48 focus:border-indigo-500">
                  <option>Inter / Noto Sans CJK</option>
                </select>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors">
                <div class="text-[13px] text-slate-300">界面缩放</div>
                <div class="flex items-center gap-3">
                  <button class="w-7 h-7 rounded bg-[#2b2d31] hover:bg-[#35373c] border border-[#3f4147] flex items-center justify-center transition-colors">-</button>
                  <span class="text-[13px] w-8 text-center font-bold">100%</span>
                  <button class="w-7 h-7 rounded bg-[#2b2d31] hover:bg-[#35373c] border border-[#3f4147] flex items-center justify-center transition-colors">+</button>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer">
                <div class="text-[13px] text-slate-300">显示托盘通知</div>
                <div class="relative inline-block w-8 h-4 rounded-full bg-indigo-500 transition-colors">
                  <div class="absolute right-1 top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-transform"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer">
                <div>
                  <div class="text-[13px] text-slate-300">VRC+ 个人资料图标</div>
                  <div class="text-[11px] text-slate-500 mt-0.5">当可用时显示 VRChat+ 专属个人资料图标</div>
                </div>
                <div class="relative inline-block w-8 h-4 rounded-full bg-indigo-500 transition-colors">
                  <div class="absolute right-1 top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-transform"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- 显示设置 -->
          <div>
            <h2 class="text-[15px] font-bold text-white mb-4">显示设置</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer">
                <div class="text-[13px] text-slate-300">显示房间 ID</div>
                <div class="relative inline-block w-8 h-4 rounded-full bg-[#3f4147] transition-colors">
                  <div class="absolute left-1 top-0.5 w-3 h-3 rounded-full bg-slate-400 shadow-sm transition-transform"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer">
                <div class="text-[13px] text-slate-300">显示本地好友备注</div>
                <div class="relative inline-block w-8 h-4 rounded-full bg-indigo-500 transition-colors">
                  <div class="absolute right-1 top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-transform"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer">
                <div class="text-[13px] text-slate-300">显示具有年龄限制的房间</div>
                <div class="relative inline-block w-8 h-4 rounded-full bg-indigo-500 transition-colors">
                  <div class="absolute right-1 top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-transform"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer">
                <div class="text-[13px] text-slate-300">斑马纹表格模式</div>
                <div class="relative inline-block w-8 h-4 rounded-full bg-[#3f4147] transition-colors">
                  <div class="absolute left-1 top-0.5 w-3 h-3 rounded-full bg-slate-400 shadow-sm transition-transform"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer">
                <div class="text-[13px] text-slate-300">无障碍版状态指示器</div>
                <div class="relative inline-block w-8 h-4 rounded-full bg-[#3f4147] transition-colors">
                  <div class="absolute left-1 top-0.5 w-3 h-3 rounded-full bg-slate-400 shadow-sm transition-transform"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer">
                <div class="text-[13px] text-slate-300">使用游戏内的状态颜色</div>
                <div class="relative inline-block w-8 h-4 rounded-full bg-indigo-500 transition-colors">
                  <div class="absolute right-1 top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-transform"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- 导航 -->
          <div>
            <h2 class="text-[15px] font-bold text-white mb-4">导航</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer">
                <div class="text-[13px] text-slate-300">显示 "新建仪表板" 按钮</div>
                <div class="relative inline-block w-8 h-4 rounded-full bg-indigo-500 transition-colors">
                  <div class="absolute right-1 top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-transform"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- 列表与表格 -->
          <div>
            <h2 class="text-[15px] font-bold text-white mb-4">列表与表格</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors">
                <div class="text-[13px] text-slate-300">收藏夹排序规则</div>
                <div class="flex bg-[#1e1f22] border border-[#3f4147] rounded p-1 gap-1">
                   <button class="px-3 py-1 text-[12px] bg-[#3f4147] text-white rounded shadow-sm transition-colors">名称</button>
                   <button class="px-3 py-1 text-[12px] text-slate-400 hover:text-slate-200 transition-colors">时间</button>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors">
                <div class="text-[13px] text-slate-300">房间玩家排序规则</div>
                <div class="flex bg-[#1e1f22] border border-[#3f4147] rounded p-1 gap-1">
                   <button class="px-3 py-1 text-[12px] bg-[#3f4147] text-white rounded shadow-sm transition-colors">时间</button>
                   <button class="px-3 py-1 text-[12px] text-slate-400 hover:text-slate-200 transition-colors">字母顺序</button>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors">
                <div class="text-[13px] text-slate-300">自定义分页</div>
                <button class="px-4 py-1.5 bg-[#2b2d31] border border-[#3f4147] rounded text-[13px] hover:bg-[#35373c] transition-colors">配置</button>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors">
                <div class="text-[13px] text-slate-300">条目读取设置</div>
                <button class="px-4 py-1.5 bg-[#2b2d31] border border-[#3f4147] rounded text-[13px] hover:bg-[#35373c] transition-colors">配置</button>
              </div>
            </div>
          </div>

          <!-- 时间 / 日期 -->
          <div>
            <h2 class="text-[15px] font-bold text-white mb-4">时间 / 日期</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors">
                <div class="text-[13px] text-slate-300">时间格式</div>
                <div class="flex bg-[#1e1f22] border border-[#3f4147] rounded p-1 gap-1">
                   <button class="px-3 py-1 text-[12px] text-slate-400 hover:text-slate-200 transition-colors">12 小时制</button>
                   <button class="px-3 py-1 text-[12px] bg-[#3f4147] text-white rounded shadow-sm transition-colors">24 小时制</button>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer">
                <div class="text-[13px] text-slate-300">强制使用 ISO 时间格式</div>
                <div class="relative inline-block w-8 h-4 rounded-full bg-[#3f4147] transition-colors">
                  <div class="absolute left-1 top-0.5 w-3 h-3 rounded-full bg-slate-400 shadow-sm transition-transform"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors">
                <div class="text-[13px] text-slate-300">一周的第一天</div>
                <select class="bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] text-slate-200 outline-none w-48 focus:border-indigo-500">
                  <option>星期一</option>
                  <option>星期日</option>
                </select>
              </div>
            </div>
          </div>

          <!-- 玩家信息 -->
          <div>
            <h2 class="text-[15px] font-bold text-white mb-4">玩家信息</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer">
                <div class="text-[13px] text-slate-300">显示线上备注</div>
                <div class="relative inline-block w-8 h-4 rounded-full bg-indigo-500 transition-colors">
                  <div class="absolute right-1 top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-transform"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer">
                <div class="text-[13px] text-slate-300">显示本地备注</div>
                <div class="relative inline-block w-8 h-4 rounded-full bg-[#3f4147] transition-colors">
                  <div class="absolute left-1 top-0.5 w-3 h-3 rounded-full bg-slate-400 shadow-sm transition-transform"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- 好友日志 -->
          <div>
            <h2 class="text-[15px] font-bold text-white mb-4">好友日志</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer">
                <div class="text-[13px] text-slate-300">隐藏 "删除好友" (或被删好友) 事件</div>
                <div class="relative inline-block w-8 h-4 rounded-full bg-[#3f4147] transition-colors">
                  <div class="absolute left-1 top-0.5 w-3 h-3 rounded-full bg-slate-400 shadow-sm transition-transform"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- 好友名称显示颜色 -->
          <div>
            <h2 class="text-[15px] font-bold text-white mb-4">好友名称显示颜色</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer">
                <div class="text-[13px] text-slate-300">从玩家 ID 随机挑选颜色</div>
                <div class="relative inline-block w-8 h-4 rounded-full bg-[#3f4147] transition-colors">
                  <div class="absolute left-1 top-0.5 w-3 h-3 rounded-full bg-slate-400 shadow-sm transition-transform"></div>
                </div>
              </div>
              
              <!-- Color Grid -->
              <div class="grid grid-cols-2 gap-x-10 gap-y-4 mt-4 px-3">
                <div class="flex flex-col gap-1.5">
                  <span class="text-[12px] text-slate-400 font-bold">游客</span>
                  <div class="flex items-center gap-2">
                    <input type="color" value="#CCCCCC" class="w-8 h-8 rounded bg-transparent border-0 cursor-pointer p-0">
                    <input type="text" value="#CCCCCC" class="bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] w-28 outline-none focus:border-indigo-500 font-mono text-slate-300">
                  </div>
                </div>
                <div class="flex flex-col gap-1.5">
                  <span class="text-[12px] text-slate-400 font-bold">萌新</span>
                  <div class="flex items-center gap-2">
                    <input type="color" value="#1778FF" class="w-8 h-8 rounded bg-transparent border-0 cursor-pointer p-0">
                    <input type="text" value="#1778FF" class="bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] w-28 outline-none focus:border-indigo-500 font-mono text-slate-300">
                  </div>
                </div>
                <div class="flex flex-col gap-1.5">
                  <span class="text-[12px] text-slate-400 font-bold">玩家</span>
                  <div class="flex items-center gap-2">
                    <input type="color" value="#2BCF5C" class="w-8 h-8 rounded bg-transparent border-0 cursor-pointer p-0">
                    <input type="text" value="#2BCF5C" class="bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] w-28 outline-none focus:border-indigo-500 font-mono text-slate-300">
                  </div>
                </div>
                <div class="flex flex-col gap-1.5">
                  <span class="text-[12px] text-slate-400 font-bold">长期玩家</span>
                  <div class="flex items-center gap-2">
                    <input type="color" value="#FF7B42" class="w-8 h-8 rounded bg-transparent border-0 cursor-pointer p-0">
                    <input type="text" value="#FF7B42" class="bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] w-28 outline-none focus:border-indigo-500 font-mono text-slate-300">
                  </div>
                </div>
                <div class="flex flex-col gap-1.5">
                  <span class="text-[12px] text-slate-400 font-bold">资深玩家</span>
                  <div class="flex items-center gap-2">
                    <input type="color" value="#B18FFF" class="w-8 h-8 rounded bg-transparent border-0 cursor-pointer p-0">
                    <input type="text" value="#B18FFF" class="bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] w-28 outline-none focus:border-indigo-500 font-mono text-slate-300">
                  </div>
                </div>
                <div class="flex flex-col gap-1.5">
                  <span class="text-[12px] text-slate-400 font-bold">VRChat 团队成员</span>
                  <div class="flex items-center gap-2">
                    <input type="color" value="#FF2826" class="w-8 h-8 rounded bg-transparent border-0 cursor-pointer p-0">
                    <input type="text" value="#FF2826" class="bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] w-28 outline-none focus:border-indigo-500 font-mono text-slate-300">
                  </div>
                </div>
                <div class="flex flex-col gap-1.5">
                  <span class="text-[12px] text-slate-400 font-bold">劣迹玩家</span>
                  <div class="flex items-center gap-2">
                    <input type="color" value="#7B2F2F" class="w-8 h-8 rounded bg-transparent border-0 cursor-pointer p-0">
                    <input type="text" value="#7B2F2F" class="bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] w-28 outline-none focus:border-indigo-500 font-mono text-slate-300">
                  </div>
                </div>
              </div>
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
            <h2 class="text-[15px] font-bold text-white mb-4">VRCX 更新设置</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors">
                <div class="flex items-center gap-2">
                  <DownloadCloud class="w-4 h-4 text-slate-400" />
                  <div class="text-[13px] text-slate-300">软件更新 (当前版本 v{{ appVersion }})</div>
                </div>
                <button
                  :disabled="isCheckingUpdate"
                  class="px-4 py-1.5 bg-[#2b2d31] hover:bg-[#35373c] border border-[#3f4147] rounded text-[13px] text-white transition-colors disabled:opacity-50 flex items-center gap-2"
                  @click="checkForUpdates(false)"
                >
                  <span v-if="isCheckingUpdate" class="w-3.5 h-3.5 border border-slate-400 border-t-transparent rounded-full animate-spin" />
                  {{ isCheckingUpdate ? '正在检查...' : '检查更新' }}
                </button>
              </div>
              <div v-if="checkUpdateStatus" class="px-3 py-2 text-[12px] text-indigo-400 ml-6">
                {{ checkUpdateStatus }}
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.autoCheckUpdate = !config.autoCheckUpdate">
                <div class="text-[13px] text-slate-300">自动检查更新</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.autoCheckUpdate ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.autoCheckUpdate ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- 首选项 -->
          <div>
            <h2 class="text-[15px] font-bold text-white mb-4">首选项</h2>
            <div class="space-y-1">
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.autoStart = !config.autoStart">
                <div>
                  <div class="text-[13px] text-slate-300">在 Windows 启动时自动启动</div>
                  <div class="text-[11px] text-slate-500 mt-0.5">开机自动运行本软件</div>
                </div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.autoStart ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.autoStart ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.minimizeToTray = !config.minimizeToTray">
                <div class="text-[13px] text-slate-300">最小化到系统托盘</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.minimizeToTray ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.minimizeToTray ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.topWindow = !config.topWindow">
                <div class="text-[13px] text-slate-300">窗口始终置顶</div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.topWindow ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.topWindow ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
              <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.enableDebugConsole = !config.enableDebugConsole">
                <div class="flex items-center gap-2">
                  <div class="text-[13px] text-slate-300">启用开发者控制台</div>
                  <span class="px-1.5 py-0.5 bg-blue-500/20 text-blue-400 text-[10px] rounded font-bold">Dev</span>
                </div>
                <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.enableDebugConsole ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                  <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.enableDebugConsole ? 'right-1' : 'left-1'"></div>
                </div>
              </div>
            </div>
          </div>

          <!-- 游戏设置 -->
          <div>
            <h2 class="text-[15px] font-bold text-white mb-4">游戏设置</h2>
            <div class="space-y-4 px-3">
              <div>
                <div class="text-[13px] text-slate-300 mb-2">VRChat 启动参数</div>
                <input
                  v-model="config.vrchatLaunchArgs"
                  type="text"
                  placeholder="例如: --no-vr --profile=2"
                  class="w-full bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] text-slate-200 outline-none focus:border-indigo-500 transition-colors"
                >
                <div class="text-[11px] text-slate-500 mt-1.5">通过本软件启动 VRChat 时将携带这些参数。</div>
              </div>
            </div>
          </div>

          <!-- API 设置 -->
          <div>
            <h2 class="text-[15px] font-bold text-white mb-4">API 设置</h2>
            <div class="space-y-1">
              <div class="flex flex-col p-3 hover:bg-white/5 rounded-lg transition-colors">
                <div class="flex items-center justify-between mb-2">
                  <div class="text-[13px] text-slate-300">后台数据刷新频率</div>
                  <div class="text-[13px] font-bold text-indigo-400">{{ config.pollInterval }} 秒</div>
                </div>
                <input
                  v-model="config.pollInterval"
                  type="range"
                  min="10"
                  max="120"
                  step="5"
                  class="w-full h-1 bg-[#3f4147] rounded-lg appearance-none cursor-pointer accent-indigo-500"
                >
              </div>
            </div>
          </div>
        </div>

          <!-- 消息通知 (Notifications) -->
          <div v-else-if="activeTab === 'notifications'" class="max-w-4xl mx-auto space-y-8 pb-10 mt-4">
            <div>
              <h2 class="text-[15px] font-bold text-white mb-4 flex items-center gap-2">桌面通知</h2>
              <div class="space-y-1">
                <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors">
                  <div class="text-[13px] text-slate-300">何时显示桌面通知</div>
                  <select v-model="config.notifyDesktopCondition" class="bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] text-slate-200 outline-none w-48 focus:border-indigo-500">
                    <option value="never">从不</option>
                    <option value="desktop">在桌面模式下</option>
                    <option value="vr">在 VR 模式下</option>
                    <option value="not_vr">非 VR 模式下</option>
                    <option value="vrc_running">当 VRChat 运行时</option>
                    <option value="vrc_not_running">当 VRChat 未运行时</option>
                    <option value="always">总是</option>
                  </select>
                </div>
                <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.notifyShowWhenAfk = !config.notifyShowWhenAfk">
                  <div class="text-[13px] text-slate-300">当您不在电脑前时显示桌面通知</div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.notifyShowWhenAfk ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.notifyShowWhenAfk ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>
              </div>
            </div>

            <div>
              <h2 class="text-[15px] font-bold text-white mb-4 flex items-center gap-2">语音播报 (TTS)</h2>
              <div class="space-y-1">
                <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors">
                  <div class="text-[13px] text-slate-300">何时启用 TTS</div>
                  <select v-model="config.notifyTtsCondition" class="bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] text-slate-200 outline-none w-48 focus:border-indigo-500">
                    <option value="never">从不</option>
                    <option value="desktop">在桌面模式下</option>
                    <option value="vr">在 VR 模式下</option>
                    <option value="not_vr">非 VR 模式下</option>
                    <option value="vrc_running">当 VRChat 运行时</option>
                    <option value="vrc_not_running">当 VRChat 未运行时</option>
                    <option value="always">总是</option>
                  </select>
                </div>
                
                <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.notifyTts = !config.notifyTts">
                  <div>
                    <div class="text-[13px] text-slate-300">启用事件语音播报</div>
                    <div class="text-[11px] text-slate-500 mt-0.5">自动将上下线等消息转化为语音播报</div>
                  </div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.notifyTts ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.notifyTts ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>

                <div v-if="config.notifyTts" class="flex flex-col p-3 hover:bg-white/5 rounded-lg transition-colors ml-6 border-l-2 border-indigo-500/30">
                  <div class="flex items-center justify-between mb-2">
                    <div class="text-[13px] text-slate-300">播报音量</div>
                    <div class="text-[13px] font-bold text-indigo-400">{{ config.notifyTtsVolume }}%</div>
                  </div>
                  <input
                    v-model="config.notifyTtsVolume"
                    type="range"
                    min="0"
                    max="100"
                    step="1"
                    class="w-full h-1 bg-[#3f4147] rounded-lg appearance-none cursor-pointer accent-indigo-500"
                  >
                </div>

                <div class="flex gap-4 p-3 mt-2">
                  <button
                    class="px-4 py-1.5 bg-[#2b2d31] hover:bg-[#35373c] border border-[#3f4147] rounded text-[13px] text-slate-300 hover:text-white transition-colors flex items-center gap-2"
                    @click="testTTS"
                  >
                    <Play class="w-3.5 h-3.5" /> 播放测试语音
                  </button>
                  <button
                    class="px-4 py-1.5 bg-[#2b2d31] hover:bg-[#35373c] border border-[#3f4147] rounded text-[13px] text-slate-300 hover:text-white transition-colors flex items-center gap-2"
                    @click="testNotification"
                  >
                    <Bell class="w-3.5 h-3.5" /> 发送测试通知
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- 网络 (Network) -->
          <div v-else-if="activeTab === 'network'" class="max-w-4xl mx-auto space-y-8 pb-10 mt-4">
            <div>
              <h2 class="text-[15px] font-bold text-white mb-4">代理服务器设置</h2>
              <div class="space-y-1">
                <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.proxyEnabled = !config.proxyEnabled">
                  <div class="flex items-center gap-2">
                    <div class="text-[13px] text-slate-300">启用网络代理</div>
                    <span class="px-1.5 py-0.5 bg-blue-500/20 text-blue-400 text-[10px] rounded font-bold">API 加速</span>
                  </div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.proxyEnabled ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.proxyEnabled ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>

                <div v-if="config.proxyEnabled" class="p-3 ml-6 border-l-2 border-indigo-500/30">
                  <div class="text-[13px] text-slate-300 mb-2">代理服务器地址 (HTTP / SOCKS5)</div>
                  <input
                    v-model="config.proxyUrl"
                    type="text"
                    placeholder="例如: http://127.0.0.1:7890"
                    class="w-full max-w-sm bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] text-slate-200 outline-none focus:border-indigo-500 font-mono transition-colors"
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
              <h2 class="text-[15px] font-bold text-white mb-4">VRChat 缓存</h2>
              <div class="space-y-4">
                <div class="p-4 bg-[#1e1f22] border border-[#3f4147] rounded-lg">
                  <div class="flex items-center justify-between mb-4">
                    <div>
                      <div class="text-[13px] text-slate-300">清理 VRChat 缓存</div>
                      <div class="text-[11px] text-slate-500 mt-0.5">将清理图片和世界缓存文件，释放磁盘空间。</div>
                    </div>
                    <button
                      :disabled="isClearing"
                      class="px-4 py-1.5 bg-red-500/10 hover:bg-red-500/20 border border-red-500/20 rounded text-[13px] text-red-400 transition-colors flex items-center gap-2 disabled:opacity-50"
                      @click="clearCache"
                    >
                      <Trash2 class="w-3.5 h-3.5" :class="{'animate-spin': isClearing}" />
                      {{ isClearing ? '正在清理...' : '一键清理' }}
                    </button>
                  </div>
                  <div v-if="actionMessage && activeTab === 'storage'" class="px-3 py-2 text-[12px] text-green-400 bg-green-400/10 rounded mb-2">{{ actionMessage }}</div>
                  <div v-if="actionError && activeTab === 'storage'" class="px-3 py-2 text-[12px] text-red-400 bg-red-400/10 rounded">{{ actionError }}</div>
                </div>

                <div class="p-3 hover:bg-white/5 rounded-lg transition-colors border border-transparent">
                  <div class="flex items-center justify-between mb-2">
                    <div class="text-[13px] text-slate-300">本地数据缓存上限</div>
                    <div class="text-[13px] font-bold text-indigo-400">{{ config.cacheLimit }} GB</div>
                  </div>
                  <div class="text-[11px] text-slate-500 mb-3">超出此限制时将自动清理最旧的缓存文件。</div>
                  <input
                    v-model="config.cacheLimit"
                    type="range"
                    min="1"
                    max="20"
                    class="w-full h-1 bg-[#3f4147] rounded-lg appearance-none cursor-pointer accent-indigo-500"
                  >
                </div>
              </div>
            </div>
          </div>

          <!-- 集成 (Discord / APIs) -->
          <div v-else-if="activeTab === 'discord'" class="max-w-4xl mx-auto space-y-8 pb-10 mt-4">
            <div>
              <h2 class="text-[15px] font-bold text-white mb-4 flex items-center gap-2">
                Discord RPC
                <span class="px-1.5 py-0.5 bg-indigo-500/20 text-indigo-400 text-[10px] rounded font-bold">VRChat 专属</span>
              </h2>
              <div class="text-[12px] text-amber-500/80 mb-4 px-1">建议在 VRChat 的 “config.json” 中停用原生的 Discord RPC 来防止冲突。</div>

              <div class="space-y-1">
                <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.discordRpcEnabled = !config.discordRpcEnabled">
                  <div class="text-[13px] text-slate-300 font-bold">启用 Rich Presence</div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.discordRpcEnabled ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.discordRpcEnabled ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>

                <template v-if="config.discordRpcEnabled">
                  <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.discordRpcEnableWorldIntegration = !config.discordRpcEnableWorldIntegration">
                    <div>
                      <div class="text-[13px] text-slate-300">启用世界集成</div>
                      <div class="text-[11px] text-slate-500 mt-0.5">允许显示详细的世界信息。</div>
                    </div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.discordRpcEnableWorldIntegration ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.discordRpcEnableWorldIntegration ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>

                  <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.discordRpcShowRoomTypeAndCount = !config.discordRpcShowRoomTypeAndCount">
                    <div class="text-[13px] text-slate-300">显示房间类型与人数</div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.discordRpcShowRoomTypeAndCount ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.discordRpcShowRoomTypeAndCount ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>

                  <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.discordRpcShowPlatform = !config.discordRpcShowPlatform">
                    <div class="text-[13px] text-slate-300">显示所在平台 (PC / Quest)</div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.discordRpcShowPlatform ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.discordRpcShowPlatform ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>

                  <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.discordRpcShowRoomInfoInPrivate = !config.discordRpcShowRoomInfoInPrivate">
                    <div class="text-[13px] text-slate-300">在私有房间中显示信息</div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.discordRpcShowRoomInfoInPrivate ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.discordRpcShowRoomInfoInPrivate ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>

                  <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.discordRpcShowJoinButton = !config.discordRpcShowJoinButton">
                    <div class="text-[13px] text-slate-300">显示“加入”按钮</div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.discordRpcShowJoinButton ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.discordRpcShowJoinButton ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>

                  <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.discordRpcShowWorldThumbnail = !config.discordRpcShowWorldThumbnail">
                    <div class="text-[13px] text-slate-300">显示世界缩略图</div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.discordRpcShowWorldThumbnail ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.discordRpcShowWorldThumbnail ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>

                  <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.discordRpcShowWorldName = !config.discordRpcShowWorldName">
                    <div class="text-[13px] text-slate-300">显示世界名称</div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.discordRpcShowWorldName ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.discordRpcShowWorldName ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>

                  <div class="p-3 border-t border-[#3f4147] mt-2 pt-4">
                    <div class="flex items-center gap-4">
                      <div class="flex-1">
                        <div class="text-[13px] text-slate-300 mb-1">自定义 Details 文本</div>
                        <input
                          v-model="config.discordRpcDetails"
                          type="text"
                          class="w-full bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] text-slate-200 outline-none focus:border-indigo-500 transition-colors"
                        >
                      </div>
                      <div class="flex-1">
                        <div class="text-[13px] text-slate-300 mb-1">自定义 State 文本</div>
                        <input
                          v-model="config.discordRpcState"
                          type="text"
                          class="w-full bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] text-slate-200 outline-none focus:border-indigo-500 transition-colors"
                        >
                      </div>
                    </div>
                  </div>
                </template>
              </div>
            </div>

            <!-- Translation API -->
            <div class="pt-4 border-t border-[#3f4147]">
              <h2 class="text-[15px] font-bold text-white mb-4">翻译 API</h2>
              <div class="space-y-1">
                <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.translationApiEnabled = !config.translationApiEnabled">
                  <div>
                    <div class="text-[13px] text-slate-300">启用翻译 API</div>
                    <div class="text-[11px] text-slate-500 mt-0.5">将游戏内的聊天消息和状态机自动翻译</div>
                  </div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.translationApiEnabled ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.translationApiEnabled ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>
                <div v-if="config.translationApiEnabled" class="p-3 ml-6 border-l-2 border-indigo-500/30">
                  <div class="text-[13px] text-slate-300 mb-2">输入 API 密钥</div>
                  <input
                    v-model="config.translationApiKey"
                    type="password"
                    placeholder="输入密钥"
                    class="w-full max-w-sm bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] text-slate-200 outline-none focus:border-indigo-500 font-mono transition-colors"
                  >
                </div>
              </div>
            </div>

            <!-- YouTube API -->
            <div class="pt-4 border-t border-[#3f4147]">
              <h2 class="text-[15px] font-bold text-white mb-4">YouTube API</h2>
              <div class="space-y-1">
                <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.youtubeApiEnabled = !config.youtubeApiEnabled">
                  <div>
                    <div class="text-[13px] text-slate-300">启用 YouTube API</div>
                    <div class="text-[11px] text-slate-500 mt-0.5">解析视频 URL 和封面</div>
                  </div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.youtubeApiEnabled ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.youtubeApiEnabled ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>
                <div v-if="config.youtubeApiEnabled" class="p-3 ml-6 border-l-2 border-indigo-500/30">
                  <div class="text-[13px] text-slate-300 mb-2">输入 API 密钥</div>
                  <input
                    v-model="config.youtubeApiKey"
                    type="password"
                    placeholder="输入密钥"
                    class="w-full max-w-sm bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1.5 text-[13px] text-slate-200 outline-none focus:border-indigo-500 font-mono transition-colors"
                  >
                </div>
              </div>
            </div>

            <!-- Remote Avatar Database -->
            <div class="pt-4 border-t border-[#3f4147]">
              <h2 class="text-[15px] font-bold text-white mb-4">远程头像数据库</h2>
              <div class="space-y-1">
                <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.remoteAvatarDbEnabled = !config.remoteAvatarDbEnabled">
                  <div>
                    <div class="text-[13px] text-slate-300">启用头像数据库查询</div>
                    <div class="text-[11px] text-slate-500 mt-0.5">在查看玩家资料时自动尝试从云端加载模型</div>
                  </div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.remoteAvatarDbEnabled ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.remoteAvatarDbEnabled ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>
                <div v-if="config.remoteAvatarDbEnabled" class="flex justify-end pt-2 px-3">
                  <button class="px-4 py-1.5 bg-[#2b2d31] hover:bg-[#35373c] border border-[#3f4147] rounded text-[13px] text-slate-300 hover:text-white transition-colors">
                    贡献我的头像信息
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- 第三方程序自动化 (Auto Launch) -->
          <div v-else-if="activeTab === 'auto_launch'" class="max-w-4xl mx-auto space-y-8 pb-10 mt-4">
            <div>
              <div class="flex items-center gap-2 mb-4 text-white">
                <Rocket :size="18" class="text-indigo-400" />
                <h2 class="text-[15px] font-bold">启动项自动化</h2>
              </div>
              <div class="text-[12px] text-slate-400 mb-6">当 VRChat 启动时自动运行以下程序，退出时自动结束它们。</div>

              <div class="space-y-4">
                <div class="space-y-1">
                  <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.killAppsOnExit = !config.killAppsOnExit">
                    <div>
                      <div class="text-[13px] text-slate-300">VRChat 退出时自动关闭应用</div>
                      <div class="text-[11px] text-slate-500 mt-0.5">当检测到游戏关闭，将结束下方列表中运行的程序</div>
                    </div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.killAppsOnExit ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.killAppsOnExit ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>

                  <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.clearCacheOnExit = !config.clearCacheOnExit">
                    <div>
                      <div class="text-[13px] text-slate-300">VRChat 退出时自动清理缓存</div>
                      <div class="text-[11px] text-slate-500 mt-0.5">保持您的磁盘空间充足</div>
                    </div>
                    <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.clearCacheOnExit ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                      <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.clearCacheOnExit ? 'right-1' : 'left-1'"></div>
                    </div>
                  </div>
                </div>

                <div class="p-4 bg-[#1e1f22] border border-[#3f4147] rounded-lg">
                  <div class="text-[13px] text-slate-300 mb-2">自动启动应用列表</div>
                  <div class="text-[11px] text-slate-500 mb-3">每行填写一个程序的完整绝对路径 (例如: C:\Tools\OSC\app.exe)</div>
                  <textarea
                    v-model="parsedAutoLaunchApps"
                    rows="6"
                    class="w-full bg-[#111214] border border-[#3f4147] rounded-lg px-3 py-2 text-[13px] text-slate-200 outline-none focus:border-indigo-500 font-mono resize-y"
                    placeholder="C:\Program Files\Example\app.exe&#10;D:\Tools\OSC\tracker.exe"
                  />
                </div>
              </div>
            </div>
          </div>

                    <div v-else-if="activeTab === 'advanced'" class="max-w-4xl mx-auto space-y-8 pb-10 mt-4">
            <div>
              <h2 class="text-[15px] font-bold text-white mb-4">高级系统选项</h2>
              <div class="space-y-1">
                <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors">
                  <div class="text-[13px] text-slate-300">Web API 超时限制</div>
                  <div class="flex items-center gap-2">
                    <input
                      v-model.number="config.webApiTimeout"
                      type="number"
                      min="1"
                      max="60"
                      class="w-16 bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1 text-[13px] text-slate-200 outline-none focus:border-indigo-500 font-mono text-center"
                    >
                    <span class="text-[13px] text-slate-500">秒</span>
                  </div>
                </div>

                <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors">
                  <div class="text-[13px] text-slate-300">并发请求上限</div>
                  <div class="flex items-center gap-2">
                    <input
                      v-model.number="config.requestLimit"
                      type="number"
                      min="1"
                      max="50"
                      class="w-16 bg-[#1e1f22] border border-[#3f4147] rounded px-3 py-1 text-[13px] text-slate-200 outline-none focus:border-indigo-500 font-mono text-center"
                    >
                    <span class="text-[13px] text-slate-500">个</span>
                  </div>
                </div>

                <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.hardwareAcceleration = !config.hardwareAcceleration">
                  <div>
                    <div class="text-[13px] text-slate-300">启用硬件加速</div>
                    <div class="text-[11px] text-red-500/80 mt-0.5">更改后必须重启生效。如遇黑屏请尝试关闭此选项。</div>
                  </div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.hardwareAcceleration ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.hardwareAcceleration ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>

                <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.customUrlScheme = !config.customUrlScheme">
                  <div class="text-[13px] text-slate-300">注册自定义 URL Scheme (vrcx://)</div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.customUrlScheme ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.customUrlScheme ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>

                <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.openLocalFilesWithVrcx = !config.openLocalFilesWithVrcx">
                  <div class="text-[13px] text-slate-300">使用本应用打开本地缓存文件</div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.openLocalFilesWithVrcx ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.openLocalFilesWithVrcx ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>

                <div class="flex items-center justify-between p-3 hover:bg-white/5 rounded-lg transition-colors cursor-pointer" @click="config.oscAutomation = !config.oscAutomation">
                  <div>
                    <div class="text-[13px] text-slate-300">启用 OSC 自动化</div>
                    <div class="text-[11px] text-slate-500 mt-0.5">允许通过 OSC 协议发送应用状态到 VRChat</div>
                  </div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors" :class="config.oscAutomation ? 'bg-indigo-500' : 'bg-[#3f4147]'">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.oscAutomation ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>
              </div>
            </div>

            <div>
              <h2 class="text-[15px] font-bold text-white mb-4">VRChat Config.json 编辑器</h2>
              <div class="space-y-4">
                <div class="p-4 bg-[#1e1f22] border border-[#3f4147] rounded-lg">
                  <div class="flex items-center justify-between mb-4">
                    <div>
                      <div class="text-[13px] text-slate-300">高级配置编辑</div>
                      <div class="text-[11px] text-slate-500 mt-0.5">直接修改 VRChat 的 config.json。如果文件损坏可能会导致游戏无法启动。</div>
                    </div>
                    <button 
                      :disabled="vrcConfigSaving" 
                      class="px-4 py-1.5 bg-indigo-500 hover:bg-indigo-600 text-white font-bold text-[13px] rounded transition-colors flex items-center gap-2 disabled:opacity-50"
                      @click="saveVrcConfig"
                    >
                      <Save v-if="!vrcConfigSaving" :size="14" />
                      <Loader2 v-else :size="14" class="animate-spin" />
                      {{ vrcConfigSaving ? '保存中...' : '保存配置' }}
                    </button>
                  </div>

                  <div class="flex items-center gap-2 mb-4">
                    <button
                      class="px-3 py-1.5 bg-[#2b2d31] hover:bg-[#35373c] border border-[#3f4147] rounded text-[13px] text-slate-300 hover:text-white transition-colors flex items-center gap-2"
                      @click="pickFolderForConfig('cache_directory')"
                    >
                      <HardDrive :size="14" /> 选择缓存目录
                    </button>
                    <button
                      class="px-3 py-1.5 bg-[#2b2d31] hover:bg-[#35373c] border border-[#3f4147] rounded text-[13px] text-slate-300 hover:text-white transition-colors flex items-center gap-2"
                      @click="pickFolderForConfig('camera_res_dir')"
                    >
                      <Image :size="14" /> 选择照片保存目录
                    </button>
                  </div>

                  <div v-if="vrcConfigError" class="mb-3 text-[12px] px-3 py-2 rounded" :class="vrcConfigError.includes('成功') ? 'bg-green-400/10 text-green-400' : 'bg-red-400/10 text-red-400'">
                    {{ vrcConfigError }}
                  </div>

                  <textarea
                    v-model="vrcConfigText"
                    spellcheck="false"
                    class="w-full h-64 p-4 bg-[#111214] text-emerald-400 font-mono text-[13px] rounded-lg border border-[#3f4147] outline-none focus:border-indigo-500 custom-scrollbar resize-y"
                    placeholder="{}"
                  />
                </div>

                <div class="p-4 bg-red-500/5 border border-red-500/20 rounded-lg flex justify-between items-center mt-4">
                  <div class="text-[13px] font-bold text-red-400">重置所有设置并重启应用</div>
                  <button class="px-4 py-1.5 bg-red-500/10 text-red-400 hover:bg-red-500 hover:text-white rounded text-[13px] font-bold transition-colors border border-red-500/20">
                    恢复出厂设置
                  </button>
                </div>
              </div>
            </div>
          </div>

<!-- 隐私安全 -->
                    <!-- 隐私安全 -->
          <div v-else-if="activeTab === 'security'" class="max-w-4xl mx-auto space-y-8 pb-10 mt-4">
            <div>
              <div class="flex items-center gap-2 mb-4 text-white">
                <Shield :size="18" class="text-indigo-400" />
                <h2 class="text-[15px] font-bold">{{ t('settings.section_security') }}</h2>
              </div>
              <div class="text-[12px] text-slate-400 mb-6">{{ t('settings.auth_security') }}</div>

              <div class="p-4 bg-red-500/5 border border-red-500/20 rounded-lg">
                <div class="flex flex-col mb-4">
                  <div class="text-[13px] font-bold text-red-400 flex items-center gap-2">
                    <Shield class="w-4 h-4" /> {{ t('settings.auth_security') }}
                  </div>
                  <div class="text-[11px] text-red-400/80 mt-1">
                    {{ t('settings.auth_security_desc') }}
                  </div>
                </div>

                <button
                  :disabled="isClearingAuth"
                  class="px-4 py-2 bg-red-500/10 hover:bg-red-500 hover:text-white text-red-400 rounded text-[13px] font-bold transition-colors border border-red-500/20 flex items-center gap-2 disabled:opacity-50"
                  @click="clearAuth"
                >
                  <Trash2 class="w-4 h-4" /> {{ isClearingAuth ? t('settings.clearing_auth') : t('settings.force_logout') }}
                </button>

                <p v-if="actionMessage && activeTab === 'security'" class="mt-3 text-[12px] font-bold text-green-400 bg-green-400/10 px-3 py-2 rounded">
                  {{ actionMessage }}
                </p>
              </div>
            </div>
          </div>

          <!-- VR -->
          <div v-else-if="activeTab === 'vr'" class="max-w-4xl mx-auto space-y-8 pb-10 mt-4">
            <div>
              <div class="flex items-center gap-2 mb-4 text-white">
                <h2 class="text-[15px] font-bold">{{ t('settings.section_vr') }}</h2>
              </div>

              <div class="space-y-4">
                <div class="flex items-center justify-between p-3 bg-[#1e1f22] border border-[#3f4147] rounded-lg">
                  <div>
                    <div class="text-[13px] text-slate-300 font-bold">{{ t('settings.vr_overlay_enable') }}</div>
                    <div class="text-[11px] text-slate-500 mt-0.5">{{ t('settings.vr_overlay_enable_desc') }}</div>
                  </div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors cursor-pointer" :class="config.vrOverlayEnabled ? 'bg-indigo-500' : 'bg-[#3f4147]'" @click="config.vrOverlayEnabled = !config.vrOverlayEnabled">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.vrOverlayEnabled ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>

                <div class="p-3 bg-[#1e1f22] border border-[#3f4147] rounded-lg">
                  <div class="text-[13px] text-slate-300 font-bold mb-2">{{ t('settings.vr_overlay_opacity') }}</div>
                  <div class="flex items-center gap-4">
                    <input
                      v-model="config.vrOverlayOpacity"
                      type="range"
                      min="10"
                      max="100"
                      step="5"
                      class="flex-1 h-1.5 bg-[#111214] rounded-lg appearance-none cursor-pointer accent-indigo-500"
                    >
                    <div class="text-[13px] text-slate-300 font-bold w-10 text-right">{{ config.vrOverlayOpacity }}%</div>
                  </div>
                </div>

                <div class="flex items-center justify-between p-3 bg-[#1e1f22] border border-[#3f4147] rounded-lg">
                  <div>
                    <div class="text-[13px] text-slate-300 font-bold">{{ t('settings.vr_hand_tracking') }}</div>
                    <div class="text-[11px] text-slate-500 mt-0.5">{{ t('settings.vr_hand_tracking_desc') }}</div>
                  </div>
                  <div class="relative inline-block w-8 h-4 rounded-full transition-colors cursor-pointer" :class="config.wristMode ? 'bg-indigo-500' : 'bg-[#3f4147]'" @click="config.wristMode = !config.wristMode">
                    <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.wristMode ? 'right-1' : 'left-1'"></div>
                  </div>
                </div>

                <div class="p-3 bg-[#1e1f22] border border-[#3f4147] rounded-lg">
                  <div class="text-[13px] text-slate-300 font-bold mb-4">{{ t('settings.auto_61453b9e') }}</div>
                  <div class="space-y-4">
                    <div class="flex items-center justify-between">
                      <div class="text-[12px] text-slate-400">{{ t('settings.auto_c1b1a54b') }}</div>
                      <div class="flex items-center gap-3 w-1/2">
                        <input v-model.number="config.transPanelMaxWidth" type="range" min="300" max="1200" step="10" class="flex-1 h-1 bg-[#111214] rounded-lg appearance-none cursor-pointer accent-indigo-500">
                        <span class="text-[12px] font-mono text-slate-300 w-12 text-right">{{ config.transPanelMaxWidth }}px</span>
                      </div>
                    </div>
                    <div class="flex items-center justify-between">
                      <div class="text-[12px] text-slate-400">{{ t('settings.auto_21da7a06') }}</div>
                      <div class="flex items-center gap-3 w-1/2">
                        <input v-model.number="config.overlayFontSize" type="range" min="12" max="72" step="1" class="flex-1 h-1 bg-[#111214] rounded-lg appearance-none cursor-pointer accent-indigo-500">
                        <span class="text-[12px] font-mono text-slate-300 w-12 text-right">{{ config.overlayFontSize }}pt</span>
                      </div>
                    </div>
                    <div class="flex items-center justify-between">
                      <div class="text-[12px] text-slate-400">{{ t('settings.auto_f6db9e99') }}</div>
                      <input v-model="config.statusColor" type="color" class="w-8 h-8 p-0 border border-[#3f4147] rounded cursor-pointer bg-[#111214]">
                    </div>
                  </div>
                </div>

                <div class="p-3 bg-[#1e1f22] border border-[#3f4147] rounded-lg">
                  <div class="text-[13px] text-slate-300 font-bold mb-4">{{ t('settings.auto_4d7096cf') }}</div>
                  <div class="flex items-center justify-between">
                    <div class="text-[12px] text-slate-400">{{ t('settings.auto_800eddf3') }}</div>
                    <div class="flex items-center gap-3 w-1/2">
                      <input v-model.number="config.gripPressureThreshold" type="range" min="0.1" max="1.0" step="0.05" class="flex-1 h-1 bg-[#111214] rounded-lg appearance-none cursor-pointer accent-indigo-500">
                      <span class="text-[12px] font-mono text-slate-300 w-10 text-right">{{ config.gripPressureThreshold }}</span>
                    </div>
                  </div>
                </div>

                <div class="p-3 bg-[#1e1f22] border border-[#3f4147] rounded-lg">
                  <div class="text-[13px] text-slate-300 font-bold mb-4">{{ t('settings.auto_e96cab7c') }}</div>
                  <div class="space-y-4">
                    <div class="flex items-center justify-between">
                      <div class="text-[12px] text-slate-400">{{ t('settings.auto_47099174') }}</div>
                      <div class="flex items-center gap-3 w-1/2">
                        <input v-model.number="config.ocrContrast" type="range" min="0.5" max="2.0" step="0.1" class="flex-1 h-1 bg-[#111214] rounded-lg appearance-none cursor-pointer accent-indigo-500">
                        <span class="text-[12px] font-mono text-slate-300 w-8 text-right">{{ config.ocrContrast }}</span>
                      </div>
                    </div>
                    <div class="flex items-center justify-between">
                      <div class="text-[12px] text-slate-400">{{ t('settings.auto_a9476266') }}</div>
                      <div class="relative inline-block w-8 h-4 rounded-full transition-colors cursor-pointer" :class="config.ocrSharpen ? 'bg-indigo-500' : 'bg-[#3f4147]'" @click="config.ocrSharpen = !config.ocrSharpen">
                        <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.ocrSharpen ? 'right-1' : 'left-1'"></div>
                      </div>
                    </div>
                    <div class="flex items-center justify-between">
                      <div class="text-[12px] text-slate-400">{{ t('settings.auto_db1dcc8d') }}</div>
                      <div class="relative inline-block w-8 h-4 rounded-full transition-colors cursor-pointer" :class="config.ocrDenoise ? 'bg-indigo-500' : 'bg-[#3f4147]'" @click="config.ocrDenoise = !config.ocrDenoise">
                        <div class="absolute top-0.5 w-3 h-3 rounded-full bg-white shadow-sm transition-all" :class="config.ocrDenoise ? 'right-1' : 'left-1'"></div>
                      </div>
                    </div>
                    <div class="flex items-center justify-between">
                      <div class="text-[12px] text-slate-400">{{ t('settings.auto_4a57a937') }}</div>
                      <div class="flex items-center gap-3 w-1/2">
                        <input v-model.number="config.ocrMergeToleranceX" type="range" min="0.0" max="1.0" step="0.05" class="flex-1 h-1 bg-[#111214] rounded-lg appearance-none cursor-pointer accent-indigo-500">
                        <span class="text-[12px] font-mono text-slate-300 w-8 text-right">{{ config.ocrMergeToleranceX }}</span>
                      </div>
                    </div>
                    <div class="flex items-center justify-between">
                      <div class="text-[12px] text-slate-400">{{ t('settings.auto_1c993b5a') }}</div>
                      <div class="flex items-center gap-3 w-1/2">
                        <input v-model.number="config.ocrMergeToleranceY" type="range" min="0.0" max="1.0" step="0.05" class="flex-1 h-1 bg-[#111214] rounded-lg appearance-none cursor-pointer accent-indigo-500">
                        <span class="text-[12px] font-mono text-slate-300 w-8 text-right">{{ config.ocrMergeToleranceY }}</span>
                      </div>
                    </div>
                  </div>
                </div>

                <div class="p-3 bg-indigo-500/5 border border-indigo-500/20 rounded-lg">
                  <div class="text-[13px] font-bold text-indigo-400 mb-1 flex items-center gap-2">{{ t('settings.auto_d0cd9337') }}</div>
                  <div class="text-[11px] text-indigo-400/80 mb-3">{{ t('settings.auto_533646d8') }}</div>
                  <div class="flex flex-wrap gap-2">
                    <button class="px-3 py-1.5 bg-indigo-500/10 hover:bg-indigo-500 hover:text-white text-indigo-400 border border-indigo-500/20 rounded text-[12px] font-bold transition-colors" @click="registerSteamVR">
                      {{ t('settings.auto_8d481df8') }}
                    </button>
                    <button class="px-3 py-1.5 bg-indigo-500/10 hover:bg-indigo-500 hover:text-white text-indigo-400 border border-indigo-500/20 rounded text-[12px] font-bold transition-colors" @click="openBindings">
                      打开按键绑定面板 (浏览器)
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- OVR OCR -->
          <div v-else-if="activeTab === 'ovr_ocr'" class="max-w-4xl mx-auto space-y-8 pb-10 mt-4">
            <div>
              <div class="flex items-center gap-2 mb-4 text-white">
                <h2 class="text-[15px] font-bold">{{ t('settings.auto_2a5ef7a4') }}</h2>
              </div>
              <div class="space-y-4">
                <div class="p-4 bg-[#1e1f22] border border-[#3f4147] rounded-lg">
                  <div class="text-[13px] text-slate-300 font-bold mb-1">{{ t('settings.auto_0ae96994') }}</div>
                  <div class="text-[11px] text-slate-500 mb-3">{{ t('settings.auto_e9c37226') }}</div>
                  <select v-model="config.ocrLanguage" class="w-full bg-[#111214] border border-[#3f4147] rounded outline-none px-3 py-2 text-[13px] text-slate-200 focus:border-indigo-500">
                    <option value="ja">{{ t('settings.auto_ee36e359') }}</option>
                    <option value="en-US">{{ t('settings.auto_24d56afe') }}</option>
                    <option value="zh-Hans-CN">{{ t('settings.auto_48aad8ae') }}</option>
                    <option value="zh-Hant-TW">{{ t('settings.auto_dd16f5f4') }}</option>
                    <option value="ko">{{ t('settings.auto_80f8fa5a') }}</option>
                  </select>
                </div>
                
                <div class="p-4 bg-[#1e1f22] border border-[#3f4147] rounded-lg">
                  <div class="text-[13px] text-slate-300 font-bold mb-3">{{ t('settings.auto_7b6bd792') }}</div>
                  <select v-model="config.ocrSpeedMode" class="w-full bg-[#111214] border border-[#3f4147] rounded outline-none px-3 py-2 text-[13px] text-slate-200 focus:border-indigo-500">
                    <option value="fast">{{ t('settings.auto_b085bc76') }}</option>
                    <option value="balanced">{{ t('settings.auto_9f909f44') }}</option>
                    <option value="accurate">{{ t('settings.auto_3252bb5f') }}</option>
                  </select>
                </div>

                <div class="p-4 bg-indigo-500/5 border border-indigo-500/20 rounded-lg">
                  <div class="text-[13px] font-bold text-indigo-400 flex items-center gap-2 mb-1">{{ t('settings.auto_5d750047') }}</div>
                  <div class="text-[11px] text-indigo-400/80">{{ t('settings.auto_5e315524') }}</div>
                </div>
              </div>
            </div>
          </div>

          <!-- OVR Translation -->
          <div v-else-if="activeTab === 'ovr_trans'" class="max-w-4xl mx-auto space-y-8 pb-10 mt-4">
            <div>
              <div class="flex items-center gap-2 mb-4 text-white">
                <h2 class="text-[15px] font-bold">{{ t('settings.auto_fb7ea758') }}</h2>
              </div>
              
              <div class="p-4 bg-[#1e1f22] border border-[#3f4147] rounded-lg mb-4">
                <div class="text-[13px] text-slate-300 font-bold mb-3">{{ t('settings.auto_f82d8f40') }}</div>
                <select v-model="config.transService" class="w-full bg-[#111214] border border-[#3f4147] rounded outline-none px-3 py-2 text-[13px] text-slate-200 focus:border-indigo-500 mb-4">
                  <option value="tencent">{{ t('settings.auto_0c26793d') }}</option>
                  <option value="baidu">{{ t('settings.auto_07edd78b') }}</option>
                  <option value="microsoft">{{ t('settings.auto_3aa75e4a') }}</option>
                  <option value="google">Google Translate</option>
                  <option value="deepl">DeepL</option>
                  <option value="openai">OpenAI (LLM)</option>
                  <option value="deepseek">DeepSeek (LLM)</option>
                  <option value="ollama">{{ t('settings.auto_cb420daa') }}</option>
                </select>
                
                <div v-if="['openai', 'deepseek', 'ollama'].includes(config.transService)" class="space-y-4 pt-4 border-t border-[#3f4147]">
                  <div>
                    <label class="block text-[12px] font-bold text-slate-400 mb-1">API Key</label>
                    <input v-model="config.transApiKey" type="password" placeholder="sk-..." class="w-full bg-[#111214] border border-[#3f4147] rounded outline-none px-3 py-1.5 text-[13px] text-slate-200 focus:border-indigo-500 font-mono">
                  </div>
                  <div>
                    <label class="block text-[12px] font-bold text-slate-400 mb-1">{{ t('settings.auto_f291aabf') }}</label>
                    <input v-model="config.transLlmModel" type="text" :placeholder="t('settings.auto_d205ac69')" class="w-full bg-[#111214] border border-[#3f4147] rounded outline-none px-3 py-1.5 text-[13px] text-slate-200 focus:border-indigo-500 font-mono">
                  </div>
                  <div>
                    <label class="block text-[12px] font-bold text-slate-400 mb-1">{{ t('settings.auto_be42d249') }}</label>
                    <input v-model="config.customApiUrl" type="text" placeholder="http://127.0.0.1:11434/v1/chat/completions" class="w-full bg-[#111214] border border-[#3f4147] rounded outline-none px-3 py-1.5 text-[13px] text-slate-200 focus:border-indigo-500 font-mono">
                  </div>
                  <div>
                    <label class="block text-[12px] font-bold text-slate-400 mb-1">{{ t('settings.auto_14735200') }}</label>
                    <textarea v-model="config.transLlmPrompt" rows="3" class="w-full bg-[#111214] border border-[#3f4147] rounded outline-none px-3 py-2 text-[13px] text-slate-200 focus:border-indigo-500 resize-y"></textarea>
                  </div>
                </div>
                
                <div v-else class="space-y-4 pt-4 border-t border-[#3f4147]">
                  <div v-if="['tencent', 'baidu'].includes(config.transService)">
                    <label class="block text-[12px] font-bold text-slate-400 mb-1">{{ t('settings.auto_f70d6b64') }}</label>
                    <input v-model="config.transAppId" type="text" class="w-full bg-[#111214] border border-[#3f4147] rounded outline-none px-3 py-1.5 text-[13px] text-slate-200 focus:border-indigo-500 font-mono">
                  </div>
                  <div>
                    <label class="block text-[12px] font-bold text-slate-400 mb-1">API Key / Secret</label>
                    <input v-model="config.transApiKey" type="password" class="w-full bg-[#111214] border border-[#3f4147] rounded outline-none px-3 py-1.5 text-[13px] text-slate-200 focus:border-indigo-500 font-mono">
                  </div>
                </div>
              </div>
              
              <div class="grid grid-cols-2 gap-4">
                <div class="p-4 bg-[#1e1f22] border border-[#3f4147] rounded-lg">
                  <div class="text-[13px] text-slate-300 font-bold mb-3">{{ t('settings.auto_04d605c2') }}</div>
                  <select v-model="config.transSourceLang" class="w-full bg-[#111214] border border-[#3f4147] rounded outline-none px-3 py-2 text-[13px] text-slate-200 focus:border-indigo-500">
                    <option value="auto">{{ t('settings.auto_ac653a57') }}</option>
                    <option value="en">英语</option>
                    <option value="ja">日语</option>
                    <option value="ko">韩语</option>
                    <option value="zh">中文</option>
                  </select>
                </div>
                <div class="p-4 bg-[#1e1f22] border border-[#3f4147] rounded-lg">
                  <div class="text-[13px] text-slate-300 font-bold mb-3">{{ t('settings.auto_129d0594') }}</div>
                  <select v-model="config.transTargetLang" class="w-full bg-[#111214] border border-[#3f4147] rounded outline-none px-3 py-2 text-[13px] text-slate-200 focus:border-indigo-500">
                    <option value="zh">{{ t('settings.auto_d688a3a4') }}</option>
                    <option value="zh-TW">{{ t('settings.auto_dd16f5f4') }}</option>
                    <option value="en">英语</option>
                  </select>
                </div>
              </div>
            </div>
          </div>

        </Transition>
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
