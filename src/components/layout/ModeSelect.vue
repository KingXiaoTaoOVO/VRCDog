<script setup lang="ts">
import { ref } from 'vue';
import { ShieldAlert, Monitor, Glasses } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { SysApi } from '../../api';
import dogImg from '../../assets/dog.jpg';
import { useUiStore } from '../../stores/uiStore';
import { storeToRefs } from 'pinia';

const { t } = useI18n();
const uiStore = useUiStore();
const { serverModePerms, modeSelectionError } = storeToRefs(uiStore);

const selectAppMode = async (mode: 'pc' | 'vr') => {
  uiStore.modeSelectionError = '';
  if (mode === 'vr') {
    try {
      const isSteamVRRunning = await SysApi.checkSteamVR();
      if (!isSteamVRRunning) {
        uiStore.modeSelectionError = t('auto_1c5874ec');
        return;
      }
    } catch (e: any) {
      uiStore.modeSelectionError = e.message || t('auto_a712a9fb');
      return;
    }
    uiStore.activeTab = 'ovr'; // VR 模式默认进入 OVR 翻译设置
  } else {
    uiStore.activeTab = 'dashboard'; // PC 模式默认进入仪表盘
  }
  uiStore.appMode = mode;

  window.addEventListener('vrc-open-detail', (e: Event) => {
    if (uiStore.activeTab !== 'search') {
      uiStore.activeTab = 'search';
    }
  });
};
</script>

<template>
  <div class="w-full h-screen flex flex-col items-center justify-center bg-background relative overflow-hidden">
    <div class="absolute inset-0 z-0 overflow-hidden pointer-events-none">
      <div class="absolute top-[-20%] left-[-10%] w-[60%] h-[60%] bg-pink-200/40 rounded-full blur-[100px] animate-pulse" />
      <div
        class="absolute bottom-[-10%] left-[20%] w-[50%] h-[50%] bg-indigo-200/40 rounded-full blur-[100px] animate-pulse"
        style="animation-delay: 2s"
      />
    </div>
    
    <div class="z-10 glass-panel p-10 max-w-xl w-full text-center">
      <img
        :src="dogImg"
        class="w-24 h-24 rounded-full border-4 border-white/60 shadow-lg mx-auto mb-6"
      >
      <h2 class="text-3xl font-extrabold text-text mb-2">
        {{ $t('app.select_mode_title') || '选择运行模式' }}
      </h2>
      <p class="text-text-muted mb-8 font-medium">
        {{ $t('app.select_mode_desc') || 'VrcDog 提供桌面管理看板与 SteamVR 沉浸式内置叠加层两种体验。' }}
      </p>
      
      <div class="grid grid-cols-2 gap-4">
        <button
          class="flex flex-col items-center gap-3 p-6 rounded-3xl transition-all group glass-panel-hover"
          :class="[
            serverModePerms['pc'] === false 
              ? 'opacity-50 cursor-not-allowed grayscale' 
              : 'hover:scale-105 active:scale-95'
          ]"
          :disabled="serverModePerms['pc'] === false"
          @click="selectAppMode('pc')"
        >
          <Monitor class="w-12 h-12 text-primary group-hover:text-primary-hover transition-colors" />
          <span class="font-bold text-text text-lg">PC Desktop</span>
          <span
            v-if="serverModePerms['pc'] !== false"
            class="text-xs text-text-muted"
          >{{ $t('auto_3c519b1c') }}</span>
          <span
            v-else
            class="text-xs text-red-500"
          >{{ $t('auto_f31a212e') }}</span>
        </button>
        <button
          class="flex flex-col items-center gap-3 p-6 rounded-3xl transition-all group glass-panel-hover"
          :class="[
            serverModePerms['vr'] === false 
              ? 'opacity-50 cursor-not-allowed grayscale' 
              : 'hover:scale-105 active:scale-95'
          ]"
          :disabled="serverModePerms['vr'] === false"
          @click="selectAppMode('vr')"
        >
          <Glasses class="w-12 h-12 text-primary group-hover:text-primary-hover transition-colors" />
          <span class="font-bold text-text text-lg">VR Overlay</span>
          <span
            v-if="serverModePerms['vr'] !== false"
            class="text-xs text-text-muted"
          >{{ $t('auto_6138dd05') }}</span>
          <span
            v-else
            class="text-xs text-red-500"
          >{{ $t('auto_f31a212e') }}</span>
        </button>
      </div>
      
      <div
        v-if="modeSelectionError"
        class="mt-6 p-3 bg-red-50 text-red-600 rounded-xl border border-red-200 text-sm font-bold flex items-center justify-center gap-2"
      >
        <ShieldAlert class="w-4 h-4" /> {{ modeSelectionError }}
      </div>
    </div>
  </div>
</template>
