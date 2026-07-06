<template>
  <div
    class="h-screen w-screen flex flex-col items-center justify-center p-8 bg-cover bg-center transition-all duration-700"
    :style="{ backgroundColor: themeConfig.colors.bgMain }"
  >
    <!-- 动态背景装饰 -->
    <div class="fixed top-0 left-0 w-full h-full pointer-events-none overflow-hidden z-0">
      <div
        class="absolute top-[-10%] left-[-10%] w-[50%] h-[50%] rounded-full blur-[100px] transition-all duration-1000"
        :style="{ background: themeConfig.colors.blob1 }"
      />
      <div
        class="absolute bottom-[-10%] right-[-10%] w-[60%] h-[60%] rounded-full blur-[120px] transition-all duration-1000"
        :style="{ background: themeConfig.colors.blob2 }"
      />
    </div>

    <div class="fixed top-4 right-4 z-50 flex gap-2">
      <button
        class="flex items-center gap-2 px-4 py-2 bg-surface hover:bg-surface backdrop-blur rounded-xl border-black/10 transition-all font-medium text-sm shadow-sm"
        :style="{ color: themeConfig.colors.textStrong }"
        @click="cycleLanguage"
      >
        <Globe class="w-4 h-4" />
        {{ currentLangLabel }}
      </button>
    </div>

    <div
      class="z-10 bg-surface backdrop-blur-2xl p-10 rounded-[32px] shadow-[0_20px_60px_-15px_rgba(0,0,0,0.1)] transition-all duration-500 w-[500px]"
      :style="{ borderColor: themeConfig.colors.borderSoft }"
    >
      <!-- 头部 Logo -->
      <div class="flex flex-col items-center mb-10">
        <div
          class="w-24 h-24 rounded-[28px] overflow-hidden shadow-lg mb-6 border-[3px] transition-transform hover:scale-105"
          :style="{ borderColor: themeConfig.colors.borderSoft }"
        >
          <img
            :src="themeConfig.logo"
            alt="Logo"
            class="w-full h-full object-cover"
          >
        </div>
        <h1
          class="text-3xl font-black tracking-tight"
          :style="{ color: themeConfig.colors.textStrong }"
        >
          {{ themeConfig.appTitle }}
        </h1>
        <p
          class="text-sm mt-2 font-medium"
          :style="{ color: themeConfig.colors.textSoft }"
        >
          {{ t('role.select_mode') }}
        </p>
      </div>

      <!-- 模式选择 -->
      <div
        v-if="!selectedRole"
        class="grid grid-cols-2 gap-4"
      >
        <button
          class="flex flex-col items-center p-6 rounded-2xl border-2 transition-all hover:scale-[1.02] active:scale-[0.98]" 
          :style="{ borderColor: themeConfig.colors.borderStrong, background: 'rgba(255,255,255,0.6)' }"
          @click="selectRole('client')"
        >
          <Monitor
            class="w-10 h-10 mb-3"
            :style="{ color: themeConfig.colors.primaryBtnBg }"
          />
          <span
            class="font-bold text-lg"
            :style="{ color: themeConfig.colors.textStrong }"
          >{{ t('role.client_mode') }}</span>
          <span
            class="text-xs mt-1 text-center"
            :style="{ color: themeConfig.colors.textSoft }"
          >{{ t('role.client_desc') }}</span>
        </button>

        <button
          class="flex flex-col items-center p-6 rounded-2xl border-2 transition-all hover:scale-[1.02] active:scale-[0.98]" 
          :style="{ borderColor: themeConfig.colors.borderStrong, background: 'rgba(255,255,255,0.6)' }"
          @click="selectRole('server')"
        >
          <Server
            class="w-10 h-10 mb-3"
            :style="{ color: themeConfig.colors.primaryBtnBg }"
          />
          <span
            class="font-bold text-lg"
            :style="{ color: themeConfig.colors.textStrong }"
          >{{ t('role.server_mode') }}</span>
          <span
            class="text-xs mt-1 text-center"
            :style="{ color: themeConfig.colors.textSoft }"
          >{{ t('role.server_desc') }}</span>
        </button>
      </div>

      <!-- 客户端配置 -->
      <div
        v-if="selectedRole === 'client'"
        class="animate-in fade-in slide-in-from-bottom-4 duration-300"
      >
        <div class="mb-4">
          <label
            class="block text-sm font-bold mb-2 ml-1"
            :style="{ color: themeConfig.colors.textStrong }"
          >{{ t('role.server_address') }}</label>
          <input
            v-model="serverUrl"
            type="text"
            :placeholder="t('role.server_address_ph')"
            class="w-full px-5 py-4 rounded-2xl bg-surface border-2 outline-none transition-all placeholder:text-text/30"
            :style="{ borderColor: themeConfig.colors.borderSoft, color: themeConfig.colors.textStrong }"
            @focus="(e) => (e.target as HTMLElement).style.borderColor = themeConfig.colors.borderStrong"
            @blur="(e) => (e.target as HTMLElement).style.borderColor = themeConfig.colors.borderSoft"
          >
        </div>
        <div class="flex gap-3 mt-6">
          <button
            class="group relative overflow-hidden px-5 py-4 rounded-2xl font-bold transition-all duration-300 flex-1 flex items-center justify-center gap-2 active:scale-95 shadow-sm hover:shadow-md"
            :style="{ background: 'var(--color-surface)', color: 'var(--color-text-strong)', border: '1px solid var(--color-border-soft)' }"
            @mouseover="(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--color-border-strong)'; (e.currentTarget as HTMLElement).style.transform = 'translateY(-2px)' }"
            @mouseleave="(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--color-border-soft)'; (e.currentTarget as HTMLElement).style.transform = 'translateY(0)' }"
            @click="selectedRole = null"
          >
            <div class="absolute inset-0 bg-gradient-to-r from-transparent via-white/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500"></div>
            <div class="absolute inset-0 opacity-0 group-hover:opacity-5 transition-opacity duration-300" style="background: currentColor;"></div>
            <ArrowLeft class="w-4 h-4 transition-all duration-300 group-hover:-translate-x-1.5 opacity-70 group-hover:opacity-100 relative z-10" />
            <span class="relative z-10 tracking-wide">{{ t('role.back') }}</span>
          </button>
          <button
            :disabled="isConnecting"
            class="px-5 py-4 rounded-2xl font-bold text-white transition-all hover:opacity-90 active:scale-[0.98] flex-[2] flex justify-center items-center"
            :style="{ background: themeConfig.colors.primaryBtnBg }"
            @click="connectToServer"
          >
            <Loader2
              v-if="isConnecting"
              class="w-5 h-5 animate-spin"
            />
            <span v-else>{{ t('role.connect_server') }}</span>
          </button>
        </div>
        <p
          v-if="connectError"
          class="text-red-500 text-sm mt-3 text-center font-medium"
        >
          {{ connectError }}
        </p>
      </div>

      <!-- 服务端配置 -->
      <div
        v-if="selectedRole === 'server'"
        class="animate-in fade-in slide-in-from-bottom-4 duration-300"
      >
        <div class="mb-4">
          <label
            class="block text-sm font-bold mb-2 ml-1"
            :style="{ color: themeConfig.colors.textStrong }"
          >{{ t('role.server_password') }}</label>
          <input
            v-model="serverPassword"
            type="password"
            :placeholder="t('role.server_password_ph')"
            class="w-full px-5 py-4 rounded-2xl bg-surface border-2 outline-none transition-all placeholder:text-text/30"
            :style="{ borderColor: themeConfig.colors.borderSoft, color: themeConfig.colors.textStrong }"
            @focus="(e) => (e.target as HTMLElement).style.borderColor = themeConfig.colors.borderStrong"
            @blur="(e) => (e.target as HTMLElement).style.borderColor = themeConfig.colors.borderSoft"
          >
        </div>
        <div class="flex gap-3 mt-6">
          <button
            class="group relative overflow-hidden px-5 py-4 rounded-2xl font-bold transition-all duration-300 flex-1 flex items-center justify-center gap-2 active:scale-95 shadow-sm hover:shadow-md"
            :style="{ background: 'var(--color-surface)', color: 'var(--color-text-strong)', border: '1px solid var(--color-border-soft)' }"
            @mouseover="(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--color-border-strong)'; (e.currentTarget as HTMLElement).style.transform = 'translateY(-2px)' }"
            @mouseleave="(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--color-border-soft)'; (e.currentTarget as HTMLElement).style.transform = 'translateY(0)' }"
            @click="selectedRole = null"
          >
            <div class="absolute inset-0 bg-gradient-to-r from-transparent via-white/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500"></div>
            <div class="absolute inset-0 opacity-0 group-hover:opacity-5 transition-opacity duration-300" style="background: currentColor;"></div>
            <ArrowLeft class="w-4 h-4 transition-all duration-300 group-hover:-translate-x-1.5 opacity-70 group-hover:opacity-100 relative z-10" />
            <span class="relative z-10 tracking-wide">{{ t('role.back') }}</span>
          </button>
          <button
            :disabled="isStarting"
            class="px-5 py-4 rounded-2xl font-bold text-white transition-all hover:opacity-90 active:scale-[0.98] flex-[2] flex justify-center items-center"
            :style="{ background: themeConfig.colors.primaryBtnBg }"
            @click="startServer"
          >
            <Loader2
              v-if="isStarting"
              class="w-5 h-5 animate-spin"
            />
            <span v-else>{{ t('role.start_server') }}</span>
          </button>
        </div>
        <p
          v-if="startError"
          class="text-red-500 text-sm mt-3 text-center font-medium"
        >
          {{ startError }}
        </p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { Monitor, Server, Loader2, Globe, ArrowLeft } from 'lucide-vue-next';
