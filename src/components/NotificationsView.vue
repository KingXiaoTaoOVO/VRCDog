<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { VrcApi, DbApi, SysApi, GamelogApi } from "../api";
import { Bell, Loader2, UserPlus, Check, X, Megaphone, HelpCircle, UsersRound, MessageSquare, Info } from 'lucide-vue-next';
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
    case 'invite': 
    case 'requestInvite': return Megaphone;
    case 'group.invite':
    case 'group.request': return UsersRound;
    case 'message': return MessageSquare;
    case 'group.informational':
    case 'group.announcement': return Info;
    default: return HelpCircle;
  }
};

const parseDetails = (details: string | any) => {
  if (!details) return null;
  if (typeof details === 'string') {
    try { 
      const parsed = JSON.parse(details); 
      return Object.keys(parsed).length > 0 ? parsed : null;
    } catch { return details; }
  }
  return Object.keys(details).length > 0 ? details : null;
};

const renderDetails = (details: any) => {
  const parsed = parseDetails(details);
  if (!parsed) return '';
  if (typeof parsed === 'string') return parsed;
  return parsed.worldName || parsed.message || parsed.imageUrl || JSON.stringify(parsed);
};
</script>

<template>
  <div class="h-full flex flex-col p-6 bg-surface-hover rounded-3xl relative overflow-hidden">
    <!-- Subtle Background Glow -->
    <div class="absolute top-0 right-0 w-96 h-96 bg-indigo-500/10 rounded-full blur-[100px] pointer-events-none -z-10" />
    <div class="absolute bottom-0 left-0 w-[500px] h-[500px] bg-blue-500/5 rounded-full blur-[120px] pointer-events-none -z-10" />

    <div class="flex items-center justify-between mb-8 shrink-0 z-10">
      <h1 class="text-3xl font-extrabold text-text tracking-tight flex items-center gap-3">
        <span class="inline-flex items-center justify-center p-2 bg-indigo-100 rounded-2xl shadow-sm border border-indigo-200/50">
          <Bell class="w-6 h-6 text-indigo-600" />
        </span>
        {{ t('notifications.title') }}
      </h1>
      <button
        :disabled="loading"
        class="p-2.5 rounded-xl bg-surface border border-border-soft shadow-sm text-text-muted hover:text-indigo-600 hover:border-indigo-200 transition-all disabled:opacity-50"
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
      class="bg-red-50 text-red-600 p-3 rounded-xl border border-red-200 text-sm font-bold mb-4 z-10"
    >
      {{ errorMsg }}
    </div>

    <div class="flex-1 overflow-y-auto pr-2 custom-scrollbar z-10 relative">
      <div
        v-if="loading && notifications.length === 0"
        class="absolute inset-0 flex flex-col items-center justify-center text-indigo-500/80 bg-surface-hover backdrop-blur-sm z-10"
      >
        <Loader2
          class="animate-spin mb-4"
          :size="48"
        />
        <span class="font-extrabold text-lg tracking-wide">{{ t('notifications.loading') }}</span>
      </div>

      <div
        v-else-if="notifications.length === 0"
        class="h-full flex flex-col items-center justify-center text-border-strong"
      >
        <Bell
          class="mb-4 opacity-30"
          :size="64"
        />
        <p class="font-bold text-xl text-text-muted">
          {{ t('notifications.empty') }}
        </p>
      </div>

      <div
        v-else
        class="grid gap-3 pb-10"
      >
        <div
          v-for="notif in notifications"
          :key="notif.id" 
          class="bg-surface backdrop-blur-xl rounded-2xl p-4 border border-border-soft shadow-sm flex items-start gap-4 transition-all hover:border-indigo-300 hover:shadow-md group"
        >
          <div class="w-12 h-12 rounded-2xl bg-indigo-50 border border-indigo-100 flex items-center justify-center flex-shrink-0 text-indigo-500 group-hover:scale-105 transition-transform">
            <component
              :is="getNotificationIcon(notif.type)"
              :size="24"
            />
          </div>
          
          <div class="flex-1 min-w-0 py-1">
            <h3 class="font-bold text-text text-base mb-1 truncate">
              {{ notif.message || t('notifications.system') }}
            </h3>
            <p
              v-if="renderDetails(notif.details)"
              class="text-sm text-text-muted bg-surface-hover p-3 rounded-xl break-words whitespace-pre-wrap leading-relaxed border border-border-soft"
            >
              {{ renderDetails(notif.details) }}
            </p>
            <p class="text-[11px] text-border-strong mt-2 font-mono font-bold tracking-wider">
              {{ new Date(notif.created_at).toLocaleString() }}
            </p>
          </div>

          <div class="flex flex-col gap-2 flex-shrink-0">
            <button
              v-if="['friendRequest', 'invite', 'requestInvite', 'group.invite', 'group.request'].includes(notif.type)"
              :disabled="processingId === notif.id"
              class="w-10 h-10 rounded-xl bg-green-500 text-white hover:bg-green-600 transition-colors flex items-center justify-center shadow-sm disabled:opacity-50"
              @click="acceptNotification(notif.id)"
            >
              <Loader2
                v-if="processingId === notif.id"
                class="animate-spin"
                :size="18"
              />
              <Check
                v-else
                :size="20"
              />
            </button>
            <button 
              :disabled="processingId === notif.id"
              class="w-10 h-10 rounded-xl bg-background/10 text-text-muted hover:bg-red-50 hover:text-red-500 hover:border hover:border-red-200 transition-all flex items-center justify-center disabled:opacity-50"
              @click="hideNotification(notif.id)"
            >
              <X :size="20" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.custom-scrollbar::-webkit-scrollbar { width: 6px; }
.custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
.custom-scrollbar::-webkit-scrollbar-thumb { background: #cbd5e1; border-radius: 10px; }
.custom-scrollbar::-webkit-scrollbar-thumb:hover { background: #94a3b8; }
</style>
