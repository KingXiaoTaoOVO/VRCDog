<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { Bell, Loader2, UserPlus, Check, X, Megaphone, HelpCircle } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import type { VrcNotification } from '../types/vrc';

const { t } = useI18n();

const notifications = ref<VrcNotification[]>([]);
const loading = ref(true);
const errorMsg = ref('');
const processingId = ref<string | null>(null);

const fetchNotifications = async () => {
  loading.value = true;
  errorMsg.value = '';
  try {
    const res: any = await DbApi.getNotifications({ limit: 100, offset: 0 });
    notifications.value = Array.isArray(res) ? res : [];
  } catch (err: any) {
    errorMsg.value = err.message || err;
  } finally {
    loading.value = false;
  }
};

const acceptNotification = async (id: string) => {
  processingId.value = id;
  try {
    await VrcApi.acceptNotification({ notificationId: id });
    await DbApi.deleteNotification({ id }); // 成功后从本地缓存删除
    await fetchNotifications();
  } catch (err: any) {
    console.error('Accept notification failed:', err);
  } finally {
    processingId.value = null;
  }
};

const hideNotification = async (id: string) => {
  processingId.value = id;
  try {
    await VrcApi.hideNotification({ notificationId: id });
    await DbApi.deleteNotification({ id }); // 成功后从本地缓存删除
    await fetchNotifications();
  } catch (err: any) {
    console.error('Hide notification failed:', err);
  } finally {
    processingId.value = null;
  }
};

onMounted(() => {
  fetchNotifications();
  window.addEventListener('vrc-notifications-synced', fetchNotifications);
  window.addEventListener('vrc-pipeline-event', handlePipelineEvent);
});

onUnmounted(() => {
  window.removeEventListener('vrc-notifications-synced', fetchNotifications);
  window.removeEventListener('vrc-pipeline-event', handlePipelineEvent);
});

const handlePipelineEvent = (e: Event) => {
  const json = (e as CustomEvent).detail;
  if (json && (json.type === 'notification' || json.type === 'hide-notification' || json.type === 'clear-notification')) {
    fetchNotifications();
  }
};

const getNotificationIcon = (type: string) => {
  switch (type) {
    case 'friendRequest': return UserPlus;
    case 'invite': return Megaphone;
    default: return HelpCircle;
  }
};

const parseDetails = (details: string | any) => {
  if (typeof details === 'string') {
    try { return JSON.parse(details); } catch { return {}; }
  }
  return details || {};
};
</script>

<template>
  <div class="h-full flex flex-col">
    <div class="flex items-center justify-between mb-4">
      <h1 class="text-2xl font-extrabold text-[#451a03] tracking-tight flex items-center gap-2">
        <Bell
          class="text-amber-500"
          :size="28"
        /> {{ t('notifications.title') }}
      </h1>
      <button
        :disabled="loading"
        class="p-2 rounded-xl bg-amber-100 text-amber-600 hover:bg-amber-200 transition-colors"
        @click="fetchNotifications"
      >
        <Loader2
          v-if="loading"
          class="animate-spin"
          :size="20"
        />
        <Bell
          v-else
          :size="20"
        />
      </button>
    </div>

    <!-- 错误 -->
    <div
      v-if="errorMsg"
      class="bg-red-50 text-red-600 p-3 rounded-xl border border-red-200 text-sm font-bold mb-4"
    >
      {{ errorMsg }}
    </div>

    <div class="flex-1 overflow-y-auto pr-1">
      <div
        v-if="loading && notifications.length === 0"
        class="flex items-center justify-center py-12 text-amber-500"
      >
        <Loader2
          class="animate-spin mr-2"
          :size="24"
        /> {{ t('notifications.loading') }}
      </div>

      <div
        v-else-if="notifications.length === 0"
        class="text-center text-amber-500 py-12 text-sm bg-white/50 backdrop-blur rounded-2xl border-2 border-dashed border-amber-200"
      >
        <Bell
          class="mx-auto mb-3 opacity-50"
          :size="48"
        />
        {{ t('notifications.empty') }}
      </div>

      <div
        v-else
        class="space-y-3"
      >
        <div
          v-for="notif in notifications"
          :key="notif.id" 
          class="bg-white/80 backdrop-blur rounded-xl p-4 border border-amber-50 shadow-sm flex items-start gap-3 transition-all hover:border-amber-300"
        >
          <div class="w-10 h-10 rounded-full bg-amber-100 flex items-center justify-center flex-shrink-0 text-amber-600">
            <component
              :is="getNotificationIcon(notif.type)"
              :size="20"
            />
          </div>
          
          <div class="flex-1 min-w-0">
            <h3 class="font-bold text-amber-900 text-sm mb-1">
              {{ notif.message || t('notifications.system') }}
            </h3>
            <p
              v-if="notif.details && parseDetails(notif.details) !== '{}'"
              class="text-xs text-amber-700 bg-amber-50 p-2 rounded-lg break-words whitespace-pre-wrap leading-relaxed border border-amber-100"
            >
              {{ parseDetails(notif.details)?.worldName || notif.details }}
            </p>
            <p class="text-[10px] text-amber-500 mt-2 font-mono">
              {{ new Date(notif.created_at).toLocaleString() }}
            </p>
          </div>

          <div class="flex flex-col gap-2 flex-shrink-0">
            <button
              v-if="notif.type === 'friendRequest' || notif.type === 'invite'" 
              :disabled="processingId === notif.id"
              class="w-10 h-10 rounded-xl bg-green-500 text-white hover:bg-green-600 transition-colors flex items-center justify-center shadow-md shadow-green-500/20 disabled:opacity-50"
              @click="acceptNotification(notif.id)"
            >
              <Loader2
                v-if="processingId === notif.id"
                class="animate-spin"
                :size="16"
              />
              <Check
                v-else
                :size="18"
              />
            </button>
            <button 
              :disabled="processingId === notif.id"
              class="w-10 h-10 rounded-xl bg-gray-100 text-gray-500 hover:bg-red-50 hover:text-red-500 transition-colors flex items-center justify-center disabled:opacity-50"
              @click="hideNotification(notif.id)"
            >
              <X :size="18" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