import { currentTheme as themeConfig } from '../theme';
import { SysApi, DbApi } from '../api';
import { useStorage } from '@vueuse/core';
import { useI18n } from 'vue-i18n';
import { getLocaleLabel, getNextLocale, setAppLocale } from '../i18n';

const { t, locale } = useI18n();

const currentLangLabel = computed(() => getLocaleLabel(locale.value));

const cycleLanguage = () => {
  const nextLang = setAppLocale(getNextLocale(locale.value), { notify: true });
  locale.value = nextLang;
  DbApi.saveSetting({ key: 'language', value: JSON.stringify(nextLang) }).catch(() => {});
};

const emit = defineEmits(['role-selected']);

const selectedRole = ref<'client' | 'server' | null>(null);

// 客户端状态
const serverUrl = useStorage('vrc_server_url', 'http://127.0.0.1:11451');
const isConnecting = ref(false);
const connectError = ref('');

// 服务端状态
const serverPassword = ref('');
const isStarting = ref(false);
const startError = ref('');

const selectRole = (role: 'client' | 'server') => {
  selectedRole.value = role;
  connectError.value = '';
  startError.value = '';
};

const connectToServer = async () => {
  if (!serverUrl.value) {
    connectError.value = t('role.error_require_url');
    return;
  }
  isConnecting.value = true;
  connectError.value = '';
  try {
    let finalUrl = serverUrl.value.trim();
    if (finalUrl.includes('0.0.0.0')) {
      finalUrl = finalUrl.replace('0.0.0.0', '127.0.0.1');
      serverUrl.value = finalUrl;
    }
    if (finalUrl.endsWith('/')) {
      finalUrl = finalUrl.slice(0, -1);
      serverUrl.value = finalUrl;
    }
    await SysApi.pingServer({ url: finalUrl });
    // 连接成功
    emit('role-selected', { role: 'client', url: finalUrl });
  } catch (err: any) {
    const msg = t('auto_90de4c5d') + (err.message || err);
    connectError.value = msg;
  } finally {
    isConnecting.value = false;
  }
};

const startServer = async () => {
  if (!serverPassword.value) {
    startError.value = t('role.error_require_pwd');
    return;
  }
  
  isStarting.value = true;
  startError.value = '';
  try {
    await SysApi.verifyServerPassword({ password: serverPassword.value });
    emit('role-selected', { role: 'server' });
  } catch (err: any) {
    startError.value = (err.message || err);
  } finally {
    isStarting.value = false;
  }
};
</script>
