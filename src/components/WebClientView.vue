<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { isTauri } from '@tauri-apps/api/core';
import { useAuthStore } from '../stores/authStore';
import { useUiStore } from '../stores/uiStore';
import { VrcApi, ServerApi, DbApi } from '../api';
import { currentTheme } from '../theme';
import {
  Users, Globe, Server, RefreshCw, LogOut, Wifi, WifiOff, UserCircle, Settings
} from 'lucide-vue-next';

const { t } = useI18n();
const authStore = useAuthStore();
const uiStore = useUiStore();

const serverUrl = computed(() => authStore.clientServerUrl || window.location.origin);
const serverConnected = computed(() => authStore.serverConnected);
const reconnectCountdown = computed(() => authStore.reconnectCountdown);
const currentUser = computed(() => authStore.currentUser);

const loading = ref(false);
const error = ref('');
const statusText = ref('');

const features = computed(() => {
  const menus = uiStore.serverMenuPerms || {};
  return Object.entries(menus).map(([key, enabled]) => ({
    key,
    label: key,
    enabled: enabled !== false,
  }));
});
const pendingSurveyCount = computed(() => authStore.pendingSurveyCount);

const connect = async () => {
  loading.value = true;
  error.value = '';
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/client/register`, {
      method: 'POST',
      params: {
        user_id: currentUser.value?.id || currentUser.value?.displayName,
        display_name: currentUser.value?.displayName || '',
        avatar_url: currentUser.value?.currentAvatarThumbnailImageUrl || '',
      },
      timeoutMs: 5000,
      maxRetries: 1,
    });
    statusText.value = data.status === 'ok' ? '已连接' : data.status;
  } catch (err: any) {
    error.value = err?.message || '连接失败';
  } finally {
    loading.value = false;
  }
};

onMounted(() => {
  if (!serverConnected.value) {
    connect();
  }
});
</script>

<template>
  <div class="w-full h-full relative" :style="{ background: currentTheme.colors.bgMain }">
    <div class="max-w-3xl mx-auto px-4 py-8">
      <div class="flex items-center justify-between mb-8">
        <div class="flex items-center gap-3">
          <img :src="currentTheme.logo" class="w-10 h-10 rounded-full border-2 border-border-soft" />
          <div>
            <h1 class="text-xl font-bold" :style="{ color: currentTheme.colors.textStrong }">
              {{ t('app.title') || 'VrcDog' }} - Web Client
            </h1>
            <p class="text-xs" :style="{ color: currentTheme.colors.textMuted }">
              {{ serverUrl }}
            </p>
          </div>
        </div>
        <button
          @click="authStore.handleLogout(false)"
          class="flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm font-bold transition-colors"
          :style="{ background: currentTheme.colors.surfaceHover, color: currentTheme.colors.textStrong }"
        >
          <LogOut :size="14" />
          {{ t('app.logout') }}
        </button>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div
          class="p-4 rounded-xl border"
          :style="{ background: currentTheme.colors.surface, borderColor: currentTheme.colors.borderSoft }"
        >
          <div class="flex items-center gap-2 mb-3">
            <UserCircle :size="18" />
            <h2 class="font-bold" :style="{ color: currentTheme.colors.textStrong }">用户信息</h2>
          </div>
          <div v-if="currentUser" class="space-y-1 text-sm" :style="{ color: currentTheme.colors.textSoft }">
            <p><strong>用户名:</strong> {{ currentUser.displayName }}</p>
            <p><strong>ID:</strong> {{ currentUser.id }}</p>
          </div>
        </div>

        <div
          class="p-4 rounded-xl border"
          :style="{ background: currentTheme.colors.surface, borderColor: currentTheme.colors.borderSoft }"
        >
          <div class="flex items-center gap-2 mb-3">
            <Wifi :size="18" />
            <h2 class="font-bold" :style="{ color: currentTheme.colors.textStrong }">连接状态</h2>
          </div>
          <div class="flex items-center gap-2 text-sm" :style="{ color: currentTheme.colors.textSoft }">
            <span
              class="inline-block w-2.5 h-2.5 rounded-full"
              :class="serverConnected ? 'bg-emerald-500' : 'bg-red-500'"
            />
            {{ serverConnected ? (t('app.server_connected') || '已连接') : (t('app.server_disconnected') || '未连接') }}
          </div>
          <p v-if="error" class="mt-2 text-sm text-red-500">{{ error }}</p>
          <button
            v-if="!serverConnected"
            @click="connect"
            :disabled="loading"
            class="mt-3 flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm font-bold text-white transition-colors"
            :style="{ background: currentTheme.colors.primaryBtnBg }"
          >
            <RefreshCw :size="14" :class="{ 'animate-spin': loading }" />
            重新连接
          </button>
        </div>
      </div>

      <div
        v-if="features.length"
        class="mt-6 p-4 rounded-xl border"
        :style="{ background: currentTheme.colors.surface, borderColor: currentTheme.colors.borderSoft }"
      >
        <div class="flex items-center gap-2 mb-3">
          <Settings :size="18" />
          <h2 class="font-bold" :style="{ color: currentTheme.colors.textStrong }">可用功能</h2>
        </div>
        <div class="flex flex-wrap gap-2">
          <span
            v-for="f in features"
            :key="f.key"
            class="px-2 py-1 rounded-md text-xs font-bold"
            :style="{ background: currentTheme.colors.surfaceHover, color: currentTheme.colors.textSoft }"
          >
            {{ f.label }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>
